//! `volume`: show, set and keep the volume of `audiod` (#336).
//!
//! ```text
//! volume              the volume, 0 to 100
//! volume 70           set it; +10 and -10 change it, mute sets 0
//! volume restore      set the saved volume (boot service)
//! ```
//!
//! `audiod` applies the volume as a cube and starts at 50. Every change is
//! saved in `/etc/audio-volume`; on network and direct boot, where `/`
//! lives in RAM, in `/scheme/data/audio-volume` when the data partition is
//! there.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

const SCHEME: &str = "/scheme/audio/volume";
const USAGE: &str = "usage: volume [<0-100> | +<n> | -<n> | mute | restore]";

/// Where the volume is kept: the data partition on images that have one
/// (they run / from RAM), else /etc.
fn saved_path() -> &'static str {
    if Path::new("/usr/lib/init.d/60_data.service").exists() && Path::new("/scheme/data").is_dir() {
        "/scheme/data/audio-volume"
    } else {
        "/etc/audio-volume"
    }
}

fn read_volume(path: &str) -> Result<i32, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
    text.trim().parse().map_err(|_| format!("{path}: not a number: {}", text.trim()))
}

fn set_volume(value: i32) -> Result<(), String> {
    fs::write(SCHEME, value.to_string()).map_err(|err| format!("{SCHEME}: {err}"))
}

fn run(arg: Option<&str>) -> Result<(), String> {
    let Some(arg) = arg else {
        println!("{}", read_volume(SCHEME)?);
        return Ok(());
    };
    if arg == "restore" {
        let path = saved_path();
        if !Path::new(path).exists() {
            return Ok(());
        }
        let value = read_volume(path)?;
        set_volume(value.clamp(0, 100))?;
        println!("volume: {value} from {path}");
        return Ok(());
    }
    let value = match arg {
        "mute" => 0,
        _ if arg.starts_with('+') || arg.starts_with('-') => {
            let step: i32 = arg.parse().map_err(|_| USAGE.to_string())?;
            read_volume(SCHEME)? + step
        }
        _ => arg.parse().map_err(|_| USAGE.to_string())?,
    };
    let value = value.clamp(0, 100);
    set_volume(value)?;
    let path = saved_path();
    if let Err(err) = fs::write(path, format!("{value}\n")) {
        eprintln!("volume: not saved, {path}: {err}");
    }
    println!("{value}");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() > 1 || matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    match run(args.first().map(String::as_str)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("volume: {err}");
            ExitCode::FAILURE
        }
    }
}
