#!/usr/bin/env python3
"""Stress test for the SD card driver on a real Pi, through devshd.

Usage:
    scripts/sd-stress.py <host> [--port 2323] [--dir /scheme/data/stress]
                         [--serial /dev/ttyUSB0 --serial-log <file>]
                         [--big-mib 256] [--small 1000] [--cycles 5]
                         [--cycle-mib 32] [--report docs/test-reports.md]
                         [--note <text>] [--keep]

Writes to the card, then restarts the Pi several times and checks that
every file still has its hash:

  write   one large file of random data (--big-mib MiB, 1 MiB per call,
          conv=fsync), --small files of 4 KiB, then a quarter of the small
          files deleted and as many new ones written (fragmentation)
  verify  the SHA-256 of every file against the hashes taken right after
          writing; the reference stays on this computer, not on the card
  cycle   restart the Pi (echo reset > /scheme/sys/kstop), wait for devshd,
          verify, write one more file of --cycle-mib MiB; --cycles times

Deletes the test directory at the end unless --keep is given. Prints a
summary and, with --report, appends it to that Markdown file. Exits with 1
if a file was missing or changed.

Program starts on the Pi can still fail now and then (T12), so a command
whose output is incomplete runs again, up to three times.

devshd sometimes stops answering after a restart; --serial runs everything
over the serial console instead. It types on the tty and reads the answers
from --serial-log, a file that something else (for example
`cat /dev/ttyUSB0 >> logs/serial.log`) keeps appending the console to.
"""

import argparse
import datetime
import os
import re
import socket
import sys
import time

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from devsh import ANSI, SHELL_PROMPT, STEP_TIMEOUT, read_until  # noqa: E402

HASH_LINE = re.compile(r"^([0-9a-f]{64})\s+(\S+)$")
COPIED = re.compile(r"copied, ([0-9.]+) s, (.+)$")
BOOT_TIMEOUT = 300


class Shell:
    def __init__(self, host, port):
        self.sock = socket.create_connection((host, port), timeout=10)
        read_until(self.sock, re.compile(rb"login: $"), STEP_TIMEOUT)
        self.sock.sendall(b"root\r")
        read_until(self.sock, re.compile(rb"password: $"), STEP_TIMEOUT)
        self.sock.sendall(b"password\r")
        read_until(self.sock, SHELL_PROMPT, STEP_TIMEOUT)

    def run(self, command, timeout=120):
        """Run one Ion command line and return its output lines, without
        the echo of the command and the prompts."""
        self.sock.sendall(command.encode() + b"\r")
        raw = read_until(self.sock, SHELL_PROMPT, timeout).encode()
        text = ANSI.sub(b"", raw).replace(b"\x00", b"").decode(errors="replace")
        lines = [l.strip() for l in text.replace("\r", "\n").split("\n")]
        return [l for l in lines if l and "root:" not in l and not l.startswith("⏎")]

    def close(self):
        try:
            self.sock.sendall(b"exit\r")
            self.sock.close()
        except OSError:
            pass


