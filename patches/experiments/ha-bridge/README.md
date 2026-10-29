# Experiment: ha-bridge, the Pi in Home Assistant

`ha-bridge` (in [`tools/mqtt`](../../../tools/mqtt#ha-bridge-home-assistant))
announces the Pi's sensors to Home Assistant by MQTT discovery: CPU
temperature, CPU clock, throttling/under-voltage, uptime, free and used
memory. GPIO inputs follow once the GPIO driver is on `main`.

```bash
scripts/build-image.sh --experiment timesync --experiment timerd \
    --experiment mqtt --experiment ha-bridge
```

The experiment has no patches, only a [`config.toml`](config.toml):

- `75_ha-bridge.service` runs `ha-bridge -d`. It announces the sensors,
  keeps `redox-rpi/<node>/availability` at `online` (with a last will of
  `offline`), and announces again when Home Assistant restarts.
- `/usr/lib/timer.d/ha-bridge.toml` runs `ha-bridge` every minute through
  `timerd` to send the values.

The broker is the one in `/etc/mqtt-host` (experiment `mqtt`; `10.0.2.2`
for QEMU). On a Pi: `echo <broker address> > /etc/mqtt-host`; the service
picks it up at its next try (within 30 s), and the timer at its next run.

## In Home Assistant

With the MQTT integration set up against the same broker, the device
"Redox redox" appears under Settings → Devices with the six sensors. No
YAML is needed. To remove it: `ha-bridge --remove`.

## Results

QEMU `raspi3b`, network boot, build `v0.3.0-dev-102-g0afe025+x3029`,
mosquitto 2.0.22 on the build host, topics watched with `mosquitto_sub`
(2026-10-29):

| Check | Result |
|---|---|
| Service start | `availability online`, six retained configs (5 `sensor`, 1 `binary_sensor`), first values, about 70 s after boot |
| Values | `{"cpu_temperature":25.0,"cpu_clock":700,"throttled":"OFF","uptime":187,"memory_free":800.9,"memory_used":12.2}` every minute from the timer |
| Memory against `free` | `free` 808408 KiB free, `ha-bridge` 789.4 MiB: the same |
| Device | `"model":"Raspberry Pi 3 Model B","sw_version":"redox-rpi v0.3.0-dev-102-g0afe025+x3029"` |
| `homeassistant/status online` | all six configs and the values again within a second |
| QEMU frozen (`kill -STOP`, like a hang) | broker published `offline` (retained) after 41 s |
| QEMU thawed | the service noticed the broken connection, connected again 30 s later: `online` |
| QEMU killed | `offline` at once |

On the Mac, `ha-bridge --remove` cleared all retained topics. Not tested
yet: a real Home Assistant (the topics and fields follow its MQTT
discovery documentation) and the Pi (temperature, clock and throttling
values from real firmware).
