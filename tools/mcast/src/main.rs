//! mcast: test IPv4 multicast in the Redox network stack.
//!
//! Usage:
//!   `mcast recv <group> <port> [count]`  join the group, print `count`
//!   datagrams (default 1) with their sender, leave the group, exit 0.
//!   `mcast send <group> <port> <text> [count]`  send the text `count` times
//!   (default 1), one second apart.
//!
//! The join goes through `setsockopt(IPPROTO_IP, IP_ADD_MEMBERSHIP)`, which
//! relibc hands to the network stack; the stack then reports the membership
//! with IGMP and accepts the group's frames.

use std::env;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::os::fd::AsRawFd;
use std::process;
use std::thread::sleep;
use std::time::Duration;

// Values of relibc's netinet/in.h (the libc crate has none for Redox).
const IPPROTO_IP: libc::c_int = 0;
const IP_ADD_MEMBERSHIP: libc::c_int = 35;
const IP_DROP_MEMBERSHIP: libc::c_int = 36;

/// struct ip_mreq: group, then local interface (any), both in network order.
fn membership(socket: &UdpSocket, option: libc::c_int, group: Ipv4Addr) -> io::Result<()> {
    let mut mreq = [0u8; 8];
    mreq[..4].copy_from_slice(&group.octets());
    let ret = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            IPPROTO_IP,
            option,
            mreq.as_ptr().cast(),
            mreq.len() as libc::socklen_t,
        )
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn recv(group: Ipv4Addr, port: u16, count: usize) -> io::Result<()> {
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port))?;
    membership(&socket, IP_ADD_MEMBERSHIP, group)?;
    println!("mcast: joined {group}, waiting on port {port}");
    let mut buf = [0u8; 1500];
    for _ in 0..count {
        let (len, from) = socket.recv_from(&mut buf)?;
        println!("mcast: {len} bytes from {from}: {}", String::from_utf8_lossy(&buf[..len]));
    }
    membership(&socket, IP_DROP_MEMBERSHIP, group)?;
    println!("mcast: left {group}");
    Ok(())
}

fn send(group: Ipv4Addr, port: u16, text: &str, count: usize) -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    for i in 0..count {
        if i > 0 {
            sleep(Duration::from_secs(1));
        }
        socket.send_to(text.as_bytes(), SocketAddrV4::new(group, port))?;
        println!("mcast: sent {} bytes to {group}:{port}", text.len());
    }
    Ok(())
}

fn usage() -> ! {
    eprintln!("usage: mcast recv <group> <port> [count]");
    eprintln!("       mcast send <group> <port> <text> [count]");
    process::exit(2);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let group: Ipv4Addr = args.get(1).and_then(|a| a.parse().ok()).unwrap_or_else(|| usage());
    let port: u16 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or_else(|| usage());
    let count = |i: usize| args.get(i).map_or(Some(1), |a| a.parse().ok()).unwrap_or_else(|| usage());
    let result = match args.first().map(String::as_str).unwrap_or_else(|| usage()) {
        "recv" => recv(group, port, count(3)),
        "send" => send(group, port, args.get(3).unwrap_or_else(|| usage()), count(4)),
        _ => usage(),
    };
    if let Err(e) = result {
        eprintln!("mcast: {e}");
        process::exit(1);
    }
}
