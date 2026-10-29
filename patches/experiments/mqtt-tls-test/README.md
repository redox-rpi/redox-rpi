# Experiment: mqtt-tls-test, MQTT over TLS against the test broker

For testing MQTT over TLS ([#56](https://github.com/redox-rpi/redox-rpi/issues/56)):
puts the CA certificate of the TLS test broker on homenodellm into the
image as `/etc/mqtt-ca.crt`. With that file present, `mqtt-pub`,
`mqtt-sub` and `ha-bridge` use TLS and port 8883 by default
([TLS in the tools' README](../../../tools/mqtt/README.md#tls)).

```bash
scripts/build-image.sh --experiment timesync --experiment timerd \
    --experiment mqtt --experiment ha-bridge --experiment mqtt-tls-test
```

Test only: the CA belongs to a throwaway test broker. A real setup puts
its own broker's CA into `/etc/mqtt-ca.crt` (or passes `--cafile`).

## The test broker

On homenodellm, `~/mqtt-broker/tls/`: a CA (`ca.crt`, EC P-256, ten years)
and a server certificate for `localhost`, `homenodellm`, `127.0.0.1`,
`10.0.2.2` and `192.168.178.120`, made with `openssl` on 2026-10-29.

- QEMU: `~/mqtt-broker/run-tls.sh` listens on `127.0.0.1:8883`, which the
  guest reaches as `10.0.2.2:8883`.
- Pi: `~/mqtt-broker/pi-test-broker-tls.sh &` listens on
  `192.168.178.120:8883` (TLS) and `:1883` (plain) for an hour and logs
  every message to `~/mqtt-test/pi-broker-tls.out`.

## Results

**Mac** (macOS, the tools built with `cargo build`), against mosquitto
2.0.22 with TLS through an SSH tunnel (2026-10-29):

| Check | Result |
|---|---|
| `mqtt-pub --cafile ca.crt`, `mqtt-sub --cafile ca.crt -t 'tls/#'` | messages arrive; QoS 1 confirmed |
| `MQTT_CAFILE=ca.crt mqtt-pub` | TLS without an option |
| `mqtt-sub -k 4`, idle for 12 s | stays connected (pings over TLS), then gets the message |
| `--tls` without the CA | `invalid peer certificate: UnknownIssuer (give the broker's CA certificate with --cafile)` |
| `-h ::1` (not in the certificate) | `certificate not valid for name "::1"; certificate is only valid for …` |
| server certificate valid from 2030 | `certificate not valid yet: … (is the clock right? check with date -u)` |
| TLS to the plain port 1883 | `unexpected end of file` |
| `--cafile /nope` | `/nope: I/O error: No such file or directory` |
| `MQTT_CAFILE=… --no-tls` to port 1883 | plain, works |

**QEMU** `raspi3b`, network boot, build `v0.3.0-dev-110-g9f47c94+x125f`
(experiments `ldso-diagnostics timesync timerd mqtt ha-bridge
mqtt-tls-test`), TLS broker on the build host (2026-10-29):

| Check | Result |
|---|---|
| Build | rustls 0.23.46 and Redox' `ring` (branch `redox-0.17.16`) build for `aarch64-unknown-redox` with the cargo template |
| `ha-bridge -d` at boot | `ha-bridge: 6 sensors announced on 10.0.2.2:8883 (TLS) as redox-rpi/redox`; `availability online` and the six configs at the TLS broker |
| `ha-bridge` timer | values over TLS every minute (one run took 12.5 s with other QEMUs busy on the host; the others 0.5-2.2 s) |
| `mqtt-pub -t tls/qemu -m hello-over-tls -r -q 1` (no TLS option: `/etc/mqtt-ca.crt`) | exit 0, at the broker |
| `mqtt-sub -t tls/qemu -C 1 -v` | gets the retained message |
| `mqtt-pub --no-tls -t plain/qemu …` | at the plain broker on port 1883 |
| `mqtt-pub --tls --cafile /etc/hostname …` | `10.0.2.2:8883: /etc/hostname: no certificates`, exit 1 |
| QEMU stopped | the broker publishes `availability offline` (last will) |

The clock was right from the start: the test CA was made before the image
was built, so the build time already lies in its validity. On a real Pi
the Pi test (test plan) checks the same against `192.168.178.120:8883`.
