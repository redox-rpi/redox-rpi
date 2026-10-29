//! rpi-update: update kernel and system of a direct-boot SD card
//! (`make-sdcard.sh --direct --ab`, issue #15) without writing a new card.
//!
//! The FAT partition, mounted by fatfsd at /scheme/boot, has two slots:
//!
//!     slot a: kernel8.img    initramfs
//!     slot b: kernel8-b.img  initramfs-b
//!
//! config.txt names the slot that boots. The initramfs holds the whole
//! root filesystem, so a slot is kernel and system together.
//!
//!     rpi-update status          slots, running slot, server's build
//!     rpi-update fetch [<url>]   new kernel8.img and initramfs into the other
//!                                slot, tryboot.txt pointing at it
//!     rpi-update try             set the firmware's tryboot flag, reboot
//!     rpi-update commit          after a tryboot start: config.txt points at
//!                                the new slot from now on
//!
//! The firmware reads tryboot.txt instead of config.txt once, after a reboot
//! with the tryboot flag. tryboot.txt adds REDOX_RPI_TRYBOOT=1 to the command
//! line, which rpi-boot-stub passes into the environment (/scheme/sys/env).
//! If the new system does not come up, the next reset boots the old slot
//! again: the flag is gone and config.txt still names it.
//!
//! The server directory (scripts/publish-rpi-update.sh) has kernel8.img,
//! initramfs, SHA256SUMS and BUILD_ID. Plain HTTP on the home network.

use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::FileExt;
use std::process;
use std::thread;
use std::time::Duration;

use sha2::{Digest, Sha256};

const BOOT: &str = "/scheme/boot";
const CONF: &str = "/etc/rpi-update.conf";
const TRY_MARK: &str = "REDOX_RPI_TRYBOOT=1";
/// SET_REBOOT_FLAGS: bit 0 makes the firmware read tryboot.txt once.
const TAG_SET_REBOOT_FLAGS: u32 = 0x0003_8064;

struct Slot {
    kernel: &'static str,
    initramfs: &'static str,
}

const SLOTS: [Slot; 2] = [
    Slot { kernel: "kernel8.img", initramfs: "initramfs" },
    Slot { kernel: "kernel8-b.img", initramfs: "initramfs-b" },
];

fn boot(name: &str) -> String {
    format!("{BOOT}/{name}")
}

/// Index of the slot a config file starts, from its `kernel=` line.
fn slot_of(config: &str) -> Option<usize> {
    let kernel = config.lines().find_map(|l| l.trim().strip_prefix("kernel="))?;
    SLOTS.iter().position(|s| s.kernel == kernel.trim())
}

/// The config with its kernel, initramfs and cmdline lines set for `slot`.
fn config_for(config: &str, slot: usize, cmdline: Option<&str>) -> String {
    let mut out = String::new();
    for line in config.lines() {
        let t = line.trim();
        if t.starts_with("kernel=") {
            out += &format!("kernel={}\n", SLOTS[slot].kernel);
        } else if t.starts_with("initramfs ") {
            out += &format!("initramfs {} followkernel\n", SLOTS[slot].initramfs);
        } else if t.starts_with("cmdline=") {
            // replaced below
        } else {
            out += line;
            out.push('\n');
        }
    }
    if let Some(cmdline) = cmdline {
        out += &format!("cmdline={cmdline}\n");
    }
    out
}

fn in_tryboot() -> bool {
    fs::read_to_string("/scheme/sys/env").is_ok_and(|env| env.lines().any(|l| l.trim() == TRY_MARK))
}

fn server_url(arg: Option<&String>) -> io::Result<String> {
    if let Some(url) = arg {
        return Ok(url.trim_end_matches('/').to_string());
    }
    let conf = fs::read_to_string(CONF)
        .map_err(|e| io::Error::new(e.kind(), format!("{CONF}: {e} (or give the URL)")))?;
    conf.lines()
        .find_map(|l| l.trim().strip_prefix("url="))
        .map(|u| u.trim().trim_end_matches('/').to_string())
        .ok_or_else(|| io::Error::other(format!("{CONF} has no url= line")))
}

