#!/usr/bin/env python3
"""Host side of the multicast test in QEMU (README.md in this directory).

QEMU's socket netdev (`scripts/qemu-raspi3b.sh --usb-net-socket
GROUP:PORT`) sends every Ethernet frame of the guest as one UDP datagram to
a host multicast group and takes frames for the guest from it. This script
joins that group, plays a second host on the guest's subnet and:

- sends a UDP datagram to the test group (default 239.1.2.3:5000) once a
  second, as an Ethernet frame to the group's MAC address, for the guest's
  `mcast recv`;
- prints the IGMP reports and the UDP datagrams to multicast groups that
  the guest sends, with their destination MAC.

Exit code 0 if it saw an IGMP report for the test group and a datagram from
the guest to a group with the matching 01:00:5e MAC within --seconds.

Usage: mcast-qemu-test.py [--socket 239.255.77.77:17777] [--seconds 600]
"""

import argparse
import socket
import struct
import sys
import time

HOST_MAC = bytes.fromhex("525400123499")
HOST_IP = "10.0.3.2"


def checksum(data):
    if len(data) % 2:
        data += b"\0"
    s = sum(struct.unpack(f"!{len(data) // 2}H", data))
    while s >> 16:
        s = (s & 0xFFFF) + (s >> 16)
    return ~s & 0xFFFF


def group_mac(group):
    b = socket.inet_aton(group)
    return bytes([0x01, 0x00, 0x5E, b[1] & 0x7F, b[2], b[3]])


def udp_frame(group, port, payload):
    src = socket.inet_aton(HOST_IP)
    dst = socket.inet_aton(group)
    udp_len = 8 + len(payload)
    pseudo = src + dst + struct.pack("!BBH", 0, 17, udp_len)
    udp = struct.pack("!HHHH", port, port, udp_len, 0) + payload
    udp = udp[:6] + struct.pack("!H", checksum(pseudo + udp) or 0xFFFF) + udp[8:]
    ip = struct.pack("!BBHHHBBH4s4s", 0x45, 0, 20 + udp_len, 0, 0, 1, 17, 0, src, dst)
    ip = ip[:10] + struct.pack("!H", checksum(ip)) + ip[12:]
    return group_mac(group) + HOST_MAC + b"\x08\x00" + ip + udp


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--socket", default="239.255.77.77:17777")
    ap.add_argument("--group", default="239.1.2.3")
    ap.add_argument("--port", type=int, default=5000)
    ap.add_argument("--seconds", type=int, default=600)
    args = ap.parse_args()
    mgroup, mport = args.socket.rsplit(":", 1)
    mport = int(mport)

    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(("", mport))
    s.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP,
                 socket.inet_aton(mgroup) + socket.inet_aton("0.0.0.0"))
    s.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 0)
    s.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_LOOP, 1)
    s.settimeout(0.2)

    saw_igmp = saw_udp = False
    end = time.time() + args.seconds
    next_send = 0
    n = 0
    while time.time() < end and not (saw_igmp and saw_udp):
        if time.time() >= next_send:
            n += 1
            s.sendto(udp_frame(args.group, args.port, f"hello {n} from the host".encode()),
                     (mgroup, mport))
            next_send = time.time() + 1
        try:
            frame = s.recv(2048)
        except socket.timeout:
            continue
        if len(frame) < 34 or frame[6:12] == HOST_MAC or frame[12:14] != b"\x08\x00":
            continue
        dst_mac, ip = frame[:6], frame[14:]
        ihl = (ip[0] & 0x0F) * 4
        proto, src, dst = ip[9], socket.inet_ntoa(ip[12:16]), socket.inet_ntoa(ip[16:20])
        if proto == 2:
            igmp = ip[ihl:]
            kind = {0x16: "v2 report", 0x17: "leave", 0x22: "v3 report"}.get(igmp[0], hex(igmp[0]))
            grp = socket.inet_ntoa(igmp[4:8]) if igmp[0] in (0x16, 0x17) else ""
            print(f"IGMP {kind} {grp} from {src} to {dst}, MAC {dst_mac.hex(':')}", flush=True)
            if grp == args.group or igmp[0] == 0x22:
                saw_igmp = True
        elif proto == 17 and ip[16] >= 224:
            sport, dport = struct.unpack("!HH", ip[ihl:ihl + 4])
            payload = ip[ihl + 8:]
            ok = dst_mac == group_mac(dst)
            print(f"UDP {src}:{sport} -> {dst}:{dport}, MAC {dst_mac.hex(':')}"
                  f" ({'matches' if ok else 'WRONG'}): {payload!r}", flush=True)
            saw_udp = saw_udp or ok
    print(f"result: igmp {'yes' if saw_igmp else 'no'}, udp from guest {'yes' if saw_udp else 'no'}")
    sys.exit(0 if saw_igmp and saw_udp else 1)


if __name__ == "__main__":
    main()