class SerialShell:
    """The same as Shell, over the serial console."""

    PROMPT = re.compile(r"root:\S*# ?$")

    def __init__(self, tty, log_path):
        self.tty, self.log_path = tty, log_path
        self.type("")
        if not self.wait_for(re.compile(r"(login|password): ?$|root:\S*# ?$"), 10):
            self.type("")
            self.wait_for(re.compile(r"login: ?$"), 10)
        if not self.PROMPT.search(self.tail()):
            self.type("root")
            self.wait_for(re.compile(r"password: ?$"), STEP_TIMEOUT)
            self.type("password")
            self.wait_for(self.PROMPT, STEP_TIMEOUT)

    def type(self, text):
        with open(self.tty, "wb", buffering=0) as tty:
            for ch in text.encode() + b"\r":
                tty.write(bytes([ch]))
                time.sleep(0.01)

    def text_since(self, offset):
        with open(self.log_path, "rb") as f:
            f.seek(offset)
            data = f.read()
        return ANSI.sub(b"", data).replace(b"\x00", b"").decode(errors="replace")

    def tail(self):
        """The last line on the console, skipping kernel debug lines that
        may come after a prompt."""
        size = os.path.getsize(self.log_path)
        text = self.text_since(max(0, size - 2000))
        noise = ("kernel::", "removing handle", "[src/")
        lines = [l for l in re.split(r"[\r\n]", text) if l.strip() and not any(n in l for n in noise)]
        return lines[-1] if lines else ""

    def wait_for(self, pattern, timeout):
        end = time.time() + timeout
        while time.time() < end:
            if pattern.search(self.tail().rstrip(" ") + " "):
                return True
            time.sleep(0.5)
        return False

    def run(self, command, timeout=120):
        start = os.path.getsize(self.log_path)
        self.type(command)
        time.sleep(1)
        end = time.time() + timeout
        size = -1
        while time.time() < end:
            now = os.path.getsize(self.log_path)
            # Done when the output stopped growing and ends with a prompt.
            if now == size and self.PROMPT.search(self.tail().rstrip(" ") + " "):
                break
            size = now
            time.sleep(1)
        text = self.text_since(start)
        lines = [l.strip() for l in text.replace("\r", "\n").split("\n")]
        return [l for l in lines if l and "root:" not in l and "call_fdread" not in l and not l.startswith("⏎")]

    def close(self):
        pass


def hashes(sh, patterns, tries=3):
    """SHA-256 of the files matching the glob `patterns` on the Pi, as
    {path: hash}. Ion expands the globs, so the command lines stay short."""
    result = {}
    for _ in range(tries):
        found = {}
        for pattern in patterns:
            for line in sh.run("sha256sum " + pattern, timeout=600):
                match = HASH_LINE.match(line)
                if match:
                    found[match.group(2)] = match.group(1)
        result.update(found)
        if found:
            break
    return result


def write_big(sh, path, mib):
    out = sh.run(f"dd if=/scheme/rand of={path} bs=1048576 count={mib} conv=fsync", timeout=60 + mib * 2)
    speed = next((COPIED.search(l).group(2) for l in out if COPIED.search(l)), "?")
    return speed


def verify(sh, reference, patterns, log):
    found = hashes(sh, patterns)
    if any(p not in found for p in reference):
        # A sha256sum that failed to start loses a whole glob: once more.
        found.update(hashes(sh, patterns))
    missing = [p for p in reference if p not in found]
    changed = [p for p in reference if p in found and found[p] != reference[p]]
    log(f"verify: {len(reference)} files, {len(missing)} missing, {len(changed)} changed")
    for path in (missing + changed)[:10]:
        log(f"  {'missing' if path in missing else 'changed'}: {path}")
    return missing, changed


def wait_for_devshd(host, port, log):
    """After a reset: wait until devshd stops answering, then until it is
    back and its login prompt appears."""
    start = time.time()
    while time.time() - start < 60:
        try:
            socket.create_connection((host, port), timeout=2).close()
            time.sleep(2)
        except OSError:
            break
    while time.time() - start < BOOT_TIMEOUT:
        try:
            sh = Shell(host, port)
            log(f"back after {time.time() - start:.0f} s")
            return sh
        except OSError:
            time.sleep(5)
    raise SystemExit(f"the Pi did not come back within {BOOT_TIMEOUT} s")


