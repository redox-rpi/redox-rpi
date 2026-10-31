# relibc: pointers for upstream

Notes for the user, who reports to Redox. Checked against upstream on
2026-10-31: relibc `master` at `6b378cb`. All four are still there. One is
a bug; the other three are missing features (the GitHub labels say the
same).

## The aarch64 signal return overwrites x18 (bug)

Our [#127](https://github.com/redox-rpi/redox-rpi/issues/127).

- **What breaks:** after a signal handler runs, the interrupted code
  continues with a wrong x18. relibc's comment assumes x18 is the reserved
  platform register. That holds on macOS or Windows, not on Redox: the
  compiler uses x18 as an ordinary register, and the prebuilt programs from
  the Redox package repository (bash, vim, curl and others) do. After a
  signal (Ctrl-C, a timer) they can crash.
- **Why:** the end of `__relibc_internal_sigentry` in
  `redox-rt/src/arch/aarch64.rs`, lines 384–390: `mov x18, sp`,
  `ldr x0, [x18]`, `mov sp, x0`, `ldp x18, x0, [x18, #16]`, `br x18`. It
  jumps back through x18, so x18 holds the return address, not the
  interrupted value. Lines 402–404 do the same on another path.
- **Reproduce:** aarch64 (QEMU is enough), with our `tools/sigregs`: it
  loads known values into the registers, checks them in a loop while
  signals arrive, and reports a register that changed. `sigregs --x18`
  includes x18 and prints `FAIL ...: x18 changed after N signals` on
  upstream; with the fix it prints `PASS`.
- **Our fix:** first experiment `sigreturn-svc`
  ([PR #61](https://github.com/redox-rpi/redox-rpi/pull/61)), now default
  patches on branch `sigreturn-default`. Two patches, which must come
  together:
  - relibc `0013`: the signal entry restores every register and ends with
    `svc #0x5352` instead of `br x18` (`redox-rt/src/arch/aarch64.rs`,
    `redox-rt/src/signal.rs`).
  - kernel `0009`: that svc sets pc and x0 from the sigcontrol struct,
    enables signal delivery again and delivers a pending signal right away
    (`src/arch/aarch64/interrupt/exception.rs`, `src/context/signal.rs`).
    This also closes the window in which a second signal could arrive
    while the end of the signal entry ran.
- **Tested:** QEMU (`sigregs --x18`); Pi 3B+ test 45, 450 of 450 starts
  of nine prebuilt programs.

## `clock_settime` and `settimeofday` missing (enhancement)

Our [#51](https://github.com/redox-rpi/redox-rpi/issues/51).

- **What's missing:** a program can't set the clock through the normal API.
  `clock_settime` fails with `ENOSYS` and logs a TODO line; `settimeofday`
  doesn't exist, so programs that call it don't link.
- **Where:** `src/platform/redox/mod.rs`, lines 238–241: `clock_settime`
  is `todo_skip!(...)` then `Err(Errno(ENOSYS))`. `src/header/sys_time/`
  has no `settimeofday`; only the `__NR_settimeofday` syscall numbers
  exist. The kernel already supports it: it keeps the real time as boot
  time plus monotonic time and takes a new boot time written to
  `/scheme/sys/update_time_offset` (root only).
- **Reproduce:** as root, a C program calling
  `clock_settime(CLOCK_REALTIME, &ts)` gets -1 with `ENOSYS`.
- **Our fix:** relibc `0011`
  ([PR #98](https://github.com/redox-rpi/redox-rpi/pull/98)), about 70
  lines in `src/platform/redox/mod.rs` and `src/header/sys_time/mod.rs`.
  `clock_settime(CLOCK_REALTIME)` computes the boot time from the new time
  and writes it to `update_time_offset`. Other clocks give `EINVAL`, a
  non-root caller `EPERM`. `settimeofday` calls `clock_settime` and
  ignores the old time zone argument. Our `timesync -s` uses it.
- **Tested:** QEMU; Pi 3B+ test 66, all passed.

## `cfsetospeed`/`cfsetispeed` fail with `EINVAL` (enhancement)

Our [#94](https://github.com/redox-rpi/redox-rpi/issues/94).

- **What's missing:** the termios speed functions work only on Linux. On
  Redox, `cfsetispeed`, `cfsetospeed` and `cfsetspeed` fail with `EINVAL`,
  and `cfgetispeed`/`cfgetospeed` return 0. Programs that set a serial
  speed (terminal programs, `stty`) can't.
- **Where:** `src/header/termios/mod.rs`. Each function has a Linux branch
  and an `EINVAL`/0 branch for everything else:
  - `cfgetispeed` (line 167) and `cfgetospeed` (line 191) return 0.
  - `cfsetispeed` (line 213, `EINVAL` at 224), `cfsetospeed` (line 238,
    `EINVAL` at 249) and `cfsetspeed` (line 268, `EINVAL` at 291).
  Redox keeps one speed for both directions in `c_cflag & 0o37`; that's
  where ptyd and the serial drivers read it.
- **Reproduce:** a C program: `tcgetattr(0, &t); cfsetospeed(&t, B9600)`
  returns -1 with `EINVAL`, and `cfgetospeed(&t)` returns 0. Our
  `tools/termios-check <device> 9600` sets and reads back a speed on a
  serial port and fails on upstream.
- **Our fix:** experiment `termios-speed`
  ([PR #97](https://github.com/redox-rpi/redox-rpi/pull/97)), relibc
  `0001`, about 40 lines in that one file. The speed goes into and comes
  from `c_cflag & 0o37`. `cfsetispeed(B0)` keeps the output speed, as
  POSIX says.
- **Tested:** QEMU, `termios-check` and a USB serial (FTDI) test,
  `scripts/test-usb-serial.sh` 18 of 18.

## `madvise` hints, `IP_TOS` and `TCP_NODELAY` (enhancement)

Our [#109](https://github.com/redox-rpi/redox-rpi/issues/109).

- **What's missing:** common calls that programs make all the time either
  fail or log a TODO line at INFO level. sshd makes all of them on every
  connection, so its log fills with TODO lines.
- **Where:**
  - `src/platform/redox/mod.rs`, lines 997–1006: `madvise` logs a TODO
    line and returns `Ok(())` for every advice, even the pure hints.
  - `src/platform/redox/socket.rs`: `getsockopt` (line 687) logs a TODO
    line (732) and returns `ENOSYS` (740) for `IP_TOS` and `TCP_NODELAY`;
    `setsockopt` (line 981) logs a TODO line (1027) and returns `Ok(())`.
- **Reproduce:** log in over sshd and watch the log: each connection
  prints `TODO: madvise ...`, `getsockopt ...` and `setsockopt ...` lines.
  A C program calling `getsockopt(fd, IPPROTO_TCP, TCP_NODELAY, ...)`
  gets `ENOSYS`.
- **Our fix:** relibc `0045` in experiment `sshd`
  ([PR #120](https://github.com/redox-rpi/redox-rpi/pull/120)), about 50
  lines in those two files:
  - `madvise`: the pure hints (`NORMAL`, `RANDOM`, `SEQUENTIAL`,
    `WILLNEED`, `DONTDUMP`) succeed without a log line. `MADV_DONTNEED`
    keeps its line, since relibc doesn't drop the pages.
  - `getsockopt(IP_TOS)` and `(TCP_NODELAY)` report 0, which is what
    netstack does (TOS 0, Nagle on).
  - `setsockopt(IP_TOS)` succeeds quietly; `setsockopt(TCP_NODELAY)`
    succeeds with a debug line, since netstack can't switch Nagle off yet
    (its socket options carry no level).
- **Tested:** QEMU; Pi 3B+ test 132, sshd log without the TODO lines.
