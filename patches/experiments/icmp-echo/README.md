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

- QEMU `raspi3b` (`v0.4.0-dev-13-g41cfa03+x0fb0`): a socket netdev on a
  QEMU hub next to the user network sends echo requests from a fake host
  `10.0.2.99` to the guest: **10 of 10 answered**. The image without the
  experiment (`v0.3.0-dev-967`) answered 0 of 10, ARP yes. `ping -c 3
  10.0.2.2` from the guest still works.
- Pi 3B+: test 196.
