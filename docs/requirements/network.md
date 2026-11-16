# Network (`NET`)

Topic: [network](../topics/network.md). Trace: [trace-v0.4.0.md](trace-v0.4.0.md).

### REQ-NET-1 DHCP on the onboard Ethernet

The system shall get an IPv4 address by DHCP on the Pi 3B+'s onboard
Ethernet (LAN7800) at every boot and accept TCP connections.

| Field | Value |
|---|---|
| Rationale | NTP, remote use and all automated tests need the network. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.3.4, 2.4.2 |
| Status | proposed |
| Since | v0.1.0-alpha |
| Implemented by | LAN7800 in `dwc2d`; #217 and #92 (open: sometimes no network) |
| Verification | test, automated: checklist v0.4.0 test 1 (pi-test logs in over the network), rows 292 (v0.4.0 build) and 285 (rc1); **gap**: no count of boots without network (#259) |

Acceptance criteria:

1. Within 60 s after power-on, `/scheme/netcfg/ifaces/eth0/addr/list` shows
   the address the DHCP server gave.
2. A TCP connection from the LAN to a listening service is accepted.
3. Network up on 100 of 100 boots (proposed; #217 and #92 are the known
   failures).
4. The network stays up for at least 1 hour at idle (proposed; #92).

### REQ-NET-2 USB Ethernet adapters

The system shall get an address by DHCP through an RTL8152 or RTL8153 USB
Ethernet adapter, on the Pi 3B+ and on the Zero 2 W.

| Field | Value |
|---|---|
| Rationale | The Zero 2 W has no Ethernet of its own. |
| Priority | must (Zero 2 W), should (Pi 3B+) |
| Kind | functional |
| Level | release |
| PBS | 2.3.4 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | base `0020` |
| Verification | test, person: checklist v0.4.0 tests 9 and 19, row 286 (rc2 visit, pass); v0.3.0: visit 193 |

Acceptance criteria:

1. On a Zero 2 W with an RTL8153 hub, `ifconfig` shows the adapter with
   an address from DHCP within 60 s after power-on.

### REQ-NET-3 Several adapters at once

The network stack shall use every Ethernet adapter at the same time, with a
DHCP lease on each.

| Field | Value |
|---|---|
| Rationale | A Pi with onboard Ethernet and a USB adapter must not lose one of them. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.4.2 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | PR #81 (from #53, which now covers only adapters that appear after boot) |
| Verification | test, person: checklist v0.4.0 test 19, row 286 (rc2 visit, pass) |

Acceptance criteria:

1. With the onboard LAN and a USB adapter both cabled, each has its own
   lease and answers on its address.

### REQ-NET-4 Long TCP transfers

The system shall receive a long TCP transfer without stalling.

| Field | Value |
|---|---|
| Rationale | Package downloads and image updates are long transfers. |
| Priority | must |
| Kind | quality |
| Level | release |
| PBS | 2.4.2, 2.2.1 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #157, #88, #119 (open), #140 (open), #141 (open) |
| Verification | test, automated: checklist v0.4.0 test 20, rows 292 (v0.4.0 build) and 285 (rc1) |

Acceptance criteria:

1. 100 MiB over HTTP from the build host arrive complete (checksum right)
   with no pause longer than 10 s (proposed).
2. Connections that ended are freed: 1000 short connections in a row leave
   no growing number of open sockets (proposed; **gap**, #88).

### REQ-NET-5 IPv4 multicast

The system shall send and receive IPv4 multicast, also through the
LAN7800.

| Field | Value |
|---|---|
| Rationale | mDNS and smart-home protocols use multicast. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.3.4, 2.4.2 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #27, PR #194, base `0083`, `0084`; side effect #216 (open) |
| Verification | test, automated: row 218 (mDNS experiment on a dev build); **gap** in the v0.4.0 checklist |

Acceptance criteria:

1. A program that joins 224.0.0.251 receives a packet sent to it from the
   LAN, and a packet it sends there reaches the LAN.

### REQ-NET-7 Name lookup (DNS)

The system shall resolve host names with the name server and local domain
it gets by DHCP, and fail at once for an unknown name.

| Field | Value |
|---|---|
| Rationale | curl, pkg and SSH need names; a wrong or slow lookup looks like a dead network. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.4.2 |
| Status | proposed |
| Since | v0.5.0 (proposed for v0.6.0) |
| Implemented by | #290, PR #358, PR #381 |
| Verification | test, automated: row 414 (router name, short name and an unknown name with curl); QEMU regression set `dns` |

Acceptance criteria:

1. The name server and the local domain (for example `.lan`) come from
   DHCP; a fully qualified name and a short name of a LAN host resolve.
2. A name outside the LAN resolves (for example the package server).
3. An unknown name fails with `EAI_NONAME` within 1 s (proposed; 0.22 s in
   row 414).
4. If the name server does not answer, the lookup retries and then fails
   within 10 s (proposed) instead of hanging.

New for v0.6.0 (Requirements agent, 2026-11-16): written from #290 and the
v0.5.0 release notes. REQ-NET-6 is reserved by the parked RFC 0003 (SSH).
Impact: release checklist (DNS step), the QEMU `dns` check, row 414's
step 10.
