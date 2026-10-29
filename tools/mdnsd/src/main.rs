//! mdnsd: a small multicast DNS responder (RFC 6762) with DNS-SD (RFC 6763).
//!
//! Answers queries for `<hostname>.local` (A record, the address of `eth0`)
//! and announces services, so other machines in the LAN find the Pi without
//! a fixed address: `ping redox-rpi.local`, `avahi-browse -r _devsh._tcp`.
//!
//! Usage: `mdnsd [-n <hostname>] [<service type>:<port>]...`, for example
//! `mdnsd _devsh._tcp:2323`. The hostname defaults to `/etc/hostname`.
//! Each service is announced as `<hostname>.<service type>.local`.
//!
//! The address comes from `/scheme/netcfg/ifaces/eth0/addr/list` and is
//! checked every few seconds; at start and whenever it changes, mdnsd
//! announces its records twice. Queries from port 5353 get a multicast
//! answer, others (one-shot resolvers) a unicast answer to the sender.
//!
//! Not done yet: probing for name conflicts (RFC 6762 section 8), goodbye
//! packets at shutdown, known-answer suppression, AAAA records.

use std::env;
use std::fs;
use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::os::fd::AsRawFd;
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};

const MDNS_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;

// Values of relibc's netinet/in.h (the libc crate has none for Redox).
const IPPROTO_IP: libc::c_int = 0;
const IP_ADD_MEMBERSHIP: libc::c_int = 35;

const TYPE_A: u16 = 1;
const TYPE_PTR: u16 = 12;
const TYPE_TXT: u16 = 16;
const TYPE_SRV: u16 = 33;
const TYPE_ANY: u16 = 255;
const CLASS_IN: u16 = 1;
/// In a record: the receiver replaces what it has cached (unique record).
const CACHE_FLUSH: u16 = 0x8000;

/// RFC 6762 section 10: 120 s for records with a host name, 75 min otherwise.
const TTL_HOST: u32 = 120;
const TTL_OTHER: u32 = 4500;

const ADDRESS_CHECK: Duration = Duration::from_secs(5);

struct Service {
    kind: String, // "_devsh._tcp"
    port: u16,
}

struct Responder {
    host: String, // "redox-rpi.local"
    instance: String, // "redox-rpi", the service instance name
    services: Vec<Service>,
    addr: Option<Ipv4Addr>,
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

    /// Answers for one question; the A record is added by the caller when an
    /// SRV record names this host.
    fn answer(&self, qname: &[String], qtype: u16, answers: &mut Vec<Record>) {
        let any = qtype == TYPE_ANY;
        if qname == labels(&self.host).as_slice() && (any || qtype == TYPE_A) {
            answers.extend(self.a_record());
        }
        if qname == labels("_services._dns-sd._udp.local").as_slice() && (any || qtype == TYPE_PTR) {
            for service in &self.services {
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
        for service in &self.services {
            for record in self.service_records(service) {
                if qname == labels(&record.name).as_slice() && (any || qtype == record.kind) {
                    answers.push(record);
                }
            }
        }
    }

    /// All records, for announcements.
    fn all_records(&self) -> Vec<Record> {
        let mut records: Vec<Record> = self.a_record().into_iter().collect();
        for service in &self.services {
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

fn eth0_address() -> Option<Ipv4Addr> {
    let list = fs::read_to_string("/scheme/netcfg/ifaces/eth0/addr/list").ok()?;
    list.lines().find_map(|line| line.split('/').next()?.trim().parse().ok())
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
    let mut additional = Vec::new();
    if answers.iter().any(|r| r.kind == TYPE_SRV || r.kind == TYPE_PTR) && !answers.iter().any(|r| r.kind == TYPE_A) {
        additional.extend(responder.a_record());
    }
    let reply = if unicast {
        // One-shot resolver (RFC 6762 section 6.7): its ID and questions back.
        message(id, &msg[12..pos], qdcount, &answers, &additional)
    } else {
        message(0, &[], 0, &answers, &additional)
    };
    let to = if unicast { from } else { SocketAddr::V4(SocketAddrV4::new(MDNS_GROUP, MDNS_PORT)) };
    if let Err(e) = socket.send_to(&reply, to) {
        eprintln!("mdnsd: answer to {to}: {e}");
    }
}

fn announce(responder: &Responder, socket: &UdpSocket) {
    let records = responder.all_records();
    let msg = message(0, &[], 0, &records, &[]);
    for i in 0..2 {
        if i > 0 {
            sleep(Duration::from_secs(1));
        }
        if let Err(e) = socket.send_to(&msg, SocketAddrV4::new(MDNS_GROUP, MDNS_PORT)) {
            eprintln!("mdnsd: announcement: {e}");
        }
    }
}

fn join(socket: &UdpSocket) -> io::Result<()> {
    let mut mreq = [0u8; 8];
    mreq[..4].copy_from_slice(&MDNS_GROUP.octets());
    let ret = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            IPPROTO_IP,
            IP_ADD_MEMBERSHIP,
            mreq.as_ptr().cast(),
            mreq.len() as libc::socklen_t,
        )
    };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

fn usage() -> ! {
    eprintln!("usage: mdnsd [-n <hostname>] [<service type>:<port>]...");
    process::exit(2);
}

fn main() {
    let mut args = env::args().skip(1);
    let mut hostname = None;
    let mut services = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "-n" {
            hostname = Some(args.next().unwrap_or_else(|| usage()));
        } else if let Some((kind, port)) = arg.rsplit_once(':') {
            let port = port.parse().unwrap_or_else(|_| usage());
            services.push(Service { kind: kind.trim_end_matches('.').to_string(), port });
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
    println!("mdnsd: {} on {MDNS_GROUP}:{MDNS_PORT}, {} services", responder.host, responder.services.len());

    let mut buf = [0u8; 1500];
    let mut next_check = Instant::now();
    loop {
        if Instant::now() >= next_check {
            next_check = Instant::now() + ADDRESS_CHECK;
            let addr = eth0_address().filter(|a| !a.is_unspecified());
            if addr != responder.addr {
                responder.addr = addr;
                if let Some(addr) = addr {
                    println!("mdnsd: {} is {addr}", responder.host);
                    announce(&responder, &socket);
                }
            }
        }
        match socket.recv_from(&mut buf) {
            Ok((len, from)) => {
                if responder.addr.is_some() {
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
