# Work breakdown structure (WBS)

All the work of the project, as work packages with an owner. The
[PBS](pbs.md) says what we build; the WBS says who does which work.
Status: draft, 2026-11-08; owned by the project lead.

How it is used:

- Each work package below gets one parent issue (label `wp`, issue type
  Task). The owner splits it into smaller issues and attaches them as
  sub-issues, so the issue tree is the detailed WBS.
- Releases are milestones; a work package delivers into several of them.
- A new work package or a change of owner is a pull request to this page.
- Packages owned by "project lead" have no owner yet and are open to a
  contributor who joins ([join page](../join.md)).

## 1 Management and process

| WP | Work | Owner | PBS | Delivers |
|---|---|---|---|---|
| 1.1 | Project management: PMP, WBS, work packages, status, risks and opportunities, reporting | project lead | — | [pmp.md](pmp.md), `STATUS.md`, `risk`/`opportunity` issues |
| 1.2 | Quality management: QMP, process audits, bug analysis, KPIs and dashboard, code quality baseline | Quality | — | `qmp.md`, KPI dashboard |
| 1.3 | Requirements: requirements management plan, IDs, acceptance criteria, trace to issues and tests | Requirements | all | `rmp.md`, `docs/requirements/` |
| 1.4 | Architecture, coding standards and rules, RFC drafts | Architect | all | [architecture.md](../architecture.md), `coding-standards.md`, RFCs |
| 1.5 | Project structure (RFC 0002): split repositories, issues per repository | Architect, project lead | — | [RFC 0002](../rfcs/), migration PRs |
| 1.6 | Community: org profile, join page, Discussions, contributor onboarding | project lead, Architect | 5.4 | profile, [join.md](../join.md) |

## 2 Platform development

| WP | Work | Owner | PBS | Topic page |
|---|---|---|---|---|
| 2.1 | Boot chain and console: firmware config, U-Boot, boot menu, bootloader, boot time, serial console, init and login | Boot and console | 2.1.1, 2.1.3–2.1.5, 2.3.1, 2.4.6, 2.5.1 | [boot](../topics/boot.md) |
| 2.2 | Kernel stability: start crashes, signals, memory, SMP, chaos test | Chaos test | 2.2.1 | [stability](../topics/stability.md), [chaos](../topics/chaos.md) |
| 2.3 | relibc on aarch64 | Release and CI | 2.4.1 | [relibc](../topics/relibc.md) |
| 2.4 | Storage: SD card driver, RedoxFS | Boot and console | 2.3.3, 2.4.3 | [sdcard](../topics/sdcard.md) |
| 2.5 | USB: host driver, hubs, input, hotplug, mass storage | Boot and console | 2.3.2 | [usb](../topics/usb.md) |
| 2.6 | Firmware interface and CPU clock | Boot and console | 2.3.8 | [firmware](../topics/firmware.md) |
| 2.7 | Network and time: Ethernet drivers, netstack, Wi-Fi, IPv6, NTP, time zones, SSH | Network | 2.3.4, 2.3.5, 2.4.2, 2.4.4, 2.5.6 | [network](../topics/network.md), [time](../topics/time.md) |
| 2.8 | Peripherals and smart home: GPIO, I2C, SPI, RTC | Network | 2.3.9 | [gpio](../topics/gpio.md) |
| 2.9 | Graphics: framebuffer, display driver vc4d, desktop | Graphics | 2.3.6, 2.3.7, 2.5.4 | [graphics](../topics/graphics.md) |
| 2.10 | Audio: PWM driver, audio server | Audio | 2.3.10, 2.4.5 | [audio](../topics/audio.md) |
| 2.11 | Developer tools on the Pi: TinyCC | TCC | 2.5.3 | [tcc](../topics/tcc.md) |
| 2.12 | Assistant (local language model) | Log triage | 2.5.5 | [assistant](../topics/assistant.md) |
| 2.13 | Open-source VPU firmware | Open firmware | 2.1.2 | [open-firmware](../topics/open-firmware.md) |
| 2.14 | Timers and DMA | project lead (no owner yet) | 2.3.11 | [dma](../topics/dma.md), [time](../topics/time.md) |

## 3 Boards

| WP | Work | Owner | PBS | Topic page |
|---|---|---|---|---|
| 3.1 | Pi Zero 2 W and the other BCM2837 boards (3B, 3A+) | V&V | 1.2, 1.3 | [zero2w](../topics/zero2w.md) |
| 3.2 | Pi 4 family (BCM2711) | project lead (no owner yet) | 1.4 | [boards](../topics/boards.md) |
| 3.3 | Pi 5 family (BCM2712, RP1) | project lead (no owner yet) | 1.5 | [boards](../topics/boards.md) |

## 4 Verification and validation

| WP | Work | Owner | PBS | Delivers |
|---|---|---|---|---|
| 4.1 | Test standards, test plan, regression set | V&V | — | `test-standards.md`, [test-plan.md](../test-plan.md) |
| 4.2 | Automated Pi tests and the Pi queue, special tests, person tests prepared | V&V | 3.3 | Pi rows, [test-reports.md](../test-reports.md) |
| 4.3 | Test lab: more boards, no card swaps | V&V | 3.3 | [boards](../topics/boards.md) |
| 4.4 | Log triage of test and nightly logs | Log triage | 3.2, 3.3 | triage notes in issues |

## 5 Build, CI and release

| WP | Work | Owner | PBS | Delivers |
|---|---|---|---|---|
| 5.1 | Reproducible build, upstream pin, drift check, rebases | Release and CI | 3.1 | [versioning.md](../versioning.md) |
| 5.2 | CI, nightly, image server, network boot server | Release and CI | 3.2, 3.4 | [topics/ci.md](../topics/ci.md) |
| 5.3 | Release candidates and releases | Release and CI | 4.1, 4.2 | [release-process.md](../release-process.md) |
| 5.4 | Package repository and prebuilt programs | Release and CI | 2.4.7 | package repo |

## 6 Documentation

| WP | Work | Owner | PBS | Delivers |
|---|---|---|---|---|
| 6.1 | User guide and getting started | project lead, with each topic owner | 5.1 | [user-guide.md](../user-guide.md) |
| 6.2 | Topic pages | each topic owner | 5.2 | [topics/](../topics/README.md) |
