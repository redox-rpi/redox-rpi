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

## Next

- Check `ha-bridge` against a real Home Assistant.
- Let other recipes use the library (now only by copying `src/lib.rs`).

## Later

- TLS (needs a TLS library that builds for Redox).
- GPIO inputs as binary sensors in `ha-bridge`, and GPIO outputs as
  switches, once the Firmware session's GPIO driver is on `main`.
