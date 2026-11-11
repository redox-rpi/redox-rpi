# v1.0 criteria: test-row drafts for V&V

Status: **draft**, 2026-11-11. Owner: the Requirements agent. These are
proposals for V&V (WBS 4.1, 4.2), not queued rows: V&V decides the rows,
their numbers and their order in the
[pi-tests queue](https://github.com/redox-rpi/pi-tests/blob/main/queue.md).

The criteria come from [RFC 0003](../rfcs/0003-v1-criteria.md) (draft). The
20 drafts below cover the criteria that have no test on a release candidate
yet ([v1-criteria.md](v1-criteria.md)). Three more criteria have rows on dev
builds only and need the same rows again on a candidate: REQ-BOARD-2 (rows
292, 286), REQ-STAB-6 (row 237) and REQ-STAB-8 (rows 264, 266, then 500
reboots and 50 cold starts). REQ-REL-6 is the trace itself.

Kinds:

- **auto**: an automated Pi row on the netboot image with devshd (as today).
- **auto, release image**: needs the image a user downloads, without devshd.
  Today automated rows never run on it, so these need a new way in (serial
  console script, or a netboot variant of the release image); see the
  questions at the end.
- **person**: needs the maintainer at the Pi.
- **inspection**: no Pi; a check of issues, files or logs.

Numbers marked (proposed) follow RFC 0003 open question 5.

| Draft | Criterion | Kind | Boards | Needs |
|---|---|---|---|---|
| T-BOARD-3 | REQ-BOARD-3 | person | 3B, 3A+ | the boards; LAN9514 driver for criterion 2 |
| T-BOARD-4 | REQ-BOARD-4 | person, later auto | Pi 400 DE | the board (ordered), WP 3.2 image |
| T-STAB-4 | REQ-STAB-4 | inspection | — | `crash` label or `severity:S1` (open question 8) |
| T-STAB-5 | REQ-STAB-5 | auto (soak) | 3B+, Zero 2 W | 7 days of Pi time per board; #259 |
| T-STAB-7 | REQ-STAB-7 | auto with plug | 3B+ | Shelly plug rules from the maintainer |
| T-FW-5 | REQ-FW-5 | auto with plug | 3B+ | plug power reading; #7 |
| T-NET-6 | REQ-NET-6 | auto | 3B+, Zero 2 W | `sshd` in the image; #45, #184, #190 |
| T-SEC-1 | REQ-SEC-1 | auto, release image | 3B+ | first-boot password (no issue yet) |
| T-SEC-2 | REQ-SEC-2 | auto, release image | 3B+ | nmap on the build host |
| T-SEC-3 | REQ-SEC-3 | auto + inspection | 3B+ | signed repository and images (#44, #237) |
| T-SEC-4 | REQ-SEC-4 | inspection | — | component version list per release |
| T-PKG-1 | REQ-PKG-1 | auto | 3B+ | project package repository (#44, #197) |
| T-PKG-2 | REQ-PKG-2 | auto | 3B+, Zero 2 W | base set (open question 6); #40 |
| T-PKG-3 | REQ-PKG-3 | auto with plug | 3B+ | update mechanism (no issue yet, RFC first) |
| T-DOC-1 | REQ-DOC-1 | person + auto | 3B+ | the guide; a newcomer |
| T-DOC-2 | REQ-DOC-2 | inspection | — | the hardware page |
| T-REL-7 | REQ-REL-7 | inspection | — | nightly reports in pi-tests (#258) |
| T-REL-8 | REQ-REL-8 | inspection | — | the support page |
| T-REL-9 | REQ-REL-9 | inspection | — | weekly drift report |
| T-CON-3 | REQ-CON-3 | person, or auto in QEMU | 3B+, Pi 400 | layout setting (#36, #305) |

## Drafts

### T-BOARD-3 Pi 3B and 3A+ boot (REQ-BOARD-3)

1. Flash the candidate SD image; boot a Pi 3B, then a 3A+.
2. Check `redox login:` on HDMI and serial; log in with a USB keyboard.
3. Pi 3B: `ifconfig` shows a DHCP address within 60 s.

Pass: steps 2 and 3 on the 3B, step 2 on the 3A+. Once per minor release.

### T-BOARD-4 Pi 400 DE (REQ-BOARD-4)

1. Boot the candidate image (or the Pi 400 image, open question 9) with no
   other keyboard plugged in; wait for `redox login:` on HDMI.
2. On the built-in keyboard with layout `de`: type `äöüÄÖÜß`, `@ € { } | \`
   (AltGr) at the login prompt and in a shell; switch terminals with the
   function keys.
3. Ethernet: DHCP address within 60 s; `date` shows NTP time.
4. Plug a USB mouse and a USB stick; both appear (stick only if USB storage
   is a 1.0 criterion).
5. `sd-stress.py` short run with 2 restarts: no loss.
6. If the desktop is in the image: the German layout types in Orbital.

Pass: all steps. Then the 1.0 automated set on the Pi 400 once it is in the
lab (EEPROM netboot, WBS 4.3).

### T-STAB-4 No open crash bugs (REQ-STAB-4)

1. At the candidate, query all project repositories for open crash bugs
   (`label:crash`, or `severity:S1` if that replaces it).
2. For each crash bug closed since the last release: a fix and a Pi row
   showing it gone, or a not-reproducible close after a test at least as
   long as the one that found it.

Pass: query empty and every closed one justified. Quality does it.

### T-STAB-5 Seven-day soak (REQ-STAB-5)

1. Governor `ondemand`. Loop for 168 h (proposed): program starts, network
   transfers both ways (host ↔ Pi), SD card writes with checksums.
2. Ping from the build host every minute; record free memory every hour.
3. At the end: read back all written data and compare checksums.

Pass: no panic, hang or crash dump on serial; no ping gap over 2 min; free
memory at the end within 10 % of hour 1; all data unchanged. One run per
board.

### T-STAB-7 Power cuts (REQ-STAB-7)

1. Start a writer that writes and syncs files with a timestamp log.
2. Cut the power with the Shelly plug at a random time (10–120 s into the
   write), wait 10 s, power on.
3. Wait for `redox login:`; check that root and data filesystems mount
   without error; check every file synced more than 2 s before the cut.
4. Repeat 50 times (proposed).

Pass: every cut boots and keeps the synced data. Runs only within the
maintainer's plug rules.

### T-FW-5 Power off (REQ-FW-5)

1. Write a file and note its checksum; run `poweroff` as root.
2. Watch serial for the shutdown messages; read the plug's power for 60 s.
3. Power cycle with the plug; boot; check the file.

Pass: power drops to the off level and stays there (no reboot); the file is
unchanged.

### T-NET-6 SSH server (REQ-NET-6)

From the build host:

1. Enable `sshd`; reboot; log in with a key, then with a password (if
   allowed).
2. Interactive shell: resize the terminal, Ctrl-C a running `sleep`, `exit`.
3. `scp` and `sftp` a 100 MiB file both ways; compare checksums.
4. 100 logins in a row; kill 10 connections from the host side and check
   that their sessions end on the Pi (#184).
5. Record the OpenSSH version for T-SEC-4.

Pass: all steps; 100/100 logins.

### T-SEC-1 No default password (REQ-SEC-1)

On a freshly flashed release image, before first-boot setup:

1. From the LAN, try SSH as `root` / `password` and `user` with an empty
   password; try devshd's port 2323.
2. Complete first-boot setup (set a password); try root with a password
   over SSH, and any account with an empty password.
3. Inspect the image: no `/usr/bin/devshd`.

Pass: every login in steps 1 and 2 is refused; step 3 finds no devshd.

### T-SEC-2 Port scan (REQ-SEC-2)

1. Boot a fresh release image; wait 2 min after login appears.
2. From the build host: `nmap -sT -p-` and `nmap -sU --top-ports 200`
   against the Pi.
3. Repeat after enabling SSH.

Pass: open ports match the documented list (none, or SSH once enabled,
plus mDNS if on by default).

### T-SEC-3 Signatures (REQ-SEC-3)

1. Serve a copy of a package with one changed byte from a test repository;
   `pkg install` it.
2. Offer a system update with a bad signature (needs REQ-PKG-3).
3. Inspection: the release files have a signature next to the checksum;
   verify it with the published key.

Pass: steps 1 and 2 fail with a signature error and change nothing; step 3
verifies.

### T-SEC-4 Security fixes (REQ-SEC-4)

1. List the versions of network-facing components (OpenSSH, TLS library,
   netstack) and others in the candidate.
2. Check each against public advisories.

Pass: no fix older than 14 days (network-facing) or 30 days (others)
(proposed) missing; the release notes name every fixed vulnerability.
Quality does it per release.

### T-PKG-1 Install and remove (REQ-PKG-1)

1. `pkg install` each package of the base set; run each once.
2. `reboot`; run each again.
3. 50 rounds of install and remove of one package; record free space
   before and after (#197).

Pass: all install and run, survive the reboot, free space back where it
was.

### T-PKG-2 Base set (REQ-PKG-2)

1. For each program of the base set (proposed: bash, nano, vim, curl, wget,
   git, tmux, ripgrep, python), one basic task: `bash` runs a script,
   `nano`/`vim` edit and save a file (scripted), `curl`/`wget` download over
   HTTPS, `git clone` a small repository, `rg` finds a string, `python`
   prints a result.
2. Check that an HTTP client is in the image without `pkg`.

Pass: every task succeeds; the HTTP client is present.

### T-PKG-3 System update (REQ-PKG-3)

1. Flash each of the last 3 releases; set a password, add an SSH key, a
   file in the home directory, an `/etc` setting, an installed package.
2. Run the update command; reboot; check the banner and everything from
   step 1.
3. Repeat with a plug power cut at a random point of the update (several
   runs, covering download, write and first boot).
4. Make the new version fail its first boot on purpose; check the fallback.

Pass: every update keeps the data; every cut boots the old or the new
version; the fallback works with no card swap.

### T-DOC-1 Beginner guide (REQ-DOC-1)

1. Person: someone new to redox-rpi follows the guide on a fresh card with
   no other help; note where they get stuck.
2. Auto: extract every command from the guide and run it on the candidate.

Pass: the newcomer reaches the end (problems fixed before 1.0); every
command runs as written.

### T-DOC-2 Hardware list (REQ-DOC-2)

Inspection: the page lists boards, USB Ethernet adapters, keyboards, mice
and SD cards, each with the release it was last tested on, and those
releases match pi-tests reports.

### T-REL-7 Fourteen green nights (REQ-REL-7)

Inspection of the nightly reports in pi-tests: 14 consecutive passing
nights (proposed), the last on the candidate's commit or one with the same
code. Needs a streak count in the nightly report (#258).

### T-REL-8 Support promise (REQ-REL-8)

Inspection: the support page exists and covers criteria 1–3 of REQ-REL-8;
the release notes link to it.

### T-REL-9 Close to upstream (REQ-REL-9)

Inspection of the drift report: the commits in `config/upstream.lock` are
at most 3 months (proposed) older than the candidate build; every patch
upstream could take was offered or says why not.

### T-CON-3 Keyboard layouts (REQ-CON-3)

1. Set the layout to `de`; reboot.
2. At the login prompt (before login) and in a shell: type
   `äöüÄÖÜß` and `@ € { } | \` (AltGr).
3. If the desktop is in the image: the same in an Orbital terminal.

Pass: all characters correct, layout kept after reboot. Person on the 3B+
and the Pi 400, or automated with QEMU's keyboard (sendkey) for steps 1–2.

## Questions for V&V

1. T-SEC-1 and T-SEC-2 need the release image without devshd. Is a serial
   console script enough, or should Release and CI build a netboot variant
   of the release image for automated rows?
2. T-STAB-5 takes the test Pi for 7 days per board. One soak per minor
   release, or one before 1.0 only?
3. T-STAB-7, T-FW-5 and T-PKG-3 switch the plug. Which plug rules apply to
   an automated test (how many cuts per hour, who is told)?
