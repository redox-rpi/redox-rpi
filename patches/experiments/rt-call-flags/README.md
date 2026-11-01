# Experiment: rt-call-flags, calls with two file descriptors

Backport of upstream relibc `112eb43c` "fix: Set extra_flags for
sys_call" (2026-10-28, after our pin), `redox-rt/src/sys.rs` only. The
`redox_syscall` pinned here already has `Call::extra_flags`, so the
`Cargo.toml`/`Cargo.lock` part of the upstream commit is not needed. The
patch goes away when the relibc pin moves past `112eb43c`.

The bug: `SYS_CALL` can take a list of up to two descriptors. For that,
`redox_syscall` passes the list's address with its length in the low bits
and sets `CallFlags::MULTIPLE_FDS2`, which `Call::extra_flags()` returns
for `&[usize]`. `redox_syscall::call_ro` and the others add these flags;
redox-rt's own `sys_call_ro`, `sys_call_wo`, `sys_call_rw` and `sys_call`
did not. Without the flag the kernel reads the address as one descriptor
number and returns `EBADF`. This is not specific to aarch64.

Affected callers in relibc:
- `redox_ring_register_files` (`src/platform/redox/ring.rs`): ring fd
  plus file table. The relibc test `redox_ring_ramfs` fails with
  `redox_ring_register_files: Bad file number` ([#14](https://github.com/redox-rpi/redox-rpi/issues/14)).
- DRM `PRIME_FD_TO_HANDLE` ioctl (`src/header/sys_ioctl/redox/drm.rs`):
  device fd plus dma-buf fd.
- `redox_sys_call_v0` in libredox (`src/platform/redox/libredox.rs`) when a
  caller passes more than one fd.

```bash
scripts/build-image.sh --experiment rt-call-flags
```

## Results

QEMU raspi3b, 2026-10-31, branch `t14-ring-call` with `t13-relibc-tests`
merged (test build only), `--experiment relibc-tests --experiment
timesync`, with and without this experiment:

| | Without (`v0.3.0-dev-803-g6e2853f+x36c5`) | With (`…+xfb68`) |
|---|---|---|
| `redox_ring_ramfs` | `redox_ring_register_files: Bad file number` | passes (`FIXED`) |
| Other differences | – | none |

Both runs show the same 4 `NEW` lines, which come from the base, not from
this patch: `pthread/exit` (experiment `self-exit-status` not enabled)
and `sys_socket/unixrecv`, `unixrecvfrom`, `unixnonblock`.

After main with the redoxfs pin (#145) was merged in: `v0.3.0-dev-848-gdc1da3e+xdd51`
(`--experiment relibc-tests --experiment timesync --experiment
self-exit-status --experiment rt-call-flags`): 16 failures, all known, none
new; `redox_ring_ramfs` passes.
