#!/usr/bin/env python3
"""Standard tests for a redox-rpi image on a real Pi, through devshd.

Usage:
    scripts/pi-test.py <host> [--port 2323] [--manifest <file>] [--starts 300]
                       [--serial-log <file>] [--serial-lines 15]
                       [--crash-log <file.jsonl>]
                       [--report <file>] [--note <text>]
                       [--extended] [--tcp-host <ip>] [--tcp-mib 10]

Checks, in this order:
  version   /etc/redox-rpi-release is readable and names a build ID (and the
            release name, if it has one)
  manifest  /etc/redox-rpi-manifest matches --manifest (build ID, libc hash)
  ps        ps runs and lists processes (output piped)
  ps-tty    ps writes its list straight to the terminal
  sdcard    a file written to /scheme/data (SD card) reads back the same
  starts    dynamically linked programs (ls, pwd) start --starts times;
            for every start that fails, the report lists the program, its
            exit status, what it printed, and the crash dump lines the
            serial console logged meanwhile (with --serial-log, e.g.
            ~/redox-rpi/logs/serial.log, or images/qrun.log under QEMU)

With --extended (the nightly, #258), also:
  audio     /scheme/audiohw/stats: measured_rate_hz within 2 % of 44 100,
            dma_faults and pwm_errors 0; `tone -s 3 > /scheme/audio` takes
            2.7-3.3 s (devsh overhead subtracted) and audiod still runs
  time      the clock was set by NTP (/tmp/timesync.synced from timesync)
            and is within 120 s of this host's clock
  tcp       --tcp-mib MiB (default 10) from this host over TCP with nc into
            /tmp, the same SHA-256 (nc's "Remote host: …" line before the
            data allowed); the speed goes into the detail
  cpu       after the starts: the ARM clock reached 1400 MHz
            (/scheme/rpi-firmware/arm_clock) and no throttle flag is set now
            (/scheme/rpi-firmware/throttled, bits 0-3)
  reboot    last: `reboot`, then devsh must answer again within 120 s with
            the same build ID; the seconds go into the detail (boot-time
            trend, #47)

Prints a summary and, with --report, appends it to that Markdown file.
With --crash-log, appends one JSON object per line for each failed start,
and for each crash dump in the serial log that matches no failed start
(see CRASH_LOG below), for counting start crashes across runs (#1).
Exits with 1 if a check failed.
"""

import argparse
import datetime
import hashlib
import json
import os
import re
import signal
import socket
import sys
import threading
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


def serial_size(path):
    """Current size of the serial log, to read only what comes after it."""
    try:
        return os.path.getsize(path) if path else None
    except OSError:
        return None


# Lines of a kernel crash dump worth quoting; the registers and the stack
# walk in between stay in the serial log.
CRASH_LINE = re.compile(
    r"grant was not|FATAL|(FAR|ELR|ESR)_EL1|UNHANDLED EXCEPTION|panicked|SIG[A-Z]{3,}"
)


def serial_lines(path, start):
    """The lines the serial log got after offset `start`, without terminal
    escape sequences."""
    if start is None:
        return []
    try:
        with open(path, "rb") as f:
            f.seek(0, os.SEEK_END)
            # A smaller file was rotated or truncated: read it from the start.
            f.seek(start if f.tell() >= start else 0)
            data = f.read()
    except OSError:
        return []
    text = ANSI.sub(b"", data).replace(b"\x00", b"").decode(errors="replace")
    return [l.rstrip() for l in text.replace("\r", "\n").split("\n") if l.strip()]


# One crash dump of the kernel, as on aarch64:
#   ...exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=100010)
#   FAR_EL1: 0xc5778b
#   ELR_EL1: 0000000000C5778B
#   ESR_EL1: 000000008A000000
#   SP_EL0: 00007FFFFFFFF560, X0: ... X30: 0000000000401E20 (some builds)
#     FP 00007ffffffff560: PC 0000000000401c48   (frame walk)
#   ...signal:INFO -- UNHANDLED EXCEPTION, CPU #0, PID 258, NAME /usr/bin/pwd, CONTEXT ...
REGISTER = re.compile(r"\b(FAR_EL1|ELR_EL1|ESR_EL1|LR|SP|SP_EL0|X30|x30)\s*[:=]\s*(?:0x)?([0-9A-Fa-f]+)\b")
REGISTER_KEY = {"FAR_EL1": "far", "ELR_EL1": "elr", "ESR_EL1": "esr",
                "LR": "lr", "X30": "lr", "x30": "lr", "SP": "sp", "SP_EL0": "sp"}
