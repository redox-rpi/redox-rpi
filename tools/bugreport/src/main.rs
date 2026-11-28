//! Report a bug from inside the image in one step (#759).
//!
//! ```text
//! bugreport                 ask "What went wrong?", write the report, upload it if set up
//! bugreport <text...>       the same, with the text from the command line
//! bugreport --no-upload     only the file
//! ```
//!
//! The report is one text file, `~/bugreport-<YYYYMMDD-HHMMSS>.txt` (UTC),
//! with the tester's text and the system state from a fixed list of
//! sources: the release file, the board, uptime, memory, processes, the
//! network and the kernel log. Nothing else is read: no file from a home
//! directory, no environment variables, no other config files. MAC
//! addresses are cut to their vendor half. Each source has a time limit, so
//! a hung driver cannot stop the report.
//!
//! With `/etc/bugreport.conf` (experiment `bugreport-upload`, dev images
//! only) the report is also sent to the build host by one HTTP POST, and
//! the receiver (`scripts/bugreport-receiver.py`) answers with an ID such as
//! `BR-0427`. Release images have no such file: the report stays a file.

use std::fs;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CONF: &str = "/etc/bugreport.conf";
const MAX_REPORT: usize = 256 * 1024;
const MAX_SUMMARY: usize = 40;
const SOURCE_TIMEOUT: Duration = Duration::from_secs(10);
const NET_TIMEOUT: Duration = Duration::from_secs(15);

fn main() {
    let mut upload = true;
    let mut words = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--no-upload" => upload = false,
            "-h" | "--help" => {
                println!("usage: bugreport [--no-upload] [text...]");
                println!("Writes ~/bugreport-<date>-<time>.txt with your text and the system state,");
                println!("and sends it to the build host where that is set up (dev images).");
                return;
            }
            _ => words.push(arg),
        }
    }
    let text = if words.is_empty() { ask() } else { words.join(" ") };

    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let report = build_report(&text, now);

    let dir = home_dir();
    let path = dir.join(format!("bugreport-{}.txt", stamp(now, true)));
    if let Err(e) = fs::write(&path, &report) {
        eprintln!("bugreport: cannot write {}: {e}", path.display());
        process::exit(1);
    }
    println!("Report written to {}", path.display());

    if !upload {
        return;
    }
    let Some(url) = upload_url() else {
        println!("Attach this file to an issue: https://github.com/redox-rpi/redox-rpi/issues/new");
        return;
    };
    print!("Sending it to the build host ... ");
    let _ = io::stdout().flush();
    match send(&url, &report) {
        Ok(id) => {
            println!("done.");
            println!("Report {id} sent. Please pass on this ID: {id}");
        }
        Err(e) => {
            println!("failed: {e}");
            println!("The file is still there; it is lost at reboot on a RAM root.");
            process::exit(2);
        }
    }
}

/// One line from the tester; on a terminal with a prompt.
fn ask() -> String {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        print!("What went wrong? (one line, then Enter): ");
        let _ = io::stdout().flush();
    }
    let mut line = String::new();
    let _ = stdin.lock().read_line(&mut line);
    let line = line.trim();
    if line.is_empty() { "(no text given)".into() } else { line.into() }
}

fn home_dir() -> PathBuf {
    // HOME only for the place of the report; no other variable is read.
    match std::env::var_os("HOME") {
        Some(h) if PathBuf::from(&h).is_dir() => PathBuf::from(h),
        _ => PathBuf::from("/tmp"),
    }
}

fn build_report(text: &str, now: u64) -> String {
    let log = read_file("/scheme/sys/log");
    let mut out = String::new();
    let mut section = |title: &str, body: &str| {
        out.push_str(&format!("== {title}\n"));
        out.push_str(body.trim_end());
        out.push_str("\n\n");
    };
    section("What went wrong", text);
    section("Report", &format!("written {} UTC by bugreport {}", stamp(now, false), env!("CARGO_PKG_VERSION")));
    section("Release", &read_file("/etc/redox-rpi-release"));
    section("Board", &board());
    section("Kernel", &read_file("/scheme/sys/uname").replace('\n', " "));
    section("Uptime", &run("uptime", &[]));
    section("Memory", &run("free", &[]));
    section("Summary of the kernel log", &summary(&log));
    section("Network", &network());
    section("Processes", &run("ps", &[]));
    let head = mask_macs(&out);

    // The kernel log comes last and loses its oldest lines if the report
    // would be too long.
    let mut log = mask_macs(&clean(&log));
    let room = MAX_REPORT.saturating_sub(head.len() + 64);
    if log.len() > room {
        let mut cut = log.len() - room;
        while !log.is_char_boundary(cut) {
            cut += 1;
        }
        let cut = log[cut..].find('\n').map(|i| cut + i + 1).unwrap_or(cut);
        log = format!("(older lines left out)\n{}", &log[cut..]);
    }
    format!("{head}== Kernel log (/scheme/sys/log)\n{}\n", log.trim_end())
}

