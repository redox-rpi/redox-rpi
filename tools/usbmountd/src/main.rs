//! usbmountd: mount USB sticks.
//!
//!     usbmountd              watch for USB disks and mount their FAT partitions
//!     usbmountd eject usb0   unmount /scheme/usb0, so the stick can be removed
//!     usbmountd list         the mounted sticks
//!
//! dwc2d serves a stick as the disk scheme `disk.usb-dwc2+<ports>-scsi`
//! (#25), with `0` for the whole stick and `0p0`, `0p1`, ... for its
//! partitions. Once a second usbmountd looks at `/scheme`: for a new USB disk
//! it starts `fatfsd <partition> usbN` for each FAT partition (or for the
//! whole stick if it has a FAT filesystem and no partition table), with the
//! lowest free N. When the disk goes away it stops that fatfsd (SIGTERM).
//!
//! `eject` sends fatfsd SIGTERM: it writes what is open, marks the volume
//! clean and ends; `eject` waits until `/scheme/usbN` is gone. The stick is
//! not mounted again until it is unplugged and plugged in again.
//!
//! The process IDs of the fatfsd instances are in `/tmp/usbmount/usbN.pid`.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::process::{self, Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};

const STATE_DIR: &str = "/tmp/usbmount";
const POLL: Duration = Duration::from_secs(1);
const EJECT_TIMEOUT: Duration = Duration::from_secs(15);

struct Mount {
    name: String,
    disk: String,
    child: Child,
    stopping: bool,
}

#[derive(PartialEq)]
enum Kind {
    Fat,
    ExFat,
    Other,
}

/// The names in `/scheme`.
fn schemes() -> BTreeSet<String> {
    fs::read_dir("/scheme")
        .map(|dir| dir.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default()
}

/// The partitions of a disk scheme in order, or the whole disk (`0`) if it
/// has none.
fn partitions(disk: &str) -> Vec<String> {
    let names: Vec<String> = match fs::read_dir(format!("/scheme/{disk}")) {
        Ok(dir) => dir.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect(),
        Err(_) => return Vec::new(),
    };
    let mut parts: Vec<(u32, String)> = names
        .iter()
        .filter_map(|n| n.strip_prefix("0p").and_then(|p| p.parse().ok()).map(|p| (p, n.clone())))
        .collect();
    parts.sort();
    if parts.is_empty() && names.iter().any(|n| n == "0") {
        return vec!["0".to_string()];
    }
    parts.into_iter().map(|(_, n)| n).collect()
}

/// What the first block of `path` holds. The disk is read in whole blocks.
fn kind(path: &str) -> Kind {
    let Ok(mut file) = File::open(path) else { return Kind::Other };
    let size = match file.metadata().map(|m| m.blksize()) {
        Ok(n) if (512..=65536).contains(&n) && n.is_power_of_two() => n as usize,
        _ => 512,
    };
    let mut b = vec![0; size];
    if file.read_exact(&mut b).is_err() || b[510] != 0x55 || b[511] != 0xAA {
        return Kind::Other;
    }
    if &b[3..11] == b"EXFAT   " {
        return Kind::ExFat;
    }
    let bytes_per_sector = u16::from_le_bytes([b[11], b[12]]);
    let sectors_per_cluster = b[13];
    let reserved = u16::from_le_bytes([b[14], b[15]]);
    let fats = b[16];
    let sane = matches!(bytes_per_sector, 512 | 1024 | 2048 | 4096)
        && sectors_per_cluster.is_power_of_two()
        && reserved > 0
        && (1..=2).contains(&fats);
    if sane {
        Kind::Fat
    } else {
        Kind::Other
    }
}

fn pid_file(name: &str) -> String {
    format!("{STATE_DIR}/{name}.pid")
}

fn free_name(mounts: &[Mount], present: &BTreeSet<String>) -> String {
    (0..)
        .map(|n| format!("usb{n}"))
        .find(|name| !present.contains(name) && !mounts.iter().any(|m| &m.name == name))
        .unwrap()
}

fn mount_new(disk: &str, mounts: &mut Vec<Mount>, present: &BTreeSet<String>) {
    for part in partitions(disk) {
        let path = format!("/scheme/{disk}/{part}");
        match kind(&path) {
            Kind::Fat => {}
            Kind::ExFat => {
                println!("usbmountd: {path}: exFAT is not supported, only FAT12/16/32");
                continue;
            }
            Kind::Other => continue,
        }
        let name = free_name(mounts, present);
        match Command::new("fatfsd").arg(&path).arg(&name).spawn() {
            Ok(child) => {
                println!("usbmountd: {path} (FAT) on /scheme/{name}");
                let _ = fs::write(pid_file(&name), format!("{}\n", child.id()));
                mounts.push(Mount { name, disk: disk.to_string(), child, stopping: false });
            }
            Err(err) => println!("usbmountd: fatfsd {path} {name}: {err}"),
        }
    }
}

fn daemon() -> ! {
    let _ = fs::create_dir_all(STATE_DIR);
    let mut mounts: Vec<Mount> = Vec::new();
    // USB disks seen: mounted, ejected or without FAT. A disk leaves the set
    // when it goes away, so plugging it in again mounts it again.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    println!("usbmountd: watching for USB disks");
    loop {
        let present = schemes();
        let disks: BTreeSet<String> = present.iter().filter(|n| n.starts_with("disk.usb")).cloned().collect();

        for m in mounts.iter_mut().filter(|m| !m.stopping && !disks.contains(&m.disk)) {
            println!("usbmountd: {} went away, stopping /scheme/{}", m.disk, m.name);
            unsafe { libc::kill(m.child.id() as libc::pid_t, libc::SIGTERM) };
            m.stopping = true;
        }
        mounts.retain_mut(|m| match m.child.try_wait() {
            Ok(Some(status)) => {
                println!("usbmountd: /scheme/{} ended ({status})", m.name);
                let _ = fs::remove_file(pid_file(&m.name));
                false
            }
            _ => true,
        });

        seen.retain(|d| disks.contains(d));
        for disk in &disks {
            if seen.insert(disk.clone()) {
                // driver-block reads the partition table when the scheme is
                // made; give a just registered disk a moment.
                sleep(Duration::from_millis(200));
                mount_new(disk, &mut mounts, &present);
            }
        }
        sleep(POLL);
    }
}

fn eject(name: &str) -> i32 {
    let pid: libc::pid_t = match fs::read_to_string(pid_file(name)).ok().and_then(|s| s.trim().parse().ok()) {
        Some(pid) => pid,
        None => {
            eprintln!("usbmountd: {name} is not mounted");
            return 1;
        }
    };
    if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
        eprintln!("usbmountd: {name}: fatfsd {pid} is not running");
        return 1;
    }
    let start = Instant::now();
    while schemes().contains(name) {
        if start.elapsed() > EJECT_TIMEOUT {
            eprintln!("usbmountd: /scheme/{name} is still there after {} s", EJECT_TIMEOUT.as_secs());
            return 1;
        }
        sleep(Duration::from_millis(100));
    }
    println!("usbmountd: {name} unmounted, the stick can be removed");
    0
}

fn list() -> i32 {
    let mut names: Vec<String> = fs::read_dir(STATE_DIR)
        .map(|dir| {
            dir.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".pid").map(String::from))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    for name in names {
        println!("/scheme/{name}");
    }
    0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        [] => daemon(),
        ["eject", name] => eject(name.trim_start_matches("/scheme/")),
        ["list"] => list(),
        _ => {
            eprintln!("usage: usbmountd | usbmountd eject usbN | usbmountd list");
            1
        }
    };
    process::exit(code);
}
