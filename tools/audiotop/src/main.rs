//! Live view of the audio driver's counters: reads `/scheme/audiohw/stats`
//! (`bcm2835-hdmiaudiod` or `bcm2835-pwmaudiod`) every interval and prints
//! one line with the rate and how much each counter grew (#565).
//!
//! ```text
//! audiotop [-i seconds] [-n rounds] [-c file.csv] [-f stats-file] [-q]
//! ```
//!
//! The default is an interval of 1 second until Ctrl-C (`-n 0`). `-c`
//! writes every numeric stats key into a CSV, one row per interval. `-q`
//! prints only the summary.
//!
//! A line ends with `!` and the names of the error counters that grew
//! (`dma_faults`, `mai_underflow_polls`, `mai_overflow_polls`,
//! `pwm_errors`), and with `~` and the names of the warning counters that
//! grew (`underruns`, `long_gaps`, `pwm_gap_polls`). `long_gaps` grows by
//! one at the start of a sound after silence, so it is only a warning.
//!
//! Exit code: 0 when no error counter grew and the rate at the end is
//! within 2 % of `sample_rate`, 1 when an error counter grew, 2 when the
//! rate is off (or missing), 3 when the stats file cannot be read.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};

const STATS: &str = "/scheme/audiohw/stats";

/// Counters whose growth means lost or broken sound.
const ERRORS: &[&str] = &["dma_faults", "mai_underflow_polls", "mai_overflow_polls", "pwm_errors"];
/// Counters whose growth is worth a look but can be harmless.
const WARNINGS: &[&str] = &["underruns", "long_gaps", "pwm_gap_polls"];
/// Counters shown as growth on every line, if the driver has them.
const SHOWN: &[(&str, &str)] = &[
    ("loud_chunks", "loud"),
    ("underruns", "under"),
    ("dma_faults", "dma"),
    ("mai_underflow_polls", "mai_uf"),
    ("pwm_errors", "pwm_err"),
    ("long_gaps", "gaps"),
    ("reprogrammed", "reprog"),
];
/// The allowed rate deviation, as in REQ-AUDIO-4 (#539).
const RATE_TOLERANCE: f64 = 0.02;

/// The numeric stats keys: the first word after the key, if it is a number
/// (decimal, or hex with `0x`). `rate_window_hz 44106 over 9.8 s` gives
/// 44106; `port hdmi0` and `stopped` give nothing.
fn parse(text: &str) -> BTreeMap<String, f64> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let (Some(key), Some(value)) = (words.next(), words.next()) else {
            continue;
        };
        let number = match value.strip_prefix("0x") {
            Some(hex) => u64::from_str_radix(hex, 16).ok().map(|n| n as f64),
            None => value.parse::<f64>().ok(),
        };
        if let Some(n) = number {
            map.insert(key.to_owned(), n);
        }
    }
    map
}

/// The text keys worth one header line (`mode`, `port`).
fn header(text: &str) -> String {
    let mut parts = Vec::new();
    for line in text.lines() {
        for key in ["mode", "port"] {
            if let Some(rest) = line.strip_prefix(key).and_then(|r| r.strip_prefix(' ')) {
                parts.push(format!("{key} {rest}"));
            }
        }
    }
    parts.join(", ")
}

/// How much `key` grew from `old` to `new`; `None` if either lacks it.
/// A counter that went down (driver restarted) counts from zero.
fn growth(old: &BTreeMap<String, f64>, new: &BTreeMap<String, f64>, key: &str) -> Option<f64> {
    let (o, n) = (old.get(key)?, new.get(key)?);
    Some(if n >= o { n - o } else { *n })
}

/// The rate to judge: the recent window if the driver has one, else the
/// rate since start.
fn rate(stats: &BTreeMap<String, f64>) -> Option<f64> {
    stats.get("rate_window_hz").or_else(|| stats.get("measured_rate_hz")).copied()
}

fn rate_ok(stats: &BTreeMap<String, f64>) -> bool {
    match (rate(stats), stats.get("sample_rate")) {
        (Some(r), Some(s)) if *s > 0.0 => ((r - s) / s).abs() <= RATE_TOLERANCE,
        _ => false,
    }
}

/// The counters in `names` that grew.
fn grown<'a>(old: &BTreeMap<String, f64>, new: &BTreeMap<String, f64>, names: &[&'a str]) -> Vec<&'a str> {
    names.iter().copied().filter(|k| growth(old, new, k).is_some_and(|g| g > 0.0)).collect()
}

