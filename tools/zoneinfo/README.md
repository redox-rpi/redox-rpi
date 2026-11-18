# zoneinfo

Time zone files (TZif) for `/usr/share/zoneinfo`, as a package for the
image (experiment `timezone`, issue
[#46](https://github.com/redox-rpi/redox-rpi/issues/46)). Rust programs
such as uutils `date` read them through `/etc/localtime` or `TZ`; relibc
has the tz database built in and only needs the zone's name.

| File | From |
|---|---|
| `share/zoneinfo/Europe/Berlin` | IANA tzdata 2026e (Ubuntu package `tzdata` 2026e-0ubuntu0.26.04.1) |
| `share/zoneinfo/Etc/UTC` | the same |

To add a zone, copy its file from a Linux system's `/usr/share/zoneinfo`
(same relative path) and note the tzdata version here.
