# Experiment: eth-speed-test

Writes `/etc/eth-speed` with `100`, so an image built with
`--experiment eth-speed --experiment eth-speed-test` switches its Ethernet
to 100 Mbit/s at boot. For the Pi test of the boot setting
([#126](https://github.com/redox-rpi/redox-rpi/issues/126)); see
[eth-speed](../eth-speed/README.md).
