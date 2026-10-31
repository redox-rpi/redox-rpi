//! reboot: restart the system.
//!
//! Flushes the file systems and asks the kernel for a reset through
//! `/scheme/sys/kstop`. Needs root. On the Raspberry Pi 3 the kernel resets
//! through the watchdog, as the firmware has no PSCI.

use std::fs::OpenOptions;
use std::io::Write;

extern "C" {
    // In relibc, but not declared for Redox by the libc crate.
    fn sync();
}

fn main() {
    if matches!(std::env::args().nth(1).as_deref(), Some("--version" | "-V")) {
        println!("reboot {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    unsafe { sync() };
    let result = OpenOptions::new()
        .write(true)
        .open("/scheme/sys/kstop")
        .and_then(|mut kstop| kstop.write_all(b"reset"));
    // Only reached if the reset failed.
    match result {
        Ok(()) => eprintln!("reboot: the kernel did not reset the system"),
        Err(err) => eprintln!("reboot: {err}"),
    }
    std::process::exit(1);
}
