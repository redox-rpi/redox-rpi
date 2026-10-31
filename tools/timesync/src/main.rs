//! timesync: set the Redox system clock from an NTP server.
//!
//! The Raspberry Pi has no real-time clock, so Redox starts at 1970. At start
//! this moves the clock forward to the newest of two times known without a
//! network, like Linux's fake-hwclock: the image's build time (`BUILD=` in
//! `/etc/redox-rpi-release`) and the time saved by an earlier run. Then it
//! asks NTP servers (SNTP, RFC 4330) for the time and saves it.
//!
//! The clock is set with `clock_settime`; with a relibc that does not have it
//! yet (ENOSYS), by writing the boot time, in nanoseconds since 1970, to the
//! kernel's `/scheme/sys/update_time_offset`, which is what it does. Needs root.
//!
//! Usage: `timesync [-n | -d] [server]...` (default: pool.ntp.org time.cloudflare.com).
//! `-n` only prints the time without setting the clock.
//! `-d` keeps running: asks the servers again and saves the time every hour.
//! `timesync -s 'YYYY-MM-DD HH:MM[:SS]'` (UTC) or `-s @<seconds since 1970>`
//! sets the clock by hand, for a Pi without a network (Redox' `date` cannot
//! set it) and saves it.
//! A server is a name, an IPv4 address or `address:port`.
//!
//! Each answer is checked: it must come from the address asked, carry the
//! request's transmit time as its origin time, and come from a synchronized
//! server (leap indicator not 3, stratum 1 to 15). A kiss-of-death (stratum
//! 0) is reported with its code; after RATE that address is not asked again
//! in this sync, after DENY or RSTR never again. It asks up to three addresses (a name such as `pool.ntp.org` has
//! several) and uses the median offset, so one wrong server cannot move the
//! clock. After the first sync it keeps the clock when the offset is below
//! 50 ms, and refuses a jump of more than an hour unless two answers agree.
//! Waits until DHCP has set a default route, then asks the servers until one
//! answers, for up to a minute. With `-d` it tries again every 30
//! seconds until the first answer: Redox' DNS lookup has no timeout, so a
//! lookup before the network is up would wait forever.

use std::fs;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::process;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

const DEFAULT_SERVERS: &[&str] = &["pool.ntp.org", "time.cloudflare.com"];
const TIME_OFFSET: &str = "/scheme/sys/update_time_offset";
/// Written after the first answer from a server, for `timerd`. /tmp is new
/// at each boot.
const SYNCED: &str = "/tmp/timesync.synced";
const RELEASE: &str = "/etc/redox-rpi-release";
/// Where the time is saved: the SD card's data partition (network boot,
/// where the root filesystem is lost at reboot), else the root filesystem.
const SAVED: &[&str] = &["/scheme/data/timesync.last", "/var/lib/timesync.last"];
/// Seconds from 1900 (NTP era 0) to 1970.
const NTP_TO_UNIX: u64 = 2_208_988_800;
/// Answers before this (2026-01-01) are wrong.
const EARLIEST: u64 = 1_767_225_600;
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);
const GIVE_UP_AFTER: Duration = Duration::from_secs(60);
const RESYNC_EVERY: Duration = Duration::from_secs(3600);
/// With `-d`, until the first answer.
const RETRY_EVERY: Duration = Duration::from_secs(30);
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(5);
/// Lookups that never returned; no new ones beyond this.
const MAX_STUCK_LOOKUPS: usize = 4;
static PENDING_LOOKUPS: AtomicUsize = AtomicUsize::new(0);
/// Answers used for one sync.
const SAMPLES: usize = 3;
/// Answers this close to the median agree with it.
const AGREE_NS: i128 = 1_000_000_000;
/// After the first sync, offsets below this leave the clock alone.
const KEEP_BELOW_NS: i128 = 50_000_000;
/// After the first sync, a jump beyond this needs two answers that agree.
const MAX_LONE_JUMP_NS: i128 = 3600 * 1_000_000_000;
/// Whether a sync has set the clock since start.
static SYNCED_ONCE: AtomicBool = AtomicBool::new(false);
/// Addresses that sent a kiss-of-death DENY or RSTR.
static BANNED: Mutex<Vec<SocketAddr>> = Mutex::new(Vec::new());
/// Routes set by DHCP (`default via …`), from smolnetd.
const ROUTES: &str = "/scheme/netcfg/route/list";

