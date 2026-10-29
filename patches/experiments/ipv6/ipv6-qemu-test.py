#!/usr/bin/env python3
"""Host side of the IPv6 test in QEMU (README.md in this directory).

Joins the multicast group of QEMU's socket netdev (`qemu-raspi3b.sh
--usb-net-socket`) and plays an IPv6 router on the guest's link:

- answers the guest's router solicitation (and repeats every 10 s) with a
  router advertisement for 2001:db8:1::/64 (on-link, autonomous);
- answers neighbor solicitations for its own addresses;
- asks for the guest's SLAAC address with a neighbor solicitation;
- pings the guest's link-local and SLAAC address (ICMPv6 echo).

Exit code 0 when it saw a router solicitation, a neighbor advertisement
from the guest for its SLAAC address and echo replies from both addresses
within --seconds.
"""

import argparse
import socket
import struct
import sys
import time

HOST_MAC = bytes.fromhex("525400123499")
PREFIX = bytes.fromhex("20010db8000100000000000000000000")


def eui64(prefix, mac):
    return prefix[:8] + bytes([mac[0] ^ 2, mac[1], mac[2], 0xFF, 0xFE, mac[3], mac[4], mac[5]])


HOST_LL = eui64(bytes.fromhex("fe80" + "00" * 14), HOST_MAC)
HOST_GLOBAL = PREFIX[:15] + b"\x01"
ALL_NODES = bytes.fromhex("ff020000000000000000000000000001")


def checksum(data):
    if len(data) % 2:
        data += b"\0"
    s = sum(struct.unpack(f"!{len(data) // 2}H", data))
    while s >> 16:
        s = (s & 0xFFFF) + (s >> 16)
    return ~s & 0xFFFF


def mcast_mac(addr):
    return b"\x33\x33" + addr[12:]


def icmp6_frame(dst_mac, src, dst, icmp, hop_limit=255):
    pseudo = src + dst + struct.pack("!I3xB", len(icmp), 58)
    icmp = icmp[:2] + struct.pack("!H", checksum(pseudo + icmp)) + icmp[4:]
    ip = struct.pack("!IHBB", 6 << 28, len(icmp), 58, hop_limit) + src + dst
    return dst_mac + HOST_MAC + b"\x86\xdd" + ip + icmp


def router_advert():
    body = struct.pack("!BBHBBHII", 134, 0, 0, 64, 0, 1800, 0, 0)
    prefix = struct.pack("!BBBBIII", 3, 4, 64, 0xC0, 86400, 14400, 0) + PREFIX
    sllao = struct.pack("!BB", 1, 1) + HOST_MAC
    return icmp6_frame(mcast_mac(ALL_NODES), HOST_LL, ALL_NODES, body + prefix + sllao)


def neighbor_advert(dst_mac, dst, target):
    body = struct.pack("!BBHI", 136, 0, 0, 0x60000000) + target + struct.pack("!BB", 2, 1) + HOST_MAC
    return icmp6_frame(dst_mac, target, dst, body)


def neighbor_solicit(target, src):
    group = bytes.fromhex("ff0200000000000000000001ff") + target[13:]
    body = struct.pack("!BBHI", 135, 0, 0, 0) + target + struct.pack("!BB", 1, 1) + HOST_MAC
    return icmp6_frame(mcast_mac(group), src, group, body)


def echo_request(dst_mac, src, dst, seq):
    body = struct.pack("!BBHHH", 128, 0, 0, 0x7272, seq) + b"redox ipv6 test"
    return icmp6_frame(dst_mac, src, dst, body, hop_limit=64)


def ntop(a):
    return socket.inet_ntop(socket.AF_INET6, a)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--socket", default="239.255.77.77:17777")
    ap.add_argument("--seconds", type=int, default=900)
    args = ap.parse_args()
    group, port = args.socket.rsplit(":", 1)
    port = int(port)

    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(("", port))
    s.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP,
                 socket.inet_aton(group) + socket.inet_aton("0.0.0.0"))
    s.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 0)
    s.settimeout(0.2)
    send = lambda frame: s.sendto(frame, (group, port))

    seen = {"router solicitation": False, "neighbor advert for SLAAC address": False,
            "echo reply link-local": False, "echo reply SLAAC": False}
    guest_mac = None
    end, next_ra, next_probe, seq = time.time() + args.seconds, 0, 0, 0
    while time.time() < end and not all(seen.values()):
        now = time.time()
        if now >= next_ra:
            send(router_advert())
            next_ra = now + 10
        if guest_mac and now >= next_probe:
            next_probe = now + 2
            seq += 1
            guest_ll = eui64(bytes.fromhex("fe80" + "00" * 14), guest_mac)
            guest_global = eui64(PREFIX, guest_mac)
            send(neighbor_solicit(guest_global, HOST_GLOBAL))
            send(echo_request(guest_mac, HOST_LL, guest_ll, seq))
            send(echo_request(guest_mac, HOST_GLOBAL, guest_global, seq))
        try:
            frame = s.recv(2048)
        except socket.timeout:
            continue
        if frame[6:12] == HOST_MAC or frame[12:14] != b"\x86\xdd" or len(frame) < 58:
            continue
        ip = frame[14:]
        src, dst, nh = ip[8:24], ip[24:40], ip[6]
        if nh != 58:
            print(f"IPv6 next header {nh} from {ntop(src)} to {ntop(dst)}", flush=True)
            continue
        icmp = ip[40:]
        kind = icmp[0]
        mac = frame[6:12]
        if kind == 133:
            guest_mac = mac
            seen["router solicitation"] = True
            print(f"router solicitation from {ntop(src)} ({mac.hex(':')})", flush=True)
            send(router_advert())
        elif kind == 135:
            target = icmp[8:24]
            print(f"neighbor solicitation from {ntop(src)} for {ntop(target)}", flush=True)
            guest_mac = guest_mac or mac
            if target in (HOST_LL, HOST_GLOBAL):
                send(neighbor_advert(mac, src, target))
        elif kind == 136:
            target = icmp[8:24]
            print(f"neighbor advert from {ntop(src)} for {ntop(target)} to {ntop(dst)}", flush=True)
            if guest_mac and target == eui64(PREFIX, guest_mac):
                seen["neighbor advert for SLAAC address"] = True
        elif kind == 129:
            print(f"echo reply from {ntop(src)} to {ntop(dst)} seq {struct.unpack('!H', icmp[6:8])[0]}",
                  flush=True)
            if src[:2] == b"\xfe\x80":
                seen["echo reply link-local"] = True
            elif src[:8] == PREFIX[:8]:
                seen["echo reply SLAAC"] = True
        elif kind == 143:
            print(f"MLDv2 report from {ntop(src)}", flush=True)
        else:
            print(f"ICMPv6 type {kind} from {ntop(src)} to {ntop(dst)}", flush=True)
    print("result: " + ", ".join(f"{k} {'yes' if v else 'no'}" for k, v in seen.items()))
    sys.exit(0 if all(seen.values()) else 1)


if __name__ == "__main__":
    main()
