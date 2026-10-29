# Smart home

User docs, with examples: [docs/smarthome.md](../smarthome.md) (branch
`docs-time-smarthome`).

## State

- Nothing on `main` yet.

## In progress

- [#49](https://github.com/redox-rpi/redox-rpi/issues/49) MQTT client:
  library and the tools `mqtt-pub` and `mqtt-sub` in
  [`tools/mqtt`](../../tools/mqtt), plain Rust without crates.io
  dependencies. Branch `mqtt` (based on `timerd`), experiments `mqtt` and
  `mqtt-demo` (a `timerd` timer publishes the temperature and uptime every
  minute, [README](../../patches/experiments/mqtt/README.md)). Passed in
  QEMU against mosquitto; Pi test #58 queued.
- Home Assistant MQTT discovery: `ha-bridge` (in `tools/mqtt`, experiment
  `ha-bridge`, same branch) announces CPU temperature, CPU clock,
  throttling, uptime and memory as a Home Assistant device, sends the
  values every minute through `timerd`, and keeps an availability topic
  with a last will ([README](../../patches/experiments/ha-bridge/README.md)).
  Passed in QEMU against mosquitto; Pi test #65 queued.
- [#56](https://github.com/redox-rpi/redox-rpi/issues/56) MQTT over TLS
  (port 8883) for all three tools: rustls with Redox' `ring` fork, Cargo
  feature `tls`; `--tls`, `--cafile`, `$MQTT_CAFILE`, `/etc/mqtt-ca.crt`.
  Branch `mqtt-tls` (based on `mqtt`), test experiment `mqtt-tls-test`
  ([README](../../patches/experiments/mqtt-tls-test/README.md)). Needs the
  right time first: see "TLS" in the [tools' README](../../tools/mqtt/README.md#tls).

## Next

- Check `ha-bridge` against a real Home Assistant.
- Let other recipes use the library (now only by copying `src/lib.rs`).
- TLS client certificates (Mosquitto `require_certificate`).

## Later

- GPIO inputs as binary sensors in `ha-bridge`, and GPIO outputs as
  switches, once the Firmware session's GPIO driver is on `main`.