fn clock_ns(clock: libc::clockid_t) -> u128 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(clock, &mut ts) };
    ts.tv_sec as u128 * 1_000_000_000 + ts.tv_nsec as u128
}

/// Nanoseconds of the kernel's monotonic clock, the base of its real time.
fn monotonic_ns() -> u128 {
    clock_ns(libc::CLOCK_MONOTONIC)
}

fn realtime_secs() -> u64 {
    (clock_ns(libc::CLOCK_REALTIME) / 1_000_000_000) as u64
}

/// Set the clock so that it showed `unix_ns` at monotonic time `at_monotonic`.
/// Sets the clock to `unix_ns`, the time at monotonic time `at_monotonic`.
fn set_clock(unix_ns: u128, at_monotonic: u128) -> io::Result<()> {
    let now = unix_ns + monotonic_ns().saturating_sub(at_monotonic);
    let ts = libc::timespec {
        tv_sec: (now / 1_000_000_000) as libc::time_t,
        tv_nsec: (now % 1_000_000_000) as libc::c_long,
    };
    if unsafe { libc::clock_settime(libc::CLOCK_REALTIME, &ts) } == 0 {
        return Ok(());
    }
    let err = io::Error::last_os_error();
    if err.raw_os_error() != Some(libc::ENOSYS) {
        return Err(err);
    }
    // An older relibc: the kernel's real time is boot time plus monotonic time.
    let boot = unix_ns - at_monotonic;
    fs::write(TIME_OFFSET, boot.to_ne_bytes())
}

/// `timesync -s`: `YYYY-MM-DD HH:MM[:SS]` or `YYYY-MM-DDTHH:MM[:SS]` in
/// UTC, or `@<seconds since 1970>`.
fn parse_time_arg(text: &str) -> Option<u64> {
    if let Some(secs) = text.strip_prefix('@') {
        return secs.parse().ok();
    }
    let (date, time) = text.trim().split_once([' ', 'T'])?;
    let time = if time.len() == 5 { format!("{time}:00") } else { time.to_string() };
    let num = |s: &str, range: std::ops::Range<usize>| s.get(range)?.parse::<u64>().ok();
    let (month, day) = (num(date, 5..7)?, num(date, 8..10)?);
    let (hour, minute, second) = (num(&time, 0..2)?, num(&time, 3..5)?, num(&time, 6..8)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    parse_utc(&format!("{date}T{time}Z"))
}

/// `timesync -s`: set the clock by hand.
fn set_by_hand(text: &str) -> ! {
    let Some(unix) = parse_time_arg(text) else {
        eprintln!("timesync: -s wants 'YYYY-MM-DD HH:MM[:SS]' (UTC) or @<seconds>, not {text:?}");
        process::exit(2);
    };
    match set_clock(u128::from(unix) * 1_000_000_000, monotonic_ns()) {
        Ok(()) => {
            println!("timesync: clock set to {} UTC by hand", format_utc(unix));
            save_time();
            process::exit(0);
        }
        Err(err) => {
            eprintln!("timesync: cannot set the clock: {err}");
            process::exit(1);
        }
    }
}

/// The server's IPv4 addresses. The lookup runs in its own thread, so a
/// lookup that never returns does not hang timesync.
fn resolve(server: &str) -> io::Result<Vec<SocketAddr>> {
    if let Ok(ip) = server.parse::<Ipv4Addr>() {
        return Ok(vec![(ip, 123).into()]);
    }
    if let Ok(addr) = server.parse::<SocketAddr>() {
        return Ok(vec![addr]);
    }
    if PENDING_LOOKUPS.load(Ordering::SeqCst) >= MAX_STUCK_LOOKUPS {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "earlier DNS lookups still hang"));
    }
    PENDING_LOOKUPS.fetch_add(1, Ordering::SeqCst);
    let (tx, rx) = mpsc::channel();
    let name = server.to_string();
    thread::spawn(move || {
        let addr = (name.as_str(), 123).to_socket_addrs().map(|a| a.filter(SocketAddr::is_ipv4).collect::<Vec<_>>());
        PENDING_LOOKUPS.fetch_sub(1, Ordering::SeqCst);
        let _ = tx.send(addr);
    });
    match rx.recv_timeout(LOOKUP_TIMEOUT) {
        Ok(Ok(addrs)) if !addrs.is_empty() => Ok(addrs),
        Ok(Ok(_)) => Err(io::Error::new(io::ErrorKind::NotFound, "no IPv4 address")),
        Ok(Err(err)) => Err(err),
        Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "DNS lookup timed out")),
    }
}

