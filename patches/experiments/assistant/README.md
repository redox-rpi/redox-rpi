# Experiment: assistant, llama.cpp and a small model on Redox (#43)

Puts llama.cpp's command-line programs and the Qwen3 0.6B model (Q4_0,
382 MB) into the SD-card image, to test the speed and memory of a local
language model on the Pi. See
[docs/topics/assistant.md](../../../docs/topics/assistant.md).

```bash
scripts/build-image.sh --sdcard --experiment assistant
```

Only for the SD-card image: the network-boot image is loaded into RAM and
holds at most 256 MiB.

- [`recipes/llama-cpp`](recipes/llama-cpp): llama.cpp `7df8b87`
  (2026-10-29) with [`redox.patch`](recipes/llama-cpp/redox.patch), built
  static against our relibc. Installs `llama-simple`, `llama-completion`
  and `llama-bench` in `/usr/bin`.
- [`recipes/qwen3-model`](recipes/qwen3-model): downloads the model at
  build time, checks its SHA-256 and installs it as
  `/usr/share/assistant/models/Qwen3-0.6B-Q4_0.gguf`. The file is not in
  git.

`scripts/build-image.sh` copies an experiment's `recipes/` into
`recipes/other` of the Redox tree.

On the Pi:

```bash
llama-simple -m /usr/share/assistant/models/Qwen3-0.6B-Q4_0.gguf -n 32 "The capital of France is"
llama-bench -m /usr/share/assistant/models/Qwen3-0.6B-Q4_0.gguf -p 64 -n 16 -t 4
```

[`redox-aarch64.cmake`](redox-aarch64.cmake) is a CMake toolchain file for
building llama.cpp by hand outside the cookbook (static; the paths point
at the Redox prefix of a build clone on homenodellm):

```bash
cmake -S . -B build-redox -DCMAKE_TOOLCHAIN_FILE=redox-aarch64.cmake \
    -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
    -DGGML_NATIVE=OFF -DGGML_CPU_ARM_ARCH=armv8-a -DGGML_OPENMP=OFF \
    -DLLAMA_CURL=OFF -DLLAMA_OPENSSL=OFF -DLLAMA_SUBPROCESS=OFF \
    -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_SERVER=OFF
cmake --build build-redox -j8 --target llama-simple llama-completion llama-bench
```
