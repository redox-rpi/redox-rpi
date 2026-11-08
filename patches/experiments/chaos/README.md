# Experiment: chaos, fault injection and recovery checks

Issue [#196](https://github.com/redox-rpi/redox-rpi/issues/196). Adds
[`tools/chaos`](../../../tools/chaos) to the image: a stressor in C, built
statically against our relibc, so it also exercises relibc's C side. The
host side is [`scripts/chaos-host.py`](../../../scripts/chaos-host.py).

```bash
scripts/build-image.sh --experiment chaos
```

## On the Pi: `chaos`

Randomized rounds; each round runs one fault in a child process with a
deadline (`--timeout`, 60 s), then checks that the system still answers:
fork + exit, exec of `chaos --noop`, a file in `/tmp` written and read back,
`/scheme/sys/uname` readable. A child that `SIGKILL` cannot remove within
10 s counts as `UNKILLABLE`.

```text
chaos --list                         # the fault kinds
chaos --rounds 50 --seed 1           # 50 random rounds, repeatable
chaos --kind fds,signals --rounds 10 # only these kinds
chaos --kind schemes --round-seed 0x1234   # repeat one round
```

| Kind | Fault |
|---|---|
| `memory` | allocate and touch 1 MiB chunks until malloc fails or only `--mem-reserve` MiB (48) are free, free them in random order; `--mem-reserve 0` goes to real exhaustion |
| `procs` | 2–5 waves of 8–63 forks that exit at once, a quarter with a grandchild; every exit code is checked |
| `threads` | 2–5 waves of 4–63 threads counting on one mutex; the total must add up |
| `signals` | 1–3 children send `SIGUSR1`/`SIGUSR2` as fast as they can while the parent writes and reads a pipe, with or without `SA_RESTART`; values must come back right |
| `fds` | open `/scheme/null`, `zero`, `rand`, `/tmp`, pipes and dups until `EMFILE` or 4096, close in random order, open again |
| `schemes` | 200–1000 random calls on the schemes in `/scheme`: `open` with odd paths (`..`, control bytes, 1 KiB names) and flags, `read`, `write`, `fcntl` with unknown commands, `lseek` with huge offsets and wrong `whence`, `fstat`, `close` twice, invalid fds. Never opened: disk, memory, irq, proc, debug, pty, input, display, the drivers, network, `log`, `tcp`/`udp`, `event`, `shm`; writes only to `null`, `zero`, `pipe`; `sys/kstop` and friends are never opened |
| `fill` | write `/scheme/data/chaos-fill` until `ENOSPC` (or `--fill-max` MiB), try a small write while full, delete, check the space comes back. Skipped without `/scheme/data` |
| `killio` | 2–7 workers writing + `fsync` in `/tmp` or `/scheme/data`, blocked on a pipe, or reading `/scheme/sys/context`, each `SIGKILL`ed after 0–300 ms; they must be gone within 10 s and their files removable |
| `net` | only with `--net HOST:PORT`: 5–30 TCP connections to `chaos-host.py`'s misbehaving server, sending garbage, half-closing, closing early or reading slowly |

Each round prints `chaos: round N/M kind=K seed=0x…`, then `ok` or `FAIL`
with the repeat command. Exit code 0 when all rounds passed.

## On the build host: `scripts/chaos-host.py`

Faults from outside: the on-Pi stressor over devsh, the `net` client and
`pkg` against a misbehaving HTTP/TLS server on the build host (stalls,
garbage, resets, half-closed and endless replies, a self-signed
certificate, garbage instead of a TLS handshake), forced `dwc2d` network
port power cycles (`echo network > /tmp/dwc2d-cycle`; `--dwc2d-restart`
for the controller restart of experiment `dwc2d-restart`), and network
loss, delay, reordering, blackouts and TCP resets with `tc netem` and
iptables, only for the Pi's address. After each fault devsh must answer
within `--recover` seconds (180) and the serial log must have no panic.

```bash
# real Pi (the netem faults run through sudo -n with --netem)
scripts/chaos-host.py 192.168.178.148 --serial-log ~/redox-rpi/logs/serial.log --rounds 20 --netem
# only print the tc/iptables commands
scripts/chaos-host.py 192.168.178.148 --faults netem-loss --print-sudo
# QEMU (scripts/qemu-raspi3b.sh --usb-net, DEVSH_PORT=12423)
scripts/chaos-host.py 127.0.0.1 --port 12423 --serve-addr 10.0.2.2 --faults stressor,server,pkg,dwc2d
```

The netem commands, if you run them by hand (`enp12s0` on homenodellm):

```bash
sudo tc qdisc replace dev enp12s0 root handle 1: prio
sudo tc qdisc add dev enp12s0 parent 1:3 handle 30: netem loss 30% 25%
sudo tc filter add dev enp12s0 protocol ip parent 1:0 prio 3 u32 match ip dst 192.168.178.148/32 flowid 1:3
sudo tc qdisc del dev enp12s0 root        # end of the fault
sudo iptables -I INPUT -s 192.168.178.148 -p tcp -j REJECT --reject-with tcp-reset
sudo iptables -D INPUT -s 192.168.178.148 -p tcp -j REJECT --reject-with tcp-reset
# homenodellm has no iptables; there tcp-reset does this instead:
for i in $(seq 60); do sudo ss -K dst 192.168.178.148; sleep 1; done
```

Power cuts are not in the script: the Shelly plug is switched only by the
Pi test agent (separate test-plan row).

## Results

QEMU raspi3b, 2026-11-04, `v0.4.0-dev-346-g5bc468d7+xa1cf` (and the two
builds before it, which differ only in `fill`, `net` and diagnostics),
`--usb-net --data` (256 MiB data partition):

- **Stressor:** 70 rounds over all kinds. Everything passes except
  `signals`, which fails every time: children send thousands of signals
  with `kill()`, it returns 0, and the handler never runs
  ([#205](https://github.com/redox-rpi/redox-rpi/issues/205)). `memory`
  goes down to 41–44 MiB free and gets it all back; `fds` reaches 4096
  without `EMFILE`; `fill` writes 253 MiB in 93–133 s until `ENOSPC`, a
  small write still works, and the space comes back after deleting;
  `killio` kills every worker within 10 s; `schemes` makes 380–820 calls on
  8 schemes with no hang.
- **Host faults:** 26 rounds over `server`, `pkg`, `dwc2d` and `stressor`.
  devsh is back after 2–5 s each time. The QEMU clock is at 1970, so `pkg`
  over TLS stops at the certificate date check. Over plain HTTP it reports
  "response header is too big" and "Peer disconnected" cleanly. A
  `network` port cycle in QEMU only forgets and finds the adapter again
  (QEMU's hub cannot disconnect it).
- No panic and no crash dump in the serial log during any run.
- Early runs found two bugs in the test itself, both fixed: `fill` ran
  into the round deadline on the slow card and left its file behind, and
  `net` rounds against stalling replies were cut off by the deadline.

Real Pi 3B+, Pi tests 237–241 (2026-11-04/05, Pi test agent; reports in
[test-reports.md](../../../docs/test-reports.md)):

- **237:** pi-test passed, and the first 40 stressor rounds too. The second
  run froze the whole Pi
  ([#207](https://github.com/redox-rpi/redox-rpi/issues/207)). On a fresh
  boot, `signals` failed 3 of 3 (#205), and 19 of 20 host faults
  recovered within 1–6 s (delay, reordering, loss, pkg, server, dwc2d).
  After round 20, an 85 s delay, the network did not come back (#92
  picture). homenodellm has no iptables, so `tcp-reset` now uses `ss -K`.
- **238, 240:** with the fix from PR #209, the freeze still came, but more
  rarely: once in the first run in 238, once in 7 runs in 240.
- **241:** with the fix and lock-wait watches, 34 runs of 40 rounds (1360
  rounds) passed without a freeze.
