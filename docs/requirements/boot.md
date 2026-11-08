# Boot (`BOOT`)

Topic: [boot](../topics/boot.md). Trace: [trace.md](trace.md).

### REQ-BOOT-1 Boot from the SD card

The system shall boot from its SD card image to the login prompt, with the
root filesystem on the card and without a network.

| Field | Value |
|---|---|
| Rationale | The SD card image is what users download; most of them have no network boot server. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.1.3, 2.1.4, 2.3.3 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | #6 (sdhost driver), #243 (U-Boot boot menu on the card) |
| Verification | test, person: checklist v0.4.0 test 6, row 286 (queued); v0.3.0: visit 193 |

Acceptance criteria:

1. The release `.img.xz` written to a microSD card of 2 GB boots a Pi 3B+
   with HDMI and a USB keyboard and **no Ethernet cable** to `redox login:`
   with no key pressed (the boot menu's default entry).
2. After login, the root filesystem is RedoxFS on the card (`df` or the
   mount list shows it; no live disk in RAM).
3. A file written to the root filesystem is still there, unchanged, after
   `reboot`.

### REQ-BOOT-2 Network boot

With a netboot SD card (firmware and U-Boot only), the system shall load
its image over the network from the build host and boot to the login
prompt.

| Field | Value |
|---|---|
| Rationale | The test Pi and the nightly run use network boot, so every test runs without a card swap. |
| Priority | must |
| Kind | functional |
| Level | test image |
| PBS | 2.1.3, 3.4 |
| Status | proposed |
| Since | v0.1.0-alpha |
| Implemented by | #243 (boot menu), [netboot.md](../netboot.md) |
| Verification | test, automated: checklist v0.4.0 test 1, row 285 |

Acceptance criteria:

1. A Pi 3B+ with the netboot card loads the image staged with
   `scripts/netboot.sh` by TFTP or HTTP and reaches `redox login:` on the
   serial console.
2. The booted build ID (`/etc/redox-rpi-release`) is the staged one.

### REQ-BOOT-3 Clean boot log

The system shall boot without error lines from services that do not apply
to the Pi.

| Field | Value |
|---|---|
| Rationale | False errors hide real ones and confuse users and log triage. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.6.1 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | T18 (unsupported services, PR #77) |
| Verification | test, automated: checklist v0.4.0 test 3, row 285 |

Acceptance criteria:

1. The serial log of a boot has no line from `pcid` or `rtcd`.
2. `audiod` starts without an error (its uring note is allowed).
3. No `panicked` line from any service up to the login prompt.

### REQ-BOOT-4 Boot time

The system shall reach the login prompt soon after power-on or `reboot`.

| Field | Value |
|---|---|
| Rationale | A user waits for it on every start; the nightly and the tests reboot many times. |
| Priority | should |
| Kind | quality |
| Level | release |
| PBS | 2.1, 2.2.1, 2.3.2 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #47 (open, milestone v0.4.0) |
| Verification | test, automated: checklist v0.4.0 test 5 (reboot time noted), row 285 (about 20 s each); row 215 (20.4–20.9 s) |

Acceptance criteria:

1. From `reboot` to `redox login:` on the serial console: at most 30 s on
   the netboot image (proposed; measured about 20 s).
2. From the kernel's first line to `redox login:`: at most 10 s (proposed;
   7.4 s in row 67).

### REQ-BOOT-5 Damaged data partition

The bootloader shall boot when a RedoxFS partition on the card is damaged
or needs a write, instead of stopping.

| Field | Value |
|---|---|
| Rationale | A card pulled during a write must not make the Pi unbootable. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.1.4 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #232, PR #234 |
| Verification | test, automated: row 270 (dev build); no checklist test (**gap** for the candidate) |

Acceptance criteria:

1. With a data partition that the bootloader cannot open read-only, the Pi
   still boots to `redox login:` from the root partition, and the
   bootloader logs that it skipped the partition.
