# Experiment: vc4d-diag, what the HVS does while the desktop draws

Test only ([#188](https://github.com/redox-rpi/redox-rpi/issues/188),
[#33](https://github.com/redox-rpi/redox-rpi/issues/33)), after `vc4d`:
builds vc4d from branch `underrun-diag` (redox-rpi/vc4d `1baefde`), which
builds on `scanout-diag`.

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

Since branch `underrun-diag` it also follows the picture past the HVS:

- `vc4d: cob: ...` at the start: the output FIFO (COB) split between the
  three channels. Write `BASE TOP` to `/tmp/vc4d.cob` to move channel 1's
  part (a test knob, not for normal use);
- `vc4d: fifo: T s: empty in E of N reads, by eighth [...]`: the channel's
  live FIFO state, sampled for 20 ms each second, by where in the frame
  the line counter was (eighths, then after the last line). DISPSTAT
  itself is useless here: the firmware clears it;
- `vc4d: pv2 at ...` and, in the fifo lines, `pv2 short lines in S of N
  reads, min M`: pixel valve 2 (HDMI), whether each line got all its
  pixels;
- `vc4d: hdmi at A and B: ...` and every minute `vc4d: hdmi: T s: N frames
  in the last minute, hotplug ..., fifo_ctl ..., vid_ctl ...`: the HDMI
  encoder, read only (offsets from Linux `vc4_hdmi_regs.h`);
- `vc4d: hpd: gpio 28 at ...` and `hpd gpio 28 level L (connected)` in the
  hdmi lines: the hotplug line from the devicetree's `hpd-gpios`. On the
  3B+ that is SoC GPIO 28 (`HDMI_HPD_N`, active low); the HDMI core's
  HOTPLUG bit stays 0 there.

On aarch64, Redox maps uncached and write-combining memory alike (Normal
non-cacheable), so a write-combining buffer is not a separate test.

## Pi test #276 (vc4d 9f931e2)

Typing 200 keys, two 30 s window drags, 2 min idle: errors in 0 of 60
reads every minute, all buffers contiguous, the channel always on our
list. The HVS reported no underrun and nothing else changed the list. The
only list findings were the context word (word 3), which the HVS writes
itself while it scans out; since `49e72eb` those words are skipped. No
screen was watched in that run, so whether the bands showed is not known.

## Pi tests #316 to #327 (branch underrun-diag)

- #316 (720x480): the FIFO was empty only in the first eighth, i.e. at
  frame start, never later in the frame.
- #318 (1920x1080): the same at full HD, with and without load. Pass.
- #324 (pixel valve 2): every line got 1920 of 1920 pixels, also with
  580 MB/s of memory load. Pass.
- #327 (HDMI encoder): about 3650 frames a minute (60 Hz), fifo_ctl
  0x4041 and vid_ctl 0x80000000 never changed under load. Pass. HOTPLUG
  read 0 throughout, which led to the GPIO read above.

So the registers show a clean picture from the HVS through the pixel
valve to the encoder. If the #275 bands come back, they are after the
encoder (cable, NanoKVM) or need the original #275 conditions (COSMIC
apps redrawing half a frame); a look at the screen is still due.
