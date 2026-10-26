#!/usr/bin/env python3
"""Run commands on a Redox system through devshd and print the output.

Usage:
    scripts/devsh.py <host> <port> [command]...

Logs in as root, runs each command, waits for its output and logs out.
Meant for automated tests; for interactive use, connect with
`socat -,raw,echo=0 tcp:<host>:<port>`.

Each step waits for the prompt that follows it (login, password, shell), so
a slow system only makes it slower. A command that does not return to the
shell prompt within COMMAND_TIMEOUT seconds is left behind.
"""

import re
import socket
import sys
import time

STEP_TIMEOUT = 30
COMMAND_TIMEOUT = 60
# Escape sequences and the prompt Ion draws, e.g. "root:~# ".
ANSI = re.compile(rb"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07")
SHELL_PROMPT = re.compile(rb"\S+:\S*# $")


def read_until(sock, pattern, timeout):
    """Collect output until it ends with `pattern` (after stripping escape
    sequences) or `timeout` seconds pass. Returns the raw output."""
    sock.settimeout(0.2)
    data = b""
    end = time.time() + timeout
    while time.time() < end:
        try:
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
        except socket.timeout:
            if pattern.search(ANSI.sub(b"", data).rstrip(b" ") + b" "):
                break
    return data.decode(errors="replace")


def main():
    host, port, commands = sys.argv[1], int(sys.argv[2]), sys.argv[3:]
    sock = socket.create_connection((host, port), timeout=10)
    print(read_until(sock, re.compile(rb"login: $"), STEP_TIMEOUT), end="")
    sock.sendall(b"root\r")
    print(read_until(sock, re.compile(rb"password: $"), STEP_TIMEOUT), end="")
    sock.sendall(b"password\r")
    print(read_until(sock, SHELL_PROMPT, STEP_TIMEOUT), end="")
    for command in commands:
        sock.sendall(command.encode() + b"\r")
        print(read_until(sock, SHELL_PROMPT, COMMAND_TIMEOUT), end="")
    sock.sendall(b"exit\r")
    print(read_until(sock, re.compile(rb"$^"), 1))
    sock.close()


if __name__ == "__main__":
    main()
