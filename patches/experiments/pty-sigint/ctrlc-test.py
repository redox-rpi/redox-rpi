#!/usr/bin/env python3
"""Ctrl-C check over devsh (issue #55).

Each round logs in, starts `sleep 40`, sends Ctrl-C after the given delay
and waits up to 30 s for Ion's "ended by signal SIGINT". The first rounds
send it 1 s after Enter, while the program is still loading. A Ctrl-C that
comes while Ion still reads the line cancels the line instead: that counts
too when the prompt comes back and no `sleep` is left running. Finally a
Ctrl-C at the prompt must leave the shell working.

  ctrlc-test.py <host> [--port 2323] [--rounds 10] [--delays 0,0.1,0.3]

--delays sends Ctrl-C after these delays in turn instead (issue #86).
"""
import argparse, importlib.util, os, socket, sys, time

here = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "pitest", os.path.join(here, "../../../scripts/pi-test.py"))
pt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pt)


def read_for(sock, secs, until=None):
    out, end = b"", time.time() + secs
    sock.settimeout(0.5)
    while time.time() < end:
        try:
            chunk = sock.recv(65536)
            if not chunk:
                break
            out += chunk
        except socket.timeout:
            pass
        text = pt.ANSI.sub(b"", out).replace(b"\x00", b"").decode(errors="replace")
        if until and until in text:
            return text
    return pt.ANSI.sub(b"", out).replace(b"\x00", b"").decode(errors="replace")


def sleeps(host, port):
    sh = pt.Shell(host, port)
    try:
        return sum("/usr/bin/sleep" in line for line in sh.run("ps"))
    finally:
        sh.sock.close()


def round_(host, port, delay, baseline):
    """Returns "signal", "cancelled" or None."""
    sh = pt.Shell(host, port)
    try:
        sh.sock.sendall(b"sleep 40\r")
        time.sleep(delay)
        sh.sock.sendall(b"\x03")
        t0 = time.time()
        text = read_for(sh.sock, 30, "ended by signal SIGINT")
    finally:
        sh.sock.close()
    if "ended by signal SIGINT" in text:
        return f"signal {time.time() - t0:.1f}"
    after = text.split("sleep 40", 1)[-1]
    if pt.SHELL_PROMPT.search(after.encode()) and sleeps(host, port) == baseline:
        return "cancelled"
    print("  session output:", repr(text[-300:]), flush=True)
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("host")
    ap.add_argument("--port", type=int, default=2323)
    ap.add_argument("--rounds", type=int, default=10)
    ap.add_argument("--delays", help="comma-separated delays in seconds")
    args = ap.parse_args()

    baseline = sleeps(args.host, args.port)
    delays = [float(d) for d in args.delays.split(",")] if args.delays else None
    failed = 0
    for i in range(args.rounds):
        if delays:
            delay = delays[i % len(delays)]
        else:
            delay = 1 if i < args.rounds // 2 else 3
        res = round_(args.host, args.port, delay, baseline)
        failed += res is None
        if res and res.startswith("signal"):
            what = f"stopped by SIGINT {res.split()[1]} s after Ctrl-C"
        else:
            what = {"cancelled": "line cancelled before start", None: "NOT stopped"}[res]
        print(f"round {i + 1}: Ctrl-C after {delay} s: {what}", flush=True)

    sh = pt.Shell(args.host, args.port)
    sh.sock.sendall(b"\x03")
    time.sleep(1)
    prompt_ok = "after-ctrl-c" in "\n".join(sh.run("echo after-ctrl-c"))
    sh.sock.close()
    print(f"Ctrl-C at the prompt: shell {'still answers' if prompt_ok else 'GONE'}")

    passed = failed == 0 and prompt_ok
    print(f"{'PASS' if passed else 'FAIL'}: {args.rounds - failed} of {args.rounds} stopped")
    sys.exit(0 if passed else 1)


main()
