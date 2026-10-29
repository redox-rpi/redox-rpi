# Experiment: assistant, llama.cpp on Redox (#43)

Not an image experiment yet: no recipe, no `config.toml`. It holds the
changes llama.cpp needs to build for `aarch64-unknown-redox`, see
[docs/topics/assistant.md](../../../docs/topics/assistant.md).

[`llama-cpp-redox.patch`](llama-cpp-redox.patch) applies to llama.cpp
`7df8b87` (2026-10-29). Built on homenodellm with the Redox gcc:

```bash
cmake -S . -B build-redox -DCMAKE_TOOLCHAIN_FILE=redox-aarch64.cmake \
    -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
    -DGGML_NATIVE=OFF -DGGML_CPU_ARM_ARCH=armv8-a -DGGML_OPENMP=OFF \
    -DLLAMA_CURL=OFF -DLLAMA_OPENSSL=OFF -DLLAMA_SUBPROCESS=OFF \
    -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_SERVER=OFF
cmake --build build-redox -j8 --target llama-simple llama-completion llama-bench
```

[`redox-aarch64.cmake`](redox-aarch64.cmake) is the toolchain file
(static linking; the paths point at the Redox prefix of a build clone).
