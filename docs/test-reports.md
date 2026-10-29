# Test reports

Results of [`scripts/pi-test.py`](../scripts/pi-test.py) on a real
Raspberry Pi 3B+, newest at the bottom. Each entry names the build ID; the
matching manifest says exactly what the image contained
([versioning](versioning.md)).

Checks: `version` (build ID readable), `manifest` (build ID and `libc.so`
hash match the build's manifest), `ps` (output piped), `ps-tty` (output
straight to the terminal), `sdcard` (write and read back a file
on the SD card's data partition), `starts` (dynamically linked `ls` and
`pwd` started repeatedly, every failed start counted).

Run on the build host, with the Pi netbooted from it:

```bash
scripts/pi-test.py 192.168.178.148 --manifest build/manifests/netboot-latest.toml \
    --report docs/test-reports.md
```

## 2026-10-27: `v0.1.0-alpha-14-gf39d20b+ldso-diagnostics`

Built 2026-10-27T06:56:23Z, `libc.so` `5a6866e092653007…`, real Pi 3B+ over netboot. Equivalent of the 17:49 sig-x1x2 image.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.1.0-alpha-14-gf39d20b+ldso-diagnostics |
| manifest | pass | build ID and libc.so hash match netboot-v0.1.0-alpha-14-gf39d20b+ldso-diagnostics.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | **fail** | 0 lines straight to the terminal, rc=1 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## 2026-10-27: `v0.1.0-alpha-14-gf39d20b`

Built 2026-10-27T06:59:49Z, `libc.so` `24ec8b13889ba284…`, real Pi 3B+ over netboot. Default image: relibc patch 0005 instead of the diagnostics.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.1.0-alpha-14-gf39d20b |
| manifest | pass | build ID and libc.so hash match netboot-v0.1.0-alpha-14-gf39d20b.toml |
| ps | pass | 105 processes (piped) |
| ps-tty | **fail** | 0 lines straight to the terminal, rc=1 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | **fail** | 36 of 300 failed; messages: "[ld.so]: enable debug output with `LD_DEBUG=all` for more information | [ld.so]: failed to link '/usr/bin/pwd': Malformed |

Findings from these two runs:

- `ldso-diagnostics` (reads every byte of fresh memory in `ld.so`) has no
  start failures on the real Pi. Relibc patch `0005` (reads one byte per
  page) has 12 %, with a new message: `ld.so` fails to link `/usr/bin/pwd`
  because the file contents it read are malformed. In QEMU, `0005` had about
  2 %. So `0005` is not enough on real hardware; the default image needs the
  full read until the cause is found ([task T2](tasks.md)).
- `ps` straight to the terminal fails with exit code 1 and no output in
  both images; piped it works. `reboot` showed the same: exit code 1, no
  message, several attempts needed ([task T5](tasks.md)).
- Every devshd session leaves a `login` process behind: 31 after about
  30 sessions ([task T11](tasks.md)).

## 2026-10-27: signal entry register test (`sigregs`), QEMU

[`sigregs`](../tools/sigregs) loads patterns into x1-x29 and v0-v31,
takes signals and checks that nothing changed
(experiment [`sigregs-test`](../patches/experiments/sigregs-test)). QEMU
raspi3b, netboot image, `sigregs <mode> -n <signals>`.

| Build | process (`kill`) | thread (`pthread_kill`) |
|---|---|---|
| `v0.1.0-alpha-39-g7cd36d6+sigregs-test` (current relibc patches) | pass, 20 signals | **fail**: 1 of 20 signals arrived, then SIGUSR1 stayed blocked |
| `…-41-gaf3c36a+sigregs-test+sigentry-by-name+sigentry-thread-signals` | pass, 2000 signals (and 9 × 100) | pass, 2000 signals |
| `…-41-gaf3c36a+sigregs-test+sigentry-thread-signals+sigentry-x1x2-revert` (negative control) | **fail**: `x1 changed` after 3, 4, 33 signals; 1 of 4 runs passed 100 | **fail**: `x1 changed` after 2 signals |

Findings:

- The test finds the x1/x2 swap that relibc patch `0004` fixes. Without
  `0004` it fails within a few signals; with it, and with the rewrite
  `sigentry-by-name`, every register is kept.
- A signal sent to one thread (`pthread_kill`) blocks every signal of that
  thread afterwards: the signal entry stores the signal number as the new
  signal word (label `3`). Experiment `sigentry-thread-signals` fixes it
  ([task T4](tasks.md)).
- `sigregs` itself often dies before its first output: in QEMU in about
  half of the starts, on the real Pi in every start. It stops in relibc's
  program start at `.expect("no filetable fd present")`, not in the signal
  code; first taken for a `fork()` crash ([task T12](tasks.md)). `sigregs`
  is now linked statically, against the `libc.a` of our relibc recipe.

## 2026-10-27: `v0.1.0-alpha-44-g8dc4889+ldso-diagnostics+sigregs-test+sigentry-by-name+sigentry-thread-signals`, real Pi

Real Pi 3B+ over netboot, `sigregs` linked statically against the `libc.a`
of this build's relibc. `sigregs -n 1000` (both modes) 10 times:

| Mode | Result |
|---|---|
| process (`kill`) | pass in 10 of 10 runs, 1000 signals each, all registers kept |
| thread (`pthread_kill`) | pass in 10 of 10 runs, 1000 signals each, all registers kept |

Before the static link, the same build had `sigregs` fail at start in 10
of 10 runs on the Pi ([task T12](tasks.md)). The static binary first
linked the toolchain's `libc.a` (unpatched upstream relibc) by mistake: in
QEMU it then failed with `x1 changed` and with the blocked thread signal,
which confirms both bugs in upstream relibc.

## 2026-10-27: `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver`

Manual test of the sdhost driver on the real Pi 3B+ over netboot, started
by hand as `bcm2835-sdhostd disk.sdhost` next to `bcm2835-sdhcid`.

| Check | Result | Detail |
|---|---|---|
| card init | pass | SDHC/SDXC card, 15193 MiB, 4-bit bus, up to 25 MHz |
| partitions | pass | `/scheme/disk.sdhost` lists `0`, `0p0`, `0p1` |
| MBR | pass | partition entries match the SD image the card was made from; disk signature differs (new per `make-sdcard.sh` run) |
| FAT boot sector | pass | `REDOXBOOT`, `FAT32` |
| read speed | pass | 8 MiB in 1.85 s, 4.5 MB/s |
| write | pass | tested with sdhost at boot, see the `sdhost-boot` report |

## 2026-10-27: `v0.1.0-alpha-51-g3d7e351+ldso-diagnostics+rpi-firmware`

Built 2026-10-27T12:58:25Z, `libc.so` `5a6866e092653007…`, real Pi 3B+ over netboot. Experiment rpi-firmware on top of ldso-diagnostics.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.1.0-alpha-51-g3d7e351+ldso-diagnostics+rpi-firmware |
| manifest | pass | build ID and libc.so hash match netboot-v0.1.0-alpha-51-g3d7e351+ldso-diagnostics+rpi-firmware.toml |
| ps | pass | 43 processes (piped) |
| ps-tty | **fail** | 0 lines straight to the terminal, rc=1 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Firmware properties read on the Pi (`/scheme/rpi-firmware`):

| File | Value |
|---|---|
| `board` | revision `0x00a020d3`, Raspberry Pi 3 Model B+, 1 GiB, serial `00000000ed08e0aa`, firmware `0x6aa809d0`, ARM memory 998 MiB, VideoCore 16 MiB, maximum temperature 85 °C |
| `temperature` | 58533 (58.5 °C) |
| `clocks` | ARM 600 MHz (max 1400), core 250 MHz (max 400), EMMC 200 MHz, UART 48 MHz, SDRAM 400 MHz (max 450), V3D/H264/ISP 250 MHz (max 300) |
| `throttled` | `0x00000000`: no undervoltage, no throttling |

In the first run right after the boot, writing to `/scheme/data` failed
once with an I/O error and devshd dropped the connection; the run above,
a few minutes later, passed. Not seen again.

## 2026-10-27: `v0.1.0-alpha-52-ga4ba2e3+ldso-diagnostics+sdhost-driver+sdhost-boot`

Real Pi 3B+ over netboot, serial console attached. `bcm2835-sdhostd`
starts at boot instead of `bcm2835-sdhcid` and serves `disk.mmc`.

| Check | Result | Detail |
|---|---|---|
| version | pass | build ID as above |
| boot | pass | 3 boots; card SDHC 15193 MiB, 4-bit bus; `/scheme/data` mounted through sdhost |
| `libc.so` | pass | `5a6866e092653007…`, as in the rpi-firmware build |
| write | pass | `dd if=/scheme/rand of=/scheme/data/sdhost-4m.bin bs=4096 count=1024`: 4 MiB in 33 s, 127 kB/s |
| write across drivers | pass | SHA-256 `2fae7622263c410a294dcb08bc5248f606c41c469eb2bf13b56a5ddbc7831522` with sdhost; the same after booting `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver` (`bcm2835-sdhcid` at boot) |
| starts | pass | 0 of 50 failed |

Not caused by sdhost, seen during this test:

- `scripts/pi-test.py` got no output after the login, twice; the same
  commands typed more slowly over devshd worked.
- The network stopped twice: devshd refused connections, later the Pi did
  not answer at all, while the serial console kept working. Once with the
  sdhost image, once with the `bcm2835-sdhcid` image.
- `reboot` crashed at start four times in a row (`brk`, see T12);
  `echo reset > /scheme/sys/kstop` restarted the Pi.

## 2026-10-27: `v0.1.0-alpha-61-g90ef0ee+ldso-diagnostics+sdhost-driver+sdhost-boot+sdhost-stats`

Real Pi 3B+ over netboot, serial console. sdhost in high speed mode, with
transfer statistics.

| Check | Result | Detail |
|---|---|---|
| card init | pass | `SDHC/SDXC card, 15193 MiB, 4-bit bus, high speed, up to 50 MHz` (31 MHz at the 250 MHz core clock) |
| write 1 MiB calls, fsync | pass | 4 MiB in 0.62 s, 6.7 MB/s (before: 4.4 MB/s) |
| write 64 KiB calls, fsync | pass | 4 MiB in 1.3 s, 3.2 MB/s (before: 2.0 MB/s) |
| write 4 KiB calls, fsync | pass | 1 MiB in 5.8 s, 181 kB/s (before: 130 kB/s); 37 MB sent to the card, see T16 |
| write, restart, read back | pass | 8 MiB, SHA-256 `fc2479e005e51b28074dfde9e3da3bda18deb479fc67019f345467169b78d20c` before and after the restart |
| read from the card | pass | 8 MiB in 0.65 s, 13 MB/s |

## 2026-10-27: release `v0.1.0-alpha` SD card image on a Pi Zero 2 W

The unchanged release image `redox-rpi-v0.1.0-alpha-pi3bplus.img.xz`
([release notes](releases/v0.1.0-alpha.md)), flashed and booted on a real
Raspberry Pi Zero 2 W by a person, without a serial console. Not a
`pi-test.py` run.

| Check | Result | Detail |
|---|---|---|
| boot | pass | boots the Pi 3B+ image unchanged |
| USB keyboard | pass | works when plugged in before boot |
| USB keyboard hotplug | **fail** | a keyboard plugged in after boot is not detected |
| network | – | none, expected: the Zero has no Ethernet of its own, and Redox has no driver for the HAT's RTL8152 yet |

## 2026-10-27: `v0.1.0-alpha-63-g23a4826+ldso-diagnostics`, network boot on a Pi Zero 2 W

The Zero 2 W image (`build-image.sh --zero2w`, 128 MiB) booted over the
network from a 32 MiB U-Boot card (`make-sdcard.sh --dir redox-zero2w`),
through the RTL8152 on the Ethernet/USB HAT. Watched on a monitor by a
person, without a serial console. Not a `pi-test.py` run.

