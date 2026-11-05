# Chaos test

Fault injection that checks Redox stays up and recovers. Issue
[#196](https://github.com/redox-rpi/redox-rpi/issues/196); belongs to the
stability topic (label `topic:stability`).

## State

Nothing on `main` yet.

## In progress

- Branch `chaos-test` (PR for #196), experiment
  [`chaos`](../../patches/experiments/chaos/README.md):
  - [`tools/chaos`](../../tools/chaos), in C against our relibc: random
    rounds of memory exhaustion, fork and thread storms, signal storms, fd
    exhaustion, a scheme fuzzer, a full `/scheme/data` and processes killed
    mid-I/O. Each round has a seed it can be repeated with, and a health
    check after it.
  - [`scripts/chaos-host.py`](../../scripts/chaos-host.py) on the build host:
    the stressor over devsh, a misbehaving HTTP/TLS server for a TCP client
    and for `pkg`, forced `dwc2d` port power cycles, and `tc netem` /
    iptables network faults for the Pi's address only. After each fault
    devsh must answer and the serial log must have no panic.
- QEMU, 2026-11-04: 30 stressor rounds, 29 pass; the misses are
  `signals`, see below. 18 host faults (server, pkg, dwc2d), all recover;
  no panic, no crash dump in the serial log. Pi test 237 is queued.

## Findings

- [#207](https://github.com/redox-rpi/redox-rpi/issues/207): the `threads`
  round froze the whole Pi (test 237) and QEMU. Cause: a context switch
  waited for a context lock without handling TLB shootdowns while another
  CPU waited for its acknowledgement. Fix in PR #209
  (experiment `kernel-switch-shootdown`).
- Pi test 237, host fault round 20: after an 85 s netem delay, the network
  did not come back (#92 picture). The network session follows it up.
- [#205](https://github.com/redox-rpi/redox-rpi/issues/205): signals sent
  with `kill()` never reach a process that loops on pipe read/write (every
  `signals` round, QEMU).

## Next

- Pi test 237: the stressor and the host faults on the real Pi, with the
  netem faults.
- Power cuts in the middle of a `fill` or `killio` round, through the Pi
  test agent (Shelly plug), as their own test-plan row.

## Later

- Run a short chaos set in the nightly test.
- More fault kinds: `mmap`/`munmap` storms, time jumps, a scheme
  daemon killed and started again (with `test-scheme-reregister.sh`).
