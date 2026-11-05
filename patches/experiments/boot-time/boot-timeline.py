#!/usr/bin/env python3
"""Time-stamp a growing console log (#47): follow the file from its current
end (or --from <offset>) and print each new line with the seconds since
the start, until a pattern appears or the timeout ends. Reads only the file,
so it can follow the serial logger's file without opening the device.

Usage: boot-timeline.py <log> [--until <regex>] [--timeout <s>] [--from <offset>]
"""
import argparse, os, re, sys, time

ap = argparse.ArgumentParser()
ap.add_argument("log")
ap.add_argument("--until", default=r"redox login:")
ap.add_argument("--timeout", type=float, default=300)
ap.add_argument("--from", dest="start", type=int)
a = ap.parse_args()

until = re.compile(a.until)
t0 = time.monotonic()
while not os.path.exists(a.log):
    time.sleep(0.05)
f = open(a.log, "rb")
f.seek(a.start if a.start is not None else 0, 0 if a.start is not None else 2)
buf = b""
while time.monotonic() - t0 < a.timeout:
    chunk = f.read()
    if not chunk:
        time.sleep(0.05)
        continue
    buf += chunk
    # \r as well as \n ends a line: U-Boot and the bootloader redraw lines
    parts = re.split(rb"[\r\n]", buf)
    buf = parts.pop()
    t = time.monotonic() - t0
    for p in parts:
        line = re.sub(r"\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07", "",
                      p.decode("utf-8", "replace")).strip()
        if line:
            print(f"{t:8.2f}  {line}", flush=True)
            if until.search(line):
                sys.exit(0)
sys.exit(1)
