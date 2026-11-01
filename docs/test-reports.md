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