/// One line for an interval.
fn line(elapsed: f64, old: &BTreeMap<String, f64>, new: &BTreeMap<String, f64>) -> String {
    let mut out = format!("{elapsed:6.1}s");
    if let Some(r) = new.get("measured_rate_hz") {
        out += &format!(" rate {r:.0}");
    }
    if let Some(w) = new.get("rate_window_hz") {
        out += &format!(" win {w:.0}");
    }
    for (key, short) in SHOWN {
        if let Some(g) = growth(old, new, key) {
            out += &format!(" {short} +{g:.0}");
        }
    }
    if let Some(q) = new.get("queued") {
        out += &format!(" queued {q:.0}");
    }
    let errors = grown(old, new, ERRORS);
    if !errors.is_empty() {
        out += &format!(" ! {}", errors.join(","));
    }
    let warnings = grown(old, new, WARNINGS);
    if !warnings.is_empty() {
        out += &format!(" ~ {}", warnings.join(","));
    }
    out
}

/// CSV header and rows: elapsed seconds, then every numeric key of the
/// first sample in key order (keys that appear later are left out, so the
/// columns stay fixed).
fn csv_header(keys: &[String]) -> String {
    let mut out = String::from("elapsed_s");
    for k in keys {
        out.push(',');
        out.push_str(k);
    }
    out
}

fn csv_row(elapsed: f64, keys: &[String], stats: &BTreeMap<String, f64>) -> String {
    let mut out = format!("{elapsed:.1}");
    for k in keys {
        out.push(',');
        if let Some(v) = stats.get(k) {
            out.push_str(&format!("{v}"));
        }
    }
    out
}

/// The summary over the whole run and the exit code.
fn summary(first: &BTreeMap<String, f64>, last: &BTreeMap<String, f64>, seconds: f64) -> (String, i32) {
    let mut out = format!("audiotop: {seconds:.1} s");
    for (key, short) in SHOWN {
        if let Some(g) = growth(first, last, key) {
            out += &format!(", {short} +{g:.0}");
        }
    }
    match (rate(last), last.get("sample_rate")) {
        (Some(r), Some(s)) => out += &format!(", rate {r:.0} of {s:.0} ({:+.2} %)", (r - s) / s * 100.0),
        _ => out += ", no rate",
    }
    let errors = grown(first, last, ERRORS);
    let warnings = grown(first, last, WARNINGS);
    if !warnings.is_empty() {
        out += &format!(", warnings: {}", warnings.join(","));
    }
    let code = if !errors.is_empty() {
        out += &format!(", ERRORS: {}", errors.join(","));
        1
    } else if !rate_ok(last) {
        out += ", RATE OFF";
        2
    } else {
        out += ", ok";
        0
    };
    (out, code)
}

struct Args {
    interval: f64,
    rounds: u64,
    csv: Option<String>,
    file: String,
    quiet: bool,
}

fn usage() -> ! {
    eprintln!("usage: audiotop [-i seconds] [-n rounds] [-c file.csv] [-f stats-file] [-q]");
    process::exit(3);
}

fn args() -> Args {
    let mut a = Args { interval: 1.0, rounds: 0, csv: None, file: STATS.to_owned(), quiet: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-i" => a.interval = it.next().and_then(|v| v.parse().ok()).filter(|v: &f64| *v > 0.0).unwrap_or_else(|| usage()),
            "-n" => a.rounds = it.next().and_then(|v| v.parse().ok()).unwrap_or_else(|| usage()),
            "-c" => a.csv = Some(it.next().unwrap_or_else(|| usage())),
            "-f" => a.file = it.next().unwrap_or_else(|| usage()),
            "-q" => a.quiet = true,
            _ => usage(),
        }
    }
    a
}

fn read(file: &str) -> String {
    fs::read_to_string(file).unwrap_or_else(|e| {
        eprintln!("audiotop: {file}: {e}");
        process::exit(3);
    })
}

