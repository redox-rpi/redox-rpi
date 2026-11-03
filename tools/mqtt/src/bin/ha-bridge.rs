//! ha-bridge: the Pi's sensors in Home Assistant, by MQTT discovery.
//!
//! Usage:
//!   `ha-bridge -d`   service: announces the sensors (retained discovery
//!                    configs), keeps the availability topic `online` with a
//!                    last will of `offline`, and announces again when Home
//!                    Assistant restarts (`homeassistant/status`).
//!   `ha-bridge`      publishes the current values once (from a timerd timer).
//!   `ha-bridge -n`   prints the configs and values, without a broker.
//!   `ha-bridge --remove`  removes the sensors from Home Assistant.
//!
//! Topics, with `<node>` from `/etc/hostname` (or `-N`):
//!   `homeassistant/<sensor|binary_sensor>/<node>/<key>/config`
//!   `redox-rpi/<node>/state`         all values as one JSON object
//!   `redox-rpi/<node>/availability`  `online` / `offline`, retained

#[path = "../cli.rs"]
mod cli;

use std::fs;
use std::thread::sleep;
use std::time::Duration;

use cli::{die, fail, Common};
use mqtt::{QoS, Will};

const NAME: &str = "ha-bridge";
const FIRMWARE: &str = "/scheme/rpi-firmware";
const RELEASE: &str = "/etc/redox-rpi-release";
/// A sensor shows as unavailable when no value came for this long; the
/// timer sends them every minute.
const EXPIRE_AFTER: u32 = 180;
const RETRY_EVERY: Duration = Duration::from_secs(30);
const KEEP_ALIVE: u16 = 30;

struct Sensor {
    key: &'static str,
    name: &'static str,
    component: &'static str,
    unit: Option<&'static str>,
    device_class: Option<&'static str>,
    measurement: bool,
}

const SENSORS: &[Sensor] = &[
    Sensor {
        key: "cpu_temperature",
        name: "CPU temperature",
        component: "sensor",
        unit: Some("°C"),
        device_class: Some("temperature"),
        measurement: true,
    },
    Sensor {
        key: "cpu_clock",
        name: "CPU clock",
        component: "sensor",
        unit: Some("MHz"),
        device_class: Some("frequency"),
        measurement: true,
    },
    Sensor {
        key: "throttled",
        name: "Throttled or under-voltage",
        component: "binary_sensor",
        unit: None,
        device_class: Some("problem"),
        measurement: false,
    },
    Sensor {
        key: "uptime",
        name: "Uptime",
        component: "sensor",
        unit: Some("s"),
        device_class: Some("duration"),
        measurement: false,
    },
    Sensor {
        key: "memory_free",
        name: "Free memory",
        component: "sensor",
        unit: Some("MiB"),
        device_class: Some("data_size"),
        measurement: true,
    },
    Sensor {
        key: "memory_used",
        name: "Memory used",
        component: "sensor",
        unit: Some("%"),
        device_class: None,
        measurement: true,
    },
];

fn usage() -> String {
    format!(
        "usage: ha-bridge [options] [-d | -n | --remove]
  -d          run as a service: discovery, availability, last will
  -n          print the discovery configs and values, do not connect
  --remove    remove the sensors from Home Assistant
  (none)      publish the current values once
  -N node     node name (default /etc/hostname)
  -D prefix   discovery prefix (default homeassistant)
{}",
        cli::COMMON_USAGE
    )
}

#[derive(PartialEq)]
enum Mode {
    Values,
    Service,
    Print,
    Remove,
}

struct Bridge {
    node: String,
    prefix: String,
    device: String,
}

impl Bridge {
    fn base(&self) -> String {
        format!("redox-rpi/{}", self.node)
    }

    fn availability(&self) -> String {
        format!("{}/availability", self.base())
    }

    fn state_topic(&self) -> String {
        format!("{}/state", self.base())
    }

