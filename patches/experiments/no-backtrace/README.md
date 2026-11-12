# No Rust backtrace on the console (#291, experiment `no-backtrace`)

Upstream `init` started every service with `RUST_BACKTRACE=1`
(`init/src/main.rs`, `InitConfig::new`). init clears the environment of
each service and gives it only its own list, so the variable reached
every service, and getty passed it on to login and the shell. Every
program a user started had it.

A tool that returns an error from `main` (anyhow), such as `ping` with a
name that does not resolve, then printed:

```
Error: failed to lookup address information: Non-recoverable error
Stack backtrace:
   0: <unknown>
   ...
```

The binaries have no symbols, so every frame is `<unknown>`. It tells
nothing and looks like a crash. A panic printed the same empty list after
its message.

## Fix

Base patch
[`0001`](base/0001-init-no-RUST_BACKTRACE-1-for-the-services-and-the-lo.patch):
init starts with an empty environment list instead of
`RUST_BACKTRACE=1`. `PATH` and `LD_LIBRARY_PATH` are added at the switch
to the root filesystem as before.

- Errors print their message only; a panic still prints its message and
  the source location (`file.rs:line`).
- Whoever wants a backtrace sets it: `RUST_BACKTRACE=1 ping …` in the
  shell, or `RUST_BACKTRACE = "1"` in a unit's `envs`.
- bootstrap still gives `RUST_BACKTRACE=1` to init itself; init does not
  pass its own environment on.

Why init and not getty: the services' backtraces are just as empty, and
programs started from the desktop or other services get their
environment from init too, not through getty.

## Check

QEMU (`qemu-raspi3b.sh --usb-net`), over devsh:

- `env` shows no `RUST_BACKTRACE`;
- `ping no.such.name` prints the error line without `Stack backtrace:`;
- `env RUST_BACKTRACE=1 ping no.such.name` still prints the backtrace, so
  the check would see it.

## Results

QEMU, 2026-11-12, build `v0.5.0-dev-102-g0d83238a+x6f6f`
(`--experiment no-backtrace`). devsh logs in through getty and login, as
a user on the console does:

| check | result |
|---|---|
| `env`, lines with `RUST_BACKTRACE` | 0 |
| `ping -c 1 no.such.name` | `Error: failed to lookup address information: Non-recoverable error`, nothing after it |
| `env RUST_BACKTRACE=1 ping -c 1 no.such.name` | the same error, then `Stack backtrace:` with frames 0–6 `<unknown>` |
| `Stack backtrace` in the whole boot log | 0 |
