//! poweroff: stop the system.
//!
//! Stops the programs started after boot, closes the file systems and asks
//! the kernel to shut down through `/scheme/sys/kstop` (see the library).
//! Needs root. The Raspberry Pi 3 has no real power switch: the kernel tells
//! the firmware to halt, and the board starts again only when the power is
//! cut and reconnected.

fn main() {
    reboot::kstop("poweroff", "shutdown");
}
