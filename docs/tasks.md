# Open tasks

The tasks are [GitHub issues](https://github.com/redox-rpi/redox-rpi/issues): each says what to do, where to look
and when it is done. Pick one, say in the issue that you take it, and read
[CONTRIBUTING.md](../CONTRIBUTING.md) first. Labels filter by topic
(`topic:…`) and by the hardware you need (`needs:pi`, `needs:qemu`,
`needs:zero2w`, `needs:nothing`). The plan for the next release is the
milestone [v0.3.0](https://github.com/redox-rpi/redox-rpi/milestone/1); the state of each topic is in [topics/](topics/).

Until 2026-10-28 the tasks were written here as T1 to T21. The issue keeps
that name in its title:

| Task | Issue | Topic | Needs |
|---|---|---|---|
| T1 | [#1 Measure the remaining start crashes](https://github.com/redox-rpi/redox-rpi/issues/1) | [stability](topics/stability.md) | QEMU, Pi |
| T2 | [#2 Find the kernel cause of the start crashes](https://github.com/redox-rpi/redox-rpi/issues/2) | [stability](topics/stability.md) | QEMU |
| T3 | [#3 Instruction cache maintenance after loading code](https://github.com/redox-rpi/redox-rpi/issues/3) | [stability](topics/stability.md) | QEMU, Pi |
| T4 | [#4 Signal mask bug in the aarch64 signal entry](https://github.com/redox-rpi/redox-rpi/issues/4) | [relibc](topics/relibc.md) | Pi |
| T5 | [#5 Programs exit with code 1 and print nothing](https://github.com/redox-rpi/redox-rpi/issues/5) | [stability](topics/stability.md) | Pi |
| T6 | [#6 sdhost as the default SD driver](https://github.com/redox-rpi/redox-rpi/issues/6) | [sdcard](topics/sdcard.md) | Pi |
| T7 | [#7 Power off and reboot](https://github.com/redox-rpi/redox-rpi/issues/7) | [firmware](topics/firmware.md) | QEMU, Pi |
| T8 | [#8 Raspberry Pi Zero 2 W board variant](https://github.com/redox-rpi/redox-rpi/issues/8) (closed) | [zero2w](topics/zero2w.md) | Zero 2 W |
| T9 | [#9 CI builds with the pinned revisions](https://github.com/redox-rpi/redox-rpi/issues/9) | [ci](topics/ci.md) | none |
| T10 | [#10 Start loop in `pi-test.py` with crash details](https://github.com/redox-rpi/redox-rpi/issues/10) | [ci](topics/ci.md) | QEMU, Pi |
| T11 | [#11 devshd sessions leave `login` processes behind](https://github.com/redox-rpi/redox-rpi/issues/11) | [stability](topics/stability.md) | Pi |
| T12 | [#12 Programs fail at start with exit code 1](https://github.com/redox-rpi/redox-rpi/issues/12) | [stability](topics/stability.md) | QEMU, Pi |
| T13 | [#13 relibc tests on Redox in CI](https://github.com/redox-rpi/redox-rpi/issues/13) | [ci](topics/ci.md) | none |
| T14 | [#14 Deeper analysis of relibc](https://github.com/redox-rpi/redox-rpi/issues/14) | [relibc](topics/relibc.md) | none |
| T15 | [#15 Boot the kernel straight from the Pi firmware](https://github.com/redox-rpi/redox-rpi/issues/15) | [boot](topics/boot.md) | Pi |
| T16 | [#16 RedoxFS writes 36 times the data for small writes](https://github.com/redox-rpi/redox-rpi/issues/16) | [sdcard](topics/sdcard.md) | QEMU, Pi |
| T17 | [#17 USB hotplug](https://github.com/redox-rpi/redox-rpi/issues/17) | [usb](topics/usb.md) | QEMU, Pi |
| T18 | [#18 No `pcid` and `audiod` on the Pi](https://github.com/redox-rpi/redox-rpi/issues/18) | [boot](topics/boot.md) | QEMU, Pi |
| T19 | [#19 Fast network boot on the Zero 2 W](https://github.com/redox-rpi/redox-rpi/issues/19) | [zero2w](topics/zero2w.md) | Zero 2 W |
| T21 | [#20 CPU clock and throttling watch through the firmware](https://github.com/redox-rpi/redox-rpi/issues/20) | [firmware](topics/firmware.md) | Pi |

New tasks get an issue only, no T number.
