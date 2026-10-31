# Experiment: tcp-send, a sender for network speed tests

Adds [`tools/tcp-send`](../../../tools/tcp-send) to the image. It connects
to a TCP port, sends zero bytes in 64 KiB writes and prints the bytes, the
seconds and MB/s. Redox's `nc` connects but sends nothing on the Pi, from a
pipe or from a file (Pi test #135), so speed tests need their own sender.

```bash
scripts/build-image.sh --experiment tcp-send
```

On the build host `nc -l <port> | wc -c`, then on the target:

```
tcp-send <build host>:<port> 16777216
```
