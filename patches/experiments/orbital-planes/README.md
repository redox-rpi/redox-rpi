# Experiment orbital-planes

Refs #563 (graphics stalls and stripes while moving windows).

Orbital composites a moved window into its screen buffer on every mouse
move; with vc4d that means copying the damaged rectangles into the
scan-out buffer, which shows as stalls and stripes on the Pi 3B+ and the
Zero 2 W.

`drag-plane.patch` lifts the window onto a KMS overlay plane at the first
move of a title drag: Orbital draws the window once into a buffer of its
own, composites the screen without it, and then only moves the plane.
When the drag ends, the next redraw composites the window in its new place
and hides the plane.

- Needs a driver with overlay planes: vc4d's `overlay` branch
  (redox-rpi/vc4d) has three. Without one, or when the driver refuses the
  buffer (vc4d: not in contiguous memory, or as large as the screen),
  Orbital keeps the old path.
- The window's content is frozen during the move.
- A window that is not on top shows above the others while it moves.
- Conflicts with the Orbital recipe of PR #456 (both replace
  `recipes/gui/orbital/recipe.toml`); merge the patches when one lands.

Check: `desktop-check.py --trace` on a build with this experiment; the
trace's copy count during `pointer-bench --super-drag` drops to near zero
and `overlay` events appear instead.