/// Whether there is a default route, that is, DHCP is done. True where
/// smolnetd's route list cannot be read (other systems).
fn network_up() -> bool {
    fs::read_to_string(ROUTES).map_or(true, |routes| routes.contains("default"))
}

/// Why an answer is not used.
#[derive(Debug, PartialEq)]
enum Refused {
    /// Kiss-of-death (stratum 0) with its four-letter code.
    Kiss(String),
    Bad(&'static str),
}

/// Check a server's answer to a request whose transmit time was `request`.
/// Returns the server's transmit time, seconds and fraction since 1900.
fn check_answer(packet: &[u8], request: &[u8; 8]) -> Result<(u64, u64), Refused> {
    if packet.len() < 48 || packet[0] & 7 != 4 {
        return Err(Refused::Bad("not a server answer"));
    }
    if packet[24..32] != request[..] {
        return Err(Refused::Bad("answer does not match the request"));
    }
    if packet[1] == 0 {
        let code = packet[12..16].iter().map(|&b| if b.is_ascii_graphic() { b as char } else { '?' });
        return Err(Refused::Kiss(code.collect()));
    }
    if packet[0] >> 6 == 3 {
        return Err(Refused::Bad("server not synchronized (leap indicator 3)"));
    }
    if packet[1] > 15 {
        return Err(Refused::Bad("server not synchronized (stratum 16 or more)"));
    }
    let seconds = u32::from_be_bytes(packet[40..44].try_into().unwrap()) as u64;
    let fraction = u32::from_be_bytes(packet[44..48].try_into().unwrap()) as u64;
    if seconds == 0 && fraction == 0 {
        return Err(Refused::Bad("no transmit time"));
    }
    Ok((seconds, fraction))
}

/// Ask one address. Returns the server's time minus the local clock, in
/// nanoseconds.
fn query(addr: SocketAddr) -> io::Result<i128> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(QUERY_TIMEOUT))?;
    // Leap indicator 0, version 4, mode 3 (client). The transmit time is
    // the local clock with the monotonic nanoseconds as the low bits, so an
    // answer to another request does not match.
    let mut request = [0u8; 8];
    let ntp_secs = (realtime_secs() + NTP_TO_UNIX) as u32;
    request[..4].copy_from_slice(&ntp_secs.to_be_bytes());
    request[4..].copy_from_slice(&(monotonic_ns() as u32).to_be_bytes());
    let mut packet = [0u8; 48];
    packet[0] = 0x23;
    packet[40..48].copy_from_slice(&request);
    let sent = Instant::now();
    socket.send_to(&packet, addr)?;
    let mut answer = [0u8; 68];
    let len = loop {
        let (len, from) = socket.recv_from(&mut answer)?;
        if from == addr {
            break len;
        }
        if sent.elapsed() > QUERY_TIMEOUT {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "no answer from the address asked"));
        }
    };
    let local = clock_ns(libc::CLOCK_REALTIME) as i128;
    let round_trip = sent.elapsed();
    let (seconds, fraction) = match check_answer(&answer[..len], &request) {
        Ok(time) => time,
        Err(Refused::Kiss(code)) => {
            if code == "DENY" || code == "RSTR" {
                BANNED.lock().unwrap().push(addr);
            }
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, format!("kiss-of-death {code}")));
        }
        Err(Refused::Bad(why)) => return Err(io::Error::new(io::ErrorKind::InvalidData, why)),
    };
    let unix = seconds.wrapping_sub(NTP_TO_UNIX);
    if unix < EARLIEST {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("implausible time {unix}")));
    }
    // The server stamped its answer about half a round trip ago.
    let ns = unix as i128 * 1_000_000_000 + ((fraction as i128 * 1_000_000_000) >> 32) + round_trip.as_nanos() as i128 / 2;
    Ok(ns - local)
}

