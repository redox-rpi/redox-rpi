#!/usr/bin/env python3
"""Fast regression set before a Pi row (#265), in QEMU or on a real Pi.

Usage:
    scripts/regression-set.py <host> [--port 2323] [--serial-log <file>]
                              [--manifest <file>] [--starts 50]
                              [--tcp-host <ip>] [--tcp-mib 4]
                              [--dead-dns <ip>] [--report <file>]
                              [--only <check>]...

Runs, through devshd, checks for regressions we already had, each named
after the issue it guards. A PR runs it in QEMU before its Pi row
(scripts/regression-qemu.sh does the boot); on the Pi it runs the same way.
Checks, in this order:

  basics        scripts/pi-test.py standard checks with --starts (version,
                manifest with --manifest, ps, ps-tty, sdcard, starts)
  signals-stop  #345: inputd and logd survive SIGSTOP / SIGCONT with the
                same PIDs; with --serial-log, a line written to /scheme/log
                still reaches the serial console
  waitpid       #386: waitpid() on a child while another child sends the
                parent only default-action signals (SIGWINCH, SIGURG)
                returns the child with its exit status, never EINTR
                (compiled with tcc; skip without the experiment tcc)
  sig-pipe      #205: /usr/share/sig-pipe/check.ion prints "SIGREPRO: pass"
                (skip without the experiments sig-pipe and tcc)
  scanf-pipe    #338: /usr/share/scanf-pipe/check.ion passes for its own
                pipe, a file and a shell pipe (skip without the experiments
                scanf-pipe and tcc)
  dns           #358, #374: a name server is set (from DHCP); an unknown
                name gives "Name does not resolve" within 5 s; with
                --dead-dns as name server a lookup gives "Try again" within
                15 s; after the name server is set back, the unknown name
                gives "Name does not resolve" again
  tcp           #364, #92: --tcp-mib MiB from this host with nc, same
                SHA-256 (pi-test.py's check); --tcp-host is this host as the
                guest sees it (QEMU user networking: 10.0.2.2)
  serial        with --serial-log: no kernel panic, "panicked at",
                "RELIBC PANIC", "UNHANDLED EXCEPTION" or "main loop stuck"
                (#92, #151) in the whole log, and no pcid or rtcd (base 0030)

Prints one line per check (PASS, FAIL or SKIP) and, with --report, appends a
Markdown table to that file. Exits 1 if a check failed. A check that is
skipped never fails the set. QEMU does not see hardware (audio rate, real
USB, the 3B+ external abort): those stay with pi-test.py --extended on the Pi.
"""

import argparse
import importlib.util
import os
import re
import subprocess
import sys
import time
import uuid

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("pi_test", os.path.join(HERE, "pi-test.py"))
pi_test = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pi_test)

SERIAL_BAD = re.compile(r"kernel panic|panicked at|RELIBC PANIC|UNHANDLED EXCEPTION|main loop stuck", re.I)
SERIAL_PCID = re.compile(r"^(pcid|rtcd)\b|\b(pcid|rtcd): ", re.M)
ANSI_TEXT = re.compile(r"\x1b\[[0-9;?]*[a-zA-Z]")

# One line of C (no #include, so ion's echo can write it): the parent waits
# for child c (exits 7 after 3 s) while child s sends it SIGWINCH (28) and
# SIGURG (23), whose default action is to ignore them.
WAITPID_C = (
    "int fork(void);int getpid(void);int kill(int,int);int usleep(unsigned);void _exit(int);"
    "int waitpid(int,int*,int);unsigned sleep(unsigned);int printf(const char*,...);int*__errno_location(void);"
    "int main(void){int me=getpid();int c=fork();if(c==0){sleep(3);_exit(7);}"
    "int s=fork();if(s==0){for(int i=0;i<40;i++){kill(me,28);kill(me,23);usleep(50000);}_exit(0);}"
    "int st=0;int r=waitpid(c,&st,0);int e=*__errno_location();int st2=0;waitpid(s,&st2,0);"
    "if(r!=c){printf(\"WAITPID-EINTR r=%d errno=%d\\n\",r,e);return 1;}"
    "printf(\"WAITPID-OK status=%d\\n\",(st>>8)&255);return 0;}"
)


def serial_text(path):
    with open(path, "rb") as f:
        return ANSI_TEXT.sub("", f.read().decode("utf-8", "replace").replace("\r", ""))


