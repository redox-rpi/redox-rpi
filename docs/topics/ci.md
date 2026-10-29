# CI and tests

## State

- GitHub Actions boot the kernel in QEMU `raspi3b` on every push and run
  relibc's `sigregs` on Linux.
- Tests on the real Pi go through a shared queue,
  [test-plan.md](../test-plan.md), and `scripts/pi-test.py`; results are in
  [test-reports.md](../test-reports.md).
- Every night at 03:30 UTC the build host builds `main`, tests it on the Pi
  with `scripts/pi-test.py` and makes it the stable network-boot image if it
  passes; a failure goes to the issue "Nightly: main fails on the Pi"
  ([test-plan.md](../test-plan.md#nightly-update)). The Pi may reboot at
  night.

## Next

- [#9](https://github.com/redox-rpi/redox-rpi/issues/9) CI with the pinned revisions; [#13](https://github.com/redox-rpi/redox-rpi/issues/13) relibc tests on
  Redox; [#10](https://github.com/redox-rpi/redox-rpi/issues/10) crash details in `pi-test.py`.