| Check | Result | Detail |
|---|---|---|
| U-Boot DHCP, `netboot.scr` over TFTP | pass | |
| image over HTTP | **fail** | `Rx: failed to receive: -5` from U-Boot's r8152 driver |
| image over TFTP, 1468-byte blocks | **fail** | same error after a few dozen KiB |
| image over TFTP, 512-byte blocks | pass | the whole 42 MB arrive; slow, many minutes, not timed |
| boot to login | pass | logged in as root |
| network in Redox | – | none, expected: no RTL8152 driver yet |

## 2026-10-27: `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver`, speed of `bcm2835-sdhcid`

Real Pi 3B+ over netboot, the same card and the same `dd` runs as for
sdhost in high speed mode, to compare the two drivers.

| Check | Result | Detail |
|---|---|---|
| write 1 MiB calls, fsync | pass | 4 MiB in 1.57 s, 2.7 MB/s (sdhost: 6.7 MB/s) |
| write 64 KiB calls, fsync | pass | 4 MiB in 2.53 s, 1.7 MB/s (sdhost: 3.2 MB/s) |
| write 4 KiB calls, fsync | pass | 1 MiB in 9.2 s, 114 kB/s (sdhost: 181 kB/s) |
| write 8 MiB, restart, read back | pass | SHA-256 `f611ca802560f2ced9628a84f134df3757367235848026cae11d5c8ef3b5b12a` before and after the restart |
| read from the card | pass | 8 MiB in 1.85 s, 4.5 MB/s (sdhost: 13 MB/s) |

`uptime` printed nothing twice in a row; `date` (the clock starts at
1970-01-01 on every boot) gave the uptime instead.

## 2026-10-28: `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver`, network boot without SD card

Real Pi 3B+ (`b8:27:eb:08:e0:aa`) without SD card, test #10 of the
[test plan](test-plan.md). Branch `firmware-netboot` (`7fbb950`): the TFTP
root holds the Pi firmware and U-Boot built with
`scripts/build-uboot.sh rpi-net 192.168.178.120`
(`scripts/netboot-firmware.sh`), and dnsmasq runs as a proxy DHCP server
next to the FritzBox (`config/dnsmasq-pxe-proxy.conf`). The build host is on
Wi-Fi. Serial console and the dnsmasq log were recorded; a person pulled
the card and switched the Pi on. Not a `pi-test.py` run.

| Check | Result | Detail |
|---|---|---|
| boot ROM PXE request reaches the proxy | pass | `PXEClient:Arch:00000:UNDI:002001` from `b8:27:eb:08:e0:aa`, answered with "Raspberry Pi Boot"; the address came from the FritzBox |
| firmware and U-Boot over TFTP | pass | from the TFTP root; no `<serial>/` directory and no card with `bootcode.bin` needed |
| U-Boot without card | pass | `Bad device specification mmc 0` (no environment), then its built-in boot command loads `redox/netboot.scr` |
| image over HTTP, boot to login | pass | "image loaded over HTTP", `redox login:` |
| data partition | – | `RedoxFS: IO ERROR`, expected: there is no card |
| netboot card back in | pass | no PXE request; U-Boot runs `boot.scr` from the card (`mmc@7e300000.bootdev.part_1`), boots to `redox login:`, devshd answers on port 2323 |

TFTP requests are not logged (`tftpd-hpa` without `--verbose`), so the exact
files the firmware fetched are not recorded.

## 2026-10-28: `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver`, network boot with a card without `bootcode.bin`

Real Pi 3B+, the same setup as the network boot without card above. In it
an old Raspberry Pi OS card (now labelled `redox-dev`) whose `bootcode.bin`
was renamed to `bootcode.bin.off`. Switched on by a person; boot log from
the serial console and the dnsmasq log.

| Check | Result | Detail |
|---|---|---|
| boot ROM skips the card and network-boots | pass | PXE request from `b8:27:eb:08:e0:aa` at 06:52 |
| boot to login | pass | `redox login:` |
| data partition | – | `redoxfs` cannot mount `/scheme/disk.mmc/0p1`, expected: partition 2 of that card is a Linux filesystem, not RedoxFS |

So a card for SD tests needs no boot files: the Pi boots from the network
whatever is on the card.

## 2026-10-27: `v0.1.0-alpha-76-g4178ec5+xbfca`

Built 2026-10-27T19:39:18Z, `libc.so` `5a6866e092653007…`, real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `rtl8152`, `sdhost-driver`, `timesync`. Release candidate v0.2.0-alpha (rc1), test #9.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.1.0-alpha-76-g4178ec5+xbfca |
| manifest | pass | build ID and libc.so hash match netboot-v0.1.0-alpha-76-g4178ec5+xbfca.toml |
| ps | pass | 44 processes (piped) |
| ps-tty | **fail** | 0 lines straight to the terminal, rc=1 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Manual checks on the same boot:

| Check | Result | Detail |
|---|---|---|
| rpi-firmware | pass | `board`: Pi 3 Model B+ (revision `0x00a020d3`), 1 GiB; `temperature` 47774 (47.8 °C); `throttled` `0x00000000`; `clocks`: ARM 600 MHz (max 1400), core 250 MHz (max 400), EMMC 200 MHz |
| timesync | pass | `timesync -n`: 20:08:02 UTC from pool.ntp.org, `date` the same as the build host; daemon running, `/scheme/data/timesync.last` written. Reboot without network not done on the Pi (passed in QEMU) |
| sdhost by hand | **fail** | `bcm2835-sdhostd disk.sdhost &` starts and lists `0p0 0p1`, but `dd` reads nothing, later `dd` from `disk.sdhost` hangs with no output; devshd logins hang (root `redoxfs` busy). Reset over the serial console. `sdhost-driver` is left out of the release (rc2) |

## 2026-10-27: `v0.1.0-alpha-76-g4178ec5+x83a0`

Built 2026-10-27T20:24:20Z, `libc.so` `5a6866e092653007…`, real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `rtl8152`, `timesync`. Release candidate v0.2.0-alpha (rc2, without sdhost-driver), test #9.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.1.0-alpha-76-g4178ec5+x83a0 |
| manifest | pass | build ID and libc.so hash match netboot-v0.1.0-alpha-76-g4178ec5+x83a0.toml |
| ps | pass | 50 processes (piped) |
| ps-tty | **fail** | 0 lines straight to the terminal, rc=1 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Manual checks on the same boot: `rpi-firmware` pass (Pi 3 Model B+, 48.3 °C,
`throttled` `0x00000000`, ARM 600/1400 MHz, core 250/400 MHz); timesync
pass (`date` 20:35:51 UTC, the same as the build host). rc2 is rc1 without
`sdhost-driver`.

USB keyboard: pass, login on HDMI works (keyboard plugged in before boot).

Pi Zero 2 W, rc2 SD card image, with the RTL8152 Ethernet hub and a LAN
cable, checked by the user: boots to the login, the network comes up, `date`
shows the current time (Wed Oct 28 06:29:22 UTC 2026). `timesync -n` printed
`2026-10-28 06:32:26 UTC from pool.ntp.org (clock not set)`, so the Zero
reaches the internet through the RTL8152. The first three `timesync -n` runs
printed nothing: start failures as in T12 (`timesync` is linked dynamically,
with `/lib/ld.so.1`). The address
was not read: the path was typed as `/schema/…` instead of `/scheme/…`.

## 2026-10-28: `v0.2.0-5-gd679bfb+xed14` (v0.3.0-rc1, SD card image)

Real Pi 3B+, SD card image `redox-rpi-v0.3.0-rc1.img.xz` written by the
user, serial console. Experiments `ldso-diagnostics`, `timesync`, `rtl8152`,
`sdhost-driver`, `sdhost-boot`: sdhost is the only SD driver and the root
filesystem is on the card. Test #12.

| Check | Result | Detail |
|---|---|---|
| version | pass | `v0.2.0-5-gd679bfb+xed14`, image `sdcard` |
| boot | pass | 2 boots to the login; one Enter at the bootloader's resolution screen |
| SD driver | pass | `bcm2835-sdhostd`: SDHC/SDXC card, 30475 MiB, 4-bit bus, high speed (a different card than before); no `bcm2835-sdhcid` process |
| root filesystem | pass | mounted from `/scheme/disk.mmc/0p1` |
| write to the root | pass | 8 MiB from `/scheme/rand` with `conv=fsync` to `/home/root`, 3.7 MB/s |
| after a restart | pass | SHA-256 `4182ec12a9acfb56f8050f5ac1e4611c4538ba30dacb9ec186580758af348ca1` before and after |
| starts | pass | 0 of 100 failed |
| network, clock | pass | LAN7800 link up, timesync: build time, then saved time, then pool.ntp.org |

RedoxFS logs "No such file or directory" for `disk.mmc/0` and `0p0` while it
looks for the root filesystem; those are not RedoxFS, it mounts `0p1` next.

## 2026-10-28: `v0.2.0-9-g43f697e+xe0ce`

Built 2026-10-28T07:49:07Z, `libc.so` `30bd563f098b1253…`, real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `sigdelivery-x0`, `timesync`. Kernel experiment sigdelivery-x0 (T12 fix); the SD card in the Pi is the v0.3.0-rc1 card, without a data partition for the netboot image.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.2.0-9-g43f697e+xe0ce |
| manifest | pass | build ID and libc.so hash match netboot-v0.2.0-9-g43f697e+xe0ce.toml |
| ps | pass | 43 processes (piped) |
| ps-tty | pass | 43 lines straight to the terminal, rc=0 |
| sdcard | **fail** | got ion: pipeline execution error: failed to redirect stdout to file '/scheme/data/pi-test.txt': No such device (os error 19) |
| starts | pass | 0 of 300 failed |

## 2026-10-28: `v0.2.0-9-g43f697e+x0f50`

Built 2026-10-28T07:45:10Z, `libc.so` `b5b10aaf4262f509…`, real Pi 3B+ over netboot. Experiments: `sigdelivery-x0`, `timesync`. Kernel experiment sigdelivery-x0 without ldso-diagnostics (relibc 0005 only), for the T1 start rate; the SD card is the netboot card.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.2.0-9-g43f697e+x0f50 |
| manifest | pass | build ID and libc.so hash match netboot-v0.2.0-9-g43f697e+x0f50.toml |
| ps | pass | 41 processes (piped) |
| ps-tty | pass | 41 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## 2026-10-28: T12 fix, further runs on the real Pi

Same builds as above, run over devshd, crashes read from the serial console.

| Build | Test | Result |
|---|---|---|
| `v0.2.0-9-g43f697e+xe0ce` | `ps` straight to the terminal | 80 of 80 with exit code 0 (20 in one session, 60 in three more) |
| `v0.2.0-9-g43f697e+xe0ce` | `reboot` over devsh | 5 of 5 rebooted at the first attempt, no `brk` |
| `v0.2.0-9-g43f697e+x0f50` | `pi-test.py --starts 600` | 1 of 600 failed: `pwd`, data abort (T1), `ELR` `0xC15C0C`, `FAR` `0x1281DD0` |

Seen on the side, not counted as T12:
- `ion` (PID 40, probably the shell of an earlier devsh session) ended in
  `abort()` once (`X17` = `abort` in `libc.so`). Its message went to the
  session, not the serial console.
- Twice a devsh login did not get past `redox login:` (two prompts, no
  echo); see T11.


## 2026-10-28: `v0.3.0-dev-15-g24fbb96+x9f76`

