//! Checks the termios speed functions of relibc on a serial port: sets a
//! speed with `cfsetspeed` and `tcsetattr`, reads it back with `tcgetattr`,
//! `cfgetospeed` and `cfgetispeed`, and checks that `cfsetspeed` refuses a
//! speed that is not a `B*` constant.
//!
//! Usage: termios-check <device> <speed>, e.g.
//!   termios-check /scheme/usb-serial/1.2 9600
//! Exit code 0 if every step passed.

use std::{env, fs::OpenOptions, io, os::fd::AsRawFd, process::ExitCode};

/// struct termios as relibc defines it on Redox.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_cc: [u8; 32],
}

extern "C" {
    fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, actions: i32, termios: *const Termios) -> i32;
    fn cfsetspeed(termios: *mut Termios, speed: u32) -> i32;
    fn cfgetospeed(termios: *const Termios) -> u32;
    fn cfgetispeed(termios: *const Termios) -> u32;
}

const TCSANOW: i32 = 0;

/// Baud rates and their `B*` constants on Redox (relibc header/termios).
const SPEEDS: [(u32, u32); 31] = [
    (0, 0o0), (50, 0o1), (75, 0o2), (110, 0o3), (134, 0o4), (150, 0o5),
    (200, 0o6), (300, 0o7), (600, 0o10), (1200, 0o11), (1800, 0o12),
    (2400, 0o13), (4800, 0o14), (9600, 0o15), (19200, 0o16), (38400, 0o17),
    (57600, 0o20), (115200, 0o21), (230400, 0o22), (460800, 0o23),
    (500000, 0o24), (576000, 0o25), (921600, 0o26), (1000000, 0o27),
    (1152000, 0o30), (1500000, 0o31), (2000000, 0o32), (2500000, 0o33),
    (3000000, 0o34), (3500000, 0o35), (4000000, 0o36),
];

fn baud(constant: u32) -> Option<u32> {
    SPEEDS.iter().find(|s| s.1 == constant).map(|s| s.0)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let (Some(device), Some(rate)) = (args.get(1), args.get(2).and_then(|s| s.parse::<u32>().ok())) else {
        eprintln!("usage: termios-check <device> <speed>");
        return ExitCode::from(2);
    };
    let Some(&(_, constant)) = SPEEDS.iter().find(|s| s.0 == rate) else {
        eprintln!("termios-check: {rate} is not a standard speed");
        return ExitCode::from(2);
    };
    let file = match OpenOptions::new().read(true).write(true).open(device) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("termios-check: {device}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let fd = file.as_raw_fd();
    let mut ok = true;
    let mut check = |name: &str, passed: bool, detail: String| {
        println!("{} {name}{detail}", if passed { "ok  " } else { "FAIL" });
        ok &= passed;
    };

    let mut termios = Termios::default();
    let r = unsafe { tcgetattr(fd, &mut termios) };
    check("tcgetattr", r == 0, if r == 0 { String::new() } else { format!(": {}", io::Error::last_os_error()) });
    let before = baud(unsafe { cfgetospeed(&termios) });
    println!("     speed before: {before:?}");

    let r = unsafe { cfsetspeed(&mut termios, constant) };
    check("cfsetspeed", r == 0, if r == 0 { String::new() } else { format!(": {}", io::Error::last_os_error()) });
    let r = unsafe { tcsetattr(fd, TCSANOW, &termios) };
    check("tcsetattr", r == 0, if r == 0 { String::new() } else { format!(": {}", io::Error::last_os_error()) });

    let mut back = Termios::default();
    unsafe { tcgetattr(fd, &mut back) };
    let out = baud(unsafe { cfgetospeed(&back) });
    let inp = baud(unsafe { cfgetispeed(&back) });
    check("cfgetospeed after tcgetattr", out == Some(rate), format!(": {out:?}"));
    check("cfgetispeed after tcgetattr", inp == Some(rate), format!(": {inp:?}"));

    let mut scratch = back;
    let r = unsafe { cfsetspeed(&mut scratch, 12345) };
    let err = io::Error::last_os_error().raw_os_error();
    check("cfsetspeed refuses 12345", r == -1 && err == Some(22), format!(": returned {r}, errno {err:?}"));

    if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
