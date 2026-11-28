# RFC 0006: Console login and passwords in release images

| | |
|---|---|
| Status | Draft |
| Created | 2026-11-28 |
| Related | [RFC 0004](0004-services-on-demand.md) (consoles only when needed) |
| Releases | v0.8.0 or later, after the decision |

## Summary

Release images get no login on the serial port by default, and no known
password. Test images keep both.

- A key in `cmdline.txt` on the FAT boot partition turns the serial
  login on (`SERIAL_LOGIN=1`) or off (`SERIAL_LOGIN=0`). It can be set on
  a Mac or PC, without a login on the Pi.
- Kernel messages stay on serial in every image.
- There is no default root password in release images. The first login
  sets one, or a file on the FAT partition provides it, as a hash.

## Motivation

- **Serial login:** anyone with a USB serial cable on the GPIO header gets
  a login prompt. With the known password, that is root.
- **The banner:** `/etc/issue` prints `root`:`password` on every console
  (it is written by `scripts/build-image.sh`). The same pair works in
  every image ever built.
- **A headless Zero without network:** serial is its only way in. So
  turning serial off must come with a simple, documented way to turn it
  back on.
- **Our tests:** every Pi test (`pi-test.py`, Pi Tests) logs in over serial
  as root. That must keep working without extra steps.

## Proposal

### Images

| | Release image | Test image (lab, CI, Pi Tests) |
|---|---|---|
| Kernel messages on serial | yes | yes |
| Login on serial | off; `SERIAL_LOGIN=1` turns it on | on; `SERIAL_LOGIN=0` turns it off |
| Login on the screen | with a monitor (RFC 0004) | with a monitor |
| Root password | none until set | `password`, as today |
| Banner with credentials | no | yes |
| `devshd` | not in the image (RFC 0004) | lab hook |

What makes a test image is the build mode that already decides other lab
parts (`config/raspi3bp.toml` against `raspi3bp-sdcard.toml` and
`--release-netboot`). There is no new flag.

### The serial switch

- **The key:** `SERIAL_LOGIN=1` or `SERIAL_LOGIN=0` in `cmdline.txt` on
  the FAT boot partition. The key overrides the image default (off in
  release images, on in test images); without the key the image default
  holds. It follows the pattern of the other boot keys there
  (`KEYMAP=de`, `RPI_FB=`, `RPI_ETH_SPEED=`): people edit one line with
  any editor on the mounted card.
- **How it reaches init:** the firmware puts the `cmdline.txt` words into
  the devicetree's `/chosen bootargs`, and the aarch64 kernel turns those
  into its environment (`kernel/src/dtb/mod.rs`). On a direct-boot card
  the boot stub puts the words into the environment. `rpi-firmwared`
  also offers the command line as a file. A unit `30_getty_debug` gets a
  boot condition on that value (RFC 0004 step 3). Redox needs no FAT
  driver at boot, and U-Boot and direct boot work the same way.
- **Netboot:** `cmdline.txt` sits on the boot server next to the image,
  so the key works the same way.
- **Documented:** a section in the user guide ("Serial console") and one
  line in the README of the release. The key appears in the boot
  log, so someone who looks at the serial output learns how to turn the
  login on.

### Passwords

There are three parts.

1. **No default root password in release images.** Root's password field
   is locked (`!`) in the image, and `user` gets no password as today.
   The banner stops printing credentials.

2. **First login sets it.** On the first login on any console as `user`
   or `root`, `login` sees the locked root and asks for a new root
   password, twice, before it starts the shell. Until a password is set,
   `sudo` and `su` refuse.

3. **Headless setup.** A file `redox/userconf.txt` on FAT holds
   `root:<hash>` (SHA-512 crypt, as `openssl passwd -6` makes it). At
   boot it is read once, written to `/etc/shadow`, and deleted. The
   user guide shows the command for macOS, Linux and Windows. This is
   the same idea as on other Pi systems, so people know it.

For sshd (experiment): no password login for root; keys only.

### Tests and person tests

- Pi Tests and `pi-test.py` use test images and change nothing.
- Person tests that use a release image (keyboard, cable, SD card) get one
  extra step: set the root password at the first login. The test row
  says the password to use. A row that should not change the card uses
  a test image.
- `devshd` has no login at all. It stays in lab images only, as RFC 0004
  already says.

## Not in this RFC

- **User accounts beyond `root` and `user`.**
- **Encrypted storage.**
- **Network login:** sshd stays an experiment, with keys only.
- **Hiding kernel messages:** they stay on serial for diagnosis.

## Open questions

1. **First-login password:**
   - Ask at the first login, or block every login until `userconf.txt`
     exists?
   - Asking is friendlier; blocking leaves no window at all.
2. **`user` account:**
   - Should `user` also need a password in release images?
3. **Recovery from a lost password:**
   - Does a second FAT file need to reset it, or is `userconf.txt` again
     enough?

## Steps

| Step | Content |
|---|---|
| 1 | Banner without credentials in release images; credentials stay in test images |
| 2 | `SERIAL_LOGIN` from `cmdline.txt` over the image default; boot condition on `30_getty_debug`; release images default off |
| 3 | Locked root in release images; `login` asks for a password at the first login |
| 4 | `redox/userconf.txt` read once at boot, then deleted |
| 5 | User guide: serial switch, first login, headless setup |
