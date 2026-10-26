#!/usr/bin/env python3
"""Run commands on a Redox system through devshd and print the output.

Usage:
    scripts/devsh.py <host> <port> [command]...

Logs in as root, runs each command, waits for its output and logs out.
Meant for automated tests; for interactive use, connect with
`socat -,raw,echo=0 tcp:<host>:<port>`.
"""

import socket
import sys
import time


def read_for(sock, seconds):
    """Collect everything the other side sends within `seconds`."""
    sock.settimeout(0.2)
    data = b""
    end = time.time() + seconds
    while time.time() < end:
        try:
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
        except socket.timeout:
            pass
    return data.decode(errors="replace")


def main():
    host, port, commands = sys.argv[1], int(sys.argv[2]), sys.argv[3:]
    sock = socket.create_connection((host, port), timeout=10)
    print(read_for(sock, 2), end="")
    for line in ("root", "password"):
        sock.sendall(line.encode() + b"\r")
        print(read_for(sock, 2), end="")
    for command in commands:
        sock.sendall(command.encode() + b"\r")
        print(read_for(sock, 4), end="")
    sock.sendall(b"exit\r")
    print(read_for(sock, 1))
    sock.close()


if __name__ == "__main__":
    main()