FRAME = re.compile(r"\bFP ([0-9A-Fa-f]+): PC ([0-9A-Fa-f]+)")
# Frames with the frame pointer above this are the kernel's own, after the
# program's.
USER_END = 0x8000_0000_0000
TRACE_FRAMES = 8
UNHANDLED = re.compile(r"UNHANDLED EXCEPTION, CPU #(\d+), PID (\d+), NAME (\S+?),?(?:\s|$)")


def split_dumps(lines):
    """The crash dumps in serial log lines, in order: dicts with kind, far,
    elr, esr, ec (exception class from ESR), lr, sp, pid, cpu, name (None
    where the dump does not say), trace (the first PCs of the program's
    frame walk)
    and their key lines."""
    dumps, cur = [], None

    def new():
        return {"kind": None, "far": None, "elr": None, "esr": None, "ec": None,
                "lr": None, "sp": None, "pid": None, "cpu": None, "name": None, "trace": [],
                "lines": []}

    for line in lines:
        fatal = "FATAL" in line or "panicked" in line
        if fatal or (cur is None and (REGISTER.search(line) or UNHANDLED.search(line))):
            if cur is not None and (fatal or cur["name"] is not None):
                dumps.append(cur)
                cur = None
            if cur is None:
                cur = new()
            if fatal:
                m = re.search(r"\(ty=(\w+)\)", line)
                cur["kind"] = f"ty={m.group(1)}" if m else line.split("--")[-1].strip()[:100]
        if cur is None:
            continue
        if CRASH_LINE.search(line):
            cur["lines"].append(line)
        for name, value in REGISTER.findall(line):
            key = REGISTER_KEY[name]
            if cur[key] is None:
                cur[key] = hex(int(value, 16))
        m = FRAME.search(line)
        if m and int(m.group(1), 16) < USER_END and len(cur["trace"]) < TRACE_FRAMES:
            cur["trace"].append(hex(int(m.group(2), 16)))
        m = UNHANDLED.search(line)
        if m:
            cur["cpu"], cur["pid"], cur["name"] = int(m.group(1)), int(m.group(2)), m.group(3)
            dumps.append(cur)
            cur = None
    if cur is not None:
        dumps.append(cur)
    for d in dumps:
        if d["esr"] is not None:
            d["ec"] = hex((int(d["esr"], 16) >> 26) & 0x3F)
    return dumps


def match_dumps(batch, dumps, lines, count):
    """Gives each failed start of one command line the first unused dump
    whose NAME is its program; returns the dumps no start took."""
    left = list(dumps)
    for c in batch:
        d = next((d for d in left if d["name"] and d["name"].rsplit("/", 1)[-1] == c["program"]), None)
        if d is not None:
            left.remove(d)
        c["crash"] = d
        c["serial"] = d["lines"][-count:] if d else []
    # No dump at all: show the end of the serial log instead.
    if not dumps and batch:
        batch[0]["serial"] = lines[-count:]
    return left


# --crash-log: one JSON object per line. Fields: host, build_id, libc
# (sha256 of /usr/lib/libc.so), time (UTC), starts (the range of the command
# line), program (null for a dump no failed start took), status, messages
# (what the program printed), crash (null if no dump was found; else kind,
# far, elr, esr, ec, lr, sp as hex strings, pid, cpu, name, each null where
# the dump does not say, and trace, the first PCs of the frame walk), serial
# (key lines of that dump).
def crash_record(info, host, c):
    crash = None
    if c.get("crash"):
        crash = {k: v for k, v in c["crash"].items() if k != "lines"}
    return {"host": host, "build_id": info.get("build_id"), "libc": info.get("libc"),
            "time": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "starts": c["starts"].replace("–", "-"), "program": c["program"], "status": c["status"],
            "messages": c["messages"], "crash": crash, "serial": c["serial"]}


