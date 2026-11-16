//! poweroff: stop the system so the power can be pulled.
//!
//! Like `shutdown`, but it first asks the headphone jack driver to ramp its
//! outputs down to 0 V (`/scheme/audiohw/stop`, #292), so pulling the power
//! afterwards does not click. Then it flushes the file systems and stops the
//! kernel through `/scheme/sys/kstop`. Needs root. The Raspberry Pi 3 has no
//! PSCI, so the power stays on: pull it once the console shows `kstop`. A
//! real power-off is #7.

use std::fs::OpenOptions;
use std::io::Write;

extern "C" {
    // In relibc, but not declared for Redox by the libc crate.
    fn sync();
}

fn main() {
    if matches!(std::env::args().nth(1).as_deref(), Some("--version" | "-V")) {
        println!("poweroff {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    // Returns once the outputs are at 0 V.
    let _ = std::fs::write("/scheme/audiohw/stop", "1");
    unsafe { sync() };
    let result = OpenOptions::new()
        .write(true)
        .open("/scheme/sys/kstop")
        .and_then(|mut kstop| kstop.write_all(b"shutdown"));
    // Only reached if the kernel did not stop.
    match result {
        Ok(()) => eprintln!("poweroff: the kernel did not stop the system"),
        Err(err) => eprintln!("poweroff: {err}"),
    }
    std::process::exit(1);
}
