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
- Not tested on the Pi yet.

## Which repository packages run unchanged (2026-10-31)

Issue [#40](https://github.com/redox-rpi/redox-rpi/issues/40), branch
`t40-prebuilt`. Instead of building programs into the image, install them
with `pkg` from Redox' aarch64 package repository and check which ones
run unchanged on our image. Result: **[allowlist.txt](allowlist.txt)**,
31 packages that ran in QEMU, with the space each one needs.

Why they can run at all: every program is linked against `/lib/ld.so.1`
and the image's `libc.so`, so it uses our relibc, not upstream's. The
repository's relibc is ad0c479, ours is c303135 (46 commits apart). No
package depends on the `relibc` or `base` packages, so `pkg install` does
not replace them.

**Static check** ([abi-check.py](abi-check.py)): for each of the 82
non-graphical packages (the other 31 are Orbital and COSMIC programs,
fonts, icons, LLVM and Rust), every library a program needs and every
symbol it imports, compared with the image's `/usr/lib` and the package's
own dependencies. 71 packages resolve completely, 9 hold no programs
(data, scripts). The only real gap: `libmpfr` needs 12 decimal-float
helpers (`__bid_*`) our `libgcc_s` does not have; nothing we tested uses
it. Upstream's newer relibc adds `dl_iterate_phdr`; no package imports it.

**Runtime check in QEMU**, build `v0.3.0-dev-772-g8dc59fc+x0e02` (main,
experiments `timesync` and `sigreturn-svc`), `pkg install --yes` over the
network, then [pkg-check.sh](pkg-check.sh) (one `T40 PASS|FAIL` line per
program):

- 28 small programs in one install (bash, dash, nano, kibi, smith, rg,
  gawk, sed, find, diff, patch, file, sqlite3, curl, wget, tmux, xz, zstd,
  bzip2, lz4, make, rsync, btm, pls, miniserve, nasm, patchelf, rldd):
  26 pass. `xz` fails because the package has no `xz` program (only
  `liblzma` and the `xzgrep`-style scripts). `rsync -a` of one local file
  spins at 100 % CPU until killed (not looked into further).
- git and vim (with bash and curl), python312 (with bash and curl) and
  rustpython: each on a fresh image, all pass (`git init` and commit,
  `vim --version`, `python3.12` with `json` and `re`, `rustpython -c`).

**Space is the limit.** The network-boot image is 256 MiB with about
130 MB free, and pkg keeps its downloads in `/tmp` on the same disk. All
32 packages at once need 237 MB: pkg stops with "No space left on device"
and rolls back cleanly ("Transaction aborted", nothing left half
installed). After the 28 small ones 62 MB is left, too little for vim,
git or python. python312 alone with bash and curl takes 122 MB and leaves
12 MB. A larger image or the SD data partition (`pkg-persist`, PR #106)
removes the limit.

pkg is also slow under QEMU: the 28 small packages took about 25 minutes,
almost all of it pkg's own CPU time unpacking and checking.

What else to keep in mind:

- `pkg install` replaces our older `diffutils` and `kibi` with upstream's;
  both still pass. A `pkg upgrade` would also replace system packages
  (kernel, relibc, redoxfs, pkgutils): the hold list in experiment
  `pkg-hold` (PR #66) is what stops that.
- `sqlite3 :memory:` prints two lines `Unknown special reference: memory:`
  from init's namespace manager; the query still works.
- Pi test #163 repeats this on the Pi over the network.

To repeat the static check on homenodellm: download the packages (`.pkgar`
and `.toml` from the repository URL above) into `pkgs/`, extract each with
`build/tools/bin/pkgar -p pub.toml -a pkgs/<name>.pkgar extract x/<name>`
(`pub.toml` is the repository's public key,
`https://static.redox-os.org/pkg/id_ed25519.pub.toml`), copy the image's
`/usr/lib` to `img/lib`, then `abi-check.py <dir>` (needs
`aarch64-linux-gnu-readelf`).
