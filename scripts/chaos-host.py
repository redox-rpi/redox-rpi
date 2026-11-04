#!/usr/bin/env python3
"""Chaos test from the build host: faults from outside the Pi (issue #196).

Usage:
    scripts/chaos-host.py <host> [--port 2323] [--rounds 10] [--seed N]
                          [--faults dwc2d,server,pkg,stressor,netem-loss,...]
                          [--serve-addr <addr>] [--serve-port 18080]
                          [--serial-log <file>] [--recover 180]
                          [--netem] [--iface <dev>] [--print-sudo]
                          [--stressor-rounds 20] [--log <file.jsonl>]

Each round picks one fault (from --faults, by --seed), injects it, and then
checks that the Pi recovers: devsh answers within --recover seconds and the
serial log got no kernel panic or crash dump meanwhile. Every round prints
its seed; `--seed S --rounds N` repeats a run.

Faults:
  stressor      run the on-Pi stressor `chaos` (tools/chaos, experiment
                `chaos`) over devsh for --stressor-rounds rounds
  server        the Pi's `chaos --kind net` client against a misbehaving
                HTTP server on this host (stalls, garbage, resets,
                half-closed connections, slow or endless replies)
  pkg           `pkg` fetches from this host's misbehaving HTTP and TLS
                server (wrong certificate, TLS garbage, resets); a
                temporary /etc/pkg.d/05_chaos points it there
  dwc2d         force the USB driver that carries the network to cut it:
                `network` (hub port power off and on, on main) or, with
                --dwc2d-restart, `restart` (experiment dwc2d-restart)
  netem-loss, netem-delay, netem-reorder, netem-blackout, tcp-reset
                network faults on this host with `tc netem` and iptables,
                only for traffic to and from the Pi's address. They need
                root: with --netem they run through `sudo -n`; otherwise,
                or with --print-sudo, the commands are printed and the
                round is skipped. Never under QEMU (user networking).

Power cuts are not here: they go through the Pi test agent (Shelly plug).

Under QEMU (scripts/qemu-raspi3b.sh --usb-net): host 127.0.0.1, --port the
DEVSH_PORT, --serve-addr 10.0.2.2, --serial-log images/qemu-test.log.

Exit code 0 when every round recovered, 1 otherwise.
"""

import argparse
import importlib.util
import json
import os
import random
import re
import shlex
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
_spec = importlib.util.spec_from_file_location("pi_test", os.path.join(HERE, "pi-test.py"))
pi_test = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pi_test)
Shell, serial_size, serial_lines = pi_test.Shell, pi_test.serial_size, pi_test.serial_lines

# Lines that mean the kernel or a driver died, on top of pi-test's crash
# dump lines. A user program's crash dump (UNHANDLED EXCEPTION in a chaos
# child killed on purpose) is reported but does not fail the round.
PANIC = re.compile(r"KERNEL PANIC|kernel panic|panicked at|FATAL|Kernel stopped|kstop", re.I)
CRASH = pi_test.CRASH_LINE

ALL_FAULTS = ["stressor", "server", "pkg", "dwc2d",
              "netem-loss", "netem-delay", "netem-reorder", "netem-blackout", "tcp-reset"]
NETEM_FAULTS = {"netem-loss", "netem-delay", "netem-reorder", "netem-blackout", "tcp-reset"}


def log(msg):
    print(f"chaos-host: {msg}", flush=True)


# ---- misbehaving server ---------------------------------------------------

SERVER_MODES = ["ok", "stall", "garbage", "reset", "half", "drip", "endless",
                "bad-length", "huge-header"]


