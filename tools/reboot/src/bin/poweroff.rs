//! poweroff: stop the system.
//!
//! Flushes the file systems and asks the kernel to shut down through
//! `/scheme/sys/kstop`. Needs root. The Raspberry Pi 3 has no real power
//! switch: the kernel tells the firmware to halt, and the board starts again
//! only when the power is cut and reconnected.

fn main() {
    reboot::kstop("poweroff", "shutdown");
}
