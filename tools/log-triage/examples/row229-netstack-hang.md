# Log triage: /home/zehrer/redox-rpi/logs/serial.log:78212:78800

589 lines, 13 noise lines dropped, 1 boots; model gpt-oss-20b, 1 requests, 8.2 s.
Compared with the good log `/home/zehrer/redox-rpi/logs/serial.log:77171:77718`.

| Boot | Lines | Version | Login | Panics | Faults | Errors | Summary |
|---|---|---|---|---|---|---|---|
| 1 | 78212-78798 | v0.4.0-dev-362-gbde20707 | yes | 0 | 0 | 10 | The system booted, logged in, and ran several daemons, but the ping command hung, indicating a network or shell issue. |

## Boot 1 (lines 78212-78798)

- Reached login: yes (regex: yes), logged in: yes
- Hang: The shell command `ping -c 3 192.168.178.120` never returned to the prompt; only the initial PING line was printed.
- Hang hint (regex): the last shell command (L78796: ping -c 3 192.168.178.120) never returned to the prompt; 2 lines after it, last: cat /scheme/sys/context; ping at L78797 never printed its statistics
- New against the good log: +L78658: dwc2d: link up on the LAN7800 at address 6
- New against the good log: +L78690: DHCP: eth0 got 192.168.178.148/24
- New against the good log: +L78691: DHCP: default route via 192.168.178.1 (eth0)
- New against the good log: +L78710: testnetd: network.test0 registered, counters in /tmp/testnetd.log
- New against the good log: +L78718: 192.168.178.148/24
- Summary: The system booted, logged in, and ran several daemons, but the ping command hung, indicating a network or shell issue.
- 155 of 587 lines sent, 1 requests, 8.1 s
