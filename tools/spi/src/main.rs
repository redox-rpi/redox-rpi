//! `spi`: SPI transfers through `/scheme/spi` (bcm2835-spid). Bytes are
//! hexadecimal.
//!
//! ```text
//! spi [-c 0|1] [-s hz] [-m 0-3] <byte>...   one transfer, prints the answer
//! spi [-c 0|1] [-s hz] [-m 0-3] -l <count>  loopback check: sends count
//!                                           bytes, compares the answer
//! spi info                                  speed and mode
//! ```
//!
//! `-s` and `-m` change the bus setting for everyone, and it stays.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::process::ExitCode;

const USAGE: &str = "usage: spi [-c 0|1] [-s hz] [-m 0-3] <byte>... | [-c 0|1] [-s hz] [-m 0-3] -l <count> | info";
const BUS: &str = "/scheme/spi/0";
const MAX_LEN: usize = 4096;

fn set(name: &str, value: &str) -> Result<(), String> {
    let path = format!("{BUS}/{name}");
    std::fs::write(&path, value).map_err(|err| format!("{path}: {err}"))
}

fn get(name: &str) -> Result<String, String> {
    let path = format!("{BUS}/{name}");
    std::fs::read_to_string(&path).map(|text| text.trim().to_string()).map_err(|err| format!("{path}: {err}"))
}

fn transfer(chip: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let path = format!("{BUS}/cs{chip}");
    let mut file = OpenOptions::new().read(true).write(true).open(&path).map_err(|err| format!("{path}: {err}"))?;
    file.write_all(data).map_err(|err| format!("transfer: {err}"))?;
    let mut received = Vec::new();
    file.read_to_end(&mut received).map_err(|err| format!("read: {err}"))?;
    Ok(received)
}

/// Bytes that are not all the same and not a short cycle, so a stuck or
/// shifted line shows.
fn pattern(count: usize) -> Vec<u8> {
    let mut x: u32 = 0x2545_f491;
    (0..count)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

fn run(args: &[String]) -> Result<(), String> {
    if args.len() == 1 && args[0] == "info" {
        println!("speed {} Hz, mode {}", get("speed")?, get("mode")?);
        return Ok(());
    }
    let mut chip = "0";
    let mut loopback = None;
    let mut bytes = Vec::new();
    let mut changed = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(USAGE.to_string());
        match arg.as_str() {
            "-c" => {
                chip = match value()?.as_str() {
                    "0" => "0",
                    "1" => "1",
                    other => return Err(format!("bad chip select {other:?} (0 or 1)")),
                }
            }
            "-s" => {
                set("speed", value()?)?;
                changed = true;
            }
            "-m" => {
                set("mode", value()?)?;
                changed = true;
            }
            "-l" => {
                let count = value()?;
                loopback = Some(count.parse::<usize>().ok().filter(|n| (1..=MAX_LEN).contains(n)).ok_or(format!("bad count {count:?} (1 to {MAX_LEN})"))?);
            }
            byte => {
                let digits = byte.strip_prefix("0x").unwrap_or(byte);
                bytes.push(u8::from_str_radix(digits, 16).map_err(|_| format!("bad byte {byte:?} (hex)"))?);
            }
        }
    }
    match (loopback, bytes.is_empty()) {
        (Some(count), true) => {
            let sent = pattern(count);
            let received = transfer(chip, &sent)?;
            let wrong = sent.iter().zip(&received).filter(|(a, b)| a != b).count() + count.abs_diff(received.len());
            println!("loopback: {count} bytes at {} Hz, {wrong} wrong", get("speed")?);
            if wrong == 0 { Ok(()) } else { Err("loopback failed (MOSI pin 19 to MISO pin 21?)".to_string()) }
        }
        (None, false) => {
            let received = transfer(chip, &bytes)?;
            let text: Vec<String> = received.iter().map(|byte| format!("{byte:02x}")).collect();
            println!("{}", text.join(" "));
            Ok(())
        }
        // Only -s or -m: a setting change.
        (None, true) if changed => {
            println!("speed {} Hz, mode {}", get("speed")?, get("mode")?);
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("spi: {USAGE}");
        return ExitCode::FAILURE;
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("spi: {err}");
            ExitCode::FAILURE
        }
    }
}
