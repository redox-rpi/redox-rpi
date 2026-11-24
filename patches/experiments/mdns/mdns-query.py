#!/usr/bin/env python3
"""One-shot mDNS query (RFC 6762 section 5.1) from the build host, for the
Pi tests when avahi is not installed.

    mdns-query.py redox-rpi.local            # A, over IPv4
    mdns-query.py redox-rpi.local AAAA -6 eth0   # AAAA, over IPv6 on eth0
    mdns-query.py _devsh._tcp.local PTR

Sends the question from an ephemeral port to 224.0.0.251 (or ff02::fb on
the given interface), prints every record of the answers within --seconds,
exit code 0 when at least one answer came.
"""

import argparse
import socket
import struct
import sys
import time

TYPES = {"A": 1, "PTR": 12, "TXT": 16, "AAAA": 28, "SRV": 33, "ANY": 255}
NAMES = {v: k for k, v in TYPES.items()}


def read_name(msg, pos):
    labels, end = [], None
    while True:
        n = msg[pos]
        if n == 0:
            return ".".join(labels), end or pos + 1
        if n & 0xC0 == 0xC0:
            end = end or pos + 2
            pos = ((n & 0x3F) << 8) | msg[pos + 1]
            continue
        labels.append(msg[pos + 1:pos + 1 + n].decode(errors="replace"))
        pos += 1 + n


def records(msg):
    qd, an, ns, ar = struct.unpack("!HHHH", msg[4:12])
    pos = 12
    for _ in range(qd):
        _, pos = read_name(msg, pos)
        pos += 4
    for _ in range(an + ns + ar):
        name, pos = read_name(msg, pos)
        rtype, _, ttl, rdlen = struct.unpack("!HHIH", msg[pos:pos + 10])
        rdata = msg[pos + 10:pos + 10 + rdlen]
        if rtype == 1:
            value = socket.inet_ntoa(rdata)
        elif rtype == 28:
            value = socket.inet_ntop(socket.AF_INET6, rdata)
        elif rtype == 33:
            value = f"port {struct.unpack('!H', rdata[4:6])[0]} target {read_name(msg, pos + 16)[0]}"
        elif rtype == 12:
            value = read_name(msg, pos + 10)[0]
        else:
            value = rdata.hex()
        yield name, NAMES.get(rtype, rtype), ttl, value
        pos += 10 + rdlen


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("name")
    ap.add_argument("type", nargs="?", default="A", choices=TYPES)
    ap.add_argument("-6", dest="iface", help="ask over IPv6 on this interface")
    ap.add_argument("--seconds", type=float, default=3)
    args = ap.parse_args()

    qid = int(time.time()) & 0xFFFF
    q = b"".join(bytes([len(l)]) + l.encode() for l in args.name.split(".")) + b"\0"
    msg = struct.pack("!HHHHHH", qid, 0, 1, 0, 0, 0) + q + struct.pack("!HH", TYPES[args.type], 1)

    if args.iface:
        s = socket.socket(socket.AF_INET6, socket.SOCK_DGRAM)
        scope = socket.if_nametoindex(args.iface)
        s.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_MULTICAST_IF, scope)
        s.sendto(msg, ("ff02::fb", 5353, 0, scope))
    else:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.sendto(msg, ("224.0.0.251", 5353))

    got = False
    end = time.time() + args.seconds
    while time.time() < end:
        s.settimeout(max(0.01, end - time.time()))
        try:
            reply, sender = s.recvfrom(9000)
        except socket.timeout:
            break
        if struct.unpack("!H", reply[:2])[0] != qid:
            continue
        got = True
        for name, rtype, ttl, value in records(reply):
            print(f"{sender[0]}: {name} {rtype} {value} (ttl {ttl})")
    if not got:
        print(f"no answer for {args.name} {args.type}", file=sys.stderr)
    sys.exit(0 if got else 1)


if __name__ == "__main__":
    main()
