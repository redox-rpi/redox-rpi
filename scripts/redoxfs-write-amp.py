#!/usr/bin/env python3
"""Measure how many bytes RedoxFS writes to the disk for small file writes.

Boots a Redox disk image in QEMU, logs in on the serial console and writes
1 MiB to a file in the home directory with `dd`, in calls of 4 KiB, 64 KiB
and 1 MiB (`conv=fsync`, so the data is on the disk when `dd` returns).
QEMU traces every write that reaches the disk image (trace event
`blk_co_pwritev`, offset and length of each write); the script adds up the
writes during each run. Nothing in the guest has to be changed, and the
numbers mean the same on every architecture and disk controller (QEMU's
`query-blockstats` would be simpler, but its SD card does not count). The
result is the bytes the disk saw per byte `dd` wrote.

A run without writing (the shell only waits) gives the background writes of
the idle system, for comparison.

Needs Python 3 and QEMU (for x86_64 also OVMF). The guest shell must be Ion,
the Redox default. Works on a copy of the image; the image itself is not
changed.

Usage, x86_64 (QEMU q35, disk on AHCI), for example the image from
`make all ARCH=x86_64 CONFIG_NAME=minimal` in the Redox build system:

    scripts/redoxfs-write-amp.py build/x86_64/minimal/harddrive.img

Any other QEMU setup: give the QEMU command after `--`, with `{image}`
where the disk image goes. The script adds the serial console on stdio as
the first serial port and the trace, so do not give those:

    scripts/redoxfs-write-amp.py harddrive.img -- qemu-system-aarch64 \\
        -M raspi3b ... -drive file={image},if=sd,format=raw

Options: --user/--password (default root/password), --repeat (default 3),
--timeout (seconds for the boot, default 600).
"""

import argparse
import os
import re
import select
import shutil
import subprocess
import sys
import tempfile
import time

TOTAL = 1024 * 1024
CALL_SIZES = [4096, 65536, 1048576]
OVMF = ["/usr/share/ovmf/OVMF.fd", "/usr/share/OVMF/OVMF.fd", "/usr/share/qemu/OVMF.fd",
        "/usr/share/edk2/x64/OVMF.fd", "/usr/share/edk2-ovmf/x64/OVMF.fd"]
# The Redox bootloader's resolution menu, answered with Enter.
BOOT_MENU = b"Arrow keys and enter select mode"
# blk_co_pwritev blk 0x... bs 0x... offset 1234 bytes 4096 flags 0x0
TRACE_WRITE = re.compile(rb"blk_co_pwritev .*?bytes (\d+)")
ANSI = re.compile(rb"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07")


