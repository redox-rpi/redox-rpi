#!/usr/bin/env python3
"""Write a Wi-Fi settings file to a running Pi through devshd, without
showing the network name or passphrase.

Usage:
    patches/experiments/wifi-sdio/wifi-config.py <host> <port> <file>

<file> is in the format of /etc/net/wifi (`ssid=…`, `psk=…`, one per line)
and stays on the build host, outside the repository (for the Pi tests:
~/.config/redox-rpi/wifi on homenodellm, which the user creates). The
script sends it as `ssid_hex=` and `psk_hex=` lines, so no shell quoting
can change it, and prints only byte counts. The shell echoes what it is
sent; that output is read and thrown away.
"""

import os
import re
import socket
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../../../scripts"))
from devsh import SHELL_PROMPT, STEP_TIMEOUT, read_until  # noqa: E402

TARGET = "/etc/net/wifi"


def hex_lines(text):
    """The settings as hex lines; raises ValueError without an ssid."""
    values = {}
    for line in text.splitlines():
        for key in ("ssid", "psk"):
            if line.startswith(key + "="):
                values[key] = line[len(key) + 1:].encode()
    if "ssid" not in values:
        raise ValueError("no ssid= line")
    if "psk" in values and not 8 <= len(values["psk"]) <= 63:
        raise ValueError(f"passphrase of {len(values['psk'])} characters, needs 8 to 63")
    return {k: v.hex() for k, v in values.items()}, {k: len(v) for k, v in values.items()}


def main():
    host, port, path = sys.argv[1], int(sys.argv[2]), sys.argv[3]
    with open(path) as f:
        lines, sizes = hex_lines(f.read())
    sock = socket.create_connection((host, port), timeout=10)
    read_until(sock, re.compile(rb"login: $"), STEP_TIMEOUT)
    sock.sendall(b"root\r")
    read_until(sock, re.compile(rb"password: $"), STEP_TIMEOUT)
    sock.sendall(b"password\r")
    read_until(sock, SHELL_PROMPT, STEP_TIMEOUT)
    redirect = ">"
    for key, value in lines.items():
        sock.sendall(f"echo {key}_hex={value} {redirect} {TARGET}\r".encode())
        read_until(sock, SHELL_PROMPT, STEP_TIMEOUT)
        redirect = ">>"
    sock.sendall(f"wc -c {TARGET}\r".encode())
    out = read_until(sock, SHELL_PROMPT, STEP_TIMEOUT)
    sock.sendall(b"exit\r")
    sock.close()
    written = re.search(rf"(\d+)\s+{re.escape(TARGET)}", out)
    expected = sum(len(f"{k}_hex={v}\n") for k, v in lines.items())
    names = ", ".join(f"{k} {n} bytes" for k, n in sizes.items())
    print(f"{TARGET}: {names}; {written.group(1) if written else '?'} of {expected} bytes written")
    sys.exit(0 if written and int(written.group(1)) == expected else 1)


if __name__ == "__main__":
    main()
