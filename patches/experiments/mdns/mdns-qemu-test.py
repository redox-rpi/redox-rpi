#!/usr/bin/env python3
"""Host side of the mDNS test in QEMU (README.md in this directory).

Like ../multicast/mcast-qemu-test.py it joins the multicast group of QEMU's
socket netdev and plays a second host, 10.0.3.2, on the guest's subnet. It
answers the guest's ARP requests and, every two seconds, asks:

1. `redox-rpi.local` A, from port 5353 (multicast answer expected),
2. `_devsh._tcp.local` PTR, from port 5353,
3. `redox-rpi.local` A, from port 40000 (one-shot query: unicast answer
   to 10.0.3.2:40000 with the same ID expected).

With --ipv6 it also asks over IPv6, from fe80::5054:ff:fe12:3499 to
ff02::fb: `redox-rpi.local` AAAA from port 5353 (multicast answer to
ff02::fb expected) and from port 40000 (unicast answer expected), and
answers the guest's neighbor solicitations.

It prints every mDNS response of the guest. Exit code 0 when it saw the
A record with --expect-addr (multicast and unicast) and an SRV record with
port 2323 within --seconds; with --ipv6 also a link-local AAAA record over
IPv6, multicast and unicast.
"""

import argparse
import socket
import struct
import sys
import time

HOST_MAC = bytes.fromhex("525400123499")
HOST_IP = socket.inet_aton("10.0.3.2")
MDNS = "224.0.0.251"
HOST_LL = bytes.fromhex("fe80000000000000") + bytes([0x52 ^ 2, 0x54, 0x00, 0xFF, 0xFE, 0x12, 0x34, 0x99])
MDNS6 = bytes.fromhex("ff0200000000000000000000000000fb")


def checksum(data):
    if len(data) % 2:
        data += b"\0"
    s = sum(struct.unpack(f"!{len(data) // 2}H", data))
    while s >> 16:
        s = (s & 0xFFFF) + (s >> 16)
    return ~s & 0xFFFF


def mac_for(ip):
    b = socket.inet_aton(ip)
    return bytes([0x01, 0x00, 0x5E, b[1] & 0x7F, b[2], b[3]])


def udp_frame(dst_mac, dst_ip, sport, dport, payload):
    dst = socket.inet_aton(dst_ip)
    ulen = 8 + len(payload)
    udp = struct.pack("!HHHH", sport, dport, ulen, 0) + payload
    pseudo = HOST_IP + dst + struct.pack("!BBH", 0, 17, ulen)
    udp = udp[:6] + struct.pack("!H", checksum(pseudo + udp) or 0xFFFF) + udp[8:]
    ip = struct.pack("!BBHHHBBH4s4s", 0x45, 0, 20 + ulen, 0, 0, 255, 17, 0, HOST_IP, dst)
    ip = ip[:10] + struct.pack("!H", checksum(ip)) + ip[12:]
    return dst_mac + HOST_MAC + b"\x08\x00" + ip + udp


def udp6_frame(dst_mac, dst, sport, dport, payload):
    udp = struct.pack("!HHHH", sport, dport, 8 + len(payload), 0) + payload
    pseudo = HOST_LL + dst + struct.pack("!I3xB", len(udp), 17)
    udp = udp[:6] + struct.pack("!H", checksum(pseudo + udp) or 0xFFFF) + udp[8:]
    ip = struct.pack("!IHBB", 6 << 28, len(udp), 17, 255) + HOST_LL + dst
    return dst_mac + HOST_MAC + b"\x86\xdd" + ip + udp


def neighbor_advert(dst_mac, dst, target):
    icmp = (struct.pack("!BBHI", 136, 0, 0, 0x60000000) + target
            + struct.pack("!BB", 2, 1) + HOST_MAC)
    pseudo = target + dst + struct.pack("!I3xB", len(icmp), 58)
    icmp = icmp[:2] + struct.pack("!H", checksum(pseudo + icmp)) + icmp[4:]
    ip = struct.pack("!IHBB", 6 << 28, len(icmp), 58, 255) + target + dst
    return dst_mac + HOST_MAC + b"\x86\xdd" + ip + icmp