class BadServer:
    """HTTP on `port`, TLS on `port + 1`. Each connection gets a random
    misbehaviour from SERVER_MODES (TLS adds a wrong certificate and
    garbage instead of a handshake)."""

    def __init__(self, port, seed):
        self.port, self.rng = port, random.Random(seed)
        self.counts = {}
        self.stop = threading.Event()
        self.tmp = tempfile.mkdtemp(prefix="chaos-host-")
        cert, key = os.path.join(self.tmp, "cert.pem"), os.path.join(self.tmp, "key.pem")
        # Self-signed, for a name the Pi does not ask for: pkg must refuse it.
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
                        "-keyout", key, "-out", cert, "-days", "2", "-subj", "/CN=chaos.invalid"],
                       check=True, capture_output=True)
        self.tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        self.tls.load_cert_chain(cert, key)
        self.socks = []
        for p, tls in ((port, False), (port + 1, True)):
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind(("0.0.0.0", p))
            s.listen(64)
            s.settimeout(0.5)
            self.socks.append(s)
            threading.Thread(target=self.accept_loop, args=(s, tls), daemon=True).start()

    def accept_loop(self, s, tls):
        while not self.stop.is_set():
            try:
                conn, _ = s.accept()
            except socket.timeout:
                continue
            except OSError:
                return
            threading.Thread(target=self.handle, args=(conn, tls), daemon=True).start()

    def handle(self, conn, tls):
        modes = SERVER_MODES + (["tls-garbage", "tls-reset"] if tls else [])
        mode = self.rng.choice(modes)
        key = ("tls:" if tls else "") + mode
        self.counts[key] = self.counts.get(key, 0) + 1
        conn.settimeout(30)
        try:
            if tls and mode == "tls-garbage":
                conn.sendall(os.urandom(self.rng.randint(1, 4096)))
                return
            if tls and mode == "tls-reset":
                conn.recv(512)
                self.reset(conn)
                return
            if tls:
                conn = self.tls.wrap_socket(conn, server_side=True)
            if mode == "reset":
                self.reset(conn)
                return
            conn.recv(4096)
            body = b"chaos\n" * 100
            if mode == "ok":
                conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: %d\r\nConnection: close\r\n\r\n" % len(body) + body)
            elif mode == "stall":
                self.stop.wait(self.rng.uniform(5, 60))
            elif mode == "garbage":
                conn.sendall(os.urandom(self.rng.randint(1, 65536)))
            elif mode == "half":
                conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\n" + body[:300])
                conn.shutdown(socket.SHUT_WR)
                self.stop.wait(5)
            elif mode == "drip":
                for b in b"HTTP/1.1 200 OK\r\nContent-Length: 600\r\n\r\n" + body:
                    if self.stop.is_set():
                        break
                    conn.sendall(bytes([b]))
                    time.sleep(self.rng.uniform(0, 0.2))
            elif mode == "endless":
                conn.sendall(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
                end = time.time() + self.rng.uniform(5, 30)
                while time.time() < end and not self.stop.is_set():
                    conn.sendall(b"4000\r\n" + os.urandom(0x4000) + b"\r\n")
            elif mode == "bad-length":
                conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: -5\r\nContent-Length: 99999999999999999999\r\n\r\n" + body)
            elif mode == "huge-header":
                conn.sendall(b"HTTP/1.1 200 OK\r\nX-Chaos: " + b"A" * (1 << 20) + b"\r\n\r\n")
        except (OSError, ssl.SSLError):
            pass
        finally:
            try:
                conn.close()
            except OSError:
                pass

    @staticmethod
    def reset(conn):
        # SO_LINGER with 0 s: close sends RST instead of FIN.
        conn.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, b"\x01\x00\x00\x00\x00\x00\x00\x00")
        conn.close()

    def close(self):
        self.stop.set()
        for s in self.socks:
            s.close()


# ---- network faults on this host ------------------------------------------

