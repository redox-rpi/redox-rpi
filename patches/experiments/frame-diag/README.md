# Experiment: frame-diag, where the COSMIC editor spends a frame

Test only ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)), after
`cosmic-apps-source` (its cosmic-edit recipe replaces that one and adds
this). softbuffer (`pop-os/softbuffer`, tag `cosmic-4.0`, `c2b2c19`), which
puts the pixels into the Orbital window, prints every 2 s while frames are
drawn:

    frame-diag: <w>x<h>: <n> frames (<full> full, <copied> copied) in 2.0 s: map <ms> ms, draw <ms> ms, present <ms> ms

- **map**: `buffer_mut`, asking Orbital for the window size and mapping
  the window buffer, each frame;
- **draw**: from `buffer_mut` to `present`, the drawing by iced/tiny-skia;
- **present**: unmapping and telling Orbital to show the window (`fsync`,
  the whole window: iced does not pass the damaged area);
- **full**: frames with buffer age 0, which iced draws in full.

With `app-log` the lines go to serial. Pi test #274: typing 10 keys/s cost
cosmic-edit 57 % of a core; this shows which step.
