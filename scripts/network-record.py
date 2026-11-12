#!/usr/bin/env python3
"""Network records of the nightly for Quality's KPIs (#259, #92, #217).

Writes JSON lines in the format of docs/process/qmp.md ("Network loss"):
one object per line, appended to --out. The nightly (scripts/update-stable.sh)
appends them to network.jsonl in redox-rpi/pi-tests.

  scripts/network-record.py boot --serial-log <file> --start <byte> --pi <ip>
                                 --build <id> --out <file> [--timeout <s>]
      Waits for "redox login:" in the serial log after byte <start>, then
      tries a devsh login every 2 s. Record: kind "boot", devsh_s (seconds
      from the login prompt to the first devsh login, null after 120 s) and
      login_s (seconds from the start of this script, right after the reset
      marker, to the login prompt). Exit 0 when the login prompt came, 1
      when not (no record then: that is a failed boot, not a network loss).

  scripts/network-record.py soak --pi <ip> --build <id> --out <file>
                                 [--minutes 15]
      A ping every 10 s and a devsh login every 60 s. A loss starts with 3
      pings in a row without an answer or 2 failed logins in a row and lasts
      until a ping and a login both work again; at_s is its first failed
      ping or login, down_s lasts to the check that brought it back. Record: kind "soak", minutes,
      pings, pings_lost, logins, logins_failed, losses [{at_s, down_s}]
      (down_s null when the network was not back by the end), and ping:
      "icmp", or "tcp" for an image that does not answer ICMP echo (#177):
      then a "ping" is a TCP connection to the closed port 9, answered by a
      reset. Always exit 0.

The serial log has no timestamps (redox-serial.service is a plain cat of
the port), so "boot" polls it every 0.5 s and takes the time it sees the
prompt.
"""

import argparse
import datetime
import json
import re
import socket
import struct
import sys
import time

DEVSH_WINDOW = 120
SHELL_PROMPT = re.compile(rb"\S+:\S*# $")
ANSI = re.compile(rb"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07")


def read_until(sock, pattern, deadline):
    sock.settimeout(0.2)
    data = b""
    while time.monotonic() < deadline:
        try:
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
        except socket.timeout:
            pass
        if pattern.search(ANSI.sub(b"", data).rstrip(b" ") + b" "):
            return True
    return False


def devsh_login(host, port, timeout=20):
    """Log in through devshd as root, run `echo`, log out. True when the
    shell prompt came back within `timeout` seconds."""
    deadline = time.monotonic() + timeout
    try:
        with socket.create_connection((host, port), timeout=5) as sock:
            if not read_until(sock, re.compile(rb"login: $"), deadline):
                return False
            sock.sendall(b"root\r")
            if not read_until(sock, re.compile(rb"password: $"), deadline):
                return False
            sock.sendall(b"password\r")
            if not read_until(sock, SHELL_PROMPT, deadline):
                return False
            sock.sendall(b"echo net-ok\r")
            ok = read_until(sock, SHELL_PROMPT, deadline)
            try:
                sock.sendall(b"exit\r")
            except OSError:
                pass
            return ok
    except OSError:
        return False


def ping(host, timeout=2.0):
    """One ICMP echo with an unprivileged ICMP socket (Linux, allowed by
    net.ipv4.ping_group_range; the build host has no ping binary). True
    when an echo reply came within `timeout` seconds."""
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_ICMP) as sock:
            sock.settimeout(timeout)
            seq = ping.seq = (getattr(ping, "seq", 0) + 1) & 0xFFFF
            payload = b"redox-rpi network-record"
            # Type 8 (echo), code 0; the kernel fills in the identifier.
            header = struct.pack("!BBHHH", 8, 0, 0, 0, seq)
            csum = checksum(header + payload)
            sock.sendto(struct.pack("!BBHHH", 8, 0, csum, 0, seq) + payload, (host, 0))
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                sock.settimeout(max(0.01, deadline - time.monotonic()))
                data = sock.recv(1024)
                if len(data) >= 8 and data[0] == 0 and struct.unpack("!H", data[6:8])[0] == seq:
                    return True
    except OSError:
        pass
    return False


def tcp_ping(host, timeout=2.0, port=9):
    """For images that do not answer ICMP echo (netstack without the
    icmp-echo experiment, #177): a TCP connection to a closed port. A reset
    (refused) or an accept means the Pi's network answered."""
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except ConnectionRefusedError:
        return True
    except OSError:
        return False


