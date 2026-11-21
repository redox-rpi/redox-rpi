# Experiment: quiet-boot, HDMI shows only the login (#518)

With `quiet` in the kernel command line (`cmdline.txt`), Redox draws no boot messages on the screen. HDMI shows the firmware's picture and then the login. The serial console and the log keep everything.

This experiment covers steps I2, I3 and I5 of the quiet-boot plan.

## How it works

| Part | What it does | Where |
|---|---|---|
| `rpi-boot-stub` | turns the word `quiet` in `/chosen/bootargs` into `QUIET=1` in the kernel environment; `QUIET=1` written directly also works | `tools/rpi-boot-stub` (default, harmless without the patches) |
| kernel | with `QUIET` set, `graphical_debug` does not set up the debug display: kernel lines and init's service output (`/scheme/debug`) go to serial and the log only | [`kernel/0001`](kernel/0001-graphical_debug-no-boot-messages-on-the-screen-with-.patch) |
| inputd | with `QUIET` set, opening the boot log (`fbbootlogd`) does not activate VT 1. The display driver leaves the framebuffer alone until the first normal console opens (the login on VT 2, or Orbital on the desktop image), which inputd activates as before | [`base/0001`](base/0001-inputd-with-QUIET-1-the-boot-log-does-not-take-the-s.patch), `10_inputd.service` inherits `QUIET` |

Without `QUIET` set, nothing changes.

## A boot that hangs

A quiet boot that stops before the login would keep the firmware's picture (or black) forever, and look like a dead Pi. So:

- **After 60 seconds without a console,** inputd shows the boot log on VT 1 by itself, and the screen shows where the boot stopped. If the boot goes on, the login still comes. `QUIET_TIMEOUT=<seconds>` in `cmdline.txt` changes the wait, `QUIET_TIMEOUT=0` turns it off.
- **A kernel panic** shows its message on the screen as in a normal boot, until the display driver takes over.
- **By hand:** Super+F1 shows the log at any time, or boot without `quiet`.

## Boot steps on our firmware's boot screen

With our firmware ([rpi-open-firmware](https://github.com/redox-rpi/rpi-open-firmware), tag `0x000f0002`, rpi-open-firmware#43), the boot screen stays on HDMI and shows Redox's steps under the progress bar ([`base/0002`](base/0002-rpi-firmwared-boot-steps-on-our-firmware-s-boot-scre.patch)):

| Step | Bar | When |
|---|---|---|
| drivers | 25 % | initfs drivers up (`45_bootstep-drivers`) |
| file system | 50 % | root file system mounted (`55_bootstep-rootfs`) |
| services | 75 % | `00_base.target` (`01_bootstep-base`) |
| network | 90 % | `10_net.target`, after the services step, only if it comes before the login (`15_bootstep-net`) |
| login (last) | 100 % | before the first console; `fbcond@` waits for it (`29_bootstep-login`) |

- **The interface:** `rpi-firmwared` sends the tag. A line `PCT [fail] [last] TEXT` written to `/scheme/rpi-firmware/bootstep` is one step, and `rpi-firmwared step …` does the same for init. It only sends with `QUIET` set.
- **Other firmware:** Broadcom's `start.elf`, QEMU and a firmware without HDMI do not draw the step. `rpi-firmwared` then logs one line and sends nothing more in this boot. The boot looks as without the steps.
- **A boot that hangs:** inputd's fallback sends a last step `boot stopped` (red) first, then shows the log.
- **Boards:** the 3B+ only, like `rpi-firmwared` today.

## The full log

`fbbootlogd` still collects the whole boot log on VT 1:
- **Super+F1** shows it.
- **Shift+PgUp/PgDn** scrolls it.
- **Super+F2** goes back to the login.

These are upstream Redox's keys, which the maintainer chose for step I5 (#517).

## The login screen (I5, #517)

- **The hint.** getty prints `/etc/issue` above the login prompt ([`base/0003`](base/0003-getty-print-etc-issue-above-the-login-prompt-on-disp.patch)). This experiment installs `/etc/issue` with the line `Super+F1: boot messages` (`config.toml`), so the login says where the log is. The file is part of the image, so the line shows with or without `quiet`. getty prints it on the display consoles only; the serial console has no Super key. Known: the line shows twice above the prompt (#599, cosmetic).
- **No black frame before the login (experiment `rpi-fb`).** With `QUIET` set, `rpi-fbd` does not clear a new framebuffer when the console sets its mode, and the first frame covers the whole screen. So the login replaces the firmware's picture in one go ([`rpi-fb/base/0074`](../rpi-fb/base/0074-rpi-fbd-under-QUIET-keep-the-firmware-s-picture-unti.patch)). `vesad` never clears at start, so images without `rpi-fb` need nothing.

## Limits

- **Direct boot only.** With U-Boot and the Redox bootloader, `quiet` does not reach the kernel, and U-Boot and the bootloader print on HDMI themselves.
- **Our firmware.** It writes `cmdline.txt` to `/chosen/bootargs` only after step I1. Broadcom's `start.elf` already does.
- **A panic after the display driver starts** shows on serial only, as in a normal boot.

## Test

QEMU direct boot:

```sh
scripts/build-image.sh --experiment quiet-boot
scripts/fw-direct-boot.sh --qemu --append quiet
```

Take screen dumps through a QEMU monitor during the boot. Results are on the PR.
