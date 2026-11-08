# GPIO, I2C, SPI (`GPIO`)

Topic: [gpio](../topics/gpio.md). Trace: [trace.md](trace.md).

All three issues are still open with milestone v0.8.0, although the drivers
start in every image since base patches `0045`–`0048`.

### REQ-GPIO-1 GPIO pins

The system shall offer the 54 GPIO pins as `/scheme/gpio`, with mode,
value and pull per pin.

| Field | Value |
|---|---|
| Rationale | GPIO is why many people use a Pi (sensors, LEDs, buttons). |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.3.9 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #38, base `bcm2835-gpiod` |
| Verification | test, automated: checklist v0.4.0 test 14, row 285 (without wiring); **gap**: no test with wiring |

Acceptance criteria:

1. `ls /scheme/gpio` lists the pins and `pins`.
2. A pin set to output and high reads back high; with a wire to another pin
   set to input, the input follows (with wiring, **gap**).
3. `/scheme/rpi-firmware/power_led` switches the 3B+ power LED.

### REQ-GPIO-2 I2C bus 1

The system shall offer I2C bus 1 (pins 3 and 5) as `/scheme/i2c` with the
`i2c` tool.

| Field | Value |
|---|---|
| Rationale | Most Pi sensors and displays use I2C. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.3.9 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #39 |
| Verification | test, automated: checklist v0.4.0 test 14, row 285 (without wiring); **gap**: no test with a device |

Acceptance criteria:

1. `i2c detect` runs and lists no device on an empty bus, without error.
2. With a known device on the bus, `i2c detect` finds its address and
   `i2c get` reads a register (**gap**).

### REQ-GPIO-3 SPI0

The system shall offer SPI0 (pins 19, 21, 23, 24, 26) as `/scheme/spi`
with the `spi` tool.

| Field | Value |
|---|---|
| Rationale | Displays, ADCs and radio modules use SPI. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.3.9 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #42 |
| Verification | test, automated: checklist v0.4.0 test 14, row 285 (without wiring); **gap**: no loopback test |

Acceptance criteria:

1. The `spi` check of `nowire-check.py` passes.
2. With MOSI wired to MISO, bytes sent come back unchanged (**gap**).
