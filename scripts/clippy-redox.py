#!/usr/bin/env python3
"""clippy for our Redox drivers, report-only (#272).

Usage:
    scripts/clippy-redox.py [--source <patched base>] [--work build/clippy-redox]
                            [--toolchain nightly-2026-05-24] [--json clippy-redox.json]
                            [--crate <name>]...

Runs `cargo clippy --target aarch64-unknown-redox -Zbuild-std` on the
crates we add to base as default patches (the drivers below), with
clippy::undocumented_unsafe_blocks on. It only type-checks: no linker, no
cookbook, no Redox sysroot. Counts each crate's own warnings, not those
of its dependencies (drivers/common is upstream's).

--source takes a base checkout with our patches applied (for example the
nightly's recipes/core/base/source); without it the script fetches base at
the commit in config/upstream.lock into --work and applies patches/base.

Writes {"<crate>": {"warnings": N, "undocumented_unsafe": M}} to --json
("upstream": true for an upstream crate we only patch)
(read by scripts/quality-kpis.py --clippy) and prints a Markdown table.
Exits 0 even when clippy finds something; 1 only when a crate cannot be
checked at all.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASE_URL = "https://gitlab.redox-os.org/redox-os/base.git"
# Our crates in base (default patches), by package name.
CRATES = ["bcm2835-pwmaudiod", "bcm2835-gpiod", "bcm2835-i2cd", "bcm2835-spid", "rpi-firmwared",
          "bcm2835-sdhcid", "bcm2835-sdhostd", "dwc2d"]
# Upstream crates we only patch: their counts are mostly upstream's code.
UPSTREAM = {"bcm2835-sdhcid"}
LINT = "clippy::undocumented_unsafe_blocks"


def run(*args, cwd=None, env=None):
    r = subprocess.run(args, cwd=cwd, env=env, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"{' '.join(args[:4])}: {r.stderr.strip()[-500:]}")
    return r.stdout


def prepare(work):
    """base at the pinned commit with patches/base applied, in work/base."""
    rev = next(line.split()[1] for line in open(os.path.join(ROOT, "config", "upstream.lock"))
               if line.split()[:1] == ["base"])
    src = os.path.join(work, "base")
    shutil.rmtree(src, ignore_errors=True)
    os.makedirs(src)
    run("git", "init", "-q", cwd=src)
    run("git", "fetch", "-q", "--depth", "1", BASE_URL, rev, cwd=src)
    run("git", "checkout", "-q", "FETCH_HEAD", cwd=src)
    patches = sorted(os.path.join(ROOT, "patches", "base", f)
                     for f in os.listdir(os.path.join(ROOT, "patches", "base")) if f.endswith(".patch"))
    run("git", "-c", "user.name=clippy-redox", "-c", "user.email=clippy-redox@localhost",
        "am", "-q", *patches, cwd=src)
    return src


def clippy(src, crate, toolchain, target_dir):
    env = dict(os.environ, CARGO_TARGET_DIR=target_dir)
    r = subprocess.run(["cargo", f"+{toolchain}", "clippy", "--target", "aarch64-unknown-redox",
                        "-Zbuild-std=std,panic_abort", "-p", crate, "--message-format", "json",
                        "--", "-W", LINT], cwd=src, env=env, capture_output=True, text=True)
    warnings = unsafe = 0
    own = False
    for line in r.stdout.splitlines():
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if msg.get("reason") != "compiler-message":
            continue
        # Only the crate itself: its package id names it, its dependencies don't.
        if f"#{crate}@" not in msg.get("package_id", "") and f"/{crate}#" not in msg.get("package_id", ""):
            continue
        own = True
        m = msg["message"]
        if m.get("level") != "warning" or not m.get("code"):
            continue
        warnings += 1
        if m["code"].get("code") == LINT:
            unsafe += 1
    if r.returncode != 0 and not own:
        return None, r.stderr.strip()[-300:]
    counts = {"warnings": warnings, "undocumented_unsafe": unsafe}
    if crate in UPSTREAM:
        counts["upstream"] = True
    return counts, None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--source", help="base checkout with our patches applied")
    ap.add_argument("--work", default=os.path.join(ROOT, "build", "clippy-redox"))
    ap.add_argument("--toolchain", default="nightly-2026-05-24")
    ap.add_argument("--json", default=os.path.join(ROOT, "build", "clippy-redox.json"))
    ap.add_argument("--crate", action="append", help="check only these crates (default: all of ours)")
    args = ap.parse_args()
    os.makedirs(args.work, exist_ok=True)
    src = args.source or prepare(args.work)
    result, failed = {}, []
    for crate in args.crate or CRATES:
        counts, err = clippy(src, crate, args.toolchain, os.path.join(args.work, "target"))
        if counts is None:
            failed.append(crate)
            print(f"{crate}: not checked: {err}", file=sys.stderr)
        else:
            result[crate] = counts
    with open(args.json, "w") as f:
        json.dump(result, f, indent=1)
    print("| Crate | clippy warnings | of them undocumented unsafe |\n|---|---|---|")
    for crate, c in sorted(result.items(), key=lambda x: -x[1]["warnings"]):
        note = " (upstream, patched)" if c.get("upstream") else ""
        print(f"| {crate}{note} | {c['warnings']} | {c['undocumented_unsafe']} |")
    print(f"| **total** | {sum(c['warnings'] for c in result.values())} | "
          f"{sum(c['undocumented_unsafe'] for c in result.values())} |")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
