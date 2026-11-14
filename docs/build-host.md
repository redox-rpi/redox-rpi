# Build host

## What works where

| Task | macOS (Apple Silicon) | x86_64 Linux |
|---|---|---|
| Build the kernel (`aarch64`) | yes, see below | yes |
| Replace files in an image (`tools/redoxfs-file`) | yes | yes |
| Run QEMU `raspi3b` / `virt` | yes | yes |
| Full image build (`make all`) | only via Podman (Linux VM), slow | **recommended** |
| Flash SD cards, serial console | yes | yes |

Redox is developed and tested on x86_64 Linux, and its prebuilt cross
toolchains target that host. Use it for full image builds.

## x86_64 Linux (Debian, Ubuntu, Fedora, Arch, …)

Follow the official script; it installs the packages and Rust toolchains:

```sh
curl -sf https://gitlab.redox-os.org/redox-os/redox/raw/master/native_bootstrap.sh -o native_bootstrap.sh
bash -e native_bootstrap.sh
```

Then build for the Pi 3:

```sh
cd redox
make all ARCH=aarch64 BOARD=raspi3bp CONFIG_NAME=minimal
make qemu ARCH=aarch64 BOARD=raspi3bp CONFIG_NAME=minimal
```

Notes:

- Plan for 50+ GB of free disk space and a few hours for the first build.
- An x86_64 VM on x86 hardware or a cloud VM works as well.

## Kernel only, on macOS

```sh
rustup toolchain install nightly-2026-05-24 --profile minimal --component rust-src,llvm-tools
cd upstream/kernel   # clone of gitlab.redox-os.org/redox-os/kernel
mkdir -p build/aarch64
RUST_TARGET_PATH=$PWD/targets cargo rustc --bin kernel \
    --target targets/aarch64-unknown-kernel.json --release \
    -Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem -Z json-target-spec \
    -- -C link-arg=-T -Clink-arg=$PWD/linkers/aarch64.ld \
       -C link-arg=-z -Clink-arg=max-page-size=0x1000 \
       --emit link=$PWD/build/aarch64/kernel.all
~/.rustup/toolchains/nightly-2026-05-24-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy \
    --strip-debug build/aarch64/kernel.all build/aarch64/kernel
```

The toolchain version comes from the kernel's `rust-toolchain.toml`. Then
boot it with `scripts/qemu-raspi3b.sh --kernel upstream/kernel/build/aarch64/kernel`.

## CI image mirror

GitHub's runners get 403 from static.redox-os.org and gitlab.redox-os.org
(#409). So the QEMU boot test in CI boots Redox's daily aarch64 server image
from our pre-release [`ci-image`](https://github.com/redox-rpi/redox-rpi/releases/tag/ci-image),
and builds U-Boot itself.

- `scripts/refresh-ci-image.py` copies the newest daily there once a week.
  It checks the copy against Redox's `SHA256SUM`, then uploads it with a
  `SHA256SUM` of its own and a `latest.txt` naming it. It keeps the last two
  copies and writes the date, source and SHA-256 into the release notes.
- `scripts/update-stable.sh` runs it on Mondays on the build host. Without
  a token it only logs that; a failure never fails the night. By hand:
  `scripts/refresh-ci-image.py [--dry-run]`.
- The workflows read `latest.txt`. While it is missing they use the copy of
  2026-11-11.
- Token: a fine-grained token for `redox-rpi/redox-rpi` only, permission
  *Contents: read and write*, in `~/.config/redox-rpi/ci-image-token`
  (mode 600) on the build host. It is separate from the nightly's token.
  Creating it is the maintainer's step.

## CI runner

Our own GitHub Actions runner on the build host (label `homenode`) runs
`qemu-boot.yml` and `upstream-weekly.yml`. It runs them only for pushes to
`main`, the schedule, and pull requests from branches of this repository.
Pull requests from forks always run on GitHub's runners, and only after a
maintainer approves them. They get the label `external` (`label-external.yml`,
which never checks out or runs their code). Who may merge what is in the
[PMP, Merge rights](process/pmp.md#merge-rights).

Until the repository variable `HOMENODE_RUNNER` is `on`, every job stays on
GitHub's runners. So the workflows can be merged before the runner exists.

Isolation (files in `scripts/ci-runner/`):
- Own Linux user `ci-runner`, not `zehrer`. It has no sudo, is not in
  `dialout` (no `/dev/ttyUSB*`), and cannot read `~zehrer` (no
  `~/.config/redox-rpi`, signing key or tokens).
- A rootless podman container (`Containerfile`). The only volume is the
  runner's own state. There are no host directories, no devices, no
  capabilities and no host loopback (`pasta --no-map-gw`). It is limited to
  6 CPUs and 12 GB RAM, so the nightly and the Pi tests keep the rest.
- One runner, one job at a time.
- nftables (`ci-runner.nft`): the user `ci-runner` reaches the internet and
  DNS only. It cannot reach the home network (Shelly plug, NanoKVM, Pi,
  router web UI) or services on the host.

### Setting it up (maintainer)

On homenodellm, as root:

1. `apt install podman uidmap passt nftables`
2. `useradd -m -s /bin/bash ci-runner && loginctl enable-linger ci-runner`
3. Check: `sudo -u ci-runner ls ~zehrer` must fail. `/home/zehrer` is mode 750
   today; keep it that way.
4. Copy `scripts/ci-runner/ci-runner.nft` to `/etc/nftables.d/`. Add
   `include "/etc/nftables.d/*.nft"` to `/etc/nftables.conf`. Then run
   `systemctl enable --now nftables`.
5. Check:
   - `sudo -u ci-runner curl -sI https://github.com` works.
   - `sudo -u ci-runner curl -m 5 http://shellyplug-01.fritz.box` fails.
   - `sudo -u ci-runner curl -m 5 http://127.0.0.1:8080` fails.

As `ci-runner` (`sudo -iu ci-runner`):

6. Clone the repository and build the container:
   `git clone https://github.com/redox-rpi/redox-rpi && podman build -t redox-rpi-runner redox-rpi/scripts/ci-runner`
7. On GitHub, go to redox-rpi/redox-rpi → Settings → Actions → Runners →
   *New self-hosted runner*. Copy only the token from the `--token` line.
   It is valid for one hour. Then:
   `install -m 600 /dev/null ~/reg-token && nano ~/reg-token` (paste it).
8. `mkdir -p ~/.config/systemd/user && cp redox-rpi/scripts/ci-runner/ci-runner.service ~/.config/systemd/user/`,
   then `systemctl --user enable --now ci-runner`. The runner registers as
   `homenode` and shows up as *Idle* on the Runners page.
9. `rm ~/reg-token`. The registration is in the volume `ci-runner-state`.

On GitHub:

10. Org redox-rpi → Settings → Actions → General → *Fork pull request
    workflows from outside collaborators*: **Require approval for all
    outside collaborators**.
11. In the same place, under *Runners*, the repository runner is enough. Do
    not add the runner to the org or give it to other repositories.
12. redox-rpi/redox-rpi → Settings → Secrets and variables → Actions →
    Variables: `HOMENODE_RUNNER` = `on`. Turn it back off (delete it or set
    `off`) to send everything to GitHub's runners again, for example while
    the build host is down.

Updating the runner: GitHub's runner updates itself. For a new base image,
run `podman build` again (step 6) and `systemctl --user restart ci-runner`.

Not covered: what a pull request from our own branches runs, it runs with
internet access and the runner's registration. That is acceptable only
because branches here are pushed by us. Fork PRs never reach the runner.
