//! `tone`: a test tone as raw audio for `/scheme/audio` (signed 16-bit
//! little endian, stereo, 44.1 kHz, the format `audiod` takes), and a check
//! of the `audiohw` contract.
//!
//! ```text
//! tone [-f hz] [-s seconds] [-a amplitude] [-c both|left|right] [-o file]
//!                     sine, to the file or to stdout:
//!                     tone -s 3 > /scheme/audio
//! tone sweep [-s seconds] [-a amplitude] [-o file]
//!                     100 Hz to 10 kHz, logarithmic
//! tone chime [-a amplitude] [-o file]
//!                     the startup sound: C, E, G and C an octave up, one
//!                     after the other, each with a soft attack and a long
//!                     fade, from left to right; about 1.8 s
//! tone check          the audiohw contract: a short write fails with
//!                     EINVAL, a full ring answers EWOULDBLOCK
//! ```
//!
//! The sound fades in and out over 10 ms against clicks. Default: 1000 Hz,
//! 2 s, amplitude 0.25 of full scale, both channels.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::mem::ManuallyDrop;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::FromRawFd;
use std::process::ExitCode;

const USAGE: &str = "usage: tone [sweep|chime] [-f hz] [-s seconds] [-a amplitude] [-c both|left|right] [-o file] | check";
const RATE: f64 = 44_100.0;
const CHUNK: usize = 2048;
const O_NONBLOCK: i32 = 0x0004_0000;

/// The chime's notes: start in seconds, frequency in Hz, place from left
/// (0) to right (1).
const CHIME: [(f64, f64, f64); 4] = [
    (0.00, 523.25, 0.30),
    (0.14, 659.26, 0.45),
    (0.28, 783.99, 0.60),
    (0.42, 1046.50, 0.70),
];
const CHIME_SECONDS: f64 = 1.8;

struct Options {
    sweep: bool,
    chime: bool,
    hz: f64,
    seconds: f64,
    amplitude: f64,
    left: bool,
    right: bool,
    output: Option<String>,
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut o = Options {
        sweep: false,
        chime: false,
        hz: 1000.0,
        seconds: 2.0,
        amplitude: 0.25,
        left: true,
        right: true,
        output: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(format!("{arg} needs a value"));
        let number = |text: &String| text.parse::<f64>().map_err(|_| format!("{arg}: not a number: {text}"));
        match arg.as_str() {
            "sweep" => o.sweep = true,
            "chime" => o.chime = true,
            "-f" => o.hz = number(value()?)?,
            "-s" => o.seconds = number(value()?)?,
            "-a" => o.amplitude = number(value()?)?,
            "-o" => o.output = Some(value()?.clone()),
            "-c" => match value()?.as_str() {
                "both" => (o.left, o.right) = (true, true),
                "left" => (o.left, o.right) = (true, false),
                "right" => (o.left, o.right) = (false, true),
                other => return Err(format!("-c: both, left or right, not {other}")),
            },
            _ => return Err(USAGE.to_string()),
        }
    }
    if !(o.hz > 0.0 && o.hz < RATE / 2.0) || !(o.seconds > 0.0) || !(0.0..=1.0).contains(&o.amplitude) {
        return Err("need 0 < hz < 22050, seconds > 0, 0 <= amplitude <= 1".to_string());
    }
    if o.chime {
        o.seconds = CHIME_SECONDS;
    }
    Ok(o)
}

/// The chime at `t` seconds, left and right, at most about 1. Each note is a
/// sine with a little of its octave, rising over 10 ms and fading with a
/// time constant of 0.5 s; the whole sound fades out over its last 0.3 s.
fn chime(t: f64) -> (f64, f64) {
    let (mut left, mut right) = (0.0, 0.0);
    for (start, hz, place) in CHIME {
        let t = t - start;
        if t < 0.0 {
            continue;
        }
        let envelope = (t / 0.01).min(1.0) * (-t / 0.5).exp();
        let w = 2.0 * std::f64::consts::PI * hz * t;
        let note = envelope * (w.sin() + 0.2 * (2.0 * w).sin());
        left += note * (1.0 - place);
        right += note * place;
    }
    let fade = ((CHIME_SECONDS - t) / 0.3).clamp(0.0, 1.0);
    // Up to four notes overlap; keep the sum below full scale.
    (left * fade / 2.0, right * fade / 2.0)
}

fn generate(o: &Options, out: &mut dyn Write) -> io::Result<()> {
    let frames = (o.seconds * RATE) as u64;
    let peak = o.amplitude * f64::from(i16::MAX);
    let mut phase = 0.0f64;
    let mut buf = Vec::with_capacity(CHUNK);
    for n in 0..frames {
        let t = n as f64 / RATE;
        if o.chime {
            let (left, right) = chime(t);
            for value in [left, right] {
                buf.extend_from_slice(&((peak * value) as i16).to_le_bytes());
            }
            if buf.len() >= CHUNK {
                out.write_all(&buf)?;
                buf.clear();
            }
            continue;
        }
        let hz = if o.sweep { 100.0 * 100f64.powf(t / o.seconds) } else { o.hz };
        phase = (phase + hz / RATE).fract();
        let fade = (t / 0.01).min((o.seconds - t) / 0.01).clamp(0.0, 1.0);
        let sample = (peak * fade * (2.0 * std::f64::consts::PI * phase).sin()) as i16;
        for on in [o.left, o.right] {
            buf.extend_from_slice(&(if on { sample } else { 0 }).to_le_bytes());
        }
        if buf.len() >= CHUNK {
            out.write_all(&buf)?;
            buf.clear();
        }
    }
    out.write_all(&buf)
}

fn check() -> Result<(), String> {
    let path = "/scheme/audiohw";
    let mut hw = OpenOptions::new()
        .write(true)
        .custom_flags(O_NONBLOCK)
        .open(path)
        .map_err(|err| format!("{path}: {err}"))?;
    match hw.write(&[0; 100]) {
        Err(err) if err.raw_os_error() == Some(22) => println!("short write: EINVAL (good)"),
        other => return Err(format!("short write: expected EINVAL, got {other:?}")),
    }
    // audiod keeps the ring full, so silence goes in until it is full.
    let silence = [0u8; CHUNK];
    let mut accepted = 0;
    loop {
        match hw.write(&silence) {
            Ok(n) if n == CHUNK => accepted += 1,
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
            other => return Err(format!("chunk write: {other:?}")),
        }
        if accepted > 64 {
            return Err("ring never full after 64 chunks".to_string());
        }
    }
    println!("full chunks accepted before EWOULDBLOCK: {accepted} (good)");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = if args.first().map(String::as_str) == Some("check") {
        check()
    } else {
        parse(&args).and_then(|o| {
            let result = match &o.output {
                Some(path) => File::create(path).and_then(|mut f| generate(&o, &mut f)),
                // Not io::stdout(): it is line buffered and would split the
                // samples at every 0x0a byte, and audiod takes whole frames
                // only.
                None => generate(&o, &mut *ManuallyDrop::new(unsafe { File::from_raw_fd(1) })),
            };
            result.map_err(|err| format!("write: {err}"))
        })
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("tone: {err}");
            ExitCode::FAILURE
        }
    }
}
