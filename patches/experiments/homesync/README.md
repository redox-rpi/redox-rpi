# Experiment homesync: back up the home directory over SSH

For [#712](https://github.com/redox-rpi/redox-rpi/issues/712), stage 1 of
[home-server-storage.md](../../../docs/home-server-storage.md). Needs the
experiments `sshd` and `timesync` (file `needs`). The OpenSSH build of
`sshd` gives the `ssh` client and `ssh-keygen`, and its relibc fixes (poll,
setitimer) are needed by the client too. `timesync` sets the clock, so the
backup folders get real dates instead of 1970-01-01.

- Package `rsync` (upstream `net/rsync` 3.4.1, with openssl3, zstd, lz4,
  xxhash).
- Package `homesync` (`tools/homesync`), a small command around ssh and
  rsync:

  ```
  homesync setup <user>@<host>[:<port>]:<dir>   key, test login, remember the server
  homesync push                                copy changes to <dir>/current/
  homesync pull [<path>]                       restore into ~/restore-<date>/
  homesync status                              server, last push, what would change
  ```

  - `setup` makes `~/.ssh/id_ed25519` if it is missing and prints the
    public key for the server account's `authorized_keys`. The server's
    host key is pinned on first use (`StrictHostKeyChecking=accept-new`).
    The server goes to `~/.config/homesync/server`.
  - `push` is a backup: `rsync -a --delete --backup` into `<dir>/current/`;
    files changed or deleted on the Pi keep their old version on the
    server in `<dir>/deleted/<date>/`. Excludes from
    `~/.config/homesync/exclude` (default `.cache/` and `/restore-*/`,
    written when missing). One summary line; the result goes to
    `~/.cache/homesync/last`.
  - `pull` never writes over local files: it restores into a new folder.
  - Server away: `homesync: server <host> port <port> not reachable,
    nothing changed`, exit 1 (ssh and rsync exit with 255).
  - Dates are UTC, from the clock `timesync` sets. Before its first NTP
    answer (or without it) the clock may still be at the build time or
    in 1970, so the folder names are then only in order.
  - Keys and passwords never go on the command line.
- Needs the default relibc patch `0028`: `getsockname` on a socket
  that is not bound or connected yet returns `0.0.0.0:0`. Without it
  relibc panics with "missing address", because OpenSSH's `ssh` client
  calls it before it connects.

## Test

`scripts/test-homesync.sh` (QEMU, after `scripts/build-image.sh
--experiment sshd --experiment timesync --experiment homesync`): a throwaway `sshd` as the
calling user on a random high port of the build host, with its own host
key and `authorized_keys` under `build/homesync-test/`, ended with the
script. The guest makes 100 files of 64 KiB, then setup, push, change one
file and push (only it is sent), delete one and push (it moves to
`deleted/<date>/`), status, pull and compare checksums, and a push to a
port where nothing listens (message, exit 1). The server folder is checked
on the host.

A Pi row needs a server on the lab network; a permanent one is a host
change for the admin session.
