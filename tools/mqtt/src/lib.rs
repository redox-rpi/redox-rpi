//! mqtt: a small MQTT 3.1.1 client in plain Rust, for Redox.
//!
//! No crates from crates.io, only `std`, so it builds for Redox like the
//! other tools here. It speaks MQTT 3.1.1 over plain TCP: connect (with
//! user name and password), publish with QoS 0 or 1 and retain, subscribe,
//! ping and disconnect. No TLS, no QoS 2, no MQTT 5.
//!
//! ```no_run
//! let mut client = mqtt::Client::connect("broker.local:1883", &mqtt::Options::new("demo"))?;
//! client.publish("redox/pi/uptime", b"42", mqtt::QoS::AtLeastOnce, false)?;
//! client.subscribe(&[("redox/#", mqtt::QoS::AtMostOnce)])?;
//! let message = client.recv()?;
//! println!("{} {}", message.topic, String::from_utf8_lossy(&message.payload));
//! client.disconnect()?;
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! A thread sends a ping every half keep-alive interval, so a client that
//! publishes only now and then stays connected.

use std::collections::VecDeque;
use std::io::{self, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, Weak};
use std::thread;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 1883;
/// Redox' DNS lookup has no timeout of its own.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

const CONNECT: u8 = 1;
const CONNACK: u8 = 2;
const PUBLISH: u8 = 3;
const PUBACK: u8 = 4;
const SUBSCRIBE: u8 = 8;
const SUBACK: u8 = 9;
const PINGREQ: u8 = 12;
const PINGRESP: u8 = 13;
const DISCONNECT: u8 = 14;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
}

impl QoS {
    pub fn from_u8(value: u8) -> Option<QoS> {
        match value {
            0 => Some(QoS::AtMostOnce),
            1 => Some(QoS::AtLeastOnce),
            _ => None,
        }
    }
}

/// Settings for [`Client::connect`].
#[derive(Clone, Debug)]
pub struct Options {
    pub client_id: String,
    /// Seconds; 0 turns pings off.
    pub keep_alive: u16,
    pub clean_session: bool,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Options {
    pub fn new(client_id: &str) -> Options {
        Options {
            client_id: client_id.to_string(),
            keep_alive: 60,
            clean_session: true,
            username: None,
            password: None,
        }
    }
}

/// A message from a subscription.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: QoS,
    pub retain: bool,
}

pub struct Client {
    writer: Arc<Mutex<TcpStream>>,
    reader: BufReader<TcpStream>,
    next_id: u16,
    /// Messages that came in while waiting for an acknowledgement.
    queued: VecDeque<Message>,
}

