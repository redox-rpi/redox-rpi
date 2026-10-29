//! Talks HCI to the Bluetooth half of the Pi 3B+ Wi-Fi/Bluetooth chip
//! (BCM43455, CYW43455) over `/scheme/btuart/data` (driver
//! `bcm2835-btuartd`), in the UART framing of the Bluetooth spec (H4): a
//! packet type byte, then the packet.
//!
//! ```text
//! bt up                 power cycle, reset, load the firmware, reset
//! bt info               version, firmware build, address, name
//! bt scan [seconds]     LE scan (default 5 s)
//! bt inquiry [seconds]  classic inquiry (default 5 s)
//! bt cmd 1001 [bytes]   send an HCI command (hex), print what comes back
//! ```
//!
//! The chip starts in ROM without the firmware (build 0); `bt up` loads
//! `BCM4345C0.hcd`, as Linux's btbcm does.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Write};
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};

const DATA: &str = "/scheme/btuart/data";
const CLOCK: &str = "/scheme/btuart/clock";
const POWER: &str = "/scheme/rpi-firmware/bt_power";
const CLOCKS: &str = "/scheme/rpi-firmware/clocks";
const FIRMWARE: &str = "/usr/lib/firmware/brcm/BCM4345C0.hcd";

const PACKET_COMMAND: u8 = 0x01;
const PACKET_ACL: u8 = 0x02;
const PACKET_SCO: u8 = 0x03;
const PACKET_EVENT: u8 = 0x04;

const EVENT_INQUIRY_COMPLETE: u8 = 0x01;
const EVENT_INQUIRY_RESULT: u8 = 0x02;
const EVENT_COMMAND_COMPLETE: u8 = 0x0e;
const EVENT_COMMAND_STATUS: u8 = 0x0f;
const EVENT_LE_META: u8 = 0x3e;
const LE_ADVERTISING_REPORT: u8 = 0x02;

const INQUIRY: u16 = 0x0401;
const SET_EVENT_MASK: u16 = 0x0c01;
const RESET: u16 = 0x0c03;
const READ_LOCAL_NAME: u16 = 0x0c14;
const WRITE_LE_HOST_SUPPORTED: u16 = 0x0c6d;
const READ_LOCAL_VERSION: u16 = 0x1001;
const READ_BD_ADDR: u16 = 0x1009;
const LE_SET_SCAN_PARAMETERS: u16 = 0x200b;
const LE_SET_SCAN_ENABLE: u16 = 0x200c;
/// Broadcom vendor commands.
const DOWNLOAD_MINIDRIVER: u16 = 0xfc2e;
const LAUNCH_RAM: u16 = 0xfc4e;

/// The default event mask (spec), plus LE Meta (bit 61).
const EVENT_MASK: u64 = 0x2000_1fff_ffff_ffff;
/// The general inquiry access code.
const GIAC: [u8; 3] = [0x33, 0x8b, 0x9e];
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);

fn error(text: String) -> io::Error {
    io::Error::new(ErrorKind::Other, text)
}

struct Hci {
    port: File,
    /// Bytes read but not yet taken as a packet.
    pending: Vec<u8>,
}

impl Hci {
    fn open() -> io::Result<Self> {
        let port = File::options()
            .read(true)
            .write(true)
            .open(DATA)
            .map_err(|err| error(format!("{DATA}: {err} (is bcm2835-btuartd running?)")))?;
        let mut hci = Hci { port, pending: Vec::new() };
        hci.drain();
        Ok(hci)
    }

    /// Drops whatever has arrived so far.
    fn drain(&mut self) {
        let mut buf = [0; 256];
        while matches!(self.port.read(&mut buf), Ok(n) if n > 0) {}
        self.pending.clear();
    }

    fn send(&mut self, opcode: u16, params: &[u8]) -> io::Result<()> {
        let mut packet = vec![PACKET_COMMAND, opcode as u8, (opcode >> 8) as u8, params.len() as u8];
        packet.extend_from_slice(params);
        self.port.write_all(&packet)
    }

