#!/usr/bin/env python3
"""vc4d as the display of the console and Orbital (#188 stage a), checked
without a person, in QEMU or on a Pi 3B+.

Usage, in a redox-rpi clone, after a build with the experiments vc4d,
pointer-bench and desktop (orbital, orbital-apps, cosmic-apps-source),
without rpi-fb:

    patches/experiments/vc4d/desktop-check.py [--image <harddrive.img>]
        [--out <dir>] [--size 1280x720]          QEMU (boots the image)
    patches/experiments/vc4d/desktop-check.py --host <pi> [--port 2323]
        [--out <dir>]                            a Pi that booted it

QEMU's raspi3b has no HVS: vc4d gives up at boot and vesad keeps the
screen. In QEMU the check starts a second vc4d by hand with VC4D_FAKE (an
HVS in memory) and VC4D_LOG=1. On the Pi, vc4d took over at boot.

  vc4d       QEMU: the console moves to display.vc4 (vc4d's first list has
             a primary plane); Pi: vc4d runs and its check passed (serial)
  console    Enter at the login (pointer-bench), then vc4shot: mostly
             background with text
  desktop    `desktop --yes`: Orbital runs on vc4d, vc4shot shows its login
             window (many colours)
  cursor     pointer-bench moves the mouse: the cursor plane is at two or
             more positions (QEMU: vc4d's list log; Pi: the HVS list that
             vc4shot reads back)
  serial     QEMU: no panic in the console log (on the Pi: the row checks
             the serial log)

Screenshots go to --out as PPM files. Exits 1 if a check failed. Orbital
keeps running afterwards: reboot before other checks.
"""

import argparse
import importlib.util
import os
import re
import socket
import subprocess
import sys
import threading
import time

ROOT = os.getcwd()
spec = importlib.util.spec_from_file_location("pi_test", os.path.join(ROOT, "scripts", "pi-test.py"))
pi_test = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pi_test)

LIST_LINE = re.compile(r"vc4d: list \d+ at")
CURSOR_QEMU = re.compile(r"x: (\d+), y: (\d+), width: 64, height: 64, alpha: true")
results = []


def check(name, ok, detail):
    word = "PASS" if ok else "FAIL"
    results.append((name, word, detail))
    print(f"{word} {name}: {detail}", flush=True)


def fetch(sh, path, dest, host_ip):
    """Copies a file from the guest: it connects to us with nc. Returns the
    bytes; nothing reliable follows nc in the same devsh session, so the
    caller starts a new one."""
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("0.0.0.0", 0))
    srv.listen(1)
    srv.settimeout(120)
    port = srv.getsockname()[1]
    data = bytearray()

    def serve():
        try:
            conn, _ = srv.accept()
            with conn:
                conn.settimeout(30)
                while chunk := conn.recv(1 << 16):
                    data.extend(chunk)
        except OSError:
            pass
        finally:
            srv.close()

    t = threading.Thread(target=serve, daemon=True)
    t.start()
    try:
        sh.run(f"nc {host_ip}:{port} < {path}", 180)
    except (RuntimeError, OSError):
        pass
    t.join(150)
    # Redox's nc may write "Remote host: ..." first; the picture starts at P6.
    start = data.find(b"P6")
    data = bytes(data[start:]) if start >= 0 else b""
    with open(dest, "wb") as f:
        f.write(data)
    sh.close()
    sh.login()
    return data


def ppm_stats(data):
    """Width, height, share of the most common colour, number of colours
    (every 7th pixel)."""
    m = re.match(rb"P6\s+(\d+)\s+(\d+)\s+(\d+)\s", data)
    if not m:
        return None
    w, h = int(m.group(1)), int(m.group(2))
    px = data[m.end():m.end() + w * h * 3]
    if len(px) < w * h * 3:
        return None
    counts = {}
    for i in range(0, len(px), 3 * 7):
        c = px[i:i + 3]
        counts[c] = counts.get(c, 0) + 1
    return w, h, max(counts.values()) / sum(counts.values()), len(counts)


def shot(sh, name, args, host_ip):
    out = sh.run(f"vc4shot /tmp/regr-{name}.ppm", 60)
    data = fetch(sh, f"/tmp/regr-{name}.ppm", os.path.join(args.out, f"{name}.ppm"), host_ip)
    return out, data, ppm_stats(data)


def stats_text(data, st):
    return f"{len(data)} bytes" + (f", {st[0]}x{st[1]}, top colour {st[2]:.0%}, {st[3]} colours" if st else "")


def hw_cursor(out):
    """The cursor plane's position from vc4shot's read-back of the HVS list:
    the 64x64 plane with alpha."""
    for line in out:
        if "vc4shot: plane" in line and "64" in line and "alpha: true" in line:
            m = re.search(r"x: (\d+), y: (\d+)", line)
            if m:
                return m.groups()
    return None


