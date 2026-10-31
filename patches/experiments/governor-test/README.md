# Experiment: governor-test

Test only, for [#23](https://github.com/redox-rpi/redox-rpi/issues/23):
the image gets `/etc/rpi-firmware/governor` with `performance`, so the
boot-time setting (`15_rpi-governor.service` and
`/usr/lib/rpi-firmware/governor.ion` in `config/raspi3bp-sdcard.toml`) can
be checked without editing an image by hand.

```bash
scripts/build-image.sh --experiment governor-test
patches/experiments/governor-test/qemu-check.sh
```

The check boots, expects `rpi-firmwared: governor performance` and mode
`performance`, then runs the script by hand with a rate (it goes to
`arm_clock`: mode `fixed`), a word both refuse (nothing changes) and
`ondemand`.
