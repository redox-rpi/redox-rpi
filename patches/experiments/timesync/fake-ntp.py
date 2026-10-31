#!/usr/bin/env python3
"""Fake NTP servers for testing timesync's answer checks (#51 follow-up).

Usage: fake-ntp.py [address] [seconds]

Listens on UDP ports 12301 to 12309 of <address> (default 127.0.0.1), one
kind of answer per port, for <seconds> (default 3600):

  12301 good           12302 leap indicator 3   12303 kiss-of-death RATE
  12304 wrong origin   12305 two hours ahead    12306 stratum 16
  12307 good (second)  12308 transmit time 0    12309 kiss-of-death DENY

In QEMU the guest reaches them as 10.0.2.2:<port>, for example
`timesync -n 10.0.2.2:12301 10.0.2.2:12305 10.0.2.2:12307`.
"""

import select
import socket
import struct
import sys
import time

NTP_TO_UNIX = 2_208_988_800
KINDS = {
    12301: "good", 12302: "leap3", 12303: "rate", 12304: "origin", 12305: "ahead",
    12306: "stratum16", 12307: "good", 12308: "zero", 12309: "deny",
}


def ntp(t):
    return struct.pack(">II", int(t) + NTP_TO_UNIX, int((t % 1) * 2**32))


def answer(kind, request):
    now = time.time() + (7200 if kind == "ahead" else 0)
    li_vn_mode = (3 << 6 if kind in ("leap3", "rate", "deny") else 0) | 4 << 3 | 4
    stratum = {"rate": 0, "deny": 0, "stratum16": 16}.get(kind, 2)
    ref_id = {"rate": b"RATE", "deny": b"DENY"}.get(kind, b"TEST")
    origin = bytes(8) if kind == "origin" else request[40:48]
    transmit = bytes(8) if kind == "zero" else ntp(now)
    return struct.pack(">BBbb", li_vn_mode, stratum, 6, -20) + bytes(8) + ref_id + ntp(now) + origin + ntp(now) + transmit


def main():
    addr = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1"
    end = time.time() + (float(sys.argv[2]) if len(sys.argv) > 2 else 3600)
    socks = {}
    for port, kind in KINDS.items():
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.bind((addr, port))
        socks[s] = kind
    while time.time() < end:
        ready, _, _ = select.select(list(socks), [], [], 1)
        for s in ready:
            request, peer = s.recvfrom(512)
            if len(request) >= 48:
                s.sendto(answer(socks[s], request), peer)
                print(f"{time.strftime('%H:%M:%S')} {socks[s]} -> {peer[0]}:{peer[1]}", flush=True)


if __name__ == "__main__":
    main()
