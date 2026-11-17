# Experiment: dwc2d-async-link, ready without waiting for the link (#536)

Issue: [#536](https://github.com/redox-rpi/redox-rpi/issues/536), from the
boot-time analysis in rpi-open-firmware #44 (pi-tests Test 470).

## Problem

Since base `0034` (was experiment `dwc2d-boottime`), `dwc2d` waits at boot
for the first network link (up to 10 s, `BOOT_LINK_WAIT`) before it reports
ready, so DHCP does not start into a dead cable. `dwc2d` is a `notify`
service and init starts nothing else meanwhile. On the Pi 3B+ that wait is
the LAN7800's autonegotiation, about 3.1 s from the chip init to the link
(Test 470: `LAN78xx chip` at +3.38 s, `link up` at +6.49 s after kernel
start), also on images without a network stack.

## Change

Base patch [`0100`](base/0100-dwc2d-report-ready-without-waiting-for-the-network-l.patch):

- After the 1 s look for devices that connect late (`BOOT_CONNECT_WAIT`,
  unchanged), `dwc2d` reports ready when it found one network adapter (or
  none), without waiting for its link. It prints `devices ready after N ms,
  link not up yet` when the link is not up.
- **Several adapters at boot** (a Pi 3B+ with an RTL815x plugged in): which
  adapter the network stack gets, and whose MAC address it keeps, depends
  on the link (an RTL815x with a cable wins over the LAN7800). So `dwc2d`
  waits for the first link as before (up to `BOOT_LINK_WAIT`, 10 s) and
  nothing is held. Without this, the stack would keep the LAN7800's
  address and the RTL815x would take it over (`select()` sets it into the
  chip), so the RTL815x would get another DHCP lease than before (Quality
  review of PR #545).
- Until the adapter in use has its link, the frames the network stack sends
  wait in a queue in `dwc2d` (`UsbNet::held`, at most 32, the oldest go
  first) instead of going into the dead link. The link is read every 100 ms
  (`LINK_WATCH`); when it comes up, the queue is sent and `dwc2d: N of M
  held frame(s) sent` is printed. So dhcpd's first Discover leaves at
  link-up, within its 5 s reply timeout: still no DHCP start without a link
  (#217, #92), with no change in dhcpd or netstack.
- The wait ends without a link when the oldest held frame is 5 s old
  (`HELD_MAX_AGE`, dhcpd's reply timeout: by then dhcpd has given up on that
  answer, and the cable is probably out): the queue is dropped, `dwc2d: no
  link yet` is printed, and dhcpd retries as it does today. With nothing
  sent (an image without a network stack) the link is watched for 10 s
  (`BOOT_LINK_WAIT`), then only by the rescans; this end prints nothing, so
  such a log has no `no link yet`. A fixed 10 s from ready was
  tried first: in QEMU netstack sends its first frame about 8 s after
  `dwc2d` is ready, so the queue was dropped just before the link came up.
- An RTL8153 whose gigabit autonegotiation takes more than 5 s gets its
  address from dhcpd's second attempt, 5 s later than with the old wait.
- An adapter that connects after the 1 s look takes over through the
  existing `select()` path with the address the stack has, as for an
  adapter plugged in after boot.

## Test only: dwc2d-async-link-test

QEMU's CDC Ethernet card has its link at once, so the queue is never used
there. `--experiment dwc2d-async-link-test` (patch `0900`, test-only, needs
this experiment) makes the cards report no link at boot, and a link 2 s
after the first frame was held (one card) or 2 s after the wait for the
first link began (several cards).

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 \
    --experiment dwc2d-async-link [--experiment dwc2d-async-link-test]
```

## Results

- **QEMU, Pi image** `v0.6.0-dev-72-g9ea1b60b+xa494`: regression set 7 pass,
  0 fail; `devices ready after 375 ms` (the CDC card has its link at once).
- **QEMU, with `dwc2d-async-link-test`** `v0.6.0-dev-73-g26ea51f9+x4b57`,
  3 of 3 boots: `devices ready after 354 ms, link not up yet`, then
  `link up`, `1 of 1 held frame(s) sent` and `DHCP: eth0 got` on the first
  attempt; regression set passes.
- `scripts/test-hotplug.sh`: 12 of 15, the same 3 failures as on main
  (the keyboard typing step and two dhcpd messages dhcpd no longer prints).
- **Pi 3B+, pi-tests row 478** (2026-11-17, against stable
  `v0.6.0-dev-65-gbed4882f+x2d42`, 3 power-cycle boots each):
  `devices ready` 1.28–1.35 s instead of 4.26 s; the banner before the
  login prompt 2.94–2.96 s earlier (5.55–5.57 s after the kernel's first
  line instead of 8.50–8.52 s); `DHCP: eth0 got` 1.14 s earlier, on the
  first attempt, with the held Discover sent at link-up; no `no link yet`;
  devsh and `scripts/pi-test.py` 6/6 as on stable. The report counts the
  prompt line, which the capture stamps only at its next newline (in two
  boots dwc2d's `link up`), see #536.
- **Not tested:** a boot without a cable (person), an RTL8153 with slow
  gigabit autonegotiation, the firmware's minimal image.
