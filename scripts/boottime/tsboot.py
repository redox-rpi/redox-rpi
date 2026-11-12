#!/usr/bin/env python3
"""Boot build/fw-direct in QEMU raspi3b and timestamp every serial line.

Usage: tsboot.py <fw-direct dir> <out.log> [expect-regex] [timeout] [append]
Each output line: "<seconds since QEMU start> <line>".
"""
import os, re, subprocess, sys, time

d, out = sys.argv[1], sys.argv[2]
expect = re.compile(sys.argv[3] if len(sys.argv) > 3 else r"redox login:")
timeout = float(sys.argv[4]) if len(sys.argv) > 4 else 120
append = sys.argv[5] if len(sys.argv) > 5 else ""
cmd = ["qemu-system-aarch64", "-M", "raspi3b", "-smp", "4", "-m", "1024",
       "-kernel", f"{d}/kernel8.img", "-initrd", f"{d}/initramfs",
       "-dtb", f"{d}/qemu.dtb", "-display", "none", "-usb", "-device", "usb-kbd",
       "-serial", "stdio", "-serial", "null", "-monitor", "none"]
if append:
    cmd += ["-append", append]
t0 = time.monotonic()
p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stdin=subprocess.DEVNULL)
os.set_blocking(p.stdout.fileno(), False)
buf, ok = b"", False
with open(out, "w") as f:
    while time.monotonic() - t0 < timeout and p.poll() is None:
        try:
            chunk = p.stdout.read()
        except BlockingIOError:
            chunk = None
        if not chunk:
            time.sleep(0.005)
            # a prompt has no newline: check the partial line too
            if buf and expect.search(buf.decode("utf-8", "replace")):
                f.write(f"{time.monotonic() - t0:8.3f} {buf.decode('utf-8', 'replace')}\n")
                ok = True
                break
            continue
        now = time.monotonic() - t0
        buf += chunk.replace(b"\0", b"")
        while b"\n" in buf:
            line, buf = buf.split(b"\n", 1)
            s = line.decode("utf-8", "replace").rstrip("\r")
            f.write(f"{now:8.3f} {s}\n")
            if expect.search(s):
                ok = True
        if ok:
            break
p.kill()
print(("PASS" if ok else "FAIL") + f" {time.monotonic() - t0:.2f} s {out}")
sys.exit(0 if ok else 1)
