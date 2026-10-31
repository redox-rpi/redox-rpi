//! Options shared by mqtt-pub and mqtt-sub (included by both).

use std::env;
use std::fs;
use std::path::Path;
use std::process;

/// Broker and login options. The broker defaults to `$MQTT_HOST`, else the
/// first line of `/etc/mqtt-host`, else localhost; the port to `$MQTT_PORT`,
/// else 8883 with TLS and 1883 without.
///
/// TLS is on with `--tls` or `--cafile`, with `$MQTT_CAFILE`, or when
/// `/etc/mqtt-ca.crt` exists; `--no-tls` turns it off. With TLS, a client
/// certificate comes from `--cert`, else `$MQTT_CERT`, else
/// `/etc/mqtt-client.crt` if it exists; its key from `--key`, else
/// `$MQTT_KEY`, else `/etc/mqtt-client.key` if it exists, else the
/// certificate file.
pub struct Common {
    pub host: String,
    /// True when `-h` was given; else `host` is the default.
    pub host_given: bool,
    /// None: the default for plain or TLS.
    pub port: Option<u16>,
    pub tls: Tls,
    pub ca_file: Option<String>,
    pub cert_file: Option<String>,
    pub key_file: Option<String>,
    pub options: mqtt::Options,
    pub qos: mqtt::QoS,
    pub topics: Vec<String>,
}

/// The default broker for all MQTT tools on this system.
const HOST_FILE: &str = "/etc/mqtt-host";
/// The broker's CA certificate; if it exists, the tools use TLS.
const CA_FILE: &str = "/etc/mqtt-ca.crt";
/// This system's client certificate and key, for brokers that want one.
const CERT_FILE: &str = "/etc/mqtt-client.crt";
const KEY_FILE: &str = "/etc/mqtt-client.key";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tls {
    /// TLS if a CA file is set by `$MQTT_CAFILE` or `/etc/mqtt-ca.crt`.
    Auto,
    On,
    Off,
}

impl Common {
    pub fn new(name: &str) -> Common {
        Common {
            host: default_host(),
            host_given: false,
            port: env::var("MQTT_PORT").ok().and_then(|p| p.parse().ok()),
            tls: Tls::Auto,
            ca_file: None,
            cert_file: None,
            key_file: None,
            options: mqtt::Options::new(&format!("{name}-{}", process::id())),
            qos: mqtt::QoS::AtMostOnce,
            topics: Vec::new(),
        }
    }

    /// Takes a shared option and its value; false if `flag` is not one.
    pub fn parse(&mut self, flag: &str, args: &mut impl Iterator<Item = String>, usage: &str) -> bool {
        let mut value = || args.next().unwrap_or_else(|| fail(&format!("{flag} needs a value"), usage));
        match flag {
            "-h" => {
                self.host = value();
                self.host_given = true;
            }
            "-p" => self.port = Some(value().parse().unwrap_or_else(|_| fail("bad port", usage))),
            "--tls" => self.tls = Tls::On,
            "--no-tls" => self.tls = Tls::Off,
            "--cafile" => {
                self.ca_file = Some(value());
                self.tls = Tls::On;
            }
            "--cert" => self.cert_file = Some(value()),
            "--key" => self.key_file = Some(value()),
            "-t" => self.topics.push(value()),
            "-q" => {
                self.qos = value()
                    .parse()
                    .ok()
                    .and_then(mqtt::QoS::from_u8)
                    .unwrap_or_else(|| fail("QoS must be 0 or 1", usage))
            }
            "-i" => self.options.client_id = value(),
            "-k" => self.options.keep_alive = value().parse().unwrap_or_else(|_| fail("bad keep-alive", usage)),
            "-u" => self.options.username = Some(value()),
            "-P" => self.options.password = Some(value()),
            _ => return false,
        }
        true
    }

    /// The CA file for TLS, or None for plain TCP. `Some(None)`: TLS with
    /// the system's or the built-in CA certificates.
    fn tls_ca_file(&self) -> Option<Option<String>> {
        let configured = || {
            env::var("MQTT_CAFILE")
                .ok()
                .filter(|f| !f.is_empty())
                .or_else(|| Path::new(CA_FILE).exists().then(|| CA_FILE.to_string()))
        };
        match self.tls {
            Tls::Off => None,
            Tls::On => Some(self.ca_file.clone().or_else(configured)),
            Tls::Auto => configured().map(Some),
        }
    }

