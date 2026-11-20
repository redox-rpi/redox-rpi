# Experiment vc4d-scale

Refs #563. vc4d reads `/etc/vc4d-mode` (`WIDTHxHEIGHT`). Clients (Orbital) see only
that mode, and the HVS scales every plane to the firmware mode: a PPF filter
upwards, TPZ for strong downscaling. This experiment sets 1280x720.

Check: `desktop-check.py --size 1920x1080` in QEMU (`VC4D_FAKE`) and the
`vc4d: mode: 1280x720 for clients, scaled to ...` line. A person checks sharpness on
a real monitor.
