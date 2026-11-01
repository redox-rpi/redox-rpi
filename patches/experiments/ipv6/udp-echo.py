#!/usr/bin/env python3
"""UDP echo on the build host for the Pi's IPv6 test: answers every datagram
on [::]:7777 (IPv4 too) with the same bytes, prints the sender, and quits
after --seconds. On the Pi: `net6 udp [<host address>]:7777 hello`."""

import argparse
import socket
import time

ap = argparse.ArgumentParser()
ap.add_argument("--port", type=int, default=7777)
ap.add_argument("--seconds", type=float, default=600)
args = ap.parse_args()

s = socket.socket(socket.AF_INET6, socket.SOCK_DGRAM)
s.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 0)
s.bind(("::", args.port))
end = time.time() + args.seconds
while time.time() < end:
    s.settimeout(max(0.01, end - time.time()))
    try:
        data, sender = s.recvfrom(2048)
    except socket.timeout:
        break
    print(f"echo {len(data)} bytes to {sender[0]} port {sender[1]}", flush=True)
    s.sendto(data, sender)
