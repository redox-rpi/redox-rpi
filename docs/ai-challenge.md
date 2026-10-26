# AI challenge: port an OS to a Raspberry Pi

> **Draft / request for comments.** Nothing here is final, nobody has signed
> up, and no company is involved. Feedback is welcome in
> [Discussions](https://github.com/redox-rpi/redox-rpi/discussions).

## Idea

A long, real-world task for AI coding agents: bring
[Redox OS](https://www.redox-os.org/) from its current state to a working
server on the Raspberry Pi 3 and Zero 2 W. Who gets furthest, with the least
effort, and with the best code?

Unlike most coding benchmarks, the task takes many sessions, touches a
kernel, drivers and a build system, and ends on real hardware that either
boots or does not.

This is independent of the Redox OS project, which does not accept
AI-generated contributions. Results live in forks.

## Stages

Each stage is checked automatically, first in QEMU `raspi3b`, then on real
boards. They follow the [roadmap](../ROADMAP.md).

| # | Stage | Check |
|---|---|---|
| 1 | Kernel log on the serial console | Expected log lines on UART |
| 2 | Login shell over serial | Scripted login, `uname -a` succeeds |
| 3 | Root filesystem on the SD card | Reboot keeps a written file |
| 4 | All 4 cores | `/scheme/sys/cpu` lists 4 CPUs |
| 5 | Ethernet | `ping` and an HTTP request from the test host |
| 6 | Zero 2 W with 512 MB RAM | Stages 2–5 on the Zero 2 W |

## Starting point and rules

- **Fixed base:** a pinned revision of the Redox repositories and of this
  repository's tools, published before the start.
- **Same hardware:** Pi 3 Model B+ and Zero 2 W with a specific
  Ethernet/USB hub HAT, connected to the test rig.
- **Licensing:** MIT for all new code. No code copied from GPL sources such
  as Linux drivers; datasheets and documentation are fine. Checked with a
  plagiarism scan.
- **Disclosure:** every human intervention is logged (prompts, hints, manual
  edits). A run with fewer interventions ranks higher at the same stage.
- **Full logs:** the complete agent transcript and git history are
  published with the result.

## Metrics

| Category | Metric | Notes |
|---|---|---|
| Result | Highest stage reached | Primary ranking |
| Effort | Cost in USD | Tokens are not comparable across vendors (different tokenizers) |
| Effort | Wall-clock time | |
| Effort | Number of human interventions | |
| Code | Lines changed against the base revision | Smaller is better at the same stage |
| Code | Number of `unsafe` blocks added | |
| Code | Clippy warnings, `rustfmt` clean | |
| Code | Documented public items | |
| Code | Blind review by humans | Reviewers do not know which agent wrote the code |

## Test rig

- **Now:** QEMU `raspi3b` driven by
  [`scripts/qemu-raspi3b.sh`](../scripts/qemu-raspi3b.sh) in CI.
- **Later:** a small rack with Pi 3 and Zero 2 W boards, USB-serial
  adapters, switchable power (USB relay or PoE) and an SD card multiplexer,
  so a script can flash, power-cycle and check each stage without a human.

## Reference run: this project

This project is itself developed with an AI coding agent. Its progress is
logged here as a first data point.

| Date | Stage | Human interventions | Effort | Notes |
|---|---|---|---|---|
| 2026-10-24 | 2 in QEMU (partly) | Direction and approvals only | Not measured | Shell reached after fixing a devicetree bug; most programs crash on an upstream regression |
| 2026-10-24 | 4 in QEMU | Direction and approvals only | Not measured | All 4 cores online: spin-table and BCM2836 IPIs, working on the first boot attempt |
| 2026-10-25 | 2 on a real Pi 3B+ (login prompt) | Direction, approvals, hardware handling (SD card, power, photos) | Not measured | Network boot from the build server; prompt on HDMI; no keyboard input yet |
| 2026-10-25 | 4 on a real Pi 3B+; USB started | Same | Not measured | 4 CPUs online on real hardware; new DWC2 driver enumerates the onboard hub on its first real run (one fix: devicetree name) |
| 2026-10-26 | 5 on a real Pi 3B+ (Ethernet) | Same, plus router settings | Not measured | CDC Ethernet in QEMU first (DHCP, ping), then LAN7800 on real hardware: DHCP lease, TCP answers. Fixes on the way: stalls after reset, hub port timing, MAC from devicetree |

## Open questions

- How to make the hardware stages fair and reproducible for remote
  participants?
- Should runs start from scratch, or from a shared base like this
  repository's fixes?
- How to weigh cost against code quality in one ranking?
- Who reviews, and how to keep reviews blind?
