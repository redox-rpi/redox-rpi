# getty panics when input arrives faster than the program reads

- **What breaks:** `ptyd` refuses input with `EWOULDBLOCK` while the
  program has not read enough earlier input. getty treats that as fatal:
  "getty: failed to write master PTY: … WouldBlock", the panic aborts
  getty and the console is gone (the shell dies with it). Fast input
  does it, for example a pasted line of a few hundred characters on a
  serial console.
- **Reproduce:** without experiment `serial-rx`, send a long command
  line at full speed, e.g. `scripts/qemu-raspi3b.sh --run "<300
  characters>"`; the log shows the panic at `getty/src/main.rs:75`.
- **Upstream code:** [`getty/src/main.rs`](https://gitlab.redox-os.org/redox-os/base/-/blob/eddcaa21fb2d11b4a6bd64478e0639153266dc04/getty/src/main.rs#L60)
  (`handle`, lines 60-85).
- **Our patch:** base `0023` in experiment
  [`serial-rx`](https://github.com/redox-rpi/redox-rpi/tree/main/patches/experiments/serial-rx): keep the
  input the PTY did not take, stop reading the TTY until it is taken,
  keep forwarding output, retry with a short poll timeout. The kernel
  patch `0007` in the same experiment fixes a separate bug (dropped
  characters, [#28](https://github.com/redox-rpi/redox-rpi/issues/28)).
- **State:** passes on the Pi (test plan #22: 20 of 20 lines of 200
  characters).
