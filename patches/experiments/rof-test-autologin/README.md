# Experiment: rof-test-autologin, root logged in on the HDMI console

For the rpi-open-firmware test card only
([rpi-open-firmware#84](https://github.com/redox-rpi/rpi-open-firmware/issues/84)).
The maintainer wants the test card to start with root logged in, so a
tester can type `reboot` right away. The card script
`redox-rpi/build-rof-card.sh` in rpi-open-firmware always adds it. It is
**never** for a redox-rpi release: the directory has a `test-only` file, so
`scripts/build-image.sh --release` refuses it.

| Part | What it does |
|---|---|
| `getty@.service` (replaced, `postinstall`) | the display consoles' getty gets `PATH=/usr/lib/rof-autologin:/usr/bin` |
| `/usr/lib/rof-autologin/login` | an ion script that getty starts as `login`: sets `HOME`, `USER` and `PATH`, then `exec ion` as root, with no password |

- The serial console (`30_getty_debug.service`) is unchanged. It still shows
  `redox login:`, so automated rows and `pi-test.py` work as before.
- When the shell exits, getty starts it again, as it would start `login` again.
