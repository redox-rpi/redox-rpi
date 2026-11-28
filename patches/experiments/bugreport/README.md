# Experiment: bugreport, report a bug from inside the image

Issue: [#759](https://github.com/redox-rpi/redox-rpi/issues/759). Adds
[`tools/bugreport`](../../../tools/bugreport): a tester at the Pi
(keyboard and screen only) writes one text file with the system state, to
attach to an issue. This experiment has no upload, so it is the variant for
release images. Dev images use
[`bugreport-upload`](../bugreport-upload/README.md), which also sends the
report to the build host and prints an ID.

```bash
scripts/build-image.sh --experiment bugreport
```

On the target:

```text
bugreport                 asks "What went wrong?" (one line), writes the report
bugreport <text...>       the same, with the text from the command line
bugreport --no-upload     only the file, also where an upload is set up
```

The report is `~/bugreport-<YYYYMMDD-HHMMSS>.txt` (UTC). The RAM root of
the direct card loses it at reboot, so copy it first, or use the upload in
dev images.

## What is in it

A fixed list of sources, nothing else:

- the tester's text;
- `/etc/redox-rpi-release`: version, build ID, experiments, commit;
- the board (the `model` of `/scheme/kernel.dtb`) and the kernel
  (`/scheme/sys/uname`);
- `uptime`, `free`, `ps`;
- the network: interfaces with MAC, link and addresses, routes and name
  server from `/scheme/netcfg`, the drivers in `/scheme/network.*`;
- a summary of the kernel log: panic, out-of-memory, page fault, crash and
  error lines, and the USB (dwc2d) lines;
- the kernel log `/scheme/sys/log`, last, losing its oldest lines if the
  report would pass 256 KiB.

## What is kept out

- No file from a home directory. `HOME` is only the place of the report.
- No environment variables, and no config files apart from the release
  file. There are no Wi-Fi or key files today; the fixed list keeps them
  out later.
- MAC addresses are cut to the vendor half (`b8:27:eb:xx:xx:xx`), since a
  report may be attached to a public issue.

Each file and command has a time limit of 10 s, so a hung driver cannot
stop the report. A missing source shows as "not available".

## Test

`scripts/regression-set.py --only bugreport` runs `bugreport --no-upload`
and checks the file: every section, the build ID, at most 256 KiB, no full
MAC, and not a canary line from a file in the home directory.
