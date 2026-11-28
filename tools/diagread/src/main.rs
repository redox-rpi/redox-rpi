//! `diagread`: read the display sampler of the open Pi firmware
//! (rpi-open-firmware #66) and find display stalls.
//!
//! ```text
//! diagread status            header: period, records, dropped, marker
//! diagread dump [N]          the last N records (default 20), decoded
//! diagread stalls [MS]       stalls of MS or more (default 50) in the ring
//! diagread watch [MS]        the same, live, until Ctrl-C
//! diagread axi [S]           AXI bus counts per second, last S seconds (10)
//! diagread save FILE         the whole ring as a file, for later
//! diagread marker VALUE      write the marker word (lines records up with
//!                            what a driver did)
//! diagread start US | stop   Linux only: start or stop by mailbox (/dev/vcio)
//! ```
//!
//! The ring is found from the devicetree (`/chosen redox-rpi,diag-ring`,
//! set by our firmware) or `--addr 0xADDR`; `--file FILE` reads a saved
//! ring instead of memory, on any host. On Redox it maps
//! `/scheme/memory/physical@dev`, on Linux `/dev/mem`; both need root.
//! Start the sampler with `diag_period=US` in config.txt.

mod ring;

use ring::{Event, Header, Record};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::ExitCode;
use std::time::Duration;

const USAGE: &str = "usage: diagread [--addr 0xADDR | --file FILE] status | dump [N] | stalls [MS] | axi [S] | watch [MS] | save FILE | marker VALUE | start US | stop";

#[cfg(target_os = "redox")]
const DTB: &str = "/scheme/kernel.dtb";
#[cfg(not(target_os = "redox"))]
const DTB: &str = "/sys/firmware/fdt";

#[cfg(target_os = "redox")]
const MEMORY: &str = "/scheme/memory/physical@dev";
#[cfg(not(target_os = "redox"))]
const MEMORY: &str = "/dev/mem";

const DT_PROP: &str = "redox-rpi,diag-ring";

/// A mapping of the ring in physical memory (uncached).
struct Mapping {
    ptr: *mut u8,
    len: usize,
    _file: File,
}

impl Mapping {
    fn map(phys: u64, len: usize, write: bool) -> Result<Mapping, String> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(write);
        #[cfg(target_os = "linux")]
        std::os::unix::fs::OpenOptionsExt::custom_flags(&mut options, libc::O_SYNC);
        let file = options.open(MEMORY).map_err(|err| format!("{MEMORY}: {err} (root needed)"))?;
        let prot = if write { libc::PROT_READ | libc::PROT_WRITE } else { libc::PROT_READ };
        // SAFETY: a fresh shared mapping of the device file; the result is
        // checked below and only used within `len`.
        let ptr = unsafe {
            libc::mmap(std::ptr::null_mut(), len, prot, libc::MAP_SHARED, file.as_raw_fd(), phys as libc::off_t)
        };
        if ptr == libc::MAP_FAILED {
            return Err(format!("map 0x{phys:x}: {}", std::io::Error::last_os_error()));
        }
        Ok(Mapping { ptr: ptr as *mut u8, len, _file: file })
    }

    fn word(&self, index: usize) -> u32 {
        assert!(index * 4 + 4 <= self.len);
        // SAFETY: inside the mapping (checked above), word aligned because
        // the mapping is page aligned; volatile because the VPU writes it.
        unsafe { std::ptr::read_volatile((self.ptr as *const u32).add(index)) }
    }

    fn set_word(&self, index: usize, value: u32) {
        assert!(index * 4 + 4 <= self.len);
        // SAFETY: as in `word`; the mapping is writable when this is called
        // (only `marker` maps with write).
        unsafe { std::ptr::write_volatile((self.ptr as *mut u32).add(index), value) }
    }

    fn words(&self, from: usize, n: usize) -> Vec<u32> {
        (from..from + n).map(|i| self.word(i)).collect()
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: unmaps exactly what `map` mapped.
        unsafe { libc::munmap(self.ptr as *mut libc::c_void, self.len) };
    }
}