/// Reads a file in a thread, so a scheme that never answers costs at most
/// SOURCE_TIMEOUT.
fn read_file(path: &'static str) -> String {
    with_timeout(move || match fs::read(path) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(e) => format!("not available ({e})"),
    })
}

fn read_bytes(path: &'static str) -> Option<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(fs::read(path).ok());
    });
    rx.recv_timeout(SOURCE_TIMEOUT).ok().flatten()
}

fn with_timeout(f: impl FnOnce() -> String + Send + 'static) -> String {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(SOURCE_TIMEOUT).unwrap_or_else(|_| "not available (no answer in 10 s)".into())
}

/// Runs a command with no input and a time limit; stdout and stderr.
fn run(cmd: &str, args: &[&str]) -> String {
    let child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return format!("not available ({cmd}: {e})"),
    };
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = stdout.as_mut() {
            let mut b = Vec::new();
            let _ = o.read_to_end(&mut b);
            s.push_str(&String::from_utf8_lossy(&b));
        }
        if let Some(e) = stderr.as_mut() {
            let mut b = Vec::new();
            let _ = e.read_to_end(&mut b);
            s.push_str(&String::from_utf8_lossy(&b));
        }
        let _ = tx.send(s);
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < SOURCE_TIMEOUT => thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return format!("not available ({cmd} gave no answer in 10 s)");
            }
        }
    }
    clean(&rx.recv_timeout(Duration::from_secs(2)).unwrap_or_default())
}

/// The root node's `model` from the flattened devicetree.
fn board() -> String {
    let Some(dtb) = read_bytes("/scheme/kernel.dtb") else {
        return "not available (/scheme/kernel.dtb)".into();
    };
    dt_model(&dtb).unwrap_or_else(|| "not available (no model in the devicetree)".into())
}

fn dt_model(dtb: &[u8]) -> Option<String> {
    let be = |o: usize| -> Option<usize> {
        Some(u32::from_be_bytes(dtb.get(o..o + 4)?.try_into().ok()?) as usize)
    };
    if be(0)? != 0xd00d_feed {
        return None;
    }
    let (structs, strings) = (be(8)?, be(12)?);
    let mut p = structs;
    let mut depth = 0usize;
    loop {
        let token = be(p)?;
        p += 4;
        match token {
            1 => {
                // BEGIN_NODE: name, NUL, padded to 4.
                let end = dtb.get(p..)?.iter().position(|&b| b == 0)?;
                p += (end + 4) & !3;
                depth += 1;
            }
            2 => depth = depth.checked_sub(1)?,
            3 => {
                let (len, nameoff) = (be(p)?, be(p + 4)?);
                let value = dtb.get(p + 8..p + 8 + len)?;
                p += 8 + ((len + 3) & !3);
                let name = dtb.get(strings + nameoff..)?;
                let name = &name[..name.iter().position(|&b| b == 0)?];
                if depth == 1 && name == b"model" {
                    let v = value.split(|&b| b == 0).next()?;
                    return Some(String::from_utf8_lossy(v).into_owned());
                }
            }
            4 => {}
            _ => return None, // END or garbage
        }
    }
}

fn network() -> String {
    let mut out = String::new();
    let ifaces = list_dir("/scheme/netcfg/ifaces");
    if ifaces.is_empty() {
        out.push_str("no interfaces in /scheme/netcfg/ifaces\n");
    }
    for name in ifaces {
        let base = format!("/scheme/netcfg/ifaces/{name}");
        let get = |f: &str| {
            let p = format!("{base}/{f}");
            with_timeout(move || fs::read_to_string(&p).map(|s| s.trim().replace('\n', ", ")).unwrap_or_else(|_| "-".into()))
        };
        out.push_str(&format!("{name}: mac {} link {} addr {}\n", get("mac"), get("link"), get("addr/list")));
    }
    out.push_str("routes:\n");
    out.push_str(read_file("/scheme/netcfg/route/list").trim_end());
    out.push('\n');
    out.push_str(&format!("name server: {}\n", read_file("/scheme/netcfg/resolv/nameserver").trim()));
    let drivers: Vec<String> = list_dir("/scheme").into_iter().filter(|s| s.starts_with("network.")).collect();
    out.push_str(&format!("drivers: {}\n", if drivers.is_empty() { "none".into() } else { drivers.join(" ") }));
    out
}