def run_checks(sh, args, qemu, console, host_ip):
    if qemu:
        script = [f"export VC4D_FAKE={args.size}", "export VC4D_LOG=1", "export INIT_NOTIFY=2",
                  "vc4d > /tmp/regr-vc4d.log ^> /tmp/regr-vc4d.err &!"]
        sh.run("rm -f /tmp/regr-vc4d.ion")
        for line in script:
            sh.run(f"echo '{line}' >> /tmp/regr-vc4d.ion")
        sh.run("ion /tmp/regr-vc4d.ion; sleep 8", 60)
        first = [l for l in sh.run("cat /tmp/regr-vc4d.log", 60) if LIST_LINE.match(l)]
        check("vc4d", bool(first) and "alpha: false" in first[0],
              f"{len(first)} display lists; " + (first[0][:150] if first else "none"))
    else:
        ps = sh.run("ps", 60)
        check("vc4d", any(l.endswith("/usr/bin/vc4d") for l in ps),
              "vc4d " + ("runs" if any(l.endswith("/usr/bin/vc4d") for l in ps) else "not in ps")
              + " (its check: in the serial log)")
    size = args.size.split("x")

    # Enter at the login prompt: new text after the handover.
    sh.run("pointer-bench --keys enter; sleep 1; pointer-bench --keys enter; sleep 2", 60)
    out, data, st = shot(sh, "console", args, host_ip)
    if not qemu:
        size = [str(st[0]), str(st[1])] if st else size
    check("console", bool(st) and st[2] > 0.80 and st[3] >= 2, stats_text(data, st))

    out = sh.run("desktop --yes > /tmp/regr-desktop.out; sleep 25; ps", 120)
    orbital = any(l.endswith("/usr/bin/orbital") or "orbital orblogin" in l for l in out)
    out, data, st = shot(sh, "desktop", args, host_ip)
    check("desktop", orbital and bool(st) and st[3] >= 20,
          f"Orbital {'runs' if orbital else 'not in ps'}; {stats_text(data, st)}")

    positions = set()
    n_before = len([l for l in sh.run("cat /tmp/regr-vc4d.log", 60) if LIST_LINE.match(l)]) if qemu else 0
    for center in ((300, 200), (700, 450)):
        sh.run(f"pointer-bench 2 --screen {size[0]} {size[1]} --center {center[0]} {center[1]} --radius 40", 60)
        if not qemu:
            out = sh.run("vc4shot /tmp/regr-cursor.ppm", 60)
            pos = hw_cursor(out)
            if pos:
                positions.add(pos)
    if qemu:
        lists = [l for l in sh.run("cat /tmp/regr-vc4d.log", 60) if LIST_LINE.match(l)]
        positions = {m.groups() for l in lists for m in [CURSOR_QEMU.search(l)] if m}
        detail = f"lists {n_before} -> {len(lists)}; cursor plane at {len(positions)} positions"
    else:
        detail = f"cursor plane in the HVS list at {sorted(positions)}"
    check("cursor", len(positions) >= 2, detail)

    if console:
        text = open(console, "rb").read().decode("utf-8", "replace")
        bad = [l.strip()[:120] for l in text.splitlines()
               if re.search(r"kernel panic|panicked at|RELIBC PANIC|UNHANDLED EXCEPTION", l)]
        check("serial", not bad, f"{len(bad)} bad lines" + (": " + "; ".join(bad[:3]) if bad else ""))
    if qemu:
        with open(os.path.join(args.out, "vc4d.log"), "w") as f:
            f.write("\n".join(sh.run("cat /tmp/regr-vc4d.log", 60)))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--host")
    ap.add_argument("--port", type=int, default=2323)
    ap.add_argument("--image", default="upstream/redox/build/aarch64/minimal/harddrive.img")
    ap.add_argument("--out", default="build/vc4d-desktop/" + time.strftime("%Y%m%dT%H%M%S", time.gmtime()))
    ap.add_argument("--size", default="1280x720", help="QEMU: VC4D_FAKE size")
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)

    if args.host:
        sh = pi_test.Shell(args.host, args.port)
        try:
            run_checks(sh, args, False, None, pi_test.local_ip(sh))
        finally:
            sh.close()
        return finish(args)

    port = int(os.environ.get("DEVSH_PORT", "12731"))
    copy = os.path.join(args.out, "harddrive.img")
    subprocess.run(["cp", args.image, copy], check=True)
    console = os.path.join(args.out, "console.log")
    env = dict(os.environ, DEVSH_PORT=str(port), SSH_PORT=str(port + 1))
    qemu = subprocess.Popen(
        ["timeout", "1500", "scripts/qemu-raspi3b.sh", "--usb-net", "--data", "--image", copy],
        stdout=open(console, "w"), stderr=subprocess.STDOUT, env=env)
    try:
        t0 = time.time()
        while b"redox login:" not in open(console, "rb").read().replace(b"\r", b""):
            if time.time() - t0 > 600 or qemu.poll() is not None:
                check("boot", False, "no login prompt")
                return finish(args)
            time.sleep(2)
        print(f"login prompt after {time.time() - t0:.0f} s", flush=True)
        sh = None
        for _ in range(60):
            try:
                sh = pi_test.Shell("127.0.0.1", port)
                break
            except OSError:
                time.sleep(2)
        try:
            run_checks(sh, args, True, console, "10.0.2.2")
        finally:
            sh.close()
    finally:
        subprocess.run(["pkill", "-f", f"[h]ostfwd=tcp::{port}-"])
        qemu.wait()
        os.remove(copy)
    return finish(args)


def finish(args):
    fails = sum(r[1] == "FAIL" for r in results)
    print(f"{len(results) - fails} pass, {fails} fail; output in {args.out}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