Built 2026-10-28T11:35:57Z, `libc.so` `d4f0dedd5403751a…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Stable network-boot image, bcm2835-sdhcid. A first run stopped at the first check: devshd gave no output for `cat /etc/redox-rpi-release`; it answered again a minute later.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-15-g24fbb96+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-15-g24fbb96+x9f76.toml |
| ps | pass | 45 processes (piped) |
| ps-tty | pass | 45 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## 2026-10-28: SD card stress test (#15), sdhcid and sdhost on the same commit

`scripts/sd-stress.py` on a real Pi 3B+ over netboot, data partition on the
netboot card (SDHC, 15193 MiB): a 256 MiB file, 1000 files of 4 KiB with 250
deleted and 250 new, then 5 restarts (`echo reset > /scheme/sys/kstop`), each
followed by a SHA-256 check of every file and 32 MiB more. Commands over the
serial console. Both images are 24fbb96 with `timesync` and `rtl8152`; the
sdhost image adds `sdhost-driver` and `sdhost-boot` (high speed, about 31 MHz).

| | `bcm2835-sdhcid` (`+x9f76`) | `bcm2835-sdhostd` (`+x6d3c`) |
|---|---|---|
| 256 MiB from `/scheme/rand` | 3.2 MB/s | 5.7 MB/s |
| 1000 small files, 250 replaced | 580 s | 555 s |
| 32 MiB per cycle | 2.6–3.4 MB/s | 4.0–6.5 MB/s |
| checks (after writing, 5 restarts) | 6 of 6: 0 missing, 0 changed | 6 of 6: 0 missing, 0 changed |
| back after a restart | 55–80 s | 50–55 s |
| files at the end, time | 1006, 37 min | 1006, 28 min |
| result | **pass** | **pass** |

The small files take the same time with both drivers: RedoxFS limits them
([#16](https://github.com/redox-rpi/redox-rpi/issues/16)), not the driver.
The script labels the sdhost run "sdhcid": `ps` cuts the process name, fixed
in the script afterwards.

A first sdhcid run on `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver`
is discarded: that image still has the T12 start crash, and the Pi stopped
responding at `sha256sum` over 750 files (power cycle needed). The script now
hashes at most about 110 files per command.

## 2026-10-28: `v0.3.0-dev-31-g115d7fe+x572d` (CPU clock, test #13)

Built 2026-10-28T13:33:47Z, `libc.so` `b59ed57f5ec14068…`, real Pi 3B+ over
netboot. Experiments `timesync`, `cpu-clock`, `rtl8152`: the stable image
plus `cpu-clock` ([#20](https://github.com/redox-rpi/redox-rpi/issues/20)).
The boot log shows `rpi-firmwared: ARM clock 600 -> 1400 MHz`.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-31-g115d7fe+x572d |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-31-g115d7fe+x572d.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed (at 1400 MHz, 54 s for the whole run) |

Speed, same boot, clock set through `/scheme/rpi-firmware/arm_clock`, 50
starts (`ls`, `pwd`) per round, three rounds each:

| ARM clock | Time for 50 starts | Failed | Temperature after |
|---|---|---|---|
| 600 MHz (`min`) | 16.4, 17.4, 17.4 s | 0 | 51.0–51.5 °C |
| 1400 MHz (`max`) | 8.3, 8.2, 8.7 s | 0 | 55.8 °C |

Long run: start loops without a pause, each time after a fresh boot:

| ARM clock | Duration | Starts | Failed | Temperature | Throttling |
|---|---|---|---|---|---|
| 1400 MHz | 30 min | 5100 | **276 (5.4 %)** | 57.5–60.1 °C | soft temperature limit reached again and again |
| 600 MHz | 30 min | 4350 | 5 (0.1 %) | 48.3–52.6 °C | none |
| 1400 MHz (repeat) | 20 min | 4000 | **128 (3.2 %)** | 56.4–60.1 °C | as in the first run |

- At 1400 MHz the first 600–900 starts pass. Failures begin when the SoC
  reaches about 59 °C and the firmware starts to use its soft temperature
  limit (`throttled` bit 19 set, bit 3 on and off; on the 3B+ the firmware
  lowers clock and voltage there). The failures are the known start crashes
  ([#1](https://github.com/redox-rpi/redox-rpi/issues/1), [#2](https://github.com/redox-rpi/redox-rpi/issues/2)): exception
  class `0x24` (data abort) and `0x3c` (`brk #1`), on all four cores.
- The throttle watch works: the log shows
  `rpi-firmwared: warning: soft temperature limit (flags 0x00080008, 60.1 °C, ARM 1400 MHz)`
  and `rpi-firmwared: no longer throttled` each time.
- `arm_clock` keeps showing the requested 1400 MHz while the firmware
  throttles; the clock it really runs at is a different property (measured
  clock, tag `0x30047`).
- devshd sessions broke off several times during the long runs; after about
  3 hours it gave no output any more.
- A run at a fixed 1200 MHz (below the soft limit) was planned but did not
  run: devshd stopped answering.

Result: the full clock doubles the speed but makes the start crashes
frequent under load. Not ready as the default; see #20.

## 2026-10-28: #1 x18 fix, `v0.3.0-dev-34-g04f0123+x4d10`, 1000 starts

