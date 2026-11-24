#!/usr/bin/env python3
"""Network adapter and DHCP time per boot, from a serial log (REQ-NET-2, #492).

REQ-NET-2 criterion 1: on a Zero 2 W with an RTL8153 hub, the adapter has an
address from DHCP within 60 s after power-on. The Zero has no network boot
and its release card has no devsh, so this is checked from the serial
console that `~/bin/zero-serial` captures, with a time in front of every
line (`HH:MM:SS.mmm text`). The same works for a Pi 3B+ log with times.

For every boot in the log it prints the build, the adapter dwc2d chose
(`RTL815x`, `LAN7800` or `CDC Ethernet`), the RTL chip version and link
line, and the seconds from the boot's first line to `DHCP: <iface> got`.
Addresses are not printed (reports go into public repositories).

A boot starts at the first line the Pi prints after power-on: the boot stub
of a card image (`rpi-boot-stub:`), U-Boot (`U-Boot 20..`) or our serial
boot monitor (`boot64:`). Broadcom's firmware before that prints nothing,
so the time from power-on is a few seconds longer than the number shown;
the check allows for that with --firmware (default 5 s).

Without times in the log the boots are still listed, without seconds.

Exit status: 0 when the last boot (or every boot with --all) used an
adapter matching --adapter (default RTL815x) and got its address within
--limit seconds (default 60) including --firmware; 1 otherwise.

Usage: adapter-dhcp-log.py <serial log> [--adapter RTL815x|LAN7800|any]
                           [--limit 60] [--firmware 5] [--all]
"""

import argparse
import re
import sys

TIME = re.compile(r"^(\d\d):(\d\d):(\d\d(?:\.\d+)?) ")
ANSI = re.compile(r"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07|\x1b[78]")
BOOT_START = re.compile(r"rpi-boot-stub: |U-Boot 20\d\d|boot64: ")
BANNER = re.compile(r"redox-rpi (v\S+)")
ADAPTER = re.compile(r"dwc2d: (RTL815x|LAN7800|CDC Ethernet) at address (\d+)$")
CHIP = re.compile(r"dwc2d: (RTL815\w+) chip version ([0-9a-f]{4})")
LINK = re.compile(r"dwc2d: (RTL815\w+|LAN7800) (link up after \d+ ms, .*|no link yet)")
SKIPPED = re.compile(r"dwc2d: RTL815x .*(not high speed, skipped|not supported)")
DHCP = re.compile(r"DHCP: (\S+) got \S+/(\d+)")
ONCE = re.compile(r"rpi-boot-stub: devicetree at|U-Boot 20")
OS = re.compile(r"dwc2d: |redox-rpi v|login:|^kernel")
NO_OFFER = re.compile(r"dhcpd: (\S+): attempt \d+: no offer")


class Boot:
    def __init__(self, line_no, at):
        self.line_no, self.start = line_no, at
        self.build = self.adapter = self.chip = self.link = self.skipped = None
        self.dhcp_iface = self.dhcp_at = None
        self.no_offers = 0
        self.os = False
        self.once = set()

    def seconds(self, at):
        if at is None or self.start is None:
            return None
        return at - self.start

    def dhcp_s(self):
        return self.seconds(self.dhcp_at)


def parse(lines):
    boots = []
    cur = None
    last_at = None
    day = 0.0
    for no, raw in enumerate(lines, 1):
        line = ANSI.sub("", raw.rstrip("\r\n")).replace("\r", "")
        at = None
        m = TIME.match(line)
        if m:
            at = int(m[1]) * 3600 + int(m[2]) * 60 + float(m[3]) + day
            if last_at is not None and at < last_at - 3600:
                day += 86400
                at += 86400
            last_at = at
            line = line[m.end():]
        # a start line begins a new boot once the current one reached the
        # system, or when a line that comes once per boot (stub, U-Boot)
        # comes again; the start lines of one boot (boot monitor, then
        # stub) stay one boot
        once = ONCE.search(line)
        if BOOT_START.search(line) and (cur is None or cur.os
                                        or (once and once[0] in cur.once)):
            cur = Boot(no, at)
            boots.append(cur)
        if once:
            cur.once.add(once[0])
        if cur is not None and OS.search(line):
            cur.os = True
        if cur is None:
            continue
        if (m := BANNER.search(line)) and not cur.build:
            cur.build = m[1]
        elif m := ADAPTER.search(line):
            cur.adapter = m[1]
        elif m := CHIP.search(line):
            cur.chip = f"{m[1]} chip version {m[2]}"
        elif m := LINK.search(line):
            cur.link = f"{m[1]} {m[2]}"
        elif m := SKIPPED.search(line):
            cur.skipped = line.strip()
        elif (m := DHCP.search(line)) and cur.dhcp_at is None and cur.dhcp_iface is None:
            cur.dhcp_iface = f"{m[1]} (/{m[2]})"
            cur.dhcp_at = at
        elif NO_OFFER.search(line) and cur.dhcp_iface is None:
            cur.no_offers += 1
    return boots


def judge(boot, want, limit, firmware):
    if want != "any" and boot.adapter != want:
        return False, f"adapter {boot.adapter or 'none'}, wanted {want}"
    if boot.dhcp_iface is None:
        return False, f"no DHCP address ({boot.no_offers} attempts without an offer)"
    s = boot.dhcp_s()
    if s is None:
        return False, "DHCP address, but the log has no times"
    if s + firmware > limit:
        return False, f"DHCP after {s:.1f} s + {firmware:g} s firmware > {limit:g} s"
    return True, f"DHCP after {s:.1f} s + {firmware:g} s firmware <= {limit:g} s"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("log")
    ap.add_argument("--adapter", default="RTL815x", choices=["RTL815x", "LAN7800", "CDC Ethernet", "any"])
    ap.add_argument("--limit", type=float, default=60)
    ap.add_argument("--firmware", type=float, default=5)
    ap.add_argument("--all", action="store_true")
    args = ap.parse_args()

    with open(args.log, encoding="utf-8", errors="replace") as f:
        boots = parse(f)
    if not boots:
        print("no boot found in the log")
        sys.exit(1)
    results = []
    for i, b in enumerate(boots, 1):
        ok, why = judge(b, args.adapter, args.limit, args.firmware)
        results.append(ok)
        print(f"boot {i} (line {b.line_no}): {b.build or 'no banner'}")
        print(f"  adapter: {b.adapter or 'none'}"
              + (f", {b.chip}" if b.chip else "") + (f", {b.link}" if b.link else ""))
        if b.skipped:
            print(f"  {b.skipped}")
        print(f"  {'pass' if ok else 'fail'}: {why}")
    ok = all(results) if args.all else results[-1]
    print(f"result: {'pass' if ok else 'fail'} ({'every boot' if args.all else 'last boot'}"
          f" of {len(boots)}, adapter {args.adapter}, {args.limit:g} s)")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
