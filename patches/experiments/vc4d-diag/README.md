# Experiment: vc4d-diag, what the HVS does while the desktop draws

Test only ([#188](https://github.com/redox-rpi/redox-rpi/issues/188),
[#33](https://github.com/redox-rpi/redox-rpi/issues/33)), after `vc4d`:
builds vc4d from branch `scanout-diag` (redox-rpi/vc4d `9f931e2`).

Pi test #275 showed noise bands over the lower half of the HDMI picture
while the editor was in use, on the NanoKVM and on a monitor, while
vc4shot (the buffer as vc4d sees it) was clean. vc4d only read the HVS
error bits in the first two minutes. With this experiment it also logs, to
serial:

- `vc4d: buffer WxH: physical A-B, contiguous` (or `NOT CONTIGUOUS from
  page N (row R)`) for each buffer: whether the HVS reads what the CPU
  wrote;
- `vc4d: scanout: T s: errors [EUFLOW, ...], ...`: HVS errors (underrun
  and others), read and cleared every second, the first five per minute;
- `vc4d: scanout: T s: list word W is X, written Y` and `channel reads the
  list at L, not ours`: something else (the firmware) changed the display
  list;
- every minute: `vc4d: scanout: T s: errors in E of N reads, F list
  findings in the last minute`.

On aarch64, Redox maps uncached and write-combining memory alike (Normal
non-cacheable), so a write-combining buffer is not a separate test.
