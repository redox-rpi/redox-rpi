//! devshd: a development shell over TCP for Redox.
//!
//! Listens on a TCP port (default 2323) and, for every connection, runs
//! `login` on a fresh pseudo-terminal and copies bytes between the socket
//! and the terminal. No encryption and no telnet protocol: meant for a
//! development board on the home network until SSH works. Connect with
//!
//!     socat -,raw,echo=0 tcp:<address>:2323
//!
//! or, line by line, with `nc <address> 2323`.

use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

extern "C" {
    fn grantpt(fd: libc::c_int) -> libc::c_int;
    fn unlockpt(fd: libc::c_int) -> libc::c_int;
    fn ptsname(fd: libc::c_int) -> *const libc::c_char;
}

/// Open a pseudo-terminal: the master side and the path of the slave side.
fn open_pty() -> io::Result<(File, String)> {
    let master = OpenOptions::new().read(true).write(true).open("/dev/ptmx")?;
    let fd = master.as_raw_fd();
    let size = libc::winsize {
        ws_row: 30,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        libc::ioctl(fd, libc::TIOCSWINSZ, &size);
        if grantpt(fd) != 0 || unlockpt(fd) != 0 {
            return Err(io::Error::last_os_error());
        }
        let name = ptsname(fd);
        if name.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok((master, CStr::from_ptr(name).to_string_lossy().into_owned()))
    }
}

fn serve(stream: TcpStream) -> io::Result<()> {
    let (master, tty) = open_pty()?;
    let slave = || OpenOptions::new().read(true).write(true).open(&tty);
    let mut child = Command::new("login")
        .stdin(Stdio::from(slave()?))
        .stdout(Stdio::from(slave()?))
        .stderr(Stdio::from(slave()?))
        .env("TERM", "xterm")
        .env("TTY", &tty)
        .spawn()?;

    // Network to terminal. When the client goes away without logging out,
    // hang up the session; otherwise login and the shell would wait for
    // input forever.
    let mut from_net = stream.try_clone()?;
    let mut to_pty = master.try_clone()?;
    let login_pid = child.id() as libc::pid_t;
    let done = Arc::new(AtomicBool::new(false));
    let net_done = done.clone();
    thread::spawn(move || {
        let _ = io::copy(&mut from_net, &mut to_pty);
        hang_up(&to_pty, login_pid, &net_done);
    });

    // Terminal to network, until login and the shell are gone.
    let mut from_pty = master;
    let mut to_net = stream.try_clone()?;
    let copier = thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = from_pty.read(&mut buf) {
            if n == 0 || to_net.write_all(&buf[..n]).is_err() {
                break;
            }
        }
    });

    let _ = child.wait();
    done.store(true, Ordering::SeqCst);
    let _ = stream.shutdown(Shutdown::Both);
    let _ = copier.join();
    Ok(())
}

/// End a session whose client is gone: SIGHUP to the terminal's foreground
/// process group (the shell, which moves itself into its own group, or the
/// program it runs), then SIGKILL to that group and to `login` if they are
/// still there after a few seconds.
fn hang_up(master: &File, login_pid: libc::pid_t, done: &AtomicBool) {
    let fd = master.as_raw_fd();
    for round in 0..6 {
        if done.load(Ordering::SeqCst) {
            return;
        }
        let signal = if round < 3 { libc::SIGHUP } else { libc::SIGKILL };
        let pgrp = unsafe { libc::tcgetpgrp(fd) };
        unsafe {
            if pgrp > 0 {
                libc::kill(-pgrp, signal);
            }
            if round >= 3 {
                libc::kill(login_pid, signal);
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
}

/// Bind the port, trying again every 5 s: after a rebind the old socket
/// may not be gone at once.
fn listen(port: u16, first: bool) -> TcpListener {
    loop {
        match TcpListener::bind(("0.0.0.0", port)) {
            Ok(listener) => {
                eprintln!("devshd: listening on port {port}");
                return listener;
            }
            Err(err) if first => {
                eprintln!("devshd: cannot listen on port {port}: {err}");
                std::process::exit(1);
            }
            Err(err) => {
                eprintln!("devshd: cannot listen on port {port} again: {err}");
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(2323);
    let mut listener = listen(port, true);
    // Accept errors used to be skipped without a word, and an error that
    // stays made the loop spin while clients were refused (#129). Now they
    // are logged, and after 20 in a row the port is bound again.
    let mut errors = 0u32;
    loop {
        let stream = match listener.accept() {
            Ok((stream, _)) => {
                errors = 0;
                stream
            }
            Err(err) => {
                errors += 1;
                if errors == 1 || errors % 20 == 0 {
                    eprintln!("devshd: accept failed ({errors} in a row): {err}");
                }
                if errors % 20 == 0 {
                    drop(listener);
                    thread::sleep(Duration::from_secs(1));
                    listener = listen(port, false);
                } else {
                    thread::sleep(Duration::from_millis(100));
                }
                continue;
            }
        };
        // No output per connection: nobody may read it after boot. A thread
        // that cannot start (out of memory) is reported, not a panic.
        let started = thread::Builder::new().spawn(move || {
            let _ = serve(stream);
        });
        if let Err(err) = started {
            eprintln!("devshd: cannot start a session: {err}");
        }
    }
}
