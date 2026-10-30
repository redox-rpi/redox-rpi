//! mdnsd: a small multicast DNS responder (RFC 6762) with DNS-SD (RFC 6763).
//!
//! Answers queries for `<hostname>.local` (A record, the IPv4 address of
//! `eth0`, and AAAA records, its IPv6 addresses) and announces services, so other machines in the LAN find the Pi without
//! a fixed address: `ping redox-rpi.local`, `avahi-browse -r _devsh._tcp`.
//!
//! Usage: `mdnsd [-n <hostname>] [<service type>:<port>[:<path>]]...`, for
//! example `mdnsd _devsh._tcp:2323 _ssh._tcp:22:/usr/bin/sshd`. The hostname
//! defaults to `/etc/hostname`. Each service is announced as
//! `<hostname>.<service type>.local`. A service with a path counts only
//! while that file exists (checked every 30 seconds), so one configuration
//! fits images with and without the program; mdnsd announces again when
//! the set of services changes.
//!
//! One socket on port 5353 listens on 224.0.0.251 and ff02::fb (netstack
//! has one port space for IPv4 and IPv6). The addresses come from
//! `/scheme/netcfg/ifaces/eth0/addr/list` and are checked every few
//! seconds; at start and whenever they change, mdnsd announces its records
//! twice, on both groups. Queries from port 5353 get a multicast answer on
//! the group they came in on, others (one-shot resolvers) a unicast answer
//! to the sender.
//!
//! Not done yet: probing for name conflicts (RFC 6762 section 8), goodbye
//! packets at shutdown, known-answer suppression.

use std::env;
use std::fs;
use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket};
use std::os::fd::AsRawFd;
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};

const MDNS_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_GROUP6: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 0xfb);
const MDNS_PORT: u16 = 5353;

// Values of relibc's netinet/in.h (the libc crate has none for Redox).
const IPPROTO_IP: libc::c_int = 0;
const IP_ADD_MEMBERSHIP: libc::c_int = 35;
const IPPROTO_IPV6: libc::c_int = 41;
const IPV6_JOIN_GROUP: libc::c_int = 20;

const TYPE_A: u16 = 1;
const TYPE_PTR: u16 = 12;
const TYPE_TXT: u16 = 16;
const TYPE_AAAA: u16 = 28;
const TYPE_SRV: u16 = 33;
const TYPE_ANY: u16 = 255;
const CLASS_IN: u16 = 1;
/// In a record: the receiver replaces what it has cached (unique record).
const CACHE_FLUSH: u16 = 0x8000;

/// RFC 6762 section 10: 120 s for records with a host name, 75 min otherwise.
const TTL_HOST: u32 = 120;
const TTL_OTHER: u32 = 4500;

const ADDRESS_CHECK: Duration = Duration::from_secs(5);

/// How often mdnsd looks whether the programs of its services are there.
const SERVICE_CHECK: Duration = Duration::from_secs(30);

struct Service {
    kind: String, // "_devsh._tcp"
    port: u16,
    /// Announced only while this file exists.
    needs: Option<String>,
    up: bool,
}

impl Service {
    fn parse(arg: &str) -> Option<Service> {
        let mut parts = arg.splitn(3, ':');
        let kind = parts.next()?.trim_end_matches('.').to_string();
        let port = parts.next()?.parse().ok()?;
        let needs = parts.next().filter(|p| !p.is_empty()).map(String::from);
        (!kind.is_empty()).then(|| Service { kind, port, up: needs.is_none(), needs })
    }

    /// Whether the service's program is there.
    fn present(&self) -> bool {
        self.needs.as_ref().map_or(true, |path| fs::metadata(path).is_ok())
    }
}

struct Responder {
    host: String, // "redox-rpi.local"
    instance: String, // "redox-rpi", the service instance name
    services: Vec<Service>,
    addr: Option<Ipv4Addr>,
    addrs6: Vec<Ipv6Addr>,
}

/// A DNS name as lowercase labels.
fn labels(name: &str) -> Vec<String> {
    name.trim_end_matches('.').split('.').map(|l| l.to_ascii_lowercase()).collect()
}

