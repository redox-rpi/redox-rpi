# Release image (`REL`)

Topics: [ci](../topics/ci.md), [release-process.md](../release-process.md).
Trace: [trace-v0.4.0.md](trace-v0.4.0.md).

### REQ-REL-1 No remote shell in the release image

The release and candidate SD card images shall not contain `devshd` or any
other unencrypted remote login.

| Field | Value |
|---|---|
| Rationale | `devshd` gives a root login over plain TCP; a downloaded image must not open it. |
| Priority | must |
| Kind | constraint |
| Level | release |
| PBS | 2.6.1, 3.3 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | [release-process.md](../release-process.md#building), `config/raspi3bp-sdcard.toml` |
| Verification | inspection by Release & CI at the candidate (criterion 1, release checklist item) and test, person, in the visit (criterion 2: checklist v0.4.0 test 23, row 286, pass on the Pi 3B+ and the Zero 2 W); criterion 1 agreed with V&V 2026-11-08, not run yet. Automated rows cannot check it: they run on the netboot image with `devshd` |

Acceptance criteria:

1. On the published SD image, `tools/redoxfs-file <img> get /usr/bin/devshd
   <file>` fails with "not found", and none of the image's init service files
   names `devshd`.
2. On a booted release image, `nc -z -w 3 <pi> 2323` from the build host
   fails, and `ps` on the console shows no `devshd`.

### REQ-REL-2 The image names its release and build

The release image shall show its release name and build ID above the login
prompt and keep them in `/etc/redox-rpi-release` and `/.uuid`.

| Field | Value |
|---|---|
| Rationale | A user report or a test must name the exact build. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.6.1, 4.1 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #237 (`/.uuid` for SwiftFlash) |
| Verification | test, automated: checklist v0.4.0 test 1 (pi-test `version`), rows 292 (v0.4.0 build) and 285 (rc1) (banner on 7 boots); `/.uuid` checked at the rc1 build |

Acceptance criteria:

1. The login banner shows `redox-rpi <release> (<build ID>)`.
2. `/etc/redox-rpi-release` has `RELEASE=<release>` and the build ID.
3. `/.uuid` has id `redox-rpi`, the version and the commit.

### REQ-REL-3 Reproducible release build

A release or candidate shall be built from a clean commit, without
`--allow-dirty`, and match its manifest.

| Field | Value |
|---|---|
| Rationale | Results only count for a build anyone can rebuild ([PMP](../process/pmp.md)). |
| Priority | must |
| Kind | constraint |
| Level | release |
| PBS | 3.1 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | `config/upstream.lock`, drift check, #257 (rc1 built from a red commit) |
| Verification | test, automated: checklist v0.4.0 test 1 (pi-test `manifest`), rows 292 (v0.4.0 build) and 285 (rc1) |

Acceptance criteria:

1. `/etc/redox-rpi-manifest` on the Pi matches the published manifest
   (build ID, libc hash).
2. The build ID has no `-dirty` part.
3. CI on the release commit is green (proposed, #257).

### REQ-REL-4 Published and checkable download

Each candidate and release shall be downloadable from the image server with
a checksum and a manifest.

| Field | Value |
|---|---|
| Rationale | Users must be able to check what they write to their card. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 4.1, 3.4 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | #237 |
| Verification | test (build agent): checklist v0.4.0 test 10 (build agent): pass |

Acceptance criteria:

1. `redox-rpi-<version>.img.xz`, its `.sha256` and `.manifest.toml` are at
   `http://homenodellm:8080/sdcard/`.
2. A download passes `sha256sum -c` and `xz -t`.

### REQ-REL-5 Fits a 2 GB card

The release image shall fit a 2 GB microSD card.

| Field | Value |
|---|---|
| Rationale | The release notes promise it; small cards are common. |
| Priority | should |
| Kind | constraint |
| Level | release |
| PBS | 2.6.1, 4.1 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | #24 (image that grows on first boot, open) |
| Verification | analysis by Release & CI at the candidate, written into the release checklist next to the checksum; agreed with V&V 2026-11-08, not run yet |

Acceptance criteria:

1. The uncompressed image is at most 1 800 000 000 bytes (some "2 GB" cards
   hold only about 1.86 GB; 1054 MiB today).
