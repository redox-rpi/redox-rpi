#!/usr/bin/env python3
"""Nightly TinyCC VC4 track, Pi part (#410, WBS 2.15 point 3).

  scripts/tcc-track-pi.py --g G.bin --t T.bin --out pi.json
                          [--rbc-load <rpi-open-firmware>/bootcode/rbc-load.sh]
                          [--pi 192.168.178.148] [--deadline 04:15]

Loads the GCC build (G) and the TCC build (T) of the bootcode monitor into
the Pi's SDRAM over the serial console and runs the same monitor commands on
both, the way of pi-tests rows 346-383, without a power cycle:

  1. devsh: `echo reset > /scheme/sys/kstop` (watchdog reset of the Redox
     image the nightly left running); the boot selector card's monitor then
     offers "press Enter".
  2. rbc-load.sh --enter G.bin c1000000 chainmon: G runs, its monitor opens.
  3. The commands below on G; `fl START.ELF` timed from sending to its "ok".
  4. rbc-load.sh T.bin c1000000 chainmon from G's monitor; the same commands.
  5. `reset` (watchdog) back to the normal boot, then "redox login:".

Never opens the serial port for reading (redox-serial.service is the only
reader; replies come from ~/redox-rpi/logs/serial.log) and never writes to
the card. Pass when every command gives the same reply on T as on G after
dropping the lines that depend on time or build (banner, BUILD_ID, timer
and microsecond values, mbox counters).

Writes --out: {"pi_pass": bool|null, "pi_back": bool, "fl_g_s", "fl_t_s", "size_g", "size_t",
"diff": [...], "step": last step reached, "note"}. pi_pass is null when the
run did not get to compare (no reset, no monitor, deadline). Exit 0 when
--out was written and the Pi is back on Redox, 1 otherwise (the Pi test
agent's runner power-cycles it then, under its own rules; this script never
touches the Shelly plug).
"""

import argparse
import datetime
import json
import os
import re
import subprocess
import sys
import time

TTY = "/dev/ttyUSB0"
LOG = os.path.expanduser("~/redox-rpi/logs/serial.log")
ADDR = "c1000000"
COMMANDS = ["sd", "fl START.ELF c2000000", "crc c2000000 1000", "ls",
            "fl bootcode.bin c2000000", "fl config.txt c2000000",
            "sdr 0 c2000000 8", "crc c2000000 1000"]
TIMED = "fl START.ELF c2000000"
# Lines that differ between builds or boots by design.
VOLATILE = re.compile(r"redox-rpi bootcode|BUILD_ID|timer|\b\d+ us\b|after \d+ us|mbox|udelay|cpuid", re.I)
ANSI = re.compile(r"\x1b\[[0-9;?]*[a-zA-Z]")


def log_size():
    return os.path.getsize(LOG)


def log_text(start):
    with open(LOG, "rb") as f:
        f.seek(start)
        return ANSI.sub("", f.read().decode("utf-8", "replace").replace("\r", ""))


def wait_log(start, pattern, timeout):
    rx = re.compile(pattern, re.M)
    end = time.time() + timeout
    while time.time() < end:
        if rx.search(log_text(start)):
            return True
        time.sleep(0.05)
    return False


def send(line):
    with open(TTY, "w") as t:
        t.write(line + "\r")
        t.flush()


def command(cmd, timeout=60):
    """Send one monitor command; return (reply lines, seconds) or (None, None)."""
    start = log_size()
    t0 = time.time()
    send(cmd)
    if not wait_log(start, r"^(ok|err)", timeout):
        return None, None
    secs = round(time.time() - t0, 2)
    text = log_text(start)
    m = re.search(r"^(ok|err).*$", text, re.M)
    lines = text[:m.end()].split("\n")
    # Drop the echoed command line itself.
    if lines and lines[0].strip().endswith(cmd):
        lines = lines[1:]
    return [l.rstrip() for l in lines if l.strip()], secs