fn put_name(out: &mut Vec<u8>, name: &str) {
    for label in name.trim_end_matches('.').split('.') {
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
}

struct Record {
    name: String,
    kind: u16,
    unique: bool,
    ttl: u32,
    data: Vec<u8>,
}

impl Record {
    fn put(&self, out: &mut Vec<u8>) {
        put_name(out, &self.name);
        out.extend_from_slice(&self.kind.to_be_bytes());
        let class = if self.unique { CLASS_IN | CACHE_FLUSH } else { CLASS_IN };
        out.extend_from_slice(&class.to_be_bytes());
        out.extend_from_slice(&self.ttl.to_be_bytes());
        out.extend_from_slice(&(self.data.len() as u16).to_be_bytes());
        out.extend_from_slice(&self.data);
    }
}

fn name_data(name: &str) -> Vec<u8> {
    let mut data = Vec::new();
    put_name(&mut data, name);
    data
}

impl Responder {
    fn a_record(&self) -> Option<Record> {
        Some(Record {
            name: self.host.clone(),
            kind: TYPE_A,
            unique: true,
            ttl: TTL_HOST,
            data: self.addr?.octets().to_vec(),
        })
    }

    fn aaaa_records(&self) -> Vec<Record> {
        self.addrs6
            .iter()
            .map(|addr| Record {
                name: self.host.clone(),
                kind: TYPE_AAAA,
                unique: true,
                ttl: TTL_HOST,
                data: addr.octets().to_vec(),
            })
            .collect()
    }

    /// The address records: A, then AAAA.
    fn address_records(&self) -> Vec<Record> {
        let mut records: Vec<Record> = self.a_record().into_iter().collect();
        records.extend(self.aaaa_records());
        records
    }

    fn has_address(&self) -> bool {
        self.addr.is_some() || !self.addrs6.is_empty()
    }

    /// The records of one service: PTR type -> instance, SRV, TXT.
    fn service_records(&self, service: &Service) -> [Record; 3] {
        let kind = format!("{}.local", service.kind);
        let full = format!("{}.{}", self.instance, kind);
        let mut srv = vec![0, 0, 0, 0]; // priority, weight
        srv.extend_from_slice(&service.port.to_be_bytes());
        put_name(&mut srv, &self.host);
        [
            Record { name: kind, kind: TYPE_PTR, unique: false, ttl: TTL_OTHER, data: name_data(&full) },
            Record { name: full.clone(), kind: TYPE_SRV, unique: true, ttl: TTL_HOST, data: srv },
            Record { name: full, kind: TYPE_TXT, unique: true, ttl: TTL_OTHER, data: vec![0] },
        ]
    }

    /// Answers for one question; the address records are added by the
    /// caller as additional records.
    fn answer(&self, qname: &[String], qtype: u16, answers: &mut Vec<Record>) {
        let any = qtype == TYPE_ANY;
        if qname == labels(&self.host).as_slice() {
            if any || qtype == TYPE_A {
                answers.extend(self.a_record());
            }
            if any || qtype == TYPE_AAAA {
                answers.extend(self.aaaa_records());
            }
        }
        if qname == labels("_services._dns-sd._udp.local").as_slice() && (any || qtype == TYPE_PTR) {
            for service in self.services.iter().filter(|s| s.up) {
                let kind = format!("{}.local", service.kind);
                answers.push(Record {
                    name: "_services._dns-sd._udp.local".into(),
                    kind: TYPE_PTR,
                    unique: false,
                    ttl: TTL_OTHER,
                    data: name_data(&kind),
                });
            }
        }
        for service in self.services.iter().filter(|s| s.up) {
            for record in self.service_records(service) {
                if qname == labels(&record.name).as_slice() && (any || qtype == record.kind) {
                    answers.push(record);
                }
            }
        }
    }

    /// All records, for announcements.
    fn all_records(&self) -> Vec<Record> {
        let mut records = self.address_records();
        for service in self.services.iter().filter(|s| s.up) {
            records.extend(self.service_records(service));
        }
        records
    }
}

/// A DNS message: header, then `questions` (copied for unicast answers) and records.
fn message(id: u16, questions: &[u8], qdcount: u16, answers: &[Record], additional: &[Record]) -> Vec<u8> {
    let mut out = Vec::with_capacity(512);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&0x8400u16.to_be_bytes()); // response, authoritative
    out.extend_from_slice(&qdcount.to_be_bytes());
    out.extend_from_slice(&(answers.len() as u16).to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&(additional.len() as u16).to_be_bytes());
    out.extend_from_slice(questions);
    for record in answers.iter().chain(additional) {
        record.put(&mut out);
    }
    out
}

/// Reads a possibly compressed name at `pos`; returns the labels and the
/// position after the name in the message.
fn read_name(msg: &[u8], mut pos: usize) -> Option<(Vec<String>, usize)> {
    let mut names = Vec::new();
    let mut end = None;
    for _ in 0..64 {
        let len = *msg.get(pos)? as usize;
        if len == 0 {
            return Some((names, end.unwrap_or(pos + 1)));
        }
        if len & 0xc0 == 0xc0 {
            let target = ((len & 0x3f) << 8) | *msg.get(pos + 1)? as usize;
            end.get_or_insert(pos + 2);
            pos = target;
            continue;
        }
        let label = msg.get(pos + 1..pos + 1 + len)?;
        names.push(String::from_utf8_lossy(label).to_ascii_lowercase());
        pos += 1 + len;
    }
    None
}

