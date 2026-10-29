//! Options shared by mqtt-pub and mqtt-sub (included by both).

use std::env;
use std::fs;
use std::process;

/// Broker and login options. The broker defaults to `$MQTT_HOST`, else the
/// first line of `/etc/mqtt-host`, else localhost; the port to `$MQTT_PORT`,
/// else 1883.
pub struct Common {
    pub host: String,
    /// True when `-h` was given; else `host` is the default.
    pub host_given: bool,
    pub port: u16,
    pub options: mqtt::Options,
    pub qos: mqtt::QoS,
    pub topics: Vec<String>,
}

/// The default broker for all MQTT tools on this system.
const HOST_FILE: &str = "/etc/mqtt-host";

impl Common {
    pub fn new(name: &str) -> Common {
        Common {
            host: default_host(),
            host_given: false,
            port: env::var("MQTT_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(mqtt::DEFAULT_PORT),
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
            "-p" => self.port = value().parse().unwrap_or_else(|_| fail("bad port", usage)),
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

    pub fn address(&self) -> String {
        if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    pub fn connect(&self, name: &str) -> mqtt::Client {
        mqtt::Client::connect(&self.address(), &self.options)
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

pub const COMMON_USAGE: &str = "\
  -h host     broker (default $MQTT_HOST, else /etc/mqtt-host, else localhost)
  -p port     port (default $MQTT_PORT, else 1883)
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
