# Network

## State

- LAN7800 (Pi 3B+ onboard Ethernet) in `dwc2d`: DHCP, TCP from the LAN.
- RTL8152/RTL8153 USB Ethernet (base patch `0020`, since v0.2.0): the
  Zero 2 W gets its time over the HAT's RTL8152.
- `devshd`: remote shell on port 2323, unencrypted, LAN only, never in
  release images.

## Next

- [#45](https://github.com/redox-rpi/redox-rpi/issues/45) SSH server instead of devshd (experiment `sshd`, branch `sshd`):
  OpenSSH from source; key and password login and an interactive shell
  work in QEMU after fixes in relibc (`setitimer`, `setgroups`, `poll`
  with a repeated fd, `ld.so` and login shells), `procmgr` (SIGCHLD of a
  child that dropped its privileges) and OpenSSH (no `chroot`, PTY
  without owner). Pi test #59 failed: no host keys, because rename does
  not work on `/scheme/data`; the keys are now made in `/etc/ssh` and
  copied there. Rerun #81 passed on the Pi (2026-10-30): login in
  1.3 s, the keys stay the same after a reset. Since then: the listening
  `sshd` no longer uses a whole CPU (netstack reported a listening socket
  always readable), and the shell exits cleanly when the session ends
  (ptyd sends SIGHUP and fails `tcgetattr` on the hung-up PTY, Ion exits
  instead of panicking). Pi rerun #101 passed (2026-10-30): 2.9 s of
  `sshd` CPU after 22 sessions, 10 dropped sessions without an Ion
  exception or leftovers. Pull request #78. Branch `sshd-keys` (on top
  of it): netstack keeps spare listening sockets, so `ssh-keyscan` with
  several types gets every key (connections that came in together were
  reset), and sshd also reads `authorized_keys` from the data partition,
  so keys survive a reset of the network-boot image. Pi test #120 passed.
  Ctrl-C while a program starts:
  [#55](https://github.com/redox-rpi/redox-rpi/issues/55), branch `pty-sigint`.
- Confirm the Zero 2 W's IP address and TCP (`timesync -n` worked).
- [#53](https://github.com/redox-rpi/redox-rpi/issues/53) several adapters
  (draft PR [#81](https://github.com/redox-rpi/redox-rpi/pull/81) on top
  of #68, base `0041`–`0044` once merged; Pi tests #74 and #122 passed,
  #122 on the branch as it will be merged):
  netstack used to take one adapter at start and dhcpd configured only
  `eth0`, once. Now netstack opens every `network.*` scheme and looks
  again every 2 s (`wlan*` for Wi-Fi, `eth*` for the rest), dhcpd gets a
  lease on each interface and keeps one default route (wired before
  Wi-Fi), DHCP broadcasts leave only on their own adapter. `testnetd` is
  a fake adapter with a DHCP server for the tests. Passes in QEMU (USB
  network plus testnetd started after boot) and on the Pi (test 74, LAN7800
  plus testnetd). Since then netstack also reports ready only after netcfg
  and the interfaces exist (test 69 race; same as boot-time base `0028`).
  The rest is draft PR
  [#118](https://github.com/redox-rpi/redox-rpi/pull/118) on top of #81
  (base `0058`): dhcpd renews leases and drops them at their end or on a
  NAK; netstack drops an adapter that goes away (scheme gone or file
  failing) with its address and routes; the default route moves to the
  next interface or is removed. QEMU passes with two testnetds; **Pi test
  140 passed**. Not done: link state (a pulled cable shows only when the lease
  ends); a restarted driver cannot take its old scheme name again
  ([#117](https://github.com/redox-rpi/redox-rpi/issues/117), kernel).
- IPv6 SLAAC took only one prefix and no router on the Pi (tests #47,
  #55: ULA address yes, global 2a02 address and default route no): smoltcp
  keeps only the last prefix option of a router advertisement, and each
  advertisement replaced the router. Fixed in `ipv6` base `0029` (every
  prefix, global first; lifetime 0 ends only that router), QEMU passes
  with a global and a ULA prefix and a second router; **Pi test #85
  passed** (global address, default route, internet over IPv6). Draft PR
  [#82](https://github.com/redox-rpi/redox-rpi/pull/82) brings the
  `multicast` and `ipv6` experiments to main; whichever of #81 and #82
  lands second gets its netstack patches rebased.

- [#26](https://github.com/redox-rpi/redox-rpi/issues/26) Wi-Fi step 5
  (branch `wifi-join`, on top of #81): after a WPA2-PSK join through the
  firmware supplicant (the driver computes the PMK itself, PBKDF2 tested
  with the IEEE vectors), `bcm2835-sdiod` stays as `network.wlan0`;
  netstack takes it as `wlan0` and dhcpd leases it. The network name and
  passphrase come from `~/.config/redox-rpi/wifi` on homenodellm (made by
  the user, never committed) through `wifi-config.py`. QEMU has no Wi-Fi
  chip: unit tests and a QEMU check of the settings path pass; Pi test
  #110.

- [#88](https://github.com/redox-rpi/redox-rpi/issues/88) netstack kept
  every accepted TCP connection (2 x 64 KiB) after it was closed: a wrong
  reference count in `dup("listen")`. Fixed in base `0049`, draft PR
  [#90](https://github.com/redox-rpi/redox-rpi/pull/90). **Pi test 114
  passed**: 120 devsh connections cost 1.1 MB, where stable lost 5.3 MB
  for 40.
  The rest of #88's growth (Pi test 111, `SHARED` of netstack and randd)
  was a kernel bug: every open of a scheme's root (`/scheme/tcp` per TCP
  connection, `/scheme/rand` per random read) left one 4 KiB page mapped in
  the daemon. Kernel `0057`, draft PR
  [#114](https://github.com/redox-rpi/redox-rpi/pull/114), merge after #90;
  QEMU: no growth over 800 MQTT runs. **Pi test 136 passed** (no
  `SHARED` growth). Also upstream; see
  [docs/upstream/netstack.md](../upstream/netstack.md).
- [#37](https://github.com/redox-rpi/redox-rpi/issues/37) mDNS: the Pi
  answers as `redox-rpi.local` and announces `_devsh._tcp` and `_ssh._tcp`
  (each only while its program is on the image); **Pi test 96 passed**,
  SSH works at the address mDNS gives; **Pi test 123 passed** with SRV,
  TXT and address records in a service's PTR answer. Draft PR
  [#93](https://github.com/redox-rpi/redox-rpi/pull/93) on top of #82.
- [#105](https://github.com/redox-rpi/redox-rpi/issues/105) `ls` of a
  netcfg directory failed with `Not a directory`: upstream's netcfg scheme
  has no `getdents`. Base `0056` (branch `netcfg-ls`) lists the names a
  directory node already reads as; QEMU passes, **Pi test 128 passed**,
  draft PR [#107](https://github.com/redox-rpi/redox-rpi/pull/107).
  Upstream problem; pointers for the user's report:
  [docs/upstream/netstack.md](../upstream/netstack.md).
- [#119](https://github.com/redox-rpi/redox-rpi/issues/119) long downloads
  on the Pi run at 0.24 MB/s or less and once ended with `os error 107`
  (test 126). QEMU doesn't show it: 3.9 MB/s steady, also with the receive
  window closed for 300 s. Diagnosis on the Pi, network alone and with
  card writes: test 141 (branch `tcp-slow`, test downloader `netget`).

## Later

- [#26](https://github.com/redox-rpi/redox-rpi/issues/26) Wi-Fi on the Pi 3B+ and Zero 2 W (milestone v0.6.0); it would be the
  first Wi-Fi in Redox.
- [#27](https://github.com/redox-rpi/redox-rpi/issues/27) IPv6 in `netstack` (milestone v0.6.0); Matter runs over IPv6.
  Analysis (2026-10-28, our pinned `base` and `relibc`):
  - **netstack** does Ethernet and ARP itself (`link/ethernet.rs`) and
    runs smoltcp 0.13 only as an IP-level interface (`Medium::Ip`).
    Turning on `proto-ipv6` gives IPv6 sockets, ICMPv6 and MLD, but not
    neighbor discovery or SLAAC: smoltcp only does those on an Ethernet
    interface. So netstack needs its own neighbor discovery next to ARP,
    must accept IPv6 frames (EtherType 0x86DD) and multicast MAC addresses
    (33:33:…), and IPv6 in the router, route table, `netcfg` and the
    socket paths (`[addr]:port`). The code has about ten places that
    assume IPv4 only. Estimate: one to two weeks.
  - **SLAAC**: smoltcp parses router advertisements, but its SLAAC needs the
    Ethernet interface, so netstack has to send router solicitations and
    form the addresses itself (a few days). Alternative: one smoltcp
    Ethernet interface per device, which brings ARP, neighbor discovery and
    SLAAC from smoltcp, but rewrites netstack's link and router layers and
    moves far from upstream.
  - **DHCPv6**: not in smoltcp, and Redox's `dhcpd` is IPv4 only. Matter
    does not need it; the DNS server can come from the router advertisement
    (RDNSS option). A DHCPv6 client is about a week.
  - **relibc**: `AF_INET6` and `sockaddr_in6` exist and `inet_pton` handles
    IPv6, but `socket()` refuses everything except `AF_INET` and
    `AF_UNIX`, and the address conversions in `bind`, `connect`,
    `accept`, `sendto` and the like are IPv4 only (about a week). DNS asks
    for A records only and `gethostbyname2(AF_INET6)` is a TODO; AAAA
    lookups and IPv6 name servers are a few days.
  - **Multicast** (mDNS, also over IPv4): netstack drops all multicast
    frames today, there are no group joins through relibc
    (`IP_ADD_MEMBERSHIP`, `IPV6_JOIN_GROUP`), and the LAN7800 driver only
    lets its own and broadcast addresses through (the RTL815x driver
    accepts all multicast).
  - **Upstream**: no IPv6 work on netstack. relibc has had small IPv6
    changes in 2026 (`inet_pton`, symbols); its maintainers note that IPv6
    functions are of little use without IPv6 sockets. Changes here stay
    patches in our fork, since Redox takes no AI-generated contributions.
  - Order: multicast and IPv6 in netstack, then relibc sockets and AAAA,
    then SLAAC; all testable in QEMU (`-device usb-net` or virtio).
- SSH and Bluetooth ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
