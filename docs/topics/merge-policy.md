# Who merges what

In force since 2026-11-13, decided by the maintainer. The goal: pull
requests that only change how we build and test do not wait for the
maintainer. Everything that ends up in the image still does.

## The Quality session merges build and test system changes

The Quality session may merge a pull request into `main` when **all** of
these hold.

**Scope.** The pull request changes only these:
- CI workflows (`.github/`).
- `build-image.sh` and the other build, staging and test scripts (`scripts/`).
- The regression set and test definitions.
- The test plan and docs (`docs/`).

It changes nothing that ends up in an image: no kernel, relibc, driver or
recipe patch (`patches/`), no recipe, no `upstream.lock`, no experiment's
code. A pull request that touches both goes to the maintainer.

**Not its own.** Quality never merges a pull request it wrote or pushed
to. Those stay with the maintainer.

**Checks:**
1. CI is green.
2. `merge-set-check` passes.
3. The base is `main`, not a stacked branch.
4. If the change affects how images are built or tested, a QEMU run or a
   Pi row passed on that head commit.
5. Quality writes a short merge comment that names the criterion it
   merged under.

**Way back.** If `main` or the nightly goes red after such a merge,
Quality reverts it at once (a revert pull request, merged under the same
rule) and tells the lead.

## Stays with the maintainer

- Every pull request that changes what ends up in an image.
- Pull requests by the Quality session.
- Releases, tags and release candidates.
- RFCs.

Docs-only changes may still go straight to `main` as a draft, and the lead
may merge docs-only pull requests. Both are unchanged.