/// The IPv4 address and the IPv6 addresses of eth0.
fn eth0_addresses() -> (Option<Ipv4Addr>, Vec<Ipv6Addr>) {
    let list = fs::read_to_string("/scheme/netcfg/ifaces/eth0/addr/list").unwrap_or_default();
    let addrs = || list.lines().filter_map(|line| line.split('/').next());
    let v4 = addrs()
        .find_map(|a| a.trim().parse::<Ipv4Addr>().ok())
        .filter(|a| !a.is_unspecified());
    let v6 = addrs()
        .filter_map(|a| a.trim().parse::<Ipv6Addr>().ok())
        .filter(|a| !a.is_unspecified() && !a.is_loopback())
        .collect();
    (v4, v6)
}

/// The mDNS group of the family of `from`.
fn group_of(from: SocketAddr) -> SocketAddr {
    match from {
        SocketAddr::V4(_) => SocketAddr::V4(SocketAddrV4::new(MDNS_GROUP, MDNS_PORT)),
        SocketAddr::V6(_) => SocketAddr::V6(SocketAddrV6::new(MDNS_GROUP6, MDNS_PORT, 0, 0)),
    }
}

fn handle_query(responder: &Responder, socket: &UdpSocket, msg: &[u8], from: SocketAddr) {
    if msg.len() < 12 || msg[2] & 0x80 != 0 {
        return; // too short, or a response
    }
    let id = u16::from_be_bytes([msg[0], msg[1]]);
    let qdcount = u16::from_be_bytes([msg[4], msg[5]]);
    let mut pos = 12;
    let mut answers = Vec::new();
    let unicast = from.port() != MDNS_PORT;
    for _ in 0..qdcount {
        let Some((qname, next)) = read_name(msg, pos) else { return };
        let Some(fixed) = msg.get(next..next + 4) else { return };
        let qtype = u16::from_be_bytes([fixed[0], fixed[1]]);
        let qclass = u16::from_be_bytes([fixed[2], fixed[3]]);
        pos = next + 4;
        if qclass & 0x7fff != CLASS_IN && qclass & 0x7fff != TYPE_ANY {
            continue;
        }
        // The QU bit (0x8000) asks for a unicast answer; a multicast one is
        // allowed too and keeps the other caches current.
        responder.answer(&qname, qtype, &mut answers);
    }
    if answers.is_empty() {
        return;
    }
    // RFC 6762 section 6.2: with an SRV or PTR answer, and with an A or AAAA
    // answer, the other address records go along as additional records.
    let mut additional: Vec<Record> = Vec::new();
    let has = |kind| answers.iter().any(|r: &Record| r.kind == kind);
    let names_host = has(TYPE_SRV) || has(TYPE_PTR) || has(TYPE_A) || has(TYPE_AAAA);
    if names_host && !has(TYPE_A) {
        additional.extend(responder.a_record());
    }
    if names_host && !has(TYPE_AAAA) {
        additional.extend(responder.aaaa_records());
    }
    let reply = if unicast {
        // One-shot resolver (RFC 6762 section 6.7): its ID and questions back.
        message(id, &msg[12..pos], qdcount, &answers, &additional)
    } else {
        message(0, &[], 0, &answers, &additional)
    };
    let to = if unicast { from } else { group_of(from) };
    if let Err(e) = socket.send_to(&reply, to) {
        eprintln!("mdnsd: answer to {to}: {e}");
    }
}

fn announce(responder: &Responder, socket: &UdpSocket) {
    let records = responder.all_records();
    let msg = message(0, &[], 0, &records, &[]);
    let mut groups = Vec::new();
    if responder.addr.is_some() {
        groups.push(SocketAddr::V4(SocketAddrV4::new(MDNS_GROUP, MDNS_PORT)));
    }
    if !responder.addrs6.is_empty() {
        groups.push(SocketAddr::V6(SocketAddrV6::new(MDNS_GROUP6, MDNS_PORT, 0, 0)));
    }
    for i in 0..2 {
        if i > 0 {
            sleep(Duration::from_secs(1));
        }
        for &group in &groups {
            if let Err(e) = socket.send_to(&msg, group) {
                eprintln!("mdnsd: announcement to {group}: {e}");
            }
        }
    }
}

fn setsockopt(socket: &UdpSocket, level: libc::c_int, name: libc::c_int, value: &[u8]) -> io::Result<()> {
    let ret = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            level,
            name,
            value.as_ptr().cast(),
            value.len() as libc::socklen_t,
        )
    };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Joins 224.0.0.251 (struct ip_mreq) and ff02::fb (struct ipv6_mreq).
