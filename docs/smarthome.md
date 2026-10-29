# Smart home: MQTT and Home Assistant

The Pi can send values to an MQTT broker and receive messages from it, and
it can show up in Home Assistant as a device with its own sensors. For
people using the image; the [user guide](user-guide.md) covers the basics
of Redox first, [time.md](time.md) the timers used here.

## What is where

All of it is on branch `mqtt` (built on branch `timerd`), issue
[#49](https://github.com/redox-rpi/redox-rpi/issues/49), planned for v0.8.0.
It passed in QEMU against a Mosquitto broker; the tests on a real Pi are
queued (#58 and #65 in the [test plan](test-plan.md)).

| Experiment | What it adds |
|---|---|
| `mqtt` | the tools `mqtt-pub`, `mqtt-sub` and `ha-bridge`, and `/etc/mqtt-host` |
| `mqtt-demo` | two timers that publish the temperature and uptime every minute |
| `ha-bridge` | the Home Assistant service and its timer |

```bash
scripts/build-image.sh --experiment timesync --experiment timerd \
    --experiment mqtt --experiment ha-bridge
```

**Not supported:** TLS (encrypted connections), MQTT 5 and QoS 2. User
name and password go over the network in plain text: use it only in your
home network.

## A broker

MQTT needs a broker, a server that passes messages on. Most homes with
Home Assistant have one: the **Mosquitto broker** add-on. Its address is
the Home Assistant machine's, port 1883; it wants a user name and
password (Home Assistant users work, or make one for the Pi).

For a test without Home Assistant, Mosquitto on any Linux machine:

```
# /etc/mosquitto/conf.d/pi.conf
listener 1883
allow_anonymous true
```

`allow_anonymous true` lets everyone in your network publish; fine for a
test, not for daily use.

## Telling the Pi where the broker is

All three tools take the broker from, in this order:

1. `-h <host>` and `-p <port>` on the command line,
2. the environment variables `MQTT_HOST` and `MQTT_PORT`,
3. the first line of `/etc/mqtt-host`,
4. `localhost`, port 1883.

So one line sets it for everything, as root:

```sh
echo 192.168.178.120 > /etc/mqtt-host
```

The image from experiment `mqtt` has `10.0.2.2` there: that is the build
machine as QEMU sees it. On a Pi, change it as above.

On the network-boot image the root filesystem is in memory, so the change
is lost at reboot. To keep it, put the address into the image's
configuration: the `[[files]]` entry for `/etc/mqtt-host` in
[`patches/experiments/mqtt/config.toml`](../patches/experiments/mqtt/config.toml).

A login goes with `-u <user> -P <password>` on each call.

## Sending and receiving

```sh
mqtt-pub -t home/pi/hello -m "hello from Redox"
mqtt-pub -t home/pi/temperature -f /scheme/rpi-firmware/temperature
mqtt-pub -t home/pi/state -m on -r -q 1      # retained, QoS 1
mqtt-pub -t home/pi/state -n -r              # delete the retained message
uptime | mqtt-pub -t home/pi/uptime -l        # one message per input line

mqtt-sub -t 'home/#' -v                        # print everything under home/
mqtt-sub -t home/pi/cmd -C 1                   # wait for one message, then exit
```

In Ion, `#` starts a comment: always quote topic filters with `#`, as
`'home/#'`. Without quotes Ion drops the `#` and `mqtt-sub` waits for a
topic that never comes.

| Option | Tool | Meaning |
|---|---|---|
| `-t topic` | both | the topic; `mqtt-sub` takes several, with `+` (one level) and `#` (the rest) |
| `-m text`, `-f file`, `-s`, `-l`, `-n` | pub | the message: text, a file's content, all of standard input, one per input line, empty |
| `-r` | pub | retained: the broker keeps it and gives it to everyone who subscribes later |
| `-q 0` or `-q 1` | both | QoS 0 (default, at most once) or 1 (the broker confirms) |
| `-v` | sub | print the topic before each message |
| `-C n` | sub | exit after n messages |
| `-u user`, `-P password` | both | login |
| `-i id`, `-k seconds` | both | client id (default `<tool>-<pid>`), keep-alive (default 60 s) |

Exit code 0 on success, 1 on a network or broker error (the message says
which: `192.168.178.120:1883: Transport endpoint is not connected` when no
broker listens there), 2 on wrong options. `--help` lists all options.

## Publishing at set times

With `timerd` (see [time.md](time.md#timers-running-commands-at-set-times)),
a timer file publishes on a schedule. Experiment `mqtt-demo` has two:

```toml
# /etc/timer.d/mqtt-temperature.toml
[timer]
description = "MQTT demo: SoC temperature (m°C) every minute"
schedule = "* * * * *"

[service]
cmd = "mqtt-pub"
args = ["-t", "redox-rpi/temperature", "-f", "/scheme/rpi-firmware/temperature"]
```

```toml
# /etc/timer.d/mqtt-uptime.toml: needs Ion for $(…)
[timer]
schedule = "* * * * *"

[service]
cmd = "ion"
args = ["-c", "mqtt-pub -t redox-rpi/uptime -m \"$(uptime)\""]
```

The temperature comes in thousandths of a degree: `47236` is 47.2 °C.
Other values from the firmware are in `/scheme/rpi-firmware`
([rpi-firmware.md](rpi-firmware.md)).

## Home Assistant

`ha-bridge` makes the Pi a device in Home Assistant, with no setup there:
it uses Home Assistant's
[MQTT discovery](https://www.home-assistant.io/integrations/mqtt/#mqtt-discovery).

**You need:** Home Assistant with the MQTT integration, connected to the
same broker (discovery is on by default), and an image with the
experiments `timesync`, `timerd`, `mqtt` and `ha-bridge`.

**On the Pi**, as root:

```sh
echo 192.168.178.50 > /etc/mqtt-host     # your broker
```

The service tries to connect every 30 seconds and reads the file again
each time, so no restart is needed. Within half a minute the boot log or
serial console shows:

```
ha-bridge: 6 sensors announced on 192.168.178.50:1883 as redox-rpi/redox
```

In Home Assistant the device **Redox redox** appears under Settings →
Devices & services → MQTT, with these sensors:

| Sensor | Unit | Source on the Pi |
|---|---|---|
| CPU temperature | °C | `/scheme/rpi-firmware/temperature` |
| CPU clock | MHz | `/scheme/rpi-firmware/arm_clock` |
| Throttled or under-voltage | on/off | `/scheme/rpi-firmware/throttled` (on: the power supply is too weak, or the Pi is too hot) |
| Uptime | s | time since boot |
| Free memory | MiB | as `free` shows |
| Memory used | % | as `free` shows |

The values come every minute from a `timerd` timer. A sensor without a
value for three minutes shows as unavailable.

**When the Pi is off:** `ha-bridge` keeps a connection open, and the
broker knows its "last will". If the Pi loses power or hangs, the broker
marks it `offline` after about 45 seconds, and Home Assistant greys out
all its sensors. When the Pi is back, `ha-bridge` connects again and sets
it `online`. When Home Assistant restarts, `ha-bridge` sends the device
description again.

**Broker with a login:** the Mosquitto add-on needs one. Add it to the
service's arguments in
[`patches/experiments/ha-bridge/config.toml`](../patches/experiments/ha-bridge/config.toml)
(`args = ["-d", "-u", "pi", "-P", "secret"]`) and to the timer's
(`args = ["-u", "pi", "-P", "secret"]`), then build the image. The
password is then readable in the image.

**Name:** the device and its topics take the name from `/etc/hostname`
(`redox`), in lower case. With several Pis, give each its own name with
`-N` in both places (`args = ["-d", "-N", "kitchen"]`), or change
`/etc/hostname` in the image.

**Commands:**

```sh
ha-bridge -n          # show what would be sent, without a broker
ha-bridge             # send the values once (what the timer does)
ha-bridge --remove    # take the device out of Home Assistant
```

`ha-bridge -n` prints each discovery message and the values, for example
`redox-rpi/redox/state {"cpu_temperature":47.2,"cpu_clock":1400,"throttled":"OFF","uptime":312,"memory_free":790.4,"memory_used":13.3}`.

**Topics**, to look at with `mosquitto_sub -v -t 'redox-rpi/#' -t 'homeassistant/#'`
on another computer:

| Topic | Content |
|---|---|
| `homeassistant/sensor/redox/<sensor>/config` | the description of each sensor, retained |
| `homeassistant/binary_sensor/redox/throttled/config` | the same for the on/off sensor |
| `redox-rpi/redox/state` | all values as one JSON object, every minute |
| `redox-rpi/redox/availability` | `online` or `offline`, retained |

**Not tested yet:** with a real Home Assistant (the messages follow its
documentation and were checked with `mosquitto_sub`), and on a real Pi
(test #65).

## Problems

| What you see | Why, and what to do |
|---|---|
| `Transport endpoint is not connected` | No broker at that address and port. Check `/etc/mqtt-host` and that the broker listens on the network, not only on `localhost`. |
| `broker refused the connection: bad user name or password` / `not authorized` | The broker wants a login: `-u` and `-P`. |
| `mqtt-sub -t home/#` waits forever | Ion dropped the `#`: quote it, `'home/#'`. |
| `ha-bridge: …; trying again in 30 s` in the log | The broker is not reachable yet; it keeps trying. |
| The device does not appear in Home Assistant | Check that Home Assistant's MQTT integration uses the same broker and discovery prefix `homeassistant`; `ha-bridge -n` shows what is sent. |
| All sensors unavailable | `redox-rpi/redox/availability` is `offline` (Pi off or the service not running), or no values for three minutes (`timerd -l` shows the `ha-bridge` timer). |
