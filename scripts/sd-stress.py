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
  cycle   restart the Pi (echo reset > /scheme/sys/kstop), wait for the new
          boot and for the filesystem of --dir, verify, write one more file
          of --cycle-mib MiB; --cycles times

Deletes the test directory at the end unless --keep is given. Prints a
summary and, with --report, appends it to that Markdown file. Exits with 1
if a file was missing or changed.

Program starts on the Pi can still fail now and then (T12), so a command
whose output is incomplete runs again, up to three times.

Give --serial-log, a file that something else (for example
`cat /dev/ttyUSB0 >> logs/serial.log`) keeps appending the console to:
after each reset the script then waits for the kernel's `kreset` and the
new boot's `redox login:` there. devshd alone does not show a restart:
it can answer during the shutdown, when the filesystem is already gone.
Without --serial-log it falls back to waiting until devshd stops
answering. Either way it then waits until the filesystem of --dir is
mounted (devshd starts before it) and stops with an error if the Pi does
not reset, come back or mount it in time.

devshd sometimes stops answering after a restart; --serial (with
--serial-log) runs everything over the serial console instead: it types on
the tty and reads the answers from the log.
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
# From the reset command to the kernel's "kreset" on the console.
RESET_TIMEOUT = 120
# From the new boot's login prompt to the filesystem of --dir. A card
# with much data written before an unclean reset can take minutes.
MOUNT_TIMEOUT = 600


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
    {path: hash}. Ion expands the globs, so the command lines stay short.
    A glob that prints nothing (a failed program start) is tried again."""
    result = {}
    for pattern in patterns:
        for _ in range(tries):
            found = {}
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


def wait_for_boot(log_path, offset, log):
    """After a reset command: wait for the kernel's "kreset" on the console
    after `offset` (the log size before the command), then for the new
    boot's login prompt after it."""
    start = time.time()
    reset_at = None
    while True:
        with open(log_path, "rb") as f:
            f.seek(offset)
            text = f.read()
        if reset_at is None:
            pos = text.find(b"kreset")
            if pos >= 0:
                reset_at = pos
                log(f"reset after {time.time() - start:.0f} s")
            elif time.time() - start > RESET_TIMEOUT:
                raise SystemExit(f"the Pi did not reset within {RESET_TIMEOUT} s (no kreset on the console)")
        if reset_at is not None and b"redox login:" in text[reset_at:]:
            log(f"login prompt after {time.time() - start:.0f} s")
            return
        if time.time() - start > RESET_TIMEOUT + BOOT_TIMEOUT:
            raise SystemExit(f"no login prompt within {BOOT_TIMEOUT} s after the reset")
        time.sleep(2)


def connect_devshd(host, port):
    """Log in to devshd, which starts a few seconds after the login prompt."""
    start = time.time()
    while True:
        try:
            return Shell(host, port)
        except OSError:
            if time.time() - start > BOOT_TIMEOUT:
                raise SystemExit(f"devshd did not answer within {BOOT_TIMEOUT} s after the login prompt")
            time.sleep(5)


def wait_for_mount(sh, connect, path, log):
    """Wait until the scheme of `path` (/scheme/data for /scheme/data/stress)
    exists: devshd starts before the filesystem on the card is mounted.
    Each check after the first logs in again, so a shell that stopped
    answering does not look like a missing filesystem. Returns the shell
    that saw it."""
    parts = path.split("/")
    if len(parts) < 3 or parts[1] != "scheme":
        return sh
    scheme = parts[2]
    start = time.time()
    answered = silent = 0
    while True:
        out = []
        try:
            if sh is None:
                sh = connect()
            out = sh.run("ls /scheme", timeout=30)
        except OSError:
            pass
        if any(scheme in line.split() for line in out):
            log(f"/scheme/{scheme} there after {time.time() - start:.0f} s more")
            return sh
        if any("null" in line.split() for line in out):
            answered += 1
        else:
            silent += 1
        if time.time() - start > MOUNT_TIMEOUT:
            raise SystemExit(
                f"/scheme/{scheme} not mounted within {MOUNT_TIMEOUT} s "
                f"({answered} checks listed /scheme without it, {silent} got no answer)")
        if sh is not None:
            sh.close()
            sh = None
        time.sleep(10)


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

    def reconnect(offset):
        if args.serial_log:
            wait_for_boot(args.serial_log, offset, log)
            if args.serial:
                time.sleep(10)
                sh = SerialShell(args.serial, args.serial_log)
            else:
                sh = connect_devshd(args.host, args.port)
        else:
            sh = wait_for_devshd(args.host, args.port, log)
        return wait_for_mount(sh, connect, d, log)

    lines = []

    def log(text):
        print(text, flush=True)
        lines.append(text)

    d = args.dir
    sh = connect()
    build = next((l.split("=", 1)[1].strip('"') for l in sh.run("cat /etc/redox-rpi-release") if l.startswith("BUILD_ID=")), "?")
    # ps cuts long names ("/scheme/initfs/bin/bcm2835-sdhci"), so match a prefix.
    driver = "sdhost" if any("sdhos" in l for l in sh.run("ps | grep sdh")) else "sdhcid"
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

    # One glob per first digit: at most about 110 files per sha256sum (750
    # in one command hung the Pi on 2026-10-28).
    patterns = [f"{d}/*.bin"] + [f"{d}/{p}{n}*" for p in "st" for n in range(1, 10)]
    reference = hashes(sh, patterns)
    expected = 1 + (args.small - quarter) + quarter
    log(f"reference: {len(reference)} hashes ({expected - len(reference)} files not written, start failures)")

    failed = []
    missing, changed = verify(sh, reference, patterns, log)
    failed += missing + changed
    results = [("after writing", len(missing), len(changed))]

    for cycle in range(1, args.cycles + 1):
        log(f"cycle {cycle}: restart")
        offset = os.path.getsize(args.serial_log) if args.serial_log else 0
        sh.run("echo reset > /scheme/sys/kstop", timeout=5)
        sh.close()
        sh = reconnect(offset)
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