fn list_dir(path: &'static str) -> Vec<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut v: Vec<String> = fs::read_dir(path)
            .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default();
        v.sort();
        let _ = tx.send(v);
    });
    rx.recv_timeout(SOURCE_TIMEOUT).unwrap_or_default()
}

/// Problem lines and USB lines from the kernel log, so the reader sees them first.
fn summary(log: &str) -> String {
    if log.starts_with("not available") {
        return format!("kernel log {log}");
    }
    let log = clean(log);
    let pick = |want: &dyn Fn(&str) -> bool| -> Vec<&str> {
        let v: Vec<&str> = log.lines().filter(|l| want(&l.to_lowercase())).collect();
        v[v.len().saturating_sub(MAX_SUMMARY)..].to_vec()
    };
    let problems = pick(&|l| {
        ["panic", "out of memory", "unhandled exception", "error", "crash", "segfault", "page fault", "killed"]
            .iter()
            .any(|w| l.contains(w))
    });
    let usb = pick(&|l| l.contains("dwc2d") || l.contains("usb"));
    let mut out = format!("problem lines ({}):\n", problems.len());
    for l in &problems {
        out.push_str(&format!("  {l}\n"));
    }
    out.push_str(&format!("USB lines ({}):\n", usb.len()));
    for l in &usb {
        out.push_str(&format!("  {l}\n"));
    }
    out
}

