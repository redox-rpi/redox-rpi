//! The connection under MQTT: plain TCP, or TLS over TCP (feature `tls`).
//!
//! Reading and writing must not block each other: `recv` waits for the
//! broker while the ping thread writes. TCP sockets do that by themselves;
//! a TLS connection is one state machine. So the reader reads raw bytes from
//! its own handle of the socket without holding the lock, then hands them to
//! rustls under the lock, and writers encrypt and send under the same lock.

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};

#[cfg(feature = "tls")]
use std::path::PathBuf;

/// TLS settings for [`crate::Options::tls`].
#[cfg(feature = "tls")]
#[derive(Clone, Debug, Default)]
pub struct Tls {
    /// PEM file with the CA certificates to trust. None: the system's
    /// `/etc/ssl/certs/ca-certificates.crt` if it exists, else the Mozilla
    /// roots built into the program.
    pub ca_file: Option<PathBuf>,
    /// The name the broker's certificate must have. None: the host part of
    /// the address (a name or an IP address).
    pub server_name: Option<String>,
}

#[cfg(feature = "tls")]
const SYSTEM_CA_FILE: &str = "/etc/ssl/certs/ca-certificates.crt";

pub(crate) struct Link {
    sock: TcpStream,
    #[cfg(feature = "tls")]
    tls: Option<rustls::ClientConnection>,
}

impl Link {
    pub(crate) fn plain(sock: TcpStream) -> Link {
        Link {
            sock,
            #[cfg(feature = "tls")]
            tls: None,
        }
    }

    /// Runs the TLS handshake with `host` (the host part of the address).
    #[cfg(feature = "tls")]
    pub(crate) fn tls(mut sock: TcpStream, host: &str, settings: &Tls) -> io::Result<Link> {
        use rustls::pki_types::ServerName;

        let name = settings.server_name.as_deref().unwrap_or(host);
        let name = ServerName::try_from(name.to_string())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("bad TLS server name {name:?}")))?;
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(tls_error)?
            .with_root_certificates(root_store(settings)?)
            .with_no_client_auth();
        let mut conn = rustls::ClientConnection::new(Arc::new(config), name).map_err(tls_error)?;
        while conn.is_handshaking() {
            conn.complete_io(&mut sock).map_err(add_time_hint)?;
        }
        Ok(Link { sock, tls: Some(conn) })
    }

    pub(crate) fn reader(link: &Arc<Mutex<Link>>) -> io::Result<LinkReader> {
        let sock = link.lock().unwrap().sock.try_clone()?;
        Ok(LinkReader { sock, link: link.clone(), raw: vec![0; 16 * 1024] })
    }

    pub(crate) fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        #[cfg(feature = "tls")]
        if let Some(tls) = &mut self.tls {
            tls.writer().write_all(data)?;
            while tls.wants_write() {
                tls.write_tls(&mut self.sock)?;
            }
            return Ok(());
        }
        self.sock.write_all(data)
    }

    pub(crate) fn shutdown(&mut self) -> io::Result<()> {
        #[cfg(feature = "tls")]
        if let Some(tls) = &mut self.tls {
            tls.send_close_notify();
            while tls.wants_write() {
                if tls.write_tls(&mut self.sock).is_err() {
                    break;
                }
            }
        }
        self.sock.shutdown(Shutdown::Both)
    }
}

/// The reading side: see the module documentation.
pub(crate) struct LinkReader {
    sock: TcpStream,
    #[cfg_attr(not(feature = "tls"), allow(dead_code))]
    link: Arc<Mutex<Link>>,
    #[cfg_attr(not(feature = "tls"), allow(dead_code))]
    raw: Vec<u8>,
}

impl Read for LinkReader {
    #[cfg(not(feature = "tls"))]
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.sock.read(out)
    }

    #[cfg(feature = "tls")]
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.link.lock().unwrap().tls.is_none() {
            return self.sock.read(out);
        }
        loop {
            {
                let mut link = self.link.lock().unwrap();
                let tls = link.tls.as_mut().unwrap();
                match tls.reader().read(out) {
                    Ok(n) => return Ok(n),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e),
                }
            }
            // Wait for the broker without the lock, so writers can go on.
            let n = self.sock.read(&mut self.raw)?;
            if n == 0 {
                return Ok(0);
            }
            let mut link = self.link.lock().unwrap();
            let Link { sock, tls } = &mut *link;
            let tls = tls.as_mut().unwrap();
            let mut data = &self.raw[..n];
            while !data.is_empty() {
                tls.read_tls(&mut data)?;
                tls.process_new_packets().map_err(tls_error)?;
            }
            // Answers the protocol itself needs (key updates, alerts).
            while tls.wants_write() {
                tls.write_tls(sock)?;
            }
        }
    }
}

#[cfg(feature = "tls")]
fn root_store(settings: &Tls) -> io::Result<rustls::RootCertStore> {
    use rustls::pki_types::{pem::PemObject, CertificateDer};

    let mut store = rustls::RootCertStore::empty();
    let file = settings.ca_file.clone().or_else(|| {
        let system = PathBuf::from(SYSTEM_CA_FILE);
        system.exists().then_some(system)
    });
    let Some(file) = file else {
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        return Ok(store);
    };
    let certs = CertificateDer::pem_file_iter(&file)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", file.display())))?;
    for cert in certs {
        let cert = cert.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", file.display())))?;
        store.add(cert).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", file.display())))?;
    }
    if store.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("{}: no certificates", file.display())));
    }
    Ok(store)
}

#[cfg(feature = "tls")]
fn tls_error(e: rustls::Error) -> io::Error {
    add_time_hint(io::Error::new(io::ErrorKind::InvalidData, format!("TLS: {e}")))
}

/// A certificate "not valid yet" or "expired" usually means the clock is
/// wrong: the Pi starts at the image's build time or the last saved time.
#[cfg(feature = "tls")]
fn add_time_hint(e: io::Error) -> io::Error {
    let text = e.to_string();
    let lower = text.to_lowercase();
    if lower.contains("notvalidyet") || lower.contains("not valid yet") || lower.contains("expired") {
        io::Error::new(e.kind(), format!("{text} (is the clock right? check with date -u)"))
    } else {
        e
    }
}
