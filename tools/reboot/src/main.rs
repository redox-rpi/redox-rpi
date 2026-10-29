//! reboot: restart the system.
//!
//! Stops the programs started after boot, closes the file systems and asks
//! the kernel for a reset through `/scheme/sys/kstop` (see the library).
//! Needs root. On the Raspberry Pi 3 the kernel resets through the watchdog,
//! as the firmware has no PSCI.

fn main() {
    reboot::kstop("reboot", "reset");
}
