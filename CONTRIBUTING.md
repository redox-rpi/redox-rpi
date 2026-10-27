# Contributing

Help is welcome, especially from people who point their own AI coding agent
at one of the [open tasks](docs/tasks.md). Each task is small enough for one
agent session or a few, says where to look, and says when it is done.

This is an unofficial fork for the Redox OS™ operating system. Redox does
not accept AI-generated contributions, so nothing from here goes upstream.
Report problems with this fork here, not to the Redox OS project.

## Workflow

1. **Pick a task** from [docs/tasks.md](docs/tasks.md) or the
   [issues](https://github.com/redox-rpi/redox-rpi/issues). Comment on its
   issue that you are working on it, so two agents do not do the same thing.
2. **Fork and clone** this repository. Read [docs/architecture.md](docs/architecture.md)
   and [docs/versioning.md](docs/versioning.md) first, then the documents
   the task links to.
3. **Analyse.** Reproduce the problem before changing anything (see
   [Tests](#tests)). Write down what you found in the issue, also when it is
   a dead end: a ruled-out cause saves the next agent a day.
4. **Change.** Changes to Redox components (kernel, relibc, base,
   bootloader) are patch files, not edits in the downloaded sources:
   - Clone the component at the commit from
     [config/upstream.lock](config/upstream.lock), apply our patches with
     `git am patches/<component>/*.patch`, commit your change on top.
   - Export it with `git format-patch -1 -o patches/<component>/ --start-number <next>`.
   - For relibc, [redox-rpi/relibc](https://github.com/redox-rpi/relibc)
     has this ready: branch `redox-rpi` is the pinned commit plus
     `patches/relibc/`, `upstream` the pinned commit, and `exp/<name>` each
     relibc experiment on top of `redox-rpi`. Static and dynamic analysis
     and tests of relibc can run there; the patch files here stay what the
     build uses.
   - A change you are not sure about goes to
     `patches/experiments/<name>/<component>/` with a README that states the
     idea and, once tested, the result. Experiments that did not help stay
     in the repository: say so in the README.
5. **Test.** Build with `scripts/build-image.sh` (it refuses uncommitted
   changes, so commit first) and run the tests that fit your change. Name
   the build ID in your results.
6. **Open a pull request**, one topic per pull request. Include:
   - what changed and why, and what you ruled out on the way;
   - the build ID and the test results (paste the `scripts/pi-test.py`
     output, or the QEMU log lines);
   - the agent and model you used, and what a human did (directions,
     manual edits, hardware handling).

## Rules

- MIT license for new code. Do not copy code from GPL sources such as Linux
  drivers; datasheets, documentation and reading for understanding are fine.
- Keep patches small and close to upstream, so the fork can follow new Redox
  releases.
- No `--allow-dirty` builds in results you report: they cannot be
  reproduced.
- Never commit credentials. `devshd` (remote shell without encryption) is
  for test networks only and stays out of release images.
- Write documentation in English, in short, plain sentences.

## Tests

| Level | Needs | Command | Details |
|---|---|---|---|
| Kernel boot in QEMU | macOS or Linux, QEMU | `scripts/qemu-raspi3b.sh --test --kernel <kernel>` | [testing.md](docs/testing.md#automated-boot-test) |
| Own image in QEMU, commands in the guest | Linux build host | `scripts/qemu-raspi3b.sh --image <harddrive.img> --run '<command>'` | [testing.md](docs/testing.md) |
| Program start loop in QEMU (start crashes) | Linux build host | `scripts/qemu-run.sh 'for i in 1..=40; ls /; pwd; end'` | [debugging-random-crashes.md](docs/debugging-random-crashes.md) |
| Real Pi 3B+ | Pi netbooted from your build host | `scripts/pi-test.py <pi-address> --manifest build/manifests/netboot-latest.toml --report docs/test-reports.md` | [netboot.md](docs/netboot.md), [test-reports.md](docs/test-reports.md) |
| CI | nothing, runs on every push | GitHub Actions `qemu-boot.yml` | [Actions](https://github.com/redox-rpi/redox-rpi/actions) |

Without a Pi, QEMU is enough for most tasks: it emulates the Pi 3 closely
enough to boot our images, but not the Ethernet chip, the USB keyboard path
or the exact timing of the real Cortex-A53. Results from a real Pi are
welcome for every change; add them to
[docs/test-reports.md](docs/test-reports.md) in the pull request.
