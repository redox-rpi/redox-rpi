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

Candidates are named `v<version>-rc<N>`, for example `v0.3.0-rc2`. `N`
only counts up: a rebuild with any change, even one experiment more or
less, gets the next number, and a number is never reused, also when its
candidate failed. The release itself has no suffix (see
[versioning.md](versioning.md#release-names)).

## Building

- The build agent builds in its own clone `~/redox-rpi-rel` on homenodellm,
  never in the shared clone `~/redox-rpi`.
- It checks out the requested commit on a branch `release-v<version>` and
  runs `scripts/build-image.sh` with the requested options. No
  `--allow-dirty`: a candidate must be reproducible from its manifest.
- The build waits for other builds on the host (`build/.build-image.lock`).
- `devshd` stays out of candidate and release images.
- The build ID (`git describe` plus the experiment hash) is recorded with
  the candidate's name, so `v0.3.0-rc2` can be traced to an exact build.

## Results

Each candidate goes to `/srv/tftp/sdcard/` on homenodellm, never to
anyone's own computer:

| File | Content |
|---|---|
| `redox-rpi-v<version>-rc<N>.img.xz` | the image, from `scripts/make-sdcard.sh --local` |
| `redox-rpi-v<version>-rc<N>.img.manifest.toml` | its manifest |
| `SHA256SUMS` | one line per file in the directory |

The image is offered at
`http://homenodellm:8080/sdcard/redox-rpi-v<version>-rc<N>.img.xz`, so
SwiftFlash or `curl` flashes it straight from there
([netboot.md](netboot.md#sd-card-images-for-download)). Network-boot
candidates are staged with `scripts/netboot.sh` only by the test that holds
the Pi.

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

A candidate that passed its Pi test becomes the release, unchanged: the
release image is the candidate's image, not a new build. Each step below
happens only after the user approves it in the build agent's session.

1. **Release notes** in `docs/releases/v<version>.md`, like
   [v0.2.0](releases/v0.2.0.md): status line, what is new, what works, known
   issues, install steps, and "Built from" with the build ID and
   experiments.
2. **Merge** the `release-v<version>` branch into `main`.
3. **Tag** `v<version>` on the merge commit and push the tag.
4. **GitHub pre-release** `v<version>` (while the status is experimental)
   with the notes as its text and three files:
   `redox-rpi-v<version>-pi3bplus.img.xz`,
   `redox-rpi-v<version>-pi3bplus.img.manifest.toml` and `SHA256SUMS`
   (the candidate's files, renamed, with the sums computed again).
5. Update the README and `ROADMAP.md` where they name the current release.