class Netem:
    """tc netem and iptables, only for the Pi's address."""

    def __init__(self, pi, iface, run_sudo):
        self.pi, self.iface, self.run_sudo = pi, iface, run_sudo
        self.active = []

    def commands(self, fault, seconds):
        dev, pi = self.iface, self.pi
        add_prio = [f"tc qdisc add dev {dev} root handle 1: prio",
                    f"tc filter add dev {dev} protocol ip parent 1:0 prio 3 u32 match ip dst {pi}/32 flowid 1:3"]
        netem = {
            "netem-loss": "loss 30% 25%",
            "netem-delay": "delay 400ms 200ms distribution normal",
            "netem-reorder": "delay 50ms reorder 25% 50% duplicate 5% corrupt 1%",
            "netem-blackout": "loss 100%",
        }
        if fault in netem:
            on = [add_prio[0], f"tc qdisc add dev {dev} parent 1:3 handle 30: netem {netem[fault]}", add_prio[1]]
            off = [f"tc qdisc del dev {dev} root"]
        elif fault == "tcp-reset":
            rule = f"INPUT -s {pi} -p tcp -j REJECT --reject-with tcp-reset"
            on = [f"iptables -I {rule}"]
            off = [f"iptables -D {rule}"]
        else:
            raise ValueError(fault)
        return on, off

    def sh(self, cmd):
        r = subprocess.run(["sudo", "-n"] + shlex.split(cmd), capture_output=True, text=True)
        if r.returncode:
            log(f"  `{cmd}` failed: {r.stderr.strip()}")
        return r.returncode == 0

    def apply(self, fault, seconds):
        on, off = self.commands(fault, seconds)
        if not self.run_sudo:
            log(f"  needs root; run on {socket.gethostname()}:")
            for c in on:
                log(f"    sudo {c}")
            log(f"    sleep {seconds}")
            for c in off:
                log(f"    sudo {c}")
            return False
        self.active = off
        ok = all(self.sh(c) for c in on)
        if not ok:
            self.clear()
        return ok

    def clear(self):
        for c in self.active:
            self.sh(c)
        self.active = []


def default_iface(addr):
    try:
        out = subprocess.run(["ip", "route", "get", addr], capture_output=True, text=True).stdout
        m = re.search(r"\bdev (\S+)", out)
        return m.group(1) if m else None
    except OSError:
        return None


def own_addr(addr):
    """The address of this host that `addr` reaches us on."""
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect((addr, 9))
        return s.getsockname()[0]
    finally:
        s.close()


# ---- checks on the Pi -----------------------------------------------------

def devsh_alive(host, port, timeout):
    """Seconds until devsh logs in and runs a command, or None after
    `timeout` seconds."""
    start = time.time()
    while time.time() - start < timeout:
        try:
            sh = Shell(host, port)
            out = sh.run("echo chaos-alive", timeout=30)
            sh.close()
            if "chaos-alive" in out:
                return time.time() - start
        except (OSError, RuntimeError, AttributeError):
            pass
        time.sleep(3)
    return None


def run_on_pi(host, port, command, timeout):
    """Output lines of one command, or None when devsh fails. A command
    that runs out of time is killed by name from a second session."""
    try:
        sh = Shell(host, port)
    except OSError as e:
        log(f"  devsh: {e}")
        return None
    try:
        return sh.run(command, timeout=timeout)
    except (OSError, RuntimeError) as e:
        log(f"  `{command}` gave no result in {timeout} s: {str(e).splitlines()[0]}")
        return None
    finally:
        try:
            sh.close()
        except OSError:
            pass


def kill_on_pi(host, port, name):
    out = run_on_pi(host, port, "ps", 30) or []
    for line in out:
        if name in line.split()[-1:] or line.rstrip().endswith("/" + name):
            pid = line.split()[0]
            if pid.isdigit():
                run_on_pi(host, port, f"kill -9 {pid}", 30)
                log(f"  killed {name} (pid {pid})")


def serial_check(path, mark):
    """(panics, crash lines) the serial log got since `mark`."""
    lines = serial_lines(path, mark)
    panics = [l for l in lines if PANIC.search(l)]
    crashes = [l for l in lines if CRASH.search(l) and l not in panics]
    return panics, crashes


# ---- faults ---------------------------------------------------------------

def fault_stressor(ctx, rng):
    seed = rng.getrandbits(48)
    cmd = f"chaos --rounds {ctx.args.stressor_rounds} --seed {seed} --quiet"
    log(f"  on the Pi: {cmd}")
    out = run_on_pi(ctx.host, ctx.port, cmd, ctx.args.stressor_rounds * 120 + 60)
    if out is None:
        kill_on_pi(ctx.host, ctx.port, "chaos")
        return "stressor run gave no result"
    for line in out:
        if "FAIL" in line or line.startswith("chaos: done") or "health" in line:
            log(f"  | {line}")
    done = [l for l in out if l.startswith("chaos: done")]
    if not done:
        return "stressor did not finish"
    if " 0 failed" not in done[-1]:
        return "stressor rounds failed: " + done[-1]
    return None