    fn config_topic(&self, sensor: &Sensor) -> String {
        format!("{}/{}/{}/{}/config", self.prefix, sensor.component, self.node, sensor.key)
    }

    fn config(&self, sensor: &Sensor) -> String {
        let mut fields = vec![
            json_field("name", sensor.name),
            json_field("unique_id", &format!("{}_{}", self.node, sensor.key)),
            json_field("state_topic", &self.state_topic()),
            json_field("value_template", &format!("{{{{ value_json.{} }}}}", sensor.key)),
            json_field("availability_topic", &self.availability()),
            format!("\"expire_after\":{EXPIRE_AFTER}"),
        ];
        if let Some(unit) = sensor.unit {
            fields.push(json_field("unit_of_measurement", unit));
        }
        if let Some(class) = sensor.device_class {
            fields.push(json_field("device_class", class));
        }
        if sensor.measurement {
            fields.push(json_field("state_class", "measurement"));
        }
        if sensor.key == "memory_used" {
            fields.push(json_field("icon", "mdi:memory"));
        }
        fields.push(format!("\"device\":{}", self.device));
        format!("{{{}}}", fields.join(","))
    }

    /// The sensors that have a value on this system.
    fn present(&self, values: &[(&str, String)]) -> Vec<&'static Sensor> {
        SENSORS.iter().filter(|s| values.iter().any(|(k, _)| *k == s.key)).collect()
    }

    fn announce(&self, client: &mut mqtt::Client, values: &[(&str, String)]) -> std::io::Result<usize> {
        let present = self.present(values);
        for sensor in &present {
            client.publish(&self.config_topic(sensor), self.config(sensor).as_bytes(), QoS::AtLeastOnce, true)?;
        }
        Ok(present.len())
    }

    fn publish_values(&self, client: &mut mqtt::Client, values: &[(&str, String)]) -> std::io::Result<()> {
        client.publish(&self.state_topic(), state_json(values).as_bytes(), QoS::AtMostOnce, false)
    }
}

fn main() {
    let usage = usage();
    let mut common = Common::new(NAME);
    let mut mode = Mode::Values;
    let mut node = None;
    let mut prefix = "homeassistant".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        if common.parse(&flag, &mut args, &usage) {
            continue;
        }
        let mut value = || args.next().unwrap_or_else(|| fail(&format!("{flag} needs a value"), &usage));
        match flag.as_str() {
            "-d" => mode = Mode::Service,
            "-n" => mode = Mode::Print,
            "--remove" => mode = Mode::Remove,
            "-N" => node = Some(value()),
            "-D" => prefix = value(),
            "--help" => {
                println!("{usage}");
                return;
            }
            _ => fail(&format!("unknown option {flag}"), &usage),
        }
    }
    let node = sanitize(&node.unwrap_or_else(|| {
        fs::read_to_string("/etc/hostname").ok().map(|h| h.trim().to_string()).filter(|h| !h.is_empty()).unwrap_or_else(|| "redox".into())
    }));
    let bridge = Bridge { device: device_json(&node), node, prefix };

    match mode {
        Mode::Print => {
            let values = read_values();
            for sensor in bridge.present(&values) {
                println!("{} {}", bridge.config_topic(sensor), bridge.config(sensor));
            }
            println!("{} {}", bridge.state_topic(), state_json(&values));
        }
        Mode::Values => {
            let mut client = common.connect(NAME);
            bridge.publish_values(&mut client, &read_values()).unwrap_or_else(|e| die(NAME, &e.to_string()));
            let _ = client.disconnect();
        }
        Mode::Remove => {
            let mut client = common.connect(NAME);
            for sensor in SENSORS {
                client
                    .publish(&bridge.config_topic(sensor), b"", QoS::AtLeastOnce, true)
                    .unwrap_or_else(|e| die(NAME, &e.to_string()));
            }
            client.publish(&bridge.availability(), b"", QoS::AtLeastOnce, true).unwrap_or_else(|e| die(NAME, &e.to_string()));
            let _ = client.disconnect();
        }
        Mode::Service => {
            common.options.keep_alive = KEEP_ALIVE;
            common.options.will =
                Some(Will { topic: bridge.availability(), payload: b"offline".to_vec(), qos: QoS::AtLeastOnce, retain: true });
            loop {
                // /etc/mqtt-host may have changed since the last try.
                if !common.host_given {
                    common.host = cli::default_host();
                }
                if let Err(e) = serve(&bridge, &common) {
                    eprintln!("{NAME}: {e}; trying again in {} s", RETRY_EVERY.as_secs());
                }
                sleep(RETRY_EVERY);
            }
        }
    }
}