def run_build(name, rbc_load, image, enter, before=None):
    """Load and chain <image>; with enter, rbc-load.sh first waits for the
    card monitor's prompt, and before() (the reset) runs once it listens."""
    args = [rbc_load] + (["--enter"] if enter else []) + [image, ADDR, "chainmon"]
    p = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if before:
        time.sleep(1)
        before()
    out, _ = p.communicate(timeout=180)
    if "chained monitor open" not in out:
        return None, f"{name}: rbc-load.sh: {out.strip().splitlines()[-1:]}"
    replies, fl = {}, None
    for i, cmd in enumerate(COMMANDS):
        lines, secs = command(cmd)
        if lines is None:
            return None, f"{name}: no reply to {cmd!r}"
        replies[f"{i} {cmd}"] = lines
        if cmd == TIMED and fl is None:
            fl = secs
    return {"replies": replies, "fl": fl}, None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--g", required=True)
    ap.add_argument("--t", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rbc-load", default=os.path.expanduser("~/tcc-track/rpi-open-firmware/bootcode/rbc-load.sh"))
    ap.add_argument("--pi", default="192.168.178.148")
    ap.add_argument("--deadline", default="04:15", help="UTC time by which the Pi must be back (Pi test agent's window)")
    args = ap.parse_args()
    here = os.path.dirname(os.path.abspath(__file__))
    result = {"pi_pass": None, "pi_back": None, "fl_g_s": None, "fl_t_s": None,
              "size_g": os.path.getsize(args.g), "size_t": os.path.getsize(args.t),
              "diff": [], "step": "start", "note": ""}

    def done(rc):
        with open(args.out, "w") as f:
            json.dump(result, f, indent=1)
        print(json.dumps({k: v for k, v in result.items() if k != "diff"}))
        return rc

    now = datetime.datetime.now(datetime.timezone.utc)
    h, m = map(int, args.deadline.split(":"))
    deadline = now.replace(hour=h, minute=m, second=0, microsecond=0)
    if deadline < now and (now - deadline).total_seconds() < 6 * 3600:
        result["note"] = f"after the deadline {args.deadline} UTC, no Pi run"
        return done(0)
    if deadline > now and (deadline - now).total_seconds() < 600:
        result["note"] = f"less than 10 minutes to {args.deadline} UTC, no Pi run"
        return done(0)

    def devsh(cmd, timeout):
        try:
            return subprocess.run([sys.executable, os.path.join(here, "devsh.py"), args.pi, "2323", cmd],
                                  capture_output=True, text=True, timeout=timeout).stdout
        except subprocess.TimeoutExpired:
            return ""

    def reset():
        # 1. Watchdog reset of the running Redox image; the connection just
        # drops, so do not wait for it.
        subprocess.Popen([sys.executable, os.path.join(here, "devsh.py"), args.pi, "2323",
                          "echo reset > /scheme/sys/kstop"], stdout=subprocess.DEVNULL,
                         stderr=subprocess.DEVNULL)

    start = log_size()
    result["step"] = "reset"
    g = t = None
    try:
        # 2-3. G from the card's monitor: rbc-load.sh --enter listens first.
        g, err = run_build("G", args.rbc_load, args.g, enter=True, before=reset)
        if g is None:
            result["note"] = err
        else:
            result["step"], result["fl_g_s"] = "G", g["fl"]
            # 4. T chained from G's monitor.
            t, err = run_build("T", args.rbc_load, args.t, enter=False)
            if t is None:
                result["note"] = err
                result["pi_pass"] = False
            else:
                result["step"], result["fl_t_s"] = "T", t["fl"]
                for key, gl in g["replies"].items():
                    a = [l for l in gl if not VOLATILE.search(l)]
                    b = [l for l in t["replies"][key] if not VOLATILE.search(l)]
                    if a != b:
                        result["diff"].append({"command": key.split(" ", 1)[1], "g": a, "t": b})
                result["pi_pass"] = not result["diff"]
    except subprocess.TimeoutExpired as e:
        result["note"] = f"timeout: {e.cmd}"
    # 5. Back to the normal boot. Without a monitor prompt the reset did not
    # happen and Redox still runs: then only check that.
    if "monitor: press Enter" not in log_text(start):
        ok = "net-ok" in devsh("echo net-ok", 30)
    else:
        back = log_size()
        send("reset")
        ok = wait_log(back, r"redox login:", 240)
    result["pi_back"] = ok
    result["step"] += ", back on Redox" if ok else ", NOT back on Redox"
    with open(os.path.splitext(args.out)[0] + "-serial.log", "w") as f:
        f.write(log_text(start))
    return done(0 if ok else 1)


if __name__ == "__main__":
    sys.exit(main())