/// Without ANSI and OSC escape sequences and carriage returns.
fn clean(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => match chars.next() {
                Some('[') => {
                    while let Some(&n) = chars.peek() {
                        chars.next();
                        if ('@'..='~').contains(&n) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(n) = chars.next() {
                        if n == '\x07' || (n == '\x1b' && chars.peek() == Some(&'\\')) {
                            if n == '\x1b' {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

/// `aa:bb:cc:dd:ee:ff` becomes `aa:bb:cc:xx:xx:xx`; the same for the
/// `aa-bb-cc-dd-ee-ff` form that `/scheme/netcfg/ifaces/<if>/mac` has.
fn mask_macs(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let hex = |c: u8| c.is_ascii_hexdigit();
    let joins = |c: u8| hex(c) || c == b':' || c == b'-';
    while i < b.len() {
        let sep = if i + 2 < b.len() { b[i + 2] } else { 0 };
        let is_mac = (sep == b':' || sep == b'-')
            && i + 17 <= b.len()
            && (0..6).all(|k| hex(b[i + 3 * k]) && hex(b[i + 3 * k + 1]))
            && (0..5).all(|k| b[i + 3 * k + 2] == sep)
            && (i == 0 || !joins(b[i - 1]))
            && (i + 17 == b.len() || !joins(b[i + 17]));
        if is_mac {
            out.extend_from_slice(&b[i..i + 9]);
            out.extend_from_slice(&[b'x', b'x', sep, b'x', b'x', sep, b'x', b'x']);
            i += 17;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// `upload = http://host:port/path` from /etc/bugreport.conf.
fn upload_url() -> Option<String> {
    let conf = fs::read_to_string(CONF).ok()?;
    conf.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .find_map(|l| l.strip_prefix("upload")?.trim_start().strip_prefix('=').map(|v| v.trim().to_string()))
        .filter(|v| v.starts_with("http://"))
}

/// One HTTP/1.0 POST; the answer's body is the ID.
fn send(url: &str, report: &str) -> Result<String, String> {
    let rest = url.strip_prefix("http://").ok_or("only http:// is supported")?;
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let hostport = if hostport.contains(':') { hostport.to_string() } else { format!("{hostport}:80") };
    let addr = hostport
        .to_socket_addrs()
        .map_err(|e| format!("{hostport}: {e}"))?
        .next()
        .ok_or_else(|| format!("{hostport}: no address"))?;
    let mut s = TcpStream::connect_timeout(&addr, NET_TIMEOUT).map_err(|e| format!("{hostport}: {e}"))?;
    let _ = s.set_read_timeout(Some(NET_TIMEOUT));
    let _ = s.set_write_timeout(Some(NET_TIMEOUT));
    let host = hostport.rsplit_once(':').map(|(h, _)| h).unwrap_or(&hostport);
    let head = format!(
        "POST {path} HTTP/1.0\r\nHost: {host}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        report.len()
    );
    s.write_all(head.as_bytes()).and_then(|_| s.write_all(report.as_bytes())).map_err(|e| format!("sending: {e}"))?;
    let mut answer = Vec::new();
    s.read_to_end(&mut answer).map_err(|e| format!("reading the answer: {e}"))?;
    let answer = String::from_utf8_lossy(&answer);
    let (status, body) = answer.split_once("\r\n\r\n").unwrap_or((&answer, ""));
    let code = status.split_whitespace().nth(1).unwrap_or("?");
    let body = body.trim();
    if code != "200" {
        return Err(format!("the build host answered {code} {}", body.lines().next().unwrap_or("")));
    }
    let id = body.lines().next().unwrap_or("").trim();
    if id.starts_with("BR-") { Ok(id.to_string()) } else { Err(format!("unexpected answer: {id}")) }
}

/// `YYYYMMDD-HHMMSS` (for the file name) or `YYYY-MM-DD HH:MM:SS`, UTC.
fn stamp(secs: u64, compact: bool) -> String {
    let (days, rem) = ((secs / 86400) as i64, secs % 86400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let (hh, mm, ss) = (rem / 3600, rem % 3600 / 60, rem % 60);
    if compact {
        format!("{y:04}{m:02}{d:02}-{hh:02}{mm:02}{ss:02}")
    } else {
        format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macs_are_cut() {
        assert_eq!(mask_macs("mac b8:27:eb:12:34:56 up"), "mac b8:27:eb:xx:xx:xx up");
        assert_eq!(mask_macs("b8:27:eb:12:34:56"), "b8:27:eb:xx:xx:xx");
        // Not a MAC: an IPv6 address and a time.
        assert_eq!(mask_macs("fe80::1 12:34:56"), "fe80::1 12:34:56");
        assert_eq!(mask_macs("aa:b8:27:eb:12:34:56"), "aa:b8:27:eb:12:34:56");
        // The netcfg form, with dashes (Test 680).
        assert_eq!(mask_macs("eth0: mac B8-27-EB-12-34-56 link up"), "eth0: mac B8-27-EB-xx-xx-xx link up");
        // Mixed separators are not a MAC; a date with dashes is not one either.
        assert_eq!(mask_macs("b8:27-eb:12:34:56"), "b8:27-eb:12:34:56");
        assert_eq!(mask_macs("2026-11-27"), "2026-11-27");
    }

    #[test]
    fn escapes_go() {
        assert_eq!(clean("\x1b[1mredox\x1b[0m\r\n\x1b]0;root\x07x"), "redox\nx");
    }

    #[test]
    fn dates() {
        assert_eq!(stamp(0, false), "1970-01-01 00:00:00");
        assert_eq!(stamp(1_795_779_000, true), "20261127-113000");
        assert_eq!(stamp(951_782_400, false), "2000-02-29 00:00:00");
    }

    #[test]
    fn devicetree_model() {
        // A minimal FDT: root node with model = "Raspberry Pi 3 Model B Plus".
        let model = b"Raspberry Pi 3 Model B Plus\0";
        let mut st = Vec::new();
        st.extend_from_slice(&1u32.to_be_bytes());
        st.extend_from_slice(&[0, 0, 0, 0]); // root name "" padded
        st.extend_from_slice(&3u32.to_be_bytes());
        st.extend_from_slice(&(model.len() as u32).to_be_bytes());
        st.extend_from_slice(&0u32.to_be_bytes());
        st.extend_from_slice(model);
        while st.len() % 4 != 0 {
            st.push(0);
        }
        st.extend_from_slice(&2u32.to_be_bytes());
        st.extend_from_slice(&9u32.to_be_bytes());
        let strings = b"model\0";
        let mut dtb = vec![0u8; 40];
        dtb[0..4].copy_from_slice(&0xd00d_feedu32.to_be_bytes());
        dtb[8..12].copy_from_slice(&40u32.to_be_bytes());
        dtb[12..16].copy_from_slice(&((40 + st.len()) as u32).to_be_bytes());
        dtb.extend_from_slice(&st);
        dtb.extend_from_slice(strings);
        assert_eq!(dt_model(&dtb).as_deref(), Some("Raspberry Pi 3 Model B Plus"));
    }

    #[test]
    fn summary_picks_lines() {
        let s = summary("ok\nDHCP: default route via 10.0.2.2\nKERNEL PANIC: x\ndwc2d: device on root port\n");
        assert!(s.contains("problem lines (1)") && s.contains("KERNEL PANIC"));
        assert!(s.contains("USB lines (1)"));
    }
}