    /// `n` bytes, reading more until the deadline; None if they did not come.
    fn take(&mut self, n: usize, deadline: Instant) -> io::Result<Option<Vec<u8>>> {
        let mut buf = [0; 512];
        while self.pending.len() < n {
            if Instant::now() > deadline {
                return Ok(None);
            }
            match self.port.read(&mut buf) {
                Ok(len) => self.pending.extend_from_slice(&buf[..len]),
                Err(err) if err.kind() == ErrorKind::WouldBlock => {}
                Err(err) => return Err(err),
            }
        }
        Ok(Some(self.pending.drain(..n).collect()))
    }

    /// The next event (code and parameters), skipping data packets and
    /// bytes that start no packet; None at the deadline.
    fn event(&mut self, deadline: Instant) -> io::Result<Option<(u8, Vec<u8>)>> {
        loop {
            let Some(kind) = self.take(1, deadline)? else { return Ok(None) };
            let (header, length): (usize, fn(&[u8]) -> usize) = match kind[0] {
                PACKET_EVENT => (2, |h| h[1] as usize),
                PACKET_ACL => (4, |h| u16::from_le_bytes([h[2], h[3]]) as usize),
                PACKET_SCO => (3, |h| h[2] as usize),
                _ => continue,
            };
            let Some(head) = self.take(header, deadline)? else { return Ok(None) };
            let Some(body) = self.take(length(&head), deadline)? else { return Ok(None) };
            if kind[0] == PACKET_EVENT {
                return Ok(Some((head[0], body)));
            }
        }
    }

    /// Sends a command and waits for its Command Complete (returns the
    /// return parameters, status first) or Command Status (returns the
    /// status). Other events in between are dropped.
    fn command_timeout(&mut self, opcode: u16, params: &[u8], timeout: Duration) -> io::Result<Vec<u8>> {
        self.send(opcode, params)?;
        let deadline = Instant::now() + timeout;
        let op = opcode.to_le_bytes();
        loop {
            match self.event(deadline)? {
                None => return Err(error(format!("no answer to HCI command {opcode:04x}"))),
                Some((EVENT_COMMAND_COMPLETE, p)) if p.len() >= 3 && p[1..3] == op => return Ok(p[3..].to_vec()),
                Some((EVENT_COMMAND_STATUS, p)) if p.len() >= 4 && p[2..4] == op => return Ok(vec![p[0]]),
                Some(_) => {}
            }
        }
    }

    /// A command that must succeed (status 0); returns its parameters after
    /// the status.
    fn command(&mut self, opcode: u16, params: &[u8]) -> io::Result<Vec<u8>> {
        let ret = self.command_timeout(opcode, params, COMMAND_TIMEOUT)?;
        match ret.first() {
            Some(0) => Ok(ret[1..].to_vec()),
            Some(status) => Err(error(format!("HCI command {opcode:04x} failed, status {status:02x}"))),
            None => Ok(Vec::new()),
        }
    }
}

fn address(bytes: &[u8]) -> String {
    bytes.iter().rev().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

fn spec_name(version: u8) -> &'static str {
    match version {
        6 => "4.0",
        7 => "4.1",
        8 => "4.2",
        9 => "5.0",
        10 => "5.1",
        11 => "5.2",
        12 => "5.3",
        13 => "5.4",
        _ => "?",
    }
}

/// One line about the version; the firmware build is in the low 12 bits of
/// the HCI revision (0 while the chip runs from ROM), as Linux's btbcm
/// prints it.
fn version(hci: &mut Hci) -> io::Result<u16> {
    let v = hci.command(READ_LOCAL_VERSION, &[])?;
    if v.len() < 8 {
        return Err(error(format!("short version answer: {}", hex(&v))));
    }
    let revision = u16::from_le_bytes([v[1], v[2]]);
    let manufacturer = u16::from_le_bytes([v[4], v[5]]);
    let subversion = u16::from_le_bytes([v[6], v[7]]);
    println!(
        "version: Bluetooth {} (HCI {}, LMP {}), manufacturer {manufacturer}{}, subversion {subversion:04x}, firmware build {}",
        spec_name(v[0]),
        v[0],
        v[3],
        if manufacturer == 15 { " (Broadcom)" } else { "" },
        revision & 0x0fff
    );
    Ok(revision & 0x0fff)
}

fn reset(hci: &mut Hci) -> io::Result<()> {
    hci.command(RESET, &[]).map(|_| ())
}

