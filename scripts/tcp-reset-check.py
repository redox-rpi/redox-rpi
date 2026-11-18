#!/usr/bin/env python3
"""Host side of the QEMU test for reads after a TCP reset (redox-rpi #141).

QEMU's user network ends the guest's connection itself and turns a reset
from the host into a normal close, so this test uses QEMU's socket netdev
instead (`scripts/qemu-raspi3b.sh --usb-net-socket GROUP:PORT`): every
Ethernet frame of the guest travels as one UDP datagram to a host
multicast group. This script joins that group and plays a second host,
10.0.3.2, on the guest's subnet, with a TCP server of its own on port 5000
(no host socket in between, so the reset reaches the guest as one):

- it answers ARP and accepts one connection;
- it sends one receive window of data (at most --total bytes) and the
  reset right behind the last segment, so the reset reaches the guest's
  netstack before the reader can have taken all of it.

Every byte sent before the reset is in the guest's receive buffer or
already read, so the reader must get all of them before it sees the error.
The script prints `result: reset after N bytes` and exits 0, or 1 if no
connection came or the guest missed a segment (it then refuses the reset
and asks for the gap: run again). scripts/test-tcp-reset-read.sh compares N
with what the guest read.

Usage: tcp-reset-check.py [--socket 239.255.77.78:17778] [--total 65535]
                          [--seconds 600]
"""

import argparse
import socket
import struct
import sys
import time

HOST_MAC = bytes.fromhex("525400123498")
HOST_IP = socket.inet_aton("10.0.3.2")
PORT = 5000
MSS = 1400
ISS = 0x10000000

SYN, RST, PSH, ACK, FIN = 0x02, 0x04, 0x08, 0x10, 0x01


def log(*a):
    print(time.strftime("%H:%M:%S"), *a, flush=True)


def checksum(data):
    if len(data) % 2:
        data += b"\0"
    s = sum(struct.unpack(f"!{len(data) // 2}H", data))
    while s >> 16:
        s = (s & 0xFFFF) + (s >> 16)
    return ~s & 0xFFFF


class Peer:
    def __init__(self, sock, group, total):
        self.sock, self.group, self.total = sock, group, total
        self.guest_mac = self.guest_ip = self.guest_port = None
        self.state = "listen"
        self.rcv_nxt = 0
        self.snd_max = ISS + 1
        self.wnd = 0
        self.reset_at = None
        self.lost = False
        self.result = None

    def send_frame(self, dst_mac, ethertype, payload):
        self.sock.sendto(dst_mac + HOST_MAC + ethertype + payload, self.group)

    def send_tcp(self, flags, seq, payload=b"", options=b""):
        hl = 20 + len(options)
        tcp = struct.pack("!HHIIBBHHH", PORT, self.guest_port, seq & 0xFFFFFFFF,
                          self.rcv_nxt & 0xFFFFFFFF, (hl // 4) << 4, flags, 65535, 0, 0)
        tcp += options + payload
        pseudo = HOST_IP + self.guest_ip + struct.pack("!BBH", 0, 6, len(tcp))
        tcp = tcp[:16] + struct.pack("!H", checksum(pseudo + tcp)) + tcp[18:]
        ip = struct.pack("!BBHHHBBH4s4s", 0x45, 0, 20 + len(tcp), 0, 0x4000, 64, 6, 0,
                         HOST_IP, self.guest_ip)
        ip = ip[:10] + struct.pack("!H", checksum(ip)) + ip[12:]
        self.send_frame(self.guest_mac, b"\x08\x00", ip + tcp)

    def arp(self, frame):
        arp = frame[14:42]
        if len(arp) < 28 or arp[6:8] != b"\x00\x01" or arp[24:28] != HOST_IP:
            return
        reply = arp[:6] + b"\x00\x02" + HOST_MAC + HOST_IP + arp[8:14] + arp[14:18]
        self.send_frame(frame[6:12], b"\x08\x06", reply)

    def burst(self):
        """Send one window of data and the reset right behind it."""
        n = min(self.total, self.wnd)
        for off in range(0, n, MSS):
            data = bytes((off + i) % 251 for i in range(min(MSS, n - off)))
            self.send_tcp(PSH | ACK, ISS + 1 + off, data)
        self.snd_max = ISS + 1 + n
        self.send_tcp(RST | ACK, self.snd_max)
        self.state = "reset"
        self.reset_at = time.time()
        self.result = n
        log(f"sent {n} bytes and the reset right behind them")

    def after_reset(self, ack):
        # The guest only answers if it missed data: then it refuses the reset
        # (wrong sequence number) and asks for the gap.
        if ack != self.snd_max:
            self.lost = True

    def tcp(self, frame):
        ip = frame[14:]
        ihl = (ip[0] & 0x0F) * 4
        if ip[9] != 6 or ip[16:20] != HOST_IP:
            return
        seg = ip[ihl:struct.unpack("!H", ip[2:4])[0]]
        sport, dport, seq, ack, off, flags, win = struct.unpack("!HHIIBBH", seg[:16])
        if dport != PORT:
            return
        payload = seg[(off >> 4) * 4:]
        if flags & SYN and self.state == "listen":
            self.guest_mac, self.guest_ip, self.guest_port = frame[6:12], ip[12:16], sport
            self.rcv_nxt = seq + 1
            log(f"SYN from {socket.inet_ntoa(self.guest_ip)}:{sport}")
            self.send_tcp(SYN | ACK, ISS, options=struct.pack("!BBH", 2, 4, MSS))
            self.state = "syn-received"
            return
        if sport != self.guest_port or self.state == "listen":
            return
        if self.state == "reset":
            if flags & ACK and not flags & RST:
                self.after_reset(ack)
            return
        if flags & RST:
            log("the guest reset the connection")
            self.state = "reset"
            return
        if payload or flags & FIN:
            if seq == self.rcv_nxt:
                self.rcv_nxt += len(payload) + (1 if flags & FIN else 0)
            self.send_tcp(ACK, ISS + 1)
        if flags & ACK and self.state == "syn-received" and ack == ISS + 1:
            log(f"connection established, guest window {win}")
            self.wnd = win
            self.state = "sending"
            self.burst()

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--socket", default="239.255.77.78:17778")
    ap.add_argument("--total", type=int, default=65535)
    ap.add_argument("--seconds", type=int, default=600)
    args = ap.parse_args()
    mgroup, mport = args.socket.rsplit(":", 1)
    group = (mgroup, int(mport))

    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(("", group[1]))
    s.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP,
                 socket.inet_aton(mgroup) + socket.inet_aton("0.0.0.0"))
    s.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 0)
    s.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_LOOP, 1)
    s.settimeout(0.2)

    peer = Peer(s, group, args.total)
    log(f"10.0.3.2:{PORT} waits for a connection on {args.socket}")
    end = time.time() + args.seconds
    while time.time() < end and not (peer.reset_at and time.time() - peer.reset_at > 5):
        try:
            frame = s.recv(2048)
        except socket.timeout:
            continue
        if len(frame) < 42 or frame[6:12] == HOST_MAC:
            continue
        if frame[12:14] == b"\x08\x06":
            peer.arp(frame)
        elif frame[12:14] == b"\x08\x00":
            peer.tcp(frame)
    if peer.result is None:
        print("result: no connection")
        sys.exit(1)
    if peer.lost:
        print(f"result: the guest missed some of the {peer.result} bytes and refused the reset")
        sys.exit(1)
    print(f"result: reset after {peer.result} bytes")
    sys.exit(0)


if __name__ == "__main__":
    main()
