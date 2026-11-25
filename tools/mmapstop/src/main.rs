//! mmapstop: repro for the kernel `todo!`s around scheme mmaps (#678).
//!
//! A child process maps a file from a user scheme (default: a new file in
//! /tmp) with MAP_PRIVATE and PROT_WRITE, and writes to it. The kernel maps
//! the scheme's pages read-only (copy on write); the write fault asks the
//! scheme for the page (RequestMmap) and blocks the child in `AwaitingMmap`.
//! Then the parent signals the blocked child:
//!
//! - `stop` (default): SIGSTOP, then SIGKILL. The kernel reached
//!   `todo!()` in `scheme/proc.rs` (Stop while AwaitingMmap).
//! - `kill`: SIGKILL only. When the child wakes without a page, the fault
//!   ends in `NonfatalInternalError` and `todo!()` in `memory/mod.rs`.
//! - `none`: no signal; only report whether the write blocks.
//!
//! `-s` maps with MAP_SHARED instead (expected: no block). `-f PATH` maps
//! another file.
//!
//! Exit codes: 0 the write did not block; 3 the write blocked and the child
//! was reaped after the signals; 4 the child could not be reaped (a hang);
//! 5 setup failed (open, mmap). A kernel panic stops the system instead.

use std::ffi::CString;
use std::io::Write;
use std::time::{Duration, Instant};

const PAGE: usize = 4096;

fn say(msg: &str) {
    println!("mmapstop: {msg}");
    let _ = std::io::stdout().flush();
}

fn fail(msg: &str) -> ! {
    say(&format!("{msg}: {}", std::io::Error::last_os_error()));
    std::process::exit(5);
}

/// The child: map the file and write to its first page.
fn child(path: &CString, shared: bool) -> ! {
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR) };
    if fd < 0 {
        fail("child: open");
    }
    let flags = if shared { libc::MAP_SHARED } else { libc::MAP_PRIVATE };
    let p = unsafe {
        libc::mmap(std::ptr::null_mut(), PAGE, libc::PROT_READ | libc::PROT_WRITE, flags, fd, 0)
    };
    if p == libc::MAP_FAILED {
        fail("child: mmap");
    }
    let p = p as *mut u8;
    say(&format!("child: mapped, first byte {}, writing", unsafe { std::ptr::read_volatile(p) }));
    unsafe { std::ptr::write_volatile(p, b'W') };
    say("child: write done");
    std::process::exit(0);
}

/// Wait up to `secs` for the child; Some(status) if it ended.
fn wait_for(pid: libc::pid_t, secs: u64) -> Option<i32> {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        let mut status = 0;
        let r = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if r == pid {
            return Some(status);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

fn main() {
    let mut action = "stop".to_string();
    let mut shared = false;
    let mut path = "/tmp/mmapstop.dat".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-s" => shared = true,
            "-f" => path = args.next().unwrap_or_else(|| fail("-f needs a path")),
            "stop" | "kill" | "none" => action = a,
            _ => {
                say("usage: mmapstop [-s] [-f PATH] [stop|kill|none]");
                std::process::exit(5);
            }
        }
    }
    if let Err(e) = std::fs::write(&path, vec![b'.'; PAGE]) {
        say(&format!("cannot write {path}: {e}"));
        std::process::exit(5);
    }
    let cpath = CString::new(path.clone()).unwrap();
    say(&format!("{} mapping of {path}, action {action}", if shared { "shared" } else { "private" }));

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        fail("fork");
    }
    if pid == 0 {
        child(&cpath, shared);
    }

    if let Some(status) = wait_for(pid, 3) {
        say(&format!("the write did not block (child status {status:#x})"));
        let _ = std::fs::remove_file(&path);
        std::process::exit(0);
    }
    say(&format!("child {pid} is blocked in the write fault (AwaitingMmap)"));

    match action.as_str() {
        "stop" => {
            say("sending SIGSTOP");
            unsafe { libc::kill(pid, libc::SIGSTOP) };
            std::thread::sleep(Duration::from_secs(1));
            say("sending SIGKILL");
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
        "kill" => {
            say("sending SIGKILL");
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
        _ => {
            say("no signal sent; the child stays blocked");
            std::process::exit(3);
        }
    }
    let code = match wait_for(pid, 5) {
        Some(status) => {
            say(&format!("child reaped (status {status:#x})"));
            3
        }
        None => {
            say("child not reaped after 5 s (hang)");
            4
        }
    };
    let _ = std::fs::remove_file(&path);
    std::process::exit(code);
}