/// Where the ring is: from `--addr` or the devicetree.
fn locate(addr: Option<u64>) -> Result<(u64, usize), String> {
    if let Some(a) = addr {
        return Ok((a, 0x10_0000));
    }
    let bytes = std::fs::read(DTB).map_err(|err| format!("{DTB}: {err}"))?;
    let fdt = fdt::Fdt::new(&bytes).map_err(|err| format!("{DTB}: {err:?}"))?;
    let value = fdt
        .find_node("/chosen")
        .and_then(|n| n.property(DT_PROP))
        .map(|p| p.value)
        .ok_or_else(|| format!("no /chosen {DT_PROP} in the devicetree: not our firmware (or an older one); try --addr"))?;
    if value.len() != 8 {
        return Err(format!("/chosen {DT_PROP}: {} bytes, not 8", value.len()));
    }
    let word = |i: usize| u32::from_be_bytes([value[i], value[i + 1], value[i + 2], value[i + 3]]);
    Ok((u64::from(word(0)), word(4) as usize))
}

/// The ring's words: from memory or a saved file.
enum Source {
    Memory { map: Mapping, phys: u64 },
    File(Vec<u32>),
}

impl Source {
    fn open(addr: Option<u64>, file: Option<&str>, write: bool) -> Result<Source, String> {
        if let Some(path) = file {
            let bytes = std::fs::read(path).map_err(|err| format!("{path}: {err}"))?;
            return Ok(Source::File(ring::words_from_bytes(&bytes)));
        }
        let (phys, len) = locate(addr)?;
        Ok(Source::Memory { map: Mapping::map(phys, len, write)?, phys })
    }

    fn header(&self) -> Result<Header, String> {
        match self {
            Source::Memory { map, .. } => Header::parse_head(&map.words(0, ring::HEADER_WORDS), map.len / 4),
            Source::File(w) => Header::parse(w),
        }
    }

    /// All words, header and records, as they are now.
    fn snapshot(&self) -> Vec<u32> {
        match self {
            Source::Memory { map, .. } => map.words(0, map.len / 4),
            Source::File(w) => w.clone(),
        }
    }

    fn record(&self, h: &Header, i: u32) -> Record {
        let n = h.rec_words as usize;
        let at = ring::HEADER_WORDS + (i % h.recs) as usize * n;
        match self {
            Source::Memory { map, .. } => Record::from_words(&map.words(at, n)),
            Source::File(w) => Record::from_words(&w[at..at + n]),
        }
    }
}

fn secs(t_us: u32) -> String {
    format!("{}.{:03}", t_us / 1_000_000, (t_us / 1000) % 1000)
}

fn print_event(e: &Event) {
    match e {
        Event::Stall { what, t_us, len_us } => println!("{:>10} s  stall  {what} stood still {} ms", secs(*t_us), len_us / 1000),
        Event::Gap { t_us, len_us } => println!("{:>10} s  gap    no sample for {} ms (VPU busy)", secs(*t_us), len_us / 1000),
        Event::Flag { what, t_us } => println!("{:>10} s  flag   {what}", secs(*t_us)),
        Event::Clock { t_us, from_mhz, to_mhz } => println!("{:>10} s  clock  ARM {from_mhz} -> {to_mhz} MHz", secs(*t_us)),
    }
}

fn print_record(r: &Record) {
    let temp = r.temp_mc().map_or("-".to_string(), |m| format!("{:.1}", f64::from(m) / 1000.0));
    println!(
        "{:>10}  {:>6}  {:>2}  {:08x}  {:08x}  {:>5}  {:>4.0}  {:08x}",
        secs(r.t_us),
        r.hdmi_frames,
        r.hvs_frame(),
        r.dispstat,
        r.pv2_stat,
        temp,
        r.arm_mhz(),
        r.marker
    );
}

fn number(s: &str) -> Result<u64, String> {
    let parsed = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => s.parse(),
    };
    parsed.map_err(|_| format!("not a number: {s}"))
}

