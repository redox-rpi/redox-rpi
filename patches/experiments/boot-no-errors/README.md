# No avoidable error lines in a normal boot (experiment `boot-no-errors`)

Issue [#556](https://github.com/redox-rpi/redox-rpi/issues/556), from the
inventory of Tests 504 (3B+) and 505 (Zero 2 W) in rpi-open-firmware#59.
Rule: a normal boot prints no avoidable `ERROR`, `WARN` or `failed` line;
when a line has a known harmless cause, fix the cause or design around it.

| # | Line in the boot log | Cause | Patch |
|---|---|---|---|
| 1 | `[ERROR redoxfs] failed to open filesystem /scheme/disk.mmc/0: …` and `…/0p0` (live-disk boot) | With `--uuid`, redoxfs tries every disk and logs each one that has no matching RedoxFS. That is normal while searching. | redoxfs `0001`: no error line during the UUID search; `not able to mount uuid` still reports when no disk has it |
| 2 | `randd: Seeding failed, no entropy source.  Random numbers on this platform are NOT SECURE` | The Cortex-A53 has no RNDR register, and randd knew no other source. | base `0002`: randd seeds from the SoC's hardware RNG (`brcm,bcm2835-rng`, from the devicetree) when RNDR gives nothing |
| 3 | `dwc2d: request [21, 0a, …] … failed: Stall` | HID SET_IDLE; the HID spec lets a device stall it, and the callers ignore the result. | base `0001`: a stalled SET_IDLE returns at once, with no retry and no line |
| 4 | `kernel::acpi:ERROR -- NO RSDP FOUND`, `kernel::scheme::acpi:WARN -- expected RXSDT_ENUM …` | The kernel looks for ACPI tables on a devicetree board. | kernel `0001`: no ACPI search when the bootloader gave no RSDP and there is a devicetree; an empty ACPI scheme is a debug line |
| 5 | `…rtc:WARN -- No PL031 RTC found`, `…irqchip:WARN -- no driver for interrupt controller "brcm,bcm2835-gpio"` | The Pi has no RTC; no kernel device uses GPIO interrupts. | kernel `0001`: both are `INFO` lines |
| 6 | About 200 `DEBUG` lines from the kernel | Upstream prints `debug!` on aarch64 and riscv64. They cost about 1.4 s at 115200 baud (rpi-open-firmware#44). | kernel `0001`: `debug!` writes to the kernel log (`/scheme/sys/log`) only; the new kernel feature `debug_serial` prints them as before |

Not changed: the 8BitDo report descriptor stall (`[81, 06, 00, 22, …]`,
Test 444) is a real failure of that gamepad, not a harmless line.

Limits:
- The hardware RNG is the Pi 1 to 3 and Zero 2 W one. The Pi 4 and Pi 400
  have a different block (`brcm,bcm2711-rng200`); randd still prints the
  old line there.
- `debug!` lines before the kernel log starts (early memory setup) are
  dropped.

## Test

1. QEMU `raspi3b`, network-boot image and direct boot: none of the lines
   above, `cat /scheme/sys/log` still has the DEBUG lines, and
   `/scheme/rand` gives data. QEMU emulates the BCM2835 RNG.
2. Pi 3B+, no person: boot log against stable, the same checks,
   `scripts/pi-test.py`.

## Results

See the PR and the pi-tests rows.
