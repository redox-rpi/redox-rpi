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
    iptables network faults for the Pi's address only (`ss -K` resets
    where there is no iptables, as on homenodellm). After each fault devsh
    must answer and the serial log must have no panic.
- QEMU, 2026-11-04: 70 stressor rounds, only `signals` fails (#205); 26
  host faults (server, pkg, dwc2d), devsh back within 2–5 s; no panic.
- Pi tests 237–241 (2026-11-04/05): the stressor found the freeze #207.
  Host faults: 19 of 20 rounds recover within 1–6 s (delay, reordering,
  loss, pkg, server, dwc2d); round 20 is below. #205 confirmed on the Pi.
  With the #209 fix and lock-wait watches, 1360 stressor rounds ran
  without a freeze (test 241).

## Findings

- [#207](https://github.com/redox-rpi/redox-rpi/issues/207): the `threads`
  round froze the whole Pi (test 237) and QEMU. Cause: a context switch
  waited for a context lock without handling TLB shootdowns while another
  CPU waited for its acknowledgement. Fix in PR #209
  (experiment `kernel-switch-shootdown`); QEMU no longer freezes. The Pi
  still froze, more rarely, in tests 238 and 240, so #207 stays open.
- Pi test 237 part B, host fault round 20: after an 85 s netem delay, the network
  did not come back (#92 picture). The network session follows it up.
- [#205](https://github.com/redox-rpi/redox-rpi/issues/205): signals sent
  with `kill()` never reach a process that loops on pipe read/write (every
  `signals` round, QEMU and Pi).

## Next

- A short chaos stressor run in the nightly test, with a time limit: in
  progress on branch `nightly-chaos` (15 rounds, at most 10 minutes, after
  pi-test, does not decide the stable image).
- Power cuts in the middle of a `fill` or `killio` round, through the Pi
  test agent (Shelly plug), as their own test-plan row.

## Later

- More fault kinds: `mmap`/`munmap` storms, time jumps, a scheme
  daemon killed and started again (with `test-scheme-reregister.sh`).
