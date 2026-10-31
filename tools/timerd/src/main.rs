//! timerd: run commands at set times, like cron.
//!
//! Redox' `init` has no timers. `timerd` reads timer files from
//! `/usr/lib/timer.d` and `/etc/timer.d` (a file in `/etc` replaces one of
//! the same name in `/usr/lib`), for example `/etc/timer.d/backup.toml`:
//!
//! ```toml
//! [timer]
//! description = "Nightly backup"
//! schedule = "30 3 * * *"   # cron format, in local time
//! persistent = true         # run once at start if a run was missed
//! utc = false               # true: the schedule is in UTC
//!
//! [service]
//! cmd = "backup"
//! args = ["--all"]
//! ```
//!
//! The Pi has no battery-backed clock, so `timerd` first waits until
//! `timesync` has the time from an NTP server (`/tmp/timesync.synced`), at
//! most five minutes. The last run of each timer is saved, on the SD card's
//! data partition if mounted, so a missed run can be caught up after a
//! restart. The directories are read again every 30 seconds.
//!
//! Schedules are in the system's local time (`/etc/localtime`, through
//! relibc), UTC if no zone is set. When the clocks go back, a time in the
//! repeated hour runs once.
//!
//! Usage: `timerd` (daemon, started by init), `timerd -l` (list timers) or
//! `timerd -t <seconds since 1970>...` (show times in local time, as the
//! schedules see them; to check the time zone, for example around the
//! summer time switch).

mod cron;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{self, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cron::{format_utc, Schedule};
use serde::Deserialize;

const TIMER_DIRS: &[&str] = &["/usr/lib/timer.d", "/etc/timer.d"];
/// Where the last runs are saved: the SD card's data partition (network
/// boot, where the root filesystem is lost at reboot), else the root.
const STATE_DIRS: &[&str] = &["/scheme/data/timerd", "/var/lib/timerd"];
/// Written by `timesync` after its first NTP answer; /tmp is new each boot.
const SYNCED: &str = "/tmp/timesync.synced";
const CLOCK_WAIT: Duration = Duration::from_secs(300);
const RESCAN: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct TimerFile {
    timer: TimerSection,
    service: ServiceSection,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct TimerSection {
    #[serde(default)]
    description: String,
    schedule: String,
    #[serde(default)]
    persistent: bool,
    #[serde(default)]
    utc: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ServiceSection {
    cmd: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    envs: BTreeMap<String, String>,
}

/// A timer file as read, and the timer if the file is valid.
struct Entry {
    text: String,
    timer: Option<Timer>,
}

struct Timer {
    file: TimerFile,
    schedule: Schedule,
    running: Arc<AtomicBool>,
    /// Local minute of the last scheduled run, so that a time in the hour
    /// repeated when the clocks go back runs once.
    last_due: Option<i64>,
}

impl Timer {
    fn next_after(&self, unix: u64) -> Option<u64> {
        if self.file.timer.utc {
            self.schedule.next_after(unix, &|_| 0)
        } else {
            self.schedule.next_after(unix, &local_offset)
        }
    }
}

fn local_tm(unix: u64) -> libc::tm {
    let t = unix as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    tm
}

/// Seconds local time is ahead of UTC at `unix`.
fn local_offset(unix: u64) -> i64 {
    local_tm(unix).tm_gmtoff as i64
}

/// "YYYY-MM-DD HH:MM:SS CET" in local time.
fn format_local(unix: u64) -> String {
    let tm = local_tm(unix);
    let zone = if tm.tm_zone.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(tm.tm_zone) }.to_string_lossy().into_owned()
    };
    let local = (unix as i64 + tm.tm_gmtoff as i64) as u64;
    format!("{} {zone}", format_utc(local))
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn parse_timer(text: &str) -> Result<(TimerFile, Schedule), String> {
    let file: TimerFile = toml::from_str(text).map_err(|err| err.to_string())?;
    let schedule = Schedule::parse(&file.timer.schedule)?;
    Ok((file, schedule))
}

/// Timer name (file name without `.toml`) to file text; `/etc` wins.
fn read_timer_files() -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    for dir in TIMER_DIRS {
        let Ok(entries) = fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            let Some(name) = name.strip_suffix(".toml") else { continue };
            if let Ok(text) = fs::read_to_string(&path) {
                files.insert(name.to_string(), text);
            }
        }
    }
    files
}

fn last_run(name: &str) -> Option<u64> {
    STATE_DIRS
        .iter()
        .filter_map(|dir| fs::read_to_string(Path::new(dir).join(name)).ok()?.trim().parse().ok())
        .max()
}

fn save_last_run(name: &str, unix: u64) {
    for dir in STATE_DIRS {
        let _ = fs::create_dir_all(dir);
        if fs::write(Path::new(dir).join(name), format!("{unix}\n")).is_ok() {
            return;
        }
    }
    eprintln!("timerd: {name}: cannot save the last run in {}", STATE_DIRS.join(" or "));
}

fn start(name: &str, timer: &Timer, reason: &str) {
    if timer.running.swap(true, Ordering::SeqCst) {
        println!("timerd: {name}: still running, skipped this run");
        return;
    }
    let started = now();
    save_last_run(name, started);
    let service = &timer.file.service;
    let mut command = Command::new(&service.cmd);
    command.args(&service.args).envs(&service.envs);
    println!("timerd: {name}: starting {} ({reason})", service.cmd);
    let running = timer.running.clone();
    let name = name.to_string();
    thread::spawn(move || {
        let begin = Instant::now();
        match command.status() {
            Ok(status) => match status.code() {
                Some(code) => println!(
                    "timerd: {name}: exited with code {code} after {:.1} s",
                    begin.elapsed().as_secs_f32()
                ),
                None => println!("timerd: {name}: ended by a signal ({status})"),
            },
            Err(err) => eprintln!("timerd: {name}: cannot start: {err}"),
        }
        running.store(false, Ordering::SeqCst);
    });
}

