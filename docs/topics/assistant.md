# Assistant

An effort analysis, written 2026-10-29 before any work was on `main`
([#43](https://github.com/redox-rpi/redox-rpi/issues/43)).
No release target, a study like [graphics](graphics.md). Sizes are rough
guesses for this project's way of working (agent sessions, one Pi on the
test network, a person for the hands-on checks).

The idea: a small language model that runs on the Pi itself, with no
network and no GPU, and helps people who know macOS, Windows or a Linux
shell but not Redox. It answers simple questions ("how do I list files?",
"where is my home folder?") and runs a few local tools for the user: the
clock, disk and memory, files in the home folder, later Wi-Fi on and off.

## State

Nothing on `main`. Work is on the branch `assistant` (see In progress).

## The limits of a Pi 3B+

| What | Value | What it means |
|---|---|---|
| RAM | 1 GiB, about 990 MiB for Redox with `gpu_mem=16` | the model, its working memory and Redox must fit; **Redox has no swap** |
| CPU | 4x Cortex-A53, 1.4 GHz, ARMv8.0 | NEON, but no `dotprod`, `i8mm` or FP16 arithmetic, so llama.cpp uses its plain NEON paths |
| Memory bandwidth | about 2 GB/s in practice (32-bit LPDDR2) | the limit for writing answers: every new word reads all the weights once |
| GPU | VideoCore IV | not usable (see [graphics](graphics.md), step 3) |
| Heat | throttles from 60 °C on the 3B+ | four busy cores for minutes; a heat sink helps |

The Zero 2 W has half the RAM (512 MiB): only the smallest models below
would fit there.

## Models

| Model | File (GGUF) | Licence | Fits 1 GiB | Notes |
|---|---|---|---|---|
| **Qwen3 0.6B**, Q4_0 | 382 MB | Apache 2.0 | yes | **first choice.** Instruction-tuned, knows tool calls (its chat template has `<tool_call>` blocks), "thinking" can be turned off with `/no_think`. Q4_0 has the fastest NEON code in llama.cpp on cores without `dotprod` |
| Qwen3 0.6B, Q4_K_M | 397 MB | Apache 2.0 | yes | a bit better quality, a bit slower on the A53 |
| Qwen3 0.6B, Q8_0 | 639 MB | Apache 2.0 | tight | best quality of the three; leaves little for context and Redox |
| FunctionGemma 270M | about 280 MB (8-bit) | Gemma terms | yes, also the Zero | made by Google for function calls only; weak at answering questions, meant to be fine-tuned on one's own tools |
| SmolLM2 360M Instruct | about 230 MB (Q4) | Apache 2.0 | yes, also the Zero | small and fast, noticeably weaker answers |
| **BitNet b1.58 2B4T** (Microsoft) | 1.19 GB (`i2_s`) | MIT | **no** | 1.58-bit weights (0.4 GB), but the word table (embedding) is stored at 16 bits; the file alone is larger than the Pi's RAM |

**BitNet:** the idea fits the Pi well (ternary weights, little memory
traffic), but the only official model does not fit in 1 GiB, and
`bitnet.cpp` is a fork of llama.cpp whose fastest ARM kernels target newer
cores. Porting it is the same work as llama.cpp plus its own kernels.
Mainline llama.cpp also reads ternary models (`TQ1_0`, `TQ2_0`), so a
smaller 1.58-bit model, if one appears, can be tried without a second
port. Not now.

### Memory for Qwen3 0.6B Q4_0

| Part | Size |
|---|---|
| Weights | 382 MB |
| Context (KV cache), 1024 tokens at 16 bits | 112 MiB (28 layers x 2 x 1024 values x 2 bytes per token); 56 MiB with an 8-bit cache |
| Working buffers | about 50 to 100 MiB, less with small batches |
| Redox with the console and network | to be measured (QEMU `free` in In progress) |
| **Total** | about 600 to 650 MiB, leaves room for the user's programs |

Without swap and with no page cache that can drop pages, the model must
stay fully in RAM. Whether Redox's `mmap` of a 382 MB file shares the
pages or copies them is one of the things the QEMU run answers.

### Speed (estimates, not measured)

- **Writing an answer:** each token reads all 382 MB of weights. At about
  2 GB/s that gives at most 5 tokens per second; with the A53's compute on
  top, **2 to 4 tokens per second** is likely. A 50-word answer takes
  15 to 30 seconds. Shown word by word as it is written, that is usable.
- **Reading the question (prompt):** compute-bound, maybe 10 to 20 tokens
  per second on four cores. A system prompt with tool descriptions and a
  help text of 300 to 500 tokens would take **half a minute or more** on
  every start. So the prompt state is computed once and saved to a file
  (llama.cpp `llama_state_save_file`), and loaded at start in a second.
- For comparison, published numbers for 0.5B models on a Pi 5 are 8 to 19
  tokens per second; the Pi 3 has about a third of its memory bandwidth.

## Porting llama.cpp to Redox aarch64

llama.cpp is C and C++17 with few dependencies. What it needs from Redox:

| Need | State on Redox |
|---|---|
| C++ compiler and `libstdc++` | Redox's toolchain has gcc 13.2 for `aarch64-unknown-redox` with `libstdc++.a` |
| Threads (`std::thread`, `pthread`) | relibc has pthreads; four threads on four cores to be tested |
| `mmap` of the model file | relibc and RedoxFS have it; sharing versus copying to be tested |
| NEON | always there on aarch64; built for plain `armv8-a`, no feature detection at runtime (`GGML_NATIVE=OFF`) |
| Dynamic linking | relibc's loader fails with prebuilt programs (ROADMAP wish list), so **static linking** |
| HTTP, subprocesses | not needed: model download and server tools left out |

What it took to compile (2026-10-29, llama.cpp `7df8b87`): three small
source changes, none of them in the model code:

- `vendor/cpp-httplib`: `#include <sys/time.h>` for `timeval`, and no
  `accept4` on Redox (relibc lacks it).
- `common/arg.cpp`, `common/download.cpp`: `<limits.h>` instead of
  `<sys/syslimits.h>` for `PATH_MAX`.
- `LLAMA_SUBPROCESS=OFF`: relibc has no
  `posix_spawn_file_actions_addchdir_np`.

`llama-simple`, `llama-completion` and `llama-bench` build as static
programs of 11 to 16 MB.

## A safe tool layer

The model never runs commands. It can only ask for one of a fixed set of
tools, and the assistant program decides what happens.

1. **Fixed tools, not a shell.** Each tool is a function in the assistant
   with typed arguments (`list_files(folder)`, `show_time()`), not a
   command line. The model's output is held to valid tool calls by a
   llama.cpp grammar (GBNF) that only allows the names and argument types
   on the list, so it cannot write anything else in a tool call.
2. **Two kinds of tools.**
   - *Look only*, run at once: time and date, uptime, free memory, disk
     space, list a folder, show the start of a text file, network address,
     Redox version (`/etc/redox-rpi-release`), board data from
     `/scheme/rpi-firmware`.
   - *Changes something*, only after a confirmation: the assistant says in
     plain words what will happen ("Create the folder `Photos` in your home
     folder? [y/N]") and does nothing unless the user types `y`. Make a
     folder, copy or rename a file in the home folder, move a file to a
     trash folder (never delete), set the time, later Wi-Fi on and off,
     reboot.
3. **Paths.** File tools work only inside the home folder, after resolving
   `..` and links; a few system files can be read, none written.
4. **"How do I ...?" questions** get an answer with the Redox command, for
   the user to type themselves. Nothing typed by the model is ever run.
5. **No network.** Redox gives each process a set of schemes (its
   namespace). The assistant starts by dropping `tcp`, `udp`, `ip` and
   `netcfg` from its own namespace (`mkns`/`setrens`), so neither the
   model code nor a tool can reach the network, whatever goes wrong.
6. **No root.** It runs as the user. A tool that needs root (Wi-Fi, time,
   reboot) goes through `sudo` and its normal password prompt, after the
   confirmation.
7. **A log.** Every tool call, confirmed or not, goes to
   `~/.assistant.log`.

**Knowledge about Redox.** A 0.6B model knows little about Redox and
invents commands. The assistant ships a short help text: common tasks in
macOS, Windows and Linux terms with the Redox command for each. The parts
that match the question (plain keyword search, no second model) go into
the prompt. This matters more for good answers than the choice of model.

## Steps

| Step | What | Size |
|---|---|---|
| 1 | llama.cpp built for Redox, the model answers in QEMU | 1 to 3 days (compiles; run in progress) |
| 2 | A cookbook recipe linking our relibc, an experiment `assistant` that puts the model on the SD image or the data partition of the network-boot card (the network-boot image lives in RAM, 256 MiB at most); speed and memory measured on the Pi | 2 to 4 days plus a Pi slot |
| 3 | The `assistant` program: chat loop with Qwen3's template, saved prompt state, grammar-limited tool calls, the look-only tools | 1 to 2 weeks |
| 4 | Tools that change things, with confirmation; the namespace sandbox; the Redox help text and a test set of 50 to 100 questions from macOS, Windows and Linux users | 1 to 2 weeks |
| 5 | Optional: fine-tune the model (LoRA) on the tools and the help text, off the Pi | 2 to 4 weeks |
| 6 | Later: other models (FunctionGemma for the Zero 2 W, a small 1.58-bit model) | open |

## In progress

Branch [`assistant`](https://github.com/redox-rpi/redox-rpi/tree/assistant),
started 2026-10-29. Clone on homenodellm: `~/redox-rpi-assistant`,
llama.cpp and models in `~/assistant-work` (model files stay there, not in
git).

- llama.cpp `7df8b87` cross-compiled with the Redox gcc, static, with the
  three changes above
  ([patch](https://github.com/redox-rpi/redox-rpi/blob/assistant/patches/experiments/assistant/recipes/llama-cpp/redox.patch)).
- **First QEMU run (`raspi3b`, 1 GiB), 2026-10-29: the model answers on
  Redox.** The v0.2.0-5 SD image (`v0.2.0-5-gd679bfb+xed14`) with a hand
  build linked against the toolchain's relibc, Qwen3 0.6B Q4_0 copied in
  with the host's `redoxfs` FUSE mount. `llama-simple` loaded the model
  (359 MiB buffer, 28 MiB context for 256 tokens, 3 MiB working buffer)
  and wrote "The capital of France is **Paris**. The capital of France".
  Redox itself used 104 MiB of 932 MiB at the shell.
- **But the four worker threads crashed:** `futex_wake` in relibc
  (`src/sync/mod.rs:103`) got `EFAULT` and panicked, in every worker; the
  main thread went on and wrote the answer. That relibc is older than our
  kernel (`Using old SYS_OPENAT_INTO`), so the next run uses ours.
- QEMU is no measure of speed: loading the 382 MB file through the emulated
  SD controller took about 25 minutes (170 to 290 KB/s). QEMU `virt` would
  be faster but did not boot the upstream server image with 1 GiB: NVMe
  hangs after identify, `virtio-blkd` crashes, AHCI is not bootable.
- **Image with the experiment** `assistant` (recipes for llama.cpp, linked
  against our relibc, and the model): `v0.3.0-dev-156-gb366886+x18f5`
  (SD card). **QEMU `raspi3b`, 2026-10-29: clean run.** "The capital of
  France is Paris. The capital of France is also the capital of the", no
  crash, no exception, normal exit. So the thread crash came from the
  toolchain's older relibc. Memory: 96 MiB used before, 137 MiB after
  (of 912 MiB). Speed in QEMU (one host core emulating four): 0.19 tokens
  per second reading, 0.04 writing; says nothing about the Pi.
- Pi test #51 in the [queue](../test-plan.md): the SD card image, with a
  person to swap the card.

## Next

- Pi test #51: `llama-bench` for real speed, `free` while the model runs.
- A Pi test without a person: the network-boot image with llama.cpp, and
  the model on the netboot card's data partition (`/scheme/data`). llama.cpp
  can download it itself over plain HTTP from the build host (`-mu`,
  cpp-httplib is built in); untested.

## Later

- Steps 3 to 6.

## Sources

- [Qwen3-0.6B GGUF files](https://huggingface.co/unsloth/Qwen3-0.6B-GGUF) (sizes)
- [BitNet b1.58 2B4T GGUF](https://huggingface.co/microsoft/bitnet-b1.58-2B-4T-gguf), [bitnet.cpp](https://github.com/microsoft/BitNet)
- [FunctionGemma 270M](https://lmstudio.ai/models/google/functiongemma-270m)
- [llama.cpp](https://github.com/ggml-org/llama.cpp), [GBNF grammars](https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md)
- Pi 5 numbers: [pi-llm](https://github.com/ravijo/pi-llm), [Stratosphere research notes](https://stratospherelinuxips.readthedocs.io/en/develop/immune/research_rpi_llm_performance.html)
