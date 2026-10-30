# Experiment: assistant-model, the language model file in the image (#43)

Adds the Qwen3 0.6B model (Q4_0, 382 MB) to the image, as
`/usr/share/assistant/models/Qwen3-0.6B-Q4_0.gguf`. The recipe
(`qwen3-model`) is in the [`assistant`](../assistant) experiment, so the
two go together, and only into the SD-card image:

```bash
scripts/build-image.sh --sdcard --experiment assistant --experiment assistant-model
```

The network-boot image is loaded into RAM and holds at most 256 MiB; it
takes `--experiment assistant` alone, and the model is fetched onto the
data partition of the network-boot card:

```bash
rinfer --download http://192.168.178.120:8080/models/Qwen3-0.6B-Q4_0.gguf /scheme/data/Qwen3-0.6B-Q4_0.gguf
ASSISTANT_MODEL=/scheme/data/Qwen3-0.6B-Q4_0.gguf assistant
```
