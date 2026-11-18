# Experiment: assistant-netboot, a network-boot image with room for the model (#43)

The network-boot image is loaded into the Pi's RAM. With the usual 256 MiB
image Redox had 745 MB in all and 607 MB free (Pi test 106), too little
for the Qwen3 0.6B model, which needs about 600 MiB with a short
conversation. This experiment only makes the image 128 MiB; the assistant
experiment needs about 90 MiB of it.

```bash
scripts/build-image.sh --experiment timesync --experiment assistant --experiment assistant-netboot
```

The model goes onto the data partition of the network-boot card, see
[`assistant-model`](../assistant-model/README.md).
