# RFC 0003: What makes v1.0

| | |
|---|---|
| Status | Draft |
| Created | 2026-11-08 |
| Updated | 2026-11-09: Raspberry Pi 400 (maintainer, customers) |
| Author | Requirements agent, for the maintainer |
| Releases | v0.5.0 … v0.9.0, v0.10.0 …, until all criteria are met |

## Summary

redox-rpi gets version 1.0 when it meets a fixed list of criteria, not on a
date. Until then versions go on counting: v0.9.0 is followed by v0.10.0,
v0.11.0 and so on (weekly releases). This RFC proposes that list. Each
criterion is a requirement in the format of the
[requirements management plan](../process/rmp.md), with acceptance
criteria and a verification method, mapped to the roadmap themes and the
[PBS](../process/pbs.md). Open questions for the maintainer are at the end.

## Motivation

A 1.0 tells a user: you can put this on your Pi, leave it running, reach it
over the network safely, install software and update it without starting
over, and find out how from the documentation. Today none of those
promises holds fully: devshd is an unencrypted root login, there is no
update path, `pkg install` is an experiment, crash bugs are open
(#207, #92, #217, #151), and the beginner documentation is spread over
several pages. A list everyone can test keeps the 1.0 decision honest and
gives each work package a target.

## What 1.0 means

1. **It runs:** stable for days on the supported boards, with no known
   crash bug.
2. **It is safe on a network:** SSH instead of devshd, no default password
   reachable from the network, only the services the user turned on.
3. **It can be kept up to date:** packages and system updates over the
   network, signed, without a new SD card and without losing data.
4. **A beginner can use it:** one guide from flashing to updating.
5. **We stand behind it:** every must requirement verified on the 1.0
   build, and a written support promise.

1.0 is about a dependable base, not about features. Wi-Fi, the desktop,
audio, Bluetooth, smart home and support for the other Pi 4 and Pi 5
boards are not 1.0 criteria unless the maintainer adds them (see open
questions); they can come before or after 1.0 as features with their own
requirements. The one exception is the **Raspberry Pi 400**: the
maintainer added it on 2026-11-09 because customers want to use it
([REQ-BOARD-4](#req-board-4-raspberry-pi-400)). It brings the Pi 4 basics
into 1.0.

## Rules

- **Must** criteria block 1.0. **Should** criteria are wanted; a 1.0 that
  misses one lists it in the release notes.
- Every requirement of the release set ([docs/requirements/](../requirements/README.md),
  draft PR #270) that is `must` and `release` level still holds; this RFC
  adds the 1.0-specific ones.
- Verification follows the RMP: only tests on a reproducible build of the
  1.0 candidate count.
- IDs continue the numbering of the v0.4.0 set (#270). The new areas `PKG`,
  `SEC` and `DOC` are added to the RMP's area table when this RFC is
  accepted.

## Overview

| ID | Criterion | Prio | Theme | PBS | Today |
|---|---|---|---|---|---|
| [REQ-BOARD-2](#req-board-2-supported-boards-for-10) | Pi 3B+ and Zero 2 W fully supported | must | v0.9 | 1.1, 1.2 | 3B+ tested; Zero 2 W by hand only |
| [REQ-BOARD-3](#req-board-3-other-bcm2837-boards) | Pi 3B and 3A+ boot and work | should | v0.9 | 1.3 | untested |
| [REQ-BOARD-4](#req-board-4-raspberry-pi-400) | Raspberry Pi 400 (German keyboard) | must | Pi 400, set when the hardware arrives | 1.4 | no Pi 4 support at all; board ordered |
| [REQ-STAB-4](#req-stab-4-no-open-crash-bugs) | no open crash bugs | must | v0.9 | 2.2, 2.3, 2.4 | #207, #92, #217, #151, #205, #4, #129 open |
| [REQ-STAB-5](#req-stab-5-long-run) | 7 days running without a fault | must | v0.9 | 2 | longest run not measured (#259) |
| [REQ-STAB-6](#req-stab-6-chaos-test) | survives the chaos test | must | v0.9 | 2.2.1, 2.4 | experiment `chaos`; froze the Pi once (#207) |
| [REQ-STAB-7](#req-stab-7-power-loss) | survives power loss | must | v0.5 | 2.3.3, 2.4.3 | not tested |
| [REQ-STAB-8](#req-stab-8-reboot-cycles) | 500 reboots, network every time | must | v0.9 | 2.1, 2.3.4 | #217 open |
| [REQ-FW-5](#req-fw-5-power-off) | `poweroff` | should | v0.5 | 2.2.1, 2.3.8 | #7 open |
| [REQ-NET-6](#req-net-6-ssh-server) | SSH server for login and file copy | must | v0.7 | 2.5.6 | experiment `sshd` |
| [REQ-SEC-1](#req-sec-1-no-default-password-on-the-network) | no default password on the network | must | v0.7 | 2.4.6, 2.5.6 | root/password, devshd in test images |
| [REQ-SEC-2](#req-sec-2-only-wanted-services-listen) | only wanted services listen | must | v0.7 | 2.4.2, 2.6.1 | not tested |
| [REQ-SEC-3](#req-sec-3-signed-packages-and-images) | signed packages, updates and images | must | v0.5, v0.9 | 2.4.7, 4.1 | upstream pkg checks signatures; images only have checksums |
| [REQ-SEC-4](#req-sec-4-security-fixes) | security fixes in time | must | v0.9 | 2.5.6, 4.1 | no process (#190) |
| [REQ-PKG-1](#req-pkg-1-install-and-remove-packages) | `pkg install` and `remove` | must | v0.5 | 2.4.7 | experiments `pkg-repo`, `pkg-persist`; #197 |
| [REQ-PKG-2](#req-pkg-2-base-set-of-programs) | a base set of programs runs | must | v0.7 | 2.4.1, 2.4.7 | many prebuilt programs fail (#40) |
| [REQ-PKG-3](#req-pkg-3-system-update-without-a-new-card) | system update without a new SD card | must | v0.9 | 2.1, 2.6, 4.1 | none |
| [REQ-DOC-1](#req-doc-1-beginner-guide) | beginner guide, followed by a newcomer | must | v0.9 | 5.1 | parts in user-guide, getting-started |
| [REQ-DOC-2](#req-doc-2-hardware-list) | list of tested hardware | should | v0.9 | 5.1 | scattered over topic pages |
| [REQ-REL-6](#req-rel-6-all-must-requirements-verified) | every must requirement verified on the 1.0 build | must | v0.9 | 3, 4 | trace exists (#270) |
| [REQ-REL-7](#req-rel-7-nightly-green-before-the-release) | nightly green for 14 nights | must | v0.9 | 3.2, 3.3 | nightly runs; no streak count |
| [REQ-REL-8](#req-rel-8-support-promise) | written support and compatibility promise | must | v0.9 | 4.2 | none |
| [REQ-REL-9](#req-rel-9-close-to-upstream) | close to upstream Redox | should | v0.9 | 3.1 | pin and drift check exist |
| [REQ-CON-3](#req-con-3-keyboard-layouts) | German keyboard layout (Pi 400 DE) | must | v0.5, Pi 400 | 2.3.2 | US only (#36) |

## Criteria

### Boards

#### REQ-BOARD-2 Supported boards for 1.0

Every 1.0 requirement shall hold on the Raspberry Pi 3 Model B+ and the
Raspberry Pi Zero 2 W, except where a board lacks the hardware.

- Kind: constraint. Priority: must. PBS 1.1, 1.2. Theme: v0.9.
- Rationale: these are the boards we test on and the vision starts at the
  Zero 2 W (maintainer, 2026-11-08). The Pi 400 is added by
  [REQ-BOARD-4](#req-board-4-raspberry-pi-400); the other Pi 4 boards and the
  Pi 5 need the same or more new drivers (GIC, GENET, PCIe, RP1, see
  [boards](../topics/boards.md)) and come after 1.0.
- Acceptance criteria:
  1. The 1.0 test set runs on both boards on the 1.0 candidate.
  2. On the Zero 2 W, network means a USB Ethernet adapter (RTL8152/8153)
     unless Wi-Fi is made a 1.0 criterion (open question 2).
- Verification: test, automated on both boards. Needs a Zero 2 W in the
  test lab (WBS 4.3); until then test, person.

#### REQ-BOARD-3 Other BCM2837 boards

The 1.0 image shall boot and offer console, keyboard and network on the
Raspberry Pi 3 Model B and 3 Model A+.

- Kind: functional. Priority: should. PBS 1.3. Theme: v0.9.
- Rationale: same SoC; "almost every Pi from the Zero 2 W" (vision). The
  3B has a different USB Ethernet chip (LAN9514) and needs a driver.
- Acceptance criteria:
  1. Boot to login on HDMI and serial, USB keyboard works.
  2. Pi 3B: Ethernet gets an address by DHCP (needs a LAN9514 driver).
- Verification: test, person, once per minor release, or automated once
  the boards are in the lab.

#### REQ-BOARD-4 Raspberry Pi 400

The 1.0 release shall run on the Raspberry Pi 400 with the German keyboard
(Pi 400 DE): boot from the SD card, console on HDMI, its built-in keyboard,
Ethernet and USB.

- Kind: functional. Priority: must. PBS 1.4. Theme: its own (Pi 400).
- Source: maintainer, 2026-11-09, for customers who want to use the Pi 400
  ("ich hab Kunden die den verwenden wollen"). The maintainer has ordered
  one. **Its priority against the other work is set when the hardware
  arrives** (maintainer: "sobald der da ist müssen wir über Prioritäten
  reden").
- Rationale: the Pi 400 is a Pi 4 (BCM2711) in a keyboard. Almost nothing
  of the Pi 3 platform carries over below the kernel's CPU code: it needs
  the Pi 4 basics, which then also serve the Pi 4 and CM4 later.
- What it takes (PBS 1.4, new parts in PBS 2.1–2.3):
  - **Boot chain:** firmware from the board's EEPROM, `start4.elf`, the
    Pi 4 devicetree (`bcm2711-rpi-400.dtb`), a U-Boot or direct boot for the
    BCM2711.
  - **Interrupts:** GIC-400 (upstream Redox has a GICv2 driver; the
    BCM2836 local controller of the Pi 3 is gone).
  - **SD card:** the `emmc2` controller (an SDHCI behind its own clock and
    DMA limits).
  - **USB:** the VL805 xHCI controller behind PCIe: a PCIe host driver for
    the BCM2711, then Redox' existing `xhcid`. The built-in keyboard is a
    USB keyboard on this bus.
  - **Ethernet:** GENET (BCM54213PE PHY), a new driver.
  - **Display:** HDMI through the firmware framebuffer as a start (as
    `rpi-fbd` on the Pi 3); no `vc4d` port for 1.0.
  - **Firmware interface:** the mailbox as on the Pi 3 (`rpi-firmwared`
    with the BCM2711 base address), CPU clock up to 1800 MHz.
- Acceptance criteria:
  1. The 1.0 SD card image (or a Pi 400 image of the same release, open
     question 9) boots a Pi 400 to `redox login:` on HDMI with no other
     keyboard plugged in.
  2. The built-in keyboard types on the console and at the login with the
     German layout ([REQ-CON-3](#req-con-3-keyboard-layouts)), including
     umlauts, `ß`, AltGr characters (`@`, `€`, `{`, `|`) and the function
     keys used to switch terminals.
  3. Ethernet gets an address by DHCP within 60 s; NTP sets the clock.
  4. A USB mouse and a USB stick on the Pi 400's USB ports are detected
     (the stick only where USB storage is a 1.0 criterion).
  5. The SD card reads and writes without loss (the `sd-stress.py` short
     run with 2 restarts).
  6. All 1.0 requirements that do not need missing hardware hold on the
     Pi 400 as on the Pi 3B+ ([REQ-BOARD-2](#req-board-2-supported-boards-for-10)).
  7. If the desktop is in the 1.0 image (open question 3), the keyboard
     types the German layout there too.
- Verification: test, person first (the maintainer's board); test,
  automated once a Pi 400 is in the test lab (network boot from its
  EEPROM, WBS 4.3).

### Stability

#### REQ-STAB-4 No open crash bugs

At the 1.0 candidate, no crash bug shall be open.

- Kind: quality. Priority: must. PBS 2.2, 2.3, 2.4. Theme: v0.9.
- Definition: a **crash bug** is a Bug issue where the kernel panics, the
  system or a service hangs or stops for good, data on the card is lost or
  changed, or a program is killed without cause. Such issues carry the
  label `crash` (to be created).
- Acceptance criteria:
  1. The query `is:open label:crash` over the project's repositories returns
     nothing.
  2. Every crash bug closed since the last release has a fix and a Pi row
     that shows it no longer happens, or was closed as not reproducible
     after a test of at least the length that found it.
- Verification: inspection by Quality at the candidate.
- Today: #207 (chaos froze the Pi), #92 and #217 (network lost), #151
  (hang during SD write and download), #205 (signals never reach a looping
  process), #4 (stuck thread signals), #129 (devshd stops; goes away with
  devshd).

#### REQ-STAB-5 Long run

Each supported board shall run 7 days under a mixed load without a fault.

- Kind: quality. Priority: must. PBS 2. Theme: v0.9.
- Acceptance criteria (proposed numbers):
  1. 7 days (168 h) of a loop of program starts, network transfers in both
     directions and SD card writes, with the CPU governor on `ondemand`.
  2. No panic, hang or crash dump on the serial console; the Pi answers on
     the network every minute (no gap over 2 minutes).
  3. Free memory at the end is within 10 % of free memory after the first
     hour (no leak).
  4. Data written during the run reads back unchanged.
- Verification: test, automated soak row (V&V; Quality counts it, #259).

#### REQ-STAB-6 Chaos test

The system shall stay usable when services and programs fail.

- Kind: quality. Priority: must. PBS 2.2.1, 2.4. Theme: v0.9.
- Acceptance criteria:
  1. The chaos test suite (experiment `chaos`, #196) runs 10 times on the
     1.0 candidate: killed services, full memory, a full disk, host faults.
  2. After each run the Pi still answers on the network and the console,
     and a restarted or new program starts. No freeze (#207).
- Verification: test, automated chaos row.

#### REQ-STAB-7 Power loss

Pulling the power shall never leave the SD card unbootable or the
filesystem corrupt.

- Kind: quality. Priority: must. PBS 2.3.3, 2.4.3. Theme: v0.5.
- Rationale: a Pi has no power button; people pull the plug.
- Acceptance criteria:
  1. 50 power cuts at random times while files are written (power plug
     switched by the test, within the maintainer's plug rules).
  2. After each cut the Pi boots to login and the root and data
     filesystems mount without error.
  3. Data written and synced more than 2 s before the cut is there
     ([REQ-SD-2](../requirements/sdcard.md#req-sd-2-small-writes)).
- Verification: test, automated with the Shelly plug.

#### REQ-STAB-8 Reboot cycles

The system shall come back with network after every restart.

- Kind: quality. Priority: must. PBS 2.1, 2.3.4. Theme: v0.9.
- Acceptance criteria:
  1. 500 `reboot` cycles: each back at login with a DHCP address within
     60 s (#217 is the known failure).
  2. 50 cold starts (power off and on) the same.
- Verification: test, automated.

#### REQ-FW-5 Power off

`poweroff` shall stop the system safely.

- Kind: functional. Priority: should. PBS 2.2.1, 2.3.8. Theme: v0.5.
- Acceptance criteria:
  1. `poweroff` as root writes all data, stops all cores, and the Pi stays
     off (no reboot) until the power is cycled.
  2. Data written before `poweroff` is there at the next boot.
- Verification: test, automated (serial log and plug power reading).

### Remote access and security

#### REQ-NET-6 SSH server

The system shall offer an SSH server for remote login and file copy.

- Kind: functional. Priority: must. PBS 2.5.6. Theme: v0.7.
- Acceptance criteria:
  1. When enabled, `sshd` starts at boot and accepts a login with a key
     and, if the user allows it, a password.
  2. An interactive shell works (terminal size, Ctrl-C, exit).
  3. `scp` and `sftp` copy a 100 MiB file in both directions unchanged.
  4. 100 logins in a row succeed; a dropped connection frees its session
     (#184).
  5. The OpenSSH version has no known vulnerability with a fix older than
     30 days ([REQ-SEC-4](#req-sec-4-security-fixes), #190).
- Verification: test, automated from the build host.

#### REQ-SEC-1 No default password on the network

No account shall be reachable from the network with a default or empty
password.

- Kind: constraint. Priority: must. PBS 2.4.6, 2.5.6. Theme: v0.7.
- Rationale: today root has the password `password` and `user` none; fine
  on a console, not on a network.
- Acceptance criteria:
  1. A fresh image accepts no network login until the user has set a
     password or added a key at first boot.
  2. SSH refuses empty passwords, and root login with a password is off by
     default.
  3. devshd is in no image a user can download
     ([REQ-REL-1](../requirements/release.md#req-rel-1-no-remote-shell-in-the-release-image)).
- Verification: test, automated (SSH attempts with the defaults on a fresh
  image), inspection of the image.

#### REQ-SEC-2 Only wanted services listen

A fresh image shall listen only on the network ports of services the user
turned on.

- Kind: constraint. Priority: must. PBS 2.4.2, 2.6.1. Theme: v0.7.
- Acceptance criteria:
  1. A TCP and UDP port scan from the LAN of a fresh image finds only the
     ports listed in the documentation (none, or SSH once set up, plus mDNS
     if on by default).
- Verification: test, automated port scan.

#### REQ-SEC-3 Signed packages and images

Packages, system updates and release images shall be signed, and the
system shall refuse a package or update whose signature does not match.

- Kind: constraint. Priority: must. PBS 2.4.7, 4.1. Theme: v0.5 (packages),
  v0.9 (images).
- Rationale: upstream `pkg` already checks package signatures against the
  repository key; our own repository and our images need the same.
- Acceptance criteria:
  1. `pkg install` of a package changed after signing fails with a
     signature error and installs nothing.
  2. A system update ([REQ-PKG-3](#req-pkg-3-system-update-without-a-new-card))
     with a bad signature is refused before anything is written.
  3. Each release image has a signature next to its checksum that the
     published key verifies.
- Verification: test, automated (tampered package and update); inspection
  of the release files.

#### REQ-SEC-4 Security fixes

A known vulnerability in a component of the release image shall be fixed
in a release soon after its fix is public.

- Kind: constraint. Priority: must. PBS 2.5.6, 4.1. Theme: v0.9.
- Acceptance criteria (proposed numbers):
  1. Network-facing components (SSH, TLS, network stack) get a fix release
     within 14 days of a public fix; others within 30 days.
  2. The release notes name each fixed vulnerability.
- Verification: inspection by Quality per release (component versions
  against advisories).

### Packages and updates

#### REQ-PKG-1 Install and remove packages

`pkg install` and `pkg remove` shall work on the Pi from the project's
repository, and installed packages shall survive reboots and updates.

- Kind: functional. Priority: must. PBS 2.4.7. Theme: v0.5.
- Acceptance criteria:
  1. `pkg install <name>` of each package in the base set
     ([REQ-PKG-2](#req-pkg-2-base-set-of-programs)) succeeds, and the
     program runs.
  2. After `reboot` the packages are still installed and run.
  3. 50 rounds of install and remove leave the free space where it was
     (#197).
- Verification: test, automated.

#### REQ-PKG-2 Base set of programs

A base set of common programs shall install and run.

- Kind: functional. Priority: must. PBS 2.4.1, 2.4.7. Theme: v0.7.
- Acceptance criteria:
  1. Each program in the base set starts and does one basic task (for
     example `nano` edits and saves a file, `curl` downloads over HTTPS,
     `bash` runs a script). Proposed set: bash, nano, vim, curl, wget, git,
     tmux, ripgrep, python (open question 6).
  2. An HTTP client is in the image itself, so the first `pkg` problem can
     be worked around.
- Verification: test, automated.

#### REQ-PKG-3 System update without a new card

The system shall update itself to the next release over the network,
keeping the user's data and settings, and fall back to the old version if
the update fails.

- Kind: functional. Priority: must. PBS 2.1, 2.6, 4.1. Theme: v0.9.
- Acceptance criteria:
  1. From each of the last 3 releases, one command updates to the
     candidate; after reboot the banner shows the new version.
  2. Home directories, `/etc` settings, installed packages, SSH keys and
     passwords are kept.
  3. A power cut at any time during the update leaves a Pi that boots
     either the old or the new version (no card needed).
  4. A failed first boot of the new version falls back to the old one
     automatically.
- Verification: test, automated (update rows from older releases, with
  power cuts).

### Documentation

#### REQ-DOC-1 Beginner guide

One guide shall take a beginner from download to an updated, reachable
Pi.

- Kind: functional. Priority: must. PBS 5.1. Theme: v0.9.
- Acceptance criteria:
  1. The guide covers: download and check, flashing (Raspberry Pi Imager
     and one other tool), first boot, setting a password, network, SSH,
     installing a package, updating, getting help and reporting a bug.
  2. A person who has not used redox-rpi before follows it on a fresh card
     without other help and reaches the end; what they got stuck on is
     fixed before 1.0.
  3. Every command in the guide runs on the 1.0 candidate as written.
- Verification: demonstration (criterion 2, person test); test, automated
  (criterion 3, commands extracted from the guide).

#### REQ-DOC-2 Hardware list

The documentation shall list the hardware tested with the release.

- Kind: functional. Priority: should. PBS 5.1. Theme: v0.9.
- Acceptance criteria:
  1. A page lists boards, USB Ethernet adapters, keyboards, mice and SD
     cards with the release they were last tested on.
- Verification: inspection.

### Release and support

#### REQ-REL-6 All must requirements verified

The 1.0 release shall have every `must` requirement verified on its
candidate.

- Kind: constraint. Priority: must. PBS 3, 4. Theme: v0.9.
- Acceptance criteria:
  1. The trace for the candidate shows no `must` requirement as gap, fails,
     queued or earlier.
  2. Each `should` requirement that is not verified is in the release notes.
- Verification: inspection of the trace (Requirements agent, Quality).

#### REQ-REL-7 Nightly green before the release

`main` shall pass the nightly Pi run for 14 nights in a row before 1.0 is
built.

- Kind: quality. Priority: must. PBS 3.2, 3.3. Theme: v0.9.
- Acceptance criteria:
  1. The nightly run (with the checks of the 1.0 set that run without a
     person) passed on 14 consecutive nights, the last one on the commit of
     the candidate or a commit with the same code.
- Verification: inspection of the nightly log (Release and CI).

#### REQ-REL-8 Support promise

The project shall publish what 1.x users can rely on.

- Kind: constraint. Priority: must. PBS 4.2. Theme: v0.9.
- Acceptance criteria (proposed):
  1. A support page says how long 1.x gets fixes (proposed: the newest 1.x
     release gets crash and security fixes; each 1.x release is supported
     until 3 months after the next one).
  2. Updates within 1.x keep data, settings and installed packages
     ([REQ-PKG-3](#req-pkg-3-system-update-without-a-new-card)); a change
     that breaks this needs 2.0.
  3. The page says how to report a bug and how fast a crash or security
     bug gets an answer (proposed: 7 days).
- Verification: inspection.

#### REQ-REL-9 Close to upstream

The 1.0 release shall be based on a recent upstream Redox.

- Kind: constraint. Priority: should. PBS 3.1. Theme: v0.9.
- Acceptance criteria (proposed):
  1. The upstream commits in `config/upstream.lock` are at most 3 months
     older than the 1.0 build.
  2. Every patch that upstream could take has been offered upstream (by the
     maintainer) or says why not.
- Verification: inspection (drift report).

### Console

#### REQ-CON-3 Keyboard layouts

The console, the login and (if in the image) the desktop shall support
keyboard layouts other than US, German first.

- Kind: functional. Priority: must (was should; the Pi 400 DE has a
  German keyboard built in, [REQ-BOARD-4](#req-board-4-raspberry-pi-400)).
  PBS 2.3.2. Theme: v0.5, Pi 400. Issue #36.
- Acceptance criteria:
  1. A setting chooses the layout; with `de`, umlauts, `ß` and the AltGr
     characters type correctly on the console and survive a reboot.
  2. The layout applies at the login prompt, before anyone is logged in.
  3. If the desktop is in the image, Orbital uses the same layout.
- Verification: test, person (or automated with QEMU's keyboard).

## Roadmap mapping

| Theme | Criteria it delivers |
|---|---|
| v0.5 Devices | REQ-FW-5, REQ-PKG-1, REQ-SEC-3 (packages), REQ-STAB-7, REQ-CON-3 |
| v0.6 Network | (Wi-Fi only if open question 2 makes it a criterion) |
| v0.7 Remote access | REQ-NET-6, REQ-SEC-1, REQ-SEC-2, REQ-PKG-2 |
| v0.8 Smart home | none (features, not 1.0 criteria) |
| Pi 400 (priority set when the board arrives) | REQ-BOARD-4, REQ-CON-3 (German layout on its keyboard) |
| v0.9 Towards 1.0 | REQ-BOARD-2, REQ-BOARD-3, REQ-STAB-4, -5, -6, -8, REQ-SEC-3 (images), REQ-SEC-4, REQ-PKG-3, REQ-DOC-1, -2, REQ-REL-6 to -9 |
| v0.10 … | whatever is still open |

## Steps

1. The maintainer decides this RFC and the open questions.
2. The Requirements agent adds the accepted criteria to
   `docs/requirements/` (or as Feature issues, after RFC 0002) and the
   areas `PKG`, `SEC`, `DOC` to the RMP.
3. The project lead creates a milestone "v1.0 criteria" and work packages
   for what has no owner (update path, signing, documentation).
4. V&V plans the long-run, chaos, power-loss, reboot and update rows;
   Quality adds the 1.0 progress (criteria met / total) to the KPI
   dashboard.

## Not in this RFC

- Features for 1.0 beyond the base (desktop, audio, Wi-Fi, Bluetooth, smart
  home): see open questions 2 and 3.
- How updates work technically (A/B partitions, package-based, or image
  based): a separate RFC by the Architect, once REQ-PKG-3 is accepted.
- Pi 4 (other than the Pi 400), CM4 and Pi 5 support: after 1.0, with
  their own requirements. Much of the Pi 400 work carries over to them.

## Open questions for the maintainer

1. **Boards:** Pi 3B+, Zero 2 W and Pi 400 must, Pi 3B and 3A+ should,
   other Pi 4 boards and Pi 5 after 1.0. Agreed? Should the Pi 4 Model B
   come with the Pi 400, as it needs almost the same work?
2. **Wi-Fi:** the Zero 2 W without a HAT has no network without Wi-Fi.
   Is Wi-Fi a 1.0 must, a should, or after 1.0?
3. **Desktop and audio:** today beta. Stay beta in 1.0 (not criteria), or
   must they leave beta for 1.0?
4. **Console passwords:** REQ-SEC-1 only covers the network. Should the
   first boot also force a new password on the console?
5. **Numbers:** 7 days long run, 500 reboots, 50 power cuts, 14 green
   nights, 14/30 days for security fixes, 3 months support, upstream at
   most 3 months old. Keep, raise or lower?
6. **Base set of programs** for REQ-PKG-2: which ones?
7. **Support:** with weekly releases, is "the newest 1.x is supported" the
   promise, or should there be longer-lived 1.x lines?
8. **Crash label:** a new label `crash` for REQ-STAB-4, or an issue field?
9. **Pi 400 image:** one image for all boards (firmware picks the right
   devicetree and kernel), or a separate Pi 400 image per release?
10. **Pi 400 priority:** set together when the board arrives; until then
    the Pi 400 criteria have no milestone.
