#!/usr/bin/env python3
"""Fast regression set before a Pi row (#265), in QEMU or on a real Pi.

Usage:
    scripts/regression-set.py <host> [--port 2323] [--serial-log <file>]
                              [--manifest <file>] [--starts 50]
                              [--tcp-host <ip>] [--tcp-mib 4]
                              [--dead-dns <ip>] [--lan-name <name>]
                              [--outside-name <name>] [--desktop]
                              [--report <file>] [--only <check>]...

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
  tcc           REQ-TCC-1 criterion 5: /usr/share/tcc-tests/run.ion (package
                tcc-bench) reaches "TCC-TEST done", and these steps give
                their output: hello.c built dynamically and run; tcc -run
                hello.c; hallo.c as a #!/usr/bin/tcc -run script with stdin
                and two arguments (exit status 42, from its shebang's
                -DANSWER=42); bench.c built and run (time reported, no
                limit); tcc builds /usr/src/tinycc/tcc.c and that tcc2
                builds hello.c, which runs. tcc -static (a known limit) and
                bench-gcc (comparison) are not judged (skip without the
                experiment tcc)
  dns           #358, #374, REQ-NET-7 (docs/requirements/network.md): a
                name server is set (from DHCP); an unknown name, fully
                qualified and short, gives "Name does not resolve" within
                1 s (criterion 3). Times come from ion's `time` on the
                guest. Also, as their own lines:
                dns-lan with --lan-name: that name (with a host part, not
                the bare domain) and its short form each resolve within 2 s
                (criterion 1);
                dns-outside with --outside-name: it resolves (criterion 2);
                dns-silent with --dead-dns as name server: "Try again"
                within 10 s for a fully qualified name and within 15 s for
                a short name with a local domain set (criterion 4); after
                the name server is set back, an unknown name gives "Name
                does not resolve" again
  launcher-sopwith
                #335, REQ-GFX-2: the launcher entry
                /usr/share/ui/apps/70_sopwith names /usr/bin/sopwith and
                an icon, and both files exist (skip without the experiment
                orbital-apps)
  ifconfig-mac  #444, #171: `ifconfig eth0` shows the MAC address netstack
                has (/scheme/netcfg/ifaces/eth0/mac), not 00:00:00:00:00:00;
                skip while ifconfig still prints the netmask as a prefix
                length (no netutils 0002 and no experiment ifconfig-mac)
  gamepad-console  #320: `gamepad-test --console` prints every event that
                `gamepad-test --send` sends through inputd, and the last
                line holds A with X, Y and RZ set (skip without
                gamepad-test, experiment gamepad-test, or when it reports
                no controllers handle or no --console: #455)
  tcp           #364, #92: --tcp-mib MiB from this host with nc, same
                SHA-256 (pi-test.py's check); --tcp-host is this host as the
                guest sees it (QEMU user networking: 10.0.2.2)
  sopwith       #335, #33, REQ-GFX-2, only with --desktop or --only
                sopwith, last and in a new devsh session: starts the
                desktop (desktop --yes) and Sopwith in it, as the launcher
                would; its log shows an Orbital window and SDL's Redox
                audio driver, it still runs after 15 s, and
                /scheme/audiohw/stats loud_chunks is higher than before
                Sopwith started (its title tune reached the driver). It is
                then ended with SIGTERM, or SIGKILL if that does not end it
                within 2 s; which one is only reported until #460 is on
                main (#405). Orbital keeps running afterwards, so the
                caller reboots before anything else (regression-qemu.sh
                ends QEMU; a Pi row reboots). Keys and audible sound stay a
                person step (rc test)
  serial        with --serial-log: no kernel panic, "panicked at",
                "RELIBC PANIC", "UNHANDLED EXCEPTION" or "main loop stuck"
                (#92, #151) in the whole log, and no pcid or rtcd (base 0030)

Prints one line per check (PASS, FAIL or SKIP) and, with --report, appends a
Markdown table to that file. A failed check that has raw output (tcc: the
failed steps' sections of run.ion; sig-pipe, scanf-pipe: the script's
output) prints it after its FAIL line, each line indented by "  | ", and
the report gets it below the table (at most RAW_MAX lines each, #611).
Exits 1 if a check failed. A check that is
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
RAW_MAX = 200  # lines of raw output kept per failed check (the last ones)

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
    check(name, ok, f"{secs:.1f} s: " + "; ".join(shown[-4:] or out[-3:]), raw=out)


DNS_LAN_S = 2        # REQ-NET-7 criterion 1, each lookup of an existing LAN name
DNS_UNKNOWN_S = 1    # REQ-NET-7 criterion 3
DNS_DEAD_FQDN_S = 10  # criterion 4, a fully qualified name
DNS_DEAD_SHORT_S = 15  # criterion 4, a short name (tried in the local domain first)
REAL = re.compile(r"real\s+([0-9.]+)s")


def lookup(sh, name):
    """(output lines, seconds) of `ping -c 1 <name>`, timed by ion's `time`
    on the guest, so the devsh round trip does not count (the 1 s
    limit is close to it). Falls back to pi_test.timed."""
    out, secs = pi_test.timed(sh, f"time ping -c 1 {name}", 60)
    for l in out:
        m = REAL.search(l)
        if m:
            return out, float(m.group(1))
    return out, secs


def says(out, text):
    return any(text in l for l in out)


TCC_RUN = "/usr/share/tcc-tests/run.ion"
TCC_LOG = "/tmp/regr-tcc.log"
TIMEIT = re.compile(r"\[timeit\] ([0-9.]+) ms, status (\d+): (\S+)")


def tcc_sections(out):
    """run.ion's output split at its "== <step>" lines: {step: lines}."""
    secs, cur = {}, None
    for l in out:
        l = ANSI_TEXT.sub("", l).rstrip()
        if l.startswith("== "):
            cur = l[3:]
            secs[cur] = []
        elif cur is not None:
            secs[cur].append(l)
    return secs


