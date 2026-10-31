# Getting started

From a fresh download to a login prompt on your Raspberry Pi, then the
first things to try. For people new to this project; no Redox knowledge
needed. Afterwards the [user guide](user-guide.md) explains the system
itself.

**This is experimental (alpha) software.** It boots and you can use a
shell, but things break. Use a spare SD card and keep nothing important on
the Pi.

## What you need

- A **Raspberry Pi 3 Model B+**, or a **Raspberry Pi Zero 2 W**. Other
  models (Pi 3 Model B, Pi 4, Pi 5) are not supported.
- A **microSD card** of at least 2 GB. Everything on it is erased.
- A **5 V power supply** for the Pi (a weak one causes odd crashes).
- One way to see and type:
  - **HDMI screen and a USB keyboard** (Pi 3B+), or
  - a **serial cable**: a 3.3 V USB-to-UART adapter (CP2102, FT232 or
    CH340). Not a 5 V one, that damages the Pi.
- An **Ethernet cable** to your router, for the network and the clock.
  Wi-Fi does not work yet. The Zero 2 W has no Ethernet port: it needs a
  USB Ethernet adapter or hub with an RTL8152/RTL8153 chip.

**No Pi?** You can try the same image in QEMU, see
[At the end: QEMU](#no-pi-try-it-in-qemu).

## 1. Download the image

Open the [releases page](https://github.com/redox-rpi/redox-rpi/releases)
and download from the newest release (now v0.2.0):

- `redox-rpi-v0.2.0-pi3bplus.img.xz`, the SD card image (about 24 MB), and
- `SHA256SUMS`, to check the download.

The same image works on the Pi 3B+ and the Zero 2 W.

Check it (optional, but it catches broken downloads), in the download
folder:

```sh
sha256sum -c --ignore-missing SHA256SUMS      # Linux
shasum -a 256 -c --ignore-missing SHA256SUMS  # macOS
```

It should print `redox-rpi-v0.2.0-pi3bplus.img.xz: OK`.

## 2. Write it to the SD card

**The easy way:** [Raspberry Pi Imager](https://www.raspberrypi.com/software/)
→ "Choose OS" → "Use custom" → pick the `.img.xz` file → choose the card →
write. Skip its offer to change settings (user name, Wi-Fi): they are for
Raspberry Pi OS and do nothing here. balenaEtcher works the same way.

**On the command line** (Linux or macOS), if you know which device is the
card. Writing to the wrong device erases that disk:

```sh
xz -dc redox-rpi-v0.2.0-pi3bplus.img.xz | sudo dd of=/dev/sdX bs=4M conv=fsync
```

On macOS the card is `/dev/rdiskN` (see `diskutil list`; unmount it first
with `diskutil unmountDisk /dev/diskN`).

## 3. Connect the Pi

With the power **off**:

- Put the card into the Pi.
- Plug in the **USB keyboard now**: a keyboard plugged in after power-on
  is not found.
- Connect HDMI and Ethernet.
- Or connect the serial adapter (it can stay connected together with the
  screen):

  | Pi pin | Signal | Adapter |
  |---|---|---|
  | 6 | GND | GND |
  | 8 | TXD (GPIO 14) | RXD |
  | 10 | RXD (GPIO 15) | TXD |

  TX goes to RX and RX to TX. Leave the adapter's VCC/3.3 V/5 V pin
  unconnected. On a Zero 2 W the header pins must be soldered on.

  On your computer, open the adapter at 115200 baud:

  ```sh
  screen /dev/ttyUSB0 115200            # Linux; exit with Ctrl-A, K
  screen /dev/tty.usbserial-* 115200    # macOS
  ```

  On Windows, use PuTTY with the adapter's COM port, "Serial", 115200.

## 4. First boot

Switch the power on. What you see, on the screen or the serial console:

1. The Raspberry Pi firmware and **U-Boot** (a few lines, a short
   countdown). Do not press a key during the countdown; if you did and
   see `U-Boot>`, type `boot` and Enter.
2. The **Redox bootloader** asks for a screen resolution. Press **Enter**
   to take the default.

   Other keys do other things here: `e` opens an editor for boot settings
   (an empty line and Enter boots normally), `l` switches "live mode".
   If you see something unexpected, power off and on again.
3. Many lines of boot messages: the kernel, the four CPU cores, drivers,
   the network. Messages about `pcid` or `audiod` are harmless (the Pi has
   no PCI bus and no sound driver).
4. A box with the login details, the image's version, and
   `redox login:`.

The first boot can take a minute or more: be patient until the login
prompt appears.

On the HDMI screen, `Super` + `F1` (the Windows key) shows the boot
messages and `Super` + `F2` goes back to the login.

## 5. Log in

| User | Password |
|---|---|
| `root` | `password` (administrator) |
| `user` | none |

Type the user name at `redox login:` and press Enter, then the password.
The keyboard layout is **US**: on a German keyboard, `y` and `z` are
swapped and `-` is on the `ß` key.

Change the root password right away if the Pi is in a shared network:
`passwd`.

## 6. First things to try

```sh
cat /etc/redox-rpi-release           # which image is this?
uname -a
ls /                                 # looks like Unix
cat /scheme/rpi-firmware/temperature # CPU temperature in m°C (47236 = 47.2 °C)
free -h                              # memory
ifconfig                             # the Pi's network address
ping -c 3 1.1.1.1                    # the internet
date                                 # the time, from the network
```

**The shell is Ion, not bash.** Simple commands work as usual; variables
and `if` / `for` look different, and an argument starting with `@` or
containing `#` must be quoted. The [user guide](user-guide.md#the-ion-shell-instead-of-bash)
has a table.

**The network** comes up by itself with DHCP. If `ifconfig` shows no
address, check the cable and that your router hands out addresses.

**The clock** is right once the network is up: the Pi has no clock that
runs while it is off, and `timesync` gets the time from the internet. More
in [time.md](time.md).

## 7. Installing programs

`pkg` installs prebuilt Redox packages over the internet, as root:

```sh
pkg install <name>
pkg list                 # installed packages
```

Be prepared for disappointment: many prebuilt programs for this kind of
Pi (bash, nano, vim, curl) do not start yet
([known issues](known-issues.md)). The editor in the image is `kibi`
(`kibi file.txt`; Ctrl-S saves, Ctrl-Q quits).

## 8. Shutting down

```sh
reboot                              # as root: restart
```

There is no real power-off yet: run `reboot`, and pull the power once the
screen goes dark or the firmware messages appear again. Pulling the power
while the Pi writes to the card can damage files.

## Logging in over the network

The SD card image has **no remote login**: no SSH yet (planned for v0.7.0).
The network-boot image for developers has `devshd`, an unencrypted remote
shell on port 2323 (`nc <pi-address> 2323`); it is only meant for a home
network. Setting that up needs your own build machine:
[netboot.md](netboot.md).

## When something goes wrong

| What you see | What to do |
|---|---|
| Nothing at all on the screen | Check the card was written completely; try the serial console, which shows the earliest messages. Rainbow square and nothing else: the card has no boot files, write it again. |
| Stops at `U-Boot>` | Type `boot` and Enter. |
| The keyboard does nothing | Plug it in before power-on, then power off and on again. Or log in on the serial console. |
| The serial console shows nothing or garbage | Swap TX and RX; check 115200 baud and that GND is connected. |
| A command stops with no output | A known rare start crash ([#1](https://github.com/redox-rpi/redox-rpi/issues/1)); run it again. |
| No network address | Cable, router; on the Zero 2 W only RTL8152/RTL8153 adapters work. |
| The time is wrong | It needs the network; see [time.md](time.md#problems). |

## Reporting problems

Report problems with this project on
[GitHub issues](https://github.com/redox-rpi/redox-rpi/issues), **not** to
the Redox OS project: this is an unofficial fork, and Redox does not
handle it. Look at [known issues](known-issues.md) first.

A good report says:

- which board (Pi 3B+ or Zero 2 W) and power supply,
- the image's version: the output of `cat /etc/redox-rpi-release`, or the
  line under the login box,
- what you did and what happened,
- the boot messages if you can: from the serial console (`screen -L` saves
  them to `screenlog.0`) or a photo of the screen.

## No Pi? Try it in QEMU

QEMU emulates a Raspberry Pi 3 well enough to boot the same image (without
Ethernet). On Linux or macOS, with `qemu-system-aarch64`, `curl` and `zstd`
installed:

```sh
git clone https://github.com/redox-rpi/redox-rpi.git && cd redox-rpi
xz -dk ~/Downloads/redox-rpi-v0.2.0-pi3bplus.img.xz
scripts/qemu-raspi3b.sh --image ~/Downloads/redox-rpi-v0.2.0-pi3bplus.img
```

The unpacked image is about 1.1 GB. The script downloads a U-Boot for
QEMU the first time. Checked with the v0.2.0 image on Linux: the login
prompt came after about 50 seconds.

Press Enter at the resolution menu, log in as above, and quit QEMU with
**Ctrl-A**, then **X**. More in [testing.md](testing.md).

## Next

- [User guide](user-guide.md): the Ion shell, files and schemes, network,
  `pkg`, logs, and what is missing.
- [Time](time.md) and [smart home](smarthome.md): newer features, on
  branches until they are tested.
- [Known issues](known-issues.md) and the [roadmap](../ROADMAP.md).
- Want to help? [CONTRIBUTING.md](../CONTRIBUTING.md) and the
  [open issues](https://github.com/redox-rpi/redox-rpi/issues).