/// The offset to use from the answers' offsets. `first`: no sync has set the
/// clock yet, so a single answer is enough for any jump.
fn choose(offsets: &[i128], first: bool) -> Result<i128, String> {
    let mut sorted = offsets.to_vec();
    sorted.sort_unstable();
    let offset = match sorted[..] {
        [] => return Err("no answer".into()),
        [a, b] if b - a > AGREE_NS => {
            return Err(format!("two answers {:.0} s apart, waiting for a third", (b - a) as f64 / 1e9));
        }
        [a, b] => a + (b - a) / 2,
        _ => sorted[sorted.len() / 2],
    };
    let agree = sorted.iter().filter(|o| (*o - offset).abs() <= AGREE_NS).count();
    if !first && offset.abs() > MAX_LONE_JUMP_NS && agree < 2 {
        return Err(format!("one answer wants a jump of {}; it needs two that agree", seconds(offset)));
    }
    Ok(offset)
}

/// An offset in seconds with a sign, such as "+0.012 s".
fn seconds(ns: i128) -> String {
    format!("{:+.3} s", ns as f64 / 1e9)
}

/// "YYYY-MM-DD HH:MM:SS" in UTC (days to civil date after Howard Hinnant).
fn format_utc(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// Unix time of "YYYY-MM-DDTHH:MM:SSZ" (civil date to days, Howard Hinnant).
fn parse_utc(s: &str) -> Option<u64> {
    let num = |range: std::ops::Range<usize>| s.get(range)?.parse::<i64>().ok();
    if s.len() != 20 || !s.ends_with('Z') {
        return None;
    }
    let (year, month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hour, minute, second) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hour * 3600 + minute * 60 + second).ok()
}

/// The image's build time from `BUILD="…"` in the release file.
fn build_time() -> Option<u64> {
    let release = fs::read_to_string(RELEASE).ok()?;
    let line = release.lines().find_map(|l| l.strip_prefix("BUILD="))?;
    parse_utc(line.trim_matches('"'))
}

/// The newest time saved by an earlier run.
fn saved_time() -> Option<u64> {
    SAVED
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok()?.trim().parse::<u64>().ok())
        .max()
}

/// Save the current time where the next boot finds it.
fn save_time() {
    let line = format!("{}\n", realtime_secs());
    for path in SAVED {
        if let Some(dir) = std::path::Path::new(path).parent() {
            let _ = fs::create_dir_all(dir);
        }
        if fs::write(path, &line).is_ok() {
            return;
        }
    }
    eprintln!("timesync: cannot save the time to {}", SAVED.join(" or "));
}

/// Move the clock forward to the build time or the saved time, whichever is
/// newer, if it is behind both.
fn restore() {
    let candidates = [(build_time(), "the build time"), (saved_time(), "the saved time")];
    let Some((floor, source)) = candidates
        .iter()
        .filter_map(|(time, source)| Some((time.as_ref().copied()?, *source)))
        .max_by_key(|(time, _)| *time)
    else {
        return;
    };
    if realtime_secs() >= floor {
        return;
    }
    match set_clock(floor as u128 * 1_000_000_000, monotonic_ns()) {
        Ok(()) => println!("timesync: clock set to {} UTC from {source}", format_utc(floor)),
        Err(err) => eprintln!("timesync: cannot set the clock through {TIME_OFFSET}: {err}"),
    }
}

/// Print a message once per sync.
fn report(reported: &mut Vec<String>, message: String) {
    if !reported.contains(&message) {
        eprintln!("timesync: {message}");
        reported.push(message);
    }
}

/// Answers from up to three addresses of the servers, as (offset, label).
/// Addresses in `skip` (kiss-of-death RATE in this sync) are not asked.
fn samples(
    servers: &[&str],
    skip: &mut Vec<SocketAddr>,
    reported: &mut Vec<String>,
    last_error: &mut String,
) -> Vec<(i128, String)> {
    let mut found = Vec::new();
    for server in servers {
        let addrs = match resolve(server) {
            Ok(addrs) => addrs,
            Err(err) => {
                *last_error = format!("{server}: {err}");
                continue;
            }
        };
        for addr in addrs {
            if found.len() == SAMPLES {
                return found;
            }
            if skip.contains(&addr) || BANNED.lock().unwrap().contains(&addr) {
                continue;
            }
            match query(addr) {
                Ok(offset) => found.push((offset, format!("{server} ({})", addr.ip()))),
                Err(err) => {
                    if err.to_string() == "kiss-of-death RATE" {
                        skip.push(addr);
                    }
                    report(reported, format!("{server} ({addr}): {err}"));
                    *last_error = format!("{server}: {err}");
                }
            }
        }
    }
    found
}