def tcc_ms(lines, prog):
    """Wall time and exit status timeit gave for prog, or (None, None)."""
    for l in lines:
        m = TIMEIT.search(l)
        if m and m.group(3) == prog:
            return float(m.group(1)), int(m.group(2))
    return None, None


def check_tcc(sh, check, tcc):
    """REQ-TCC-1 criterion 5: the named subset of run.ion. stderr goes to
    the log too: timeit writes its times there."""
    if not tcc or not has(sh, TCC_RUN):
        check("tcc", None, f"no {TCC_RUN} or no tcc (experiment tcc)")
        return
    out, secs = pi_test.timed(sh, f"ion {TCC_RUN} &> {TCC_LOG}; cat {TCC_LOG}", 1800)
    s = tcc_sections(out)
    prefixes = {"dynamic": "hello (dynamic)", "-run": "tcc -run", "script": "shebang",
                "bench": "bench, compiled by tcc", "self-hosting": "self-hosting"}

    def sec(prefix):
        return next((v for k, v in s.items() if k.startswith(prefix)), [])

    def says(lines, text):
        return any(text in l for l in lines)

    hello = "Hello from tcc on Redox ("
    script = sec("shebang")
    bench = sec("bench, compiled by tcc")
    selfh = sec("self-hosting")
    bench_ms = next((l.split()[-2] for l in bench if l.startswith("total")), None)
    self_ms, _ = tcc_ms(selfh, "tcc")
    steps = [
        ("dynamic", says(sec("hello (dynamic)"), hello + "./hello)")),
        ("-run", says(sec("tcc -run"), hello)),
        ("script", says(script, "argv[1]=a") and says(script, "argv[2]=b")
         and says(script, "Hallo, stdin 6 bytes") and says(script, "rc=42")),
        ("bench", bench_ms is not None and tcc_ms(bench, "./bench")[1] == 0),
        ("self-hosting", says(selfh, hello + "./hello2)")),
        ("done", any(l.strip() == "TCC-TEST done" for l in out)),
    ]
    failed = [n for n, ok in steps if not ok]
    detail = (f"{secs:.1f} s; " + ("all 5 steps and TCC-TEST done" if not failed
                                   else "failed: " + ", ".join(failed))
              + (f"; bench total {bench_ms} ms" if bench_ms else "")
              + (f"; tcc.c built in {self_ms:.0f} ms" if self_ms is not None else ""))
    # Raw output for #611: the failed steps' sections, or the whole log when
    # run.ion did not finish or a section is missing.
    raw = []
    for n in failed:
        key = next((k for k in s if k.startswith(prefixes.get(n, "\0"))), None)
        if key is None:
            raw = [ANSI_TEXT.sub("", l).rstrip() for l in out]
            break
        raw += [f"== {key}"] + s[key]
    check("tcc", not failed, detail, raw=raw)


