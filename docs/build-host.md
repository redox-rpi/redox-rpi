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
