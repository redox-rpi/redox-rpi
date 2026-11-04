#!/usr/bin/env python3
"""Pi test for the GPIO, I2C and SPI drivers (#38, #39, #42) that needs no
person and no wiring. Uses the Shell class of scripts/pi-test.py.

- GPIO: GPIO 5 (pin 29) as output reads back 1 and 0; GPIO 6 (pin 31) as
  input follows its pull (up 1, down 0); the activity LED (GPIO 29) and
  `power_led` take a write without an error. None of these pins is part of
  the wiring of tests #38-#40, so the check is safe with or without it.
- I2C: bus 1 is in alt0, `i2c detect` finishes (lists 68 if a DS3231 is
  wired), a missing address gives an error and no hang.
- SPI: pins 7-11 in alt0. If MOSI (pin 19) and MISO (pin 21) are jumpered,
  loopback at several speeds. Without the jumper, the pull on MISO (GPIO 9)
  decides what comes back: ff with pull up, 00 with pull down, at two speeds
  and with both chip selects.

Usage: nowire-check.py <repo> <host>"""

import importlib.util
import sys

spec = importlib.util.spec_from_file_location("pitest", sys.argv[1] + "/scripts/pi-test.py")
pitest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pitest)

sh = pitest.Shell(sys.argv[2], 2323)
G = "/scheme/gpio"
results = []


def check(name, ok, detail=""):
    results.append(ok)
    print(f"{'PASS' if ok else 'FAIL'}: {name}{': ' + detail if detail else ''}", flush=True)


def run(cmd, timeout=60):
    return [line.strip() for line in sh.run(cmd, timeout=timeout)]


def status(cmd, timeout=60):
    out = run(f"{cmd} && echo RC-OK || echo RC-FAIL", timeout)
    return "RC-OK" in out, [l for l in out if not l.startswith("RC-")]


def read(path):
    out = run(f"cat {path}")
    return out[-1] if out else ""


try:
    # GPIO
    run(f"echo out > {G}/5/mode")
    for level in ["1", "0"]:
        run(f"echo {level} > {G}/5/value")
        got = read(f"{G}/5/value")
        check(f"GPIO 5 output {level} reads back", got == level, got)
    run(f"echo in > {G}/5/mode")
    run(f"echo in > {G}/6/mode")
    for pull, level in [("up", "1"), ("down", "0"), ("up", "1")]:
        run(f"echo {pull} > {G}/6/pull")
        got = read(f"{G}/6/value")
        check(f"GPIO 6 input with pull {pull} reads {level}", got == level, got)
    run(f"echo none > {G}/6/pull")
    ok, out = status(f"echo out > {G}/29/mode; echo 1 > {G}/29/value; echo 0 > {G}/29/value")
    check("activity LED (GPIO 29) takes writes", ok, " ".join(out))
    ok, out = status("echo off > /scheme/rpi-firmware/power_led; echo on > /scheme/rpi-firmware/power_led")
    check("power_led takes off and on", ok, " ".join(out))
    pins = run(f"cat {G}/pins")
    check("pins table has 54 rows", sum(1 for l in pins if l[:1].isdigit()) >= 54, f"{len(pins)} lines")

    # I2C
    modes = run(f"cat {G}/2/mode {G}/3/mode")
    check("I2C pins 2 and 3 in alt0", modes[-2:] == ["alt0", "alt0"], " ".join(modes))
    ok, out = status("i2c detect", timeout=120)
    found = sorted({w for l in out[1:] for w in l.split()[1:] if w != "--"})
    check("i2c detect finishes", ok, f"answering: {' '.join(found) or 'none'}")
    ok, out = status("i2c get 50 0")
    check("missing address 50 gives an error", not ok, " ".join(out))

    # SPI
    pins = run(f"cat {G}/7/mode {G}/8/mode {G}/9/mode {G}/10/mode {G}/11/mode")
    check("SPI pins 7-11 in alt0", pins[-5:] == ["alt0"] * 5, " ".join(pins))
    run("spi -s 1000000 -m 0")
    answer = run("spi 12 34 56")
    jumper = bool(answer) and answer[-1] == "12 34 56"
    print(f"SPI: jumper pin 19 to 21 {'found' if jumper else 'not found'} ({answer[-1:] or 'no answer'})", flush=True)
    if jumper:
        for speed in ["1000000", "10000000", "100000"]:
            ok, out = status(f"spi -s {speed} -l 4096", timeout=120)
            check(f"loopback 4096 bytes at {speed} Hz", ok, " ".join(out))
        ok, out = status("spi -c 1 -s 1000000 -l 64")
        check("loopback on chip select 1", ok, " ".join(out))
    else:
        for pull, byte in [("up", "ff"), ("down", "00")]:
            run(f"echo {pull} > {G}/9/pull")
            for speed in ["1000000", "10000000"]:
                run(f"spi -s {speed}")
                for chip in ["0", "1"]:
                    got = run(f"spi -c {chip} 12 34 56 78")
                    got = got[-1] if got else ""
                    check(f"MISO pull {pull}, {speed} Hz, cs{chip}: reads {byte}", got == " ".join([byte] * 4), got)
        run(f"echo none > {G}/9/pull")
        ok, out = status("spi -s 1000000 -l 4096", timeout=120)
        check("4096-byte transfer finishes (wrong bytes expected without jumper)", any("loopback:" in l for l in out), " ".join(out))
    run("spi -s 1000000 -m 0")
    print(f"{sum(results)} of {len(results)} checks passed", flush=True)
finally:
    sh.close()
