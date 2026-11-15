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

## What the project needs from a build server

Network boot and the image downloads need a build server on the same LAN
as the Pi. How it is set up is up to whoever runs it; the project only
relies on these services and paths. Setup steps for your own server are in
[netboot.md](netboot.md).

| Service | What for |
|---|---|
| TFTP (UDP 69), root `/srv/tftp` | Pi firmware network boot, U-Boot's `netboot.scr`, image fallback |
| Proxy DHCP | only for Pis that boot from the network with no SD card |
| HTTP on port 8080, same root | see below |
| Serial console reader | Pi tests read the Pi's serial log |

### What the HTTP server serves

| Path | Content | Read by |
|---|---|---|
| `/` | `harddrive.img.gz`, `bootloader-live.efi`, `netboot.scr` | U-Boot's `wget` (falls back to TFTP) |
| `/stable/` | the promoted stable network-boot image | the nightly |
| `/sdcard/` | SD card images, `.sha256`, manifests, `index.json` | SwiftFlash, browsers, Raspberry Pi Imager |
| `/visit/` | images for person tests, `index.html` | people |
| `/pkg/` | our package repository | `pkg` on the Pi |

Port 8080 and the paths must stay as they are: U-Boot on cards already in
use has them built in.

What the server has to do:
- plain HTTP, no redirect to HTTPS (U-Boot and `pkg` follow no redirects
  and cannot do TLS)
- a `Content-Length` and no chunked or compressed replies: U-Boot's lwIP
  `wget` needs the length and cannot decode chunks; `.gz` and `.pkgar`
  files go out byte for byte, without `Content-Encoding`
- range requests (resume a broken download), `ETag` or `Last-Modified`,
  IPv4 and IPv6, directory listings for people
- read-only access to the served directory

Discussion of the server: [#436](https://github.com/redox-rpi/redox-rpi/issues/436).

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

Some jobs (`qemu-boot.yml`, `upstream-weekly.yml`) can run on a
self-hosted runner with the label `homenode` (only that label, so not
`self-hosted`). Until the repository variable `HOMENODE_RUNNER` is `on`,
every job runs on GitHub's runners; set it to `off` to go back, for example
while the runner is down. Pull requests from forks run on GitHub's runners
only, after a maintainer approves them, and get the label `external`
(`label-external.yml`, which never checks out their code). Who may merge
what is in the [PMP, Merge rights](process/pmp.md#merge-rights).

What the project requires from such a runner (the admins implement it):
- **Forks never run on it.** For a pull request GitHub takes the workflow
  from the PR, so a fork can change `runs-on`; the runner itself must refuse
  any pull-request job whose head is not `redox-rpi/redox-rpi`, before the
  first step. Still read the workflow diff before approving a fork PR that
  touches `.github/`.
- **Isolated:** its own user and container, no access to the maintainer's
  files, secrets or signing key, no devices (no serial ports), no access to
  the local network or the host's own services, internet only. Limited CPU
  and RAM, so builds and Pi tests keep running.
- **Registered for this repository only**, one job at a time.