def crash_summary(d):
    """FAR, ELR, ESR and PID of a dump in one line for the report."""
    parts = [f"{k.upper()} {d[k]}" for k in ("far", "elr", "esr", "lr", "sp") if d.get(k)]
    if d.get("pid") is not None:
        parts.append(f"PID {d['pid']}")
    return ", ".join(parts)


# Ion's message when it cannot start a program of the start loop.
NOT_STARTED = re.compile(r"ion: .*?(?:command not found|No such file|Permission denied|Exec format).*?\b(ls|pwd)\b")


def describe_status(status):
    """Ion's $? as text; above 128 it is a signal number plus 128. A program
    the kernel ends for an exception shows exit 1 in QEMU, so the serial
    lines tell crashes apart."""
    if not status.isdigit():
        return "exit status unknown"
    code = int(status)
    if code > 128:
        try:
            return f"exit {code}, signal {code - 128} ({signal.Signals(code - 128).name})"
        except ValueError:
            return f"exit {code}, signal {code - 128}"
    return f"exit {code}"


def kv(lines):
    """`name value` lines (stats files) as a dict."""
    out = {}
    for l in lines:
        parts = l.split(None, 1)
        if len(parts) == 2:
            out[parts[0]] = parts[1].strip()
    return out


def timed(sh, command, timeout=120):
    """(output lines, seconds) of one command, without the devsh round trip
    (measured with `true` right before)."""
    t0 = time.time()
    sh.run("true")
    base = time.time() - t0
    t0 = time.time()
    out = sh.run(command, timeout)
    return out, max(0.0, time.time() - t0 - base)


def check_audio(sh, check):
    stats = kv(sh.run("cat /scheme/audiohw/stats"))
    rate = int(stats.get("measured_rate_hz", "0") or 0)
    faults = stats.get("dma_faults", "?")
    # "pwm_errors 0 bits=0x000": the count is the first word.
    pwm = (stats.get("pwm_errors", "0").split() or ["0"])[0]
    ok = abs(rate - 44100) <= 882 and faults == "0" and pwm == "0"
    out, secs = timed(sh, "tone -s 3 > /scheme/audio; echo rc=$?", 60)
    rc = out[-1] if out else "no output"
    alive = any("audiod" in l for l in sh.run("ps"))
    ok = ok and rc == "rc=0" and 2.7 <= secs <= 3.3 and alive
    check("audio", ok, f"mode {stats.get('mode', '?')}, measured {rate} Hz, dma_faults {faults}, "
          f"pwm_errors {pwm}; tone -s 3 took {secs:.2f} s, {rc}; audiod {'running' if alive else 'gone'}")


def check_time(sh, check):
    out = sh.run("cat /tmp/timesync.synced; date +%s")
    nums = [int(l) for l in out if l.isdigit()]
    if len(nums) < 2:
        check("time", False, "no NTP answer yet (/tmp/timesync.synced missing): " + "; ".join(out[:2]))
        return
    synced, now = nums[0], nums[-1]
    skew = now - int(time.time())
    when = datetime.datetime.fromtimestamp(synced, datetime.timezone.utc).strftime("%H:%M:%S")
    check("time", abs(skew) <= 120, f"set by NTP at {when} UTC; Pi clock {skew:+d} s against this host")


def local_ip(sh):
    return sh.sock.getsockname()[0]


def check_tcp(sh, check, host, mib):
    payload = os.urandom(mib << 20)
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("0.0.0.0", 0))
    srv.listen(1)
    srv.settimeout(120)
    port = srv.getsockname()[1]

    def serve():
        try:
            conn, _ = srv.accept()
            with conn:
                # A short pause on both ends: with a small payload sent and
                # closed at once, nc on Redox lost the data (QEMU,
                # 2026-11-11); this check is about the transfer.
                time.sleep(0.5)
                conn.sendall(payload)
                time.sleep(1)
                conn.shutdown(socket.SHUT_WR)
        except OSError:
            pass
        finally:
            srv.close()

    threading.Thread(target=serve, daemon=True).start()
    out, secs = timed(sh, f"nc {host}:{port} > /tmp/pi-test-tcp.bin; sha256sum /tmp/pi-test-tcp.bin; "
                      "rm /tmp/pi-test-tcp.bin", 300)
    got = next((l.split()[0] for l in out if l.endswith("pi-test-tcp.bin")), "")
    # Redox's nc writes "Remote host: <ip>:<port>" to stdout before the data.
    digests = {hashlib.sha256(payload).hexdigest(),
               hashlib.sha256(f"Remote host: {host}:{port}\n".encode() + payload).hexdigest()}
    ok = got in digests
    speed = f", {mib / secs:.1f} MiB/s" if secs > 0 else ""
    check("tcp", ok, f"{mib} MiB from {host}:{port} in {secs:.1f} s{speed}, "
          f"SHA-256 {'matches' if ok else 'differs: ' + (got[:16] or '; '.join(out[:2]))}")


