//! Network diagnosis in one snapshot (#566): the counters of every network
//! driver (`/scheme/network.<x>/stats`), the interfaces from netcfg, the
//! interrupts per line (`/scheme/sys/irq`) and the CPU time per process
//! (`/scheme/sys/context`).
//!
//! ```text
//! netdiag                print the snapshot
//! netdiag save <file>    store it
//! netdiag diff <file>    what changed since the stored one
//! ```
//!
//! A typical use: `netdiag save /tmp/a`, run the transfer or wait for the
//! problem, `netdiag diff /tmp/a`. Frames that grow in a driver but not
//! further up point at netstack; a driver whose counters stand still while
//! its interrupt line grows (or the other way round) points at the driver.

use std::collections::BTreeMap;
use std::fs;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// Key → value, sorted by key. Values are numbers or short texts.
type Snapshot = BTreeMap<String, String>;

/// `HH:MM:SS.cc` to hundredths of a second.
fn parse_time(text: &str) -> Option<u64> {
    let (hms, cc) = text.split_once('.')?;
    let mut parts = hms.split(':').map(|p| p.parse::<u64>().ok());
    let (h, m, s) = (parts.next()??, parts.next()??, parts.next()??);
    if parts.next().is_some() || cc.len() != 2 {
        return None;
    }
    Some(((h * 60 + m) * 60 + s) * 100 + cc.parse::<u64>().ok()?)
}

/// `name value` lines of a stats file under `prefix/`.
fn add_stats(snap: &mut Snapshot, prefix: &str, text: &str) {
    for line in text.lines() {
        if let Some((name, value)) = line.trim().split_once(' ') {
            snap.insert(format!("{prefix}/{name}"), value.trim().to_owned());
        }
    }
}

/// `/scheme/sys/irq` lines `<line>: <count>` under `irq/`; lines that
/// never fired are left out.
fn add_irqs(snap: &mut Snapshot, text: &str) {
    for line in text.lines() {
        if let Some((irq, count)) = line.split_once(':') {
            if count.trim().parse::<u64>().is_ok_and(|n| n > 0) {
                snap.insert(format!("irq/{}", irq.trim()), count.trim().to_owned());
            }
        }
    }
}

/// CPU time (ACTTIME, hundredths of a second) per process name from
/// `/scheme/sys/context`, summed over its threads, under `cpu/`. The name
/// is the last field of a row.
fn add_cpu(snap: &mut Snapshot, text: &str) {
    let mut sum: BTreeMap<String, u64> = BTreeMap::new();
    for line in text.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(time) = fields.iter().find_map(|f| parse_time(f)) else { continue };
        let Some(name) = fields.last() else { continue };
        let name = name.rsplit('/').next().unwrap_or(name);
        // netdiag itself is a new process each time.
        if name == "netdiag" {
            continue;
        }
        *sum.entry(name.to_owned()).or_default() += time;
    }
    for (name, time) in sum {
        snap.insert(format!("cpu/{name}"), time.to_string());
    }
}

fn read(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// A file inside a network scheme. A plain open of the whole path fails
/// with ENODEV there, so as netstack does: open the scheme root as a
/// directory, then the file in it.
fn read_scheme_file(scheme: &str, file: &str) -> Option<String> {
    use std::ffi::CString;
    use std::io::Read;
    use std::os::fd::FromRawFd;
    let root = CString::new(format!("/scheme/{scheme}")).ok()?;
    let name = CString::new(file).ok()?;
    // SAFETY: plain libc calls with valid C strings; each fd is owned once.
    unsafe {
        let dir = libc::open(root.as_ptr(), libc::O_DIRECTORY | libc::O_RDONLY | libc::O_CLOEXEC);
        if dir < 0 {
            return None;
        }
        let dir = fs::File::from_raw_fd(dir);
        let fd = libc::openat(std::os::fd::AsRawFd::as_raw_fd(&dir), name.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC);
        if fd < 0 {
            return None;
        }
        let mut text = String::new();
        fs::File::from_raw_fd(fd).read_to_string(&mut text).ok()?;
        Some(text)
    }
}

fn take() -> Snapshot {
    let mut snap = Snapshot::new();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis());
    snap.insert("time_ms".to_owned(), now.to_string());
    let mut schemes: Vec<String> = fs::read_dir("/scheme")
        .map(|dir| dir.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect())
        .unwrap_or_default();
    schemes.retain(|name| name.starts_with("network."));
    schemes.sort();
    for scheme in schemes {
        match read_scheme_file(&scheme, "stats").or_else(|| read(&format!("/scheme/{scheme}/stats"))) {
            Some(text) => add_stats(&mut snap, &scheme, &text),
            None => {
                snap.insert(format!("{scheme}/stats"), "none".to_owned());
            }
        }
    }
    for iface in read("/scheme/netcfg/ifaces").unwrap_or_default().lines().map(str::trim) {
        if iface.is_empty() {
            continue;
        }
        for cfg in ["mac", "addr/list"] {
            if let Some(value) = read(&format!("/scheme/netcfg/ifaces/{iface}/{cfg}")) {
                let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
                snap.insert(format!("netcfg/{iface}/{cfg}"), value);
            }
        }
    }
    if let Some(route) = read("/scheme/netcfg/route/list") {
        snap.insert("netcfg/route".to_owned(), route.lines().collect::<Vec<_>>().join("; "));
    }
    if let Some(text) = read("/scheme/sys/irq") {
        add_irqs(&mut snap, &text);
    }
    if let Some(text) = read("/scheme/sys/context") {
        add_cpu(&mut snap, &text);
    }
    snap
}

