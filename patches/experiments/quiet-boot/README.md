# Experiment: quiet-boot, HDMI shows only the login (#518)

With `quiet` in the kernel command line (`cmdline.txt`), Redox draws no boot messages on the screen. HDMI shows the firmware's picture and then the login. The serial console and the log keep everything.

This experiment covers steps I2 and I3 of the quiet-boot plan.

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

## The full log

`fbbootlogd` still collects the whole boot log on VT 1:
- **Super+F1** shows it.
- **Shift+PgUp/PgDn** scrolls it.
- **Super+F2** goes back to the login.

This is upstream Redox, unchanged. Whether it is the key the maintainer wants is graphics' step I5.

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