def fault_server(ctx, rng):
    target = f"{ctx.serve_addr}:{ctx.args.serve_port}"
    rounds = 1 + rng.randint(0, 3)
    cmd = f"chaos --kind net --net {target} --rounds {rounds} --seed {rng.getrandbits(48)} --timeout 90"
    log(f"  on the Pi: {cmd}")
    out = run_on_pi(ctx.host, ctx.port, cmd, rounds * 150 + 60)
    if out is None:
        kill_on_pi(ctx.host, ctx.port, "chaos")
        return "net client run gave no result"
    for line in out:
        if "net:" in line or "FAIL" in line:
            log(f"  | {line}")
    return None  # rounds cut off by the timeout are fine; recovery decides


def fault_pkg(ctx, rng):
    scheme, port = rng.choice([("http", ctx.args.serve_port), ("https", ctx.args.serve_port + 1)])
    url = f"{scheme}://{ctx.serve_addr}:{port}/pkg"
    log(f"  pkg against {url}")
    run_on_pi(ctx.host, ctx.port, f"echo {url} > /etc/pkg.d/05_chaos", 30)
    try:
        for _ in range(1 + rng.randint(0, 2)):
            out = run_on_pi(ctx.host, ctx.port, "pkg install --yes chaos-does-not-exist", 120)
            if out is None:
                kill_on_pi(ctx.host, ctx.port, "pkg")
            else:
                log(f"  | {(out or ['(no output)'])[-1][:160]}")
    finally:
        run_on_pi(ctx.host, ctx.port, "rm /etc/pkg.d/05_chaos", 30)
    return None


def fault_dwc2d(ctx, rng):
    what = "restart" if ctx.args.dwc2d_restart else "network"
    log(f"  echo {what} > /tmp/dwc2d-cycle (devsh drops with the network)")
    try:
        sh = Shell(ctx.host, ctx.port)
        sh.sock.sendall(f"echo {what} > /tmp/dwc2d-cycle\r".encode())
        time.sleep(2)
        sh.sock.close()
    except OSError as e:
        return f"could not send the request: {e}"
    return None


def fault_netem(ctx, rng, fault):
    if ctx.qemu:
        log("  skipped under QEMU")
        return "skip"
    seconds = rng.randint(20, 90)
    if not ctx.netem.apply(fault, seconds):
        return "skip"
    log(f"  {fault} for {seconds} s; devsh meanwhile:")
    end, tries, ok = time.time() + seconds, 0, 0
    try:
        while time.time() < end:
            tries += 1
            out = run_on_pi(ctx.host, ctx.port, "echo chaos-alive", 20)
            ok += bool(out and "chaos-alive" in out)
            time.sleep(2)
    finally:
        ctx.netem.clear()
    log(f"  {ok} of {tries} devsh commands answered during the fault")
    return None


FAULT_FN = {"stressor": fault_stressor, "server": fault_server, "pkg": fault_pkg, "dwc2d": fault_dwc2d}


