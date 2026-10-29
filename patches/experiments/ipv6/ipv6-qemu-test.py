#!/usr/bin/env python3
"""Host side of the IPv6 test in QEMU (README.md in this directory).

Joins the multicast group of QEMU's socket netdev (`qemu-raspi3b.sh
--usb-net-socket`) and plays an IPv6 router on the guest's link:

- answers the guest's router solicitation with a router advertisement for
  2001:db8:1::/64 (on-link, autonomous), repeated every 10 s; without a
  solicitation the first one goes out after 240 s;
- answers neighbor solicitations for its own addresses;
- asks for the guest's SLAAC address with a neighbor solicitation;
- pings the guest's link-local and SLAAC address (ICMPv6 echo);
- names itself as DNS server in the advertisement (RDNSS) and answers DNS
  queries over IPv6: `matter.test` AAAA 2001:db8:1::99, no A record;
- echoes UDP datagrams to port 7 of its IPv6 addresses.

Exit code 0 when it saw a router solicitation, a neighbor advertisement
from the guest for its SLAAC address and echo replies from both addresses
within --seconds; with --udp also an AAAA query and a UDP echo over IPv6
(`net6 resolve matter.test`, `net6 udp [2001:db8:1::1]:7 hello` in the
guest).
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
DNS_NAME = "matter.test"
DNS_AAAA = PREFIX[:15] + b"\x99"


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
    rdnss = struct.pack("!BBHI", 25, 3, 0, 1800) + HOST_GLOBAL
    return icmp6_frame(mcast_mac(ALL_NODES), HOST_LL, ALL_NODES, body + prefix + sllao + rdnss)


def udp6_frame(dst_mac, src, dst, sport, dport, payload):
    udp = struct.pack("!HHHH", sport, dport, 8 + len(payload), 0) + payload
    pseudo = src + dst + struct.pack("!I3xB", len(udp), 17)
    udp = udp[:6] + struct.pack("!H", checksum(pseudo + udp) or 0xFFFF) + udp[8:]
    ip = struct.pack("!IHBB", 6 << 28, len(udp), 17, 64) + src + dst
    return dst_mac + HOST_MAC + b"\x86\xdd" + ip + udp


def dns_answer(query):
    """Answer for a DNS query, and its name and type."""
    i, labels = 12, []
    while i < len(query) and query[i]:
        labels.append(query[i + 1:i + 1 + query[i]].decode(errors="replace"))
        i += 1 + query[i]
    qtype = struct.unpack("!H", query[i + 1:i + 3])[0]
    name = ".".join(labels)
    question = query[12:i + 5]
    answers = b""
    if name == DNS_NAME and qtype == 28:
        answers = b"\xc0\x0c" + struct.pack("!HHIH", 28, 1, 60, 16) + DNS_AAAA
    rcode = 0 if name == DNS_NAME else 3
    header = query[:2] + struct.pack("!HHHHH", 0x8180 | rcode, 1, 1 if answers else 0, 0, 0)
    return header + question + answers, name, qtype


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
    ap.add_argument("--udp", action="store_true", help="also require a DNS query and a UDP echo")
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
    if args.udp:
        seen.update({"AAAA query over IPv6": False, "UDP echo over IPv6": False})
    guest_mac = None
    end, next_probe, seq = time.time() + args.seconds, 0, 0
    next_ra = time.time() + 240
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
        if frame[6:12] == HOST_MAC:
            continue
        if guest_mac is None and frame[6] & 1 == 0:
            guest_mac = frame[6:12]
            print(f"guest MAC {guest_mac.hex(':')}", flush=True)
        if frame[12:14] != b"\x86\xdd" or len(frame) < 58:
            continue
        ip = frame[14:]
        src, dst, nh = ip[8:24], ip[24:40], ip[6]
        if nh == 17 and dst in (HOST_LL, HOST_GLOBAL):
            sport, dport = struct.unpack("!HH", ip[40:44])
            payload = ip[48:40 + struct.unpack("!H", ip[44:46])[0]]
            if dport == 53:
                answer, name, qtype = dns_answer(payload)
                print(f"DNS query from [{ntop(src)}]:{sport} for {name} type {qtype}", flush=True)
                if name == DNS_NAME and qtype == 28:
                    seen["AAAA query over IPv6"] = True
                send(udp6_frame(frame[6:12], dst, src, 53, sport, answer))
            elif dport == 7:
                print(f"UDP echo from [{ntop(src)}]:{sport}: {payload!r}", flush=True)
                seen["UDP echo over IPv6"] = True
                send(udp6_frame(frame[6:12], dst, src, 7, sport, payload))
            else:
                print(f"UDP from [{ntop(src)}]:{sport} to port {dport}", flush=True)
            continue
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
            next_ra = time.time() + 10
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
