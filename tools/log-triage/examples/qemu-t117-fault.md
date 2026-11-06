# Log triage: qemu-t117.log

489 lines, 15 noise lines dropped, 2 boots; model gpt-oss-20b, 1 requests, 14.0 s.
Compared with the good log `good.log`.

| Boot | Lines | Version | Login | Panics | Faults | Errors | Summary |
|---|---|---|---|---|---|---|---|
| 2 | 5-488 | v0.3.0-dev-499-g1427db8+x12cb | yes | 0 | 1 | 15 | The system reached the login prompt but failed to mount the RedoxFS, then a kernel exception in /tmp/r117 caused a crash. No daemons exited unexpectedly. |

## Boot 2 (lines 5-488)

- Reached login: yes (regex: yes), logged in: no
- Fault at line 418 (/tmp/r117): `kernel::arch::aarch64::interrupt::exception:ERROR -- FATAL: Not an SVC induced synchronous exception (ty=111100)`
- First error, line 386: `[ERROR redoxfs] failed to open filesystem /scheme/disk.mmc/0p1: No such file or directory`
- New against the good log: +L26: Booting: Label: mmc 0 Device path: /VenHw(e61d73b9-a384-4acc-aeab-82e828f3628b,0000000000000000)/VenHw(e61d73b9-a384-4acc-aeab-82e828f3628b,7200000000000000)/SD(0)/SD(0)
- New against the good log: +L27: No RNG device
- New against the good log: +L326: kernel::startup:DEBUG -- Env: Ok("DTB_ADDR=000000003aaa5000\nDTB_SIZE=0000000000006000\nREDOXFS_BLOCK=0000000000000000\nREDOXFS_UUID=b8d85b6d-7174-47dc-a9b2-94e94bb3cd97\nFRAMEBUFFER_ADDR=000000003c100000\nFRAMEBUFFER_VIRT=ffff80003c100000\nFRAMEBUFFER_WIDTH=0000000000000280\nFRAMEBUFFER_HEIGHT=00000000000001e0\nFRAMEBUFFER_STRIDE=0000000000000280\n")
- New against the good log: +L352: dwc2d: device on root port, Full speed
- New against the good log: +L386: [ERROR redoxfs] failed to open filesystem /scheme/disk.mmc/0p1: No such file or directory
- Summary: The system reached the login prompt but failed to mount the RedoxFS, then a kernel exception in /tmp/r117 caused a crash. No daemons exited unexpectedly.
- 117 of 484 lines sent, 1 requests, 14.0 s