Test #17 for [#1](https://github.com/redox-rpi/redox-rpi/issues/1): the
stable image's experiments (`timesync`, `rtl8152`) plus
[`sigentry-x18`](https://github.com/redox-rpi/redox-rpi/tree/t1-data-abort/patches/experiments/sigentry-x18), which builds relibc with x18 reserved. Real Pi 3B+
over netboot, default ARM clock (600 MHz), one boot,
`scripts/pi-test.py --starts 250` five times in a row.

| Chunk | Checks | Starts | Failed |
|---|---|---|---|
| 1 | version, manifest, ps, ps-tty, sdcard pass | 250 | 0 |
| 2 | devsh gave no output for the first command (`cat /etc/redox-rpi-release`); nothing started | 0 | – |
| 3 | all pass | 250 | 0 |
| 4 | all pass | 250 | 0 |
| 5 | all pass | 250 | 0 |
| **Total** | | **1000** | **0** |

No `UNHANDLED EXCEPTION` on the serial console during the run. For
comparison: the stable image without the fix had 0 of 300, and 1 of 900
on `v0.2.0-9-g43f697e+x0f50` (T12 fix only); QEMU went from 8 of 700 to 0
of 700 with the fix. Not tested at 1400 MHz, where the stable image fails
3-5 % of the starts (test #13).

## 2026-10-28: `v0.3.0-dev-59-g1559bca+x9906` (CPU clock with the x18 fix, test #19)

Built 2026-10-28T16:53:25Z, `libc.so` `e42257f7a7bb4c47…`, real Pi 3B+ over
netboot. Experiments `timesync`, `rtl8152`, `sigentry-x18`
([#1](https://github.com/redox-rpi/redox-rpi/issues/1)) and `cpu-clock`
([#20](https://github.com/redox-rpi/redox-rpi/issues/20)): the same test as
#13, now with the x18 fix. The boot log shows
`rpi-firmwared: ARM clock 600 -> 1400 MHz`.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-59-g1559bca+x9906 |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-59-g1559bca+x9906.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Long run at 1400 MHz from a fresh boot
(`patches/experiments/cpu-clock/clock-soak.py`, 30 minutes):

| Build | Starts | Failed | Temperature | Throttling |
|---|---|---|---|---|
| with the x18 fix (this test) | 7000 | **0** | 56.9–60.7 °C | soft temperature limit on and off, as in #13 |
| without it (#13) | 5100 | 276 (5.4 %) | 57.5–60.1 °C | the same |

- No unhandled exception in the serial log during the run, and no devshd
  session broke off.
- The crashes under load in #13 were the x18 bug from #1, made more frequent
  by load and speed, not a fault of the higher clock: with the fix the soft
  temperature limit is reached just as often and nothing fails.
- The throttle watch logged the soft limit 59 times in 30 minutes (each with
  a "no longer throttled"). Correct, but a lot of lines for a state the
  firmware handles itself.

Result: **pass**. With the x18 fix, 1400 MHz is ready for v0.3.0.

## 2026-10-28: `v0.3.0-dev-52-g7df70f9+xcb30` (dwc2d with interrupts, test #5)

Branch `dwc2-interrupts` (7df70f9) with `--experiment timesync --experiment
rtl8152 --experiment pi-clean-boot --experiment dwc2-interrupts`, real Pi
3B+ over netboot. The build predates the x18 fix on `main` (e2b987b), so
start failures of about 1 % would not have been a `dwc2d` problem; there
were none.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-52-g7df70f9+xcb30 |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-52-g7df70f9+xcb30.toml |
| ps | pass | 44 processes (piped) |
| ps-tty | pass | 44 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

- The boot log shows `dwc2d: interrupt /scheme/irq/phandle-1/1,9`: the
  driver waits for the controller's interrupt on the real BCM2837, through
  the LAN7800 (`link up after 1710 ms`). No "unexpected irq" lines.
- Network over the LAN7800: 2000 lines of output through devshd in 2 s.
- CPU time of `dwc2d` (`ps`, user + system): 2.85 s in 62 s with nothing
  to do, about 5 % of one core. The polling driver keeps one core busy all
  the time (4 min 21 s in about 80 s in QEMU, see the experiment's README).
  The rest is mostly the keyboard: 100 polls per second, each a split
  transaction through the hub that waits by spinning.
- USB keyboard: the maintainer logged in on the HDMI console, ran `ls`
  and `ps`; both worked.
- `pi-clean-boot` (#18): no `pcid`, `pcid-spawner`, `audiod` or `rtcd`
  messages in the boot log.
- First boot with the Anker 565 hub plugged in but no cable on it: `dwc2d`
  took the hub's RTL8153B as the network adapter (it is preferred over the
  LAN7800), had no link, `dhcpd` timed out and did not retry when the
  cable was plugged in later. Netbooting through the hub, U-Boot's r8152
  driver fails the HTTP download (`Rx: failed to receive: -5`). Passed on
  to test #4.
- Afterwards, on the stable image, devshd logins hung (#11), so the
  polling driver's CPU time was not measured on the Pi.

Result: **pass**.

## 2026-10-28: devshd hang-up (#11), `v0.3.0-dev-77-g6bcd0fb+x9f76`

Test #21 for [#11](https://github.com/redox-rpi/redox-rpi/issues/11): the
stable image's experiments (`timesync`, `rtl8152`) on branch
[`devshd-hangup`](https://github.com/redox-rpi/redox-rpi/tree/devshd-hangup), where devshd sends SIGHUP to the terminal's
foreground process group (then SIGKILL) when a client closes the
connection without `exit`. Real Pi 3B+ over netboot, 600 MHz, one boot.

| Step | Result |
|---|---|
| `scripts/pi-test.py` | version, manifest, ps, ps-tty, sdcard pass; 0 of 300 starts failed |
| 10 clean devsh sessions (`echo`, `exit`) | all work, about 1.3 s each |
| 3 dropped sessions (logged in, socket closed) | nothing left behind |
| 10 dropped sessions, 5 of them with `sleep 600` running | nothing left behind |
| `ps` afterwards | only the serial console's `login` processes, the current session and devshd with 4 threads |

The first run of the session script stopped silently after the third
dropped session (no error output was kept); the Pi was clean at that
point, and the repeat with 10 dropped sessions ran through. Before the fix,
in QEMU, each dropped session left a `login`, an `ion` and two devshd
threads behind. The reset over devsh needed 2 tries before the test and
1 after it.

## 2026-10-29: #30, `v0.3.0-dev-80-g8bba533+x197a` (sdhost default + kernel memory fix), real Pi

Rerun of #23 ([#6](https://github.com/redox-rpi/redox-rpi/issues/6),
[#31](https://github.com/redox-rpi/redox-rpi/issues/31)): branch
`sdhost-ctxleak` = `sdhost-default` + experiment `ctx-leak-fix`, with
`timesync` and `rtl8152`. Built 2026-10-28T23:31:57Z, real Pi 3B+ over
netboot, 15 GB SDHC card.

#23 (`v0.3.0-dev-76-g13092c5+x9f76`, same without the fix) had ended in the
kernel panic `not yet implemented: oom` while `sd-stress.py` wrote the 250
new small files: the kernel kept ~76 KB per `dd` run (#31).

Boot log: `rpi-firmwared` (ARM clock 600 -> 1400 MHz) before
`bcm2835-sdhostd: core clock 250 MHz, up to 400 MHz`, `SDHC/SDXC card, 15193
MiB, 4-bit bus, high speed, up to 50 MHz`, `serving /scheme/disk.mmc`; no
`bcm2835-sdhcid`; `/scheme/data` mounted.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-80-g8bba533+x197a |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-80-g8bba533+x197a.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |
| memory | pass | `free` used 141 MB after boot, 143 MB after the 300 starts (743 MB total) |
| sd-stress write | pass | 256 MiB at 7.9 MB/s; 1000 small files, 250 deleted, 250 new, 328 s; no `oom` |
| sd-stress verify | pass | 1001 files, 0 missing, 0 changed |
| sd-stress cycles | **stopped** | restart 1 done (back after 89 s), then no devsh answer from 08:25 UTC: the devshd hang ([#11](https://github.com/redox-rpi/redox-rpi/issues/11)), whose fix is not in this image. The Pi itself was alive (local console login). Stopped at the maintainer's request |

The memory fix holds on the Pi: the workload that ended #23 in a panic ran
through. Rerun the cycles with the devshd hangup fix included.

## 2026-10-29: #30 rerun, `v0.3.0-dev-120-g5ffb097+x197a` (sdhost default + kernel memory fix + devshd fix), real Pi

Rerun of the stopped first run above with the devshd fix
([#11](https://github.com/redox-rpi/redox-rpi/issues/11)) merged in:
branch `sdhost-ctxleak` 5ffb097 + experiment `ctx-leak-fix`, with
`timesync` and `rtl8152`. Real Pi 3B+ over netboot, 15 GB SDHC card.

Boot log: `rpi-firmwared` (ARM clock 600 -> 1400 MHz) before
`bcm2835-sdhostd: core clock 250 MHz, up to 400 MHz`, `SDHC/SDXC card, 15193
MiB, 4-bit bus, high speed, up to 50 MHz`, `serving /scheme/disk.mmc`; no
`bcm2835-sdhcid` after the reset; `/scheme/data` mounted.

| Check | Result | Detail |
|---|---|---|
| version, manifest | pass | build ID and libc.so hash match the manifest |
| ps, ps-tty, sdcard | pass | 42 processes; write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |
| memory | pass | `free` used 120 MB after boot, 120 MB after three `sd-stress.py` runs and their restarts (743 MB total) |
| sd-stress write | pass | 256 MiB at 7.7, 7.4 and 7.2 MB/s in three runs; 1000 small files, 250 deleted, 250 new, about 320 s each; no `oom` |
| data after a restart | pass | two real restarts in different runs, each followed by a SHA-256 check of all 1001 files: 0 missing, 0 changed; a third restart left all 1002 files in place (not hashed) |
| sd-stress result | fail (test harness) | see below; no data was lost |

Why `sd-stress.py` reported **fail** although the data survived:

1. Run 1: after `echo reset > /scheme/sys/kstop`, devshd went on
   answering during the shutdown, so the script checked the dying system
   (`/scheme/data` already gone: 1001 missing), and its second "restart" went
   to the same shutdown. Only one real reset happened; the check after it
   found all 1001 files unchanged.
2. Run 3 (with a local copy that waits for `kreset` on the console first):
   restart 1 passed. After restart 2, devshd answered 30 s after the reset,
   before `/scheme/data` was mounted, so all 1002 files were "missing"
   and the 32 MiB write failed; two minutes later all 1002 files were there.
   (Run 2 only timed out in the local copy's own wait.)

Afterwards devshd stopped accepting connections once (the console still
worked): the Pi was reset from the serial console and the stable image
`v0.3.0-dev-15-g24fbb96+x9f76` restaged.

Follow-ups for `scripts/sd-stress.py`: wait for the new boot (console
`kreset`, or a changed `uptime`) instead of devshd going away, and wait for
`/scheme/data` before checking.

## 2026-10-28: `v0.3.0-dev-94-gb1fcbe1+xe9c4`

Built 2026-10-28T19:56:45Z, `libc.so` `8eb8f49492564a29…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `serial-rx`, `timesync`. Experiment serial-rx (#28) on the stable base: serial-input-test.sh 20 of 20 lines of 200 characters complete. First pi-test.py attempt got no devsh output (#11), second attempt below

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-94-gb1fcbe1+xe9c4 |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-94-gb1fcbe1+xe9c4.toml |
| ps | pass | 46 processes (piped) |
| ps-tty | pass | 46 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Serial console: `patches/experiments/serial-rx/serial-input-test.sh 20`
wrote 20 lines of `echo` plus 200 characters, each in one piece at
115200 baud: **20 of 20 complete**. Login with `root` and the password at
normal speed worked, and the reset command at the end, sent in one piece,
arrived complete (before: cut off after 25 characters). No crash on the
console except the known `pcid` panic (no `pi-clean-boot` in this image).
The first `pi-test.py` attempt got no answer from devsh (#11); the second
attempt is the table above.

## 2026-10-29: `v0.3.0-dev-93-g2143ad7+x1044` (timerd, test #27)

Built 2026-10-28T20:16:41Z, real Pi 3B+ over netboot, branch `timerd`.
Experiments: `ldso-diagnostics`, `timesync`, `timerd`, `timerd-test` (two
test timers: `date` every minute, `uname -a` every 2 minutes with
`persistent = true`). Issue [#22](https://github.com/redox-rpi/redox-rpi/issues/22).

| Check | Result | Detail |
|---|---|---|
| version, manifest | pass | build ID and `libc.so` hash match |
| ps, ps-tty | pass | 43 processes |
| sdcard | pass | |
| starts | pass | 0 of 100 failed |
| timers run | pass | both test timers on schedule, each `exited with code 0`; `timerd -l` shows next and last run; last runs in `/scheme/data/timerd/` |
| waits for timesync | pass | boot 3: `timerd: clock set by timesync, 2026-10-29 10:47:41 UTC`, timers from 10:48 |
| catch-up | pass | after `echo 1793000000 > /scheme/data/timerd/test-catchup` and a reset: `test-catchup: starting uname (missed since 2026-10-26 07:33:20 UTC)` once; `test-minute` did not catch up |
| without NTP time | pass | boots 1 and 2: `timerd: no time from timesync after 300 s, going on with …`, then the timers ran on the saved time |

**timesync hangs when the network comes up late** (boots 1 and 2): DHCP was
not done when `timesync` started (`No route found for destination:
9.9.9.9`, the DNS server). `timesync` then printed nothing more, neither a
time nor "no time server answered", and did not write
`/tmp/timesync.synced`; `timesync -n` by hand got the time. Its DNS lookup
probably waits forever. The clock stayed 72 minutes behind (saved time).
In boot 3 the network was up in time and everything worked. Not a `timerd`
problem; needs a fix in `timesync` (a timeout around the lookup).

Boot 1 also looked hung after 3 minutes: no log output, no echo on the
serial console, devshd unreachable ("No route to host"); power-cycled with
the user's permission. Boot 2 with the same image did not hang, so the
cause is unclear.

## 2026-10-29: `v0.3.0-dev-148-g54f7433+x657a`, USB hotplug (#35, [#17](https://github.com/redox-rpi/redox-rpi/issues/17))

Branch `usb-hotplug` (`dwc2-default` + main + `--experiment usb-hotplug`),
real Pi 3B+ over netboot, the maintainer at the Pi for part B. **Pass.**

Part A, automated:

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-148-g54f7433+x657a |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-148-g54f7433+x657a.toml |
| ps | pass | 41 processes (piped) |
| ps-tty | pass | 41 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |
| hubs | info | hub 1 (4 ports) and hub 2 (3 ports, the LAN7515's) switch power per port; the external hub with the keyboard (05e3:0610) is ganged |
| port cycles | pass | `scripts/pi-hotplug-test.sh 192.168.178.148 3`: 3 of 3 times the LAN7800's port (hub 2 port 1) lost power, disconnected after 102–104 ms, was found again and took the network back; devshd answered after 7, 9 and 10 s |
| dwc2d CPU | info | 2.84 s user + 2.98 s system after boot, `pi-test.py` and the 300 starts |

Part B, with a person:

| Check | Result | Detail |
|---|---|---|
| keyboard hotplug | pass | the keyboard behind its external hub unplugged and plugged into 4 different ports (hub 1 ports 3 and 2, hub 2 ports 3 and 2): each time `keyboard removed`, then the hub and `keyboard at address 5` again; typing on the HDMI console worked each time |
| DHCP retry | pass | LAN cable pulled during the Redox boot: `LAN78xx no link yet`, `dhcpd: failed to receive offer … trying again every 7s`; cable back after `redox login:`: `dhcpd: configured after 4 attempts`, devshd answers |

Not tested: the Anker hub (RTL8153B) steps; they stay with test #4. Stable
image restored afterwards.

## 2026-10-29: `v0.3.0-dev-95-g0d47c4f+x43ba` (timesync with late DHCP, test #41)

Built 2026-10-29T11:05:15Z, real Pi 3B+ over netboot, branch `timerd`.
Experiments: `ldso-diagnostics`, `timesync`, `timerd`, `timerd-test`,
`late-dhcp` (starts `dhcpd` 90 seconds late). Checks the fix for the
`timesync` hang found in test #27
([#22](https://github.com/redox-rpi/redox-rpi/issues/22)).

| Check | Result | Detail |
|---|---|---|
| version, manifest | pass | build ID and `libc.so` hash match |
| ps, ps-tty | pass | 46 processes |
| sdcard | pass | |
| starts | pass | 0 of 100 failed |
| late DHCP | pass | `no time server answered within a minute (last: no default route (DHCP not done))`, `trying again every 30 s`, then `clock set to 2026-10-29 11:49:31 UTC from pool.ntp.org` |
| timerd | pass | `timerd: clock set by timesync` one second later; `test-catchup` caught up its run missed since test #27 |
| clock | pass | `date` the same as the build host to the second; `/tmp/timesync.synced` exists |

## 2026-10-29: `v0.3.0-dev-164-gf8f70a5+x9f76` (serial-rx as default patches, #28, #32), QEMU

Branch `serial-rx-base`: base `0024` and kernel `0007` on main after
`dwc2-default`, with `timesync` and `rtl8152`. QEMU `raspi3b`,
`qemu-raspi3b.sh --run` types one line of 5 × `echo <200 characters>`
(about 1050 characters) on the serial console.

- 3 of 3 runs: login, all 5 lines complete, no getty panic.
- For #32, the same test before: stable `v0.3.0-dev-15-g24fbb96+x9f76`
  4 of 7 runs with `getty: failed to write master PTY: ... WouldBlock` and
  a dead console; experiment image `v0.3.0-dev-94-gb1fcbe1+xe9c4` 7 of 7
  complete.
- The known `pcid` panic is still there (clean boot is not merged yet).


## 2026-10-29: `v0.3.0-dev-139-g66f7cba+x0002`

Built 2026-10-29T10:30:39Z, `libc.so` `454e580b0fb8d86b…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `sigregs-test`, `sigreturn-svc`, `timesync`. test 36 (#1 sigreturn-svc), ARM at 1400 MHz

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-139-g66f7cba+x0002 |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-139-g66f7cba+x0002.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 1000 failed |

Test 36 ([#1](https://github.com/redox-rpi/redox-rpi/issues/1)), branch
`t1-sigreturn`, experiment `sigreturn-svc`: the signal entry returns
through `svc #0x5352`. Extra checks over devsh:

- `sigregs process -n 3000`: pass, all registers including x18 kept.
- `sigregs thread -n 3000`: **fail** in 3 of 3 runs (also with `-n 300`):
  only 1 signal arrives, then none for 5 s, and SIGUSR1 stays blocked.
  QEMU passes it. Test 45 already expects this without the #4 fix; test 37
  checks the image with patch `0002`.
- No exception in the serial log during the run.
- The CPU ran at 1400 MHz (`rpi-firmwared` raises the clock at boot), not
  600 MHz as the plan row says. Under this load at 1400 MHz test 21 saw
  3-5 % failed starts; here 0 of 1000.

## 2026-10-29: `v0.3.0-dev-144-g2e6f4f9+x9c38`

Built 2026-10-29T11:53:44Z, `libc.so` `070b096e3e3d4be2…`, real Pi 3B+ over netboot. Experiments: `prebuilt-programs`, `rtl8152`, `serial-rx`, `sigregs-test`, `sigreturn-svc`, `timesync`. test 45 ([#40](https://github.com/redox-rpi/redox-rpi/issues/40) prebuilt programs), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-144-g2e6f4f9+x9c38 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 45, branch `prebuilt-sigreturn` (`prebuilt-programs` + `t1-sigreturn`).
Extra checks over devsh, with the ARM at 1400 MHz:

- 50 starts each of `bash -c true`, `nano --version`, `vim --version`,
  `find /usr/bin -name ls`, `gawk 'BEGIN{}'`, `tmux -V`, `rg --version`,
  `curl --version`, `wget --version`: 450 of 450 exit 0 (tmux 3.6a,
  nano 9.2).
- `bash -c 'for j in $(seq 1 200); do true; /usr/bin/true; x=$(echo a); done'`
  5 times: 5 of 5 exit 0 (3000 bash children). The first devsh session
  gave no output for this command, and `pi-test.py`'s retry ran it again
  in a new session; no `bash` was left over afterwards.
- `sigregs process -n 3000`: pass, all registers kept.
- `sigregs thread -n 3000`: fail as expected (only 1 of 3000 signals,
  SIGUSR1 still blocked; #4 fix not in this image).
- Serial log: no exception except the known `pcid` panic at boot (this
  base has no clean boot yet).

## 2026-10-29: `v0.3.0-dev-15-g24fbb96+x9f76`

Built 2026-10-28T11:35:57Z, `libc.so` `d4f0dedd5403751a…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. test 48 (#10 start loop with crash details), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-15-g24fbb96+x9f76 |
| manifest | pass | present in the image |
| ps | pass | 48 processes (piped) |
| ps-tty | pass | 48 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | **fail** | 3 of 1000 failed: ls exit 1 (1×); pwd exit 1 (2×) |

Failed starts:

- starts 151–200: `pwd`, exit 1, last serial lines:

  ```
  kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100010)
  FAR_EL1: 0xc5778b
  ELR_EL1: 0000000000C5778B
  ESR_EL1: 000000008A000000
  kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #0, PID 258, NAME /usr/bin/pwd, CONTEXT 0xfffffe8000156630
  ```
- starts 701–750: `pwd`, exit 1, last serial lines:

  ```
  kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100010)
  FAR_EL1: 0x1
  ELR_EL1: 0000000000000001
  ESR_EL1: 000000008A000000
  kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #0, PID 793, NAME /usr/bin/pwd, CONTEXT 0xfffffe8000156630
  ```
- starts 751–800: `ls`, exit 1, last serial lines:

  ```
  kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)
  FAR_EL1: 0x1281dd0
  ELR_EL1: 0000000000C15C0C
  ESR_EL1: 0000000092000006
  kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #3, PID 833, NAME /usr/bin/ls, CONTEXT 0xfffffe8000156630
  ```

Test 48 ([#10](https://github.com/redox-rpi/redox-rpi/issues/10)):
`scripts/pi-test.py` from branch `t10-start-loop` (e904958) with
`--starts 1000 --serial-log ~/redox-rpi/logs/serial.log`, against the
stable image already on the Pi, without a reset. **Pass** for #10: the run
finished in 9 minutes, and each of the 3 failed starts is listed with its
program, exit status and the crash dump from the serial log (PID and NAME
match the program). None of the 3 printed anything, so the "messages" part
stayed empty. The failures themselves are the known start crashes of the
stable image (#1, no x18 fix; ARM at 1400 MHz), not a problem of the
script.

## 2026-10-29: `v0.3.0-dev-140-g55bd363+x68b9`

Built 2026-10-29T10:35:46Z, `libc.so` `435c647e19cf8b13…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `sigentry-thread-signals`, `sigregs-test`, `timesync`. test 37 ([#4](https://github.com/redox-rpi/redox-rpi/issues/4) thread signals, patch 0002), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-140-g55bd363+x68b9 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 37 ([#4](https://github.com/redox-rpi/redox-rpi/issues/4)), branch
`t4-thread-signals`, experiment `sigentry-thread-signals` with review patch
`0002`. Extra checks over devsh, ARM at 1400 MHz:

- `sigregs sender`: pass, `si_pid 50 from the thread, 51 from the child`.
- `sigregs thread -n 3000`: pass, all registers kept (95 s). Tests 36 and
  45, without this fix, got only 1 of 3000 signals here.
- `sigregs process -n 3000`: pass, all registers kept (79 s).
- No exception in the serial log during the run.

## 2026-10-29: `v0.3.0-dev-164-g2ce732b+x8cc4`

Built 2026-10-29T11:11:56Z, `libc.so` `08e0353fe25191f9…`, real Pi 3B+ over netboot. Experiments: `redoxfs-write-coalesce`. test 46 ([#16](https://github.com/redox-rpi/redox-rpi/issues/16) RedoxFS write coalescing), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-164-g2ce732b+x8cc4 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 41 processes (piped) |
| ps-tty | pass | 41 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 46 ([#16](https://github.com/redox-rpi/redox-rpi/issues/16)), branch
`t16-write-amp`, experiment `redoxfs-write-coalesce`. Extra checks over
devsh on `/scheme/data`, against the stable image `v0.3.0-dev-15-g24fbb96+x9f76`
on the same card right after:

| `dd` to `/scheme/data`, 1 MiB, `conv=fsync` | this image | stable |
|---|---|---|
| `if=/scheme/rand bs=4096 count=256` | 0.40, 0.44, 0.38, 0.39 s | 10.07, 10.23 s |
| `if=/scheme/rand bs=1048576 count=1` | 0.37, 0.30, 0.27, 0.27 s | 0.31, 0.30 s |

- The 4 KiB run now takes about as long as the 1 MiB run (about 25 times
  faster than stable): **pass**.
- A 1 MiB random file copied with `bs=4k`, `bs=1000` and `bs=512` has the
  source's SHA-256, also after `echo reset > /scheme/sys/kstop`: **pass**.
- 300 `echo line $i >> log` give 300 lines, also after the reset: **pass**.
- Found, not caused by the fix: copying a file to another file on
  `/scheme/data` with small blocks stays slow, 1 MiB with `bs=4k` in 11.9 s
  (stable 12.7 s), `bs=1000` in 45 s, `bs=512` in 86 s (stable 94 s). Each
  half alone is fast: reading the file with `bs=512` 0.6 s, writing 2048
  blocks of 512 from `/scheme/rand` 0.5 s, or from `/tmp` 1.0 s. So it is
  slow only when reads and writes on RedoxFS take turns.
- Serial log: only the known `pcid` panic at boot.

## 2026-10-29: `v0.3.0-dev-186-gb11ec12+x4e3e`

Built 2026-10-29T12:30:31Z, `libc.so` `da6936b2d6a81fd7…`, real Pi 3B+ over netboot. Experiments: `pkg-hold`, `timesync`. test 52 ([#44](https://github.com/redox-rpi/redox-rpi/issues/44) pkg hold list), Pi test agent, before pkg

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-186-gb11ec12+x4e3e |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 52 ([#44](https://github.com/redox-rpi/redox-rpi/issues/44)), branch
`pkg-hold`. Extra checks over devsh, after `timesync` set the clock from
pool.ntp.org:

- `/etc/pkg/hold` lists `base`, `base-initfs`, `bootloader`, `kernel`,
  `relibc`, `pkgutils`.
- `pkg update --yes kernel relibc base pkgutils` (0.5 s) and
  `pkg install --yes kernel relibc` (0.1 s) print `<name> is held
  (etc/pkg/hold), keeping the installed version` for each, exit 0.
- SHA-256 of `/usr/lib/boot/kernel` (`21af7677…`), `/usr/lib/libc.so`
  (`da6936b2…`) and `/usr/bin/pkg` (`0c314836…`) are the same before,
  after the update and after installing nano: **pass**.
- `pkg install --yes nano`: 214 files, 27 s (QEMU about 9 minutes), exit 0;
  `nano --version` gives GNU nano 9.2: **pass**.
- `scripts/pi-test.py` standard checks again afterwards: all pass, 0 of 300
  starts failed, 47 processes.
- No exception in the serial log.

## 2026-10-29: `v0.3.0-dev-214-g67fdc06+x19b9`

Built 2026-10-29T13:54:46Z, `libc.so` `8eb8f49492564a29…`, real Pi 3B+ over netboot. Experiments: `boot-time`, `rtl8152`, `timesync`. test 53 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47) boot time), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-214-g67fdc06+x19b9 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 47 processes (piped) |
| ps-tty | pass | 47 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 53 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47)), branch
`boot-time` (`~/redox-rpi-t18` 43102e8, image `~/t18-work/image-bt3/`, staged
with that tree's `netboot.sh`, so the new `netboot.cmd` asks DHCP once).
Four boots, each timed with `boot-timeline.py` on the serial log from the
reset (`echo reset > /scheme/sys/kstop` over devsh); timelines in
`~/t18-work/tl-pi-{stable-1,stable-2,bt-1,bt-2}.txt` on homenodellm.

| Seconds from reset | stable-1 | stable-2 | bt-1 | bt-2 |
|---|---|---|---|---|
| U-Boot banner | 4.18 | 3.96 | 4.21 | 3.76 |
| DHCP bound | 11.57 | 11.40 | 11.66 | 11.17 |
| image loaded over HTTP | 15.39 | 15.21 | 15.42 | 14.97 |
| bootloader mode menu | 17.70 | 17.56 | 17.77 | 17.33 |
| kernel starts | 23.41 | 23.25 | 19.48 | 19.03 |
| `redox login:` | **33.75** | **33.70** | **29.51** | **29.05** |

- About 4.5 s faster, almost all from the bootloader wait (`in 5 seconds`
  → `Continuing with the selected mode in 1 s`). The second DHCP that the
  stable image asks costs only 0.1 s on the Pi (bound twice at 11.57 and
  11.67 s); the boot-time image asks once.
- Kernel start to `redox login:` about 10 s in both.
- Units of 100 ms or more (same in both boots): `40_dwc2d.service ready
  after 6176 ms`, `40_hwd.service` 338/339 ms, `fbcond@2.service` 122/102
  ms, `60_data.service` 367/377 ms.
- `redox-rpi v0.3.0-dev-214-g67fdc06+x19b9` in the log; standard checks
  pass, 0 of 300 starts failed. Only the known `pcid` panic in the log.

## 2026-10-29: `v0.3.0-dev-203-g6cfdbe3+xf552`

Built 2026-10-29T12:45:26Z, `libc.so` `f4c29e0c5fc7c368…`, real Pi 3B+ over netboot. Experiments: `ipv6`, `lan78xx-multicast`, `mdns`, `multicast`, `rtl8152`, `timesync`. test 47 ([#27](https://github.com/redox-rpi/redox-rpi/issues/27) IPv6, [#37](https://github.com/redox-rpi/redox-rpi/issues/37) mDNS), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-203-g6cfdbe3+xf552 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 43 processes (piped) |
| ps-tty | pass | 43 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 47 ([#27](https://github.com/redox-rpi/redox-rpi/issues/27),
[#37](https://github.com/redox-rpi/redox-rpi/issues/37)), branch
`net-pi-test`. Extra checks; homenodellm is on Wi-Fi (`wlp13s0`) and has no
`ping`, so the pings used an unprivileged ICMP socket from Python:

- `/scheme/netcfg/ifaces/eth0/addr/list`: `192.168.178.148/24`,
  `fe80::ba27:ebff:fe08:e0aa/64`, `fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa/64`
  (SLAAC from the Fritz!Box's ULA prefix). **No global `2a02:…` address**,
  also after 9.5 minutes, and no IPv6 default route in
  `/scheme/netcfg/route/list`. The Fritz!Box does send router
  advertisements with the global prefix: homenodellm has
  `2a02:590:1507:c201::/64 proto ra` and `fdeb:d4ef:a7ac::/64 proto ra`,
  with the default route via `fe80::b6fc:7dff:fe53:b289`. So the Pi takes
  the ULA prefix from the same advertisements and leaves out the global
  one and the default router.
- Ping from homenodellm: `fe80::ba27:ebff:fe08:e0aa%wlp13s0` 3 of 3
  (2–4 ms), `fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa` 3 of 3; no global
  address to ping.
- `mdns-query.py redox-rpi.local`: `A 192.168.178.148` from the Pi (ttl
  120), also from the Fritz!Box's cache. `mdns-query.py _devsh._tcp.local
  PTR`: `redox-rpi._devsh._tcp.local`; asked for SRV: `port 2323 target
  redox-rpi.local`, TXT empty: **pass**.
- Standard checks pass; only the known `pcid` panic in the serial log.
- Afterwards the first boot of the stable image had no network: `dwc2d: 1
  device(s) enumerated` (the LAN7800 behind the hub was not found),
  `smoltcpd: no network adapter found`. A second reset over the serial
  console found all 5 devices.

## 2026-10-29: `v0.3.0-dev-209-ga15774d+xf552`

Built 2026-10-29T13:25:24Z, `libc.so` `f4f18ab1a43e88c1…`, real Pi 3B+ over netboot. Experiments: `ipv6`, `lan78xx-multicast`, `mdns`, `multicast`, `rtl8152`, `timesync`. test 55 ([#27](https://github.com/redox-rpi/redox-rpi/issues/27) IPv6 steps 3–4, [#37](https://github.com/redox-rpi/redox-rpi/issues/37) mDNS over IPv6), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-209-ga15774d+xf552 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 43 processes (piped) |
| ps-tty | pass | 43 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 55 ([#27](https://github.com/redox-rpi/redox-rpi/issues/27),
[#37](https://github.com/redox-rpi/redox-rpi/issues/37)), branch `mdns6`,
scripts `udp-echo.py` and `mdns-query.py` from `mdns6`:

- Addresses: `fe80::ba27:ebff:fe08:e0aa/64` and the ULA
  `fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa/64`; **no global `2a02:…` address
  and no IPv6 default route**, as in test 47.
- `/scheme/netcfg/resolv/nameserver6`: `fdeb:d4ef:a7ac:0:b6fc:7dff:fe53:b289`
  (the Fritz!Box, from RDNSS): **pass**.
- `net6 resolve example.com` in 0.22 s: two IPv4 and two IPv6 addresses
  (`2606:4700:10::ac42:93f3`, `2606:4700:10::6814:179a`): **pass**.
- `net6 udp`: to homenodellm's ULA `[fdeb:…:4a45:e6ff:fea0:b063]:7777`
  and to `192.168.178.120:7777` the reply comes back (the echo saw the Pi's
  ULA and IPv4 address): **pass**. To homenodellm's global
  `[2a02:590:1507:c201:4a45:e6ff:fea0:b063]:7777`: `receive: Connection
  timed out`, nothing reached the echo (no global address or IPv6 route on
  the Pi).
- Ping from homenodellm to the Pi's ULA: 3 of 3; no `2a02` address to ping.
- `mdns-query.py redox-rpi.local AAAA -6 wlp13s0`: the Pi answers over
  IPv6 with AAAA `fe80::…` and `fdeb:…` plus A `192.168.178.148`;
  `mdns-query.py redox-rpi.local` over IPv4: A plus both AAAA as
  additional records: **pass** (the LAN7800 takes `33:33:00:00:00:fb`).
- Standard checks pass; only the known `pcid` panic in the serial log.


## 2026-10-29: `v0.3.0-dev-129-g6917241+x9f76` (clean boot as the default patch, #18), QEMU

Branch `clean-boot-default`: base `0024` (was experiment `pi-clean-boot`; base `0030` on main)
on main, with `timesync` and `rtl8152`. QEMU `raspi3b`, serial console,
no network card (`scripts/qemu-raspi3b.sh --run ps`).

- Login reached, `/etc/redox-rpi-release` shows the build ID.
- No `pcid`, `pcid-spawner`, `audiod` or `rtcd` line in the console and
  no such process in `ps`; no `pcid` panic.
- `netstack` "exited without notifying readiness": this run has no network
  card. The Pi test (#33) checks the network.

## 2026-10-29: #33, `v0.3.0-dev-129-g6917241+x9f76` (clean boot as the default patch, #18), real Pi

Same image as the QEMU check above, real Pi 3B+ over netboot.

Boot log: none of the `pcid` panic, `pcid-spawner`, `audiod` or `rtcd`
lines; no other failed service. `dwc2d: LAN78xx link up`, DHCP bound to
192.168.178.148. `ps` over devsh: no `pcid`, `audiod` or `rtcd` process.

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-129-g6917241+x9f76 |
| manifest | pass | present in the image |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Stable image `v0.3.0-dev-15-g24fbb96+x9f76` staged again afterwards and
booted to the login prompt.

## 2026-10-29: `v0.3.0-dev-122-g74a1e24+x607a`

Built 2026-10-29T10:18:15Z, `libc.so` `08e0353fe25191f9…`, real Pi 3B+ over netboot. Experiments: `ctx-leak-fix`, `rtl8152`, `sdhost-dma`, `timesync`. test 42 ([#21](https://github.com/redox-rpi/redox-rpi/issues/21) sdhost with DMA), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-122-g74a1e24+x607a |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 47 processes (piped) |
| ps-tty | pass | 47 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 42 ([#21](https://github.com/redox-rpi/redox-rpi/issues/21)), branch
`sdhost-dma`. Boot log: `bcm2835-sdhostd: DMA channel 4 at 0x3f007000`,
`SDHC/SDXC card, 15193 MiB, 4-bit bus, high speed, up to 50 MHz`,
`serving /scheme/disk.mmc`; no `DMA error`, `DMA timeout` or `no DMA`, no
`bcm2835-sdhcid`.

| 256 MiB on `/scheme/data`, `bs=1048576` | time | MB/s | CPU time of `bcm2835-sdhostd` |
|---|---|---|---|
| write `conv=fsync` (run 1) | 37.8 s | 7.1 | 45.3 s for write and read together |
| write `conv=fsync` (run 2) | 30.3 s | 8.8 | +23.7 s (45.30 → 69.02 s) |
| read (run 2) | 15.1 s | 18 | +13.7 s (69.02 → 82.72 s) |
| `sd-stress.py` write | | 6.9 | |

With DMA the driver still uses the CPU for about 80 % of the write time
and 90 % of the read time (it looks like it waits for DMA by polling). For
comparison, #30 with PIO wrote at 7.9 MB/s (`sd-stress.py`); no CPU time was
noted there.

`scripts/sd-stress.py --cycles 2 --serial-log ~/redox-rpi/logs/serial.log`
(main with 56e2df4):

- write: 256 MiB at 6.9 MB/s; 1000 small files, 250 deleted, 250 new,
  358 s.
- verify: 1001 files, 0 missing, 0 changed: **pass**.
- cycle 1: reset, login prompt after 28 s, then **`/scheme/data not mounted
  within 300 s`**, and the script stopped (fail). When I looked right after,
  at 5 min 50 s uptime, `/scheme/data` was mounted with all 1001 files
  (270 MB), and the script's own check (`ls /scheme` shows `data`) passed.
  So either the mount took more than about 5 minutes, or the devsh session
  stopped answering during the wait; the log does not say which. Test 30
  saw a similar late mount after a reset ("two minutes later all 1002 files
  were there"). The boot log of that boot looks like a normal one.
- Afterwards: standard checks pass, 0 of 300 starts failed; `free` used
  145 MB (138 MB after boot, 203 MB right after the 256 MiB runs). Test
  files deleted.

## 2026-10-29: `v0.3.0-dev-212-gc9fcc6d+x12cb`

Built 2026-10-29T13:30:57Z, `libc.so` `08e0353fe25191f9…`, real Pi 3B+ over netboot. Experiments: `timesync`. test 61 (v0.3.0 merge candidate for [#6](https://github.com/redox-rpi/redox-rpi/issues/6) and [#31](https://github.com/redox-rpi/redox-rpi/issues/31)), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-212-gc9fcc6d+x12cb |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-212-gc9fcc6d+x12cb.toml |
| ps | pass | 47 processes (piped) |
| ps-tty | pass | 47 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |


## 2026-10-29: `v0.3.0-dev-212-gc9fcc6d+x12cb`, SD card stress test (sdhost)

`scripts/sd-stress.py`: 256 MiB file (8.0 MB/s), 1000 files of 4 KiB with 250 replaced, then 2 restarts, each followed by a check of every file and 32 MiB more; 1003 files at the end, 9 minutes. test 61, sd-stress rerun with 3f6079c, Pi test agent

| Check | Missing | Changed |
|---|---|---|
| after writing | 0 | 0 |
| restart 1 | 0 | 0 |
| restart 2 | 0 | 0 |

Result: **pass**.

Test 61 (v0.3.0 merge candidate for [#6](https://github.com/redox-rpi/redox-rpi/issues/6) and [#31](https://github.com/redox-rpi/redox-rpi/issues/31)), branch `sdhost-ctxleak` c9fcc6d:

- Boot log: `bcm2835-sdhostd: SDHC/SDXC card, 15193 MiB, 4-bit bus, high speed, up to 50 MHz`, `serving /scheme/disk.mmc`; no `bcm2835-sdhcid`. Known `pcid` panic only.
- `free` used: 137 MB after boot, 145 MB after the first stress run, and the same check after the second run (see the commit's result line).
- First `sd-stress.py` run (main before 3f6079c): write and verify passed (1001 files, 6.6 MB/s), then after restart 1 `/scheme/data not mounted within 300 s`; at 5 min 52 s uptime it was mounted with all files. Same as test 42.
- Rerun with 3f6079c (above): **pass**, `/scheme/data` there 11 s and 12 s after the login prompt; 0 missing, 0 changed over 2 restarts.

## 2026-10-29: `v0.3.0-dev-176-g752ac70+x8057`

Built 2026-10-29T14:51:21Z, `libc.so` `94f45c69b01420be…`, real Pi 3B+ over netboot. Experiments: `dwc2d-boottime`, `usb-hotplug`. test 67 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47) shorter dwc2d boot), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-176-g752ac70+x8057 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 41 processes (piped) |
| ps-tty | pass | 41 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 67 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47)), branch
`dwc2d-boottime`. Two boots timed with `boot-timeline.py` from the reset
over devsh; timelines in `~/dwc2boot-work/tl-pi-{1,2}.txt` on homenodellm.

| Seconds from reset | boot 1 | boot 2 |
|---|---|---|
| kernel starts (`Redox OS starting...`) | 23.06 | 23.16 |
| `dwc2d: controller at` | 25.43 | 25.47 |
| `link up on the LAN7800 at address 5` | 28.69 | 28.78 |
| `dwc2d: devices ready after 3294 ms` | 28.69 | 28.78 |
| `redox login:` | 30.50 | 30.58 |
| kernel to `redox login:` | **7.44 s** | **7.42 s** |

- `devices ready after 3294 ms` in both boots (test 53: 6176 ms): well
  below, though above the expected 2000–2500 ms. About 2.1 s of it is the
  LAN7800's link wait (`LAN7800 at address 5` at 26.63 s, link up at
  28.69 s).
- Kernel to login 7.4 s instead of about 10.3 s in test 53. (This image
  still has the 5 s bootloader wait, so reset to login is 30.5 s.)
- Devices: `hub at address 1` (4 ports, per port), `2` (3 ports, per port),
  `3` (4 ports, ganged), `keyboard at address 4`, `LAN7800 at address 5`,
  `network address b8:27:eb:08:e0:aa` (as before). The plan row expected
  the Anker hub at address 4; here the third hub is at address 3. No
  `no link yet`, `reset timed out` or `request … failed`: **pass**.
- `scripts/pi-test.py` standard checks: pass, 0 of 300 starts failed.
- `scripts/pi-hotplug-test.sh <pi> 3` (from `~/redox-rpi-dwc2boot`): 15 of
  15 checks pass, run twice; port off, disconnected after 104 ms, LAN7800
  found again, link up, devshd answers again each cycle.
  `KEYBOARD=1 … 1`: keyboard removed and found again: **pass**.

## 2026-10-29: `v0.3.0-dev-216-gfb78c4a+xd4e7`

Built 2026-10-29T13:55:43Z, `libc.so` `e73ca44da45e3559…`, real Pi 3B+ over netboot. Experiments: `ctx-leak-fix`, `rtl8152`, `timesync`, `wifi-sdio`. test 57 ([#26](https://github.com/redox-rpi/redox-rpi/issues/26) Wi-Fi SDIO probe), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-216-gfb78c4a+xd4e7 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 47 processes (piped) |
| ps-tty | pass | 47 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 57 ([#26](https://github.com/redox-rpi/redox-rpi/issues/26)), branch `wifi`, experiment `wifi-sdio`. Every `bcm2835-sdiod:` line from the serial log:

```
bcm2835-sdiod: wifi_power was 0
bcm2835-sdiod: wifi_power 0: I/O error (os error 5)
bcm2835-sdiod: wifi_power 1: I/O error (os error 5)
bcm2835-sdiod: GPFSEL3 0x3ffff000 GPFSEL4 0x24200924 GPFSEL5 0x00000924
bcm2835-sdiod: base clock 200000000 Hz, SLOTISR_VER 0x99020000
bcm2835-sdiod: CMD5 R4 0x30ffff00, 3 functions
bcm2835-sdiod: card selected, RCA 0x0001
bcm2835-sdiod: CCCR revision 0x32, capability 0x02
bcm2835-sdiod: CIS vendor 0x02d0 device 0xa9a6
bcm2835-sdiod: function 1 ready
bcm2835-sdiod: ALP clock on (CHIPCLKCSR 0x68)
bcm2835-sdiod: CMD52 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: CMD53 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: at 25000000 Hz, 4 bits:
bcm2835-sdiod: CMD53 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: OK
```

- **Pass**: `bcm2835-sdiod: OK`; `CMD5 R4 0x30ffff00, 3 functions`; `CIS
  vendor 0x02d0`; `function 1 ready`; `ALP clock on`; the three `chipid`
  lines (CMD52, CMD53, CMD53 at 25 MHz 4 bits) all `chip 0x4345 rev 6`.
- Odd: writing `wifi_power` (0 and 1) gave `I/O error (os error 5)`, and it
  "was 0" before; afterwards `cat /scheme/rpi-firmware/wifi_power` gives
  `1`, and the chip answered.
- `scripts/pi-test.py` standard checks pass, 0 of 300 starts failed; no
  `bcm2835-sdhostd` errors. Only the known `pcid` panic in the log.

## 2026-10-29: `v0.3.0-dev-218-g7d69076+xd4e7`

Built 2026-10-29T14:24:37Z, `libc.so` `e73ca44da45e3559…`, real Pi 3B+ over netboot. Experiments: `ctx-leak-fix`, `rtl8152`, `timesync`, `wifi-sdio`. test 60 ([#26](https://github.com/redox-rpi/redox-rpi/issues/26) Wi-Fi step 2, firmware), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-218-g7d69076+xd4e7 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 60 ([#26](https://github.com/redox-rpi/redox-rpi/issues/26)), branch `wifi`, experiment `wifi-sdio`. Every `bcm2835-sdiod:` line of the boot (minimal firmware):

```
bcm2835-sdiod: EMMC at 0x3f300000
bcm2835-sdiod: wifi_power was 0
bcm2835-sdiod: wifi_power 0: I/O error (os error 5)
bcm2835-sdiod: wifi_power 1: I/O error (os error 5)
bcm2835-sdiod: GPFSEL3 0x3ffff000 GPFSEL4 0x24200924 GPFSEL5 0x00000924
bcm2835-sdiod: base clock 200000000 Hz, SLOTISR_VER 0x99020000
bcm2835-sdiod: CMD5 R4 0x30ffff00, 3 functions
bcm2835-sdiod: card selected, RCA 0x0001
bcm2835-sdiod: CCCR revision 0x32, capability 0x02
bcm2835-sdiod: CIS vendor 0x02d0 device 0xa9a6
bcm2835-sdiod: function 1 ready
bcm2835-sdiod: ALP clock on (CHIPCLKCSR 0x68)
bcm2835-sdiod: CMD52 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: CMD53 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: at 25000000 Hz, 4 bits:
bcm2835-sdiod: CMD53 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: chip ID OK
bcm2835-sdiod: core 0x800 rev 51 at 0x18000000, wrapper 0x18100000
bcm2835-sdiod: core 0x812 rev 54 at 0x18001000, wrapper 0x18101000
bcm2835-sdiod: core 0x83e rev 9 at 0x18002000, wrapper 0x18102000
bcm2835-sdiod: core 0x83c rev 14 at 0x18003000, wrapper 0x18103000
bcm2835-sdiod: core 0x829 rev 21 at 0x18004000, wrapper 0x18104000
bcm2835-sdiod: RAM 0x00198000, 800 KiB
bcm2835-sdiod: minimal firmware, 548728 bytes; NVRAM 1748 bytes
bcm2835-sdiod: firmware written (548728 bytes in 298 ms) and read back
bcm2835-sdiod: NVRAM written at 0x0025f92c (1748 bytes)
bcm2835-sdiod: ARM started, reset vector 0xb83ef198
bcm2835-sdiod: HT clock on (CHIPCLKCSR 0xd0)
bcm2835-sdiod: function 2 ready, firmware running
bcm2835-sdiod: mailbox from the firmware 0x00040008
bcm2835-sdiod: event frame (channel 1), 0 bytes
bcm2835-sdiod: firmware version: wl0: Nov  1 2021 00:37:12 version 7.45.241 (1a2f2fa CY) FWID 01-5209c09
bcm2835-sdiod: mailbox from the firmware 0x00040002
bcm2835-sdiod: MAC address b8:27:eb:5d:b5:ff
bcm2835-sdiod: OK
```

Then over devsh as root, `/usr/lib/drivers/bcm2835-sdiod standard` (exit 0), lines that differ from the boot:

```
bcm2835-sdiod: wifi_power was 1
bcm2835-sdiod: standard firmware, 609309 bytes; NVRAM 1748 bytes
bcm2835-sdiod: firmware written (609312 bytes in 303 ms) and read back
bcm2835-sdiod: firmware version: wl0: Aug 29 2023 01:47:08 version 7.45.265 (28bca26 CY) FWID 01-b677b91b
```

- **Pass**: chip ID OK; cores 0x83e, 0x812, 0x829 (and 0x800, 0x83c);
  RAM 0x00198000, 800 KiB; minimal firmware 548728 bytes written in 298 ms
  and read back; NVRAM written; ARM started; HT clock on; function 2 ready,
  firmware running; `firmware version: wl0: Nov  1 2021 … version 7.45.241`;
  MAC address `b8:27:eb:5d:b5:ff`; last line `OK`.
- Between `ARM started` and `OK`: mailbox `0x00040008`, one event frame
  (channel 1, 0 bytes), the version, mailbox `0x00040002`, the MAC.
- Standard firmware: 609312 bytes in 303 ms, version 7.45.265 (Aug 29
  2023), same MAC, `OK`: **pass**.
- Writing `wifi_power` still gives `I/O error (os error 5)` (as in test 57).
- `scripts/pi-test.py` standard checks pass, 0 of 300 starts failed.

## 2026-10-29: `v0.3.0-dev-220-gbec0fd5+xd4e7`

Built 2026-10-29T14:43:58Z, `libc.so` `e73ca44da45e3559…`, real Pi 3B+ over netboot. Experiments: `ctx-leak-fix`, `rtl8152`, `timesync`, `wifi-sdio`. test 63 ([#26](https://github.com/redox-rpi/redox-rpi/issues/26) Wi-Fi steps 3–4, scan), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-220-gbec0fd5+xd4e7 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 63 ([#26](https://github.com/redox-rpi/redox-rpi/issues/26)), branch `wifi` (bec0fd5). Every `bcm2835-sdiod:` line of the boot (the driver ran twice at boot):

```
bcm2835-sdiod: EMMC at 0x3f300000
bcm2835-sdiod: wifi_power was 0
bcm2835-sdiod: wifi_power 0: I/O error (os error 5)
bcm2835-sdiod: wifi_power 1: I/O error (os error 5)
bcm2835-sdiod: GPFSEL3 0x3ffff000 GPFSEL4 0x24200924 GPFSEL5 0x00000924
bcm2835-sdiod: base clock 200000000 Hz, SLOTISR_VER 0x99020000
bcm2835-sdiod: CMD5 R4 0x30ffff00, 3 functions
bcm2835-sdiod: card selected, RCA 0x0001
bcm2835-sdiod: CCCR revision 0x32, capability 0x02
bcm2835-sdiod: CIS vendor 0x02d0 device 0xa9a6
bcm2835-sdiod: function 1 ready
bcm2835-sdiod: ALP clock on (CHIPCLKCSR 0x68)
bcm2835-sdiod: CMD52 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: CMD53 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: at 25000000 Hz, 4 bits:
bcm2835-sdiod: CMD53 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: chip ID OK
bcm2835-sdiod: core 0x800 rev 51 at 0x18000000, wrapper 0x18100000
bcm2835-sdiod: core 0x812 rev 54 at 0x18001000, wrapper 0x18101000
bcm2835-sdiod: core 0x83e rev 9 at 0x18002000, wrapper 0x18102000
bcm2835-sdiod: core 0x83c rev 14 at 0x18003000, wrapper 0x18103000
bcm2835-sdiod: core 0x829 rev 21 at 0x18004000, wrapper 0x18104000
bcm2835-sdiod: RAM 0x00198000, 800 KiB
bcm2835-sdiod: minimal firmware, 548728 bytes; NVRAM 1748 bytes
bcm2835-sdiod: firmware written (548728 bytes in 272 ms) and read back
bcm2835-sdiod: NVRAM written at 0x0025f92c (1748 bytes)
bcm2835-sdiod: ARM started, reset vector 0xb83ef198
bcm2835-sdiod: HT clock on (CHIPCLKCSR 0xd0)
bcm2835-sdiod: function 2 ready, firmware running
bcm2835-sdiod: mailbox from the firmware 0x00040008
bcm2835-sdiod: data frame of 0 bytes
bcm2835-sdiod: FAILED
```

- **Fail**: up to `function 2 ready, firmware running` and `mailbox from
  the firmware 0x00040008` as in test 60, then `data frame of 0 bytes` and
  `FAILED`; no `MAC address`, no CLM, no radio up, no scan. In test 60
  (image 218-g7d69076) the same place logged `event frame (channel 1),
  0 bytes` and went on to the version and MAC, so the 0-byte frame that
  the firmware sends first is now taken as a data frame and ends the run.
- Second run over devsh (`/usr/lib/drivers/bcm2835-sdiod`, exit 1) and a
  run with `standard` (609312 bytes in 301 ms): the same `data frame of 0
  bytes`, `FAILED`.
- `scripts/pi-test.py` standard checks pass, 0 of 300 starts failed.

## 2026-10-29: `v0.3.0-dev-187-g0f28c70+x0042`

Built 2026-10-29T14:16:08Z, `libc.so` `bd047155ea800b72…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `serial-rx`, `sshd`, `timesync`. test 59 ([#45](https://github.com/redox-rpi/redox-rpi/issues/45) SSH server), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-187-g0f28c70+x0042 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 59 ([#45](https://github.com/redox-rpi/redox-rpi/issues/45)), branch
`sshd`. **Fail**: sshd does not start, so no SSH login could be tried.

- Boot log: no `ssh-keygen: generating new host keys`. `sshd-keys` runs
  `ssh-keygen -A -f /scheme/data`, which takes about a minute (RSA) and then
  logs `Unable to move /scheme/data/etc/ssh/ssh_host_{rsa,ecdsa,ed25519,mldsa44_ed25519}_key.pub
  into position: No such file or directory`; init: `sshd-keys failed with
  exit status: 1`; then `sshd: no hostkeys available -- exiting.`
- `/scheme/data/etc/ssh` holds all eight key files under their temporary
  names (`ssh_host_ed25519_key.XXXXz37Ax9`, …), never renamed.
- Cause: `rename` does not work on `/scheme/data`. `echo x > /scheme/data/t59c;
  mv /scheme/data/t59c /scheme/data/t59d` gives `mv: No such file or
  directory (os error 2)` (also with relative names, also in
  `/scheme/data/etc/ssh`); the same in `/tmp` works. **The stable image
  `v0.3.0-dev-15-g24fbb96+x9f76` has the same `mv` failure on
  `/scheme/data`**, so it is not new in this branch.
- `/scheme/data` was mounted; standard checks pass, 0 of 300 starts failed
  (devshd). The temporary key files were deleted afterwards.

## 2026-10-29: `v0.3.0-dev-100-g0a85e1e+xd656`

Built 2026-10-29T14:09:20Z, `libc.so` `d19d472ff4ba92c2…`, real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `mqtt`, `mqtt-demo`, `timerd`, `timesync`. test 58 ([#49](https://github.com/redox-rpi/redox-rpi/issues/49) MQTT), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-100-g0a85e1e+xd656 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 59 processes (piped) |
| ps-tty | pass | 59 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 58 ([#49](https://github.com/redox-rpi/redox-rpi/issues/49)), branch
`mqtt`; mosquitto on homenodellm (`~/mqtt-broker/pi-test-broker.sh`).

- `echo 192.168.178.120 > /etc/mqtt-host`; `mqtt-pub -t redox/pi/hello -q 1
  -m hello` exit 0, the broker has `redox/pi/hello hello`: **pass**.
- `mosquitto_pub … -t redox/cmd/x -m to-pi` while `mqtt-sub -t 'redox/cmd/#'
  -q 1 -v -C 1` runs on the Pi: prints `redox/cmd/x to-pi`, exit 0: **pass**.
- `cat /scheme/rpi-firmware/temperature`: 58533.
- **Found**: the two demo timers published nothing for 8 minutes. Their
  first `mqtt-pub` runs (PIDs 41 and 44, started at boot, before
  `/etc/mqtt-host` was set, so to the default host 10.0.2.2) hung and never
  ended, and timerd logged `mqtt-uptime: still running, skipped this run`
  (and the same for `mqtt-temperature`) every minute. `timerd -l` over devsh
  also hung then. After `kill 41 44` both timers published once a minute:
  `redox-rpi/temperature 56920`, `redox-rpi/uptime 11m 56s`, and so on;
  `timerd -l` shows both with a last run. So `mqtt-pub` needs a connect
  timeout (or the demo should wait for `/etc/mqtt-host`).
- No `RELIBC PANIC` or `exited with code 1` in the serial log. Standard
  checks pass, 0 of 300 starts failed. devsh did not answer some sessions
  in between (one `uptime` timed out).

## 2026-10-29: `v0.3.0-dev-104-g439640d+x3029`

Built 2026-10-29T14:41:39Z, `libc.so` `d19d472ff4ba92c2…`, real Pi 3B+ over netboot. Experiments: `ha-bridge`, `ldso-diagnostics`, `mqtt`, `timerd`, `timesync`. test 65 ([#49](https://github.com/redox-rpi/redox-rpi/issues/49) ha-bridge), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-104-g439640d+x3029 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 53 processes (piped) |
| ps-tty | pass | 53 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 65 ([#49](https://github.com/redox-rpi/redox-rpi/issues/49)), branch
`mqtt`, experiment `ha-bridge`; the same broker as test 58.

- The boot took longer than the others: the `redox-rpi v…` banner came 80 s
  after `network boot` (others about 22 s).
- `echo 192.168.178.120 > /etc/mqtt-host`, then nothing reached the broker
  for 2 minutes. As in test 58, the first `ha-bridge` run (PIDs 38, 42,
  started by timerd at 17:48, before the host was set) hung, and timerd
  logged `ha-bridge: still running, skipped this run` every minute. After
  `kill 38 42`:
- `redox-rpi/redox/state` once a minute, for example
  `{"cpu_temperature":58.5,"cpu_clock":1400,"throttled":"OFF","uptime":347,"memory_free":546.2,"memory_used":25.0}`:
  temperature and clock plausible, `throttled` OFF (firmware `0x00080008`:
  soft temperature limit seen before, not now). `memory_free` 546 MB
  matches `free` (548 MB free), but `memory_used` 25.0 does not match
  `free`'s 170–198 MB used.
- **Missing**: no `redox-rpi/redox/availability online` and none of the six
  `homeassistant/…/config` topics, neither live nor retained (a fresh
  `mosquitto_sub -t 'homeassistant/#'` got nothing). After `mosquitto_pub -t
  homeassistant/status -m online` the configs did not come either.
- **Missing**: after `echo reset > /scheme/sys/kstop` no `availability
  offline` (last will) within 2 minutes.
- Standard checks pass, 0 of 300 starts failed.
- Probably the same cause as test 58: discovery and the last will are sent
  only by the first run, which hung before `/etc/mqtt-host` was set.
  Afterwards the broker was stopped and the stable image restored.

## 2026-10-29: `v0.3.0-dev-98-g723b343+x7f63`

Built 2026-10-29T14:48:42Z, `libc.so` `100761987443777d…`, real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `timerd`, `timesync`. test 66 ([#51](https://github.com/redox-rpi/redox-rpi/issues/51) clock_settime and timesync -s), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-98-g723b343+x7f63 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 47 processes (piped) |
| ps-tty | pass | 47 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 66 ([#51](https://github.com/redox-rpi/redox-rpi/issues/51)), branch
`clock-settime` (relibc `0008`, `timesync -s`). All **pass**:

- Serial log: `timesync: clock set to 2026-10-29 18:00:18 UTC from
  pool.ntp.org` (after the saved time).
- `timesync -s "2030-01-02 03:04:05"`: `clock set to 2030-01-02 03:04:05 UTC
  by hand`, exit 0; `date -u` `Wed Jan  2 03:04:05 UTC 2030`; `touch /tmp/x`
  gives `Jan  2 03:04`.
- `timesync -s '@1793282730'`: `2026-10-29 14:05:30 UTC by hand`, exit 0.
- `timesync`: first `from the build time` (the clock was before the build
  time), then `2026-10-29 18:02:05 UTC from pool.ntp.org`, exit 0; `date -u`
  18:02:06, the same second as homenodellm.
- `timesync -s 2030`: `-s wants 'YYYY-MM-DD HH:MM[:SS]' (UTC) or
  @<seconds>, not "2030"`, exit 2.
- `/scheme/data/timesync.last` afterwards holds the NTP time (18:02), only
  its file date is 2 Jan 2030 from the test; the stable image booted with
  the right time from it.
- Standard checks pass, 0 of 300 starts failed (every program with the
  changed C library).

## 2026-10-29: `v0.3.0-dev-104-g3f88563+x508a`

Built 2026-10-28T20:36:54Z, `libc.so` `f97ca1fa2be411ab…`, real Pi 3B+ over netboot. Experiments: `icache-sync`, `rtl8152`, `timesync`. test 28 ([#3](https://github.com/redox-rpi/redox-rpi/issues/3) icache sync), ARM at 1400 MHz, Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-104-g3f88563+x508a |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 53 processes (piped) |
| ps-tty | pass | 53 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 1000 failed |

Test 28 ([#3](https://github.com/redox-rpi/redox-rpi/issues/3)), branch
`t3-icache`, experiment `icache-sync`.

- `scripts/pi-test.py --starts 1000`: **0 of 1000 failed**, no exception in
  the serial log (only the known `pcid` panic).
- Not at 600 MHz as the row asks: this image raises the ARM clock at boot
  (`rpi-firmwared: ARM clock 600 -> 1400 MHz`, `clocks`: `arm 1400000000`),
  and without `cpu-ondemand` there is no `arm_clock` file to set 600 MHz.
- Start loop on the Pi, `time ion -c "for i in 1..=500; ls /usr > /dev/null; end"`,
  two runs each:

  | Image | ARM | 500 starts |
  |---|---|---|
  | this image (`icache-sync`) | 1400 MHz | 88.5 s, 90.6 s |
  | stable `v0.3.0-dev-15-g24fbb96+x9f76` | 600 MHz | 162.5 s, 162.2 s |
  | test 37 image `v0.3.0-dev-140-g55bd363+x68b9` (no `icache-sync`) | 1400 MHz | 127.0 s, 136.1 s |

  None of these is the same build without the patch, so the cost of the
  cache maintenance itself cannot be read from this: the test 37 image at
  the same clock is even slower than this one. A like-for-like number needs
  the `t3-icache` base built once without `icache-sync`.

## 2026-10-29: `v0.3.0-dev-93-g2d1a2e4+xc99c`

Built 2026-10-28T20:09:08Z, `libc.so` `e42257f7a7bb4c47…`, real Pi 3B+ over netboot. Experiments: `cpu-ondemand`, `rtl8152`, `timesync`. test 25 ([#23](https://github.com/redox-rpi/redox-rpi/issues/23) CPU clock follows the load), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-93-g2d1a2e4+xc99c |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 53 processes (piped) |
| ps-tty | pass | 53 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 25 ([#23](https://github.com/redox-rpi/redox-rpi/issues/23)), branch `cpu-ondemand`. Boot log: `rpi-firmwared: ARM clock 600 -> 1400 MHz`. `ondemand-check.py <repo> <pi> 2 30` (from `cpu-ondemand`), first lines:

```
idle: mode ondemand target max load 100 switches 16 min 600000000 max 1400000000 1400000000
ondemand: 50 starts in 8.1 s, 0 failed; now mode ondemand target max load 100 switches 18 min 600000000 max 1400000000 1400000000
performance: 50 starts in 8.3 s, 0 failed; now mode performance target max load 61 switches 18 min 600000000 max 1400000000 1400000000
powersave: 50 starts in 16.0 s, 0 failed; now mode powersave target min load 100 switches 19 min 600000000 max 1400000000 600000000
ondemand: 50 starts in 8.1 s, 0 failed; now mode ondemand target max load 83 switches 22 min 600000000 max 1400000000 1400000000
performance: 50 starts in 11.7 s, 0 failed; now mode performance target max load 97 switches 22 min 600000000 max 1400000000 1400000000
powersave: 50 starts in 16.0 s, 0 failed; now mode powersave target min load 92 switches 23 min 600000000 max 1400000000 600000000
after timing: mode ondemand target max load 81 switches 26 min 600000000 max 1400000000 1400000000
soak: 0 of 50 starts failed; mode ondemand target max load 99 switches 30 min 600000000 max 1400000000 1400000000
…
soak: 0 of 6500 starts failed; mode ondemand target max load 100 switches 354 min 600000000 max 1400000000 1400000000
end: mode ondemand target max load 100 switches 354 min 600000000 max 1400000000 1400000000 ['59610', '0x00080008']
```

- Timed 50 starts: ondemand 8.1 and 8.1 s, performance 8.3 and 11.7 s,
  powersave 16.0 and 16.0 s (600 MHz): **pass**.
- Soak, 30 minutes of rounds of 50 starts with 3 s pauses: **0 of 6500
  starts failed**, switches grew from 30 to 354: **pass**. The serial log
  has `rpi-firmwared: no longer throttled` 94 times (soft temperature
  limit, about 60 °C, `throttled 0x00080008` at the end) and no exception
  except the known `pcid` panic.
- **Fail: idle does not go to the minimum.** The first `idle` line has
  `target max load 100`, 1400 MHz. Checked by hand after the soak: after
  `sleep 10` and `sleep 30` with nothing else running, still `target max`,
  load 72–100, 1400 MHz, and 30 more switches during the 30 s. `ps`: the
  initfs `redoxfs` (PID 20) had 14 min 11 s CPU time (`netstack` 1 min,
  `rpi-firmwared` 9 s), so something keeps a CPU busy and the governor
  sees load. 
- Standard checks pass, 0 of 300 starts failed.
