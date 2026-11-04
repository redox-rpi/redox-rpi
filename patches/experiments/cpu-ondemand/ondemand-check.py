#!/usr/bin/env python3
"""Pi test for issue #23: the governor goes to the minimum when idle, and 50
starts are timed in the modes ondemand, performance and powersave. Then a
soak: rounds of 50 starts with 3 s pauses, so the clock goes up and down
each round. Uses the Shell class of scripts/pi-test.py.
Usage: ondemand-check.py <repo> <host> [rounds] [soak minutes]"""

import importlib.util
import sys
import time

spec = importlib.util.spec_from_file_location("pitest", sys.argv[1] + "/scripts/pi-test.py")
pitest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pitest)

host = sys.argv[2]
rounds = int(sys.argv[3]) if len(sys.argv) > 3 else 2
soak = float(sys.argv[4]) if len(sys.argv) > 4 else 0
FW = "/scheme/rpi-firmware"
LOOP = "for i in 1..=25; ls / > /dev/null || echo FAILED ls; pwd > /dev/null || echo FAILED pwd; end"

sh = pitest.Shell(host, 2323)


def governor():
    out = sh.run(f"cat {FW}/governor {FW}/arm_clock")
    return " ".join(out)


def timed_loop():
    t = time.time()
    out = sh.run(LOOP, timeout=600)
    return time.time() - t, sum(1 for l in out if l.startswith("FAILED"))


try:
    sh.run(f"echo ondemand > {FW}/governor")
    time.sleep(5)
    print("idle:", governor(), flush=True)
    for _ in range(rounds):
        for mode in ["ondemand", "performance", "powersave"]:
            sh.run(f"echo {mode} > {FW}/governor")
            time.sleep(3)
            seconds, failed = timed_loop()
            print(f"{mode}: 50 starts in {seconds:.1f} s, {failed} failed; now {governor()}", flush=True)
    sh.run(f"echo ondemand > {FW}/governor")
    time.sleep(5)
    print("after timing:", governor(), flush=True)
    end = time.time() + 60 * soak
    starts = failed = 0
    while time.time() < end:
        _, n = timed_loop()
        starts += 50
        failed += n
        time.sleep(3)
        print(f"soak: {failed} of {starts} starts failed; {governor()}", flush=True)
    print("end:", governor(), sh.run(f"cat {FW}/temperature {FW}/throttled"), flush=True)
finally:
    sh.close()