/// Wait until timesync has set the clock from a server, at most CLOCK_WAIT.
fn wait_for_clock() {
    let begin = Instant::now();
    while !Path::new(SYNCED).exists() {
        if begin.elapsed() >= CLOCK_WAIT {
            println!(
                "timerd: no time from timesync after {} s, going on with {}",
                CLOCK_WAIT.as_secs(),
                format_local(now())
            );
            return;
        }
        sleep(Duration::from_secs(1));
    }
    println!("timerd: clock set by timesync, {}", format_local(now()));
}

fn list() {
    let now = now();
    for (name, text) in read_timer_files() {
        match parse_timer(&text) {
            Ok((file, schedule)) => {
                let timer = Timer { file, schedule, running: Default::default(), last_due: None };
                let next = timer.next_after(now).map_or("never".into(), format_local);
                let last = last_run(&name).map_or("never".into(), format_local);
                let file = timer.file;
                println!("{name}: {} \"{}\", next {next}, last {last}", file.timer.description, file.timer.schedule);
                println!("    {} {}", file.service.cmd, file.service.args.join(" "));
            }
            Err(err) => println!("{name}: error: {err}"),
        }
    }
}

/// `timerd -t`: each time in UTC and in local time, with the UTC offset.
fn show_times(args: impl Iterator<Item = String>) {
    let mut any = false;
    for arg in args {
        let Ok(unix) = arg.trim_start_matches('@').parse::<u64>() else {
            eprintln!("timerd: not seconds since 1970: {arg}");
            process::exit(2);
        };
        let offset = local_offset(unix);
        let sign = if offset < 0 { '-' } else { '+' };
        println!(
            "{} UTC = {} ({sign}{:02}:{:02})",
            format_utc(unix),
            format_local(unix),
            offset.abs() / 3600,
            offset.abs() % 3600 / 60
        );
        any = true;
    }
    if !any {
        eprintln!("usage: timerd -t seconds...");
        process::exit(2);
    }
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("-l") => return list(),
        Some("-t") => return show_times(std::env::args().skip(2)),
        Some(_) => {
            eprintln!("usage: timerd [-l | -t seconds...]");
            process::exit(2);
        }
    }

    wait_for_clock();
    let mut entries: BTreeMap<String, Entry> = BTreeMap::new();
    // Each timer runs when its schedule has a time in (checked, now].
    let mut checked = now();
    let mut last_scan: Option<Instant> = None;
    loop {
        let now = now();
        if now < checked {
            // The clock went back: do not run anything twice.
            checked = now;
        }

        if last_scan.map_or(true, |t| t.elapsed() >= RESCAN) {
            last_scan = Some(Instant::now());
            let files = read_timer_files();
            entries.retain(|name, _| {
                let keep = files.contains_key(name);
                if !keep {
                    println!("timerd: {name}: removed");
                }
                keep
            });
            for (name, text) in files {
                if entries.get(&name).is_some_and(|e| e.text == text) {
                    continue;
                }
                let timer = match parse_timer(&text) {
                    Ok((file, schedule)) => Some(load(&name, file, schedule, now)),
                    Err(err) => {
                        eprintln!("timerd: {name}: {err}");
                        None
                    }
                };
                entries.insert(name, Entry { text, timer });
            }
        }

        let mut wake = now + RESCAN.as_secs();
        for (name, entry) in &mut entries {
            let Some(timer) = &mut entry.timer else { continue };
            if let Some(due) = timer.next_after(checked).filter(|due| *due <= now) {
                let local_due = (due as i64 + local_offset(due)) / 60;
                if timer.file.timer.utc || timer.last_due != Some(local_due) {
                    timer.last_due = Some(local_due);
                    start(name, timer, "scheduled");
                }
            }
            if let Some(next) = timer.next_after(now) {
                wake = wake.min(next);
            }
        }
        checked = now;
        sleep(Duration::from_secs(wake.saturating_sub(now).max(1)));
    }
}

/// A new or changed timer: report it and catch up a missed run.
fn load(name: &str, file: TimerFile, schedule: Schedule, now: u64) -> Timer {
    let timer = Timer { file, schedule, running: Arc::new(AtomicBool::new(false)), last_due: None };
    let next = timer.next_after(now).map_or("never".into(), format_local);
    println!("timerd: {name}: \"{}\", next run {next}", timer.file.timer.schedule);
    match last_run(name) {
        Some(last) if timer.file.timer.persistent => {
            if timer.next_after(last).is_some_and(|due| due <= now) {
                start(name, &timer, &format!("missed since {}", format_local(last)));
            }
        }
        Some(_) => {}
        // From now on a missed run can be found.
        None => save_last_run(name, now),
    }
    timer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_file() {
        let (file, _) = parse_timer(
            "[timer]\nschedule = \"*/5 * * * *\"\npersistent = true\n\n[service]\ncmd = \"date\"\nargs = [\"-u\"]\n",
        )
        .unwrap();
        assert!(file.timer.persistent);
        assert_eq!(file.service.args, ["-u"]);
        assert!(parse_timer("[timer]\nschedule = \"* * *\"\n[service]\ncmd = \"x\"\n").is_err());
        assert!(parse_timer("[timer]\nschedule = \"* * * * *\"\nbogus = 1\n[service]\ncmd = \"x\"\n").is_err());
    }
}
