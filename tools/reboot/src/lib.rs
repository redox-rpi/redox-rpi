//! Shared code of `reboot` and `poweroff`: stop the programs started after
//! boot and ask the kernel to stop through `/scheme/sys/kstop`. Needs root.
//!
//! Redox has no shutdown sequence in `init`, and `sync()` does nothing in
//! relibc. RedoxFS needs none: each change is a transaction that writes its
//! blocks and then a new header before the call returns. So the tools:
//!
//! 1. Send SIGTERM to the programs started after boot (not from the
//!    initfs): shells, `login`, `getty`, `devshd`, `timesync`, network
//!    daemons, so they can save what they hold. After 5 seconds SIGKILL to
//!    those still there. Kept: drivers and daemons from the initfs, `ptyd`,
//!    and RedoxFS.
//! 2. Wait a second, so the SD card can finish writing.
//! 3. Write to `kstop`.

use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Programs have this long to end after SIGTERM.
const GRACE: Duration = Duration::from_secs(5);
/// Rest before `kstop` for the SD card.
const SETTLE: Duration = Duration::from_secs(1);

/// Programs not from the initfs that step 1 leaves running.
const KEEP: &[&str] = &["ptyd", "redoxfs"];

/// Reports go to the serial console (`/scheme/debug`). Not to the terminal:
/// it is a pty that `getty` holds, and once step 1 has ended `getty`, a
/// write to it never returns.
struct Report {
    tool: &'static str,
    console: Option<File>,
}

impl Report {
    fn say(&mut self, text: &str) {
        if let Some(console) = &mut self.console {
            let _ = writeln!(console, "{}: {text}", self.tool);
        }
    }
}

/// Points standard input, output and error at `/scheme/null`, away from
/// the terminal.
fn leave_terminal() {
    if let Ok(null) = OpenOptions::new().read(true).write(true).open("/scheme/null") {
        use std::os::fd::AsRawFd;
        for fd in 0..3 {
            unsafe { libc::dup2(null.as_raw_fd(), fd) };
        }
    }
}

struct Process {
    pid: u32,
    name: String,
}

/// User processes that have not ended, from `/scheme/sys/context`; one
/// entry per process (the list has a row per thread).
fn processes() -> Vec<Process> {
    let Ok(list) = std::fs::read_to_string("/scheme/sys/context") else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    list.lines()
        .skip(1)
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let pid = words.first()?.parse().ok()?;
            let stat = words.get(3)?;
            let name = words.last()?.to_string();
            (stat.starts_with('U') && !stat.contains('Z') && seen.insert(pid)).then_some(Process { pid, name })
        })
        .collect()
}

fn base_name(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

fn signal(pid: u32, signal: i32) {
    unsafe { libc::kill(pid as libc::pid_t, signal) };
}

/// Step 1. Returns how many ended on SIGTERM and how many needed SIGKILL.
fn stop_programs() -> (usize, usize) {
    let me = std::process::id();
    let programs: Vec<Process> = processes()
        .into_iter()
        .filter(|p| p.pid > 1 && p.pid != me)
        .filter(|p| !p.name.starts_with("/scheme/initfs/") && !KEEP.contains(&base_name(&p.name)))
        .collect();
    let targets: BTreeSet<u32> = programs.iter().map(|p| p.pid).collect();
    for p in &programs {
        signal(p.pid, libc::SIGTERM);
    }
    let alive = || -> BTreeSet<u32> {
        processes().into_iter().map(|p| p.pid).filter(|pid| targets.contains(pid)).collect()
    };
    let deadline = Instant::now() + GRACE;
    let mut left = alive();
    while !left.is_empty() && Instant::now() < deadline {
        sleep(Duration::from_millis(100));
        left = alive();
    }
    for &pid in &left {
        signal(pid, libc::SIGKILL);
    }
    (targets.len() - left.len(), left.len())
}

/// Stops the system as described above and writes `request` (`reset` or
/// `shutdown`) to `kstop`. Returns only if the kernel did not stop.
pub fn kstop(tool: &'static str, request: &str) -> ! {
    // Step 1 ends the shell and the terminal this tool runs in: leave their
    // session, ignore what they send on the way out, and write no more to
    // the terminal.
    unsafe {
        libc::setsid();
        for sig in [libc::SIGHUP, libc::SIGINT, libc::SIGTERM, libc::SIGPIPE] {
            libc::signal(sig, libc::SIG_IGN);
        }
    }
    let mut report = Report {
        tool,
        console: OpenOptions::new().write(true).open("/scheme/debug").ok(),
    };
    eprintln!("{tool}: stopping the system, the rest is on the serial console");
    leave_terminal();
    let (stopped, killed) = stop_programs();
    report.say(&format!("{stopped} programs stopped, {killed} killed"));
    sleep(SETTLE);
    report.say(&format!("{request} through kstop"));
    let result = OpenOptions::new()
        .write(true)
        .open("/scheme/sys/kstop")
        .and_then(|mut kstop| kstop.write_all(request.as_bytes()));
    match result {
        Ok(()) => report.say(&format!("the kernel did not {request} the system")),
        Err(err) => report.say(&format!("kstop: {err}")),
    }
    std::process::exit(1);
}
