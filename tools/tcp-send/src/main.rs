//! Sends zero bytes to a TCP port and prints how long it took, for network
//! speed tests. Redox's `nc` connects but sends nothing on the Pi, from a
//! pipe or from a file (Pi test #135).
//!
//! On the other side, e.g. `nc -l <port> | wc -c`.
//!
//! Usage: tcp-send <host>:<port> [bytes] (default 16 MiB)
//! Exit code 0 if every byte was sent.

use std::{
    env,
    io::Write,
    net::{Shutdown, TcpStream},
    process,
    time::Instant,
};

/// Bytes per write, like `dd bs=65536`.
const CHUNK: usize = 65536;

fn main() {
    let args: Vec<String> = env::args().collect();
    let Some(addr) = args.get(1) else {
        eprintln!("usage: tcp-send <host>:<port> [bytes]");
        process::exit(2);
    };
    let total: usize = match args.get(2).map(|s| s.parse()) {
        None => 16 << 20,
        Some(Ok(n)) => n,
        Some(Err(_)) => {
            eprintln!("tcp-send: bytes must be a number");
            process::exit(2);
        }
    };

    let mut stream = TcpStream::connect(addr).unwrap_or_else(|err| {
        eprintln!("tcp-send: connect to {addr}: {err}");
        process::exit(1);
    });
    let buf = vec![0u8; CHUNK];
    let start = Instant::now();
    let mut sent = 0;
    while sent < total {
        let len = CHUNK.min(total - sent);
        if let Err(err) = stream.write_all(&buf[..len]) {
            eprintln!("tcp-send: after {sent} bytes: {err}");
            break;
        }
        sent += len;
    }
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
    let secs = start.elapsed().as_secs_f64();
    println!(
        "tcp-send: {sent} bytes in {secs:.2} s, {:.2} MB/s",
        sent as f64 / secs / 1e6
    );
    process::exit(if sent == total { 0 } else { 1 });
}
