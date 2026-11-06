//! Who uses the CPU right now: samples `/scheme/sys/context` (the kernel's
//! CPU time per thread, ACTTIME), `/scheme/sys/stat` (busy and idle
//! milliseconds per core) and `/scheme/sys/irq` (interrupts per line) twice
//! and prints the difference.
//!
//! ```text
//! cputop [-n rounds] [-l lines] [-w] [seconds]
//! ```
//!
//! The default is one round of 5 seconds and the 10 busiest threads; `-w`
//! sorts by wakeups instead. A thread at 100 % keeps one core busy. `ps`
//! only shows the CPU time since a thread started, which does not say who
//! is busy now.
//!
//! With kernel patch `cputop` `0001`, `/scheme/sys/cputime` has the CPU
//! time per thread in nanoseconds and how often each thread was switched
//! to (its wakeups); without it the times are in hundredths of a second
//! and the wakeup column is empty.

use std::collections::HashMap;
use std::fs;
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};

const CONTEXT: &str = "/scheme/sys/context";
const STAT: &str = "/scheme/sys/stat";
const CPUTIME: &str = "/scheme/sys/cputime";
const IRQ: &str = "/scheme/sys/irq";

/// One row of `/scheme/sys/context`: a thread.
struct Thread {
    pid: String,
    stat: String,
    cpu: String,
    /// CPU time in nanoseconds.
    time: u64,
    /// How often the thread was switched to, if the kernel counts it.
    switches: Option<u64>,
    name: String,
}

type Key = (String, String, usize);

/// The key of a thread: PID, name and position among the rows with the
/// same PID and name (the kernel lists a row per thread, sorted by PID).
fn key<T>(map: &HashMap<Key, T>, pid: &str, name: &str) -> Key {
    let mut n = 0;
    while map.contains_key(&(pid.to_owned(), name.to_owned(), n)) {
        n += 1;
    }
    (pid.to_owned(), name.to_owned(), n)
}

/// `HH:MM:SS.cc` to hundredths of a second.
fn parse_time(text: &str) -> Option<u64> {
    let (hms, cc) = text.split_once('.')?;
    let mut parts = hms.split(':').map(|p| p.parse::<u64>().ok());
    let (h, m, s) = (parts.next()??, parts.next()??, parts.next()??);
    Some(((h * 60 + m) * 60 + s) * 100 + cc.parse::<u64>().ok()?)
}

/// The threads from `sys:context`, with the exact times from `sys:cputime`
/// where the kernel has it.
fn threads() -> Result<HashMap<Key, Thread>, String> {
    let text = fs::read_to_string(CONTEXT).map_err(|err| format!("{CONTEXT}: {err}"))?;
    let mut threads = HashMap::new();
    for line in text.lines().skip(1) {
        // PID EUID EGID STAT CPU AFFINITY ACTTIME SCHTIME PRIVATE SHARED NAME;
        // the memory columns can be "12 KB", so the name is found after
        // the times.
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(at) = fields.iter().position(|f| parse_time(f).is_some()) else {
            continue;
        };
        if at < 4 || fields.len() < at + 2 {
            continue;
        }
        let Some(time) = parse_time(fields[at]) else { continue };
        let rest = &fields[at + 2..];
        // PRIVATE and SHARED are a number with or without a unit each.
        let mut skip = 0;
        for _ in 0..2 {
            if rest.get(skip).is_some_and(|f| f.parse::<f64>().is_ok()) {
                skip += 1;
                if matches!(rest.get(skip).copied(), Some("B" | "KB" | "MB" | "GB")) {
                    skip += 1;
                }
            }
        }
        let name = rest.get(skip..).map(|n| n.join(" ")).unwrap_or_default();
        let thread = Thread {
            pid: fields[0].to_owned(),
            stat: fields[3].to_owned(),
            cpu: if at >= 6 { fields[4].to_owned() } else { "?".to_owned() },
            time: time * 10_000_000,
            switches: None,
            name,
        };
        threads.insert(key(&threads, &thread.pid, &thread.name), thread);
    }
    if let Ok(text) = fs::read_to_string(CPUTIME) {
        let mut seen = HashMap::new();
        for line in text.lines().skip(1) {
            // PID CPU NANOS SWITCHES NAME
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [pid, _cpu, nanos, switches, ref name @ ..] = fields[..] else { continue };
            let (Ok(nanos), Ok(switches)) = (nanos.parse(), switches.parse()) else { continue };
            let k = key(&seen, pid, &name.join(" "));
            seen.insert(k.clone(), ());
            if let Some(thread) = threads.get_mut(&k) {
                thread.time = nanos;
                thread.switches = Some(switches);
            }
        }
    }
    Ok(threads)
}

/// Interrupts per line (`N: count`).
fn irqs() -> HashMap<String, u64> {
    let text = fs::read_to_string(IRQ).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let (irq, count) = line.split_once(':')?;
            Some((irq.trim().to_owned(), count.trim().parse().ok()?))
        })
        .collect()
}

