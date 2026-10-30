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
| CPU | 4x Cortex-A53, 1.4 GHz, ARMv8.0 | NEON, but no `dotprod` (`sdot`), `i8mm` or FP16 arithmetic: kernels written for newer ARM cores stop with an illegal instruction |
| Memory bandwidth | about 2 GB/s in practice (32-bit LPDDR2) | the limit for writing answers: every new word reads all the weights once |
| GPU | VideoCore IV | not usable (see [graphics](graphics.md), step 3) |
| Heat | throttles from 60 °C on the 3B+ | four busy cores for minutes; a heat sink helps |

The Zero 2 W has half the RAM (512 MiB): only the smallest models below
would fit there.

## Models

| Model | File (GGUF) | Licence | Fits 1 GiB | Notes |
|---|---|---|---|---|
| **Qwen3 0.6B**, Q4_0 | 382 MB | Apache 2.0 | yes | **first choice.** Instruction-tuned, knows tool calls (its chat template has `<tool_call>` blocks), "thinking" can be turned off with `/no_think`. Q4_0 is the simplest 4-bit format to write a fast kernel for |
| Qwen3 0.6B, Q4_K_M | 397 MB | Apache 2.0 | yes | a bit better quality, a bit slower on the A53 |
| Qwen3 0.6B, Q8_0 | 639 MB | Apache 2.0 | tight | best quality of the three; leaves little for context and Redox |
| FunctionGemma 270M | about 280 MB (8-bit) | Gemma terms | yes, also the Zero | made by Google for function calls only; weak at answering questions, meant to be fine-tuned on one's own tools |
| SmolLM2 360M Instruct | about 230 MB (Q4) | Apache 2.0 | yes, also the Zero | small and fast, noticeably weaker answers |
| **BitNet b1.58 2B4T** (Microsoft) | 1.19 GB (`i2_s`) | MIT | **no** | 1.58-bit weights (0.4 GB), but the word table (embedding) is stored at 16 bits; the file alone is larger than the Pi's RAM |

**BitNet:** the idea fits the Pi well (ternary weights, little memory
traffic), but the only official model does not fit in 1 GiB. In Rust it
would be a kernel of its own in the hand-written runtime below: weights
packed 2 bits each (`TQ2_0`/`I2_S`), the dot product only adds and
subtracts, done with NEON table lookups; 300 to 500 lines, **1 to 2
weeks**, worth it only once a 1.58-bit model of 1B parameters or less with
a small word table exists. Not now.

### Memory for Qwen3 0.6B Q4_0

| Part | Size |
|---|---|
| Weights | 382 MB |
| Context (KV cache), 1024 tokens at 16 bits | 112 MiB (28 layers x 2 x 1024 values x 2 bytes per token); 56 MiB with an 8-bit cache |
| Working buffers | about 50 to 100 MiB, less with small batches |
| Redox with the console and network | about 96 MiB at the shell (QEMU `free`) |
| **Total** | about 600 to 650 MiB, leaves room for the user's programs |

Without swap and with no page cache that can drop pages, the model must
stay fully in RAM. candle reads it into the heap (no `mmap`), which is
fine at this size; a hand-written runtime can do the same.

Measured with `rinfer` (candle) in QEMU: 468 MiB heap after loading,
497 MiB at the peak, 674 MiB of 912 MiB used in total while it ran. Redox
backs memory with RAM as soon as it is allocated, even if it is never
touched (Linux does not), and runs out with a kernel panic (no swap, no
OOM handling). So every large buffer counts, not only the ones in use.

### RAM the assistant needs

Measured 2026-10-30 (`rinfer --serve` heap counter on the build host and
in QEMU, Redox `free` in QEMU, which shows 912 MiB):

| Part | MiB |
|---|---|
| Redox at the shell (console, network) | 96 to 100 |
| Model loaded, cache for 256 tokens | 468 (497 at the peak while loading) |
| Each further 256 tokens of conversation (key/value cache, f32) | +56 |
| System prompt with the tools (380 tokens), first question | 527, peak 565 |
| Conversation of about 700 tokens | 583, peak 594 |
| `assistant` itself | about 1 |

