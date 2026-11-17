# Experiment: vc4d-flip, double-buffered primary plane

Test only ([#188](https://github.com/redox-rpi/redox-rpi/issues/188),
stage a). Build it after `vc4d`, instead of `vc4d-mode-hdmi`,
`vc4d-mode-write`, `vc4d-mode`, `vc4d-edid` or `vc4d-diag`, because each of
them replaces the vc4d recipe. It builds vc4d from branch `flip`
(redox-rpi/vc4d `b4fe1ac`, on `mode-hdmi`) and starts it with
`VC4D_MODE=write`, as `vc4d-mode-hdmi` does.

**What changes:** until now the HVS scanned out the client's buffer
directly while Orbital copied each changed region into it, so a frame
could show half old and half new content (tearing).
With `flip`:

- clients (fbcond, fbbootlogd, Orbital) draw into buffers in normal,
  cached memory;
- on each dirty-framebuffer call, vc4d copies the changed region into the
  one of its two scanout buffers that the HVS isn't reading, together with
  the previous frame's change, which that buffer lacks;
- vc4d then points the display list at that buffer, and the HVS takes it
  at the next frame;
- before the next copy, vc4d waits for that frame (frame counter, at most
  40 ms);
- the cursor plane is still scanned out directly from Orbital's 64x64
  buffer.

`VC4D_FLIP=off` in the bootloader environment brings back the old direct
scanout. vc4shot now shows the scanout buffer, so it checks the copy too.

The log shows `vc4d: flip: two scanout buffers, damage copied`. With
`VC4D_LOG=1` it also shows a copy count now and then.

For the desktop on vc4d, build:

    --experiment timesync --experiment vc4d --experiment vc4d-flip \
    --experiment orbital --experiment orbital-apps \
    --experiment cosmic-apps-source --experiment desktop \
    --experiment pointer-bench

and check it with `patches/experiments/vc4d/desktop-check.py` (QEMU, or
`--host <pi>`).