/// Switches the chip off and on, if this board lets us.
fn power_cycle() {
    if fs::metadata(POWER).is_err() {
        println!("power: no {POWER} (not a Pi 3B+?), leaving it as it is");
        return;
    }
    let result = fs::write(POWER, "0").and_then(|()| {
        sleep(Duration::from_millis(50));
        fs::write(POWER, "1")
    });
    match result {
        Ok(()) => {
            // The chip needs a moment before it answers on the UART.
            sleep(Duration::from_millis(200));
            println!("power: cycled");
        }
        Err(err) => println!("power: cannot switch it ({err}), going on"),
    }
}

/// Tells the driver the current core clock, which the baud rate depends on.
fn update_clock() {
    let core = fs::read_to_string(CLOCKS)
        .ok()
        .and_then(|text| text.lines().find_map(|l| l.strip_prefix("core ")?.split_whitespace().next().map(str::to_owned)));
    if let Some(core) = core {
        if fs::write(CLOCK, &core).is_ok() {
            let baud = fs::read_to_string("/scheme/btuart/baud").unwrap_or_default();
            println!("uart: core clock {core} Hz, {} baud", baud.trim());
        }
    }
}

/// Loads the firmware: Download Minidriver, then the file's records (each
/// an HCI command, mostly Write RAM), the last being Launch RAM, after
/// which the chip restarts from RAM.
fn load_firmware(hci: &mut Hci, path: &str) -> io::Result<()> {
    let image = fs::read(path).map_err(|err| error(format!("{path}: {err}")))?;
    hci.command(DOWNLOAD_MINIDRIVER, &[])?;
    sleep(Duration::from_millis(50));
    let (mut at, mut records) = (0, 0);
    let started = Instant::now();
    while at + 3 <= image.len() {
        let opcode = u16::from_le_bytes([image[at], image[at + 1]]);
        let len = image[at + 2] as usize;
        let params = image.get(at + 3..at + 3 + len).ok_or_else(|| error(format!("{path}: cut off at byte {at}")))?;
        if opcode == LAUNCH_RAM {
            // The chip may restart before it answers.
            let _ = hci.command_timeout(opcode, params, Duration::from_millis(500));
        } else {
            hci.command(opcode, params)?;
        }
        records += 1;
        at += 3 + len;
    }
    println!("firmware: {path}, {records} records in {} ms", started.elapsed().as_millis());
    sleep(Duration::from_millis(250));
    hci.drain();
    Ok(())
}

fn up() -> io::Result<()> {
    power_cycle();
    update_clock();
    let mut hci = Hci::open()?;
    reset(&mut hci)?;
    let before = version(&mut hci)?;
    if before != 0 {
        println!("firmware: already running build {before}, loading it again");
    }
    load_firmware(&mut hci, FIRMWARE)?;
    reset(&mut hci)?;
    if version(&mut hci)? == 0 {
        return Err(error("firmware build still 0 after loading".into()));
    }
    println!("up");
    Ok(())
}

fn info() -> io::Result<()> {
    let mut hci = Hci::open()?;
    version(&mut hci)?;
    let addr = hci.command(READ_BD_ADDR, &[])?;
    println!("address: {}", address(&addr[..addr.len().min(6)]));
    let name = hci.command(READ_LOCAL_NAME, &[])?;
    let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    println!("name: {}", String::from_utf8_lossy(&name[..end]));
    Ok(())
}

/// The name in advertising data: Complete (09) or Shortened (08) Local Name.
fn ad_name(mut data: &[u8]) -> Option<String> {
    while let [len, rest @ ..] = data {
        let len = *len as usize;
        if len == 0 || rest.len() < len {
            break;
        }
        if matches!(rest[0], 0x08 | 0x09) {
            return Some(String::from_utf8_lossy(&rest[1..len]).into_owned());
        }
        data = &rest[len..];
    }
    None
}