/// One connection of the service; returns when it breaks.
fn serve(bridge: &Bridge, common: &Common) -> std::io::Result<()> {
    let mut client = mqtt::Client::connect(&common.address(), &common.mqtt_options())?;
    client.publish(&bridge.availability(), b"online", QoS::AtLeastOnce, true)?;
    let values = read_values();
    let count = bridge.announce(&mut client, &values)?;
    bridge.publish_values(&mut client, &values)?;
    let status = format!("{}/status", bridge.prefix);
    client.subscribe(&[(&status, QoS::AtLeastOnce)])?;
    let tls = if common.uses_tls() { " (TLS)" } else { "" };
    eprintln!("{NAME}: {count} sensors announced on {}{tls} as {}", common.address(), bridge.base());
    loop {
        let message = client.recv()?;
        if message.topic == status && message.payload == b"online" {
            // Home Assistant restarted: it wants the configs again.
            let values = read_values();
            bridge.announce(&mut client, &values)?;
            bridge.publish_values(&mut client, &values)?;
            eprintln!("{NAME}: Home Assistant is back, sensors announced again");
        }
    }
}

/// The current values, as JSON values, for the sensors that have one.
fn read_values() -> Vec<(&'static str, String)> {
    let mut values = Vec::new();
    if let Some(temperature) = read_number(&format!("{FIRMWARE}/temperature")) {
        values.push(("cpu_temperature", format!("{:.1}", temperature as f64 / 1000.0)));
    }
    if let Some(clock) = read_number(&format!("{FIRMWARE}/arm_clock")) {
        values.push(("cpu_clock", (clock / 1_000_000).to_string()));
    }
    if let Some(flags) = fs::read_to_string(format!("{FIRMWARE}/throttled"))
        .ok()
        .and_then(|s| u64::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok())
    {
        // Bits 0-3: under-voltage, frequency capped, throttled, soft
        // temperature limit, now (bits 16-19: since boot).
        values.push(("throttled", json_string(if flags & 0xf != 0 { "ON" } else { "OFF" })));
    }
    if let Some(uptime) = sys::uptime() {
        values.push(("uptime", uptime.to_string()));
    }
    if let Some((total, free)) = sys::memory() {
        values.push(("memory_free", format!("{:.1}", free as f64 / 1_048_576.0)));
        if total > 0 {
            values.push(("memory_used", format!("{:.1}", 100.0 * (total - free) as f64 / total as f64)));
        }
    }
    values
}

fn read_number(path: &str) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn device_json(node: &str) -> String {
    let board = fs::read_to_string(format!("{FIRMWARE}/board")).unwrap_or_default();
    let model = board.lines().find_map(|l| l.strip_prefix("model ")).unwrap_or("Raspberry Pi").trim().to_string();
    let release = fs::read_to_string(RELEASE).unwrap_or_default();
    let version = release
        .lines()
        .find_map(|l| l.strip_prefix("BUILD_ID="))
        .map(|v| v.trim_matches('"').to_string())
        .unwrap_or_else(|| "unknown".into());
    format!(
        "{{{},{},{},{},{}}}",
        format!("\"identifiers\":[{}]", json_string(&format!("redox-rpi_{node}"))),
        json_field("name", &format!("Redox {node}")),
        json_field("manufacturer", "Raspberry Pi"),
        json_field("model", &model),
        json_field("sw_version", &format!("redox-rpi {version}")),
    )
}

