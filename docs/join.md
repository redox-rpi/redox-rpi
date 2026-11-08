# Join the project

Redox OS on the Raspberry Pi is an open project in the GitHub organisation
[redox-rpi](https://github.com/redox-rpi). The goal: Redox runs on (almost)
every Raspberry Pi from the Zero 2 W upwards. Today one maintainer and a
team of AI agent sessions do the work, and we would like more people, and
more agents, to join.

You don't need to be a kernel developer. Testing an image on your own Pi
and writing down what happened helps already.

## Who we are looking for

| You have or like | You could |
|---|---|
| A Pi 3B+, 3B, 3A+ or Zero 2 W | flash a release or a release candidate, run the checks in the [user guide](user-guide.md), report what works and what doesn't |
| A Pi 4, 400, CM4 or Pi 5 | help start the next board family ([boards topic](topics/boards.md)) |
| A USB serial adapter | send boot logs; they are the most useful bug report there is |
| Rust | drivers in user space (USB, SD card, network, display, audio), the kernel, relibc |
| C, U-Boot, firmware | the boot chain and the [open firmware](topics/open-firmware.md) experiment |
| Your own AI coding agent | point it at an issue labelled [`needs:nothing`](https://github.com/redox-rpi/redox-rpi/issues?q=is%3Aopen+label%3Aneeds%3Anothing) or [`needs:qemu`](https://github.com/redox-rpi/redox-rpi/issues?q=is%3Aopen+label%3Aneeds%3Aqemu); the issues are written so that one agent session can finish them |
| Writing, testing, organising | documentation, test plans, requirements, reviewing pull requests |

## How to start

1. Read the [org page](https://github.com/redox-rpi) and the
   [topics](topics/README.md) to see where we are.
2. Pick an issue, or open one for what you want to do, and say in a
   comment that you take it.
3. Work as described in [CONTRIBUTING.md](../CONTRIBUTING.md): a branch, a
   pull request that names the issue, test results with the build ID.
4. Questions and ideas: [Discussions](https://github.com/redox-rpi/redox-rpi/discussions).

## Growing into a role

The project is organised in topics and roles
([project management plan](process/pmp.md)). Each topic has an owner who
plans its work, fixes its bugs and keeps its topic page current. Today the
owners are agent sessions; a person or another agent who knows a topic well
can take over or share a topic. Topics without an owner (for example the
Pi 4 and Pi 5) are open now. Say so in an issue or a discussion.

## Rules for everyone

The same rules apply to people and agents: comment on your issue as you go,
in short and plain English; one topic per pull request; every change that
reaches `main` was tested on a real Pi; only maintainers merge.

This is an unofficial fork, not part of the Redox OS project. Redox does
not accept AI-generated contributions, so nothing from here goes upstream;
please report problems here, not to Redox. Everything is MIT-licensed like
Redox itself.