/// GET over plain HTTP/1.0; calls `sink` with the body in pieces.
fn http_get(url: &str, mut sink: impl FnMut(&[u8]) -> io::Result<()>) -> io::Result<u64> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| io::Error::other(format!("{url}: only http:// is supported")))?;
    let (host, path) = rest.split_once('/').map(|(h, p)| (h, format!("/{p}"))).unwrap_or((rest, "/".into()));
    let addr = if host.contains(':') { host.to_string() } else { format!("{host}:80") };
    let mut stream = TcpStream::connect(&addr)?;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: {host}\r\n\r\n")?;
    let mut reader = BufReader::with_capacity(1 << 16, stream);
    let mut status = String::new();
    reader.read_line(&mut status)?;
    if status.split_whitespace().nth(1) != Some("200") {
        return Err(io::Error::other(format!("{url}: {}", status.trim())));
    }
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line.trim().is_empty() {
            break;
        }
    }
    let mut buf = vec![0; 1 << 16];
    let mut total = 0;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            return Ok(total);
        }
        sink(&buf[..n])?;
        total += n as u64;
    }
}

fn http_text(url: &str) -> io::Result<String> {
    let mut body = Vec::new();
    http_get(url, |b| {
        body.extend_from_slice(b);
        Ok(())
    })?;
    String::from_utf8(body).map_err(io::Error::other)
}

