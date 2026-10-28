//! Shared code of `reboot` and `poweroff`: flush the file systems and ask
//! the kernel to stop through `/scheme/sys/kstop`. Needs root.

use std::fs::OpenOptions;
use std::io::Write;

extern "C" {
    // In relibc, but not declared for Redox by the libc crate.
    fn sync();
}

/// Writes `request` (`reset` or `shutdown`) to `kstop`. Returns only if the
/// kernel did not stop the system.
pub fn kstop(tool: &str, request: &str) -> ! {
    unsafe { sync() };
    let result = OpenOptions::new()
        .write(true)
        .open("/scheme/sys/kstop")
        .and_then(|mut kstop| kstop.write_all(request.as_bytes()));
    match result {
        Ok(()) => eprintln!("{tool}: the kernel did not {request} the system"),
        Err(err) => eprintln!("{tool}: {err}"),
    }
    std::process::exit(1);
}