fn le_scan(seconds: u64) -> io::Result<()> {
    let mut hci = Hci::open()?;
    hci.command(SET_EVENT_MASK, &EVENT_MASK.to_le_bytes())?;
    // Older controllers want the host to say it does LE; newer ones may
    // not know the command.
    let _ = hci.command(WRITE_LE_HOST_SUPPORTED, &[1, 0]);
    // Active scan (scan requests bring the names), 60 ms interval, 30 ms
    // window, public address, all advertisers.
    hci.command(LE_SET_SCAN_PARAMETERS, &[1, 0x60, 0x00, 0x30, 0x00, 0, 0])?;
    hci.command(LE_SET_SCAN_ENABLE, &[1, 1])?;
    println!("scan: LE for {seconds} s");
    let mut seen: BTreeMap<String, (i8, Option<String>)> = BTreeMap::new();
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while let Some((code, p)) = hci.event(deadline)? {
        if code != EVENT_LE_META || p.first() != Some(&LE_ADVERTISING_REPORT) {
            continue;
        }
        // Reports one after the other: type, address type, address, data
        // length, data, RSSI.
        let mut rest = &p[2..];
        for _ in 0..p.get(1).copied().unwrap_or(0) {
            if rest.len() < 9 || rest.len() < 9 + rest[8] as usize + 1 {
                break;
            }
            let len = rest[8] as usize;
            let entry = seen.entry(address(&rest[2..8])).or_insert((0, None));
            entry.0 = rest[9 + len] as i8;
            if let Some(name) = ad_name(&rest[9..9 + len]) {
                entry.1 = Some(name);
            }
            rest = &rest[10 + len..];
        }
    }
    hci.command(LE_SET_SCAN_ENABLE, &[0, 0])?;
    for (addr, (rssi, name)) in &seen {
        println!("le {addr} rssi {rssi} {}", name.as_deref().unwrap_or(""));
    }
    println!("scan: {} devices", seen.len());
    Ok(())
}

fn inquiry(seconds: u64) -> io::Result<()> {
    let mut hci = Hci::open()?;
    // The length is in units of 1.28 s.
    let units = (seconds * 100).div_ceil(128).clamp(1, 0x30) as u8;
    hci.command(INQUIRY, &[GIAC[0], GIAC[1], GIAC[2], units, 0])?;
    println!("inquiry: {} s", u64::from(units) * 128 / 100);
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let deadline = Instant::now() + Duration::from_millis(u64::from(units) * 1280 + 2000);
    while let Some((code, p)) = hci.event(deadline)? {
        match code {
            EVENT_INQUIRY_COMPLETE => break,
            // Arrays: n addresses, n page scan modes, n * 2 reserved,
            // n classes of device, n clock offsets.
            EVENT_INQUIRY_RESULT => {
                let n = p.first().copied().unwrap_or(0) as usize;
                if p.len() < 1 + n * 14 {
                    continue;
                }
                for i in 0..n {
                    let class = &p[1 + n * 9 + i * 3..][..3];
                    seen.insert(address(&p[1 + i * 6..][..6]), format!("{:06x}", u32::from_le_bytes([class[0], class[1], class[2], 0])));
                }
            }
            _ => {}
        }
    }
    for (addr, class) in &seen {
        println!("bredr {addr} class {class}");
    }
    println!("inquiry: {} devices", seen.len());
    Ok(())
}

fn raw(args: &[String]) -> io::Result<()> {
    let parse = |s: &String| u16::from_str_radix(s, 16).map_err(|_| error(format!("not hex: {s}")));
    let opcode = parse(&args[0])?;
    let params = args[1..].iter().map(|s| parse(s).map(|v| v as u8)).collect::<io::Result<Vec<u8>>>()?;
    let mut hci = Hci::open()?;
    println!("{}", hex(&hci.command_timeout(opcode, &params, COMMAND_TIMEOUT)?));
    Ok(())
}

fn seconds(arg: Option<&String>) -> u64 {
    arg.and_then(|s| s.parse().ok()).filter(|&s| s > 0).unwrap_or(5)
}

fn usage() -> ! {
    eprintln!("usage: bt up | info | scan [seconds] | inquiry [seconds] | cmd <opcode> [bytes...]   (hex)");
    process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("up") => up(),
        Some("info") => info(),
        Some("scan") => le_scan(seconds(args.get(1))),
        Some("inquiry") => inquiry(seconds(args.get(1))),
        Some("cmd") if args.len() >= 2 => raw(&args[1..]),
        _ => usage(),
    };
    if let Err(err) = result {
        eprintln!("bt: {err}");
        process::exit(1);
    }
}
