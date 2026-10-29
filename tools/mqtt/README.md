# mqtt

A small MQTT 3.1.1 client for Redox: a Rust library and the tools
`mqtt-pub` and `mqtt-sub`, which work like mosquitto's `mosquitto_pub` and
`mosquitto_sub` ([#49](https://github.com/redox-rpi/redox-rpi/issues/49)).

It uses only `std`, with no crates from crates.io, so it builds for Redox
like the other tools here and needs no port of a C library.

**Supported:** MQTT 3.1.1 over TCP. Connect with a user name and password,
publish with QoS 0 or 1 and retain, subscribe with `+` and `#`, and keep-alive
pings. **Not supported:** TLS, QoS 2, last will, MQTT 5.

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

## Library

```rust
let mut client = mqtt::Client::connect("192.168.178.120:1883", &mqtt::Options::new("my-service"))?;
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
