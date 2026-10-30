# Smart home

User docs, with examples: [docs/smarthome.md](../smarthome.md) (branch
`docs-time-smarthome`).

## State

- Nothing on `main` yet.

## In progress

All three on branch `mqtt-tls` (it contains `mqtt`, which is based on
`timerd`), in draft PR #89 (merge after the `timerd` PR #59):

- [#49](https://github.com/redox-rpi/redox-rpi/issues/49) MQTT client:
  library and the tools `mqtt-pub` and `mqtt-sub` in
  [`tools/mqtt`](../../tools/mqtt), experiments `mqtt` and `mqtt-demo` (a
  `timerd` timer publishes the temperature and uptime every minute,
  [README](../../patches/experiments/mqtt/README.md)). A broker that does
  not answer now ends the connect after 10 s (Pi tests #58 and #65 hung
  there). **Pi test 88 passed** (plain MQTT, 2026-10-30).
- Home Assistant MQTT discovery: `ha-bridge` (in `tools/mqtt`, experiment
  `ha-bridge`) announces CPU temperature, CPU clock, throttling, uptime and
  memory as a Home Assistant device, sends the values every minute through
  `timerd`, and keeps an availability topic with a last will
  ([README](../../patches/experiments/ha-bridge/README.md)). **Pi tests 88
  (plain) and 89 (TLS) passed.**
- [#56](https://github.com/redox-rpi/redox-rpi/issues/56) MQTT over TLS
  (port 8883) for all three tools: rustls with Redox' `ring` fork, Cargo
  feature `tls`; `--tls`, `--cafile`, `$MQTT_CAFILE`, `/etc/mqtt-ca.crt`;
  test experiment `mqtt-tls-test`
  ([README](../../patches/experiments/mqtt-tls-test/README.md)). Needs the
  right time first: see "TLS" in the [tools' README](../../tools/mqtt/README.md#tls).
  **Pi test 89 passed**, but free memory fell 5.5 MB a minute; the MQTT
  part was the connect watchdog thread, which the kernel did not free
  (#31, kernel `0008`, on main). **Pi test 111 passed** with main merged:
  0.21 MB a minute over 30 minutes, `ha-bridge` does not grow.

## Next

- `netstack` still grows about 58 KB a minute with a connection a minute,
  and `randd`'s shared memory 12 KB a minute (one random read per TLS run;
  Pi test 111). Network topic for `netstack`.
- Check `ha-bridge` against a real Home Assistant.
- Let other recipes use the library (now only by copying `src/lib.rs`).
- TLS client certificates (Mosquitto `require_certificate`).

## Later

- GPIO inputs as binary sensors in `ha-bridge`, and GPIO outputs as
  switches, once the Firmware session's GPIO driver is on `main`.
