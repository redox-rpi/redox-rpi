//! `i2c`: I2C devices through `/scheme/i2c` (bcm2835-i2cd), like the
//! i2c-tools of Linux. Addresses, registers and values are hexadecimal.
//!
//! ```text
//! i2c detect                     addresses that answer, as a table
//! i2c dump <addr>                all 256 registers
//! i2c get <addr> <reg> [count]   count registers from reg on (default 1)
//! i2c set <addr> <reg> <byte>... write bytes from reg on
//! ```
//!
//! The bus is 1 (header pins 3 and 5) unless `-b <bus>` comes first.

use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::process::ExitCode;

const USAGE: &str = "usage: i2c [-b bus] detect | dump <addr> | get <addr> <reg> [count] | set <addr> <reg> <byte>...";

fn hex(text: &str, max: u32, what: &str) -> Result<u32, String> {
    let digits = text.strip_prefix("0x").unwrap_or(text);
    match u32::from_str_radix(digits, 16) {
        Ok(value) if value <= max => Ok(value),
        _ => Err(format!("bad {what} {text:?} (hex, at most {max:x})")),
    }
}

fn device(bus: &str, address: &str, write: bool) -> Result<File, String> {
    let address = hex(address, 0x77, "address")?;
    let path = format!("/scheme/i2c/{bus}/{address:02x}");
    OpenOptions::new().read(true).write(write).open(&path).map_err(|err| format!("{path}: {err}"))
}

fn read(file: &File, reg: u32, count: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0; count];
    let len = file.read_at(&mut buf, u64::from(reg)).map_err(|err| format!("read: {err}"))?;
    buf.truncate(len);
    Ok(buf)
}

fn detect(bus: &str) -> Result<(), String> {
    let path = format!("/scheme/i2c/{bus}/scan");
    let scan = std::fs::read_to_string(&path).map_err(|err| format!("{path}: {err}"))?;
    let found: Vec<u32> = scan.lines().filter_map(|line| u32::from_str_radix(line, 16).ok()).collect();
    println!("     0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f");
    for row in (0..0x80).step_by(16) {
        let mut line = format!("{row:02x}:");
        for address in row..row + 16 {
            if !(0x03..=0x77).contains(&address) {
                line.push_str("   ");
            } else if found.contains(&address) {
                line.push_str(&format!(" {address:02x}"));
            } else {
                line.push_str(" --");
            }
        }
        println!("{}", line.trim_end());
    }
    Ok(())
}

fn dump(bus: &str, address: &str) -> Result<(), String> {
    let data = read(&device(bus, address, false)?, 0, 256)?;
    println!("     0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f");
    for (row, chunk) in data.chunks(16).enumerate() {
        let bytes: Vec<String> = chunk.iter().map(|byte| format!("{byte:02x}")).collect();
        println!("{:02x}: {}", row * 16, bytes.join(" "));
    }
    Ok(())
}

fn run(args: &[String]) -> Result<(), String> {
    let (bus, args) = match args {
        [flag, bus, rest @ ..] if flag == "-b" => (bus.as_str(), rest),
        _ => ("1", args),
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args[..] {
        ["detect"] => detect(bus),
        ["dump", address] => dump(bus, address),
        ["get", address, reg] | ["get", address, reg, _] => {
            let reg = hex(reg, 0xff, "register")?;
            let count = match args.get(3) {
                Some(count) => hex(count, 0x100 - reg, "count")? as usize,
                None => 1,
            };
            let data = read(&device(bus, address, false)?, reg, count)?;
            let bytes: Vec<String> = data.iter().map(|byte| format!("{byte:02x}")).collect();
            println!("{}", bytes.join(" "));
            Ok(())
        }
        ["set", address, reg, ref bytes @ ..] if !bytes.is_empty() => {
            let reg = hex(reg, 0xff, "register")?;
            let data = bytes.iter().map(|byte| hex(byte, 0xff, "byte").map(|b| b as u8)).collect::<Result<Vec<_>, _>>()?;
            if reg as usize + data.len() > 256 {
                return Err("past register ff".to_string());
            }
            device(bus, address, true)?.write_all_at(&data, u64::from(reg)).map_err(|err| format!("write: {err}"))
        }
        _ => Err(USAGE.to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("i2c: {err}");
            ExitCode::FAILURE
        }
    }
}
