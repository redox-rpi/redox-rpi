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