def wait_for_serial(tty, log_path, log):
    """After a reset: wait for the new boot's login prompt on the console
    (typing earlier would reach the bootloader), then log in."""
    start, offset = time.time(), os.path.getsize(log_path)
    while time.time() - start < BOOT_TIMEOUT:
        with open(log_path, "rb") as f:
            f.seek(offset)
            text = f.read()
        if b"devshd: listening" in text or b"redox login:" in text:
            time.sleep(10)
            log(f"back after {time.time() - start:.0f} s")
            return SerialShell(tty, log_path)
        time.sleep(5)
    raise SystemExit(f"the Pi did not come back within {BOOT_TIMEOUT} s")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("host")
    parser.add_argument("--port", type=int, default=2323)
    parser.add_argument("--dir", default="/scheme/data/stress")
    parser.add_argument("--big-mib", type=int, default=256)
    parser.add_argument("--small", type=int, default=1000)
    parser.add_argument("--cycles", type=int, default=5)
    parser.add_argument("--cycle-mib", type=int, default=32)
    parser.add_argument("--report")
    parser.add_argument("--note", default="")
    parser.add_argument("--keep", action="store_true")
    parser.add_argument("--serial", help="the Pi's console tty, instead of devshd")
    parser.add_argument("--serial-log", help="file the console output is appended to")
    args = parser.parse_args()
    if args.serial and not args.serial_log:
        parser.error("--serial needs --serial-log")

    def connect():
        return SerialShell(args.serial, args.serial_log) if args.serial else Shell(args.host, args.port)

    def reconnect():
        if args.serial:
            return wait_for_serial(args.serial, args.serial_log, log)
        return wait_for_devshd(args.host, args.port, log)

    lines = []

    def log(text):
        print(text, flush=True)
        lines.append(text)

    d = args.dir
    sh = connect()
    build = next((l.split("=", 1)[1].strip('"') for l in sh.run("cat /etc/redox-rpi-release") if l.startswith("BUILD_ID=")), "?")
    driver = "sdhost" if any("bcm2835-sdhostd" in l for l in sh.run("ps | grep sdh")) else "sdhcid"
    log(f"build {build}, driver {driver}")

    sh.run(f"rm -r {d}", timeout=300)
    sh.run(f"mkdir {d}")
    begin = time.time()
    big_speed = write_big(sh, f"{d}/big.bin", args.big_mib)
    log(f"write: {args.big_mib} MiB at {big_speed}")

    t = time.time()
    sh.run(f"for i in 1..={args.small}; dd if=/scheme/rand of={d}/s$i bs=4096 count=1 ^> /dev/null; end",
           timeout=60 + args.small)
    quarter = args.small // 4
    sh.run(f"for i in 1..={quarter}; rm {d}/s$i; end", timeout=60 + quarter)
    sh.run(f"for i in 1..={quarter}; dd if=/scheme/rand of={d}/t$i bs=4096 count=1 ^> /dev/null; end",
           timeout=60 + quarter)
    log(f"write: {args.small} small files, {quarter} deleted, {quarter} new, {time.time() - t:.0f} s")

    patterns = [f"{d}/*.bin", f"{d}/s*", f"{d}/t*"]
    reference = hashes(sh, patterns)
    expected = 1 + (args.small - quarter) + quarter
    log(f"reference: {len(reference)} hashes ({expected - len(reference)} files not written, start failures)")

    failed = []
    missing, changed = verify(sh, reference, patterns, log)
    failed += missing + changed
    results = [("after writing", len(missing), len(changed))]

    for cycle in range(1, args.cycles + 1):
        log(f"cycle {cycle}: restart")
        sh.run("echo reset > /scheme/sys/kstop", timeout=5)
        sh.close()
        sh = reconnect()
        missing, changed = verify(sh, reference, patterns, log)
        failed += missing + changed
        results.append((f"restart {cycle}", len(missing), len(changed)))
        path = f"{d}/c{cycle}.bin"
        speed = write_big(sh, path, args.cycle_mib)
        reference.update(hashes(sh, [path]))
        log(f"cycle {cycle}: wrote {args.cycle_mib} MiB at {speed}")

    if not args.keep:
        sh.run(f"rm -r {d}", timeout=300)
    sh.close()
    minutes = (time.time() - begin) / 60

    summary = [
        "",
        f"## {datetime.date.today()}: `{build}`, SD card stress test ({driver})",
        "",
        f"`scripts/sd-stress.py`: {args.big_mib} MiB file ({big_speed}), {args.small} files of 4 KiB with "
        f"{quarter} replaced, then {args.cycles} restarts, each followed by a check of every file and "
        f"{args.cycle_mib} MiB more; {len(reference)} files at the end, {minutes:.0f} minutes. {args.note}".rstrip(),
        "",
        "| Check | Missing | Changed |",
        "|---|---|---|",
    ]
    summary += [f"| {name} | {m} | {c} |" for name, m, c in results]
    summary.append("")
    summary.append(f"Result: **{'fail' if failed else 'pass'}**.")
    print("\n".join(summary))
    if args.report:
        with open(args.report, "a") as f:
            f.write("\n".join(summary) + "\n")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
