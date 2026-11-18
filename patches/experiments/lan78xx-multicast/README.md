# LAN7800 receives all multicast (experiment `lan78xx-multicast`)

**Default patch since branch `multicast-default`:** base `0084` (was
`0024` here, on the `mdns` branch). `--experiment lan78xx-multicast` still
works and does nothing. Since base `0095` (#216) the LAN7800 uses a
multicast hash filter with the groups netstack joined (base `0085`)
instead of all multicast.

`dwc2d` sets the LAN7800's `RFE_CTL_MCAST_EN`, as Linux does for
all-multicast. Before, the chip took only its own address and broadcast,
so it dropped mDNS (`01:00:5e:00:00:fb`, `33:33:00:00:00:fb`) and IPv6
neighbor discovery (`33:33:ff:…`). The network stack doesn't tell the
driver its groups, so there is no per-group hash filter. The RTL815x
driver already receives all multicast.
