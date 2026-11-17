# Experiment serial-rx2: fewer lost characters on fast serial input (#28, #481)

**Goal:** REQ-CON-2 criterion 2 says that `serial-input-test.sh 20` delivers 20 of 20 lines. On the v0.4.0 builds it delivered 19 of 20 (rows 285 and 292). The v0.5.0 build delivered 20 of 20 (row 414). The losses are rare but real.

## Cause

The console is the PL011 on GPIO 14/15. Kernel patch `0007` (experiment [serial-rx](../serial-rx/README.md)) covers the long gap while the kernel writes to the port: bytes go into a stash. That leaves every other stretch in which the receive interrupt cannot be taken in time:
- interrupts masked elsewhere in the kernel;
- core 0, which gets the interrupt on the BCM2837, busy in another interrupt or a long system call.

With the receive interrupt at 1/2 full, the 16-byte FIFO leaves 8 bytes of room after the interrupt is raised. At 115200 baud that is 0.7 ms. A line of `serial-input-test.sh` is 206 bytes, or 18 ms. The shell redraws the line after every key, so the console's output is many times its input: getty used 72 s of CPU during one 20-line run (row 285), nearly all of it in the serial write. A single gap of more than 0.7 ms in those 20 × 18 ms loses a byte.

## Change (kernel `0001`)

- **Receive interrupt at 1/8 full** (2 bytes) instead of 1/2: 14 bytes, 1.2 ms, of room. The receive timeout interrupt still takes the last bytes of a burst. More interrupts per burst (one per 2+ bytes instead of 8+) cost little at 11.5 kB/s.
- **Counters in `/scheme/sys/uart`:**
  - `rx_bytes`: bytes read from the FIFO;
  - `rx_overruns`: the OE bit in the data register, meaning bytes were lost because the FIFO was full;
  - `rx_stashed`, `rx_stash_max`, `rx_stash_full`: what patch `0007`'s stash took.

  A test can then tell where a lost character went. An overrun means the interrupt came too late. A loss without an overrun means it went later, in getty, ptyd or the shell.

## Test

1. **QEMU `raspi3b`:** login, long lines, `cat /scheme/sys/uart`. QEMU cannot lose characters here: its PL011 takes host input only while the FIFO has room. So QEMU shows only that the counters work and that nothing broke.
2. **Pi 3B+, no person:**
   - `serial-input-test.sh` (in [serial-rx](../serial-rx/serial-input-test.sh)) with 20 lines, then with 100;
   - `cat /scheme/sys/uart` before and after each run;
   - on the stable image as well, for comparison, which has no counters (only the line count).

## Results

- **Test 490 (Pi 3B+, 2026-11-17): no characters lost.** `rx_overruns` 0 in all 5 readings; `rx_bytes` matches the bytes sent within 1 byte; the stash peaked at 194 bytes and never filled. The script marked 1 of 20 and 3 and 4 of 100 lines "LOST", but each of them is complete in the console log. They came late, not short: the shell redraws the whole line for each character it reads, about 31 KB of console output per 209-byte command, and the script waited only 30 s. The stable image showed the same (98 of 100).
- Since then `serial-input-test.sh` waits 120 s per line and checks unanswered lines again over the whole log at the end. It reports complete, late and LOST, and fails only on LOST.
- **Test 491 (Pi 3B+, 2026-11-17): pass** with the new script. serial-rx2: 20 of 20 and 100 of 100, 0 late, 0 LOST, `rx_overruns` 0, received bytes equal to sent bytes, pi-test 6/6. The stable image, without this patch, also 100 of 100 with 0 LOST. The lost lines in #28 were the old test's 30 s wait, not the UART. A line takes 4 to 28 s (median 5 to 6 s), because the shell redraws the line for each character.
- So the receive path was not losing bytes, and this experiment is not needed as a fix. It stays for its counters (`/scheme/sys/uart`), which show at once whether a future loss happens in the UART or above it, and for the larger headroom (1.2 ms instead of 0.7 ms).