fn join(socket: &UdpSocket) -> io::Result<()> {
    let mut mreq = [0u8; 8];
    mreq[..4].copy_from_slice(&MDNS_GROUP.octets());
    setsockopt(socket, IPPROTO_IP, IP_ADD_MEMBERSHIP, &mreq)?;
    let mut mreq6 = [0u8; 20];
    mreq6[..16].copy_from_slice(&MDNS_GROUP6.octets());
    if let Err(e) = setsockopt(socket, IPPROTO_IPV6, IPV6_JOIN_GROUP, &mreq6) {
        eprintln!("mdnsd: cannot join {MDNS_GROUP6}, IPv4 only: {e}");
    }
    Ok(())
}

fn usage() -> ! {
    eprintln!("usage: mdnsd [-n <hostname>] [<service type>:<port>[:<path>]]...");
    process::exit(2);
}

fn main() {
    let mut args = env::args().skip(1);
    let mut hostname = None;
    let mut services = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "-n" {
            hostname = Some(args.next().unwrap_or_else(|| usage()));
        } else if let Some(service) = Service::parse(&arg) {
            services.push(service);
        } else {
            usage();
        }
    }
    let hostname = hostname
        .or_else(|| fs::read_to_string("/etc/hostname").ok())
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "redox".into());
    let mut responder = Responder {
        host: format!("{hostname}.local"),
        instance: hostname,
        services,
        addr: None,
        addrs6: Vec::new(),
    };

    let socket = match UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, MDNS_PORT))
        .and_then(|s| join(&s).map(|_| s))
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdnsd: cannot listen on {MDNS_GROUP}:{MDNS_PORT}: {e}");
            process::exit(1);
        }
    };
    let _ = socket.set_read_timeout(Some(ADDRESS_CHECK));
    println!(
        "mdnsd: {} on {MDNS_GROUP} and {MDNS_GROUP6} port {MDNS_PORT}, {} services configured",
        responder.host,
        responder.services.len()
    );

    let mut buf = [0u8; 1500];
    let mut next_check = Instant::now();
    let mut next_service_check = Instant::now();
    loop {
        if Instant::now() >= next_service_check {
            next_service_check = Instant::now() + SERVICE_CHECK;
            let mut changed = false;
            for service in &mut responder.services {
                let present = service.present();
                if present != service.up {
                    service.up = present;
                    changed = true;
                    let state = if present { "announced" } else { "no longer announced" };
                    println!("mdnsd: {}.{}.local port {} {state}", responder.instance, service.kind, service.port);
                }
            }
            if changed && responder.has_address() {
                announce(&responder, &socket);
            }
        }
        if Instant::now() >= next_check {
            next_check = Instant::now() + ADDRESS_CHECK;
            let (addr, addrs6) = eth0_addresses();
            if addr != responder.addr || addrs6 != responder.addrs6 {
                responder.addr = addr;
                responder.addrs6 = addrs6;
                if responder.has_address() {
                    let list: Vec<String> = addr
                        .iter()
                        .map(ToString::to_string)
                        .chain(responder.addrs6.iter().map(ToString::to_string))
                        .collect();
                    println!("mdnsd: {} is {}", responder.host, list.join(", "));
                    announce(&responder, &socket);
                }
            }
        }
        match socket.recv_from(&mut buf) {
            Ok((len, from)) => {
                if responder.has_address() {
                    handle_query(&responder, &socket, &buf[..len], from);
                }
            }
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => {
                eprintln!("mdnsd: receive: {e}");
                sleep(Duration::from_secs(1));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Service;

    #[test]
    fn service_arguments() {
        let s = Service::parse("_devsh._tcp:2323").unwrap();
        assert_eq!((s.kind.as_str(), s.port, s.needs.is_none(), s.up), ("_devsh._tcp", 2323, true, true));
        let s = Service::parse("_ssh._tcp.:22:/usr/bin/sshd").unwrap();
        assert_eq!((s.kind.as_str(), s.port, s.needs.as_deref(), s.up), ("_ssh._tcp", 22, Some("/usr/bin/sshd"), false));
        assert!(Service::parse("_ssh._tcp").is_none());
        assert!(Service::parse("_ssh._tcp:x").is_none());
        assert!(Service::parse(":22").is_none());
    }

    #[test]
    fn service_present() {
        assert!(Service::parse("_a._tcp:1").unwrap().present());
        assert!(Service::parse("_a._tcp:1:/").unwrap().present());
        assert!(!Service::parse("_a._tcp:1:/no/such/file").unwrap().present());
    }
}
