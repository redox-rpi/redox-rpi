# Experiment: serverfs, a home server directory as a scheme (read-only)

Adds [`tools/serverfs`](../../../tools/serverfs): a directory on a WebDAV
server shows up as `/scheme/<name>/`. Read-only spike for
[#713](https://github.com/redox-rpi/redox-rpi/issues/713), stage 2 of
[docs/home-server-storage.md](../../../docs/home-server-storage.md).

```bash
scripts/build-image.sh --experiment serverfs
```

On the target, with a WebDAV server in the home network (for example
`rclone serve webdav <dir> --addr :8443 --cert server.pem --key server.key
--htpasswd users`):

```text
serverfs -d --auth ~/.serverfs-auth --ca /etc/home-ca.pem https://<server>:8443/ server
ls /scheme/server
cat /scheme/server/notes.txt
```

- `--auth <file>`: one line `user:password` for HTTP Basic login. Over
  `http://` the password goes over the network in plain text, so use
  `https://` (or `http://` only in a trusted network).
- `--ca <file>`: PEM file with the certificates to trust (a home CA or the
  server's own certificate). Without it serverfs trusts
  `/etc/ssl/certs/ca-certificates.crt` if the image has it, else the
  Mozilla roots built into the program.
- The clock must be right for the certificate check (experiment
  `timesync`); a certificate that is not valid yet gives a hint to check
  `date -u`.

Writes give `EROFS`; a server that does not answer gives `EHOSTUNREACH`, a
refused login `EACCES`, a TLS failure `EIO` with the reason on stderr.

Test in QEMU: `scripts/test-serverfs.sh` (throwaway WebDAV servers, HTTP and
HTTPS with a throwaway CA, on the build host's loopback; nothing installed).
