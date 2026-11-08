# Release image (`REL`)

Topics: [ci](../topics/ci.md), [release-process.md](../release-process.md).
Trace: [trace.md](trace.md).

### REQ-REL-1 No remote shell in the release image

The release and candidate SD card images shall not contain `devshd` or any
other unencrypted remote login.

| Field | Value |
|---|---|
| Rationale | `devshd` gives a root login over plain TCP; a downloaded image must not open it. |
| Priority | must |
| Level | release |
| Status | proposed |
| Since | v0.2.0 |
| Issues | [release-process.md](../release-process.md#building), `config/raspi3bp-sdcard.toml` |
| Verified by | **gap** (no test checks it) |

Acceptance criteria:

1. The release image's file list has no `devshd` binary and no service that
   starts it.
2. On a booted release image, a TCP connection to port 2323 is refused.

### REQ-REL-2 The image names its release and build

The release image shall show its release name and build ID above the login
prompt and keep them in `/etc/redox-rpi-release` and `/.uuid`.

| Field | Value |
|---|---|
| Rationale | A user report or a test must name the exact build. |
| Priority | must |
| Level | release |
| Status | proposed |
| Since | v0.3.0 |
| Issues | #237 (`/.uuid` for SwiftFlash) |
| Verified by | checklist v0.4.0 test 1 (pi-test `version`), row 285 (banner on 7 boots); `/.uuid` checked at the rc1 build |

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
| Level | release |
| Status | proposed |
| Since | v0.3.0 |
| Issues | `config/upstream.lock`, drift check, #257 (rc1 built from a red commit) |
| Verified by | checklist v0.4.0 test 1 (pi-test `manifest`), row 285 |

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
| Level | release |
| Status | proposed |
| Since | v0.2.0 |
| Issues | #237 |
| Verified by | checklist v0.4.0 test 10 (build agent): pass |

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
| Level | release |
| Status | proposed |
| Since | v0.2.0 |
| Issues | #24 (image that grows on first boot, open) |
| Verified by | **gap** (no check of the uncompressed size) |

Acceptance criteria:

1. The uncompressed image is at most 1 900 MB (proposed; 1054 MiB today).
