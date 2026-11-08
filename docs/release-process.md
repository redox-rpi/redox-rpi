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
([PMP](process/pmp.md#sprints-and-weekly-releases)). The version says which
week it is and whether a roadmap theme was completed:

- **Each weekly release raises the patch number:** v0.4.0, v0.4.1, v0.4.2,
  … It may carry features and fixes alike; until 1.0 the patch number does
  not promise "fixes only".
- **The minor number goes up when a roadmap theme is complete**
  ([ROADMAP.md](../ROADMAP.md)), and the patch starts at 0 again: v0.4.3 →
  v0.5.0. The project lead proposes it at sprint planning, the maintainer
  decides, at the latest at the cut-off; the week's milestone is renamed
  then. The theme is named in the release notes.
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
candidate failed. A candidate built before the Friday is allowed; the
Friday candidate from `main` is the one that is tested for the release.
The release itself has no suffix (see
[versioning.md](versioning.md#release-names)).

## Weekly schedule

| When | Release and CI does |
|---|---|
| Monday | Opens the checklist `docs/releases/v<version>-checklist.md` from the previous one and the release issue |
| Monday to Thursday | Builds early candidates when asked, keeps the checklist's "in the image" table current with what is merged |
| Thursday evening | Cut-off: only what the maintainer has merged by then is in |
| Friday | Builds the candidate from `main` ([Building](#building)), runs the [candidate checks](#candidate-checks), queues the Pi rows and the regression set, drafts the release notes |
| Weekend | The person test, if one is due, on the Friday candidate; the tag and the GitHub release after the maintainer's OK on the exact commit ([Release](#release)) |

A release never waits for a feature. If the Friday candidate fails and
neither a fix nor taking the change out works by Sunday, the week has no
release.

## Building

- The build agent builds in its own clone `~/redox-rpi-rel` on homenodellm,
  never in the shared clone `~/redox-rpi`.
- It checks out the requested commit on a branch `release-v<version>` and
  runs `scripts/build-image.sh --release v<version>-rc<N>` with the
  requested options. No `--allow-dirty`: a candidate must be reproducible
  from its manifest.
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
| `SHA256SUMS` | one line per file in the directory |

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
   [test-plan.md](test-plan.md): name and build ID, the experiments, the
   download link, the test from the request, and who runs it.
2. The test follows the rules of the test plan: it takes the Pi only when
   it is first in the queue, and gives it back afterwards.
3. The results go to [test-reports.md](test-reports.md) with the build ID,
   and the test moves to "Done" with its result.
4. A failed candidate stays in "Done" with what failed. The fix comes as a
   new request and a new `rc<N+1>`.

## Release

A candidate that passed its Pi test becomes the release. The release image
is built again from the candidate's commit with the same options, only with
`--release v<version>` instead of `--release v<version>-rc<N>`, so it gets
the same build ID. Before it is published, it boots in QEMU and once on
the Pi, and its manifest must list the same patches and package hashes as
the candidate's. Each step below happens only after the user approves it in
the build agent's session.

1. **Release notes** in `docs/releases/v<version>.md`, like
   [v0.2.0](releases/v0.2.0.md): status line, what is new, what works, known
   issues, install steps, and "Built from" with the build ID and
   experiments. "What is new" may link the closed issues of the release's
   [milestone](https://github.com/redox-rpi/redox-rpi/milestones) (v0.3.0 is
   milestone 1).
2. **Merge** the `release-v<version>` branch into `main`. Only maintainers
   merge ([CONTRIBUTING.md](../CONTRIBUTING.md#merging)): the build agent
   does it as the maintainer account, after the user's approval.
3. **Tag** `v<version>` on the merge commit and push the tag.
4. **GitHub pre-release** `v<version>` (while the status is experimental)
   with the notes as its text and three files:
   `redox-rpi-v<version>-pi3bplus.img.xz`,
   `redox-rpi-v<version>-pi3bplus.img.manifest.toml` and `SHA256SUMS`
   (from the release build).
5. Update the README and `ROADMAP.md` where they name the current release.
   **Close the milestone** `v<version>`; issues still open in it move to
   the next milestone or lose theirs.
6. **Development tag:** right after a release `vX.Y.Z` is tagged, tag the
   next commit on `main` as `vX.Y.(Z+1)-dev` (annotated) and push it, so
   development builds show `vX.Y.(Z+1)-dev-N-g…`
   ([versioning.md](versioning.md#development-tags)). Tell the other
   sessions to `git fetch --tags`.
