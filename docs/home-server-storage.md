# Storing your files on a home server

Status: **design note, draft**, 2026-11-24 (Architect). Not an RFC yet;
see [When this becomes an RFC](#when-this-becomes-an-rfc).

## Goal

People who use Redox on a Pi 400 at home keep their files on a home
server:

- the files survive a lost or rewritten SD card (backup);
- the same files can be used from other machines;
- the Pi still works when the server or the network is away, and nothing
  written then is lost.

Two stages: first a backup and sync command built from tools that already
exist, then a file system daemon that shows a server directory under
`/scheme/server/`.

## What Redox has today

| Piece | State |
|---|---|
| SSH | OpenSSH from source, experiment `sshd` ([#45](https://github.com/redox-rpi/redox-rpi/issues/45)): key and password login tested on the Pi; `authorized_keys` on the data partition survives a reset |
| rsync, curl, OpenSSL 3, CA certificates | upstream recipes (`net/rsync` 3.4.1, `net/curl`, `libs/openssl3`, `data/ca-certificates`); not yet built and tested for the Pi by this project |
| rclone | upstream `wip` recipe only |
| TLS in Rust | rustls 0.23 with Redox's `ring` fork builds for `aarch64-unknown-redox` and works in QEMU (branch `mqtt-tls`, [#56](https://github.com/redox-rpi/redox-rpi/issues/56)) |
| Network file systems | none: no NFS, no SMB, no FUSE |
| Identity, keys | [RFC 0001](rfcs/0001-identity-and-secure-channels.md) (`keys:`, `secure:`, `accounts:`): draft, nothing built yet |
| Clock | right only after `timesync` ([time.md](time.md)); TLS certificate checks fail before that |

Redox needs no FUSE. A file system in user space is a normal daemon that
registers a scheme; programs open `/scheme/server/…` like any file.
RedoxFS, the FAT driver and `ramfs` work the same way.

## Stage 1: `homesync`, backup over SSH

A small command around rsync over SSH. No new daemon.

```
homesync setup <user>@<server>:<dir>   create the key, show it, test the login
homesync push                          copy changes from the Pi to the server
homesync pull [<path>]                 restore from the server
homesync status                        last run, server reachable, what would change
```

- **Direction:** push is a backup, the Pi is the source. Files deleted on
  the Pi are moved to a dated folder on the server (`rsync --backup
  --backup-dir`), not deleted. Pull restores, by default into a new
  folder, so it never overwrites newer local files. No two-way sync in this
  stage, so no conflicts.
- **What:** the user's home directory, without caches; a list of excludes
  in `~/.config/homesync/exclude`.
- **When:** by hand, and once a day from a `timerd` timer
  ([time.md](time.md)) if the user wants it.
- **Keys:** `setup` makes an Ed25519 key in `~/.ssh` and prints the public
  key, to be added to the server account's `authorized_keys`. It pins the
  server's host key on first use (SSH's normal `known_hosts`).
- **Server:** any machine with `sshd` and `rsync`: a Linux box, most NAS
  systems. One server account per Pi user, or one directory per user in a
  shared account restricted to `rsync` by a forced command.

## Stage 2: `serverfs`, a server directory as a scheme

A daemon that talks WebDAV over HTTPS to the server and serves the
directory as `/scheme/server/<name>/…`. The user can link it into the home
directory (for example `~/Server`).

### Why WebDAV

- It is plain HTTP: one TLS stack (rustls) and a small client are enough.
- Many servers speak it: `rclone serve webdav`, nginx, Apache, Nextcloud,
  most NAS systems. macOS, iOS, Windows and Linux file managers can open
  the same share.
- The part we need is small (below), so a server that is not a plain file
  share can serve it too: a program that stores its data elsewhere, for
  example in a database, and shows it as collections and files.

### The WebDAV subset (server profile)

`serverfs` uses only:

| Request | Use |
|---|---|
| `PROPFIND`, `Depth: 0` and `1` | stat and directory listing: name, size, modification time, `getetag`, collection or not |
| `GET` (with `Range` later) | read a file |
| `PUT` with `If-Match` / `If-None-Match: *` | write a file without overwriting a change from elsewhere |
| `DELETE` with `If-Match` | delete |
| `MKCOL` | create a directory |
| `MOVE` with `Overwrite: F` | rename |

Not used: `LOCK`/`UNLOCK`, `PROPPATCH`, `COPY`, `Depth: infinity`. A
server must give every file an ETag that changes when the content
changes; that is what makes offline use and conflict checks work. This
table is the contract for any server, including a custom one later.

### Mapping scheme calls

| Program does | `serverfs` does |
|---|---|
| `open` for reading | serves from the cache if the ETag is current (checked with a `PROPFIND`, at most every few seconds per directory); otherwise `GET` into the cache |
| `read`, `mmap` | from the cache file |
| `write` | into the cache file, marked dirty |
| `close`, `fsync` | `PUT` with `If-Match: <old ETag>`; on success store the new ETag |
| `getdents` | `PROPFIND Depth: 1`, cached |
| `unlink`, `rename`, `mkdir` | `DELETE`, `MOVE`, `MKCOL` |

First version: whole files only. Partial reads with `Range` come when big
files (videos) need them.

### Cache and offline use

- The cache lives on the local RedoxFS, per user
  (`~/.cache/serverfs/<name>/`), with a small index: path, ETag, size,
  modification time, dirty or clean.
- A size limit; clean files are dropped oldest first. Files the user marks
  "keep offline" are never dropped.
- **Server away:** cached files can be read and written; changes are
  queued and uploaded when the server is back. Files that are not cached
  fail with `EHOSTUNREACH`, and listings show the last known state.
- `/scheme/server/<name>/.status` shows: online or not, number of queued
  changes, conflicts.

### Conflicts

A conflict is a `PUT` or `DELETE` that the server refuses with
`412 Precondition Failed`: the file changed on the server since the Pi
last saw it.

- Nothing is overwritten silently. The server's version keeps the name;
  the Pi's version is uploaded next to it as
  `<name> (conflict <device> <date>).<ext>`, and `.status` lists it.
- A delete of a file that changed on the server is dropped; the changed
  file stays.
- Directories: a rename into a name that exists (`MOVE` with
  `Overwrite: F` refused) is reported as `EEXIST`, as on a local disk.

## Logins and keys

| | Stage 1 | Stage 2, first version | Later, with RFC 0001 |
|---|---|---|---|
| Server proves who it is | SSH host key, pinned on first use | TLS certificate: the home CA's certificate or the server's own, pinned in the user's config; public CAs work too | the server's key in `trust:` |
| User proves who they are | SSH key in `~/.ssh` | user name and an app password over TLS (HTTP Basic), stored in a file only the user can read | key in `keys:`; client certificate signed through `keys:`, or a `secure:` channel with the user's identity checked by the server's `accounts:` |
| Where the private key is | user's files | user's files | inside `keyd` only |

- **Per user:** each user has their own server account (or app password)
  and their own cache. `serverfs` runs **once per user**, started at
  login, so one user's password or key never sits in a process that serves
  another user. Each instance registers its own scheme; the user's
  namespace shows it as `server`.
- App passwords first, because every WebDAV server supports them. A client
  certificate is the next step: rustls can sign the handshake through an
  external signer, so the key can move into `keyd` without changes to the
  protocol.
- The transport is a small interface inside `serverfs`: `tcp:` with TLS
  now, `secure:` (RFC 0001) when it exists and the server speaks it. The
  WebDAV part does not change.

## TLS in Redox today

- rustls 0.23 with Redox's `ring` fork builds and works (branch
  `mqtt-tls`); `serverfs` uses the same, plus `webpki-roots` for public
  CAs. OpenSSL 3 and curl have upstream recipes but are not needed.
- Every program that uses TLS links its own copy; there is no system TLS
  service. RFC 0001's `secure:` would be that for Redox-to-Redox and
  Redox-to-HomeNode connections.
- The clock must be right before a certificate can be checked. `serverfs`
  waits for `timesync` (or a clock module) before it connects and says so
  in `.status`; until then it works offline from the cache.

## The server side

| Server | Stage 1 | Stage 2 | Notes |
|---|---|---|---|
| Linux or NAS with `sshd` and `rsync` | yes | — | simplest; one account per user |
| `rclone serve webdav` | — | yes | one program; `--htpasswd` for users, `--cert`/`--key` for TLS, `--client-ca` for client certificates later; good for tests |
| nginx with `dav` and the `dav-ext` module | — | yes | `dav-ext` is needed for `PROPFIND`; ETags on PUT need checking |
| Apache `mod_dav` | — | yes | full WebDAV, ETags and `If-Match` |
| Nextcloud | — | yes | WebDAV under `remote.php/dav`, app passwords built in |
| A custom server, for example a home server program in Rust | — | yes, if it serves the profile above | data can live in any store as long as paths, listings and ETags are stable |

## Smallest first step

**Stage 1, on the Pi 3B+ (Pi 400 when it is in the lab):**

1. Build the `rsync` recipe and the OpenSSH client (the `sshd` experiment
   already builds OpenSSH) for the Pi, as an experiment `homesync`.
2. A Pi test against a test server on the lab network: push a folder of
   about 100 files (50 MB), change one file, push again (only that file
   goes over the wire), delete one, push (it lands in the dated folder),
   restore into `/tmp` and compare checksums. Also a push while the server
   is off: a clear error, nothing changed.
3. Then the `homesync` script around it.

**Stage 2 spike, in QEMU first:** `serverfs` read-only (`PROPFIND` and
`GET`) against `rclone serve webdav` over plain HTTP in the lab, then with
TLS. Writes, cache and offline queue after that.

## Steps

| Step | Content | Test |
|---|---|---|
| 1 | rsync and ssh client for the Pi (experiment), [#712](https://github.com/redox-rpi/redox-rpi/issues/712) | Pi row as above |
| 2 | `homesync` command and daily timer ([#712](https://github.com/redox-rpi/redox-rpi/issues/712)) | Pi row: setup, push, pull, server off |
| 3 | `serverfs` read-only over HTTP, then TLS, [#713](https://github.com/redox-rpi/redox-rpi/issues/713) | QEMU against `rclone serve webdav` |
| 4 | TLS, pinned certificate, app password | QEMU and Pi |
| 5 | writes with `If-Match`, cache, offline queue, conflicts | QEMU tests that stop the server mid-way; Pi row |
| 6 | per-user start at login, namespace name `server` | Pi, two users |
| 7 | keys from `keyd`, `secure:` transport | after RFC 0001 steps 1 and 2 |

## Open questions

- Should a user's whole home directory live on the server (with the cache
  as the local copy), or only a folder in it? This note assumes a folder;
  the whole home needs `serverfs` up before login.
- How the per-user scheme gets the name `server` in the user's namespace,
  and who starts it (login, or a user session manager).
- Cache size default on a 16 GB card.

## When this becomes an RFC

Stage 1 is a tool and needs no RFC. Stage 2 adds a system scheme that
programs depend on, and ties into login and RFC 0001. The interface
(`/scheme/server/…`, `.status`, the per-user start) should become an RFC
before `serverfs` goes into a default image, not before the spike.
