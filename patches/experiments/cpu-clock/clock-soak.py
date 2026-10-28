#!/usr/bin/env python3
"""Pi test #13: 30 minutes of start loops at full clock, temperature and
throttling after each round. Usage: clock-soak.py <repo> <host> [minutes]"""
import importlib.util, sys, time
spec = importlib.util.spec_from_file_location("pitest", sys.argv[1] + "/scripts/pi-test.py")
pitest = importlib.util.module_from_spec(spec); spec.loader.exec_module(pitest)
FW = "/scheme/rpi-firmware"
LOOP = "for i in 1..=25; ls / > /dev/null || echo FAILED ls; pwd > /dev/null || echo FAILED pwd; end"
sh = pitest.Shell(sys.argv[2], 2323)
CLOCK = sys.argv[4] if len(sys.argv) > 4 else None
if CLOCK:
    sh.run(f"echo {CLOCK} > {FW}/arm_clock")
    print("clock set:", sh.run(f"cat {FW}/arm_clock"), flush=True)
end = time.time() + 60 * float(sys.argv[3] if len(sys.argv) > 3 else 30)
starts = failed = 0
peak = 0
flags_seen = set()
t0 = time.time()
session_errors = 0
try:
    while time.time() < end:
        try:
            out = sh.run(LOOP, timeout=600)
            starts += 50
            failed += sum(1 for l in out if l.startswith("FAILED"))
            clock, temp, flags = sh.run(f"cat {FW}/arm_clock {FW}/temperature {FW}/throttled")[:3]
        except (RuntimeError, OSError, ValueError) as e:
            session_errors += 1
            print(f"{(time.time()-t0)/60:5.1f} min: session error, reconnecting: {str(e).splitlines()[0]}", flush=True)
            try:
                sh.sock.close()
            except OSError:
                pass
            while True:
                time.sleep(5)
                try:
                    sh = pitest.Shell(sys.argv[2], 2323)
                    if CLOCK:
                        sh.run(f"echo {CLOCK} > {FW}/arm_clock")
                    break
                except (RuntimeError, OSError, ValueError):
                    session_errors += 1
                    if time.time() > end:
                        raise
            continue
        peak = max(peak, int(temp)); flags_seen.add(flags)
        print(f"{(time.time()-t0)/60:5.1f} min: {starts} starts, {failed} failed, ARM {int(clock)//1000000} MHz, {int(temp)/1000:.1f} C, throttled {flags}", flush=True)
finally:
    sh.close()
print(f"SUMMARY: {starts} starts, {failed} failed, {session_errors} session errors, peak {peak/1000:.1f} C, throttled values {sorted(flags_seen)}")
