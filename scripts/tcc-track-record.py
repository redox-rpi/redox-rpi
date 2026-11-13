#!/usr/bin/env python3
"""Record of the nightly TinyCC VC4 track (#410, format in #411).

  scripts/tcc-track-record.py --host <dir>/host.json [--pi <dir>/pi.json]
                              --build <id> --out <file>

Appends one JSON line to --out (scripts/update-stable.sh appends it to
tcc.jsonl in redox-rpi/pi-tests, as network.jsonl):

  {"date", "build", "source": "nightly", "kind": "tcc-night", "tinycc",
   "fat_boot", "gcc", "resim_pass", "pi_pass", "size_g", "size_t",
   "fl_g_s", "fl_t_s"}

--host is scripts/tcc-track.sh's host.json. --pi is the Pi part's result
(scripts/tcc-track-pi.sh, the Pi test agent): {"pi_pass": bool,
"fl_g_s": float, "fl_t_s": float, ...}; without it, or when it is missing
or unreadable, pi_pass, fl_g_s and fl_t_s are null (no Pi run that night).
"""

import argparse
import datetime
import json
import sys


def load(path):
    if not path:
        return None
    try:
        with open(path) as f:
            return json.load(f)
    except (OSError, ValueError) as e:
        print(f"{path}: {e}", file=sys.stderr)
        return None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--host", required=True)
    ap.add_argument("--pi")
    ap.add_argument("--build", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    host = load(args.host)
    if host is None:
        return 1
    pi = load(args.pi) or {}
    rec = {"date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d"),
           "build": args.build, "source": "nightly", "kind": "tcc-night",
           "tinycc": host.get("tinycc"), "fat_boot": host.get("fat_boot"), "gcc": host.get("gcc"),
           "resim_pass": bool(host.get("resim_pass")),
           "pi_pass": pi.get("pi_pass") if isinstance(pi.get("pi_pass"), bool) else None,
           "size_g": host.get("size_g"), "size_t": host.get("size_t"),
           "fl_g_s": pi.get("fl_g_s"), "fl_t_s": pi.get("fl_t_s")}
    line = json.dumps(rec)
    with open(args.out, "a") as f:
        f.write(line + "\n")
    print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
