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
- Not tested on the Pi yet.