fn main() {
    let a = args();
    let start = Instant::now();
    let text = read(&a.file);
    let first = parse(&text);
    let keys: Vec<String> = first.keys().cloned().collect();
    let mut csv = a.csv.as_ref().map(|path| {
        let mut f = fs::File::create(path).unwrap_or_else(|e| {
            eprintln!("audiotop: {path}: {e}");
            process::exit(3);
        });
        let _ = writeln!(f, "{}", csv_header(&keys));
        let _ = writeln!(f, "{}", csv_row(0.0, &keys, &first));
        f
    });
    if !a.quiet {
        println!("audiotop: {} every {} s, {}", a.file, a.interval, header(&text));
    }
    let mut old = first.clone();
    let mut round = 0;
    while a.rounds == 0 || round < a.rounds {
        sleep(Duration::from_secs_f64(a.interval));
        let new = parse(&read(&a.file));
        let elapsed = start.elapsed().as_secs_f64();
        if !a.quiet {
            println!("{}", line(elapsed, &old, &new));
        }
        if let Some(f) = csv.as_mut() {
            let _ = writeln!(f, "{}", csv_row(elapsed, &keys, &new));
        }
        old = new;
        round += 1;
    }
    let (text, code) = summary(&first, &old, start.elapsed().as_secs_f64());
    println!("{text}");
    process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    const HDMI: &str = "mode hdmi-dma\nport hdmi0\nhdmi 0xfef00700\ndreq 10\nsample_rate 44100\n\
        uptime_s 12.5\nmeasured_rate_hz 44097\nrate_window_hz 44106 over 9.8 s\nlong_gaps 1\n\
        loud_chunks 133\nunderruns 0\nqueued 3\ndma_faults 0\nmai_underflow_polls 0\nstopped\n";

    fn with(text: &str, changes: &[(&str, f64)]) -> BTreeMap<String, f64> {
        let mut m = parse(text);
        for (k, v) in changes {
            m.insert((*k).to_owned(), *v);
        }
        m
    }

    #[test]
    fn parses_numbers_hex_and_skips_text() {
        let m = parse(HDMI);
        assert_eq!(m["rate_window_hz"], 44106.0);
        assert_eq!(m["hdmi"], 0xfef0_0700u32 as f64);
        assert_eq!(m["uptime_s"], 12.5);
        assert!(!m.contains_key("port") && !m.contains_key("mode") && !m.contains_key("stopped"));
        assert_eq!(header(HDMI), "mode hdmi-dma, port hdmi0");
    }

    #[test]
    fn growth_and_restart() {
        let a = parse(HDMI);
        let b = with(HDMI, &[("loud_chunks", 392.0)]);
        assert_eq!(growth(&a, &b, "loud_chunks"), Some(259.0));
        assert_eq!(growth(&b, &a, "loud_chunks"), Some(133.0), "a counter that went down counts from zero");
        assert_eq!(growth(&a, &b, "pwm_errors"), None);
    }

    #[test]
    fn line_marks_errors_and_warnings() {
        let a = parse(HDMI);
        let b = with(HDMI, &[("loud_chunks", 392.0), ("long_gaps", 2.0)]);
        let l = line(1.0, &a, &b);
        assert!(l.contains("rate 44097 win 44106 loud +259"), "{l}");
        assert!(l.ends_with("~ long_gaps") && !l.contains('!'), "{l}");
        let c = with(HDMI, &[("dma_faults", 1.0)]);
        assert!(line(2.0, &a, &c).contains("! dma_faults"));
    }

    #[test]
    fn summary_exit_codes() {
        let a = parse(HDMI);
        let (text, code) = summary(&a, &with(HDMI, &[("long_gaps", 2.0)]), 10.0);
        assert_eq!(code, 0, "{text}");
        assert!(text.contains("(+0.01 %)") && text.ends_with("ok"), "{text}");
        assert_eq!(summary(&a, &with(HDMI, &[("mai_underflow_polls", 3.0)]), 10.0).1, 1);
        assert_eq!(summary(&a, &with(HDMI, &[("rate_window_hz", 40000.0)]), 10.0).1, 2);
    }

    #[test]
    fn pwm_driver_without_window() {
        let pwm = "mode pwm-dma\nsample_rate 44100\nmeasured_rate_hz 44500\npwm_errors 0 bits=0x000\nunderruns 0\n";
        let m = parse(pwm);
        assert_eq!(m["pwm_errors"], 0.0);
        assert_eq!(rate(&m), Some(44500.0));
        assert!(rate_ok(&m));
        assert_eq!(summary(&m, &with(pwm, &[("pwm_errors", 1.0)]), 5.0).1, 1);
    }

    #[test]
    fn csv_columns_stay_fixed() {
        let m = parse(HDMI);
        let keys: Vec<String> = m.keys().cloned().collect();
        assert!(csv_header(&keys).starts_with("elapsed_s,dma_faults,dreq,"));
        let mut later = m.clone();
        later.insert("new_key".into(), 1.0);
        later.remove("dreq");
        let row = csv_row(1.0, &keys, &later);
        assert_eq!(row.split(',').count(), keys.len() + 1);
        assert!(row.starts_with("1.0,0,,"), "{row}");
    }
}
