//! hwinfo: what this Raspberry Pi is and how it is doing.
//!
//! Board, memory, CPUs, ARM clock and governor, temperature and throttling
//! come from `rpi-firmwared` (`/scheme/rpi-firmware`), the CPU count from
//! `/scheme/sys/stat`, the build from `/etc/redox-rpi-release` and the USB
//! devices from `dwc2d`. Missing sources are skipped.

use std::fs;

const FW: &str = "/scheme/rpi-firmware";

fn read(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// The value of `key` in `key value` lines.
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.strip_prefix(key)?.strip_prefix(' ')).map(str::trim)
}

fn mhz(hz: &str) -> String {
    match hz.trim().parse::<u64>() {
        Ok(hz) => format!("{} MHz", hz / 1_000_000),
        Err(_) => hz.trim().to_string(),
    }
}

/// The bits of the firmware's throttling flags, as Linux's `vcgencmd
/// get_throttled` explains them.
fn throttled(flags: u32) -> String {
    const BITS: [(u32, &str); 4] = [
        (0, "undervoltage"),
        (1, "ARM clock capped"),
        (2, "throttled"),
        (3, "soft temperature limit"),
    ];
    let now: Vec<&str> = BITS.iter().filter(|(bit, _)| flags & (1 << bit) != 0).map(|(_, n)| *n).collect();
    let since: Vec<&str> = BITS.iter().filter(|(bit, _)| flags & (1 << (bit + 16)) != 0).map(|(_, n)| *n).collect();
    let mut text = if now.is_empty() { "none now".to_string() } else { format!("now: {}", now.join(", ")) };
    if !since.is_empty() {
        text += &format!("; since boot: {}", since.join(", "));
    }
    format!("0x{flags:08x} ({text})")
}

fn row(key: &str, value: impl AsRef<str>) {
    println!("{key:<12} {}", value.as_ref());
}

fn main() {
    if let Some(release) = read("/etc/redox-rpi-release") {
        let value = |key: &str| {
            release.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix('=')).map(|v| v.trim_matches('"').to_string())
        };
        if let Some(id) = value("BUILD_ID") {
            row("build", id);
        }
        if let Some(experiments) = value("EXPERIMENTS").filter(|e| !e.is_empty()) {
            row("experiments", experiments);
        }
    }

    match read(&format!("{FW}/board")) {
        Some(board) => {
            if let Some(model) = field(&board, "model") {
                row("model", model);
            }
            if let Some(revision) = field(&board, "revision") {
                row("revision", revision);
            }
            if let Some(serial) = field(&board, "serial") {
                row("serial", serial);
            }
            if let Some(memory) = field(&board, "memory") {
                row("memory", memory);
            }
            for (key, name) in [("arm_memory", "ARM memory"), ("vc_memory", "GPU memory")] {
                if let Some((_, size)) = field(&board, key).and_then(|v| v.split_once(' ')) {
                    if let Ok(size) = size.parse::<u64>() {
                        row(name, format!("{} MiB", size >> 20));
                    }
                }
            }
            if let Some(firmware) = field(&board, "firmware") {
                row("firmware", firmware);
            }
        }
        None => row("board", format!("unknown ({FW} missing: not a Pi, or rpi-firmwared not running)")),
    }

    if let Some(stat) = read("/scheme/sys/stat") {
        let cpus = stat
            .lines()
            .filter(|l| l.strip_prefix("cpu").and_then(|r| r.split_whitespace().next()).is_some_and(|n| n.parse::<u32>().is_ok()))
            .count();
        row("CPUs", cpus.to_string());
    }

    if let Some(clock) = read(&format!("{FW}/arm_clock")) {
        let max = read(&format!("{FW}/clocks"))
            .and_then(|c| c.lines().find_map(|l| l.strip_prefix("arm ")?.split_whitespace().nth(1).map(mhz)));
        match max {
            Some(max) => row("ARM clock", format!("{} (max {max})", mhz(&clock))),
            None => row("ARM clock", mhz(&clock)),
        }
    }
    if let Some(governor) = read(&format!("{FW}/governor")) {
        let mode = field(&governor, "mode").unwrap_or("?");
        let load = field(&governor, "load").unwrap_or("?");
        row("governor", format!("{mode}, load {load} %"));
    }
    if let Some(clocks) = read(&format!("{FW}/clocks")) {
        let others: Vec<String> = clocks
            .lines()
            .filter_map(|l| {
                let mut f = l.split_whitespace();
                let name = f.next()?;
                let rate = f.next()?;
                (name != "arm").then(|| format!("{name} {}", mhz(rate)))
            })
            .collect();
        if !others.is_empty() {
            row("clocks", others.join(", "));
        }
    }
    if let Some(temperature) = read(&format!("{FW}/temperature")) {
        if let Ok(milli) = temperature.trim().parse::<u32>() {
            row("temperature", format!("{}.{} °C", milli / 1000, milli % 1000 / 100));
        }
    }
    if let Some(flags) = read(&format!("{FW}/throttled")) {
        if let Ok(flags) = u32::from_str_radix(flags.trim().trim_start_matches("0x"), 16) {
            row("throttling", throttled(flags));
        }
    }

    match hwinfo::usb_devices() {
        Ok(devices) => row("USB", format!("{} devices (lsusb for the list)", devices.len())),
        Err(_) => row("USB", "no device list"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_throttling() {
        assert_eq!(throttled(0), "0x00000000 (none now)");
        assert_eq!(throttled(0x0008_0008), "0x00080008 (now: soft temperature limit; since boot: soft temperature limit)");
        assert_eq!(throttled(0x0005_0000), "0x00050000 (none now; since boot: undervoltage, throttled)");
    }

    #[test]
    fn reads_fields() {
        let board = "revision 0x00a020d3\nmodel Pi 3 Model B+\nmemory 1 GiB\narm_memory 0x00000000 1006632960\n";
        assert_eq!(field(board, "model"), Some("Pi 3 Model B+"));
        assert_eq!(field(board, "memory"), Some("1 GiB"));
        assert_eq!(field(board, "mem"), None);
    }
}
