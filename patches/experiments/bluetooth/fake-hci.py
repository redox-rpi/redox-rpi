#!/usr/bin/env python3
"""A pretend BCM43455 Bluetooth controller for the QEMU check: listens on a
TCP port that QEMU's mini UART connects to and answers HCI (H4 framing)
like the chip would: reset, firmware upload (Download Minidriver, Write
RAM, Launch RAM), version (build 0 before the upload, 344 after), address,
name, an LE scan with two advertisers and an inquiry with one device.
Writes what it was sent to the log file.

Usage: fake-hci.py <port> <log>
"""

import socket
import struct
import sys

port, log_path = int(sys.argv[1]), sys.argv[2]
log = open(log_path, "w", buffering=1)


def event(code, params):
    return bytes([0x04, code, len(params)]) + params


def complete(opcode, ret):
    return event(0x0E, bytes([1]) + struct.pack("<H", opcode) + ret)


def status(opcode, st=0):
    return event(0x0F, bytes([st, 1]) + struct.pack("<H", opcode))


ADDR = bytes.fromhex("ac1f00c04543")  # 43:45:C0:00:1F:AC, the chip's default
BEACON = bytes.fromhex("665544332211")  # 11:22:33:44:55:66
PHONE = bytes.fromhex("0c0b0a090807")  # 07:08:09:0A:0B:0C
PC = bytes.fromhex("feedbeefcafe")  # FE:CA:EF:BE:ED:FE


def adv_report(addr, data, rssi):
    return event(0x3E, bytes([0x02, 1, 0x00, 0x00]) + addr + bytes([len(data)]) + data + struct.pack("<b", rssi))


def answer(opcode, params, state):
    if opcode == 0x0C03:
        return complete(opcode, b"\x00")
    if opcode == 0xFC2E:
        state["minidriver"] = True
        return complete(opcode, b"\x00")
    if opcode == 0xFC4C:
        if not state["minidriver"]:
            return complete(opcode, b"\x0c")  # command disallowed
        state["records"] += 1
        return complete(opcode, b"\x00")
    if opcode == 0xFC4E:
        state["records"] += 1
        state["build"] = 344
        log.write(f"launch after {state['records']} records\n")
        return complete(opcode, b"\x00")
    if opcode == 0x1001:
        # HCI 7 (4.1), revision = build, LMP 7, Broadcom, subversion 0x6119
        return complete(opcode, b"\x00" + struct.pack("<BHBHH", 7, state["build"], 7, 15, 0x6119))
    if opcode == 0x1009:
        return complete(opcode, b"\x00" + ADDR)
    if opcode == 0x0C14:
        return complete(opcode, b"\x00" + b"BCM4345C0".ljust(248, b"\x00"))
    if opcode in (0x0C01, 0x0C6D, 0x200B):
        return complete(opcode, b"\x00")
    if opcode == 0x200C:
        out = complete(opcode, b"\x00")
        if params[:1] == b"\x01":
            name = b"redox-beacon"
            out += adv_report(BEACON, bytes([2, 0x01, 0x06, len(name) + 1, 0x09]) + name, -42)
            out += adv_report(PHONE, bytes([2, 0x01, 0x1A]), -70)
        return out
    if opcode == 0x0401:
        # One device: address, page scan mode, 2 reserved, class, clock offset.
        result = event(0x02, bytes([1]) + PC + b"\x01\x00\x00" + bytes.fromhex("0c010a") + b"\x00\x00")
        return status(opcode) + result + event(0x01, b"\x00")
    return complete(opcode, b"\x01")  # unknown command


server = socket.socket()
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(("127.0.0.1", port))
server.listen(1)
log.write(f"listening on {port}\n")
conn, _ = server.accept()
state = {"minidriver": False, "records": 0, "build": 0}
buf = b""
while True:
    data = conn.recv(4096)
    if not data:
        break
    buf += data
    while buf:
        if buf[0] != 0x01:
            log.write(f"stray byte {buf[0]:02x}\n")
            buf = buf[1:]
            continue
        if len(buf) < 4 or len(buf) < 4 + buf[3]:
            break
        opcode = buf[1] | buf[2] << 8
        params = buf[4 : 4 + buf[3]]
        buf = buf[4 + buf[3] :]
        if opcode != 0xFC4C:
            log.write(f"command {opcode:04x} {params.hex()}\n")
        if opcode == 0x0C03 and state["build"] == 0:
            state["minidriver"] = False
        conn.sendall(answer(opcode, params, state))
log.write(f"closed, {state['records']} firmware records\n")