    /// The client certificate and key files; the key may be None (in the
    /// certificate file).
    fn client_identity(&self) -> Option<(String, Option<String>)> {
        let from = |given: &Option<String>, var: &str, file: &str| {
            given
                .clone()
                .or_else(|| env::var(var).ok().filter(|f| !f.is_empty()))
                .or_else(|| Path::new(file).exists().then(|| file.to_string()))
        };
        let cert = from(&self.cert_file, "MQTT_CERT", CERT_FILE)?;
        Some((cert, from(&self.key_file, "MQTT_KEY", KEY_FILE)))
    }

    /// True when the connection will show a client certificate.
    #[allow(dead_code)] // only ha-bridge reports it
    pub fn uses_client_cert(&self) -> bool {
        self.uses_tls() && self.client_identity().is_some()
    }

    pub fn uses_tls(&self) -> bool {
        self.tls_ca_file().is_some()
    }

    pub fn address(&self) -> String {
        let default = if self.uses_tls() { mqtt::DEFAULT_TLS_PORT } else { mqtt::DEFAULT_PORT };
        let port = self.port.unwrap_or(default);
        if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]:{}", self.host, port)
        } else {
            format!("{}:{}", self.host, port)
        }
    }

    /// The options with TLS filled in; read each time, so a CA file put in
    /// place later counts.
    pub fn mqtt_options(&self) -> mqtt::Options {
        #[cfg_attr(not(feature = "tls"), allow(unused_mut))]
        let mut options = self.options.clone();
        #[cfg(feature = "tls")]
        {
            let identity = self.client_identity();
            options.tls = self.tls_ca_file().map(|ca_file| mqtt::Tls {
                ca_file: ca_file.map(Into::into),
                server_name: None,
                cert_file: identity.as_ref().map(|(cert, _)| cert.into()),
                key_file: identity.as_ref().and_then(|(_, key)| key.as_ref().map(Into::into)),
            });
        }
        options
    }

    pub fn connect(&self, name: &str) -> mqtt::Client {
        #[cfg(not(feature = "tls"))]
        if self.uses_tls() {
            die(name, "built without TLS (feature tls)");
        }
        mqtt::Client::connect(&self.address(), &self.mqtt_options())
            .unwrap_or_else(|e| die(name, &e.to_string()))
    }
}

pub fn default_host() -> String {
    env::var("MQTT_HOST").ok().or_else(host_file).unwrap_or_else(|| "localhost".into())
}

fn host_file() -> Option<String> {
    let text = fs::read_to_string(HOST_FILE).ok()?;
    let host = text.lines().next()?.trim();
    (!host.is_empty()).then(|| host.to_string())
}

pub const COMMON_USAGE: &str = "  -h host     broker (default $MQTT_HOST, else /etc/mqtt-host, else localhost)
  -p port     port (default $MQTT_PORT, else 8883 with TLS, 1883 without)
  --tls       TLS (also on with $MQTT_CAFILE or if /etc/mqtt-ca.crt exists)
  --cafile f  TLS, trusting the CA certificates in PEM file f
              (default $MQTT_CAFILE, else /etc/mqtt-ca.crt, else the system's)
  --no-tls    plain TCP even if a CA file is set
  --cert f    TLS client certificate, PEM (default $MQTT_CERT, else
              /etc/mqtt-client.crt if it exists)
  --key f     its private key, PEM (default $MQTT_KEY, else
              /etc/mqtt-client.key if it exists, else in the --cert file)
  -q 0|1      QoS (default 0)
  -i id       client id (default <tool>-<pid>)
  -k seconds  keep-alive (default 60)
  -u user     user name
  -P password password";

pub fn fail(message: &str, usage: &str) -> ! {
    eprintln!("{message}\n{usage}");
    process::exit(2);
}

pub fn die(name: &str, message: &str) -> ! {
    eprintln!("{name}: {message}");
    process::exit(1);
}
