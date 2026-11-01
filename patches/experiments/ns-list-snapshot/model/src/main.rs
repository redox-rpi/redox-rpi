// Model of initnsmgr's getdents on /scheme: names in a hashbrown HashMap,
// paged by position, 512-byte buffers as relibc's readdir uses. One scheme
// ("sudo") is registered between two getdents calls. Counts the cases in
// which "network.usb" is missing from the listing.
use hashbrown::HashMap;

const BASE: &[&str] = &[
    "chan", "debug", "disk.mmc", "display.vesa", "event", "fbcon.2", "file", "icmp", "initfs",
    "input", "ip", "irq", "kernel.acpi", "kernel.dtb", "log", "logging", "memory", "netcfg",
    "network.usb", "null", "pipe", "proc", "pty", "rand", "rpi-firmware", "serio", "shm", "sys",
    "tcp", "time", "udp", "uds_dgram", "uds_stream", "zero", "dwc2", "hwd", "lived", "inputd",
    "fbbootlog", "rtc", "audio", "pcid", "acpi", "vesa", "orbital", "ptyd", "ramfs.tmp",
    "keymap", "timesync", "devsh", "gpio", "i2c", "spi", "bt", "wifi", "usb-serial", "extra1",
    "extra2", "extra3", "extra4",
];

// Redox DirentHeader is 19 bytes, then the name and a NUL; entries 8-aligned.
fn entry_len(name: &str) -> usize {
    (19 + name.len() + 1 + 7) & !7
}

fn page(map: &HashMap<String, ()>, offset: usize) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut used = 0;
    for (i, (name, _)) in map.iter().enumerate().skip(offset) {
        let len = entry_len(name);
        if used + len > 512 && !out.is_empty() {
            break;
        }
        used += len;
        out.push((i + 1, name.clone()));
    }
    out
}

fn main() {
    let mut cases = 0;
    let mut missed = 0;
    for n in 15..=BASE.len() {
        let mut names: Vec<&str> = BASE[..n].to_vec();
        if !names.contains(&"network.usb") {
            names.push("network.usb");
        }
        // Which page the registration falls after.
        for after_page in 1..=4 {
            let mut map: HashMap<String, ()> = HashMap::new();
            for name in &names {
                map.insert(name.to_string(), ());
            }
            let mut seen = Vec::new();
            let mut offset = 0;
            let mut pages = 0;
            loop {
                let p = page(&map, offset);
                if p.is_empty() {
                    break;
                }
                offset = p.last().unwrap().0;
                seen.extend(p.into_iter().map(|(_, s)| s));
                pages += 1;
                if pages == after_page {
                    map.insert("sudo".to_string(), ());
                }
            }
            if pages < after_page {
                continue;
            }
            cases += 1;
            let lost: Vec<&&str> = names.iter().filter(|n| !seen.iter().any(|s| s == *n)).collect();
            if !lost.is_empty() {
                println!("{} schemes, sudo after page {}: {} missing {:?}", names.len(), after_page, lost.len(), lost);
            }
            if !seen.iter().any(|s| s == "network.usb") {
                missed += 1;
                println!("{} schemes, sudo after page {}: network.usb missing", names.len(), after_page);
            }
        }
    }
    println!("{missed} of {cases} cases miss network.usb");
}
