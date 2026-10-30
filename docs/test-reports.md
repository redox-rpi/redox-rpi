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

## 2026-10-29: `v0.3.0-dev-189-gd5ac445+x02a9`

Built 2026-10-29T14:17:39Z, `libc.so` `e42257f7a7bb4c47…`, real Pi 3B+ over netboot. Experiments: `hwinfo`, `timesync`, `usb-hotplug`. test 32 ([#35](https://github.com/redox-rpi/redox-rpi/issues/35) lsusb and hwinfo), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-189-gd5ac445+x02a9 |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 43 processes (piped) |
| ps-tty | pass | 43 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 32 ([#35](https://github.com/redox-rpi/redox-rpi/issues/35)), branch `hwinfo`. Output over devsh:

```
### lsusb  (0.3 s)
Bus 001 Device 001: ID 0424:2514 Microchip USB2514 hub (USB 2.00, high speed)
Bus 001 Device 002: ID 0424:2514 Microchip USB2514 hub (USB 2.00, high speed)
Bus 001 Device 003: ID 0424:7800 Microchip LAN7800 Gigabit Ethernet (Pi 3B+) (USB 2.10, high speed)
Bus 001 Device 004: ID 05e3:0610 hub (USB 2.00, high speed)
Bus 001 Device 005: ID 04d9:0006 keyboard, HID (USB 2.00, low speed)
### lsusb -t  (0.3 s)
root port: address 1, 0424:2514 Microchip USB2514 hub, high speed
port 1: address 2, 0424:2514 Microchip USB2514 hub, high speed
port 1: address 3, 0424:7800 Microchip LAN7800 Gigabit Ethernet (Pi 3B+), high speed
port 2: address 4, 05e3:0610 hub, high speed
port 1: address 5, 04d9:0006 keyboard, HID, low speed
### hwinfo  (5.1 s)
build        v0.3.0-dev-189-gd5ac445+x02a9
experiments  hwinfo timesync usb-hotplug
model        Raspberry Pi 3 Model B+
revision     0x00a020d3
serial       00000000ed08e0aa
memory       1 GiB
ARM memory   998 MiB
GPU memory   16 MiB
firmware     0x6aa809d0
CPUs         4
ARM clock    1400 MHz (max 1400 MHz)
clocks       emmc 200 MHz, uart 48 MHz, core 250 MHz, v3d 250 MHz, h264 250 MHz, isp 250 MHz, sdram 400 MHz
temperature  56.9 °C
throttling   0x00000000 (none now)
USB          5 devices (lsusb for the list)
```

- `lsusb`, `lsusb -t`, `lsusb -v`: both 3B+ hubs `0424:2514`, the LAN7800
  `0424:7800`, plus a `05e3:0610` hub and a `04d9:0006` keyboard; the tree
  matches dwc2d's boot log; `-v` lists the interfaces: **pass**.
- `hwinfo`: Raspberry Pi 3 Model B+, 1 GiB (998 MiB ARM, 16 MiB GPU), 4
  CPUs, ARM 1400 MHz, the other clocks the same as `/scheme/rpi-firmware/clocks`,
  56.9 °C (`temperature` a moment later 55843), throttling none: **pass**.
- Standard checks pass, 0 of 300 starts failed. The optional plug test
  with a person was not done.

## 2026-10-29: `v0.3.0-dev-228-g5d21b70+x29ce`

Built 2026-10-29T15:06:10Z, `libc.so` `447cc92099c9a1d9…`, real Pi 3B+ over netboot. Experiments: `relibc-tests`, `rtl8152`, `sigentry-thread-signals`, `sigregs-test`, `sigsegv`, `timesync`. test 68 ([#13](https://github.com/redox-rpi/redox-rpi/issues/13) sigsegv and relibc tests), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-228-g5d21b70+x29ce |
| manifest | pass | build ID and libc.so hash match manifest.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 68 ([#13](https://github.com/redox-rpi/redox-rpi/issues/13)), branch
`t-sigsegv-pi` (`sigsegv` + `sigentry-thread-signals`).

- `sigregs sender` (si_pid 371 from the thread, 372 from the child),
  `sigregs thread -n 3000`, `sigregs process -n 3000`: all **pass**.
- `ion /usr/share/relibc-tests/run.ion` over devsh: finished
  (`RELIBC-TESTS rc=1`), output saved on homenodellm in
  `~/pitest-tools/t68-relibc-tests.txt`. `scripts/relibc-tests-compare.py`
  from `t-sigsegv-pi` with `config/relibc-tests-expected.txt`:
  `SUMMARY v0.3.0-dev-228-g5d21b70+x29ce: 319 tests, 11 failed (11 known,
  0 flaky, 0 new), 14 fixed`, exit 0: **pass**.
  - FIXED: `netdb/getaddrinfo`, `pthread/rwlock_randtest`, `sigaction`,
    **`sigaltstack`**, `signals/sigpause-revert`, `signals/sigprocmask-3`,
    **`signals/sigsegv`**, `signals/sigset-5`, **`signals/stackoverflow`**,
    `sys_select/pselect`, `sys_socket/getpeername`, `sys_socket/getsockopt`,
    `sys_socket/recv`, `sys_socket/recvfrom` (QEMU: 8 fixed; on the Pi the
    socket and netdb tests pass too).
  - Known failures: `stdio/fscanf_offby1`, `stdio/mutex`, `stdio/popen`,
    `stdio/ungetc_ftell`, `stdlib/system`, `time/timer`, `tls_align`
    (dynamic and static), `tls_shared`, `unistd/exec`.
- Serial log: no `UNHANDLED EXCEPTION` except the known `pcid` panic at
  boot (the exceptions the tests cause on purpose now arrive as signals).
- Standard checks pass, 0 of 300 starts failed.

## 2026-10-29: `v0.3.0-dev-217-g1d028a4+x19b9`

Built 2026-10-29T15:07:19Z, `libc.so` `8eb8f49492564a29…`, real Pi 3B+ over netboot. Experiments: `boot-time`, `rtl8152`, `timesync`. test 69 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47) boot time, second round), Pi test agent

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-217-g1d028a4+x19b9 |
| manifest | pass | present in the image |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

Test 69 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47)), branch
`boot-time` 1d028a4, image `~/t18-work/image-bt5/`, staged with
`~/t18-work/bt5-stage/scripts/netboot.sh` (boot-time `netboot.cmd`, the
image's own bootloader with the 1 s wait, md5 `ce0d8b09…`). Three boots,
timelines in `~/t18-work/tl-pi-bt5-{1,2,3}.txt` on homenodellm:

| Seconds | boot 1 (devsh reset) | boot 2 (serial reset) | boot 3 (devsh reset) |
|---|---|---|---|
| `kreset` | about 1.6 | 5.19 | 1.56 |
| kernel starts | 19.52 | 22.74 | 19.07 |
| `switchroot to /usr` | 22.38 | 26.47 | 21.98 |
| `redox login:` | **23.34** | **27.32** | **22.89** |
| `redox login:` after `kreset` | about 21.7 | 22.1 | 21.3 |
| `40_dwc2d.service ready after` | 6283 ms | 6252 ms | 6199 ms |
| network | **none**: `dhcpd: Can't open /scheme/netcfg/ifaces/eth0/mac` | yes | yes |

- `init: switchroot to /usr` now comes before `40_dwc2d.service ready`
  (dwc2d is ready about 4 s after the login prompt, at init 6.5–7.3 s):
  **pass**. Login about 6.5 s earlier than test 53's boot-time image
  (29.05–29.51 s from the reset, about 27.3 s after `kreset`).
- **Fail**: in boot 1 `dhcpd` started before dwc2d had made `eth0` and gave
  up (`Can't open /scheme/netcfg/ifaces/eth0/mac`), so the Pi had no network
  and devsh never answered (reset over the serial console). Boots 2 and 3
  got their address. So `dhcpd` (and what needs the network) must still
  wait for the adapter, or retry.
- First devsh answer: not measured exactly; in boots 2 and 3 devsh answered
  within a minute of the reset.
- Standard checks pass (boot 3), 0 of 300 starts failed. Other units of
  100 ms or more: `40_rpi-firmwared` 852 ms, `40_hwd` about 445 ms,
  `40_bcm2835-sdhcid` about 445 ms, `00_ptyd` about 158 ms, `60_data`
  413–433 ms.

## 2026-10-30: `v0.3.0-dev-261-gaca5de2+x368f`

Real Pi 3B+ over netboot. Experiment: `netstack-multi`. test 74 ([#53](https://github.com/redox-rpi/redox-rpi/issues/53) netstack with several adapters), Pi test agent

**Pass.** Branch `netstack-multi`,
image `~/redox-rpi-zrtl/keep/nsm-v0.3.0-dev-261-gaca5de2+x368f.img`, staged
with stable's bootloader and reset over the serial console.

- Boot: `DHCP: eth0 got 192.168.178.148/24` and
  `DHCP: default route via 192.168.178.1 (eth0)`: **pass**.
- Before testnetd: eth0 `192.168.178.148/24`; routes `default via
  192.168.178.1 dev eth0`, `127.0.0.0/8 dev loopback`, `192.168.178.0/24 dev eth0`.
- `/usr/lib/drivers/testnetd &` over devsh: within a few seconds the serial log
  has `DHCP: eth1 got 10.0.9.2/24` (the `network.test0 registered` line went to
  devsh, not to the serial log). eth1 addr `10.0.9.2/24`; routes still have
  `default via 192.168.178.1 dev eth0` plus `10.0.9.0/24 dev eth1`: **pass**.
- `ping -c 3 10.0.9.1`: 3/3, about 3 ms. `ping -c 3 192.168.178.1`: 3/3,
  0.8–1.7 ms: **pass**.
- `/tmp/testnetd.log`: `dhcp 2 dhcp_foreign 0 foreign_src 0 ping 3`: no eth0
  traffic leaked onto eth1.
- devsh answered throughout. Standard checks pass before and after testnetd
  (0 of 300 starts failed each time).
- Every `DHCP:` and `smolnetd` line of the serial log (there were no
  `smolnetd` lines; the two `DHCP client bound` lines are U-Boot's netboot):

```
DHCP client bound to address 192.168.178.148 (6 ms)
DHCP client bound to address 192.168.178.148 (2 ms)
DHCP: eth0 got 192.168.178.148/24
DHCP: default route via 192.168.178.1 (eth0)
DHCP: eth1 got 10.0.9.2/24
```

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-218-g8deec9e+xbe97`

Real Pi 3B+ over netboot. Experiments: `sdhost-dma`, `timesync`. test 75 ([#21](https://github.com/redox-rpi/redox-rpi/issues/21) sdhost with DMA, second try: wait for the DMA interrupt), Pi test agent

**Data safe, but the CPU time did not go down.** Image
`~/redox-rpi-sd/keep/netboot-v0.3.0-dev-218-g8deec9e+xbe97.img`, manifest
matches.

- Boot log (all 4 boots): `bcm2835-sdhostd: DMA interrupt
  /scheme/irq/phandle-1/1,20`, `DMA channel 4 at 0x3f007000`, `SDHC/SDXC card,
  15193 MiB, 4-bit bus, high speed, up to 50 MHz`; no `cannot use DMA
  interrupt`, `DMA error`, `DMA timeout` or `no DMA`: **pass**.
- Standard checks pass, manifest matches, 0 of 300 starts failed.

256 MiB on `/scheme/data`, `bs=1048576`, from `/dev/urandom` (it makes 256 MiB
in 4.0 s, 68 MB/s). `ps` shows the driver as `/scheme/initfs/bin/bcm2835-sdhos`;
both time columns are given:

| | time | MB/s | ACTTIME of `bcm2835-sdhostd` | SCHTIME |
|---|---|---|---|---|
| write `conv=fsync` | 35.0 s | 7.7 | +28.9 s (53.53 → 82.39) | +4.3 s (13.60 → 17.94) |
| read | 15.3 s | 18 | +14.0 s (82.39 → 96.43) | +1.1 s (17.94 → 19.06) |
| read with `sha256sum` after the reset | 21.5 s | | +10.2 s (0.08 → 10.31) | +2.7 s |
| test 42 write (polling) | 30.3 s | 8.8 | +23.7 s | |
| test 42 read (polling) | 15.1 s | 18 | +13.7 s | |

- The ACTTIME column (the one test 42 used) is still about 82 % of the write
  time and 92 % of the read time, the same as with polling. If SCHTIME is the
  real CPU time, it is 12 % and 7 %. I can't tell from here which column counts
  waiting for the interrupt; the driver session knows.
- Speed is the same as test 42.
- An earlier run from `/dev/zero` wrote 256 MiB in 2.9 s (92 MB/s) and read it
  in 0.3 s. RedoxFS probably does not store blocks of zeros, so that run is not
  a disk measurement.
- SHA-256 of a 256 MiB random file before and after `echo reset >
  /scheme/sys/kstop`: `42bf6207…1c07b97` both times: **pass**.
- `scripts/sd-stress.py --cycles 2 --serial-log ~/redox-rpi/logs/serial.log`:
  write 7.8 MB/s, 1001 files checked after writing and 1001 and 1002 after
  restarts 1 and 2, 0 missing, 0 changed. `/scheme/data` was there 0 s and
  11 s after the login prompt (cycle writes 7.1 and 8.8 MB/s): **pass**.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-110-g9f47c94+x125f`

Real Pi 3B+ over netboot. Experiments: `ha-bridge`, `ldso-diagnostics`, `mqtt`, `mqtt-tls-test`, `timerd`, `timesync`. test 76 ([#56](https://github.com/redox-rpi/redox-rpi/issues/56) MQTT over TLS), Pi test agent

**TLS works; fails on the ha-bridge start and a memory loss.** Broker
`~/mqtt-broker/pi-test-broker-tls.sh` (TLS 192.168.178.120:8883, plain :1883);
capture saved as `~/pitest-tools/broker76.out` on homenodellm.

- Standard checks pass, manifest matches, 0 of 300 starts failed.
- `date -u`: `Fri Oct 30 07:10:12 UTC 2026` (after the CA date).
  `head -1 /etc/mqtt-ca.crt`: `-----BEGIN CERTIFICATE-----`. Then
  `echo 192.168.178.120 > /etc/mqtt-host` at 07:10:24.
- **Fail:** no `ha-bridge: 6 sensors announced …` within 30 s, and no
  availability or discovery. The image starts ha-bridge only from timerd, every
  minute, without `-d` (`/usr/lib/timer.d/ha-bridge.toml`, `cmd = "ha-bridge"`).
  Its first run started at 07:08, before `/etc/mqtt-host` existed (so it tried
  `localhost`), and hung like in tests 58 and 65 (`still running, skipped this
  run`, two `ha-bridge` processes). After `kill`, the next runs worked over TLS:
  `redox-rpi/redox/state` once a minute from 07:12, each run 0.3 s.
- `ha-bridge -d &` started by hand at 07:20:38: `redox-rpi/redox/availability
  online` and all six `homeassistant/…/config` topics at once: **pass**.
- `mqtt-pub -t tls/pi -m hello -q 1`: exit 0, `tls/pi hello` arrived.
  `mqtt-pub -t tls/pi -m again -r`, then `mqtt-sub -t tls/pi -C 1 -v`:
  `tls/pi again`. `mqtt-pub --no-tls -t plain/pi -m x`: exit 0, arrived.
  `mqtt-pub --tls --cafile /etc/hostname …`: `/etc/hostname: no certificates`,
  exit 1. All **pass**.
- `time mqtt-pub -t tls/pi -m t`: **0.09 s** (QEMU 0.5–2 s).
- Reset through `echo reset > /scheme/sys/kstop` (with `ha-bridge -d`
  running): `redox-rpi/redox/availability offline` at 07:21:39, about 35 s
  after the reset: **pass**. Retained `tls/pi` cleared, broker stopped.
- **Memory loss:** between 07:12 and 07:21 free memory went from 534 MB
  (ha-bridge's `memory_free`) to 56 MB (`free`), about 15 MB a minute. It
  still went on after the hung ha-bridge was killed. No process grew (all
  `PRIVATE` under 40 MB). Over the same time the Pi stopped answering ping
  (devsh still worked). On the serial console, `cat
  /scheme/netcfg/ifaces/eth0/addr/list` took about 3 minutes, and `netstack`
  had used 3 min 20 s of CPU by 07:17 (it stayed there afterwards). `dwc2d`
  showed ACTTIME 9 min 41 s, SCHTIME 0.34 s. This build has no `ctx-leak-fix`,
  so it may be that leak. The reset cleared it before it ran out.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-223-g7a3641a+xd4e7`

Real Pi 3B+ over netboot. Experiments: `ctx-leak-fix`, `rtl8152`, `timesync`, `wifi-sdio`. test 77 ([#26](https://github.com/redox-rpi/redox-rpi/issues/26) Wi-Fi steps 3–4 again, rerun of test 63), Pi test agent

**Fail: the CLM file is missing from the image.** Image
`~/redox-rpi-zrtl/keep/wifiscan2-v0.3.0-dev-223-g7a3641a+xd4e7.img`.

- The 0-byte frame fix works: where test 63 stopped at `data frame of 0
  bytes`, the driver now reads `firmware version: wl0: Nov  1 2021 00:37:12
  version 7.45.241 (1a2f2fa CY) FWID 01-5209c09` and `MAC address
  b8:27:eb:5d:b5:ff`. Everything test 60 checks passes.
- Then: `bcm2835-sdiod: /usr/lib/firmware/brcm/cyfmac43455-sdio.clm_blob: No
  such file or directory (os error 2)` and `bcm2835-sdiod: FAILED`. So there is
  no `CLM loaded`, `radio up` or scan. `/usr/lib/firmware/brcm/` has only
  `LICENCE.cypress`, `brcmfmac43455-sdio.raspberrypi,3-model-b-plus.txt`,
  `cyfmac43455-sdio-minimal.bin` and `cyfmac43455-sdio-standard.bin`.
- Second run over devsh: same lines, same `FAILED` (firmware written in
  272 ms, 293 ms at boot).
- `wifi_power`: **still fails**. `echo 0 > /scheme/rpi-firmware/wifi_power;
  echo $?` gives `I/O error (os error 5)` and 1, the same for `echo 1`. `cat`
  shows the value written (0, then 1). The driver logs the same I/O errors.
- Standard checks pass, 0 of 300 starts failed.

Boot `bcm2835-sdiod:` lines (core and CMD lines as in test 63):

```
bcm2835-sdiod: wifi_power was 0
bcm2835-sdiod: wifi_power 0: I/O error (os error 5)
bcm2835-sdiod: wifi_power 1: I/O error (os error 5)
bcm2835-sdiod: CMD5 R4 0x30ffff00, 3 functions
bcm2835-sdiod: CMD52 chipid 0x15264345: chip 0x4345 rev 6 cores 5
bcm2835-sdiod: chip ID OK
bcm2835-sdiod: RAM 0x00198000, 800 KiB
bcm2835-sdiod: minimal firmware, 548728 bytes; NVRAM 1748 bytes
bcm2835-sdiod: firmware written (548728 bytes in 293 ms) and read back
bcm2835-sdiod: NVRAM written at 0x0025f92c (1748 bytes)
bcm2835-sdiod: ARM started, reset vector 0xb83ef198
bcm2835-sdiod: HT clock on (CHIPCLKCSR 0xd0)
bcm2835-sdiod: function 2 ready, firmware running
bcm2835-sdiod: mailbox from the firmware 0x00040008
bcm2835-sdiod: firmware version: wl0: Nov  1 2021 00:37:12 version 7.45.241 (1a2f2fa CY) FWID 01-5209c09
bcm2835-sdiod: mailbox from the firmware 0x00040002
bcm2835-sdiod: MAC address b8:27:eb:5d:b5:ff
bcm2835-sdiod: /usr/lib/firmware/brcm/cyfmac43455-sdio.clm_blob: No such file or directory (os error 2)
bcm2835-sdiod: FAILED
```

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-151-g13c2757+x7d9f`

Real Pi 3B+ over netboot. Experiments: `gpio`, `bluetooth`. test 79 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54) Bluetooth over the mini UART), Pi test agent

**Fail: Bluetooth power cannot be switched, so the controller never answers.**
Image `~/redox-rpi-cpu/keep/v0.3.0-dev-151-g13c2757+x7d9f/harddrive.img`.

- Standard checks pass, 0 of 300 starts failed.
- Boot: `bcm2835-btuartd: mini UART at 0x3f215000, 115313 baud (core 250 MHz)`,
  no `a 1 ms sleep did not come back`, no `received bytes lost`.
- `cat /scheme/rpi-firmware/bt_power`: `0`.
- `bt up`: `power: cannot switch it (I/O error (os error 5)), going on`,
  `uart: core clock 250000000 Hz, 115313 baud`, `bt: no answer to HCI command
  0c03` (2.1 s). `bt_power` stays `0`.
- `bt info`, `bt scan 10`, `bt inquiry 10`: `no answer to HCI command` 1001,
  0c01 and 0401. `bt cmd 0c03` once more: no answer. `throttled`:
  `0x00000000`.
- `echo 1 > /scheme/rpi-firmware/bt_power`: `I/O error (os error 5)`, exit 1
  (`cat` then shows 1). The serial log has one `rpi-firmwared: tag 0x00038041:
  not supported by the firmware` per try. 0x00038041 is SET_GPIO_STATE (the
  expander GPIO for BT and Wi-Fi power). Test 77's `wifi_power` I/O error is
  the same problem.
- Possible cause: `/srv/tftp/config.txt` has `gpu_mem=16`, so the Pi boots the
  cut-down `start_cd.elf`/`fixup_cd.dat`. `start.elf` is from Sep 14 2026.
  Whether `start_cd.elf` lacks this tag needs a check with `gpu_mem` raised (a
  change on the shared TFTP server, so not done here).

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-236-gd47707f+x810f`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152`, `pty-sigint`. test 80 ([#55](https://github.com/redox-rpi/redox-rpi/issues/55) Ctrl-C while a program starts), Pi test agent

**Pass over devsh; the serial console check is late.** Image
`~/redox-rpi-relibc/keep/netboot-v0.3.0-dev-236-gd47707f+x810f.img`.

- Standard checks pass, 0 of 300 starts failed.
- `patches/experiments/pty-sigint/ctrlc-test.py 192.168.178.148 --rounds 20`
  (from `origin/pty-sigint`): rounds 1–10 Ctrl-C after 1 s, rounds 11–20
  after 3 s, all `stopped by SIGINT`. `Ctrl-C at the prompt: shell still
  answers`. `PASS: 20 of 20 stopped`.
- `ttyinfo` over devsh: `pgrp = 482`, `tcgetpgrp(0) = 482`, `ISIG=true`,
  `VINTR = 0x3`, `SIGINT = default, blocked=false`: **pass**.
- Serial console, logged in as root: `sleep 30`, Enter, Ctrl-C right after
  it. Ion printed `ion: pipeline execution error: process (484) ended by signal
  SIGINT` only **after about 32 s**, so `sleep` ran its full 30 s and the
  signal stopped it only at the end. Over devsh the same case passes.
- Serial log: the only exceptions are pcid's
  (`Pci::CfgAccess::read on this architecture`), the same on every image and
  on stable. Nothing else.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-305-g2d679a2+x4ec1`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152`, `sshd`. test 81 ([#45](https://github.com/redox-rpi/redox-rpi/issues/45) SSH server, rerun of test 59 with the host key fix), Pi test agent

**Pass.** Image `~/redox-rpi-relibc/keep/netboot-v0.3.0-dev-305-g2d679a2+x4ec1.img`.

- Boot log: `ssh-keygen: generating new host keys: RSA ECDSA ED25519
  MLDSA44-ED25519`, then sshd starts. No `Unable to move`, no `no hostkeys
  available`. Only the expected `TODO: madvise(…)` lines.
- `ls -l /scheme/data/etc/ssh`: the 8 files `ssh_host_{rsa,ecdsa,ed25519,
  mldsa44_ed25519}_key` (`-rw-------`) and `.pub` (`-rw-r--r--`), no leftover
  temporary files: **pass**.
- Standard checks pass, 0 of 300 starts failed.
- Test key put into `/home/root/.ssh/authorized_keys` over devsh. `ssh
  root@192.168.178.148 'echo OK; id; uname -a'`: `OK`, `uid=0(root)
  gid=0(root)`, `Redox redox 0.5.12 … aarch64 Redox`, **1.3 s** including the
  login: **pass**.
- `ssh-session.exp` (host and port set to the Pi, `-p 22`, key file): prompt,
  command, prompt, backspace, prompt and `exit` all ok, exit at once: **pass**.
- Host keys: RSA `SHA256:nggT0H31…`, ECDSA `SHA256:MXb/OY+z…`, ED25519
  `SHA256:6b9SpKQH…`. After `echo reset > /scheme/sys/kstop` they are the
  same, and that boot log has no `generating new host keys`: **pass**.
- Notes:
  - `ssh-keyscan` asked for several key types in one call returned only RSA.
    One type per call gives all three.
  - After the reset, key login is refused, because `authorized_keys` lives in
    `/home/root` on the root file system, which is in RAM on a network boot.
    That is expected.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-311-g9dcb81b+xd71d`

Real Pi 3B+ over netboot. Experiments: `timesync`, `redoxfs-rename`. test 83 ([#57](https://github.com/redox-rpi/redox-rpi/issues/57) `rename` on `/scheme/data`), Pi test agent

**Pass.** Image `~/redox-rpi-sd/keep/netboot-v0.3.0-dev-311-g9dcb81b+xd71d.img`,
manifest matches.

- `echo x > /scheme/data/t83a; mv /scheme/data/t83a /scheme/data/t83b`:
  exit 0. `ls /scheme/data` shows `t83b`, no `t83a`.
- Relative names in `/scheme/data` (`mv t83c t83d`): exit 0, `t83d` there.
- `mv /scheme/data/t83d /scheme/data/t83dir/` into a new directory: exit 0.
  `cat /scheme/data/t83dir/t83d` gives `y`.
- `mv /tmp/x /tmp/y`: exit 0, content kept.
- `rm -r /scheme/data/t83b /scheme/data/t83dir`: exit 0, both gone.
- Standard checks pass, 0 of 300 starts failed.
- `/scheme/data` also has `etc` (test 81's SSH host keys) and older test
  files, left as they are.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-93-g2d1a2e4+xc99c` and `v0.3.0-dev-317-g40be428+xe828`

Real Pi 3B+ over netboot. test 82 ([#23](https://github.com/redox-rpi/redox-rpi/issues/23) why the Pi never looks idle, follow-up to test 25), Pi test agent. Raw output on homenodellm in `~/pitest-tools/t82/` (`s82-a.txt`, `s82-b.txt`, `c82.txt`).

**Found: on the `cpu-ondemand` image `dwc2d` holds CPU 2 the whole time. On
current main (the `cputop` image) all four CPUs are idle.**

### First image, `cpu-ondemand` (same as test 25)

No soak or start loop before. Samples 60 s after `redox login:`, 30 s apart:

| | cpu0 busy | cpu1 | cpu2 | cpu3 | governor |
|---|---|---|---|---|---|
| idle, after boot | 93 ms | 546 ms | **10 ms busy, 0 ms idle** | 0 ms | target `max`, load 100 |
| idle, 60 s after the start loop | 91 ms | 3 ms | **0 ms busy, 0 ms idle** | 597 ms | target `max`, load 100 |

The other CPUs gained about 31,000 ms of idle time each over 30 s. CPU 2
gained neither busy nor idle time.

`/scheme/sys/context`: `dwc2d` (pid 14) is `UR+` (running) on `#2` in all
four samples. Its ACTTIME stays at **5 min 22.39 s** (SCHTIME 0.26 s), never
changing. It was taken about 5 min 20 s after the reset, so `dwc2d` has had
CPU 2 since boot and is never switched out. That is why the counters don't
move, and why the governor sees load 100 and keeps `max`.

Top threads by ACTTIME growth over the 30 s (both samples alike, `dwc2d`
can't be measured):

| thread | after boot | after the start loop |
|---|---|---|
| initfs `redoxfs` | +0.44 s | +0.44 s |
| `netstack` | +0.16 s | +0.22 s |
| `rpi-firmwared` (thread 2) | +0.10 s | +0.11 s |
| `ptyd` | +0.03 s | +0.04 s |
| `devshd` / `procmgr` | +0.01 s | +0.01 s |

Standard checks pass between the samples, 0 of 300 starts failed.

### Second image, `cputop` (main + `cputop`, no governor)

| `cputop` | cpu0 | cpu1 | cpu2 | cpu3 | busiest thread |
|---|---|---|---|---|---|
| 30 s, 60 s after boot | 0 % | 0 % | 0 % | 0 % (2 ms) | `dwc2d` 2.2 %, then everything 0.0 % |
| 30 s, 60 s after the start loop | 0 % (0 ms) | 0 % (2 ms) | 0 % | 0 % | `dwc2d` 2.2 %, `netstack` 0.1 % |
| `-n 3 -l 5 10` (3 × 10 s) | 0 % | 0 % | 0 % | 0 % | `dwc2d` 2.6 / 1.8 / 2.2 % |

- Every CPU idles (each gained about 28–30 s of idle in 30 s, 100–290 switches/s).
  `dwc2d` is blocked (`UB`) and moves between CPUs.
- The initfs `redoxfs` TOTAL went from 0.8 s to **33.6 s** during the start
  loop, then stayed at 0.0 %. So its 14 minutes in test 25 come from the
  6500-start soak, not from idling.
- Standard checks pass, 0 of 300 starts failed.

So the always-busy CPU comes from the `dwc2d` of the old `cpu-ondemand` base
(v0.3.0-dev-93). The current `dwc2d` (tests 53 and 67) sleeps. Rebasing
`cpu-ondemand` on main should let the governor go down to `min`.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-193-ge4713b6+xb43c`

Real Pi 3B+ over netboot. Experiments: `timesync`, `pkg-hold`, `pkg-repo`. test 73 ([#44](https://github.com/redox-rpi/redox-rpi/issues/44) our package repository and `pkg update` that replaces files), Pi test agent

**Pass.** Image `~/redox-rpi-boot/keep/repoC-harddrive.img`. `/etc/pkg.d`:
`40_redox-rpi` = `http://192.168.178.120:8080/pkg`, `50_redox` =
`https://static.redox-os.org/pkg`.

| file | before | after `pkg update --yes` |
|---|---|---|
| `/usr/lib/boot/kernel` | `21af7677…66158` | same |
| `/usr/lib/libc.so` | `da6936b2…08d57` | same |
| `/usr/bin/pkg` | `265a1682…153f6` | same |
| `/usr/bin/reboot` | `e0bf9d74…08a56` | **`ff39315b…6e4d6`** |

- `reboot --version`: `reboot 0.1.1` before, **`reboot 0.1.2`** after.
- `pkg update --yes` (1.1 s): `Packages to update: ~ reboot`, 194 KB, `Commit
  complete.` No `Some transactions are ignored`. pkg doesn't print which
  repository a package came from. Only ours has reboot 0.1.2
  (`/srv/tftp/pkg/aarch64-unknown-redox/reboot.pkgar`).
- `pkg install --yes nano` (32 s, from upstream): `+ nano + ncursesw +
  terminfo`, `Commit complete.`, `nano --version` gives `GNU nano, version
  9.2`. Here pkg did print `Some transactions are ignored: usr/bin/clear, usr/bin/reset by
  …static.redox-os.org_ncursesw.pkgar (already exists)`, from upstream ncursesw
  and not from the update.
- The `setsockopt(…, 6, 1, …) - unknown option` TODO lines (TCP_NODELAY) are
  harmless.
- Standard checks after the update and install pass, 0 of 300 starts failed.
  I didn't run a separate standard check before the update; the boot itself
  was normal.
- The repository at `http://192.168.178.120:8080/pkg` can be used again.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-261-ge881cdb+xda44`

Real Pi 3B+ over netboot. Experiments: `timesync`, `welcome`. test 78 ([#24](https://github.com/redox-rpi/redox-rpi/issues/24) first-boot welcome app), Pi test agent

**Pass.** Staged with `~/t18-work/wel-stage/scripts/netboot.sh --server
192.168.178.120 --tftp-dir /srv/tftp --image ~/t18-work/image-wel/harddrive.img`
(the image's own bootloader and `netboot.scr`). Afterwards the saved stable
files went back into `/srv/tftp/redox`, including `netboot.cmd`/`netboot.scr`.

- After boot the serial console showed `Welcome to Redox OS on the Raspberry
  Pi.` and `This is the first start. Press Enter to set …` and waited, with no
  `redox login:`.
- Prompts and answers on the serial console:
  - Enter.
  - `Keyboard layouts: dvorak, us, gb, azerty, bepo, it`, `Keyboard layout
    [us]:` → `gb`.
  - `Time zone: no zone files in /usr/share/zoneinfo; the clock shows UTC.`
    (no prompt).
  - `Name of this computer (hostname) [redox]:` → `pi-test`.
  - `Set a new root password? The image's is "password" (y/n) [n]:` → Enter.
  - `Wi-Fi network name []:` → `TestNet`.
  - `Passphrase (8 to 63 characters, empty for an open network):` →
    `secret123`, **not echoed** (it isn't anywhere in the serial log).
- `Saved; the Wi-Fi driver joins it at the next boot.`, `Done. Log in as root.
  To change these settings later, run welcome --force.`, then the banner and
  `redox login:` at once: **pass**.
- Logged in on serial as root: `/etc/keymap` ends in `gb`, `/etc/hostname`
  is `pi-test` (no newline at the end), `/etc/net/wifi` has
  `ssid=TestNet`/`psk=secret123`, and `/etc/welcome.done` says `First-boot setup
  done. …`. `ls -l /etc/net/wifi`: `-rw------- 1 root root 78`: **pass**.
- Standard checks pass, 0 of 300 starts failed (devsh not affected).
- NanoKVM/HDMI not checked.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-172-g80279b9+x8cc4`

Real Pi 3B+ over netboot. Experiment: `redoxfs-write-coalesce` (patches 0002 and 0003). test 70 ([#16](https://github.com/redox-rpi/redox-rpi/issues/16) RedoxFS write coalescing, second round), Pi test agent

**Pass.** Image `~/redox-rpi-sd/keep/netboot-v0.3.0-dev-172-g80279b9+x8cc4.img`,
manifest matches.

| on `/scheme/data`, 1 MiB, `conv=fsync` | this image | test 46 | stable (test 46) |
|---|---|---|---|
| random source written, `bs=1048576` | 0.27 s | 0.27–0.37 s | 0.30 s |
| copy file to file, `bs=4k` | **0.49 s** | 11.9 s | 12.7 s |
| copy, `bs=1000` | **0.82 s** | 45 s | |
| copy, `bs=512` | **1.22 s** | 86 s | 94 s |
| `if=/scheme/rand bs=4096 count=256` | 0.39, 0.41 s | 0.38–0.44 s | 10.1 s |

- Copies with small blocks now take about as long as writing 1 MiB at once
  (QEMU: 0.94 s and 1.6 s).
- The SHA-256 of all three copies equals the source (`b8363f26…6ec90`), also
  after `echo reset > /scheme/sys/kstop`: **pass**.
- 300 `echo line $i >> log` (4.1 s) give 300 lines, also after the reset: **pass**.
- `ion -c "echo one; echo two; echo three; read x" > t.txt` left waiting in
  one devsh session. About 6 s later `echo reset > /scheme/sys/kstop` from a
  second session. After the boot, `cat t.txt` shows `one`, `two`, `three`:
  **pass** (the 2 s flush works).
- No `no timer for collected writes` in the serial log.
- Standard checks pass, 0 of 300 starts failed. Test files deleted.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-122-g74a1e24+x607a`

Real Pi 3B+ over netboot. Experiments: `ctx-leak-fix`, `rtl8152`, `sdhost-dma`, `timesync` (test 42's image, DMA with polling). test 87 ([#21](https://github.com/redox-rpi/redox-rpi/issues/21) sdhost DMA, CPU time with polling for comparison with test 75), Pi test agent

**Done: with polling, write SCHTIME is 3.4–8.6 s per 35 s; read about 1 s
per 15 s.** Manifest matches.

256 MiB from `/dev/urandom` to `/scheme/data`, `dd bs=1048576 conv=fsync`, then
read back. Columns of `bcm2835-sdhostd` (`ps` shows `bcm2835-sdhos`):

| | time | MB/s | SCHTIME | ACTTIME |
|---|---|---|---|---|
| write, run 1 (first after boot) | 35.0 s | 7.7 | +8.61 s (0.05 → 8.66) | +28.6 s |
| read, run 1 | 15.1 s | 18 | +1.00 s (8.66 → 9.66) | +13.7 s |
| write, run 2 | 35.3 s | 7.6 | +3.41 s (9.66 → 13.07) | +28.9 s |
| read, run 2 | 15.1 s | 18 | +1.04 s (13.07 → 14.11) | +13.7 s |
| test 75 write (interrupt) | 35.0 s | 7.7 | +4.3 s | +28.9 s |
| test 75 read (interrupt) | 15.3 s | 18 | +1.1 s | +14.0 s |

- Reads cost about 1 s of CPU in both drivers (7 % of the transfer time).
- Writes vary: 8.6 s on the first run after boot, 3.4 s on the second. Test
  75's single write (4.3 s) is in that range, so this data does not show that
  the interrupt saves CPU time when writing. More runs of each would tell.
- ACTTIME is the same in both, as expected if it also counts idle time.
- Standard checks pass, 0 of 300 starts failed. Test file deleted.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-98-g9b91443+x9d2b`

Real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `timerd`, `timerd-test`, `timesync`, `timezone`. test 49 ([#46](https://github.com/redox-rpi/redox-rpi/issues/46) time zone Europe/Berlin), Pi test agent

**Pass, but the Pi froze for 7 minutes.** Image `~/tz-images/netboot-harddrive.img`
(the row's build; the folder now also has a newer `tz-netboot-harddrive.img`
from 08:23).

- `date`: `Fri Oct 30 09:36:09 CET 2026`, `date -u`: `08:36:10 UTC`, the same
  as `TZ=Europe/Berlin date` on homenodellm: **pass**.
- `ls -l /etc/localtime`: `-> /usr/share/zoneinfo/Europe/Berlin`. `date -d
  '@1782907200'`: `Wed Jul  1 14:00:00 CEST 2026`: **pass**.
- timerd logs local times (`clock set by timesync, 2026-10-30 09:27:45
  Europe/Berlin`, `next run … Europe/Berlin`). `test-catchup` caught up a run
  missed since 2026-10-29 12:50, and `test-minute` and `test-catchup` run
  every minute: **pass**.
- Standard checks pass, 0 of 300 starts failed.
- **Freeze:** from 08:28:00 to 08:35:19 UTC (about 2 minutes after the login
  prompt) the whole Pi stopped: no serial echo, no ARP answer, devsh down, no
  timer runs. The plug read 4.3 W. Then it carried on by itself: `timerd:
  test-catchup: exited with code 0 after 438.0 s` and the same for
  `test-minute`. Afterwards `free` showed 583 MB free (of 745 MB).
  `/scheme/sys/stat` had CPU 0 busy 440 s and CPU 3 busy 439 s, the other two
  under 3 s. In `/scheme/sys/context` the initfs `redoxfs` had ACTTIME 7 min 20 s
  and SCHTIME 1 min 18 s, and `dwc2d` sat at `UR+` on `#2` (as in test 82). So
  the initfs `redoxfs` was busy for the whole freeze. This build is old
  (v0.3.0-dev-98, before `ctx-leak-fix`). No panic or new exception in the
  serial log (only the usual `pcid` one).

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-15-g24fbb96+x9f76` (stable), start loop with crash details

Real Pi 3B+ over netboot, the stable image already on the Pi. test 92 ([#10](https://github.com/redox-rpi/redox-rpi/issues/10) start loop with crash details, second round, branch `t10-start-loop` 61ea421), Pi test agent

**Pass for the tool.** `scripts/pi-test.py 192.168.178.148 --starts 1000
--serial-log ~/redox-rpi/logs/serial.log --crash-log ~/t10-crash.jsonl --report
/tmp/t10-report.md`. Copies are on homenodellm in
`~/pitest-tools/t92-crash.jsonl` and `~/pitest-tools/t92-report.md`.

- The run ended. The other standard checks pass, and `starts` counts the failures:
  `FAIL starts: 6 of 1000 failed: ls exit 1 (3×); pwd exit 1 (3×)` (test 48 on
  the same image: 3 of 1000).
- `~/t10-crash.jsonl` has 6 lines, one per failed start, each with `host`,
  `build_id`, `libc`, `program`, `status` and `crash` with `far`, `elr`,
  `esr`, `pid` and `name`. Each dump's NAME ends in the record's program
  (`/usr/bin/ls`, `/usr/bin/pwd`), and the PIDs differ (144, 202, 209, 296,
  665, 893): **pass**. No dumps without a failed start.
- 5 of the 6 are the same crash: FAR 0x1281dc8/0x1281dd0, ELR 0xc15c0c, ESR
  0x92000006 (data abort, write), LR 0xc03d30, the same 7-address trace. One
  (`pwd`, PID 209) jumped to address 1: FAR 0x1, ELR 0x1, ESR 0x8a000000, LR
  0xbcc7b0.

The report's `Failed starts` section as written by the tool:

Failed starts (build `v0.3.0-dev-15-g24fbb96+x9f76`):

- starts 51–100: `pwd`, exit 1; FAR 0x1281dd0, ELR 0xc15c0c, ESR 0x92000006, LR 0xc03d30, SP 0x7ffffffff590, PID 144, serial lines:

  ```
  kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)
  FAR_EL1: 0x1281dd0
  ELR_EL1: 0000000000C15C0C
  ESR_EL1: 0000000092000006
  kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #2, PID 144, NAME /usr/bin/pwd, CONTEXT 0xfffffe800015eb50
  ```
- starts 101–150: `ls`, exit 1; FAR 0x1281dd0, ELR 0xc15c0c, ESR 0x92000006, LR 0xc03d30, SP 0x7ffffffff590, PID 202, serial lines:

  ```
  kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)
  FAR_EL1: 0x1281dd0
  ELR_EL1: 0000000000C15C0C
  ESR_EL1: 0000000092000006
  kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #1, PID 202, NAME /usr/bin/ls, CONTEXT 0xfffffe800014c330
  ```
- starts 101–150: `pwd`, exit 1; FAR 0x1, ELR 0x1, ESR 0x8a000000, LR 0xbcc7b0, SP 0x7ffffffff9d0, PID 209, serial lines:

  ```
  kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100010)
  FAR_EL1: 0x1
  ELR_EL1: 0000000000000001
  ESR_EL1: 000000008A000000
  kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #0, PID 209, NAME /usr/bin/pwd, CONTEXT 0xfffffe800014c730
  ```
- starts 201–250: `ls`, exit 1; FAR 0x1281dc8, ELR 0xc15c0c, ESR 0x92000006, LR 0xc03d30, SP 0x7ffffffff590, PID 296, serial lines:

  ```
  kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)
  FAR_EL1: 0x1281dc8
  ELR_EL1: 0000000000C15C0C
  ESR_EL1: 0000000092000006
  kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #0, PID 296, NAME /usr/bin/ls, CONTEXT 0xfffffe800014c730
  ```
- starts 551–600: `ls`, exit 1; FAR 0x1281dd0, ELR 0xc15c0c, ESR 0x92000006, LR 0xc03d30, SP 0x7ffffffff590, PID 665, serial lines:

  ```
  kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)
  FAR_EL1: 0x1281dd0
  ELR_EL1: 0000000000C15C0C
  ESR_EL1: 0000000092000006
  kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #2, PID 665, NAME /usr/bin/ls, CONTEXT 0xfffffe800014c730
  ```
- starts 801–850: `pwd`, exit 1; FAR 0x1281dc8, ELR 0xc15c0c, ESR 0x92000006, LR 0xc03d30, SP 0x7ffffffff590, PID 893, serial lines:

  ```
  kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)
  FAR_EL1: 0x1281dc8
  ELR_EL1: 0000000000C15C0C
  ESR_EL1: 0000000092000006
  kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #0, PID 893, NAME /usr/bin/pwd, CONTEXT 0xfffffe800014c730
  ```

The crash log lines (for #1):

```
{"host": "192.168.178.148", "build_id": "v0.3.0-dev-15-g24fbb96+x9f76", "libc": "d4f0dedd5403751a71d75fabd5eaf3caa5e2035b135dcde39f2c64d65b3b1815", "time": "2026-10-30T08:48:29Z", "starts": "51-100", "program": "pwd", "status": "exit 1", "messages": [], "crash": {"kind": "ty=100100", "far": "0x1281dd0", "elr": "0xc15c0c", "esr": "0x92000006", "ec": "0x24", "lr": "0xc03d30", "sp": "0x7ffffffff590", "pid": 144, "cpu": 2, "name": "/usr/bin/pwd", "trace": ["0xc03d30", "0xbcb654", "0xc044a0", "0xbcc6cc", "0xbcd428", "0xc2c6f0", "0xad1038"]}, "serial": ["kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)", "FAR_EL1: 0x1281dd0", "ELR_EL1: 0000000000C15C0C", "ESR_EL1: 0000000092000006", "kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #2, PID 144, NAME /usr/bin/pwd, CONTEXT 0xfffffe800015eb50"]}
{"host": "192.168.178.148", "build_id": "v0.3.0-dev-15-g24fbb96+x9f76", "libc": "d4f0dedd5403751a71d75fabd5eaf3caa5e2035b135dcde39f2c64d65b3b1815", "time": "2026-10-30T08:48:29Z", "starts": "101-150", "program": "ls", "status": "exit 1", "messages": [], "crash": {"kind": "ty=100100", "far": "0x1281dd0", "elr": "0xc15c0c", "esr": "0x92000006", "ec": "0x24", "lr": "0xc03d30", "sp": "0x7ffffffff590", "pid": 202, "cpu": 1, "name": "/usr/bin/ls", "trace": ["0xc03d30", "0xbcb654", "0xc044a0", "0xbcc6cc", "0xbcd428", "0xc2c6f0", "0xad1038"]}, "serial": ["kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)", "FAR_EL1: 0x1281dd0", "ELR_EL1: 0000000000C15C0C", "ESR_EL1: 0000000092000006", "kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #1, PID 202, NAME /usr/bin/ls, CONTEXT 0xfffffe800014c330"]}
{"host": "192.168.178.148", "build_id": "v0.3.0-dev-15-g24fbb96+x9f76", "libc": "d4f0dedd5403751a71d75fabd5eaf3caa5e2035b135dcde39f2c64d65b3b1815", "time": "2026-10-30T08:48:29Z", "starts": "101-150", "program": "pwd", "status": "exit 1", "messages": [], "crash": {"kind": "ty=100010", "far": "0x1", "elr": "0x1", "esr": "0x8a000000", "ec": "0x22", "lr": "0xbcc7b0", "sp": "0x7ffffffff9d0", "pid": 209, "cpu": 0, "name": "/usr/bin/pwd", "trace": ["0xbcd428", "0xc2c6f0", "0xad1038"]}, "serial": ["kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100010)", "FAR_EL1: 0x1", "ELR_EL1: 0000000000000001", "ESR_EL1: 000000008A000000", "kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #0, PID 209, NAME /usr/bin/pwd, CONTEXT 0xfffffe800014c730"]}
{"host": "192.168.178.148", "build_id": "v0.3.0-dev-15-g24fbb96+x9f76", "libc": "d4f0dedd5403751a71d75fabd5eaf3caa5e2035b135dcde39f2c64d65b3b1815", "time": "2026-10-30T08:48:29Z", "starts": "201-250", "program": "ls", "status": "exit 1", "messages": [], "crash": {"kind": "ty=100100", "far": "0x1281dc8", "elr": "0xc15c0c", "esr": "0x92000006", "ec": "0x24", "lr": "0xc03d30", "sp": "0x7ffffffff590", "pid": 296, "cpu": 0, "name": "/usr/bin/ls", "trace": ["0xc03d30", "0xbcb654", "0xc044a0", "0xbcc6cc", "0xbcd428", "0xc2c6f0", "0xad1038"]}, "serial": ["kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)", "FAR_EL1: 0x1281dc8", "ELR_EL1: 0000000000C15C0C", "ESR_EL1: 0000000092000006", "kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #0, PID 296, NAME /usr/bin/ls, CONTEXT 0xfffffe800014c730"]}
{"host": "192.168.178.148", "build_id": "v0.3.0-dev-15-g24fbb96+x9f76", "libc": "d4f0dedd5403751a71d75fabd5eaf3caa5e2035b135dcde39f2c64d65b3b1815", "time": "2026-10-30T08:48:29Z", "starts": "551-600", "program": "ls", "status": "exit 1", "messages": [], "crash": {"kind": "ty=100100", "far": "0x1281dd0", "elr": "0xc15c0c", "esr": "0x92000006", "ec": "0x24", "lr": "0xc03d30", "sp": "0x7ffffffff590", "pid": 665, "cpu": 2, "name": "/usr/bin/ls", "trace": ["0xc03d30", "0xbcb654", "0xc044a0", "0xbcc6cc", "0xbcd428", "0xc2c6f0", "0xad1038"]}, "serial": ["kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)", "FAR_EL1: 0x1281dd0", "ELR_EL1: 0000000000C15C0C", "ESR_EL1: 0000000092000006", "kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #2, PID 665, NAME /usr/bin/ls, CONTEXT 0xfffffe800014c730"]}
{"host": "192.168.178.148", "build_id": "v0.3.0-dev-15-g24fbb96+x9f76", "libc": "d4f0dedd5403751a71d75fabd5eaf3caa5e2035b135dcde39f2c64d65b3b1815", "time": "2026-10-30T08:48:29Z", "starts": "801-850", "program": "pwd", "status": "exit 1", "messages": [], "crash": {"kind": "ty=100100", "far": "0x1281dc8", "elr": "0xc15c0c", "esr": "0x92000006", "ec": "0x24", "lr": "0xc03d30", "sp": "0x7ffffffff590", "pid": 893, "cpu": 0, "name": "/usr/bin/pwd", "trace": ["0xc03d30", "0xbcb654", "0xc044a0", "0xbcc6cc", "0xbcd428", "0xc2c6f0", "0xad1038"]}, "serial": ["kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100100)", "FAR_EL1: 0x1281dc8", "ELR_EL1: 0000000000C15C0C", "ESR_EL1: 0000000092000006", "kernel::context::signal:INFO -- UNHANDLED EXCEPTION, CPU #0, PID 893, NAME /usr/bin/pwd, CONTEXT 0xfffffe800014c730"]}
```

## 2026-10-30: `v0.3.0-dev-354-gc656808+x6039`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152`, `cpu-ondemand`, `cputop`. test 93 ([#23](https://github.com/redox-rpi/redox-rpi/issues/23) CPU clock follows the load, second round), Pi test agent. Raw output on homenodellm in `~/pitest-tools/t93/`.

**Pass.** Image `~/redox-rpi-cpu/keep/v0.3.0-dev-354-gc656808+x6039/harddrive.img`.
Boot log `rpi-firmwared: ARM clock 600 -> 1400 MHz`. Shelly read only, every
5 s (13 readings per 60 s).

| | share at min over 60 s | Shelly mean |
|---|---|---|
| (1) ondemand, idle, 60 s after login | **96 %** (Δ`time_min_ms` 57,996, Δ`time_max_ms` 2,371) | **3.05 W** |
| (2) `powersave`, 60 s | (min) | **3.00 W** |
| (3) `performance`, 60 s | (max) | **3.20 W** |
| (7) ondemand, idle, 60 s after the start loops | **96 %** (Δ 58,063 / 2,324) | **3.01 W** |

- ondemand at idle stays at the minimum clock almost all the time and uses as
  little power as `powersave`. After load it goes back down. The power
  difference between the two clocks at idle is small (0.2 W).
- `target` still reads `max` with `load` 87–100 in every reading, because
  reading the file starts a program (as the row says). The time counters show
  the real picture.
- (5) `cputop 30` at idle: cpu0 0 %, cpu1 0 %, cpu2 0 % (116 ms), cpu3 2 %
  (741 ms). Busiest: `dwc2d` 2.3 %, `rpi-firmwared` 0.3 %, `netstack` 0.1 %,
  the rest 0.0 %.
- (6) Standard checks pass, 0 of 300 starts failed. `ondemand-check.py`, 50 starts
  each, two rounds:

| mode | round 1 | round 2 |
|---|---|---|
| ondemand | 8.0 s | 8.0 s |
| performance | 7.9 s | 7.9 s |
| powersave | 15.9 s | 15.9 s |

  0 failed. ondemand is as fast as performance. End: `throttled 0x00000000`,
  temperature 49.9 °C.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-339-g801d1fe+x9f76`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152` (today's main, the stable image config). test 94 ([#1](https://github.com/redox-rpi/redox-rpi/issues/1) remaining start crashes), Pi test agent

**Pass: 0 of 3000 starts failed.** Staged with
`~/t18-work/t1-stage/scripts/netboot.sh --server 192.168.178.120 --tftp-dir
/srv/tftp --image ~/t18-work/image-t1/harddrive.img`, bootloader md5
`7a5e2023…`. Afterwards the saved stable files went back into
`/srv/tftp/redox` (`netboot.cmd`/`netboot.scr` included).

- ARM clock: `/scheme/rpi-firmware/clock` does not exist; `clocks` shows `arm
  1400000000 1400000000` (1400 MHz).
- `pi-test.py --manifest …/netboot-v0.3.0-dev-339-g801d1fe+x9f76.toml --starts
  3000 --serial-log … --crash-log ~/t18-work/t1-pi.jsonl --report
  ~/t18-work/t1-pi.md` (from a checkout of `t10-start-loop` 61ea421; I couldn't run
  `~/t18-work/t10/` as written because it has no `scripts/` folder): all checks
  pass, manifest matches, **0 of 3000 starts failed**. So no crash log file was written.
- Serial log from login to the end of the run: no crash dump. Only the usual
  `pcid` exception at boot. For comparison, the stable image (v0.3.0-dev-15)
  had 6 of 1000 in test 92 today.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-217-g2d2f24c+xf552`

Real Pi 3B+ over netboot. Experiments: `multicast`, `ipv6`, `lan78xx-multicast`, `mdns`, `rtl8152`, `timesync`. test 85 ([#27](https://github.com/redox-rpi/redox-rpi/issues/27) IPv6 global address and default route), Pi test agent

**Pass.** Image `~/redox-rpi-zrtl/keep/slaac-v0.3.0-dev-217-g2d2f24c+xf552.img`.
The Fritz!Box's global prefix today is `2a02:590:1504:9901::/64` (homenodellm:
`2a02:590:1504:9901:4a45:e6ff:fea0:b063`).

- About a minute after boot, `eth0` addresses: `fe80::ba27:ebff:fe08:e0aa/64`,
  **`2a02:590:1504:9901:ba27:ebff:fe08:e0aa/64`**,
  `fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa/64`, `192.168.178.148/24`: **pass**.
- Routes: **`default via fe80::b6fc:7dff:fe53:b289 dev eth0 src
  2a02:590:1504:9901:ba27:ebff:fe08:e0aa`**, plus `2a02:590:1504:9901::/64`,
  `fdeb:d4ef:a7ac::/64` and `fe80::/64` on eth0: **pass**.
- The serial log has no `router advertisement from …` lines (this image
  doesn't print them). The only router seen is the Fritz!Box,
  `fe80::b6fc:7dff:fe53:b289`.
- `ping -6` to the Pi's global address from homenodellm: 3/3 (0.9–4.3 ms):
  **pass**.
- `net6 udp [2a02:…:b063]:7777 hello`: reply `hello`, and `udp-echo.py` saw
  the Pi's `2a02:590:1504:9901:ba27:ebff:fe08:e0aa` (it timed out in test 55):
  **pass**. To the ULA `fdeb:…:b063`: reply, echo saw the Pi's ULA: **pass**.
- `net6 resolve example.com`: `2606:4700:10::6814:179a`,
  `2606:4700:10::ac42:93f3` (and two IPv4 addresses). `net6 tcp
  [2606:4700:10::6814:179a]:80 hello`: from `[2a02:…:e0aa]:49152`, reply
  `HTTP/1.1 400 Bad Request`, `Server: cloudflare`: **pass** (internet over IPv6).
- `/scheme/netcfg/resolv/nameserver6`: `fdeb:d4ef:a7ac:0:b6fc:7dff:fe53:b289`.
- `mdns-query.py redox-rpi.local AAAA -6 wlp13s0`: the Pi answers with three
  AAAA records (fe80, 2a02 and fdeb) and the A record. The Fritz!Box also
  answers from its cache: **pass**.
- After 10 minutes (09:25 UTC): the same addresses and routes.
- Standard checks pass, 0 of 300 starts failed.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-218-ge3e4424+x19b9`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152`, `boot-time`. test 84 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47) boot time, third round, base `0028`), Pi test agent

**Pass: network in 5 of 5 boots, login about 21.4 s after `kreset`.** Staged
with `~/t18-work/bt6-stage/scripts/netboot.sh` (bootloader md5 `549fc80b…`).
Afterwards the saved stable files went back into `/srv/tftp/redox`. Timelines
are on homenodellm in `~/t18-work/tl-pi-bt6-{1..5}.txt`; each boot was a devsh
reset (boot 1 from the stable image).

| Seconds | boot 1 | boot 2 | boot 3 | boot 4 | boot 5 |
|---|---|---|---|---|---|
| `kreset` | 2.02 | 1.55 | 1.50 | 1.55 | 1.58 |
| kernel starts | 18.35 | 17.31 | 17.34 | 17.28 | 17.39 |
| `switchroot to /usr` | 22.21 | 21.15 | 21.21 | 21.13 | 21.24 |
| `redox login:` | 23.92 | 22.91 | 22.91 | 22.88 | 22.95 |
| **`redox login:` after `kreset`** | **21.90** | **21.36** | **21.41** | **21.33** | **21.37** |
| `40_dwc2d.service ready after` | 8409 ms | 6741 ms | 6246 ms | 6429 ms | 6304 ms |
| network | yes | yes | yes | yes | yes |

- No `Can't open /scheme/netcfg/ifaces/eth0/mac` in any boot, and devsh
  answered after every boot: **pass** (test 69 boot 1 had no network).
- Login is about as fast as test 69 (21.3–22.1 s after `kreset`).
- Standard checks pass, 0 of 300 starts failed.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-238-g03161ba+x6026`

Real Pi 3B+ over netboot. Experiments: `relibc-tests`, `timesync`, `rtl8152`, `sigentry-thread-signals`, `sigsegv`, `sigregs-test`, `timer-sigmask`. test 86 ([#13](https://github.com/redox-rpi/redox-rpi/issues/13) relibc test suite with timer-sigmask, dash and test-order fixes), Pi test agent

**Pass: 319 tests, 4 failed (all known), 0 new, 14 fixed.** Image
`~/redox-rpi-t12/build/g3-pi.img`.

- Standard checks pass, 0 of 300 starts failed.
- `sigregs sender`: `PASS sender: si_pid 374 from the thread, 375 from the
  child`. `sigregs thread -n 3000`: `PASS thread: 3000 signals, all registers
  kept` (95 s). `sigregs process -n 3000`: `PASS process: 3000 signals, all
  registers kept` (95 s).
- `ion /usr/share/relibc-tests/run.ion` over devsh: 51 s. Output saved on
  homenodellm as `~/pitest-tools/relibc86.txt` (copy in
  `~/redox-rpi-t12/build/relibc86-pi.txt`). `relibc-tests-compare.py` and
  `config/relibc-tests-expected.txt` from `t-g3-pi`:
  `SUMMARY v0.3.0-dev-238-g03161ba+x6026: 319 tests, 4 failed (4 known, 0
  flaky, 0 new), 14 fixed`. No `NEW` line.
- The 4 failures: `bins_dynamic/redox_ring_ramfs` (exit status 1),
  `bins_static/tls_align`, `bins_dynamic/tls_align`, `bins_dynamic/tls_shared`
  (stdout mismatch). `time/timer`, `stdio/popen`, `stdlib/system`,
  `unistd/exec`, `stdio/fscanf_offby1`, `stdio/ungetc_ftell` and `stdio/mutex`
  pass.
- 14 `FIXED`, among them `netdb/getaddrinfo`, `pthread/rwlock_randtest`,
  `sigaction`, `sigaltstack`, `signals/sigpause-revert`,
  `signals/sigprocmask-3`, `signals/sigsegv`, `signals/sigset-5`,
  `signals/stackoverflow`, `sys_select/pselect`, `sys_socket/getpeername`,
  `getsockopt`, `recv`, `recvfrom`. That is more than QEMU's 8: the expected list
  is based on QEMU's 10 failures, and the Pi fails fewer.

Stable image restored afterwards.


## 2026-10-29: `v0.3.0-dev-260-g480ba01+xda44-dirty.371f590ffeb2` (welcome app, #24), QEMU

Branch `welcome-app`, experiment `welcome`. `test-welcome.sh` answers the
first-boot questions on the serial console: layout `gb`, hostname
`pi-test`, no new password, Wi-Fi `TestNet` / `secret123`. The files are
written (`/etc/net/wifi` mode 600), the passphrase is not echoed, the
login prompt follows at once. The second boot of the same card shows no
welcome and keeps hostname `pi-test`. **Pass.** The time zone question was
skipped (no zone files in this image). Pi test 78 passed later; the rebased branch (`v0.3.0-dev-381-g5d6d934+xda44`, patch `0040`) passes this test as well.

## 2026-10-30: `v0.3.0-dev-109-ga21be68+x192b`

Real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `timesync`, `timerd`, `mqtt`, `mqtt-demo`, `ha-bridge`. test 88 ([#49](https://github.com/redox-rpi/redox-rpi/issues/49) retest of tests 58 and 65 with the connect fix), Pi test agent

**Pass.** Broker `~/mqtt-broker/pi-test-broker.sh` (plain, 192.168.178.120:1883);
capture saved as `~/pitest-tools/broker88.out` on homenodellm. Manifest matches.

- Standard checks pass, 0 of 300 starts failed.
- First 3 minutes, default broker 10.0.2.2: `mqtt-pub: 10.0.2.2:1883: no
  answer from the broker within 10 s` (6×), `ha-bridge: … no answer … trying
  again in 30 s` (the service, 5×) and `ha-bridge: … no answer …` (the timer
  run, 3×). Every timer run ended `exited with code 1 after 10.1–10.4 s`, and
  there was **no** `still running, skipped this run`. `timerd -l` answered at once
  (0.3 s) with all three timers and their last runs: **pass**.
- `echo 192.168.178.120 > /etc/mqtt-host` at 09:47:52, nothing killed. The log
  had `ha-bridge: 6 sensors announced on 192.168.178.120:1883 as
  redox-rpi/redox` after **40 s** (a little over the 30 s in the row: the
  service retries every 30 s, plus the 10 s timeout). The broker got
  `redox-rpi/redox/availability online` and the six `homeassistant/…/config`
  topics (5 `sensor`, 1 `binary_sensor`). Once a minute it got
  `redox-rpi/temperature`, `redox-rpi/uptime` and `redox-rpi/redox/state`
  (the timer runs already published at 09:48:00): **pass**.
- `mosquitto_pub … -t homeassistant/status -m online`: the six configs came
  again (6 → 12 in the capture): **pass**.
- `mqtt-pub -h 192.168.178.99 -t x -m x`: `no answer from the broker within
  10 s`, exit 1, after 10.05 s: **pass**.
- `echo reset > /scheme/sys/kstop` at 09:50:33. `redox-rpi/redox/availability
  offline` came at 09:51:03, **about 30 s** after the reset: **pass**.
- `free` every 3 minutes (total 745,252 KB): 09:48 562,016 KB free, 09:51
  597,492 KB free (the 09:45 reading timed out during the standard checks).
  No loss over 6 minutes; the row's TLS rerun (#89) is the comparison.

Broker stopped, stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-115-g78c6372+x125f`

Real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `timesync`, `timerd`, `mqtt`, `ha-bridge`, `mqtt-tls-test`. test 89 ([#56](https://github.com/redox-rpi/redox-rpi/issues/56) rerun of test 76, MQTT over TLS with the connect fix), Pi test agent

**Pass, but free memory falls about 5.5 MB a minute while ha-bridge runs over
TLS.** Broker `~/mqtt-broker/pi-test-broker-tls.sh`; capture saved as
`~/pitest-tools/broker89.out` on homenodellm. Manifest matches.

- Standard checks pass, 0 of 300 starts failed. A first try right after the
  boot printed nothing (probably before devsh was up); the second passed.
- Before the broker was set (about 9 minutes): `ha-bridge: 10.0.2.2:8883: no
  answer from the broker within 10 s; trying again in 30 s` (the service) and
  the timer runs, each `exited with code 1 after 10.1 s`. **No** `still
  running, skipped this run`: **pass**. `/usr/lib/init.d/75_ha-bridge.service`
  runs `ha-bridge -d`; `ps` shows `ha-bridge` (pid 38, two threads).
- `echo 192.168.178.120 > /etc/mqtt-host` at 10:04:04, nothing killed. `ha-bridge:
  6 sensors announced on 192.168.178.120:8883 (TLS) as redox-rpi/redox` after
  about 26 s. The broker got `redox-rpi/redox/availability online`, the six
  configs, and `redox-rpi/redox/state` once a minute: **pass**.
- `mqtt-pub -t tls/pi -m hello -q 1`: exit 0, `tls/pi hello` arrived: **pass**.
- `echo reset > /scheme/sys/kstop` at 10:15:19, `availability offline` at
  10:15:45, **about 27 s** later: **pass**.
- `free` (free KB of 745,252):

| time | free | |
|---|---|---|
| 09:58:40 | 581,704 | after boot, broker not set |
| 10:01:49 | 506,400 | right after the standard checks |
| 10:04:51 | 501,876 | broker just set (TLS from 10:04:30) |
| 10:11:32 | 460,308 | |
| 10:14:33 | 448,308 | |

  From 10:04:51 to 10:14:33, 53.6 MB less in 9.7 minutes, **about 5.5 MB a
  minute**. Test 88 (same features without TLS) stayed level, and test 76 (older
  image) lost about 15 MB a minute. So the loss comes with the TLS connections
  (one `ha-bridge` connection a minute plus the service's).

Broker stopped, retained `tls/pi` cleared, stable image restored afterwards.