/// Downloads `url` to `dest`, returns the SHA-256 in hex.
fn download(url: &str, dest: &str) -> io::Result<String> {
    let mut file = fs::File::create(dest)?;
    let mut hash = Sha256::new();
    let size = http_get(url, |b| {
        hash.update(b);
        file.write_all(b)
    })?;
    file.sync_all()?;
    eprintln!("rpi-update: {url}: {} MiB", size >> 20);
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn status(url: Option<&String>) -> io::Result<()> {
    let config = fs::read_to_string(boot("config.txt"))?;
    let committed = slot_of(&config);
    for (i, slot) in SLOTS.iter().enumerate() {
        let size = fs::metadata(boot(slot.kernel)).map(|m| m.len()).ok();
        println!(
            "slot {}: {} {}{}",
            ["a", "b"][i],
            slot.kernel,
            size.map_or("missing".into(), |s| format!("{} KiB", s >> 10)),
            if committed == Some(i) { ", in config.txt" } else { "" }
        );
    }
    if let Ok(t) = fs::read_to_string(boot("tryboot.txt")) {
        println!("tryboot.txt: slot {}", slot_of(&t).map_or("?", |i| ["a", "b"][i]));
    }
    println!("running: {}", if in_tryboot() { "tryboot (not committed)" } else { "config.txt" });
    if let Ok(rel) = fs::read_to_string("/etc/redox-rpi-release") {
        if let Some(id) = rel.lines().find_map(|l| l.strip_prefix("BUILD_ID=")) {
            println!("this build: {}", id.trim_matches('"'));
        }
    }
    if let Ok(url) = server_url(url) {
        match http_text(&format!("{url}/BUILD_ID")) {
            Ok(id) => println!("server build: {} ({url})", id.trim()),
            Err(e) => println!("server: {e}"),
        }
    }
    Ok(())
}

fn fetch(url: Option<&String>) -> io::Result<()> {
    if in_tryboot() {
        return Err(io::Error::other("running a tryboot start: commit or reboot first"));
    }
    let url = server_url(url)?;
    let config = fs::read_to_string(boot("config.txt"))?;
    let current = slot_of(&config).ok_or_else(|| io::Error::other("config.txt starts no known slot"))?;
    let new = 1 - current;
    let sums = http_text(&format!("{url}/SHA256SUMS"))?;
    let expected = |name: &str| {
        sums.lines()
            .find_map(|l| l.split_once(char::is_whitespace).filter(|(_, n)| n.trim().trim_start_matches('*') == name))
            .map(|(h, _)| h.to_string())
            .ok_or_else(|| io::Error::other(format!("SHA256SUMS has no {name}")))
    };
    let id = http_text(&format!("{url}/BUILD_ID")).unwrap_or_default();
    eprintln!("rpi-update: {} into slot {}", id.trim(), ["a", "b"][new]);
    // Old tryboot files first, so a half-written slot is never started.
    let _ = fs::remove_file(boot("tryboot.txt"));
    for (remote, local) in [("kernel8.img", SLOTS[new].kernel), ("initramfs", SLOTS[new].initramfs)] {
        let got = download(&format!("{url}/{remote}"), &boot(local))?;
        if got != expected(remote)? {
            let _ = fs::remove_file(boot(local));
            return Err(io::Error::other(format!("{remote}: checksum does not match SHA256SUMS")));
        }
    }
    let cmdline = fs::read_to_string(boot("cmdline.txt")).unwrap_or_default();
    fs::write(boot("cmdline-try.txt"), format!("{} {TRY_MARK}\n", cmdline.trim()).trim_start())?;
    fs::write(boot("tryboot.txt"), config_for(&config, new, Some("cmdline-try.txt")))?;
    println!("slot {} ready; `rpi-update try` starts it once", ["a", "b"][new]);
    Ok(())
}

/// Sets the tryboot flag through rpi-firmwared's raw property file.
fn set_tryboot_flag() -> io::Result<()> {
    let words: [u32; 7] = [7 * 4, 0, TAG_SET_REBOOT_FLAGS, 4, 0, 1, 0];
    let msg: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let prop = fs::OpenOptions::new().read(true).write(true).open("/scheme/rpi-firmware/property")?;
    prop.write_at(&msg, 0)?;
    let mut answer = vec![0; msg.len()];
    let n = prop.read_at(&mut answer, 0)?;
    answer.truncate(n);
    let word = |i: usize| answer.get(i * 4..i * 4 + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    // Response code 0x80000000, tag answered (bit 31 of the value length).
    if word(1) != Some(0x8000_0000) || word(4).is_none_or(|v| v & 0x8000_0000 == 0) {
        return Err(io::Error::other(format!("the firmware did not take the tryboot flag ({answer:02x?})")));
    }
    Ok(())
}

fn try_boot() -> io::Result<()> {
    let t = fs::read_to_string(boot("tryboot.txt")).map_err(|e| io::Error::new(e.kind(), format!("tryboot.txt: {e}, run fetch first")))?;
    let slot = slot_of(&t).ok_or_else(|| io::Error::other("tryboot.txt starts no known slot"))?;
    set_tryboot_flag()?;
    eprintln!("rpi-update: rebooting into slot {} once", ["a", "b"][slot]);
    process::Command::new("reboot").status()?;
    Err(io::Error::other("reboot returned"))
}

fn commit() -> io::Result<()> {
    if !in_tryboot() {
        println!("not a tryboot start, nothing to commit");
        return Ok(());
    }
    // At boot the service may start before fatfsd has mounted the card.
    for _ in 0..30 {
        if fs::metadata(boot("config.txt")).is_ok() {
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
    let t = fs::read_to_string(boot("tryboot.txt"))?;
    let slot = slot_of(&t).ok_or_else(|| io::Error::other("tryboot.txt starts no known slot"))?;
    let config = fs::read_to_string(boot("config.txt"))?;
    fs::write(boot("config.txt"), config_for(&config, slot, None))?;
    fs::remove_file(boot("tryboot.txt"))?;
    let _ = fs::remove_file(boot("cmdline-try.txt"));
    println!("slot {} committed", ["a", "b"][slot]);
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("status") => status(args.get(2)),
        Some("fetch") => fetch(args.get(2)),
        Some("try") => try_boot(),
        Some("commit") => commit(),
        _ => {
            eprintln!("usage: rpi-update status|fetch [<url>]|try|commit");
            process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("rpi-update: {e}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "arm_64bit=1\nkernel=kernel8.img\ninitramfs initramfs followkernel\n";

    #[test]
    fn slots() {
        assert_eq!(slot_of(CONFIG), Some(0));
        let b = config_for(CONFIG, 1, Some("cmdline-try.txt"));
        assert_eq!(b, "arm_64bit=1\nkernel=kernel8-b.img\ninitramfs initramfs-b followkernel\ncmdline=cmdline-try.txt\n");
        assert_eq!(slot_of(&b), Some(1));
        assert_eq!(config_for(&b, 1, None), "arm_64bit=1\nkernel=kernel8-b.img\ninitramfs initramfs-b followkernel\n");
    }
}