def query(qid, name, qtype):
    q = b"".join(bytes([len(l)]) + l.encode() for l in name.split(".")) + b"\0"
    return struct.pack("!HHHHHH", qid, 0, 1, 0, 0, 0) + q + struct.pack("!HH", qtype, 1)


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
    out = []
    for _ in range(an + ns + ar):
        name, pos = read_name(msg, pos)
        rtype, rclass, ttl, rdlen = struct.unpack("!HHIH", msg[pos:pos + 10])
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
        out.append((name, {1: "A", 12: "PTR", 16: "TXT", 28: "AAAA", 33: "SRV"}.get(rtype, rtype), ttl, value))
        pos += 10 + rdlen
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--socket", default="239.255.77.77:17777")
    ap.add_argument("--expect-addr", default="10.0.3.15")
    ap.add_argument("--seconds", type=int, default=900)
    ap.add_argument("--ipv6", action="store_true", help="also query over IPv6 for AAAA")
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

    seen = {"multicast A": False, "unicast A": False, "SRV 2323": False}
    if args.ipv6:
        seen.update({"multicast AAAA over IPv6": False, "unicast AAAA over IPv6": False})
    end, next_ask, qid = time.time() + args.seconds, 0, 0x4242
    while time.time() < end and not all(seen.values()):
        if time.time() >= next_ask:
            next_ask = time.time() + 2
            for sport, name, qtype, i in ((5353, "redox-rpi.local", 1, 0),
                                          (5353, "_devsh._tcp.local", 12, 0),
                                          (40000, "redox-rpi.local", 1, qid)):
                s.sendto(udp_frame(mac_for(MDNS), MDNS, sport, 5353, query(i, name, qtype)),
                         (group, port))
            if args.ipv6:
                for sport, i in ((5353, 0), (40000, qid + 1)):
                    s.sendto(udp6_frame(b"\x33\x33" + MDNS6[12:], MDNS6, sport, 5353,
                                        query(i, "redox-rpi.local", 28)), (group, port))
        try:
            frame = s.recv(2048)
        except socket.timeout:
            continue
        if frame[6:12] == HOST_MAC:
            continue
        etype = frame[12:14]
        if etype == b"\x08\x06" and frame[38:42] == HOST_IP and frame[20:22] == b"\x00\x01":
            guest_mac, guest_ip = frame[22:28], frame[28:32]
            reply = (guest_mac + HOST_MAC + b"\x08\x06" + frame[14:20] + b"\x00\x02"
                     + HOST_MAC + HOST_IP + guest_mac + guest_ip)
            s.sendto(reply, (group, port))
            print(f"ARP reply to {socket.inet_ntoa(guest_ip)}", flush=True)
            continue
        if etype == b"\x86\xdd":
            ip6 = frame[14:]
            src6, dst6, nh = ip6[8:24], ip6[24:40], ip6[6]
            if nh == 58 and ip6[40] == 135 and ip6[48:64] == HOST_LL:
                s.sendto(neighbor_advert(frame[6:12], src6, HOST_LL), (group, port))
                print(f"neighbor advert to {socket.inet_ntop(socket.AF_INET6, src6)}", flush=True)
            if nh != 17:
                continue
            sport, dport = struct.unpack("!HH", ip6[40:44])
            if sport != 5353:
                continue
            msg = ip6[48:]
            dst = socket.inet_ntop(socket.AF_INET6, dst6)
            rid = struct.unpack("!H", msg[:2])[0]
            recs = records(msg)
            print(f"IPv6 response to [{dst}]:{dport} id {rid:#x}: {recs}", flush=True)
            for name, rtype, _, value in recs:
                if rtype == "AAAA" and value.startswith("fe80:") and name == "redox-rpi.local":
                    if dst6 == MDNS6:
                        seen["multicast AAAA over IPv6"] = True
                    elif dport == 40000 and rid == qid + 1:
                        seen["unicast AAAA over IPv6"] = True
            continue
        if etype != b"\x08\x00":
            continue
        ip = frame[14:]
        ihl = (ip[0] & 0x0F) * 4
        if ip[9] != 17:
            continue
        sport, dport = struct.unpack("!HH", ip[ihl:ihl + 4])
        if sport != 5353:
            continue
        msg = ip[ihl + 8:]
        dst = socket.inet_ntoa(ip[16:20])
        rid = struct.unpack("!H", msg[:2])[0]
        recs = records(msg)
        print(f"response to {dst}:{dport} id {rid:#x}: {recs}", flush=True)
        for name, rtype, _, value in recs:
            if rtype == "A" and value == args.expect_addr and name == "redox-rpi.local":
                if dst == MDNS:
                    seen["multicast A"] = True
                elif dport == 40000 and rid == qid:
                    seen["unicast A"] = True
            if rtype == "SRV" and value.startswith("port 2323 "):
                seen["SRV 2323"] = True
    print("result: " + ", ".join(f"{k} {'yes' if v else 'no'}" for k, v in seen.items()))
    sys.exit(0 if all(seen.values()) else 1)


if __name__ == "__main__":
    main()