impl Client {
    /// Connects to `address` ("host" or "host:port") and logs in.
    pub fn connect(address: &str, options: &Options) -> io::Result<Client> {
        let stream = open(address)?;
        let mut client = Client {
            writer: Arc::new(Mutex::new(stream.try_clone()?)),
            reader: BufReader::new(stream),
            next_id: 1,
            queued: VecDeque::new(),
        };
        client.send(CONNECT << 4, &connect_body(options))?;
        let (kind, body) = client.read_packet()?;
        if kind >> 4 != CONNACK || body.len() != 2 {
            return Err(invalid("expected CONNACK"));
        }
        if body[1] != 0 {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("broker refused the connection: {}", connack_reason(body[1])),
            ));
        }
        if options.keep_alive > 0 {
            start_pings(Arc::downgrade(&client.writer), options.keep_alive);
        }
        Ok(client)
    }

    /// Publishes a message. With QoS 1 it waits until the broker has it.
    pub fn publish(&mut self, topic: &str, payload: &[u8], qos: QoS, retain: bool) -> io::Result<()> {
        check_topic(topic, false)?;
        let mut body = Vec::with_capacity(topic.len() + payload.len() + 4);
        put_str(&mut body, topic)?;
        let id = self.packet_id();
        if qos == QoS::AtLeastOnce {
            body.extend_from_slice(&id.to_be_bytes());
        }
        body.extend_from_slice(payload);
        self.send(PUBLISH << 4 | (qos as u8) << 1 | retain as u8, &body)?;
        if qos == QoS::AtLeastOnce {
            self.wait_for(PUBACK, id)?;
        }
        Ok(())
    }

    /// Subscribes to topic filters (`+` and `#` wildcards allowed). Returns
    /// the QoS the broker granted for each.
    pub fn subscribe(&mut self, filters: &[(&str, QoS)]) -> io::Result<Vec<QoS>> {
        let id = self.packet_id();
        let mut body = id.to_be_bytes().to_vec();
        for (filter, qos) in filters {
            check_topic(filter, true)?;
            put_str(&mut body, filter)?;
            body.push(*qos as u8);
        }
        self.send(SUBSCRIBE << 4 | 0b0010, &body)?;
        let granted = self.wait_for(SUBACK, id)?;
        granted
            .iter()
            .zip(filters)
            .map(|(&code, (filter, _))| {
                QoS::from_u8(code).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("broker refused the subscription to {filter}"),
                    )
                })
            })
            .collect()
    }

    /// Waits for the next message from a subscription.
    pub fn recv(&mut self) -> io::Result<Message> {
        if let Some(message) = self.queued.pop_front() {
            return Ok(message);
        }
        loop {
            let (kind, body) = self.read_packet()?;
            if kind >> 4 == PUBLISH {
                return self.incoming(kind, &body);
            }
            // PINGRESP and late acknowledgements need nothing.
        }
    }

    pub fn disconnect(self) -> io::Result<()> {
        self.send(DISCONNECT << 4, &[])?;
        let stream = self.writer.lock().unwrap();
        stream.shutdown(std::net::Shutdown::Both)
    }

    fn packet_id(&mut self) -> u16 {
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).unwrap_or(1);
        id
    }

    fn send(&self, header: u8, body: &[u8]) -> io::Result<()> {
        let packet = encode(header, body)?;
        self.writer.lock().unwrap().write_all(&packet)
    }

    fn read_packet(&mut self) -> io::Result<(u8, Vec<u8>)> {
        read_packet(&mut self.reader)
    }

    /// Reads until the acknowledgement `kind` for packet `id` comes; returns
    /// what follows the packet id.
    fn wait_for(&mut self, kind: u8, id: u16) -> io::Result<Vec<u8>> {
        loop {
            let (header, body) = self.read_packet()?;
            match header >> 4 {
                PUBLISH => {
                    let message = self.incoming(header, &body)?;
                    self.queued.push_back(message);
                }
                k if k == kind && body.len() >= 2 && body[..2] == id.to_be_bytes() => {
                    return Ok(body[2..].to_vec());
                }
                PINGRESP | PUBACK | SUBACK => {}
                other => return Err(invalid(&format!("unexpected packet type {other}"))),
            }
        }
    }

    fn incoming(&mut self, header: u8, body: &[u8]) -> io::Result<Message> {
        let (message, id) = parse_publish(header, body)?;
        if let Some(id) = id {
            self.send(PUBACK << 4, &id.to_be_bytes())?;
        }
        Ok(message)
    }
}

/// Resolves and connects, both with a timeout.
fn open(address: &str) -> io::Result<TcpStream> {
    let has_port = !address.ends_with(']')
        && address.rsplit_once(':').is_some_and(|(_, port)| port.parse::<u16>().is_ok());
    let address = if has_port { address.to_string() } else { format!("{address}:{DEFAULT_PORT}") };
    let addrs = resolve(&address)?;
    let mut last = io::Error::new(io::ErrorKind::NotFound, format!("{address}: no address"));
    for addr in addrs {
        let result = match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
            Err(e) if e.kind() == io::ErrorKind::Unsupported => TcpStream::connect(addr),
            other => other,
        };
        match result {
            // No set_nodelay: relibc logs an unknown option on each connect.
            Ok(stream) => return Ok(stream),
            Err(e) => last = io::Error::new(e.kind(), format!("{addr}: {e}")),
        }
    }
    Err(last)
}

