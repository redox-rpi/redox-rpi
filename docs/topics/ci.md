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
- `scripts/pi-test.py --serial-log --crash-log` records the crash output of
  every failed start (#10). `scripts/qemu-raspi3b.sh --data` gives QEMU a
  data partition, so all standard checks also run in QEMU.
- `scripts/pr-order.py` lists the open pull requests in an order they can be
  merged in, with CI, Pi test, patch numbers and conflicts.
- The "Topic pages" job warns when a pull request or a push to main
  changes a topic's files without touching its page here.
  `config/topics.map` maps paths to topics. It is a warning only.

## In progress

- [#10](https://github.com/redox-rpi/redox-rpi/issues/10) crash details in `pi-test.py`, branch `t10-start-loop`,
  [PR #74](https://github.com/redox-rpi/redox-rpi/pull/74): each failed start with its program, exit
  status and its own kernel crash dump (FAR, ELR, ESR, LR, SP, PID, stack
  walk); `--crash-log` writes them as JSON lines for counting start crashes
  (#1); starts Ion could not run count as failures. Pi tests #48 and #92
  passed (#92: 1000 starts on the stable image, 6 failures, 6 records).

## Next

- [#9](https://github.com/redox-rpi/redox-rpi/issues/9) CI with the pinned revisions; [#13](https://github.com/redox-rpi/redox-rpi/issues/13) relibc tests on
  Redox.
