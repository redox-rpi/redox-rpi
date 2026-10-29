# Smart home

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

## Next

- Let other recipes use the library (now only by copying `src/lib.rs`).

## Later

- TLS (needs a TLS library that builds for Redox).
- A service that publishes GPIO inputs and switches GPIO outputs by MQTT
  (with the `gpio` topic).
- Home Assistant MQTT discovery, so the Pi shows up by itself.
