# CI and tests

## State

- GitHub Actions boot the kernel in QEMU `raspi3b` on every push and run
  relibc's `sigregs` on Linux.
- Tests on the real Pi go through a shared queue,
  [test-plan.md](../test-plan.md), and `scripts/pi-test.py`; results are in
  [test-reports.md](../test-reports.md).
- Every night at 03:30 UTC the build host builds `main`, tests it on the Pi
  with `scripts/pi-test.py` and makes it the stable network-boot image if it
  passes, followed by a short chaos stressor set with a time limit
  ([chaos.md](chaos.md)); a failure goes to the issue "Nightly: main fails on the Pi"
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
- Every Monday the "Upstream drift" job applies every patch, experiments
  included, on the pinned and on the newest upstream commits
  (`scripts/check-upstream-drift.py`) and updates the issue "Upstream drift
  report": per patch applies, 3-way, already upstream or conflicts, with the
  upstream commits behind each conflict. It never fails.

## In progress

- [#10](https://github.com/redox-rpi/redox-rpi/issues/10) crash details in `pi-test.py`, branch `t10-start-loop`,
  [PR #74](https://github.com/redox-rpi/redox-rpi/pull/74): each failed start with its program, exit
  status and its own kernel crash dump (FAR, ELR, ESR, LR, SP, PID, stack
  walk); `--crash-log` writes them as JSON lines for counting start crashes
  (#1); starts Ion could not run count as failures. Pi tests #48 and #92
  passed (#92: 1000 starts on the stable image, 6 failures, 6 records).

- [#200](https://github.com/redox-rpi/redox-rpi/issues/200) log triage with the local model, branch `log-triage`
  ([below](#log-triage)).

## Next

- [#9](https://github.com/redox-rpi/redox-rpi/issues/9) CI with the pinned revisions; [#13](https://github.com/redox-rpi/redox-rpi/issues/13) relibc tests on
  Redox.

## Log triage

homenodellm runs vLLM with `gpt-oss-20b` (unit `vllm.service`, config
`/etc/vllm/vllm.env`, `http://127.0.0.1:8000/v1`, 16k context, about
43 tokens/s). It listens on 0.0.0.0 without an API key, so anyone on the
home network can use it.

- `tools/log-triage/log-triage.py LOG [--baseline GOOD]` pre-sorts a serial or
  QEMU log: per boot, reached login, panics and faults with line and program,
  failed daemons, first error, hang, lines new against a good log. Markdown
  and JSON; examples in `tools/log-triage/examples/`. Details in its README.
- Regexes find the facts and pick the lines; the model only reads boots that
  did not end clean, about 100–200 lines each (1k–6k tokens).
- Speed: one boot 2–25 s (mostly 5–15 s). The day log
  `serial-2026-11-01T0317.log.gz` (120k lines, 135 boots, 14 not clean) takes
  52 s with 4 boots in parallel; `--no-model` takes 6 s.
- Accuracy on 17 boots checked by hand (a good boot, the netstack hang of
  test 229, a QEMU fault, and the 14 not-clean boots of 2026-11-01):
  reached login 17/17; panics and faults with the right line and program
  16/17 (the sigregs test's 1000 handled faults were blamed on the kernel);
  hangs and boots that stopped early 5/5, no false hang; failed daemons
  right where named. The one-line summary had a wrong cause in 7 of 17
  ("failed to mount, leading to a crash in /tmp/r117"). **Use the facts and
  line numbers, not the reasons.**
- OpenCode 1.19.5 for small agent tasks on homenodellm:
  `~/.opencode/bin/opencode` (official script, not on PATH), provider
  `vllm/gpt-oss-20b` in `~/.config/opencode/opencode.json`, autoupdate and
  sharing off. `opencode run "…"` in a folder works with tool calls (glob,
  read): a list-and-read task took 6 s.