fn status(src: &Source) -> Result<(), String> {
    let h = src.header()?;
    if let Source::Memory { phys, .. } = src {
        println!("ring      0x{phys:x}");
    }
    println!("period    {} us", h.period_us);
    println!("written   {} ({} in the ring of {})", h.written, h.valid(), h.recs);
    println!("dropped   {}", h.dropped);
    println!("marker    0x{:08x}", h.marker);
    if h.axi {
        let names: Vec<_> = h.axi_buses.iter().enumerate().map(|(w, b)| ring::axi_bus_name(w, *b)).collect();
        println!("axi       system {} | VPU {}", names[..3].join(" "), names[3..].join(" "));
    }
    if h.valid() > 0 {
        let last = src.record(&h, h.written - 1);
        println!("last      t {} s, hdmi frame {}, ARM {:.0} MHz", secs(last.t_us), last.hdmi_frames, last.arm_mhz());
    }
    Ok(())
}

fn dump(src: &Source, n: u32) -> Result<(), String> {
    let words = src.snapshot();
    let h = Header::parse(&words)?;
    let recs = ring::records(&words, &h);
    println!("{:>10}  {:>6}  {:>2}  {:>8}  {:>8}  {:>5}  {:>4}  {:>8}", "t s", "hdmi", "hv", "dispstat", "pv2stat", "°C", "MHz", "marker");
    for r in &recs[recs.len().saturating_sub(n as usize)..] {
        print_record(r);
    }
    Ok(())
}

/// AXI counts per second: reads, writes and wait cycles per bus watcher.
fn axi(src: &Source, seconds: u32) -> Result<(), String> {
    let words = src.snapshot();
    let h = Header::parse(&words)?;
    if !h.axi {
        return Err("the sampler runs without AXI counters (diag_axi=1 in config.txt, or start with 0x80000000 added)".into());
    }
    let recs = ring::records(&words, &h);
    let names: Vec<_> = h.axi_buses.iter().enumerate().map(|(w, b)| ring::axi_bus_name(w, *b)).collect();
    print!("{:>10}", "t s");
    for n in &names {
        print!("  {:>22}", format!("{n} r/w/wait k"));
    }
    println!();
    let per = (1_000_000 / h.period_us.max(1)).max(1) as usize;
    let skip = recs.len().saturating_sub(seconds as usize * per);
    for chunk in recs[skip..].chunks(per) {
        print!("{:>10}", secs(chunk[0].t_us));
        for w in 0..6 {
            let (r, wr, wait) = chunk.iter().fold((0u64, 0u64, 0u64), |(a, b, c), x| {
                (a + u64::from(x.axi[w].reads), b + u64::from(x.axi[w].writes), c + u64::from(x.axi[w].wait))
            });
            print!("  {:>22}", format!("{}/{}/{}", r / 1000, wr / 1000, wait / 1000));
        }
        println!();
    }
    Ok(())
}

fn stalls(src: &Source, ms: u32) -> Result<(), String> {
    let words = src.snapshot();
    let h = Header::parse(&words)?;
    let recs = ring::records(&words, &h);
    let events = ring::analyse(&recs, ms * 1000, h.period_us);
    if let (Some(a), Some(b)) = (recs.first(), recs.last()) {
        println!("{} records, {} s to {} s, period {} us, dropped {}", recs.len(), secs(a.t_us), secs(b.t_us), h.period_us, h.dropped);
    }
    for e in &events {
        print_event(e);
    }
    println!("{} events (stalls of {ms} ms or more)", events.len());
    Ok(())
}

fn watch(src: &Source, ms: u32) -> Result<(), String> {
    let h = src.header()?;
    let mut detector = ring::Detector::new(ms * 1000, h.period_us);
    let mut next = h.written;
    let mut beat = std::time::Instant::now();
    let mut events = Vec::new();
    println!("watching, stalls of {ms} ms or more (Ctrl-C ends)");
    loop {
        std::thread::sleep(Duration::from_millis(200));
        let h = src.header()?;
        if h.written.wrapping_sub(next) > h.recs {
            println!("lost {} records (reader too slow)", h.written - next - h.recs);
            next = h.written - h.recs;
        }
        while next != h.written {
            detector.feed(&src.record(&h, next), &mut events);
            next = next.wrapping_add(1);
        }
        for e in events.drain(..) {
            print_event(&e);
        }
        if beat.elapsed() >= Duration::from_secs(10) && h.valid() > 0 {
            beat = std::time::Instant::now();
            let r = src.record(&h, h.written - 1);
            let temp = r.temp_mc().map_or("-".into(), |m| format!("{:.1} °C", f64::from(m) / 1000.0));
            println!("{:>10} s  alive  hdmi frame {}, {temp}, ARM {:.0} MHz, dropped {}", secs(r.t_us), r.hdmi_frames, r.arm_mhz(), h.dropped);
        }
    }
}

