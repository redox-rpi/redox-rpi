#!/usr/bin/env python3
"""Speed benchmark for the opt-a53 experiment: the same workloads on image A
(stable) and image B (opt-a53), timed from the build host, through devshd.

Usage:
    patches/experiments/opt-a53/bench.py <pi-host> --label A|B
                       [--port 2323] [--rounds 3] [--local-ip <host ip>]
                       [--out bench.jsonl]

Workloads (each round):
  spawn     500 starts of `ls /usr` in one Ion loop: kernel, relibc, ld.so
  net-rx    32 MiB from this host to `nc -l` on the Pi: netstack, dwc2d
  net-tx    32 MiB from the Pi to this host with `nc`: netstack, dwc2d
  sd-write  50 MiB written to /scheme/data with dd: sdhost driver, RedoxFS
  sd-read   the same file read back with dd

Times are wall-clock seconds on this host around each command, so the round
trip to the Pi (a few ms) is included. Appends one JSON object per
measurement to --out and prints the median of each workload.
"""

import argparse
import importlib.util
import json
import os
import socket
import statistics
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "pitest", os.path.join(HERE, "..", "..", "..", "scripts", "pi-test.py"))
pitest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pitest)

NET_BYTES = 32 * 1024 * 1024
PORT = 5135


def local_ip_for(host):
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.connect((host, 9))
    ip = s.getsockname()[0]
    s.close()
    return ip


def timed(shell, command, timeout=600):
    t = time.monotonic()
    out = shell.run(command, timeout)
    return time.monotonic() - t, out


def net_rx(shell, host):
    """The Pi listens and counts; this host sends."""
    result = {}

    def listen():
        result["out"] = shell.run(f"nc -l 0.0.0.0:{PORT} | wc -c", 600)

    th = threading.Thread(target=listen)
    th.start()
    time.sleep(2)
    data = b"\0" * (1024 * 1024)
    t = time.monotonic()
    with socket.create_connection((host, PORT), timeout=60) as s:
        for _ in range(NET_BYTES // len(data)):
            s.sendall(data)
    th.join()
    return time.monotonic() - t, result.get("out")


def net_tx(shell, local_ip):
    """This host listens and counts; the Pi sends /tmp/tx."""
    srv = socket.create_server(("", PORT))
    srv.settimeout(60)
    got = {"bytes": 0}

    def accept():
        conn, _ = srv.accept()
        conn.settimeout(60)
        got["t0"] = time.monotonic()
        while True:
            chunk = conn.recv(1 << 16)
            if not chunk:
                break
            got["bytes"] += len(chunk)
        got["t1"] = time.monotonic()
        conn.close()

    th = threading.Thread(target=accept)
    th.start()
    shell.run(f"nc {local_ip}:{PORT} < /tmp/tx", 600)
    th.join()
    srv.close()
    return got["t1"] - got["t0"], [str(got["bytes"])]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("host")
    ap.add_argument("--label", required=True)
    ap.add_argument("--port", type=int, default=2323)
    ap.add_argument("--rounds", type=int, default=3)
    ap.add_argument("--local-ip")
    ap.add_argument("--out", default="bench.jsonl")
    a = ap.parse_args()
    local_ip = a.local_ip or local_ip_for(a.host)

    shell = pitest.Shell(a.host, a.port)
    build = shell.run("cat /etc/redox-rpi-release")
    clocks = shell.run("cat /scheme/rpi-firmware/clocks")
    shell.run(f"dd if=/dev/zero of=/tmp/tx bs=65536 count={NET_BYTES // 65536}")

    results = {}
    with open(a.out, "a") as log:
        for rnd in range(1, a.rounds + 1):
            runs = {
                "spawn": lambda: timed(shell, 'ion -c "for i in 1..=500; ls /usr > /dev/null; end"'),
                "net-rx": lambda: net_rx(shell, a.host),
                "net-tx": lambda: net_tx(shell, local_ip),
                "sd-write": lambda: timed(shell, "dd if=/dev/zero of=/scheme/data/opt-bench bs=65536 count=800"),
                "sd-read": lambda: timed(shell, "dd if=/scheme/data/opt-bench of=/dev/null bs=65536"),
            }
            for name, run in runs.items():
                secs, out = run()
                results.setdefault(name, []).append(secs)
                rec = {"label": a.label, "build": build, "round": rnd, "workload": name,
                       "seconds": round(secs, 3), "output": out,
                       "time": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
                log.write(json.dumps(rec) + "\n")
                log.flush()
                print(f"{a.label} round {rnd} {name}: {secs:.2f} s", flush=True)
            shell.run("rm /scheme/data/opt-bench")
    shell.run("rm /tmp/tx")
    clocks_after = shell.run("cat /scheme/rpi-firmware/clocks")
    shell.close()

    print(f"\n{a.label}: {' '.join(build)}")
    print("clocks before:", " | ".join(clocks))
    print("clocks after: ", " | ".join(clocks_after))
    print("| Workload | Median (s) | Runs (s) |\n|---|---|---|")
    for name, secs in results.items():
        print(f"| {name} | {statistics.median(secs):.2f} | {', '.join(f'{s:.2f}' for s in secs)} |")


if __name__ == "__main__":
    main()
