//! polltest [port]: listen on 0.0.0.0:<port> (default 2299), then poll the
//! listening socket for POLLIN five times with a 2 s timeout and try a
//! non-blocking accept after each. With no client, every poll should time
//! out (return 0) after about 2 s.

use std::net::TcpListener;
use std::os::unix::io::AsRawFd;
use std::time::Instant;

fn main() {
    let port: u16 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(2299);
    let listener = TcpListener::bind(("0.0.0.0", port)).expect("bind");
    listener.set_nonblocking(true).expect("nonblocking");
    let fd = listener.as_raw_fd();
    for i in 0..5 {
        let mut pfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
        let start = Instant::now();
        let ret = unsafe { libc::poll(&mut pfd, 1, 2000) };
        let ms = start.elapsed().as_millis();
        let accept = match listener.accept() {
            Ok((_, addr)) => format!("accepted {}", addr),
            Err(e) => format!("accept: {}", e),
        };
        println!(
            "poll {}: ret {} revents {:#x} after {} ms; {}",
            i, ret, pfd.revents, ms, accept
        );
    }
}
