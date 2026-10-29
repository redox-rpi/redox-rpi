# Notes for upstream reports

Pointers for bugs we found that also exist upstream in Redox. They are
not issue text: the maintainer writes and sends every report to the
Redox project. Each note says what breaks, how to reproduce it, where
upstream the code is (at the pinned revisions in
[config/upstream.lock](../../config/upstream.lock)) and where our patch is.

| Note | Upstream project | Our issue | Reported |
|---|---|---|---|
| [aarch64-tls-layout.md](aarch64-tls-layout.md) | relibc (`ld.so`) | [#14](https://github.com/redox-rpi/redox-rpi/issues/14) | no |
| [aarch64-sigreturn-x18.md](aarch64-sigreturn-x18.md) | relibc and kernel | [#1](https://github.com/redox-rpi/redox-rpi/issues/1) | no |
| [aarch64-thread-signals.md](aarch64-thread-signals.md) | relibc | [#4](https://github.com/redox-rpi/redox-rpi/issues/4) | no |
| [getty-wouldblock.md](getty-wouldblock.md) | base (`getty`) | [#32](https://github.com/redox-rpi/redox-rpi/issues/32) | no |

Already reported: the x1/x2 swap in the signal entry,
[relibc#309](https://gitlab.redox-os.org/redox-os/relibc/-/work_items/309)
(see [patches/README.md](../../patches/README.md)).
