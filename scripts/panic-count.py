#!/usr/bin/env python3
"""Panic KPI (#675, part of #673): panic paths in the kernel and kernel
panics seen on the Pis.

Usage: scripts/panic-count.py [--json] [--kernel DIR] [--logs DIR] [--days N] [--rev REV]

--kernel: the kernel source with our patches applied (the nightly's
  upstream/redox/recipes/core/kernel/source). Counts todo!, unimplemented!,
  expect("TODO, panic!, unreachable!, .unwrap() and .expect( in src/,
  aarch64 and generic code only (KERNEL_SKIP), comment lines left out.
  The "TODO" expects are counted apart, not in expect.
--rev: the redox-rpi revision whose patches/kernel and patches/base are
  read (default HEAD). Counts the same on the lines the patches add.
--logs: the build host's serial log directory (serial.log, serial-zero.log,
  serial-pi400.log and the daily rotations in serial/). Counts KERNEL PANIC
  lines per board and day, with the build that printed them.

scripts/quality-kpis.py --kernel/--logs uses measure_kernel(),
measure_patches() and measure_logs() for the KPI page.
"""

import argparse
import collections
import datetime
import gzip
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PATTERNS = {
    "todo": re.compile(r"\btodo!\s*\("),
    "unimplemented": re.compile(r"\bunimplemented!\s*\("),
    "expect_todo": re.compile(r"\.expect\(\s*\"TODO"),
    "panic": re.compile(r"\bpanic!\s*\("),
    "unreachable": re.compile(r"\bunreachable!\s*\("),
    "unwrap": re.compile(r"\.unwrap\(\)"),
    "expect": re.compile(r"\.expect\("),
}
# Other architectures and ACPI: a directory or file with one of these names.
KERNEL_SKIP = {"x86", "x86_64", "x86_shared", "riscv64", "acpi"}
PATCH_DIRS = ("kernel", "base")
# Ratchet (baseline 2026-11-24, kernel c951af42 with patches up to 0056):
# a value above its limit fails the KPI. Lower a limit when a fix lands.
LIMITS = {"todo": 2, "unimplemented": 1, "expect_todo": 4}
PATCH_LIMITS = {"kernel": {"panic": 1, "todo": 0, "unimplemented": 0, "expect_todo": 0}}
PANIC_LINE = "KERNEL PANIC"
BUILD_RE = re.compile(r"redox-rpi (v\S+)(?: \((v[^)]+)\))?")
ROTATED_RE = re.compile(r"^serial(?:-(zero|pi400))?-(\d{4}-\d{2}-\d{2})T\d{4}\.log\.gz$")
CURRENT = {"serial.log": "3b+", "serial-zero.log": "zero2w", "serial-pi400.log": "pi400"}


def count_lines(lines):
    c = collections.Counter()
    for line in lines:
        s = line.strip()
        if s.startswith(("//", "/*", "*")):
            continue
        for key, pat in PATTERNS.items():
            c[key] += len(pat.findall(line))
    c["expect"] -= c["expect_todo"]
    return c


def measure_kernel(src):
    """Panic paths in <src>/src, aarch64 and generic code only."""
    c = collections.Counter()
    for d, dirs, files in os.walk(os.path.join(src, "src")):
        dirs[:] = sorted(x for x in dirs if x not in KERNEL_SKIP)
        for f in files:
            if f.endswith(".rs") and f[:-3] not in KERNEL_SKIP:
                with open(os.path.join(d, f), errors="replace") as fh:
                    c += count_lines(fh)
    return {key: c[key] for key in PATTERNS}


def measure_patches(rev="HEAD", root=ROOT):
    """Panic paths on the lines our patches add, per patch directory."""
    out = {}
    for d in PATCH_DIRS:
        names = subprocess.run(["git", "-C", root, "ls-tree", "--name-only", f"{rev}:patches/{d}"],
                               capture_output=True, text=True).stdout.split()
        c = collections.Counter()
        for n in names:
            if n.endswith(".patch"):
                text = subprocess.run(["git", "-C", root, "show", f"{rev}:patches/{d}/{n}"],
                                      capture_output=True, text=True, errors="replace").stdout
                c += count_lines(l[1:] for l in text.splitlines() if l.startswith("+") and not l.startswith("+++"))
        out[d] = {key: c[key] for key in PATTERNS}
    return out


def log_files(logs, since):
    """(board, day, path) of the serial logs from `since` on. A rotated file
    is named after the day it was rotated; the current log is today."""
    rot = os.path.join(logs, "serial")
    for f in sorted(os.listdir(rot)) if os.path.isdir(rot) else []:
        m = ROTATED_RE.match(f)
        if m and m.group(2) >= since:
            yield {None: "3b+", "zero": "zero2w"}.get(m.group(1), m.group(1)), m.group(2), os.path.join(rot, f)
    today = datetime.date.today().isoformat()
    for f, board in CURRENT.items():
        if os.path.exists(os.path.join(logs, f)):
            yield board, today, os.path.join(logs, f)


def measure_logs(logs, days=30):
    """[{board, day, panics, builds, file}] for every log with a KERNEL PANIC
    line, and the number of logs read per board."""
    since = (datetime.date.today() - datetime.timedelta(days=days)).isoformat()
    found, read = [], collections.Counter()
    for board, day, path in log_files(logs, since):
        opener = gzip.open if path.endswith(".gz") else open
        build, panics, builds = "unknown", 0, []
        try:
            with opener(path, "rt", errors="replace") as fh:
                for line in fh:
                    m = BUILD_RE.search(line)
                    if m:
                        build = m.group(2) or m.group(1)
                    if PANIC_LINE in line:
                        panics += 1
                        builds.append(build)
        except (OSError, EOFError) as e:
            print(f"{path}: {e}", file=sys.stderr)
            continue
        read[board] += 1
        if panics:
            found.append({"board": board, "day": day, "panics": panics,
                          "builds": sorted(set(builds)), "file": os.path.basename(path)})
    return {"panics": found, "logs_read": dict(read), "since": since}


def over_limits(kernel, patches):
    """The counts above their ratchet limit, as text."""
    out = [f"{k} {kernel[k]} > {v}" for k, v in LIMITS.items() if kernel and kernel[k] > v]
    for d, lim in PATCH_LIMITS.items():
        out += [f"{d} patches {k} {patches[d][k]} > {v}" for k, v in lim.items() if d in patches and patches[d][k] > v]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--kernel")
    ap.add_argument("--logs")
    ap.add_argument("--days", type=int, default=30)
    ap.add_argument("--rev", default="HEAD")
    ap.add_argument("--json", action="store_true")
    a = ap.parse_args()
    r = {"patches": measure_patches(a.rev)}
    if a.kernel:
        r["kernel"] = measure_kernel(a.kernel)
    if a.logs:
        r["runtime"] = measure_logs(a.logs, a.days)
    r["over_limits"] = over_limits(r.get("kernel"), r["patches"])
    if a.json:
        print(json.dumps(r, indent=1))
        return
    for name, c in [("kernel", r.get("kernel"))] + [(f"{d} patches", v) for d, v in r["patches"].items()]:
        if c:
            print(f"{name}: " + ", ".join(f"{k} {v}" for k, v in c.items()))
    for p in r.get("runtime", {}).get("panics", []):
        print(f"{p['board']} {p['day']}: {p['panics']} KERNEL PANIC ({', '.join(p['builds'])}) in {p['file']}")
    print("over the limit: " + (", ".join(r["over_limits"]) or "none"))
    sys.exit(1 if r["over_limits"] else 0)


if __name__ == "__main__":
    main()
