#!/usr/bin/env python3
"""Write a Wi-Fi settings file to a running Pi, without showing the network
name or passphrase.

Usage:
    patches/experiments/wifi-sdio/wifi-config.py <host> <port> <file>
    patches/experiments/wifi-sdio/wifi-config.py --serial <tty> <serial log> <file>

<file> is in the format of /etc/net/wifi (`ssid=…`, `psk=…`, one per line)
and stays on the build host, outside the repository (for the Pi tests:
~/.config/redox-rpi/wifi on homenodellm, which the user creates). The
script prints only byte counts, never the values.

Over devshd (<host> <port>) it sends the file as `ssid_hex=` and `psk_hex=`
lines, so no shell quoting can change it; the shell echoes what it is sent,
and that output is read and thrown away.

Over the serial console (--serial) a root shell must be at its prompt. The
script runs `wifi-set` on the Pi, which reads both values with the echo
switched off, and sends each value only after its prompt has appeared in
the serial log: the logger keeps reading the console, this script only
writes to the tty and reads the log file. Without the prompt it sends
nothing. At the end it checks that neither value reached the log.
"""

import os
import re
import socket
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../../../scripts"))

TARGET = "/etc/net/wifi"
SSID_PROMPT = b"wifi-set: network name (not shown): "
PSK_PROMPT = b"wifi-set: passphrase (not shown, empty for an open network): "
WRITTEN = re.compile(rb"wifi-set: \S+ written: network name (\d+) bytes, passphrase (\d+) bytes")
WAIT = 20


def read_settings(text):
    """The values as bytes; raises ValueError for a bad file."""
    values = {}
    for line in text.splitlines():
        for key in ("ssid", "psk"):
            if line.startswith(key + "="):
                values[key] = line[len(key) + 1:].encode()
    if "ssid" not in values:
        raise ValueError("no ssid= line")
    if not 1 <= len(values["ssid"]) <= 32:
        raise ValueError(f"network name of {len(values['ssid'])} bytes, needs 1 to 32")
    if "psk" in values and not 8 <= len(values["psk"]) <= 63:
        raise ValueError(f"passphrase of {len(values['psk'])} characters, needs 8 to 63")
    if any(b in v for v in values.values() for b in b"\r\n"):
        raise ValueError("line break inside a value")
    return values


def over_devsh(host, port, values):
    from devsh import SHELL_PROMPT, STEP_TIMEOUT, read_until  # noqa: E402

    lines = {k: v.hex() for k, v in values.items()}
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
    names = ", ".join(f"{k} {len(v)} bytes" for k, v in values.items())
    print(f"{TARGET}: {names}; {written.group(1) if written else '?'} of {expected} bytes written")
    return 0 if written and int(written.group(1)) == expected else 1


class Log:
    """Reads what the logger appends to the serial log after `start`."""

    def __init__(self, path):
        self.path = path
        self.start = os.path.getsize(path)

    def since(self):
        with open(self.path, "rb") as f:
            f.seek(self.start)
            return f.read()

    def wait_for(self, pattern, timeout=WAIT):
        end = time.time() + timeout
        while time.time() < end:
            text = self.since()
            found = pattern.search(text) if hasattr(pattern, "search") else (pattern in text)
            if found:
                return text, found
            time.sleep(0.2)
        return self.since(), None


def send(fd, data):
    # Slowly: the Pi's serial input has lost characters in fast bursts.
    for i in range(0, len(data), 8):
        os.write(fd, data[i:i + 8])
        time.sleep(0.02)


def over_serial(tty, log_path, values):
    log = Log(log_path)
    fd = os.open(tty, os.O_WRONLY | os.O_NOCTTY)
    try:
        send(fd, b"wifi-set\r")
        _, found = log.wait_for(SSID_PROMPT)
        if not found:
            print("wifi-set did not ask for the network name; nothing sent")
            return 1
        send(fd, values["ssid"] + b"\r")
        _, found = log.wait_for(PSK_PROMPT)
        if not found:
            print("wifi-set did not ask for the passphrase; passphrase not sent")
            send(fd, b"\x03")
            return 1
        send(fd, values.get("psk", b"") + b"\r")
        text, found = log.wait_for(WRITTEN)
    finally:
        os.close(fd)
    leaked = any(v in text for v in values.values())
    sizes = (len(values["ssid"]), len(values.get("psk", b"")))
    if not found:
        print(f"{TARGET}: no confirmation from wifi-set; values in the log: {'YES' if leaked else 'no'}")
        return 1
    got = (int(found.group(1)), int(found.group(2)))
    print(
        f"{TARGET}: network name {got[0]} of {sizes[0]} bytes, passphrase {got[1]} of {sizes[1]} bytes; "
        f"values in the log: {'YES' if leaked else 'no'}"
    )
    return 0 if got == sizes and not leaked else 1


def main():
    args = sys.argv[1:]
    if len(args) == 4 and args[0] == "--serial":
        _, tty, log_path, path = args
    elif len(args) == 3:
        host, port, path = args
    else:
        print(__doc__.strip().split("\n\n")[1], file=sys.stderr)
        return 2
    with open(path) as f:
        values = read_settings(f.read())
    if args[0] == "--serial":
        return over_serial(tty, log_path, values)
    return over_devsh(host, int(port), values)


if __name__ == "__main__":
    sys.exit(main())
