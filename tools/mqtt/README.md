# mqtt

A small MQTT 3.1.1 client for Redox: a Rust library, the tools `mqtt-pub`
and `mqtt-sub`, which work like mosquitto's `mosquitto_pub` and
`mosquitto_sub` ([#49](https://github.com/redox-rpi/redox-rpi/issues/49)),
and `ha-bridge`, which shows the Pi's sensors in Home Assistant.

It uses only `std`, with no crates from crates.io, so it builds for Redox
like the other tools here and needs no port of a C library.

**Supported:** MQTT 3.1.1 over TCP. Connect with a user name, password and
last will, publish with QoS 0 or 1 and retain, subscribe with `+` and `#`, and keep-alive
pings. **Not supported:** TLS, QoS 2, MQTT 5.

## Tools

```sh
mqtt-pub -h 192.168.178.120 -t home/pi/hello -m "hello"
mqtt-pub -t home/pi/temp -f /scheme/rpi-firmware/temperature -r
uptime | mqtt-pub -t home/pi/uptime -l -q 1
mqtt-sub -h 192.168.178.120 -t 'home/#' -v
mqtt-sub -t home/pi/cmd -C 1          # wait for one message, then exit
```

| Option | Tool | Meaning |
|---|---|---|
| `-h host` | both | broker (default `$MQTT_HOST`, else the first line of `/etc/mqtt-host`, else `localhost`) |
| `-p port` | both | port (default `$MQTT_PORT`, else 1883) |
| `-t topic` | both | topic; `mqtt-sub` takes several filters |
| `-q 0\|1` | both | QoS (default 0) |
| `-i id` | both | client id (default `<tool>-<pid>`) |
| `-k seconds` | both | keep-alive (default 60) |
| `-u user`, `-P password` | both | login |
| `-m message` | pub | the message |
| `-f file` | pub | the content of a file, without a trailing newline |
| `-s` | pub | all of standard input |
| `-l` | pub | each line of standard input as its own message |
| `-n` | pub | an empty message (with `-r`, clears a retained message) |
| `-r` | pub | retain on the broker |
| `-v` | sub | print the topic before each message |
| `-C count` | sub | exit after this many messages |

Exit code 1 on a network or broker error, 2 on bad options.

## ha-bridge: Home Assistant

`ha-bridge` announces the Pi's sensors to Home Assistant by
[MQTT discovery](https://www.home-assistant.io/integrations/mqtt/#mqtt-discovery),
so the Pi shows up as a device without any setup in Home Assistant:

| Sensor | From | Unit |
|---|---|---|
| CPU temperature | `/scheme/rpi-firmware/temperature` | °C |
| CPU clock | `/scheme/rpi-firmware/arm_clock` | MHz |
| Throttled or under-voltage (binary) | `/scheme/rpi-firmware/throttled`, bits 0-3 | on/off |
| Uptime | `clock_gettime(CLOCK_MONOTONIC)` | s |
| Free memory, memory used | `statvfs("/scheme/memory")`, as `free` | MiB, % |

A sensor without a source (no `rpi-firmware` in QEMU without the driver,
for example) is not announced.

```sh
ha-bridge -d        # service: discovery, availability, last will
ha-bridge           # send the values once (a timerd timer, every minute)
ha-bridge -n        # print the configs and values, no broker
ha-bridge --remove  # take the sensors out of Home Assistant again
```

Topics, with `<node>` from `/etc/hostname` in lower case (`-N` to set it):

| Topic | Content |
|---|---|
| `homeassistant/sensor/<node>/<key>/config` (and `binary_sensor`) | discovery config, retained |
| `redox-rpi/<node>/state` | all values as one JSON object, e.g. `{"cpu_temperature":47.2,"cpu_clock":1400,"throttled":"OFF","uptime":312,"memory_free":790.4,"memory_used":13.3}` |
| `redox-rpi/<node>/availability` | `online`, retained; the last will sets `offline` |

The service keeps one connection with a 30 s keep-alive. If the Pi loses
power or hangs, the broker publishes `offline` after about 45 s and Home
Assistant greys out the sensors. Each sensor also expires after 3 minutes
without a value. When Home Assistant restarts and publishes `online` on
`homeassistant/status`, the service sends the configs again. It tries to
connect every 30 s until the broker answers.

## Library

```rust
let mut options = mqtt::Options::new("my-service");
options.will = Some(mqtt::Will {
    topic: "home/pi/state".into(),
    payload: b"offline".to_vec(),
    qos: mqtt::QoS::AtLeastOnce,
    retain: true,
});
let mut client = mqtt::Client::connect("192.168.178.120:1883", &options)?;
client.publish("home/pi/state", b"on", mqtt::QoS::AtLeastOnce, true)?;
client.subscribe(&[("home/pi/cmd", mqtt::QoS::AtMostOnce)])?;
loop {
    let message = client.recv()?;
    // ...
}
```

A background thread sends a ping every half keep-alive interval, so a
service that publishes only now and then stays connected. The DNS lookup
has a 5 s timeout, because Redox' own lookup has none.

The recipe's source is this directory only, so a service in another
directory cannot use it as a `path` dependency in an image build yet. Until
then, a service can run `mqtt-pub`, or copy `src/lib.rs`.

## Tests

`cargo test` checks the packet encoding. Results against mosquitto 2.0.22
on the Mac and in QEMU are in
[patches/experiments/mqtt/README.md](../../patches/experiments/mqtt/README.md).
