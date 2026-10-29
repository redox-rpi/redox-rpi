# Experiment: mqtt, an MQTT client

[`tools/mqtt`](../../../tools/mqtt) is a small MQTT 3.1.1 client in plain
Rust: a library and the tools `mqtt-pub` and `mqtt-sub`
([#49](https://github.com/redox-rpi/redox-rpi/issues/49)).

```bash
scripts/build-image.sh --experiment mqtt
# with the demo:
scripts/build-image.sh --experiment timesync --experiment timerd \
    --experiment mqtt --experiment mqtt-demo
```

- **`mqtt`** adds the `mqtt` package (`mqtt-pub`, `mqtt-sub`, `ha-bridge`)
  and `/etc/mqtt-host`, the default broker of all three: `10.0.2.2`, the
  build host as QEMU's user network sees it. On a Pi:
  `echo 192.168.178.120 > /etc/mqtt-host`.
- **`mqtt-demo`** adds two `timerd` timers that publish every minute:
  - `redox-rpi/temperature`, the SoC temperature in m°C from
    `/scheme/rpi-firmware/temperature`;
  - `redox-rpi/uptime`, the output of `uptime`.
- **`ha-bridge`**: Home Assistant discovery, see
  [its README](../ha-bridge/README.md).

## Results

Against mosquitto 2.0.22 on the build host (2026-10-29).

**On the Mac** (`cargo test`, then the tools against the broker): the packet
tests pass. mosquitto_pub to `mqtt-sub` works with QoS 1, `+` and `#`, and
two filters. A 2 s keep-alive held the connection for 8 s with pings.
`mqtt-pub` to mosquitto_sub works with QoS 0 and 1, `-l`, `-f` and retain.
Wildcards in a publish topic, a refused port and an unknown host each give
exit code 1 and a clear message.

**In QEMU** (`raspi3b`, network boot, build
`v0.3.0-dev-100-g0a85e1e+xd656`, broker at 10.0.2.2), commands over devsh:

| Check | Result |
|---|---|
| `mqtt-pub -q 1 -m "hello from Redox"` | broker got it, exit 0 |
| retain, then `mqtt-sub -C 1` | `redox/qemu/retained kept` |
| `mqtt-sub -t 'redox/cmd/#' -q 1 -k 5 -C 2`, host publishes | both messages, QoS 1 and 0 |
| `echo "a b" \| mqtt-pub -l -q 1` | broker got `a b` |
| `-n -r` clears the retained message | broker got an empty retained message |
| broker from `/etc/mqtt-host` (no `-h`) | broker got it |
| `-p 1` (closed port) | `10.0.2.2:1: Transport endpoint is not connected`, exit 1 |
| `mqtt-demo` timers | `redox-rpi/temperature 25000` and `redox-rpi/uptime 4m 25s` every minute for 10 minutes; QEMU's firmware always says 25000 |

Notes:

- On Redox a refused connection shows only at the first write (error 107,
  not "connection refused").
- relibc does not know `TCP_NODELAY` and logs each try, so the client does
  not set it.
- Ion reads `#` as the start of a comment: quote filters like
  `'home/#'`.
