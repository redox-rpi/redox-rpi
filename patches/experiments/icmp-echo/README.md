# netstack answers ping (experiment `icmp-echo`)

Issue: [#177](https://github.com/redox-rpi/redox-rpi/issues/177).

The Pi didn't answer ping from other hosts (Pi test 164: 0 of 180), while
TCP worked. smoltcp answers ICMP echo requests only with its feature
`auto-icmp-echo-reply`. That is a default feature, but netstack sets
`default-features = false` and didn't list it, so smoltcp's
`process_icmpv4` skipped the request.

**Base `0001`** adds the feature to `netstack/Cargo.toml`. One line.

The experiments `ipv6` and `multicast` (PR #82) change the same lines;
`ipv6` already adds this feature. When this becomes a default patch, their
`netstack/Cargo.toml` hunks need the line dropped from their context.

Build: `--experiment icmp-echo`.

## Tests

- QEMU `raspi3b`: see the pull request.
