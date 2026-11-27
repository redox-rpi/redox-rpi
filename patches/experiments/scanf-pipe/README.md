# Experiment: scanf-pipe, scanf keeps its look-ahead character on a pipe

For [#338](https://github.com/redox-rpi/redox-rpi/issues/338) (topic
relibc). Found with qbe, which read cproc's piped `data` as `dta`
([#327](https://github.com/redox-rpi/redox-rpi/issues/327)).

**The bug:** relibc's scanf reads a `FILE` through `FileReader`
(`src/header/stdio/reader.rs`), which returns each character and then
`fseek`s back to it, so the character scanf stops at stays in the stream.
On a pipe `fseek` drops the `FILE`'s read buffer and fails: the look-ahead
character and all input already buffered from the pipe are lost, and every
later read returns EOF. On a regular file it works. Upstream relibc master
(3e4160f6, 2026-11-10) has the same code; upstream has no issue or merge
request about it.

**The fix** (`relibc/0001`): no seeking. Each character's bytes go back
into the `FILE`'s `unget` buffer (as with `ungetc`) and are taken out when
the next character is read, so the stream ends where it did before on a
file, and now also on a pipe. `wscanf` uses the same reader. The patch adds
relibc test `stdio/fscanf_pipe` (expected output from glibc).

**A second, older bug that the fix exposed** (ld.so, [#375](https://github.com/redox-rpi/redox-rpi/issues/375)): with
`0001` alone, timesync and devshd panicked at boot (`RELIBC PANIC …
pthread/tls.rs:57: RefCell already borrowed`). `0001` shrinks libc.so's
TLS segment, and the new layout shows a misplacement in ld.so's static
TLSDESC on aarch64 (16 bytes, the TCB). The fix was this experiment's
patch `0002`; it is now the default relibc patch
[`0025`](../../relibc/0025-ld.so-aarch64-static-TLSDESC-points-at-the-copied-TL.patch),
because any relibc change that moves libc's TLS hits the same bug (#744).

**Check:** build with `--experiment tcc --experiment scanf-pipe` (tcc
compiles the test program on the Pi), then

    ion /usr/share/scanf-pipe/check.ion

runs `scanf-pipe.c` on its own `pipe()`, on a file and on a shell pipe;
each must print `expected.txt` (glibc's output).

(Over devsh the file line shows as `/ pass`: ion or devsh treats
`file:` as a scheme prefix. Read it as `file: pass`.)

## Results

- **Real Pi 3B+, 2026-11-11**, pi-tests row 336
  ([report](https://github.com/redox-rpi/pi-tests/blob/main/reports/336.md)),
  same image `v0.5.0-dev-51-gc1eeb499+x98c1`: no panic in the boot log;
  own-pipe, file and shell-pipe pass (2.0 s); `tcc -run` hello rc 0;
  pi-test 6/6, 0 of 300 starts failed. **Pass.**
- **QEMU raspi3b, 2026-11-11**, `v0.5.0-dev-51-gc1eeb499+x98c1`
  (`--experiment timesync --experiment tcc --experiment scanf-pipe`):
  own-pipe, file and shell-pipe pass; no `RELIBC PANIC` in the boot log.
  With `0001` alone (`…-50-g7dcbe598+x98c1`) the boot showed three
  `tls.rs:57` panics and devsh could not run the check; without the
  experiment (A/B, two boots each) none.
