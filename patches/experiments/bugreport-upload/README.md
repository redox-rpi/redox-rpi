# Experiment: bugreport-upload, send bug reports to the build host

Issue: [#759](https://github.com/redox-rpi/redox-rpi/issues/759). The
variant of [`bugreport`](../bugreport/README.md) for dev images: the same
tool, plus `/etc/bugreport.conf` with the receiver on the build host
(`@BUILD_SERVER@`, port 8090). After writing the file, `bugreport` sends it
by one HTTP POST over a plain TCP socket (no curl; the Zero 2 W image has
none), and prints the ID the receiver gives:

```text
root:~# bugreport
What went wrong? (one line, then Enter): the screen stays black after reboot
Report written to /home/root/bugreport-20261127-142301.txt
Sending it to the build host ... done.
Report BR-0007 sent. Please pass on this ID: BR-0007
```

The tester passes on the ID; a session reads
`~/redox-rpi/logs/bugreports/BR-0007.txt` on the build host
(`scripts/bugreport-receiver.py --list` lists them) and files the issue. If
the upload fails, `bugreport` says why and keeps the file.

It is test-only (`test-only`), so `--release` refuses it: release images
never upload.

```bash
scripts/build-image.sh --experiment bugreport-upload
```

## Receiver

[`scripts/bugreport-receiver.py`](../../../scripts/bugreport-receiver.py),
Python standard library:

```bash
scripts/bugreport-receiver.py                 # port 8090, reports in ~/redox-rpi/logs/bugreports
scripts/bugreport-receiver.py --list          # ID and first line of each report
```

It accepts only `POST /bugreport`, text, at most 1 MiB, and stores at most
500 reports. It has no authentication: lab network only. The permanent
service on the build host is set up outside this repository.
