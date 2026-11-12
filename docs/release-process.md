# Release process

One build agent session builds all release candidates and releases, and
only those. Feature sessions keep doing their own development builds. This
page says how to ask for a candidate and what happens until the release.

## Requesting a candidate

Send the build agent a request that names:

- the **branch or commit** to build (a branch is pinned to its commit when
  the build starts);
- the **experiments**, in order (`--experiment <name>` ...);
- the **image type**: SD card (`--sdcard`), network boot, or Zero 2 W
  network boot (`--zero2w`);
- the **test to run** on the Pi: what to check and what counts as a pass.

Everything a candidate needs must be committed and pushed. The build agent
does not build uncommitted work or edit feature branches.

## Numbering

From v0.4.0 on there is one release per weekly sprint
([PMP](process/pmp.md#sprints-and-weekly-releases)). The version says
whether the week brought something new (the maintainer, 2026-11-08):

- **A release with at least one new feature raises the minor number:**
  v0.4.1 → v0.5.0. A feature is a closed issue of type Feature in the
  release's milestone.
- **A release with only fixes raises the patch number:** v0.4.0 → v0.4.1.
- **After v0.9 comes v0.10**, not v1.0. Version 1.0 is a separate decision
  with its own criteria (an RFC by Requirements).
- **Who decides:** the project lead proposes the number at sprint planning,
  the maintainer confirms it. If a feature lands or drops out by the
  cut-off, the number is fixed at the cut-off, and the week's milestone is
  renamed then (for example `v0.4.1` → `v0.5.0`).
- **Example:** after v0.4.0 the week's milestone started as `v0.4.1`. On
  2026-11-09 the maintainer chose v0.5.0, because the week brings
  features (the game on the desktop, TinyCC, the startup sound); the
  milestone was renamed to `v0.5.0` and the dev tag `v0.5.0-dev` set on
  `main`. The tag `v0.4.1-dev` stays, so builds named `v0.4.1-dev-…`
  remain traceable.
- **A week without a release** (failed candidate, nothing merged) does not
  use up a number: the next week takes it.
- **A fix between two weekly releases** that cannot wait gets the next
  patch number; the weekly release that follows takes the one after.
- **Milestones** are named after the release (`v0.4.1`). The project lead
  creates next week's milestone at planning; open issues of a closed
  milestone move to the next one.

Candidates are named `v<version>-rc<N>`, for example `v0.4.1-rc2`. `N`
only counts up: a rebuild with any change, even one experiment more or
less, gets the next number, and a number is never reused, also when its
candidate failed. A candidate built before the Saturday is allowed; the
Saturday candidate from `main` is the one that is tested for the release.
The release itself has no suffix (see
[versioning.md](versioning.md#release-names)).

## Weekly schedule

| When | Release and CI does |
|---|---|
| Monday, after the release | Opens the checklist `docs/releases/v<version>-checklist.md` from the previous one and the release issue |
| Monday to Friday | Builds early candidates when asked, keeps the checklist's "in the image" table current with what is merged |
| Friday evening | Cut-off: only what the maintainer has merged by then is in |
| Saturday | Builds the candidate from `main` ([Building](#building)), runs the [candidate checks](#candidate-checks), queues the Pi rows and the regression set, drafts the release notes |
| Sunday evening | The person test, if one is due, on the Saturday candidate. The maintainer's OK on the exact commit closes the release and the sprint: the tag `v<version>` and the next development tag at once; the GitHub release follows ([Release](#release)). Monday evening is the fallback |

A release never waits for a feature. If the Saturday candidate fails and
neither a fix nor taking the change out works by the Monday evening
fallback, the week has no release.

## Building

- The build agent builds in its own clone `~/redox-rpi-rel` on homenodellm,
  never in the shared clone `~/redox-rpi`.
- It checks out the requested commit on a branch `release-v<version>` and
  runs `scripts/build-image.sh --release v<version>-rc<N>` with the
  requested options. No `--allow-dirty`: a candidate must be reproducible
  from its manifest.
- `--release` leaves out the test experiments of
  `config/test-experiments.conf` (diagnostics such as `netstack-watch`)
  and refuses any experiment with a `test-only` file, so a candidate never
  has them; its manifest shows the experiments it has.
- `--release` puts the candidate's name above the login prompt, in
  `/etc/redox-rpi-release` and in the manifest
  ([versioning.md](versioning.md#release-name-in-the-image)), so a booted
  Pi shows which candidate it runs. The first candidate with it is
  v0.3.0-rc2.
- The build waits for other builds on the host (`build/.build-image.lock`).
- `devshd` stays out of candidate and release images.
- The build ID (`git describe` plus the experiment hash) is shown next to
  the candidate's name, so `v0.3.0-rc2` can be traced to an exact build.

## Results

Each candidate goes to `/srv/tftp/sdcard/` on homenodellm, never to
anyone's own computer:

| File | Content |
|---|---|
| `redox-rpi-v<version>-rc<N>.img.xz` | the image, from `scripts/make-sdcard.sh --local`, published with `scripts/publish-sdcard.sh <card> --family redox-rpi --version v<version>-rc<N> --manifest <manifest>` ([ci.md](topics/ci.md#image-server)) |
| `redox-rpi-v<version>-rc<N>.img.xz.sha256`, `.manifest.toml` | its checksum and manifest, written by `publish-sdcard.sh` |
| `index.json` | the list SwiftFlash reads, rewritten by `publish-sdcard.sh` (`scripts/sdcard-index.py`); there is no `SHA256SUMS` in `/srv/tftp/sdcard/`, each image has its own `.sha256` |

The image is offered at
`http://homenodellm:8080/sdcard/redox-rpi-v<version>-rc<N>.img.xz`, so
SwiftFlash or `curl` flashes it straight from there
([netboot.md](netboot.md#sd-card-images-for-download)). Network-boot
candidates are staged with `scripts/netboot.sh` only by the test that holds
the Pi.

## Candidate checks

The build agent runs these on every candidate's published SD image and
writes the results into the release checklist next to the checksum. They
come from the requirements REQ-REL-1 and REQ-REL-5 (`docs/requirements/release.md`,
draft [PR #270](https://github.com/redox-rpi/redox-rpi/pull/270); method
agreed with V&V on 2026-11-08).

- **REQ-REL-1, no remote shell.** The automated Pi rows run the netboot
  image, which has `devshd`, so only these checks look at the SD image
  itself:
  1. Inspection by the build agent: `redoxfs-file` reads only GPT images,
     so check that the card's partition 2 is a byte copy of partition 3
     of the build's `harddrive.img` (whose hash the manifest lists), then
     `redoxfs-file harddrive.img get /usr/bin/devshd <file>` must fail with
     "No such file or directory", and the string `devshd` must not occur
     anywhere in the card's partition 2, so no init service file names
     it. A control file such as `/usr/bin/ion` must be found.
  2. In the visit, on the booted SD image: `nc -z -w 3 <pi> 2323` from the
     build host fails, and `ps` on the console shows no `devshd`.
- **Green CI on the commit ([#257](https://github.com/redox-rpi/redox-rpi/issues/257)).**
  A candidate is built only from a commit whose `QEMU boot test` run on
  main passed; the checklist names that run (its URL). While the issue
  "CI: main is red" is open (opened and closed by
  `.github/workflows/ci-main-red.yml`), no candidate is built.
- **REQ-REL-5, fits a 2 GB card:** the uncompressed image
  (`xz -dc <image>.img.xz | wc -c`) is at most 1 800 000 000 bytes.

```bash
# on homenodellm, in ~/redox-rpi-rel after the candidate build
img=/srv/tftp/sdcard/redox-rpi-v<version>-rc<N>.img.xz
hd=upstream/redox/build/aarch64/sdcard/harddrive.img
tool=tools/redoxfs-file/target/release/redoxfs-file
d=$(mktemp -d); xz -dc "$img" > $d/card.img
stat -c %s $d/card.img                                  # REQ-REL-5: <= 1800000000
read cs cn < <(partx -g -o START,SECTORS -n 2 $d/card.img)
read hs hn < <(partx -g -o START,SECTORS -n 3 $hd)
cmp <(dd if=$d/card.img bs=512 skip=$cs count=$hn status=none) \
    <(dd if=$hd bs=512 skip=$hs count=$hn status=none)  # same root filesystem
$tool $hd get /usr/bin/devshd $d/x                      # REQ-REL-1: must fail
$tool $hd get /usr/bin/ion $d/ion                       # control: must work
dd if=$d/card.img bs=512 skip=$cs count=$cn status=none | grep -c -a devshd   # must print 0
rm -rf $d
```

## Pi test

1. The build agent adds the candidate to the queue in
   [pi-tests queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md): name and build ID, the experiments, the
   download link, the test from the request, and who runs it.
2. The test follows the rules of the queue: it takes the Pi only when
   it is first in the queue, and gives it back afterwards.
3. The results go to [pi-tests reports/](https://github.com/redox-rpi/pi-tests/tree/main/reports) with the build ID,
   and the test moves to "Done" with its result. The evidence of the
   release is collected in `releases/<version>.md` there.
4. A failed candidate stays in "Done" with what failed. The fix comes as a
   new request and a new `rc<N+1>`.

## Release

A candidate that passed its Pi test becomes the release. **The maintainer's
OK on the exact commit closes the release and the sprint** (the maintainer,
2026-11-09). From that moment the next version counts: the tags come first,
everything else follows and holds nothing up. The OK is given in the build
agent's session.

**At the OK, right away:**

1. **Tag** `v<version>` (annotated) on the approved commit and push it.
2. **Development tag:** tag the newest commit on `main` as
   `vX.Y.(Z+1)-dev` (annotated) and push it, so the nightly and every other
   build from then on show `vX.Y.(Z+1)-dev-N-g…`
   ([versioning.md](versioning.md#development-tags)). Tell the other
   sessions to `git fetch --tags`.

**Afterwards, in this order, without holding up the next sprint:**

3. **Release image:** built again from the approved commit with the same
   options, only with `--release v<version>` instead of
   `--release v<version>-rc<N>`, so it gets the same build ID. Its manifest
   must list the same patches and package hashes as the candidate's; a
   difference is named in the release notes. It boots in QEMU and the
   [candidate checks](#candidate-checks) run on it.
4. **Release notes** in `docs/releases/v<version>.md`, like
   [v0.4.0](releases/v0.4.0.md): status line, what is new, known issues,
   install steps, and "Built from" with the build ID, experiments and
   checksum. "What is new" may link the closed issues of the release's
   [milestone](https://github.com/redox-rpi/redox-rpi/milestones).
5. **GitHub release** `v<version>` with the notes as its text and three
   files from the release build: `redox-rpi-v<version>-pi3bplus.img.xz`,
   `redox-rpi-v<version>-pi3bplus.img.manifest.toml` and `SHA256SUMS`.
6. **One boot on the Pi** of the release image (the netboot image of the
   same build), queued for the Pi test agent. If it fails, the release
   notes and the GitHub release get a warning, and the fix goes into the
   next release.
7. **Close the milestone** `v<version>`; issues still open in it move to
   the next milestone. The release issue is closed with the release link.
8. The `release-v<version>` branch (checklist, candidate notes, release
   notes) reaches `main` by a docs pull request. The README, `ROADMAP.md`
   and the profile README name the new release.