def check_dns(sh, check, args):
    """REQ-NET-7 (docs/requirements/network.md), one line per criterion."""
    ns = [l.strip() for l in sh.run("cat /scheme/netcfg/resolv/nameserver") if l.strip()]
    if not ns:
        check("dns", False, "no name server set (DHCP, #358)")
        return
    orig = ns[0]
    dom = [l.strip() for l in sh.run("cat /scheme/netcfg/resolv/domain") if l.strip()]
    orig_dom = dom[0] if dom else ""
    tag = uuid.uuid4().hex[:6]

    # 1: names of a LAN host, fully qualified and short (Pi rows: --lan-name)
    if args.lan_name:
        short = args.lan_name.split(".")[0]
        names = [args.lan_name] + ([short] if short != args.lan_name else [])
        res = []
        for n in names:
            out, secs = lookup(sh, n)
            res.append((n, says(out, "packets transmitted") or says(out, "PING "), secs))
        check("dns-lan", all(ok and s <= DNS_LAN_S for _, ok, s in res),
              f"local domain '{orig_dom or 'none'}'; "
              + "; ".join(f"{n}: {'resolves' if ok else 'FAILS'} ({s:.2f} s"
                          + (f", over {DNS_LAN_S} s" if ok and s > DNS_LAN_S else "") + ")"
                          for n, ok, s in res) + f" (limit {DNS_LAN_S} s each)")
    # 2: a name outside the LAN
    if args.outside_name:
        out, secs = lookup(sh, args.outside_name)
        ok = says(out, "PING ")
        check("dns-outside", ok, f"{args.outside_name}: " + ("resolves" if ok else "; ".join(out[:2])[:100])
              + f" ({secs:.2f} s)")
    # 3: an unknown name fails with "Name does not resolve" within 1 s
    res = []
    for n in (f"nosuch-{tag}.invalid", f"nosuch-{tag}"):
        out, secs = lookup(sh, n)
        res.append((n, says(out, "Name does not resolve") and secs <= DNS_UNKNOWN_S, secs, out))
    check("dns", all(r[1] for r in res), f"name server {orig}; unknown names: " + "; ".join(
        f"{n} {s:.2f} s" + ("" if ok else " FAIL: " + "; ".join(o[:2])[:80]) for n, ok, s, o in res)
        + f" (limit {DNS_UNKNOWN_S} s)")
    # 4: a silent name server: "Try again" within 10 s (fully qualified)
    #    or 15 s (short, with a local domain set)
    if args.dead_dns:
        sh.run(f"echo {args.dead_dns} > /scheme/netcfg/resolv/nameserver")
        if not orig_dom:
            sh.run("echo regr > /scheme/netcfg/resolv/domain")
        try:
            out1, s1 = lookup(sh, f"regr-{tag}.example.com")
            out2, s2 = lookup(sh, f"regr-{tag}")
        finally:
            sh.run(f"echo {orig} > /scheme/netcfg/resolv/nameserver")
            if not orig_dom:
                sh.run("echo > /scheme/netcfg/resolv/domain")
        fqdn = says(out1, "Try again") and s1 <= DNS_DEAD_FQDN_S
        short = says(out2, "Try again") and s2 <= DNS_DEAD_SHORT_S
        out3, _ = lookup(sh, f"nosuch-{tag}.invalid")
        # the name server and domain read back as before (an empty write
        # sets "no domain")
        ns_now = [l.strip() for l in sh.run("cat /scheme/netcfg/resolv/nameserver") if l.strip()]
        dom_now = [l.strip() for l in sh.run("cat /scheme/netcfg/resolv/domain") if l.strip()]
        restored = ns_now[:1] == [orig] and (dom_now[0] if dom_now else "") == orig_dom
        back = says(out3, "Name does not resolve") and restored
        check("dns-silent", fqdn and short and back,
              f"silent name server {args.dead_dns}: fully qualified {s1:.1f} s "
              + ("Try again" if says(out1, "Try again") else "; ".join(out1[:2])[:60])
              + f" (limit {DNS_DEAD_FQDN_S} s), short name {s2:.1f} s "
              + ("Try again" if says(out2, "Try again") else "; ".join(out2[:2])[:60])
              + f" (limit {DNS_DEAD_SHORT_S} s); set back: "
              + ("resolves again" if says(out3, "Name does not resolve") else "; ".join(out3[:2])[:80])
              + ("" if restored else f"; NOT restored: name server {ns_now[:1]}, domain {dom_now[:1]}"))


SOPWITH_ENTRY = "/usr/share/ui/apps/70_sopwith"
SOPWITH_SCRIPT = "/tmp/regr-sopwith.ion"
SOPWITH_LOG = "/tmp/regr-sopwith.log"