/// Per core: busy (user + nice + kernel + irq) and idle milliseconds, and
/// context switches (`cpuN user nice kernel idle irq switches`).
fn cores() -> Result<Vec<(String, u64, u64, u64)>, String> {
    let text = fs::read_to_string(STAT).map_err(|err| format!("{STAT}: {err}"))?;
    Ok(text
        .lines()
        .filter(|line| line.starts_with("cpu") && !line.starts_with("cpu "))
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?.to_owned();
            let v: Vec<u64> = fields.filter_map(|f| f.parse().ok()).collect();
            let [user, nice, kernel, idle, irq, ref rest @ ..] = v[..] else { return None };
            Some((name, user + nice + kernel + irq, idle, rest.first().copied().unwrap_or(0)))
        })
        .collect())
}

fn round(seconds: f64, lines: usize, by_wakeups: bool) -> Result<(), String> {
    let (threads_before, cores_before, irqs_before) = (threads()?, cores()?, irqs());
    let started = Instant::now();
    // On Redox, sleep can come back early (seen as "0.0 s" rounds while
    // pointer-bench starts in the background), so sleep until it is over.
    while let Some(left) = Duration::from_secs_f64(seconds).checked_sub(started.elapsed()) {
        sleep(left);
    }
    let (threads_after, cores_after, irqs_after) = (threads()?, cores()?, irqs());
    let elapsed = started.elapsed().as_secs_f64();

    println!("cputop: {elapsed:.1} s, {} threads", threads_after.len());
    for (name, busy, idle, switches) in &cores_after {
        let Some((_, busy0, idle0, switches0)) = cores_before.iter().find(|(n, ..)| n == name) else { continue };
        let (busy, idle) = (busy.saturating_sub(*busy0), idle.saturating_sub(*idle0));
        let share = if busy + idle > 0 { busy * 100 / (busy + idle) } else { 0 };
        let switches = (switches.saturating_sub(*switches0) as f64 / elapsed) as u64;
        println!("{name:<5} busy {share:>3} %  ({busy} ms busy, {idle} ms idle, {switches} switches/s)");
    }

    let mut lines_irq: Vec<(u64, &String)> = irqs_after
        .iter()
        .map(|(irq, count)| (count.saturating_sub(irqs_before.get(irq).copied().unwrap_or(0)), irq))
        .filter(|(delta, _)| *delta > 0)
        .collect();
    lines_irq.sort_by(|a, b| b.0.cmp(&a.0));
    if !lines_irq.is_empty() {
        let total: u64 = lines_irq.iter().map(|(delta, _)| delta).sum();
        let each: Vec<String> = lines_irq
            .iter()
            .map(|(delta, irq)| format!("{irq}: {:.0}", *delta as f64 / elapsed))
            .collect();
        println!("irq   {:.0}/s  ({})", total as f64 / elapsed, each.join(", "));
    }

    // CPU time (ns) and wakeups in the interval.
    let mut used: Vec<(u64, Option<u64>, &Thread)> = threads_after
        .iter()
        .map(|(key, t)| {
            let before = threads_before.get(key);
            let time = t.time.saturating_sub(before.map_or(0, |b| b.time));
            let wakeups = t.switches.map(|s| s.saturating_sub(before.and_then(|b| b.switches).unwrap_or(0)));
            (time, wakeups, t)
        })
        .collect();
    if by_wakeups {
        used.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    } else {
        used.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)).then_with(|| b.2.time.cmp(&a.2.time)));
    }
    println!("{:>6} {:>6} {:>7} {:>12} {:<5} {:<4} NAME", "PID", "CPU%", "WAKE/s", "TOTAL", "STAT", "CPU");
    for (delta, wakeups, t) in used.iter().take(lines) {
        let share = *delta as f64 / (elapsed * 1e9) * 100.0;
        let wakeups = wakeups.map_or("-".to_owned(), |w| format!("{:.0}", w as f64 / elapsed));
        let total = t.time / 10_000_000;
        let seconds = total / 100;
        println!(
            "{:>6} {:>6.1} {:>7} {:>4}:{:02}:{:02}.{:01} {:<5} {:<4} {}",
            t.pid,
            share,
            wakeups,
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60,
            total % 100 / 10,
            t.stat,
            t.cpu,
            t.name
        );
    }
    Ok(())
}

fn usage() -> ! {
    eprintln!("usage: cputop [-n rounds] [-l lines] [-w] [seconds]");
    process::exit(2);
}

fn main() {
    let (mut rounds, mut lines, mut seconds, mut by_wakeups) = (1u32, 10usize, 5.0f64, false);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = |args: &mut dyn Iterator<Item = String>| args.next().and_then(|v| v.parse().ok());
        match arg.as_str() {
            "-n" => rounds = value(&mut args).unwrap_or_else(|| usage()),
            "-l" => lines = value(&mut args).map(|v: u32| v as usize).unwrap_or_else(|| usage()),
            "-w" => by_wakeups = true,
            "-h" | "--help" => usage(),
            s => seconds = s.parse().ok().filter(|&s: &f64| s > 0.0).unwrap_or_else(|| usage()),
        }
    }
    for i in 0..rounds {
        if i > 0 {
            println!();
        }
        if let Err(err) = round(seconds, lines, by_wakeups) {
            eprintln!("cputop: {err}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_time;

    #[test]
    fn times() {
        assert_eq!(parse_time("00:00:01.50"), Some(150));
        assert_eq!(parse_time("00:14:11.03"), Some(85103));
        assert_eq!(parse_time("01:00:00.00"), Some(360000));
        assert_eq!(parse_time("12KB"), None);
        assert_eq!(parse_time("3.5"), None);
    }
}
