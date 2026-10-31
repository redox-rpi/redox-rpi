//! netget URL [OUT] [--slow MS] [--pause S]
//!
//! Fetches URL over plain HTTP/1.0 and prints the speed every 16 MiB, so a
//! download that gets slower shows where. OUT is a file to write; without it
//! the data is thrown away. --slow MS sleeps MS after every read, to act like
//! a slow card and let the receive window fill up. --pause S stops reading
//! for S seconds after every 16 MiB, so the window stays closed that long.

use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::process::exit;
use std::thread::sleep;
use std::time::{Duration, Instant};

const STEP: u64 = 16 << 20;

fn main() {
    let mut args = env::args().skip(1);
    let mut url = None;
    let mut out = None;
    let mut slow = 0u64;
    let mut pause = 0u64;
    while let Some(a) = args.next() {
        if a == "--slow" {
            slow = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if a == "--pause" {
            pause = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if url.is_none() {
            url = Some(a);
        } else {
            out = Some(a);
        }
    }
    let Some(url) = url else {
        eprintln!("usage: netget URL [OUT] [--slow MS] [--pause S]");
        exit(2);
    };
    let Some(rest) = url.strip_prefix("http://") else {
        eprintln!("netget: only http:// URLs");
        exit(2);
    };
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let addr = if host.contains(':') { host.to_string() } else { format!("{host}:80") };
    let Some(addr) = addr.to_socket_addrs().ok().and_then(|mut a| a.next()) else {
        eprintln!("netget: cannot resolve {host}");
        exit(1);
    };
    let mut s = TcpStream::connect(addr).unwrap_or_else(|e| {
        eprintln!("netget: connect {addr}: {e}");
        exit(1);
    });
    write!(s, "GET {path} HTTP/1.0\r\nHost: {host}\r\n\r\n").unwrap();

    let mut file = out.map(|o| {
        File::create(&o).unwrap_or_else(|e| {
            eprintln!("netget: {o}: {e}");
            exit(1);
        })
    });
    let mut buf = vec![0u8; 64 << 10];
    let mut head = Vec::new();
    // None until the header is in; then the Content-Length, if any.
    let mut length: Option<Option<u64>> = None;
    let mut total = 0u64;
    let start = Instant::now();
    let mut last = (start, 0u64);
    loop {
        let n = match s.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let t = start.elapsed().as_secs_f64();
                eprintln!("netget: read after {total} bytes, {t:.1} s: {e}");
                exit(1);
            }
        };
        let body = if length.is_none() {
            head.extend_from_slice(&buf[..n]);
            let Some(i) = head.windows(4).position(|w| w == b"\r\n\r\n") else { continue };
            let text = String::from_utf8_lossy(&head[..i]).to_string();
            println!("{}", text.lines().next().unwrap_or(""));
            length = Some(text.lines().find_map(|l| {
                let l = l.to_ascii_lowercase();
                l.strip_prefix("content-length:").and_then(|v| v.trim().parse().ok())
            }));
            head.split_off(i + 4)
        } else {
            buf[..n].to_vec()
        };
        total += body.len() as u64;
        if let Some(f) = file.as_mut() {
            f.write_all(&body).unwrap();
        }
        if total - last.1 >= STEP {
            let now = Instant::now();
            let mbs = (total - last.1) as f64 / now.duration_since(last.0).as_secs_f64() / 1e6;
            println!("{:5} MiB  {:6.1} s  {:.2} MB/s", total >> 20, start.elapsed().as_secs_f64(), mbs);
            if pause > 0 {
                sleep(Duration::from_secs(pause));
            }
            last = (Instant::now(), total);
        }
        if slow > 0 {
            sleep(Duration::from_millis(slow));
        }
    }
    let t = start.elapsed().as_secs_f64();
    println!("{total} bytes in {t:.1} s, {:.2} MB/s", total as f64 / t / 1e6);
    if let Some(Some(l)) = length {
        if l != total {
            eprintln!("netget: short, expected {l} bytes");
            exit(1);
        }
    }
}
