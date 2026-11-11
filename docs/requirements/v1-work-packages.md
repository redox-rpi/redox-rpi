# v1.0 criteria without an implementing issue: proposed work packages

Status: **draft**, 2026-11-11. Owner: the Requirements agent. These are
proposals for the project lead (RFC 0003, Steps 3). Nothing here is decided:
RFC 0003 is a draft, and the work packages exist only when the project lead
adds them to the [WBS](../process/wbs.md).

Eight criteria of [RFC 0003](../rfcs/0003-v1-criteria.md) have no issue
that implements them (see [v1-criteria.md](v1-criteria.md)). Each one below
gets a scope, an owner area (a WBS package and its owner today), a first
step and what blocks it. "Owner area" names where the work fits best; the
project lead assigns it.

| Criterion | Prio | Proposed package | Owner area | Blocked by |
|---|---|---|---|---|
| REQ-BOARD-3 | should | LAN9514 driver and 3B/3A+ bring-up | WP 3.1 (V&V) with WP 2.7 (Network) | boards in the lab |
| REQ-STAB-7 | must | Power-loss safety of RedoxFS and the SD driver | WP 2.4 (Boot and console) | Shelly plug rules for tests |
| REQ-SEC-1 | must | First-boot password and safe SSH defaults | WP 2.1 (Boot and console) with WP 2.7 (Network) | REQ-NET-6, open question 4 |
| REQ-SEC-2 | must | Default service list and port audit | WP 2.7 (Network) | none |
| REQ-PKG-3 | must | System update with fallback | WP 1.4 (Architect, RFC), then WP 5.4 (Release and CI) | update RFC |
| REQ-DOC-1 | must | Beginner guide | WP 6.1 (project lead, topic owners) | REQ-SEC-1, REQ-PKG-3 for the last chapters |
| REQ-DOC-2 | should | Tested hardware list | WP 6.1 with WP 4.3 (V&V) | none |
| REQ-REL-8 | must | Support promise page | WP 5.3 (Release and CI), decision by the maintainer | open question 7 |

## REQ-BOARD-3 Other BCM2837 boards

- **Scope:** boot to login with console and keyboard on the Pi 3B and 3A+.
  Write a driver for the Pi 3B's LAN9514 (SMSC95xx USB Ethernet, same family
  as the LAN7515 of the 3B+). The 3A+ has no Ethernet; there the network
  means a USB adapter, as on the Zero 2 W.
- **Owner area:** WP 3.1 (V&V) for bring-up and the lab; the driver in WP 2.7
  (Network).
- **First step:** boot the current image on a 3B and a 3A+ (person test)
  and record what works. Then open a Task issue for the LAN9514 driver in
  the network driver's repository.
- **Done when:** REQ-BOARD-3 criteria 1 and 2 pass on a candidate.

## REQ-STAB-7 Power loss

- **Scope:** pulling the power must never leave the card unbootable or
  RedoxFS corrupt. Check RedoxFS's write order and sync, the SD driver's
  write completion, and what boot does with a dirty filesystem (check or
  repair at mount).
- **Owner area:** WP 2.4 (Storage, Boot and console agent). The test rig
  (Shelly plug) is V&V's (WP 4.2).
- **First step:** a short power-cut run of 10 cuts by hand or with the
  plug, within the maintainer's plug rules, to find out how bad it is today.
  Open a Task for each failure found.
- **Done when:** 50 cuts pass (REQ-STAB-7 criteria 1–3).

## REQ-SEC-1 No default password on the network

- **Scope:** a fresh image accepts no network login until a password is set
  or a key is added. Add a first-boot step (console prompt or a file on the
  boot partition), and make `sshd` refuse empty passwords and root password
  logins by default. devshd stays out of every downloadable image
  (REQ-REL-1, already met).
- **Owner area:** WP 2.1 (init and login, Boot and console) for the
  first-boot step; WP 2.7 (Network) for the `sshd` configuration.
- **First step:** a short design note in the boot topic page: where the
  first-boot password comes from (console prompt, `userconf` file as on
  Raspberry Pi OS, or both). Ask the maintainer about the console part
  (open question 4).
- **Done when:** REQ-SEC-1 criteria 1–3 pass on a fresh image.

## REQ-SEC-2 Only wanted services listen

- **Scope:** list the ports a fresh image may open, and turn off everything
  else by default.
- **Owner area:** WP 2.7 (Network).
- **First step:** a port scan of today's SD release image from the LAN
  (nmap, TCP and UDP), done by V&V. Write the allowed list into the network
  topic page; open a Task for each unwanted listener.
- **Done when:** the scan of a fresh candidate matches the documented list.

## REQ-PKG-3 System update without a new card

- **Scope:** one command updates to the next release over the network,
  keeps data and settings, survives a power cut and falls back on a failed
  first boot.
- **Owner area:** WP 1.4 (Architect) writes the update RFC first (A/B
  partitions, package-based or image-based; RFC 0003, Not in this RFC).
  The build and server side then belongs to WP 5.4 and WP 5.2 (Release and
  CI), the boot fallback to WP 2.1 (Boot and console).
- **First step:** the Architect drafts the update RFC. It depends on
  REQ-SEC-3 (signed updates) and on how the boot chain can choose between
  two systems.
- **Done when:** REQ-PKG-3 criteria 1–4 pass on update rows from the last 3
  releases.

## REQ-DOC-1 Beginner guide

- **Scope:** one guide from download to an updated, reachable Pi. Build it
  from [user-guide.md](../user-guide.md) and the getting-started page.
- **Owner area:** WP 6.1 (project lead, with each topic owner for their
  chapter).
- **First step:** an outline with the chapters of REQ-DOC-1 criterion 1,
  marking which exist today and which wait on other work (password at first
  boot, SSH, update).
- **Done when:** a newcomer finishes it without help (person test), and the
  commands in it run on the candidate.

## REQ-DOC-2 Hardware list

- **Scope:** one page with boards, USB Ethernet adapters, keyboards, mice
  and SD cards, each with the release it was last tested on.
- **Owner area:** WP 6.1; the data comes from V&V's test rows and the lab
  (WP 4.3).
- **First step:** collect what the pi-tests reports already name (boards,
  adapters, keyboards, cards) into a first table.
- **Done when:** the page exists and each row names a release.

## REQ-REL-8 Support promise

- **Scope:** a support page: how long 1.x gets fixes, that updates within
  1.x keep data and packages, how to report a bug and how fast a crash or
  security bug gets an answer.
- **Owner area:** WP 5.3 (Release and CI) writes the page; the promise
  itself is the maintainer's decision.
- **First step:** a draft page with the RFC's proposed numbers marked as
  proposed, once the maintainer has answered open question 7.
- **Done when:** the page is published and linked from the release notes.
