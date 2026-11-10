# Experiment: vc4d-diag, what the HVS does while the desktop draws

Test only ([#188](https://github.com/redox-rpi/redox-rpi/issues/188),
[#33](https://github.com/redox-rpi/redox-rpi/issues/33)), after `vc4d`:
builds vc4d from branch `scanout-diag` (redox-rpi/vc4d `49e72eb`).

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

## Pi test #276 (vc4d 9f931e2)

Typing 200 keys, two 30 s window drags, 2 min idle: errors in 0 of 60
reads every minute, all buffers contiguous, the channel always on our
list. The HVS reported no underrun and nothing else changed the list. The
only list findings were the context word (word 3), which the HVS writes
itself while it scans out; since `49e72eb` those words are skipped. No
screen was watched in that run, so whether the bands showed is not known.