def check_launcher_sopwith(sh, check, args):
    if not has(sh, SOPWITH_ENTRY):
        check("launcher-sopwith", None, f"no {SOPWITH_ENTRY} (experiment orbital-apps)")
        return
    entry = dict(l.split("=", 1) for l in sh.run(f"cat {SOPWITH_ENTRY}") if "=" in l)
    binary, icon = entry.get("binary", ""), entry.get("icon", "")
    missing = [f for f in (binary, icon) if not f or not has(sh, f)]
    ok = binary.endswith("/sopwith") and icon and not missing
    detail = (f"name={entry.get('name', '?')}, binary={binary or '?'}, icon={icon or '?'}"
              + (f"; missing: {', '.join(m or '(empty)' for m in missing)}" if missing else "; both exist"))
    check("launcher-sopwith", ok, detail)


def loud_chunks(sh):
    for l in sh.run("cat /scheme/audiohw/stats"):
        m = re.match(r"loud_chunks\s+(\d+)$", l)
        if m:
            return int(m.group(1))
    return None


def check_sopwith(sh, check, args):
    if not has(sh, "/usr/bin/sopwith") or not has(sh, "/usr/bin/desktop"):
        check("sopwith", None, "no /usr/bin/sopwith or /usr/bin/desktop (experiments orbital-apps, desktop)")
        return
    notes = []
    if "orbital" not in context_pids(sh, ["orbital"]):
        out = sh.run("desktop --yes > /tmp/regr-desktop.out; cat /tmp/regr-desktop.out", 90)
        if "orbital" not in context_pids(sh, ["orbital"]):
            check("sopwith", False, "Orbital did not start: " + "; ".join(out[-2:])[:150])
            return
        notes.append("desktop started")
    else:
        notes.append("Orbital already ran")
    before = loud_chunks(sh)
    # Through a script: "cmd &!" followed by "; echo" (how Shell.run wraps a
    # command) is not valid Ion. &! disowns it, so it outlives the script.
    sh.run(f"echo 'sopwith &> {SOPWITH_LOG} &!' > {SOPWITH_SCRIPT}; ion {SOPWITH_SCRIPT}")
    t0 = time.time()
    pid = None
    while time.time() - t0 < 15:
        time.sleep(3)
        pid = context_pids(sh, ["sopwith"]).get("sopwith")
        if not pid:
            break
    log = sh.run(f"cat {SOPWITH_LOG}")
    after = loud_chunks(sh)
    window = any("Initializing orbital window" in l for l in log)
    driver = any("SDL redox audio driver" in l for l in log)
    sound = before is not None and after is not None and after > before
    notes += [f"window {'yes' if window else 'no'}", f"Redox audio driver {'yes' if driver else 'no'}",
              f"running after 15 s: {'yes, PID ' + pid if pid else 'no'}",
              f"loud_chunks {before} -> {after}"]
    if pid:
        # kill, wait and the process list in one line: ion's notice that a
        # job ended would otherwise come in the next command's output.
        alive = any(re.search(rf"^\s*{pid}\b.*sopwith", l)
                    for l in sh.run(f"kill {pid}; sleep 2; ps"))
        if alive:
            sh.run(f"kill -KILL {pid}; sleep 1; ps")
            notes.append("SIGTERM did not end it within 2 s, SIGKILL did (#405, info until #460)")
        else:
            notes.append("ended by SIGTERM")
    elif log:
        notes.append("log: " + " / ".join(log[-3:])[:150])
    check("sopwith", bool(window and driver and pid and sound), "; ".join(notes))


MAC = re.compile(r"\b[0-9a-fA-F]{2}(?::[0-9a-fA-F]{2}){5}\b")


def check_ifconfig_mac(sh, check, args):
    want = "".join(sh.run("cat /scheme/netcfg/ifaces/eth0/mac")).strip().replace("-", ":").lower()
    out = sh.run("ifconfig eth0")
    text = " ".join(out)
    shown = [m.lower() for m in MAC.findall(text)]
    if want and want in shown:
        check("ifconfig-mac", True, f"ifconfig shows {want}, as netstack has it")
        return
    # netutils 0002 (and the experiment ifconfig-mac) print the netmask
    # dotted; without either, ifconfig still has upstream's placeholder.
    if not re.search(r"\b255\.\d+\.\d+\.\d+\b", text):
        check("ifconfig-mac", None, "ifconfig without netutils 0002 (#446) or experiment ifconfig-mac")
        return
    check("ifconfig-mac", False, f"netstack has {want or 'no MAC'}, ifconfig shows "
          f"{', '.join(shown) or 'none'}: " + "; ".join(l.strip() for l in out[:4])[:120])


