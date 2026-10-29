# Experiment: prebuilt-programs

Adds bash, nano, vim, findutils, gawk, tmux, ripgrep, curl and wget from
Redox' aarch64 package repository (binary packages) to the image, to find
out why they fail at start.

```bash
scripts/build-image.sh --experiment prebuilt-programs
```

## Results

Issue [#40](https://github.com/redox-rpi/redox-rpi/issues/40). All nine
programs are dynamically linked (`/lib/ld.so.1`), so relibc patches reach
them through the image's `libc.so`.

- QEMU, build `v0.3.0-dev-131-g622d6b8+xbeb4` (main with this experiment):
  each program started 20 times (`bash -c true`, `nano --version`,
  `vim --version`, `find`, `gawk`, `tmux -V`, `rg --version`,
  `curl --version`, `wget --version`): 180 of 180 exit with 0. The old `brk`
  exceptions and the `libonig.so.5` link error do not come back.
- QEMU, same image: `bash -c 'for j in $(seq 1 200); do true; /usr/bin/true;
  x=$(echo a); done'`, 8 runs (3200 child processes, each exit a signal to
  bash) before the time limit: no crash, all exit 0.
- `sigregs --x18` (build `v0.3.0-dev-132-g315c25a+x2ea1`, with
  `sigregs-test`) fails: the signal return overwrites x18, and these
  gcc-built programs use x18. The fix is in [#1](https://github.com/redox-rpi/redox-rpi/issues/1)'s experiment
  `sigreturn-svc`, to be rechecked with these programs.
- QEMU, build `v0.3.0-dev-143-gd6d2b70+xab7f` (this branch merged with
  `t1-sigreturn`; experiments `prebuilt-programs`, `sigreturn-svc`,
  `sigregs-test`, `serial-rx`): the same 180 starts all exit 0; 4 runs of the
  bash loop (1600 child processes) before the time limit, all exit 0;
  `sigregs process -n 2000` (x18 checked) passes. `sigregs thread` still
  fails with the signal mask bug [#4](https://github.com/redox-rpi/redox-rpi/issues/4), whose fix is not in this build.
  Without `serial-rx` the long test command line made getty panic
  (`failed to write master PTY: WouldBlock`, #28).
- Real Pi, test plan #45, `v0.3.0-dev-144-g2e6f4f9+x9c38` (branch
  `prebuilt-sigreturn`, with `sigreturn-svc`): standard checks pass, 0 of
  300 starts failed; 9 programs × 50 starts, 450 of 450 exit 0; bash loop
  5 of 5; `sigregs process` passes, `sigregs thread` fails as expected
  (#4 not in the image). Report on main 944f0f2.
