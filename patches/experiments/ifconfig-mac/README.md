# ifconfig shows the MAC address (experiment `ifconfig-mac`)

Issue: [#171](https://github.com/redox-rpi/redox-rpi/issues/171).

`ifconfig` (upstream `netutils`, `src/ifconfig/interface.rs`) printed a
fixed `00:00:00:00:00:00` as the MAC address, and the prefix length (`24`)
as the netmask. netstack has the address in
`/scheme/netcfg/ifaces/<name>/mac` as `52-54-00-12-34-57`.

**netutils `0001`** reads it there and prints it with colons, and prints the
netmask as `255.255.255.0`. netutils is pinned in `config/upstream.lock`,
and the experiment's `config.toml` builds it from source; without this
experiment the image takes the upstream package as before.

Build: `--experiment ifconfig-mac`.

## Tests

- QEMU `raspi3b` (`v0.3.0-dev-967-g311e46d+x9b05`): `ifconfig` shows
  `MAC Address: 52:54:00:12:34:57` and `Netmask: 255.255.255.0`; the `mac`
  file has `52-54-00-12-34-57`. Without the experiment (`v0.3.0-dev-795`):
  `00:00:00:00:00:00` and `24`.
- Pi 3B+, test 194: **pass** ([report](https://github.com/redox-rpi/redox-rpi/blob/main/docs/test-reports.md#2026-11-01-v030-dev-967-g311e46dx9b05-ifconfig-shows-the-mac-address)). `MAC Address: b8:27:eb:08:e0:aa`,
  the same as the `mac` file, `Netmask: 255.255.255.0`; 0 of 300 starts
  failed.