So the assistant needs about **700 MiB** of the Pi's RAM for a
conversation of 700 tokens, about 760 MiB for 1,000 tokens, and leaves
about 150 to 200 MiB. Raising `gpu_mem` from 16 to 64 MiB (48 MiB less
for Redox) still fits; 128 MiB leaves only about 50 to 100 MiB and needs a
shorter conversation (the assistant would start over after about 500
tokens); more does not fit. An 8-bit or 16-bit cache would halve or
quarter the +56 MiB steps (not in candle's CPU path yet).

### Speed (estimates, not measured)

- **Writing an answer:** each token reads all 382 MB of weights. At about
  2 GB/s that gives at most 5 tokens per second; with the A53's compute on
  top, **2 to 4 tokens per second** is likely. A 50-word answer takes
  15 to 30 seconds. Shown word by word as it is written, that is usable.
- **Reading the question (prompt):** compute-bound, maybe 10 to 20 tokens
  per second on four cores. A system prompt with tool descriptions and a
  help text of 300 to 500 tokens would take **half a minute or more** on
  every start. So the prompt state (KV cache) is computed once and saved
  to a file, and loaded at start in a second.
- For comparison, published numbers for 0.5B models on a Pi 5 are 8 to 19
  tokens per second; the Pi 3 has about a third of its memory bandwidth.

## Runtime: Rust

For Redox the runtime should be Rust, like the rest of the system. The
choice is between an existing Rust library and a small inference loop of
our own. llama.cpp stays as a reference for correctness and speed (see In
progress).

| | candle (Hugging Face) | mistral.rs | burn | own loop |
|---|---|---|---|---|
| Qwen3 GGUF | yes: `quantized_qwen3`, GGUF reader, the tokenizer from the GGUF file | yes (built on a candle fork) | no ready LLM path for GGUF; general framework | to write: GGUF reader, Qwen3 forward pass, BPE tokenizer |
| Dependencies | about 100 crates (gemm, rayon, half, safetensors, zip, `tokenizers`) | candle plus tokio, an HTTP server, many more | large; backends (ndarray, wgpu) | `std` and maybe one regex crate |
| Threads | rayon (std threads, so relibc pthreads) | same plus tokio | backend-specific | `std::thread`, 4 workers |
| `mmap` | not used for GGUF (read into the heap); `memmap2` is a dependency | uses `mmap` | n/a | not needed |
| NEON on the A53 | **breaks:** the quantized dot products use `sdot` unconditionally; needs a fallback without `dotprod` (about 30 lines, `vmull_s8`/`vpadalq_s16` as in llama.cpp) | same kernels | no hand-made quantized NEON kernels | written for ARMv8.0 NEON from the start |
| Binary size (estimate) | 10 to 15 MB | 30 MB and more | large | 1 to 2 MB |
| Speed (estimate) | memory-bound like llama.cpp; with the fallback maybe 60 to 90 % of it | as candle | slower (no quantized kernels) | same as llama.cpp is reachable: the work is memory-bound |
| Effort to first answer on the Pi | **2 to 4 days** (build for Redox, the `sdot` fallback, recipe) | 1 to 2 weeks, mostly trimming | weeks | **2 to 3 weeks** |

**Recommendation:** start with **candle**. The model code exists, the
tokenizer comes from the GGUF file, and the two known problems are small:
the `sdot` fallback (also a useful upstream patch) and building its
dependency tree for `aarch64-unknown-redox` (relibc, static). If candle
turns out too big or too slow on the Pi, write **our own loop**: Qwen3 is a
plain transformer (RMSNorm, RoPE, grouped-query attention, SwiGLU), and a
Q4_0/Q8_0-only runtime with NEON kernels for ARMv8.0 is about 1500 to 2500
lines. It would also be the place for a BitNet kernel. mistral.rs and burn
are not a fit for 1 GiB and an A53.

What a Rust runtime needs from Redox:

| Need | State on Redox |
|---|---|
| Rust `std` for `aarch64-unknown-redox` | in the Redox prefix (rustc 1.98-dev) |
| Threads | std threads over relibc pthreads; llama.cpp's four threads ran cleanly with our relibc in QEMU |
| Files | plain `read`; the model is read once into memory |
| SIMD | `core::arch::aarch64` NEON intrinsics; no runtime feature detection on Redox (`is_aarch64_feature_detected!` reports nothing), so build for `cortex-a53` and avoid `dotprod` |
| Linking | static, against our relibc (`-L native=…/relibc/stage/usr/lib`, as `sigregs` does); relibc's dynamic loader still fails with prebuilt programs |
| C code in crates | to avoid: `tokenizers` without `onig` and `esaxx` (feature `fancy-regex`, pure Rust) |

## A safe tool layer

The model never runs commands. It can only ask for one of a fixed set of
tools, and the assistant program decides what happens.

1. **Fixed tools, not a shell.** Each tool is a function in the assistant
   with typed arguments (`list_files(folder)`, `show_time()`), not a
   command line. While the model writes a tool call, the runtime masks
   every token that would not fit the call's grammar (only the names and
   argument types on the list, as llama.cpp's GBNF grammars do), so it
   cannot write anything else in a tool call. In Rust this is a small
   state machine over the tokenizer's vocabulary.
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
| 1 | Reference: llama.cpp built for Redox, the model answers in QEMU; recipe and SD image | done; Pi test #51 for reference numbers |
| 2 | Rust runtime: candle built for Redox with an `sdot` fallback, a small test program, recipe in the experiment; answers in QEMU, speed against llama.cpp on the Pi | answers in QEMU, worker-pool hang fixed (4 days); Pi speed test open |
| 2b | Only if candle is too big or slow: our own Qwen3 runtime in Rust (GGUF, Q4_0/Q8_0 NEON kernels for ARMv8.0, tokenizer) | 2 to 3 weeks |
| 3 | The `assistant` program in Rust: chat loop with Qwen3's template, saved prompt state, grammar-limited tool calls, the look-only tools | chat loop and tools with the model run in QEMU (1 day); saved prompt state and grammar mask open, about 1 week |
| 4 | Tools that change things, with confirmation; the namespace sandbox; the Redox help text and a test set of 50 to 100 questions from macOS, Windows and Linux users | 1 to 2 weeks |
| 5 | Optional: fine-tune the model (LoRA) on the tools and the help text, off the Pi | 2 to 4 weeks |
| 6 | Later: other models (FunctionGemma for the Zero 2 W); a Rust 1.58-bit kernel once a small BitNet model exists | open (kernel 1 to 2 weeks) |

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
  person to swap the card. Gives the llama.cpp reference numbers.
- **Rust runtime, 2026-10-29: `rinfer` (candle `12bb5d1`) answers in
  QEMU.** A test program in the experiment
  ([recipe](https://github.com/redox-rpi/redox-rpi/tree/assistant/patches/experiments/assistant/recipes/rinfer)):
  Qwen3 from the GGUF file, the tokenizer from the same file, greedy
  decoding, timings and a heap counter. Built static against our relibc
  with cargo; 4.9 MB binary. Three changes to candle were needed:
  - `sdot`: candle's NEON dot products for Q4_0, Q8_0 and the K formats
    use the `dotprod` instruction, which the A53 lacks.
    [sdot-fallback.patch](https://github.com/redox-rpi/redox-rpi/blob/assistant/patches/experiments/assistant/recipes/rinfer/sdot-fallback.patch)
    gives the same result with plain NEON (multiply-long and pairwise
    add); the binary has no `sdot` left.
  - The token table (151,936 x 1,024) was turned into f32, 622 MB, and
    read twice. [qwen3-memory.patch](https://github.com/redox-rpi/redox-rpi/blob/assistant/patches/experiments/assistant/recipes/rinfer/qwen3-memory.patch)
    keeps it quantized and unpacks only the rows of the input tokens; the
    output layer shares it.
  - The key/value cache was preallocated for 4,096 positions per layer:
    32 MiB x 28 layers, 900 MiB. On Redox that is real memory, so the
    first runs ended in an out-of-memory kernel panic. The same patch adds
    `CANDLE_KV_CACHE_LEN`; `rinfer` starts with 256 positions and the
    cache doubles when full. Heap peak on the build host: 1,337 MiB
    before, 497 MiB after.
  - candle sizes its thread pool from the CPU count, which Redox does not
    report (one thread); `rinfer` sets four.

  QEMU `raspi3b`, prompt "Hi" (17 tokens with the template): "Hello! How
  can", 7 seconds per token for the prompt and for writing (0.14 to
  0.15 tokens per second; llama.cpp in QEMU: 0.19 reading, 0.04
  writing). QEMU speed says nothing about the Pi.
- **The whole prompt at once hung, 2026-10-30:** candle's worker pool
  parks idle threads with `thread::park`, and on Redox a wakeup was
  sometimes lost, so a matmul waited forever (a benchmark without the
  model, `cbench`, hung within 200 calls; with threads that never park,
  no hang). The relibc session traced it to a race in the kernel's
  `FUTEX_WAIT`, fixed upstream after our pin (kernel `57ccc8f`,
  `5805059`); with that backport `cbench` runs 600 calls without a hang.
  Until it is in our kernel, rinfer's
  [redox-thread-pool.patch](https://github.com/redox-rpi/redox-rpi/blob/assistant/patches/experiments/assistant/recipes/rinfer/redox-thread-pool.patch)
  lets idle workers poll with a 200 µs sleep. In QEMU the whole prompt at
  once is now about 9 times faster than one token at a time: 380 tokens in
  280 s.
- **The assistant answers with the model, 2026-10-30:** `rinfer --serve`
  keeps the model loaded; `assistant` starts it, puts the tools in the
  system message in Qwen3's format, runs `<tool_call>` replies through the
  same allow-list and confirmation as typed calls, and hands the result
  back. The 0.6B model sometimes writes a call without the tags
  (`list_files` or `list_files(folder="Documents")`); those are accepted
  too. On the build host it answered the time, the files in the home
  folder and in `Documents`, "Paris", "use the `cp` command", and asked
  before `make_folder` (`n` changed nothing, `y` created it). Misses: for
  "Show me my files" it asked for a folder named `home`. QEMU `raspi3b`:
  "What time is it?" ran `show_time` and answered from it, heap peak
  565 MiB, no out-of-memory; reading the system prompt took 280 s and
  writing ran at 0.4 to 0.5 tokens per second (QEMU, not the Pi).
- **A Pi test without a person, 2026-10-30:** the experiment `assistant`
  (llama.cpp, `rinfer`, `assistant`) now fits the network-boot image; the
  model file moved to the experiment `assistant-model` (SD card only).
  On a network-booted Pi, `rinfer --download` fetches the model over plain
  HTTP from the build host onto the data partition `/scheme/data`, in
  16 MiB parts (`/srv/tftp/models/qwen3-parts/`), each on its own
  connection (curl from the prebuilt packages crashes on aarch64). In QEMU
  the parts arrived until the 256 MiB disk was full; a full disk then hung
  the next write instead of failing it.
- **Tool layer, 2026-10-29:** [`tools/assistant`](https://github.com/redox-rpi/redox-rpi/tree/assistant/tools/assistant),
  plain Rust with no crates (264 KB static binary for Redox). It runs tool
  calls in the form Qwen3 writes them
  (`<tool_call>{"name": ..., "arguments": {...}}</tool_call>`); without a
  model they are typed by hand (`assistant --call '…'`, or `/call` in the
  interactive mode). Tools: `show_time` and `list_files` (look only), and
  `make_folder` (changes something, so it asks `[y/N]` first). Only listed
  tools and arguments are accepted, arguments must be text, paths are
  taken from the home folder and may not leave it or pass a link, and
  every call goes to `~/.assistant.log`. Unit tests on the build host;
  in the image `v0.3.0-dev-161-g5b7ccbc+x18f5` (SD card) in QEMU
  `raspi3b`: all tools work, `n` leaves nothing changed, `y` creates the
  folder, `/etc` and an unknown `run_shell` tool are refused, the log has
  all six calls. Missing: the namespace sandbox (needs the
  `redox_syscall` crate or a raw system call) and the grammar mask, which
  comes with the model runtime.

## Next

- A Pi test without a person (network boot, the model fetched onto
  `/scheme/data` with `rinfer --download`): speed of `rinfer` with the
  whole prompt and one token at a time, memory, and the assistant with
  tools. If candle reaches about 2 tokens per second, it stays; if not,
  our own loop (step 2b). Pi test #51 (SD card, llama.cpp numbers) still
  waits for a card swap.
- Save the state after the system prompt to a file, so the assistant
  does not read 380 tokens on every start.
- A grammar mask while a tool call is written, and a test set of
  questions; a cap on the conversation length (memory).

## Later

- Steps 3 to 6.

## Sources

- [Qwen3-0.6B GGUF files](https://huggingface.co/unsloth/Qwen3-0.6B-GGUF) (sizes)
- [BitNet b1.58 2B4T GGUF](https://huggingface.co/microsoft/bitnet-b1.58-2B-4T-gguf), [bitnet.cpp](https://github.com/microsoft/BitNet)
- [FunctionGemma 270M](https://lmstudio.ai/models/google/functiongemma-270m)
- [candle](https://github.com/huggingface/candle) (`candle-core/src/quantized/neon.rs`), [mistral.rs](https://github.com/EricLBuehler/mistral.rs), [burn](https://github.com/tracel-ai/burn)
- [llama.cpp](https://github.com/ggml-org/llama.cpp), [GBNF grammars](https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md)
- Pi 5 numbers: [pi-llm](https://github.com/ravijo/pi-llm), [Stratosphere research notes](https://stratospherelinuxips.readthedocs.io/en/develop/immune/research_rpi_llm_performance.html)
