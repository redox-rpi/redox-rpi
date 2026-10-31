# Patch numbers until 2026-10-31

This table and list were kept in `patches/README.md` until 2026-10-31.
Every merge edited the same lines, so open pull requests conflicted with
each other after each merge. They are frozen here as a record. Newer
patches describe themselves: the commit message of each patch says what
it does and which experiment, issue and Pi test it came from, and an
experiment that became default patches says so in its README.

| Number | Where | What |
|---|---|---|
| `0001`–`0016`, `0018`, `0020`–`0028`, `0030`–`0033`, `0037`, `0040`, `0045`–`0048`, `0056` | `base/` | default patches; `0018` is `rpi-firmwared`, `0020` RTL8152/RTL8153 Ethernet in `dwc2d` (was experiment `rtl8152`), `0021` the ARM clock and throttle watch of `rpi-firmwared`, `0022` `dwc2d` waits for its interrupt (was experiment `dwc2-interrupts`, with kernel `0004`–`0006`), `0023` `dwc2d` picks the network adapter with a link, `0024` getty keeps input the PTY does not take yet (was experiment `serial-rx`, with kernel `0007`; #28, #32), `0025`–`0028` the sdhost driver `bcm2835-sdhostd`, started at boot instead of `bcm2835-sdhcid` (were experiments `sdhost-driver` and `sdhost-boot`, #6), `0030` no PCI and audio services on the Pi (was experiment `pi-clean-boot`; touches only init, `hwd` and unit files, so it applies with any experiment), `0031` `dwc2d` finds USB devices plugged in or removed after boot, `0032` `dhcpd` tries again until the server answers, `0033` the hub port cycle command for hotplug tests (`0031`–`0033` were experiment `usb-hotplug`; #17), `0037` `dwc2d` asks again for a USB reply shorter than the fields it reads, instead of panicking (was experiment `dwc2d-short-reply`; test 98, test 105), `0040` the `rpi-firmwared` governor, the ARM clock follows the load (was experiment `cpu-ondemand`, with kernel `0040`; #23, Pi test #93), `0045` GPIO driver `bcm2835-gpiod`, `0046` `rpi-firmwared` `power_led` (after `0040`, same lines), `0047` I2C driver `bcm2835-i2cd`, `0048` SPI driver `bcm2835-spid` (were experiments `gpio`, `i2c`, `spi`; #38, #39, #42, Pi test #99), `0056` netstack lists its `netcfg` directories, so `ls /scheme/netcfg/ifaces` works (#105) |
| `0001`–`0008`, `0040` | `kernel/` | default patches; `0008` frees a thread that ends itself when its syscall returns (was experiment `ctx-leak-fix`, #31), `0040` CPU times in `sys:stat` without rounding (with base `0040`, #23) |
| `0029` | `experiments/sdhost-stats/base/` | transfer statistics at boot |

Experiments apply after all default patches. Combinations that build:
`sdhost-stats`, `timesync`, `ldso-diagnostics`.
`rtl8152`, `dwc2-interrupts`, `serial-rx`, `pi-clean-boot`, `sdhost-driver`,
`sdhost-boot`, `ctx-leak-fix`, `usb-hotplug`, `dwc2d-short-reply`, `cpu-ondemand`, `gpio`, `i2c` and `spi` are default patches now; the names are still accepted
and do nothing.
