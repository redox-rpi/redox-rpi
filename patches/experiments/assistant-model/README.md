# Experiment: assistant-model, the language model file in the image (#43)

Adds the Qwen3 0.6B model (Q4_0, 382 MB) to the image, as
`/usr/share/assistant/models/Qwen3-0.6B-Q4_0.gguf`, and llama.cpp's
programs for comparison. The recipes (`qwen3-model`, `llama-cpp`) are in
the [`assistant`](../assistant) experiment, so the two go together, and
only into the SD-card image:

```bash
scripts/build-image.sh --sdcard --experiment assistant --experiment assistant-model
```

The network-boot image is loaded into RAM; it takes `--experiment
assistant --experiment assistant-netboot`, and the model is fetched onto
the data partition of the network-boot card, in 16 MiB parts:

```bash
rinfer --download http://192.168.178.120:8080/models/qwen3-parts/Qwen3-0.6B-Q4_0.gguf.partNNN /scheme/data/Qwen3-0.6B-Q4_0.gguf
ASSISTANT_MODEL=/scheme/data/Qwen3-0.6B-Q4_0.gguf assistant
```
