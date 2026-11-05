#!/usr/bin/env python3
"""Try every patch on the pinned and on the newest upstream commits.

Usage:
    scripts/check-upstream-drift.py [--work <dir>] [--out <file.md>]
                                    [--json <file.json>] [--no-fetch]

For each patched component (patches/<name>/, plus the components of the
experiments) it fetches the upstream repository, then applies our patches
one by one, in order, on the commit pinned in config/upstream.lock and on
the newest commit of the upstream default branch. Each patch gets one
result:

  applies    applies as it is
  3-way      applies only with a 3-way merge (context moved; refresh it)
  upstream   already upstream (the reverse patch applies); drop it
  conflicts  does not apply; the files and the upstream commits since the
             pin that touch them are listed

A patch that conflicts is left out, and the next ones are tried without
it, so one conflict does not hide the rest. Experiments are tried on top of
the default patches, with the experiments they need first (needs() in
scripts/check-patches.sh). scripts/check-patches.sh stops at the first
patch that fails and has no 3-way merge; this script is the full report.

Prints a Markdown report (or writes it to --out). Exit status 0 even when
patches conflict: drift is a report, not a failure. 2 on a setup error.
Needs git only; builds nothing.
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPOS = {
    "kernel": "https://gitlab.redox-os.org/redox-os/kernel.git",
    "relibc": "https://gitlab.redox-os.org/redox-os/relibc.git",
    "base": "https://gitlab.redox-os.org/redox-os/base.git",
    "bootloader": "https://gitlab.redox-os.org/redox-os/bootloader.git",
    "redoxfs": "https://gitlab.redox-os.org/redox-os/redoxfs.git",
    "pkgutils": "https://gitlab.redox-os.org/redox-os/pkgutils.git",
    "fdt": "https://github.com/repnop/fdt.git",
}
ID = ["-c", "user.name=drift", "-c", "user.email=drift@localhost"]
PLUS = re.compile(r"^\+\+\+ b/(\S+)", re.M)


def git(repo, *args, check=False):
    p = subprocess.run(["git", *ID, "-C", str(repo), *args], capture_output=True, text=True)
    if check and p.returncode:
        sys.exit(f"git -C {repo} {' '.join(args)}: {p.stderr.strip()}")
    return p


def pins():
    out = {}
    for line in (ROOT / "config/upstream.lock").read_text().splitlines():
        f = line.split()
        if len(f) == 2 and not line.startswith("#"):
            out[f[0]] = f[1]
    return out


def needs():
    """needs() of scripts/check-patches.sh as {experiment: [experiments]}."""
    text = (ROOT / "scripts/check-patches.sh").read_text()
    body = re.search(r"^needs\(\) \{\n(.*?)^\}", text, re.M | re.S)
    return {m.group(1): m.group(2).split()
            for m in re.finditer(r"^\s*([\w-]+)\) echo ([^;]+) ;;", body.group(1) if body else "", re.M)}


def subject(patch):
    m = re.search(r"^Subject: (?:\[PATCH[^\]]*\] )?(.+(?:\n .+)*)", patch.read_text(errors="replace"), re.M)
    return re.sub(r"\n ", " ", m.group(1)) if m else patch.name


def try_patch(repo, patch, pin):
    """Apply one patch to the checkout and commit it; return its result."""
    r = {"patch": str(patch.relative_to(ROOT / "patches")), "subject": subject(patch)}
    if git(repo, "apply", "--check", str(patch)).returncode == 0:
        git(repo, "apply", "--index", str(patch), check=True)
        r["result"] = "applies"
    elif git(repo, "apply", "-R", "--check", str(patch)).returncode == 0:
        r["result"] = "upstream"
        return r
    elif git(repo, "apply", "--3way", "--index", str(patch)).returncode == 0:
        r["result"] = "3-way"
    else:
        files = git(repo, "diff", "--name-only", "--diff-filter=U").stdout.split()
        files = files or sorted(set(PLUS.findall(patch.read_text(errors="replace"))))
        git(repo, "reset", "-q", "--hard")
        git(repo, "clean", "-qfd")
        log = git(repo, "log", "--format=%h %cs %s", f"{pin}..HEAD", "--", *files).stdout
        r.update(result="conflicts", files=files, commits=log.strip().splitlines()[:5])
        return r
    git(repo, "commit", "-q", "--no-verify", "-m", r["patch"], check=True)
    return r


def series(repo, start, patches, pin):
    git(repo, "checkout", "-q", "-f", "--detach", start, check=True)
    git(repo, "clean", "-qfd")
    return [try_patch(repo, p, pin) for p in patches]


def component(name, work, pin, fetch, need):
    repo = work / name
    if not (repo / ".git").exists():
        git(work, "init", "-q", name, check=True)
        git(repo, "remote", "add", "origin", REPOS[name], check=True)
    if fetch:
        # Blobless: history for the log, file contents only when a patch needs them.
        git(repo, "fetch", "-q", "--filter=blob:none", "origin", "HEAD", check=True)
        git(repo, "update-ref", "refs/drift/head", "FETCH_HEAD", check=True)
        if git(repo, "cat-file", "-e", f"{pin}^{{commit}}").returncode:
            git(repo, "fetch", "-q", "--filter=blob:none", "origin", pin, check=True)
    head = git(repo, "rev-parse", "refs/drift/head", check=True).stdout.strip()
    defaults = sorted((ROOT / "patches" / name).glob("*.patch"))
    comp = {"pin": pin, "head": head,
            "behind": int(git(repo, "rev-list", "--count", f"{pin}..{head}").stdout or 0),
            "pinned": series(repo, pin, defaults, pin)}
    pinned_tip = git(repo, "rev-parse", "HEAD").stdout.strip()
    comp["latest"] = series(repo, head, defaults, pin)
    latest_tip = git(repo, "rev-parse", "HEAD").stdout.strip()
    comp["experiments"] = {}
    for d in sorted((ROOT / "patches/experiments").glob(f"*/{name}")):
        exp = d.parent.name
        files = []
        for dep in need.get(exp, []):
            files += sorted((ROOT / "patches/experiments" / dep / name).glob("*.patch"))
        files += sorted(d.glob("*.patch"))
        if files:
            comp["experiments"][exp] = {"pinned": series(repo, pinned_tip, files, pin),
                                        "latest": series(repo, latest_tip, files, pin)}
    return comp


def counts(results):
    c = {}
    for r in results:
        c[r["result"]] = c.get(r["result"], 0) + 1
    return ", ".join(f"{n} {k}" for k, n in sorted(c.items())) or "no patches"


def report(data):
    out = ["### Our patches on the upstream heads", ""]
    problems = 0
    out += ["| Component | Pinned | Head | Behind | On the pin | On the head |",
            "|---|---|---|---|---|---|"]
    for name, c in data.items():
        out.append(f"| {name} | `{c['pin'][:8]}` | `{c['head'][:8]}` | {c['behind']} | "
                   f"{counts(c['pinned'])} | {counts(c['latest'])} |")
    out.append("")
    for name, c in data.items():
        rows = []
        for r in c["latest"]:
            if r["result"] != "applies":
                rows.append(("default", r))
        for exp, e in c["experiments"].items():
            for r in e["latest"]:
                if r["result"] != "applies":
                    pinned = {p["patch"]: p["result"] for p in e["pinned"]}
                    r = dict(r, at_pin=pinned.get(r["patch"]))
                    rows.append((f"experiment `{exp}`", r))
        if not rows:
            continue
        out += [f"#### {name}", "", "| Patch | Set | Result | Why |", "|---|---|---|---|"]
        for where, r in rows:
            why = ""
            if r["result"] == "conflicts":
                problems += 1
                why = "files " + ", ".join(f"`{f}`" for f in r["files"][:4])
                if r.get("at_pin") == "conflicts":
                    why += "; conflicts on the pin too (needs another experiment first?)"
                elif r["commits"]:
                    why += "; upstream: " + "; ".join(r["commits"][:3])
            elif r["result"] == "3-way":
                why = "context moved upstream; refresh the patch"
            elif r["result"] == "upstream":
                why = "the change is upstream; drop the patch at the next lock move"
            name_ = r["patch"].rsplit("/", 1)[-1].removesuffix(".patch")[:56]
            out.append(f"| `{name_}` | {where} | {r['result']} | {why} |")
        out.append("")
    head = (f"**{problems} patch(es) conflict on the newest upstream.**" if problems
            else "**Every patch applies to the newest upstream** (some may need a 3-way merge).")
    return "\n".join([head, ""] + out), problems


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--work", default=str(ROOT / "build/upstream-drift"))
    ap.add_argument("--out", help="write the Markdown report here")
    ap.add_argument("--json", help="write the full results as JSON here")
    ap.add_argument("--no-fetch", action="store_true", help="use the clones in --work as they are")
    args = ap.parse_args()

    work = Path(args.work)
    work.mkdir(parents=True, exist_ok=True)
    lock, need = pins(), needs()
    names = sorted({p.name for p in (ROOT / "patches").iterdir()
                    if p.is_dir() and p.name != "experiments"}
                   | {d.name for d in (ROOT / "patches/experiments").glob("*/*")
                      if d.is_dir() and d.name in REPOS})
    data = {}
    for name in names:
        if name not in REPOS or name not in lock:
            print(f"skipped {name}: no repository or no line in config/upstream.lock", file=sys.stderr)
            continue
        data[name] = component(name, work, lock[name], not args.no_fetch, need)
        print(f"{name}: {counts(data[name]['latest'])} on the head", file=sys.stderr)

    text, problems = report(data)
    if args.out:
        Path(args.out).write_text(text + "\n")
    else:
        print(text)
    if args.json:
        Path(args.json).write_text(json.dumps(data, indent=1))


if __name__ == "__main__":
    main()
