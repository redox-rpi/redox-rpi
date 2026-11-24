#!/usr/bin/env python3
"""Host side of the QEMU test for ARP during a long download (redox-rpi #140).

netstack forgets a neighbour's MAC address 60 s after it learned it. Every
packet to that host then waits in a queue of 10 until an ARP reply comes,
so a busy connection drops its ACKs once a minute (`Dropped packet on eth0
because waiting queue was full`). QEMU's user network answers ARP at once,
so the drop never shows there. This script plays a host that answers ARP
late, over QEMU's socket netdev (`scripts/qemu-raspi3b.sh --usb-net-socket
GROUP:PORT`): every Ethernet frame of the guest is one UDP datagram to a
host multicast group.

It plays 10.0.3.2 on the guest's subnet, answers each ARP request after
--arp-delay seconds (default 1.5), accepts one TCP connection on port 5000
and streams data for --seconds (default 150, two expiries), as fast as the
guest's window allows, then closes with FIN. It counts the guest's ARP
requests, the resends, and every pause of more than 0.5 s in the guest's
ACKs: that is where the guest held its ACKs back (or dropped them) while it
asked for the address again. netstack's own `waiting queue was full`
warning does not reach the serial console at its log level, so the pauses
are the measure.

Prints `result: N bytes in S s, A ARP requests, R resends, P ACK pauses
over 0.5 s (...)` and exits 0 when the stream ran to the end and the guest
closed too.

Usage: tcp-arp-check.py [--socket 239.255.77.79:17779] [--seconds 150]
                        [--arp-delay 1.5] [--wait 600]
"""

import argparse
import heapq
import socket
import struct
import sys
import time

HOST_MAC = bytes.fromhex("525400123497")
HOST_IP = socket.inet_aton("10.0.3.2")
PORT = 5000
MSS = 1400
ISS = 0x20000000
RTO = 0.5

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
    def __init__(self, sock, group, seconds):
        self.sock, self.group, self.seconds = sock, group, seconds
        self.guest_mac = self.guest_ip = self.guest_port = None
        self.state = "listen"
        self.rcv_nxt = 0
        self.snd_una = self.snd_nxt = self.snd_max = ISS + 1
        self.wnd = 0
        self.last_ack = time.time()
        self.started = None
        self.fin_seq = None
        self.arp_requests = 0
        self.resends = 0
        self.stalls = []  # (seconds into the stream, length) of ACK pauses over RTO
        self.prev_ack_at = None
        self.timers = []

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

    def arp(self, frame, delay):
        arp = frame[14:42]
        if len(arp) < 28 or arp[6:8] != b"\x00\x01" or arp[24:28] != HOST_IP:
            return
        self.arp_requests += 1
        log(f"ARP request {self.arp_requests} from the guest, answering in {delay} s")
        reply = arp[:6] + b"\x00\x02" + HOST_MAC + HOST_IP + arp[8:14] + arp[14:18]
        heapq.heappush(self.timers, (time.time() + delay, frame[6:12], reply))

    def run_timers(self):
        while self.timers and self.timers[0][0] <= time.time():
            _, mac, reply = heapq.heappop(self.timers)
            self.send_frame(mac, b"\x08\x06", reply)

    def pump(self):
        if self.state != "sending":
            return
        if time.time() - self.started >= self.seconds and self.fin_seq is None:
            self.fin_seq = self.snd_max
        limit = self.fin_seq if self.fin_seq is not None else None
        while self.snd_nxt - self.snd_una < self.wnd and (limit is None or self.snd_nxt < limit):
            n = min(MSS, self.wnd - (self.snd_nxt - self.snd_una))
            if limit is not None:
                n = min(n, limit - self.snd_nxt)
            if n <= 0:
                break
            off = self.snd_nxt - ISS - 1
            self.send_tcp(PSH | ACK, self.snd_nxt, bytes((off + i) % 251 for i in range(n)))
            self.snd_nxt += n
            self.snd_max = max(self.snd_max, self.snd_nxt)
        if self.fin_seq is not None and self.snd_nxt == self.fin_seq:
            self.send_tcp(FIN | ACK, self.fin_seq)
            self.snd_nxt = self.fin_seq + 1
            self.snd_max = max(self.snd_max, self.snd_nxt)

    def retransmit(self):
        if self.state == "sending" and self.snd_una != self.snd_max \
                and time.time() - self.last_ack > RTO:
            self.resends += 1
            self.snd_nxt = self.snd_una
            self.last_ack = time.time()
            self.pump()

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
        if sport != self.guest_port or self.state in ("listen", "done"):
            return
        if flags & RST:
            log("the guest reset the connection")
            self.state = "done"
            return
        if payload or flags & FIN:
            if seq == self.rcv_nxt:
                self.rcv_nxt += len(payload) + (1 if flags & FIN else 0)
            self.send_tcp(ACK, self.snd_max)
            if flags & FIN and self.fin_seq is not None:
                log("the guest closed its side too")
                self.state = "done"
                return
        if flags & ACK:
            if self.state == "syn-received" and ack == ISS + 1:
                self.state = "sending"
                self.started = time.time()
                log("connection established, streaming")
            if 0 < ack - self.snd_una <= self.snd_max - self.snd_una:
                now = time.time()
                if self.prev_ack_at is not None and now - self.prev_ack_at > RTO:
                    self.stalls.append((self.prev_ack_at - self.started, now - self.prev_ack_at))
                    log(f"ACKs paused {now - self.prev_ack_at:.1f} s at {self.prev_ack_at - self.started:.0f} s")
                self.prev_ack_at = now
                self.snd_una = ack
                self.snd_nxt = max(self.snd_nxt, ack)
                self.last_ack = time.time()
            self.wnd = win
            self.pump()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--socket", default="239.255.77.79:17779")
    ap.add_argument("--seconds", type=float, default=150)
    ap.add_argument("--arp-delay", type=float, default=1.5)
    ap.add_argument("--wait", type=float, default=600)
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
    s.settimeout(0.05)

    peer = Peer(s, group, args.seconds)
    log(f"10.0.3.2:{PORT} waits for a connection on {args.socket}")
    end = time.time() + args.wait
    while time.time() < end and peer.state != "done":
        try:
            frame = s.recv(2048)
        except socket.timeout:
            frame = None
        peer.run_timers()
        if frame and len(frame) >= 42 and frame[6:12] != HOST_MAC:
            if frame[12:14] == b"\x08\x06":
                peer.arp(frame, args.arp_delay)
            elif frame[12:14] == b"\x08\x00":
                peer.tcp(frame)
        peer.retransmit()
        peer.pump()
    sent = peer.snd_una - ISS - 1 - (1 if peer.fin_seq is not None and peer.snd_una > peer.fin_seq else 0)
    took = time.time() - peer.started if peer.started else 0
    pauses = ", ".join(f"{g:.1f} s at {t:.0f} s" for t, g in peer.stalls) or "none"
    print(f"result: {sent} bytes in {took:.0f} s, {peer.arp_requests} ARP requests, "
          f"{peer.resends} resends, {len(peer.stalls)} ACK pauses over {RTO} s ({pauses})")
    sys.exit(0 if peer.state == "done" and peer.fin_seq is not None else 1)


if __name__ == "__main__":
    main()