def has(sh, path):
    return any(l.strip() == "yes" for l in sh.run(f"test -e {path} && echo yes || echo no"))


def context_pids(sh, names):
    """PID of each daemon from /scheme/sys/context: the first number on the
    line that names it as a word."""
    pids = {}
    for line in sh.run("cat /scheme/sys/context"):
        for name in names:
            if name not in pids and re.search(rf"(^|[\s/]){re.escape(name)}(\s|$)", line):
                m = re.search(r"\b(\d+)\b", line)
                if m:
                    pids[name] = m.group(1)
    return pids


def check_basics(sh, check, args):
    cmd = [sys.executable, os.path.join(HERE, "pi-test.py"), args.host, "--port", str(args.port),
           "--starts", str(args.starts)]
    if args.manifest:
        cmd += ["--manifest", args.manifest]
    if args.serial_log:
        cmd += ["--serial-log", args.serial_log]
    t0 = time.time()
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=1800)
    lines = [l for l in r.stdout.splitlines() if l.startswith(("PASS ", "FAIL "))]
    fails = [l for l in lines if l.startswith("FAIL ")]
    detail = (f"{len(lines) - len(fails)} of {len(lines)} pass in {time.time() - t0:.0f} s"
              + ("; " + "; ".join(f[:120] for f in fails[:3]) if fails else ""))
    check("basics", r.returncode == 0 and lines and not fails, detail or r.stderr.strip()[-200:])


def check_signals_stop(sh, check, args):
    before = context_pids(sh, ["inputd", "logd"])
    if len(before) < 2:
        check("signals-stop", False, f"inputd or logd not in /scheme/sys/context: {before}")
        return
    for pid in before.values():
        sh.run(f"kill -STOP {pid}; sleep 2; kill -CONT {pid}; sleep 1", 60)
    after = context_pids(sh, ["inputd", "logd"])
    ok = after == before
    detail = f"inputd {before['inputd']}, logd {before['logd']}; after STOP/CONT: {after}"
    if args.serial_log:
        token = f"REGRESSION-LOG-{uuid.uuid4().hex[:8]}"
        sh.run(f"echo {token} > /scheme/log")
        deadline = time.time() + 10
        seen = False
        while time.time() < deadline and not seen:
            seen = token in serial_text(args.serial_log)
            time.sleep(0.5)
        ok = ok and seen
        detail += f"; /scheme/log line {'on' if seen else 'not on'} the serial console"
    check("signals-stop", ok, detail)


def check_waitpid(sh, check, args, tcc):
    if not tcc:
        check("waitpid", None, "no tcc in the image (experiment tcc)")
        return
    out = sh.run(f"echo '{WAITPID_C}' > /tmp/regr-waitpid.c; "
                 "tcc -o /tmp/regr-waitpid /tmp/regr-waitpid.c && /tmp/regr-waitpid", 120)
    res = next((l for l in out if l.startswith("WAITPID-")), "")
    check("waitpid", res == "WAITPID-OK status=7", res or "; ".join(out[-3:]))


def check_ion(sh, check, name, script, tcc, done, passed):
    if not tcc or not has(sh, script):
        check(name, None, f"no {script} or no tcc (experiments {name}, tcc)")
        return
    out, secs = pi_test.timed(sh, f"ion {script}", 600)
    ok = passed(out) and any(done in l for l in out)
    shown = [l for l in out if "pass" in l.lower() or "fail" in l.lower()]
    check(name, ok, f"{secs:.1f} s: " + "; ".join(shown[-4:] or out[-3:]))


def check_dns(sh, check, args):
    ns = [l.strip() for l in sh.run("cat /scheme/netcfg/resolv/nameserver") if l.strip()]
    if not ns:
        check("dns", False, "no name server set (DHCP, #358)")
        return
    orig = ns[0]
    out, secs = pi_test.timed(sh, "ping -c 1 nosuchname.invalid", 60)
    unknown = any("Name does not resolve" in l for l in out) and secs <= 5
    detail = f"name server {orig}; unknown name: {secs:.1f} s, " + (
        "Name does not resolve" if unknown else "; ".join(out[:2])[:100])
    ok = unknown
    if args.dead_dns:
        sh.run(f"echo {args.dead_dns} > /scheme/netcfg/resolv/nameserver")
        try:
            out, secs = pi_test.timed(sh, f"ping -c 1 regr-{uuid.uuid4().hex[:6]}.example.com", 60)
        finally:
            sh.run(f"echo {orig} > /scheme/netcfg/resolv/nameserver")
        again = any("Try again" in l for l in out) and secs <= 15
        out2, secs2 = pi_test.timed(sh, "ping -c 1 nosuchname.invalid", 60)
        back = any("Name does not resolve" in l for l in out2)
        ok = ok and again and back
        detail += (f"; dead name server {args.dead_dns}: {secs:.1f} s, "
                   + ("Try again" if again else "; ".join(out[:2])[:100])
                   + f"; set back: {'resolves again' if back else '; '.join(out2[:2])[:100]}")
    check("dns", ok, detail)


