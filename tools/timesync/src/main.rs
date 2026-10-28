//! timesync: set the Redox system clock from an NTP server.
//!
//! The Raspberry Pi has no real-time clock, so Redox starts at 1970. At start
//! this moves the clock forward to the newest of two times known without a
//! network, like Linux's fake-hwclock: the image's build time (`BUILD=` in
//! `/etc/redox-rpi-release`) and the time saved by an earlier run. Then it
//! asks NTP servers (SNTP, RFC 4330) for the time and saves it.
//!
//! The clock is set by writing the boot time, in nanoseconds since 1970, to
//! the kernel's `/scheme/sys/update_time_offset`. Needs root.
//!
//! Usage: `timesync [-n | -d] [server]...` (default: pool.ntp.org time.cloudflare.com).
//! `-n` only prints the time without setting the clock.
//! `-d` keeps running: asks the servers again and saves the time every hour.
//! Tries the servers in turn until one answers, for up to a minute, because
//! DHCP may still be running when it starts.

use std::fs;
use std::io;
use std::net::{ToSocketAddrs, UdpSocket};
use std::process;
use std::thread::sleep;
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
fn set_clock(unix_ns: u128, at_monotonic: u128) -> io::Result<()> {
    // The kernel's real time is boot time plus monotonic time.
    let boot = unix_ns - at_monotonic;
    fs::write(TIME_OFFSET, boot.to_ne_bytes())
}

/// Ask one server. Returns the Unix time in nanoseconds and the monotonic
/// clock at the moment the answer arrived.
fn query(server: &str) -> io::Result<(u128, u128)> {
    let addr = (server, 123)
        .to_socket_addrs()?
        .find(|a| a.is_ipv4())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no IPv4 address"))?;
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(QUERY_TIMEOUT))?;
    // Leap indicator 0, version 4, mode 3 (client).
    let mut packet = [0u8; 48];
    packet[0] = 0x23;
    let sent = Instant::now();
    socket.send_to(&packet, addr)?;
    let (len, _) = socket.recv_from(&mut packet)?;
    let received = monotonic_ns();
    let round_trip = sent.elapsed();

    if len < 48 || packet[0] & 7 != 4 || packet[1] == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not a valid server answer"));
    }
    // Transmit timestamp: seconds and fraction since 1900.
    let seconds = u32::from_be_bytes(packet[40..44].try_into().unwrap()) as u64;
    let fraction = u32::from_be_bytes(packet[44..48].try_into().unwrap()) as u128;
    let unix = seconds.wrapping_sub(NTP_TO_UNIX);
    if unix < EARLIEST {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("implausible time {unix}")));
    }
    // The server stamped its answer about half a round trip ago.
    let ns = unix as u128 * 1_000_000_000 + (fraction * 1_000_000_000 >> 32) + round_trip.as_nanos() / 2;
    Ok((ns, received))
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

/// Ask the servers in turn, for up to a minute. Returns the time it set.
fn sync(servers: &[&str], dry_run: bool) -> Result<u64, String> {
    let start = Instant::now();
    let mut last_error = String::new();
    while start.elapsed() < GIVE_UP_AFTER {
        for server in servers {
            match query(server) {
                Ok((unix_ns, at_monotonic)) => {
                    let unix = (unix_ns / 1_000_000_000) as u64;
                    if dry_run {
                        println!("timesync: {} UTC from {server} (clock not set)", format_utc(unix));
                        return Ok(unix);
                    }
                    if let Err(err) = set_clock(unix_ns, at_monotonic) {
                        return Err(format!("cannot set the clock through {TIME_OFFSET}: {err}"));
                    }
                    println!("timesync: clock set to {} UTC from {server}", format_utc(unix));
                    let _ = fs::write(SYNCED, format!("{unix}\n"));
                    return Ok(unix);
                }
                Err(err) => last_error = format!("{server}: {err}"),
            }
        }
        sleep(Duration::from_secs(2));
    }
    Err(format!("no time server answered within a minute (last: {last_error})"))
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
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
    let result = sync(&servers, dry_run);
    if let Err(err) = &result {
        eprintln!("timesync: {err}");
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
    fn utc_round_trip() {
        assert_eq!(parse_utc("2026-01-01T00:00:00Z"), Some(EARLIEST));
        for unix in [EARLIEST, 1_793_088_000, 4_107_542_399, 951_782_400] {
            let text = format_utc(unix).replace(' ', "T") + "Z";
            assert_eq!(parse_utc(&text), Some(unix), "{text}");
        }
        assert_eq!(parse_utc("2026-10-27 08:00:00"), None);
    }
}
