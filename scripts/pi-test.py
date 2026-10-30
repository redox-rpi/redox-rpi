#!/usr/bin/env python3
"""Standard tests for a redox-rpi image on a real Pi, through devshd.

Usage:
    scripts/pi-test.py <host> [--port 2323] [--manifest <file>] [--starts 300]
                       [--report docs/test-reports.md] [--note <text>]

Checks, in this order:
  version   /etc/redox-rpi-release is readable and names a build ID (and the
            release name, if it has one)
  manifest  /etc/redox-rpi-manifest matches --manifest (build ID, libc hash)
  ps        ps runs and lists processes (output piped)
  ps-tty    ps writes its list straight to the terminal
  sdcard    a file written to /scheme/data (SD card) reads back the same
  starts    dynamically linked programs (ls, pwd) start --starts times;
            every start that fails is counted

Prints a summary and, with --report, appends it to that Markdown file.
Exits with 1 if a check failed.
"""

import argparse
import datetime
import hashlib
import re
import socket
import sys
import time

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from devsh import ANSI, SHELL_PROMPT, STEP_TIMEOUT, read_until  # noqa: E402

BEGIN, END = "PITEST-BEGIN", "PITEST-END"


class Shell:
    def __init__(self, host, port):
        self.host, self.port = host, port
        self.login()

    def login(self):
        self.sock = socket.create_connection((self.host, self.port), timeout=10)
        read_until(self.sock, re.compile(rb"login: $"), STEP_TIMEOUT)
        self.sock.sendall(b"root\r")
        read_until(self.sock, re.compile(rb"password: $"), STEP_TIMEOUT)
        self.sock.sendall(b"password\r")
        read_until(self.sock, SHELL_PROMPT, STEP_TIMEOUT)

    def run(self, command, timeout=120):
        """Run one Ion command line and return its output lines. A command
        that gives no output is tried once more in a new session: a devsh
        session sometimes stops answering (#11)."""
        try:
            return self.run_once(command, timeout)
        except (RuntimeError, OSError) as e:
            print(f"retry in a new session: {str(e).splitlines()[0]}", flush=True)
            try:
                self.sock.close()
            except OSError:
                pass
            self.login()
            return self.run_once(command, timeout)

    def run_once(self, command, timeout):
        """The output sits between two marker lines; the echo of the typed
        command never matches them exactly."""
        line = f"echo {BEGIN}; {command}; echo {END}"
        self.sock.sendall(line.encode() + b"\r")
        # Ion redraws its prompt while the line is typed, so wait for the
        # end marker on a line of its own instead of the next prompt.
        end_line = re.compile(rb"[\r\n]" + END.encode() + rb"\r?\n")
        data, deadline = b"", time.time() + timeout
        self.sock.settimeout(1)
        begin_line = re.compile(rb"[\r\n]" + BEGIN.encode() + rb"\r?\n")
        while time.time() < deadline and not end_line.search(ANSI.sub(b"", data).replace(b"\x00", b"")):
            try:
                chunk = self.sock.recv(65536)
            except socket.timeout:
                # Ion drops the rest of the line, end marker included, when
                # a command in it fails (a redirect, for example): stop at
                # the prompt that follows the output.
                text = ANSI.sub(b"", data).replace(b"\x00", b"")
                after = text[begin_line.search(text).end():] if begin_line.search(text) else b""
                if after and SHELL_PROMPT.search(after.rstrip(b" ") + b" "):
                    break
                continue
            if not chunk:
                break
            data += chunk
        read_until(self.sock, SHELL_PROMPT, 5)
        text = ANSI.sub(b"", data).decode(errors="replace")
        lines = [l.replace("\x00", "").strip() for l in text.replace("\r", "\n").split("\n")]
        if BEGIN not in lines:
            raise RuntimeError(f"no output for: {command}\n{text[-1500:]!r}")
        stop = lines.index(END) if END in lines else len(lines)
        out = [l for l in lines[lines.index(BEGIN) + 1 : stop] if "root:" not in l]
        # Ion marks output without a final newline with a "⏎" line.
        return [l for l in out if l and not l.startswith("⏎")]

    def close(self):
        self.sock.sendall(b"exit\r")
        self.sock.close()


