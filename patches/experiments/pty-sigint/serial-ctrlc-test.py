#!/usr/bin/env python3
"""Ctrl-C check on the serial console (issue #86).

Logs in as root on the serial console and, for each delay, starts
`sleep 40`, sends Ctrl-C after that many seconds and waits up to 50 s for
Ion's "ended by signal SIGINT". It prints how long that took after the
Ctrl-C: on Pi test 80 it came only when `sleep` ended by itself.

  serial-ctrlc-test.py --qemu <image> [--delays 0,0.1,0.3,1,3]
  serial-ctrlc-test.py --device /dev/ttyUSB0 [--delays ...]

--qemu boots the image with scripts/qemu-raspi3b.sh (run it from the
repository's top directory). --device uses a serial port at 115200 baud;
the Pi must show its login prompt, or a root shell. Nothing else may read
the port meanwhile (stop a logging `cat` first).
"""
import argparse, os, pty, re, select, sys, termios, time, tty

PROMPT = rb"# \x1b\[0m"
ap = argparse.ArgumentParser()
ap.add_argument("--qemu")
ap.add_argument("--device")
ap.add_argument("--delays", default="0,0.1,0.3,1,3")
ap.add_argument("--password", default="password")
ap.add_argument("--log", default="serial-ctrlc.log")
args = ap.parse_args()
if bool(args.qemu) == bool(args.device):
    sys.exit("give --qemu or --device")

pid = None
if args.qemu:
    env = dict(os.environ, DEVSH_PORT="12423", SSH_PORT="12422")
    pid, fd = pty.fork()
    if pid == 0:
        os.execvpe("scripts/qemu-raspi3b.sh",
                   ["qemu-raspi3b.sh", "--image", args.qemu, "--usb-net"], env)
else:
    fd = os.open(args.device, os.O_RDWR | os.O_NOCTTY)
    tty.setraw(fd)
    attrs = termios.tcgetattr(fd)
    attrs[4] = attrs[5] = termios.B115200
    termios.tcsetattr(fd, termios.TCSANOW, attrs)

log = open(args.log, "wb")
buf = b""


def read(timeout):
    global buf
    if select.select([fd], [], [], timeout)[0]:
        try:
            data = os.read(fd, 4096)
        except OSError:
            sys.exit("serial side closed")
        log.write(data)
        log.flush()
        buf += data


def drain():
    """Reads until the port is quiet for 1 s and forgets it."""
    global buf
    n = -1
    while n != len(buf):
        n = len(buf)
        read(1)
    buf = b""


def wait(pattern, timeout):
    """Waits for the pattern and drops the output up to it; returns the match."""
    global buf
    end = time.time() + timeout
    while time.time() < end:
        m = re.search(pattern, buf)
        if m:
            buf = buf[m.end():]
            return m.group(0)
        read(0.05)
    return None


if args.qemu:
    if not wait(rb"Arrow keys and enter select mode", 300):
        sys.exit("no boot menu")
    os.write(fd, b"\r")
    if not wait(rb"login:", 600):
        sys.exit("no login prompt")
    time.sleep(3)
else:
    os.write(fd, b"\r")
seen = wait(rb"login:|" + PROMPT, 10)
if seen is None:
    sys.exit("neither login prompt nor shell")
if seen.endswith(b"login:"):
    os.write(fd, b"root\r")
    if not wait(rb"assword", 30):
        sys.exit("no password prompt")
    # login turns echo off after printing the prompt; keys before that are lost
    time.sleep(2)
    os.write(fd, args.password.encode() + b"\r")
    if not wait(PROMPT, 60):
        sys.exit("no shell prompt")
time.sleep(5)

late = 0
for delay in [float(d) for d in args.delays.split(",")]:
    drain()
    os.write(fd, b"sleep 40\r")
    time.sleep(delay)
    os.write(fd, b"\x03")
    t0 = time.time()
    # Ion ends an accepted line with a newline; a Ctrl-C that comes while
    # Ion still reads the line cancels it, and nothing is started.
    accepted = ended = False
    while time.time() - t0 < 50:
        read(0.05)
        accepted = accepted or re.search(rb"sleep 40\r*\n", buf) is not None
        ended = b"ended by signal SIGINT" in buf
        if ended or (not accepted and time.time() - t0 > 3):
            break
    took = time.time() - t0
    if ended:
        what = f"stopped by SIGINT {took:.1f} s after Ctrl-C"
    elif not accepted:
        what = "line cancelled before start"
    else:
        what = "NOT stopped within 50 s"
    late += accepted and (not ended or took > 2)
    print(f"Ctrl-C {delay:.2f} s after Enter: {what}", flush=True)
    if not wait(PROMPT, 45):
        sys.exit("no prompt after the round")

print(f"{'PASS' if late == 0 else 'FAIL'}: {late} late of "
      f"{len(args.delays.split(','))} (late: more than 2 s)")
if pid:
    os.kill(pid, 9)
sys.exit(1 if late else 0)