def check_gamepad_console(sh, check, args):
    if not has(sh, "/usr/bin/gamepad-test"):
        check("gamepad-console", None, "no gamepad-test (experiment gamepad-test)")
        return
    sh.run("rm -f /tmp/regr-gp.out /tmp/regr-gp.err; "
           "gamepad-test --console > /tmp/regr-gp.out ^> /tmp/regr-gp.err &")
    time.sleep(2)
    # `test -e` does not see /scheme/input/controllers (no stat on inputd's
    # scheme), so the tool says itself when the handle or --console is
    # missing: inputd without base patch 0098 (#455), or an older binary.
    err = " ".join(sh.run("cat /tmp/regr-gp.err"))
    if "controllers handle" in err or "unknown argument" in err:
        check("gamepad-console", None, f"gamepad-test --console not usable yet (#455): {err[:100]}")
        return
    sent = [l for l in sh.run("gamepad-test --send", 120) if l.startswith("gamepad-test: sent ")]
    time.sleep(2)
    for pid in context_pids(sh, ["gamepad-test"]).values():
        sh.run(f"kill {pid}")
    got = [l for l in sh.run("cat /tmp/regr-gp.out") if l.startswith("gamepad-test: controller ")]
    last = got[-1] if got else ""
    ok = (len(sent) > 0 and len(got) == len(sent) and "held A" in last
          and all(f"{a} " in last.split("| axes", 1)[-1] for a in ("X", "Y", "RZ")))
    detail = f"sent {len(sent)}, --console printed {len(got)}"
    if not got:
        detail += "; " + "; ".join(sh.run("cat /tmp/regr-gp.err"))[:120]
    else:
        detail += f"; last: {last.split(':', 2)[-1].strip()[:100]}"
    check("gamepad-console", ok, detail)


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
    ap.add_argument("--dead-dns", help="an address where no name server answers (QEMU: 10.0.2.99; "
                    "anywhere: 192.0.2.1, a documentation address)")
    ap.add_argument("--lan-name", help="fully qualified name of a LAN host for REQ-NET-7 criterion 1 "
                    "(the short name is checked too)")
    ap.add_argument("--outside-name", help="a name outside the LAN for REQ-NET-7 criterion 2")
    ap.add_argument("--desktop", action="store_true",
                    help="also run sopwith: starts the desktop, which then runs until the next reboot")
    ap.add_argument("--report")
    ap.add_argument("--only", action="append", help="run only these checks")
    args = ap.parse_args()

    results = []
    raws = {}  # name: raw output of a failed check (#611)

    def check(name, ok, detail, raw=None):
        word = "SKIP" if ok is None else "PASS" if ok else "FAIL"
        results.append((name, word, detail))
        print(f"{word} {name}: {detail}", flush=True)
        if word == "FAIL" and raw:
            raws[name] = [ANSI_TEXT.sub("", l).rstrip() for l in raw][-RAW_MAX:]
            for l in raws[name]:
                print(f"  | {l}", flush=True)

    def want(name):
        if name == "sopwith":  # opt-in: it leaves Orbital running
            return args.desktop or bool(args.only and name in args.only)
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
            ("tcc", lambda: check_tcc(sh, check, tcc)),
            ("dns", lambda: check_dns(sh, check, args)),
            ("launcher-sopwith", lambda: check_launcher_sopwith(sh, check, args)),
            ("ifconfig-mac", lambda: check_ifconfig_mac(sh, check, args)),
            ("gamepad-console", lambda: check_gamepad_console(sh, check, args)),
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
    # Last, in its own session (nothing follows tcp in one): it leaves
    # Orbital running, so the caller reboots afterwards.
    if want("sopwith"):
        sh = pi_test.Shell(args.host, args.port)
        try:
            check_sopwith(sh, check, args)
        except Exception as e:
            check("sopwith", False, f"error: {type(e).__name__}: {str(e).splitlines()[0][:150]}")
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
            for name, lines in raws.items():
                f.write(f"### {name}: raw output of the failed check (last {RAW_MAX} lines at most)\n\n"
                        "```text\n" + "\n".join(l.replace("```", "'''") for l in lines) + "\n```\n\n")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