fn state_json(values: &[(&str, String)]) -> String {
    let fields: Vec<String> = values.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
    format!("{{{}}}", fields.join(","))
}

/// Node names go into topics and ids: lower case, `[a-z0-9_-]` only.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c.to_ascii_lowercase() } else { '_' })
        .collect()
}

fn json_field(key: &str, value: &str) -> String {
    format!("\"{key}\":{}", json_string(value))
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Uptime and memory from relibc, declared here instead of using the libc
/// crate, so the package keeps building without crates.io.
mod sys {
    #[repr(C)]
    struct Timespec {
        tv_sec: i64,
        tv_nsec: i64,
    }

    extern "C" {
        fn clock_gettime(clock: i32, tp: *mut Timespec) -> i32;
    }

    #[cfg(target_os = "redox")]
    const CLOCK_MONOTONIC: i32 = 4;
    #[cfg(target_os = "macos")]
    const CLOCK_MONOTONIC: i32 = 6;
    #[cfg(not(any(target_os = "redox", target_os = "macos")))]
    const CLOCK_MONOTONIC: i32 = 1;

    /// Seconds since boot.
    pub fn uptime() -> Option<u64> {
        let mut ts = Timespec { tv_sec: 0, tv_nsec: 0 };
        let result = unsafe { clock_gettime(CLOCK_MONOTONIC, &mut ts) };
        (result == 0 && ts.tv_sec >= 0).then_some(ts.tv_sec as u64)
    }

    /// Total and free memory in bytes, from `statvfs` on the kernel's
    /// memory scheme (what `free` shows).
    #[cfg(target_os = "redox")]
    pub fn memory() -> Option<(u64, u64)> {
        // relibc's struct statvfs on 64-bit: eleven unsigned longs.
        #[repr(C)]
        #[derive(Default)]
        struct StatVfs {
            f_bsize: u64,
            f_frsize: u64,
            f_blocks: u64,
            f_bfree: u64,
            f_bavail: u64,
            f_files: u64,
            f_ffree: u64,
            f_favail: u64,
            f_fsid: u64,
            f_flag: u64,
            f_namemax: u64,
        }
        extern "C" {
            fn statvfs(path: *const std::ffi::c_char, buf: *mut StatVfs) -> i32;
        }
        let mut stat = StatVfs::default();
        let result = unsafe { statvfs(c"/scheme/memory".as_ptr(), &mut stat) };
        (result == 0 && stat.f_blocks > 0).then(|| (stat.f_blocks * stat.f_bsize, stat.f_bfree * stat.f_bsize))
    }

    #[cfg(not(target_os = "redox"))]
    pub fn memory() -> Option<(u64, u64)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_and_state() {
        let bridge = Bridge { node: "pi".into(), prefix: "homeassistant".into(), device: "{}".into() };
        let sensor = &SENSORS[0];
        assert_eq!(bridge.config_topic(sensor), "homeassistant/sensor/pi/cpu_temperature/config");
        let config = bridge.config(sensor);
        assert!(config.contains("\"value_template\":\"{{ value_json.cpu_temperature }}\""), "{config}");
        assert!(config.contains("\"unit_of_measurement\":\"°C\""));
        assert!(config.contains("\"availability_topic\":\"redox-rpi/pi/availability\""));
        assert_eq!(state_json(&[("a", "1".into()), ("b", json_string("ON"))]), "{\"a\":1,\"b\":\"ON\"}");
    }

    #[test]
    fn names() {
        assert_eq!(sanitize("Redox Pi.3"), "redox_pi_3");
        assert_eq!(json_string("a\"b\\\n"), "\"a\\\"b\\\\\\u000a\"");
    }
}
