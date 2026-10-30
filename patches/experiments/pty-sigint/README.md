# pty-sigint: Ctrl-C while a program starts

Issue [#55](https://github.com/redox-rpi/redox-rpi/issues/55). A Ctrl-C in the
shell sometimes did nothing, over SSH, devsh and the serial console.

## Cause

The PTY service (ptyd), relibc's `tcsetpgrp` and Ion's job control work: ptyd
turns Ctrl-C into SIGINT for the terminal's foreground process group, and a
program that is already running stops. The Ctrl-C was lost only while a
program was still starting, in two places:

1. **Ion** made the new program the terminal's foreground group only after
   `spawn()` returned, which on Redox is after the exec. Until then the
   Ctrl-C went to Ion, which ignores it.
2. **procmgr and relibc**: a signal is marked in a control struct in the
   process's memory. After an exec, the new program's libc registers its own
   struct only at the end of its start, after ld.so has loaded everything.
   procmgr then dropped the old struct and the pending signal with it.

A start takes about 0.3 s in QEMU; the first coreutils start in an SSH
session took 5 to 23 s (the listening sshd uses much CPU, see #45). So both
windows were easy to hit.

## Patches

- `recipes/core/ion/0001`: the child sets its process group and takes the
  terminal itself before the exec (as bash does), and first resets SIGHUP,
  SIGINT and SIGTERM to the default action: until the exec is done it runs
  Ion's handlers, which would only note the signal. `ion = "source"` in
  `config.toml` builds Ion with it.
- `base/0001`: procmgr copies pending signals into the new control struct
  (POSIX keeps pending signals across exec).
- `relibc/0001`: right after that, libc sets its signal mask again, which
  delivers such a signal at once.
- `tools/ttyinfo` (in `config.toml`): shows a terminal's foreground group,
  ISIG, the SIGINT disposition, and waits for SIGINT.

The experiment `pty-sigint-debug` makes ptyd log every Ctrl-C and every change
of a terminal's foreground group (QEMU only).

## Results (QEMU, 2026-10-29)

Before (branch `sshd` image): over SSH a Ctrl-C 1 or 3 s after starting
`sleep 25` did nothing (0 of 8); ptyd's log showed it going to Ion's group.
Over devsh, 3 s after Enter it mostly worked.

With all three patches (`v0.3.0-dev-249-g28e2c4c+xfb51`, `pty-sigint` merged
with `sshd` and `pty-sigint-debug`):

- SSH, Ctrl-C 1 s after Enter: 10 of 10 stopped, 3–4 s: 7 of 7, 8 s: 5 of 5.
  (A 12 s wait was once too short after first-boot key generation: the
  signal comes when the program's libc has started.)
- devsh with `ctrlc-test.py`: 12 of 12; Ctrl-C at the prompt leaves the shell
  working. Earlier runs had 2 of 20 where the Ctrl-C came while Ion still
  read the line: Ion cancelled the line (the test counts that now). One devsh
  miss had no Ctrl-C at ptyd at all (devsh session, #11).
- Pipelines, background jobs and normal commands work as before.

Pi 3B+, test #80 (2026-10-30, `v0.3.0-dev-236-gd47707f+x810f`):

- `ctrlc-test.py --rounds 20` over devsh: 20 of 20 stopped (Ctrl-C after 1 s
  and 3 s), the shell still answers after a Ctrl-C at the prompt.
- `ttyinfo`: `tcgetpgrp(0)` equals the program's pgrp, `ISIG`, `VINTR 0x3`.
- Standard checks pass, 0 of 300 starts failed.
- Open: on the serial console (not a ptyd terminal), `sleep 30` with Ctrl-C
  right after Enter printed `ended by signal SIGINT` only after about 32 s.
  The signal was kept but took effect only when `sleep` ended.

With only the Ion patch, SSH still failed: ptyd sent SIGINT to the right group
while the program loaded, and the signal was lost (reason 2).

## Check

    patches/experiments/pty-sigint/ctrlc-test.py <host> [--port 2323] [--rounds 10]
