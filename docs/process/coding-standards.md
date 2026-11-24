# Coding standards and rules (draft)

Owner: Architect. Scope and status are listed in the [PMP](pmp.md). This page
grows one section at a time; sections not written yet are listed at the end.

## Drivers: a status file in the scheme

Every driver offers a read-only status file in its own scheme, so a problem
can be looked at on a running system without a debug build.

- **Name:** `status`, for example `/scheme/disk.mmc/status` or
  `/scheme/usb.dwc2/status`. If the scheme's path space has no room for it,
  use `/scheme/<scheme>/.status` and say so in the driver's README.
- **Content:** plain text, one `name: value` per line, grouped under short
  headings. Include what helps find a fault:
  - the hardware state: important registers (raw hex, with the bit fields
    decoded where it is cheap), clocks and their sources, and the bus or
    link speed;
  - what the driver assumed or chose, next to what the hardware reports
    (for example the SDHCI base clock: the value from the capabilities
    register and the value the driver uses);
  - counters since start: requests, bytes, interrupts, errors by kind,
    timeouts, retries, resets;
  - the current state: queue depth, open handles, the last error with its
    time.
- **Cost:** reading the file may read registers, but must not change the
  hardware state or wait for the device. Counters are plain or atomic
  integers, updated on the normal path; no extra logging.
- **Stable enough for scripts:** new lines may be added, but existing names
  keep their meaning. Units go in the name (`_us`, `_bytes`, `_hz`).
- **Kernel:** the same idea in `/scheme/sys/<name>`, for example
  `/scheme/sys/irqstat` (#564) and `/scheme/sys/uart` (experiment
  `serial-rx2`).

New drivers have the file from the start. Existing drivers get it when
someone works on them for another reason, or when a fault needs it.

## Still to write

- Language and style per component (Rust, C, shell, Python)
- Error handling and `unsafe` code
- Logging: levels, what goes to the console at boot
- Patch format: small, close to upstream, one concern per patch
- Commit messages
- Documentation in code: issue numbers at workarounds (see the PMP)