/// Ask the servers, for up to a minute. Returns the time it set.
fn sync(servers: &[&str], dry_run: bool) -> Result<u64, String> {
    let start = Instant::now();
    let mut last_error = String::from("no default route (DHCP not done)");
    let (mut skip, mut reported) = (Vec::new(), Vec::new());
    while start.elapsed() < GIVE_UP_AFTER {
        if !network_up() {
            sleep(Duration::from_secs(1));
            continue;
        }
        let found = samples(servers, &mut skip, &mut reported, &mut last_error);
        if !found.is_empty() {
            let offsets: Vec<i128> = found.iter().map(|(o, _)| *o).collect();
            let first = !SYNCED_ONCE.load(Ordering::SeqCst);
            match choose(&offsets, first) {
                Ok(offset) => {
                    let detail: Vec<String> = found.iter().map(|(o, from)| format!("{from} {}", seconds(*o))).collect();
                    let unix_ns = (clock_ns(libc::CLOCK_REALTIME) as i128 + offset) as u128;
                    let unix = (unix_ns / 1_000_000_000) as u64;
                    let detail = detail.join(", ");
                    if dry_run {
                        println!("timesync: {} UTC, offset {} (clock not set; {detail})", format_utc(unix), seconds(offset));
                        return Ok(unix);
                    }
                    if !first && offset.abs() < KEEP_BELOW_NS {
                        println!("timesync: offset {}, clock kept ({detail})", seconds(offset));
                        return Ok(unix);
                    }
                    if let Err(err) = set_clock(unix_ns, monotonic_ns()) {
                        return Err(format!("cannot set the clock through {TIME_OFFSET}: {err}"));
                    }
                    println!("timesync: clock set to {} UTC, offset {} ({detail})", format_utc(unix), seconds(offset));
                    SYNCED_ONCE.store(true, Ordering::SeqCst);
                    let _ = fs::write(SYNCED, format!("{unix}\n"));
                    return Ok(unix);
                }
                Err(err) => {
                    report(&mut reported, err.clone());
                    last_error = err;
                }
            }
        }
        sleep(Duration::from_secs(2));
    }
    Err(format!("no time server answered within a minute (last: {last_error})"))
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("-s") {
        match args.get(1) {
            Some(text) if args.len() == 2 => set_by_hand(text),
            _ => {
                eprintln!("usage: timesync -s 'YYYY-MM-DD HH:MM[:SS]' | timesync -s @<seconds>");
                process::exit(2);
            }
        }
    }
    let flag = args.first().filter(|a| *a == "-n" || *a == "-d").cloned();
    if flag.is_some() {
        args.remove(0);
    }
    let dry_run = flag.as_deref() == Some("-n");
    let daemon = flag.as_deref() == Some("-d");
    let servers: Vec<&str> = if args.is_empty() {
        DEFAULT_SERVERS.to_vec()
    } else {
        args.iter().map(String::as_str).collect()
    };

    if !dry_run {
        restore();
    }
    let mut result = sync(&servers, dry_run);
    if let Err(err) = &result {
        eprintln!("timesync: {err}");
    }
    if daemon && result.is_err() {
        eprintln!("timesync: trying again every {} s", RETRY_EVERY.as_secs());
        save_time();
        while result.is_err() {
            sleep(RETRY_EVERY);
            result = sync(&servers, false);
        }
    }
    if dry_run {
        process::exit(i32::from(result.is_err()));
    }
    if !daemon {
        if result.is_ok() {
            save_time();
        }
        process::exit(i32::from(result.is_err()));
    }
    // A clock that only came from the build or saved time is saved too, so
    // the next boot starts at least where this one ended.
    loop {
        save_time();
        sleep(RESYNC_EVERY);
        if let Err(err) = sync(&servers, false) {
            eprintln!("timesync: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_arg() {
        assert_eq!(parse_time_arg("1970-01-01 00:00"), Some(0));
        assert_eq!(parse_time_arg("2026-07-01 12:00:00"), Some(1_782_907_200));
        assert_eq!(parse_time_arg("2026-10-29T14:05:30"), Some(1_793_282_730));
        assert_eq!(parse_time_arg("@1793282730"), Some(1_793_282_730));
        assert_eq!(parse_time_arg("2024-02-29 00:00"), Some(1_709_164_800));
        assert_eq!(parse_time_arg("2026-13-01 00:00"), None);
        assert_eq!(parse_time_arg("2026-10-29"), None);
        assert_eq!(parse_time_arg("2026-10-29 9:00"), None);
        assert_eq!(format_utc(parse_time_arg("2026-12-31 23:59:59").unwrap()), "2026-12-31 23:59:59");
    }

    fn answer(li_vn_mode: u8, stratum: u8, request: &[u8; 8]) -> [u8; 48] {
        let mut p = [0u8; 48];
        p[0] = li_vn_mode;
        p[1] = stratum;
        p[24..32].copy_from_slice(request);
        p[40..44].copy_from_slice(&((EARLIEST + NTP_TO_UNIX) as u32).to_be_bytes());
        p
    }

    #[test]
    fn answer_checks() {
        let req = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(check_answer(&answer(0x24, 2, &req), &req), Ok(((EARLIEST + NTP_TO_UNIX), 0)));
        assert_eq!(check_answer(&answer(0x24, 2, &req)[..47], &req), Err(Refused::Bad("not a server answer")));
        assert_eq!(check_answer(&answer(0x23, 2, &req), &req), Err(Refused::Bad("not a server answer")));
        assert_eq!(
            check_answer(&answer(0x24, 2, &[0; 8]), &req),
            Err(Refused::Bad("answer does not match the request"))
        );
        assert!(matches!(check_answer(&answer(0xe4, 2, &req), &req), Err(Refused::Bad(w)) if w.contains("leap")));
        assert!(matches!(check_answer(&answer(0x24, 16, &req), &req), Err(Refused::Bad(w)) if w.contains("stratum")));
        let mut kiss = answer(0xe4, 0, &req);
        kiss[12..16].copy_from_slice(b"RATE");
        assert_eq!(check_answer(&kiss, &req), Err(Refused::Kiss("RATE".into())));
        let mut zero = answer(0x24, 2, &req);
        zero[40..48].fill(0);
        assert_eq!(check_answer(&zero, &req), Err(Refused::Bad("no transmit time")));
    }

    #[test]
    fn choosing() {
        const S: i128 = 1_000_000_000;
        assert!(choose(&[], true).is_err());
        assert_eq!(choose(&[5 * S], false), Ok(5 * S));
        // One answer may move the clock far only at the first sync.
        assert_eq!(choose(&[7200 * S], true), Ok(7200 * S));
        assert!(choose(&[7200 * S], false).is_err());
        assert_eq!(choose(&[S / 10, S / 5], false), Ok(S * 3 / 20));
        assert!(choose(&[0, 7200 * S], true).is_err());
        // One wrong server among three does not move the clock.
        assert_eq!(choose(&[S / 100, 7200 * S, S / 50], false), Ok(S / 50));
        assert_eq!(choose(&[7200 * S, 7200 * S + S / 2, 0], false), Ok(7200 * S));
        assert!(choose(&[-7200 * S, 0, 7200 * S], false).is_ok());
        assert!(choose(&[-7200 * S, 3 * 3600 * S, 7200 * S], false).is_err());
    }

    #[test]
    fn utc_round_trip() {
        assert_eq!(parse_utc("2026-01-01T00:00:00Z"), Some(EARLIEST));
        for unix in [EARLIEST, 1_793_088_000, 4_107_542_399, 951_782_400] {
            let text = format_utc(unix).replace(' ', "T") + "Z";
            assert_eq!(parse_utc(&text), Some(unix), "{text}");
        }
        assert_eq!(parse_utc("2026-10-27 08:00:00"), None);
    }
}