def check_serial(check, args):
    text = serial_text(args.serial_log)
    bad = [l.strip()[:100] for l in text.splitlines() if SERIAL_BAD.search(l)]
    pcid = SERIAL_PCID.findall(text)
    detail = (f"{len(bad)} bad lines" + (": " + "; ".join(bad[:3]) if bad else "")
              + ("; pcid or rtcd started" if pcid else "; no pcid, no rtcd"))
    check("serial", not bad and not pcid, detail)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("host")
    ap.add_argument("--port", type=int, default=2323)
    ap.add_argument("--serial-log", help="serial console log of this boot (QEMU: its console output)")
    ap.add_argument("--manifest")
    ap.add_argument("--starts", type=int, default=50)
    ap.add_argument("--tcp-host", help="this host as the guest sees it (default: the address devsh came from)")
    ap.add_argument("--tcp-mib", type=int, default=4)
    ap.add_argument("--dead-dns", help="an address where no name server answers (QEMU: 10.0.2.99)")
    ap.add_argument("--report")
    ap.add_argument("--only", action="append", help="run only these checks")
    args = ap.parse_args()

    results = []

    def check(name, ok, detail):
        word = "SKIP" if ok is None else "PASS" if ok else "FAIL"
        results.append((name, word, detail))
        print(f"{word} {name}: {detail}", flush=True)

    def want(name):
        return not args.only or name in args.only

    t0 = time.time()
    if want("basics"):
        check_basics(None, check, args)
    sh = pi_test.Shell(args.host, args.port)
    try:
        tcc = any(l.strip() == "yes" for l in sh.run("which tcc > /dev/null && echo yes || echo no"))
        steps = [
            ("signals-stop", lambda: check_signals_stop(sh, check, args)),
            ("waitpid", lambda: check_waitpid(sh, check, args, tcc)),
            ("sig-pipe", lambda: check_ion(sh, check, "sig-pipe", "/usr/share/sig-pipe/check.ion", tcc,
                                           "SIGREPRO", lambda out: any("SIGREPRO: pass" in l for l in out))),
            ("scanf-pipe", lambda: check_ion(sh, check, "scanf-pipe", "/usr/share/scanf-pipe/check.ion", tcc,
                                             "SCANF-PIPE done",
                                             lambda out: sum(l.strip().endswith(": pass") or l.strip() == "/ pass"
                                                             for l in out) == 3
                                             and not any("FAIL" in l for l in out))),
            ("dns", lambda: check_dns(sh, check, args)),
            # Last: after nc, the next command in the same devsh session gave
            # no output in both QEMU runs (2026-11-12), so nothing follows it.
            ("tcp", lambda: pi_test.check_tcp(sh, check, args.tcp_host or pi_test.local_ip(sh), args.tcp_mib)),
        ]
        for name, step in steps:
            if want(name):
                try:
                    step()
                except Exception as e:  # one broken check must not hide the others
                    check(name, False, f"error: {type(e).__name__}: {str(e).splitlines()[0][:150]}")
    finally:
        sh.close()
    if args.serial_log and want("serial"):
        check_serial(check, args)

    fails = [r for r in results if r[1] == "FAIL"]
    summary = (f"{sum(r[1] == 'PASS' for r in results)} pass, {len(fails)} fail, "
               f"{sum(r[1] == 'SKIP' for r in results)} skip in {time.time() - t0:.0f} s")
    print(summary)
    if args.report:
        with open(args.report, "a") as f:
            f.write(f"## Regression set ({summary})\n\n| Check | Result | Detail |\n|---|---|---|\n")
            for name, word, detail in results:
                f.write(f"| {name} | {word} | {detail.replace('|', '/')} |\n")
            f.write("\n")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