def checksum(data):
    if len(data) % 2:
        data += b"\0"
    total = sum(struct.unpack(f"!{len(data) // 2}H", data))
    total = (total >> 16) + (total & 0xFFFF)
    total += total >> 16
    return ~total & 0xFFFF


def write(out, record):
    record = {"date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d"),
              "build": record.pop("build"), "source": "nightly", **record}
    line = json.dumps(record)
    with open(out, "a") as f:
        f.write(line + "\n")
    print(line)


def boot(args):
    t0 = time.monotonic()
    login_at = None
    while time.monotonic() - t0 < args.timeout:
        with open(args.serial_log, "rb") as f:
            f.seek(args.start)
            if b"redox login:" in f.read().replace(b"\r", b""):
                login_at = time.monotonic()
                break
        time.sleep(0.5)
    if login_at is None:
        print(f"no login prompt within {args.timeout} s", file=sys.stderr)
        return 1
    devsh_s = None
    while time.monotonic() - login_at < DEVSH_WINDOW:
        tried = time.monotonic()
        if devsh_login(args.pi, args.port, timeout=min(20, max(1, DEVSH_WINDOW - (tried - login_at)))):
            devsh_s = round(time.monotonic() - login_at, 1)
            break
        time.sleep(max(0, 2 - (time.monotonic() - tried)))
    write(args.out, {"build": args.build, "kind": "boot", "devsh_s": devsh_s,
                     "login_s": round(login_at - t0, 1)})
    return 0


def soak(args):
    t0 = time.monotonic()
    end = t0 + args.minutes * 60
    pings = pings_lost = logins = logins_failed = 0
    ping_run = login_run = 0
    # Seconds of the first failure in the current run of failures.
    ping_run_at = login_run_at = 0.0
    ping_ok_since = login_ok_since = True
    losses, down_at = [], None
    next_ping = next_login = t0
    target = args.ping_host or args.pi
    # ICMP if the image answers it, else a TCP connection to a closed port.
    method = "icmp" if any(ping(target) for _ in range(3)) else "tcp"
    probe = ping if method == "icmp" else tcp_ping
    print(f"ping method: {method}", file=sys.stderr)
    while True:
        now = time.monotonic()
        if now >= end:
            break
        if now >= next_ping:
            next_ping += 10
            pings += 1
            if probe(target):
                ping_run, ping_ok_since = 0, True
            else:
                pings_lost += 1
                if ping_run == 0:
                    ping_run_at = now - t0
                ping_run += 1
                ping_ok_since = False
        if now >= next_login:
            next_login += 60
            logins += 1
            if devsh_login(args.pi, args.port):
                login_run, login_ok_since = 0, True
            else:
                logins_failed += 1
                if login_run == 0:
                    login_run_at = now - t0
                login_run += 1
                login_ok_since = False
        at = time.monotonic() - t0
        if down_at is None and (ping_run >= 3 or login_run >= 2):
            # The loss starts with the first failure of the run that counts.
            down_at = ping_run_at if ping_run >= 3 else login_run_at
            # A loss lasts until a ping and a login both work after it.
            ping_ok_since = login_ok_since = False
            print(f"loss at {round(down_at)} s", file=sys.stderr)
        elif down_at is not None and ping_ok_since and login_ok_since:
            losses.append({"at_s": round(down_at), "down_s": round(at - down_at)})
            print(f"network back after {round(at - down_at)} s", file=sys.stderr)
            down_at = None
        time.sleep(max(0, min(next_ping, next_login, end) - time.monotonic()))
    if down_at is not None:
        losses.append({"at_s": round(down_at), "down_s": None})
    write(args.out, {"build": args.build, "kind": "soak", "minutes": args.minutes,
                     "pings": pings, "pings_lost": pings_lost, "logins": logins,
                     "logins_failed": logins_failed, "losses": losses, "ping": method})
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="kind", required=True)
    b = sub.add_parser("boot")
    b.add_argument("--serial-log", required=True)
    b.add_argument("--start", type=int, required=True)
    b.add_argument("--timeout", type=int, default=1500)
    s = sub.add_parser("soak")
    s.add_argument("--minutes", type=int, default=15)
    for p in (b, s):
        p.add_argument("--pi", required=True)
        p.add_argument("--port", type=int, default=2323, help="devshd port (default 2323)")
        p.add_argument("--ping-host", help="host to ping (default --pi; for QEMU tests)")
        p.add_argument("--build", required=True)
        p.add_argument("--out", required=True)
    args = parser.parse_args()
    return boot(args) if args.kind == "boot" else soak(args)


if __name__ == "__main__":
    sys.exit(main())
