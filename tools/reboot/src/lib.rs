//! Shared code of `reboot` and `poweroff`: stop the session programs,
//! close the file systems and ask the kernel to stop through
//! `/scheme/sys/kstop`. Needs root.
//!
//! Redox has no shutdown sequence in `init`, and `sync()` does nothing in
//! relibc (RedoxFS writes each change through to the disk). So the tools do
//! it themselves, in this order:
//!
//! 1. SIGTERM to the programs started after boot (from `/usr/bin` and
//!    elsewhere, not from the initfs): shells, `login`, `getty`, `devshd`,
//!    `timesync`, network daemons. After 5 seconds SIGKILL to those still
//!    there. Kept: drivers and daemons from the initfs, `ptyd`, and RedoxFS.
//! 2. SIGTERM to each RedoxFS: it unmounts after the next request (which a
//!    `stat` of `/` sends to the root), committing its allocator.
//! 3. A second of rest, so the SD card can finish writing.
//! 4. `kstop`.

use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Programs in step 1 have this long to end after SIGTERM.
const GRACE: Duration = Duration::from_secs(5);
/// Rest before `kstop` for the SD card.
const SETTLE: Duration = Duration::from_secs(1);

/// Programs not from the initfs that step 1 leaves running.
const KEEP: &[&str] = &["ptyd", "redoxfs"];

/// Reports go to the terminal and to the serial console (`/scheme/debug`,
/// opened before anything stops): the terminal may close in step 1.
struct Report {
    tool: &'static str,
    console: Option<File>,
}

impl Report {
    fn say(&mut self, text: &str) {
        eprintln!("{}: {text}", self.tool);
        if let Some(console) = &mut self.console {
            let _ = writeln!(console, "{}: {text}", self.tool);
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
fn stop_programs(report: &mut Report) -> (usize, usize) {
    let me = std::process::id();
    let programs: Vec<Process> = processes()
        .into_iter()
        .filter(|p| p.pid > 1 && p.pid != me)
        .filter(|p| !p.name.starts_with("/scheme/initfs/") && !KEEP.contains(&base_name(&p.name)))
        .collect();
    let targets: BTreeSet<u32> = programs.iter().map(|p| p.pid).collect();
    report.say(&format!("SIGTERM to {} programs", targets.len()));
    for p in &programs {
        report.say(&format!("SIGTERM {} {}", p.pid, p.name));
        signal(p.pid, libc::SIGTERM);
    }
    report.say("waiting");
    let alive = || -> BTreeSet<u32> {
        processes().into_iter().map(|p| p.pid).filter(|pid| targets.contains(pid)).collect()
    };
    let deadline = Instant::now() + GRACE;
    let mut left = alive();
    while !left.is_empty() && Instant::now() < deadline {
        sleep(Duration::from_millis(100));
        left = alive();
        report.say(&format!("{} left", left.len()));
    }
    for &pid in &left {
        signal(pid, libc::SIGKILL);
    }
    (targets.len() - left.len(), left.len())
}

/// Step 2. Returns how many RedoxFS processes were asked to unmount.
fn close_file_systems() -> usize {
    let filesystems: Vec<u32> =
        processes().into_iter().filter(|p| base_name(&p.name) == "redoxfs").map(|p| p.pid).collect();
    for &pid in &filesystems {
        signal(pid, libc::SIGTERM);
    }
    // RedoxFS notices the signal only after its next request.
    let _ = std::fs::metadata("/");
    filesystems.len()
}

/// Stops the system as described above and writes `request` (`reset` or
/// `shutdown`) to `kstop`. Returns only if the kernel did not stop.
pub fn kstop(tool: &'static str, request: &str) -> ! {
    // Step 1 ends the shell and the terminal this tool runs in: leave their
    // session, and ignore what they send on the way out.
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
    report.say("stopping programs");
    let (stopped, killed) = stop_programs(&mut report);
    report.say(&format!("{stopped} programs stopped, {killed} killed"));
    let filesystems = close_file_systems();
    report.say(&format!("{filesystems} file systems closed"));
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
