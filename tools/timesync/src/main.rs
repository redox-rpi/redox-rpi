//! timesync: set the Redox system clock from an NTP server.
//!
//! The Raspberry Pi has no real-time clock, so Redox starts at 1970. This
//! asks NTP servers (SNTP, RFC 4330) for the time once the network is up and
//! sets the clock by writing the boot time, in nanoseconds since 1970, to
//! the kernel's `/scheme/sys/update_time_offset`. Needs root.
//!
//! Usage: `timesync [-n] [server]...` (default: pool.ntp.org time.cloudflare.com).
//! `-n` only prints the time without setting the clock.
//! Tries the servers in turn until one answers, for up to a minute, because
//! DHCP may still be running when it starts.

use std::io;
use std::net::{ToSocketAddrs, UdpSocket};
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};

const DEFAULT_SERVERS: &[&str] = &["pool.ntp.org", "time.cloudflare.com"];
const TIME_OFFSET: &str = "/scheme/sys/update_time_offset";
/// Seconds from 1900 (NTP era 0) to 1970.
const NTP_TO_UNIX: u64 = 2_208_988_800;
/// Answers before this (2026-01-01) are wrong.
const EARLIEST: u64 = 1_767_225_600;
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);
const GIVE_UP_AFTER: Duration = Duration::from_secs(60);

/// Nanoseconds of the kernel's monotonic clock, the base of its real time.
fn monotonic_ns() -> u128 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as u128 * 1_000_000_000 + ts.tv_nsec as u128
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

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let dry_run = args.first().is_some_and(|a| a == "-n");
    if dry_run {
        args.remove(0);
    }
    let servers: Vec<&str> = if args.is_empty() {
        DEFAULT_SERVERS.to_vec()
    } else {
        args.iter().map(String::as_str).collect()
    };

    let start = Instant::now();
    let mut last_error = String::new();
    while start.elapsed() < GIVE_UP_AFTER {
        for server in &servers {
            match query(server) {
                Ok((unix_ns, at_monotonic)) => {
                    let unix = (unix_ns / 1_000_000_000) as u64;
                    if dry_run {
                        println!("timesync: {} UTC from {server} (clock not set)", format_utc(unix));
                        return;
                    }
                    // The kernel's real time is boot time plus monotonic time.
                    let boot = unix_ns - at_monotonic;
                    if let Err(err) = std::fs::write(TIME_OFFSET, boot.to_ne_bytes()) {
                        eprintln!("timesync: cannot set the clock through {TIME_OFFSET}: {err}");
                        process::exit(1);
                    }
                    println!("timesync: clock set to {} UTC from {server}", format_utc(unix));
                    return;
                }
                Err(err) => last_error = format!("{server}: {err}"),
            }
        }
        sleep(Duration::from_secs(2));
    }
    eprintln!("timesync: no time server answered within a minute (last: {last_error})");
    process::exit(1);
}
