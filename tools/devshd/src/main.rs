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
use std::thread;

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

    // Network to terminal.
    let mut from_net = stream.try_clone()?;
    let mut to_pty = master.try_clone()?;
    thread::spawn(move || {
        let _ = io::copy(&mut from_net, &mut to_pty);
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
    let _ = stream.shutdown(Shutdown::Both);
    let _ = copier.join();
    Ok(())
}

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(2323);
    let listener = match TcpListener::bind(("0.0.0.0", port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("devshd: cannot listen on port {port}: {err}");
            std::process::exit(1);
        }
    };
    eprintln!("devshd: listening on port {port}");
    for stream in listener.incoming().flatten() {
        thread::spawn(move || {
            // No output per connection: nobody may read it after boot.
            let _ = serve(stream);
        });
    }
}
