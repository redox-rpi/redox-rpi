# Experiment: welcome, first-boot setup (#24)

A small console program, [`tools/welcome`](../../../tools/welcome/), asks
on the first boot for:

| Question | File it writes |
|---|---|
| keyboard layout (the list of `inputd --keymaps`), applied at once with `inputd -K` | `/etc/keymap` (read at boot with the `keyboard-de` experiment, #36) |
| time zone (the files under `/usr/share/zoneinfo`; skipped without them) | `/etc/localtime`, a link to the zone file (as the `timezone` experiment, #46) |
| hostname | `/etc/hostname` (no newline: relibc's `gethostname` returns the whole file) |
| a new root password (optional, runs `passwd root`) | `/etc/shadow` through `passwd` |
| Wi-Fi network and passphrase (optional, not echoed) | `/etc/net/wifi`, mode 600, `ssid=` and `psk=` lines, as the Wi-Fi driver reads it (#26) |

Then it writes `/etc/welcome.done` and does not run again. Delete that file
to get it at the next boot, or run `welcome --force` from a shell.

How it starts: base patch
[`0040`](base/0040-getty-run-the-first-boot-welcome-app-instead-of-logi.patch)
makes getty start `welcome --login` instead of `login` while
`/usr/bin/welcome` exists and `/etc/welcome.done` does not. When it is
done, the welcome app becomes `login` (exec, the same process). A first
version had getty start `login` after the welcome app exited; the login
prompt then waited for a key, because getty's events missed the new
program's first output.

Every console (serial and screen) shows the welcome; the setup runs on the
first one where someone presses Enter (a lock in `/tmp`). The other
consoles go on to the login prompt when Enter is pressed there. devsh and
other network logins do not see it.

It runs long after `growfs` (the SD session's half of #24), which grows the
card in the initfs before the root file system is mounted, restarting if
needed; getty starts only after that.

On a network boot the root file system is in RAM, so the welcome comes on
every boot. It is meant for SD card images.

```bash
scripts/build-image.sh --experiment timesync --experiment welcome
patches/experiments/welcome/test-welcome.sh
```

[`test-welcome.sh`](test-welcome.sh) answers the questions on QEMU's
serial console, logs in and checks the files, then boots the same card
again and checks that the welcome does not come back.

## Results

- QEMU `raspi3b`, `v0.3.0-dev-260-g480ba01+xda44-dirty.371f590ffeb2`:
  layout `gb`, hostname `pi-test`, Wi-Fi `TestNet` with a passphrase.
  `/etc/keymap`, `/etc/hostname`, `/etc/net/wifi` (`-rw-------`) and
  `/etc/welcome.done` are written, the passphrase is not echoed, the login
  prompt follows at once, and the second boot goes straight to the login
  prompt with hostname `pi-test`. Not covered: the time zone question
  (this image has no zone files) and the screen console.
- Real Pi 3B+, test 78, `v0.3.0-dev-261-ge881cdb+xda44` (network boot):
  the welcome waits on the serial console with no login prompt, all
  questions answered there (`gb`, `pi-test`, `TestNet`), the passphrase is
  not in the serial log, the login prompt follows at once, the files are
  written as in QEMU (`/etc/net/wifi` `-rw-------`), standard checks pass,
  0 of 300 starts failed. Not checked: the screen console (HDMI) and the
  time zone question.

See also [test-reports.md](../../../docs/test-reports.md).
