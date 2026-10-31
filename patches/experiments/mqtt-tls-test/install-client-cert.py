#!/usr/bin/env python3
"""Copy a test client certificate and key into a running Redox over devsh (#115).

Usage:
    install-client-cert.py <host> <port> <cert.pem> <key.pem>

Writes them as /etc/mqtt-client.crt and /etc/mqtt-client.key, line by line
with echo, the key readable only by root. The files come from
client-cert-broker.sh on the build host; they are test files and never go
into the repository or an image config.
"""

import os
import subprocess
import sys

DEVSH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../scripts/devsh.py")


def echo_commands(path, target):
    lines = [line.strip() for line in open(path) if line.strip()]
    for line in lines:
        # PEM lines are base64 and BEGIN/END markers: nothing Ion expands.
        if not all(c.isalnum() or c in "+/= -" for c in line):
            sys.exit(f"{path}: unexpected characters in {line!r}")
    commands = [f"rm -f {target}"]
    commands += [f'echo "{line}" >> {target}' for line in lines]
    return commands


def main():
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    host, port, cert, key = sys.argv[1:]
    commands = echo_commands(cert, "/etc/mqtt-client.crt")
    commands += echo_commands(key, "/etc/mqtt-client.key")
    commands += ["chmod 600 /etc/mqtt-client.key", "ls -l /etc/mqtt-client.crt /etc/mqtt-client.key"]
    out = subprocess.run([sys.executable, DEVSH, host, port, *commands], capture_output=True, text=True).stdout
    print(out[-400:])


if __name__ == "__main__":
    main()