fn resolve(address: &str) -> io::Result<Vec<SocketAddr>> {
    if let Ok(addr) = address.parse::<SocketAddr>() {
        return Ok(vec![addr]);
    }
    let (tx, rx) = mpsc::channel();
    let name = address.to_string();
    thread::spawn(move || {
        let _ = tx.send(name.to_socket_addrs().map(|a| a.collect::<Vec<_>>()));
    });
    match rx.recv_timeout(LOOKUP_TIMEOUT) {
        Ok(result) => result.map_err(|e| io::Error::new(e.kind(), format!("{address}: {e}"))),
        Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, format!("{address}: no DNS answer"))),
    }
}

/// Sends PINGREQ every half keep-alive interval until the client is gone.
fn start_pings(writer: Weak<Mutex<TcpStream>>, keep_alive: u16) {
    let every = Duration::from_millis(u64::from(keep_alive) * 500);
    thread::spawn(move || loop {
        thread::sleep(every);
        let Some(writer) = writer.upgrade() else { return };
        let result = writer.lock().unwrap().write_all(&[PINGREQ << 4, 0]);
        if result.is_err() {
            return;
        }
    });
}

fn connect_body(options: &Options) -> Vec<u8> {
    let mut body = Vec::new();
    put_str(&mut body, "MQTT").unwrap();
    body.push(4); // protocol level 3.1.1
    let mut flags = 0u8;
    if options.clean_session {
        flags |= 0x02;
    }
    if options.username.is_some() {
        flags |= 0x80;
    }
    if options.password.is_some() {
        flags |= 0x40;
    }
    body.push(flags);
    body.extend_from_slice(&options.keep_alive.to_be_bytes());
    put_str(&mut body, &options.client_id).unwrap();
    if let Some(username) = &options.username {
        put_str(&mut body, username).unwrap();
    }
    if let Some(password) = &options.password {
        put_str(&mut body, password).unwrap();
    }
    body
}

fn connack_reason(code: u8) -> &'static str {
    match code {
        1 => "protocol version not supported",
        2 => "client id rejected",
        3 => "server unavailable",
        4 => "bad user name or password",
        5 => "not authorized",
        _ => "unknown reason",
    }
}

fn parse_publish(header: u8, body: &[u8]) -> io::Result<(Message, Option<u16>)> {
    let qos = QoS::from_u8((header >> 1) & 3).ok_or_else(|| invalid("QoS 2 is not supported"))?;
    let (topic, mut rest) = take_str(body)?;
    let mut id = None;
    if qos == QoS::AtLeastOnce {
        if rest.len() < 2 {
            return Err(invalid("PUBLISH without packet id"));
        }
        id = Some(u16::from_be_bytes([rest[0], rest[1]]));
        rest = &rest[2..];
    }
    let message = Message { topic, payload: rest.to_vec(), qos, retain: header & 1 == 1 };
    Ok((message, id))
}

/// Fixed header, remaining length and body.
fn encode(header: u8, body: &[u8]) -> io::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(body.len() + 5);
    packet.push(header);
    encode_length(&mut packet, body.len())?;
    packet.extend_from_slice(body);
    Ok(packet)
}

fn encode_length(out: &mut Vec<u8>, mut length: usize) -> io::Result<()> {
    if length > 268_435_455 {
        return Err(invalid("packet larger than 256 MB"));
    }
    loop {
        let mut byte = (length % 128) as u8;
        length /= 128;
        if length > 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if length == 0 {
            return Ok(());
        }
    }
}

fn read_packet(reader: &mut impl Read) -> io::Result<(u8, Vec<u8>)> {
    let mut byte = [0u8; 1];
    reader.read_exact(&mut byte)?;
    let header = byte[0];
    let mut length = 0usize;
    for shift in 0..4 {
        reader.read_exact(&mut byte)?;
        length |= usize::from(byte[0] & 0x7f) << (7 * shift);
        if byte[0] & 0x80 == 0 {
            let mut body = vec![0u8; length];
            reader.read_exact(&mut body)?;
            return Ok((header, body));
        }
    }
    Err(invalid("remaining length longer than 4 bytes"))
}

