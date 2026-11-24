# Triage of the old open pull requests (draft)

State 2026-11-12, by the build agent for the lead. Covers the 44 pull
requests in redox-rpi/redox-rpi that were opened before 2026-11-10 and are
still open (#64 to #229). Nothing here is closed or merged: closing and
merging stay with the maintainer.

How it was checked: every head fetched, diffed against its merge base, and
each file it changes compared with `origin/main` (identical, differs, or
not on main); patch numbers checked against `patches/*/` on main; linked
issues checked for their state. "Behind" is how many commits main has
moved on since the branch. Nothing was built or booted for this page.

**Recommendations:**
- **close**: superseded by main, or by another pull request; the
  maintainer closes it.
- **rebase + test**: still wanted, not on main. The owner rebases it onto
  main and queues a Pi row before merge-next.
- **decide**: needs the maintainer's choice first, about direction or
  release content.
- **park**: an experiment worth keeping, but not for the next weeks. Leave
  it open as a draft, or close it with a pointer in the issue.

## Summary

| Recommendation | Pull requests |
|---|---|
| close (superseded) | #64, #65, #72, #130, #150, #170, #120 (FIN part), #139 |
| rebase + test, high value | #142 (#4), #121 (#55), #174 (#108), #172 (#171), #138 (#129), #164 (with #309) |
| rebase + test | #80, #103, #118, #123, #133, #134, #135, #143, #144, #153, #154, #176, #191, #208, #229 |
| decide | #112, #150 and #152 (which prebuilts), #189, #198, #213, #226 |
| park (experiments, own topics) | #70, #97, #89, #116, #132, #136, #202 |
| Network has it | #209 |
| docs | #162 |

## Table

| PR | What | Base, state | Against main | Owner (topic) | Recommendation |
|---|---|---|---|---|---|
| #64 | relibc 0008/0009 thread signals (#4) | `t13-relibc-tests`, behind 1647 | replaced by #142 (same patches, on main) | relibc | **close**: superseded by #142 |
| #65 | relibc 0010 signals for exceptions, sigaltstack | `pr-thread-signals`, draft, behind 1636 | replaced by #143 | relibc | **close**: superseded by #143 |
| #70 | USB serial adapters in dwc2d (#50) | main, draft, behind 787 | base 0035 free; dwc2d changed a lot since (#344, #373) | USB | **park**: rebase against today's dwc2d when #50 is planned |
| #72 | RedoxFS rename on non-root filesystems (#57) | main, conflicts | `patches/redoxfs/0036-Rename…` and the experiment are on main (#73 rebased them) | SD card | **close**: superseded; check whether #57 can close |
| #80 | initnsmgr: complete /scheme listings (#79) | main, conflicts, behind 1291 | base 0036 free, not on main | Network | **rebase + test** if #79 is still seen; otherwise close with #79 |
| #89 | MQTT client, ha-bridge, MQTT over TLS (#49, #56) | main, conflicts | not on main | smart home (no owner today) | **park**: needs an owner; HomeNode work may replace it |
| #97 | relibc baud rates (termios-speed) | `usb-serial` (#70), draft | stacked on #70 | USB | **park** with #70 |
| #103 | lsusb and hwinfo as defaults (#35) | main, conflicts, behind 769 | base 0053 free; touches dwc2d experiments that moved | USB | **rebase + test** (small user-visible win) |
| #112 | SD image that grows to the card on first boot (grow-root, #24) | main, draft, conflicts | not on main | Boot | **decide**: overlaps the direct-boot card (#324) and first-boot setup (#24); after #324 |
| #116 | MQTT client certificates (#115) | `mqtt-tls` (#89), draft | stacked on #89 | smart home | **park** with #89 |
| #118 | netstack/dhcpd lease renewal (base 0058) | `netstack-multi`, draft | 0058 free; experiment netstack-multi is on main | Network | **rebase + test** onto main (retarget); Network decides with #92/#217 |
| #120 | relibc quiet madvise, IP_TOS, TCP_NODELAY for sshd (#109) | `sshd-keys`, behind 835 | its base 0047 (TCP FIN) is superseded by #364 (merged); the relibc 0045 part is not on main | relibc / Network | **close** the FIN part; the relibc part goes into #191 or a new small PR |
| #121 | Ctrl-C while a program starts (pty-sigint, #55) | main, mergeable | experiment not on main; #55 is a known issue | stability | **rebase + test**, high value (known issue #55) |
| #123 | ld.so aarch64 static TLS layout (#14) | `t13-relibc-tests` (stale), draft | not on main | relibc | **rebase + test**: retarget to main, re-check with the relibc bump (#309 step 1) |
| #130 | Assistant: Qwen3 with rinfer (#43) | main, conflicts | experiment on main (22 files identical, #204 and others); 5 files differ | assistant | **close** after the assistant owner moves anything still missing into a new PR |
| #132 | German keyboard layout (keyboard-de, #36) | main, draft, conflicts | not on main; inputd changed (#376) | USB | **park**: rebase when #36 is planned; needs a person keyboard test |
| #133 | USB sticks in dwc2d (#25) | main, draft | not on main | USB | **rebase + test** against today's dwc2d |
| #134 | Card readers, several sticks (#52) | `usb-storage` (#133), draft | stacked on #133 | USB | **rebase + test** after #133 |
| #135 | timesync: median of three NTP servers | `clock-settime` (stale), draft | timesync is on main; this change is not | Time | **rebase + test**: retarget to main |
| #136 | Bluetooth (bluetooth, #54) | main, conflicts, behind 1280 | not on main; base 0071–0076 inside the experiment | firmware | **park** until #54 is planned |
| #138 | devshd: log failed accepts, bind again (#129) | main, mergeable, behind 1261 | not on main | CI | **rebase + test**, small; helps automated rows |
| #139 | dwc2d waits for the channel interrupt | `usb-storage-more`, draft, conflicts | dwc2d's waiting changed with #373 (yield loop) and #344 | USB | **close** as superseded by #373's approach, unless the USB owner wants the interrupt wait measured again |
| #142 | relibc 0008/0009 thread signals, on main (fixes #4) | main, mergeable | 0008/0009 free on main; #4 is a known issue | relibc | **rebase + test**, high value (known issue #4); Pi row with the relibc tests |
| #143 | relibc 0014 signals for exceptions, on main | `thread-signals-main` (#142), draft | 0014 free; stacked on #142 | relibc | **rebase + test** after #142 |
| #144 | netstack stale ARP, buffered data before ENOTCONN (base 0082), netget | main, draft, conflicts | 0082 free; `netget` is in no image (row 326) | Network | **rebase + test**: Network decides; netget may be dropped now that curl is in |
| #150 | prebuilt programs back in the images (#40) | `sigreturn-default`, draft, conflicts | experiment on main; prebuilts run since #137; curl is in via #368 | relibc | **close** as superseded; **decide** which other prebuilts (bash, vim, …) go into the default image |
| #152 | prebuilt programs: allowlist and checks (#40) | main, draft | allowlist tools not on main | relibc | **rebase + test** if prebuilts go into the default image (see #150) |
| #153 | icache-sync as default kernel 0010/0011 (#3) | `t3-icache`, conflicts | **number clash**: main has kernel 0010 (pipe); experiment is on main | stability | **rebase + test**: renumber, retarget to main |
| #154 | relibc: calls with two fds (rt-call-flags backport) | main, draft | not on main | relibc | **rebase + test**; drop it if the relibc bump (#309 step 1) already has upstream 112eb43c |
| #162 | User guide: USB devices section | main, draft | describes USB features partly not on main | USB | **docs**: trim to what main has, then merge as docs |
| #164 | base 0024 (getty) applies to the pinned and newest base | main, mergeable | 0024 on main differs; this is the refresh #309 needs (0024 conflicts upstream) | build agent / stability | **rebase + test** as part of upstream-bump step 2 (#309) |
| #170 | relibc tests: expected failures after rc4 test 15 | main, draft | list on main has moved on | relibc | **close**: stale; the relibc owner refreshes the list from a new run |
| #172 | ifconfig shows the MAC address (#171) | main, conflicts, behind 257 | not on main; #171 is in the v0.5.0 milestone | Network | **rebase + test**, high value (known gap since v0.3) |
| #174 | kernel 0013 self-exit-status by default (#108) | main, draft, conflicts | 0013 free on main; experiment on main; #108 is a known issue | stability | **rebase + test**, high value (known issue #108) |
| #176 | U-Boot r8152 16 KiB receive, Zero 2 W netboot over HTTP (#19) | main, draft, conflicts | no U-Boot patches on main | Zero 2 W | **rebase + test** with a Zero 2 W netboot row |
| #189 | rpi-update on the direct-boot card, tryboot fallback (#15) | main, draft | not on main | Boot | **decide** after #324 (direct-boot card) |
| #191 | sshd: OpenSSH 10.6p1 (#190) | `tcp-close`, draft | its FIN base is merged (#364); retarget to main | Network | **rebase + test**: retarget to main, take #120's relibc part |
| #198 | Boot time step 2: no U-Boot countdown, 192 MiB image (#47) | main, draft, conflicts | not on main | Boot | **decide**: with the direct-boot card (#324) U-Boot remains only for netboot |
| #202 | Wi-Fi on the Pi 3B+ (wifi-sdio, #26) | `netstack-multi`, draft | stacked on #118 | Network | **park** behind #118 |
| #208 | redoxfs: free the last partial record (#197) | main, draft | not on main; redoxfs 0037–0040 on main sit in the same area | SD card | **rebase + test** with the redoxfs tests |
| #209 | kernel: TLB shootdowns while a context switch waits (#207) | main, conflicts | not on main | Network (per lead) | Network has it |
| #213 | Nightly: short chaos stressor set (#196) | main, draft, conflicts | update-stable.sh changed a lot (#325, #343, #363, #369) | chaos / CI | **decide**: the nightly now runs pi-test --extended plus a 15-min soak; chaos adds up to 10 min more; then rebase |
| #226 | Experiment opt-a53: speed build for the Cortex-A53 | main, draft | not on main | build agent | **decide**: benchmark first; experiment only |
| #229 | cputop: sleep the full round time | main, mergeable | not on main | stability | **rebase + test** (QEMU check of cputop) |

## Questions for the maintainer

1. **Close the superseded ones?** #64, #65, #72, #130 (after the owner's check), #150, #170, and the FIN part of #120. Can the build agent close them with a pointer comment, or do you close them?
2. **Prebuilt programs:** which of bash, vim, nano, tmux, ripgrep etc. should go into the default image (#150, #152)? curl is in via #368.
3. **Nightly length:** with #363 and #369 the nightly grows by about 15 minutes. Should the chaos set of #213 (up to 10 more minutes) go in too?
4. **Direct-boot follow-ups:** #112, #189 and #198 wait for #324.
