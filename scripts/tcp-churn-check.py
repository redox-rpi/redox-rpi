#!/usr/bin/env python3
"""Check that Redox frees the sockets of short TCP connections (REQ-NET-4
criterion 2, #494).

Runs on the host. Needs an image with experiment `netdiag` (base 0102:
socket counts in /scheme/netcfg/stats) and devshd.

  tcp-churn-check.py <redox> [--devsh-port 2323] [--count 1000]
                     [--mode out|in|both] [--host-ip <ip>] [--port 12460]
                     [--settle 30]

out: Redox connects <count> times to a server on this host (`nc` in an
     ion loop over devsh); the host sends a line and closes, so Redox
     reads end of input, nc exits and Redox closes its side.
in:  this host connects <count> times to devshd on Redox, reads the login
     prompt and closes; devshd and netstack must free the accepted
     connection.

Before and after each part (after --settle seconds, so TIME-WAIT has
ended) it reads `sockets_tcp` and the `tcp_state_*` lines. The devsh
session that reads them is one connection itself, the same each time.

Pass: every connection went through, and `sockets_tcp` after is at most
2 above before. Exit status 0 on pass, 1 otherwise.
"""
import argparse
import os
import re
import socket
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import devsh  # noqa: E402

t0 = time.time()


def log(*a):
    print(f"[{time.time() - t0:7.1f}s]", *a, flush=True)


def run(host, port, command, timeout):
    """One devsh session: log in, run `command`, return its output."""
    sock = socket.create_connection((host, port), timeout=10)
    try:
        devsh.read_until(sock, re.compile(rb"login: $"), devsh.STEP_TIMEOUT)
        sock.sendall(b"root\r")
        devsh.read_until(sock, re.compile(rb"password: $"), devsh.STEP_TIMEOUT)
        sock.sendall(b"password\r")
        devsh.read_until(sock, devsh.SHELL_PROMPT, devsh.STEP_TIMEOUT)
        sock.sendall(command.encode() + b"\r")
        out = devsh.read_until(sock, devsh.SHELL_PROMPT, timeout)
        sock.sendall(b"exit\r")
        # read_until returns text; drop the escape sequences ion draws
        return re.sub(r"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07", "", out)
    finally:
        sock.close()


def sockets(host, port):
    """sockets_* and tcp_state_* from netstack's stats."""
    out = run(host, port, "cat /scheme/netcfg/stats", 30)
    counts = {}
    for line in out.splitlines():
        m = re.match(r"\s*(sockets_\w+|tcp_state_\w+) (\d+)\s*$", line)
        if m:
            counts[m[1]] = int(m[2])
    if "sockets_tcp" not in counts:
        sys.exit("no sockets_tcp in /scheme/netcfg/stats: image without netdiag base 0102?")
    return counts


def host_ip_towards(redox):
    """This host's address on the way to Redox."""
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect((redox, 9))
        return s.getsockname()[0]
    finally:
        s.close()


def part_out(args):
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("0.0.0.0", args.port))
    srv.listen(64)
    srv.settimeout(1)
    accepted = [0]
    stop = threading.Event()

    def serve():
        while not stop.is_set():
            try:
                c, _ = srv.accept()
            except socket.timeout:
                continue
            with c:
                accepted[0] += 1
                try:
                    c.sendall(b"churn\n")
                except OSError:
                    pass

    threading.Thread(target=serve, daemon=True).start()
    host_ip = args.host_ip or host_ip_towards(args.redox)
    loop = f"for i in 0..{args.count}; nc {host_ip}:{args.port} > /scheme/null; end; echo churn-done"
    log(f"out: Redox connects {args.count} times to {host_ip}:{args.port}")
    out = run(args.redox, args.devsh_port, loop, max(120, args.count))
    stop.set()
    srv.close()
    done = "churn-done" in out
    log(f"out: {accepted[0]} of {args.count} accepted" + ("" if done else ", the loop did not finish"))
    return accepted[0] == args.count and done


def part_in(args):
    ok = 0
    log(f"in: {args.count} connections to devshd on {args.redox}:{args.devsh_port}")
    for _ in range(args.count):
        try:
            with socket.create_connection((args.redox, args.devsh_port), timeout=10) as c:
                devsh.read_until(c, re.compile(rb"login: $"), devsh.STEP_TIMEOUT)
                ok += 1
        except OSError:
            pass
    log(f"in: {ok} of {args.count} reached the login prompt")
    return ok == args.count


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("redox")
    ap.add_argument("--devsh-port", type=int, default=2323)
    ap.add_argument("--count", type=int, default=1000)
    ap.add_argument("--mode", choices=["out", "in", "both"], default="both")
    ap.add_argument("--host-ip", help="this host as Redox reaches it (QEMU: 10.0.2.2)")
    ap.add_argument("--port", type=int, default=12460, help="host port for --mode out")
    ap.add_argument("--settle", type=float, default=30)
    args = ap.parse_args()

    results = []
    parts = ["out", "in"] if args.mode == "both" else [args.mode]
    for part in parts:
        before = sockets(args.redox, args.devsh_port)
        log(f"{part}: before {before}")
        went = part_out(args) if part == "out" else part_in(args)
        time.sleep(args.settle)
        after = sockets(args.redox, args.devsh_port)
        log(f"{part}: after {after}")
        grown = after["sockets_tcp"] - before["sockets_tcp"]
        ok = went and grown <= 2
        log(f"{part}: {'pass' if ok else 'fail'}: sockets_tcp {before['sockets_tcp']} -> "
            f"{after['sockets_tcp']} ({grown:+d})")
        results.append(ok)
    ok = all(results)
    print(f"result: {'pass' if ok else 'fail'}")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