fn put_str(out: &mut Vec<u8>, s: &str) -> io::Result<()> {
    let length = u16::try_from(s.len()).map_err(|_| invalid("string longer than 65535 bytes"))?;
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(s.as_bytes());
    Ok(())
}

fn take_str(data: &[u8]) -> io::Result<(String, &[u8])> {
    if data.len() < 2 {
        return Err(invalid("string too short"));
    }
    let length = usize::from(u16::from_be_bytes([data[0], data[1]]));
    let bytes = data.get(2..2 + length).ok_or_else(|| invalid("string too short"))?;
    let s = String::from_utf8(bytes.to_vec()).map_err(|_| invalid("string is not UTF-8"))?;
    Ok((s, &data[2 + length..]))
}

/// Topic names must not have wildcards; filters may, `#` only at the end.
fn check_topic(topic: &str, filter: bool) -> io::Result<()> {
    if topic.is_empty() || topic.contains('\0') {
        return Err(invalid("empty topic"));
    }
    let levels: Vec<&str> = topic.split('/').collect();
    for (i, level) in levels.iter().enumerate() {
        let wild = level.contains('+') || level.contains('#');
        if wild && !filter {
            return Err(invalid(&format!("topic {topic}: + and # only in subscriptions")));
        }
        if wild && !(*level == "+" || (*level == "#" && i == levels.len() - 1)) {
            return Err(invalid(&format!("topic filter {topic}: bad wildcard")));
        }
    }
    Ok(())
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_length() {
        for (length, bytes) in [
            (0, &[0x00][..]),
            (127, &[0x7f]),
            (128, &[0x80, 0x01]),
            (16_383, &[0xff, 0x7f]),
            (16_384, &[0x80, 0x80, 0x01]),
            (268_435_455, &[0xff, 0xff, 0xff, 0x7f]),
        ] {
            let mut out = Vec::new();
            encode_length(&mut out, length).unwrap();
            assert_eq!(out, bytes, "{length}");
        }
        assert!(encode_length(&mut Vec::new(), 268_435_456).is_err());
    }

    #[test]
    fn packet_round_trip() {
        let body = vec![7u8; 300];
        let packet = encode(PUBLISH << 4, &body).unwrap();
        assert_eq!(&packet[..3], &[0x30, 0xac, 0x02]);
        let (header, read) = read_packet(&mut &packet[..]).unwrap();
        assert_eq!(header, 0x30);
        assert_eq!(read, body);
    }

    #[test]
    fn connect_packet() {
        let mut options = Options::new("pi");
        options.username = Some("u".into());
        options.password = Some("p".into());
        let packet = encode(CONNECT << 4, &connect_body(&options)).unwrap();
        assert_eq!(
            packet,
            [0x10, 20, 0, 4, b'M', b'Q', b'T', b'T', 4, 0xc2, 0, 60, 0, 2, b'p', b'i', 0, 1, b'u', 0, 1, b'p']
        );
    }

    #[test]
    fn publish_parse() {
        let mut body = Vec::new();
        put_str(&mut body, "a/b").unwrap();
        body.extend_from_slice(&[0, 9]);
        body.extend_from_slice(b"hi");
        let (message, id) = parse_publish(PUBLISH << 4 | 0b0011, &body).unwrap();
        assert_eq!(id, Some(9));
        assert_eq!(message.topic, "a/b");
        assert_eq!(message.payload, b"hi");
        assert!(message.retain);
        assert_eq!(message.qos, QoS::AtLeastOnce);
        assert!(parse_publish(PUBLISH << 4 | 0b0100, &body).is_err());
    }

    #[test]
    fn topics() {
        assert!(check_topic("redox/pi/temp", false).is_ok());
        assert!(check_topic("redox/+/temp", false).is_err());
        assert!(check_topic("redox/+/temp", true).is_ok());
        assert!(check_topic("redox/#", true).is_ok());
        assert!(check_topic("redox/#/x", true).is_err());
        assert!(check_topic("redox/a#", true).is_err());
        assert!(check_topic("", true).is_err());
    }
}
