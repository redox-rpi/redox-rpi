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

- QEMU `raspi3b`: see the pull request.
