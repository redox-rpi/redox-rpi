//! Tests IPv6 from a program's point of view, through std and relibc:
//!
//! - `net6 resolve <name>`: getaddrinfo, prints every address;
//! - `net6 udp <[addr]:port> <text>`: sends the text from an IPv6 socket
//!   and prints the reply and who sent it;
//! - `net6 tcp <[addr]:port> <text>`: connects, sends the text, prints the
//!   reply and both endpoints.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::process::exit;
use std::time::Duration;

fn usage() -> ! {
    eprintln!("usage: net6 resolve <name> | udp <[addr]:port> <text> | tcp <[addr]:port> <text>");
    exit(2);
}

fn fail(what: &str, err: std::io::Error) -> ! {
    eprintln!("net6: {what}: {err}");
    exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["resolve", name] => {
            let addrs = (name, 0)
                .to_socket_addrs()
                .unwrap_or_else(|err| fail("resolve", err));
            let mut any = false;
            for addr in addrs {
                any = true;
                println!("{name}: {}", addr.ip());
            }
            if !any {
                eprintln!("net6: {name}: no address");
                exit(1);
            }
        }
        ["udp", target, text] => {
            let target: SocketAddr = target.parse().unwrap_or_else(|_| usage());
            let bind = if target.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
            let socket = UdpSocket::bind(bind).unwrap_or_else(|err| fail("bind", err));
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap_or_else(|err| fail("timeout", err));
            socket
                .send_to(text.as_bytes(), target)
                .unwrap_or_else(|err| fail("send", err));
            let mut buf = [0; 1500];
            let (len, from) = socket
                .recv_from(&mut buf)
                .unwrap_or_else(|err| fail("receive", err));
            println!(
                "udp reply from {from}: {}",
                String::from_utf8_lossy(&buf[..len])
            );
        }
        ["tcp", target, text] => {
            let target: SocketAddr = target.parse().unwrap_or_else(|_| usage());
            let mut stream = TcpStream::connect_timeout(&target, Duration::from_secs(5))
                .unwrap_or_else(|err| fail("connect", err));
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap_or_else(|err| fail("timeout", err));
            let local = stream.local_addr().unwrap_or_else(|err| fail("local address", err));
            let peer = stream.peer_addr().unwrap_or_else(|err| fail("peer address", err));
            stream
                .write_all(text.as_bytes())
                .unwrap_or_else(|err| fail("send", err));
            let mut buf = [0; 1500];
            let len = stream
                .read(&mut buf)
                .unwrap_or_else(|err| fail("receive", err));
            println!(
                "tcp {local} -> {peer} reply: {}",
                String::from_utf8_lossy(&buf[..len])
            );
        }
        _ => usage(),
    }
}
