#!/usr/bin/env python3
"""Pi test #13 (issue #20): start loop timed at the minimum and maximum ARM
clock in the same boot, plus temperature and throttling. Uses the Shell
class of scripts/pi-test.py. Usage: clock-timing.py <repo> <host> [rounds]"""

import sys
import time

sys.path.insert(0, sys.argv[1] + "/scripts")
import importlib.util

spec = importlib.util.spec_from_file_location("pitest", sys.argv[1] + "/scripts/pi-test.py")
pitest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pitest)

host = sys.argv[2]
rounds = int(sys.argv[3]) if len(sys.argv) > 3 else 3
FW = "/scheme/rpi-firmware"
LOOP = "for i in 1..=25; ls / > /dev/null || echo FAILED ls; pwd > /dev/null || echo FAILED pwd; end"

sh = pitest.Shell(host, 2323)


def state():
    out = sh.run(f"cat {FW}/arm_clock {FW}/temperature {FW}/throttled")
    return " ".join(out)


def timed_loop():
    t = time.time()
    out = sh.run(LOOP, timeout=600)
    return time.time() - t, sum(1 for l in out if l.startswith("FAILED"))


try:
    print("start:", state(), flush=True)
    for setting in ["min", "max"] * rounds:
        sh.run(f"echo {setting} > {FW}/arm_clock")
        seconds, failed = timed_loop()
        print(f"{setting}: 50 starts in {seconds:.1f} s, {failed} failed; now {state()}", flush=True)
    t = time.time()
    sh.run("echo max > /scheme/rpi-firmware/arm_clock")
    print("end:", state(), flush=True)
finally:
    sh.close()
