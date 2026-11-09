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

## 2026-10-30: `v0.3.0-dev-225-g9ed4e48+xd4e7`

Real Pi 3B+ over netboot. Experiments: `ctx-leak-fix`, `timesync`, `rtl8152`, `wifi-sdio`. test 91 part A ([#26](https://github.com/redox-rpi/redox-rpi/issues/26) Wi-Fi steps 3–4, rerun of test 77), Pi test agent

**Part A (`gpu_mem=16`): pass, the Wi-Fi scan works. Part B (`gpu_mem=64`) not
done:** my permission check blocked the change to the shared
`/srv/tftp/config.txt`, so it needs the user's OK. The file is unchanged
(`gpu_mem=16`). Image
`~/redox-rpi-zrtl/keep/wifiscan4-v0.3.0-dev-225-g9ed4e48+xd4e7.img`.

- Boot: `rpi-firmwared: Wi-Fi power 0, tag not marked answered` and `Wi-Fi
  power 1, tag not marked answered`, with no `I/O error` any more. Everything test
  60 checks up to `MAC address b8:27:eb:5d:b5:ff`, then `CLM loaded (2676 bytes,
  2 chunks)`, `radio up`, `event other (54), status 0, reason 0, flags 0x0`,
  `frame on channel 3, 4 bytes`, `frame on channel 3, 1396 bytes`, **`9
  networks`**, `no /etc/net/wifi, not joining a network`, `bcm2835-sdiod: OK`:
  **pass**.
- Scan list (BSSIDs and the neighbours' names left out here; all `Wpa2`): the
  Fritz!Box `zehrer.net` on ch 1 (−57 dBm) and ch 64 (−60 dBm), `zehrer.xyz`
  on ch 1 (−58 dBm), and six other networks on ch 6 (−70, −71), 11 (−75),
  108 (−79, −79) and 116 (−85). Channels and levels look plausible.
- Over devsh: `echo 0 > /scheme/rpi-firmware/wifi_power; echo $?` → `0`, `cat`
  → `0`; `echo 1 …` → `0`, `cat` → `1`: **pass** (tests 57, 60 and 77 gave I/O error).
- `/usr/lib/drivers/bcm2835-sdiod` once more (power-cycles the chip): the same
  lines up to `OK`, **8 networks**, the same as at boot without the weakest
  (ch 116, −85 dBm). Levels within 1–2 dB.
- `free`: total 743,612 KB, 606,428 KB free.
- Standard checks on the stable image afterwards: 1 of 300 starts failed (stable's
  known start crashes, see test 92).

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-155-g16b10cb+x7d9f`

Real Pi 3B+ over netboot. Experiments: `gpio`, `bluetooth`. test 90 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54) Bluetooth, rerun of test 79 with the power switch fixed), Pi test agent

**Fail: the power switch works now, but the controller still never answers.**
Image `~/redox-rpi-cpu/keep/v0.3.0-dev-155-g16b10cb+x7d9f/harddrive.img`.

- Standard checks pass, 0 of 300 starts failed.
- Boot: `bcm2835-btuartd: mini UART at 0x3f215000, 115313 baud (core 250 MHz)`.
  No `tag 0x00038041: not supported` any more, no `received bytes lost`, no
  `a 1 ms sleep did not come back`.
- `cat /scheme/rpi-firmware/bt_power`: `0`. `bt up` (2.4 s): **`power:
  cycled`** (no I/O error: the fix works), `uart: core clock 250000000 Hz,
  115313 baud`, then `bt: no answer to HCI command 0c03`. `bt_power` then reads `1`.
- `bt info`, `bt scan 10`, `bt inquiry 10`: `no answer to HCI command` 1001,
  0c01 and 0401.
- `echo 0 > …/bt_power; echo 1 > …/bt_power; bt cmd 0c03`: no answer.
- `throttled`: `0x00080000` (the soft temperature limit was reached at some
  point since boot; the temperature is 56.9 °C, nothing throttled now).
- Possible cause: `/srv/tftp/config.txt` has `enable_uart=1` and
  `dtoverlay=disable-bt` (the PL011 goes to the serial console on GPIO 14/15).
  The BT chip's UART lines (GPIO 30–33) may then not be routed to the mini
  UART, or still be set for another function. A GPIO function readout of
  GPIO 30–33 after `bt up` would show it.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-341-gc3ad5d3+x59c0`

Real Pi 3B+ over netboot. Experiments: `multicast`, `ipv6`, `lan78xx-multicast`, `mdns`, `sshd`, `rtl8152`, `timesync`. test 96 ([#37](https://github.com/redox-rpi/redox-rpi/issues/37) mDNS announces SSH), Pi test agent

**Pass.** Image `~/redox-rpi-zrtl/keep/mdns-ssh-v0.3.0-dev-341-gc3ad5d3+x59c0.img`.

- Standard checks pass, 0 of 300 starts failed.
- `mdnsd:` lines: `redox-rpi.local on 224.0.0.251 and ff02::fb port 5353, 2
  services configured`, `redox-rpi._devsh._tcp.local port 2323 announced`,
  `redox-rpi._ssh._tcp.local port 22 announced`, `redox-rpi.local is
  192.168.178.148, fe80::ba27:ebff:fe08:e0aa, 2a02:590:1504:9901:ba27:ebff:fe08:e0aa,
  fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa`.
- `mdns-query.py redox-rpi.local`: A `192.168.178.148` (ttl 120) and the three
  AAAA: **pass**.
- `mdns-query.py _ssh._tcp.local PTR`: PTR `redox-rpi._ssh._tcp.local` (ttl
  4500), plus A/AAAA. `redox-rpi._ssh._tcp.local SRV`: **SRV port 22 target
  `redox-rpi.local`** (ttl 120), and `ANY` adds `TXT 00`: **pass**.
  - Note: the PTR answer carries A/AAAA as extra records but not the SRV and
    TXT. The Mac on the same network sends them with its PTR. Browsers that
    ask SRV separately (avahi, dns-sd) still work. Adding them to the PTR
    answer would save one query.
- `_devsh._tcp.local PTR`: `redox-rpi._devsh._tcp.local`, SRV (asked
  directly) port 2323: **pass**.
- `_services._dns-sd._udp.local PTR`: the Pi lists `_devsh._tcp.local` and
  `_ssh._tcp.local`: **pass**.
- `ssh -v root@192.168.178.148` (the address from mDNS, test key put into
  `/home/root/.ssh/authorized_keys` over devsh): `Server host key: ssh-ed25519
  SHA256:6b9SpKQH…` (the same as test 81, kept on `/scheme/data`),
  `Authenticated … using "publickey"`, `OK`, `Redox redox-rpi 0.5.12 …`: **pass**.
- Idle CPU over 30 s (`ps`, there is no `top`): `mdnsd` SCHTIME +0.02 s, `sshd`
  +0.00 s.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-362-g4ddab1f+x0937`

Real Pi 3B+ over netboot. Experiments: `futex-wait-lock`, `relibc-tests`, `timesync`. test 98 ([#43](https://github.com/redox-rpi/redox-rpi/issues/43) lost futex wakeups, kernel backport of upstream `57ccc8f` and `5805059`), Pi test agent

**Pass: 40 of 40 `cond_stress` runs without a lost wakeup; relibc suite 0 new
failures.** Image `~/redox-rpi-t12/build/futex-pi.img`. Output on homenodellm
in `~/redox-rpi-t12/build/cond98-pi.txt` and `relibc98-pi.txt`.

- **First boot: no network.** `dwc2d` panicked during USB enumeration:
  `thread 'main' (1) panicked at drivers/usb/dwc2d/src/enumerate.rs:173:40:
  index out of bounds: the len is 0 but the index is 7`. Then `smoltcpd: no
  network adapter found` and `dhcpd: Can't open /scheme/netcfg/ifaces/eth0/mac`.
  The second boot (serial-console reset) was normal (`LAN78xx link up after
  1710 ms`). So 1 of 2 boots; this build has no `rtl8152` experiment.
- Standard checks pass, 0 of 300 starts failed.
- `cd /usr/share/relibc-tests; for i in 1..=40; ./bins_dynamic/pthread/cond_stress;
  echo rc=$?; end` (247 s, about 6 s each): 40 × `rc=0`, 40 × `lost wakeups:
  0`, no `lost wakeup after`: **pass** (QEMU without the fix: 3 of 21).
- `ion /usr/share/relibc-tests/run.ion` (71 s), compared with
  `relibc-tests-compare.py` and `config/relibc-tests-expected.txt` from
  `t-futex-wait-lock`: `SUMMARY v0.3.0-dev-362-g4ddab1f+x0937: 320 tests, 16
  failed (16 known, 0 flaky, 0 new), 6 fixed`. No `NEW` line: **pass**.
- Fixed: `netdb/getaddrinfo`, `pthread/rwlock_randtest`,
  `sys_socket/getpeername`, `getsockopt`, `recv`, `recvfrom`. The 16 known
  failures are the ones without the sigsegv/sigentry experiments (`sigaltstack`,
  `signals/sigsegv`, `stackoverflow`, `sigset-5`, `sigprocmask-3`, `sigaction`,
  `sys_select/pselect` SIGILL), plus `time/timer`, `stdio/mutex`, `fscanf_offby1`,
  `ungetc_ftell`, `tls_align` ×2, `tls_shared` and `redox_ring_ramfs`.

Stable image restored afterwards.

## 2026-10-29: `v0.3.0-dev-141-g3fd1bba+x9ea9` (German keyboard layout, #36), QEMU

Branch `keyboard-layouts`, experiment `keyboard-de` with `timesync` and
`rtl8152`. QEMU `raspi3b`, USB keyboard through the QEMU monitor
(`patches/experiments/keyboard-de/test-keymap.sh`).

- `inputd -K de` on the serial console, then login on the console with
  the USB keyboard: `root`, `password` unchanged.
- Typed as German key positions: `cat > /home/root/de.txt` and
  `zyß@{[]}\~|#öäü€"§'<>`. The file holds exactly that line: z and y
  swapped, umlauts, ß, AltGr for @ { [ ] } \ ~ | €, the < > key. **Pass.**
- An earlier attempt with `echo '…'` failed in ion ("`[` is out of place"
  even in single quotes), not in the layout: the screen showed the line
  typed correctly.

## 2026-10-29: `v0.3.0-dev-143-ge940bf6+x9ea9` (German layout from boot, #36), QEMU

Same branch, now with `config.toml` (`/etc/init.d/05_keymap.service`
runs `inputd -K de`). `test-keymap.sh` no longer switches the layout on
the serial console: the login and the typed line come out German from
boot, the file holds `zyß@{[]}\~|#öäü€"§'<>`. **Pass.**

## 2026-10-29: `v0.3.0-dev-239-g43c6882+x9ea9` (dead keys, layout from /etc/keymap, #36), QEMU

Branch `keyboard-layouts`: base `0026` adds dead keys and `inputd
--keymap-file`; the boot unit reads `/etc/keymap` (`de`). `test-keymap.sh`
types the earlier line plus ´ e, Shift+´ a, ^ o, ´ space, ^ x: the file
holds `zyß@{[]}\~|#öäü€"§'<>éàô´^x`. **Pass.** The German layout comes from
`/etc/keymap` at boot, without `inputd -K` by hand.

## 2026-10-30: `v0.3.0-dev-424-g2b56378+x9ea9` (German keyboard layout, #36), QEMU

Branch `keyboard-layouts` with main merged, patches renumbered `0041`,
`0042`. `test-keymap.sh` types `zyß@{[]}\~|#öäü€"§'<>éàô´^x` on QEMU's USB
keyboard, and it comes out exactly as typed: **pass**. This image replaces
`v0.3.0-dev-239-g43c6882+x9ea9` for Pi test 64.

## 2026-10-29: `v0.3.0-dev-216-gfcd1b8d+x19b9-dirty` (parallel init, #47), QEMU

Branch `boot-time` with base `0027`: init starts `notify` services in
parallel; the root file system no longer waits for `dwc2d`. QEMU `raspi3b`
with `--usb-net`: `switchroot to /usr` right after `50_rootfs` (1.5 s after
init's first unit), `40_dwc2d.service` ready at 2.2 s alongside the `/usr`
units, `netstack` after it, `redox login:` 6.0 s after init's first unit
(14.9 s with `v0.3.0-dev-214-g67fdc06+x19b9`, on a slower run). DHCP and
`ping -c 2 10.0.2.2` pass, `/tmp` is `drwxrwxrwt`. Without a network card
the boot ends at the login as well. **Pass.** Pi test queued.

## 2026-10-30: `v0.3.0-dev-217-g1d028a4+x19b9-dirty` (netstack ready after eth0, #47), QEMU

Branch `boot-time` with base `0028`: `netstack` reports ready after
`Smolnetd::new` has set up `eth0` and its schemes (Pi test #69 boot 1 had
`dhcpd` fail with `Can't open /scheme/netcfg/ifaces/eth0/mac`). QEMU
`raspi3b` with `--usb-net`, three boots: DHCP and `ping -c 1 10.0.2.2` pass
each time, no `eth0/mac` error. **Pass.** Pi test queued.

## 2026-10-30: `v0.3.0-rc2` (`v0.3.0-dev-402-g78a673d+x12cb`)

Real Pi 3B+ over netboot. `--release v0.3.0-rc2 --experiment timesync`, main 78a673d. test 102 (v0.3.0 rc2 automated, checklist tests 1–5 and 11), Pi test agent

**Pass: all six checks, and no `dwc2d` panic in 6 of 6 boots.** Image
`~/redox-rpi-rel/keep/v0.3.0-rc2-netboot.img`, staged with the build agent's
`scripts/netboot.sh` (its bootloader and manifest). Afterwards the saved stable
files went back into `/srv/tftp/redox`.

| # | check | result |
|---|---|---|
| 1 | `pi-test.py --starts 1000` (`t10-start-loop` copy) | **pass**: `PASS version: v0.3.0-rc2 (v0.3.0-dev-402-g78a673d+x12cb)`, all checks pass, **0 of 1000 starts failed** |
| 2 | clock | **pass**: `rpi-firmwared: ARM clock 600 -> 1400 MHz`, `clocks` shows `arm 1400000000 1400000000`, no throttle or under-voltage line |
| 3 | SD and unused drivers | **pass**: `bcm2835-sdhostd: SDHC/SDXC card, 15193 MiB, 4-bit bus, high speed, up to 50 MHz`, `serving /scheme/disk.mmc`; no `bcm2835-sdhcid`, `pcid`, `pcid-spawner`, `audiod` or `rtcd` line in any of the six boots |
| 4 | time | **pass**: `timesync: clock set to 2026-10-30 10:47:50 UTC from pool.ntp.org`; `date -u` on the Pi and the build host both `10:51:36` |
| 5 | `reboot` over devsh × 5 | **pass**: each back to `redox login:` after 22 s, devsh answers, one `kreset` each |
| 11 | `serial-input-test.sh 20` (logged in on the console) | **pass**: `20 of 20 lines complete`, no getty panic |

- `dwc2d` panic check (build agent): the first boot and all 5 reboots show
  `dwc2d: LAN78xx link up after 1710 ms` (once 1840 ms) and **no
  `panicked`**. The network came up every time: 0 of 6 boots with test 98's
  `enumerate.rs:173:40` panic.
- Copies of the start-loop report and the reboot list are on homenodellm in
  `~/redox-rpi-rel/keep/` (`t102-report.md`, `t102-reboots.txt`).

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-419-g65391ae+x1a87`

Real Pi 3B+ over netboot. Experiment: `dwc2d-short-reply`. test 105 (dwc2d no longer panics on a short USB reply, after test 98), Pi test agent

**Pass: 5 of 5 boots without a panic, the network up every time.** Image
`~/redox-rpi-usb/keep/short-v0.3.0-dev-419-g65391ae+x1a87.img`. Boot 1 through a
serial-console reset, boots 2–5 through `echo reset > /scheme/sys/kstop`.

| boot | `panicked` | LAN | devsh | standard checks |
|---|---|---|---|---|
| 1 | 0 | `dwc2d: LAN78xx link up after 1710 ms` | ok | pass, 0 of 300 failed |
| 2 | 0 | the same | ok | pass, 0 of 300 |
| 3 | 0 | the same | ok | pass, 0 of 300 |
| 4 | 0 | the same | ok | pass, 0 of 300 |
| 5 | 0 | the same | ok | pass, 0 of 300 |

- No `dwc2d: request … bytes (attempt n)` line in any boot: no short reply came,
  so the retry didn't run. So this shows nothing breaks, not that the fix
  catches the case (test 98 hit it in 1 of 2 boots, rc2 in 0 of 6).
- The row expects `dwc2d: network on the adapter …`. This build prints `LAN78xx
  link up after … ms` instead.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-372-g6d1e186+x99f7`

Real Pi 3B+ over netboot. Experiment: `ns-list-snapshot`. test 100 ([#79](https://github.com/redox-rpi/redox-rpi/issues/79) complete `/scheme` listings), Pi test agent

**Pass: 3 of 3 boots.** Image
`~/redox-rpi-dwc2boot/keep/nslist-v0.3.0-dev-372-g6d1e186+x99f7.img`. Boot 1
through a serial-console reset, boots 2 and 3 through `echo reset >
/scheme/sys/kstop`.

| boot | `no network adapter found` | `ls -U -1 /scheme` | `network.` lines | standard checks |
|---|---|---|---|---|
| 1 | none | 37 entries, each once, sorted | 1 | pass, 0 of 300 failed |
| 2 | none | 37, each once, sorted | 1 | pass, 0 of 300 |
| 3 | none | 37, each once, sorted | 1 | pass, 0 of 300 |

The list (the same in all three boots): `chan data debug disk.live disk.mmc
display.vesa event fbcon.2 file icmp initfs input ip irq kernel.acpi kernel.dtb
log logging memory netcfg network.usb_lan78xx null pipe proc pty rand
rpi-firmware serio shm sudo sys tcp time udp uds_dgram uds_stream zero`.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-402-g787acab+x4ec1`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152`, `sshd`. test 101 ([#45](https://github.com/redox-rpi/redox-rpi/issues/45) SSH server rerun with the CPU fix and the session-end fix, PR #78), Pi test agent

**Pass.** Image `~/redox-rpi-relibc/keep/netboot-v0.3.0-dev-402-g787acab+x4ec1.img`.

- Boot: no `Unable to move`, no `no hostkeys available`. No `generating new host
  keys`, because the keys from test 81 are still on `/scheme/data/etc/ssh`, all 8
  files, private ones `-rw-------`. Standard checks pass, 0 of 300 starts failed.
- `polltest`: 5 × `poll N: ret 0 revents 0x0 after 2004–2010 ms; accept: Try
  again (os error 11)`: **pass** (before the fix: `ret 1` at once).
- `ssh root@192.168.178.148 'echo OK; id'`: `OK`, `uid=0(root) gid=0(root)`,
  **1.23 s** including the login. `ssh-session.exp` (Pi, port 22, key file):
  prompt, command, backspace, `exit` all ok, exit at once: **pass**.
- `sshd` CPU (`ps`, SCHTIME): 0.43 s about 1 minute after boot, 1.81 s after
  12 logins, 2.94 s after 22 sessions: **pass** (it doesn't grow by a second per
  second any more).
- 5 sessions `ssh -tt`, `echo hi` (answered `hi`), then `kill -9` of the local
  client; and 5 sessions `sleep 40`, client killed after about 12 s. The serial
  log has 10 lines `ptyd: pty … hangup, SIGHUP to groups [...]` (one group for
  the prompt cases, two with `sleep`) and **no `UNHANDLED EXCEPTION`**, no
  `panicked`. Afterwards `ps` shows no leftover `ion` or `sleep` from those
  sessions (only devsh's own `login`/`ion`): **pass**.

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-393-ga24b8b8+xcd36`

Real Pi 3B+ over netboot. Branch `spi`. test 99 ([#38](https://github.com/redox-rpi/redox-rpi/issues/38), [#39](https://github.com/redox-rpi/redox-rpi/issues/39), [#42](https://github.com/redox-rpi/redox-rpi/issues/42) GPIO, I2C and SPI drivers, the part without a person or wiring), Pi test agent

**Pass: 21 of 21.** Image `~/redox-rpi-cpu/keep/v0.3.0-dev-393-ga24b8b8+xcd36/harddrive.img`.

- Boot: `bcm2835-gpiod: 54 pins at 0x3f200000`, `bcm2835-i2cd: bus 1 at
  0x3f804000, 100 kHz (core 250 MHz)`, `bcm2835-spid: bus 0 at 0x3f204000,
  1000 kHz (core 250 MHz)`.
- Standard checks pass, 0 of 300 starts failed.
- `patches/experiments/gpio/nowire-check.py` (branch `spi`), no jumper, so
  the SPI part checks the MISO pull instead of the loopback:

```
PASS: GPIO 5 output 1 reads back: 1
PASS: GPIO 5 output 0 reads back: 0
PASS: GPIO 6 input with pull up reads 1: 1
PASS: GPIO 6 input with pull down reads 0: 0
PASS: GPIO 6 input with pull up reads 1: 1
PASS: activity LED (GPIO 29) takes writes
PASS: power_led takes off and on
PASS: pins table has 54 rows: 54 lines
PASS: I2C pins 2 and 3 in alt0: alt0 alt0
PASS: i2c detect finishes: answering: none
PASS: missing address 50 gives an error: i2c: read: No such device or address (os error 6)
PASS: SPI pins 7-11 in alt0: alt0 alt0 alt0 alt0 alt0
SPI: jumper pin 19 to 21 not found (['00 00 00'])
PASS: MISO pull up, 1000000 Hz, cs0: reads ff: ff ff ff ff
PASS: MISO pull up, 1000000 Hz, cs1: reads ff: ff ff ff ff
PASS: MISO pull up, 10000000 Hz, cs0: reads ff: ff ff ff ff
PASS: MISO pull up, 10000000 Hz, cs1: reads ff: ff ff ff ff
PASS: MISO pull down, 1000000 Hz, cs0: reads 00: 00 00 00 00
PASS: MISO pull down, 1000000 Hz, cs1: reads 00: 00 00 00 00
PASS: MISO pull down, 10000000 Hz, cs0: reads 00: 00 00 00 00
PASS: MISO pull down, 10000000 Hz, cs1: reads 00: 00 00 00 00
PASS: 4096-byte transfer finishes (wrong bytes expected without jumper): loopback: 4096 bytes at 1000000 Hz, 4082 wrong spi: loopback failed (MOSI pin 19 to MISO pin 21?)
21 of 21 checks passed
```

Stable image restored afterwards.

## 2026-10-30: `v0.3.0-dev-172-gc7bee13+xb847`

Real Pi 3B+ over netboot. Experiments: `timesync`, `assistant`. test 106 ([#43](https://github.com/redox-rpi/redox-rpi/issues/43) Qwen3 0.6B with `rinfer` and the assistant), Pi test agent

**Fail: the first `rinfer` run never answered, and the kernel then panicked with
`oom` and halted the Pi.** Image
`~/redox-rpi-assistant/keep/netboot-v0.3.0-dev-172-gc7bee13+xb847.img`.

- Standard checks pass, 0 of 300 starts failed. `free`: 606,764 KB free of
  745,252. `/scheme/data`: 898 MB free.
- `rinfer --download …partNNN /scheme/data/Qwen3-0.6B-Q4_0.gguf`: the file
  reached the right size, **382,156,480 bytes**, at 12:08 UTC after about 26
  minutes (from 11:42, about 0.24 MB/s). The part list was cut off by my
  devsh tool's 1500 s timeout.
- `rinfer /scheme/data/Qwen3-0.6B-Q4_0.gguf 'What is the capital of
  France?' 32` (started 12:09): it printed only `neon: true, threads: 4`, and
  no `loaded in … s` within 20 minutes. From a second devsh, `free` showed
  550 → 535 MB free over the first 2 minutes, and `rinfer`'s PRIVATE grew
  only 7 → 22 MB while it used 2 min 11 s of CPU (SCHTIME). So it was loading
  the model very slowly. Later devsh calls timed out.
- The serial log then has `KERNEL PANIC: panicked at src/memory/mod.rs:1459:34:
  not yet implemented: oom` in a page fault of `/usr/bin/rinfer`, then the same
  panic in `logd` and `netstack`, and `HALT`. The last serial line came at
  about 12:50 UTC. Soft temperature warnings at 60.1 °C came often the whole
  time. The Shelly plug is on (4.3 W). The Pi needs a power cycle (the user's
  OK).
- So no answer, no tokens per second, no assistant or `llama-bench` run. The
  model file stays on `/scheme/data` for the rerun.
- `/srv/tftp/redox` has the stable image again for the next boot.

## 2026-10-30: `v0.3.0-dev-426-g5dc3790+x19b9`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152`, `boot-time`. test 107 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47) boot time, rerun of test 84 on today's main), Pi test agent

**Pass: 3 of 3 boots came up with network and the SD card, `redox login:`
about 21 s after the reset.** Staged with the owner's
`~/t18-work/bt8-stage/scripts/netboot.sh` (bootloader md5 `1780eeee…`),
image `~/t18-work/image-bt8/harddrive.img`.

| Boot | `redox login:` after `kreset` | `40_bcm2835-sdhostd.service ready` | `40_dwc2d.service ready` |
|---|---|---|---|
| 1 | 21.1 s | 0.42 s (196 ms) | 6.46 s (6229 ms) |
| 2 | 21.2 s | 0.41 s (184 ms) | 6.53 s (6267 ms) |
| 3 | 21.0 s | 0.55 s (285 ms) | 6.51 s (6236 ms) |

- In every boot the network came up: DHCP bound, no
  `Can't open /scheme/netcfg/ifaces/eth0/mac`, devsh answered. No panic.
- `bcm2835-sdhostd` served the card in every boot (SDHC 15193 MiB, 4-bit,
  50 MHz).
- Test 84 had 21.4 s; this is the same or a little faster. U-Boot plus the
  43 MB image over HTTP take about 15 s of it.
- Standard checks (`--manifest …netboot-v0.3.0-dev-426-g5dc3790+x19b9.toml`)
  pass: version, manifest, ps, ps-tty, `/scheme/data` write and read back,
  0 of 300 starts failed.
- `/srv/tftp/redox` has the stable image again for the next boot.

## 2026-10-30: `v0.3.0-dev-463-gac3dad4+x125f`

Real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `timesync`, `timerd`, `mqtt`, `ha-bridge`, `mqtt-tls-test`. test 111 ([#56](https://github.com/redox-rpi/redox-rpi/issues/56) memory with MQTT over TLS, rerun of #89), Pi test agent

**Pass: free memory fell 0.21 MB a minute over 30 minutes (#89: 5.5 MB a
minute; goal under 1).** Image
`~/mqtt-test/keep/leak-netboot-v0.3.0-dev-463-gac3dad4+x125f.img`, manifest
matches. TLS broker `~/mqtt-broker/pi-test-broker-tls.sh` on homenodellm.

- Standard checks pass, 0 of 300 starts failed.
- `echo 192.168.178.120 > /etc/mqtt-host` at 18:14:07 UTC; 25 s later the
  serial log has `ha-bridge: 6 sensors announced on 192.168.178.120:8883 (TLS)
  as redox-rpi/redox`. The broker got the state once a minute, 37 times in
  31 minutes.
- Readings, one devsh command `free; cat /scheme/sys/context` every 3 minutes:

| Time (UTC) | free KB | `netstack` PRIVATE | `ha-bridge` | `redoxfs` | `timerd` | `devshd` | `randd` SHARED |
|---|---|---|---|---|---|---|---|
| 18:14:38 | 592,640 | 3.97 MB | 8.59 MB | 34.3 MB | 5.47 MB | 11.2 MB | 128 KB |
| 18:20:38 | 591,392 | 4.22 MB | 8.59 MB | 34.3 MB | 5.52 MB | 11.3 MB | 200 KB |
| 18:26:38 | 590,148 | 4.59 MB | 8.59 MB | 34.3 MB | 5.57 MB | 11.3 MB | 272 KB |
| 18:32:38 | 588,880 | 4.97 MB | 8.59 MB | 34.3 MB | 5.61 MB | 11.4 MB | 344 KB |
| 18:38:38 | 587,624 | 5.34 MB | 8.59 MB | 34.3 MB | 5.66 MB | 11.4 MB | 416 KB |
| 18:44:38 | 586,364 | 5.72 MB | 8.59 MB | 34.3 MB | 5.71 MB | 11.4 MB | 488 KB |

- In all: 6,276 KB less free in 30 minutes, very even (about 630 KB per
  3 minutes). The `ha-bridge` service and `redoxfs` do not grow.
- What still grows, slowly: `netstack` PRIVATE +1.75 MB (about 58 KB a
  minute, the biggest part), `randd` SHARED +360 KB (12 KB a minute, one
  random read per TLS run), `timerd` +240 KB, `devshd` +200 KB (my readings),
  `procmgr` SHARED +56 KB, `zerod` SHARED +80 KB. These add up to about 2.7 MB
  of the 6.1 MB.
- Since the loss is far below 1 MB a minute, I did not kill the service.
- Reset at 18:45:16 to end the test: `redox-rpi/redox/availability offline` at
  the broker at 18:46:02, **46 s** later (the row expected about 30 s; likely
  the 60 s keep-alive). Broker stopped; it ran without persistence, so no
  retained topics stay.
- `/srv/tftp/redox` has the stable image again; stable booted to `redox login:`.

## 2026-10-30: `v0.3.0-dev-175-g29052d7+x4590`

Real Pi 3B+ over netboot. Experiments: `timesync`, `assistant`, `assistant-netboot`. test 112 ([#43](https://github.com/redox-rpi/redox-rpi/issues/43) assistant, rerun of #106), Pi test agent

**Pass, with one finding: no `oom`, `rinfer` answers Paris and both
assistant questions run the right tool. Writing is slow, about 1 token a
second (goal 2). One `devshd` thread hit a `RELIBC PANIC` while memory was low.**
Image `~/redox-rpi-assistant/keep/netboot-v0.3.0-dev-175-g29052d7+x4590.img`,
manifest matches. The Pi had a power cycle (by the user) after #106.

- Standard checks pass, 0 of 300 starts failed. `free`: **738,408 KB free**
  of 876,068 (#106: 607 MB of 745). The old `Qwen3-0.6B-Q4_0.gguf` from #106
  was still there; removed.
- `rinfer --download …partNNN`: `part 000` to `part 020`,
  `341455552 bytes in 1497 s` (about 25 minutes, 0.23 MB/s).
- `rinfer … 'What is the capital of France?' 32`: `loaded in 77.8 s (539
  reads of the file)`, heap 375 MiB (peak 377), answer
  `The capital of France is **Paris**.`, prompt 23 tokens at **2.19 t/s**,
  generated 9 tokens at **1.03 t/s**, heap peak 378 MiB. `free` from a
  second devsh: 689 → 365 → 298 → 192 MB free while it loaded and wrote.
- `assistant --ask 'What time is it?'` (`ASSISTANT_DEBUG=1`), 451 s in all:
  loaded in 77.0 s; `rinfer: read 380 tokens in 168.3 s` (system prompt),
  read 22 tokens in 14.4 s, ran **`show_time`** (wrote 2 tokens at 0.94 t/s),
  tool result `It is Friday, 30 October 2026, 19:24 UTC.`, read 40 tokens in
  25.5 s, answer `The current time is Friday, 30 October 2026, 19:24 UTC.`
  (24 tokens in 163.3 s, **0.15 t/s**). Heap peak 430 MiB. `date` right after:
  19:27:43.
- `assistant --ask 'Which files are in my home folder?'`, 338 s: loaded in
  89.2 s, read 380 tokens in 171.7 s, ran **`list_files`** (`~ has 3 entries:
  .assistant.log .config/ .local/`), answer lists `.config`, `.local`, `.log`
  (19 tokens in 30.8 s, 0.62 t/s; it shortened `.assistant.log`). Heap peak
  430 MiB.
- `free` every 30 s from a second devsh during both questions (26 readings):
  lowest **90,192 KB free** (19:31:53), mostly 105–120 MB; never under 50 MB.
  After the run: 609,704 KB free.
- `temperature` 66604 (66.6 °C), `throttled` 0x00080008 (soft temperature
  limit now and before). Soft-limit warnings at 59–61 °C came often on
  serial; the clock stayed at 1400 MHz.
- No kernel panic, no `oom`. **Finding:** during the assistant run the serial
  log has `RELIBC PANIC: panicked at src/sync/mod.rs:103:54: called
  Result::unwrap() on an Err value: Errno(14)` and then `UNHANDLED EXCEPTION
  … PID 37, NAME /usr/bin/devshd` (a guard-page fault). PID 37 is a devshd
  thread; devshd went on answering, and the test went on. Errno 14 is EFAULT.
  It happened while free memory was near its lowest.
- `Qwen3-0.6B-Q4_0e.gguf` removed at the end. `/srv/tftp/redox` has the
  stable image again.

## 2026-10-30: `v0.3.0-rc3` (`v0.3.0-dev-453-g036c039+x12cb`)

Real Pi 3B+ over netboot. `--release v0.3.0-rc3 --experiment timesync`, main 036c039. test 113 (v0.3.0 rc3 automated, checklist tests 1–5 and 11), Pi test agent

**Pass on 5 of 6 checks, and no `dwc2d` panic in 6 of 6 boots. Check 11 lost
characters in 2 of 20 lines on the first run, then passed 20 of 20 on a
rerun.** Image `~/redox-rpi-rel/keep/v0.3.0-rc3-netboot.img`, staged with the
build agent's `scripts/netboot.sh` (its bootloader and manifest). Afterwards
the saved stable files went back into `/srv/tftp/redox`, and stable booted.

| # | check | result |
|---|---|---|
| 1 | `pi-test.py --starts 1000` (`t10-start-loop` copy, 61ea421) | **pass**: `PASS version: v0.3.0-rc3 (v0.3.0-dev-453-g036c039+x12cb)`, manifest, ps (42), ps-tty, sdcard all pass, **0 of 1000 starts failed** |
| 2 | clock | **pass**: `rpi-firmwared: ARM clock 600 -> 1400 MHz`, no throttle, under-voltage or temperature warning in any boot; `/scheme/rpi-firmware/clocks`: `arm 1400000000` |
| 3 | SD driver, no PC services | **pass**: `bcm2835-sdhostd: SDHC/SDXC card, 15193 MiB, 4-bit bus, high speed, up to 50 MHz`, `serving /scheme/disk.mmc`; no `bcm2835-sdhcid`, `pcid`, `pcid-spawner`, `audiod` or `rtcd` line |
| 4 | time | **pass**: `timesync: clock set to … from pool.ntp.org`; `date +%s` on the Pi equal to the host's (0 s apart) |
| 5 | `reboot` over devsh × 5 | **pass**: each back to `redox login:` after 22–24 s, devsh answers (one boot needed one retry, 3 s) |
| 11 | `serial-input-test.sh 20` (logged in on the console) | **first run 18 of 20** (`S18 LOST characters` and one more line), no getty panic; **rerun 20 of 20** |

- **Every boot** (the first and the 5 reboots, 6 in all): no `dwc2d` panic
  (no `enumerate.rs`, no `panicked`), `dwc2d: 5 device(s) enumerated`, the
  network came up (devsh answered each time).
- Check 11: rc2 (#102) had 20 of 20. Lost characters at full speed came
  before too (#28); this time 2 lines in the first run, none in the second.

## 2026-10-30: `v0.3.0-dev-452-g64a4e94+x7d9f`

Real Pi 3B+ over netboot. Experiments: `gpio`, `bluetooth`. test 109 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54) Bluetooth diagnosis after #90), steps 1–3, Pi test agent

**The chip answers with no flow setting. Nothing comes back, the TX FIFO
empties (except with `auto-low`), and CTS reads 1 the whole time. Step 4 (a
second boot with `dtoverlay=miniuart-bt`) waits for the user: it changes the
shared `/srv/tftp/config.txt`, which my permission check does not allow on its
own.** Image `~/redox-rpi-cpu/keep/v0.3.0-dev-452-g64a4e94+x7d9f/harddrive.img`
(no manifest).

- Boot log: `bcm2835-btuartd: mini UART at 0x3f215000, 115313 baud (core 250 MHz)`.
- (1) `bt diag`, the whole output:

```
gpio 30: mode alt5, level 1, pull up
gpio 31: mode alt5, level 1, pull none
gpio 32: mode alt5, level 1, pull none
gpio 33: mode alt5, level 1, pull up
gpio 43: mode alt0, level 1, pull unknown
bt_power: 0
power: cycled
uart: core clock 250000000 Hz, 115313 baud
flow auto:
before: cts_line 1, rts 1, tx_fifo 0, rx_fifo 0, cntl 0x3f, msr 0x00
after: cts_line 1, rts 1, tx_fifo 0, rx_fifo 0, cntl 0x3f, msr 0x00
received: nothing
flow auto-low:
before: cts_line 1, rts 0, tx_fifo 0, rx_fifo 0, cntl 0xff, msr 0x00
after: cts_line 1, rts 0, tx_fifo 4, rx_fifo 0, cntl 0xff, msr 0x00
received: nothing
flow none:
before: cts_line 1, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x00
after: cts_line 1, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x00
received: nothing
no answer with any flow setting
```

- (2) None answered, so `echo none > /scheme/btuart/flow; bt up` once:
  `power: cycled`, `uart: core clock 250000000 Hz, 115313 baud`,
  `bt: no answer to HCI command 0c03`.
- (3) `cat /scheme/btuart/status`: `cts_line 1`, `rts_status 0`, `tx_fifo 0`,
  `rx_fifo 0`, `tx_done 1`, `lsr 0x60`, `msr 0x00`, `cntl 0x03`,
  `stat 0x0000038e`.
- Reading by the row's guide: with `auto` and `none` the 4 bytes of HCI Reset
  left the FIFO (`tx_fifo 0`), and nothing at all came back. That points at
  the pins or the chip, not the baud rate. With `auto-low` the bytes stayed
  (`tx_fifo 4`), because CTS reads 1 and that setting waits for a low CTS. So
  CTS stays high the whole time, and the chip never pulls it low, also after
  the power cycle. GPIO 43 (`bt_power` pin, alt0) reads level 1 though
  `bt_power: 0`.
- (4) Not done yet: it needs `dtoverlay=disable-bt` → `dtoverlay=miniuart-bt`
  in the shared `/srv/tftp/config.txt` for one boot, and the user's OK.
- Note: before this test, stable (booted at 19:51 after #113) had lost its
  network within a few minutes: no devsh, no ping, while the serial console
  still answered. I reset it from the serial console.
- `/srv/tftp/redox` has the stable image again; stable booted.

## 2026-10-30: `v0.3.0-dev-458-ga97f08a+x12cb`

Real Pi 3B+ over netboot. Experiments: `timesync`, base `0049`. test 114 ([#88](https://github.com/redox-rpi/redox-rpi/issues/88) netstack leak per accepted TCP connection), Pi test agent

**Pass: netstack grew 1.12 MB over 120 connections, and not batch by batch
(stable: 5.25 MB for one batch of 40).** Image
`~/redox-rpi-zrtl/keep/leak-fix-v0.3.0-dev-458-ga97f08a+x12cb.img`.

- Standard checks pass, 0 of 300 starts failed.
- 120 devsh connections from homenodellm (connect to 2323, wait for
  `login:`, close, 0.3 s apart), 3 batches of 40; every connection got
  `login:` (0 failed). netstack PRIVATE from one devsh `cat /scheme/sys/context`:

| Reading | Time (UTC) | netstack PRIVATE |
|---|---|---|
| start | 20:04:59 | 3.41 MB |
| after 40 | 20:05:17 | 4.41 MB |
| after 80 | 20:05:36 | 4.41 MB |
| after 120 | 20:05:54 | 4.53 MB |
| 1 min later | 20:06:56 | 4.53 MB |

  The same steps as QEMU (3.41 → 4.28 → 4.28 → 4.41 MB).
- Stable image (`v0.3.0-dev-15-g24fbb96+x9f76`) for comparison, one batch of
  40 right after boot: 3.47 → **8.72 MB** (+5.25 MB), 8.84 MB a minute later;
  1 of 40 connections got no `login:`.
- Maybe related: tonight stable twice lost its network a few minutes after
  booting (no ping, no devsh, the serial console still worked), while the test
  images in between kept theirs. If something on the LAN opens many
  connections to the Pi, this leak would explain it. Not checked.
- `/srv/tftp/redox` has the stable image again; stable booted.

## 2026-10-30: `v0.3.0-dev-481-gcfa7d09+x9d2b`

Real Pi 3B+ over netboot. Experiments: `ldso-diagnostics`, `timesync`, `timerd`, `timerd-test`, `timezone`. test 95 ([#46](https://github.com/redox-rpi/redox-rpi/issues/46) local time Europe/Berlin, and rerun of #49's freeze), Pi test agent

**Pass: all time zone outputs as expected, and no freeze in 20 minutes
(slowest timer run 5.3 s, #49: 438 s). One note: the Pi did not answer ping
from homenodellm, though devsh answered every minute.** Image
`~/tz-images/keep/tz-netboot-v0.3.0-dev-481-gcfa7d09+x9d2b.img`, manifest
matches.

- Standard checks pass, 0 of 300 starts failed.
- Boot log: `timesync: clock set to 2026-10-30 20:11:23 UTC from pool.ntp.org`,
  `timerd: clock set by timesync, 2026-10-30 21:11:23 CET`.
- Over devsh:
  - `date` `Fri Oct 30 21:15:02 CET 2026`, `date -u` `Fri Oct 30 20:15:02 UTC 2026`
  - `ls -l /etc/localtime` `-> /usr/share/zoneinfo/Europe/Berlin`
  - `date -d '@1792889999'` `Sun Oct 25 02:59:59 CEST 2026`;
    `'@1792890000'` `Sun Oct 25 02:00:00 CET 2026`;
    `'@1774746000'` `Sun Mar 29 03:00:00 CEST 2026`
  - `timerd -t …` six lines: `02:30:00 CEST (+02:00)`, `02:59:59 CEST`,
    `02:00:00 CET (+01:00)`, `02:30:00 CET` on 25 October;
    `01:59:59 CET (+01:00)`, `03:00:00 CEST (+02:00)` on 29 March
  - `env TZ=UTC timerd -t 1792890000` `2026-10-25 01:00:00 UTC (+00:00)`
- **Freeze check**, 21:11:23–21:32 CET (20 minutes): 33 timer runs, all
  `exited with code 0`, the slowest 5.3 s (`test-catchup` at 21:28, when it
  started in the same second as `test-minute`), most under 1 s. A `date` line
  came every minute, 21:12:00 to 21:32:00, no gap. `echo pong` over devsh
  answered all 16 times. No panic.
- **Ping:** my ping from homenodellm (ICMP, works against the router) got no
  answer in any of the 16 minutes. `ping -c 2 192.168.178.120` from the Pi
  works, 0 % loss. So the Pi is reachable and the network works, but it does
  not answer incoming ping. I did not look into it further.
- `/srv/tftp/redox` has the stable image again; stable booted.

## 2026-10-30: `v0.3.0-dev-456-gfe96015+x204a`

Real Pi 3B+ over netboot. Experiments: `timesync`, `pkg-hold`, `pkg-repo`, `sigreturn-svc`. test 115 ([#44](https://github.com/redox-rpi/redox-rpi/issues/44) `pkg update` from both repositories), Pi test agent

**Pass: 15 upstream packages updated, none of ours, our files unchanged,
300 starts afterwards without a failure, nano and bash install and run,
`bash-ok`.** Image `~/redox-rpi-boot/keep/repoG-harddrive.img`, repository
`http://192.168.178.120:8080/pkg` from the same build.

- Standard checks pass, 0 of 300 starts failed.
- `pkg update --yes` at **20:37:18 UTC**, 11.5 s: `Packages to update:`
  ca-certificates, coreutils, diffutils, extrautils, ion, kibi, libgcc,
  libonig, libstdcxx, netdb, netutils, redoxfs, userutils, uutils,
  uutils-procps (15; 12.9 MB download, 43.2 MB install). None of base,
  kernel, relibc, pkgutils, reboot. No `Some transactions are ignored`. Ends
  with `Commit complete.` (Several `TODO: setsockopt(…, 6, 1, …) - unknown
  option` info lines from relibc, harmless.)
- `sha256sum` before and after, unchanged: kernel `d1e4f5d9…`, `libc.so`
  `7ec8c067…`, `pkg` `9d058799…`, `reboot` `ff39315b…`.
- `pkg info coreutils`: `remote: static.redox-os.org`. `pkg info pkgutils`:
  `remote: 192.168.178.120` (0.3.3, build time 2026-10-30T20:01:51Z).
- Second `pkg update --yes`: only `Fetch complete.`, nothing to do (0.9 s).
- Standard checks again with the updated upstream programs: all pass, **0 of
  300 starts failed**.
- `pkg install --yes nano`, `nano --version`: `GNU nano, version 9.2`.
- `pkg install --yes bash` (27 files), then
  `bash -c "for i in {1..200}; do ls / > /dev/null; done; echo bash-ok"`:
  **`bash-ok`** after 31.6 s.
- No panic and no unhandled exception on the serial console.
- `/srv/tftp/redox` has the stable image again.

## 2026-10-30: `v0.3.0-dev-200-gd73095c+xced2`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152`, `dwc2-interrupts`, `rpi-fb`. test 116 ([#33](https://github.com/redox-rpi/redox-rpi/issues/33) rpi-fb, serial only), **boot 1 only**, Pi test agent

**Boot 1 passes, with one note: `rpi-fbd` could not get its 1824x984 buffer
(`Out of memory`). Boots 2 and 3 are not done: they need typing at the
bootloader menu (`e`), and my rules allow typing on the console only after
`redox login:`.** Image
`~/redox-rpi-graphics/keep/netboot-v0.3.0-dev-200-gd73095c+xced2.img`,
manifest matches.

- `rpi-fbd: EDID 256 bytes, monitor NanoKVM`
- `rpi-fbd: boot framebuffer 1824x984 stride 1824 at 0x3e914000`
- `rpi-fbd: DMA channel 5 at 0x3f007000, 2D mode moves YLENGTH+1 rows`
  (YLENGTH+1, like QEMU)
- **`rpi-fbd: dumb buffer 1824x984 in normal memory: Out of memory`**, twice
  (once before and once after `fbcond: Performing handoff`). `free` then shows
  591,580 KB free, so this is not the system running out. Probably the
  allocation of one buffer that size fails, while `vc_memory` is only 16 MiB.
- `fbcond: Performing handoff`; no `rpi-fbd` panic, no `DMA error` or
  `DMA timeout`.
- `cat /scheme/rpi-firmware/board`: `revision 0x00a020d3`,
  `model Raspberry Pi 3 Model B+`, `memory 1 GiB`, firmware `0x6aa809d0`,
  `arm_memory 0x00000000 1046478848`, `vc_memory 0x3e600000 16777216`.
- Standard checks pass, 0 of 300 starts failed.
- `/srv/tftp/redox` has the stable image again.

## 2026-10-30: `v0.3.0-dev-475-gbcd8e4d+xbe97`

Real Pi 3B+ over netboot. Experiments: `timesync`, `sdhost-dma`. test 117 ([#30](https://github.com/redox-rpi/redox-rpi/issues/30) sdhost with DMA, rebuilt on main), Pi test agent

**Pass for the data: DMA comes up in all 4 boots without an error, the file
reads back right after a reset, sd-stress passes. But the write is slower and
uses more driver CPU than in #75.** Image
`~/redox-rpi-sd/keep/netboot-v0.3.0-dev-475-gbcd8e4d+xbe97.img`, manifest
matches.

- Boot log (all 4 boots): `bcm2835-sdhostd: DMA interrupt
  /scheme/irq/phandle-1/1,20`, `DMA channel 4 at 0x3f007000`,
  `SDHC/SDXC card, 15193 MiB, 4-bit bus, high speed, up to 50 MHz`; no
  `cannot use DMA interrupt`, `DMA error`, `DMA timeout` or `no DMA`, no panic.
- 256 MiB on `/scheme/data`, `bs=1048576`, from `/dev/urandom` (as in #75).
  `ps` columns of `/scheme/initfs/bin/bcm2835-sdhos`:

| | time | MB/s | ACTTIME | SCHTIME | #75 |
|---|---|---|---|---|---|
| write `conv=fsync` | 48.6 s | **5.5** | +28.6 s (1.70 → 30.34) | **+16.5 s** (0.49 → 16.96) | 7.7 MB/s, SCHTIME +4.3 s in 35 s |
| read | 15.1 s | 18 | +13.8 s (30.34 → 44.14) | +1.0 s (16.96 → 17.98) | 18 MB/s, +1.1 s in 15 s |
| `sha256sum` after the reset | 21.9 s | | 10.02 after boot and read | 2.90 | |

- A first try with `/dev/zero` wrote at 90 MB/s and read at 858 MB/s: the
  filesystem does not store zeros on the card, so those numbers mean nothing.
- SHA-256 before and after `echo reset > /scheme/sys/kstop`: the same,
  `d77d9aec746f19c5…`. File removed afterwards.
- Standard checks pass, manifest matches, 0 of 300 starts failed.
- `scripts/sd-stress.py --cycles 2 --serial-log …`: **pass**. 256 MiB at
  6.6 MB/s, 1000 small files with 250 replaced in 275 s, 2 restarts, 0 missing
  and 0 changed after each; cycles wrote 32 MiB at 4.3 and 7.0 MB/s.
- So the driver's real CPU time (SCHTIME) during the write is about a third of
  the write time (16.5 of 48.6 s), much more than #75's 12 %, while reading
  stays at 7 %.
- `/srv/tftp/redox` has the stable image again.

## 2026-10-30: `v0.3.0-dev-359-ga6966cc+x810f`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152`, `pty-sigint` (Ion patch `0002`). test 118 ([#86](https://github.com/redox-rpi/redox-rpi/issues/86) Ctrl-C on the serial console right after Enter), Pi test agent

**Pass: on the serial console all 12 Ctrl-Cs stopped `sleep 40` within about
0.1 s (#80: about 32 s), over devsh 10 of 10.** Staged with
`~/t18-work/c86-stage/scripts/netboot.sh` (bootloader md5 `4fccdb94…`), image
`~/t18-work/image-c86/harddrive.img` (md5 `caaff094…`).

- Serial console, logged in as root, delays
  0.05, 0.1, 0.2, 0.3, 0.5, 1 s after Enter, twice: **every round
  `stopped by SIGINT 0.1 s after Ctrl-C`**, `PASS: 0 late of 12`. The serial
  log has 12 times `ion: pipeline execution error: process (…) ended by signal
  SIGINT`. No round fell in the raw-mode window (no `line cancelled`).
- How: not the branch's `serial-ctrlc-test.py` itself, which reads
  `/dev/ttyUSB0` directly and types at full speed. Instead the same rounds
  with the same timing and checks (`~/pitest-tools/ctrlc118.py`): the logging
  service stayed on, the script read its file, and it typed `sleep 40` one
  character every 0.15 s. The delay counts from the Enter, as in the
  original, and the Ctrl-C is one byte.
- devsh: `patches/experiments/pty-sigint/ctrlc-test.py 192.168.178.148
  --rounds 10` (a6966cc): 10 of 10 `stopped by SIGINT 0.0 s after Ctrl-C`
  (5 after 1 s, 5 after 3 s), Ctrl-C at the prompt leaves the shell answering:
  **PASS**.
- Standard checks with `--manifest`: all pass, 0 of 300 starts failed.
- No panic or unhandled exception on serial.
- `/srv/tftp/redox` has the stable image again; stable booted at 21:02 UTC.

## 2026-10-30: `v0.3.0-dev-454-g06437b9+x7d9f`

Real Pi 3B+ over netboot. Experiments: `gpio`, `bluetooth`. test 109 step 5 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54) Bluetooth, the 32 kHz sleep clock on GPIO 43), Pi test agent

**The sleep clock runs: GPIO 43 sampled 102 high and 98 low, and the clock
reports `enabled 1, busy 1, rate_hz 32768`. The chip still answers with no flow
setting, so a missing clock is not the reason.** Image
`~/redox-rpi-cpu/keep/v0.3.0-dev-454-g06437b9+x7d9f/harddrive.img`.

- Boot log: `bcm2835-btuartd: mini UART at 0x3f215000, 115313 baud (core 250 MHz)`.
- The two new lines:
  - `gpio 43 sampled 200 times: 102 high, 98 low`
  - `lpo: enabled 1, busy 1, source 1, divi 585, divf 3840, rate_hz 32768, ctl 0x00000291, div 0x00249f00`
- The rest is the same as steps 1–3: GPIO 30–33 alt5 (pulls up/none/none/up),
  GPIO 43 alt0; `bt_power: 0`, `power: cycled`, 115313 baud; `auto` and
  `none` leave `tx_fifo 0`, `auto-low` keeps `tx_fifo 4`; `cts_line 1` before
  and after every Reset; `received: nothing` each time;
  `no answer with any flow setting`.
- Step 4 still waits for the user's OK.
- `/srv/tftp/redox` has the stable image again.

## 2026-10-30: `v0.3.0-dev-407-g741b667+x4ec1`

Real Pi 3B+ over netboot. Experiments: `timesync`, `rtl8152`, `sshd`. test 120 ([#45](https://github.com/redox-rpi/redox-rpi/issues/45) SSH server leftovers), Pi test agent

**Pass: 3 host key types, key login from the data partition, still working
after a reset with the same host keys.** Image
`~/redox-rpi-relibc/keep/netboot-v0.3.0-dev-407-g741b667+x4ec1.img` (no
manifest next to it).

- (1) `ssh-keyscan -t rsa,ecdsa,ed25519 192.168.178.148`: **3 keys**,
  `ssh-rsa`, `ecdsa-sha2-nistp256`, `ssh-ed25519` (#81: RSA only).
  `ssh-keyscan 192.168.178.148` without `-t` (about 30 s later): the same 3
  types. No dropped connection.
- (2) `/home/root/.ssh/authorized_keys` does not exist on the image. A new
  test key (`ed25519`, no comment) went into
  `/scheme/data/home/root/.ssh/authorized_keys` (after `mkdir -p`).
  `ssh -i <key> root@192.168.178.148 echo ok`: `ok` in 1.2 s.
- (3) After `echo reset > /scheme/sys/kstop`: the same key login works
  (`ok`, `uid=0(root) gid=0(root)`) without adding the key again. Host key
  fingerprints before and after the reset are the same: RSA
  `SHA256:nggT0H31…`, ECDSA `SHA256:MXb/OY+z…`, ED25519 `SHA256:6b9SpKQH…`. No
  `generating new host keys` on serial.
- (4) `ssh-session.exp` with that key: prompt, command, backspace, `exit` all
  ok, exit at once. `sshd` SCHTIME 0.85 s before, 2.15 s after 11 sessions
  (#101: 1.81 s after 12): about the same.
- Standard checks pass, 0 of 300 starts failed. No panic or unhandled
  exception apart from the known pcid one.
- The test key file removed from `/scheme/data/home/root/.ssh/`. Stable image
  staged again.

## 2026-10-31: `v0.3.0-dev-482-g1ff88ec+x056b` (dwc2d-netstats), long idle run

Real Pi 3B+ over netboot. Test 119 ([#92](https://github.com/redox-rpi/redox-rpi/issues/92)), Pi test agent.

**Pass: the network stayed up for the whole run, 4 h 13 min (21:12 to 01:25 UTC).**
Image `~/redox-rpi-dwc2boot/keep/netstats-v0.3.0-dev-482-g1ff88ec+x056b.img`,
its manifest, staged with the build agent's `scripts/netboot.sh`. The run was
shortened from 03:00 to 01:30 UTC so tests 109 step 6 and 121 fit before 03:15.
Afterwards the saved stable files went back into `/srv/tftp/redox`.

- 25 readings, one every 10 to 11 minutes, each a single devsh connection: **all
  25 answered**, eth0 kept `192.168.178.148/24` and the default route every time,
  each `cat` of `addr/list` took 0.4 s, and **`ping -c 3 192.168.178.1` from the
  Pi had 0 % loss every time**.
- No devsh failure, so no serial readings or reset were needed.
- The host's ping to the Pi got no answer in all 260 tries (one a minute). This is
  the same as earlier tests: the Pi pings out fine but answers no ICMP from
  homenodellm. So this ping says nothing about #92.
- Counters: `uptime_ms`, `rx_frames` and `irqs` grew steadily all night
  (about 7,200 frames per 10 minutes). **`rx_errors` went to 1 once**
  (`rx_last_error 0x92`, at `uptime_ms` 7,449,997, about 23:15 UTC) and stayed at 1;
  traffic went on normally. `tx_errors` stayed 0.
- Memory: dwc2d stayed at 1.70 MB. netstack grew from 3.34 MB to 6.72 MB, about
  0.14 MB per devsh connection. That matches the leak #114 measured on stable
  (5.25 MB per 40 connections); this image doesn't have the #114 fix. `free`
  went down by 10 MB (609,916 to 599,576 KB).
- dwc2d CPU time: ACTTIME 3 min 42 s after 2 h 47 min uptime (about 2 % of one core).

| UTC | uptime_ms | rx_frames | rx_err | tx_frames | tx_err | irqs | netstack | dwc2d | free KB | router ping loss |
|---|---|---|---|---|---|---|---|---|---|---|
| 21:17 | 310673 | 2438 | 0 | 270 | 0 | 2393 | 3.34 MB | 1.70 MB | 609916 | 0 % |
| 21:18 | 360775 | 2866 | 0 | 359 | 0 | 2816 | 3.72 MB | 1.70 MB | 608956 | 0 % |
| 21:29 | 1007086 | 7489 | 0 | 459 | 0 | 7387 | 3.91 MB | 1.70 MB | 608072 | 0 % |
| 21:39 | 1653398 | 12113 | 0 | 552 | 0 | 11998 | 4.09 MB | 1.70 MB | 607672 | 0 % |
| 21:50 | 2294699 | 16701 | 0 | 662 | 0 | 16565 | 4.09 MB | 1.70 MB | 607292 | 0 % |
| 22:01 | 2930992 | 21173 | 0 | 749 | 0 | 20982 | 4.28 MB | 1.70 MB | 606868 | 0 % |
| 22:12 | 3587313 | 25823 | 0 | 878 | 0 | 25558 | 4.47 MB | 1.70 MB | 606416 | 0 % |
| 22:22 | 4223604 | 30311 | 0 | 962 | 0 | 29982 | 4.47 MB | 1.70 MB | 605988 | 0 % |
| 22:33 | 4869915 | 34892 | 0 | 1075 | 0 | 34503 | 4.66 MB | 1.70 MB | 605612 | 0 % |
| 22:44 | 5516224 | 39603 | 0 | 1179 | 0 | 39158 | 4.84 MB | 1.70 MB | 605236 | 0 % |
| 22:55 | 6162527 | 44292 | 0 | 1278 | 0 | 43798 | 4.84 MB | 1.70 MB | 604868 | 0 % |
| 23:05 | 6803824 | 48864 | 0 | 1360 | 0 | 48298 | 5.03 MB | 1.70 MB | 604488 | 0 % |
| 23:16 | 7450127 | 53530 | 1 | 1478 | 0 | 52902 | 5.22 MB | 1.70 MB | 604120 | 0 % |
| 23:27 | 8091426 | 58126 | 1 | 1578 | 0 | 57454 | 5.22 MB | 1.70 MB | 603752 | 0 % |
| 23:38 | 8732724 | 62752 | 1 | 1662 | 0 | 62023 | 5.41 MB | 1.70 MB | 603368 | 0 % |
| 23:48 | 9374025 | 67380 | 1 | 1763 | 0 | 66597 | 5.59 MB | 1.70 MB | 602988 | 0 % |
| 23:59 | 10010311 | 71985 | 1 | 1864 | 0 | 71132 | 5.59 MB | 1.70 MB | 602612 | 0 % |
| 00:10 | 10656619 | 76602 | 1 | 1972 | 0 | 75724 | 5.78 MB | 1.70 MB | 602232 | 0 % |
| 00:20 | 11297915 | 81197 | 1 | 2062 | 0 | 80303 | 5.97 MB | 1.70 MB | 601800 | 0 % |
| 00:31 | 11934208 | 85802 | 1 | 2166 | 0 | 84904 | 5.97 MB | 1.70 MB | 601492 | 0 % |
| 00:42 | 12580535 | 90471 | 1 | 2272 | 0 | 89551 | 6.16 MB | 1.70 MB | 601068 | 0 % |
| 00:52 | 13221838 | 95094 | 1 | 2365 | 0 | 94171 | 6.34 MB | 1.70 MB | 600696 | 0 % |
| 01:03 | 13863143 | 99736 | 1 | 2466 | 0 | 98805 | 6.53 MB | 1.70 MB | 600324 | 0 % |
| 01:14 | 14514456 | 104440 | 1 | 2582 | 0 | 103498 | 6.53 MB | 1.70 MB | 599956 | 0 % |
| 01:24 | 15150748 | 109045 | 1 | 2666 | 0 | 108094 | 6.72 MB | 1.70 MB | 599576 | 0 % |

Raw readings: homenodellm `~/pitest-tools/t119/` (`r-HHMM.txt`, `ping.txt`).
Issue 92 didn't show up in this run. Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-455-g9486f60+x7d9f` (test 109 step 6, BT_ON the Linux way)

Real Pi 3B+ over netboot. Test 109 step 6 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54)), Pi test agent.

**Still no answer from the Bluetooth chip.** `bt_power` now switches BT_ON as an
output, and the switch works: `state 0` before the power cycle, `state 1` after,
`direction out` both times. Even so, HCI Reset gets no reply with any flow
setting, and CTS stays at 1 as in steps 1–3 and 5. So a BT_ON kept as an input
wasn't the cause. Image `~/redox-rpi-cpu/keep/v0.3.0-dev-455-g9486f60+x7d9f/harddrive.img`
(no manifest). Stable's bootloader was used. Boot log:
`bcm2835-btuartd: mini UART at 0x3f215000, 115313 baud (core 250 MHz)`.

- (a) `cat /scheme/rpi-firmware/bt_power_config`, read first after boot:
  `direction out, polarity 0, term_en 0, pull_up 0, state 0`. The firmware
  already had BT_ON as an output, not `direction in`.
- (b) `bt diag`:

```
gpio 30: mode alt5, level 1, pull up
gpio 31: mode alt5, level 1, pull none
gpio 32: mode alt5, level 1, pull none
gpio 33: mode alt5, level 1, pull up
gpio 43: mode alt0, level 1, pull unknown
gpio 43 sampled 200 times: 97 high, 103 low
lpo: enabled 1, busy 1, source 1, divi 585, divf 3840, rate_hz 32768, ctl 0x00000291, div 0x00249f00
bt_power: 0
bt_power_config before: direction out, polarity 0, term_en 0, pull_up 0, state 0
power: cycled
bt_power_config after: direction out, polarity 0, term_en 0, pull_up 0, state 1
uart: core clock 250000000 Hz, 115313 baud
flow auto:
before: cts_line 1, rts 1, tx_fifo 0, rx_fifo 0, cntl 0x3f, msr 0x00
after: cts_line 1, rts 1, tx_fifo 0, rx_fifo 0, cntl 0x3f, msr 0x00
received: nothing
flow auto-low:
before: cts_line 1, rts 0, tx_fifo 0, rx_fifo 0, cntl 0xff, msr 0x00
after: cts_line 1, rts 0, tx_fifo 4, rx_fifo 0, cntl 0xff, msr 0x00
received: nothing
flow none:
before: cts_line 1, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x00
after: cts_line 1, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x00
received: nothing
no answer with any flow setting
```

- (c) Not run: no flow setting answered.
- `cat /scheme/btuart/status` afterwards: `cts_line 1, rts_status 1, tx_fifo 0,
  rx_fifo 0, tx_done 1, lsr 0x60, msr 0x00, cntl 0x3f, stat 0x000003ce`.
- The 32 kHz clock runs as in step 5 (97 high / 103 low, `rate_hz 32768`).

Step 4 (`miniuart-bt` in the shared config.txt) still waits for the user's OK.
Stable image restored afterwards.

## 2026-10-31: test 121, sdhost DMA × RedoxFS write coalescing (2×2)

Real Pi 3B+ over netboot. Test 121 ([#30](https://github.com/redox-rpi/redox-rpi/issues/30), [#16](https://github.com/redox-rpi/redox-rpi/issues/16)), Pi test agent.

**All four images pass and none had a short read (`256+0 records in` every time).
With the same image, the write speed and the driver's SCHTIME change a lot from
run to run, so #117's 5.5 MB/s and +16.5 s look like noise, not a slower build.**
DMA uses less ACTTIME (about 18–21 s per write against about 26 s) but more SCHTIME
(+3.4 to +11.5 s against +3.1 to +3.5 s); PIO was steadier and a little faster
here (8.1–9.0 MB/s). Coalescing made no clear difference with DMA or PIO.

Images from homenodellm `~/redox-rpi-sd/keep/`, each manifest matches, staged in
order A–D with stable's bootloader:

- `scripts/pi-test.py` standard checks pass on all 4 (version, manifest, ps, ps-tty,
  sdcard, 0 of 300 starts failed).
- Boot log: A and B show `bcm2835-sdhostd: DMA interrupt /scheme/irq/phandle-1/1,20`
  and `DMA channel 4 at 0x3f007000`. C and D have no DMA line. No panic in any boot.

On `/scheme/data`, 256 MiB with `bs=1048576` from `/dev/urandom`. Steps:
1 = `conv=fsync`; 2 = with `iflag=fullblock`; 3 = read to `/dev/null`;
4 = step 2 again. ACTTIME and SCHTIME are the `ps` change of `bcm2835-sdhos`
across the step.

| image | step | UTC | records in | time | MB/s | ACTTIME | SCHTIME |
|---|---|---|---|---|---|---|---|
| A DMA | 1 write | 01:37:29 | 256+0 | 40.5 s | 6.6 | +21.0 s | **+11.5 s** |
| A DMA | 2 write fullblock | 01:38:12 | 256+0 | 34.0 s | 7.9 | +27.1 s | +6.6 s |
| A DMA | 3 read | 01:38:51 | 256+0 | 15.3 s | 18 | +13.8 s | +1.0 s |
| A DMA | 4 write fullblock | 01:39:08 | 256+0 | 40.5 s | 6.6 | +34.0 s | +4.3 s |
| B DMA + coalescing (#117 image) | 1 write | 01:41:36 | 256+0 | 32.9 s | 8.2 | +18.3 s | +7.9 s |
| B | 2 write fullblock | 01:42:15 | 256+0 | 37.5 s | 7.2 | +30.9 s | +3.6 s |
| B | 3 read | 01:42:59 | 256+0 | 15.6 s | 17 | +14.2 s | +0.7 s |
| B | 4 write fullblock | 01:43:16 | 256+0 | 35.0 s | 7.7 | +28.5 s | +3.4 s |
| C PIO | 1 write | 01:45:45 | 256+0 | 32.8 s | 8.2 | +26.4 s | +3.3 s |
| C PIO | 2 write fullblock | 01:46:20 | 256+0 | 32.6 s | 8.2 | +26.2 s | +3.5 s |
| C PIO | 3 read | 01:46:58 | 256+0 | 14.0 s | 19 | +12.6 s | +0.9 s |
| C PIO | 4 write fullblock | 01:47:14 | 256+0 | 33.0 s | 8.1 | +26.6 s | +3.4 s |
| D PIO + coalescing | 1 write | 01:49:31 | 256+0 | 32.0 s | 8.4 | +25.8 s | +3.2 s |
| D | 2 write fullblock | 01:50:09 | 256+0 | 29.9 s | 9.0 | +23.8 s | +3.1 s |
| D | 3 read | 01:50:40 | 256+0 | 13.8 s | 19 | +12.7 s | +0.9 s |
| D | 4 write fullblock | 01:51:00 | 256+0 | 32.9 s | 8.2 | +26.7 s | +3.5 s |

- The #117 image (B) wrote at 8.2 MB/s with SCHTIME +7.9 s this time, against
  5.5 MB/s and +16.5 s in #117. Plain DMA (A) had the slowest first write
  (6.6 MB/s, +11.5 s). So the slow #117 run came from DMA's variance, not from
  coalescing.
- Every image ran steps 1–4 right after the standard checks, on a fresh boot.
  The files were removed after each step.

Raw output: homenodellm `~/pitest-tools/t121/` (`X-boot.txt`, `X-pitest.txt`, `X-dd.txt`).
Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-457-gfd3a726+x7d9f` (test 109 step 7, drive test and WL_ON)

Real Pi 3B+ over netboot. Test 109 step 7 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54)), Pi test agent.

**With WL_ON on, the Bluetooth chip starts: it drives its TX line high and sends
bytes back. Without WL_ON it stays silent.** WL_ON was `0` at boot. With
WL_ON 1, GPIO 33 (the chip's TX) went high 1 ms after BT_ON and stayed high,
and CTS went from 1 to 0. With flow `none`, 7 bytes came back after HCI Reset:
`10 10 10 13 80 00 f0`. That is not an `04 0e …` event, which per the reading
guide points at a wrong baud rate. Image
`~/redox-rpi-cpu/keep/v0.3.0-dev-457-gfd3a726+x7d9f/harddrive.img` (no manifest),
with stable's bootloader. Boot log:
`bcm2835-btuartd: mini UART at 0x3f215000, 115313 baud (core 250 MHz)`.

- Before `bt diag`: `cat /scheme/rpi-firmware/wifi_power` → `0`;
  `bt_power_config` → `direction out, polarity 0, term_en 0, pull_up 0, state 0`.
- (a) `bt diag`:

```
gpio 30: mode alt5, level 1, pull up
gpio 31: mode alt5, level 1, pull none
gpio 32: mode alt5, level 1, pull none
gpio 33: mode alt5, level 1, pull up
gpio 43: mode alt0, level 0, pull unknown
gpio 43 sampled 200 times: 97 high, 103 low
lpo: enabled 1, busy 1, source 1, divi 585, divf 3840, rate_hz 32768, ctl 0x00000291, div 0x00249f00
bt_power: 0
bt_power_config before: direction out, polarity 0, term_en 0, pull_up 0, state 0
wifi_power_config before: direction out, polarity 0, term_en 0, pull_up 0, state 0
drive test, WL_ON as found: BT_ON 0: gpio 30 1, gpio 33 0
drive test, WL_ON as found: BT_ON 1, 5551 samples in 2 s: gpio 30 low 0 (never), gpio 33 high 0 (never)
wifi_power: 1
drive test, WL_ON 1: BT_ON 0: gpio 30 1, gpio 33 1
drive test, WL_ON 1: BT_ON 1, 7409 samples in 2 s: gpio 30 low 0 (never), gpio 33 high 7409 (first after 1 ms)
power: cycled
bt_power_config after: direction out, polarity 0, term_en 0, pull_up 0, state 1
wifi_power_config after: direction out, polarity 0, term_en 0, pull_up 0, state 1
uart: core clock 250000000 Hz, 115313 baud
flow auto:
before: cts_line 1, rts 1, tx_fifo 0, rx_fifo 0, cntl 0x3f, msr 0x00
after: cts_line 1, rts 1, tx_fifo 0, rx_fifo 0, cntl 0x3f, msr 0x00
received: nothing
flow auto-low:
before: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0xff, msr 0x10
after: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0xff, msr 0x10
received: nothing
flow none:
before: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x10
after: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x10
received 7 bytes: 10 10 10 13 80 00 f0
no answer with any flow setting
```

- `cat /scheme/btuart/status` afterwards: `cts_line 0, rts_status 1, tx_fifo 0,
  rx_fifo 0, tx_done 1, lsr 0x60, msr 0x10, cntl 0x3f, stat 0x0000034e`.
- (b) not run: `bt diag` counts no flow setting as answered.
- One note: with WL_ON 1 and BT_ON 0, GPIO 33 already read 1. GPIO 30 never
  went low in either case.

Reading: the chip needs WL_ON. Next is the UART side: the baud rate, and why
`auto`/`auto-low` see nothing while `none` gets bytes. Step 4 still waits for the
user's OK. Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-478-g1764270` (PR #81 netstack-multi, as it will be merged)

Real Pi 3B+ over netboot. Test 122 ([#53](https://github.com/redox-rpi/redox-rpi/issues/53)), Pi test agent.

**Pass, as in test 74, with one difference: `ls /scheme/netcfg/ifaces` fails.** It
answers `ls: general io error: Not a directory (os error 20)`, while
`cat /scheme/netcfg/ifaces` says `Is a directory`. Both `eth0/addr/list` and
`eth1/addr/list` can be read, so both adapters are there; only listing the
directory doesn't work. Image `~/redox-rpi-zrtl/keep/nsm-v0.3.0-dev-478-g1764270.img`
(no manifest next to it; the one in the image passes), with stable's bootloader.

- `scripts/pi-test.py` standard checks, before and after testnetd: **pass**
  (version, manifest present, ps 47 then 50 processes, ps-tty, sdcard,
  0 of 300 starts failed both times).
- `/usr/lib/drivers/testnetd &`, then after 30 s:
  - `cat /scheme/netcfg/ifaces/eth1/addr/list` → **`10.0.9.2/24`**;
    `eth0/addr/list` → `192.168.178.148/24`.
  - `cat /scheme/netcfg/route/list` has **one default route**, plus
    `10.0.9.0/24 dev eth1`:

```
default via 192.168.178.1 dev eth0 src 192.168.178.148
127.0.0.0/8  dev loopback src 127.0.0.1
10.0.9.0/24  dev eth1 src 10.0.9.2
192.168.178.0/24  dev eth0 src 192.168.178.148
```

  - `ping -c 3 10.0.9.1`: 3/3 (2.4–2.9 ms); `ping -c 3 192.168.178.1`: 3/3 (1.0–1.8 ms).
  - `cat /tmp/testnetd.log`: `dhcp 2 dhcp_foreign 0 foreign_src 0 ping 3`.
- Every `DHCP:`/`smolnetd` line of the serial log (no `smolnetd` line and no panic):

```
DHCP client bound to address 192.168.178.148 (5 ms)
DHCP client bound to address 192.168.178.148 (8 ms)
DHCP: eth0 got 192.168.178.148/24
DHCP: default route via 192.168.178.1 (eth0)
DHCP: eth1 got 10.0.9.2/24
```

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-520-g38cb884+xf552` (PR #93 mDNS, PTR answer with SRV, TXT, address records)

Real Pi 3B+ over netboot. Test 123 ([#37](https://github.com/redox-rpi/redox-rpi/issues/37)), Pi test agent.

**Pass: the PTR answer carries SRV, TXT, A and AAAA in one reply, and (2)–(4)
are as described.** Image `~/redox-rpi-zrtl/keep/mdns-v0.3.0-dev-520-g38cb884+xf552.img`
(no manifest next to it; the one in the image passes), with stable's bootloader.
Query tool from `origin/mdns:patches/experiments/mdns/mdns-query.py`.

- `scripts/pi-test.py` standard checks: **pass** (version, manifest present,
  ps 48, ps-tty, sdcard, 0 of 300 starts failed).
- `mdnsd:` serial lines:

```
mdnsd: redox-rpi.local on 224.0.0.251 and ff02::fb port 5353, 2 services configured
mdnsd: redox-rpi._devsh._tcp.local port 2323 announced
mdnsd: redox-rpi.local is 192.168.178.148, fe80::ba27:ebff:fe08:e0aa, 2a02:590:1504:9901:ba27:ebff:fe08:e0aa, fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa
```

- (1) `python3 /tmp/mdns-query.py _devsh._tcp.local PTR`: one answer with PTR,
  **SRV, TXT, A**, and three AAAA:

```
192.168.178.148: _devsh._tcp.local PTR redox-rpi._devsh._tcp.local (ttl 4500)
192.168.178.148: redox-rpi._devsh._tcp.local SRV port 2323 target redox-rpi.local (ttl 120)
192.168.178.148: redox-rpi._devsh._tcp.local TXT 00 (ttl 4500)
192.168.178.148: redox-rpi.local A 192.168.178.148 (ttl 120)
192.168.178.148: redox-rpi.local AAAA fe80::ba27:ebff:fe08:e0aa (ttl 120)
192.168.178.148: redox-rpi.local AAAA 2a02:590:1504:9901:ba27:ebff:fe08:e0aa (ttl 120)
192.168.178.148: redox-rpi.local AAAA fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa (ttl 120)
```

- (2) `python3 /tmp/mdns-query.py redox-rpi.local`: A `192.168.178.148`, plus the
  three AAAA. The router also answered from its cache with the same A (ttl 10).

```
192.168.178.148: redox-rpi.local A 192.168.178.148 (ttl 120)
192.168.178.148: redox-rpi.local AAAA fe80::ba27:ebff:fe08:e0aa (ttl 120)
192.168.178.148: redox-rpi.local AAAA 2a02:590:1504:9901:ba27:ebff:fe08:e0aa (ttl 120)
192.168.178.148: redox-rpi.local AAAA fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa (ttl 120)
192.168.178.1: redox-rpi.local A 192.168.178.148 (ttl 10)
```

- (3) `python3 /tmp/mdns-query.py redox-rpi.local AAAA -6 wlp13s0`: answered from
  the Pi's link-local address with link-local, `2a02:…` and `fdeb:…`, plus the A:

```
fe80::ba27:ebff:fe08:e0aa: redox-rpi.local AAAA fe80::ba27:ebff:fe08:e0aa (ttl 120)
fe80::ba27:ebff:fe08:e0aa: redox-rpi.local AAAA 2a02:590:1504:9901:ba27:ebff:fe08:e0aa (ttl 120)
fe80::ba27:ebff:fe08:e0aa: redox-rpi.local AAAA fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa (ttl 120)
fe80::ba27:ebff:fe08:e0aa: redox-rpi.local A 192.168.178.148 (ttl 120)
```

- (4) `python3 /tmp/mdns-query.py _services._dns-sd._udp.local PTR`: the Pi's lines
  are below. Other devices on the LAN answered too; their lines are left out here.
  The Pi lists only **`_devsh._tcp.local`, no `_ssh._tcp`**:

```
192.168.178.148: _services._dns-sd._udp.local PTR _devsh._tcp.local (ttl 4500)
192.168.178.148: redox-rpi.local A 192.168.178.148 (ttl 120)
192.168.178.148: redox-rpi.local AAAA fe80::ba27:ebff:fe08:e0aa (ttl 120)
192.168.178.148: redox-rpi.local AAAA 2a02:590:1504:9901:ba27:ebff:fe08:e0aa (ttl 120)
192.168.178.148: redox-rpi.local AAAA fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa (ttl 120)
```

- mdnsd CPU when idle: `top -n 1` printed no mdnsd line, so I used `ps`. ACTTIME
  went from 0.06 s to 0.07 s over 30 s idle (about 3 min after boot), 4.92 MB.
- The TXT record is a single empty string (`00`). The serial log says
  "2 services configured" but announces only devsh; this image has no sshd.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-486-gdd28c2f` (PR #103, `lsusb` and `hwinfo` as defaults)

Real Pi 3B+ over netboot. Test 125 ([#35](https://github.com/redox-rpi/redox-rpi/issues/35)), Pi test agent.

**Pass, with one gap: `hwinfo` has no governor line.** Image
`~/redox-rpi-cpu/keep/v0.3.0-dev-486-gdd28c2f/harddrive.img` (no manifest next
to it; the one in the image passes), with stable's bootloader. No panic in the boot log.

- `scripts/pi-test.py` standard checks: **pass** (version, manifest present, ps 47,
  ps-tty, sdcard, 0 of 300 starts failed).
- `lsusb` lists both hubs `0424:2514`, the LAN7800 `0424:7800`, and the USB hub and
  keyboard that are plugged in. `lsusb -t` shows the tree; my devsh helper strips
  leading spaces, so the indentation is lost below. `lsusb -v` gives each
  interface's class.

```
Bus 001 Device 001: ID 0424:2514 Microchip USB2514 hub (USB 2.00, high speed)
Bus 001 Device 002: ID 0424:2514 Microchip USB2514 hub (USB 2.00, high speed)
Bus 001 Device 003: ID 0424:7800 Microchip LAN7800 Gigabit Ethernet (Pi 3B+) (USB 2.10, high speed)
Bus 001 Device 004: ID 05e3:0610 hub (USB 2.00, high speed)
Bus 001 Device 005: ID 04d9:0006 keyboard, HID (USB 2.00, low speed)

root port: address 1, 0424:2514 Microchip USB2514 hub, high speed
port 1: address 2, 0424:2514 Microchip USB2514 hub, high speed
port 1: address 3, 0424:7800 Microchip LAN7800 Gigabit Ethernet (Pi 3B+), high speed
port 2: address 4, 05e3:0610 hub, high speed
port 1: address 5, 04d9:0006 keyboard, HID, low speed
```

- `ls /scheme/usb.dwc2` → `devices`.
- `hwinfo`:

```
build        v0.3.0-dev-486-gdd28c2f
model        Raspberry Pi 3 Model B+
revision     0x00a020d3
memory       1 GiB
ARM memory   998 MiB
GPU memory   16 MiB
firmware     0x6aa809d0
CPUs         4
ARM clock    1400 MHz (max 1400 MHz)
clocks       emmc 200 MHz, uart 48 MHz, core 250 MHz, v3d 250 MHz, h264 250 MHz, isp 250 MHz, sdram 400 MHz
temperature  53.6 °C
throttling   0x00000000 (none now)
USB          5 devices (lsusb for the list)
```

  Model, 1 GiB, 4 CPUs, clocks, temperature and throttling are there. The USB
  count (5) matches `lsusb`. **No governor line.** (The serial number line is left
  out here.)
- Optional port cycle (`pi-hotplug-test.sh`) not run.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-462-g3783750+xc526` (pkg-persist, packages that survive a reset)

Real Pi 3B+ over netboot with the usual card. Test 124 ([#44](https://github.com/redox-rpi/redox-rpi/issues/44)), Pi test agent.

**Pass for (1) to (5), with one point for the owner.** After the reboot, the copy
line appears in the serial log after `timesync: … from pool.ntp.org` and after
`redox login:`. The row asks for it before any network use. The service starts
after `60_data`, and the line is printed when the copy ends, so the log can't
show whether the copy started before timesync used the network.
Image `~/redox-rpi-boot/keep/persistC-harddrive.img` (no manifest next to it;
the one in the image passes), with stable's bootloader.

- First boot: `pkg-persist: /scheme/data/pkg does not exist, nothing to copy`,
  no `60_data` failure, then
  `timesync: clock set to 2026-10-31 02:08:51 UTC from pool.ntp.org`.
- `scripts/pi-test.py` standard checks: **pass**, both after the first boot and
  at the end (version, manifest present, ps 47, ps-tty, sdcard, 0 of 300 starts failed).
- (1) `mv`: `echo hi > /scheme/data/t124; mv /scheme/data/t124 /scheme/data/t124b`
  gave no error, `ls /scheme/data` showed `t124b`, then it was removed.
- (2) `pkg-persist install nano` (39 s) ends with `Commit complete.` and
  `pkg-persist: 214 files copied into /, 2 kept from the image`.
  `nano --version` prints `GNU nano, version 9.2`. `pkg-persist list` shows
  `nano`, `ncursesw` and `terminfo`.
- (3) `reboot` (same image, one `kreset`), back to `redox login:` after 25 s.
  Serial log order: `timesync: … from the saved time`, `timesync: clock set to
  2026-10-31 02:11:24 UTC from pool.ntp.org`, `redox login:`, then
  **`pkg-persist: 214 files copied into /, 2 kept from the image`** (same count).
  `nano --version` works again. `ls /scheme/data/pkg/var/lib/packages` lists
  `nano.pkgar_head ncursesw.pkgar_head terminfo.pkgar_head`.
- (4) `pkg-persist restore` again: `pkg-persist: 0 files copied into /, 2 kept from the image`.
- (5) `rm -r /scheme/data/pkg`; `/scheme/data` is as before the test.

`pkg-persist:` lines in order:

```
pkg-persist: /scheme/data/pkg does not exist, nothing to copy
pkg-persist: 214 files copied into /, 2 kept from the image      (install)
pkg-persist: 214 files copied into /, 2 kept from the image      (boot after reboot)
pkg-persist: 0 files copied into /, 2 kept from the image        (restore again)
```

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-179-g950dd30+x4590` (test 126, first try, not finished)

Real Pi 3B+ over netboot. Test 126 ([#43](https://github.com/redox-rpi/redox-rpi/issues/43)), Pi test agent.

**No result. My test runner cut the model download short, so the test must run
again.** The runner gave the `rinfer --download` command 900 s. In that time
223,081,729 of 341,455,552 bytes arrived (about 0.25 MB/s), and then the devsh
connection closed. Every model load that followed stopped with `Error: tensor
needs 4096 bytes at offset 224546496, only 0 remaining in file`, and the
assistant fell back to "tools only". These are not real readings.

- Standard checks pass (version, manifest present, ps 47, ps-tty, sdcard,
  0 of 300 starts failed). The boot log has
  `thread 'main' (1) panicked at drivers/pcid/src/cfg_access/fallback.rs:88:9`
  (this image is based on dev-179, before pcid was left out on the Pi).
- `free` before: 737,936 KB free. The lowest free reading during the failed
  loads was 443,780 KB.
- At the end, `temperature` 60148 (60.1 °C) and `throttled` `0x00080000`
  (soft temperature limit reached since boot, not active now).
- `prompt.kv` and the partial model file were removed; `/scheme/data` is as before.

Next try: the download needs about 25 minutes at this speed, and the runner now
allows 50. Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-533-g6c729cd+x9f76`

Built 2026-10-31T03:30:26Z, `libc.so` `d8e9a5ac4a3e35a3…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Nightly stable image test (scripts/update-stable.sh).

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-533-g6c729cd+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-533-g6c729cd+x9f76.toml |
| ps | pass | 42 processes (piped) |
| ps-tty | pass | 42 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## 2026-10-31: `v0.3.0-dev-458-g990871c+x7d9f` (test 109 step 8, baud sweep)

Real Pi 3B+ over netboot. Test 109 step 8 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54)), Pi test agent.

**The chip answers HCI Reset correctly when the divider assumes a 400 MHz core
clock: `04 0e 04 01 03 0c 00 <- Command Complete`.** The firmware reports
250 MHz before and after every try. So the UART runs at a baud rate for 400 MHz
while the firmware says 250 MHz. Per the row's reading guide, the core clock is
not what the firmware reports, and the fix is to measure or fix it.
**(b) did not work yet**: `bt up` set the UART back to the firmware's 250 MHz.
Image `~/redox-rpi-cpu/keep/v0.3.0-dev-458-g990871c+x7d9f/harddrive.img` (no
manifest), with the new stable's bootloader.

- (a) `bt diag` begins with the same lines as step 7: pins, the 32 kHz clock
  (99 high / 101 low), the WL_ON drive test (GPIO 33 high after 1 ms with
  WL_ON 1), and flow `none` getting `10 10 10 13 80 00 f0`. The sweep:

```
baud sweep: cts_line 0 after 0 ms
assumed core 250 MHz (firmware says 250000000 before, 250000000 after): nothing
assumed core 275 MHz (firmware says 250000000 before, 250000000 after): 18 0c 86 03 0c 80
assumed core 300 MHz (firmware says 250000000 before, 250000000 after): nothing
assumed core 325 MHz (firmware says 250000000 before, 250000000 after): 08 03 28 60 00
assumed core 350 MHz (firmware says 250000000 before, 250000000 after): 0c 81 16 08 e0
assumed core 375 MHz (firmware says 250000000 before, 250000000 after): 04 10 01 00
assumed core 400 MHz (firmware says 250000000 before, 250000000 after): 04 0e 04 01 03 0c 00  <- Command Complete
assumed core 450 MHz (firmware says 250000000 before, 250000000 after): nothing
assumed core 500 MHz (firmware says 250000000 before, 250000000 after): nothing
uart: core clock 250000000 Hz, 115313 baud
baud sweep answered for core MHz: 400
```

- (b) `echo 400000000 > /scheme/btuart/clock; echo none > /scheme/btuart/flow`, then:
  - `bt info` → `bt: no answer to HCI command 1001`.
  - `bt up` → `power: cycled`, **`uart: core clock 250000000 Hz, 115313 baud`**,
    `bt: no answer to HCI command 0c03`. The power cycle in `bt up` went back to
    the firmware's 250 MHz instead of the 400 MHz written to `clock`.
  - `bt info` → no answer to `1001`; `bt scan 10` → no answer to `0c01`.
  - `cat /scheme/btuart/status` afterwards: `cts_line 0, rts_status 0, tx_fifo 0,
    rx_fifo 0, tx_done 1, lsr 0x60, msr 0x10, cntl 0x03, stat 0x0000030e`.
- The serial console showed `bcm2835-btuartd: N received bytes lost so far`
  221 times after login, up to 221 bytes lost.

Next: `bt up` (and `bt info` before it) should keep the clock written to
`/scheme/btuart/clock`, or the core clock should be fixed. Step 4 still waits
for the user's OK. Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-464-g3835c4d+xc526` (pkg-persist recheck, blocking oneshot)

Real Pi 3B+ over netboot with the usual card. Test 127 ([#44](https://github.com/redox-rpi/redox-rpi/issues/44)), Pi test agent.

**Pass. The boot copy now takes 3940 ms and comes before timesync's lines.**
Image `~/redox-rpi-boot/keep/persistD-harddrive.img` (no manifest next to it;
the one in the image passes), with the new stable's bootloader.

- First boot: `pkg-persist: /scheme/data/pkg does not exist, nothing to copy`.
- `pkg-persist install nano`: nano, ncursesw and terminfo (1.19 MB download),
  `Commit complete.`, `pkg-persist: 214 files copied into /, 2 kept from the image (3736 ms)`.
- `reboot` (one `kreset`), back after 26 s. Serial log order:

```
redox-rpi v0.3.0-dev-464-g3835c4d+xc526 (netboot, 2026-10-31T02:18:59Z)
experiments: pkg-hold pkg-persist pkg-repo sigreturn-svc timesync
redox login: pkg-persist: 214 files copied into /, 2 kept from the image (3940 ms)
timesync: clock set to 2026-10-31 03:50:23 UTC from the saved time
timesync: clock set to 2026-10-31 03:52:02 UTC from pool.ntp.org
```

  **N = 3940 ms.** The line comes right after `redox login:` (the row expects
  that) and now **before** both timesync lines. In #124 it came after
  timesync's NTP line.
- `nano --version` → `GNU nano, version 9.2`.
- `scripts/pi-test.py` standard checks: **pass** (version, manifest present,
  ps 47, ps-tty, sdcard, 0 of 300 starts failed).
- Clean up: `rm -r /scheme/data/pkg`; `/scheme/data` is as before.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-459-g799ed17+x7d9f` (test 109 step 9, measured core clock)

Real Pi 3B+ over netboot. Test 109 step 9 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54)), Pi test agent.

**The measured core clock is 400 MHz (`core_clock` 400000000), and with it HCI
Reset is answered correctly in `bt diag` (flow `auto-low` and `none`).
`bt up` still fails**: with the default flow `auto` it gets no answer to Reset.
With flow `none` it gets past Reset but stops at the next command (`1001`). The
console logged up to 262 lost received bytes, which by the row's reading points
at the polling receive thread. Image
`~/redox-rpi-cpu/keep/v0.3.0-dev-459-g799ed17+x7d9f/harddrive.img`, with the new
stable's bootloader. Boot log:
`bcm2835-btuartd: mini UART at 0x3f215000, 115207 baud (core 400 MHz)`.

- (a) `core_clock` → `400000000`. `clocks` has `core 250000000 400000000`
  (reported, max). `cat /scheme/btuart/clock` → `400000000` / `firmware`.
- (b) `bt diag`, the lines after the drive test (pins, 32 kHz clock and the
  WL_ON drive test are as in steps 7 and 8):

```
core clock idle: firmware reports 250000000, measures 399999000
core clock under load: firmware reports 250000000, measures 400000000
core clock after: firmware reports 250000000, measures 399999000
uart: core clock 400000000 Hz set by firmware (firmware measures 400000000), 115207 baud
flow auto:
before: cts_line 1, rts 1, tx_fifo 0, rx_fifo 0, cntl 0x3f, msr 0x00
after: cts_line 1, rts 1, tx_fifo 0, rx_fifo 0, cntl 0x3f, msr 0x00
received: nothing
flow auto-low:
before: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0xff, msr 0x10
after: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0xff, msr 0x10
received 7 bytes: 04 0e 04 01 03 0c 00
flow none:
before: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x10
after: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x10
received 7 bytes: 04 0e 04 01 03 0c 00
answered with flow: auto-low, none
baud sweep: cts_line 0 after 0 ms
assumed core 250 MHz (firmware reports 250000000, measures 400000000): nothing
assumed core 275 MHz (firmware reports 250000000, measures 400000000): 18 0c 86 03 0c 80
assumed core 300 MHz (firmware reports 250000000, measures 400000000): nothing
assumed core 325 MHz (firmware reports 250000000, measures 400000000): 08 03 28 60 00
assumed core 350 MHz (firmware reports 250000000, measures 400000000): 0c c1 16 08 e0
assumed core 375 MHz (firmware reports 250000000, measures 400000000): 04 10 01 00
assumed core 400 MHz (firmware reports 250000000, measures 400000000): 04 0e 04 01 03 0c 00  <- Command Complete
assumed core 450 MHz (firmware reports 250000000, measures 400000000): nothing
assumed core 500 MHz (firmware reports 250000000, measures 400000000): nothing
uart: core clock 400000000 Hz set by firmware (firmware measures 400000000), 115473 baud
baud sweep answered for core MHz: 400
power: cycled
uart: core clock 400000000 Hz set by firmware (firmware measures 400000000), 115473 baud
reset at the driver's clock, flow auto: nothing
reset at the driver's clock, flow none: 04 0e 04 01 03 1c 01  <- Command Complete
```

  The core clock stays at 400 MHz idle, under load and after. The last reset
  answer is `03 1c 01` instead of `03 0c 00`, a few bits off, after the baud
  rate changed from 115207 to 115473.
- (c) `bt up` → `uart: core clock 400000000 Hz set by firmware …, 115473 baud`,
  `bt: no answer to HCI command 0c03`. `bt info` → no answer to `1001`.
  `bt scan 10` → no answer to `0c01`. `bt inquiry 10` → no answer to `0401`.
- (d) `echo 400000000 > /scheme/btuart/clock; bt up` → `… set by hand …`,
  still no answer to `0c03`. `bt info` and `bt scan 10` have no answer.
  `cat /scheme/btuart/clock` → `400000000` / `hand` (the hand setting stays).
  The flow was `auto`.
- Extra, not in the row: `echo none > /scheme/btuart/flow; bt up` → **no answer to
  `1001`**, so Reset (`0c03`) was answered and it failed one command later.
  `bt info` → no answer to `1001`. `bt scan 10` → `scan: LE for 10 s`, no answer to `200c`.
- (e) `cat /scheme/btuart/clock` → `400000000 hand`; `status` → `cts_line 0,
  rts_status 0, tx_fifo 0, rx_fifo 0, tx_done 1, lsr 0x60, msr 0x10, cntl 0x03,
  stat 0x0000030e`. **262 `received bytes lost` lines** on the serial console
  (up to `262 received bytes lost so far`).

No `firmware: …, 323 records` line: `bt up` never got that far. Step 4 still
waits for the user's OK. Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-560-gc85276b+x4590` (test 126, tries 2 and 3, no result)

Real Pi 3B+ over netboot. Test 126 ([#43](https://github.com/redox-rpi/redox-rpi/issues/43)), Pi test agent.

**No result: the model download didn't finish, so nothing about the saved
prompt state or the writing speed could be measured.** The standard checks pass
(6 of 6) in both boots. No kernel panic and no `RELIBC PANIC`.

- **Try 2** (03:55–04:54 UTC): my runner allowed 3000 s. In that time 322,202,182 of
  341,455,552 bytes arrived (about 0.1 MB/s). The devsh connection closed
  before the end, and every model load then stopped with `Error: tensor needs
  1769472 bytes at offset 321972928, only 229254 remaining in file`.
- **Try 3** (05:02–06:01 UTC), a fresh download with 5400 s allowed. **`rinfer
  --download` stopped by itself**:

```
part 000, 16 MiB
…
part 011, 192 MiB
Error: after 204051684 bytes: Transport endpoint is not connected (os error 107)
```

  That is 204 MB in 59 minutes, about 0.058 MB/s, slower than try 2 and much
  slower than try 1's 0.25 MB/s. The first `assistant --ask` afterwards
  printed `The language model did not start (Interrupted system call (os error 4))`.
  The later ones stopped on the short file
  (`tensor needs 4096 bytes at offset 206833344, only 0 remaining`).
- `temperature` was 52–53 °C before and after, and `throttled` was
  `0x00000000` throughout. `free` was 736,592 KB at the start and 629,620 KB at
  the end. The lowest reading during the failed loads was 673,380 KB.
- The short model file (204 MB) is still on `/scheme/data`. The next image
  resumes the download.

Requeued with the assistant session's new image `v0.3.0-dev-561-g3118d0a+x4590`
(download resumes). Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-462-gf830bfd+x7d9f` (test 109 step 10, flow auto-low, no baud drift)

Real Pi 3B+ over netboot. Test 109 step 10 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54)), Pi test agent.

**First full answer from the chip: with flow `none`, `bt info` reads `version:
Bluetooth 4.1 (HCI 7, LMP 7), manufacturer 15 (Broadcom), subversion 6119,
firmware build 0`, `name: BCM4345C0`. But `bt up` still gets no answer to Reset
(`0c03`) with either flow, so there's no firmware upload yet.** No bytes were
lost in (a) and (b); 224 were lost during (c), which includes `bt diag`.
Image `~/redox-rpi-cpu/keep/v0.3.0-dev-462-gf830bfd+x7d9f/harddrive.img`, with
the new stable's bootloader. Boot log:
`bcm2835-btuartd: mini UART at 0x3f215000, 115207 baud (core 400 MHz)`.
Booted with a serial-console reset: the devsh reset from the #126 image didn't
take.

- (a) `cat /scheme/btuart/flow /scheme/btuart/baud /scheme/btuart/clock` →
  `auto-low`, `115207`, `400000000` / `firmware`, as expected.
- (b)
  - `bt up` → `power: cycled`, `uart: core clock 400000000 Hz set by firmware
    (firmware measures 400000000), 115207 baud, flow auto-low`,
    **`bt: no answer to HCI command 0c03`**.
  - `bt info` → no answer to `1001`.
  - `bt scan 10` and `bt inquiry 10` → `bt: Connection timed out (os error 110)`.
  - **0 `received bytes lost` lines** up to here.
- (c) `bt diag` (from the clock lines on):

```
core clock idle: firmware reports 250000000, measures 400000000
core clock under load: firmware reports 250000000, measures 400000000
core clock after: firmware reports 250000000, measures 400000000
uart: core clock 400000000 Hz set by firmware (firmware measures 400000000), 115207 baud, flow auto-low
flow auto:
before: cts_line 0, rts 1, tx_fifo 0, rx_fifo 0, cntl 0x3f, msr 0x10
after: cts_line 0, rts 1, tx_fifo 4, rx_fifo 0, cntl 0x3f, msr 0x10
received: nothing
flow auto-low:
before: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0xff, msr 0x10
after: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0xff, msr 0x10
received 7 bytes: 04 0e 04 01 03 0c 00
flow none:
before: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x10
after: cts_line 0, rts 0, tx_fifo 0, rx_fifo 0, cntl 0x03, msr 0x10
received 7 bytes: 04 0e 04 01 03 0c 00
answered with flow: auto-low, none
baud sweep: cts_line 0 after 0 ms
assumed core 250 MHz (firmware reports 250000000, measures 400000000): nothing
assumed core 275 MHz (firmware reports 250000000, measures 400000000): 18 0c 84 03 0c 80
assumed core 300 MHz (firmware reports 250000000, measures 400000000): nothing
assumed core 325 MHz (firmware reports 250000000, measures 400000000): 08 03 28 20 00
assumed core 350 MHz (firmware reports 250000000, measures 399999000): 04 c1 12 08 e0
assumed core 375 MHz (firmware reports 250000000, measures 400000000): 04 10 01 00
assumed core 400 MHz (firmware reports 250000000, measures 400000000): 04 0e 04 01 03 0c 00  <- Command Complete
assumed core 450 MHz (firmware reports 250000000, measures 400000000): nothing
assumed core 500 MHz (firmware reports 250000000, measures 400000000): nothing
uart: core clock 400000000 Hz set by firmware (firmware measures 400000000), 115207 baud, flow none
baud sweep answered for core MHz: 400
power: cycled
uart: core clock 400000000 Hz set by firmware (firmware measures 400000000), 115207 baud, flow auto-low
reset at the driver's clock, flow auto-low: 04 0e 04 01 03 1c 01  <- Command Complete
reset at the driver's clock, flow none: nothing
```

  Then `echo none > /scheme/btuart/flow`:
  - `bt up` → `… 115207 baud, flow none`, **`bt: no answer to HCI command 0c03`**.
  - `bt info` → **answered**:

```
version: Bluetooth 4.1 (HCI 7, LMP 7), manufacturer 15 (Broadcom), subversion 6119, firmware build 0
address: AA:AA:AA:AA:AA:AA
name: BCM4345C0
```

  - `status` → `cts_line 0, rts_status 0, tx_fifo 0, rx_fifo 0, tx_done 1,
    lsr 0x60, msr 0x10, cntl 0x03, stat 0x0000030e`.
- (d) **0 lost bytes after (a)/(b), 224 after (c)** (`224 received bytes lost so far`).
  So the bytes were lost during `bt diag` or the `bt up`/`bt info` with flow `none`.
- In `bt diag`, the baud rate stays 115207 throughout (no drift, as intended).
  The reset at the driver's clock with `auto-low` still answers `03 1c 01`
  instead of `03 0c 00`.
- Reading: the chip talks (version, BCM4345C0, the default address before the
  firmware is loaded). `bt up`'s Reset right after the power cycle gets no
  answer, while a later `bt info` does. That looks like `bt up` sends too soon
  after power-on.

Step 4 still waits for the user's OK. Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-548-ge36b623` (`ls` on the netcfg directories)

Real Pi 3B+ over netboot. Test 128 ([#105](https://github.com/redox-rpi/redox-rpi/issues/105)), Pi test agent.

**Pass: every `ls` lists what the row expects, with no `Not a directory`.**
Image `~/redox-rpi-zrtl/keep/ncls-v0.3.0-dev-548-ge36b623.img` (no manifest next
to it; the one in the image passes), with the new stable's bootloader. No panic.

- `scripts/pi-test.py` standard checks: **pass** (version, manifest present,
  ps 46, ps-tty, sdcard, 0 of 300 starts failed).
- Over devsh:

```
ls /scheme/netcfg                         → ifaces  resolv  route
ls /scheme/netcfg/ifaces                  → eth0
ls /scheme/netcfg/ifaces/eth0             → addr  mac
ls /scheme/netcfg/ifaces/eth0/addr        → list  set
ls /scheme/netcfg/route                   → add  list  rm
cat /scheme/netcfg/ifaces/eth0/addr/list  → 192.168.178.148/24
```

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-205-g6b46fc0+xced2` (graphics-dma-order, kernel `0049`)

Real Pi 3B+ over netboot, NanoKVM on HDMI, serial only. Test 130 ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)), Pi test agent.

**Not shown: `rpi-fbd` didn't use DMA at all, so the larger DMA buffers were
never allocated.** Its DMA test blit failed (`test blit of 2 rows moved 4`), and it
fell back to CPU blits. There is no `DMA channel 5 … rows` line, and also no
`dumb buffer … in normal memory` line. No kernel panic and no `out of memory`.
Standard checks pass. Image
`~/redox-rpi-graphics/keep/netboot-v0.3.0-dev-205-g6b46fc0+xced2.img`, manifest
matches, with the new stable's bootloader.

- Display lines in the serial log:

```
vesad: 1824x984 stride 1824 at 0x3E914000
rpi-fbd: EDID 256 bytes, monitor NanoKVM
rpi-fbd: boot framebuffer 1824x984 stride 1824 at 0x3e914000
rpi-fbd: blits with the CPU: test blit of 2 rows moved 4
fbcond: Performing handoff
```

- The boot log also has `thread 'main' (1) panicked at
  drivers/pcid/src/cfg_access/fallback.rs:88:9`. That's pcid in userspace: this
  image is based on dev-205, from before pcid was left out on the Pi. It is not
  a kernel panic.
- `free`: 137,020 KB used of 743,312 (606,292 free).
- `scripts/pi-test.py` standard checks: **pass** (version, manifest matches, ps 48,
  ps-tty, sdcard, 0 of 300 starts failed). No new panic or `out of memory` during them.

So the allocator change ran under 300 starts without trouble, but the DMA path
it is meant for wasn't reached. Next: why the DMA test blit moves 4 instead of
2 rows. Stable image restored afterwards.

## 2026-10-31: test 127, icache-sync cost, like for like (`v0.3.0-dev-469-g7021aa9`)

Real Pi 3B+ over netboot. Test 127 ([#3](https://github.com/redox-rpi/redox-rpi/issues/3)), Pi test agent.

**Both images pass 1000 starts without a failure or exception. With the patch,
the `ls` loop is 1.0 s slower per 500 runs (about 1.3 %, 2 ms per program
start), the same in both rounds.** Images A
`~/redox-rpi-icache/build/icache-base.img` (`+x12cb`, without) and B
`icache-icache.img` (`+x12c9`, with `icache-sync`), run A, B, A, B, each on a fresh
boot with the new stable's bootloader. Both run at `arm 1400000000`.

- `scripts/pi-test.py --starts 1000` (first boot of each image): **pass** for A
  and B, 0 of 1000 starts failed.
- No exception and no panic in the serial log of any of the four boots (not
  even the `pcid` one).
- `time ion -c "for i in 1..=500; ls /usr > /dev/null; end"`, three times per boot:

| round | A without | B with `icache-sync` |
|---|---|---|
| 1 | 77.77 s, 77.81 s, 77.76 s | 78.75 s, 78.77 s, 78.84 s |
| 2 | 77.59 s, 77.46 s, 77.61 s | 78.62 s, 78.55 s, 78.55 s |
| mean | **77.67 s** | **78.68 s** (+1.01 s, +1.3 %) |

Within a boot, the runs vary by less than 0.15 s, so the difference is real but
small. Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-417-gd2482d8+x9903` (tls-variant1, static TLS layout in `ld.so`)

Real Pi 3B+ over netboot. Test 129 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14)), Pi test agent.

**Pass.** `stdio/mutex`, `tls_align` (dynamic and static) and `tls_shared` are
`FIXED`. The only `NEW` line is `pthread/exit` (SIGKILL), which the row allows.
Image `~/redox-rpi-t12/build/tls-pi.img`, with the new stable's bootloader.

- `scripts/pi-test.py --starts 1000`: **pass**, 0 of 1000 starts failed. No
  exception in the serial log during the starts.
- `cd /usr/share/relibc-tests`, 10 × `./bins_dynamic/stdio/mutex`: **10 × `rc=0`**.
- `ion /usr/share/relibc-tests/run.ion` (60 s, `COMMAND_TIMEOUT=60`), compared
  with `relibc-tests-compare.py` and `config/relibc-tests-expected.txt` from
  `t-tls-mutex`:

```
NEW ./bins_dynamic/pthread/exit: # ./bins_dynamic/pthread/exit: process exited unsuccessfully: signal: 9 (SIGKILL)
FIXED ./bins_dynamic/netdb/getaddrinfo: on the list, but passed
FIXED ./bins_dynamic/pthread/rwlock_randtest: on the list, but passed
FIXED ./bins_dynamic/stdio/mutex: on the list, but passed
FIXED ./bins_dynamic/sys_socket/getpeername: on the list, but passed
FIXED ./bins_dynamic/sys_socket/getsockopt: on the list, but passed
FIXED ./bins_dynamic/sys_socket/recv: on the list, but passed
FIXED ./bins_dynamic/sys_socket/recvfrom: on the list, but passed
FIXED ./bins_dynamic/tls_align: on the list, but passed
FIXED ./bins_dynamic/tls_shared: on the list, but passed
FIXED ./bins_static/tls_align: on the list, but passed
SUMMARY v0.3.0-dev-417-gd2482d8+x9903: 319 tests, 13 failed (12 known, 0 flaky, 1 new), 10 fixed
```

- The 6 `FATAL: Not an SVC induced synchronous exception` lines in the serial log
  all come from the known signal tests in `run.ion` (`signals/sigprocmask…`,
  `sigpause-…`, `sigsegv`, `sigset-5`, `stackoverflow`). None came from the 1000 starts.

Raw output: homenodellm `~/pitest-tools/t129/`. Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-515-gf41008d+x810f` (ctrlc-raw, Ctrl-C 0.01–0.03 s after Enter)

Real Pi 3B+ over netboot. Test 131 (left over from [#86](https://github.com/redox-rpi/redox-rpi/issues/86)), Pi test agent.

**Pass: on the serial console all 19 Ctrl-Cs stopped `sleep 40` 0.1 s after the
Ctrl-C, including the 15 at 0.01–0.03 s after Enter. Over devsh, 10 of 10.**
Staged with `~/t18-work/c86-stage/scripts/netboot.sh` (bootloader md5
`4fccdb94…`), image `~/t18-work/image-craw/harddrive.img` (md5 `2d55fab0…`),
manifest next to it.

- Serial console, logged in as root after the boot had settled (devshd answering).
  Delays `3,0.01,0.02,0.03,0.01,0.02,0.03,0.01,0.02,0.03,0.01,0.02,0.03,0.01,0.02,0.03,0.05,0.1,1`:
  **every round `stopped by SIGINT 0.1 s after Ctrl-C`**, `PASS: 0 late of 19`.
  The serial log has 19 times `ended by signal SIGINT`.
- How, as in #118: not `serial-ctrlc-test.py` itself, which reads
  `/dev/ttyUSB0` directly. I ran the same rounds and checks with
  `~/pitest-tools/ctrlc118.py`: the logging service stayed on and the script
  read its file. `sleep 40` was typed one character every 0.15 s, and the delay
  counts from the Enter. The Ctrl-C is one byte.
- devsh: `patches/experiments/pty-sigint/ctrlc-test.py 192.168.178.148 --rounds 10
  --delays 0,0.01,0.02,0.03,0.1` (from `ctrlc-raw`): 10 of 10 `stopped by SIGINT
  0.1 s after Ctrl-C`, and after a Ctrl-C at the prompt the shell still answers: **PASS**.
- Standard checks with `--manifest`: all pass (manifest matches, 0 of 300 starts failed).

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-410-g313d292+x4ec1` (relibc-quiet, sshd without TODO lines)

Real Pi 3B+ over netboot. Test 132 ([#109](https://github.com/redox-rpi/redox-rpi/issues/109)), Pi test agent.

**Pass: 5 key logins and `ssh-session.exp` work, and the serial log has no
`TODO: madvise`, `getsockopt` or `setsockopt` line.** The only `TODO` line in
the whole boot is one `TODO: ioctl TIOCSCTTY` from the `ssh-session.exp` session
(a terminal session, not one of the three this row is about). Image
`~/redox-rpi-relibc/keep/netboot-v0.3.0-dev-410-g313d292+x4ec1.img` (no manifest
next to it), with the new stable's bootloader. Same setup as #120.

- `TODO` lines in the serial log after boot, before any login: **0**.
- The test key (`ed25519`) went into `/scheme/data/home/root/.ssh/authorized_keys`, as in #120.
- 5 × `ssh -i <key> root@192.168.178.148 echo ok`: **5 × `ok`**, 0.98–1.06 s each.
- `ssh-session.exp`: command, prompt again, backspace, `exit` all ok; it exits at once.
- `TODO` lines after all of this: 1, `[INFO sshd-session@src/header/sys_ioctl/redox/mod.rs:141]
  TODO: ioctl TIOCSCTTY`. No `madvise`, `getsockopt` or `setsockopt` (#101 had
  several per connection).
- `ssh-keyscan -t rsa,ecdsa,ed25519 192.168.178.148`: **3 keys** (`ssh-rsa`,
  `ecdsa-sha2-nistp256`, `ssh-ed25519`).
- Standard checks: all pass (manifest present in the image, 0 of 300 starts
  failed). No panic or unhandled exception.
- The key file was removed from `/scheme/data/home/root/.ssh/`.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-574-g55e47fb+xb3d9` (self-exit-status, `pthread_exit` of the last thread)

Real Pi 3B+ over netboot. Test 133 ([#108](https://github.com/redox-rpi/redox-rpi/issues/108)), Pi test agent.

**Pass: `pthread/exit` exits with status 0 (10 of 10), no memory growth over
200 `dd` runs, and the relibc suite has no `NEW` line.** Image
`~/redox-rpi-t12/keep/selfexit-v0.3.0-dev-574-g55e47fb+xb3d9.img` (no manifest
next to it; the one in the image passes), with the new stable's bootloader.

- Standard checks: all pass (manifest present in the image, 0 of 300 starts failed).
- `cd /usr/share/relibc-tests`, 10 × `./bins_dynamic/pthread/exit`: **10 × `rc=0`** (6.5 s).
- `free` → 137,348 KB used. Then 200 × `dd if=/dev/zero of=/dev/null bs=1k count=1`
  (33 s). `free` → 138,172 KB used: **+824 KB**, about 4 KB per run (before
  `0008`: about 76 KB per run). Under the 1 MB limit.
- `ion /usr/share/relibc-tests/run.ion` (65 s, `COMMAND_TIMEOUT=60`), compared
  with `relibc-tests-compare.py` and `config/relibc-tests-expected.txt` from
  `t13-relibc-tests`:

```
FIXED ./bins_dynamic/netdb/getaddrinfo: on the list, but passed
FIXED ./bins_dynamic/pthread/rwlock_randtest: on the list, but passed
FIXED ./bins_dynamic/sys_socket/getpeername: on the list, but passed
FIXED ./bins_dynamic/sys_socket/getsockopt: on the list, but passed
FIXED ./bins_dynamic/sys_socket/recv: on the list, but passed
FIXED ./bins_dynamic/sys_socket/recvfrom: on the list, but passed
SUMMARY v0.3.0-dev-574-g55e47fb+xb3d9: 319 tests, 16 failed (16 known, 0 flaky, 0 new), 6 fixed
```

  No `NEW` line, and `pthread/exit` passes in the suite too (in #129 it was `NEW` with SIGKILL).

Raw output: homenodellm `~/pitest-tools/t133/`. Stable image restored afterwards.

## 2026-10-31: test 135, dwc2d waits for the channel interrupt (baseline vs `dwc2d-wait-irq`)

Real Pi 3B+ over netboot. Test 135 ([#104](https://github.com/redox-rpi/redox-rpi/issues/104)), Pi test agent.

**Partly done. Both images pass the standard checks and hotplug (network 3 of 3,
keyboard 1 of 1) with no hang, and the wait-irq image has no `transfers spin`
line. The TCP step was not measured: the Pi's `nc` doesn't send what it reads
from a pipe. No USB stick was in, so the stick step falls to the USB visit.** So
the main comparison, dwc2d ACTTIME during long transfers, is still open.
Images `~/redox-rpi-usb/keep/storage3-v0.3.0-dev-441-g81d8d88+xf757.img`
(baseline, as #71) and `wait-irq-v0.3.0-dev-449-g461061e+x885b.img`, both
manifests match, with the new stable's bootloader.

- Boot log, both images: `dwc2d: interrupt /scheme/irq/phandle-1/1,9`,
  `dwc2d: keyboard at address 5` (the NanoKVM's USB), no `mass storage` line, no
  panic. The wait-irq image has **no `dwc2d: transfers spin` line**.
- Standard checks with `--manifest`: **pass** for both (0 of 300 starts failed).
- dwc2d ACTTIME (from `grep dwc2d /scheme/sys/context`) before → after:

| step | baseline | wait-irq |
|---|---|---|
| (1) 30 s idle | 1.77 → 2.49 s (**+0.72 s**) | 2.14 → 2.96 s (**+0.82 s**) |
| (3) `pi-hotplug-test.sh 3` with `KEYBOARD=1` | 9.88 → 11.02 s (+1.14 s) | 10.35 → 11.30 s (+0.95 s) |
| (2) TCP send | not measured | not measured |
| (4) stick | no stick | no stick |

- (3) Hotplug: **pass on both**. Network: port off, device removed, found again,
  network on the adapter again, devshd answers again, 3 of 3 each. Keyboard:
  1 of 1. `cycle: hub 2 port 1 disconnected after` 102–114 ms (baseline) and
  101–113 ms (wait-irq).
- (2) Why no TCP number: the Pi's `nc` (`nc [-l] host:port`) connects
  (`Remote host: 192.168.178.120:5135`), but with
  `dd if=/dev/zero bs=65536 count=256 | nc 192.168.178.120:5135` the host
  receives 0 bytes and `dd` stops with `Broken pipe` after `0+0 records`. The
  same happens with `echo hello`. A try the other way round, with the Pi
  listening (`nc -l 0.0.0.0:5136 > /dev/null`), accepted the connection but gave
  no clear byte count. There's no `sh`, `curl` or `wget` on these images.
  During these tries dwc2d's ACTTIME on the wait-irq image rose from 18.7 s to
  34.4 s, so the failed connections cost CPU in dwc2d. That needs a look, but it's
  no measurement.

The row stays queued for the TCP step, which needs a sender that works on the Pi.
Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-568-g81ac5d9` (kernel `0057`, scheme-root open leaves no page)

Real Pi 3B+ over netboot. Test 136 ([#88](https://github.com/redox-rpi/redox-rpi/issues/88)), Pi test agent.

**Pass: `SHARED` of netstack and randd is the same before and after 200 opens
of each scheme root, and the standard checks pass.** Image
`~/redox-rpi-zrtl/keep/tail-v0.3.0-dev-568-g81ac5d9.img` (no manifest next to it;
the one in the image passes), with the new stable's bootloader. No panic.

- `scripts/pi-test.py` standard checks: **pass** (manifest present, ps 41, ps-tty,
  sdcard, 0 of 300 starts failed).
- One devsh command (65 s), the four `grep` lines:

```
22    0     0     UB    #2    F          00:00:00.20 00:00:00.24 3.22 MB 16.0 KB /usr/bin/netstack
3     0     0     UB    #3    F          00:00:00.00 00:00:00.01 1.55 MB 0 B     /scheme/initfs/bin/randd
22    0     0     UB    #0    F          00:00:00.30 00:00:00.36 3.22 MB 16.0 KB /usr/bin/netstack
3     0     0     UB    #2    F          00:00:00.01 00:00:00.01 1.55 MB 0 B     /scheme/initfs/bin/randd
```

  netstack `SHARED` 16.0 KB → 16.0 KB, randd 0 B → 0 B. Without the fix, each
  grows about 800 KB. PRIVATE didn't change either.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-482-g7623557` (netstack-multi2, lease renewal and an adapter that goes away)

Real Pi 3B+ over netboot. Test 140 ([#53](https://github.com/redox-rpi/redox-rpi/issues/53)), Pi test agent.

**Pass: renewals, dropping a lost adapter, a new adapter, and a lease running
out all work. eth0 kept the default route and devsh answered throughout.**
Image `~/redox-rpi-zrtl/keep/nsm2-v0.3.0-dev-482-g7623557.img` (no manifest next
to it; the one in the image passes), with the new stable's bootloader. No panic.

- Standard checks before and after: **pass** both times (ps 47 then 50, 0 of 300 starts failed).
- (1) `/usr/lib/drivers/testnetd network.test0 30 &`, then 50 s:
  `DHCP: eth1 got 10.0.9.2/24` and three `DHCP: eth1 renewed 10.0.9.2 for 30 s`
  (a fourth came later). `/tmp/testnetd.log`:
  `dhcp 5 dhcp_foreign 0 foreign_src 0 ping 0 renew 3`. `route/list` keeps
  `default via 192.168.178.1 dev eth0`, plus `10.0.9.0/24 dev eth1`.
- (2) `kill $(cat /tmp/network.test0.pid)` (pid 368), then 10 s: `… link eth1:
  No such device (os error 19); dropping it` and `DHCP: eth1 is gone`.
  `route/list` has **no eth1 line** and still the eth0 default route.
  `cat /scheme/netcfg/ifaces/eth1/addr/list` fails with `Invalid argument`.
- (3) `/usr/lib/drivers/testnetd network.test1 30 &`, then 30 s:
  `DHCP: eth2 got 10.0.9.2/24`, `ping -c 3 10.0.9.1` **3/3** (2.8–3.7 ms),
  `route/list` has `10.0.9.0/24 dev eth2` and the same eth0 default route.
- (4) `echo x > /tmp/testnetd-nodhcp`, then 50 s: `DHCP: eth2 lease of 10.0.9.2
  ended`; eth0 default route unchanged. After `rm /tmp/testnetd-nodhcp`, eth2 got
  the lease again and renewed it.
- Every `DHCP:` line and the `dropping it` line from the serial log:

```
DHCP: eth0 got 192.168.178.148/24
DHCP: default route via 192.168.178.1 (eth0)
DHCP: eth1 got 10.0.9.2/24
DHCP: eth1 renewed 10.0.9.2 for 30 s
DHCP: eth1 renewed 10.0.9.2 for 30 s
DHCP: eth1 renewed 10.0.9.2 for 30 s
DHCP: eth1 renewed 10.0.9.2 for 30 s
smolnetd@netstack::link::ethernet:359 ERROR] Failed to read ethernet device on link eth1: No such device (os error 19); dropping it
DHCP: eth1 is gone
DHCP: eth2 got 10.0.9.2/24
DHCP: eth2 renewed 10.0.9.2 for 30 s
DHCP: eth2 renewed 10.0.9.2 for 30 s
DHCP: eth2 renewed 10.0.9.2 for 30 s
DHCP: eth2 lease of 10.0.9.2 ended
DHCP: eth2 got 10.0.9.2/24
DHCP: eth2 renewed 10.0.9.2 for 30 s
```

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-562-g55a1014+x4590` (test 126, saved prompt state and writing speed)

Real Pi 3B+ over netboot. Test 126 ([#43](https://github.com/redox-rpi/redox-rpi/issues/43)), Pi test agent.

**The download now finishes, the assistant answers with the time, and writing
speed is measured. The saved prompt state doesn't work on the Pi:** the cache is
written as `prompt.kv.part` with exactly 43,581,693 bytes but never renamed to
`prompt.kv`. So there is no `saved` line, and the second run reads the 380
tokens again instead of loading them. This looks like the rename problem on
`/scheme/data` that test 124's image fixed in RedoxFS; this image doesn't have
that fix. **After the first run, devshd stopped accepting connections**
(`Connection refused`); the serial console still worked and the devshd process
was still there. Image
`~/redox-rpi-assistant/keep/netboot-v0.3.0-dev-562-g55a1014+x4590.img`, with the
new stable's bootloader. Standard checks pass (6 of 6).

- Download (resumed): `kept parts 000 to 011 from an earlier try, 192 MiB`, then
  parts 012–020 at 0.48–1.95 MB/s, `341455552 bytes in 126 s`. Two
  `smolnetd: Dropped packet on eth0 because waiting queue was full` lines at 07:57.
- First `assistant --ask 'What time is it?'` (07:58):
  `loaded in 21.6 s`, **`rinfer: read 380 tokens in 166.1 s`** (#112: about 170 s),
  then the answer `It is Saturday, 31 October 2026, 08:01 UTC.` Then devsh went
  silent; the next commands got `Connection refused`. `ls -l /scheme/data` over
  the serial console: `prompt.kv.part` 43,581,693 bytes, no `prompt.kv`. I
  reset over the serial console, with the same image and the model kept.
- Second `assistant --ask` after the reset (09:01–09:05, 255 s total):
  `loaded in 20.7 s`, again **`read 380 tokens in 170.5 s`** (no `loaded 380 tokens`),
  answer `It is Saturday, 31 October 2026, 09:05 UTC.` Again only `prompt.kv.part`.
  devsh kept working this time.
- Writing speed, `rinfer … 'Name three colours.' 32` (answer `Three colours are
  blue, green, and red.`):

| threads | load | prompt | **generated** |
|---|---|---|---|
| 4 (default) | 20.7 s | 20 tokens, 2.03 t/s | **10 tokens in 12.8 s, 0.78 t/s** |
| 3 (`RAYON_NUM_THREADS=3`) | 20.5 s | 20 tokens, 2.25 t/s | **10 tokens in 8.1 s, 1.24 t/s** |

  In the assistant runs, the writing speed was 0.86–1.22 t/s (#112: 1.0).
- Temperature: 52.1 °C and `throttled 0x00000000` before. After the runs, 60.1 °C
  and `0x00080000`: the soft temperature limit was hit, and the console logged
  `soft temperature limit … 60.1 °C, ARM 1400 MHz` and `no longer throttled`
  many times. So the 4-thread numbers may be held back by the heat.
- `free` from a second devsh: the lowest was 120,256 KB during the first run and
  130,444 KB during the second (start 736,520 KB), so it never got near 50 MB.
  Heap peak 430 MiB. No kernel panic, `oom` or `RELIBC PANIC` on the console.
- At the end, `prompt.kv.part` was removed. The model stays on the card
  (`/scheme/data/Qwen3-0.6B-Q4_0e.gguf`, 341,455,552 bytes).

Stable image restored afterwards.

## 2026-10-31: test 134, boot time without U-Boot countdown and `boot_delay` (stable `v0.3.0-dev-533-g6c729cd+x9f76`)

Real Pi 3B+ over netboot. Test 134 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47)), Pi test agent. Run with the user's OK.

**Stopped as the row says: with `/srv/tftp/ed08e0aa` in place, the Pi still
counted `2 1 0` and ran the same U-Boot build, so it didn't use the folder.**
B boots 1 and 2 are the same as A to within 0.1 s. Most likely the Pi loads its
firmware and U-Boot from its SD card (U-Boot then fetches only
`/redox/harddrive.img.gz` and `bootloader-live.efi` over HTTP, as the server log
shows), so bootcode.bin never asks for `/srv/tftp/<serial>/`. tftpd-hpa runs
without request logging, so I can't confirm this from the TFTP side.

Seconds after the start of the timeline (reset over devsh), with `boot-timeline.py`:

| boot | `kreset` | U-Boot banner | autoboot `2` / `1` / `0` | DHCP bound | `redox login:` |
|---|---|---|---|---|---|
| A1 | 1.55 | 3.76 | 8.27 / 9.27 / 9.27 | 11.22 | 33.51 |
| A2 | 1.55 | 3.76 | 8.28 / 9.29 / 9.29 | 11.19 | 33.04 |
| A3 | 1.56 | 3.77 | 8.24 / 9.25 / 9.25 | 11.21 | 33.11 |
| B1 (folder in place) | 1.55 | 3.75 | 8.26 / 9.26 / 9.26 | 11.16 | 33.11 |
| B2 (folder in place) | 1.55 | 3.76 | 8.26 / 9.26 / 9.26 | 11.17 | 33.12 |

- The U-Boot banner is `U-Boot 2026.10 (Oct 25 2026 - 08:30:37 +0000)` in every
  boot, A and B, so the branch's `u-boot.bin` was not used either.
- Reset to banner is 2.2 s; banner to `redox login:` is about 29.3 s.
- After B2: the runner was stopped, `rm -r /srv/tftp/ed08e0aa` was done (checked:
  gone), and one more boot of stable counted `2 1 0` and reached `redox login:` at
  32.96 s. No standard checks for B, since B never ran the new files.

To test this, the new `config.txt` and `u-boot.bin` must go where this Pi really
loads them from, likely the SD card's FAT partition, which needs a card swap or
another approach. The row stays queued. Stable image in place.

## 2026-10-31: test 135 step 2 again, TCP send from a file (`v0.3.0-dev-449-g461061e+x885b`)

Real Pi 3B+ over netboot. Test 135 ([#104](https://github.com/redox-rpi/redox-rpi/issues/104)), Pi test agent.

**Not measurable: the file form doesn't send anything either.** On the
wait-irq image, `dd if=/dev/zero of=/tmp/tx bs=65536 count=256` made
`/tmp/tx` (16,777,216 bytes). Then twice `time nc 192.168.178.120:5135 < /tmp/tx`:
each printed `Remote host: 192.168.178.120:5135` and ended after 0.04 s
(`real 0.043949064s`, `0.042097759s`). `nc -l 5135 | wc -c` on homenodellm counted
**0 bytes** both times. dwc2d ACTTIME across each `nc`: 0.64 → 0.65 s and
7.74 → 7.76 s. Between the two tries it rose at its idle rate (about 1.6 s per
minute, as in step 1), so the failed sends themselves cost nothing.
`/tmp/tx` was removed afterwards.

With this, step 2 is recorded as not measurable, as the usb session asked; the
rest is in the first 135 report. Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-561-g772e33b+x4af4` (PR #113 image with `0068`, DMA test blit)

Real Pi 3B+ over netboot, NanoKVM on HDMI, serial only. Test 138 ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)), Pi test agent.

**DMA now works (`2D mode moves YLENGTH+1 rows`, no `not zero` line), but the
first boot hung right after `init: switchroot to /usr /etc`: no login, no
network, no devsh, from 09:25 UTC on.** Only a power cycle can get the Pi back,
and that waits for the user's OK. So there is no second boot, `free` or
standard check yet. Image
`~/redox-rpi-graphics/keep/netboot-v0.3.0-dev-561-g772e33b+x4af4.img`, manifest
matches, with the new stable's bootloader.

- Boot 1, serial log:

```
redox-rpi: network boot from 192.168.178.120
redox-rpi: image loaded over HTTP
init: switchroot to /scheme/initfs /scheme/initfs/etc
rpi-fbd: EDID 256 bytes, monitor NanoKVM
rpi-fbd: boot framebuffer 1824x984 stride 1824 at 0x3e914000
rpi-fbd: DMA channel 5 at 0x3f007000, 2D mode moves YLENGTH+1 rows
Finished graphical debug
fbbootlogd: handoff requested
init: switchroot to /usr /etc
```

  Nothing came after that, not even the `fbcond: Performing handoff` that
  follows in a normal boot. There's no `DMA test buffer had N of 64 words not
  zero` line, no `blits with the CPU` line, no `dumb buffer … in normal memory`
  line, and no panic or `out of memory` line.
- The Pi is unreachable (`No route to host`). The Shelly plug reads `output: true`,
  4.1 W. The stable image is staged again, so the power cycle boots stable.

The hang comes after rpi-fbd started DMA blits, around the handoff to fbcond.
That fits the DMA path or the large DMA buffers (`0049`) stalling the boot, but
the log doesn't prove it. The row stays queued for the second boot and the checks.

## 2026-10-31: `v0.3.0-dev-491-g67c7d1d+x125f` (mqtt-cert, MQTT client certificates)

Real Pi 3B+ over netboot. Test 139 ([#115](https://github.com/redox-rpi/redox-rpi/issues/115)), Pi test agent.

**Pass: all three steps and the standard checks.** Image
`~/mqtt-test/keep/cc-netboot-v0.3.0-dev-491-g67c7d1d+x125f.img`, manifest
matches, with the stable bootloader. Broker `client-cert-broker.sh
192.168.178.120 3600` from branch `mqtt-cert` (port 8884, `require_certificate`);
certificate installed with `install-client-cert.py` (`/etc/mqtt-client.crt`
595 bytes, `/etc/mqtt-client.key` 241 bytes, mode 600).

| check | result |
|---|---|
| standard checks | **pass**: version, manifest, ps, ps-tty, sdcard; 0 of 300 starts failed |
| (1) `mqtt-pub -p 8884 -t cc/pi -m with-cert -q 1` | **pass**: exit 0; broker subscriber `2026-10-31T12:49:18+0000 cc/pi with-cert`; mosquitto `New client connected from 192.168.178.148:49157 as mqtt-pub-396 (p2, c1, k60, u'redox-rpi')` |
| (2) without the certificate | **pass**: exit 1, `mqtt-pub: 192.168.178.120:8884: TLS: received fatal alert: CertificateRequired (the broker wants a client certificate: give it with --cert and --key)`; nothing new at the subscriber; mosquitto `peer did not return a certificate`, `Client <unknown> disconnected: Protocol error.` |
| (3) `ha-bridge -p 8884` | **pass**: exit 0; `redox-rpi/redox/state {"cpu_temperature":54.8,"cpu_clock":1400,"throttled":"OFF","uptime":135,"memory_free":576.3,"memory_used":20.6}`; mosquitto `as ha-bridge-402 (… u'redox-rpi')` |

- `free` afterwards: 743612 total, 152048 used, 591564 free; `throttled`
  `0x00000000`; no `panicked` or `RELIBC PANIC` on the serial console.
- Broker stopped afterwards (port 8884 closed); the other test brokers on
  homenodellm were left running.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-601-g5db6321+x675f` (tcp-slow, long download)

Real Pi 3B+ over netboot. Test 141 ([#119](https://github.com/redox-rpi/redox-rpi/issues/119)), Pi test agent.

**Network alone is fast and steady: 382156480 bytes in 51.5 s, 7.42 MB/s, no
errors. Network and card: the Pi hung.** No serial line and no `netget` output
before it stopped. Image
`~/redox-rpi-zrtl/keep/tcpdiag-v0.3.0-dev-601-g5db6321+x675f.img` (manifest in
the image), stable bootloader. Standard checks pass (version, manifest, ps,
ps-tty, sdcard, 0 of 300 starts failed).

| step | result |
|---|---|
| (1) at uptime 61 s | `rx_frames 624`, `rx_errors 0`, `tx_frames 257`, `tx_errors 0`, `irqs 598`, `ticks 3149`; netstack 0.19 s / 0.23 s, dwc2d 1.81 s / 2.88 s; ping 1.7–94.6 ms, avg 35.6 ms, 0 % loss |
| (2) network only | `HTTP/1.0 200 OK`, then each 16 MiB in 2.2–2.4 s (7.25 to 7.60 MB/s, no slowdown), **`382156480 bytes in 51.5 s, 7.42 MB/s`** |
| (2) second devsh (5 min, after the download) | ping 1.34–1.55 ms; `rx_frames 270989`, `rx_errors 0`, `tx_frames 131961`, `tx_errors 0`, `irqs 259198`, `ticks 703506` |
| (3) at uptime 397 s | `rx_frames 271082`, `rx_errors 0`, `tx_errors 0`; netstack 21.24 s / 34.06 s, dwc2d 22.47 s / 9.80 s; ping 1.8–22.9 ms, 0 % loss |
| (4) network and card | `netget … /scheme/data/netget.bin` started 12:59:03 UTC (server log: GET at 12:59:05). **No output in 60 minutes; the Pi hung.** The serial console wrote nothing after the boot. The HTTP server kept sending until `OSError: [Errno 113] No route to host` at 13:19:57, so the Pi stopped answering some time before that. No ping, ARP incomplete |
| (5) | not run (Pi unreachable) |

- Serial lines with `Dropped`, `netstack` or `dwc2d`: the boot's `dwc2d`
  enumeration (LAN7800, `link up after 1710 ms`) and two `smolnetd … Dropped
  packet on eth0 because waiting queue was full` at uptime 151 s (during step 2).
  Nothing later; no panic line.
- `df` does not list `/scheme/data`; the card has room for #126's 341 MB model.
- `netget`'s lines only reach devsh when it ends, so the step 4 progress is lost.
  A rerun could write them to the serial console or a file to show how far it got.
- The Pi's clock read 1970 (no timesync in this image).
- Found during #143: the hang left `/scheme/data/netget.bin` with 359464809 of
  382156480 bytes (94 %, 342.8 MiB), so step 4 hung late in the download. Removed.

Stable image staged afterwards; the Pi needs a power cycle to boot it.

## 2026-10-31: `v0.3.0-dev-564-gfaa76a7+x4590` (assistant, saved prompt state)

Real Pi 3B+ over netboot. Test 143 ([#43](https://github.com/redox-rpi/redox-rpi/issues/43)), Pi test agent.

**Pass: the prompt state is saved as `prompt.kv` and loaded on the next run.**
The prompt step goes from 179.4 s to 2.3 s, and the whole question from 266 s to
75 s. Image `~/redox-rpi-assistant/keep/netboot-v0.3.0-dev-564-gfaa76a7+x4590.img`,
manifest matches, stable bootloader. Standard checks pass (0 of 300 starts
failed). The model was on the card (`Qwen3-0.6B-Q4_0e.gguf` 341455552 bytes);
no `prompt.kv.part` or `prompt.kv` before.

| | first run | second run |
|---|---|---|
| model | `loaded in 21.2 s`, threads 3 | `loaded in 20.7 s` |
| prompt | `read 380 tokens in 179.4 s`, **`saved 380 tokens in 5.8 s`** | **`loaded 380 tokens in 2.3 s`** |
| answer | `It is Saturday, 31 October 2026, 14:35 UTC.`, then `The current time is 14:35 UTC.` | same, 14:37 UTC |
| writing | 1.24 t/s and 1.06 t/s | 1.44 t/s and 1.23 t/s |
| total (`date -u` before/after) | 14:31:40 to 14:36:06, 266 s | 14:36:20 to 14:37:35, 75 s |
| peak heap | 430 MiB | 407 MiB |

- `prompt.kv` after the first run: 43581693 bytes, as expected.
- The first run prints `rinfer: No such file or directory (os error 2)` once
  before reading the prompt (no cache yet); harmless.
- devsh accepted connections after each run (#126: `Connection refused`).
- `free` from a second devsh during the second run (14:37:10): 733584 used,
  140844 free of 874428; after it 237548 used.
- Temperature 52.6 °C before, 62.3 °C after; `throttled` `0x00000000` before,
  **`0x00080008` after** (soft temperature limit reached and active).
- No panic, no `oom`, no `RELIBC PANIC`. `prompt.kv` removed; the model kept.
- Also removed `/scheme/data/netget.bin` left by #141's hang. It held
  359464809 of 382156480 bytes (94 %), so the #141 hang came late in the
  download to the card.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-451-g7203785` baseline `+x644c` and wait-irq `+x98e9` (TCP send)

Real Pi 3B+ over netboot. Test 144 ([#104](https://github.com/redox-rpi/redox-rpi/issues/104)), Pi test agent.

**Pass: the host counted 16777216 bytes in all four runs, and devshd answered
afterwards.** Both images send at 8.3 to 8.9 MB/s. dwc2d's ACTTIME rises the same
on both, by about 1.0 s per 16 MiB send, so wait-irq is not higher, but also no
lower. Images from `~/redox-rpi-usb/keep/` with their manifests (match), stable
bootloader. Standard checks with `--manifest` pass on both (0 of 300 starts
failed). Host side: OpenBSD `nc -l <port> | wc -c`.

| image | run | `tcp-send` | host bytes | dwc2d ACTTIME before → after | SCHTIME before → after |
|---|---|---|---|---|---|
| baseline | 1 | `16777216 bytes in 1.89 s, 8.88 MB/s` | 16777216 | 1.80 → 2.80 s (**+1.00**) | 1.93 → 2.05 s (+0.12) |
| baseline | 2 | `16777216 bytes in 2.00 s, 8.39 MB/s` | 16777216 | 2.95 → 3.96 s (**+1.01**) | 2.19 → 2.22 s (+0.03) |
| wait-irq | 1 | `16777216 bytes in 2.01 s, 8.33 MB/s` | 16777216 | 1.67 → 2.66 s (**+0.99**) | 1.82 → 1.84 s (+0.02) |
| wait-irq | 2 | `16777216 bytes in 2.01 s, 8.36 MB/s` | 16777216 | 2.72 → 3.72 s (**+1.00**) | 1.89 → 1.91 s (+0.02) |

- ACTTIME is the first time column of `/scheme/sys/context` (as in #135); each
  before/after pair also covers about 0.4 s of devsh between the commands.
- So dwc2d is busy about half the 2 s a send takes, the same with and without
  wait-irq: the sent frames still spin, as the row expected.
- `free` afterwards: baseline 134300 used of 743188, wait-irq 145984 of 743180.
  No panic.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-562-g9a33382+x4af4` (rpi-fb-pr, DMA test with CPU fallback)

Real Pi 3B+ over netboot. Test 145 ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)), Pi test agent.

**Boot hung during the DMA test, at the 64-row blit.** The 3 s fallback did not
catch it; the machine stopped. Image
`~/redox-rpi-graphics/keep/netboot-v0.3.0-dev-562-g9a33382+x4af4.img` (manifest
next to it), stable bootloader, staged and reset over devsh at about 14:42 UTC.

Every `rpi-fbd` line, in order (serial console):

```
rpi-fbd: EDID 256 bytes, monitor NanoKVM
rpi-fbd: boot framebuffer 1824x984 stride 1824 at 0x3e914000
rpi-fbd: DMA test buffer had 34 of 64 words not zero
rpi-fbd: DMA channel 5 at 0x3f007000, 2D mode moves YLENGTH+1 rows
rpi-fbd: DMA test: buffer of 1824x984
rpi-fbd: DMA test: 1 rows into the framebuffer
rpi-fbd: DMA test: 64 rows into the framebuffer
```

- **Last line: `DMA test: 64 rows into the framebuffer`**, then nothing; the
  serial log stopped at 14:43:38. By the row's key: the 64-row DMA blit stopped
  the machine (the full-screen buffer and the 1-row blit worked). No
  `984 rows`, no `whole screen in`, no `blits with the CPU` line, no
  `fbcond: Performing handoff`, no login, no panic line.
- The last init line before it was `init: switchroot to /scheme/initfs
  /scheme/initfs/etc` (#138 got as far as `switchroot to /usr /etc`).
- The Pi did not answer ping (ARP incomplete) 10 minutes later.
- Stable staged, then the Pi was power-cycled with the Shelly plug at 14:53
  UTC; stable booted to login and devsh answered.
- As the coordinator asked, no second graphics image today.

## 2026-10-31: `v0.3.0-dev-671-g157dad0+x9d6b` (t117-test, a scheme name registered again)

Real Pi 3B+ over netboot. Test 146 ([#117](https://github.com/redox-rpi/redox-rpi/issues/117)), Pi test agent.

**Pass: after `pkill -9 fatfsd`, a new fatfsd registers `t117` again and serves
the boot files; no `File exists`.** Image
`~/redox-rpi-sd/keep/t117-v0.3.0-dev-671-g157dad0+x9d6b.img`, manifest matches,
stable bootloader. Standard checks with `--manifest` pass (0 of 300 starts
failed). Read only, on the FAT boot partition `/scheme/disk.mmc/0p0`.

| step | result |
|---|---|
| `fatfsd /scheme/disk.mmc/0p0 t117 &`, `ls /scheme/t117` | `ion: bg [0] 364`; lists `LICENCE.broadcom`, `bcm2710-rpi-3-b-plus.dtb`, `boot.scr`, `bootcode.bin`, `config.txt`, `fixup.dat`, `overlays`, `start.elf`, `u-boot.bin` … |
| `pkill -9 fatfsd`, `ps` | no fatfsd in `ps` |
| `ls -1 /scheme` | still lists `t117` |
| `ls /scheme/t117` | `No such device` (os error 19) |
| `fatfsd /scheme/disk.mmc/0p0 t117 &`, `ls /scheme/t117` | `ion: bg [0] 381`; **lists the boot files again**; no `File exists (os error 17)` |
| serial | `[src/initnsmgr.rs:92 WARN] nsmgr: scheme t117 ended, registering it again` |
| `pkill fatfsd` | `fatfsd: unmounting /scheme/t117`, `fatfsd: unmounted, the disk can be removed`; no fatfsd left |

- The `fatfsd: serving /scheme/t117` line did not show: a background job's
  output does not reach devsh, and fatfsd does not write to the serial console.
  The second fatfsd served the files, which shows it registered.
- No panic.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-464-gff58612+x7d9f` (bluetooth, step 11: power-on wait and Reset retries)

Real Pi 3B+ over netboot. Test 109 step 11 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54)), Pi test agent.

**The chip did not answer Reset on any try, with `auto-low` or `none`, so no
upload ran.** No `ignored` events, no lost bytes. Image
`~/redox-rpi-cpu/keep/v0.3.0-dev-464-gff58612+x7d9f/harddrive.img`, manifest
matches, stable bootloader. Standard checks with `--manifest` pass (0 of 300
starts failed). Flow at start: `auto-low`.

(a) flow `auto-low`:

```
### bt up  (12.0 s)
power: cycled
uart: core clock 400000000 Hz set by firmware (firmware measures 400000000), 115207 baud, flow auto-low
reset: try 1: no answer to HCI command 0c03, again
reset: try 2: no answer to HCI command 0c03, again
bt: Connection timed out (os error 110)
### bt info  (0.9 s)
bt: Connection timed out (os error 110)
### bt scan 10  (0.9 s)
bt: Connection timed out (os error 110)
### bt inquiry 10  (5.8 s)
bt: Connection timed out (os error 110)
```

(b) flow `none`:

```
### bt up  (8.6 s)
power: cycled
uart: core clock 400000000 Hz set by firmware (firmware measures 400000000), 115207 baud, flow none
reset: try 1: no answer to HCI command 0c03, again
reset: try 2: no answer to HCI command 0c03, again
bt: no answer to HCI command 0c03
### bt info  (2.4 s)
bt: no answer to HCI command 1001
### cat /scheme/btuart/status
cts_line 1, rts_status 0, tx_fifo 0, rx_fifo 0, tx_done 1, lsr 0x60, msr 0x00, cntl 0x03, stat 0x0000038e
```

- (c) `received bytes lost` lines on the serial console: **0**. No
  `(event … ignored: …)` line and no `reset: answered` in either run. Serial:
  `bcm2835-btuartd: mini UART at 0x3f215000, 115207 baud (core 400 MHz)`. No
  panic.
- With `auto-low` the third try ends in `Connection timed out (os error 110)`;
  after that every `bt` command times out at once (0.9 s). With `none` the third
  try ends in `no answer`.
- Unlike step 10, `bt info` with `none` did not answer here either. In step 10 it
  ran before any `bt up`; here it came after two `bt up` power cycles. CTS
  stays `1` (high) after both.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-466-g54ea9bf+x7d9f` (bluetooth, step 12: WL_ON before `bt up`)

Real Pi 3B+ over netboot. Test 109 step 12 ([#54](https://github.com/redox-rpi/redox-rpi/issues/54)), Pi test agent.

**Pass: Bluetooth comes up.** With WL_ON switched on first, Reset answers, the
323-record firmware loads in 5.9 s, the chip reports build 382 with a real
address, and an LE scan finds 22 devices. Image
`~/redox-rpi-cpu/keep/v0.3.0-dev-466-g54ea9bf+x7d9f/harddrive.img` (no manifest
next to it), stable bootloader. Nothing else ran over devsh before; flow at
`auto-low`.

```
### cat /scheme/rpi-firmware/wifi_power
0
### bt up  (7.9 s)
power: wifi_power was 0, switched on
power: cycled
uart: core clock 399999000 Hz set by firmware (firmware measures 399999000), 115207 baud, flow auto-low
version: Bluetooth 4.1 (HCI 7, LMP 7), manufacturer 15 (Broadcom), subversion 6119, firmware build 0
firmware: /usr/lib/firmware/brcm/BCM4345C0.hcd, 323 records in 5866 ms
version: Bluetooth 5.0 (HCI 9, LMP 9), manufacturer 305, subversion 6119, firmware build 382
up
### bt info  (0.4 s)
version: Bluetooth 5.0 (HCI 9, LMP 9), manufacturer 305, subversion 6119, firmware build 382
address: 43:45:C0:00:1F:AC
name: BCM43455 37.4MHz Raspberry Pi 3+-0190
### bt scan 10  (10.4 s)
scan: LE for 10 s
… 22 lines, among them ShellyPlugSG3-543204689994 rssi -44, ShellyPlus1-441793A8302C rssi -70,
  Qingping Alarm Clock rssi -69, Govee_H70B3_2B93 rssi -85, SBBT-002C rssi -68
scan: 22 devices
### bt inquiry 10  (10.6 s)
inquiry: 10 s
inquiry: 0 devices
### cat /scheme/rpi-firmware/wifi_power
1
```

- (c) not needed: (b) worked with `auto-low`.
- (e) `received bytes lost` lines on the serial console: **0**. No `reset:`
  line (Reset answered on the first try, which `bt up` does not print) and no
  `ignored` event.
- Classic inquiry found 0 devices without an error: nothing near was
  discoverable over BR/EDR (passes by the row's reading).
- So WL_ON was the difference between `bt diag` and `bt up` (steps 10 and 11).
- No panic.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-703-g7ce5389+x9024` (power-save, idle power per switch)

Real Pi 3B+ over netboot. Test 147 ([#126](https://github.com/redox-rpi/redox-rpi/issues/126)), Pi test agent.

**Idle power: 3.23 W on stable, 2.99 W on the power-save image. No single switch
saves 0.1 W or more.** Driver: every write was taken and devsh and the serial
console kept working. But the unused power domains were **already off at
boot**, and writing `on` leaves them off. Shelly plug read only (`shelly-sample.sh`
from `origin/power-save`, 18 readings 10 s apart), nothing over devsh during a
sample. Image `~/redox-rpi-cpu/keep/v0.3.0-dev-703-g7ce5389+x9024/harddrive.img`
(manifest in the image), stable bootloader. Each sample started 3 minutes after
`redox login:` or right after the step's command.

| sample | mean | min | max | vs previous |
|---|---|---|---|---|
| A-stable (`v0.3.0-dev-533-g6c729cd+x9f76`) | 3.23 W | 3.1 | 3.3 | |
| B1-idle | 2.99 W | 2.9 | 3.1 | −0.24 W vs stable |
| B2-blank | 2.91 W | 2.8 | 3.0 | −0.08 (none) |
| B3-hdmi-off | 2.99 W | 2.9 | 3.1 | +0.08 (none) |
| B4-power-led-off | 2.99 W | 2.9 | 3.1 | 0 |
| B5-act-led-off | 3.02 W | 2.9 | 3.1 | +0.03 (none) |
| B6-domains-off | 2.99 W | 2.9 | 3.0 | −0.03 (none) |
| B7-all-powersave | 3.00 W | 2.9 | 3.1 | +0.01 (none) |

(2) before any switch:

```
sd on / uart0 off / uart1 off / usb on / i2c0 off / i2c1 off / i2c2 off / spi off / ccp2tx off
display: on
power_led: on
gpio 29 mode: out, value: 0
governor: mode ondemand, target max, load 100, switches 4, min 600000000, max 1400000000, time_min_ms 356209, time_max_ms 14644
```

(6) each `echo "<name> off"` was taken (no error), and devsh answered after each
(`alive`). `cat power_domains` afterwards shows the same list as in (2): **all six
were already off.** `usb` and `uart0` were not touched.

(8) `cputop 30` (governor back at ondemand):

```
cputop: 30.0 s, 44 threads
cpu0  busy   0 %  (0 ms busy, 30011 ms idle, 107 switches/s)
cpu1  busy   2 %  (741 ms busy, 29269 ms idle, 291 switches/s)
cpu2  busy   0 %  (34 ms busy, 29974 ms idle, 174 switches/s)
cpu3  busy   0 %  (114 ms busy, 29894 ms idle, 186 switches/s)
16  2.4  0:00:35.8  UB #1  dwc2d
 8  0.4  0:00:04.9  US #3  rpi-firmwared
25  0.1  0:00:02.2  UB #1  netstack
```

(9) everything back: each write taken. But **`power_domains` still lists all six
`off`** after `echo "<name> on"` (the firmware answers `[id, 0]`, state 0, to `on`
as well). `display` and `power_led` read `on`, governor `ondemand`. Standard
checks pass (0 of 300 starts failed). The picture on the NanoKVM was not checked
(no person, no screenshot).

`rpi-firmwared:` lines on the serial console:

```
rpi-firmwared: mailbox at 0x3f00b880, revision 0x00a020d3, model Raspberry Pi 3 Model B+
rpi-firmwared: ARM clock 600 -> 1400 MHz
rpi-firmwared: display blank (firmware answered [1])
rpi-firmwared: display unblank (firmware answered [0])
rpi-firmwared: display off (firmware answered [0])
rpi-firmwared: power domain i2c0 off (firmware answered [4, 0])
rpi-firmwared: power domain i2c2 off (firmware answered [6, 0])
rpi-firmwared: power domain uart1 off (firmware answered [2, 0])
rpi-firmwared: power domain ccp2tx off (firmware answered [8, 0])
rpi-firmwared: power domain i2c1 off (firmware answered [5, 0])
rpi-firmwared: power domain spi off (firmware answered [7, 0])
rpi-firmwared: governor powersave
rpi-firmwared: governor ondemand
rpi-firmwared: power domain i2c0 on (firmware answered [4, 0])
rpi-firmwared: power domain i2c2 on (firmware answered [6, 0])
rpi-firmwared: power domain uart1 on (firmware answered [2, 0])
rpi-firmwared: power domain ccp2tx on (firmware answered [8, 0])
rpi-firmwared: power domain i2c1 on (firmware answered [5, 0])
rpi-firmwared: power domain spi on (firmware answered [7, 0])
rpi-firmwared: display on (firmware answered [0])
```

- `power_led` and the GPIO 29 writes log nothing on the serial console.
- GPIO 29 (activity LED) was already `out`/`0` at boot, so B5 changed nothing.
- `display off` answered `[0]` and saved nothing measurable. On the 3B+ HDMI
  off may need more than the blank call (power domain or clock).
- The 0.24 W between stable and B1 is the clearest difference, but stable and
  the power-save image differ in more than the switches (base `spi`/PR #87).
- No panic.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-553-gf4b1432+x12cb` (boot-time-2, 192 MiB network-boot image)

Real Pi 3B+ over netboot. Test 142 ([#47](https://github.com/redox-rpi/redox-rpi/issues/47)), Pi test agent.

**Pass: three of three B boots reach `redox login:` with network, standard checks
pass, nano installs.** Image loaded to kernel start is only **0.25 s shorter**
(8.18 s → 7.93 s). The total memory is **64 MiB larger** (743612 → 809060 KiB).
A: stable as staged. B: `cp ~/redox-rpi-boot/keep/bt2A-redox/* /srv/tftp/redox/`.
Times are seconds after `kreset`, from `boot-timeline.py`
(`origin/boot-time`).

| boot | image loaded over HTTP | mode menu | kernel start (first `kernel::` line) | `redox login:` | loaded → kernel |
|---|---|---|---|---|---|
| A stable (256 MiB) | 13.48 | 15.94 | 21.66 | 31.58 | 8.18 |
| B1 (192 MiB) | 13.49 | 15.80 | 21.41 | 31.35 | 7.92 |
| B2 | 13.44 | 15.74 | 21.35 | 31.33 | 7.91 |
| B3 | 13.48 | 15.83 | 21.40 | 31.34 | 7.92 |

- The download hardly changes: gzip 43112933 bytes (A) and 43037135 bytes (B),
  both 11.3 MiB/s, about 3.64 s. The unused space compresses to almost nothing.
  The saving left is U-Boot unpacking less and the bootloader copying
  `live: 158/158 MiB` instead of `222/222 MiB`. Both copies took 0.2 to 0.3 s on
  the console. Login comes about 0.25 s sooner.
- Most of the time between image loaded and the kernel is the bootloader's
  5 s mode menu (`Continuing with the selected mode in 5 seconds`).
- `df -h /`: A `222.0M` size, 86.5M used, 135.5M free; B `158.0M`, 86.6M used,
  71.4M free (as expected), after nano 91.7M used, 66.3M free.
- `free -m` (prints KiB here): A total 743612, free 596720; B total 809060, free
  671616 (+65448 KiB total, +74896 KiB free).
- `pkg install --yes nano` took 27.4 s and committed 214 files; `nano --version`
  prints `GNU nano, version 9.2`. One `TODO: setsockopt(7, 6, 1, …) - unknown
  option` info line from pkg.
- Network on every B boot: `dwc2d: LAN78xx link up after 1710 ms`. Standard
  checks with `--manifest` pass (0 of 300 starts failed). No panic.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-764-gcf2025e+x9f76` (main cf2025e, start-crash recount)

Real Pi 3B+ over netboot. Test 149 ([#1](https://github.com/redox-rpi/redox-rpi/issues/1)), Pi test agent.

**Pass: 0 of 3000 starts failed** (test 94 on `v0.3.0-dev-339`: 0 of 3000).
Image `~/redox-rpi-t12/keep/t1-v0.3.0-dev-764-gcf2025e+x9f76.img`, manifest
matches, stable bootloader. `scripts/pi-test.py 192.168.178.148 --starts 3000
--manifest … --serial-log ~/redox-rpi/logs/serial.log --crash-log
t149-crashes.jsonl`:

```
PASS version: v0.3.0-dev-764-gcf2025e+x9f76
PASS manifest: build ID and libc.so hash match netboot-v0.3.0-dev-764-gcf2025e+x9f76.toml
PASS ps: 51 processes (piped)
PASS ps-tty: 51 lines straight to the terminal, rc=0
PASS sdcard: write, read back, delete on /scheme/data
PASS starts: 0 of 3000 failed
```

- The crash log stayed empty (no file written: no failed start, so no
  FAR/ELR/ESR to report).
- Serial log during the starts (16:17 to 16:25 UTC): **0** lines with `FATAL`,
  `exception`, `panicked`, `Data abort` or `Instruction abort`.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-773-g2ffa68b+x7f63` (timesync-harden, checked answers and several servers)

Real Pi 3B+ over netboot. Test 152 (follow-up to [#51](https://github.com/redox-rpi/redox-rpi/issues/51)), Pi test agent.

**Pass: every check as expected, and the standard checks pass.** Image
`~/ntp-test/keep/ntp-v0.3.0-dev-773-g2ffa68b+x7f63.img`, manifest matches, stable
bootloader. `fake-ntp.py 192.168.178.120 1800` from `origin/timesync-harden`
(UDP 12301–12309). H = 192.168.178.120.

**Boot** (serial):

```
timesync: clock set to 2026-10-31 16:17:43 UTC from the saved time
timesync: clock set to 2026-10-31 16:27:07 UTC, offset +564.272 s (pool.ntp.org (139.177.202.26) +564.281 s, pool.ntp.org (162.254.225.152) +564.272 s, pool.ntp.org (67.223.208.241) +564.266 s)
timerd: clock set by timesync, 2026-10-31 16:27:08 UTC
```

| step | result |
|---|---|
| (1) `timesync -n` | `offset +0.004 s (clock not set; pool.ntp.org (139.177.202.26) +0.009 s, pool.ntp.org (162.254.225.152) +0.004 s, pool.ntp.org (67.223.208.241) -0.007 s)` |
| (2) `timesync -n H:12301 H:12305 H:12307` | `offset -0.007 s (… 12301 -0.007 s, 12305 +7199.993 s, 12307 -0.007 s)`: the +7200 s answer is outvoted |
| (3a) 12302 | `server not synchronized (leap indicator 3)`, then `no time server answered within a minute (last: …)` |
| (3b) 12303 | `kiss-of-death RATE`, then `no time server answered within a minute (last: …)` |
| (3c) 12304 | `answer does not match the request`, then `no time server answered within a minute (last: …)` |
| (3d) 12301 + 12305 | `two answers 7200 s apart, waiting for a third`, then `no time server answered within a minute (last: …)` |
| (4) `timesync H:12305; date -u` | `clock set to 2026-10-31 18:29:41 UTC, offset +7199.994 s`; `date -u` `18:29:41` |
| (4) `timesync; date -u` | `clock set to 2026-10-31 16:29:42 UTC, offset -7199.999 s` (three pool answers −7200.000, −7199.999, −7199.998 s); `date -u` `16:29:42`, host `16:29:47` (5 s later, after the command) |
| `grep -c rate fake-ntp.log` | **1** |

- The saved time was 9 min 24 s behind at boot (the Pi was off since the last
  save); pool answers fixed it.
- No panic. `fake-ntp.py` stopped afterwards.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-768-g7d71e64+x1258` (sigreturn-default, aarch64 signal return as default patches)

Real Pi 3B+ over netboot. Test 154 ([#127](https://github.com/redox-rpi/redox-rpi/issues/127)), Pi test agent.

**Pass: normal boot to login with no `FATAL` or panic, four of four `sigregs`
runs PASS, and the standard checks pass.** Image
`~/redox-rpi-relibc/keep/netboot-v0.3.0-dev-768-g7d71e64+x1258.img`, manifest
matches, stable bootloader.

```
### sigregs process --x18 -n 500  (13.0 s)
process: sending from child 47 to 46
PASS process: 500 signals, all registers kept
### sigregs process --x18 -n 500  (15.2 s)
process: sending from child 50 to 49
PASS process: 500 signals, all registers kept
### sigregs process --x18 -n 500  (15.2 s)
process: sending from child 53 to 52
PASS process: 500 signals, all registers kept
### sigregs process -n 500  (20.1 s)
process: sending from child 56 to 55
PASS process: 500 signals, all registers kept
```

- `scripts/pi-test.py --manifest … --starts 300 --serial-log …`: version,
  manifest (build ID and libc.so hash), ps, ps-tty, sdcard pass; **0 of 300
  starts failed**.
- Serial console from the boot to the end: 0 lines with `FATAL` or `panicked`.
- `sigregs thread` not run (fails on main too until PR #64).

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-816-g7376c46+x4590` (assistant + devshd-accept, devshd under a memory-heavy run)

Real Pi 3B+ over netboot. Test 153 ([#129](https://github.com/redox-rpi/redox-rpi/issues/129)), Pi test agent.

**Pass: devsh accepted a new connection after each of the three assistant runs.**
The serial console's only `devshd:` line was the start-up `devshd: listening on
port 2323`: no accept errors, no rebind, no session thread error. #126's
`Connection refused` did not come back. Image
`~/redox-rpi-assistant/keep/netboot-v0.3.0-dev-816-g7376c46+x4590.img`, manifest
matches, stable bootloader. Standard checks pass (0 of 300 starts failed).
Model on the card (341455552 bytes). 4 threads (`RAYON_NUM_THREADS=4`);
`prompt.kv` removed before each run, so each run read the system prompt.

| run | total (`date -u`) | prompt read | saved | answers | writing | new devsh `echo alive` |
|---|---|---|---|---|---|---|
| 1 | 16:48:58 → 16:53:14 (256 s) | 380 tokens in 167.2 s | 6.1 s | `It is Saturday, 31 October 2026, 16:52 UTC.`, `The current time is 16:52 UTC.` | 1.21, 0.72 t/s | **alive** at 16:53:14 |
| 2 | 16:53:18 → 16:58:40 (322 s) | 166.1 s | 11.3 s | same, 16:56 UTC | 1.33, **0.15** t/s | **alive** at 16:58:40 |
| 3 | 16:58:50 → 17:03:32 (283 s) | 174.4 s | 8.6 s | same, 17:02 UTC (second: `… 31 October 2026 at 17:02 UTC.`) | **0.16**, 0.87 t/s | **alive** at 17:03:33 |

- `free` from a second devsh during run 2 (16:54:51): 725120 used, 145524 free
  of 870644. That connection needed one retry (`no output for: echo start`),
  then answered. After the runs: 254104 used, 616540 free.
- Writing slowed to 0.15–0.16 t/s in two answers (#126 with 4 threads:
  0.78 t/s). Both came after the chip reached its soft temperature limit
  (below); the second devsh during run 2 may also have cost time.
- End: temperature 61.2 °C, `throttled` `0x00080008` (soft temperature limit
  reached and active).
- No `oom`, no kernel panic, no `RELIBC PANIC`. `prompt.kv` removed; the model
  kept.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-806-gb2dd152` wait-irq `+x98e9` and tx-irq `+x3388` (sent frames without the spin)

Real Pi 3B+ over netboot. Test 155 ([#104](https://github.com/redox-rpi/redox-rpi/issues/104), sent frames without the spin), Pi test agent. (Queued as a second 152 by mistake; renumbered to 155.)

**Pass: the host counted 16777216 bytes in all four runs, and devshd answered
afterwards.** dwc2d's ACTTIME is **not lower with tx-irq**: +1.22 and +1.10 s
per 16 MiB against +1.03 and +1.14 s for wait-irq. Sending is about as fast
(7.1 to 8.3 MB/s). Images from `~/redox-rpi-usb/keep/` with their manifests
(match), stable bootloader, wait-irq first. Standard checks with `--manifest`
pass on both (0 of 300 starts failed). Host side: `nc -l <port> | wc -c`.

| image | run | `tcp-send` | host bytes | dwc2d ACTTIME before → after | SCHTIME before → after |
|---|---|---|---|---|---|
| wait-irq | 1 | `16777216 bytes in 2.31 s, 7.28 MB/s` | 16777216 | 1.74 → 2.77 s (**+1.03**) | 1.88 → 1.91 s (+0.03) |
| wait-irq | 2 | `16777216 bytes in 2.02 s, 8.30 MB/s` | 16777216 | 2.97 → 4.11 s (**+1.14**) | 2.10 → 2.22 s (+0.12) |
| tx-irq | 1 | `16777216 bytes in 2.36 s, 7.12 MB/s` | 16777216 | 1.61 → 2.83 s (**+1.22**) | 1.74 → 1.76 s (+0.02) |
| tx-irq | 2 | `16777216 bytes in 2.11 s, 7.94 MB/s` | 16777216 | 2.90 → 4.00 s (**+1.10**) | 1.82 → 1.85 s (+0.03) |

- ACTTIME as in #144 (first time column of `/scheme/sys/context`); each pair
  also covers about 0.4 s of devsh between the commands.
- #144 (wait-irq `451`): +0.99 and +1.00 s at 8.3 MB/s. So the time dwc2d spends
  per 16 MiB sent is unchanged on all three builds.
- No dwc2d line on the serial console about the transmit interrupt or spin (only
  the enumeration). `free` afterwards: 138928 used (wait-irq), 140308 (tx-irq) of
  739432. No panic.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-795-g187edb8+xff6c` (netstack-arp-ack, stale ARP and reads after a TCP reset)

Real Pi 3B+ over netboot. Test 156 ([#140](https://github.com/redox-rpi/redox-rpi/issues/140), [#141](https://github.com/redox-rpi/redox-rpi/issues/141)), Pi test agent.

**Pass: after a reset, netget still reads the 65535 buffered bytes. Three
downloads of 382 MB complete at 7.67–7.78 MB/s, with no `waiting queue was
full` line.** Image `~/redox-rpi-zrtl/keep/arpack-v0.3.0-dev-795-g187edb8+xff6c.img`
(manifest in the image), stable bootloader. Standard checks pass before and
after (0 of 300 starts failed each time).

(1) Reset test, `rstserver.py 18998 1000000`:

```
### netget http://192.168.178.120:18998/x --slow 5000  (10.3 s)
GET http://192.168.178.120:18998/x
HTTP/1.0 200 OK
read after 65535 bytes, 10.0 s: Transport endpoint is not connected (os error 107)
### netget http://192.168.178.120:18998/x --slow 5000  (10.3 s)
GET http://192.168.178.120:18998/x
HTTP/1.0 200 OK
read after 65535 bytes, 10.0 s: Transport endpoint is not connected (os error 107)
```

Server: `17:39:00 192.168.178.148: header, 437576 body bytes into the socket,
then reset`, and the same at 17:39:11. N = 65535 both times (pass: above 30000).

(2) ARP, `netget http://192.168.178.120:8080/models/Qwen3-0.6B-Q4_0.gguf --log /scheme/debug` three times:

| run | result | 16 MiB steps |
|---|---|---|
| 1 | `382156480 bytes in 49.8 s, 7.67 MB/s` | 7.38–7.97 MB/s |
| 2 | `382156480 bytes in 49.1 s, 7.78 MB/s` | 7.6–7.9 MB/s |
| 3 | `382156480 bytes in 49.6 s, 7.71 MB/s` | 7.4–7.9 MB/s |

- Serial during the three runs: **0 `waiting queue was full`** lines, no
  `Dropped` and no `smolnetd` line. The whole boot had none either. No panic.
- The three runs took about 2.5 minutes together, so they spanned two or three
  60 s ARP refreshes.
- Every `netget:` line from the serial log (`--log /scheme/debug`):

```
netget: GET http://192.168.178.120:8080/models/Qwen3-0.6B-Q4_0.gguf
netget: HTTP/1.0 200 OK
netget:    16 MiB     2.2 s  7.75 MB/s
netget:    32 MiB     4.3 s  7.87 MB/s
netget:    48 MiB     6.6 s  7.45 MB/s
netget:    64 MiB     8.8 s  7.64 MB/s
netget:    80 MiB    11.0 s  7.65 MB/s
netget:    96 MiB    13.1 s  7.75 MB/s
netget:   112 MiB    15.3 s  7.78 MB/s
netget:   128 MiB    17.5 s  7.69 MB/s
netget:   144 MiB    19.6 s  7.92 MB/s
netget:   160 MiB    21.7 s  7.95 MB/s
netget:   176 MiB    23.9 s  7.75 MB/s
netget:   192 MiB    26.0 s  7.92 MB/s
netget:   208 MiB    28.3 s  7.38 MB/s
netget:   224 MiB    30.6 s  7.46 MB/s
netget:   240 MiB    32.8 s  7.60 MB/s
netget:   256 MiB    35.0 s  7.55 MB/s
netget:   272 MiB    37.2 s  7.70 MB/s
netget:   288 MiB    39.4 s  7.71 MB/s
netget:   304 MiB    41.5 s  7.82 MB/s
netget:   320 MiB    43.7 s  7.97 MB/s
netget:   336 MiB    45.9 s  7.58 MB/s
netget:   352 MiB    48.1 s  7.38 MB/s
netget: 382156480 bytes in 49.8 s, 7.67 MB/s
netget: GET http://192.168.178.120:8080/models/Qwen3-0.6B-Q4_0.gguf
netget: HTTP/1.0 200 OK
netget:    16 MiB     2.2 s  7.63 MB/s
netget:    32 MiB     4.4 s  7.64 MB/s
netget:    48 MiB     6.6 s  7.73 MB/s
netget:    64 MiB     8.7 s  7.77 MB/s
netget:    80 MiB    10.9 s  7.85 MB/s
netget:    96 MiB    13.1 s  7.75 MB/s
netget:   112 MiB    15.3 s  7.66 MB/s
netget:   128 MiB    17.5 s  7.67 MB/s
netget:   144 MiB    19.6 s  7.89 MB/s
netget:   160 MiB    21.8 s  7.77 MB/s
netget:   176 MiB    23.9 s  7.80 MB/s
netget:   192 MiB    26.1 s  7.75 MB/s
netget:   208 MiB    28.2 s  7.90 MB/s
netget:   224 MiB    30.4 s  7.84 MB/s
netget:   240 MiB    32.5 s  7.89 MB/s
netget:   256 MiB    34.6 s  7.86 MB/s
netget:   272 MiB    36.8 s  7.80 MB/s
netget:   288 MiB    38.9 s  7.93 MB/s
netget:   304 MiB    41.1 s  7.76 MB/s
netget:   320 MiB    43.2 s  7.86 MB/s
netget:   336 MiB    45.4 s  7.79 MB/s
netget:   352 MiB    47.5 s  7.89 MB/s
netget: 382156480 bytes in 49.1 s, 7.78 MB/s
netget: GET http://192.168.178.120:8080/models/Qwen3-0.6B-Q4_0.gguf
netget: HTTP/1.0 200 OK
netget:    16 MiB     2.2 s  7.74 MB/s
netget:    32 MiB     4.3 s  7.80 MB/s
netget:    48 MiB     6.4 s  8.01 MB/s
netget:    64 MiB     8.6 s  7.75 MB/s
netget:    80 MiB    10.7 s  7.87 MB/s
netget:    96 MiB    12.9 s  7.83 MB/s
netget:   112 MiB    15.2 s  7.35 MB/s
netget:   128 MiB    17.4 s  7.60 MB/s
netget:   144 MiB    19.6 s  7.52 MB/s
netget:   160 MiB    21.8 s  7.81 MB/s
netget:   176 MiB    23.9 s  7.82 MB/s
netget:   192 MiB    26.1 s  7.69 MB/s
netget:   208 MiB    28.3 s  7.76 MB/s
netget:   224 MiB    30.5 s  7.79 MB/s
netget:   240 MiB    32.7 s  7.64 MB/s
netget:   256 MiB    34.8 s  7.89 MB/s
netget:   272 MiB    37.0 s  7.78 MB/s
netget:   288 MiB    39.1 s  7.70 MB/s
netget:   304 MiB    41.4 s  7.63 MB/s
netget:   320 MiB    43.5 s  7.64 MB/s
netget:   336 MiB    45.7 s  7.66 MB/s
netget:   352 MiB    47.9 s  7.67 MB/s
netget: 382156480 bytes in 49.6 s, 7.71 MB/s
```

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-798-gb225933+x1258` (t4-with-137, signals sent to a thread)

Real Pi 3B+ over netboot. Test 157 ([#4](https://github.com/redox-rpi/redox-rpi/issues/4)), Pi test agent.

**Pass: boot to login with no `FATAL` or panic, all five `sigregs` runs PASS
(signals to a thread now arrive, 500 of 500), and the standard checks pass.**
Image `~/redox-rpi-relibc/keep/netboot-v0.3.0-dev-798-gb225933+x1258.img`,
manifest matches, stable bootloader.

```
### sigregs thread --x18 -n 500  (15.3 s)
PASS thread: 500 signals, all registers kept
### sigregs thread --x18 -n 500  (15.2 s)
PASS thread: 500 signals, all registers kept
### sigregs thread --x18 -n 500  (20.1 s)
PASS thread: 500 signals, all registers kept
### sigregs process --x18 -n 500  (15.2 s)
process: sending from child 53 to 52
PASS process: 500 signals, all registers kept
### sigregs sender  (0.2 s)
PASS sender: si_pid 55 from the thread, 56 from the child
```

- `scripts/pi-test.py --manifest … --starts 300 --serial-log …`: version,
  manifest (build ID and libc.so hash), ps, ps-tty, sdcard pass; **0 of 300
  starts failed**.
- Serial console from the boot to the end: 0 lines with `FATAL` or `panicked`.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-799-g68620a9+x1258` (sigsegv-main, signals for CPU exceptions)

Real Pi 3B+ over netboot. Test 158 (PR #143), Pi test agent.

**Pass: 2 × 500 faults each handled and continued; thread, process and sender
pass; standard checks pass; no panic.** Image
`~/redox-rpi-relibc/keep/netboot-v0.3.0-dev-799-g68620a9+x1258.img`, manifest
matches, stable bootloader. The boot reached login.

```
### sigregs segv -n 500  (76.8 s)
PASS segv: 500 faults, each handled and continued
### sigregs segv -n 500  (76.8 s)
PASS segv: 500 faults, each handled and continued
### sigregs thread --x18 -n 500  (15.3 s)
PASS thread: 500 signals, all registers kept
### sigregs process --x18 -n 500  (20.1 s)
process: sending from child 53 to 52
PASS process: 500 signals, all registers kept
### sigregs sender  (0.3 s)
PASS sender: si_pid 55 from the thread, 56 from the child
```

- Kernel register dumps on the serial console: exactly **1000** (one per fault),
  each `FATAL: Not an SVC induced synchronous exception (ty=100100)`,
  `FAR_EL1: 0x10`, `ESR_EL1: 0000000092000007` (data abort at EL0), from
  `synchronous_exception_at_el0`. Expected by the row. No other `FATAL` line, 0
  `panicked`.
- Each `segv -n 500` run takes 77 s, longer than devshd's 60 s guide, because
  the 500 dumps go out over the 115200-baud console. pirun waited (300 s
  timeout), and both runs completed.
- `scripts/pi-test.py --manifest … --starts 300 --serial-log …`: all pass, **0 of
  300 starts failed**.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-800-g1682576+x4590` (assistant-speed, candle's pool spin limit)

Real Pi 3B+ over netboot. Test 159 ([#43](https://github.com/redox-rpi/redox-rpi/issues/43)), Pi test agent.

**A spin limit of 250000 or more removes the pool sleeps. Writing goes from 1.38
to about 2.0 t/s with 3 threads, and 4 threads with the high limit give
2.56 t/s.** The default (25000) sleeps 236 times per token when cool and 48 times
when warm. Image `~/redox-rpi-assistant/keep/netboot-v0.3.0-dev-800-g1682576+x4590.img`,
manifest matches, stable bootloader. Standard checks pass (0 of 300 starts
failed). Model on the card (341455552 bytes). `rinfer
/scheme/data/Qwen3-0.6B-Q4_0e.gguf 'Name three colours.' 32`; every run
answered `Three colours are blue, green, and red.` (10 tokens).

| run | threads | spin limit | prompt (20 tokens) | generated (10 tokens) | per token | temp after | throttled after |
|---|---|---|---|---|---|---|---|
| before | | | | | | 54.8 °C | `0x00000000` |
| 1 | 3 | 25000 (default) | 9.7 s, 2.06 t/s | 7.2 s, **1.38 t/s** | model 723 ms, sampling 1 ms, **pool sleeps 236** | 58.0 °C | `0x00080000` |
| 2 | 3 | 250000 | 7.4 s, 2.69 t/s | 5.0 s, **2.00 t/s** | model 498 ms, sampling 1 ms, pool sleeps 0 | 60.1 °C | `0x00080008` |
| 3 | 3 | 2500000 | 7.6 s, 2.62 t/s | 5.2 s, 1.91 t/s | model 523 ms, sampling 1 ms, pool sleeps 0 | 61.8 °C | `0x00080008` |
| 4 | 3 | 25000000 | 7.9 s, 2.52 t/s | 5.0 s, 2.01 t/s | model 498 ms, sampling 1 ms, pool sleeps 0 | 62.3 °C | `0x00080008` |
| 5 | 4 | 25000000 | 5.8 s, 3.45 t/s | 3.9 s, **2.56 t/s** | model 389 ms, sampling 1 ms, pool sleeps 0 | 63.4 °C | `0x00080008` |
| 6 | 3 | 25000 (again) | 8.2 s, 2.43 t/s | 5.9 s, 1.70 t/s | model 588 ms, sampling 1 ms, **pool sleeps 48** | 60.1 °C | `0x00080008` |

- Each run's first line was `neon: true, threads: N, pool spin limit: …` as in
  the table (run 1 `(default)`).
- Run 6 repeats run 1 warm (soft temperature limit active), and it sleeps less
  (48 vs 236) and writes faster (1.70 vs 1.38 t/s). A slower clock under the soft
  limit seems to make the 25000 spins last longer. The default's number depends
  on heat.
- `loaded in` 20.5 to 31.1 s (539 reads); the longer loads (runs 3 and 4) came
  with the second devsh (run 3) and the hotter chip.
- `free` from a second devsh during run 3 (18:00:37, while loading): 674112 used,
  196532 free of 870644.
- `throttled` showed the soft temperature limit from run 1 on (`0x00080000`,
  then `0x00080008`). No `oom`, no kernel panic, no `RELIBC PANIC`. Model kept.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-823-ga961ba0+xb3d9` (t-pin-redoxfs + t13-relibc-tests, redoxfs pinned)

Real Pi 3B+ over netboot. Test 160 ([#145](https://github.com/redox-rpi/redox-rpi/issues/145)), Pi test agent.

**Pass: the pinned redoxfs boots and runs (0 of 1000 starts failed), the four
Unix socket tests pass, and the relibc suite has no new failure.** Image
`~/redox-rpi-t12/keep/pinfs-v0.3.0-dev-823-ga961ba0+xb3d9.img` (manifest in the
image, none next to it), stable bootloader.

- `scripts/pi-test.py --starts 1000 --serial-log …`: version, manifest (present),
  ps, ps-tty, sdcard pass; **0 of 1000 starts failed**.
- In `/usr/share/relibc-tests/bins_dynamic/sys_socket`:
  - `./unixrecvfrom`: `send lorem`, `recvfrom lorem`, **rc=0**
  - `./unixrecv`: `send ipsum`, `recv ipsum`, **rc=0**
  - `./unixnonblock`: `send ipsum`, `recv ipsum`, **rc=0**
  - `./unixsocketpair`: SOCK_STREAM and SOCK_DGRAM `child: ping`, `parent: pong`, **rc=0**
  - no `connect failed: Invalid argument`; 0 `handle_bind` lines on the serial
    console.
- `ion /usr/share/relibc-tests/run.ion` (64.6 s, `COMMAND_TIMEOUT=60`), compared
  with `relibc-tests-compare.py` and `config/relibc-tests-expected.txt` from
  `t13-relibc-tests` (the suite prints to devsh, so I compared the devsh output):

```
FIXED ./bins_dynamic/netdb/getaddrinfo: on the list, but passed
FIXED ./bins_dynamic/pthread/rwlock_randtest: on the list, but passed
FIXED ./bins_dynamic/sys_socket/getpeername: on the list, but passed
FIXED ./bins_dynamic/sys_socket/getsockopt: on the list, but passed
FIXED ./bins_dynamic/sys_socket/recv: on the list, but passed
FIXED ./bins_dynamic/sys_socket/recvfrom: on the list, but passed
SUMMARY v0.3.0-dev-823-ga961ba0+xb3d9: 319 tests, 16 failed (16 known, 0 flaky, 0 new), 6 fixed
```

  **No `NEW` line**, the same as #133.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-793-g9ba1d403` (A) and `+x3d23` (B) (governor-config, CPU clock setting at boot)

Real Pi 3B+ over netboot. Test 161 ([#23](https://github.com/redox-rpi/redox-rpi/issues/23)), Pi test agent.

**Pass: the default image stays on ondemand with no error. The test image boots
in `performance` from `/etc/rpi-firmware/governor`, and writing the file plus
`governor.ion` switches to fixed 600 MHz and back to ondemand. Standard checks
pass on both.** One point is not as the row expected: `switches` did not grow
during A's standard checks, because the clock was already at max when they
started (details below). Images from `~/redox-rpi-cpu/keep/` (manifest in the
image), stable bootloader.

**(A) default image** (no file in `/etc`). Serial: no `governor.ion … failed`
line (the only `failed` lines are the usual U-Boot `mmc 2`, randd and redoxfs
probe lines).

| read | mode | target | load | switches | time_min_ms | time_max_ms |
|---|---|---|---|---|---|---|
| 3 min after login | ondemand | max | 81 | 6 | 187810 | 16021 |
| 60 s later | ondemand | max | 100 | 12 | 249966 (**+62156**) | 23560 |
| after the standard checks | ondemand | max | 64 | 12 (**+0**) | 249966 | 76138 (+52578) |

- The 60 s wait put 62.2 s at the minimum clock (pass: at least 50000).
- During the standard checks the clock stayed at 1400 MHz the whole time
  (`time_max_ms` +52.6 s, `time_min_ms` +0). The second read had already sent
  it to max (`load 100`), so it went up before the checks rather than during
  them. Standard checks pass (0 of 300 starts failed).

**(B) test image** (`/etc/rpi-firmware/governor` = `performance`):

- Serial, after `init: switchroot to /usr /etc`: `rpi-firmwared: governor
  performance`.
- `arm_clock` 3 min after login and 60 s later: `1400000000` both.
  `governor`: `mode performance`, `switches 0`, `time_min_ms 0`.
- Shelly (read only): `B-performance: mean 3.26 W, min 3.2, max 3.4, n 18`
  (test 93: 3.20 W performance, 3.05 W ondemand).
- `echo 600000000 > /etc/rpi-firmware/governor; ion /usr/lib/rpi-firmware/governor.ion; cat …`:
  `600000000`, `mode fixed`, `target -`. Serial `rpi-firmwared: ARM clock 600
  MHz, governor off`. **The command also printed `cat: /etc/rpi-firmware/governor:
  Invalid argument` once**, before the values; the clock was set anyway.
- `echo ondemand > …; ion …governor.ion; cat …`: `mode ondemand`, `switches 1`.
  Serial `rpi-firmwared: governor ondemand`.
- Standard checks pass (0 of 300 starts failed). No panic in A or B.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-801-gea8b5c2` without (`+x8a5e`) and with `icache-sync` (`+x9080`) (instruction cache stress test)

Real Pi 3B+ over netboot. Test 162 ([#3](https://github.com/redox-rpi/redox-rpi/issues/3)), Pi test agent.

**Pass, with a clear result. Without `icache-sync` the Cortex-A53 runs stale
code: 31–33 % of `new-page` rounds and 53 % of `same-page` rounds. With
`icache-sync` it is 0 of 10000 in every test, on both boots.** 0 of 1000 starts
failed on both images, and the serial log shows no exception. Images
`~/t18-work/image-t3/without.img` (md5 `32ee2042…`) and `with.img` (md5
`1437c1b6…`), staged with `~/t18-work/t3-stage/scripts/netboot.sh` (bootloader md5
`4fccdb94` each time), order A, B, A, B, each a fresh boot over a devsh reset.

`icachetest 10000`:

| test | A1 (without) | B1 (with) | A2 (without) | B2 (with) |
|---|---|---|---|---|
| `new-page` | **3146** of 10000 stale | 0 | **3244** | 0 |
| `same-page` | **5310** | 0 | **5349** | 0 |
| `file-map` | 0 | 0 | 0 | 0 |
| `user-flush` | 0 | 0 | 0 | 0 |
| result | `FAIL` (68.2 s) | **`PASS`** (67.8 s) | `FAIL` (62.7 s) | **`PASS`** (63.3 s) |

- `pi-test.py --starts 1000` on the first boot of each: **0 of 1000 failed** for
  A1 and B1. All standard checks pass.
- `time ion -c "for i in 1..=500; ls /usr > /dev/null; end"`, three times:

| boot | run 1 | run 2 | run 3 |
|---|---|---|---|
| A1 | 118.87 s | 118.99 s | 119.06 s |
| B1 | 116.77 s | 116.75 s | 116.86 s |
| A2 | 116.19 s | 116.79 s | 116.83 s |
| B2 | 114.34 s | 114.44 s | 114.29 s |

  `icache-sync` costs nothing measurable; B was even 1–2 s faster. These loops
  take longer than test 127's 77.7 vs 78.7 s, because this image is a newer main
  with more in `/usr`; A and B are like for like.
- Serial log of each boot: 0 lines with `FATAL`, `exception` or `panicked`. The
  stale rounds in A did not crash anything, because `icachetest` checks the
  result itself.
- So the gap is real on the Pi (`file-map` and `user-flush` are already safe),
  and `icache-sync` closes it.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-770-g0b9da6d+x9f76` (prebuilt-default, nine prebuilt programs in the image)

Real Pi 3B+ over netboot. Test 165 ([#40](https://github.com/redox-rpi/redox-rpi/issues/40)), Pi test agent.

**Pass: 450 of 450 starts succeed, the bash loop prints `LOOPa50` 5 of 5 times,
and the standard checks pass.** But `/etc/ssh` is not in this image, so the
`find` loop counted 50 runs of `find` on a missing path (details below). Image
`~/redox-rpi-relibc/keep/netboot-v0.3.0-dev-770-g0b9da6d+x9f76.img`, manifest
matches, stable bootloader. The boot reached login with no `FATAL` or panic
line.

| program, 50 runs in an Ion loop | succeeded |
|---|---|
| `bash -c true` | 50 |
| `nano --version` | 50 |
| `vim --version` | 50 |
| `find /etc/ssh` | 50 (see below) |
| `gawk --version` | 50 |
| `tmux -V` | 50 |
| `rg --version` | 50 |
| `curl --version` | 50 |
| `wget --version` | 50 |

- `bash -c 'for j in $(seq 1 50); do true; /usr/bin/true; x=$(echo a); done; echo LOOP$x$j'`
  five times: `LOOPa50` each time.
- `df -h`: `/` 222.0M size, **162.3M used, 59.7M free** (73 %); QEMU had 162 MiB
  used.
- `find`: `/etc/ssh` does not exist here (`ls: cannot access '/etc/ssh'`; this
  image has no SSH experiment). Each run printed `Error: /etc/ssh: description()
  is deprecated; use Display` and still exited 0. `find /nonexistent` does the
  same, rc=0. GNU find exits 1 and says `No such file or directory`. On a real
  directory `find /etc -maxdepth 1` lists the entries (`/etc`,
  `/etc/login_schemes.toml`, `/etc/hostname`, `/etc/net`, `/etc/os-release` …),
  rc=0. `find --version` is `Unrecognized flag`.
- `scripts/pi-test.py --manifest … --starts 300 --serial-log …`: all pass, **0 of
  300 starts failed**. Serial: 0 `FATAL` or `panicked` lines.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-772-g8dc59fc+x0e02` (main 8dc59fc, prebuilt programs installed with `pkg`)

Real Pi 3B+ over netboot. Test 163 ([#40](https://github.com/redox-rpi/redox-rpi/issues/40)), Pi test agent.

**Pass: the 26 programs of step 2, git, python312 and sqlite3 all print `T40
PASS`, the standard checks pass, and no exception or panic.** Much faster than
QEMU: the 26-package install took 3 minutes. Image
`~/redox-rpi-t40/keep/t40A-harddrive.img` (md5 `d5e0fd76`), staged with
`~/redox-rpi-boot/scripts/netboot.sh` (bootloader md5 `64685a6f`).

**Boot 1**

- (1) Standard checks `--starts 300`: all pass, 0 of 300 starts failed.
- (2) `pkg install --yes bash dash … rldd`: 19:47:10 to 19:50:09 UTC (**3 min**),
  ends `Download complete.`, `Commit complete.`
- (3) `t40.sh`:

```
T40 PASS bash
T40 PASS dash
T40 PASS nano
T40 FAIL vim (127): /tmp/t40.sh: line 12: vim: command not found
T40 PASS kibi
T40 PASS smith
T40 PASS ripgrep
T40 PASS gawk
T40 PASS sed
T40 PASS findutils
T40 PASS diffutils
T40 PASS patch
T40 PASS file
T40 FAIL python312 (127): /tmp/t40.sh: line 12: python3.12: command not found
T40 FAIL rustpython (127): /tmp/t40.sh: line 12: rustpython: command not found
T40 PASS sqlite3
T40 FAIL git (127): bash: line 1: git: command not found
T40 PASS curl
T40 PASS wget
T40 PASS tmux
T40 FAIL xz (1): bash: line 1: xz: command not found bash: line 1: xz: command not found
T40 PASS zstd
T40 PASS bzip2
T40 PASS lz4
T40 PASS gnu-make
T40 FAIL rsync (127): bash: line 1: rsync: command not found
T40 PASS bottom
T40 PASS pls
T40 PASS miniserve
T40 PASS nasm
T40 PASS patchelf
T40 PASS rldd
T40 DONE
```

  The six expected failures (`vim`, `python312`, `rustpython`, `git`, `xz`,
  `rsync`, not installed yet) end in `command not found`. The other 26 pass.
  `curl` prints four `TODO: setsockopt(… ) - unknown option` info lines.
- (4) `pkg install --yes git`: 19:50:26 to 19:51:09 (**43 s**, install size
  24.0 MB), `Commit complete.`; `T40 PASS git`.
- (5) `df -h /`: 222.0M size, 205.9M used, **16.1M free** (92 %).

**Boot 2** (fresh boot of the same image)

- (6) `pkg install --yes bash curl python312`: 19:52:01 to 20:00:49 (**8 min
  48 s**), `Commit complete.` (one `usr/bin/reset … (already exists)` line from
  ncursesw).
- (7) `T40 PASS python312`, `T40 PASS sqlite3`.
- (8) `df -h /`: 222.0M, 210.3M used, **11.7M free** (94 %).
- Serial log over both boots: 0 lines with `FATAL`, `exception` or `panicked`.
- The root filesystem is nearly full after these sets (11.7M to 16.1M free).
  Bigger sets need a larger image or a disk.

Stable image restored afterwards.

## 2026-10-31: stable `v0.3.0-dev-533-g6c729cd+x9f76` (does the network stay up when idle)

Real Pi 3B+ over netboot. Test 164 ([#92](https://github.com/redox-rpi/redox-rpi/issues/92)), Pi test agent.

**Pass: the network stayed up for 62 minutes idle.** devsh answered all five
readings (0, 15, 30, 47 and 62 min), the address stayed, and 180 of 180 TCP
probes in the second half were answered. The serial console had no `netstack`,
`dwc2d` or `Dropped` line. Two notes: the Pi does not answer ICMP echo from
the build host, and netstack's memory grew by 1.1 MB in the hour. Stable as
staged, reset over devsh; standard checks pass (0 of 300 starts failed).

- **ICMP ping does not work here:** the build host has no `ping`, and
  `ping.py` (ICMP datagram socket) got **0 of 180** answers from the Pi
  (20:02:55 to 20:32:45). It also got 0 of 3 later while devsh answered; the
  router 192.168.178.1 answers 2 of 2. So the Pi (or netstack) does not answer
  echo requests from this host. I took that half as not measurable.
- **Instead, minutes 32 to 62:** a TCP connect to closed port 9 every 10 s;
  each reset counts as answered. Result: **180 of 180 answered**, 2.1 to 87.0 ms.
- Readings over devsh (`uptime`; netstack and dwc2d ACTTIME and memory;
  `eth0` address; `free` used):

| uptime | netstack ACTTIME / mem | dwc2d ACTTIME / mem | eth0 | used |
|---|---|---|---|---|
| 1m 26s | 0.19 s / 3.59 MB | 1.52 s / 1.70 MB | 192.168.178.148/24 | 137600 |
| 16m 29s | 0.70 s / 3.72 MB | 21.57 s / 1.70 MB | same | 137980 |
| 31m 26s | 1.22 s / 3.84 MB | 41.50 s / 1.70 MB | same | 138368 |
| 47m 36s | 1.78 s / 3.97 MB | 62.95 s / 1.70 MB | same | 138748 |
| 1h 2m 26s | 2.29 s / **4.72 MB** | 82.62 s / 1.70 MB | same | 139984 |

- netstack memory grows about 0.13 MB per 15 minutes, then +0.75 MB in the last
  15 (the 180 probe connections). dwc2d stays flat at 1.70 MB but uses 1.33 s of
  CPU per idle minute (about 2 %). `free` used grows 2.4 MB in the hour.
- Serial: no `netstack`, `dwc2d` or `smolnetd` error or warning, no `Dropped`,
  no panic.

Stable image stays staged.

## 2026-10-31: `v0.3.0-dev-848-gdc1da3e+xdd51` (t14-ring-call + t13-relibc-tests, rt-call-flags)

Real Pi 3B+ over netboot. Test 166 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14)), Pi test agent.

**Partly: 0 of 1000 starts failed and `redox_ring_ramfs` passes (rc=0, no `Bad
file number`). But the relibc suite hung the whole Pi at
`pthread/rwlock_randtest`**, after 98 tests: no answer on devsh, TCP or serial.
The Pi needed a power cycle. Image
`~/redox-rpi-t12/keep/rcf-v0.3.0-dev-848-gdc1da3e+xdd51.img` (manifest in the
image), stable bootloader.

- `scripts/pi-test.py --starts 1000 --serial-log …`: version, manifest
  (present), ps, ps-tty, sdcard pass; **0 of 1000 starts failed**.
- `./bins_dynamic/redox_ring_ramfs`: `Write Benchmark: 32000 ops in 0.622 seconds
  (200.82 MB/s)`, `Read Benchmark: 32000 ops in 1.897 seconds (65.90 MB/s)`,
  `Tests finished successfully!`, **rc=0**.
- `ion /usr/share/relibc-tests/run.ion` (`COMMAND_TIMEOUT=60`): the last test
  started was `./bins_dynamic/pthread/rwlock_randtest` (`# waiting 1011ms`). Then
  no more output; devsh gave up after 305 s of silence. The serial log stopped
  at 21:08:47. Afterwards devsh `No route to host`, TCP probe 0 of 2. In #160
  (same suite, without `rt-call-flags`) `rwlock_randtest` passed.
- `relibc-tests-compare.py`: `the suite did not finish (98 tests started, no
  'RELIBC-TESTS rc=' line)`, so no FIXED/NEW verdict.
- Serial during the suite: kernel dumps for the signal tests that are on the
  known list (`sigprocmask-3`, `sigpause-revert`, `sigsegv`, `sigset-5`,
  `stackoverflow`; `UNHANDLED EXCEPTION`), and `get_connected_peer … Bad file
  number` twice from the socket tests. No kernel panic line before the hang; the
  hang left no message.
- Stable staged, Pi power-cycled with the Shelly plug at 21:14 UTC; stable booted
  to login and devsh answers.

Next: run the suite again on this image, or `rwlock_randtest` alone a few
times, to tell a hang from `rt-call-flags` from a rare one.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-838-g64fa695+x8116` (t151-test, long write to the card without network load)

Real Pi 3B+ over netboot. Test 167 ([#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.

**Pass: all 25 steps of 16 MiB finished, 400 MiB in 64 KiB writes in 182 s of
dd time (1.1 to 3.6 MB/s). devsh answered after each step, and the serial
console showed no error.** So the card alone takes a long write; test 141's
hang needs the network as well. Image
`~/redox-rpi-sd/keep/t151-v0.3.0-dev-838-g64fa695+x8116.img` (md5 `15e52989`),
staged with `~/redox-rpi-boot/scripts/netboot.sh`.

- (1) `df -h /scheme/data`: 960.0M size, 393.7M used, 566.3M free. No
  `netget.bin` (removed after #141, see #143); `rm` said `No such file`.
- (2) `dd if=/scheme/rand of=/tmp/r16 bs=65536 count=256`: 16 MiB in 0.76 s.
  `free` used 162280.
- (3) the 25 steps (`dd … seek=<i×256> conv=notrunc`, then `free`):

| step | start (UTC) | dd time | rate | `free` used |
|---|---|---|---|---|
| 0 | 21:16:38 | 14.9571 s | 1.1 MB/s | 183964 |
| 1 | 21:16:55 | 7.82 s | 2.1 MB/s | 191280 |
| 2 | 21:17:10 | 5.33287 s | 3.1 MB/s | 193724 |
| 3 | 21:17:22 | 6.40854 s | 2.6 MB/s | 194708 |
| 4 | 21:17:40 | 6.21403 s | 2.7 MB/s | 195280 |
| 5 | 21:17:58 | 10.9027 s | 1.5 MB/s | 195696 |
| 6 | 21:18:11 | 5.65799 s | 3.0 MB/s | 196084 |
| 7 | 21:18:24 | 5.45208 s | 3.1 MB/s | 196468 |
| 8 | 21:18:41 | 10.9754 s | 1.5 MB/s | 196848 |
| 9 | 21:18:54 | 5.22359 s | 3.2 MB/s | 197236 |
| 10 | 21:19:01 | 6.25465 s | 2.7 MB/s | 197608 |
| 11 | 21:19:09 | 12.9977 s | 1.3 MB/s | 197976 |
| 12 | 21:19:24 | 6.26754 s | 2.7 MB/s | 198380 |
| 13 | 21:19:32 | 5.2923 s | 3.2 MB/s | 198748 |
| 14 | 21:19:49 | 5.97413 s | 2.8 MB/s | 199132 |
| 15 | 21:20:07 | 6.8708 s | 2.4 MB/s | 199500 |
| 16 | 21:20:21 | 6.47109 s | 2.6 MB/s | 199896 |
| 17 | 21:20:34 | 5.08902 s | 3.3 MB/s | 200268 |
| 18 | 21:20:46 | 4.71222 s | 3.6 MB/s | 200664 |
| 19 | 21:20:53 | 12.5595 s | 1.3 MB/s | 201028 |
| 20 | 21:21:07 | 4.8601 s | 3.5 MB/s | 201412 |
| 21 | 21:21:24 | 6.60228 s | 2.5 MB/s | 201784 |
| 22 | 21:21:32 | 4.8109 s | 3.5 MB/s | 202164 |
| 23 | 21:21:49 | 8.61827 s | 1.9 MB/s | 202540 |
| 24 | 21:22:04 | 5.31831 s | 3.2 MB/s | 202920 |

- Every step `rc=0`, devsh answered at once. Writes alternate between about
  3 MB/s and slower steps (1.1 to 1.9 MB/s, steps 0, 5, 8, 11, 19, 23): the
  card's own pauses, no trend. Memory: +21.7 MB in step 0 (first write to the
  file), then about +0.38 MB per step (`used` 183964 → 202920).
- (4) `ls -l /scheme/data/w151.bin`: **419430400** bytes. `df -h`: 793.8M used,
  166.2M free. `rm` worked, so the partition is back to the model only.
- Serial: only the usual boot lines (`bcm2835-sdhostd: SDHC/SDXC card, 15193 MiB,
  4-bit bus, high speed, up to 50 MHz`); no `sdhost` or `redoxfs` error, no panic.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-838-g64fa695+x8116` (t151-test, the test 141 download into the card again)

Real Pi 3B+ over netboot. Test 168 ([#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.

**The hang came back, and it is not a frozen machine: the Pi's USB hub with the
network chip dropped off.** The last serial lines are `dwc2d: hub 1 port 1:
device at address 2 removed` and `dwc2d: keyboard removed`, at 160 MiB. The
system kept running: login over the serial console worked, `free` normal. But
the network was gone, and reading netstack's address list hung. The plug read
**2.0 W** (idle with network about 3.0 W). Image
`~/redox-rpi-sd/keep/t151-v0.3.0-dev-838-g64fa695+x8116.img` (md5 `15e52989`),
staged with `~/redox-rpi-boot/scripts/netboot.sh`.

- (1) `df -h /scheme/data`: 960.0M, 393.7M used, 566.3M free; no `netget.bin`
  or `w151.bin`.
- (2) `netget http://192.168.178.120:8080/models/Qwen3-0.6B-Q4_0.gguf /scheme/data/netget.bin --log /scheme/debug`,
  serial lines:

```
netget: HTTP/1.0 200 OK
netget:    16 MiB    15.0 s  1.12 MB/s
netget:    32 MiB    22.1 s  2.35 MB/s
netget:    48 MiB    30.6 s  2.00 MB/s
netget:    64 MiB    40.9 s  1.63 MB/s
netget:    80 MiB    55.7 s  1.13 MB/s
netget:    96 MiB    69.4 s  1.24 MB/s
netget:   112 MiB   100.2 s  0.54 MB/s
netget:   128 MiB   106.9 s  2.53 MB/s
netget:   144 MiB   122.0 s  1.11 MB/s
netget:   160 MiB   142.6 s  0.82 MB/s
dwc2d: hub 1 port 1: device at address 2 removed
dwc2d: keyboard removed
```

  (The serial log stopped at 21:26:36.)
- (3) second devsh every 30 s:

| time | `free` used | `netget.bin` | dwc2d `rx_frames` | rx/tx errors |
|---|---|---|---|---|
| 21:24:29 | 186572 | 49887006 | 29338 | 0 / 0 |
| 21:25:01 | 188236 | 88552524 | 64001 | 0 / 0 |
| 21:25:39 | 188780 | 141738264 | 99397 | 0 / 0 |
| 21:26:21 | 189220 | 174441981 | 122256 | 0 / 0 |
| 21:26:58 to 21:32:02 | no answer (10 tries) | | | |

- **Over the serial console afterwards** (logged in as root at about 21:33):
  - `uptime` 9m 58s and `free` 184824 used, 555000 free: the system ran on.
  - `ls -l /scheme/data/netget.bin`: 182400465 bytes.
  - `/tmp/dwc2d-netstats` frozen at `uptime_ms 167953`: `rx_frames 128439`,
    `rx_errors 0`, `tx_frames 59314`, `tx_errors 0`, `irqs 110830`.
  - `ps`: netget (PID 59) still there, blocked. **`bcm2835-sdhostd` ACTTIME
    2:16.26 in 10 minutes** (the card driver busy most of the time). `redoxfs`
    (31) SCHTIME 1:27.10. dwc2d 11.0 s, state `US`. netstack 10.7 s.
  - `cat /scheme/netcfg/ifaces/eth0/addr/list` never returned: netstack is
    stuck. `throttled` was not read (it was typed behind the stuck command).
- Power-cycled with the Shelly plug at 21:35 UTC (same image). After the reboot
  `ls -l /scheme/data/netget.bin`: **182400465** bytes (174 MiB) kept, then
  removed.
- No panic line. #167 (the same writes without network) passed. So the card
  writes alone are fine, but together with the network the USB hub drops. The
  2.0 W reading and the hub removal point at power to the USB hub/LAN chip, or
  at dwc2d losing the hub, rather than at redoxfs.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-845-g7590e1c+x8a5e` (icache-default, icache-sync as default kernel patches)

Real Pi 3B+ over netboot. Test 169 ([#3](https://github.com/redox-rpi/redox-rpi/issues/3)), Pi test agent.

**Pass: `icachetest 10000` PASS with 0 stale rounds in all four tests, standard
checks pass, 0 of 1000 starts failed, and no exception or panic.** Image
`~/t18-work/image-icd/harddrive.img` (md5 `511ef4ab`), staged with
`~/t18-work/t3-stage/scripts/netboot.sh` (bootloader md5 `4fccdb94`, as in
tests 131 and 162).

```
### icachetest 10000  (68.0 s)
new-page: 0 of 10000 rounds ran stale code
same-page: 0 of 10000 rounds ran stale code
file-map: 0 of 10000 rounds ran stale code
user-flush: 0 of 10000 rounds ran stale code
PASS
```

(test 162 without `icache-sync`: up to 3244 `new-page` and 5349 `same-page`.)

- `scripts/pi-test.py --starts 1000`: version, manifest (present), ps, ps-tty,
  sdcard pass; **0 of 1000 starts failed**.
- `time ion -c "for i in 1..=500; ls /usr > /dev/null; end"`: **116.99 s,
  116.87 s, 116.93 s** (test 162 with `icache-sync`: 116.8 s; without: 116.2 to
  119.1 s).
- Serial log from the boot to the end: 0 `FATAL`, `exception` or `panicked`.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-848-gdc1da3e` without (`+xb3d9`) and with `rt-call-flags` (`+xdd51`) (`rwlock_randtest` hang from 166)

Real Pi 3B+ over netboot. Test 170 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14)), Pi test agent.

**`rt-call-flags` does not cause the hang. The image without it (A) froze the
whole Pi twice in 9 runs. The image with it (B) passed 20 of 20.** Each run was
its own devsh command with a 60 s client timeout. Images
`~/redox-rpi-t12/keep/rcfbase-dc1da3e.img` (A) and
`rcf-v0.3.0-dev-848-gdc1da3e+xdd51.img` (B, the image of 166), stable
bootloader. The banners confirmed each boot (`+xb3d9` with `experiments:
relibc-tests self-exit-status timesync`; `+xdd51` with `rt-call-flags`).

| image | runs | passed | timed out | after the timeout |
|---|---|---|---|---|
| A1 (without) | 1 | 0 | **1** (run 1) | devsh dead, 0 serial bytes since the run started, power cycle 21:56 |
| B1 (with) | 20 | **20** | 0 | |
| A2 (without) | 9 | 8 | **1** (run 9) | devsh dead, 0 serial bytes, power cycle 22:09 |

- A passing run takes **1.3–1.5 s** (`real 1.31` to `1.48 s`), not the 11–17 s
  the row expected; the same for A and B.
- In both hangs the Pi stopped completely: no devsh answer within 10 s, no new
  serial output. The serial log around them holds nothing (1 byte). It looks
  like test 166: a machine-wide stop, not a slow run.
- With 2 hangs in 10 A runs and 0 in 20 B runs, `rt-call-flags` is not the cause
  (if anything B looks better, but 20 runs can't show that). Neither image has the
  futex fix `futex-wait-lock` (#98), the likely next suspect.
- Stable staged and the Pi power-cycled at the end; stable runs
  (`BUILD_ID="v0.3.0-dev-533-g6c729cd+x9f76"`).

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-838-g64fa695+x8116` (t151-test, the 168 download again with `throttled`)

Real Pi 3B+ over netboot. Test 171 ([#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.

**The download completed this time (`382156480 bytes in 385.2 s, 0.99 MB/s`),
although the USB hub dropped 8 times and came back every time. `throttled` read
`0x00000000` before, during and after: no undervoltage (bit 16 never set).** Same
image as #167/#168 (md5 `15e52989`), staged with `~/redox-rpi-boot/scripts/netboot.sh`.

- Before: `throttled` `0x00000000`; `/scheme/data` 393.7M used, no old files.
- Every 30 s from a second devsh (11 readings, 22:12:05 to 22:18:17): `throttled`
  **`0x00000000` every time**; devsh answered every time. `free` used 185744 →
  190932, 197688 at the end of the download, 183076 after. `netget.bin`:
   44819700,  81799239,  137663334,  170732067,  203508243,  237738846,  258632514,  278771868,  319671519,  364532904,  382156480
  bytes.
- Each drop on the serial console was `dwc2d: hub 1 port 1: device at address 2
  removed`, `keyboard removed`, then `hub 1 port 1: device connected`, the whole
  tree enumerated again, `LAN78xx no link yet`, `network on the adapter at address
  3`. So dwc2d brought the hub and the network back each time, and TCP carried on.
  In #168, the same image did not bring it back after its one drop.
- netget and drop lines in order:

```
netget: GET http://192.168.178.120:8080/models/Qwen3-0.6B-Q4_0.gguf
netget: HTTP/1.0 200 OK
dwc2d: hub 1 port 1: device at address 2 removed
netget:    16 MiB    18.5 s  0.91 MB/s
netget:    32 MiB    26.0 s  2.24 MB/s
dwc2d: hub 1 port 1: device at address 2 removed
netget:    48 MiB    46.4 s  0.82 MB/s
netget:    64 MiB    54.3 s  2.14 MB/s
dwc2d: hub 1 port 1: device at address 2 removed
netget:    80 MiB    77.4 s  0.73 MB/s
netget:    96 MiB    85.2 s  2.17 MB/s
netget:   112 MiB    94.9 s  1.73 MB/s
netget:   128 MiB   106.8 s  1.41 MB/s
dwc2d: hub 1 port 1: device at address 2 removed
netget:   144 MiB   133.1 s  0.64 MB/s
netget:   160 MiB   144.9 s  1.43 MB/s
netget:   176 MiB   164.7 s  0.85 MB/s
netget:   192 MiB   182.2 s  0.96 MB/s
netget:   208 MiB   202.1 s  0.84 MB/s
netget:   224 MiB   215.7 s  1.24 MB/s
netget:   240 MiB   225.5 s  1.72 MB/s
dwc2d: hub 1 port 1: device at address 2 removed
dwc2d: hub 1 port 1: device at address 2 removed
netget:   256 MiB   281.3 s  0.30 MB/s
netget:   272 MiB   288.3 s  2.42 MB/s
netget:   288 MiB   301.9 s  1.24 MB/s
netget:   304 MiB   318.0 s  1.05 MB/s
netget:   320 MiB   326.3 s  2.02 MB/s
dwc2d: hub 1 port 1: device at address 2 removed
netget:   336 MiB   344.8 s  0.91 MB/s
dwc2d: hub 1 port 1: device at address 2 removed
netget:   352 MiB   371.6 s  0.63 MB/s
netget: 382156480 bytes in 385.2 s, 0.99 MB/s
```

- After the download: `ls -l` 382156480 bytes; `throttled` `0x00000000`;
  `/tmp/dwc2d-netstats` `uptime_ms 430142`, `rx_frames 271000`, `rx_errors 0`,
  `tx_frames 121678`, `tx_errors 0`, `irqs 227065`. `netget.bin` removed.
- No `Dropped`, `smolnetd` or `sdhost` error line, no panic. The plug read 4.2 to
  4.3 W during the download.
- The drops come in the slow 16 MiB steps (0.30 to 0.91 MB/s). The first came
  right after `HTTP/1.0 200 OK`, before much was written. With no undervoltage
  flag, a power dip at the hub that the firmware does not flag can't be ruled out,
  but dwc2d losing the hub under card load is now the more likely cause.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-857-g7ed9070+xa901` (dwc2d hub-drop fix, the 171 download again)

Real Pi 3B+ over netboot. Test 172 (#151 follow-up, USB session), Pi test agent.

**Pass: the download completed (`382156480 bytes in 429.6 s, 0.89 MB/s`) with
no hub drop at all (#171 on the old image: 8 drops). dwc2d counted 0 USB errors
and 0 recoveries, and `throttled` stayed `0x00000000`.** Image
`~/redox-rpi-usb/keep/hubdrop-v0.3.0-dev-857-g7ed9070+xa901.img` (manifest next
to it; `experiments: dwc2d-netstats netget timesync usb-hub-drop`), staged with
`~/redox-rpi-boot/scripts/netboot.sh`.

- Serial: 0 `device at address 2 removed` lines; the only `network on the
  adapter` is the boot's (`LAN78xx link up after 1737 ms`). No `Dropped`,
  `smolnetd` or panic line.
- Second devsh every 30 s, 12 readings from 22:20:40 to 22:27:47 (`throttled`,
  file size, then rc):

| time | throttled | `netget.bin` | rc |
|---|---|---|---|
| 22:20:40 | 0x00000000 | 55885428 | rc=0 |
| 22:21:19 | 0x00000000 | 101310174 | rc=0 |
| 22:21:57 | 0x00000000 | 136131708 | rc=0 |
| 22:22:40 | 0x00000000 | 164645694 | rc=0 |
| 22:23:14 | 0x00000000 | 201046317 | rc=124 |
| 22:24:09 | 0x00000000 | 221145252 | rc=0 |
| 22:24:43 | 0x00000000 | 259483101 | rc=0 |
| 22:25:16 | 0x00000000 | 296764140 | rc=0 |
| 22:25:55 | 0x00000000 | 326639163 | rc=0 |
| 22:26:34 | 0x00000000 | 351882966 | rc=0 |
| 22:27:13 | 0x00000000 | 375474162 | rc=0 |
| 22:27:47 | 0x00000000 | - | rc=124 |

  The two `rc=124` readings were slow, not lost: at 22:23:14 `throttled` took
  7.0 s and `free` 10.2 s under the download, then the 25 s limit ran out. At
  22:27:47 the first connect needed a retry. devsh answered the next reading
  both times.
- Near the end (22:27:13): `/tmp/dwc2d-usbstats` `xact_errors 0`, `babbles 0`,
  `stalls 0`, `toggle_errors 0`, `ahb_errors 0`, `timeouts 0`, `port_events 0`,
  `root_events 1`, **`recoveries 0`**, `hprt0 0x1405`, `gintsts 0x54000009`;
  `/tmp/dwc2d-step` `447920 idle`; netstats `rx_frames 266157`, `rx_errors 0`,
  `tx_frames 107807`, `tx_errors 0`.
- netget lines from the serial console:

```
netget: GET http://192.168.178.120:8080/models/Qwen3-0.6B-Q4_0.gguf
netget: HTTP/1.0 200 OK
netget:    16 MiB    12.4 s  1.36 MB/s
netget:    32 MiB    19.8 s  2.26 MB/s
netget:    48 MiB    30.1 s  1.64 MB/s
netget:    64 MiB    49.1 s  0.88 MB/s
netget:    80 MiB    58.0 s  1.90 MB/s
netget:    96 MiB    72.0 s  1.20 MB/s
netget:   112 MiB    85.5 s  1.25 MB/s
netget:   128 MiB   115.8 s  0.56 MB/s
netget:   144 MiB   131.0 s  1.11 MB/s
netget:   160 MiB   153.9 s  0.73 MB/s
netget:   176 MiB   171.5 s  0.95 MB/s
netget:   192 MiB   223.5 s  0.32 MB/s
2026-10-31T22-23-59.316Z [smolnetd@netstack::link::ethernet:246 WARN] Dropped packet on eth0 because
netget:   208 MiB   237.8 s  1.18 MB/s
netget:   224 MiB   252.1 s  1.17 MB/s
netget:   240 MiB   265.6 s  1.24 MB/s
netget:   256 MiB   280.0 s  1.17 MB/s
netget:   272 MiB   306.1 s  0.64 MB/s
netget:   288 MiB   316.4 s  1.63 MB/s
netget:   304 MiB   341.2 s  0.68 MB/s
netget:   320 MiB   359.8 s  0.91 MB/s
netget:   336 MiB   385.7 s  0.65 MB/s
netget:   352 MiB   419.8 s  0.50 MB/s
netget: 382156480 bytes in 429.6 s, 0.89 MB/s
```

- Afterwards `netget.bin` 382156480 bytes, removed; `/scheme/data` 394.2M used.

Stable image restored afterwards.

## 2026-10-31: `v0.3.0-dev-855-g81b7717+x8ea9` (image A of 170 plus `futex-wait-lock`)

Real Pi 3B+ over netboot. Test 173 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14), [#43](https://github.com/redox-rpi/redox-rpi/issues/43)), Pi test agent.

**Fail: `futex-wait-lock` does not stop the freeze. 11 runs passed, then run 12
froze the whole Pi.** Same as in 170: no devsh, no serial output, power cycle.
Image `~/redox-rpi-t12/keep/rwfutex-v0.3.0-dev-855-g81b7717+x8ea9.img`, stable
bootloader; banner `redox-rpi v0.3.0-dev-855-g81b7717+x8ea9`.

- `./bins_dynamic/pthread/rwlock_randtest`, each its own devsh command with a
  60 s client timeout: runs 1–11 **pass**, `real` 1.27 to 1.43 s.
- Run 12 (started about 22:33): no result after 65 s, `echo ok` no answer,
  **0 serial bytes** since the run started. Power-cycled at 22:34:13 with stable
  staged.
- `cond_stress` not run (the freeze ended the row).
- Together: A (170) froze in 2 of 10 runs, B with `rt-call-flags` (170) 0 of
  20, C with `futex-wait-lock` 1 of 12, and the suite in 166 once. It is neither
  `rt-call-flags` nor the futex deadlock of `57ccc8f`. The whole machine stops
  without a word (no panic), which points at the kernel (a lock or IRQ path on
  aarch64), not at relibc's rwlock.

Stable image restored afterwards.

## 2026-10-31/11-01: hub-drop fix `v0.3.0-dev-857-g7ed9070+xa901` (new) against `v0.3.0-dev-838-g64fa695+x8116` (old), alternating

Real Pi 3B+ over netboot. Test 174 (#151 follow-up, USB session), Pi test agent.

**The new image never dropped the hub (0 in 3 runs); the old one dropped it 12
and 4 times. But two of the three new-image runs stalled without a drop.**
Analysis of N2 while it was stalled: a runnable `netget` that never got CPU, and
a zero TCP window. This looks like a scheduler problem, not USB. `throttled` was
`0x00000000` before and after every run. Each run on a fresh boot, staged with
`~/redox-rpi-boot/scripts/netboot.sh`; download `netget …/Qwen3-0.6B-Q4_0.gguf
/scheme/data/netget.bin --log /scheme/debug`, no 30 s readings.

| run | image | result | last netget line | hub drops / back |
|---|---|---|---|---|
| N1 | new | **stalled**, then no devsh after (power cycle 22:58) | `160 MiB 121.4 s 1.11 MB/s` | 0 / 0 |
| O1 | old | **complete** | `382156480 bytes in 445.0 s, 0.86 MB/s` | 12 / 12 |
| N2 | new | **stalled** at 208147776 bytes from 23:10 | `192 MiB 137.1 s 0.87 MB/s` | 0 / 0 |
| O2 | old | **stalled**, then no devsh (power cycle 23:51) | `224 MiB 226.3 s 0.76 MB/s` | 4 / **3**, the last drop not recovered |
| N3 | new (extra run) | **complete** | `382156480 bytes in 345.1 s, 1.11 MB/s` | 0 / 0 |

**N2 during the stall (23:26–23:29, devsh worked):**
- `/tmp/dwc2d-usbstats`: all error counters 0, `port_events 0`, `root_events 1`,
  `recoveries 0`, `hprt0 0x1005`. `/tmp/dwc2d-step` `1159257 idle`.
  `throttled` `0x00000000`.
- `netget` (PID 55) `UR` on CPU #0, ACTTIME 0.94 s and SCHTIME 7.23 s,
  **unchanged in two samples 10 s apart**. `bcm2835-sdhostd` 2:16.44 and the
  user redoxfs (8.44 s / 1:28.23) unchanged too; initfs redoxfs 15.58 → 16.13 s.
- Host side (`ss -tin`): `Send-Q 390987`, `backoff:15`, `lastsnd:1032951` ms,
  `lastack:73299` ms. The Pi still answered window probes but took no data,
  because netget never ran to read.
- I did not get the full `/scheme/sys/context` (dwc2d's CPU) before N2's run
  timed out. N3 was added to capture it on a stall, but N3 completed.

So `usb-hub-drop` stops the hub drops (0 of 3 runs against 16 drops in 2 old
runs). The stalls in N1 and N2 (and the end of O2) have another cause: the
scheduler freeze/starvation seen in #170 and #173 is the likely one.

Stable image restored afterwards (`BUILD_ID="v0.3.0-dev-533-g6c729cd+x9f76"`).

## 2026-11-01: `v0.3.0-dev-859-g56364c4+xc41d` (A, main) and `v0.3.0-dev-886-g548e397+xc41d` (B, PR #73 write coalescing) (card write speed by write size)

Real Pi 3B+ over netboot. Test 175 ([#155](https://github.com/redox-rpi/redox-rpi/issues/155)), Pi test agent.

**Data check passes: every file's sha256 matches its source, on both images.
Coalescing (B) makes small writes much faster and cuts the card writes for them
by about 23 times.** 4 KiB writes go from 121–240 kB/s to 2.7–2.9 MB/s, and
64 KiB from 2.4–2.7 to 6.1–6.5 MB/s. 1 MiB writes get a little slower (11–12 →
7.8–8.7 MB/s). Images from `~/redox-rpi-sd/keep/` with manifests (match), stable
bootloader, each on a fresh boot. Clocks before and after on both: `arm
1400000000 1400000000`, `core 250000000 400000000`. `/scheme/data` 394.5M used
before each.

| write | A pass 1 | A pass 2 | B pass 1 | B pass 2 |
|---|---|---|---|---|
| 4 KiB × 1024 (4 MiB) | 34.7 s, **121 kB/s** | 17.47 s, 240 kB/s | 1.43 s, **2.9 MB/s** | 1.55 s, 2.7 MB/s |
| 64 KiB × 256 (16 MiB) | 6.16 s, 2.7 MB/s | 7.12 s, 2.4 MB/s | 2.60 s, **6.5 MB/s** | 2.73 s, 6.1 MB/s |
| 1 MiB × 16 (16 MiB) | 1.44 s, **12 MB/s** | 1.57 s, 11 MB/s | 2.15 s, 7.8 MB/s | 1.93 s, 8.7 MB/s |

- sha256: `/tmp/r16` `94b79f96…`, its first 4 MiB `2fae7622…`. Every 64 KiB and
  1 MiB file `94b79f96…`, every 4 KiB file `2fae7622…`. **All match.**
- `bcm2835-sdhostd` statistics. A prints a line every 5 s while busy; B prints
  one line per busy period (`28 s:`, `22 s:` …). Lines come at the end of an
  interval, so they don't split exactly at the dd boundaries. The read lines with
  about 33000–37000 blocks are the `sha256sum` reads.
- **4 KiB write, pass 1:** A wrote 1755 + 1833 + 1075 + 2314 + 164 + 127 + 993
  requests, **232744 blocks** (about 114 MiB on the card for a 4 MiB file),
  about 31 s of write time. B wrote 275 requests, **10136 blocks** (5 MiB),
  1022 ms (397 ms busy).
- All lines in order:

A:
```
bcm2835-sdhostd: 43 s: read 532 requests 4242 blocks 245 ms; write 25 requests 200 blocks 37 ms, 16 ms of it busy
bcm2835-sdhostd: 5 s: read 140 requests 1864 blocks 76 ms; write 1755 requests 48760 blocks 4416 ms, 1387 ms of it busy
bcm2835-sdhostd: 5 s: read 491 requests 6256 blocks 262 ms; write 1833 requests 51864 blocks 4190 ms, 1461 ms of it busy
bcm2835-sdhostd: 5 s: read 279 requests 3704 blocks 145 ms; write 1075 requests 30432 blocks 4536 ms, 897 ms of it busy
bcm2835-sdhostd: 5 s: read 665 requests 9600 blocks 351 ms; write 2314 requests 65376 blocks 3950 ms, 1647 ms of it busy
bcm2835-sdhostd: 5 s: read 54 requests 1224 blocks 43 ms; write 164 requests 3792 blocks 4941 ms, 201 ms of it busy
bcm2835-sdhostd: 5 s: read 37 requests 296 blocks 18 ms; write 127 requests 3800 blocks 4983 ms, 444 ms of it busy
bcm2835-sdhostd: 18 s: read 273 requests 3784 blocks 151 ms; write 993 requests 28720 blocks 4116 ms, 741 ms of it busy
bcm2835-sdhostd: 5 s: read 389 requests 10784 blocks 341 ms; write 773 requests 23480 blocks 2268 ms, 735 ms of it busy
bcm2835-sdhostd: 17 s: read 349 requests 3512 blocks 173 ms; write 1306 requests 40256 blocks 2909 ms, 1043 ms of it busy
bcm2835-sdhostd: 8 s: read 620 requests 36704 blocks 995 ms; write 9 requests 72 blocks 9 ms, 4 ms of it busy
bcm2835-sdhostd: 20 s: read 46 requests 368 blocks 26 ms; write 249 requests 33736 blocks 1212 ms, 306 ms of it busy
bcm2835-sdhostd: 6 s: read 210 requests 33424 blocks 795 ms; write 0 requests 0 blocks 0 ms, 0 ms of it busy
bcm2835-sdhostd: 5 s: read 158 requests 1512 blocks 72 ms; write 1722 requests 47008 blocks 2831 ms, 1006 ms of it busy
bcm2835-sdhostd: 5 s: read 716 requests 9640 blocks 376 ms; write 2223 requests 62896 blocks 3949 ms, 1542 ms of it busy
bcm2835-sdhostd: 5 s: read 762 requests 9616 blocks 387 ms; write 2181 requests 61616 blocks 3955 ms, 1615 ms of it busy
bcm2835-sdhostd: 22 s: read 810 requests 8768 blocks 379 ms; write 2144 requests 61296 blocks 3009 ms, 1198 ms of it busy
bcm2835-sdhostd: 5 s: read 324 requests 10264 blocks 312 ms; write 432 requests 12776 blocks 2478 ms, 467 ms of it busy
bcm2835-sdhostd: 23 s: read 601 requests 5888 blocks 274 ms; write 1647 requests 50960 blocks 3574 ms, 1312 ms of it busy
bcm2835-sdhostd: 8 s: read 693 requests 37288 blocks 1003 ms; write 9 requests 72 blocks 10 ms, 4 ms of it busy
bcm2835-sdhostd: 23 s: read 41 requests 328 blocks 20 ms; write 250 requests 33744 blocks 1348 ms, 347 ms of it busy
```

B:
```
bcm2835-sdhostd: 48 s: read 580 requests 4626 blocks 269 ms; write 25 requests 200 blocks 292 ms, 65 ms of it busy
bcm2835-sdhostd: 28 s: read 0 requests 0 blocks 0 ms; write 275 requests 10136 blocks 1022 ms, 397 ms of it busy
bcm2835-sdhostd: 22 s: read 56 requests 448 blocks 26 ms; write 1058 requests 40328 blocks 2100 ms, 868 ms of it busy
bcm2835-sdhostd: 6 s: read 323 requests 34328 blocks 840 ms; write 0 requests 0 blocks 0 ms, 0 ms of it busy
bcm2835-sdhostd: 6 s: read 2 requests 16 blocks 2 ms; write 10 requests 80 blocks 11 ms, 5 ms of it busy
bcm2835-sdhostd: 20 s: read 27 requests 216 blocks 15 ms; write 249 requests 33736 blocks 1933 ms, 364 ms of it busy
bcm2835-sdhostd: 6 s: read 196 requests 33312 blocks 797 ms; write 0 requests 0 blocks 0 ms, 0 ms of it busy
bcm2835-sdhostd: 22 s: read 15 requests 120 blocks 18 ms; write 284 requests 10208 blocks 1113 ms, 502 ms of it busy
bcm2835-sdhostd: 6 s: read 0 requests 0 blocks 0 ms; write 8 requests 64 blocks 10 ms, 3 ms of it busy
bcm2835-sdhostd: 26 s: read 53 requests 424 blocks 25 ms; write 1047 requests 40240 blocks 2246 ms, 790 ms of it busy
bcm2835-sdhostd: 8 s: read 451 requests 35352 blocks 901 ms; write 10 requests 80 blocks 10 ms, 4 ms of it busy
bcm2835-sdhostd: 25 s: read 46 requests 368 blocks 28 ms; write 249 requests 33736 blocks 1701 ms, 484 ms of it busy
```

- No panic or error on the serial console.

Stable image restored afterwards.

## 2026-11-01: `v0.3.0-dev-894-g593bd2c+xccf1` (image C of 173 plus `pi-heartbeat`)

Real Pi 3B+ over netboot. Test 176 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14), [#157](https://github.com/redox-rpi/redox-rpi/issues/157)), Pi test agent.

**No freeze: 40 of 40 `rwlock_randtest` runs passed** (`real` 1.25 to 1.45 s),
each its own devsh command with a 60 s client timeout. All four CPUs beat every
10 s the whole time. 173 froze at run 12 on the same image without the
heartbeat, so the heartbeat may change the timing enough to hide the freeze.
Image `~/redox-rpi-t12/keep/hb-v0.3.0-dev-894-g593bd2c+xccf1.img`, stable
bootloader.

- Heartbeat lines after boot and at the end (`[hb <cpu> <n> s<sched> w<switch> i<idle> p<pit>]`):

```
[hb 1 1 s1284 w96 i1188 p0]
[hb 2 1 s1647 w426 i1220 p0]
[hb 3 1 s1523 w194 i1329 p0]
[hb 0 1 s1706 w854 i839 p0]
[hb 1 2 s1307 w3 i1304 p0]
[hb 2 2 s1308 w0 i1308 p0]
[hb 3 2 s1474 w411 i1063 p0]
[hb 0 2 s2722 w1605 i1116 p0]
…
[hb 1 36 s1739 w675 i1061 p0]
[hb 2 36 s2102 w146 i1956 p0]
[hb 3 36 s2124 w168 i1956 p0]
[hb 0 36 s2957 w1995 i959 p0]
```

  Every CPU scheduled 1300 to 3000 times per 10 s with `p0`–`p2`; none showed
  `s0`.
- One `rwlock_randtest` run (PID 216, CPU #3, about beat 23) hit `FATAL: Not an
  SVC induced synchronous exception (ty=111100)`, `ESR_EL1 F2000001` (a `brk`
  trap, as from an abort or assert), `FAR_EL1 0x583000`, `ELR_EL1 0x516C4`, then
  `UNHANDLED EXCEPTION … NAME ./bins_dynamic/pthread/rwlock_ra`. All 40 devsh runs
  still reported `rc=0`, so it was probably a thread of a run or its exit. Worth
  a look.
- No freeze, so no `/scheme/sys/context` capture and no power cycle. Stable
  staged and the Pi reset into it (`BUILD_ID="v0.3.0-dev-533-g6c729cd+x9f76"`).

Stable image restored afterwards.

## 2026-11-01: `v0.3.0-dev-898-g34bafa2+x03d3` (C, `sdhost-bigreq` on coalescing) and the #175 images A and B again (larger sdhost commands)

Real Pi 3B+ over netboot. Test 177 ([#155](https://github.com/redox-rpi/redox-rpi/issues/155)), Pi test agent.

**Pass: every sha256 matches, and no `bcm2835-sdhostd: write of block …` or
`read of block …` line on any image.** But `sdhost-bigreq` changes almost
nothing: 1 MiB writes on C average **130.5 blocks per request** (A 127.7, B
129.2), so few requests are larger than 128 blocks. 1 MiB write speed is the same
on all three within the scatter. Images from `~/redox-rpi-sd/keep/` with
manifests, stable bootloader, each on a fresh boot (C 00:19, A 00:25, B 00:29
UTC). Clocks before and after on all: `core 250000000 400000000`. `/tmp/r16`
sha256 `6b359fc8…`, its first 4 MiB `e14fc5a4…`.

**C only**

| step | dd | sha256 |
|---|---|---|
| 4 KiB × 1024 (4 MiB) | 1.08 s, **3.9 MB/s** (B in #175: 2.7–2.9) | `e14fc5a4…` match |
| 64 KiB × 256, 1 | 5.35 s, 3.1 MB/s | `6b359fc8…` match |
| 64 KiB × 256, 2 | 3.17 s, 5.3 MB/s | match |
| 64 MiB file, 4 × 16 MiB writes | 1.58, 1.53, 1.56, 5.96 s (11, 11, 11, 2.8 MB/s) | see below |
| 64 MiB read to `/dev/null` | 3.35 s, **20 MB/s** | |

- 64 MiB readback: the row's `dd … skip=… count=16 | sha256sum` gave
  `0+16 records out`, `1048576 bytes`: **a pipe takes 64 KiB per write, and dd
  does not retry the short writes**, so only 1 MiB reached `sha256sum` (all four
  `c8a4fdbf…`). That is a pipe/dd effect, not the card. Rechecked on a fresh
  boot of C through a file: four 16 MiB writes (10–11 MB/s), then `dd
  if=big.bin of=/tmp/p bs=1048576 skip=<16×i> count=16; sha256sum /tmp/p`: **all
  four `6b359fc8…`, match** (reads 16–17 MB/s). `big.bin` 67108864 bytes, removed.

**1 MiB × 16 (16 MiB), 5 times each**

| image | 1 | 2 | 3 | 4 | 5 | sha256 |
|---|---|---|---|---|---|---|
| A (main) | 10 MB/s | 11 | 4.5 | 11 | 9.9 | all match |
| B (coalescing) | 11 MB/s | 9.7 | 5.3 | 10 | 6.3 | all match |
| C (+ bigreq) | 8.5 MB/s | 7.7 | 9.6 | 6.4 | 4.1 | all match |

(dd times: A 1.66, 1.57, 3.72, 1.56, 1.69 s; B 1.59, 1.72, 3.14, 1.62, 2.68 s; C 1.98, 2.19, 1.74, 2.60, 4.12 s.)

- So the #175 result (B slower than A at 1 MiB) does not hold: over 5 repeats A
  and B are alike, with one slow run each. C is a little slower on average.
- sdhost statistics for the 1 MiB writes (sum of the `write` parts): A 1061
  requests / 135464 blocks, B 1310 / 169200, C 1296 / 169088, about 128–130
  blocks per request on all three. C's 4 KiB and 64 KiB steps: 2411 requests /
  90952 blocks (37.7 per request).
- `bcm2835-sdhostd` lines per step are kept on homenodellm
  (`~/pitest-tools/t177/*-stats.txt`). No error line, no panic.

Stable image restored afterwards.

## 2026-11-01: C `v0.3.0-dev-855-g81b7717+x8ea9` and D `v0.3.0-dev-895-gff40d9c+xf49c` (`rwlock_randtest`: control run and a quiet heartbeat)

Real Pi 3B+ over netboot. Test 178 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14), [#157](https://github.com/redox-rpi/redox-rpi/issues/157)), Pi test agent.

**No freeze on either image: C (no heartbeat, the image of 173) 40 of 40, and D
(quiet heartbeat + `t1-memmap`) 40 of 40.** So the freeze of 173 (run 12) is
rare even without a heartbeat. Each run was its own devsh command with a 60 s
client timeout. C staged 00:36:57, D 00:42:24; stable bootloader.

- C: one `rwlock_randtest` thread hit the same `brk` as in 176: `FATAL: Not an
  SVC induced synchronous exception (ty=111100)`, `ESR_EL1 F2000001`, `FAR_EL1
  0xa7efc0`, **`ELR_EL1 0x516C4` (the same address as in 176)**, then
  `UNHANDLED EXCEPTION, CPU #0, PID 168, NAME ./bins_dynamic/pthread/rwlock_ra`.
  All 40 runs still reported `rc=0`. C has no `t1-memmap`, so there is no memory
  map.
- D: no exception in 40 runs, so `t1-memmap` printed nothing. Heartbeat: 16 beats
  (4 per CPU, one per 60 s), e.g. `[hb 0 1 s13505 w2929 i10565 p0]` … `[hb 0 4
  s10987 w2919 i8064 p0]`. **No `[hb stall …]` or `[hb nosched …]` line.**
- No power cycle needed.

Stable image restored afterwards.

## 2026-11-01: `v0.3.0-dev-879-gdb4fdca+x741f` (netget download stall with a quiet heartbeat)

Real Pi 3B+ over netboot. Test 179 ([#157](https://github.com/redox-rpi/redox-rpi/issues/157), [#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.
Image `hbusb-v0.3.0-dev-879-gdb4fdca+x741f.img` (`t14-hb-usb`), staged with its own bootloader. Download of 172 (`netget` of the 382156480-byte model to
`/scheme/data/netget.bin`), file size polled over devsh every 30 s.

**All 3 runs stalled, and this time the whole user side stopped: devsh stopped answering too.**
**The kernel did not: all 4 CPUs kept beating once per 60 s until the power cycle.**
**No `[hb stall …]` or `[hb nosched …]` line in any run.**

| Run | Start (UTC) | Last netget line | Last size seen | devsh |
|---|---|---|---|---|
| 1 | 00:48:46 (reset) | 240 MiB at 226 s | 221636586 | no answer from 00:53:40 on |
| 2 | 01:10:27 (power cycle) | 128 MiB at 108 s | 139827684 | no answer from 01:13:40 on |
| 3 | 01:31:33 (power cycle) | 272 MiB at 267 s | 258316644 | no answer from 01:37:19 on |

- No `hub drop` (`address 2 removed`) in any run.
- Speed before the stall: 0.3–2.3 MB/s (writing to the card), slowing down in the last minute before each stall.
- Heartbeat: 20 beats per CPU in every run (`[hb <cpu> <n> s… w… i… p…]`). The beats went on for about 17 minutes after the stall, until the next power cycle.
  - Before the stall, the CPUs are busy, e.g. run 2 `[hb 1 2 s35267 w11390 i23856 p0]`.
  - After the stall, the numbers freeze at a low, flat level, e.g. run 2 from beat 4 on, every minute: `[hb 1 n s7638 w1092 i6546]`, `[hb 2 n s7635 w1090 i6545]`, `[hb 3 n s6569 w24 i6545]`, `[hb 0 n s6000 w0 i6000]`.
  - Runs 1 and 3 look the same. In run 1, CPU 3 keeps `w≈8500` per minute.
  - So the timer and `schedule()` run on every CPU, but almost nothing is runnable.
- **Not captured:** `/scheme/sys/context` and the dwc2d stats. devsh was already dead at the first poll after the stall, so the runner's stall check (two equal sizes) never fired.
- **Not checked:** whether the network stack still answered TCP.
- For a rerun: read the context over the serial console when devsh stops.
- This differs from 174, where devsh still answered and `netget` sat runnable on CPU 0.
- Power cycles: 01:10:27, 01:31:33, 01:52:59 (the last one with stable staged; stable came up to login and devsh).

Stable image restored afterwards.

## 2026-11-01: `v0.3.0-dev-877-g6879e2a+x4d89` (pipe write takes all bytes)

Real Pi 3B+ over netboot. Test 180 (relibc), Pi test agent. Stable bootloader.

**Pass: a pipe write now takes the whole 1 MiB block.** This was 64 KiB before, `0+16 records out`, see 177.

- `dd if=/dev/zero bs=1M count=16 | wc -c`: `16+0 records out`, `wc` counts **16777216**.
- `dd if=/dev/urandom of=/tmp/r16 bs=1M count=16`, then `sha256sum /tmp/r16`, `dd if=/tmp/r16 bs=1M count=16 | sha256sum` and `cat /tmp/r16 | sha256sum`:
  - all three give the same hash, `6b359fc8…b3bae775bc`;
  - `dd` shows `16+0 records out`.
- `dd if=/dev/zero bs=1M count=16 | head -c 100 | wc -c`:
  - prints `100`;
  - `dd` stops with `dd: IO error: Broken pipe` after `0+2 records out` (64 KiB);
  - returns at once, and the next command (`echo prompt-back`) runs normally.
- pi-test: version, manifest (build ID and `libc.so` hash), `ps`, `ps` to the terminal and SD card all pass; 0 of 300 starts failed.

Stable image restored afterwards, and stable came up to devsh.

## 2026-11-01: `v0.3.0-dev-881-g394af65+x741f` (netget stall with a kernel context dump)

Real Pi 3B+ over netboot. Test 181 ([#157](https://github.com/redox-rpi/redox-rpi/issues/157), [#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.
Image `hbusb2-v0.3.0-dev-881-g394af65+x741f.img`, staged with its own bootloader.
Download of 172 to `/scheme/data/netget.bin`. Progress was read from the `netget:` lines on serial, and a stall meant no new line for 5.5 min. After each run: `cat /scheme/sys/context` over devsh, then a power cycle.

**2 of 3 runs stalled. Unlike 179, devsh still answered, and the picture is the same as in 174:**
- `netget` is **runnable (`UR`) on CPU 0**.
- CPU 0 does **almost no work** (`w0`–`w24` per minute after the stall, against several thousand before).
- The kernel's automatic dump did not fire, because the other CPUs still switch contexts.

| Run | Start (UTC) | Result | Size reached |
|---|---|---|---|
| 1 | 02:07:16 (reset) | stalled, last progress 02:12:41 | 272 MiB at 255.5 s (0.86 MB/s) |
| 2 | 02:18:25 (power cycle) | **complete**: `382156480 bytes in 313.3 s, 1.22 MB/s` | all |
| 3 | 02:24:52 (power cycle) | stalled, last progress 02:28:57 | 256 MiB at 187.3 s |

- **`[hbctx …]` dumps:** only the baseline dump, about 2 min after boot (`[hbctx begin beat 2 w…]` … `[hbctx end]`), in each run.
  - No dump came after either stall, 5.5 min in each case.
  - The trigger, fewer than 100 switches per minute on all CPUs together, was never met. The other CPUs still made about 6000–18000 schedule calls per minute.
  - In the baseline dump, `netget` (pid 54) is `B` in `UserInner::call`, `netstack` in `EventQueue::read`, `dwc2d` in `EventQueue::read`, and `redoxfs` in `UserInner::call`/`UserInner::read (v2)`.
- **After the stall, `/scheme/sys/context` (devsh):**
  - run 1: `54 UR #0 /usr/bin/netget`, `20 UR #2 /scheme/initfs/bin/redoxfs`, `25 UB #3 netstack`, `16 UB #3 dwc2d`, `31 UB #0 /usr/bin/redoxfs`;
  - run 3: the same, with `netget UR #0` and initfs `redoxfs UR #3`.
- **Heartbeat after the stall:**
  - CPU 0: run 1 `[hb 0 10 s6440 w0 i6440]`, run 3 `[hb 0 9 s6436 w0 i6436]`.
  - So CPU 0 takes its timer and calls `schedule()`, but always picks idle, while `netget` waits on its run queue.
  - The other CPUs keep doing some work, e.g. `[hb 2 10 s18139 w8243 …]`.
- No `[hb stall …]`/`[hb nosched …]` line and no hub drop in any run.
- Suggestion: the dump could also trigger on "a CPU did no work for 60 s while one of its contexts was runnable".

Stable image restored afterwards (02:34), and stable came up to devsh.

## 2026-11-01: `v0.3.0-dev-891-g55d0446+x9f76`

Built 2026-11-01T03:30:25Z, `libc.so` `d8e9a5ac4a3e35a3…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Nightly stable image test (scripts/update-stable.sh).

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.3.0-dev-891-g55d0446+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.3.0-dev-891-g55d0446+x9f76.toml |
| ps | pass | 46 processes (piped) |
| ps-tty | pass | 46 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## 2026-11-01: `v0.3.0-dev-946-gd7fa50e+x4ec1` (sshd with relibc `0040`/`0041` reworked)

Real Pi 3B+ over netboot. Test 182 ([#45](https://github.com/redox-rpi/redox-rpi/issues/45)), Pi test agent. Stable bootloader.
Staged 04:44, and the boot reached login with no `FATAL` or panic line.

**Partly: logins, the session script and pi-test pass. The login grace time does not work as asked:**
- With an idle TCP connection, sshd never closed it: not in 200, 240 or 331 s.
- `Timeout before authentication` is written to serial only **after the client closes the connection**, not about 2 min after it opens.

- `ls /etc/ssh`: the host keys were already there at the first look (04:45), ed25519, ecdsa, rsa and mldsa44_ed25519.
- The test key `/tmp/qkey.pub` went into `/home/root/.ssh/authorized_keys` over devsh, in single quotes. `wc -c` gives `100` (the same as the file on homenodellm).
- 5 key logins `ssh -i /tmp/qkey root@192.168.178.148 'echo loginN'`: `login1` … `login5`, all rc 0, with no failed login.
- `ssh-session.exp` (port 22, Pi address): prompt, command, prompt-again, backspace, prompt-again, exit, all `ok`, rc 0.
- **Login grace time:** the client sent nothing after the server banner `SSH-2.0-OpenSSH_10.5`. Three tries:
  - held 200 s: the server did not close. The line (`pid = 99`) was in the log right after the client closed.
  - held 240 s, log checked every 5 s: no line while open. The line (`pid = 101`) appeared after the close.
  - held 331 s, log checked every 3 s: no line and no close from the server while open. The line (`pid = 109`) appeared after the client closed.
  - So the `SIGALRM` from `setitimer` seems not to reach sshd while it waits in `read`. It only acts once the read returns.
- Serial: no `setitimer`/`setgroups` error and no `madvise`/`getsockopt`/`setsockopt` TODO line. The only TODO is `sshd-session: TODO: ioctl TIOCSCTTY`.
- pi-test (`--manifest`): version, manifest (build ID and `libc.so` hash), `ps`, `ps` to the terminal and SD card all pass; 0 of 300 starts failed.

Stable restored afterwards: the nightly's new stable `v0.3.0-dev-891-g55d0446+x9f76` from 03:30, which is now the backup in `tftp-stable`. It came up to devsh.

## 2026-11-01: `v0.3.0-dev-883-g62d7c39+x741f` (netget stall, context dump with run queue state)

Real Pi 3B+ over netboot. Test 183 ([#157](https://github.com/redox-rpi/redox-rpi/issues/157), [#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.
Image `hbusb3-…`, own bootloader, the download of 172 as in 181. A stall means no new `netget:` line for 5.5 min.

**2 of 3 runs stalled, and the new dump fired after both stalls.**
- It shows a user context that is **runnable, in its CPU's run queue and active (`Rqa`), never running**.
- Its CPU shows `q1` but makes **no switch at all (`w0`)**, while `[kmain]` (idle) stays `R+c` on that CPU.
- The stuck context was not `netget` itself: it was `dwc2d` in run 2 and `netstack` in run 3. `netget` is blocked (`B`) behind it.

| Run | Start (UTC) | Result | Size reached | Dumps after the stall |
|---|---|---|---|---|
| 1 | 05:47:13 (reset) | **complete**: `382156480 bytes in 351.4 s, 1.09 MB/s` | all | – (only the baseline dump) |
| 2 | 05:54:09 (power cycle) | stalled, last progress 05:57:25 | 192 MiB at 145.2 s | 3 × `runnable-on-idle-cpu` (beats 5, 6, 7) |
| 3 | 06:03:08 (power cycle) | stalled, last progress 06:08:19 | 304 MiB at 264.7 s | 3 × `runnable-on-idle-cpu` (beats 7, 8, 9) |

**Run 2:** the first dump after the stall, `[hbctx begin beat 5 runnable-on-idle-cpu]`. Beats 6 and 7 are the same.
```
[hbcpu 0 w24 q0]
[hbcpu 1 w0 q1]
[hbcpu 2 w2182 q0]
[hbcpu 3 w0 q0]
[hbctx 0 [kmain] R+c cpu1 ]
[hbctx 16 /scheme/initfs/bin/dwc2d Rqa sys cpu1 ]
[hbctx 25 /usr/bin/netstack B sys cpu1 UserInner::call]
[hbctx 20 /scheme/initfs/bin/redoxfs B sys cpu1 UserInner::read (v2)]
[hbctx 32 /usr/bin/redoxfs B sys cpu1 UserInner::read (v2)]
[hbctx 54 /usr/bin/netget B sys cpu1 UserInner::call]
```
- `netget`: `B`, no flags, cpu1, `UserInner::call`.

**Run 3:** the dump at beat 7. Beats 8 and 9 are the same.
```
[hbcpu 0 w0 q1]
[hbcpu 1 w9380 q0]
[hbcpu 2 w0 q0]
[hbcpu 3 w24 q0]
[hbctx 0 [kmain] R+c cpu0 ]
[hbctx 25 /usr/bin/netstack Rqa sys cpu0 ]
[hbctx 16 /scheme/initfs/bin/dwc2d B sys cpu1 EventQueue::read]
[hbctx 20 /scheme/initfs/bin/redoxfs B sys cpu1 UserInner::read (v2)]
[hbctx 32 /usr/bin/redoxfs B sys cpu0 UserInner::read (v2)]
[hbctx 54 /usr/bin/netget B sys cpu0 UserInner::call]
```
- `netget`: `B`, no flags, cpu0, `UserInner::call`.

**Other details:**
- In all dumps, the only non-blocked user contexts besides the stuck one are `rpi-firmwared` and `timesync` (`S`, `nanosleep`).
- devsh did not log in after either stall (`pirun` login failed), so there is no `/scheme/sys/context` from devsh this time.
- No `[hb stall …]`/`[hb nosched …]` line and no hub drop in any run.
- Power cycles: 05:54:09, 06:03:08, 06:14:08.

Stable image restored afterwards (the nightly's `v0.3.0-dev-891-g55d0446+x9f76`).

## 2026-11-01: `v0.3.0-dev-906-g13b2b1a+xf576` (`rwlock_randtest` with the `futex_wake` EFAULT fix)

Real Pi 3B+ over netboot. Test 184 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14)), Pi test agent.
Stable bootloader, staged 06:14 (power cycle). Each run is its own devsh command with a 60 s client timeout, with stderr kept, as in 178.

**57 runs passed, then run 58 froze the Pi.**
- **No `RELIBC PANIC`** in the stderr of any run, and no `brk`/`UNHANDLED EXCEPTION`/`FATAL` on serial. So the panic of 176/178 did not show in 57 runs. It came about once in 40 runs before, so 57 clean runs is a hint, not proof.
- Runs 1–57: all `rc=0`. `real` times were 1.27–1.47 s, median 1.37 s.
- Run 58 started about 06:24 and printed nothing: no output, no `rc`, client timeout after 65 s.
  - devsh no longer answered.
  - Serial showed nothing new in the 90 s after the freeze.
  - This image has no heartbeat, so there is no kernel view of the freeze.
- This is the same kind of freeze as in 170 and 173 (run 12).
- Power cycle at 06:27:11 with stable staged.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`).

## 2026-11-01: `v0.3.0-dev-885-g626efe2+x741f` (netget stall: scheduler skip counters)

Real Pi 3B+ over netboot. Test 185 ([#157](https://github.com/redox-rpi/redox-rpi/issues/157), [#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.
Image `hbusb4-…`, own bootloader, as in 183. A stall means no new `netget:` line for 5.5 min.

**1 of 3 runs stalled. On the stuck CPU none of the three skip counters grows after the stall.**
- The queued context `Rqa` on CPU 0 (this time `bcm2835-sdhostd`) is never passed over for a taken lock or a Skip.
- So `select_next_context` on CPU 0 seems not to look at it at all, or CPU 0 never gets that far: `w0` and the same counters in all 3 dumps.

| Run | Start (UTC) | Result | Size reached | Dumps after the stall |
|---|---|---|---|---|
| 1 | 06:47:02 (reset) | stalled, last progress 06:50:58 | 208 MiB at 162.7 s | 3 × `runnable-on-idle-cpu` (beats 5, 6, 7) |
| 2 | 06:56:45 (power cycle) | **complete**: `382156480 bytes in 402.8 s, 0.95 MB/s` | all | – (baseline only) |
| 3 | 07:04:44 (power cycle) | **complete**: `382156480 bytes in 398.4 s, 0.96 MB/s` | all | – (baseline only) |

Run 1, the baseline dump at beat 2, during the download:
```
[hbcpu 0 w3605 q1 sl24 sa22 sr141]
[hbcpu 1 w11252 q0 sl671 sa240 sr943]
[hbcpu 2 w10169 q0 sl664 sa206 sr1198]
[hbcpu 3 w9043 q0 sl523 sa140 sr1130]
[hbctx 17 /scheme/initfs/bin/bcm2835-sdhos R+ca sys cpu0 ]
```
Run 1, the dumps after the stall. The `[hbctx 17 /scheme/initfs/bin/bcm2835-sdhos Rqa cpu0 ]` line is the same in all three. It has no `sys` field, unlike `dwc2d`/`netstack` in 183.
```
beat 5: [hbcpu 0 w0 q1 sl85 sa45 sr264] [hbcpu 1 w1130 q0 sl850 sa271 sr1332] [hbcpu 2 w24 q0 sl943 sa283 sr1659] [hbcpu 3 w8941 q0 sl945 sa209 sr1814]
beat 6: [hbcpu 0 w0 q1 sl85 sa45 sr264] [hbcpu 1 w2203 q0 sl850 sa271 sr1332] [hbcpu 2 w24 q0 sl943 sa283 sr1659] [hbcpu 3 w8199 q0 sl945 sa209 sr1814]
beat 7: [hbcpu 0 w0 q1 sl85 sa45 sr264] [hbcpu 1 w2209 q0 sl850 sa271 sr1332] [hbcpu 2 w24 q0 sl943 sa283 sr1659] [hbcpu 3 w8210 q0 sl945 sa209 sr1814]
```
- **Counters after the stall:**
  - All counters stay the same from beat 5 to beat 7, on every CPU, including CPU 3, which still switches (`w8199`, `w8210`).
  - Between the baseline and beat 5, CPU 0's counters grew (`sl24→85`, `sa22→45`, `sr141→264`) while it still worked.
- **Other contexts:** `netget` is `B sys cpu0 UserInner::call`, `/usr/bin/redoxfs` is `B sys cpu0 UserInner::call`, and `dwc2d`, `netstack` and the initfs `redoxfs` are `B` on cpu3.
- **Not seen:** no `[hb stall …]`/`[hb nosched …]` line and no hub drop.
- **Power cycles:** 06:56:45, 07:04:44.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`).

## 2026-11-01: `v0.3.0-dev-915-gc9b7a47+x913e` (`rwlock_randtest` with the futex fix and the heartbeat)

Real Pi 3B+ over netboot. Test 186 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14), [#157](https://github.com/redox-rpi/redox-rpi/issues/157)), Pi test agent. Stable bootloader, staged 07:12.
80 single runs as in 184: each run is its own devsh command, with a 60 s client timeout and stderr kept.

**Pass: 80 of 80 runs, all `rc=0`, no freeze.**
- No `RELIBC PANIC` in any run's stderr, and no `brk`/`UNHANDLED EXCEPTION`/`FATAL` on serial.
- Run times: median 1.37 s, range 1.28–3.40 s. Run 41 took 3.40 s and run 11 took 2.42 s; all others took ≤ 1.54 s.
- **Heartbeat:** 10 beats per CPU, plus the baseline dump at beat 2.
  - One `[hb stall 0 seen-by 2 t33997 s74738]` came between beats 5 and 6: CPU 2 saw CPU 0 take no timer interrupt for 2 s.
  - CPU 0 kept beating afterwards (`[hb 0 6 s13485 w7386 …]` … `[hb 0 9 …]`).
  - No `runnable-on-idle-cpu` dump and no `[hb nosched …]`.
- Note: the freeze of 184 (run 58) did not come back in 80 runs. With 170/173/184 it showed about once in 12–60 runs, so this is a hint, not proof.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`).

## 2026-11-01: `v0.3.0-dev-887-g19628e3+x741f` (netget stall: the real run queue in the dump)

Real Pi 3B+ over netboot. Test 187 ([#157](https://github.com/redox-rpi/redox-rpi/issues/157), [#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.
Image `hbusb5-…`, own bootloader, as in 185. A stall means no new `netget:` line for 5.5 min.

**1 of 3 runs stalled, but this time no dump followed the stall.**
- Neither `runnable-on-idle-cpu` nor the low-switch trigger fired, so there are no `[hbq …]` lines from after the stall. Only the baseline dump exists.
- After the stall, CPU 1 makes no switch (`w0`) for 3 beats. A dump did not fire, so there was presumably no queued runnable user context on CPU 1 this time.

| Run | Start (UTC) | Result | Size reached | Dumps after the stall |
|---|---|---|---|---|
| 1 | 08:00:55 (reset) | **complete**: `382156480 bytes in 367.5 s, 1.04 MB/s` | all | – |
| 2 | 08:08:05 (power cycle) | stalled, last progress 08:10:46 | 144 MiB at 104.4 s | **none** |
| 3 | 08:16:32 (power cycle) | **complete**: `382156480 bytes in 378.9 s, 1.01 MB/s` | all | – |

**Run 2 heartbeat after the stall.** It was the same every minute from beat 5 to beat 7, with no `[hb stall …]`/`[hb nosched …]`:
```
[hb 1 5 s6549 w0 i6549 p0]
[hb 2 5 s7674 w1124 i6550 p0]
[hb 3 5 s7551 w1100 i6451 p0]
[hb 0 5 s13981 w7647 i6334 p0]
```
- CPU 1 does nothing.
- CPU 0 keeps up about 7700 switches per minute, against about 1100 on CPUs 2 and 3. In 181/183/185 the stuck CPU showed `w0` and one other CPU stayed busy, so the pattern is similar.

**Run 2 baseline dump** (beat 2, during the download, before the stall):
```
[hbcpu 0 w5076 q0 sl27 sa10 sr292 sv1304 ss33]
[hbcpu 1 w16618 q1 sl819 sa469 sr855 sv71 ss245]
[hbcpu 2 w5569 q0 sl573 sa140 sr1369 sv10 ss139]
[hbcpu 3 w8648 q0 sl446 sa102 sr1676 sv6 ss123]
[hbq 0 v96422 n0 tw48460]
[hbq 1 v1076625898 n1 tw2048]
[hbq 1 vt18446744073709409936 id36 25 /usr/bin/netstack own]
[hbq 2 v18446744072109357999 n0 tw0]
[hbq 3 v18446744073488998990 n0 tw0]
[hbctx 17 /scheme/initfs/bin/bcm2835-sdhos R+ca sys cpu1 ]
[hbctx 25 /usr/bin/netstack B sys cpu1 EventQueue::read]
[hbctx 54 /usr/bin/netget B sys cpu1 UserInner::call]
```

**Points for the t1 session:**
- In the baseline, `netstack` is in CPU 1's queue (`own`) while its state is `B` (blocked in `EventQueue::read`).
- Several `vt`/`v` values are near 2^64. CPU 2's and CPU 3's queue `v` and netstack's `vt` look like a negative number wrapped around (`vt` = 2^64 − 141680).
- CPU 0's queue has `n0` but `tw48460`.

**Other notes:**
- devsh no longer logged in after the stall, so there is no `/scheme/sys/context`.
- Run 2 had 3 `smolnetd … Dropped packet on eth0 because waiting queue was full` warnings at 08:09:40, about 1 min before the stall. Run 3 had 1 and completed; run 1 had none.
- No hub drop in any run.
- Power cycles: 08:08:05, 08:16:32.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`).

## 2026-11-01: `v0.3.0-dev-889-g7349b1c+x741f` (netget stall: where the scheduler's virtual time wraps)

Real Pi 3B+ over netboot. Test 188 ([#157](https://github.com/redox-rpi/redox-rpi/issues/157), [#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.
Image `hbusb6-…`, own bootloader, as in 187. A stall means no new `netget:` line for 5.5 min. A host-side watcher on the serial log wrote down the time of every `[hbv …]`, `[hbctx begin …]` and `netget:` line.

**1 of 3 runs stalled. In the stall, `netstack` sits in CPU 1's queue as `Rqa` with `vt18446744073709551615` = exactly 2^64 − 1 (`u64::MAX`), and CPU 1 does nothing (`w0 q1`).**
- This value shows from the dump at the moment of the stall (beat 6, 08:57:22; last progress 08:57:23) through all later dumps, unchanged.
- The `[hbv]` lines (virtual time above 2^62) come in **every** run, including the two that completed. They all appear together, about 40 s into the download.

| Run | Start (UTC) | Result | Size reached | Dumps |
|---|---|---|---|---|
| 1 | 08:37:12 (reset) | **complete**: `382156480 bytes in 308.0 s, 1.24 MB/s` | all | baseline, timed beat 4 |
| 2 | 08:43:36 (power cycle) | **complete**: `382156480 bytes in 357.2 s, 1.07 MB/s` | all | baseline, timed 4, 6 |
| 3 | 08:50:57 (power cycle) | stalled, last progress 08:57:23 | 336 MiB at 332.1 s | baseline, timed 4, 6, 8, 10; `runnable-on-idle-cpu` 7, 9, 11 |

**`[hbv …]` lines with the time they appeared.** Each set came in one block, between the `48 MiB` and `64 MiB` netget lines:
- run 1, 08:38:47 (download started 08:38:05):
  ```
  [hbv 1 cpu3 5429956981653779096 5429956981653779096 2036253472034782219 36]
  [hbv 2 cpu0 15611067510684501972 15611067510670520911 30000104 15611067510671305635]
  [hbv 3 cpu3 5429956981653779096 3393734875881000556 31366261423856 2036253472034202396]
  [hbv 4 cpu3 5429956981653805407 5429956981653779096 56459 1024]
  [hbv 5 cpu3 5429956981653779096 2036253472034782219 0 56459]
  ```
- run 2, 08:45:00:
  ```
  [hbv 1 cpu2 5087070066968239213 5087070066968239213 2543574260915891133 36]
  [hbv 2 cpu1 17049878228753944369 17049878228740317518 29240052 17049878228741964617]
  [hbv 3 cpu2 5087070066968239213 2543574260914814450 78454861415877 2543574260914840640]
  [hbv 4 cpu2 5087070066968261252 5087070066968239213 47291 1024]
  [hbv 5 cpu2 5087070066968239213 2543574260915891133 0 47291]
  ```
- run 3, 08:52:21:
  ```
  [hbv 1 cpu2 7054430493981558515 7054430493981558515 3527878091474425974 36]
  [hbv 2 cpu2 14324592221479799892 14324592221479610154 407135 14324592221480627588]
  [hbv 3 cpu2 7054430493981558515 3527878091473271231 1325688965024312 3527878091473311596]
  [hbv 4 cpu2 7054430493981578199 7054430493981558515 42239 1024]
  [hbv 5 cpu2 7054430493981558515 3527878091474425974 0 42239]
  ```

**Run 3, the first dump after the stall** (beat 7, 08:58:22, `runnable-on-idle-cpu`):
```
[hbcpu 0 w1115 q0 sl228 sa235 sr471 sv5327 ss161]
[hbcpu 1 w0 q1 sl964 sa368 sr2748 sv1524 ss215]
[hbcpu 2 w0 q0 sl1036 sa468 sr3058 sv506 ss210]
[hbcpu 3 w8645 q0 sl441 sa153 sr1359 sv8283 ss87]
[hbq 0 v296843 n0 tw15028]
[hbq 1 v126242 n1 tw7168]
[hbq 1 vt18446744073709551615 id36 25 /usr/bin/netstack own]
[hbq 2 v18446744073451387897 n0 tw0]
[hbq 3 v18446744073455149703 n0 tw10572]
[hbctx 25 /usr/bin/netstack Rqa sys cpu1 ]
[hbctx 54 /usr/bin/netget B sys cpu1 UserInner::call]
```
- The other user contexts are blocked or asleep.
- The timed dump at beat 6 (08:57:22) already has the same `hbq 1` lines. Its `[hbcpu 1 w5058 q1 …]` shows CPU 1 still switching in that minute.
- The beat-4 dump (08:55:22) shows CPU 1's queue `v3459342 n0`, so it was empty then.
- From beat 6 on, `[hbq 1 v126242 n1 tw7168]` and the netstack entry do not change.
- CPU 1's counters stay the same from beat 7 on. `sv` (vtime filter) does not grow, so the entry is not counted as filtered either.

**Runs 1 and 2 (complete):** their dumps show queue entries near 2^64 that later cleared up.
- run 1: `[hbq 1 vt18446744073709550695 id27 16 dwc2d own]` and `[hbq 3 vt18446744073564933720 id36 25 netstack stale]`.
- run 2: netstack `stale` in CPU 0's queue across several dumps (`vt18446744072764220028` …), then `own` with `vt18446744073709551450`.

**Other notes:**
- No `[hb stall …]`/`[hb nosched …]` line and no hub drop.
- Power cycles: 08:43:36, 08:50:57, 09:03:12.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`).

## 2026-11-01: `v0.3.0-dev-890-gfa908ca+x578b` (netget download stall: confirm `sched-vtime-clamp`)

Real Pi 3B+ over netboot. Test 189 ([#157](https://github.com/redox-rpi/redox-rpi/issues/157), [#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent.
Image `vc-…`, own bootloader, as in 188. 6 runs, with a power cycle before each run after the first. A stall means no new `netget:` line for 5.5 min.

**The virtual time no longer runs away:**
- no `[hbv …]` line in any of the 6 runs;
- the largest `v`/`vt` value in any dump is 3.38e13, far below 2^62 (4.6e18), where 188 had about 5e18–1.8e19 in every run;
- no `runnable-on-idle-cpu` dump in any run.

**But only 5 of 6 runs completed.** Run 5 stalled at 320 MiB from a different cause: **the USB controller (`dwc2d`) stopped working**. Every hub request then timed out with `Timeout("channel halt")`, and the scheduler state looked normal.

| Run | Start (UTC) | Result | Time, speed | `[hbv]` | largest v/vt |
|---|---|---|---|---|---|
| 1 | 09:27:06 (reset) | **complete** | 868.1 s, 0.44 MB/s (slow stretch at 0.07–0.4 MB/s near the start) | 0 | 3.36e13 |
| 2 | 09:43:02 (power cycle) | **complete** | 305.7 s, 1.25 MB/s | 0 | 3.34e13 |
| 3 | 09:49:33 (power cycle) | **complete** | 308.9 s, 1.24 MB/s | 0 | 3.38e13 |
| 4 | 09:55:50 (power cycle) | **complete** | 288.5 s, 1.32 MB/s | 0 | 3.36e13 |
| 5 | 10:01:41 (power cycle) | **stalled**, last progress 10:07:09 | 320 MiB at 275.1 s | 0 | 3.34e13 |
| 6 | 10:12:55 (power cycle) | **complete** | 422.4 s, 0.90 MB/s | 0 | 3.34e13 |

**Run 5, the order of events on serial:**
- 10:04:18 and 10:05:18: 6 × `smolnetd … Dropped packet on eth0 because waiting queue was full`. The download kept going.
- After `netget: 320 MiB 275.1 s`:
  - at 10:07:43, 2 × `smolnetd … ERROR Dropped outboud packet on eth0 (failed to write to network file)`;
  - then `dwc2d: request [a3, 00, 00, 00, 01, 00, 04, 00] to address 1 (max packet 64, High speed) failed: Timeout("channel halt")` / `dwc2d: hub 1 port 1: Timeout("channel halt")`;
  - the same follows for every port of hub 1 and hub 2, over and over: 542 `channel halt` lines in all.
- No `address 2 removed` (hub drop) line.

**Run 5, the first dump after the stall** (beat 8, `timed`):
```
[hbcpu 0 w629 q0 sl180 sa179 sr448 sv1 ss297]
[hbcpu 1 w0 q0 sl854 sa153 sr1717 sv50 ss155]
[hbcpu 2 w2424 q0 sl924 sa313 sr1898 sv71 ss280]
[hbcpu 3 w0 q0 sl677 sa121 sr971 sv130 ss81]
[hbq 0 v33351827909844 n0 tw55656]
[hbq 1 v37594558179 n0 tw48128]
[hbq 2 v45102940205 n0 tw0]
[hbq 3 v27713052001 n0 tw115380]
[hbctx 0 [kmain] R cpu0 ]
[hbctx 16 /scheme/initfs/bin/dwc2d R+ca cpu0 ]
[hbctx 25 /usr/bin/netstack B sys cpu0 UserInner::call]
[hbctx 54 /usr/bin/netget B sys cpu0 UserInner::call]
```
- All run queues are empty (`n0`), and nothing is left queued.
- `dwc2d` is the running context on CPU 0, with no `sys`.
- Everything else on the network path waits behind it.
- devsh did not log in after the stall.

**`[hb nosched …]` lines**, which did not show in 188:
- run 2: `[hb nosched 3 seen-by 2 t317 s1531587]` (and seen by 1);
- run 3: `[hb nosched 0 seen-by 1 t300 s1515061]`;
- run 5: `[hb nosched 0 seen-by 1 t55 s87007]` (and seen by 3), at boot before login;
- run 6: `[hb nosched 0 seen-by 3 t299 s1528449]` (and seen by 1).

Each came once per run, and the CPU kept beating afterwards. No `[hb stall …]` line and no hub drop in any run.

Power cycles: 09:43:02, 09:49:33, 09:55:50, 10:01:41, 10:12:55. Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`).

## 2026-11-01: `v0.3.0-dev-974-g3cf2086+x809a` (dwc2d restarts a stuck USB controller)

Real Pi 3B+ over netboot. Test 190 ([#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent. Stable bootloader, staged 10:57.

**Fail: the restart runs, but the network never comes back.**
- Part A: the first forced restart printed `controller restarted`, but no device came back and devsh was gone for good. The second forced restart printed nothing.
- Part B: 4 of 5 valid runs completed. In run 6 the controller got stuck at 240 MiB, and the restart failed with `controller restart failed: Timeout("AHB idle")`. The network did not come back.

**Part A (forced restart while idle):** reset 10:57:19, at login 10:57:40.
- Restart 1, at 10:58:00 (`echo restart > /tmp/dwc2d-cycle` over devsh). Serial:
  ```
  dwc2d: controller stuck, restarting it: GINTSTS 0x54000009 HPRT0 0x1405 HFNUM 0xbf93e2
  dwc2d: errors so far: xact 0 babble 0 stall 0 toggle 0 ahb 0 timeouts 0, last 0x0 at address 0 endpoint 0, 12060 ms ago
  dwc2d: controller restarted
  dwc2d: keyboard removed
  ```
  - Then nothing more from dwc2d: no `device on root port`, no `network on the adapter`.
  - devsh did not answer within 264 s (or later).
- Restart 2: not sent. devsh was dead, so the command did not reach the Pi, and there was no new dwc2d line.
- Read over the serial console at 11:10 (`/tmp/dwc2d-usbstats`):
  - `restarts 1`, `halt_failures 0`, `timeouts 0`, `recoveries 0`;
  - **`hprt0 0x400`** (port power on, no device connected), `last_hprt0 0x100f`, `gintsts 0x54000029`;
  - `/tmp/dwc2d-step`: `795232 idle`.
- `grep -E … /scheme/sys/context` did not run: Redox `grep` answers `Invalid parameter '-E'`.

**Part B (the #189 download, 382156480 bytes):**

| Run | Start (UTC) | Result | Bytes, seconds |
|---|---|---|---|
| 1 | 11:11 | **not run**: the Pi was still without network from part A, and the runner's reset goes over devsh, so the Pi was never reset (my runner's fault, not the image's) | – |
| 2 | 11:26:05 (power cycle) | **complete** | 382156480 in 366.8 s, 1.04 MB/s |
| 3 | 11:33:33 (power cycle) | **complete** | 382156480 in 309.1 s, 1.24 MB/s |
| 4 | 11:39:53 (power cycle) | **complete** | 382156480 in 292.4 s, 1.31 MB/s |
| 5 | 11:45:57 (power cycle) | **complete** | 382156480 in 333.4 s, 1.15 MB/s |
| 6 | 11:52:51 (power cycle) | **stalled** at `240 MiB 196.2 s`, last progress 11:57:03 | – |

- Runs 2–5: no `stuck`, `restarted`, `removed` or `Timeout` line.
- Run 6, in order:
  ```
  netget:   240 MiB   196.2 s  1.81 MB/s
  [smolnetd … ERROR] Dropped outboud packet on eth0 (failed to write to network file)    (11:57:10)
  dwc2d: request [a3, 00, 00, 00, 01, 00, 04, 00] to address 1 (max packet 64, High speed) failed: Timeout("channel halt")
  dwc2d: hub 1 port 1: Timeout("channel halt")
  … the same for hub 1 ports 2, 3, 4 …
  dwc2d: controller stuck, restarting it: GINTSTS 0x54000039 HPRT0 0x1005 HFNUM 0x2033c6 GNPTXSTS 0x8080400 HPTXSTS 0x9d080400 HAINT 0x0 GRSTCTL 0x40000000; ch0 HCCHAR 0xc0500040 HCINT 0x0 HCTSIZ 0x60080008; ch1 HCCHAR 0x81588a00 HCINT 0x20 HCTSIZ 0xc0d83800; ch2 …
  dwc2d: errors so far: xact 0 babble 0 stall 0 toggle 0 ahb 0 timeouts 31, last 0x80000000 at address 1 endpoint 0, 59 ms ago
  dwc2d: controller restart failed: Timeout("AHB idle")
  dwc2d: keyboard removed
  dwc2d: device on root port, High speed
  dwc2d: request [80, 06, 00, 01, 00, 00, 08, 00] to address 0 (max packet 64, High speed) failed: Timeout("channel halt")
  dwc2d: root port: Timeout("channel halt")
  dwc2d: root port: HPRT0 0x100d
  ```
  - Then nothing more. The full `controller stuck` line, with all channels, is in `t190/r6-stuckline.txt` on homenodellm.
  - Lines in run 6: `stuck` 1, `restarted` 0, `removed` 1 (`keyboard removed`), `Timeout` 11.
  - Read over serial at 12:02, `/tmp/dwc2d-usbstats`: `halt_failures 40`, `restarts 1`, `timeouts 36`, `last_error 0x80000000`, `root_events 2`, `hprt0 0x1005`, `last_hprt0 0x100d`, `gintsts 0x54000039`. `/tmp/dwc2d-step`: `582139 idle`.
  - netget's TCP connection did not survive, and the network did not come back within 6 min. devsh was dead.
- The download stall itself looks the same as #189 run 5, and the new code does detect it (`controller stuck`).

Afterwards: power cycle 12:03:44 with stable staged. `rm /scheme/data/netget.bin` was not done at the end because devsh was dead; the stable image does not use that file. Stable came up to devsh.

## 2026-11-01: `v0.3.0-dev-978-gf2e0318+x809a` (dwc2d restarts a stuck USB controller, second try)

Real Pi 3B+ over netboot. Test 191 ([#151](https://github.com/redox-rpi/redox-rpi/issues/151)), Pi test agent. Stable bootloader, staged 12:15.

**Pass:**
- Part A: both forced restarts recovered, each with a single `restart 1`, and devsh answered again after 17 s and 24 s.
- Part B: **6 of 6 downloads completed**, so the stuck controller of 189/190 did not show and the restart was not needed during a download.

**Part A (forced restart while idle):** reset 12:15:55, at login 12:16:15.
- Restart 1, at 12:16:35. devsh was back after 17 s:
  ```
  dwc2d: restart 1 (up to 5 in a row without devices)
  dwc2d: controller stuck, restarting it: GINTSTS 0x56000029 HPRT0 0x1005 HFNUM 0x12cf1b05 GNPTXSTS 0x8080400 HPTXSTS 0x1f080400 HAINT 0x2 GRSTCTL 0x80000000; …
  dwc2d: controller restarted
  dwc2d: keyboard removed
  dwc2d: device on root port, High speed
  dwc2d: network on the adapter at address 5
  ```
- Restart 2, at 12:17:52. devsh was back after 24 s, with the same sequence (`GINTSTS 0x54000009 HPRT0 0x1005 …`).
- No `failed` and no `Timeout` line.
- `/tmp/dwc2d-usbstats` afterwards (over devsh): `restarts 2`, `halt_failures 0`, `timeouts 0`.
- In `/scheme/sys/context`, `dwc2d` and `netstack` are both `UB`, which is normal.

**Part B (the #189 download, 382156480 bytes):**

| Run | Start (UTC) | Result |
|---|---|---|
| 1 | 12:19:28 (reset) | **complete**, 378.5 s, 1.01 MB/s |
| 2 | 12:26:53 (power cycle) | **complete**, 293.1 s, 1.30 MB/s |
| 3 | 12:33:05 (power cycle) | **complete**, 261.5 s, 1.46 MB/s |
| 4 | 12:38:40 (power cycle) | **complete**, 296.3 s, 1.29 MB/s |
| 5 | 12:44:59 (power cycle) | **complete**, 361.2 s, 1.06 MB/s |
| 6 | 12:52:23 (power cycle) | **complete**, 267.9 s, 1.43 MB/s |

- No `restart`, `stuck`, `restarted`, `failed`, `removed` or `Timeout` line in any run.
- After run 6: `restarts 0`, `halt_failures 0`, `timeouts 0`.
- So a restart during a real stuck download is still untested on the Pi. It came 1 time in 6 runs in 189 and in 190.

Afterwards `rm /scheme/data/netget.bin`, then stable restored (`v0.3.0-dev-891-g55d0446+x9f76`), reset, and stable came up to login.

## 2026-11-01: test 193, v0.3.0-rc4 SD card with the user (`v0.3.0-dev-963-g15e885d+x12cb`)

Release checklist tests 6–9, one visit. Card written from
`redox-rpi-v0.3.0-rc4.img.xz` (SHA-256 `bd6e9f8f…f35c`), built with
`--release v0.3.0-rc4 --experiment timesync --sdcard`. Sources: the serial
log of the Pi 3B+ (15:18 UTC) and the user's report, relayed by the
coordinator. The Zero 2 W has no serial console here, so its results are
from the user alone.

| Test | Pi 3B+ | Zero 2 W | Detail |
|---|---|---|---|
| 6 SD card boot | **pass** | **pass** (user) | Serial banner `redox-rpi v0.3.0-rc4 (v0.3.0-dev-963-g15e885d+x12cb) (sdcard)`, then `redox login:`. The `dd`/`sha256sum` write check was not reported. |
| 7 HDMI, keyboard, `Super`+`F1`/`F2` | **pass** (user) | **pass** (user) | "Super F1/F2 hat funktioniert." |
| 8 USB hotplug | **pass** | **pass** (user) | The keyboard worked before and after it was unplugged and plugged in again (found at address 4 on the 3B+). The cable was plugged in after boot: link up on the LAN7800, `dhcpd` configured the address, `timesync` set the clock from `pool.ntp.org`. |
| 9 Zero 2 W with the RTL8153 hub | — | **pass** (user) | Boots from the same card, keyboard and `Super`+`F1`/`F2` work. Not confirmed from a log. |

Remarks:
- One undervoltage throttle on the 3B+ (ARM clock to 600 MHz) right
  before the cable was plugged in, then `no longer throttled`. That is the
  power supply, not a failure.
- `ifconfig` shows no MAC address, on both boards. To be checked; not a
  release blocker.
- There is no `ip` command. The visit steps wrongly named it; use
  `ifconfig` and the boot log.

Afterwards the netboot card goes back into the Pi 3B+; then row 192 (rc4
automated) can run.

## 2026-11-01: `v0.3.0-rc4` (`v0.3.0-dev-963-g15e885d+x12cb`), automated checklist 1–5, 11, 13, 14

Real Pi 3B+ over netboot, after the user's visit (#193). Test 192, Pi test agent.
Image `~/redox-rpi-rel/keep/v0.3.0-rc4-netboot.img`, staged 15:36 with the stable bootloader. The manifest matched at staging.

**Pass: all checks.**

- **(1) pi-test** `--starts 1000 --manifest netboot-v0.3.0-dev-963-g15e885d+x12cb.toml`:
  - `PASS version: v0.3.0-rc4 (v0.3.0-dev-963-g15e885d+x12cb)`;
  - manifest (build ID and `libc.so` hash), `ps` (51), `ps-tty`, SD card all pass;
  - **0 of 1000 starts failed**;
  - no panic, `FATAL` or `dwc2d` panic in any of the 6 boots (first boot plus 5 reboots).
- **(2) clock:**
  - every boot logs `rpi-firmwared: ARM clock 600 -> 1400 MHz`;
  - `throttled` is `0x00000000`, with no throttle or undervoltage line;
  - `clocks`: `arm 1400000000 1400000000`, `core 250000000 400000000`, `emmc 200000000`, `sdram 400000000 450000000`;
  - sdhost: `SDHC/SDXC card, 15193 MiB, 4-bit bus, high speed, up to 50 MHz`.
- **(3)** No `pcid`, `audiod` or `rtcd` line in any of the 6 boots.
- **(4) time:**
  - `timesync: clock set to 2026-11-01 15:37:08 UTC from pool.ntp.org`;
  - `date -u` on the Pi gave `Sun Nov 1 15:37:56 UTC 2026`, the same second as homenodellm.
- **(5)** `reboot` over devsh, 5 times: each came back to `redox login:` (20–22 s) and devsh answered (15:44:39 … 15:48:22).
- **(11)** `serial-input-test.sh 20` (logged in as root on the console): **20 of 20 lines complete**.
- **(13) governor:**
  - `mode ondemand`, `min 600000000`, `max 1400000000`;
  - during the 1000 starts, three `clocks` reads all gave `arm 1400000000`.
  - Idle needed a second boot. Staged 15:53 without the manifest, see the note below. I read the governor 2 min after login, then twice more, 30 s apart:
    - each read itself shows `target max` and `arm 1400000000`, because the devsh login loads the CPU (`load 55–100`);
    - but between the reads `time_min_ms` grew by 28062 and 29563 ms, and `time_max_ms` by only 4031 and 5674 ms;
    - in total, 130 s at 600 MHz against 12 s at 1400 MHz since boot.
  - So when idle it runs at 600 MHz, and it goes to 1400 MHz under load and while being looked at.
- **(14)** `nowire-check.py` (as test 99): **21 of 21 checks passed**.
  - GPIO 5 output reads back, GPIO 6 follows its pulls, the activity and power LEDs take writes, and the pins table has 54 rows.
  - I2C pins are in alt0; `i2c detect` finishes (`answering: none`); a missing address gives `No such device or address (os error 6)`.
  - SPI pins 7–11 are in alt0; the MISO pull reads `ff`/`00` at 1 and 10 MHz on cs0 and cs1; the 4096-byte transfer finishes.
  - `ls /scheme/gpio` lists `0` … `53` and `pins`.
- **Note:** `~/redox-rpi-rel/build/manifests/netboot-v0.3.0-dev-963-g15e885d+x12cb.toml` was rewritten at 15:42, after this test's staging. Since then, `stage.sh` says `MANIFEST MISMATCH` for the 15:11 image `v0.3.0-rc4-netboot.img`. The pi-test above ran against the manifest as it was at 15:36 and passed. The build agent should check which manifest belongs to the release image.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`), and stable came up to devsh at 15:58.

## 2026-11-01: `v0.3.0-dev-967-g311e46d+x9b05` (`ifconfig` shows the MAC address)

Real Pi 3B+ over netboot. Test 194 ([#171](https://github.com/redox-rpi/redox-rpi/issues/171)), Pi test agent. Image `ifcmac-v0.3.0-dev-967-g311e46d+x9b05.img` (branch `ifconfig-mac`), staged 16:07 with the stable bootloader. Network on the LAN7800 (`dwc2d: LAN7800 at address 4`).

**Pass:**
- `ifconfig` over devsh, rc 0:
  ```
  eth0:
  MAC Address: b8:27:eb:08:e0:aa
  IP Address: 192.168.178.148
  Netmask: 255.255.255.0
  ```
- `cat /scheme/netcfg/ifaces/eth0/mac` gives `b8-27-eb-08-e0-aa`, the same address. It is not `00:00:00:00:00:00`.
- Standard checks: version, manifest present, `ps` (45), `ps-tty`, SD card all pass; 0 of 300 starts failed.
- No panic or `FATAL` line.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`), and stable came up to devsh.

## 2026-11-01: `v0.3.0-3-g5ea80f3+x36c5` (kernel `0013` self-exit-status as a default patch)

Real Pi 3B+ over netboot. Test 195 ([#108](https://github.com/redox-rpi/redox-rpi/issues/108)), Pi test agent. Image `sx-v0.3.0-3-g5ea80f3+x36c5.img` (branch `t108-self-exit-default`), staged 16:11 with the stable bootloader.

**Pass:**
- **(1)** In `/usr/share/relibc-tests`, `./bins_dynamic/pthread/exit; echo rc=$?` 10 times: **10 of 10 `rc=0`**, 1.6 s in total.
- **(2)** `free` before and after 200 runs of `dd if=/dev/zero of=/dev/null bs=4k count=1` (all 200 printed `records out`, 32.3 s):
  - used memory went from 151084 to 141188 KB, so it **fell by 9.9 MB**, with no growth;
  - test 133 had +824 KB, and #31 was about 76 KB per run.
- **(3)** Standard checks: version, manifest present, `ps` (46), `ps-tty`, SD card all pass; 0 of 300 starts failed.
- No panic or `FATAL` line on serial.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`), and stable came up to devsh.

## 2026-11-01: `v0.4.0-dev-13-g16d3ba8+x4af4` (graphics PR #113 as it stands: DMA off, CPU blits)

Real Pi 3B+ over netboot. Test 148 ([#33](https://github.com/redox-rpi/redox-rpi/issues/33), [PR #113](https://github.com/redox-rpi/redox-rpi/pull/113)), Pi test agent. Image `netboot-v0.4.0-dev-13-g16d3ba8+x4af4.img`, staged 16:15 with the stable bootloader. Default boot, no typing.

**Pass: the boot does not hang.**
- Serial:
  ```
  rpi-fbd: EDID 256 bytes, monitor NanoKVM
  rpi-fbd: boot framebuffer 1824x984 stride 1824 at 0x3e914000
  rpi-fbd: blits with the CPU (DMA only with RPI_FB_DMA=on)
  fbcond: Performing handoff
  ```
- No `DMA test` or `DMA channel` line.
- Login at 16:16, about 40 s after the reset.
- No panic or `FATAL` line of any kind.
- `free`: total 737928, used 143528, free 594400 KB.
- pi-test (`--manifest`): version, manifest (build ID and `libc.so` hash), `ps` (52), `ps-tty`, SD card all pass; 0 of 300 starts failed.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`), and stable came up to devsh.

## 2026-11-01: `v0.4.0-dev-14-g735e3ea+xa731` (graphics, DMA on through the service file)

Real Pi 3B+ over netboot. Test 151 ([#33](https://github.com/redox-rpi/redox-rpi/issues/33), [PR #113](https://github.com/redox-rpi/redox-rpi/pull/113)), Pi test agent. Image `netboot-v0.4.0-dev-14-g735e3ea+xa731.img` (branch `rpi-fb-dma-test`, `rpi-fb-dma-on`), staged 16:31 with the stable bootloader. Plain netboot, no `cmdline.txt`, no typing.

**Partly: no freeze, but DMA was not used.** The 1-row and 64-row DMA blits went through; the 64-row one froze the Pi in #145. The whole-screen blit (984 rows) then gave a wrong row 0, so `rpi-fbd` fell back to the CPU, as designed.

`rpi-fbd` lines, in order:
```
rpi-fbd: EDID 256 bytes, monitor NanoKVM
rpi-fbd: boot framebuffer 1824x984 stride 1824 at 0x3e914000
rpi-fbd: DMA test buffer had 33 of 64 words not zero
rpi-fbd: DMA channel 5 at 0x3f007000, 2D mode moves YLENGTH+1 rows
rpi-fbd: DMA test: buffer of 1824x984
rpi-fbd: DMA test: 1 rows into the framebuffer
rpi-fbd: DMA test: 64 rows into the framebuffer
rpi-fbd: DMA test: 984 rows into the framebuffer
rpi-fbd: blits with the CPU: DMA test: row 0 wrong after a blit of 984 rows
fbcond: Performing handoff
```
- No `whole screen in N ms` line.
- Login reached about 40 s after the reset.
- No panic or `FATAL` line.
- `free`: total 737928, used 143640, free 594288 KB.
- pi-test (`--manifest`): version, manifest (build ID and `libc.so` hash), `ps` (52), `ps-tty`, SD card all pass; 0 of 300 starts failed.
- No power cycle needed.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`), and stable came up to devsh.

## 2026-11-01: `v0.4.0-dev-13-g41cfa03+x0fb0` (the Pi answers ping)

Real Pi 3B+ over netboot. Test 196 ([#177](https://github.com/redox-rpi/redox-rpi/issues/177)), Pi test agent. Image `icmp-v0.4.0-dev-13-g41cfa03+x0fb0.img` (branch `icmp-echo`), staged 16:35 with the stable bootloader. homenodellm has no `ping` binary, so I used `ping.py` (ICMP datagram socket, one request per second, 2 s timeout).

**Pass:**
- **Before, with stable 891:** 0 of 3 answered.
- **This image:** `ping.py 192.168.178.148 20` → **20 of 20 answered**, times 0.8–12.4 ms.
- **On the Pi:** `ping -c 3 192.168.178.1` → `3 packets transmitted, 3 packets received, 0.00% packet loss`, rtt 0.825/1.970/3.424 ms.
- **Standard checks:** version, manifest present, `ps` (45), `ps-tty`, SD card all pass; 0 of 300 starts failed. No panic or `FATAL` line.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`), and stable came up to devsh.

## 2026-11-01: `v0.4.0-dev-14-g52d9598+x424d` (`rwlock_randtest` with `sched-vtime-clamp`, no heartbeat)

Real Pi 3B+ over netboot. Test 197 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14), [#157](https://github.com/redox-rpi/redox-rpi/issues/157)), Pi test agent. Image `vcrw-v0.4.0-dev-14-g52d9598+x424d.img`, staged 16:38 with the stable bootloader. Runs as in 184: one devsh command per run, 60 s client timeout, stderr kept.

**Fail: the freeze is still there. 7 runs passed, then run 8 froze.** The fix of PR #168 does not cure it.

- **Runs 1–7:** all `rc=0`, `real` 1.25–1.38 s (median 1.33 s). No `RELIBC PANIC`, `brk`, `UNHANDLED EXCEPTION` or `FATAL` anywhere.
- **Run 8, started about 16:40:40:** no output, then the client timed out (16:41:46). devsh did not answer any more.
  - The only serial line since the run started: `[src/procmgr.rs:297 WARN] Cancellation for unknown id Id(4)`.
- **Not a whole-Pi freeze this time.** The serial console still worked:
  - I logged in as root over serial at about 16:53 and ran `cat /scheme/sys/context`;
  - `rwlock_randtest` was gone (exited);
  - all user contexts were `UB`/`US`, except the initfs `redoxfs` (pid 20), which was `UR` on CPU 2 and never running;
  - `dwc2d` was `US`, and `netstack` and the `devshd` threads were `UB`, all on CPU 2;
  - CPU 2's `[kmain]` showed `RR` without `+` while my `cat` ran there (`UR+`). The other three idle contexts were `RR+`.
  - `ping -c 2 192.168.178.1` on the serial console printed `PING …` and then hung for good. Ctrl-C did not stop it, so the network stack is stuck.
- `rpi-firmwared` still printed (`warning: undervoltage, throttled (flags 0x00050005 …)`, `no longer throttled`) when the plug was switched off.
- **Recovery failed:**
  - Shelly power cycles at 16:43:39 (12 s off), 16:49:47 (12 s off) and 16:58:39 (40 s off, the plug read `apower 0.0`). Each time the Pi only logged a brief undervoltage, kept running the frozen image, and never rebooted.
  - **Since the user's visit, the Pi gets power from another source**, probably the HDMI or USB cables to the NanoKVM or the keyboard. Remote recovery is not possible until that is fixed.
  - Stable is staged in `/srv/tftp/redox`.
- Not run: standard checks.

Stable image staged afterwards (`v0.3.0-dev-891-g55d0446+x9f76`). **The Pi is still running the frozen test image and needs a person.**

## 2026-11-01: `v0.4.0-dev-15-geaa48df+xa731` (graphics, kernel 0050 DMA cache fix)

Real Pi 3B+ over netboot. Test 198 ([#33](https://github.com/redox-rpi/redox-rpi/issues/33), [PR #113](https://github.com/redox-rpi/redox-rpi/pull/113)), Pi test agent. Image `~/redox-rpi-graphics/keep/netboot-v0.4.0-dev-15-geaa48df+xa731.img` (manifest matches), staged with the stable bootloader. Plain netboot, no `cmdline.txt`. Two boots: a devsh reset, then a Shelly power cycle.

**Pass for the DMA fix: no `not zero` line, whole-screen DMA blit works. Boot 1 lost its network after the checks; boot 2 did not.**

`rpi-fbd` lines, the same on both boots:
```
rpi-fbd: DMA channel 5 at 0x3f007000, 2D mode moves YLENGTH+1 rows
rpi-fbd: DMA test: buffer of 1824x984
rpi-fbd: DMA test: 1 rows into the framebuffer
rpi-fbd: DMA test: 64 rows into the framebuffer
rpi-fbd: DMA test: 984 rows into the framebuffer
rpi-fbd: DMA test: whole screen in 29.8 ms      (boot 2: 20.5 ms)
fbcond: Performing handoff
```
- No `DMA test buffer had N of 64 words not zero` line (#151: 33). No `blits with the CPU` line. No panic.
- pi-test (`--manifest`) passes on both boots: version, manifest, `ps` (52), `ps-tty`, SD card; 0 of 300 starts failed.
- **Boot 1:** right after pi-test, devsh and ping from the host stopped (`No route to host`), and `free` could not run. The serial console still worked: `dwc2d` sleeping (`US`, CPU time stuck at 1.71 s), `netstack` blocked, nothing busy. On the Pi, `ifconfig` printed nothing and `ping -c 2 192.168.178.120` never returned. No USB or network error in the serial log. This image has no `sched-vtime-clamp` and no dwc2d stats files, so I can't tell whether this is the known netstack/USB stall (#188, #189) or 0050. In #151 (same image without 0050) the network stayed up after pi-test.
- **Boot 2** (power cycle): pi-test passes, then `free` (used 159428 KB), `ping -c 3` 3 of 3, then three runs of `dd if=/dev/zero bs=65536 count=800` over devsh (52 MB each, about 200 s each; the client only got dd's status lines), then `ping -c 20` 20 of 20, `free` used 167876 KB. 35 `Dropped packet on eth0 because waiting queue was full` warnings at 19:46 during pi-test. The image has no `netget`, so no large download.
- Shelly check before the test: plug off gave 0 W, no ping, then a fresh U-Boot boot, so power cycles work again.

Stable image restored afterwards (`v0.3.0-dev-891-g55d0446+x9f76`).

## 2026-11-01: `v0.4.0-dev-15-gac0e81c+x58dc` (relibc tests, `sched-vtime-clamp`, `pi-heartbeat`)

Real Pi 3B+ over netboot. Test 199 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14) `rwlock_randtest` freeze with the heartbeat), Pi test agent. Image `~/redox-rpi-icache/keep/vcrwhb-v0.4.0-dev-15-gac0e81c+x58dc.img`, staged 19:57 with the stable bootloader. Boot line: `experiments: pi-heartbeat relibc-tests sched-vtime-clamp timesync`.

**Fail: froze at run 2, a hard freeze. No heartbeat line came at all, and the serial console died too.**

- Run 1: `rc=0`, `real 1.378597498s`.
- Run 2 started at about 19:58:40, about one minute after the kernel started. The client timed out at 19:59:44. devsh was dead and ping got no answer.
- I waited 6 minutes without a reset, as the row asks. The serial log has **no** `[hb …]` line, no `[hb stall …]`/`[hb nosched …]`, and no dump, not even from before the freeze. The last serial output is the `redox login:` prompt at about 19:58:25. Nothing came after that.
- Serial console: Enter and `root` (one character every 0.15 s) got no echo, 0 new bytes. So I could not run `cat /scheme/sys/context`. In #197 the serial console still answered after the freeze.
- The first beat comes one minute after the kernel starts, so the freeze probably came before any beat. After it, the kernel printed nothing more, not even the stall check that should fire after 2 s.

Stable image restored, then a Shelly power cycle (20:05). Stable `v0.3.0-dev-891-g55d0446+x9f76` came up to devsh. The power cycle worked; the plug resets the Pi again.

## 2026-11-01: `v0.4.0-dev-15-geaa48df+xa731` (graphics, kernel 0050 DMA cache fix, rerun)

Real Pi 3B+ over netboot. Test 198 rerun ([#33](https://github.com/redox-rpi/redox-rpi/issues/33), [PR #113](https://github.com/redox-rpi/redox-rpi/pull/113)), Pi test agent. The first run (report above) was interrupted: the user was rearranging the Pi's power supply, and the serial log shows a reboot at about 19:41 that no test caused. Boot 1's network loss in that report happened during that work, so it may be a power problem. Same image (manifest matches), staged 20:08, devsh reset.

**Pass: DMA fix works, standard checks pass, network stable for 12 minutes of load.**

- `rpi-fbd`: `DMA test:` 1, 64 and 984 rows, then `whole screen in 29.8 ms`. No `not zero` line, no `blits with the CPU`. `fbcond: Performing handoff`, login, no panic.
- pi-test (`--manifest`): version, manifest, `ps` (52), `ps-tty`, SD card pass; 0 of 300 starts failed.
- `free`: used 159428 KB; `ping -c 3` 3 of 3.
- Three runs of `dd if=/dev/zero bs=65536 count=800` over devsh, 52 MB each, 159–167 s each (about 320 kB/s), all `rc=0`.
- Then `ping -c 20` 20 of 20, `free` used 166028 KB, `throttled` `0x00000000`.
- 34 `Dropped packet on eth0 because waiting queue was full` warnings (also in the first run). No other errors.
- No unplanned reboot (one U-Boot, from the reset).

Stable image restored afterwards and came up to devsh.

## 2026-11-01: `v0.4.0-dev-17-g8ca190f+x58dc` (relibc tests, `sched-vtime-clamp`, `pi-heartbeat` with kernel spin watches)

Real Pi 3B+ over netboot. Test 200 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14) `rwlock_randtest` hard freeze: which CPU spins where?), Pi test agent. Image `~/redox-rpi-icache/keep/spin-v0.4.0-dev-17-g8ca190f+x58dc.img`, staged 20:22 with the stable bootloader. Boot line: `experiments: pi-heartbeat relibc-tests sched-vtime-clamp timesync`.

**Fail: froze at run 54. No `[spin …]` line at all. The kernel kept beating on all 4 CPUs, and nothing was runnable.**

- Runs 1–53: `rc=0` (run 53: `real 1.348210692s`).
- Run 54 started at about 20:30:47 and the client timed out at 20:31:52. devsh was dead and stayed dead after 5 min.
- After 5 min, Enter on the serial console gave a fresh `redox login:` prompt. So getty and the serial driver were still working (unlike 199). I did not log in.
- **`[spin …]` lines: none**, in the whole boot. There were also no `[hb stall …]`, `[hb nosched …]`, `few-switches` or `runnable-on-idle-cpu` lines.
- Beats from run 54 on (`[hb <cpu> <beat> s… w… i… p…]`):
  - beat 8 (run 54 running): CPU 0 `w735`, CPU 1 `w5134`, CPU 2 `w1456`, CPU 3 `w2810`.
  - beats 9–14 (after the freeze): CPUs 0, 1 and 2 `w0` (beat 14: CPU 1 `w100`), CPU 3 about `w2200` every beat. `s` ≈ `i` + `w` on every CPU, so every CPU still takes timer interrupts and runs `schedule()`.
- Dumps (`timed`, beats 8, 10, 12, 14):
  - `[hbq n] … n0` on every CPU: all run queues are empty. Only `hbq 3`'s `v` keeps growing (140.1e9 → 147.3e9 → 157.2e9 → 164.4e9); `hbq 0–2` stay unchanged from beat 8 on.
  - The only `R` contexts are the four `[kmain]` idle contexts (`R+c cpu0`–`cpu3`).
  - **No `rwlock_randtest` context in any dump**, already in the beat 8 dump while run 54 was still running. Its shell, `ion` (417), is `B sys cpu3 UserInner::call`.
  - Blocked in `UserInner::call` (waiting for an answer from a scheme): `init` (1), `netstack` (25), `devshd` (34, several threads), `login` (31, 36, 416), `ion` (417). `dwc2d` is `S nanosleep`, and the rest wait in `UserInner::read`/`EventQueue::read` as usual.
  - The `hbcpu` counters `sl sa sr sv ss` don't change from beat 8 to 14.
- Serial log from the start of run 54 to the power cycle: 269 lines, all beats and dumps.

Shelly power cycle at 20:37 after the 5 min wait, then stable restored. Stable `v0.3.0-dev-891-g55d0446+x9f76` came up to devsh.

## 2026-11-01: `v0.4.0-dev-19-g73763d6+x7f75` (relibc tests, `sched-vtime-clamp`, `pi-heartbeat`, `dwc2d-netstats`, `usb-hub-drop`)

Real Pi 3B+ over netboot. Test 201 ([#14](https://github.com/redox-rpi/redox-rpi/issues/14) `rwlock_randtest` freeze, now without the dwc2d hang), Pi test agent. Image `~/redox-rpi-icache/keep/hubdrop-v0.4.0-dev-19-g73763d6+x7f75.img`, staged 20:55 with the stable bootloader. Boot line: `experiments: dwc2d-netstats pi-heartbeat relibc-tests sched-vtime-clamp timesync usb-hub-drop`.

**Pass: 120 of 120 runs, no freeze.**

- 120 single runs of `./bins_dynamic/pthread/rwlock_randtest` over devsh, all `rc=0`. `real` average 1.34 s, max 1.43 s.
- Whole serial log of the boot: no `[spin …]` line, no `[hb stall …]`, `[hb nosched …]`, `few-switches` or `runnable-on-idle-cpu` line, and no panic. Beats came through beat 12.
- For comparison: 197 froze at run 8, 199 at run 2, 200 at run 54 (dwc2d's sleep loop). 186 also passed 80 of 80 without a fix, so one clean series is good but not proof.
- No power cycle needed.

Stable image restored afterwards (devsh reset). Stable `v0.3.0-dev-891-g55d0446+x9f76` came up to devsh.

## 2026-11-02: `v0.4.0-dev-35-gc13ceff+x9f76`

Built 2026-11-02T03:30:26Z, `libc.so` `a4ffbf84bf004b93…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Nightly stable image test (scripts/update-stable.sh).

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.4.0-dev-35-gc13ceff+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.4.0-dev-35-gc13ceff+x9f76.toml |
| ps | pass | 46 processes (piped) |
| ps-tty | pass | 46 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## 2026-11-02: `v0.4.0-dev-60-ga206e99+x6bdd` (graphics screen visit, SD card)

Real Pi 3B+, user at the HDMI monitor. Test 202 ([#33](https://github.com/redox-rpi/redox-rpi/issues/33), in place of #34, #43, #44, #54), Pi test agent on the serial console. The user booted the SD card image of the same build (`redox-rpi-graphics-visit-v0.4.0-dev-60-ga206e99.img.xz`), not netboot: `redox-rpi v0.4.0-dev-60-ga206e99+x6bdd (sdcard)`, `experiments: orbital rpi-fb rpi-fb-dma-on timesync`.

**Pass: screen, DMA blits, Orbital and keyboard work. No mouse driver in this image (as expected).**

- `rpi-fbd: EDID 256 bytes, monitor ANMITE`, `boot framebuffer 1920x1200 stride 1920`, `DMA test:` 1, 64 and 1200 rows, `whole screen in 49.3 ms`; no `blits with the CPU`.
- `fbcond: Performing handoff`, login, no panic.
- USB: keyboard `04d9:0006` at address 5 works. Mouse `093a:2510` (low-speed HID boot mouse) enumerated, but no driver.
- `free` (serial console, the SD image has no devshd): used 298172 of 965316 KB. `throttled`: `0x00050000` (undervoltage and throttling since boot; the user was rearranging the power supply that day).
- User at the screen: boots fast; Orbital desktop with correct colours (blue) and the taskbar. Misses the mouse, so test 203 followed with the mouse image.

Ended when the Pi was rebooted over netboot for 203.

## 2026-11-02: row 56 (USB mouse, #48), covered by test 203

Test 56 ([#48](https://github.com/redox-rpi/redox-rpi/issues/48) USB mouse in `dwc2d`) was not run on its own image. Test 203 (screen visit with the mouse, `v0.4.0-dev-67-g47859bc+x227e`, experiment `usb-mouse`) covers its main point.

**Pass (covered by 203).**

- Serial: `dwc2d: mouse at address 3: relative, 3 button(s), wheel` for the user's wired mouse `093a:2510`. The keyboard (`keyboard at address 5`) works at the same time.
- User at the screen, in Orbital: the pointer moves, clicks work, a terminal opens.
- Not checked in 203: the `/tmp/dwc2d-input-log` lines (move, each button, wheel), unplugging and plugging the mouse in again, and `echo mouse > /tmp/dwc2d-cycle`.

## 2026-11-02: `v0.4.0-dev-67-g47859bc+x227e` (graphics screen visit with USB mouse)

Real Pi 3B+ over netboot, user at the HDMI monitor with USB keyboard and mouse. Test 203 ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)), Pi test agent. Image `~/redox-rpi-graphics/keep/netboot-v0.4.0-dev-67-g47859bc+x227e.img` (manifest matches), staged with the stable bootloader, Shelly power cycle 17:11. `experiments: orbital rpi-fb rpi-fb-dma-on timesync usb-mouse`.

**Pass: mouse, keyboard, screen and Orbital work; dragging windows is slow.**

- `rpi-fbd: EDID 256 bytes, monitor ANMITE`, `boot framebuffer 1920x1200 stride 1920`, `DMA test:` 1, 64 and 1200 rows, `whole screen in 49.6 ms`; no `blits with the CPU`.
- `dwc2d: mouse at address 3: relative, 3 button(s), wheel` (`093a:2510`), `dwc2d: keyboard at address 5` (`04d9:0006`).
- `fbcond: Performing handoff`, login 32 s after power-on, no panic.
- devsh: `free` used 331068 of 737908 KB; `throttled` `0x00000000`.
- User at the screen: the mouse pointer and clicks work, a terminal opens. Dragging windows is slow (with the background picture and the transparent terminal).

Stable image restored afterwards (`v0.4.0-dev-35-gc13ceff+x9f76`, the nightly of 2026-11-02), Shelly power cycle 20:37, stable came up to devsh.

## 2026-11-02: `v0.4.0-dev-35-gc6faab6d+x95e1` (direct firmware boot, card)

Real Pi 3B+, user at the Pi, card `redox-rpi-t18-direct-v0.4.0-dev-35-gc6faab6d+x95e1.img.xz` flashed and put in by the user. Test 18 ([#15](https://github.com/redox-rpi/redox-rpi/issues/15) direct firmware boot without U-Boot), Pi test agent on the serial console. Two boots: the user's, then a Shelly power cycle at 20:46.

**Partly: the direct boot works (stub, framebuffer, kernel, login, board MAC, card partitions), but USB does not come up, so there is no network, no keyboard and no pi-test.**

- No U-Boot. `rpi-boot-stub:` lines: `devicetree at 0x1f972100`, `initramfs 0x1f97b000..0x2f000000, 252436 KiB`, `framebuffer 1920x1200 stride 1920 at 0x3e731000`, `live disk 0x21200000, 222 MiB`, `starting the kernel at 0xffffff00000a2678`.
- Banner `redox-rpi v0.4.0-dev-35-gc6faab6d+x95e1 (netboot, 2026-11-01T16:00:24Z)` (it says netboot on a card boot), `experiments: redox-fatfs2 timesync`, `redox login:` 28 s after power-on, no panic.
- `dwc2d: network address b8:27:eb:08:e0:aa`: the board MAC, not the fallback.
- **USB fails on both boots:** `dwc2d: device on root port, Full speed`, then `request [80, 06, 00, 01, …] to address 0 (max packet 8, Full speed) failed: Timeout("channel halt")`, two retries, `enumeration failed: Timeout("channel halt")`. With U-Boot (netboot) the same Pi sees the on-board hub at high speed and finds the LAN7800, keyboard and mouse. So no LAN7800: `dhcpd: failed to receive offer: Connection timed out`, no devsh, and pi-test could not run. The USB keyboard did not work either.
- Serial console (root): `ls /scheme/data` shows `README.txt  timesync.last` (data partition from the card). `fatfsd /scheme/disk.mmc/0p0 boot &` gives `Fat32, label "REDOXBOOT"`, `serving /scheme/boot`, and `ls /scheme/boot` shows `config.txt`, `kernel8.img`, `initramfs`, `start.elf`, `start_cd.elf`, `bootcode.bin`, the dtbs and `overlays`. `throttled` `0x00000000`. `free`: used 145712 of 765548 KB.
- Likely cause: without U-Boot nobody resets the on-board USB hub/LAN chip or powers it up the way U-Boot's `usb start` does, so the root port sees a full-speed device that does not answer. That needs a look in `dwc2d` or the stub.

Afterwards the netboot card goes back in and stable is restored.

## 2026-11-03: `v0.4.0-dev-64-g954be33+x9f76`

Built 2026-11-03T03:30:25Z, `libc.so` `a4ffbf84bf004b93…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Nightly stable image test (scripts/update-stable.sh).

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.4.0-dev-64-g954be33+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.4.0-dev-64-g954be33+x9f76.toml |
| ps | pass | 46 processes (piped) |
| ps-tty | pass | 46 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## #205: direct firmware boot with the USB fix, plus new power supply (2026-11-03, Pi test agent)

Card `redox-rpi-t18-direct-v0.4.0-dev-36-g1107e5f7+x95e1` (branch `fw-direct-boot`),
real Pi 3B+, booted at 16:29 UTC on the new power supply. **Pass**. The user reports
that the USB keyboard works at the login prompt.

**Boot, serial log**
- `rpi-boot-stub: USB power on (was Some(0))`.
- `dwc2d: device on root port, High speed` (row 18 had Full speed).
- Hubs 0424:2514 ×2 and 05e3:0610 found.
- Keyboard 04d9:0006 at address 4 (`dwc2d: keyboard at address 4`).
- LAN7800 at address 6, link up, `network address b8:27:eb:08:e0:aa`.
- No `channel halt` or timeout lines.
- `redox login:`.
- Small thing: the banner says `(netboot, 2026-11-02T20:49:36Z)` on a direct-boot image.

**Checks over devsh**
- pi-test: version, manifest (build ID and libc.so hash), ps (57), ps-tty, sdcard,
  starts 0/300, all PASS.
- `/scheme/data` from the card: shows `README.txt` and `timesync.last`. Writing,
  reading back and deleting a file work.
- `fatfsd /scheme/disk.mmc/0p0 boot &` then `ls /scheme/boot` shows `config.txt`,
  `kernel8.img`, `initramfs`, the firmware and the dtbs. `config.txt` reads correctly.
- Ping to the server: 5/5 at idle.

**New power supply**
- With the old supply, row 202 showed `throttled 0x50000`.
- `throttled` read 0x00000000 at every reading:
  - idle (53.7 °C);
  - after pi-test;
  - 10 readings during 5 min with dd on all 4 cores (1.0 GB/s each), peak 55.8 °C,
    plug 4.4 W;
  - 3 readings during 4 cores plus 3×50 MB of output over devsh (321 kB/s, the same
    as netboot in row 198);
  - after the load.
- Earlier the same afternoon, stable dev-64 on the netboot card (16:18 UTC) also
  read 0x0 at idle, after pi-test (pass) and under 2-core load.
- The image has no `netget`, so the download test was replaced by output over devsh.

**Seen under combined load**
- netstack logged 12 `Dropped packet on eth0 because waiting queue was full`.
- One ping out of 5 was lost right after the load.
- devsh stayed up and no reboot happened.

## New power supply (2026-11-03, Pi test agent)

The user put a new power supply on the Pi. The old one showed `throttled 0x50000`
in row 202 (under-voltage had happened).

- Stable `v0.4.0-dev-64-g954be33+x9f76` on the netboot card booted at 16:18 UTC and
  devsh answers. `throttled` 0x0 at idle (44 °C), after pi-test (all pass) and during
  2-core dd load (6 readings, peak 50 °C, plug 3.2 W).
- On the #205 direct-boot card (dev-36-g1107e5f7): `throttled` 0x0 at idle, after
  pi-test, at 10 readings during 5 min of dd on all 4 cores (peak 55.8 °C, plug 4.4 W),
  at 3 readings during 4 cores plus 3×50 MB of network output over devsh, and after
  the load.
- The image has no `netget`, so the download was replaced by output over devsh.
- **Result:** no under-voltage with the new supply. Details in the #205 report above.

## #206: current network-boot image on the Pi Zero 2 W (2026-11-03, Pi test agent)

Image `v0.4.0-dev-135-g2416130+x9f76` (`build-image.sh --zero2w`, main 2416130,
experiments `timesync` and `rtl8152`, no graphics). It replaces the alpha-90 image
of 2026-10-27 in `/srv/tftp/redox-zero2w`; the old files are backed up on
homenodellm in `~/pitest-tools/tftp-zero2w-alpha90`. The Zero has the PoE HAT
(RTL8152 `00:e0:4c:36:0e:47`, 192.168.178.194) and its old netboot card. The
person restarted it by unplugging the PoE plug. **Pass**, except `/scheme/data`
(this card has no data partition).

- Boot: devsh on .194 answered at 18:36 UTC, uptime 1 min 38 s. The old image
  showed a login prompt on devsh; this one opens the shell as on the Pi 3B+.
- `/etc/redox-rpi-release`: `v0.4.0-dev-135-g2416130`, `IMAGE="zero2w"`.
- pi-test:
  - PASS: version, manifest (build ID and libc.so hash), ps (45), ps-tty,
    starts (0 of 300 failed).
  - FAIL: sdcard. `/scheme/data` is missing (`No such device`). The card has only
    the boot partition (`/scheme/disk.mmc/0p0`, 32 MB, card 62 MB), and
    `60_data.service` wants `0p1`. That is the card, not the image. A card from
    `scripts/make-sdcard.sh` with a data partition would fix it.
- Network: ping to the server 5/5, 1.3-1.7 ms. Time from NTP is correct.
- Free memory: 191068 of 333196 KB free after boot (142 MB used), 46 processes.
- `throttled` 0x0, 53.7 °C.

### #206 second boot: new netboot card with a data partition (2026-11-03)

The user flashed a 64 MB card with
`redox-rpi-zero2w-netboot-v0.4.0-dev-135-g2416130-b32-d24.img.xz`
(`make-sdcard.sh --dir redox-zero2w --boot-size 32 --data-size 24`, 56 MiB:
FAT16 boot partition of 31 MiB and a RedoxFS data partition of 24 MiB) and booted the
Zero from it. Same image `v0.4.0-dev-135-g2416130`. **Pass.**

- pi-test: version, manifest, ps (46), ps-tty, **sdcard**, starts (0 of 300 failed),
  all PASS.
- `/scheme/data` is mounted from `/scheme/disk.mmc/0p1`. It shows `README.txt` and
  `timesync.last` (timesync writes it after boot).
- Writing, reading back and deleting a small file work.
- A 10 MiB file of zeros was written at 385 kB/s and deleted again.
- `df /scheme/data`: 24576 KB, 23512 KB free before and 23480 KB after.
  - The 10 MiB of zeros added only 40 KB, so RedoxFS seems to store zero blocks
    without taking space. 23 MB is what real data can use.
- Free memory 186920 of 333068 KB, `throttled` 0x0.

## #207: sshd ends an idle connection after the login grace time (2026-11-03, Pi test agent)

Image `v0.4.0-dev-195-g28be1ddd+x4ec1` (test branch `tcp-close-test` = PR #185 with
PR #90, experiments timesync, rtl8152, sshd). Real Pi 3B+ over netboot, reset at
19:38 UTC. The boot reached login with no `FATAL` or panic line. **Pass.** Fixes the
failed grace-time step of #182 ([#184](https://github.com/redox-rpi/redox-rpi/issues/184)).

**Login grace time**
- The host keys were in `/etc/ssh` at the first look (19:38:56).
- Each try opened a TCP connection from homenodellm to port 22, read the banner
  `SSH-2.0-OpenSSH_10.5`, and sent nothing.
  - Try 1: **the Pi closed the connection after 125 s** (FIN, `recv` returned empty).
    Serial: `Timeout before authentication for connection from 192.168.178.120 to
    192.168.178.148, pid = 60`, at the same time.
  - Try 2: closed after **124 s**, with the same line (`pid = 62`).
  - That matches QEMU (122-127 s). In #182 the connection stayed open until the
    client closed it.

**Other #182 steps**
- Test key into `/home/root/.ssh/authorized_keys` over devsh, in single quotes:
  `wc -c` 100.
- 5 key logins `ssh -i /tmp/qkey root@192.168.178.148 'echo loginN'`: login1 to
  login5, all rc 0.
- `ssh-session.exp` (port 22, Pi address): prompt, command, prompt-again, backspace,
  prompt-again and exit all `ok`, rc 0.
- 40 devsh connections in a row: 40/40.
- pi-test (`--manifest --starts 300 --serial-log`): version, manifest, ps (53), ps-tty
  and sdcard pass, and 0 of 300 starts failed.
- Serial: the only TODO line is `sshd-session: TODO: ioctl TIOCSCTTY` (known).

Stable `v0.4.0-dev-64-g954be33+x9f76` was restored afterwards (19:46) and devsh answers.

## #134: U-Boot without countdown, firmware boot_delay=0, U-Boot settings on the card (2026-11-03, Pi test agent)

Card variant. The person copied the new `u-boot.bin` and `config.txt` from
`redox-rpi-t134-netboot-bootfiles.zip` onto the network-boot card (old files kept as
`.orig`). The Pi booted at about 20:45 UTC. Stable image `v0.4.0-dev-64-g954be33+x9f76`.
**Pass.** One expected gain did not show: the U-Boot banner comes no earlier.

**First boot (person, 20:45)**
- `U-Boot 2026.10 (Oct 31 2026 - 15:23:39 +0000)`.
- `Loading Environment from FAT...Unable to read "uboot.env" from mmc2:1...`. The
  old `Bad device specification mmc 0` is gone.
- `Hit any key to stop autoboot: 0` with no countdown.
- It reached `redox login:`.

**Three timed boots** (`boot-timeline.py`, seconds after the devsh reset; A = 2026-10-31,
old card files, image dev-533, server on Wi-Fi)

| Step | A1 | N1 | N2 | N3 |
|---|---|---|---|---|
| U-Boot banner | 3.76 | 3.78 | 3.76 | 3.76 |
| autoboot starts (`Hit any key … 0`) | 9.27 | 7.25 | 7.21 | 7.22 |
| DHCP bound | 11.22 | 10.43 | 10.37 | 10.37 |
| image loaded over HTTP | 15.03 | 12.38 | 12.38 | 12.38 |
| `Redox OS starting` | 23.44 | | 20.44 (N2) | |
| `redox login:` | 33.5 | 29.62 | 29.31 | 29.32 |

- **U-Boot countdown gone:** autoboot starts about 2 s earlier, and DHCP is bound
  0.85 s earlier.
- **`boot_delay=0`/`disable_splash=1`:** no effect seen. The U-Boot banner comes at
  3.76 s as in A. (These are devsh resets, as in A.)
- **Login 4.2 s earlier,** but only part of it comes from this change:
  - the download took 2.0 s instead of 3.7 s, because homenodellm is now on cable;
  - the image is newer (kernel start to login 8.9 s instead of 10.1 s).
- pi-test (`--manifest`): version, manifest, ps (51), ps-tty, sdcard and 300 starts
  (0 failed) all pass.

**Saving settings from the network** (stable's `netboot.cmd` matched, so the env
test ran)
- Boot E1: `Saving Environment to FAT... OK` and `redox-rpi: saved redoxtest`, then
  login.
- Boot E2: `Loading Environment from FAT... OK` and
  `redox-rpi: uboot.env has redoxtest=kept`, then login.
- Stable's `netboot.scr` was put back.
- Boot E3: `Loading Environment from FAT... OK` (the card keeps `uboot.env`),
  normal boot, login at 29.82 s.
- `/srv/tftp/redox` equals the stable backup again, and devsh answers. The new files
  stay on the card.

## 2026-11-04: `v0.4.0-dev-201-gb714bca+x9f76`

Built 2026-11-04T03:30:26Z, `libc.so` `a4ffbf84bf004b93…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Nightly stable image test (scripts/update-stable.sh).

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.4.0-dev-201-gb714bca+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.4.0-dev-201-gb714bca+x9f76.toml |
| ps | pass | 46 processes (piped) |
| ps-tty | pass | 46 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## #208: sshd with OpenSSH 10.6p1 (2026-11-04, Pi test agent)

Image `v0.4.0-dev-250-gbabedfd0+x4ec1` (branch `openssh-10.6`, draft PR #191; experiments
timesync, rtl8152, sshd). Real Pi 3B+ over netboot, reset at 06:53 UTC. The boot
reached login with no `FATAL` or panic line. **Pass.**

- Host keys were in `/etc/ssh` at the first look (06:53:49).
- Banner from homenodellm: `SSH-2.0-OpenSSH_10.6`.
- Test key into `/home/root/.ssh/authorized_keys` and `/home/user/.ssh/authorized_keys`
  over devsh, in single quotes: `wc -c` 100 for both.
  - `chown -R user:user` was rejected (`failed to parse uid user`: Redox `chown` wants
    numbers), so the user's key file belongs to 0:0. The `user` login worked anyway.
- 5 key logins as root: login1 to login5, all rc 0.
- `ssh-session.exp` (port 22, Pi address): prompt, command, prompt-again, backspace,
  prompt-again and exit all `ok`, rc 0.
- **Privilege separation:** `ssh -tt -i /tmp/qkey user@… 'id; sleep 25'` printed
  `uid=1001(user) gid=1001(user)`, rc 0. `cat /scheme/sys/context` during the session:
  - `sshd` (pid 53) and the monitor `sshd-session` (pid 113) as EUID 0;
  - the session `sshd-session` (pid 115), `ion` (116) and `sleep` (118) as
    **EUID/EGID 1001**;
  - the other `ion` as 0 is the devsh shell running `ps`.
- Idle connection to port 22 (banner read, nothing sent): **closed by the Pi after
  124 s**, with `Timeout before authentication … pid = 62` on serial.
- 40 devsh connections in a row: 40/40.
- pi-test (`--manifest --starts 300 --serial-log`): version, manifest, ps (53), ps-tty,
  sdcard pass, and 0 of 300 starts failed.
- Serial: the only TODO line is `sshd-session: TODO: ioctl TIOCSCTTY` (2×, known).

Stable is now the nightly `v0.4.0-dev-201-gb714bca+x9f76` (2026-11-04 03:30). It was
restored afterwards (07:00), and devsh answers.

## #209: package server end to end (2026-11-04, Pi test agent)

Image `v0.4.0-dev-269-gd3ebd24c+x42ad` (branch `pkg-persist` d3ebd24, PR #106), staged
with `netboot.sh` from `~/redox-rpi-boot/keep/t44p-v1.img` (bootloader md5 `64685a6f…`),
the usual network-boot card. **Partly: boot 1 and the reset pass, but in boot 2 the
Pi loses its network, so step 6 (`pkg update` replaces files) could not be done.**

**Boot 1** (reset 07:27:42 UTC)
- (1) pi-test `--starts 300`: version, manifest, ps (51), ps-tty, sdcard pass, and 0
  of 300 starts failed. `date`: 2026-11-04. `/scheme/data/pkg` did not exist.
- (2) `pkg-t44` into `/etc/pkg.d/40_redox-rpi`, then `pkg update --yes`: `Commit
  complete.` (it updated `uutils-procps`). Took **17 s**.
- (3) `pkg-persist install redox-fatfs2`: `pkg-persist: 1 files copied into /, 0 kept
  from the image (161 ms)`, **2 s** in all.
- (3) `pkg-persist install nano`: `pkg-persist: 214 files copied into /, 2 kept from
  the image (3895 ms)`, **50 s** in all.
- `pkg info redox-fatfs2`: `remote: 192.168.178.120` (version 0.1.0).
- `pkg info nano`: `remote: static.redox-os.org` (9.2).
  - Both show `status: not installed`, which looks odd after the install.
- `pkg-persist list`: nano, ncursesw, redox-fatfs2, terminfo.
- (4) `reboot` at 07:30:26.

**Boot 2** (after `reboot`)
- Banner after 18 s, login at 07:30:44.
- (5) Right after the login prompt: `pkg-persist: 215 files copied into /, 2 kept from
  the image (4092 ms)`.
- `timesync` set the clock from pool.ntp.org at 07:30:48 and `devshd: listening`, so
  the network worked at first.
- But homenodellm got `No route to host` (no ARP answer) on every devsh try for
  6 minutes. Steps 5-7 got no answer, and the devsh reset for the restore did not get
  through either: power cycle at 07:41.

**Boot 2 again** (stable's devsh reset at 07:43:55, packages still on the card)
- Login at 07:44:15, `pkg-persist: 215 files copied into /, 2 kept from the image
  (4085 ms)`, NTP at 07:44:18.
- devsh answered at 07:44:20.
- (5) `nano --version`: `GNU nano, version 9.2`. `ls -l /usr/bin/fatfsd` exists.
  `pkg-persist list` shows the four packages, so **the packages survived the reset**.
- (6) `pkg-persist` without arguments: `usage: pkg-persist install|remove <package>... |
  update | list | restore`.
- (6) `pkg-t44-v2` into `40_redox-rpi`, then `pkg update --yes`: it started downloading
  `uutils.pkgar` (3.07 MiB) and stopped at 149 KiB. From then on the Pi did not answer
  on the network again (`No route to host`), and power cycle at 08:02.
- The serial log has no error, panic, exception, USB removal or `channel halt` line
  after login in either boot 2.

**Result**
- Steps 1-4 pass.
- Step 5 passes: copy line after login, nano 9.2 and fatfsd after the reset.
- Step 6 fails: in both boots with persisted packages, the network dies within the
  first minutes, the second time during the `pkg update` download. Boot 1 (no
  persisted packages) had the network for its whole 3 min, downloads included.
- I cannot tell from this whether the persisted files or the boot are the cause.
- (7) `rm -r /scheme/data/pkg` was done afterwards from stable.

Stable `v0.4.0-dev-201-gb714bca+x9f76` restored (power cycle 08:02); devsh answers.

## #210: multicast and the LAN7800 multicast filter as default patches, with IPv6 (2026-11-04, Pi test agent)

Image `v0.4.0-dev-292-ge95959aa+xfa87` (branch `multicast-default` on PR #82, base
`0083`, `0084`, relibc `0016`; experiments `multicast` (only the `mcast` tool) and
`ipv6`). Real Pi 3B+ over netboot, reset at 08:47:58 UTC, login after 20 s. There was
no panic, exception or `FATAL` line. **Pass.**

- **(1) Multicast over the LAN7800:** `mcast recv 239.1.2.3 5000 3` on the Pi while
  homenodellm sent 10 datagrams to 239.1.2.3:5000 (TTL 2).
  - The Pi printed `mcast: joined 239.1.2.3, waiting on port 5000`, then
    `hello 0`, `hello 1`, `hello 2` from 192.168.178.120, and `mcast: left 239.1.2.3`.
- **(2) IPv6** (about a minute after boot):
  - `/scheme/netcfg/ifaces/eth0/addr/list`:
    - `fe80::ba27:ebff:fe08:e0aa/64`;
    - `2a02:590:1509:6101:ba27:ebff:fe08:e0aa/64` (the provider's prefix has
      changed since test 85);
    - `fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa/64`.
  - `/scheme/netcfg/route/list`: `default via fe80::b6fc:7dff:fe53:b289 dev eth0 src
    2a02:…`, plus `ff00::/8` and `224.0.0.0/4` on eth0.
  - ICMPv6 echo from homenodellm to the global address: 5/5 (0.4-8.3 ms); to the `fd…`
    address: 3/3.
    - homenodellm has no `ping` any more, so this used a small Python ICMPv6 socket
      (`~/pitest-tools/ping6.py`).
- **(3) Standard checks:** pi-test version, manifest, ps (45), ps-tty and sdcard pass,
  and 0 of 300 starts failed.

Stable `v0.4.0-dev-201-gb714bca+x9f76` restored (08:51); devsh answers.

## #211: does the network loss of #209 come from the pkg-persist boot copy, or is it #92? (2026-11-04, Pi test agent)

Image `v0.4.0-dev-269-gd3ebd24c+x42ad` (branch `pkg-persist` d3ebd24, PR #106),
`~/redox-rpi-boot/keep/t44p-v1.img`, repository `/srv/tftp/pkg-t44`. Real Pi 3B+ over
netboot, reset at 09:21:26 UTC. On every boot, homenodellm ran a devsh `echo ok` every
30 s. **Result: it is not the boot copy.** Both boots with persisted packages kept the
network through every step. The control boot without packages lost it during the vim
download. That points to #92.

| Boot | Packages copied at boot | Idle 10 min | `pkg install vim` | 2× dd 52 MB | Watch 5 min |
|---|---|---|---|---|---|
| B1 | 215 files (4107 ms) | ok | ok (4:45 min) | ok (159 s each, 330 kB/s) | ok |
| B2 | 215 files (4145 ms) | ok | ok (4:45 min) | ok (158 s each) | ok |
| C2 (no `/scheme/data/pkg`) | "does not exist, nothing to copy" | ok | **network lost** about 40 s after the start | not run | not run |

- **Boot A** (prepare): `40_redox-rpi` set to pkg-t44.
  - `pkg-persist install redox-fatfs2` gave 1 file (158 ms).
  - `pkg-persist install nano` gave 214 files and kept 2 from the image (3867 ms).
  - Then `reboot`.
- **B1, B2:** the copy line came right after `redox login:`, then timesync's NTP line
  (09:23:08 and 09:50:21).
  - devsh missed only the one probe that fell into each reboot.
- **Boot C:** the copy line came again (215 files, 4151 ms). On the serial console,
  `rm -r /scheme/data/pkg` and `reboot`.
- **C2:** pkg-persist printed "does not exist, nothing to copy", and NTP came at 10:18:34.
  - The 10 idle minutes passed with devsh ok.
  - `pkg install --yes vim` started at 10:28:48. devsh failed from 10:29:27 and never
    came back (22 fails until 10:41).
  - The serial log has no error, panic or link-down line.
- **#92 checklist on the serial console** (10:30:43, 12 min 38 s after boot):
  - `cat /scheme/sys/context` answered. `pkg` (pid 138, 17 s CPU) was still running.
  - netstack was blocked, with 1.6 s CPU and 224 KB shared.
  - dwc2d was sleeping, with 16.6 s CPU.
  - No process showed high CPU.
  - `cat /scheme/netcfg/ifaces/eth0/addr/list` **never returned** (more than 120 s).
    netstack no longer answers its own config scheme, so the serial shell hung too.
    `route/list`, `ping`, `free` and `uptime` could not run after it.
- **Not done:** pi-test (planned for boot C2), because the network was gone.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored. The serial shell was stuck, so a
  `reboot` was not possible. A Shelly power cycle at 10:48:05 brought stable up, and
  devsh answers.
  - `/scheme/data/pkg` is removed (boot C).

## #212: rerun of #209 step 6, `pkg update` after a new publish with persisted packages (2026-11-04, Pi test agent)

Image `v0.4.0-dev-269-gd3ebd24c+x42ad` (branch `pkg-persist` d3ebd24, PR #106),
`~/redox-rpi-boot/keep/t44p-v1.img` (md5 `fec05fb1…`). Repositories are `pkg-t44` and
`pkg-t44-v2`. Real Pi 3B+ over netboot, reset at 10:55:49 UTC, login after 20 s.
devsh `echo ok` every 30 s on both boots. The only miss was the probe that fell into the
reboot. **The network stayed up (no #92 this time). No panic, exception or `FATAL`
line. Pass.**

- **(1) Boot 1:**
  - The `date` command gave `Wed Nov 4 10:56:12 UTC 2026`.
  - `/scheme/data/pkg` did not exist (removed in #211), so no extra reset was needed.
  - pi-test `--starts 300`: version, manifest, ps (48), ps-tty, sdcard pass, and 0 of 300 starts failed.
- **(2)** `40_redox-rpi` was set to pkg-t44.
  - `pkg-persist install nano` ended with `pkg-persist: 214 files copied into /, 2 kept from the image (3965 ms)` (50 s in all).
  - `pkg-persist list` shows nano, ncursesw and terminfo. Then `reboot`.
- **(3) Boot 2** (login 11:02:12): the serial log has
  `pkg-persist: 214 files copied into /, 2 kept from the image (3979 ms)` right after `redox login:`.
  - It is 214 and not 215 because this row persisted nano only, not redox-fatfs2.
  - `nano --version` prints `GNU nano, version 9.2`.
  - `/etc/pkg.d/40_redox-rpi` was back to the image's `…:8080/pkg`. /etc is not persisted, and step 5 sets it anyway.
- **(4)** `head -1 /tmp/p.txt` gave `usage: pkg-persist install|remove <package>... | update | list | restore`.
- **(5)** `40_redox-rpi` was set to pkg-t44-v2, then `pkg update --yes` ran.
  - It ended with `Download complete.` and `Commit complete.` in **15 s** (11:02:38 to 11:02:53).
  - Then `head -1` gave `usage (t44 v2): pkg-persist install|remove <package>... | update | list | restore`.
  - `nano --version` still prints 9.2, and `pkg-persist list` is unchanged.
- **(6)** `rm -r /scheme/data/pkg` done, and `ls /scheme/data` no longer shows `pkg`.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored. devsh `echo reset > /scheme/sys/kstop`
  did not reset the Pi this time (devsh needed 5 s for `echo start`). A Shelly power cycle
  at 11:09:43 brought stable up, devsh answers, and `/scheme/data` has no `pkg`.

## #213: #92, where netstack hangs, with the netstack-watch image (2026-11-04, Pi test agent)

Image `v0.4.0-dev-280-gf1ba9a30+x5f6a` (branch `t92-netstack-watch`, experiments
`netstack-watch`, `dwc2d-netstats`, `usb-hub-drop`, `dwc2d-restart`) on a real Pi 3B+ over
netboot. `40_redox-rpi` was set to pkg-t44 and there was no `/scheme/data/pkg`. devsh
`echo ok` ran every 30 s from homenodellm. **Result: the hang came once (run 1) but no
`netstack-watch` line was printed. After that, 3 installs passed without a hang (run 3).**
No panic, exception or `FATAL` line, and no dwc2d `Timeout`, `removed` or `restart` line in any run.

- **This image has no timesync, so the clock starts at 1970.** In run 2 every
  `pkg install vim` stopped at once with `invalid peer certificate: certificate not valid
  yet` and downloaded nothing, so run 2 does not count.
  - For run 3, I copied stable's `timesync` binary to `/scheme/data/timesync-bin`, ran it
    once after boot, and removed it at the end.
  - Run 1 was also at 1970, so its pkg would have failed the same way within a second. The
    network was lost anyway.
- **Run 1** (reset 11:27:18): `pkg install --yes vim` started at 11:28:20.
  - devsh failed from 11:28:48, 28 s later, and never came back. pkg's devsh call never
    returned either.
  - **No `netstack-watch:` line** on the serial console, from the loss until the power cycle
    at 11:33:32 (more than 4 min). No dwc2d line either.
  - Serial snapshots, 49 s apart (11:29:5x and 11:30:4x, uptime 2m37s and 3m26s):

    | | snapshot 1 | snapshot 2 |
    |---|---|---|
    | `dwc2d-netstats` rx_frames | 1464 | 1775 (still receiving) |
    | rx_last_ms | 140224 | 185178 |
    | tx_frames | 452 | 455 (almost nothing sent) |
    | tx_last_ms | 128975 | 163408 |
    | rx/tx errors | 0 | 0 |
    | `dwc2d-step` | `idle` | `idle` |
    | `dwc2d-usbstats` | all errors, timeouts, recoveries, restarts 0; hprt0 0x1005 | the same; hprt0 0x1405 |

    So dwc2d kept running and handing frames up, but almost nothing went out.
  - The context check `grep -e netstack -e dwc2d /scheme/sys/context` failed: Redox grep
    rejects two `-e` options ("Invalid parameters"). So there are no netstack or dwc2d
    process states from this hang. I did not read `/scheme/netcfg`, as the row says.
- **Run 3** (reset 11:40:21, clock set to 11:40:49 from pool.ntp.org): three times
  `pkg install --yes vim` (10.3 MiB at 1.3–2.0 MiB/s, about 5 min each with the commit),
  each ending in `Commit complete.`, then `pkg remove --yes vim`.
  - devsh answered 30 of 30.
  - After install 3, dwc2d counted rx 44660 and tx 20107 frames, with no errors, timeouts or restarts.
  - No `netstack-watch` line, which is expected while netstack works.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored after each run (run 1 by a Shelly power
  cycle, since the network was gone; runs 2 and 3 by a devsh reset). devsh answers.
  `/scheme/data` has no `pkg` and no `timesync-bin`.
- **For Network:** in run 1, netstack stopped sending for good while dwc2d kept receiving,
  and the watch thread printed nothing within 4 min. Either the main loop was not inside a
  device read or write, or the watch thread was stuck too. Next time, snapshot
  `cat /scheme/sys/context` (whole) as well.

## #214: #92, where netstack hangs, second try with a heartbeat and timesync (2026-11-04, Pi test agent)

Image `v0.4.0-dev-287-g5baac82a+x53a7` (branch `t92-netstack-watch`, experiments
`netstack-watch` with heartbeat, `dwc2d-netstats`, `usb-hub-drop`, `dwc2d-restart`,
`timesync`) on a real Pi 3B+ over netboot, reset at 12:07:13 UTC.
- The boot log has `timesync: clock set to 2026-11-04 12:07:32 UTC from pool.ntp.org`.
- `40_redox-rpi` was set to pkg-t44, and there was no `/scheme/data/pkg`.
- devsh `echo ok` ran every 30 s from homenodellm.

**Result: 5 installs without a hang. Pass (second criterion). #92 did not show this time.**
No panic, exception or `FATAL` line, and no dwc2d `Timeout`, `removed` or `restart` line.

- Five times, `pkg install --yes vim` (10.3 MiB, last shown rate 3.0 to 4.1 MiB/s, about 5 min each with the commit) ended in `Commit complete.`. Each was followed by `pkg remove --yes vim` (20 to 25 s). This ran from 12:07:44 to 12:35:15.
- devsh answered 51 of 51.
- **Heartbeat:** 30 `netstack-watch: alive …` lines, one every 60 s from 60 s to 1805 s.
  - All of them say `idle for 0 s`, except the last one, which says `timer event for 0 s`.
  - Events grew from 13240 to 113320 (79356 network), and frames in from 9845 to 76033.
  - Each install shows as a jump of about 10000 events between two heartbeats.
  - There was no `stuck` line.
- After install 5, dwc2d counted rx 74848 and tx 34019 frames, with no errors, timeouts or restarts.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset. devsh answers, and `/scheme/data` has no `pkg`.
- Counting test 213 run 3 (3 installs, clock set by hand), that is 8 vim installs in a row without the hang on this branch. The one hang (213 run 1) came at a 1970 clock, 28 s after a `pkg install` that had to fail on its certificate.

## #215: #47 boot time, PR #85 with today's main (2026-11-04, Pi test agent)

Image `v0.4.0-dev-301-g7a735169+x5bf8` (branch `boot-time` with main merged, experiments
`timesync`, `boot-time`), `~/t18-work/image-bt85/harddrive.img` (md5 `01676e7e…`). Staged
with `~/t18-work/bt85-stage/scripts/netboot.sh`, and the bootloader on the TFTP server is the
image's own (md5 `9036b6c2…`). Real Pi 3B+ over netboot, 3 boots, each with
`echo reset > /scheme/sys/kstop`, timed on the serial log. No panic, exception or `FATAL`
line. **Pass.**

| Boot | `kreset` | `Continuing with the selected mode in 1 s` | `redox login:` | from `kreset` |
|---|---|---|---|---|
| 1 | 1.60 s | 14.98 s | **20.89 s** | 19.29 s |
| 2 | 1.55 s | 14.77 s | **20.44 s** | 18.89 s |
| 3 | 1.55 s | 14.88 s | **20.49 s** | 18.94 s |

- The time is counted from the devsh reset command, as in #107 (21.0–21.2 s).
  This run is 0.3 to 0.7 s faster.
- On every boot, U-Boot asked DHCP once (bound at about 10.6 s), and the image came over
  HTTP in about 1.9 s (21.9 MiB/s).
- On every boot, about 10 s after devsh answered:
  - `eth0` had `192.168.178.148/24` (DHCP lease);
  - `/scheme/data` was mounted (`README.txt` and the other files there);
  - the build ID was right.
- pi-test `--starts 300`: version, manifest, ps (51), ps-tty, sdcard pass, and 0 of 300 starts failed.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset (13:17). devsh answers,
  and `/srv/tftp/redox` matches the stable copy.

## #216: #46 time zones, PR #102 with today's main (2026-11-04, Pi test agent)

Image `v0.4.0-dev-300-gf71c5a4+x0598` (branch `timezone`, experiments `ldso-diagnostics`,
`timerd`, `timesync`, `timezone`), `~/tz-test/keep/tz-v0.4.0-dev-300-gf71c5a4+x0598.img`.
Real Pi 3B+ over netboot, reset at 13:19:21 UTC. No panic, exception or `FATAL` line.
**Pass:** every output as given.

- Boot log:
  - `timesync: clock set to 2026-11-04 13:19:41 UTC from pool.ntp.org`;
  - `timerd: clock set by timesync, 2026-11-04 14:19:42 CET`.
- Over devsh:
  - `date` gave `Wed Nov 4 14:19:57 CET 2026`, and `date -u` gave `Wed Nov 4 13:19:57 UTC 2026` (one hour ahead, `CET`).
  - `ls -l /etc/localtime` gave `/etc/localtime -> /usr/share/zoneinfo/Europe/Berlin`.
  - `date -d '@1792890000'` gave `Sun Oct 25 02:00:00 CET 2026`.
  - `timerd -t 1792886400 1792890000 1774746000` gave:
    - `2026-10-25 00:00:00 UTC = 2026-10-25 02:00:00 CEST (+02:00)`
    - `2026-10-25 01:00:00 UTC = 2026-10-25 02:00:00 CET (+01:00)`
    - `2026-03-29 01:00:00 UTC = 2026-03-29 03:00:00 CEST (+02:00)`
  - `env TZ=UTC timerd -t 1792890000` gave `2026-10-25 01:00:00 UTC = 2026-10-25 01:00:00 UTC (+00:00)`.
- pi-test: version, manifest, ps (47), ps-tty, sdcard pass, and 0 of 300 starts failed.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset (13:21). devsh answers.

## #217: PRs #161, #163 and #164 together on main (2026-11-04, Pi test agent)

Image `v0.4.0-dev-296-g64a62d9+x9f76` (branch `pitest-161-163-164` on main 399bec8, experiments
`timesync`, `rtl8152`), `~/redox-rpi-rebase/keep/pt-161-163-164.img` with its manifest.
Real Pi 3B+ over netboot, reset at 13:24:33 UTC, login after 20 s. No panic, exception or `FATAL`
line. **Pass.**

- **(1)** `redox login:` on the serial console, no panic.
- **(2)** The boot log has `timesync: clock set to 2026-11-04 13:24:52 UTC from pool.ntp.org`
  (relibc `0011`). `date -u` on the Pi and on homenodellm both gave
  `Wed Nov 4 13:25:10 UTC 2026`.
- **(3)** Logged in on the serial console, then
  `patches/experiments/serial-rx/serial-input-test.sh 20`: **20 of 20 lines complete**
  (getty `0024`).
- **(4)** pi-test `--starts 300` with the row's manifest: version, manifest (build ID and
  libc.so hash match), ps (47), ps-tty, sdcard pass, and 0 of 300 starts failed.
- **`0015` does no harm:** the image starts `bcm2835-sdhostd`, which prints `SDHC/SDXC card,
  15193 MiB, 4-bit bus, high speed` and `serving /scheme/disk.mmc`. There is no
  `bcm2835-sdhcid` line.
  - The two `[ERROR redoxfs] failed to open filesystem /scheme/disk.mmc/0…` lines are in
    every netboot image's log (rows 214–216 too), so they are not new.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset (13:30). devsh answers.

## #218: #37 mDNS, PR #93 with today's main (2026-11-04, Pi test agent)

Image `v0.4.0-dev-346-gfd355e58+x3d8d` (branch `mdns`, experiments `multicast`, `ipv6`,
`lan78xx-multicast`, `mdns`), `~/mdns-test/keep/mdns-v0.4.0-dev-346-gfd355e58+x3d8d.img`.
Real Pi 3B+ over netboot, reset at 13:32:54 UTC. No panic, exception or `FATAL` line.
**Pass.**

Serial log:

```
mdnsd: redox-rpi.local on 224.0.0.251 and ff02::fb port 5353, 2 services configured
mdnsd: redox-rpi._devsh._tcp.local port 2323 announced
mdnsd: redox-rpi.local is 192.168.178.148, fe80::ba27:ebff:fe08:e0aa, 2a02:590:1509:6101:ba27:ebff:fe08:e0aa, fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa
```

`mdns-query.py` from `origin/mdns`, run on homenodellm:

- **(1)** `_devsh._tcp.local PTR`: one answer from the Pi with PTR, SRV, TXT, A and AAAA:
  ```
  192.168.178.148: _devsh._tcp.local PTR redox-rpi._devsh._tcp.local (ttl 4500)
  192.168.178.148: redox-rpi._devsh._tcp.local SRV port 2323 target redox-rpi.local (ttl 120)
  192.168.178.148: redox-rpi._devsh._tcp.local TXT 00 (ttl 4500)
  192.168.178.148: redox-rpi.local A 192.168.178.148 (ttl 120)
  192.168.178.148: redox-rpi.local AAAA fe80::ba27:ebff:fe08:e0aa (ttl 120)
  192.168.178.148: redox-rpi.local AAAA 2a02:590:1509:6101:ba27:ebff:fe08:e0aa (ttl 120)
  192.168.178.148: redox-rpi.local AAAA fdeb:d4ef:a7ac:0:ba27:ebff:fe08:e0aa (ttl 120)
  ```
- **(2)** `redox-rpi.local`: the Pi answers with A `192.168.178.148` and the three AAAA. The
  Fritz!Box (192.168.178.1) also repeats the A with ttl 10.
- **(3)** `redox-rpi.local AAAA -6 wlp13s0`: from `fe80::ba27:ebff:fe08:e0aa` (the Pi) came the
  AAAA `fe80::…`, `2a02:590:1509:6101:…` and `fdeb:…`, plus the A. The router repeated
  them with ttl 10.
  - homenodellm is on cable now (`enp12s0`). The same query over `enp12s0` gives the same answers.
- **(4)** `_services._dns-sd._udp.local PTR`: the Pi answers
  `192.168.178.148: _services._dns-sd._udp.local PTR _devsh._tcp.local (ttl 4500)` (plus its
  A/AAAA), and no `_ssh._tcp` from 192.168.178.148.
  - The other devices on the LAN that answered are left out here.
- pi-test: version, manifest, ps (46), ps-tty, sdcard pass, and 0 of 300 starts failed.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset (13:35). devsh answers.

## #219: #55/#86 Ctrl-C, PR #121 with today's main (2026-11-04, Pi test agent)

Image `v0.4.0-dev-314-g9b7948a8+xc169` (branch `ctrlc-raw` with main merged, experiments
`timesync`, `pty-sigint`), `~/t18-work/image-c121/harddrive.img` (md5 `f5e80496…`).
Staged with `~/t18-work/c121-stage/scripts/netboot.sh`; the bootloader is md5 `da1a7595…`.
Real Pi 3B+ over netboot, reset at 13:36:38 UTC, `devshd: listening` at 13:36:58. No panic,
exception or `FATAL` line. **Fail: on the serial console, 3 of 10 rounds were not
stopped, the same 3 in two runs. devsh 10 of 10 and the standard checks pass.**

- **Serial** (from 13:37:36, about 40 s after `devshd: listening`; root login with
  `sercmd.py`, then the logger was paused while
  `serial-ctrlc-test.py --device /dev/ttyUSB0 --delays 0.01,0.02,0.03,0.01,0.02,0.03,0.05,0.1,0.3,1` ran).
  Both runs gave exactly the same result:

  | Round | Delay after Enter | Run 1 (13:37) | Run 2 (13:41) |
  |---|---|---|---|
  | 1 | 0.01 s | SIGINT after 0.2 s | SIGINT after 0.2 s |
  | 2 | 0.02 s | **not stopped in 50 s** | **not stopped in 50 s** |
  | 3 | 0.03 s | SIGINT after 0.1 s | SIGINT after 0.1 s |
  | 4 | 0.01 s | **not stopped in 50 s** | **not stopped in 50 s** |
  | 5 | 0.02 s | SIGINT after 0.2 s | SIGINT after 0.2 s |
  | 6 | 0.03 s | **not stopped in 50 s** | **not stopped in 50 s** |
  | 7–10 | 0.05, 0.1, 0.3, 1 s | SIGINT after 0.0–0.1 s | the same |

  - The pattern: after an early Ctrl-C (0.01–0.03 s) that worked, the next early one is missed. After a missed round, the next early one works again. From 0.05 s on, every round works.
  - In the missed rounds the console shows `sleep 40` and then only the next prompt, with no `ended by signal SIGINT` line. That is what a `sleep 40` that ran to its end looks like.
  - #131 (before the main merge) had 19 of 19 on the same delays.
  - The logs are in `~/pitest-tools/t219/serial-ctrlc.log` and `serial-ctrlc2.log` on homenodellm.
- **devsh:** `ctrlc-test.py 192.168.178.148 --delays 0,0.01,0.02,0.03,0.1` gave **10 of 10 stopped**
  (0.1–0.2 s after Ctrl-C), `Ctrl-C at the prompt: shell still answers`, and devsh answered afterwards.
- pi-test `--starts 300`: version, manifest, ps (47), ps-tty, sdcard pass, and 0 of 300 starts failed.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset (13:45). devsh answers, and the serial logger is running again.

## #222: #92 netstack hang, third try: many fast HTTPS failures (2026-11-04, Pi test agent)

Image `v0.4.0-dev-287-g5baac82a+x5f6a` (branch `t92-netstack-watch`, experiments
`netstack-watch`, `dwc2d-netstats`, `usb-hub-drop`, `dwc2d-restart`, **no timesync**),
`~/redox-rpi-zrtl/keep/t92pi3-v0.4.0-dev-287-g5baac82a+x5f6a.img`. Real Pi 3B+ over netboot.
The clock stayed at 1970 (`date`: `Thu Jan 1 00:00:50 UTC 1970`), and I did not set it.
The loop was typed on the serial console, as the row says. **Result: no stall and no hang
in 6200 fast HTTPS failures. Not one `netstack-watch: … stuck` or `back after` line; every
heartbeat said `idle for 0 s`.**

- **Run 1, as in the row** (reset 14:18:08): `for i in 1..201; echo ROUND $i; pkg install --yes vim; ping -c 1 192.168.178.1; end`
  (ion's `..` leaves out the end, so 200 rounds).
  - It finished in **40 s** (14:19:04 to 14:19:44), not 20 to 40 min. On the Pi, every
    `pkg` fails `invalid peer certificate: certificate not valid yet` in about 0.2 s.
  - 200 of 200 rounds failed that way. Ping: 190 of 200 answered and 10 lost (single rounds).
- **Run 2, longer, to give it the 20 minutes the row meant** (reset 14:20:58): the same loop with `1..6001`.
  - 6000 rounds from 14:21:55 to 14:41:48 (about 300 rounds per minute). 5999 certificate failures.
  - Heartbeats came every 60 s for the whole run (20 lines, at about every 300 rounds). Each
    one we can read says `alive at … s, idle for 0 s`. Some lines are mixed with the loop's
    own output on the console, but they are there.
  - **No `stuck in …` and no `back after N s` line.** No dwc2d `Timeout`, `removed` or `restart` line.
  - Ping: **5486 answered and 514 lost (8.6 %)**, almost all single rounds (472 groups,
    longest 4 rounds in a row, 2599–2602), spread evenly over the whole run. The loop never slowed down.
  - **Once, in round 2058, `pkg` itself crashed:** `FATAL: Not an SVC induced synchronous
    exception (ty=100010)`, `ELR_EL1 0x9` (a jump to address 9),
    `UNHANDLED EXCEPTION, CPU #3, PID 4155, NAME /usr/bin/pkg`. The loop went straight on with
    the ping, and netstack was not affected. This is a pkg crash in user space, not #92.
- Step 3 (a hang) was not needed.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset after each run (14:20 and 14:42). devsh answers.
- **For Network:** QEMU's stalls (6–11 s in 5 of 59 rounds) do not show on the Pi. Here a
  failed TLS start takes about 0.2 s, against several seconds in QEMU, so many more rounds
  fit in the same time and none stalled. The only Pi hang so far (213 run 1) is still unexplained.

## #223: #55/#86 Ctrl-C, PR #121 with ptyd 0058 (2026-11-04, Pi test agent)

Image `v0.4.0-dev-315-g4f3ed615+xc169` (branch `ctrlc-raw` with main merged, experiments
`timesync`, `pty-sigint` with ptyd `0057` and `0058`), `~/t18-work/image-c121/harddrive.img`
(md5 `207316e4…`). Staged with `~/t18-work/c121-stage/scripts/netboot.sh`; the bootloader is
md5 `da1a7595…`. Real Pi 3B+ over netboot, reset at 14:45:08 UTC, `devshd: listening` at
14:45:28. No panic, exception or `FATAL` line. **Pass.**

- **Serial** (from 14:46:07; root login with `sercmd.py`, then the logger was paused while
  `serial-ctrlc-test.py --device /dev/ttyUSB0 --delays 0.01,0.02,0.03,0.01,0.02,0.03,0,0,0.05,0.1,0.3,1` ran):
  **12 of 12 stopped by SIGINT** (`PASS: 0 late of 12`).
  - 0.00–0.05 s after Enter: SIGINT 0.2 s after the Ctrl-C.
  - 0.1 s: SIGINT after 0.1 s.
  - 0.3 s and 1 s: SIGINT after 0.0 s.
  - The rounds that failed in #219 (the second, fourth and sixth early Ctrl-C) now stop like the others, and so do the two rounds with no delay.
- **devsh:** `ctrlc-test.py 192.168.178.148 --delays 0,0.01,0.02,0.03,0.1` gave **10 of 10 stopped**,
  `Ctrl-C at the prompt: shell still answers`.
- pi-test `--starts 300`: version, manifest, ps (52), ps-tty, sdcard pass, and 0 of 300 starts failed.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset (14:48). devsh answers, and the serial logger is running again.

## #226: #92 netstack hang, fourth try: long downloads with netstack-watch and dwc2d-watch (2026-11-04, Pi test agent)

Image `v0.4.0-dev-349-g1bf74324+x7850` (branch `t92-netstack-watch` 1bf74324, experiments
`netstack-watch`, `dwc2d-netstats`, `usb-hub-drop`, `dwc2d-restart`, `dwc2d-watch`, `timesync`),
`~/redox-rpi-zrtl/keep/t92pi4-v0.4.0-dev-349-g1bf74324+x7850.img`. Real Pi 3B+ over netboot,
reset at 14:55:45 UTC. The clock was right (`date`: `Wed Nov 4 14:56:29 UTC 2026`). The loop
`for i in 1..9; echo ROUND $i; pkg install --yes vim; pkg remove --yes vim; ping -c 1 192.168.178.1; end`
(8 rounds) was typed on the serial console and ran from 14:56:45 to 15:39:03 (about 5.3 min a
round). **Result: 8 rounds without a hang (pass). No `stuck` or `back after` line from
either watch. But round 8's install failed with `No space left on device`.**

- **Rounds 1–7:** `pkg install --yes vim` downloaded the 10 MB `vim.pkgar` and ended in
  `Commit complete.`. `pkg remove --yes vim` also ended in `Commit complete.`. Ping answered 1 of 1.
- **Round 8:** the download finished, then
  `error: pkgar error: No space left on device (os error 28) (None) "Copying and hashing file"`.
  The remove had nothing to do, and ping answered.
  - So after 7 install/remove cycles the netboot root filesystem is full. Something, maybe
    pkg's download cache, is not freed by `pkg remove`.
  - The network and both watches were not affected.
- **Watches:** no `netstack-watch: … stuck`, no `dwc2d-watch: … stuck`, and no `back after` line in the whole run.
  - Heartbeats kept coming every 60 s from both: 39 from netstack and 42 from dwc2d. All say `idle for 0 s`.
  - Last ones: `netstack-watch: alive at 2526 s, idle for 0 s, 109378 events (95486 network), 84376 frames in`
    and `dwc2d-watch: alive at 2530 s, idle for 0 s, events: 42284 interrupt, 219143 scheme, 119050 other`.
- No panic, exception, `FATAL` or `UNHANDLED` line, and no dwc2d `Timeout`, `removed` or `restart` line.
- Step 3 (a hang) was not needed.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset (15:39). devsh answers.

## #225: #24 first-boot welcome app, PR #77 with today's main (2026-11-04, Pi test agent)

Image `v0.4.0-dev-351-ga0bbce34+xda44` (branch `welcome-app` with main merged, experiments
`timesync`, `welcome`), `~/t18-work/image-w77/harddrive.img` (md5 `fa627956…`). Staged with
`~/t18-work/w77-stage/scripts/netboot.sh`; the bootloader is md5 `da1a7595…`. Real Pi 3B+ over
netboot, reset at 15:41:44 UTC. No panic, exception or `FATAL` line. **Pass.**

- The serial console showed `Welcome to Redox OS on the Raspberry Pi.` and waited, with no
  `redox login:` and no banner before it.
- The prompts were answered on serial (one character every 0.15 s, each after its prompt), 15:45:53 to 15:46:00:
  - Enter at "Press Enter to set";
  - `gb` at `Keyboard layout [us]:`;
  - `pi-test` at `Name of this computer (hostname) [redox]:`;
  - Enter at `Set a new root password? … [n]:`;
  - `TestNet` at `Wi-Fi network name []:`;
  - `secret123` at `Passphrase` (**not echoed**: the serial log has no `secret123` from this step).
- Then `Saved; the Wi-Fi driver joins it at the next boot.`,
  `Done. Log in as root. To change these settings later, run welcome --force.`, and right after
  that the banner (`redox-rpi v0.4.0-dev-351-ga0bbce34+xda44 (netboot, …)`) and `redox login:`.
  - The welcome also said `Time zone: no zone files in /usr/share/zoneinfo; the clock shows UTC.`
    This image has no `timezone` experiment.
- After a root login on serial:
  - `cat /etc/keymap /etc/hostname /etc/net/wifi /etc/welcome.done` shows `gb`, `pi-test`,
    `ssid=TestNet` / `psk=secret123`, and
    `First-boot setup done. Delete this file to run it again before the next login.`
    - Small detail: `/etc/hostname` has no newline at the end, so the next file's first line starts on the same line.
  - `ls -l /etc/net/wifi` gives `-rw------- 1 root root 78 Nov 4 15:46 /etc/net/wifi`.
- pi-test (devsh): version, manifest, ps (46), ps-tty, sdcard pass, and 0 of 300 starts failed.
- I did not check the NanoKVM (no screenshot needed). The second-boot check stays in QEMU, as the row says.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset (15:50). devsh answers.

## #227: #47 boot time step 2, PR #198 with today's main: the 192 MiB network-boot image (2026-11-04, Pi test agent)

Image `v0.4.0-dev-351-gece48235` (branch `boot-time-step2`, no experiments),
`~/t18-work/image-bt2/harddrive.img` (md5 `33886bd0…`). Staged with
`~/t18-work/bt2-stage/scripts/netboot.sh`; the bootloader is md5 `da1a7595…`. Real Pi 3B+ over
netboot, 3 boots with `echo reset > /scheme/sys/kstop`, timed on the serial log. No panic,
exception or `FATAL` line. **Pass, except `pkg install nano`: this image has no timesync, so
the clock is at 1970 and the HTTPS certificate check fails.**

| Boot | `kreset` | image loaded over HTTP | kernel read (`usr/lib/boot/kernel: 1/1 MiB`) | `redox login:` | reset → login |
|---|---|---|---|---|---|
| 1 | 1.90 s | 12.92 s | 20.53 s | 29.81 s | 27.9 s |
| 2 | 1.55 s | 12.44 s | 20.06 s | 29.18 s | 27.6 s |
| 3 | 1.55 s | 12.48 s | 20.09 s | 29.32 s | 27.8 s |

- U-Boot reports `Uncompressed size: 201326592 = 0xC000000` (192 MiB). The bootloader copies `live: …/158 MiB`.
- Image loaded → kernel read takes about 7.6 s. Of that, 5 s is the bootloader's wait
  (`Continuing with the selected mode in 5 seconds`): this branch does not have #85's 1-second wait.
  - Reset → login is 27.6–27.9 s, against 20.4–20.9 s in #215 with #85.
- **Memory and disk, against stable dev-201 just before:**

  | | stable | this image |
  |---|---|---|
  | `df -h` `/` | 222.0M, 133.6M free | 158.0M, **69.9M free** |
  | `free -m` total | 739892 | **805296** (+65404, about 64 MiB more) |

  (`free -m` prints KiB on Redox.)
- DHCP lease `192.168.178.148/24` on every boot.
- **`pkg install --yes nano`:** `error: Download error: … invalid peer certificate: certificate not valid yet: verification time 49 (UNIX)`.
  - The image has no `timesync`, so the clock starts at 1970, and static.redox-os.org's certificate is "not valid yet". This is not a size problem: `/` still has 69.9M free.
  - With `--experiment timesync`, as in the stable image, it should work.
- pi-test: version, manifest, ps (50), ps-tty, sdcard pass, and 0 of 300 starts failed.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset (16:03). devsh answers.

## #228 #126 power saving step 1: idle power with Ethernet at 100 Mbit/s against gigabit (2026-11-04, Pi test agent)

Image `v0.4.0-dev-355-g20e2dfc1+x0e33` (branch `power-eth` 20e2dfc1, experiment `eth-speed`,
base `0090`). Real Pi 3B+ over netboot, reset at 17:27:59 UTC. The Shelly plug was only read.
**Result: 100 Mbit/s saves about 0.35 W at idle** (3.24 W to 2.89 W, about 11 %). That is
well above the plug's 0.1 W step, and the same in both rounds.

- Boot: `redox login:` 20 s after the reset. The switch port is gigabit. The boot line reads
  `dwc2d: link up on the LAN7800 at address 6, 1000 Mbit/s`, not the
  `LAN78xx link up after N ms` that the row expected.
- pi-test passes: manifest matches, ps, ps-tty, sdcard, 0 of 300 starts failed.
- Each sample: `shelly-sample.sh <label> 36 10`, 36 readings over 6 min, idle, nothing else
  running. The speed was changed on the serial console.

| Sample | Link | Mean | Min | Max |
|---|---|---|---|---|
| g1 | 1000 Mbit/s | 3.23 W | 3.2 | 3.3 |
| m1 | 100 Mbit/s | 2.89 W | 2.8 | 3.2 |
| g2 | 1000 Mbit/s | 3.24 W | 3.2 | 3.4 |
| m2 | 100 Mbit/s | 2.88 W | 2.7 | 3.1 |

- Gigabit mean 3.235 W, 100 Mbit/s mean 2.885 W, **difference 0.35 W**.
  - The higher first reading in m1 and m2 (3.2 and 3.1) is the time just after the switch.
    Leaving it out gives 2.88 and 2.87 W.
- Every change: `dwc2d: link: advertising up to N Mbit/s, negotiating again`, then
  `link up on the LAN7800 at address 6, N Mbit/s` within 4 s. `ping -c 3` to the server gave
  3 of 3 every time. devsh answered at the end at 1000 Mbit/s.
- Small finding, not about the image: after the third change (`echo 100`), my serial helper did
  not see the shell prompt for 120 s. The link line was already there, so it only made the gap
  before m2 longer.
- The plug's `aenergy.total` counter updates only in steps, so it can't give a finer number
  over 6 min. I used the `apower` readings only.
- Afterwards `echo 1000 > /tmp/dwc2d-link`, then stable `v0.4.0-dev-201-gb714bca+x9f76`
  restored with a devsh reset: banner after 20 s, devsh answers, `/srv/tftp/redox` matches the
  backup.

## #229 #53 rerun of test 122 on PR #81 with testnetd as the second adapter (2026-11-04, Pi test agent)

Image `v0.4.0-dev-362-gbde20707` (branch `netstack-multi` bde20707, PR #81, no experiments).
Real Pi 3B+ over netboot, reset at 17:59:44 UTC, `redox login:` after 21 s.
**Result: the second interface works, but the network was lost afterwards.** All the
step-2 checks pass, including the directory listing that failed in 122. Then, during the
second pi-test, eth0 stopped answering. It looks like #92.

- **(1) pi-test** before testnetd: pass (manifest matches, 0 of 300 starts failed).
- **(2) After `/usr/lib/drivers/testnetd &`** (18:03:09, on serial):
  - testnetd printed `testnetd: network.test0 registered`, then `DHCP: eth1 got 10.0.9.2/24`.
  - `ls /scheme/netcfg/ifaces` gave `eth0  eth1`. This fails in 122; it passes now.
  - `eth1/addr/list` gave `10.0.9.2/24`; `eth0/addr/list` gave `192.168.178.148/24`.
  - `route/list` had exactly one `default via 192.168.178.1 dev eth0`, plus
    `10.0.9.0/24 dev eth1`, `192.168.178.0/24 dev eth0` and loopback.
  - `ping -c 3 10.0.9.1`: 3/3, about 2.5 ms. `ping -c 3 192.168.178.1`: 3/3.
  - `/tmp/testnetd.log`: `dhcp 2 dhcp_foreign 0 foreign_src 0 ping 3`.
- **(3) pi-test again: fail.** version, manifest, ps, ps-tty and sdcard passed. The 300-start
  loop over devsh then got no output, and devsh could not reconnect.
- **Serial console** about 15 min later (18:19):
  - The shell answered. `cat /scheme/sys/context` showed netstack blocked with little CPU
    (0.8 s), dwc2d sleeping, and testnetd still running.
  - The initfs `redoxfs` (pid 20) was running with 52 s CPU. Most of that is probably the
    600 starts.
  - `ping -c 3 192.168.178.120` printed its first line, then hung, and Ctrl-C did not stop it.
  - This is the #92 picture as in tests 211 and 213. It came after a second interface was
    added, but this run cannot say whether testnetd is the cause.
- **(4) Serial log:** `DHCP client bound to address 192.168.178.148` twice at boot,
  `DHCP: eth0 got 192.168.178.148/24`, `DHCP: default route via 192.168.178.1 (eth0)`, and
  `DHCP: eth1 got 10.0.9.2/24`. There was no panic and no netstack error line before or
  after the loss.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a Shelly power cycle at 18:23:18 (the
  network was gone). The banner came after 30 s, devsh answers, and `/srv/tftp/redox`
  matches the backup.

## #230 #155 small writes collected up to 1 MiB, A/B against PR #73 (2026-11-04, Pi test agent)

Real Pi 3B+ over netboot, the same card as #175, `/scheme/data` on the SD card.
- **A:** `v0.4.0-dev-157-g71d7ba7d+x08c5` (PR #73 + `sdhost-stats`), reset at 18:24:34.
- **B:** `v0.4.0-dev-158-gb1e99de3+xcad5` (`sd-write-1m` + `sdhost-stats` + `redoxfs-coalesce-1m`),
  reset at 18:29:30.

**Result: pass.** All 18 written files match the sha256 of `/tmp/r4` (`c4c401d0…`). There
is no `write of block`/`read of block` error, and pi-test passes on both images (0 of 300
starts failed). **B sends small writes to the card the way 1 MiB writes go.** For 4 MiB in
4 KiB calls, B needs 86 requests of about 100 blocks, against 283 of 36 on A. That is about
as few as 1 MiB writes take (77–78 requests).

**dd MB/s** (4 MiB, `conv=fsync`, runs 1 / 2 / 3):

| Size | A | B |
|---|---|---|
| 4096 | 1.9 / 3.7 / 3.7 | 5.4 / 3.9 / 5.2 |
| 65536 | 1.1 / 4.9 / 5.5 | 7.9 / 8.8 / 5.1 |
| 1048576 | 9.2 / 10 / 8.0 | 6.7 / 10 / 9.6 |

**sdhostd statistics**, one line per run, write part (requests, blocks, ms, busy ms):

| Size | A | B |
|---|---|---|
| 4096 | 283 req, 10200 blk, 1730 / 671 / 666 ms, busy 253 / 253 / 240 ms | 86 / 86 / 85 req, 8624 / 8624 / 8616 blk, 335 / 634 / 358 ms, busy 109 / 113 / 105 ms |
| 65536 | 274 req 10248 blk 3805 ms busy 253 (run 1; with a 9-request line after it), then 283 req 10320 blk 705 / 626 ms, busy 243 / 252 ms | 86 / 85 / 86 req, 8744 / 8736 / 8744 blk, 465 / 361 / 720 ms, busy 97 / 85 / 99 ms |
| 1048576 | 77 / 78 req, 8552 / 8560 blk, 391 / 346 ms, busy 103 / 111 ms | 78 / 77 req, 8560 / 8552 blk, 569 / 350 ms, busy 109 / 115 ms |

- On both images, the line for the third 1 MiB run came after the serial log was cut, so
  it is missing.
- A 4 KiB: 283 requests of 36 blocks, as in #175 (275 of 37).
- B 4 KiB and 64 KiB: 85–86 requests, 8616–8744 blocks, and busy 85–113 ms. That is the same
  as 1 MiB writes on either image (77–78 requests, 8552–8560 blocks, busy 103–115 ms).
- Disk blocks per 4 MiB: A 10200 against B 8620, which is 1.22 to 1.03 times the data
  (8192 blocks).
- dd MB/s is noisy, and the first run on A is slow at every size. Still, B's 4 KiB and 64 KiB
  runs (3.9–8.8 MB/s) are clearly closer to 1 MiB (6.7–10 MB/s) than A's (1.1–5.5).
- After each run, `sha256sum` of the file and `rm` went fine.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 20 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #231 #126 power saving step 2: Ethernet speed as a boot setting, and firmware clock states (2026-11-04, Pi test agent)

Image `v0.4.0-dev-356-g4270128b+xdd41` (branch `power-eth` 4270128b, experiments
`clock-states eth-speed eth-speed-test`, `/etc/eth-speed` = `100`). Real Pi 3B+ over netboot,
reset at 18:35:24 UTC, `redox login:` after 20 s. The Shelly plug was only read.

**Result: the boot setting works, and idle power is 2.88 W, the same as the 100 Mbit/s
samples of #228.** One finding: `clock_states` reports `off` for `arm`, `core` and `sdram`,
which are clearly running, so its on/off reading looks wrong.

- **Boot log:**
  - `dwc2d: link up on the LAN7800 at address 6, 1000 Mbit/s` during the initfs.
  - After the banner: `devshd: listening on port 2323`, then
    `dwc2d: link: advertising up to 100 Mbit/s, negotiating again`,
    `link down on the LAN7800 at address 6`, `link up … 100 Mbit/s`.
  - devsh answered after that, and pi-test started at once.
  - There is a `kernel::scheme::user:DEBUG -- call_fdread: payload: 8 metadata: 2` line just
    before.
  - No `DHCP:` line from Redox in this image, but the address is 192.168.178.148 and devsh
    works.
- **pi-test:** pass (version, manifest matches, ps, ps-tty, sdcard, 0 of 300 starts failed).
- **`cat /scheme/rpi-firmware/clock_states`**, every line:
  `emmc off`, `uart on`, `arm off`, `core off`, `v3d off`, `h264 off`, `isp off`,
  `sdram off`, `pixel off`, `pwm off`.
  - The ARM, core and SDRAM clocks can't really be off while the system runs, and `clocks`
    shows them at full rate. So the state bit is probably read wrongly, and `v3d h264 isp
    pwm pixel off` can't be trusted yet.
- **`cat /scheme/rpi-firmware/clocks`** (current, max in Hz), every line:
  `emmc 200000000 200000000`, `uart 48000000 1000000000`, `arm 1400000000 1400000000`,
  `core 250000000 400000000`, `v3d 250000000 300000000`, `h264 250000000 300000000`,
  `isp 250000000 300000000`, `sdram 400000000 450000000`.
  - There is no line for `pixel` or `pwm`.
- **Idle power** (`shelly-sample.sh boot100 36 10`, 6 min, nothing else running):
  **mean 2.88 W**, min 2.8, max 2.9, n 36. #228 had 2.89 and 2.88 W at 100 Mbit/s, and
  3.23–3.24 W at gigabit.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 20 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #233 #53 rerun of 229 on PR #81 with the netstack hang fix (2026-11-04, Pi test agent)

Image `v0.4.0-dev-363-gd0e656c4` (branch `netstack-multi` d0e656c4, PR #81, no experiments).
Real Pi 3B+ over netboot, no HDMI connected. **Result: pass. The network was not lost in
two boots, 40 pi-test runs, and 34 minutes with testnetd running.**

- **Boot 1** (reset 20:18:03, `redox login:` after 20 s):
  - pi-test pass. `/usr/lib/drivers/testnetd &` at 20:21:28, then step 2 as in 229.
  - pi-test three more times in a row, all pass (0 of 300 starts failed each).
  - Each run takes only about 50 s, so testnetd ran only about 4 min here. In 229, the
    second run only looked long because it hung. So I added boot 2.
- **Boot 2** (reset 20:26:42, login after 20 s):
  - pi-test pass. testnetd at 20:30:04, then step 2 again.
  - Then pi-test in a loop for 30 min: **36 runs, all pass**, each with 0 of 300 starts
    failed and devsh never lost.
  - At 21:01 on serial, `/tmp/testnetd.log` showed `dhcp 2 dhcp_foreign 0 foreign_src 0
    ping 3`, and `ping -c 3 10.0.9.1` gave 3/3.
- **Step 2, the same in both boots:**
  - testnetd: `network.test0 registered`, then `DHCP: eth1 got 10.0.9.2/24`.
  - `ls /scheme/netcfg/ifaces` gave `eth0  eth1`. eth1 had `10.0.9.2/24`, and eth0 had
    `192.168.178.148/24`.
  - `route/list` had exactly one `default via 192.168.178.1 dev eth0`, plus
    `10.0.9.0/24 dev eth1`.
  - `ping -c 3 10.0.9.1` and `ping -c 3 192.168.178.1`: 3/3 each.
  - `/tmp/testnetd.log`: `dhcp 2 dhcp_foreign 0 foreign_src 0 ping 3`.
- **Serial log:** `DHCP: eth0 got 192.168.178.148/24`, `DHCP: default route via
  192.168.178.1 (eth0)`, `DHCP: eth1 got 10.0.9.2/24`. There was no netstack error line and
  no panic.
- 229 lost the network in its second pi-test, about 10 min after testnetd started. Here
  testnetd ran 4 + 30 min and more than 11000 program starts went over devsh with no loss.
  The hang window is small, so this is good evidence, not proof.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored after each boot with a devsh reset:
  banner after 20 s, devsh answers, and `/srv/tftp/redox` matches the backup.

## #232 #126 power saving step 2b: fixed clock_states, RPI_ETH_SPEED from the command line (2026-11-04, Pi test agent)

Image `v0.4.0-dev-357-g347fc319+x6f4a` (branch `power-eth` 347fc319, experiments
`clock-states eth-speed`, no `/etc/eth-speed`). Real Pi 3B+ over netboot, reset at 21:04:04
UTC, `redox login:` after 20 s. No HDMI connected.

**Result: clock_states is fixed. RPI_ETH_SPEED did not reach the Pi.** The firmware does not
read `/srv/tftp/cmdline.txt` in this setup. So this run can't say whether U-Boot or the
bootloader passes the firmware command line on.

- **(1) RPI_ETH_SPEED:**
  - `/srv/tftp/cmdline.txt` = `RPI_ETH_SPEED=100` (one line) was created at 21:03:49, before
    the reset.
  - The boot log has no `RPI_ETH_SPEED` line at all. The only link line is
    `dwc2d: link up on the LAN7800 at address 6, 1000 Mbit/s`.
  - Why: this Pi starts the GPU firmware (`start.elf`) and U-Boot **from its SD card**. The
    log shows `Loading Environment from FAT... OK` and `bootflow 'mmc@7e300000.bootdev.part_1'
    with script`. Only then does U-Boot fetch `redox/netboot.scr` and the image from
    homenodellm.
  - The firmware reads `cmdline.txt` from the card's FAT partition, so the file in
    `/srv/tftp` is never read. dnsmasq logged only DHCP for the Pi, and no TFTP request for
    `cmdline.txt`.
  - To test the path, `RPI_ETH_SPEED=100` must go into `cmdline.txt` on the netboot card
    (that needs a person and a card swap), or into U-Boot's `bootargs` in `netboot.scr`.
- **(2) pi-test:** pass (version, manifest matches, ps, ps-tty, sdcard, 0 of 300 starts failed).
- **(3) `cat /scheme/rpi-firmware/clock_states`** (name, state, measured Hz, state word), every
  line:
  `emmc on 200000000 0x0`, `uart on 48000000 0x1`, `arm on 1399998000 0x0`,
  `core on 400000000 0x0`, `v3d on 300000000 0x0`, `h264 off 0 0x0`, `isp off 0 0x0`,
  `sdram off 0 0x0`, `pixel off 0 0x0`, `pwm off 0 0x0`.
  - arm and core now read `on` with a measured rate (#231 had them `off`).
  - **sdram still reads `off 0`**, although `clocks` says 400 MHz. So the firmware's measured
    rate for SDRAM seems to be 0 (not measurable this way), and the state is derived from it.
  - core is measured at 400 MHz while `clocks` reports 250 MHz current; v3d is measured at
    300 MHz against 250.
  - h264, isp, pixel and pwm read off at idle.
- **`cat /scheme/rpi-firmware/clocks`**, every line:
  `emmc 200000000 200000000`, `uart 48000000 1000000000`, `arm 1400000000 1400000000`,
  `core 250000000 400000000`, `v3d 250000000 300000000`, `h264 250000000 300000000`,
  `isp 250000000 300000000`, `sdram 400000000 450000000`.
- **(4)** `/srv/tftp/cmdline.txt` was deleted at 21:05:21, before the restore. Stable
  `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 20 s, devsh
  answers, and `/srv/tftp/redox` matches the backup.

## #234 #126 power saving step 2c: Ethernet speed through U-Boot bootargs (2026-11-04, Pi test agent)

Image `v0.4.0-dev-358-ga2f7248b+x6f4a` (branch `power-eth` a2f7248b, experiments `eth-speed`
and `clock-states`). Real Pi 3B+ over netboot, no HDMI connected. **Result: pass. The setting
reaches the kernel, and the first link is 100 Mbit/s.**

- **(1)** After staging, `netboot-bootargs.sh "RPI_ETH_SPEED=100"` (from `power-eth`
  5f6ceaf0) put `# netboot-bootargs.sh` and the `setenv bootargs` line at lines 55–56 of
  `/srv/tftp/redox/netboot.cmd`, before `bootefi` (line 57), and compiled `netboot.scr`.
- **(2)** Reset at 21:17:39 UTC, `redox login:` after 18 s. Boot log:
  - `dwc2d: RPI_ETH_SPEED=100 from the command line`
  - `dwc2d: LAN78xx advertising up to 100 Mbit/s (RPI_ETH_SPEED)`
  - `dwc2d: link up on the LAN7800 at address 6, 100 Mbit/s`. This is the first link line;
    there is no 1000 line at all.
  - Side note: in this boot the LAN7800 disappeared once right after the setting was applied
    (`hub 2 port 1: device at address 6 removed`, then `device connected`). It was set up
    again with the same advertising line, and the `link up … 100 Mbit/s` line came twice.
    `devices ready after 4262 ms` (#228/#231–#233: 4666–4828 ms, and none of those boots
    had the removal). The network worked normally.
- **(3) pi-test:** pass (version, manifest matches, ps, ps-tty, sdcard, 0 of 300 starts failed).
- **(4) `cat /scheme/rpi-firmware/clock_states`:**
  `emmc on 200000000 0x0`, `uart on 48000000 0x1`, `arm on 1400000000 0x0`,
  `core on 399999000 0x0`, `v3d on 300000000 0x0`, `h264 off 0 0x0`, `isp off 0 0x0`,
  **`sdram unknown 0 0x0`**, `pixel off 0 0x0`, `pwm off 0 0x0`.
  - sdram reads `unknown` as intended; the others are as in #232.
  - `clocks` is unchanged from #232.
- **(5)** Stable `v0.4.0-dev-201-gb714bca+x9f76` restored, including `netboot.scr` without the
  line: banner after 18 s, devsh answers, and `/srv/tftp/redox` matches the backup.

## #91 #26 Wi-Fi steps 3–4, part A rerun on the wifi1 image (2026-11-04, Pi test agent)

Image `v0.4.0-dev-365-ge557d3fe+xe9a9` (branch `wifi-sdio`, PR #202, experiments
`timesync wifi-sdio`). Real Pi 3B+ over netboot, `/srv/tftp/config.txt` unchanged
(`gpu_mem=16`), no HDMI connected. Reset at 21:20:53 UTC, `redox login:` after 20 s.
**Result: part A passes. Part B was not run** (see the end).

- **Boot:**
  - `bcm2835-sdiod: wifi_power was 0`, then `rpi-firmwared: Wi-Fi power 0, tag not marked
    answered` and `Wi-Fi power 1, tag not marked answered`. There is **no `I/O error`**
    anywhere in the log.
  - The probe went up to `chip ID OK`: CMD5 3 functions, CIS 0x02d0, chip 0x4345 rev 6, 5 cores.
  - Minimal firmware 548728 bytes written in 277 ms and read back, NVRAM 1748 bytes, ARM
    started, HT clock on.
  - Firmware 7.45.241, `MAC address b8:27:eb:5d:b5:ff`.
- **`CLM loaded (2676 bytes, 2 chunks)`**, `radio up`, then **`10 networks`**, one line each
  (BSSID, channel, dBm, security, name).
  - All are `Wpa2`, on channels 1, 6, 11, 36, 100 and 116, at −53 to −83 dBm.
  - Our own Fritz!Box networks `zehrer.net`/`zehrer.xyz` are on ch 100 (−53), ch 6
    (−58/−59) and ch 1/116 (−77 to −81). 4 neighbour networks.
  - Then `no /etc/net/wifi, not joining a network` and `bcm2835-sdiod: OK`.
- **devsh:**
  - `cat wifi_power` gave 1.
  - `echo 0 > /scheme/rpi-firmware/wifi_power; echo $?` gave `0` (took 5 s), and `cat` gave 0.
  - `echo 1 > …; echo $?` gave `0`, and `cat` gave 1.
  - Each write logs `Wi-Fi power N, tag not marked answered`.
- **`/usr/lib/drivers/bcm2835-sdiod` once more:**
  - Power-cycled the chip (Wi-Fi power 0 and 1 logged), the same probe and firmware, CLM
    loaded, radio up, **`15 networks`**, then `OK`, rc=0.
  - The list overlaps the first one (the same own networks, the same channels, within
    1–3 dB) plus 5 more neighbour networks that were weaker or not caught the first time.
- **`free`:** total 739904 KiB, used 126640, free 613264.
- **Part B (`gpu_mem=64`) not run.** It would change `/srv/tftp/config.txt`, but #232 showed
  that this Pi loads `start.elf`, `config.txt` and U-Boot from its **SD card**. The TFTP
  `config.txt` is never read, so a change there could not change the memory split.
  Part B needs `gpu_mem=64` in `config.txt` on the netboot card (a person and a card swap).
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 20 s.
  pi-test on stable passes (0 of 300 starts failed), and `/srv/tftp/redox` matches the backup.

## #235 #175 assistant: the conversation cap keeps memory flat (2026-11-04, Pi test agent)

Image `v0.4.0-dev-47-g06188c2+x4590` (branch `assistant-history-cap`, experiments
`timesync assistant assistant-netboot`). Real Pi 3B+ over netboot, no HDMI connected.
Reset at 21:38:46 UTC, `redox login:` after 20 s.

**Result: pass, with one finding.**
- All 14 answers came, and the note came once.
- After the note, the heap stayed at 434 MiB to the end, and `free` stopped growing.
- No `oom`, no `RELIBC PANIC`, no panic in the serial log. `assistant` ended with rc=0.
- The finding: saving the prompt cache fails with `No space left on device`.

- **pi-test:** pass (manifest matches, 0 of 300 starts failed).
- **`ls -l /scheme/data`:** `Qwen3-0.6B-Q4_0e.gguf` with 341455552 bytes.
- **`free` before:** total 870656, used 126968, free 743688 KiB.
- **`assistant < /tmp/q.txt`** with `ASSISTANT_DEBUG=1`, 21:40:11 to 21:53:46 (812.8 s):
  - The model loaded in 22.1 s (heap 375 MiB), using neon and 3 threads.
  - `rinfer: No such file or directory (os error 2)`: there was no saved prompt cache yet.
  - System prompt: `read 380 tokens in 171.8 s, heap 392 MiB (peak 430 MiB)`.
  - Then **`rinfer: No space left on device (os error 28)`** (saving `prompt.kv` fails).

| Q | Answer | read | wrote | cache | heap (peak) |
|---|---|---|---|---|---|
| My name is Anna. | "Anna is a user who wants to know about their files and time. …" | 22 tok 12.1 s | 24 tok 21.7 s | 426 | 399 (430) |
| capital of France | Paris | 25 / 14.8 s | 7 / 3.9 s | 458 | 406 (430) |
| three colours | red, blue, green | 22 / 17.5 s | 10 / 14.3 s | 490 | 406 (430) |
| capital of Italy | Rome | 25 / 15.1 s | 7 / 5.4 s | 522 | 413 (430) |
| three animals | "a dog, a cat, and a dog" | 22 / 13.5 s | 13 / 10.0 s | 557 | 413 (430) |
| capital of Spain | Madrid | 25 / 16.0 s | 7 / 5.5 s | 589 | 420 (430) |
| three fruits | apple, banana, mango | 22 / 14.4 s | 13 / 8.2 s | 624 | 420 (431) |
| capital of Germany | Berlin | 25 / 17.8 s | 7 / 4.1 s | 656 | 427 (439) |
| three trees | oak, pine, maple | 22 / 15.5 s | 13 / 7.8 s | 691 | 427 (439) |
| capital of Austria | Vienna | 25 / 18.1 s | 7 / 5.9 s | 723 | 434 (447) |
| three birds | parrot, sparrow, robin | 22 / 17.0 s | 15 / 15.4 s | 760 | 434 (447) |
| capital of Poland | Warsaw | 24 / 20.7 s | 7 / 11.4 s | 515 | 434 (472) |
| three flowers | rose, daisy, sunflower | 22 / 19.1 s | 15 / 12.2 s | 552 | 434 (472) |
| What is my name? | "My name is Alex." (Anna was forgotten, allowed) | 23 / 23.8 s | 5 / 2.9 s | 580 | 434 (472) |

- **Restart before question 12**, after 760 tokens:
  - `rinfer: /home/root/.cache/assistant/prompt.kv is for another model or prompt`.
  - The system prompt was read again: `read 380 tokens in 188.1 s, heap 434 MiB (peak 472 MiB)`.
  - `No space left on device` again.
  - `read 104 tokens in 65.9 s` (the kept exchanges).
  - Then **`(The conversation got long; I keep only the last 3 exchanges and forget the rest.)`**
    once.
  - The restart took about 4.5 min before the Poland answer. The cache went from 760 down
    to 515 tokens.
- **heap:** it grows by about 7 MiB every two answers up to 434 MiB at 760 tokens, then
  **stays at 434 MiB** for all answers after the note. The peak rose once to 472 MiB during the
  re-read. QEMU had 434 MiB and a peak of 450.
- **`free` from a second devsh**, used KiB, total 870656:
  21:41:41 709580; 21:43:13 746732; 21:44:44 728900; 21:46:16 746072;
  21:47:52 766708; 21:49:24 791428 (during the restart); 21:50:55 767500; 21:52:28 768456.
  - After the restart it is flat at about 768 MB used.
  - The last sample (21:53:59) fell into the reset.
- **Finding:** writing the prompt cache fails with `No space left on device (os error 28)`,
  twice. So every start reads the 380-token system prompt again (172–188 s), and the
  "for another model or prompt" line came at the restart. The model on `/scheme/data` was
  kept.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 20 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #236 #175 assistant: the saved prompt state goes next to the model (2026-11-04, Pi test agent)

Image `v0.4.0-dev-432-g9357191+x4590` (branch `assistant-history-cap`, experiments
`timesync assistant assistant-netboot`). Real Pi 3B+ over netboot, no HDMI connected.
Reset at 22:14:35 UTC, `redox login:` after 18 s.

**Result: pass.** The second start takes 2.6 s for the system prompt instead of 191 s.

- **pi-test:** pass (manifest matches, 0 of 300 starts failed).
- **`ls -l /scheme/data` before:** only `Qwen3-0.6B-Q4_0e.gguf` with 341455552 bytes.
- **Run 1** (22:15:56–22:19:54):
  - The model loaded in 21.9 s.
  - `rinfer: No such file or directory (os error 2)` twice: no saved state next to the model
    or in `~/.cache` yet.
  - `read 380 tokens in 191.4 s`.
  - **`rinfer: saved 380 tokens in 5.3 s`**, with no `os error 28`.
  - "The capital of France is Paris." (7 tokens in 4.7 s), rc=0.
- **Run 2** (22:19:54–22:20:38, 44 s in all):
  - The model loaded in 22.2 s.
  - **`rinfer: loaded 380 tokens in 2.6 s, heap 390 MiB`**, then `read 0 tokens in 0.0 s`.
  - "The capital of France is Paris." (7 tokens in 3.9 s), rc=0.
- **`ls -l /scheme/data` after:** `Qwen3-0.6B-Q4_0e.gguf.prompt.kv` with **43581693 bytes**,
  next to the model.
- **`free`:** used 126744 KiB before, 236764 after.
- No panic, no `RELIBC PANIC`, no `oom` in the serial log.
- The model and the new file were kept on `/scheme/data`.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 21 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #237 #196 chaos test on the Pi (PR #206) (2026-11-04, Pi test agent)

Image `v0.4.0-dev-346-g5bc468d7+xa1cf` (branch `chaos-test`, experiment `chaos`, md5
`e0ad950e…`), staged from `~/redox-rpi-chaos`. Real Pi 3B+ over netboot, no HDMI connected.
Reset at 22:22:49 UTC, `devshd: listening` after 20 s.

**Result: the Pi hung completely in stressor round 2.** Steps 3 and 4 were not run.

- **(1) pi-test:** pass (manifest matches, 0 of 300 starts failed).
- **(2) Stressor,** `chaos-host.py … --faults stressor --stressor-rounds 40 --stressor-kinds
  memory,procs,threads,fds,schemes,fill,killio --rounds 2 --seed 237`:
  - **Round 1 ok:** `chaos --rounds 40 --seed 142046529496316 --quiet --kind …` printed
    `done: 40 rounds, 0 failed, 384 s, free-mem=504MiB`, and devsh was back after 1 s
    (jsonl line at 22:30:33).
  - **Round 2 hang:** `chaos --rounds 40 --seed 152374357964539 --quiet --kind
    memory,procs,threads,fds,schemes,fill,killio` started at about 22:30:35 and never
    finished.
  - At 23:26 devsh no longer accepted connections (connect timeout). I stopped chaos-host,
    so there is no jsonl line for round 2.
  - The serial log has nine `kernel::context::memory:DEBUG -- Lacks grant` lines after the
    login prompt, and nothing after them: no panic, no `oom`, no crash dump.
  - The serial console did not even echo typed characters (`root` + Enter, 120 s), so the
    whole system was stuck, not only the network.
  - Repeat on the Pi: `chaos --rounds 40 --seed 152374357964539 --quiet --kind
    memory,procs,threads,fds,schemes,fill,killio`, or from the host
    `scripts/chaos-host.py 192.168.178.148 --faults stressor --stressor-rounds 40
    --stressor-kinds memory,procs,threads,fds,schemes,fill,killio --seed 237 --rounds 2`.
- **(3) signals and (4) host faults: not run** (the Pi was hung).
  - For (4): homenodellm has **no iptables** (`sudo: 'iptables': command not found`), so
    the `tcp-reset` fault could not run in any case.
  - `tc` was saved before (`fq_codel` on enp12s0) and is still `fq_codel`; no netem was
    ever applied.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a Shelly power cycle at 23:30:02:
  banner after 39 s, devsh answers, and `/srv/tftp/redox` matches the backup.

## #237 part B: chaos test steps 3 and 4 on a fresh boot (2026-11-04/05, Pi test agent)

The same image `v0.4.0-dev-346-g5bc468d7+xa1cf`, booted fresh. Reset at 23:31:58 UTC,
`devshd: listening` after 20 s, no HDMI connected. The stressor was skipped, as the
coordinator asked: the chaos session is chasing the part A hang in QEMU.

**Result: signals fail as #205 says. Host faults pass 19 of 20 rounds, but after round 20 the
network was lost** (#92 picture). No panic.

- **(3) `chaos --kind signals --rounds 3 --seed 237`** over devsh: **3 of 3 rounds FAIL**,
  exit 6, rc=1.
  - **0 signals received every time**, although the senders sent between 997 and 109342
    each. Round 1 had 3 senders for 873 ms (SA_RESTART); round 2, 3 senders for 2469 ms;
    round 3, 1 sender for 1352 ms.
  - Repeat commands: `chaos --kind signals --round-seed 0x94d1c519db3ce3b8`,
    `… 0xff17862427248c42`, `… 0x903c52a17da8d2`.
  - devsh was fine afterwards. This confirms #205 on real hardware.
- **(4) Host faults:** `chaos-host.py … --faults
  server,pkg,dwc2d,netem-loss,netem-delay,netem-reorder,netem-blackout --netem --rounds 20
  --seed 237`, 23:33:15–00:00:56.
  - No `tcp-reset`, because homenodellm has no iptables.
  - tc ran through `sudo -n` only for 192.168.178.148, and a trap put enp12s0 back.
- **Rounds 1–19 ok.** devsh was back after 1–6 s each time.
  - netem-delay ×4, netem-reorder ×5, netem-loss ×4, pkg ×2, server ×3, dwc2d ×1.
  - The dwc2d round power-cycled the LAN7800 port, and the network came back.
  - The server served bad-length 5, drip 4, endless 4, half 5, huge-header 6, stall 5,
    TLS garbage/reset and others.
  - netem-blackout was never drawn by this seed.
- **Round 20 FAIL:** `netem-delay` (400 ms ±200 ms) for 85 s.
  - Only 1 of 9 devsh commands answered during the fault.
  - After the delay was removed, **devsh did not come back within 180 s** (round ended after
    355 s), and then `No route to host`.
  - On serial, the shell answered (`echo hi`), but `ping -c 1 192.168.178.1` printed its
    first line and hung, and the next command got no prompt either.
  - That is the #92 picture: netstack stuck after heavy delay, with no error or panic line.
  - Repeat from the host: `scripts/chaos-host.py 192.168.178.148 --faults
    server,pkg,dwc2d,netem-loss,netem-delay,netem-reorder,netem-blackout --seed 237
    --rounds 20` (with `--netem`).
- **tc:** `tc qdisc show` after the run is the same as before (`fq_codel` on enp12s0, diff
  empty). No iptables change was possible.
- The jsonl is at homenodellm `~/pitest-tools/t237b/chaos-237b.jsonl`.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a Shelly power cycle at 00:06:00:
  banner after 30 s, devsh answers, and `/srv/tftp/redox` matches the backup.

## #238 #207 fix for the chaos-test freeze (PR #209) (2026-11-05, Pi test agent)

Image `v0.4.0-dev-352-gc582b16d+x6c52` (`chaos-test` + `tlb-switch-shootdown`, experiments
`chaos kernel-switch-shootdown kernel-tlb-debug`, md5 `5bca177b…`). Real Pi 3B+ over netboot,
no HDMI connected. Reset at 01:27:37 UTC, `devshd: listening` after 20 s.

**Result: fail. The Pi froze again in step 2, with the same seed as #237. No
`TLB shootdown stalled` line came.**

- **(1) pi-test:** pass (manifest matches, 0 of 300 starts failed).
- **(2)** `chaos --rounds 40 --seed 152374357964539 --quiet --kind
  memory,procs,threads,fds,schemes,fill,killio` over devsh started at 01:28:58 and **never
  finished**.
  - The serial log has 8 `kernel::context::memory:DEBUG -- Lacks grant` lines after the login
    prompt (silent from about 01:30), then nothing. That is the same as #237.
  - **No `TLB shootdown stalled` line and no `CPU #N: in set …` lines**, although the
    `kernel-tlb-debug` experiment is in the image (`experiments: chaos
    kernel-switch-shootdown kernel-tlb-debug` in the banner). Its 5 s repeat never appeared.
  - At 01:47, devsh gave `No route to host`. The serial console did not echo `root` + Enter.
  - No panic, no `oom`.
- **(3) threads loop and (4) chaos-host stressor: not run.**
- Either the fix in #209 does not cover what freezes the Pi, or this is a second cause.
  The stall detector didn't report anything, so either the freeze is not in a TLB shootdown
  wait, or the detector itself can't run once it happens (for example, if every CPU is stuck
  with interrupts off).
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a Shelly power cycle at 01:47:48:
  banner after 32 s, devsh answers, and `/srv/tftp/redox` matches the backup.

## #239 #58 cputop at idle: who keeps the Pi busy (2026-11-05, Pi test agent)

Image `v0.4.0-dev-394-g26cfa3f+xe828` (branch `cputop-58`, experiments
`timesync rtl8152 cputop`, md5 `9a17615f…`). Real Pi 3B+ over netboot, no HDMI connected.
Reset at 01:49:36 UTC, `devshd: listening` after 20 s.

**Result: pass.** `sys:cputime` answers, `WAKE/s` has numbers, and there was no panic.
At idle the Pi is 98 % idle on cpu1 and close to 100 % on the others. **dwc2d is the main
waker**, with 2.5 % CPU and about 77 wakeups/s, all on cpu1.

- **pi-test:** pass (manifest matches, 0 of 300 starts failed).
- 5 minutes idle (01:50:51–01:55:51), then the two cputop rounds over devsh.
- **Top 5 by CPU %:**
  1. dwc2d 2.5 % (78/s)
  2. rpi-firmwared 0.3 % (9/s)
  3. netstack 0.1 % (29/s)
  4. redoxfs (initfs) 0.1 % (1/s)
  5. ptyd 0.0 % (10/s)
- **Top 5 by wakeups/s:**
  1. dwc2d 76
  2. [kmain] cpu1 62
  3. netstack 27
  4. [kmain] cpu3 10
  5. rpi-firmwared 9 (ptyd 9 as well)
- **Busy per core:** cpu0 0 %, cpu1 2 % (1681 ms in 60 s, 300 switches/s), cpu2 0 %,
  cpu3 0 % (211 ms, 188 switches/s). cpu0 has 107 switches/s and cpu2 169.
- **Interrupts per second:** 108/s in all; line 3 (the timer) 100/s, line 50 8/s (7/s in
  the second round). `/scheme/sys/irq` lists only lines 3 and 50 as non-zero (48599 and 4258,
  then 49743 and 4393 about 16 s later); every other line up to 223 is 0.

`cputop -l 15 60`:

```text
cputop: 60.0 s, 45 threads
cpu0  busy   0 %  (0 ms busy, 60001 ms idle, 107 switches/s)
cpu1  busy   2 %  (1681 ms busy, 58320 ms idle, 300 switches/s)
cpu2  busy   0 %  (4 ms busy, 60000 ms idle, 169 switches/s)
cpu3  busy   0 %  (211 ms busy, 59795 ms idle, 188 switches/s)
irq   108/s  (3: 100, 50: 8)
PID   CPU%  WAKE/s        TOTAL STAT  CPU  NAME
16    2.5      78    0:00:10.7 UB    #1   /scheme/initfs/bin/dwc2d
8    0.3       9    0:00:01.3 US    #3   /scheme/initfs/bin/rpi-firmwared
25    0.1      29    0:00:00.6 UB    #1   /usr/bin/netstack
20    0.1       1    0:00:32.3 UB    #1   /scheme/initfs/bin/redoxfs
21    0.0      10    0:00:00.2 UB    #1   /usr/bin/ptyd
366    0.0       0    0:00:00.0 UR+   #2   /usr/bin/cputop
0    0.0       1    0:00:00.3 UB    #2   initnsmgr
35    0.0       1    0:00:00.0 UB    #1   /usr/bin/devshd
8    0.0       0    0:00:00.0 US    #3   /scheme/initfs/bin/rpi-firmwared
0    0.0      62    0:00:00.0 RR+   #1   [kmain]
0    0.0      10    0:00:00.0 RR+   #3   [kmain]
0    0.0       0    0:00:00.0 RR    #2   [kmain]
27    0.0       0    0:00:00.5 UB    #2   /usr/bin/getty
0    0.0       0    0:00:00.4 UB    #2   procmgr
2    0.0       0    0:00:00.3 UB    #1   /scheme/initfs/bin/logd
```

`cputop -w -l 15 60`:

```text
cputop: 60.0 s, 45 threads
cpu0  busy   0 %  (4 ms busy, 60008 ms idle, 107 switches/s)
cpu1  busy   2 %  (1680 ms busy, 58329 ms idle, 297 switches/s)
cpu2  busy   0 %  (0 ms busy, 60012 ms idle, 168 switches/s)
cpu3  busy   0 %  (208 ms busy, 59805 ms idle, 187 switches/s)
irq   107/s  (3: 100, 50: 7)
PID   CPU%  WAKE/s        TOTAL STAT  CPU  NAME
16    2.5      76    0:00:12.3 UB    #1   /scheme/initfs/bin/dwc2d
0    0.0      62    0:00:00.0 RR+   #1   [kmain]
25    0.1      27    0:00:00.7 UB    #1   /usr/bin/netstack
0    0.0      10    0:00:00.0 RR+   #3   [kmain]
8    0.3       9    0:00:01.5 US    #3   /scheme/initfs/bin/rpi-firmwared
21    0.0       9    0:00:00.2 UB    #1   /usr/bin/ptyd
0    0.0       1    0:00:00.3 UB    #0   initnsmgr
20    0.1       1    0:00:32.4 UB    #1   /scheme/initfs/bin/redoxfs
35    0.0       1    0:00:00.0 UB    #1   /usr/bin/devshd
8    0.0       0    0:00:00.0 US    #3   /scheme/initfs/bin/rpi-firmwared
0    0.0       0    0:00:00.0 RR    #0   [kmain]
0    0.0       0    0:00:00.4 UB    #1   procmgr
371    0.0       0    0:00:00.0 UR+   #0   /usr/bin/cputop
3    0.0       0    0:00:00.0 UB    #0   /scheme/initfs/bin/randd
17    0.0       0    0:00:00.3 UB    #1   /scheme/initfs/bin/bcm2835-sdhos
```

- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 18 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #240 #207 which chaos round freezes the Pi (2026-11-05, Pi test agent)

Image `v0.4.0-dev-411-gfaac52d7+x6c52` (`t207-test` = `chaos-test` + `tlb-switch-shootdown`
0ff030fc, experiments `chaos kernel-tlb-debug kernel-switch-shootdown`, md5 `83fca0bd…`).
Real Pi 3B+ over netboot, no HDMI connected. 1 freeze, so within the cap of 4. Stable was
up by 02:23.

**Result: one freeze, in a `threads` round (round 4), with no `Lacks grant` or TLB line. It
did not happen again.** The same seed passed all 40 rounds on a later boot. Rounds 1–4 alone
passed, and so did that exact threads round alone.

- **pi-test:** pass (manifest matches, 0 of 300 starts failed).
- **(2) Boot A (reset 01:59:23):** `chaos --rounds 40 --seed 152374357964539 --kind
  memory,procs,threads,fds,schemes,fill,killio > /scheme/debug` at 02:00:38. The last
  `chaos:` lines before the freeze:

```text
chaos: start seed=0x8a956b7ab6fb rounds=40 timeout=60s mem-reserve=48MiB free-mem=603MiB
chaos: round 1/40 kind=fds seed=0xb3de112de9c5e48
chaos:   fds: 4096 open when stopped (limit)
chaos: round 1 ok (535 ms)
chaos: round 2/40 kind=fill seed=0xd5c789b4923dee9
chaos:   fill: wrote 482 MiB in 45022 ms of 516 MiB free (out of time, not full)
chaos:   fill: free after delete 516 MiB
chaos: round 2 ok (45310 ms)
chaos: round 3/40 kind=memory seed=0x642d11f9a2ec84e
chaos:   memory: 464 MiB allocated (reserve), free before 508 MiB, at peak 43 MiB
chaos: round 3 ok (728 ms)
chaos: round 4/40 kind=threads seed=0x1c478a6213952b27
```

  - After that, nothing: **no `Lacks grant: page …` line, no `TLB shootdown stalled`, no
    `CPU #N: in set …`**, and no panic.
  - The serial log was silent for 3 min, and devsh did not answer twice. Freeze detected at
    02:04:42, then a power cycle.
- **(3) One kind per fresh boot** (power cycle each time), all `> /scheme/debug`, none froze:
  - `chaos --kind memory --rounds 5`: 5 ok.
  - `chaos --kind fill --rounds 3`: 3 ok (45 s each).
  - `chaos --kind fds --rounds 5`: 5 ok.
  - No `Lacks grant` line in any of them. `--mem-reserve 200` was not needed (memory did not
    freeze).
- **Extra, to narrow it down** (fresh boots, freeze 1 of 4, well before the 03:05 limit):
  - The exact round alone, `chaos --kind threads --round-seed 0x1c478a6213952b27`: ok
    (199 threads in 5 waves). Then `chaos --kind threads --rounds 5`: 5 ok.
  - The same seed with `--rounds 4` (fds, fill 461 MiB, memory 464 MiB, threads): **4 ok**.
  - **The same 40-round seed again: 40 of 40 ok** (332 s; 9 threads, 7 fill, 6 killio,
    5 each of memory, procs and schemes, 3 fds).
  - The only `Lacks grant` lines came in that run's round 18 (`schemes`, seed
    `0x395ed8a72165b4f`): `Lacks grant: page 0x216000, Read, CPU #3` and
    `Lacks grant: page 0x2a8000, Read, CPU #3`, 4 times each, without harm (QEMU saw the
    same in schemes rounds).
- **So:** the freeze is intermittent. It struck once in round 4 (`threads`) right after the
  fds, fill and memory rounds. It is not tied to that round's seed, and in this freeze the
  `Lacks grant` lines did not appear at all. #237 and #238 also ended with `Lacks grant`
  lines. Here they only came from a `schemes` round that did not freeze, so they may be
  unrelated noise.
  - No `TLB shootdown stalled` line in any boot. If the freeze is in the kernel with
    interrupts off on every CPU, that detector cannot fire.
- The per-boot serial logs are on homenodellm in `~/pitest-tools/t240/serial-*.log`.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset at the end: banner
  after 28 s, devsh answers, and `/srv/tftp/redox` matches the backup.

## #242 #211 dwc2d wakeups at idle, before and after (2026-11-05, Pi test agent)

Second try with new images, both built on main 542e0947. The first try at 03:33 got no
network on the before image (`v0.3.0-dev-1390-gc9b8cdf+xf5e2`: no `devshd: listening`).
Real Pi 3B+ over netboot, no HDMI connected, NanoKVM USB unplugged.
- **Before:** `v0.3.0-dev-1399-g0b9cd6c+xf5e2` (`timesync cputop`), reset at 05:37:21.
- **After:** `v0.3.0-dev-1399-g0b9cd6c+xceb2` (`timesync cputop dwc2d-idle`), reset at 05:47:59.
- Both reached `devshd: listening` after 20 s.

**Result: pi-test passes on both, and dwc2d is not higher after. But it is also not
lower:** 2.5 % CPU and 75–76 wakeups/s against 2.5–2.6 % and 76–78/s. `/tmp/dwc2d-wakeups`
shows why. The main source is `time` at 50.0/s (6.6 s busy in 300 s); the hotplug check
is only 1.0/s.
- **Hotplug:** both port cycles happened (serial log), and devsh came back. The script
  reports FAIL for cycle 2 because of a timing race (details below).

| | before | after |
|---|---|---|
| dwc2d CPU % | 2.5 / 2.6 / 2.5 (‑w: 2.6) | 2.5 / 2.5 / 2.5 (‑w: 2.5) |
| dwc2d wakeups/s | 76 / 78 / 76 (‑w: 77) | 75 / 75 / 76 (‑w: 76) |
| netstack wakeups/s | 26 / 29 / 27 (‑w: 28) | 26 / 27 / 28 (‑w: 27) |
| interrupts | 107/s: line 3 100/s, line 50 7/s (all rounds) | 107/s: line 3 100/s, line 50 7/s (all rounds) |
| busiest core | cpu3 2 % (1622–1706 ms), others 0 % | 2 % on the core dwc2d runs on (cpu0 or cpu3, 1596–1674 ms), others 0 % |

- **IRQ line 50 (LAN7800):** 7/s before and after, so it did not change.
- **`/tmp/dwc2d-wakeups` (after)**, after the 5 idle minutes:

```text
seconds 300
irq 2158 7.2/s busy 53 ms
scheme 5730 19.1/s busy 73 ms
time 14996 50.0/s busy 6558 ms
hotplug 297 1.0/s busy 559 ms
hotplug cycle-request busy 79 ms
hotplug scan busy 445 ms
hotplug links busy 26 ms
hotplug select-tick busy 3 ms
```

  and after the cputop rounds:

```text
seconds 540
irq 4025 7.5/s busy 101 ms
scheme 10841 20.1/s busy 145 ms
time 26998 50.0/s busy 11822 ms
hotplug 534 1.0/s busy 1031 ms
hotplug cycle-request busy 148 ms
hotplug scan busy 801 ms
hotplug links busy 66 ms
hotplug select-tick busy 6 ms
```

- **Hotplug** (`pi-hotplug-test.sh 192.168.178.148 2`):
  - Cycle 1 passes fully: port off, device removed, found again, network on the adapter,
    devshd back after 9 s.
  - Cycle 2: the serial log has the whole sequence again (`cycle: hub 2 port 1 power off`,
    `disconnected after 104 ms`, `device at address 6 removed`, `power on`,
    `LAN7800 at address 6`, `network on the adapter`, `link up`).
  - But the script got "devshd answers again" after only 2 s, before dwc2d had read the
    request (it now looks every 3 s), so it checked the log too early. It reports 4 FAILs.
  - This is a race in the script, not a USB failure. The script should wait for the
    `power off` line, or wait more than 3 s before polling devsh. devsh worked afterwards
    (the stable reset went over devsh).
- **Full cputop output:** homenodellm `~/pitest-tools/t242b/cputop*-{before,after}.txt`. The
  `-w` rounds:

before:

```text
cputop: 60.0 s, 45 threads
cpu0  busy   0 %  (191 ms busy, 59814 ms idle, 125 switches/s)
cpu1  busy   0 %  (6 ms busy, 60000 ms idle, 168 switches/s)
cpu2  busy   0 %  (0 ms busy, 60006 ms idle, 168 switches/s)
cpu3  busy   2 %  (1706 ms busy, 58298 ms idle, 297 switches/s)
irq   107/s  (3: 100, 50: 7)
PID   CPU%  WAKE/s        TOTAL STAT  CPU  NAME
16    2.6      77    0:00:13.8 UB    #3   /scheme/initfs/bin/dwc2d
0    0.0      62    0:00:00.0 RR+   #3   [kmain]
25    0.1      28    0:00:00.7 UB    #3   /usr/bin/netstack
21    0.0      10    0:00:00.1 UB    #3   /usr/bin/ptyd
0    0.0       9    0:00:00.0 RR    #0   [kmain]
8    0.3       9    0:00:01.6 US    #0   /scheme/initfs/bin/rpi-firmwared
0    0.0       1    0:00:00.1 UB    #1   initnsmgr
20    0.1       1    0:00:01.2 UB    #3   /scheme/initfs/bin/redoxfs
8    0.0       0    0:00:00.0 US    #0   /scheme/initfs/bin/rpi-firmwared
0    0.0       0    0:00:00.0 RR    #1   [kmain]
0    0.0       0    0:00:00.0 UB    #3   procmgr
50    0.0       0    0:00:00.0 UR+   #1   /usr/bin/cputop
3    0.0       0    0:00:00.0 UB    #1   /scheme/initfs/bin/randd
35    0.0       0    0:00:00.0 UB    #3   /usr/bin/devshd
33    0.0       0    0:00:00.0 UB    #0   /usr/bin/redoxfs
```

after:

```text
cputop: 60.0 s, 45 threads
cpu0  busy   2 %  (1596 ms busy, 58416 ms idle, 229 switches/s)
cpu1  busy   0 %  (32 ms busy, 59981 ms idle, 112 switches/s)
cpu2  busy   0 %  (200 ms busy, 59814 ms idle, 127 switches/s)
cpu3  busy   0 %  (13 ms busy, 60000 ms idle, 124 switches/s)
irq   107/s  (3: 100, 50: 7)
PID   CPU%  WAKE/s        TOTAL STAT  CPU  NAME
16    2.5      76    0:00:13.8 UB    #0   /scheme/initfs/bin/dwc2d
0    0.0      57    0:00:00.0 RR    #0   [kmain]
25    0.1      27    0:00:00.7 UB    #0   /usr/bin/netstack
21    0.0       9    0:00:00.1 UB    #3   /usr/bin/ptyd
8    0.3       9    0:00:01.7 US    #2   /scheme/initfs/bin/rpi-firmwared
0    0.0       9    0:00:00.0 RR+   #2   [kmain]
0    0.0       7    0:00:00.0 RR+   #3   [kmain]
20    0.0       2    0:00:01.4 UB    #0   /scheme/initfs/bin/redoxfs
0    0.0       2    0:00:00.0 RR+   #1   [kmain]
10    0.0       1    0:00:00.0 UB    #0   /scheme/initfs/bin/lived
0    0.0       0    0:00:00.1 UB    #0   initnsmgr
8    0.0       0    0:00:00.0 US    #0   /scheme/initfs/bin/rpi-firmwared
0    0.0       0    0:00:00.0 UB    #0   procmgr
55    0.0       0    0:00:00.0 UR+   #0   /usr/bin/cputop
3    0.0       0    0:00:00.0 UB    #3   /scheme/initfs/bin/randd
```

- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 20 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #241 #207 catch the rare chaos freeze with lock watches (2026-11-05, Pi test agent)

Image `v0.4.0-dev-413-g7672b842+x6c52` (`t207-test` = `chaos-test` + `tlb-switch-shootdown`
16d700a1, experiments `chaos kernel-switch-shootdown kernel-tlb-debug`, md5 `52119076…`).
Real Pi 3B+ over netboot, no HDMI connected. It ran in three blocks around the 03:30
nightly and row 242.

**Result: no freeze in 34 loops (1360 rounds).** There were no `lock wait stalled`,
`TLB shootdown stalled` or `CPU #N: in set …` lines, and no panic. The freeze of #237, #238
and #240 did not show up.

- **(1) pi-test:** pass (manifest matches, 0 of 300 starts failed).
- **(2) Seed 152374357964539** (`for i in 1..=15; echo LOOP $i > /scheme/debug; chaos
  --rounds 40 --seed 152374357964539 --kind memory,procs,threads,fds,schemes,fill,killio
  --quiet > /scheme/debug; end`):
  - **Block 1**, boot 02:37:34: 4 loops `done: 40 rounds, 0 failed` (332–338 s each).
    Loop 5 was stopped at 03:05 for the nightly.
  - **Block 2**, fresh boot 04:10:44: **15 of 15 loops** done, 04:11–05:35.
  - So 19 complete loops of this seed, each starting with the #240 sequence (fds, fill,
    memory, threads).
- **(3) Seed 241**, the same loop, fresh boot 06:01:00: **15 of 15 loops**
  `done: 40 rounds, 0 failed` (246 s each), 06:01–07:03.
- **Serial lines:**
  - Only `kernel::context::memory:DEBUG -- Lacks grant` pairs: `page 0x216000, Read` and
    `page 0x2a8000, Read`, on varying CPUs, in schemes rounds. Seed 241 had 15 of each.
  - Plus once a new one, **`Lacks grant: page 0x3aa000, Write, CPU #2`**, in seed 241
    round 3 (`threads`, seed `0xa60e4d449c5baf8a`) of one loop, without harm.
  - `lock wait stalled` / `TLB shootdown stalled`: 0 in all three blocks.
- The nightly at 03:30 skipped its work, because this row was still marked "on the Pi"
  (see the coordinator's note). From now on, paused rows get "paused".
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored after each block with a devsh reset, last
  at 07:04: devsh answers, and `/srv/tftp/redox` matches the backup.

## #243 #211 dwc2d idle with the keyboard poll on the interrupt (2026-11-05, Pi test agent)

Image `v0.3.0-dev-1409-gb6b5aef+xceb2` (`t-idle-cputop` b6b5aef, experiments
`timesync cputop dwc2d-idle`, md5 `3d28f053…`). Real Pi 3B+ over netboot, no HDMI connected.
The person part was not done (nobody at the Pi). Reset at 07:05:23,
`devshd: listening` after 20 s.

**Result: fail.**
- After the second network-port cycle, the LAN7800 was never found again: the network was
  lost and the Pi needed a power cycle.
- dwc2d's CPU is lower (1.2 % against 2.5 % in #242). But its wakeups went up to 237–479/s,
  IRQ line 50 to 168–408/s, and the keyboard poll has 619 timeouts (not near 0).

- **pi-test:** pass (manifest matches, 0 of 300 starts failed).
- **cputop after 5 min idle** (`-n 3 60`, three rounds):
  - dwc2d 1.2 % / 237, 261, 258 wakeups/s; netstack 29–31/s.
  - irq 268, 292, 287/s (line 50: 168, 192, 187; line 3: 100).
  - cpu1 1 % (886–940 ms), others 0 % (cpu3 about 220 ms).
  - Switches per second went up to 768–848 on cpu1 (#242: about 290).
- **`cputop -w -l 15 60`:**
  - dwc2d 1.7 % / 479 wakeups/s, [kmain] cpu3 460/s, netstack 31/s.
  - irq 508/s (line 50: 408).
  - cpu3 2 % busy (1498 ms), 1495 switches/s.

```text
cputop: 60.0 s, 45 threads
cpu0  busy   0 %  (11 ms busy, 59994 ms idle, 521 switches/s)
cpu1  busy   0 %  (3 ms busy, 60005 ms idle, 563 switches/s)
cpu2  busy   0 %  (0 ms busy, 60003 ms idle, 563 switches/s)
cpu3  busy   2 %  (1498 ms busy, 58507 ms idle, 1495 switches/s)
irq   508/s  (50: 408, 3: 100)
PID   CPU%  WAKE/s        TOTAL STAT  CPU  NAME
16    1.7     479    0:00:08.2 UB    #3   /scheme/initfs/bin/dwc2d
0    0.0     460    0:00:00.0 RR    #3   [kmain]
25    0.2      31    0:00:00.8 UB    #3   /usr/bin/netstack
8    0.3      10    0:00:01.7 US    #3   /scheme/initfs/bin/rpi-firmwared
21    0.0       9    0:00:00.1 UB    #3   /usr/bin/ptyd
0    0.0       6    0:00:00.0 RR+   #0   [kmain]
36    0.0       2    0:00:00.0 UB    #0   /usr/bin/devshd
20    0.1       2    0:00:01.2 UB    #3   /scheme/initfs/bin/redoxfs
10    0.0       1    0:00:00.0 UB    #3   /scheme/initfs/bin/lived
0    0.0       0    0:00:00.1 UB    #3   initnsmgr
8    0.0       0    0:00:00.0 US    #0   /scheme/initfs/bin/rpi-firmwared
0    0.0       0    0:00:00.0 UB    #3   procmgr
50    0.0       0    0:00:00.0 UR+   #3   /usr/bin/cputop
0    0.0       0    0:00:00.0 RR+   #1   [kmain]
3    0.0       0    0:00:00.0 UB    #1   /scheme/initfs/bin/randd
```

- **`/tmp/dwc2d-wakeups`** after the cputop rounds (540 s since boot):

```text
seconds 540
irq 182748 338.2/s busy 2198 ms
scheme 11970 22.2/s busy 161 ms
time 28027 51.9/s busy 2180 ms
hotplug 534 1.0/s busy 1025 ms
hotplug cycle-request busy 149 ms
hotplug scan busy 804 ms
hotplug links busy 56 ms
hotplug select-tick busy 6 ms
keyboard polls 25984 reports 0 timeouts 619 start busy 1867 ms
```

  - **keyboard polls 25984, reports 0, timeouts 619** (2.4 %). The irq source is now
    338/s (#242: 7.5/s).
  - The second `cat` after the hotplug test got no answer (no network).
- **Hotplug** (`KEYBOARD=1 pi-hotplug-test.sh 192.168.178.148 2`, from `~/redox-rpi-usb`):
  - **Cycle 1 passes:** port off, removed, LAN7800 found again, network on the adapter,
    devshd back after 8 s.
  - **Cycle 2 fails:** `cycle: hub 2 port 1 power off`, `disconnected after 101 ms`,
    `device at address 6 removed`, `cycle: hub 2 port 1 power on`. **Then nothing more in
    the serial log**: no `device connected`, no `LAN7800 at address`, and no other dwc2d
    line.
  - devshd did not come back (the script gave up after 258 s). The keyboard cycle could
    not be requested (3× FAIL).
  - This is not the early-check issue of #242: the device really did not come back.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a Shelly power cycle at 07:29:18
  (no network for the devsh reset): banner after 30 s, devsh answers, and `/srv/tftp/redox`
  matches the backup.

## #244 #126 eth-speed request every 5 s (PR #215) (2026-11-05, Pi test agent)

Image `v0.4.0-dev-437-gb0f6b8e+x23c2` (`eth-link-check` + `cputop-58`, experiments
`eth-speed cputop`, md5 `d3d2de48…`; there is no manifest file next to it, so pi-test ran
without `--manifest`). Real Pi 3B+ over netboot, no HDMI connected. Reset at 07:56:06 UTC,
`devshd: listening` after 20 s.

**Result: pass on what the row asks.** pi-test passes, and both speed changes come within
the 5 s check plus negotiation, with no panic. **But idle wakeups are higher than in
row 239, not lower:** dwc2d 110/s (239: 76), netstack 60/s (27–29), IRQ line 50 19/s (7–8).
dwc2d's CPU stays at 2.6 %.

- **pi-test:** pass (version, manifest present in the image, ps, ps-tty, sdcard,
  0 of 300 starts failed).
- **`cputop -w -l 10 60`** after 5 min idle:

```text
cputop: 60.0 s, 44 threads
cpu0  busy   0 %  (21 ms busy, 59991 ms idle, 137 switches/s)
cpu1  busy   0 %  (201 ms busy, 59815 ms idle, 189 switches/s)
cpu2  busy   0 %  (0 ms busy, 60017 ms idle, 170 switches/s)
cpu3  busy   2 %  (1793 ms busy, 58225 ms idle, 315 switches/s)
irq   119/s  (3: 100, 50: 19)
PID   CPU%  WAKE/s        TOTAL STAT  CPU  NAME
16    2.6     110    0:00:11.2 UB    #3   /scheme/initfs/bin/dwc2d
0    0.0      66    0:00:00.0 RR+   #3   [kmain]
25    0.3      60    0:00:01.3 UB    #3   /usr/bin/netstack
0    0.0      10    0:00:00.0 RR+   #1   [kmain]
8    0.3       9    0:00:01.3 US    #1   /scheme/initfs/bin/rpi-firmwared
0    0.0       9    0:00:00.0 RR    #0   [kmain]
21    0.0       9    0:00:00.2 UB    #0   /usr/bin/ptyd
0    0.0       1    0:00:00.3 UB    #0   initnsmgr
20    0.1       1    0:00:32.4 UB    #3   /scheme/initfs/bin/redoxfs
8    0.0       0    0:00:00.0 US    #1   /scheme/initfs/bin/rpi-firmwared
```

- **Compared with #239:**
  - dwc2d 110/s (76), 2.6 % (2.5 %).
  - initnsmgr 1/s (1), redoxfs 1/s (1).
  - netstack 60/s (27–29), [kmain] on dwc2d's core 66/s (62).
  - irq 119/s: line 3 100, line 50 19 (239: 107–108 with line 50 7–8).
  - The extra line-50 interrupts and netstack wakeups suggest more Ethernet traffic in this
    minute, or more link polling, not the file check. This is one round only.
- **Speed changes on the serial console.** The times are from Enter, and the log was polled
  every 2 s, so they are accurate to about 2 s:
  - `echo 100 > /tmp/dwc2d-link`: `advertising up to 100 Mbit/s, negotiating again` after
    about 4 s, `link down`, then `link up on the LAN7800 at address 6, 100 Mbit/s` after
    about 8 s.
  - `echo 1000 > /tmp/dwc2d-link`: `advertising up to 1000 Mbit/s` after about 2 s, `link
    down`, then `link up … 1000 Mbit/s` after about 6 s. devsh answered again.
- No panic in the serial log.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 20 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #245 #126 A/B for PR #215: idle wakeups, noise or the image? (2026-11-05, Pi test agent)

Real Pi 3B+ over netboot, no HDMI connected, A and B back to back with nothing changed in
between.
- **A:** `v0.4.0-dev-436-gd5d8e81+x23c2` (before #215, md5 `317b7cbc…`), reset at 08:07:27.
- **B:** `v0.4.0-dev-437-gb0f6b8e+x23c2` (with #215, md5 `d3d2de48…`), reset at 08:17:00.
- Both had `devshd: listening` after about 20 s, pi-test passed with the manifest (0 of 300
  starts failed), and there was no panic.

**Result: the higher wakeups of #244 come from the base, not from #215.** A (without #215)
already has dwc2d at 113–116/s and netstack at 62–65/s. B has slightly fewer: dwc2d
106–109/s, netstack 57–60/s, line 50 18–19/s against 20–21/s. dwc2d's CPU is 2.6 % in B
against 2.6–2.7 % in A. #239's 76/s was on an older base
(`v0.4.0-dev-394-g26cfa3f`, `timesync rtl8152 cputop`).

| Round | dwc2d /s (CPU %) | netstack /s (CPU %) | initnsmgr /s | redoxfs /s (CPU %) | [kmain] dwc2d's core /s | irq total (line 50) |
|---|---|---|---|---|---|---|
| A1 | 113 (2.6) | 62 (0.3) | 2 | 2 (0.1) | 72 | 120 (20) |
| A2 | 116 (2.7) | 65 (0.3) | 2 | 2 (0.1) | 76 | 121 (21) |
| A3 | 113 (2.7) | 62 (0.3) | 2 | 2 (0.1) | 75 | 120 (20) |
| B1 | 107 (2.6) | 57 (0.3) | 1 | 1 (0.1) | 66 | 118 (18) |
| B2 | 109 (2.6) | 60 (0.3) | 1 | 1 (0.1) | 70 | 119 (19) |
| B3 | 106 (2.6) | 57 (0.3) | 1 | 1 (0.1) | 70 | 118 (18) |

- **Busy core:** A cpu2 3 % (2063–2123 ms in 60 s), B cpu1 2–3 % (1790–1827 ms). The other
  cores are 0 %, apart from about 210 ms on cpu2 in B.
- The full `cputop -w -n 3 -l 10 60` output for both is on homenodellm in
  `~/pitest-tools/t245/cputop-{before,after}.txt`.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 20 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #246 #126 which merge raised the idle wakeups? (2026-11-05, Pi test agent)

Three images (experiments `timesync rtl8152 cputop`, with PR #210's `cputop`), one after the
other on the real Pi 3B+ over netboot, no HDMI connected.
- **1** `v0.4.0-dev-418-g182177f+xe828`: before #203 and #194.
- **2** `v0.4.0-dev-427-gf00b4e7+x949e`: after #203, plus `eth-speed`.
- **3** `v0.4.0-dev-432-g8245e09+x949e`: after #194, plus `eth-speed`.

**Result: #194 (multicast on by default) raised the idle wakeups.** From 2 to 3, dwc2d
went from 82–84 to 108–112/s, netstack from 29–31 to 57–61/s, and IRQ line 50 from 8 to
18–19/s. #203 with `eth-speed` (1 to 2) adds only about 3 dwc2d wakeups/s. All three
booted, pi-test passed with the manifest (0 of 300 starts failed), and there was no panic.

- **First try (08:39–09:24):** image 1 booted to `redox login:` but **had no network**.
  - dwc2d was fine (LAN7800, link up, network address), but there was **no
    `devshd: listening`**, and timesync set the clock `from the build time`.
  - devsh did not answer for 8 min, so nothing could run. Images 2 and 3 were never booted
    (the reset goes over devsh).
  - This is the same picture as the first try of #242 this morning, with a different image.
  - Stable came back with a power cycle.
- **Second try (09:25–09:54):** with a fallback (power cycle once if there is no devshd
  in 3 min). It was not needed: all three images got the network on the first boot
  (`devshd: listening` after 20 s).
  - So the no-network boot is intermittent. Two of about 10 test-image boots today had it
    (#242 try 1, #246 try 1).

| Round | dwc2d /s (CPU %) | netstack /s (CPU %) | initnsmgr /s | redoxfs /s (CPU %) | [kmain] dwc2d's core /s | irq total (line 50) |
|---|---|---|---|---|---|---|
| 1-1 | 79 (2.5) | 30 (0.1) | 1 | 1 (0.1) | 59 | 108 (8) |
| 1-2 | 80 (2.5) | 31 (0.2) | 1 | 1 (0.1) | 63 | 108 (8) |
| 1-3 | 80 (2.5) | 31 (0.2) | 1 | 1 (0.1) | 63 | 108 (8) |
| 2-1 | 82 (2.6) | 29 (0.1) | 2 | 2 (0.1) | 58 | 108 (8) |
| 2-2 | 82 (2.5) | 30 (0.2) | 2 | 2 (0.1) | 59 | 108 (8) |
| 2-3 | 84 (2.6) | 31 (0.2) | 2 | 2 (0.1) | 60 | 108 (8) |
| 3-1 | 109 (2.6) | 58 (0.3) | 2 | 2 (0.1) | 70 | 118 (18) |
| 3-2 | 108 (2.6) | 57 (0.3) | 2 | 2 (0.1) | 74 | 118 (18) |
| 3-3 | 112 (2.6) | 61 (0.3) | 2 | 2 (0.1) | 75 | 119 (19) |

- **Busy core:**
  - Image 1: cpu2 2 % (1653–1687 ms in 60 s).
  - Image 2: cpu0 2 % (1706–1758 ms).
  - Image 3: cpu2 3 % (2069–2105 ms).
  - The other cores are about 0 % (about 210 ms on one core in 1 and 2).
- With #194, about 10 more LAN7800 interrupts per second arrive. Each one wakes dwc2d and
  netstack, which looks like multicast traffic on the LAN now being received (#194 opens the
  LAN7800's multicast filter).
- The full output is on homenodellm in `~/pitest-tools/t246b/cputop-{1,2,3}.txt`, and the
  no-network boot log in `~/pitest-tools/t246/serial-1.log`.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 21 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #247 #216 / #126 LAN7800 multicast hash filter, A/B (2026-11-05, Pi test agent)

Real Pi 3B+ over netboot, no HDMI connected. Both images are from one tree (experiments
`timesync rtl8152 cputop`, with PR #210's `cputop`):
- **A:** `v0.4.0-dev-455-ge763514+xe828`, without the filter.
- **B:** `v0.4.0-dev-455-ge763514+x4753`, with `lan78xx-mcast-hash`.

**Result: pass. The filter brings the idle wakeups back to the level from before #194.**
B logs **`dwc2d: LAN7800 multicast filter: 1 groups`** (no timeout or error line), and its
network works. Without the filter (A), dwc2d has 106–110 wakeups/s, netstack 57–62/s and
line 50 18–20/s. With it (B), the numbers are 80–82/s, 29–32/s and 8/s (#246 before #194:
80/s, 30/s, 8/s). No panic.

| Round | dwc2d /s (CPU %) | netstack /s (CPU %) | [kmain] dwc2d's core /s | irq total (line 50) |
|---|---|---|---|---|
| A1 | 106 (2.6) | 57 (0.3) | 65 | 118 (18) |
| A2 | 109 (2.7) | 60 (0.3) | 70 | 119 (19) |
| A3 | 110 (2.6) | 62 (0.3) | 71 | 120 (20) |
| B1 | 80 (2.5) | 29 (0.1) | 58 | 108 (8) |
| B2 | 81 (2.6) | 30 (0.2) | 59 | 108 (8) |
| B3 | 82 (2.5) | 32 (0.2) | 60 | 108 (8) |

- **Busy core:** A cpu1 2–3 % (1790–1862 ms in 60 s), B cpu0 2 % (1654–1696 ms). The other
  cores are about 0 % (about 200 ms on one core in both).
- **pi-test:** pass on both with the manifest (0 of 300 starts failed).
- **Ping:**
  - From the Pi, `ping -c 3 192.168.178.120` gave 3/3 on A and B.
  - From homenodellm (no `ping` binary there, so a raw ICMP echo via `sudo -n python3`), the
    Pi did not answer on A, on B, **or on stable dev-201** (tried before the test). So the
    Pi doesn't answer echo requests at all; this is not caused by the filter. The router
    answers the same probe.
- **A's first boot (10:24) had a network problem.** `devshd: listening`, and timesync got
  the time from pool.ntp.org at 10:24:42. But from the first devsh connect on (about
  10:24:47) homenodellm got `No route to host` (no ARP answer) for a minute, so nothing ran
  on A.
  - B was then started with a power cycle and was fine, and A's rerun at 10:42 was fine too.
  - That is the third intermittent boot-network failure today, after #242 try 1 and #246
    try 1. Those two had no `devshd: listening` at all.
- Full output on homenodellm: `~/pitest-tools/t247a/cputop-A.txt`, `t247/cputop-B.txt`.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: banner after 20 s,
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #224 open-source VPU firmware: does the open bootcode.bin start on our Pi 3B+? (2026-11-05, Pi test agent)

Card image `redox-rpi-openfw-3bp-20261104.img.xz` (sha256 `bd3537a5…`, `bootcode.bin`
`f22e42e8…`, BUILDATE Nov 4 2026 14:45:31). Real Pi 3B+. The user swapped the cards, and the
Pi test agent switched the plug. No HDMI connected.

**Result: the VPU, SDRAM and ARM bring-up work on the 3B+. The SD card is found, but
reading it fails, and the chainloader then crashes with `Undefined instruction` and
`panic()`.** The payload was not reached.

- (1) The plug was switched off at 11:34:02 UTC. (2) The user put in the open-firmware card.
- (3) Plug on at 11:34:43, 60 s of serial (150 lines, on homenodellm at
  `~/pitest-tools/t224/serial-openfw.raw`/`.log`), plug off at 11:35:43. In order:
  - `Booting Raspberry Pi....`, `Copyright 2016-2017 rpi-open-firmware authors`: the VPU is
    alive and the UART is up.
  - `[SDRAM:sdram_init]: SDRAM Type: Elpida 1GB LPDDR2 (BC=0x58)`, self test at
    0xC0000000–0xFFF00000 ok, **`SDRAM initialization completed successfully!`**.
  - Image, USB and UsbPhy power domains started. ARM code copied (57412 bytes), `ARM ID:
    0x364D5241`, `ARM clock succesfully initialized!`, async bridge `PM_PROC is now:
    0x107F`, `ARM, continuing boot from here ...`.
  - `[BRINGUP:main]: Firmware data: SDRAM_SIZE=0, VPU_CPUID=0x4000140`,
    `Security extensions are supported!`, **`Execution mode: Supervisor`**.
  - eMMC: `ERROR: sdhost status: 0x40`, 20× `waiting for SD`, then **`SD card has arrived!`**,
    `SD064 59MB SD Card`, switched to 25 MHz.
  - **`[EMMC:read_block]: ERROR: transfer error on FIFO word 113: 0x21`**, then
    `eMMC driver sucessfully started!` and `[MBRDISK:read_mbr]: Reading master boot record ...`.
  - **`Fatal Exception: Undefined instruction`** (pc `0x0000bfe0`, lr `0x00010009`,
    sp `0x40804801`, cpsr `0x600001d3`).
  - **Last line: `panic(): "Fatal CPU exception!"@trap.cc:47`**.
  - No `cmdline.txt`/`rpi.dtb`/`zImage` read, no `Jumping to the Linux kernel`, no test
    payload (`MIDR` etc.).
- (4) Off. (5) The user put the network-boot card back in. The plug was already off.
- (6) Plug on at 11:36:25. The stable `v0.4.0-dev-201-gb714bca+x9f76` booted to
  `redox login:` after 28 s, and devsh answers. **pi-test pass** (version, manifest, ps,
  ps-tty, sdcard, 0 of 300 starts failed).
- The full log went to the open-firmware session for `docs/topics/open-firmware.md`.

## #248 #197 RedoxFS fix `redoxfs-truncate` (PR #208): pkg install/remove rounds (2026-11-05, Pi test agent)

Image `t197-fix.img` (`boot-pkg-space` 6b650106, `v0.4.0-dev-373-g6b650106+x0247`,
experiments `timesync redoxfs-truncate`, md5 `191c2462…`), with the mirror
`/srv/tftp/pkg-mirror-t197`. Real Pi 3B+ over netboot, no HDMI connected. Reset at
11:39:11 UTC, devsh after 20 s.

**Result: fail. RedoxFS hangs during `pkg install vim`, 4 times.** The space fix itself
works: in the rounds that finished, the free space after each remove stayed at
124.6–124.8 MB, within 0.2 MB. Only 5 of 12 rounds finished.

- **(1) pi-test** `--starts 300`: pass (0 of 300 starts failed). `date`: Thu Nov 5 2026.
- **(2)** `50_redox` moved away, `40_mirror` set. **(3)** `df /`: 136764 KB free.
- **(4) Rounds**, free KB after install / after remove; a "hang" stopped at 25 min:

| Round | Boot | after install | after remove |
|---|---|---|---|
| 1 | 1 | **hang** (11:40–12:05) | – |
| 1 | 2 | **hang** (12:10–12:35) | – |
| 1 | 3 | 89308 | **124620** |
| 2 | 3 | 89244 | **124836** |
| 3 | 3 | 89172 | **124784** |
| 4 | 3 | **hang** (12:55–13:20) | – |
| 4 | 4 | 89304 | **124620** |
| 5 | 4 | 89240 | **124836** |
| 6 | 4 | **hang** (13:34–13:59), then stopped (more than 3 restarts) | – |

  - Every finished install and remove ended with `Commit complete.` (rc=0). The remove
    figures don't drop the way they did without the fix (QEMU 111.8, 98.4, …): rounds 1–3
    on one boot give 124620, 124836, 124784.
- **The hangs:** the last one (round 6) stopped at `Extracting vim [ 21%] … 493/2316` and
  never moved on. Each time, devsh stopped answering, but the serial shell still worked.
  `cat /scheme/sys/context` showed:
  - **`/scheme/initfs/bin/redoxfs` (pid 20, the network-boot root) in state `UR`**, with
    1:32 to 2:15 min of CPU and growing (99 MB in round 4).
  - `pkg` blocked (`UB`), with 1–3.5 min of CPU.
  - netstack blocked with little CPU (normal idle), dwc2d sleeping. No panic, no exception.
  - So the root filesystem was busy without end while `pkg` wrote vim's files. It was not
    a network loss (the mirror is local, and no `No route` came before the hang).
  - Each time I power-cycled, set the mirror up again and repeated the round.
  - This looks like the QEMU hang the row mentions (3 h in a `pkg remove`), but here it is
    in the install, and much more often: 4 times in 9 install attempts.
- **(5)** not run cleanly: after the last hang devsh did not answer, so `rm -r
  /tmp/pkg_download` and `df` failed.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a Shelly power cycle at 14:08:10:
  banner after 30 s, devsh answers, and `/srv/tftp/redox` matches the backup.
- The context dumps are on homenodellm in `~/pitest-tools/t248/diag-{1,4,6}.txt`.

## #249 open-source VPU firmware, rerun with the fixed bootcode.bin (2026-11-05, Pi test agent)

Card image `redox-rpi-openfw-3bp-20261105.img.xz` (sha256 checked). The fixes: a missing
`return` in `read_block<Mbr>`, SD read errors are now reported, and the SD driver retries at
half the clock. Real Pi 3B+, no HDMI connected. The user wrote and swapped the cards, and
the Pi test agent switched the plug for the second card and the return.

**Result: pass on both cards. The open firmware reaches the 32-bit test payload on the
Pi 3B+**: VPU → SDRAM → ARM → SD → MBR → FAT → `cmdline.txt`/`rpi.dtb`/`zImage` → `Jumping
to the Linux kernel...` → payload. No panic.

- **64 MB card** (the user switched on at about 14:33 UTC; log from serial.log, 174 lines):
  - `SD064 59MB SD Card`, 25 MHz. **Sector 0 failed once**
    (`transfer error on FIFO word 113: 0x21`, `status: 0x20`), then
    `read of sector 0 failed, retrying with SH_CDIV 0x12 (was 0x8)` worked.
  - MBR, `Boot partition mounted!`, `cmdline.txt` 32 bytes
    (`console=ttyAMA0,115200 earlycon`), `rpi.dtb` 35252 bytes (valid FDT at 0xF000000),
    `zImage` 444 bytes at 0x2000000, `Jumping to the Linux kernel...`.
  - Then `redox-rpi: test payload reached (32-bit ARM, loaded as zImage)`,
    `CPSR = 0x600001D3`, **`MIDR = 0x410FD034`**, `MPIDR = 0x80000000`,
    `r2 (FDT) = 0x0F000000`, **`done, waiting (wfe)`**.
  - The agent switched off at 14:34:50.
- **SDHC card** (plug on at 14:37:28, 60 s, 152 lines):
  - `Copyright 2016-2017 rpi-open-firmware authors`. `Detected SD card: Product N/A,
    CSD Ver 2.0, Capacity 62412800, Size 60949`, `N/A 1803MB SDHC Card`. Capacity 62412800
    blocks is about 32 GB, so the 1803 MB looks like an overflow in the size print.
  - **No read error and no retry** at 25 MHz. MBR, boot partition, the same three files,
    `Jumping to the Linux kernel...`, the same payload lines.
  - **Last line `redox-rpi: done, waiting (wfe)`**. Off at 14:38:39.
- **Network-boot card back:** plug on at 14:39:54. Stable `v0.4.0-dev-201-gb714bca+x9f76`
  booted to `redox login:` after 29 s, and devsh answers. **pi-test pass** (version,
  manifest, ps, ps-tty, sdcard, 0 of 300 starts failed).
- Logs on homenodellm: `~/pitest-tools/t249/serial-openfw-1433.log` (64 MB card) and
  `serial-2.log` (SDHC). The open-firmware session has the details.

## #250 #197 / #157 the hang of #248: with the RedoxFS fix only, and is it the scheduler? (2026-11-05, Pi test agent)

Real Pi 3B+ over netboot, no HDMI connected. Both images have `pi-heartbeat`:
- **A:** `t197-hbA.img`, `v0.4.0-dev-374-g81dc5797+xf45c`, without `redoxfs-truncate`,
  md5 `7401e17e…`.
- **B:** `t197-hbB.img`, `v0.4.0-dev-374-g81dc5797+x212a`, with it, md5 `5ad79d99…`.
- Run order A, B, A, B with 3 rounds each (14:50–16:05 UTC). Mirror `pkg-mirror-t197`.
- A hang was declared when an install or remove did not finish within 8 min (a normal one
  takes about 2.5 min).

**Result: hangs only with the RedoxFS fix (B: 2 in 6 rounds, A: 0 in 6). It is not the
#157 scheduler picture.** In both hangs no runnable context was waiting on an idle CPU:
every run queue was empty, and the dump came from the 60 s timer, not from the
runnable-on-idle check. The root's `redoxfs` (pid 20) was **blocked in
`UserInner::read`**, waiting for new requests. At the same time `pkg`, `ion`, `devshd` and
`login` were all **blocked in `UserInner::call`**, waiting for an answer. That looks like a
request (or its wakeup) lost between the kernel and RedoxFS. No exception or panic.

**Free space after each remove (KB):**
- A: 111612, 98532, 85188 | 111612, 98528, 85184. It drops about 13.3 MB per round, as
  expected without the fix.
- B: 124588, –, 124580 | 124580, 124796, –. Flat (the fix works); "–" is a hang.

| Part | Round 1 | Round 2 | Round 3 |
|---|---|---|---|
| A1 | ok | ok | ok |
| B1 | ok | **hang** in install | ok (after power cycle) |
| A2 | ok | ok | ok |
| B2 | ok | ok | **hang** in install |

- **Hang B1-2** (install from 15:11:08):
  - Last progress line `Extracting vim [ 38%] … 871/2316`.
  - `cat /scheme/sys/context`: initfs `redoxfs` pid 20 `UR` on #3, 52.8 s user CPU,
    99.1 MB. `pkg` pid 72, two threads `UB` (one with 1:44 CPU). `/usr/bin/redoxfs`
    (pid 32, `/scheme/data`) `UB`.
  - Heartbeat dump `[hbctx begin beat 14 timed]`:
    - `hbcpu 0 w1092 q0`, `hbcpu 1 w0 q0`, `hbcpu 2 w456 q0`, `hbcpu 3 w970 q0`.
    - `hbq` n0 on all four CPUs.
    - All four `[kmain] R+c` (idle).
    - `redoxfs` pid 20 `B sys cpu3 UserInner::read (v2)`.
    - `pkg` pid 72 `B sys cpu3 UserInner::call` and `B sys cpu1 futex`.
    - `ion` 73 `B UserInner::call`, `devshd` 35 (4 threads) `B UserInner::call`, `login`
      33/37/69 `B UserInner::call`, `netstack` `B UserInner::call`.
- **Hang B2-3** (install from 15:53:46):
  - Last progress line `Extracting vim [ 52%] … 1193/2316`.
  - Context: `redoxfs` pid 20 `UR` on #2, 1:36 CPU. `pkg` pid 88 `UB` (2:14 CPU).
  - Dump `[hbctx begin beat 20 timed]`: all `q0`, `hbq` n0, `[kmain] R+c` on all CPUs.
    `redoxfs` pid 20 `B sys cpu2 UserInner::read (v2)`. `ion` 86/89, `devshd`, `login`
    85, `netstack` `B UserInner::call`.
- **Note:** `context` (taken first) shows pid 20 as `UR`, but the heartbeat dump 1–2 min
  later shows it `B` in `UserInner::read`. So RedoxFS was busy at first, then went back to
  waiting while the callers kept waiting.
- Logs on homenodellm: `~/pitest-tools/t250/hang-{B1-2,B2-3}-install-{context.txt,serial.log}`,
  and per round `t250/{A,B}{1,2}-k-{install,remove}.txt`.
- After each hang, a power cycle and the mirror set up again. Stable
  `v0.4.0-dev-201-gb714bca+x9f76` restored at the end with a power cycle: devsh answers,
  and `/srv/tftp/redox` matches the backup.

## #252 #197 RedoxFS hang with `redoxfs-truncate`: did RedoxFS answer every request? (2026-11-05, Pi test agent)

Image `t197-hbC.img` (`v0.4.0-dev-375-g949f068d+xf366`, experiments
`timesync redoxfs-truncate pi-heartbeat redoxfs-diag`, md5 `c4e81a49…`), mirror
`pkg-mirror-t197`. Real Pi 3B+ over netboot, no HDMI connected. 16:34–17:31 UTC.

**Result: 1 hang in 10 rounds (round 7, in the install). RedoxFS had answered every request
it got, and after the hang no new request reached it.**
- At and after the hang, `redoxfs-diag /scheme/file` shows `in378541 out378541`,
  unchanged for more than 7 min.
- `busy0`, and every `skip-cancel`/`recvfd`/`msync`/`munmap`/`mmap` count is 0.
- `pkg` stayed blocked in its call. So the request `pkg` waits for was lost before it
  reached RedoxFS (on the kernel or scheme-queue side), not dropped or left unanswered by
  RedoxFS.
- No exception or panic.

- **Free space after each remove (KB):** 124636, 124852, 124800, 124740, 124680, 124616,
  – (hang), 124628, 124844, 124792. Flat; the fix works.
- **Hang, round 7** (install from 17:04:46):
  - Last progress line `Extracting vim [ 54%] [00:02:09] … 1251/2316`.
  - `redoxfs-diag /scheme/file` before and at the hang (one line every 30 s):
    `t1861 in369123 out369123`, `t1891 in373540 out373540`, `t1921 in377814 out377814`,
    **`t1951 in378541 out378541`**, then the same values at t1981 … t2401 (15 lines).
    `close78634 sendfd0 skip-cancel0 recvfd0 msync0 munmap0 mmap0 busy0 max-busy74`.
  - `/scheme/data` unchanged at `in9 out9`.
  - During installs, `in` and `out` sometimes differ by 1 (a request in progress, for
    example `t690 in134387 out134386`). At the hang they are equal.
  - `cat /scheme/sys/context` after the 2-minute wait:
    - initfs `redoxfs` pid 20: one context `UR` on #1 with 4:21 user and 1:38 system CPU,
      102 MB, and one `US`.
    - `pkg` pid 152: two contexts `UB`, one with 2:20 CPU.
    - `/usr/bin/redoxfs` pid 33 `UB`/`US`.
  - The `UR` RedoxFS context with growing CPU, while the request counters stand still,
    suggests a RedoxFS thread spinning outside request handling, or the kernel showing it
    runnable.
  - Heartbeats `[hb N 37–39 …]` kept coming on all four CPUs (`p0`). No `[hbctx …]` dump
    came in this window, because the timed dump didn't fall into it and no
    runnable-on-idle was detected.
- After the hang: power cycle, mirror set up again, rounds 8–10 ok.
- Logs on homenodellm: `~/pitest-tools/t252/serial-all.log` (whole run),
  `hang-C-7-install-{serial.log,context.txt}`, and the per-round files.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a devsh reset: devsh answers, and
  `/srv/tftp/redox` matches the backup.

## #251 open firmware step 2: librerpi/lk-overlay on the Pi 3B+ (2026-11-05, Pi test agent)

Card image `redox-rpi-lkoverlay-3bp-20261105.img.xz` (sha256 checked) on the user's 32 GB
SDHC card. Real Pi 3B+, no HDMI connected. The plug was off at 19:32:18 UTC, the user put
in the card, plug on at 19:34:11, 90 s of serial, off at 19:35:42.

**Result: stage 1 starts, prints one line and stops.** Raw 52 bytes
(`~/pitest-tools/t251/serial-lk.raw` on homenodellm): a `0x00` byte, then
`U  0.001128 [VPU:PLATFORM:platform_early_init]: b` + CRLF, then nothing for the rest of
the 90 s. No garbage, so no baud change. The Open firmware session traced it to a USB
register write and fixed it in #253. The ARM was not reached. Stable was checked after #253
(same card swap session).

## #253 open firmware step 2 rerun: lk-overlay with the USB write skipped (2026-11-05, Pi test agent)

Card image `redox-rpi-lkoverlay-3bp-20261105b.img.xz` (sha256 checked) on the same SDHC
card, written by the user. Real Pi 3B+, no HDMI connected. Plug on at 19:42:40 UTC, 90 s of
serial (9097 bytes, 248 lines, `~/pitest-tools/t253/serial-lk.raw` on homenodellm), off at
19:44:10.

**Result: the VPU starts the ARM in AArch64, and lk/MP runs on the ARM. Then the ARM-side
loader fails with `mount failure: -2` on `sdhostp1`.** No panic.

- **Stage 1 (VPU):**
  - Markers `b`, `b1 (USB_DCTL skipped)`, `b2 intc_init`, `c`. `PM_RSTS 0x1000` (power-on
    reset). PLLC, PLLA and PLLH up.
  - `SDRAM Type: Elpida 1GB LPDDR2`, `398.4 MHz DDR clock`, self test successful.
  - EMMC: `This is an SDHC card!`, 4-bit mode, `30475MB SDHC Card` (the size is now right),
    25 MHz.
  - Image and USB PHY up, `stage1 entry`, `trying to boot from sdhostp1`,
    `flushing uart tx and chainloading...`.
- **Stage 2 (VPU lk):**
  - `welcome to lk`, HVS and VEC init (NTSC), usbphy.
  - `[ARM:choose_arm_payload]: detected a bcm2837, picking payload … size 0x127a8`,
    **`enabling aarch64`**, payload and fdt (317 bytes at 0x1000000) placed.
  - `ARM clock succesfully initialized!`, `bridge init done, PM_PROC is now: 0x107F!`,
    `starting app properties`.
  - Two values look like print overflows: `PLLC_PER at 9088281640219705594MHz` and
    `pllc: 2251800063685248`.
- **ARM:**
  - `[ARM:PLATFORM:platform_early_init]: c` at 3.65 s, **`welcome to lk/MP`**,
    `hdr: 0xffff0000000122c0` (64-bit kernel addresses), `DTB should be at 0x1000000`.
  - DTB nodes framebuffer (620 x 420 @ 0x6000000), timestamps and otp.
  - `platform_init`, `UART break detected`, `starting app loader`,
    `trying to boot from sdhostp1`, **`mount failure: -2`**.
  - Last line `temp changed 412.000000 -> 34.862000`.
- **Network-boot card back** (plug on at 19:47:53): stable `v0.4.0-dev-201-gb714bca+x9f76`
  booted to `redox login:` after 28 s, and devsh answers. **pi-test pass** (version,
  manifest, ps, ps-tty, sdcard, 0 of 300 starts failed).

## #220a #188 vc4d stage 1, automated check with HDMI attached (2026-11-05, Pi test agent)

Image `v0.4.0-dev-78-g8bed0132+x9736` (branch `graphics-vc4d`, experiments `vc4d orbital`).
Real Pi 3B+ over netboot, **HDMI attached at power-on (NanoKVM)**, power cycle at
20:20:25 UTC.

The first try at 20:12 booted stable by mistake: the Graphics clone has no U-Boot build, so
`netboot.sh` stopped (`mkimage missing`) and the old stable files stayed in place. The
rerun used a staging folder with the Graphics scripts, config and bootloader, plus
`mkimage` from `~/redox-rpi-cpu/build`.

**Result: the check says FAILED, 9 passed and 2 failed.** Both failures are the list
read-back: the list the HVS holds does not read back as written. Everything else passes:
list start, frame count, active word, cursor at (16,16) and (960,540), and no HVS errors.
The 2–120 s watch is all `PASS frames advance, errors []`. Serial login works, pi-test
passes with the manifest (0 of 300 starts failed), and there was no panic.

- **(1)** `vc4d: HVS at 0x3f400000, id 0x64647276, enabled`,
  `dispctrl 0x9a0c0fff dispstat 0x00000000`. Channel lines before vc4d touched anything:
  - `channel 0: ctrl 0x00000000 bkgnd 0x00000000 list 0 active 0 stat 0x119cb000 (mode 0, frame 11)`
  - `channel 1: ctrl 0x80780438 bkgnd 0x01000000 list 1636 active 1636 stat 0xa00311b8 (mode 2, frame 49)`
  - `channel 1: 1920x1080, list [4e005807, ff016000, 040b0780, 43d80720, 03d70000, fe914000, fefeaf80, 00001c80, 00008300, 40f33261, 40f36260, 00017d36, 00000ff4, 00000ff4, 80000000]`
  - `channel 2: ctrl 0x00000000 bkgnd 0x00000000 list 0 active 0 stat 0x10000000 (mode 0, frame 0)`
  - `taking over channel 1 at 1920x1080`. The stable boot an hour earlier with the same
    NanoKVM had a 1824x984 framebuffer.
- **(2)** Check:
  - `PASS channel 1 list start is 2176`.
  - **`FAIL list at 2176 reads back as written: [47006017, ff000000, 44380780, 018e0000, c5000000, c52ea400, 00001e00, 47006017, ff010010, 00400040, 003f0000, c57ec000, c57eff00, 00000100, 80000000]`**.
  - `PASS frame count advances (2 frames)`, `PASS active word 2176 is in the list`,
    `PASS cursor at (16, 16)`.
  - Then for list 2240: `PASS list start`, **`FAIL list at 2240 reads back as written: [47006017, ff000000, 44380780, 00580000, c5000000, c50a5000, 00001e00, 47006017, ff21c3c0, 00400040, 00000000, c57ec000, c57ec000, 00000100, 80000000]`**.
  - `PASS frame count`, `PASS active word`, `PASS cursor at (960, 540)`,
    `PASS no HVS errors on channel 1: []`.
  - `back on the firmware list 1636; 9 passed, 2 failed`, **`vc4d: check: FAILED`**.
- **(3)** `vc4d: no primary plane, firmware list again` (twice). Then:
  - `list 1 at 2048`: one plane, 1920x1080, pitch 7680.
  - `list 2 at 2112`: one plane.
  - `list 3 at 2048`: plane 1920x1080 plus a cursor plane 64x64 with alpha.
- **(4)** After 2, 5, 10, 30, 60 and 120 s: `PASS frames advance, errors []`, list 2048,
  active 2048.
- **(5)** Login on serial works (`echo serial-ok`), and there is no panic.
- The full `vc4d:` part (homenodellm `~/pitest-tools/t220a/vc4d.txt`) went to the Graphics
  session.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a power cycle (HDMI attached):
  devsh answers, and `/srv/tftp/redox` matches the backup.

## 2026-11-06: `v0.4.0-dev-565-gc70f394e+x9f76`

Built 2026-11-06T03:30:26Z, `libc.so` `e1f1d71b8c10c33f…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Nightly stable image test (scripts/update-stable.sh).

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.4.0-dev-565-gc70f394e+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.4.0-dev-565-gc70f394e+x9f76.toml |
| ps | pass | 46 processes (piped) |
| ps-tty | pass | 46 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## #220 #188 vc4d stage 1: screen check through the NanoKVM (2026-11-06, Pi test agent and the user)

Image `netboot-pb-vc4d-v0.4.0-dev-598-g6763d5b7+xf33c` (the newest Graphics build, merged
vc4d; experiments `cputop orbital pointer-bench vc4d`), staged as for #220a. Real Pi 3B+
over netboot, **HDMI to the NanoKVM at power-on**, power cycle at 07:28:06 UTC, login 30 s
later. The user looked at the screen through the NanoKVM.

**Result: pass for the picture.** vc4d shows Orbital's desktop and taskbar at 1920x1080, the
pointer is visible, and the keyboard works. The mouse did not work only because this image
has no `--experiment usb-mouse`, so the pointer and drag points (5) and (6) were not
checked.
- Seen on screen: **black bars at the sides**, probably the KVM capture or the HDMI timing,
  for Graphics to check.
- Seen on screen: **a dark bar at the top**, probably Orbital's own content (Graphics is
  writing `vc4shot` to check).

- **Serial:**
  - `vc4d: HVS at 0x3f400000, id 0x64647276, enabled`, channel lines as in #220a (channel
    1 `ctrl 0x80780438`, firmware list 1636, 1920x1080).
  - `taking over channel 1 at 1920x1080`.
  - **`vc4d: check: back on the firmware list 1636; 11 passed, 0 failed`, `vc4d: check:
    PASSED`.** The list read-back now passes with "4 context words filled in by the HVS"
    (#220a: 9 of 11).
  - `no primary plane, firmware list again` (twice), `list 1 at 2048` (fbcond), `list 2 at
    2112`, `list 3 at 2048` (Orbital's 1920x1080 plane plus the 64x64 cursor plane with
    alpha).
  - Watch at 2, 5, 10, 30 and 60 s: `PASS frames advance, errors []`. No panic.
- The user's words when done: "ok kann man eh nicht abmelden, kannst wieder übernehmen"
  (logging out isn't possible anyway).
- The vc4d log (homenodellm `~/pitest-tools/t220/vc4d.txt`) went to the Graphics session.
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a power cycle at 07:37:58 (HDMI
  attached): devsh answers, and `/srv/tftp/redox` matches the backup.

## #255 PR #223 USB mouse as a default patch (2026-11-06, Pi test agent)

Image `mousepi-v0.4.0-dev-570-ge437e901+x12cb` (branch `usb-mouse-default` e437e901, only
`--experiment timesync`, md5 `3190d2c9…`). Real Pi 3B+ over netboot with the usual USB
mouse (093a:2510) and keyboard, HDMI to the NanoKVM. Reset at 07:45:41 UTC,
`devshd: listening` after 20 s.

**Result: pass.** The default image drives the mouse, it comes back after a port power
cycle, and devsh keeps answering. No panic.

- **(1)** Boot: `dwc2d: keyboard at address 4` and **`dwc2d: mouse at address 5: relative,
  3 button(s), wheel`**.
- **(2) pi-test:** pass (manifest matches, 0 of 300 starts failed).
- **(3)** `echo mouse > /tmp/dwc2d-cycle` over devsh at 07:46:54, then in the serial log:
  - `cycle: hub 1 port 3 power off`, `disconnected after 101 ms`,
    `hub 1 port 3: device at address 5 removed`, `cycle: hub 1 port 3 power on`,
    `mouse removed`.
  - `device connected`, `address 5: 093a:2510, USB 1.10, Low speed`, interface class 3/1/2,
    endpoint 0x81 interrupt max packet 4.
  - `request [21, 0a, …] to address 5 … failed: Stall`. This is SET_IDLE, which many mice
    refuse; it did no harm.
  - Then **`dwc2d: mouse at address 5: relative, 3 button(s), wheel`** again.
  - devsh answered 15 s later.
- **(4)** `/tmp/dwc2d-input-log` touched for 5 s and removed. Nobody moved the mouse, so
  there were no move lines (optional).
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a power cycle (HDMI attached): devsh
  answers, and `/srv/tftp/redox` matches the backup.

## #256 #188 what pointer moves cost: vc4d against rpi-fbd and vesad (2026-11-06, Pi test agent)

Three network boots on the real Pi 3B+, **HDMI to the NanoKVM at power-on** (power cycle
each). Staged as for #220a (staging folder with the Graphics scripts and bootloader, plus
`mkimage` from `~/redox-rpi-cpu/build`). Each boot: Orbital's login window up (75 s after
devsh), `cputop -n 1 -l 12 20` at idle, then `pointer-bench 70 > /tmp/pb.txt &` and right
after it `cputop -n 1 -l 12 20`, then after 60 s `cat /tmp/pb.txt`.
- **vc4d:** `v0.4.0-dev-600-g1d7116cf+x8ba8`.
- **rpi-fb:** `v0.4.0-dev-599-g0db53051+x7cb3`.
- **vesad:** `v0.4.0-dev-599-g0db53051+x6042`.

**Result: moving the pointer keeps one core about 92–100 % busy with vc4d and with vesad.
Most of it is `orblogin` (54 %) and `orbital` (29–46 %); the display daemon itself costs
little** (vc4d 0.5 %, vesad 16 %). On the rpi-fb image the measurement failed twice:
`cputop` returned at once (0.0 s) as soon as `pointer-bench` ran, and `pointer-bench` wrote
nothing. vc4d's check PASSED. The vc4shot screenshot is black, because vc4shot may not map
the planes.

| Boot | phase | busiest core | orblogin | orbital | display daemon | inputd | dwc2d | pointer-bench result |
|---|---|---|---|---|---|---|---|---|
| vc4d | idle | cpu2 5 % | – | – | – | – | 4.9 % (101/s) | |
| vc4d | moving | **cpu1 92 %**, cpu0 13 % | **53.5 %** (117/s) | **45.8 %** (274/s) | vc4d 0.5 % (85/s) | 0.6 % (173/s) | 5.2 % | `4200 moves in 70.0 s (60.0/s)` |
| rpi-fb | idle | cpu2 5 % | – | – | rpi-fbd 0.0 % | – | 5.0 % | |
| rpi-fb | moving | **measurement failed** (`cputop: 0.0 s`) | (543 % in a 25 ms sample) | | | | | **empty `/tmp/pb.txt`** |
| vesad | idle | cpu0 5 % | – | – | – | – | 4.9 % | |
| vesad | moving | **cpu2 100 %** | **54.5 %** (151/s) | **28.8 %** (289/s) | vesad **16.2 %** (53/s) | 0.2 % (146/s) | 5.2 % | `4200 moves in 70.0 s (60.0/s)` |

- **vc4d boot:** `vc4d: check: PASSED`. `vc4shot /tmp/shot.ppm`:
  - `channel 1 1920x1080, list at 2048, 2 planes`.
  - plane 0 1920x1080 format 7, and plane 1 64x64 with alpha at **x 1259, y 524**
    (the pointer).
  - Both `cannot map 0x6800000` / `0x7624000: Error Operation not permitted`. It still
    `wrote /tmp/shot.ppm (1920x1080)`, 6220817 bytes, copied to homenodellm by base64 over
    devsh (`~/pitest-tools/t256/shot.ppm`). **All pixel bytes are 0**, so it can't show the
    dark bar yet.
- **rpi-fb boot** (twice, 08:20 and 08:29):
  - rpi-fbd found `EDID 256 bytes, monitor NanoKVM`, `boot framebuffer 1824x984`, and
    `blits with the CPU`.
  - The idle cputop measures normally (20.0 s), and `sleep 5` takes 5 s (second boot).
  - But the cputop started right after `pointer-bench … &` returns in 0.4 s with
    `cputop: 0.0 s, 60 threads`. In the first boot it showed orblogin at 543 % in a 25 ms
    window; in the second, all zeros.
  - `/tmp/pb.txt` stays empty 60 s later.
  - So with rpi-fbd, something goes wrong as soon as pointer-bench drives the pointer: a
    timer or clock problem for other programs, or pointer-bench dies. Graphics should
    check. The clock was at 1970 (no timesync in these images).
- **Interrupts:** 115–121/s (timer 100, line 50 15–21) in every phase except the broken
  rpi-fb sample.
- Full outputs are on homenodellm in `~/pitest-tools/t256/` and `t256b/` (rpi-fb repeat).
- Stable `v0.4.0-dev-201-gb714bca+x9f76` restored with a power cycle (HDMI attached):
  devsh answers, and `/srv/tftp/redox` matches the backup.

## #257 #197 RedoxFS hang with `call-diag`: which scheme does pkg's blocked call wait on? (2026-11-06, Pi test agent)

Image `t197-hbE.img` (`v0.4.0-dev-376-g842d0fa4+x6c8e`, experiments
`timesync redoxfs-truncate pi-heartbeat redoxfs-diag call-diag`, md5 `cd052b0a…`), mirror
`pkg-mirror-t197`. Real Pi 3B+ over netboot, HDMI attached. 08:41–09:37 UTC.

**Result: 3 hangs in 6 rounds, so the run stopped after round 6. `pkg`'s blocked call does
not wait on RedoxFS.**
- Hang E-1: `call 39/21/6/3`, handler pid 21 = **`ptyd`**, opcode 6.
- Hang E-6: `call 43/25/5/3`, handler pid 25 = **`netstack`**, opcode 5.
- In both, the handler sits idle: `ptyd` in `EventQueue::read`, and `netstack` in its own
  call `36/16/5/0` to pid 16 = `dwc2d`, which sleeps in `nanosleep`.
- RedoxFS again answered every request (`in = out`, `busy0`, every skip count 0).
- So the waiting request is lost on the way to a scheme that is not RedoxFS, or its wakeup
  is lost.
- The "spinning" initfs `redoxfs` (pid 20) is `UR` at **pc `0x47ad24` with no block
  reason and `call 0/0/0/0`**. That is the same PC at which the data `redoxfs` (pid 32)
  waits in `UserInner::read`, so it is the return point of the read syscall. Its CPU grows
  only slowly (E-1: 9.62 → 9.95 s user over 40 s). It looks like a context that is marked
  runnable but sits at the syscall return.
- No exception or panic.

- **Free space after each remove (KB):** E-2 124476, E-3 124692, E-4 124640. Flat. E-1,
  E-5 and E-6 hung.
- **E-1** (install from 08:42): stuck at `Extracting vim … 482/2316`. The 3 context dumps
  (20 s apart) are the same apart from slowly growing CPU:
  - `20 UR #1 /scheme/initfs/bin/redoxfs call 0/0/0/0 pc 0x47ad24` (9.62 / 9.79 / 9.95 s
    user), and its second context `US … nanosleep`.
  - `32 UB /usr/bin/redoxfs call 0/0/0/0 pc 0x47ad24 UserInner::read (v2)`.
  - **`56 UB #0 /usr/bin/pkg call 39/21/6/3 pc 0x4f6870 UserInner::call`** (1:12 user),
    and `56 UB #2 pkg … futex`.
  - `54`/`57 UB /usr/bin/ion call 18/0/28/7` and `/8 … UserInner::call` (devsh shells
    waiting on scheme 18, handled in the kernel's pid 0).
  - **`21 UB /usr/bin/ptyd call 0/0/0/0 pc 0x4269c0 EventQueue::read`**.
  - `25 UB /usr/bin/netstack call 36/16/5/0 pc 0x464a18 UserInner::call`,
    `16 US /scheme/initfs/bin/dwc2d … nanosleep`.
  - `redoxfs-diag /scheme/file t540 … t570 in13037 out13037 … busy0` (unchanged).
- **E-5** (install from 09:10): the last progress line was `Extracting terminfo [100%]
  108/108`, then nothing. In all 3 dumps **`pkg` no longer appears**. Only the devsh
  `ion` (102) waits in `call 18/0/28/10`, then `/11`.
  - Initfs `redoxfs` pid 20 `UR` at pc `0x47ad24` (2:08.90 → 2:09.24 user, 102 MB).
  - `redoxfs-diag /scheme/file t1500 … t1530 in178023 out178023 busy0`.
  - So this time `pkg` seems to have ended (or died), and the shell around it never
    returned.
- **E-6** (install from 09:22): no `Extracting` line, so it hung during the download from
  the mirror.
  - **`56 UB #3 /usr/bin/pkg call 43/25/5/3 pc 0x4f395c UserInner::call`**, waiting on
    `netstack`.
  - `netstack` itself in `call 36/16/5/0` to `dwc2d`, and dwc2d in `nanosleep`.
  - Initfs `redoxfs` `UR` at pc `0x47ad24` (3.47 → 3.63 s).
  - `redoxfs-diag /scheme/file t660 … t750 in4019 out4019 busy0`.
  - Heartbeat dump: `[hbctx 20 … redoxfs B sys cpu3 UserInner::read (v2)]`, `[hbctx 56
    /usr/bin/pkg B sys cpu3 UserInner::call]`. No runnable-on-idle dump.
- The 3 context dumps of each hang (whole) and the serial log are on homenodellm in
  `~/pitest-tools/t257/hang-E-{1,5,6}-install-{context1,2,3.txt,serial.log}`.
- After each hang, a power cycle and the mirror set up again. Stable
  `v0.4.0-dev-201-gb714bca+x9f76` restored with a power cycle: devsh answers, and
  `/srv/tftp/redox` matches the backup.

## #258 #197 pkg hang with the kernel futex fix `futex-wake-race` (2026-11-06, Pi test agent)

Image `t197-hbF.img` (`v0.4.0-dev-377-g8ab74b8b+x1409`, experiments
`timesync redoxfs-truncate pi-heartbeat redoxfs-diag call-diag futex-wake-race`, md5
`961e24b6…`, checked), mirror `pkg-mirror-t197`. Real Pi 3B+ over netboot, HDMI attached.
15:54–16:48 UTC.

**Result: pass. 10 of 10 rounds with no hang** (257 on the same base without the futex fix:
3 hangs in 6 rounds; 252: 1 in 10; 250 B: 2 in 6; 248: 4 in 9 installs).
- Every `pkg install --yes vim` and `pkg remove --yes vim` ended with rc=0. Each round
  (install plus remove) took about 5 min 15 s, very even (16:00:48, 16:06:02, … 16:47:59).
- **Free space after each remove (KB):** 124468, 124688, 124632, 124572, 124512, 124720,
  124668, 124608, 124548, 124484. Flat (124.5–124.7 MB), the space fix still works.
- Free after each install: 89016–89292 KB.
- No exception or panic on serial. No hang, so no context dumps were needed.
- Stable image staged again, booted to `v0.4.0-dev-201-gb714bca`, devsh answers,
  `/srv/tftp/redox` matches the backup.

- `date` showed 2026 (15:55:35 UTC). `redoxfs-diag` lines stayed `in = out`, `busy0`.

So the hang is gone with the backport of the upstream `FUTEX_WAIT` lost-wakeup fix
(57ccc8f1, 58050594). One run of 10 clean rounds is not proof, but before the fix the same
test hung in about 1 of 3 installs.

## #261 #224 `socdump`: the real 3B+'s clock, PLL and GPIO registers against the firmware (2026-11-06, Pi test agent)

Image `netboot-socdump-v0.4.0-dev-587-g9bf64d7.img` (`v0.4.0-dev-587-g9bf64d7`, branch
`socdump` 9bf64d7). Real Pi 3B+ over netboot, HDMI attached. 17:21–17:23 UTC.
The row's `2>&1` is not valid in ion, so step 1 ran as `socdump &> /tmp/sd.txt`.

**Result: pass. All 4 calls answered in under 0.4 s, no hang or panic, rc=0 for step 1.**
- **PLLs** (crystal 19.2 MHz, `CM_LOCK 0x00001f1f`, all locked):

  | PLL | MHz | Taps |
  |---|---|---|
  | plla | 2400 | core /4 = 600 |
  | pllb | 2800 | **`pllb_arm` /2 = 1400.000** |
  | pllc | 2400 | core0 /2 = 1200, per /2 = 1200 |
  | plld | 2000 | core /4 = 500, per /4 = 500 |
  | pllh | 1485 | pix /1/10 = 148.5 |

- **Clocks that are on:**

  | Clock | Source | MHz |
  |---|---|---|
  | v3d | plla_core /2 | 300 |
  | emmc | pllc_per /6 | 200 |
  | uart | plld_per /10.4167 | 48.000 |
  | gp1 | | 24 |
  | hsm | | 163.683 |
  | timer | | 1.000 |
  | tsens | | 1.920 |
  | otp | | 4.800 |

- **Clocks that read as off:** `vpu`, `h264`, `isp` and `sdram` read as not enabled, at
  0 MHz (ctl `0x245`/`0x244`/`0x244`/`0x4006`).
- **GPIO:** 14/15 `alt0` (UART), 2/3 and 7–11 `alt0`, 40–43 and 48–53 `alt0`.
- **`socdump check`** (rc=1): emmc, uart and arm `ok`; these are **`DIFFERENT`**:

  | Clock | Firmware MHz | socdump MHz |
  |---|---|---|
  | core/vpu | 250 | 0 |
  | v3d | 250 | 300 |
  | h264 | 250 | 0 |
  | isp | 250 | 0 |
  | sdram | 400 | 0 |

- **`/scheme/rpi-firmware/clocks`** (rate, max):

  | Clock | Rate (Hz) | Max (Hz) |
  |---|---|---|
  | emmc | 200000000 | 200000000 |
  | uart | 48000000 | 1000000000 |
  | arm | 1400000000 | 1400000000 |
  | core | 250000000 | 400000000 |
  | v3d | 250000000 | 300000000 |
  | h264 | 250000000 | 300000000 |
  | isp | 250000000 | 300000000 |
  | sdram | 400000000 | 450000000 |

- Stable image staged again, booted, devsh answers. The raw output went to the open-firmware
  session (homenodellm `~/pitest-tools/t261/`).

## #259 #188 pointer moves and a window drag on the logged-in desktop: vc4d, rpi-fbd, vesad (2026-11-06, Pi test agent)

Images on branch `graphics-vc4d` 9f2c802d:

| Boot | Display driver | Build |
|---|---|---|
| 1 | vc4d | `v0.4.0-dev-602-g9f2c802d+x8ba8` |
| 2 | rpi-fb | `+x7cb3` |
| 3 | vesad | `+x6042` |

Each image has `usb-mouse pointer-bench cputop orbital`. Real Pi 3B+ over netboot, HDMI to
the NanoKVM at power-on. Two passes:
- **Part A**, 16:49–17:07 UTC: steps 1–3, the rpi-fb test and vc4shot.
- **Part B**, 17:23–17:40 UTC: steps 1, 2, 4 and 5 again. In part A the background
  `… > /tmp/pb.txt 2>&1 &` was an ion syntax error, so pointer-bench never started. Part B
  uses `&>`.

**Result: done, with gaps.**
- **Login:** works on all three boots in both passes. `--type '\n'` prints `typed 1 keys`,
  `--super t` prints `Super+t`, and then orblogin is gone and cosmic-term runs.
- **vc4d check:** `PASSED` 11/11, in both passes.
- **Drag:** orbital takes about 90 % of one core on rpi-fb (90.9 %) and vesad (89.4 %).
- **Moving:** cheap on vc4d and vesad, about 1.5–1.7 % for orbital.
- **Two of the six load measurements failed:** cputop returned at once (`0.0 s`), as in
  #256: vc4d drag and rpi-fb moving.
- **pointer-bench totals:** `/tmp/pb.txt` and `/tmp/pd.txt` were still empty when read. The
  row reads them 60 s after the start of a 70 s run, and pointer-bench prints only at the
  end.
- **No exception or panic.**

**Idle, logged in** (part A, `cputop -n 1 -l 12 10`): about 5 % of one core on all three
boots, almost all of it dwc2d (5.0–5.1 %, 110–117 wakeups/s).

| | vc4d | rpi-fb | vesad |
|---|---|---|---|
| orbital | 0.3 % | 0.2 % | 0.2 % |
| display daemon | 0.0 % | 0.1 % | 0.1 % |

cosmic-term is not in the top 12.

**Moving** (part B, pointer-bench 70, `cputop -l 12 20`):

| | vc4d | rpi-fb | vesad |
|---|---|---|---|
| busiest core | 8 % | cputop 0.0 s | 5 % |
| orbital | 1.7 % (364/s) | (4.7 %, 1389/s in a 0.0 s sample) | 1.5 % (360/s) |
| cosmic-term | 0.8 % (240/s) | – | 0.8 % (240/s) |
| display daemon | vc4d 0.3 % (63/s) | – | vesad 0.4 % (61/s) |
| inputd | 0.4 % (180/s) | – | 0.3 % (180/s) |
| orblogin | gone | gone | gone |

The failed rpi-fb sample showed netstack 12037/s, devshd 12963/s and dwc2d 9954/s, in a few
ms.

**Window drag** (part B, `--drag 960 540 --super-drag`, `cputop -l 12 20`):

| | vc4d | rpi-fb | vesad |
|---|---|---|---|
| busiest core | cputop 0.0 s | cpu2 45 %, cpu3 20 % | cpu1 95 % |
| orbital | – | **90.9 %** (266/s) | **89.4 %** (116/s) |
| display daemon | – | rpi-fbd 7.6 % | vesad 10.2 % |
| inputd | – | 0.8 % | 0.6 % |
| cosmic-term | – | 0.4 % | – |

**rpi-fb test** (part A): `pointer-bench 5; echo rc=$?` printed `300 moves in 5.0 s (60.0/s)`
and `rc=0`. Serial: `rpi-fbd: EDID 256 bytes, monitor NanoKVM`, `boot framebuffer 1824x984`,
`blits with the CPU`.

**vc4shot** works now:
- **Part A:** 2 planes, the 1920x1080 primary and the 64x64 cursor with alpha.
  `vc4shot: wrote /tmp/shot.ppm (6220817 bytes)`, serial `vc4d: shot: wrote /tmp/vc4shot.ppm
  (1920x1080, 2 planes)`. The picture shows the desktop, the terminal window, the bottom
  taskbar and the clock 00:05, with **no side bars and no top bar**. So the #220 bars come
  from the HDMI/KVM side.
- **Part B, during a drag:** the terminal has moved left, partly off screen. The cursor plane
  is reported at (523, 272).
- Both pictures are on homenodellm in `~/pitest-tools/t259/shot.png` and
  `~/pitest-tools/t259b/shot.png`.

Stable image (now `v0.4.0-dev-565-gc70f394e+x9f76`) staged again, booted, devsh answers.

## #260 PR #226 opt-a53: kernel, relibc and base built for speed and the Cortex-A53, A/B (2026-11-06, Pi test agent)

Images:

| | Build | What it is |
|---|---|---|
| A | `v0.4.0-dev-565-gc70f394e+x9f76` | stable |
| B | `v0.4.0-dev-567-gd91a247f+x9b71` | `opt-a53` |

Both are on homenodellm in `~/redox-rpi-opt/keep/{A,B}/harddrive.img`. Real Pi 3B+ over
netboot, HDMI attached. Benchmark: `patches/experiments/opt-a53/bench.py --rounds 3`, from
branch `exp-opt-a53`.

**Result: pass. B is faster in CPU- and network-bound work and the same on the SD card.**
pi-test passed on all 4 boots of the final pass (0 of 300 starts failed).

**Final pass** (bench.py 9cb724b1, order B, A, B, A, 20:47–21:18 UTC, 6 rounds per image,
median and range in seconds):

| Workload | A (stable) | B (opt-a53) | B against A |
|---|---|---|---|
| spawn, 500 × `ls /usr` | 77.60 (77.55–77.78) | 75.37 (75.18–75.74) | **2.9 % faster** |
| net-rx, 32 MiB to the Pi | 3.02 (2.74–3.14) | 2.59 (2.46–2.79) | **14 % faster** |
| net-tx, 32 MiB from the Pi | 4.89 (4.82–4.93) | 4.38 (4.33–4.44) | **10.5 % faster** |
| sd-write, 50 MiB | 6.70 (5.85–7.89) | 6.73 (5.77–8.35) | same (card-bound, noisy) |
| sd-read, 50 MiB | 0.56 (0.56–0.57) | 0.55 (0.54–0.56) | about the same |

- **net-tx:** no retries were needed in this pass.
- **Clocks:** the same before and after on every boot (arm 1400 MHz, core 250, sdram 400).
  So there was no throttling.

**Boot to `redox login:`**, from 12 boots over 4 passes (boot-timeline.py):

| Image | Times (s) |
|---|---|
| A | 30.51, 30.36, 35.17, 30.36, 35.39, 39.88, 35.24, 30.31 |
| B | 29.97, 30.12, 30.16, 39.83, 30.01, 30.06, 34.99, 30.60 |

Both images take about 30 s, with the same 35–40 s outliers. No difference.

**Earlier passes**, each a run of 4 boots in the order A, B, A, B:
- **Pass 1** (17:08): bench.py stopped at net-rx. Redox `nc -l 5135` is refused; it needs
  `nc -l 0.0.0.0:5135` (fixed in faf707e3).
- **Pass 2** (17:40): bench.py stopped at net-tx. Redox `nc` does not close the connection
  at the end of its input (fixed in f4f63618).
- **Pass 3** (18:57): net-tx did not connect in 3 of 4 tries (retries added in 9cb724b1).
- **Boot 4 (B) of pass 3:** pi-test hung in the 300 starts after the sdcard PASS (timed out
  at 600 s), and then devsh gave "No route to host". There was no serial line and no panic.
  It did not happen again in pass 4 (B twice, 600 starts). It may be the old intermittent
  network loss or the #197 futex hang; neither image has the futex fix.
- **Spawn and net-rx numbers from passes 1–3** agree with pass 4: A 77.75–77.95 s, B
  75.42–75.64 s.

Stable image staged again, booted, devsh answers. Raw data on homenodellm:
`~/pitest-tools/t260{,b,c,d}/opt-a53.jsonl`.

## 2026-11-07: `v0.4.0-dev-604-g8c4c513f+x9f76`

Built 2026-11-07T03:30:26Z, `libc.so` `e1f1d71b8c10c33f…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Nightly stable image test (scripts/update-stable.sh).

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.4.0-dev-604-g8c4c513f+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.4.0-dev-604-g8c4c513f+x9f76.toml |
| ps | pass | 46 processes (piped) |
| ps-tty | pass | 46 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## #263 #92 repeat of #237 part B (host faults) with lock watches (2026-11-07, Pi test agent)

Image `t92cw-v0.4.0-dev-361-g4273f649+x9040.img` (branch `t92-chaos-watch` 4273f649,
experiments `chaos netstack-watch`, md5 `51bf810c…`). `chaos-host.py` from the same
branch. Real Pi 3B+ over netboot. 06:31–07:20 UTC.

**Result: pass. 40 of 40 host-fault rounds, the network never got lost.**
- Boot: `devshd: listening` 20 s after the reset. `pi-test.py` pass (ps, ps-tty, sdcard,
  starts 0 of 300 failed).
- **Seed 237** (the #237 run): the same fault order as in #237, round 20 again `netem-delay`
  with the same round seed 3107139973. All 20 rounds ok, devsh back after 1–6 s. In #237
  that round 20 lost the network for good.
- **Seed 238:** 20 of 20 ok, devsh back after 1–6 s. Faults: netem-blackout ×4, server ×5,
  pkg ×4, netem-delay ×2, netem-loss ×2, netem-reorder ×2, dwc2d ×1.
- `netstack-watch:` printed `alive …, idle for 0 s` every 60 s to the end (2942 s,
  285192 events, 177569 frames in). No `dwc2d-watch:` line (it only prints when a call
  is stuck). No panic, no crash dump lines.
- tc on enp12s0 back to `fq_codel` afterwards (checked).
- The stable image (`v0.4.0-dev-604-g8c4c513f`) was booted again as the first step of #264.

So the #92 hang did not show up in 40 rounds with the watches in the image. It is rare
(1 in 20 rounds in #237), so this does not prove it is gone; the watches stayed quiet.
Outputs in `~/pitest-tools/t263/` on homenodellm.

## #264 #217 boot race: 10 warm resets out of stable, with watches (2026-11-07, Pi test agent)

Image `bnr-v0.4.0-dev-443-g6e17ce46+x814f.img` (branch `boot-net-race` 6e17ce46, experiments
`dwc2d-netstats usb-hub-drop dwc2d-restart dwc2d-watch netstack-watch`, md5 `da1ef6dd…`).
Stable `v0.4.0-dev-604-g8c4c513f`. Real Pi 3B+ over netboot. 07:21–07:34 UTC.

**Result: pass, 10 of 10 boots with devsh and ping. The bad boot did not happen.**
- Each cycle: stable booted (devsh answers), this image staged, `echo reset > /scheme/sys/kstop`
  over devsh, then wait for devsh in this image, then stable staged and reset again.
- In this image devsh answered 30–37 s after the reset, every time. `ping -c 3 192.168.178.1`
  on the Pi: 3 of 3 received in all 10 boots.
- From homenodellm, `ip neigh show` had the Pi `REACHABLE` (b8:27:eb:08:e0:aa) after every
  boot, so it answered ARP. The plain `ping` from homenodellm did not run: there is no `ping`
  command on this host (only the runner's mistake; devsh over TCP worked in all 10).
- Serial of every boot: the `redox-rpi v0.4.0-dev-443-g6e17ce46+x814f` banner, `devshd: starting`
  then `devshd: listening`; no `devshd: no answer from the network stack` line, no `dwc2d-watch:`
  line, no USB device removed, no panic. No `netstack-watch:` line either: the boots were reset
  after about 60 s, before its first 60 s report.
- All 11 stable boots in between came up with network on the first try.
- Stable image staged again and booted (`v0.4.0-dev-604-g8c4c513f`), devsh answers,
  `/srv/tftp/redox` matches `/srv/tftp/stable`.

So 0 of 10 this time (earlier 3 of about 14), so 10 boots were not enough to catch it.
Outputs (serial per boot) in `~/pitest-tools/t264/`.

## #266 #217 boot race: 40 warm resets out of stable (2026-11-07, Pi test agent)

Image `bnr-v0.4.0-dev-443-g6e17ce46+x814f.img` (as #264). Stable `v0.4.0-dev-604-g8c4c513f`.
Runner `run264.sh` with 40 cycles and an ARP check only. 07:36–08:37 UTC.

**Result: the #217 bug was not caught in 39 boots, but then the Pi stopped booting at all.**
- Boots 1–39 of this image: devsh after 30–37 s, `ping -c 3 192.168.178.1` 3 of 3, the Pi
  `REACHABLE` in ARP from homenodellm. Every boot `devshd: starting` then `listening`; no
  `no answer from the network stack`, no `dwc2d-watch:` line, no panic. The 39 stable boots
  in between all had network on the first try.
- **After boot 39**, the reset back into stable (08:23) never reached the kernel. Every boot
  since then, 4 in a row including 3 power cycles, stops in the Redox bootloader:
  `Looking for RedoxFS:` … `SETUP PANIC: panicked at src/os/uefi/disk.rs:31:9: internal error:
  entered unreachable code`. U-Boot, TFTP and the HTTP image load (44177622 bytes, the stable
  file, checked) are fine.
- Cause: `disk.rs:31` is `DiskOrFileEfi::write_at` (`unreachable!()`). The bootloader opens every
  RedoxFS it finds, and our patch `bootloader/0002` (only boot a RedoxFS with a kernel) checks
  each one with `fs.tx(...)`, which ends in `tx.commit()`. The SD card's data partition
  (`/scheme/data`, which it scans before the network image) must now be in a state where that
  commit writes, so the bootloader panics before it reaches the network image. Before, it
  printed `RedoxFS on …HD(0x2,MBR,…) has no kernel, skipping`. What changed the data
  partition is not known; it happened between boot 39 of this image and the next stable boot.
- **The Pi does not boot now** (any image, it panics in a loop). Stable is staged in
  `/srv/tftp/redox` (matches `/srv/tftp/stable`).
- Fix without a person: a bootloader where `has_kernel` does not commit (or `write_at` returns an
  error instead of panicking), staged as the stable image. Otherwise a person has to repair or
  reformat the data partition on the network-boot card.

Outputs (serial per boot, `stuck-serial.log` with the panics) in `~/pitest-tools/t266/`.

## #254 open firmware step 2 rerun: lk-overlay with an ARM-side SD driver (2026-11-07, Pi test agent)

Card `redox-rpi-lkoverlay-3bp-20261105c.img.xz` on the spare card (user swap). 90 s of serial,
08:47–08:48 UTC. No HDMI connected.

**Result: the ARM loader now mounts the card, but reading the files fails.**
- VPU stage as in #253: PLLs up, `SDRAM Type: Elpida 1GB LPDDR2`, self test ok, the VPU's eMMC
  driver at 27 MHz reads the card (30475 MB SDHC), stage 1 chainloads, `enabling aarch64`,
  `welcome to lk/MP` with the DTB at 0x1000000.
- On the ARM: `[EMMC:...]` lines, `SD card has arrived`, `card now in 4bit mode`, then
  `vpu clock is 0, cdiv 0`, `Card initialization complete: N/A 30475MB SDHC Card`, MBR dump
  (p0 type 0xc at 0x800, p1 type 0x83 at 0x10800).
- `trying to boot from sdhostp1` **without** `mount failure`, then `dtb: rpi3.dtb, kernel:
  Image-aarch64`. A 576-byte file was read fine.
- Reading the 35252-byte raw dtb: 136 lines `transfer error on FIFO word N: 0x21` (N = 112, 368,
  496, 624, 880) with `Transfer error, status: 0x20`. Last line (4.065 s):
  `[EMMC:sdhost_read_block_wrap]: read of sector 73520, count 8 failed`, then silence to 90 s.
- No magic, no `passing control off to linux`, no payload line, no panic or exception.
- Probable cause: `vpu clock is 0, cdiv 0`, so the ARM side runs the SD clock with divider 0.
- Raw log sent to the open-firmware session (`~/pitest-tools/t254/serial-raw.log`). The stable
  network-boot check follows when the Pi boots again (see #266).

## #265 #230 our own bootcode.bin, milestone M1 (2026-11-07, Pi test agent)

Card `redox-rpi-bootcode-m1-3bp-20261107.img.xz` on the spare card (user swap, same visit as
#254). 90 s of serial, 08:52–08:54 UTC. No HDMI connected.

**Result: pass.** All expected lines, in order:
`redox-rpi bootcode m1-20261107 (M1)`, `VPU cpuid 0x4000140`, `from the Boot ROM: CM_VPUCTL 0x241
CM_VPUDIV 0x1000 PLLC_CTRL 0x10000 PLLC_FRAC 0x0 SD_CS 0x794200 timer 2679049 us`,
`[sdram] controller up, SD_CS 0x218E42`, `[sdram] Elpida 1 GB LPDDR2 (MR5 0x3, MR8 0x58)`,
`[sdram] DDR PLL locked, 398400 kHz`, `[sdram] self test passed, 1024 MB, 32 regions, 28088 us`,
`SDRAM ready after 52436 us`, `M1 done: stage 1 runs, SDRAM works. Nothing to load yet.`, then
`alive 10 s` to `alive 60 s`. Last line `alive 60 s`, nothing after it up to 90 s. No PANIC or
EXCEPTION. Raw log `~/pitest-tools/t265/serial-bootcode-m1.log`, sent to the open-firmware
session. The stable network-boot check follows when the Pi boots again (see #266).

## #268 open firmware step 2: lk-overlay with the ARM SD clock fixed (2026-11-07, Pi test agent)

Card `redox-rpi-lkoverlay-3bp-20261107.img.xz` on the spare card (user swap). 90 s of serial,
08:58–08:59 UTC. No HDMI connected.

**Result: the SD reads work now; the loader stops at the DTB with `fdt_open_into() == -3`.**
- VPU stage and `welcome to lk/MP` as #254.
- ARM side: `vpu clock is 250, cdiv 10`, `card now in 4bit mode`, `changing clock to 25MHz for
  data mode`, 30475 MB SDHC.
- `trying to boot from sdhostp1`, `dtb: rpi3.dtb, kernel: Image-aarch64`, read 576 bytes, then
  `read 35252 bytes` of the raw dtb. **0 transfer errors** in the whole log (#254: 136).
- Then `loaded DTB file to 0xffff000003100000`, `failed to open initrd: -1`,
  `ERROR: fdt_open_into() == -3` (libfdt: no space). After that only three `temp changed` lines;
  last line `temp changed 27.868011 -> 30.020020`, then silence to 90 s.
- No magic, no `passing control off to linux`, no payload, no panic or exception.
- Raw log `~/pitest-tools/t268/serial-raw.log`, sent to the open-firmware session. The stable
  network-boot check follows when the Pi boots again (see #266).

## #269 #230 serial monitor in our own bootcode.bin (test build) (2026-11-07, Pi test agent)

Card `redox-rpi-bootcode-monitor-3bp-20261107b.img.xz` on the spare card (user swap, checksum
checked by the user). Everything else over serial (`~/rbc-build/payloads/rbc-load.sh`, which only
writes to `/dev/ttyUSB0` and reads the serial log) and the Shelly. 09:29–09:38 UTC. No HDMI.

**Result: A, B, C, D pass; E reaches U-Boot over the network but stops there.** No PANIC or
EXCEPTION in any part. No load retries were needed (every load matched its crc32 at once). The first
start of B failed only because the updated `rbc-load.sh` was not executable (rc 126, nothing sent);
it was then run with `bash`.
- **A, no input:** `redox-rpi bootcode mon-20261107b (M1, monitor, test build)`, Elpida 1 GB,
  `[sdram] self test passed, 1024 MB, 32 regions`, `rbc> monitor: press Enter within 5 s`,
  `rbc> no input, normal boot`, `M1 done`, `alive 10 s`, `alive 20 s` (30 s window).
- **B, commands:** `loaded 8 bytes … crc32 89E09EC4`; `r 7e003004` → `ok 7E003004 0039203D`;
  `w`/`r c1100000` → `ok C1100000 12345678`; `s … f0000000` → `F2345678`; `c … f` → `F2345670`;
  `d c1000000 8` → `C1000000: 10 42 40 B0 00 10 5A 00`, `ok`; `crc` → `ok crc32 89E09EC4`;
  `g c1000000 5 7` → `ok ret 0000100C`; `foo` → `err unknown command or wrong argument count: foo`;
  `boot` → `ok boot`, `M1 done`, `alive 10 s`. All as expected.
- **C, lk stage 2 over serial:** 296048 bytes loaded in about 52 s, `crc32 1EEB420E` matched.
  `g c4000c72`: `PLLC target 500 MHz`, `welcome to lk`, `enabling aarch64`, `welcome to lk/MP`,
  `vpu clock is 250, cdiv 10`, 25 MHz, `dtb: rpi3.dtb, kernel: Image-aarch64`, dtb read 35252
  bytes, **no `fdt_open_into` error**, `magic: 0x644d5241`, `core 0 passing control off to
  linux!!!`, `redox-rpi: 64-bit test payload reached (loaded as Image-aarch64)`, `EL = 1`,
  `MIDR = 0x410FD034`, `MPIDR = 0x80000000`, `x0 FDT = 0x3000000`, `done, waiting (wfe)`.
  Last line `temp changed 25.716003 -> 27.868011`.
- **D, chain lk stage 1:** 49956 bytes, `crc32 4ACB9CA3`, `ok chain 49956 bytes, start
  0x80000200`. lk's stage 1 started and ran its SDRAM set-up a second time without trouble
  (Elpida 1GB, self test successful), loaded stage 2 from the card, and reached the payload the same
  way as C. Last line `redox-rpi: done, waiting (wfe)`.
- **E, chain Broadcom bootcode:** 52624 bytes, `crc32 53C9B136`, `ok chain`. Broadcom's bootcode fell
  back to the network and U-Boot came up (`U-Boot 2026.10 (Oct 27 2026)` from /srv/tftp), but with
  **`DRAM: 240 MiB`** (normally about 948 MiB). DHCP and the `redox/netboot.scr` download worked, then
  `wget error: trying to overwrite reserved memory`, `HTTP client error 4`, the TFTP fallback for
  `harddrive.img.gz` at 0x20000000 failed the same way, and it stopped at `U-Boot>`. So the monitor
  card cannot stay in the Pi yet.
- Logs `~/pitest-tools/t269/part-A.log` to `part-E.log`, sent to the open-firmware session. The Pi
  is off. The network-boot card goes back next, and the stable check follows when the bootloader fix
  for #266 arrives.

## #270 #232 bootloader fix for the SD data partition panic (PR #234) (2026-11-07, Pi test agent)

Image `~/t18-work/image-blw/harddrive.img` (`v0.4.0-dev-643-g58df647b+x9f76`, branch
`bootloader-write-error`, timesync + rtl8152, md5 `bfb95c43…`), staged with the t18 tree;
`/srv/tftp/redox/bootloader-live.efi` md5 `a9ec1934…` (checked). Network-boot card back in the Pi,
the same card whose data partition made the old bootloader panic in #266. 09:48–09:51 UTC.

**Result: pass. The Pi boots again.**
- **Boot 1** (power on): `Looking for RedoxFS:`, `DiskEfi::write_at 0xA09 not implemented`,
  `WARN - BlockIo error: I/O error`, then `RedoxFS f1b8bded…: 222 MiB` (the network image) and
  `redox login:` 30 s after power on. No `SETUP PANIC`. devsh answers, `/scheme/data` mounted
  (timesync.last, the Qwen model and older files all there).
- **Boots 2 and 3** (power cycles): the data partition shows `RedoxFS on … has no kernel,
  skipping`, then the network image, login after 30–31 s, no panic, `/scheme/data` mounted.
- `pi-test.py`: version, manifest, ps (51), ps-tty, sdcard, starts 0 of 300 failed, all PASS.
- `gpu_mem` unchanged: `/scheme/rpi-firmware/board` shows `vc_memory 0x3e600000 16777216`
  (16 MiB), `arm_memory 1046478848`.
- This image stays staged as stable until PR #234 is merged and the nightly builds with it
  (copy in `~/pitest-tools/tftp-stable-643`; `/srv/tftp/stable` still has 604 with the old bootloader).

## #204 #33 desktop with apps, user at the screen (2026-11-07, Pi test agent + user)

Image `netboot-204-vc4d-v0.4.0-dev-695-g0a7a91b9+x77c4.img` (md5 `b08e9e05…`, experiments
`orbital orbital-apps timesync usb-mouse vc4d`), network boot by power cycle with HDMI (NanoKVM)
attached, USB keyboard and mouse. The staged bootloader has patch 0003. 10:31–10:46 UTC.

**Result: login and Files work; the Text Editor starts but is very slow, and Save opens no dialog.**
- Serial: `vc4d: check: PASSED` (11 passed, 0 failed, list read-back now passes too), watch to
  120 s PASS, `experiments: orbital orbital-apps timesync usb-mouse vc4d`, login after 32 s. No panic.
- **User:** login with Enter works. The launcher shows Terminal, COSMIC Files, COSMIC Text Editor and
  Viewer. COSMIC Files opens and draws. The editor needs about 40 s to show its window; its menus are
  very slow (File opened once); choosing Save did nothing (no dialog). Edge bars still visible on the
  NanoKVM.
- **Agent, editor start (10:35):** cosmic-edit (pid 45) had used about 12 s of CPU for its font set-up
  (`cosmic_text … failed to get system locale, falling back to en-US`), then its main thread sat
  blocked (UB) with no CPU growth over 3 samples, before the window appeared.
- **Agent, editor open (10:43–10:44, ps deltas over two 11 s windows):** cosmic-edit 8.6–8.8 s CPU,
  the root filesystem daemon (initfs `redoxfs`, pid 20) 4.9–10.9 s, dwc2d 1.1–1.2 s, orbital
  0.5 s, the rest about 0. So the editor and RedoxFS together keep about 1.5–2 cores busy: the
  editor seems to read from the filesystem constantly. (`cputop` is not in this image.)
- With both apps open: `free` 343520 KB of 739924 KB used (396404 KB free); `df /` 180224 of
  227328 KB used (79 %).
- vc4shot at 10:44 (`~/pitest-tools/t204/shot.png`): the editor full screen with `hallo Redox`
  typed and the `Save changes before closing?` dialog; taskbar with four launchers and the clock;
  colours and pointer right; **no edge bars in vc4d's own picture** (as in #259), so the bars come
  from the HDMI/NanoKVM side.
- Serial after login: only the launcher's `failed to wait for background (43): No child processes`
  and cosmic-files' thumbnail-directory note (expected on first run). No panic.
- **User, addendum:** on a second start the editor came up faster; clicking File reacts unreliably;
  File → Save as… never showed a file dialog; then the editor window vanished. At 10:49 `ps` has no
  cosmic-edit any more (orbital, launcher and cosmic-files still run) and serial shows no line for its
  end (no panic, no crash message).

## #272 #33 COSMIC Text Editor without a person, with app-log and redoxfs-reads (2026-11-07, Pi test agent)

Image `netboot-edit-vc4d-v0.4.0-dev-698-g51017bba+xd62c.img` (experiments `timesync orbital usb-mouse
orbital-apps vc4d cputop pointer-bench app-log redoxfs-reads`), network boot by power cycle, HDMI
(NanoKVM) attached. **No serial console:** the USB serial adapter disappeared from homenodellm at
10:39 (during #204), so instead the kernel log `/scheme/sys/log` (it holds the `/scheme/debug` lines)
was copied over devsh every 5 s and merged by overlap (no gaps). Note: each copy is a devsh login plus
`cat`, so the `login`, `ion`, `cat`, `ld.so.1` and `libgcc_s.so.1` reads in the log are mostly the copier's.
First try at 11:51: the image never answered devsh or ARP (cause unknown without serial; U-Boot's DHCP
was seen); the second boot at 12:02 came up in 41 s. Steps 12:04–12:09 UTC.

**Result: the editor works but is slow; Save as opens its dialog only after 20–55 s; Ctrl+Q did not quit.**
- 12:04:11 login with Enter (pointer-bench). 12:04:17 `MARK-START`, `app-log: start cosmic-edit`.
  The binary is read twice in full (`usr/bin/cosmic-edit read 27040671`, then `27094856`), then
  `cosmic_text … falling back to en-US`, `fontdb … Fallback to loading from known font dir paths`,
  the five Fira fonts mapped (each 2–5 times), icon themes read.
- **cputop, first start** (20 s each): A: redoxfs 26.5 % (411 wakeups/s), fbbootlogd 8 %, dwc2d 4.9 %,
  cosmic-edit 4.4 % (743/s), orbital 0.9 %. B and C: everything idle (redoxfs 2.8 %, dwc2d 5 %,
  fbbootlogd 3.7 %); cosmic-edit not in the list.
- **Typing** (12:06:53, 100 keys at 10/s, `typed 100 keys`; cputop 15 s): cosmic-edit 38.1 % (815/s),
  redoxfs 23.7 % (384/s), orbital 8.5 %, dwc2d 5.5 %. All 100 characters arrived (picture 1).
- **Save as** (12:07:47 Ctrl+Shift+S; cputop 20 s): **redoxfs 78.6 % (1239/s), cosmic-edit 55.3 %
  (4153/s)**, fbbootlogd 13.1 %, cpu1 93 % busy. Picture 1 at 12:08:10 (about 20 s later): the editor
  with the typed line, **no dialog yet**. Picture 2 at 12:08:43 (after Ctrl+Q at 12:08:21): the **Save
  file dialog is open** (Home = /root, `Empty folder (has hidden items)`, details pane `Size:
  Calculating…`). So the dialog took between 20 and 55 s to appear, and Ctrl+Q went to the dialog:
  cosmic-edit still ran at 12:08:49 (main thread UR, 1 min 9 s system time).
- **redoxfs-reads from MARK-START to the end (797 log lines), leaving out the copier's reads:**
  `usr/share/icons/Cosmic/index.theme` (mapped 8192) 114 times: 15 at start, 6 typing, 51 after Save,
  42 after Quit; `usr/share/icons/hicolor/index.theme` (mapped 57344) 42 times; `usr/lib/libonig.so.5`
  (read 649136) 38 times (19 at start; may partly be the copier's ion); fonts 2–5 times each; every
  program binary is read twice in full when it starts (for example `usr/bin/true` about 10.8 MB twice,
  `usr/bin/base64` 10.8 MB twice). No `redoxfs-reads` line explains the 79 % redoxfs CPU during the
  dialog by itself: the index.theme files are mapped, not read, so the load is probably many small
  reads or directory lookups below the 4 KiB threshold.
- `app-log`: only `start cosmic-edit`; no `exited` line (it was still running). No panic in the log.
- Stable 643 staged again and booted (devsh at 12:10:38).
- Files: `~/pitest-tools/t272/` (cputop outputs, `log-from-mark-start.txt`, `save.png`, `quit.png`).

## #274 #33 COSMIC editor with the icon lookup fix, real save (2026-11-07, Pi test agent)

Image `netboot-cas-vc4d-v0.4.0-dev-701-gc873bed8+x1225.img` (experiments as #272 plus
`cosmic-apps-source`), network boot by power cycle (first boot fine, devsh after 38 s), HDMI (NanoKVM).
Still no serial adapter: `/scheme/sys/log` copied over devsh every 5 s and merged by overlap (no gaps);
the copier's own `login`, `ion`, `cat`, `ld.so.1`, `libgcc_s.so.1` reads are left out below.
Steps 12:56–13:01 UTC.

**Result: pass. The fix works: index.theme is read once per theme, the Save dialog comes up, the save
writes the file and Ctrl+Q exits with 0.**
- 12:56:05 login; 12:56:16 `MARK-START`, `app-log: start cosmic-edit`.
- **First start** (cputop 3 × 20 s): A redoxfs 24.9 %, cosmic-edit 5.7 %; B and C idle (redoxfs 2.8 % / 4.0 %).
  (#272: 26.5 % / 4.4 %, then idle; the start itself is about the same.)
- **Typing 100 keys** (12:58:46, `typed 100 keys`; cputop 15 s): **cosmic-edit 56.7 % (134 wakeups/s),
  orbital 13.7 %, redoxfs 4.5 %** (#272: cosmic-edit 38 % at 815/s, redoxfs 24 %). So the redoxfs load
  while typing is gone; the editor itself still needs about half a core for 10 keys/s.
- **Ctrl+Shift+S** (12:59:31; cputop 20 s right after): cosmic-edit 72.4 % (2161/s), redoxfs 33.8 % (544/s),
  orbital 3.8 % (#272: redoxfs 78.6 %, cosmic-edit 55.3 %). The first picture, at 12:59:59 (28 s after the
  keys, the first moment it was taken), shows the **Save file dialog open** (Home = /root, `Size: 0 B`, no
  more `Calculating…`); the second at 13:00:34 the same. (#272: no dialog at +20 s, open at +55 s.)
- **Save:** `pointer-bench --type 'savetest\n'` at 13:00:46; `ls -l /home/root` at 13:00:58:
  `savetest`, **100 bytes**.
- **Ctrl+Q** at 13:01:00: `app-log: cosmic-edit exited with 0`; the picture at 13:01:26 shows the empty
  desktop with the taskbar; `ps` has no cosmic-edit.
- **redoxfs-reads from MARK-START:** `usr/share/icons/Cosmic/index.theme` **once**,
  `usr/share/icons/hicolor/index.theme` **once** (#272: 114 and 42 times). Fonts mapped 1–5 times each,
  `cosmic-edit` read twice in full at start, `libonig.so.5` 45 times (probably the copier's ion logins).
- No panic. Stable 643 staged again and booted.
- Files: `~/pitest-tools/t274/` (cputop outputs, `log-from-mark-start.txt`, `4-shot.png`, `4-shot2.png`, `6-shot.png`).

## #273 #240 ld.so reads a program once (relibc patch 0017, PR #241) (2026-11-07, Pi test agent)

Image `~/t18-work/image-dbl/harddrive.img` (`v0.4.0-dev-674-gc350bf7c+x9f76`, stable-style), staged
with the t18 tree (`bootloader-live.efi` md5 `a9ec1934…`, checked), power cycle. Serial was back
(adapter replugged at 13:39), so the serial log was kept too. 13:42–13:46 UTC.

**Result: pass; the timing shows no difference at this resolution.**
- devsh answered 29 s after power on. `pi-test.py`: version, manifest, ps (46), ps-tty, sdcard,
  starts 0 of 300 failed, all PASS.
- `env`, `ls /usr/bin`, `uname -a` (`Redox redox 0.5.12 … aarch64 Redox`) work.
  `echo y | pkg install nano` completed (`Commit complete.`), `nano --version`: `GNU nano, version 9.2`.
- Timing, `ion -c "for i in 1..=10; ion -c true; end"` over devsh, 3 runs each: stable 643
  0.9, 0.9, 0.9 s; this image 0.9, 0.9, 0.9 s. devsh reports to 0.1 s, so 10 starts of ion are too
  few to show the saving (about 2.4 MB less to read per start); a loop of 100 or more would be needed.
- No panic on serial. Stable 643 staged again and booted (devsh at 13:46:14).

## #275 #33 desktop with apps, person at the screen (2026-11-07, Pi test agent)

Network image `netboot-204b-v0.4.0-dev-702-ge7bafd21+x35fd.img` (branch `graphics-vc4d`, experiments
`app-log cosmic-apps-source orbital orbital-apps timesync usb-mouse vc4d`), staged with the graphics
`netboot.sh`, power cycle 13:53:32 UTC. The desktop SD card was in the Pi (so no `/scheme/data`).
The user at the HDMI monitor, logged in as root (the 702 image starts no app for `user`).

**Result: fail. The editor's Save dialog never appeared, and the monitor showed noise bands.**
- Boot: login 13:54:16, `vc4d: check: … PASS` at 2, 5, 10, 30 and 120 s, no panic.
- `app-log: start cosmic-files`, `start cosmic-edit` (13:57), `cosmic-edit exited with 0`, then
  `start cosmic-edit` again (14:02). After the `cosmic_files::localize` line at 14:02:14 (the Save
  dialog starting) nothing more came. The user saw no dialog.
- Check at 14:04 over devsh: all apps idle (ps deltas), running as uid 0. `free` 339884 of
  739796 KB used, `df /` 79 %. `vc4shot` was clean (`t275/shot.png`), so the bands the user saw on
  the monitor are after the HVS (pixel valve or HDMI), not in the picture vc4d draws. That is
  row #276.
- Log noise: 5× `ion: … failed to redirect stdout to file '/scheme/debug': Operation not permitted`,
  `procmgr … failed to send SIGCHLD to parent PID 37: Operation not permitted`, one `funmap passed
  length 0xc800 instead of 0xd000` warning.
- **After the visit the Pi hung.** The netboot card was put back and the 702 image booted again
  (about 14:08). It stopped after `init: switchroot to /usr /etc` (14:09:15): no data partition
  mount line, no login, no devsh. The Shelly showed power on, 3.2 W. Power cycle at 15:49 with
  stable 643 staged: devsh after 20 s, `/scheme/data` mounts. One hang, not repeated.

Stable `v0.4.0-dev-643-g58df647b+x9f76` staged and booted, devsh answers. Outputs in
`~/pitest-tools/t275/` on homenodellm.

## #276 #188 #33 vc4d scanout while the desktop draws (2026-11-07, Pi test agent)

Network image `netboot-vdiag-v0.4.0-dev-707-gc1432a37+xb3fe.img` (branch `graphics-vc4d` c1432a37,
vc4d `scanout-diag`; experiments `app-log cosmic-apps-source cputop orbital orbital-apps
pointer-bench timesync usb-mouse vc4d vc4d-diag`). HDMI connected as at #275, serial on.
15:51–16:08 UTC. No person, so no look at the monitor; the NanoKVM was not used.

**Result: the HVS side looks clean. No underrun, no foreign display list, buffers contiguous.**
- Boot 1: login and vc4d checks fine, network link up, but no `devshd: listening` line and no
  devsh for 3 min (looks like the #217 boot race). Power cycle; boot 2: devsh after 40 s.
- `vc4d: buffer …: contiguous` for every buffer, no `NOT CONTIGUOUS`. `vc4d: check:` PASS at
  2, 5, 10, 30, 60 and 120 s.
- `vc4d: scanout:` minute lines at 180–482 s: `errors in 0 of 60 reads` every time, through
  typing (MARK-TYPE), both drags (MARK-DRAG1/2) and the idle minutes. No `EUFLOW`, no
  `channel reads the list at …, not ours`.
- `60 list findings in the last minute` all the time, idle too: always word 3 of the active list
  (`list word 2051` or `2115`, written `0xc0c0c0c0`, read back a changing `0x0…0000`). That is the
  plane's dummy position word, which the HVS uses as its own scratch word (Linux writes a dummy
  there too). So it is very likely a false finding of the diagnostic, not a display list fault.
- Typing 200 letters (`pointer-bench --type`): cosmic-edit 82 %, orbital 19 %, cpu0 69 % busy.
  Dragging the title bar (`pointer-bench 30 --drag 1000 156`, 1800 moves in 30 s, twice): the
  window moved. `free`: 305–315 MB used of 740 MB.
- vc4shots: `3-shot.png` (editor up), `5-shot.png` (typing) clean. `4-drag.png` (during the drag)
  shows the window half redrawn: the top at the new place, the old lower body still over the
  taskbar. That is in the frame vc4d scans out (Orbital's damage during the drag), not HDMI noise.
- No panic. app-log: `start cosmic-edit`.

So nothing on the HVS side matches the bands the user saw at #275. Either the fault is after the
HVS (pixel valve or HDMI), or what the user saw was the half-redrawn frames during moves.
A look at the monitor during a drag would tell. Stable `v0.4.0-dev-643-g58df647b+x9f76` restored,
devsh answers. Outputs (serial, shots, cputop) in `~/pitest-tools/t276/` on homenodellm.

## #277 #243 boot menu: network boot with the new netboot.scr (PR #244) (2026-11-07, Pi test agent)

Image `~/t18-work/image-blw/harddrive.img` (md5 `bfb95c43…`, the stable `v0.4.0-dev-643-g58df647b+x9f76`,
`bootloader-live.efi` md5 `a9ec1934…`), staged with `~/t18-work/menu-stage/scripts/netboot.sh`
(branch `boot-menu`, new `netboot.scr` 5146 bytes, md5 `9528ce56…`). 3 power cycles, 16:09–16:13 UTC.
The serial adapter was plugged in, so serial was logged too.

**Result: pass. 3 of 3 boots, devsh after 31, 29 and 32 s (stable: about 29 s).**
- Each boot on serial: `Filename 'redox/netboot.scr'`, then `Unknown command 'bootmenu' - try
  'help'` (this U-Boot has no `bootmenu` yet, as expected), `redox-rpi: boot net redox/`,
  `network boot from 192.168.178.120`, `image loaded over HTTP`. No panic.
- `BUILD_ID` after each boot: `v0.4.0-dev-643-g58df647b+x9f76`.
- `pi-test.py`: pass (version, manifest, ps 51, ps-tty, sdcard on `/scheme/data`, starts 0 of 300).
- The stable files (old `netboot.scr` too) went back into `/srv/tftp/redox` (`stable/` not touched),
  and the stable image booted, devsh answers.

The menu on HDMI with the new `u-boot.bin` is still to test with a person. Outputs in
`~/pitest-tools/t277/` on homenodellm.

## #278 #247 TinyCC compiles and runs C on the Pi (2026-11-07, Pi test agent)

Network image `netboot-tcc-v0.4.0-dev-689-gb453b397+xd12c.img` (branch `tcc` b453b397, experiments
`tcc timesync`). Real Pi 3B+, power cycle 16:31:25 UTC, devsh after 29 s. 16:31–16:33 UTC.

**Result: pass.** `Hello from tcc on Redox` three times (`./hello`, `tcc -run`, `./hello2` from the
self-built `tcc2`); bench results the same as gcc's; no `UNHANDLED EXCEPTION` or panic on serial.
- `tcc version 0.9.28rc 2026-11-07 HEAD@70adbb12* (AArch64 Linux)`.
- hello (dynamic): `[timeit] 100.0 ms` tcc, `29.9 ms` ./hello.
- hello (static): expected failure, `46718.3 ms, status 1`, `Unknown relocation type for got: 562`
  (also 563, 564, 569).
- `tcc -run`: `130.0 ms`.
- bench compiled by tcc: `# 4616 lines, 0.084 s, 54952 lines/s`, compile `162.3 ms`.

| bench | tcc run 1 | tcc run 2 | gcc -O2 run 1 | gcc -O2 run 2 |
|---|---|---|---|---|
| fib(32) = 2178309 | 120.1 ms | 120.3 ms | 36.1 ms | 36.1 ms |
| sieve 5x2M = 148933 | 1245.4 ms | 1245.7 ms | 951.1 ms | 948.2 ms |
| mandel 200 = 9800 | 115.3 ms | 118.3 ms | 35.3 ms | 35.3 ms |
| total (timeit) | 1514.2 ms | 1519.7 ms | 1059.1 ms | 1054.3 ms |

- Self-hosting, tcc compiles tcc.c (77143 lines): run 1 `0.703 s, 109733 lines/s`, `[timeit] 822.8 ms`;
  warm run `0.716 s, 107741 lines/s`, `831.1 ms`. `./tcc2` builds hello2 in `101.2 ms`.
- Whole run.ion took about 50 s; most of it is the static link (47 s; QEMU 80–95 s).
- `free` afterwards: 159820 of 739896 KB used.

Stable `v0.4.0-dev-643-g58df647b+x9f76` restored, devsh answers. Outputs in `~/pitest-tools/t278/`.

## #279 #250 PWM audio driver, stages A1 and A2 (2026-11-07, Pi test agent)

Network image `pwm-audio-v0.4.0-dev-704-gfa8f2ef9+x6d83.img` (branch `pwm-audio`, md5 `0fcec28e39f8…`).
Real Pi 3B+, power cycle 16:54:20 UTC, devsh after 45 s. Nothing connected to the audio jack. 16:54–16:59 UTC.

**Result: fail. Everything runs, but the PWM plays at half the rate: 22050 instead of 44100 frames/s.**
- (1) Boot: `bcm2835-pwmaudiod: PWM at 0x3f20c000, clock 250 MHz (PLLD/2), range 5669 (44099 Hz), DMA
  channel 2 at 0x3f007000, 8 x 512 frames` (44099, the row says 44098). No `playing to the clock`. The
  only audiod line is `uring communication is not supported …`. No panic, no `UNHANDLED EXCEPTION`.
- (2) `/scheme/audio` and `/scheme/audiohw` are there; `/scheme/gpio/pins`: `40 alt0 1`, `41 alt0 1`.
- (3) stats at 37.7 s and 68.5 s: `mode pwm-dma`, **`measured_rate_hz 22050` and `22051`** (pass range
  43900–44300). `played_slots` 1625 → 2952 (+1327 in 30.8 s; about 2580 expected). `dma_faults 0`,
  `underruns 1` (no change), `queued 6`, `pwm_errors 2093 → 4981 bits=0x030` (STA bits 4 and 5, GAPO1 and
  GAPO2: gap on both channels, about 94 per second).
- (4) `echo 3 > /scheme/audiohw/tone` returned after 6.3 s (not 3 s): `tone seconds=6.07 words=264600
  rate_hz=21798 fifo_empty=66 sta=0x030`. 264600 words is 3 s of stereo at 44100, so the CPU tone also
  ran at half speed. 10 s later: `played_slots` 3026 → 3491 (rising), `dma_faults 0`.
- (5) `tone check`: `short write: EINVAL (good)`, `full chunks accepted before EWOULDBLOCK: 0 (good)`.
- (6) `time tone -s 5 > /scheme/audio`: `real 9.869 s`, rc 0 (twice the 5 s). `tone sweep -s 5` rc 0,
  about 10 s. `chunks` 3240 → 4252, `underruns 1`, `queued 7`.
- (7) CPU around `tone -s 30 > /scheme/audio` (took 60 s, rc 0), ps ACTTIME/SCHTIME:
  `bcm2835-pwmaudiod` 6.33/6.26 s → 6.60/6.48 s (+0.27/+0.22 s, about 0.8 % of a core). It had used
  6.3 s in its first 2.5 min (most of that before the test, probably during boot or the CPU tone).
  `audiod` two threads: 0.12/0.11 → 0.35/0.27 s and 0.31/0.47 → 0.63/0.92 s (+0.84 s total).
  stats after: `measured_rate_hz 21296`, `played_slots 7395`, `dma_faults 0`, `pwm_errors 6069`.
- (8) `pi-test.py`: pass (version, manifest match, ps 53, ps-tty, sdcard, starts 0 of 300).

So the DMA ring and the driver work without faults, but the PWM takes words at half the expected
speed (about 44100 words/s for stereo, so 22050 frames/s), with GAPO on both channels. Either the PWM
clock is 125 MHz and not 250 MHz, or each channel takes its word once per range period only on
alternate periods. That is for the audio session. Stable `v0.4.0-dev-643-g58df647b+x9f76` restored,
devsh answers. Outputs in `~/pitest-tools/t279/`.

## #281 #250 why the PWM plays at half rate: diagnosis (2026-11-07, Pi test agent)

Network image `pwm-audio-v0.4.0-dev-706-g54a81fc1+x6d83.img` (branch `pwm-audio` 54a81fc1, md5
`a1b04fde1193…`). Real Pi 3B+, power cycle 17:04:46 UTC, devsh after 30 s. 17:04–17:07 UTC.

**Result: the channels take turns on one FIFO word per period. The 250 MHz clock is right.**
Words per second follow only range and divisor, not mono or stereo, so stereo gets half the frames:

| case | words/s | frames/s | expected (take turns) | expected (half clock) |
|---|---|---|---|---|
| (a) `2` stereo | 44099 | 22050 | 44100 words/s | |
| (b) `2 mono` | 43515 | 43515 | 44100 | 22050 |
| (c) `2 range=2834` | 88215 | 44107 | 88200 | |
| (d) `2 mono range=2834` | 84715 | 84715 | 88200 | 44100 |
| (e) `2 div=4` | 22050 | 11025 | 22050 | |
| (f) `2 mono div=4` | 21918 | 21918 | 22050 | |

The six lines (from serial; over devsh `grep '^tone' /scheme/audiohw/stats` printed nothing, the
final `cat` showed the same last line):
```
bcm2835-pwmaudiod: tone 1000 Hz, stereo range=5669 div=2 seconds=4.00 words=176400 words_per_s=44099 rate_hz=22050 fifo_empty=0 sta=0x030
bcm2835-pwmaudiod: tone 1000 Hz, mono range=5669 div=2 seconds=2.03 words=88200 words_per_s=43515 rate_hz=43515 fifo_empty=24 sta=0x000
bcm2835-pwmaudiod: tone 1000 Hz, stereo range=2834 div=2 seconds=2.00 words=176400 words_per_s=88215 rate_hz=44107 fifo_empty=0 sta=0x030
bcm2835-pwmaudiod: tone 1000 Hz, mono range=2834 div=2 seconds=1.04 words=88200 words_per_s=84715 rate_hz=84715 fifo_empty=32 sta=0x000
bcm2835-pwmaudiod: tone 1000 Hz, stereo range=5669 div=4 seconds=8.00 words=176400 words_per_s=22050 rate_hz=11025 fifo_empty=0 sta=0x030
bcm2835-pwmaudiod: tone 1000 Hz, mono range=5669 div=4 seconds=4.02 words=88200 words_per_s=21918 rate_hz=21918 fifo_empty=65 sta=0x000
```
- Stereo runs end with `sta=0x030` (GAPO1/GAPO2) and `fifo_empty=0`; mono runs `sta=0x000` with
  `fifo_empty` 24–65.
- Afterwards `cat /scheme/audiohw/stats`: `played_slots` 1271 → 1522 in 5.8 s (rising), `dma_faults 0`,
  `underruns 1`, `measured_rate_hz` 12859/13807 (still recovering from the tones), `pwm_errors` 1570 → 1724
  `bits=0x030`.
- No panic. Stable `v0.4.0-dev-643-g58df647b+x9f76` restored, devsh answers. Outputs in `~/pitest-tools/t281/`.

So a stereo fix is range 2834 (c: 44107 frames/s), or another way to give each channel its own word per period.

## #282 #247 tcc: a C file run as a script (`#!/usr/bin/tcc -run`) (2026-11-07, Pi test agent)

Network image `netboot-tcc-v0.4.0-dev-716-gc6d4de06+xd12c.img` (branch `tcc` c6d4de06, experiments
`tcc timesync`). Real Pi 3B+, power cycle 17:07:54 UTC, devsh after 29 s. 17:07–17:09 UTC.

**Result: pass.**
- (1) `echo hello | timeit ./hallo.c a b`: `argv[0]=./hallo.c`, `argv[1]=a`, `argv[2]=b`,
  `Hallo, stdin 6 bytes`, `[timeit] 128.5 ms, status 42: ./hallo.c`, `rc=42`.
- (2) warm: the same lines, `[timeit] 127.4 ms, status 42`, `rc=42`.
- (3) `/tmp/t282/hallo.c x < /dev/null`: `argv[0]=/tmp/t282/hallo.c`, `argv[1]=x`, `Hallo, stdin 0 bytes`,
  `rc=42`.
- No panic or `UNHANDLED EXCEPTION` on serial.

Stable `v0.4.0-dev-643-g58df647b+x9f76` restored, devsh answers. Outputs in `~/pitest-tools/t282/`.

## #283 #250 PWM audio rerun with the half-rate fix (range 2834) (2026-11-07, Pi test agent)

Network image `pwm-audio-v0.4.0-dev-707-g7c92bc80+x6d83.img` (branch `pwm-audio` 7c92bc80, md5 `2dbdd7ba6e4e…`).
Real Pi 3B+, power cycle 17:11:39 UTC, devsh after 30 s. Nothing connected to the jack. 17:11–17:15 UTC.

**Result: pass. The PWM now runs at 44.1 kHz; times match the audio length.**
- (1) Boot: `bcm2835-pwmaudiod: PWM at 0x3f20c000, clock 250 MHz (PLLD/2), range 2834 (44107 Hz), DMA channel
  2 at 0x3f007000, 8 x 512 frames`. No `playing to the clock`; the only audiod line is `uring communication is
  not supported …`. No panic, no `UNHANDLED EXCEPTION`.
- (2) `/scheme/audio` and `/scheme/audiohw` there; `40 alt0 1`, `41 alt0 1`.
- (3) stats at 22.1 s and 52.9 s: `mode pwm-dma`, `measured_rate_hz 44111` both times, `played_slots`
  1900 → 4556 (+2656 in 30.8 s), `dma_faults 0`, `pwm_errors 0 bits=0x000`, `underruns 1` (no change),
  `queued 7/6`, `pwm_gap_polls` 1414 → 2759 (about 44 per second).
- (4) `echo 3 > /scheme/audiohw/tone` returned after 3.2 s: `tone stereo range=2834 div=2 seconds=3.00
  words=264600 words_per_s=88107 rate_hz=44053 fifo_empty=5 sta=0x030`. 10 s later `played_slots` 4697 →
  5629, `dma_faults 0`. (`measured_rate_hz` 41811–42952 in the minutes after the tone: the average
  includes the tone's pause of the ring.)
- (5) `tone check`: `short write: EINVAL (good)`, `full chunks accepted before EWOULDBLOCK: 0 (good)`.
- (6) `time tone -s 5 > /scheme/audio`: `real 4.956 s`, rc 0. `tone sweep -s 5`: 5.2 s, rc 0. `chunks`
  5115 → 6290, `underruns 1`.
- (7) `tone -s 30 > /scheme/audio`: 30.2 s, rc 0. ps ACTTIME/SCHTIME before → after: `bcm2835-pwmaudiod`
  3.39/0.29 → 3.61/0.42 s (+0.35 s in about 32 s, about 1 % of a core); `audiod` two threads 0.10/0.09 →
  0.30/0.19 and 0.37/0.64 → 0.68/1.12 s (+1.09 s total, about 3 %). After: `played_slots 9594`,
  `dma_faults 0`, `pwm_errors 0`, `pwm_gap_polls 4373`.
- (8) `pi-test.py`: pass (version, manifest match, ps 53, ps-tty, sdcard, starts 0 of 300).

Stable `v0.4.0-dev-643-g58df647b+x9f76` restored, devsh answers. Outputs in `~/pitest-tools/t283/`.

## #284 #250 PWM audio as default patches, short check (2026-11-07, Pi test agent)

Network image `default-pwm-audio-v0.4.0-dev-730-gaa76c1f9.img` (PR #251 at aa76c1f9, default patches, no
experiments, md5 `ee2b1fa16e83…`). Real Pi 3B+, power cycle 17:24:44 UTC, devsh after 29 s. 17:24–17:27 UTC.

**Result: pass.**
- (1) Boot: `bcm2835-pwmaudiod: PWM at 0x3f20c000, clock 250 MHz (PLLD/2), range 2834 (44107 Hz), DMA channel
  2 at 0x3f007000, 8 x 512 frames`; the only audiod line is `uring communication is not supported …`. No panic.
- (2) stats at 20.4 s and 51.2 s: `mode pwm-dma`, `measured_rate_hz 44125` / `44111`, `played_slots` 1758 → 4411,
  `dma_faults 0`, `pwm_errors 0 bits=0x000`, `underruns 1`, `pwm_gap_polls` 1298 → 2729.
- (3) `time tone -s 5 > /scheme/audio`: `real 4.955 s`, rc 0. `tone check`: `short write: EINVAL (good)`,
  `full chunks accepted before EWOULDBLOCK: 0 (good)`. Stats after: `measured_rate_hz 44110`, `dma_faults 0`,
  `pwm_errors 0`.
- (4) `pi-test.py`: pass (version, manifest match, ps 53, ps-tty, sdcard, starts 0 of 300).

Stable `v0.4.0-dev-643-g58df647b+x9f76` restored, devsh answers. Outputs in `~/pitest-tools/t284/`.

## #280 #250 listening test of the headphone jack, with a person (2026-11-07, Pi test agent)

Network image `default-pwm-audio-v0.4.0-dev-730-gaa76c1f9.img` (same as #284, default patches). Power on
20:28:22 UTC, login and devsh at 20:28:52. The user with headphones in the 3.5 mm jack typed the steps.
Results from the user, through the coordinator.

**Result: pass.** Steps (a)–(e) all ran fine as described (CPU tone, tone through audiod, 440 Hz in the
**left** ear, sweep, volume 100).
- Hiss: very faint hiss while nothing plays.
- Crackle: a little, which the user puts down to the cheap headphones.
- Boot click: not checked (headphones were plugged in after the boot).
- Extra: the user played their own WAV (ffmpeg to s16le, 44.1 kHz, stereo, sent with `nc` from homenodellm
  to `/tmp`, then `cat … > /scheme/audio`). It played, somewhat overdriven, maybe from the file itself.
- Side notes: the image has no curl or wget; Redox `nc` writes `Remote host: …` (34 bytes) to stdout and does
  not exit at EOF; Mac → Pi `nc` was blocked (probably the macOS firewall).
- Driver at the end (44 min uptime): `measured_rate_hz 44008`, `played_slots 228323`, `dma_faults 0`,
  `pwm_errors 0`, `underruns 1` (the one from the boot). The CPU tone line: `rate_hz=44107 fifo_empty=0`.
  No panic on serial.

Stable `v0.4.0-dev-643-g58df647b+x9f76` restored, devsh answers. Serial and stats in `~/pitest-tools/t280/`.

## #285 v0.4.0-rc1 automated, checklist tests 1–5, 11, 13, 14, 16, 18, 20 (2026-11-07, Pi test agent)

Network image `~/redox-rpi-rel/keep/v0.4.0-rc1-netboot.img` (`v0.4.0-dev-738-g4804fe63+x12cb`, `--release
v0.4.0-rc1 --experiment timesync`). Real Pi 3B+, power cycle 22:15:46 UTC, devsh after 30 s. Main run
22:15–22:30, extra boot 22:30–22:37 for the download and two re-checks. [#253](https://github.com/redox-rpi/redox-rpi/issues/253)

**Result: pass, with one known weak spot: serial input 19 of 20 (#28).**
- **(1–5) pi-test** `--starts 1000 --manifest …rc1-netboot.manifest.toml`: `PASS version: v0.4.0-rc1
  (v0.4.0-dev-738-g4804fe63+x12cb)`, manifest match, ps 61, ps-tty, sdcard, **starts 0 of 1000 failed**.
  Banner `redox-rpi v0.4.0-rc1 (v0.4.0-dev-738-g4804fe63+x12cb)` on every boot.
- **Boot log** (7 boots): `rpi-firmwared: ARM clock 600 -> 1400 MHz` each time, `throttled 0x00000000`, no
  throttle warning, no `pcid` or `rtcd` line, `audiod` only the `uring … not supported` note,
  `timesync: clock set … from pool.ntp.org` each boot. `date -u` on the Pi 22:16:37, host 22:16:39.
- **(13) Governor:** `mode ondemand`. During the 1000 starts every 5 s sample was `1400000000` (22 of 22).
  Idle (extra boot): over 20 s, `time_min_ms` +18.2 s and `time_max_ms` +2.6 s, so mostly 600 MHz. Reading
  `arm_clock` over devsh always shows 1400 MHz, because the devsh call itself is load.
- **Reboots:** `reboot` 5 times over devsh, `redox login:` and devsh back each time, about 20 s.
- **(11) Serial input:** `serial-input-test.sh 20` logged in as root on the console: **19 of 20** (S15 lost
  characters); again on the extra boot: 19 of 20 (S19). Known (#28): v0.3.0-rc3 had 2 lost in one run and 0 in
  the next. While the test runs, the console's `getty` (PID 28) takes about 72 s of CPU (0.56 s before the
  test, 72.75 s after it).
- **(14) GPIO/I2C/SPI:** `ls /scheme/gpio` 0–53 and `pins`; `nowire-check.py`: **21 of 21** (I2C detect
  finishes, missing address errors, SPI MISO pull reads ff/00, the 4096-byte transfer finishes).
- **(16) Audio:** `measured_rate_hz 43930`, `dma_faults 0`, `pwm_errors 0`, `underruns 1`; `tone check` both
  `(good)`; `time tone -s 3 > /scheme/audio`: `real 2.959 s`, rc 0; `audiod` still running.
- **(18) RedoxFS:** `dd … bs=4096 count=256`: `real 0.19 s` (26 MB/s). `sd-stress.py --big-mib 32 --small 100
  --cycles 1`: **pass**, 0 missing and 0 changed after writing and after the restart.
- **(20) Pipe and download:** `dd if=/dev/zero bs=65536 count=256 | cat > /scheme/data/p`: 16777216 bytes.
  The image has no `netget`, curl or wget, so the 100 MiB download used `nc` with an HTTP GET (extra boot):
  `printf 'GET /t285-100m.bin HTTP/1.0\r\n\r\n' | nc 192.168.178.120:8080 > /scheme/data/dl.bin`. Size by
  time: 22.3 MB after 4 s, 44.8 after 9 s, 68.7 after 14 s, 88.0 after 18 s, complete after 23 s
  (about 4.5 MB/s, no stall). SHA-256 of the 100 MiB payload matches the file on homenodellm. (Redox `nc`
  needs `host:port` and does not exit at EOF.)
- No panic, no `UNHANDLED EXCEPTION` on serial.

**Also seen (not rc1):** the first try staged nothing (my clone has no U-Boot `mkimage`), so the stable image
`v0.4.0-dev-643-g58df647b+x9f76` ran pi-test. About a minute into its 1000 starts (21:41:40) the network
went silent: no ARP answer, devsh gone, while netstack, dwc2d and devshd all kept running; nothing on serial.
A reset over the serial console brought it back. This is like #92/#217 (rare network loss).

Stable `v0.4.0-dev-643-g58df647b+x9f76` restored, devsh answers. Outputs in `~/pitest-tools/t285/` (try 1 in
`try1/`, the extra boot in `extra/`).

## #287 #33 optional desktop started from the console (v0.4.0-rc2 candidate) (2026-11-07, Pi test agent)

Branch `desktop-console` 5697fc1d. A: `netboot-desk-v0.4.0-dev-756-g5697fc1d+xce4c-dirty.0fae5a5cea1a.img`;
B: `netboot-desk-pb-v0.4.0-dev-756-g5697fc1d+x0f22-dirty.f717e04d32c7.img`. Real Pi 3B+, HDMI to the NanoKVM
(`rpi-fbd: EDID 256 bytes, monitor NanoKVM`, `boot framebuffer 1824x984`). 23:02–23:17 UTC, 3 boots of B.

**Result: A passes; B is open: the login through `pointer-bench` keys never finishes, and no screenshot
was possible.**
- **A1:** boots to the text console, `redox login:` on serial, no Orbital in `ps` (`grep -c orbital` 0).
  `experiments: cosmic-apps-source desktop orbital orbital-apps rpi-fb rtl8152 timesync`.
- **A2:** `pi-test.py`: pass (version, manifest match, ps 57, ps-tty, sdcard, starts 0 of 300).
- **A3:** `desktop; echo rc=$?` returns in 1.8 s: `desktop: starting Orbital, its log goes to /tmp/desktop.log`,
  `desktop: Super+F2 goes back to the console, Super+F3 to the desktop`, `rc=0`. `/usr/bin/orbital` runs
  (and `/usr/bin/orblogin`). `/tmp/desktop.log` stayed empty (polled for 60 s). `free`: 146500 KB used
  before, 226732 KB after (+80 MB).
- **Screenshot:** not possible. The image has no `vc4shot` (it is for `vc4d`, and this build uses `rpi-fbd`),
  and the NanoKVM was not used. So the time from `desktop` to the login screen is not measured; `orblogin`
  ran within 30 s.
- **B:** Orbital does not run at boot (`ps` before `desktop`: no orbital). `desktop` rc 0, Orbital PID 61 and
  `orblogin` PID 65. Then `pointer-bench --type 'user\n'` and `--type 'password\n'` (as the row says): both
  `typed … keys`, but 30 s later `orblogin` is still there and no session started (no launcher or other user
  processes). Same with an empty password (`user\n`, then `\n`, as the README says). `pointer-bench --keys
  super+f2` runs, and the second `desktop` prints only the switch line, rc 0. `ps` after all of it: exactly one
  `/usr/bin/orbital`. Whether the screen switched cannot be seen without a screenshot.
- No panic and no `UNHANDLED EXCEPTION` on serial; `/tmp/desktop.log` empty in all runs.

So the start from the console works and does not start a second Orbital. The login with injected keys does
not get through `orblogin`: either the keys from `pointer-bench` do not reach Orbital on VT 3, or the login
window needs something else. A person (part C) or a screenshot path for `rpi-fbd` would tell. Stable
`v0.4.0-dev-643-g58df647b+x9f76` restored, devsh answers. Outputs in `~/pitest-tools/t287/` (`b2/`, `b3/`
the B reruns with full `ps`).

## #288 #33 #287 part B again, with the right login keys (2026-11-07, Pi test agent)

Image `netboot-desk-pb-v0.4.0-dev-756-g5697fc1d+x0f22-dirty.f717e04d32c7.img` (as #287 B). Real Pi 3B+, fresh
boot (reset from stable) 23:20 UTC, then over devsh. NanoKVM not used.

**Result: pass.**
- Before `desktop`: no orbital in `ps`; `free` 146624 KB used.
- `desktop; echo rc=$?`: `rc=0` in 0.6 s. 5 s later `pointer-bench --type '\n'` (`typed 1 keys`).
- 10–20 s later `ps`: `/usr/bin/orbital` (PID 63), `/usr/bin/orblogin` (68, still there), and
  **`/usr/bin/launcher` as uid 1000** (PID 75): the empty-password login went through. `free` 186332 KB used.
  (The row's `grep -e … -e …` gives `Invalid parameters '-e'` on Redox grep, so this is read from the full `ps`.)
- `pointer-bench --keys super+f2`, then `desktop; echo rc=$?`: `rc=0`; `ps` after it: exactly one
  `/usr/bin/orbital`, launcher still running. Whether the screen switched is not visible without a screenshot.
- `/tmp/desktop.log` empty. No panic, no `UNHANDLED EXCEPTION` on serial.

Stable `v0.4.0-dev-643-g58df647b+x9f76` restored, devsh answers. Outputs in `~/pitest-tools/t288/`.

## 2026-11-08: `v0.4.0-dev-748-g36bd5f6a+x9f76`

Built 2026-11-08T03:30:26Z, `libc.so` `a0b375a7d2180b3f…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Nightly stable image test (scripts/update-stable.sh).

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.4.0-dev-748-g36bd5f6a+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.4.0-dev-748-g36bd5f6a+x9f76.toml |
| ps | pass | 51 processes (piped) |
| ps-tty | pass | 51 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## #289 PR #226 opt-a53 split: opt-level 3 with LTO against target-cpu=cortex-a53 (2026-11-08, Pi test agent)

Follow-up to #260 (the user: "Erst Effekte trennen"). Real Pi 3B+, network boot, 8 boots in
the order A, B2, C, D2, D2, C, B2, A (05:39–06:39 UTC). On each boot: boot-timeline.py,
`scripts/pi-test.py` and `patches/experiments/opt-a53/bench.py --rounds 3`, run read-only
from `~/redox-rpi-opt` at d6fb1cf8 with `PYTHONDONTWRITEBYTECODE=1`.

| | Build | What it is |
|---|---|---|
| A | `v0.4.0-dev-565-gc70f394e+x9f76` | stable, as in #260 |
| B2 | `v0.4.0-dev-575-gd6fb1cf8+x9b71` | all: opt-level 3, fat LTO, 1 codegen unit, target-cpu cortex-a53 (`-aes,-sha2`) |
| C | `v0.4.0-dev-574-gf58b489f+xb439` | opt-speed: opt-level 3, fat LTO, 1 codegen unit |
| D2 | `v0.4.0-dev-575-gd6fb1cf8+xf367` | cpu-a53: target-cpu cortex-a53 (`-aes,-sha2`) only |

**Result: pass. The network gain comes from opt-level 3 with LTO (C); target-cpu alone (D2) gives nothing.**
All 8 boots reached `redox login:`. pi-test passed all 6 checks on every boot (0 of 300 starts
failed). No panic. Clocks were the same before and after every bench (arm 1400 MHz, core 250,
sdram 400), so there was no throttling.

Median of 6 rounds per image (2 boots × 3 rounds), with the range, in seconds; boot time is 2 boots:

| Workload | A (stable) | B2 (all) | C (opt-speed) | D2 (cpu-a53) | B2 vs A | C vs A | D2 vs A |
|---|---|---|---|---|---|---|---|
| spawn, 500 × `ls /usr` | 77.68 (77.42–77.92) | 75.47 (75.17–75.94) | 77.09 (76.98–77.24) | 79.58 (79.38–79.80) | **−2.8 %** | −0.8 % | **+2.4 % (slower)** |
| net-rx, 32 MiB to the Pi | 3.07 (2.75–3.13) | 2.69 (2.52–2.87) | 2.84 (2.79–3.08) | 3.04 (2.75–3.37) | **−12.3 %** | **−7.4 %** | −0.7 % |
| net-tx, 32 MiB from the Pi | 4.91 (4.68–4.96) | 4.37 (4.30–4.44) | 4.35 (4.32–4.45) | 5.00 (4.92–5.01) | **−11.2 %** | **−11.4 %** | +1.7 % |
| sd-write, 50 MiB | 6.76 (6.32–8.43) | 6.71 (5.74–8.95) | 6.49 (5.75–6.97) | 6.83 (5.78–7.34) | same | same | same (card-bound, noisy) |
| sd-read, 50 MiB | 0.55 (0.55–0.55) | 0.54 (0.53–0.54) | 0.55 (0.54–0.55) | 0.55 (0.55–0.55) | −2 % | same | same |
| boot to `redox login:` | 35.14, 30.26 | 30.12, 30.61 | 30.25, 35.15 | 30.67, 30.35 | | | no difference (the known 35 s outliers) |

- **net-tx:** C alone gives all of B2's gain. **net-rx:** C gives about 60 % of it; the rest
  shows only when both are on.
- **spawn:** target-cpu alone makes it 2.4 % slower, opt-speed alone 0.8 % faster, and both
  together 2.8 % faster. So the two interact; target-cpu without LTO does not pay.
- **net-tx retries:** bench.py's "nc did not connect" retry was needed 13 times over all boots
  (on A, B2 and C; never on D2). Every round finished. One retry left a harmless
  `TimeoutError` traceback from the accept thread (boot 1, A).
- Raw data: homenodellm `~/pitest-tools/t289/` (`opt-a53-split.jsonl`, bench, pi-test, boot
  and serial logs per boot).

Stable `v0.4.0-dev-748-g36bd5f6a+x9f76` restored and checked over devsh.

## #290 #33 / PR #255 optional desktop, clean build with the beta warning and the login hint (2026-11-08, Pi test agent)

Real Pi 3B+, network boot, serial console and devsh, no person, 14:51–15:03 UTC.

| | Build | What it is |
|---|---|---|
| A | `v0.4.0-dev-785-gf0a43752+xce4c` | branch `desktop-console` f0a43752, clean build: the exact build for PR #255 |
| B | `v0.4.0-dev-784-gd41026d9+x0f22-dirty.82ef20819842` | the same branch plus `pointer-bench`, exploratory (dirty build) |

**Result: fail on A5 only. Everything else passes.** Over devsh, `desktop` without `--yes`
asks the question and waits instead of refusing. It never started Orbital. This happened in 3
of 3 tries on 2 boots, so it is not flaky. A1–A4, A6, A7 and all of B pass. No panic.

| Step | Result | Detail |
|---|---|---|
| A1 boot | pass | text console; 0 × `/usr/bin/orbital` in `ps`; 147 MB used |
| A2 serial root login | pass | after the login: "Type `desktop` to start the graphical desktop (beta); as user: `sudo desktop`." |
| A3 pi-test | pass | 6 of 6, manifest match, 0 of 300 starts failed |
| A4 serial `desktop`, answer `n` | pass | beta warning, `Start the desktop? [y/N]`, `n` → `desktop: not started`, 0 orbital. The question line showed `answer=…` on the console as the key was typed (cosmetic) |
| A5 devsh `desktop; echo rc=$?` | **fail** | It printed the warning and `Start the desktop? [y/N]`, then waited. It never printed "no terminal to ask on". devsh got nothing more for 30 s (first try) and 120 s (2 reruns on a fresh boot), and the session was dropped. Then 2 × `answer=`, no `rc=`, 0 orbital. With `desktop < /dev/null` it prints `desktop: no terminal to ask on, not starting (desktop --yes starts it)`, rc=1. So the terminal check counts devsh's input as a terminal |
| A6 devsh `desktop --yes` | pass | rc=0; one `/usr/bin/orbital` and one `/usr/bin/orblogin`; `/tmp/desktop.log` empty; used 179 → 220 MB |
| A7 serial `desktop` again | pass | no question, only `desktop: Super+F2 goes back to the console, Super+F3 to the desktop`, rc=0; still one orbital |
| B login | pass | `desktop --yes` rc 0, `pointer-bench --type '\n'`, 15 s later `/usr/bin/launcher` runs as uid 1000 |
| B switches | pass | `pointer-bench --keys super+f2`, then `desktop` rc 0 with no question; exactly one orbital; `/tmp/desktop.log`: `failed to load icon None`; 186 MB used |

- NanoKVM screenshots were not taken (optional).
- Raw data: homenodellm `~/pitest-tools/t290/` (rerun in `t290/rerun/`).

Stable `v0.4.0-dev-748-g36bd5f6a+x9f76` restored and checked over devsh.

## #292 v0.4.0-rc2 automated, checklist tests 1–5, 11, 13, 14, 16, 18, 20 and 22 A (2026-11-08, Pi test agent)

Build `v0.4.0-dev-809-g5723a434+x4aaf` (`v0.4.0-rc2`, netboot image
`~/redox-rpi-rel/keep/v0.4.0-rc2-netboot.img`). Real Pi 3B+, network boot, serial console,
no person, 19:31–19:58 UTC. [#253](https://github.com/redox-rpi/redox-rpi/issues/253)

**Result: pass.** Every automated checklist step passes. Serial input is 19 of 20, inside the
row's limit (#28). No panic, no throttling, and no boot without network.

| Test | Result | Detail |
|---|---|---|
| (1) boot, banner | pass | first boot after a power cycle; `redox-rpi v0.4.0-rc2` banner on all 7 boots; experiments `cosmic-apps-source desktop orbital orbital-apps rpi-fb timesync` |
| (2) pi-test | pass | 6 of 6; version `v0.4.0-rc2`; manifest: build ID and `libc.so` hash match `v0.4.0-rc2-netboot.manifest.toml`; **0 of 1000 starts failed** |
| (3) boot log | pass | `rpi-firmwared: ARM clock 600 -> 1400 MHz` on every boot; no throttle warning; throttled `0x00000000`; no `pcid` or `rtcd` line; `bcm2835-pwmaudiod: … range 2834 (44107 Hz)`; `audiod` only prints its known "uring … falling back to standard IO" note, no error |
| (4) time | pass | `timesync: clock set … from pool.ntp.org` on every boot, and `from the saved time` before it after each reboot |
| (5) governor | pass | `mode ondemand`, min 600 MHz, max 1400 MHz, 10 switches. Over devsh `arm_clock` always reads 1400 MHz (23 samples), because the devsh call itself is load, as in rc1 |
| (13) reboot | pass | `reboot` over devsh 5 times; each back at `redox login:` and devsh within 23 s |
| (11) serial input | pass (19 of 20) | `serial-input-test.sh 20`: 19 of 20 lines complete, S07 lost characters (#28; the limit is 19) |
| (14) GPIO, I2C, SPI without wiring | pass | `ls /scheme/gpio`; `nowire-check.py` 21 of 21 |
| (16) audio | pass | stats `mode pwm-dma`, `sample_rate 44107`, `measured_rate_hz 43972`, `dma_faults 0`, `pwm_errors 0`, 1 underrun since boot; `tone check` both good; `tone -s 3 > /scheme/audio` took 2.96 s, rc 0 |
| (18) RedoxFS | pass | `dd bs=4096 count=256` in 0.19 s (26 MB/s); `sd-stress.py --big-mib 32 --small 100 --cycles 1`: 0 missing, 0 changed, also after the restart |
| (20) pipe and download | pass | 16 MiB pipe complete (16777216 bytes). 100 MiB over HTTP with `nc` (still no `netget`, curl or wget in the image) in about 22–28 s (about 4 MB/s), no stall; the SHA-256 of the payload matches the file on homenodellm |
| (22 A) desktop | pass | after the serial root login: "Type `desktop` to start the graphical desktop (beta); as user: `sudo desktop`."; `desktop --yes` rc 0; 15 s later `ps` shows `/usr/bin/orbital`, `/usr/bin/orblogin` and `rpi-fbd`; `/tmp/desktop.log` empty; used 304 → 360 MB; no panic in the serial log |

- NanoKVM screenshot: not taken (optional).
- Raw data: homenodellm `~/pitest-tools/t292/`. The test file was deleted afterwards.

Stable `v0.4.0-dev-748-g36bd5f6a+x9f76` restored and checked over devsh.

## #286 part 1: v0.4.0-rc2 on the Zero 2 W, from the SD card, by the maintainer (2026-11-08, recorded by the Pi test agent)

Build `v0.4.0-dev-809-g5723a434+x4aaf` (`redox-rpi-v0.4.0-rc2.img.xz`), SD card in the Zero 2 W,
person test (level 5). The maintainer reported this through the coordinator; their own words:

> "hab gerade das RC2 auf dem zero getestet, sieht gut aus. Text kann man im text editor unter
> cosmic nicht speicher. Aber sonst ging alles, netz, kb nach booten, mouse nach booten."

English: "just tested RC2 on the Zero, looks good. You can't save text in the text editor under
COSMIC. But everything else worked: network, keyboard after booting, mouse after booting."

| Checklist | Result | Detail |
|---|---|---|
| (9, 19) Zero 2 W network | pass | "netz" worked. Asked again, the maintainer said: "ja läuft noch IP ist 192.168.178.118. Ich konnte vom pi einen ping an den server geben." (yes, still running, the IP is 192.168.178.118, and a ping from the Pi to the server worked). So DHCP gave it an address. At 20:10 UTC the build host saw 192.168.178.118 with MAC `00:e0:4c:36:14:8e` (Realtek, the RTL8153 hub adapter). Asked about the time: "uhrzeit stimmt, hab mit date abgefragt und auch gesehen das im log die timesync lief." (the time is right, checked with `date`, and timesync ran in the log). So NTP works too |
| (23) REQ-REL-1, no devshd | pass (Zero 2 W) | from the build host at 20:10 UTC: `nc -z -w 3 192.168.178.118 2323` fails (rc 1). The `ps` check on the console was not done |
| keyboard plugged in after boot | pass (by report) | "kb nach booten" |
| mouse plugged in after boot | pass (by report) | "mouse nach booten" |
| (22) desktop, COSMIC Text Editor save | fail, known | saving text in the editor does not work; already known. v0.4.1 replaces the editor with the game ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)) |

**Result: pass for the Zero 2 W part (9, 19, keyboard and mouse hotplug; devsh port closed).** The editor save
failure is the known desktop gap. The Pi 3B+ part of #286 is still open (Monday 11-09).

## #286 part 2 and whole row: v0.4.0-rc2 on the Pi 3B+, from the SD card, by the maintainer (2026-11-08, recorded by the Pi test agent)

Build `v0.4.0-dev-809-g5723a434+x4aaf` (`redox-rpi-v0.4.0-rc2.img.xz`), SD card in the Pi 3B+
with HDMI, USB keyboard and mouse, headphones, cable plugged in later. Person test (level 5),
20:00–21:00 UTC. The maintainer's results came through the coordinator and are quoted as they
said them. The serial log (`~/redox-rpi/logs/serial.log`, read only) confirms what it can show.
Part 1 (Zero 2 W) is the entry above.
[#253](https://github.com/redox-rpi/redox-rpi/issues/253)

**Result of #286: pass.** Every checklist item of the visit passes on both boards. The known
gaps (editor save, no MAC in `ifconfig`) and the usability notes do not block v0.4.0.

| Checklist | Result | Detail |
|---|---|---|
| (6) boot menu, card boot | pass | "ja das bootmenü startet die karte : siehe serial." Serial: U-Boot autoboot boots `mmc 2`; the Redox bootloader shows "best resolution: 1920x1200 … Continuing with the selected mode in 5 seconds" and boots on with no key; RedoxFS 990 MiB; banner `redox-rpi v0.4.0-rc2 (v0.4.0-dev-809-g5723a434+x4aaf) (sdcard, 2026-11-08T18:56:24Z)`. In one boot a key stopped U-Boot, and `reboot` and `help` were typed there; the next boot went normally. The `panic` lines in that part of the log are U-Boot's help text |
| (7) login on HDMI, time | pass | "date: OK"; timesync set the time from the saved time and then from `pool.ntp.org` |
| (8) network after plugging the cable | pass | "net kommt nach dem anstecken". Serial: 3 × `dhcpd: eth0 … no offer` without the cable, then `dwc2d: link up on the LAN7800`, `DHCP: eth0 got 192.168.178.148/24`, `timesync: clock set to 2026-11-08 20:37:54 UTC from pool.ntp.org` |
| (8) keyboard out and in | pass | "ja geht, seht auch im log dass er sie wieder gefunden hat, bei zweite mal auch auf anderen port". Serial: `hub 1 port 3: device at address 4 removed`, `keyboard removed`, `hub 1 port 3: device connected`, `keyboard at address 5`; again removed, then `hub 1 port 2: device connected`, `keyboard at address 5`. The keyboard (04d9:0006) is behind a small USB hub (05e3:0610) |
| (21) mouse | pass | `dwc2d: mouse at address 3: relative, 3 button(s), wheel` (address 6 in an earlier boot); no panic in any rc2 boot |
| (17) audio | pass | "ton geht, beim ausschalten gibt es einen kleinen knacks, und beim neustart den gleich wie beim ausschlaten. aber nichts schlimmes." A small click at power-off and the same at reboot; nothing was said about a click at boot or about the left-only tone separately |
| (22) desktop | pass, with notes | The desktop worked, and so did Super+F2 back to the console. "it takes about 7 sec to start the editor" (the beta warning says about 40 s). Usability: "die Anleitung das man mit 'super+F2' auf die console zurück schalten kann man erst lesen wenn man auf die console zurück schaltet und das weiß. das sollte auch in dem text stehen. braucht auch keine ja/nein frage sein, nur ein info. in der sollte auch stehen das save in editor nicht geht." → Graphics, v0.4.1. Saving in the editor fails, as on the Zero 2 W (known, #33) |
| `ifconfig` | known gap | "same probelm as in the zero: no mac in ifconfig": no MAC shown, on the 3B+ and the Zero 2 W (Network, v0.4.1) |
| (23) REQ-REL-1, no devshd | pass | photo of `ps` on HDMI ("ich sehe es nicht"): the list ends with `/usr/bin/ps`, no `devshd`. From the build host at 20:50:51 UTC: `nc -z -w 3 192.168.178.148 2323` fails (rc 1); .148 is the Pi (`b8:27:eb:08:e0:aa`), and the last serial banner is rc2 `sdcard` |

**Summary of #286 (both parts):**
- **Zero 2 W (9, 19):** DHCP 192.168.178.118 on the RTL8153 hub, ping, NTP, keyboard and mouse hotplug, devsh port closed.
- **Pi 3B+ (6, 7, 8, 17, 21, 22, 23):** all pass.
- **For v0.4.1:** editor save (#33), MAC in `ifconfig` (Network), desktop info text and editor start time in the warning (Graphics).

The netboot card goes back in when the maintainer is done. Then the Pi test agent checks that
stable `v0.4.0-dev-748-g36bd5f6a+x9f76` boots.

## 2026-11-09: `v0.4.0-dev-819-gde8aa6e5+x9f76`

Built 2026-11-09T03:30:26Z, `libc.so` `a0b375a7d2180b3f…`, real Pi 3B+ over netboot. Experiments: `rtl8152`, `timesync`. Nightly stable image test (scripts/update-stable.sh).

| Check | Result | Detail |
|---|---|---|
| version | pass | v0.4.0-dev-819-gde8aa6e5+x9f76 |
| manifest | pass | build ID and libc.so hash match netboot-v0.4.0-dev-819-gde8aa6e5+x9f76.toml |
| ps | pass | 51 processes (piped) |
| ps-tty | pass | 51 lines straight to the terminal, rc=0 |
| sdcard | pass | write, read back, delete on /scheme/data |
| starts | pass | 0 of 300 failed |

## v0.4.0 release: one boot on the Pi (2026-11-09, Pi test agent)

Build `v0.4.0-dev-809-g5723a434+x4aaf`, `RELEASE="v0.4.0"` (netboot image
`~/redox-rpi-rel/keep/v0.4.0-netboot.img`, manifest `v0.4.0-netboot.manifest.toml`; same commit,
options and build ID as the SD card image `redox-rpi-v0.4.0-pi3bplus.img.xz` and as rc2, #292).
Level 4: real Pi 3B+, network boot, serial console, no person, 06:45–06:48 UTC. Release process
step "one boot on the Pi", before the tag. [#253](https://github.com/redox-rpi/redox-rpi/issues/253)

**Result: pass.** The release image boots and shows the v0.4.0 banner, login works, no panic, and
the pi-test standard checks pass.

| Check | Result | Detail |
|---|---|---|
| boot | pass | first boot after a power cycle; devsh answered 35 s later |
| banner | pass | `redox-rpi v0.4.0 (v0.4.0-dev-809-g5723a434+x4aaf) (netboot, 2026-11-08T21:01:23Z)`, not `-rc2`; `/etc/redox-rpi-release`: `RELEASE="v0.4.0"`, `COMMIT="5723a434"`, experiments `cosmic-apps-source desktop orbital orbital-apps rpi-fb timesync` |
| login | pass | `redox login:` on serial; root login on the serial console, `cat /etc/redox-rpi-release` answered |
| no panic | pass | no `panic`, exception or `UNKILLABLE` in the serial log of the boot |
| pi-test | pass | 6 of 6: version, manifest (build ID and `libc.so` hash match), ps (57), ps-tty, sdcard, 0 of 300 starts failed |
| other | observe | motd has the `desktop` line; `throttled` 0x0; free 131 of 736 MB used; df 182 of 227 MB used (80 %) |

Unexpected events: none. Restore: stable `v0.4.0-dev-819-gde8aa6e5+x9f76` (promoted by the nightly
at 03:35) staged and answered on devsh. Data: homenodellm `~/pitest-tools/trel040/`.

## #293 TinyCC in the v0.4.1 release image (2026-11-09, Pi test agent)

Build `v0.4.0-dev-819-gab8fedb5+xc80e` (branch `tcc-release`, release options plus
`--experiment tcc`; netboot image `~/redox-rpi-tcc/keep/rel-netboot-v0.4.0-dev-819-gab8fedb5+xc80e.img`,
manifest next to it), read from the Pi. Level 4: real Pi 3B+, network boot, serial console, no person,
06:49–06:52 UTC. [#293](https://github.com/redox-rpi/redox-rpi/issues/293)

**Result: pass.** Every step meets its criteria. The static Hello World fails, as the row expects
(reloc 562). No panic.

| Step | Result | Detail |
|---|---|---|
| (1) `tcc -v` | pass | `tcc version 0.9.28rc 2026-11-07 HEAD@70adbb12* (AArch64 Linux)` |
| (2) `tcc -run hello.c` | pass | `Hello from tcc on Redox (/usr/share/tcc-tests/hello.c)`, rc 0; `timeit` 120.3 ms (#278: 130 ms) |
| (3) compile and run | pass | `tcc -o hello` 98.9 ms (#278: 100 ms); `./hello` prints `(./hello)`, rc 0 |
| (4) script `./hallo.c a b` | pass | argv 0–2, `Hallo, stdin 6 bytes`, rc 42 (twice) |
| (5) `run.ion` | pass | to `TCC-TEST done` in about 45 s; static hello fails as known (40.1 s, reloc 562–569); shebang 119 ms, rc 42; bench by tcc 1.40 s, by gcc -O2 0.98 s (same results); tcc compiles tcc.c (77k lines) in 0.78 s, `tcc2` builds hello2 |
| (6) still the release image | pass | motd has the `desktop` line; `EXPERIMENTS="cosmic-apps-source desktop orbital orbital-apps rpi-fb tcc timesync"`; df `/ 227328 184932 42396 81%` (QEMU: 185 of 227 MB); free 131 of 736 MB used after boot, 163 MB after the tcc run; throttled 0x0 |
| (7) pi-test | pass | 6 of 6: version, manifest (build ID and `libc.so` hash match), ps, ps-tty, sdcard, 0 of 300 starts failed |

Compared with the v0.4.0 release image (same day, one boot): the root filesystem uses 2.6 MB more
(184932 against 182312 KB), 81 % against 80 %.

Unexpected events: none. Restore: row 291 ran next; the stable image was put back after it.
Data: homenodellm `~/pitest-tools/t293/`.

## #291 desktop for v0.4.1: Sopwith, `desktop` without the question, info window (2026-11-09, Pi test agent)

A: build `v0.4.0-dev-796-g20ce9139+xce4c` (branch `desktop-game` 20ce9139, clean), B:
`v0.4.0-dev-796-g20ce9139+x0f22-dirty.82ef20819842` (same plus `pointer-bench`, exploratory, dirty
build), both read from the Pi. Level 4: real Pi 3B+, network boot, serial console, no person, no
HDMI screen attached (the firmware gave the no-screen mode 656x416; `rpi-fbd: 656x416 stride 656`),
06:52–07:02 UTC. [#33](https://github.com/redox-rpi/redox-rpi/issues/33)

**Result: fail.** Sopwith crashes right after it opens its window: an unhandled write fault in
`/usr/bin/sopwith` at address `0x854000`, the same on 3 of 3 fresh boots. Everything else passes.

| Step | Result | Detail |
|---|---|---|
| A1 `desktop` | pass | prints `Redox desktop (beta)`, `Super+F2   back to the text console`, the limits, and starts at once, rc 0, no question; one `/usr/bin/orbital` |
| A2 `sopwith` | **fail** | `sop.log`: `Initializing orbital window at 8,8 with size 640x400 with flags 9` (screen 656x416, not the 1920x1080 the row assumed); then serial: `FATAL: Not an SVC induced synchronous exception`, `ESR_EL1 0x92000047` (data abort, write), `FAR_EL1 0x854000`, `ELR_EL1 0x6ED980`, `X1 0xA00` (2560 = 640 × 4 bytes, one row of the window), `X2 0xFF000000` (a black pixel), `UNHANDLED EXCEPTION … /usr/bin/sopwith` |
| A3 CPU | fail | sopwith stays in `ps` with CPU time 00:00:00.00 in both samples (it is stopped after the fault); temperature 48.3 °C; free 180 of 736 MB used |
| A4 app lists | pass | `sopwith.desktop`, `70_sopwith`; no `com.system76.CosmicEdit.desktop` |
| A5 pi-test | pass | 6 of 6, 0 of 300 starts failed |
| B login, info window | pass | `desktop` as in A1; `pointer-bench --type '\n'` typed 1 key; 20 s later `ps` shows `/usr/bin/launcher`, `/usr/bin/orbterm` and `/usr/bin/desktop-welcome` as uid 1000; free 198 of 736 MB used. No screenshot (no screen attached) |

Rerun (rerun rule): A2 on two more fresh boots of image A: the same fault at `FAR 0x854000`,
`ELR 0x6ED980` both times, 3 of 3 in all. A guess for graphics: the window at 8,8 with 640x400 plus
its title bar does not fit the 656x416 screen, Orbital gives a smaller window, and SDL's Orbital
driver keeps writing 640x400 pixels. In QEMU (640x480 screen) Sopwith chose 320x200 and passed.

Unexpected events: none besides the crash. Restore: stable `v0.4.0-dev-819-gde8aa6e5+x9f76`, answered.
Data: homenodellm `~/pitest-tools/t291/` (rerun in `t291/rerun/`).