fn save(src: &Source, path: &str) -> Result<(), String> {
    let words = src.snapshot();
    let h = Header::parse(&words)?;
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    std::fs::write(path, bytes).map_err(|err| format!("{path}: {err}"))?;
    println!("saved {} records to {path}", h.valid());
    Ok(())
}

fn marker(src: &Source, value: u32) -> Result<(), String> {
    src.header()?;
    match src {
        Source::Memory { map, .. } => {
            map.set_word(ring::MARKER_OFFSET / 4, value);
            Ok(())
        }
        Source::File(_) => Err("marker needs the ring in memory, not a file".into()),
    }
}

/// Mailbox tag 0x000f0001 (period us, 0 stops) through Linux's /dev/vcio.
#[cfg(target_os = "linux")]
fn mailbox_diag(period: u32) -> Result<(), String> {
    // _IOWR(100, 0, char *): the property interface of Linux vcio.
    const IOCTL_MBOX_PROPERTY: libc::c_ulong = (3 << 30) | ((std::mem::size_of::<usize>() as libc::c_ulong) << 16) | (100 << 8);
    let mut buf: [u32; 8] = [32, 0, 0x000f_0001, 8, 4, period, 0, 0];
    let f = std::fs::OpenOptions::new().read(true).write(true).open("/dev/vcio").map_err(|e| format!("/dev/vcio: {e}"))?;
    // SAFETY: the ioctl reads and writes the 32-byte buffer it is given.
    let r = unsafe { libc::ioctl(f.as_raw_fd(), IOCTL_MBOX_PROPERTY as _, buf.as_mut_ptr()) };
    if r < 0 {
        return Err(format!("/dev/vcio: {}", std::io::Error::last_os_error()));
    }
    if buf[1] != 0x8000_0000 || buf[4] & 0x8000_0000 == 0 {
        return Err(format!("the firmware did not answer tag 0x000f0001 (code 0x{:08x}): not our firmware?", buf[1]));
    }
    if period == 0 {
        println!("sampler stopped");
    } else {
        println!("sampler on, every {period} us, ring at bus 0x{:08x}, {} bytes", buf[5], buf[6]);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn mailbox_diag(_period: u32) -> Result<(), String> {
    Err("start/stop by mailbox works on Linux only; on Redox set diag_period=US in config.txt".into())
}

fn run(args: &[String]) -> Result<(), String> {
    let mut addr = None;
    let mut file = None;
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--addr" => addr = Some(number(it.next().ok_or(USAGE)?)?),
            "--file" => file = Some(it.next().ok_or(USAGE)?.as_str()),
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            _ => rest.push(a.as_str()),
        }
    }
    let arg = |i: usize, default: u64| rest.get(i).map_or(Ok(default), |s| number(s));
    match rest.first().copied().unwrap_or("status") {
        "status" => status(&Source::open(addr, file, false)?),
        "dump" => dump(&Source::open(addr, file, false)?, arg(1, 20)? as u32),
        "axi" => axi(&Source::open(addr, file, false)?, arg(1, 10)? as u32),
        "stalls" => stalls(&Source::open(addr, file, false)?, arg(1, 50)? as u32),
        "watch" => watch(&Source::open(addr, file, false)?, arg(1, 50)? as u32),
        "save" => save(&Source::open(addr, file, false)?, rest.get(1).ok_or(USAGE)?),
        "marker" => marker(&Source::open(addr, file, true)?, number(rest.get(1).ok_or(USAGE)?)? as u32),
        "start" => mailbox_diag(number(rest.get(1).ok_or(USAGE)?)? as u32),
        "stop" => mailbox_diag(0),
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("diagread: {err}");
            ExitCode::FAILURE
        }
    }
}
