# CI and tests

## State

- GitHub Actions boot the kernel in QEMU `raspi3b` on every push and run
  relibc's `sigregs` on Linux.
- Tests on the real Pi go through a shared queue,
  [test-plan.md](../test-plan.md), and `scripts/pi-test.py`; results are in
  [test-reports.md](../test-reports.md).
- Every night at 03:30 UTC the build host builds `main`, tests it on the Pi
  with `scripts/pi-test.py` and makes it the stable network-boot image if it
  passes; a failure goes to the issue "Nightly: main fails on the Pi"
  ([test-plan.md](../test-plan.md#nightly-update)). The Pi may reboot at
  night.
- After it, the images in `config/visit-images.conf` for tests with a
  person at the Pi are built on the build host, never on the Pi
  ([below](#nightly-visit-images)).
- `scripts/pi-test.py --serial-log --crash-log` records the crash output of
  every failed start (#10). `scripts/qemu-raspi3b.sh --data` gives QEMU a
  data partition, so all standard checks also run in QEMU.
- `scripts/pr-order.py` lists the open pull requests in an order they can be
  merged in, with CI, Pi test, patch numbers and conflicts.
- The "Topic pages" job warns when a pull request or a push to main
  changes a topic's files without touching its page here.
  `config/topics.map` maps paths to topics. It is a warning only.
- Every Monday the "Upstream drift" job applies every patch, experiments
  included, on the pinned and on the newest upstream commits
  (`scripts/check-upstream-drift.py`) and updates the issue "Upstream drift
  report": per patch applies, 3-way, already upstream or conflicts, with the
  upstream commits behind each conflict. It never fails.

## In progress

- [#10](https://github.com/redox-rpi/redox-rpi/issues/10) crash details in `pi-test.py`, branch `t10-start-loop`,
  [PR #74](https://github.com/redox-rpi/redox-rpi/pull/74): each failed start with its program, exit
  status and its own kernel crash dump (FAR, ELR, ESR, LR, SP, PID, stack
  walk); `--crash-log` writes them as JSON lines for counting start crashes
  (#1); starts Ion could not run count as failures. Pi tests #48 and #92
  passed (#92: 1000 starts on the stable image, 6 failures, 6 records).

- [#200](https://github.com/redox-rpi/redox-rpi/issues/200) log triage with the local model, branch `log-triage`
  ([below](#log-triage)).

## Next

- [#9](https://github.com/redox-rpi/redox-rpi/issues/9) CI with the pinned revisions; [#13](https://github.com/redox-rpi/redox-rpi/issues/13) relibc tests on
  Redox.

## Nightly visit images

[#235](https://github.com/redox-rpi/redox-rpi/issues/235). A test with a
person at the Pi should never wait for a build, so the images for such
tests are built every night.

- `update-stable.sh` starts the systemd user unit `redox-visit.service`
  ([config/redox-visit.service](../../config/redox-visit.service)) when the
  stable image passed or nothing changed since it; not when it failed or a
  test was on the Pi. The unit runs
  [`scripts/build-visit-images.sh`](../../scripts/build-visit-images.sh)
  from `~/redox-rpi-stable`, a unit of its own so the nightly ends as before.
- It builds in its own clone `~/redox-rpi-visit`: the stable image's commit,
  or the newest `origin/<branch>` for an image with `ref=`. It never touches
  the Pi, the serial console, `/srv/tftp/redox` or the stable image, and
  makes no release and no package repository.
- An image whose newest build has the same commit and options is not built
  again. Builds still running at the deadline in the config (07:00 UTC)
  are stopped.
- Results: `/srv/tftp/visit/<name>/<build ID>/` (`redox/` with the
  network-boot files of `scripts/netboot.sh` and `manifest.toml`, to copy to
  `/srv/tftp/redox/`; `BUILD_ID`; `visit.txt` with ref, commit and
  options), the link
  `/srv/tftp/visit/<name>/latest`, for `--sdcard` also
  `/srv/tftp/sdcard/redox-rpi-visit-<name>-<build ID>.img.xz` with
  `.sha256`, `.manifest.toml` and `-latest` links. The last 3 stay.
- **Desktop smoke test** ([#261](https://github.com/redox-rpi/redox-rpi/issues/261)),
  for an image with `check=desktop` (today `desktop`): after its network-boot
  build, [`scripts/qemu-desktop-check.sh`](../../scripts/qemu-desktop-check.sh)
  boots it in QEMU raspi3b, runs `desktop` on the serial console, presses
  Enter at Orbital's login window through the QEMU monitor and checks: no
  panic, `orbital` and the launcher still running a minute later, and a
  screenshot that shows the launcher bar (many colours, not the dark text
  console). Report-only: the image counts as built whatever it says. The
  result is the `desktop-check =` line in `visit.txt`; `report.md`, `run.log`,
  `desktop.png` and `console.png` are in
  `/srv/tftp/visit/<name>/<build ID>/desktop-check/`. QEMU's framebuffer is
  the firmware mailbox model, so it checks Orbital, `rpi-fbd`, the launcher
  and the session, not HDMI. It also runs by hand on any desktop image:
  `scripts/qemu-desktop-check.sh --image <harddrive.img>`.
- **For a tester:** http://homenodellm:8080/visit/ (`index.html` and
  `README.txt`, from `scripts/visit-index.py` after each image): per image
  OK or FAILED, build ID, date, the command to stage it, the SD image with
  its sha256 or why there is none, and the queued rows of
  [test-plan.md](../test-plan.md) whose Image column names
  "nightly visit `<name>`", with what they check.
- `/srv/tftp/visit/status.txt` has one line per image: built, unchanged,
  failed or deadline, with build ID, commit and ref. Log:
  `~/redox-rpi/logs/visit/<date>.log`. A failure does not touch the nightly.
- **Adding an image:** a line in the config with name, owner topic and the
  `scripts/build-image.sh` options, `ref=<branch>` (a branch of this
  repository) while its experiments are not on main (drop it at the
  merge), `clean=recipes/<path>` for a recipe under `upstream/redox/` to
  remove first, and `--sdcard` where the image boots from the card; if it
  can't, a comment at the end of the line says why (shown on the page). How a test row uses it:
  [test-plan.md](../test-plan.md#nightly-visit-images).
- Install once on the build host:
  `cp config/redox-visit.service ~/.config/systemd/user/ && systemctl --user daemon-reload`.

## Image server

SD card images for people are at http://homenodellm:8080/sdcard/
(`/srv/tftp/sdcard/`, served by `redox-http.service`, `python3 -m
http.server` on `/srv/tftp`), laid out for SwiftFlash
([build-server.md](https://github.com/zehrer/SwiftFlashCLI/blob/main/docs/build-server.md),
[#237](https://github.com/redox-rpi/redox-rpi/issues/237)). A tester:

```bash
swiftflash source add redox http://homenodellm:8080/sdcard/
sudo swiftflash redox:redox-rpi-openfw-3bp
```

**How to publish:** every SD image goes in with
`scripts/publish-sdcard.sh <card.img or card.img.xz> --family <family>
[--manifest build/manifests/<image>.toml] [--owner <topic>]`, never by
copying into the directory. It takes a card from `scripts/make-sdcard.sh` or
any finished card image (firmware-only cards too), and prints the URL, the
sha256 and the SwiftFlash command for the test row.

The rules it keeps:

- **Names** `<family>-<version>.img.xz`. The family is the image variant
  (`redox-rpi-<topic>-<variant>`, lower case, no version in it) and the
  version starts with `v<major>.<minor>` (the build ID from `--manifest`
  or `--version`) or a date `YYYYMMDD`. Without a manifest or `--version`
  it takes today's date, then `b`, `c`, … for more builds that day.
- **Never overwrite:** a name that is published already is refused; a new
  build gets a new name. Deleting old builds is fine.
- **Checksum** `<image>.sha256` in `sha256sum` format, of the `.xz`.
- **`/.uuid`** on the card's FAT partition: `schemaVersion` 1 and only the
  image block (`id` = family, `version` = the version in the file name,
  `build`, `commit`), never a device or flash UUID. Cards without a FAT
  first partition need `--no-uuid`.
- **`index.json`**, rewritten atomically after each image
  (`scripts/sdcard-index.py`, safe to run by hand): per image file
  `family`, `version` and `date`, plus our own `board`, `commit`,
  `build_id`, `experiments`, `owner` and `rows` (the queued test-plan rows
  that name the family or file). The user unit `redox-sdcard-index.path`
  ([config/redox-sdcard-index.path](../../config/redox-sdcard-index.path))
  rewrites it whenever the directory changes, so a file copied in by hand
  is not hidden: SwiftFlash reads only `index.json` when there is one. Older images get their fields from the
  file name and manifest; `redox-rpi-zero2w-fastnet-db5ea27` has version
  `db5ea27`.
- **`-latest` links** (`--latest`, used by the visit images) are for
  people. `index.json` leaves them out, as SwiftFlash finds the newest build
  itself; `http.server` gives a link the size and Last-Modified of its
  target, so a new target is a new download.
- **ETag:** `http.server` sends none; SwiftFlash then uses Last-Modified
  plus size. `scripts/http-etag-server.py` is the same server with an ETag
  (`"<mtime>-<size>"`), tested on a spare port, not in use: switching
  `redox-http.service` to it (it also serves U-Boot's HTTP boot) is the
  maintainer's call.

## Log triage

homenodellm runs vLLM with `gpt-oss-20b` (unit `vllm.service`, config
`/etc/vllm/vllm.env`, `http://127.0.0.1:8000/v1`, 16k context, about
43 tokens/s). It listens on 0.0.0.0 without an API key, so anyone on the
home network can use it.

- `tools/log-triage/log-triage.py LOG [--baseline GOOD]` pre-sorts a serial or
  QEMU log: per boot, reached login, panics and faults with line and program,
  failed daemons, first error, hang, lines new against a good log. Markdown
  and JSON; examples in `tools/log-triage/examples/`. Details in its README.
- Regexes find the facts and pick the lines; the model only reads boots that
  did not end clean, about 100–200 lines each (1k–6k tokens).
- Speed: one boot 2–25 s (mostly 5–15 s). The day log
  `serial-2026-11-01T0317.log.gz` (120k lines, 135 boots, 14 not clean) takes
  52 s with 4 boots in parallel; `--no-model` takes 6 s.
- Accuracy on 17 boots checked by hand (a good boot, the netstack hang of
  test 229, a QEMU fault, and the 14 not-clean boots of 2026-11-01):
  reached login 17/17; panics and faults with the right line and program
  16/17 (the sigregs test's 1000 handled faults were blamed on the kernel);
  hangs and boots that stopped early 5/5, no false hang; failed daemons
  right where named. The one-line summary had a wrong cause in 7 of 17
  ("failed to mount, leading to a crash in /tmp/r117"). **Use the facts and
  line numbers, not the reasons.**
- OpenCode 1.19.5 for small agent tasks on homenodellm:
  `~/.opencode/bin/opencode` (official script, not on PATH), provider
  `vllm/gpt-oss-20b` in `~/.config/opencode/opencode.json`, autoupdate and
  sharing off. `opencode run "…"` in a folder works with tool calls (glob,
  read): a list-and-read task took 6 s.
