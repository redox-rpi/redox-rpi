#!/usr/bin/env python3
"""Warn when a change touches a topic area but not its docs/topics/ page.

Usage:
    scripts/check-topics.py [--base <rev>] [--head <rev>] [--range]

Compares <base>...<head> (default: origin/main...HEAD, the changes since
the branch left main). With --range (for a push to main) it looks only at
the commits pushed directly, <base>..<head> without merges and the branches
they bring in: a merged pull request was checked as a pull request. The
path-to-topic map is config/topics.map. For each topic whose files changed
while none of its pages did, it prints a warning (a GitHub annotation in
Actions) and writes a table to $GITHUB_STEP_SUMMARY when that is set.

Always exits 0 for warnings: whether a page needs an update is for the
author to decide (CONTRIBUTING.md, "Topics and issues"). Exits 1 only if
the map is broken.
"""

import argparse
import fnmatch
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MAP = os.path.join(ROOT, "config", "topics.map")
AREAS = ("patches/", "scripts/", "tools/", "config/")


def read_map():
    rules = []
    for num, line in enumerate(open(MAP), 1):
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        parts = line.split()
        if len(parts) != 2:
            sys.exit(f"{MAP}:{num}: expected a pattern and pages, got {line!r}")
        pattern, pages = parts
        pages = [] if pages == "-" else pages.split(",")
        for p in pages:
            if not os.path.exists(os.path.join(ROOT, "docs", "topics", p + ".md")):
                sys.exit(f"{MAP}:{num}: no page docs/topics/{p}.md")
        rules.append((pattern, pages))
    return rules


def topics_of(path, rules):
    for pattern, pages in rules:
        if fnmatch.fnmatchcase(path, pattern):
            return pages
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--base", default="origin/main")
    ap.add_argument("--head", default="HEAD")
    ap.add_argument("--range", action="store_true", help="only the commits pushed directly in base..head")
    args = ap.parse_args()

    rules = read_map()

    def git(*a):
        return subprocess.run(["git", "-C", ROOT, *a], capture_output=True, text=True,
                              check=True).stdout.split()

    if args.range:
        files = sorted({f for c in git("rev-list", "--first-parent", "--no-merges",
                                       f"{args.base}..{args.head}")
                        for f in git("diff-tree", "--no-commit-id", "--name-only", "-r", c)})
    else:
        files = git("diff", "--name-only", f"{args.base}...{args.head}")
    pages = {f[len("docs/topics/"):-3] for f in files
             if f.startswith("docs/topics/") and f.endswith(".md")}

    # Files per topic (a rule with several pages counts as one group).
    missing, unmapped = {}, []
    for f in files:
        topics = topics_of(f, rules)
        if topics is None:
            if f.startswith(AREAS):
                unmapped.append(f)
            continue
        if topics and not pages & set(topics):
            missing.setdefault(", ".join(topics), []).append(f)

    gh = os.environ.get("GITHUB_ACTIONS") == "true"
    for topics, fs in sorted(missing.items()):
        names = " or ".join(f"docs/topics/{t}.md" for t in topics.split(", "))
        shown = ", ".join(fs[:3]) + (f" and {len(fs) - 3} more" if len(fs) > 3 else "")
        msg = f"{shown} changed, but not {names}. Update the page if the state of the topic changed."
        print(f"::warning title=Topic page::{msg}" if gh else f"warning: {msg}")
    for f in unmapped:
        print(f"note: {f} has no topic in config/topics.map")
    if not missing:
        print("Topic pages: nothing missing" + (f" ({len(pages)} pages changed)" if pages else ""))

    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary and (missing or unmapped):
        with open(summary, "a") as out:
            out.write("### Topic pages\n\n")
            if missing:
                out.write("| Topic page | Changed files |\n|---|---|\n")
                for topics, fs in sorted(missing.items()):
                    out.write(f"| {topics} | {', '.join(f'`{f}`' for f in fs)} |\n")
            if unmapped:
                out.write("\nNo topic in `config/topics.map`: "
                          + ", ".join(f"`{f}`" for f in unmapped) + "\n")


if __name__ == "__main__":
    main()
