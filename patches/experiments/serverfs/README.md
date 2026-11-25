# Experiment: serverfs, a home server directory as a scheme (read-only)

Adds [`tools/serverfs`](../../../tools/serverfs): a directory on a WebDAV
server shows up as `/scheme/<name>/`. Read-only spike for
[#713](https://github.com/redox-rpi/redox-rpi/issues/713), stage 2 of
[docs/home-server-storage.md](../../../docs/home-server-storage.md).

```bash
scripts/build-image.sh --experiment serverfs
```

On the target, with a WebDAV server in the home network (for example
`rclone serve webdav <dir> --addr :8080` on any machine):

```text
serverfs -d http://<server>:8080/ server
ls /scheme/server
cat /scheme/server/notes.txt
```

`--auth <file>` reads `user:password` for HTTP Basic login. There is no TLS
yet, so the password goes over the network in plain text: only in a
trusted home network. Writes give `EROFS`; a server that does not answer
gives `EHOSTUNREACH`, a refused login `EACCES`.

Test in QEMU: `scripts/test-serverfs.sh` (a throwaway WebDAV server on the
build host's loopback, nothing installed).