def parse_manifest(path):
    values = {}
    for line in open(path):
        m = re.match(r'^"?([^"=\s]+)"?\s*=\s*"([^"]*)"', line)
        if m:
            values.setdefault(m.group(1), m.group(2))
    return values


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("host")
    ap.add_argument("--port", type=int, default=2323)
    ap.add_argument("--manifest")
    ap.add_argument("--starts", type=int, default=300)
    ap.add_argument("--report")
    ap.add_argument("--note", default="")
    args = ap.parse_args()

    results = []  # (check, ok, detail)
    info = {}

    def check(name, ok, detail):
        results.append((name, ok, detail))
        print(f"{'PASS' if ok else 'FAIL'} {name}: {detail}", flush=True)

    sh = Shell(args.host, args.port)
    try:
        release = dict(
            l.split("=", 1) for l in sh.run("cat /etc/redox-rpi-release") if "=" in l
        )
        info["build_id"] = release.get("BUILD_ID", release.get("VERSION", "?")).strip('"')
        info["built"] = release.get("BUILD", "?").strip('"')
        # Images built before the short build IDs name the experiments in
        # the ID itself and have no EXPERIMENTS line.
        info["experiments"] = release.get("EXPERIMENTS", "").strip('"')
        # Release candidates and releases also have RELEASE (v0.3.0-rc2).
        info["release"] = release.get("RELEASE", "").strip('"')
        title = f"{info['release']} ({info['build_id']})" if info["release"] else info["build_id"]
        check("version", info["build_id"] != "?", title)

        libc = sh.run("sha256sum /usr/lib/libc.so")
        info["libc"] = libc[0].split()[0] if libc else "?"
        if args.manifest:
            m = parse_manifest(args.manifest)
            ok = m.get("build_id") == info["build_id"] and m.get("/usr/lib/libc.so") == info["libc"]
            check("manifest", ok, f"build ID and libc.so hash {'match' if ok else 'differ from'} {args.manifest.rsplit('/', 1)[-1]}")
        else:
            on_pi = sh.run("cat /etc/redox-rpi-manifest")
            ok = any(l.startswith("build_id") for l in on_pi)
            check("manifest", ok, "present in the image" if ok else "missing in the image")

        ps = sh.run("ps | wc -l")
        count = int(ps[0]) - 1 if ps and ps[0].isdigit() else 0
        check("ps", count > 5, f"{count} processes (piped)")
        tty = sh.run("ps; echo rc=$?")
        ok = tty[-1:] == ["rc=0"] and len(tty) > 5
        check("ps-tty", ok, f"{len(tty) - 1} lines straight to the terminal, {tty[-1] if tty else 'no output'}")

        token = hashlib.sha256(str(time.time()).encode()).hexdigest()[:16]
        out = sh.run(
            f"echo {token} > /scheme/data/pi-test.txt; cat /scheme/data/pi-test.txt; rm /scheme/data/pi-test.txt"
        )
        check("sdcard", token in out, "write, read back, delete on /scheme/data" if token in out else f"got {'; '.join(out[:2]) or 'nothing'}")

        failures, errors, done = 0, set(), 0
        per_run = 25  # iterations per command line, 2 starts each
        while done < args.starts:
            n = min(per_run, (args.starts - done + 1) // 2)
            out = sh.run(
                f"for i in 1..={n}; ls / > /dev/null || echo FAILED ls; pwd > /dev/null || echo FAILED pwd; end",
                timeout=300,
            )
            for l in out:
                if l.startswith("FAILED"):
                    failures += 1
                elif l and not l.startswith("/"):
                    errors.add(l[:100])
            done += 2 * n
        detail = f"{failures} of {done} failed"
        if errors:
            detail += "; messages: " + "; ".join(f'"{e}"' for e in sorted(errors)[:5])
        check("starts", failures == 0, detail)
        info["starts"] = (failures, done)
    finally:
        sh.close()

    passed = all(ok for _, ok, _ in results)
    if args.report:
        date = datetime.date.today().isoformat()
        with open(args.report, "a") as f:
            name = f"{info['release']}, " if info.get("release") else ""
            f.write(f"\n## {date}: {name}`{info.get('build_id', '?')}`\n\n")
            f.write(f"Built {info.get('built', '?')}, `libc.so` `{info.get('libc', '?')[:16]}…`, "
                    f"real Pi 3B+ over netboot.")
            if info.get("experiments"):
                f.write(f" Experiments: {', '.join(f'`{x}`' for x in info['experiments'].split())}.")
            if args.note:
                f.write(f" {args.note}")
            f.write("\n\n| Check | Result | Detail |\n|---|---|---|\n")
            for name, ok, detail in results:
                detail = detail.replace("|", "\\|")
                f.write(f"| {name} | {'pass' if ok else '**fail**'} | {detail} |\n")
    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