fn encode(snap: &Snapshot) -> String {
    snap.iter().map(|(k, v)| format!("{k}\t{v}\n")).collect()
}

fn decode(text: &str) -> Snapshot {
    text.lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// Lines for what changed from `old` to `new`: numbers with their
/// difference, texts as old → new, and keys that came or went. CPU times
/// are shown in seconds.
fn diff(old: &Snapshot, new: &Snapshot) -> Vec<String> {
    let mut out = Vec::new();
    if let (Some(a), Some(b)) = (old.get("time_ms"), new.get("time_ms")) {
        if let (Ok(a), Ok(b)) = (a.parse::<u64>(), b.parse::<u64>()) {
            out.push(format!("{:.1} s since the snapshot", b.saturating_sub(a) as f64 / 1000.0));
        }
    }
    for (key, value) in new {
        if key == "time_ms" {
            continue;
        }
        let Some(before) = old.get(key) else {
            out.push(format!("{key} {value} (new)"));
            continue;
        };
        if before == value {
            continue;
        }
        match (before.parse::<i128>(), value.parse::<i128>()) {
            (Ok(a), Ok(b)) if key.starts_with("cpu/") => {
                out.push(format!("{key} {:+.2} s", (b - a) as f64 / 100.0));
            }
            (Ok(a), Ok(b)) => out.push(format!("{key} {a} -> {b} ({:+})", b - a)),
            _ => out.push(format!("{key} {before} -> {value}")),
        }
    }
    for key in old.keys().filter(|k| !new.contains_key(*k)) {
        out.push(format!("{key} gone"));
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        [] => {
            print!("{}", encode(&take()).replace('\t', " "));
            Ok(())
        }
        ["save", file] => fs::write(file, encode(&take())).map_err(|e| format!("{file}: {e}")),
        ["diff", file] => fs::read_to_string(file).map_err(|e| format!("{file}: {e}")).map(|text| {
            for line in diff(&decode(&text), &take()) {
                println!("{line}");
            }
        }),
        _ => Err("usage: netdiag [save <file> | diff <file>]".to_owned()),
    };
    if let Err(err) = result {
        eprintln!("netdiag: {err}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_irq_and_cpu() {
        let mut s = Snapshot::new();
        add_stats(&mut s, "network.usb", "rx_frames 12\nadapter_4 lan7800 link 1 active\n");
        add_irqs(&mut s, "  9: 1200\n189: 7\n190: 0\nbad line\n");
        add_cpu(
            &mut s,
            "PID   PGID  PPID  STAT  CPU  AFFINITY  ACTTIME      SCHTIME  MEM  NAME\n\
             12    12    1     S     #0   0x1  00:00:03.04  00:00:03.50  1 MB  /usr/lib/drivers/bcmgenetd\n\
             13    13    1     R     #1   0x1  00:01:00.10  00:01:00.20  2 MB  netstack\n\
             13    13    1     S     #1   0x1  00:00:00.90  00:00:01.00  2 MB  netstack\n",
        );
        assert_eq!(s["network.usb/rx_frames"], "12");
        assert_eq!(s["network.usb/adapter_4"], "lan7800 link 1 active");
        assert_eq!(s["irq/9"], "1200");
        assert_eq!(s["irq/189"], "7");
        assert!(!s.contains_key("irq/190"));
        assert_eq!(s["cpu/bcmgenetd"], "304");
        assert_eq!(s["cpu/netstack"], "6100");
    }

    #[test]
    fn diff_numbers_texts_and_keys() {
        let old = decode("time_ms\t1000\na\t5\nb\tx\ncpu/n\t100\ngone\t1\n");
        let new = decode("time_ms\t3500\na\t9\nb\ty\ncpu/n\t350\nfresh\t2\n");
        assert_eq!(
            diff(&old, &new),
            ["2.5 s since the snapshot", "a 5 -> 9 (+4)", "b x -> y", "cpu/n +2.50 s", "fresh 2 (new)", "gone gone"]
        );
        assert_eq!(decode(&encode(&new)), new);
    }
}