class Guest:
    """QEMU with the guest's serial console on a pipe and its disk writes
    traced to a file."""

    def __init__(self, command, workdir, log_path):
        self.trace_path = os.path.join(workdir, "trace.log")
        full = [command[0], "-serial", "stdio", "-trace", f"enable=blk_co_pwritev,file={self.trace_path}"] + command[1:]
        self.log = open(log_path, "wb")
        self.proc = subprocess.Popen(full, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.output = b""

    def read(self, timeout):
        """Collect console output for up to `timeout` seconds."""
        end = time.time() + timeout
        while time.time() < end:
            ready, _, _ = select.select([self.proc.stdout], [], [], max(0, end - time.time()))
            if not ready:
                break
            chunk = os.read(self.proc.stdout.fileno(), 65536)
            if not chunk:
                raise RuntimeError("QEMU exited")
            self.log.write(chunk)
            self.log.flush()
            self.output += chunk
            return

    def wait_for(self, pattern, timeout, answer_menu=False):
        """Wait until the console output after the last call matches."""
        end = time.time() + timeout
        start = len(self.output)
        answered = False
        while time.time() < end:
            self.read(1)
            text = ANSI.sub(b"", self.output[start:])
            if answer_menu and not answered and BOOT_MENU in text:
                self.type("")
                answered = True
            if re.search(pattern, text):
                return text
        raise RuntimeError(f"timeout waiting for {pattern!r}, see the console log")

    def type(self, text):
        # Slowly: a Redox console line discipline can drop bytes typed fast.
        for ch in text.encode() + b"\r":
            self.proc.stdin.write(bytes([ch]))
            self.proc.stdin.flush()
            time.sleep(0.01)

    def run(self, command, marker, timeout=600):
        """Run a shell command and wait for it to finish: the next line
        prints a marker whose text only appears in the output."""
        self.type(command)
        self.type(f"echo WA-END-$((40 + {marker}))")
        self.wait_for(rf"WA-END-{40 + marker}\b".encode(), timeout)

    def disk_writes(self):
        """Bytes and number of writes the disk image got so far."""
        with open(self.trace_path, "rb") as f:
            lengths = [int(m) for m in TRACE_WRITE.findall(f.read())]
        return sum(lengths), len(lengths)

    def close(self):
        self.proc.kill()
        self.proc.wait()
        self.log.close()


def default_command():
    command = ["qemu-system-x86_64", "-machine", "q35", "-m", "2048", "-smp", "2",
               "-drive", "file={image},format=raw", "-display", "none", "-nic", "none"]
    # Redox boots x86_64 through UEFI, like `make qemu`; the BIOS boot path
    # shows nothing on the serial console.
    for firmware in OVMF:
        if os.path.exists(firmware):
            command += ["-bios", firmware]
            break
    else:
        sys.exit("no UEFI firmware (OVMF) found, install ovmf or give the QEMU command")
    if os.access("/dev/kvm", os.R_OK | os.W_OK):
        command += ["-enable-kvm", "-cpu", "host"]
    return command


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("image")
    parser.add_argument("--user", default="root")
    parser.add_argument("--password", default="password")
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--log", default="redoxfs-write-amp.log", help="console log (default: %(default)s)")
    argv = sys.argv[1:]
    split = argv.index("--") if "--" in argv else len(argv)
    args = parser.parse_args(argv[:split])
    command = argv[split + 1:] or default_command()

    workdir = tempfile.mkdtemp(prefix="redoxfs-write-amp-")
    image = os.path.join(workdir, "disk.img")
    shutil.copyfile(args.image, image)
    guest = Guest([part.replace("{image}", image) for part in command], workdir, args.log)
    try:
        print(f"booting {args.image} ({command[0]}), console log in {args.log}", file=sys.stderr)
        guest.wait_for(rb"login: ?$", args.timeout, answer_menu=True)
        time.sleep(2)
        guest.type(args.user)
        guest.wait_for(rb"assword: ?$", 30)
        time.sleep(2)
        guest.type(args.password)
        guest.wait_for(rb"[#$] ?$", 60)
        # Let the boot settle: services may still write to the disk.
        time.sleep(20)
        guest.run("rm -f wa.bin", 0)

        rows = []
        marker = 1
        for _ in range(args.repeat):
            before = guest.disk_writes()
            guest.run("sleep 5", marker)
            marker += 1
            time.sleep(2)
            after = guest.disk_writes()
            rows.append(("idle (sleep 5)", 0, after[0] - before[0], after[1] - before[1]))
        for size in CALL_SIZES:
            for _ in range(args.repeat):
                before = guest.disk_writes()
                guest.run(f"dd if=/scheme/rand of=wa.bin bs={size} count={TOTAL // size} conv=fsync", marker)
                marker += 1
                time.sleep(2)
                after = guest.disk_writes()
                rows.append((f"{size // 1024} KiB", TOTAL, after[0] - before[0], after[1] - before[1]))
                guest.run("rm -f wa.bin", marker)
                marker += 1
                time.sleep(2)
    finally:
        guest.close()
        shutil.rmtree(workdir, ignore_errors=True)

    print("| Call size | Bytes written by dd | Bytes the disk saw | Disk writes | Factor |")
    print("|---|---|---|---|---|")
    for name, written, seen, ops in rows:
        factor = f"{seen / written:.1f}" if written else "-"
        print(f"| {name} | {written} | {seen} | {ops} | {factor} |")


if __name__ == "__main__":
    main()
