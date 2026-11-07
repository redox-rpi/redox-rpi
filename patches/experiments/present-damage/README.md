# Experiment: present-damage, Orbital redraws only what the editor changed

Test only ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)), after
`cosmic-apps-source` and `frame-diag` (its cosmic-edit recipe replaces
theirs and adds this).

iced's tiny-skia compositor (`pop-os/iced` `a31e6c6`, the submodule of
libcosmic `60ad2cc` that cosmic-edit uses) works out which part of the
window changed and draws only that, but then calls softbuffer's
`present()` without the damage. softbuffer's Orbital backend turns that
into `fsync` on the window, so Orbital composites the whole window again
for every key, and also for frames in which nothing changed.

The patch `iced-0001-tiny-skia-pass-damage-to-present.patch`:

- converts the damage rectangles to physical pixels and calls
  `present_with_damage`, which softbuffer sends to Orbital as
  `Y,x,y,w,h,...`;
- does not present at all when nothing changed (it just unmaps the window
  buffer).

The softbuffer timing output of `frame-diag` is kept and also shows the
damaged share of the window.

## Upstream report

For the user to file (pop-os/iced, tiny_skia `window/compositor.rs`):
`present()` computes the damage but presents without it, so window systems
that take damage (Orbital, Wayland through softbuffer) redraw the whole
window each frame. Passing the grouped damage to
`Buffer::present_with_damage` and skipping the present for empty damage
fixes it.