def check_cpu(sh, check):
    clocks = []
    for _ in range(3):
        out = sh.run("cat /scheme/rpi-firmware/arm_clock")
        clocks += [int(l) for l in out if l.isdigit()]
    thr = (sh.run("cat /scheme/rpi-firmware/throttled") or ["?"])[0]
    try:
        flags = int(thr, 16 if thr.lower().startswith("0x") else 10)
    except ValueError:
        flags = None
    top = max(clocks) if clocks else 0
    ok = top >= 1400000000 and flags is not None and flags & 0xF == 0
    check("cpu", ok, f"ARM clock up to {top // 1000000} MHz, throttled {thr}")


def check_reboot(sh, check, build_id):
    try:
        sh.sock.sendall(b"reboot\r")
        time.sleep(1)
        sh.sock.close()
    except OSError:
        pass
    t0 = time.time()
    time.sleep(5)
    while time.time() - t0 < 120:
        try:
            again = Shell(sh.host, sh.port)
        except (OSError, RuntimeError):
            time.sleep(2)
            continue
        secs = time.time() - t0
        release = " ".join(again.run("cat /etc/redox-rpi-release"))
        again.close()
        ok = build_id in release
        check("reboot", ok, f"devsh back {secs:.0f} s after reboot"
              + ("" if ok else f", but the build ID is not {build_id}"))
        return
    check("reboot", False, "devsh did not answer within 120 s after reboot")


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
    ap.add_argument("--serial-log", help="serial console log to quote when a start fails")
    ap.add_argument("--serial-lines", type=int, default=15)
    ap.add_argument("--crash-log", help="append failed starts and crash dumps as JSON lines")
    ap.add_argument("--report")
    ap.add_argument("--note", default="")
    ap.add_argument("--extended", action="store_true",
                    help="also audio, time, tcp, cpu and reboot (the nightly, #258)")
    ap.add_argument("--tcp-host", help="this host's address as the Pi sees it (default: from the devsh connection)")
    ap.add_argument("--tcp-mib", type=int, default=10)
    args = ap.parse_args()

    results = []  # (check, ok, detail)
    info = {}
    crashes = []  # failed starts: starts (range), program, status, messages, crash, serial
    stray = []  # crash dumps that match no failed start, same fields

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

        if args.extended:
            for name, fn in (("audio", lambda: check_audio(sh, check)),
                             ("time", lambda: check_time(sh, check)),
                             ("tcp", lambda: check_tcp(sh, check, args.tcp_host or local_ip(sh), args.tcp_mib))):
                try:
                    fn()
                except (OSError, RuntimeError, ValueError) as e:
                    check(name, False, f"could not run: {str(e).splitlines()[0][:200]}")

        done, others = 0, set()
        per_run = 25  # iterations per command line, 2 starts each
        while done < args.starts:
            n = min(per_run, (args.starts - done + 1) // 2)
            mark = serial_size(args.serial_log)
            out = sh.run(
                f"for i in 1..={n}; ls / > /dev/null || echo FAILED ls $?; pwd > /dev/null || echo FAILED pwd $?; end",
                timeout=300,
            )
            # A failing program prints its messages before the FAILED line.
            messages, batch = [], []
            not_started = None
            for l in out:
                if l.startswith("FAILED"):
                    _, program, *status = l.split()
                    if not_started and not_started["program"] == program:
                        # Ion said why, then the || branch ran after all.
                        not_started["status"] += f", {describe_status(status[0] if status else '')}"
                    else:
                        batch.append({"starts": f"{done + 1}–{done + 2 * n}", "program": program,
                                      "status": describe_status(status[0] if status else ""),
                                      "messages": messages})
                    messages, not_started = [], None
                elif m := NOT_STARTED.match(l):
                    # Ion could not start it (no such file, not executable).
                    # It then ends the whole command line: no || branch, and
                    # the rest of this line's starts do not run.
                    not_started = {"starts": f"{done + 1}–{done + 2 * n}", "program": m.group(1),
                                   "status": "did not start, Ion skipped the rest of these starts",
                                   "messages": messages + [l[:200]]}
                    batch.append(not_started)
                    messages = []
                elif l and not l.startswith("/"):
                    messages.append(l[:200])
            others.update(messages)
            if batch:
                time.sleep(2)  # the crash dump reaches the serial log a bit later
            lines = serial_lines(args.serial_log, mark)
            dumps = split_dumps(lines)
            # One command line covers 50 starts: the dumps go to the failed
            # starts by program name.
            for d in match_dumps(batch, dumps, lines, args.serial_lines):
                stray.append({"starts": f"{done + 1}–{done + 2 * n}", "program": None, "status": None,
                              "messages": [], "crash": d, "serial": d["lines"][-args.serial_lines:]})
            crashes.extend(batch)
            done += 2 * n
        detail = f"{len(crashes)} of {done} failed"
        if crashes:
            counts = {}
            for c in crashes:
                key = f"{c['program']} {c['status']}"
                counts[key] = counts.get(key, 0) + 1
            detail += ": " + "; ".join(f"{k} ({v}×)" for k, v in sorted(counts.items()))
        if others:
            detail += "; other messages: " + "; ".join(f'"{e[:100]}"' for e in sorted(others)[:5])
        if stray:
            detail += f"; {len(stray)} other crash dump(s) in the serial log"
        check("starts", not crashes, detail)
        for i, c in enumerate(crashes + stray):
            print(f"  starts {c['starts']}: {c['program'] or 'no failed start'}, {c['status'] or ''}"
                  f"{'; ' + crash_summary(c['crash']) if c['crash'] else ''}", flush=True)
            for l in c["messages"]:
                print(f"    says: {l}", flush=True)
            # The serial lines of the first few; all of them are in the report.
            for l in c["serial"] if i < 3 else []:
                print(f"    serial: {l}", flush=True)
        info["starts"] = (len(crashes), done)
        if args.extended:
            try:
                check_cpu(sh, check)
            except (OSError, RuntimeError, ValueError) as e:
                check("cpu", False, f"could not run: {str(e).splitlines()[0][:200]}")
            # Last: the session ends with the reboot.
            check_reboot(sh, check, info["build_id"])
            sh = None
    finally:
        if sh:
            sh.close()

    passed = all(ok for _, ok, _ in results)
    if args.crash_log and (crashes or stray):
        with open(args.crash_log, "a") as f:
            for c in crashes + stray:
                f.write(json.dumps(crash_record(info, args.host, c)) + "\n")
        print(f"{len(crashes) + len(stray)} record(s) appended to {args.crash_log}", flush=True)
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
            for title, items in (("Failed starts", crashes), ("Crash dumps without a failed start", stray)):
                if not items:
                    continue
                f.write(f"\n{title} (build `{info.get('build_id', '?')}`):\n\n")
                for c in items[:20]:
                    f.write(f"- starts {c['starts']}")
                    if c["program"]:
                        f.write(f": `{c['program']}`, {c['status']}")
                    elif c["crash"] and c["crash"]["name"]:
                        f.write(f": `{c['crash']['name']}`")
                    if c["crash"]:
                        f.write(f"; {crash_summary(c['crash'])}")
                    if c["messages"]:
                        f.write("; printed: " + "; ".join(f'"{m}"' for m in c["messages"][:3]))
                    if not c["serial"]:
                        f.write("; no serial log lines\n")
                        continue
                    f.write(", serial lines:\n\n  ```\n")
                    f.write("".join(f"  {l}\n" for l in c["serial"]))
                    f.write("  ```\n")
                if len(items) > 20:
                    f.write(f"- … and {len(items) - 20} more\n")
    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
