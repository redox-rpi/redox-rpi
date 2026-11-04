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
```

Power cuts are not in the script: the Shelly plug is switched only by the
Pi test agent (separate test-plan row).

## Results