class Ctx:
    pass


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("host")
    ap.add_argument("--port", type=int, default=2323)
    ap.add_argument("--rounds", type=int, default=10)
    ap.add_argument("--seed", type=int, default=None)
    ap.add_argument("--faults", default=",".join(ALL_FAULTS))
    ap.add_argument("--serve-addr", help="address the Pi reaches this host on (QEMU: 10.0.2.2)")
    ap.add_argument("--serve-port", type=int, default=18080)
    ap.add_argument("--serial-log")
    ap.add_argument("--recover", type=int, default=180, help="seconds the Pi has to answer again")
    ap.add_argument("--netem", action="store_true", help="run tc/iptables through sudo -n")
    ap.add_argument("--print-sudo", action="store_true", help="only print the tc/iptables commands")
    ap.add_argument("--iface")
    ap.add_argument("--dwc2d-restart", action="store_true", help="full controller restart (experiment dwc2d-restart)")
    ap.add_argument("--stressor-rounds", type=int, default=20)
    ap.add_argument("--log", help="append one JSON object per round")
    args = ap.parse_args()

    faults = [f for f in args.faults.split(",") if f]
    for f in faults:
        if f not in ALL_FAULTS:
            ap.error(f"unknown fault {f}; known: {', '.join(ALL_FAULTS)}")
    seed = args.seed if args.seed is not None else random.SystemRandom().getrandbits(32)
    rng = random.Random(seed)

    ctx = Ctx()
    ctx.args, ctx.host, ctx.port = args, args.host, args.port
    ctx.qemu = args.host in ("127.0.0.1", "localhost")
    ctx.serve_addr = args.serve_addr or (own_addr(args.host) if not ctx.qemu else "10.0.2.2")
    iface = args.iface or default_iface(args.host)
    ctx.netem = Netem(args.host, iface, args.netem and not args.print_sudo)

    server = None
    if {"server", "pkg"} & set(faults):
        server = BadServer(args.serve_port, seed)
        log(f"misbehaving server on ports {args.serve_port} (HTTP) and {args.serve_port + 1} (TLS), "
            f"the Pi reaches it at {ctx.serve_addr}")

    log(f"start seed={seed} rounds={args.rounds} faults={','.join(faults)} target={args.host}:{args.port}")
    first = devsh_alive(args.host, args.port, 60)
    if first is None:
        log("devsh does not answer before the first round, stopping")
        return 1

    failed = skipped = 0
    try:
        for r in range(1, args.rounds + 1):
            fault = rng.choice(faults)
            round_seed = rng.getrandbits(32)
            log(f"round {r}/{args.rounds} fault={fault} seed={round_seed}")
            mark = serial_size(args.serial_log)
            start = time.time()
            rrng = random.Random(round_seed)
            problem = fault_netem(ctx, rrng, fault) if fault in NETEM_FAULTS else FAULT_FN[fault](ctx, rrng)
            if problem == "skip":
                skipped += 1
                log(f"round {r} skipped")
                continue
            back = devsh_alive(args.host, args.port, args.recover)
            time.sleep(1)
            panics, crashes = serial_check(args.serial_log, mark) if args.serial_log else ([], [])
            result = "ok"
            reasons = []
            if back is None:
                reasons.append(f"devsh did not answer within {args.recover} s")
            if panics:
                reasons.append(f"{len(panics)} panic line(s): {panics[0][:160]}")
            if problem:
                reasons.append(problem)
            if reasons:
                result = "FAIL"
                failed += 1
            secs = time.time() - start
            log(f"round {r} {result} ({secs:.0f} s, devsh back after "
                f"{'never' if back is None else f'{back:.0f} s'}"
                f"{f', {len(crashes)} crash dump lines' if crashes else ''})"
                + (": " + "; ".join(reasons) if reasons else ""))
            for line in crashes[:6]:
                log(f"  serial: {line[:160]}")
            if result == "FAIL":
                log(f"  repeat (rounds 1 to {r}): scripts/chaos-host.py {args.host} --faults {args.faults} --seed {seed} --rounds {r}")
            if args.log:
                with open(args.log, "a") as f:
                    f.write(json.dumps({"time": time.strftime("%Y-%m-%dT%H:%M:%S"), "seed": seed, "round": r,
                                        "fault": fault, "round_seed": round_seed, "result": result,
                                        "reasons": reasons, "recover_s": back, "panics": panics[:5],
                                        "crashes": crashes[:20]}) + "\n")
            if back is None:
                log("the Pi does not answer any more, stopping")
                break
    finally:
        ctx.netem.clear()
        if server:
            log(f"server connections by mode: {dict(sorted(server.counts.items()))}")
            server.close()
    log(f"done: {args.rounds} rounds, {failed} failed, {skipped} skipped, seed={seed}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
