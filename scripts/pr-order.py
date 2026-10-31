#!/usr/bin/env python3
"""List the open pull requests in an order they can be merged in.

Usage:
    scripts/pr-order.py [--repo redox-rpi/redox-rpi] [--no-fetch] [--json]

For every open pull request it shows what the coordinator checks before a
merge (docs/test-plan.md, "Merging through pull requests"):

  base      the branch it merges into; stacked pull requests come after
            the one they build on
  CI        green, failed, running or none
  Pi test   the newest docs/test-plan.md row that names the branch or the
            pull request, and whether it passed
  patches   numbered patches it adds (patches/<area>/NNNN-*.patch), with
            numbers already on main or added by another pull request marked
  conflicts with main, and with other open pull requests that change the
            same files

The suggested order merges ready pull requests first (not a draft, CI
green, no conflict with main, Pi test passed), each base before the pull requests stacked on it.
The script tries the order in git: it merges each pull request into the
result of the ones before it, without changing any branch, and reports the
first one that conflicts. Pull requests with a patch number clash merge
fine in git but need their patches renumbered first.

Only reads: it fetches the pull request heads to refs/remotes/origin/pr/N,
nothing else. Needs gh (logged in) and git 2.38 or newer.
"""

import argparse
import json
import re
import subprocess
import sys

PATCH = re.compile(r"^patches/(base|kernel|relibc|bootloader|fdt)/(\d{4})-.*\.patch$")
# Files most pull requests touch; a shared one is no reason to merge in order.
COMMON = {"docs/test-plan.md", "docs/test-reports.md", "patches/README.md"}
# A result that starts with a bold "fail"/"no"/"not" failed, even if it also
# says "standard checks pass"; "0 of 100 starts failed" is still a pass.
FAILED = re.compile(r"\*\*(fail|no\b|not\b|found\b)", re.I)
PASSED = re.compile(r"\bpass", re.I)


def run(*cmd, check=True):
    p = subprocess.run(cmd, capture_output=True, text=True)
    if check and p.returncode:
        sys.exit(f"{' '.join(cmd)}: {p.stderr.strip()}")
    return p


def git(*args, check=True):
    return run("git", *args, check=check).stdout.strip()


def merge_tree(a, b):
    """(tree ID, conflicting files) of merging a and b; the tree is None on a conflict."""
    p = run("git", "merge-tree", "--write-tree", "--name-only", "--no-messages", a, b,
            check=False)
    lines = p.stdout.split("\n")
    if p.returncode == 0:
        return lines[0], []
    return None, sorted({f for f in lines[1:] if f})


def ci_state(checks):
    states = set()
    for c in checks:
        result = c.get("conclusion") or c.get("state") or ""
        if c.get("status") not in (None, "COMPLETED"):
            states.add("running")
        elif result in ("SUCCESS", "NEUTRAL", "SKIPPED"):
            states.add("green")
        else:
            states.add("failed")
    for s in ("failed", "running", "green"):
        if s in states:
            return s
    return "none"


def test_rows(text):
    """Rows of the queue and the done list in docs/test-plan.md, newest first.

    Queue rows are "| # | Test | Owner | Image | Needs | Status |", done rows
    "| Date | Test | Build ID | Result |" with the number at the start of
    the test cell (older rows have none)."""
    rows, section = [], ""
    for line in text.splitlines():
        if line.startswith("## "):
            section = line[3:].strip()
            continue
        if not line.startswith("|") or line.startswith("|---"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if section == "Queue" and cells[0].isdigit():
            rows.append({"test": int(cells[0]), "state": "queued", "date": "",
                         "line": line, "status": cells[-1]})
        elif section == "Done" and re.match(r"\d{4}-\d\d-\d\d$", cells[0]):
            m = re.match(r"(\d+)\b", cells[1])
            result = cells[-1]
            state = ("failed" if FAILED.search(result) else
                     "passed" if PASSED.search(result) else "done")
            rows.append({"test": int(m.group(1)) if m else None, "state": state,
                         "date": cells[0], "line": line, "status": result})
    return rows


def pi_test(rows, pr):
    """The test-plan rows that name the branch or the pull request: the
    newest done one and a queued one, if any."""
    names = [f"`{pr['head']}`", f"PR #{pr['number']} ", f"/pull/{pr['number']})"]
    found = [r for r in rows if any(n in r["line"] for n in names)]
    done = [r for r in found if r["state"] != "queued"]
    queued = [r for r in found if r["state"] == "queued"]
    if not found:
        return None
    # The done list is newest first within a date; take the first of the newest date.
    best = max(done, key=lambda r: r["date"]) if done else None
    return {
        "state": best["state"] if best else "queued",
        "test": best["test"] if best else queued[0]["test"],
        "date": best["date"] if best else "",
        "queued": [r["test"] for r in queued],
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--repo", default="redox-rpi/redox-rpi")
    ap.add_argument("--no-fetch", action="store_true", help="use the refs as they are")
    ap.add_argument("--json", action="store_true", help="print JSON instead of Markdown")
    args = ap.parse_args()

    fields = "number,title,headRefName,baseRefName,isDraft,statusCheckRollup"
    prs = json.loads(run("gh", "pr", "list", "--repo", args.repo, "--state", "open",
                         "--limit", "200", "--json", fields).stdout)
    if not args.no_fetch:
        git("fetch", "-q", "origin", "+refs/heads/*:refs/remotes/origin/*",
            "+refs/pull/*/head:refs/remotes/origin/pr/*")

    rows = test_rows(git("show", "origin/main:docs/test-plan.md"))
    on_main = set()
    for path in git("ls-tree", "-r", "--name-only", "origin/main", "patches").splitlines():
        m = PATCH.match(path)
        if m:
            on_main.add(m.groups())

    info = {}
    for p in prs:
        n, head = p["number"], f"origin/pr/{p['number']}"
        base = f"origin/{p['baseRefName']}"
        files = git("diff", "--name-only", f"{base}...{head}").splitlines()
        added = git("diff", "--name-only", "--diff-filter=A", f"{base}...{head}").splitlines()
        info[n] = {
            "number": n, "title": p["title"], "head": p["headRefName"],
            "base": p["baseRefName"], "draft": p["isDraft"],
            "ci": ci_state(p["statusCheckRollup"] or []),
            "ref": head, "files": files,
            "patches": sorted(m.groups() for m in map(PATCH.match, added) if m),
            "main": "clean" if merge_tree("origin/main", head)[0] else "conflict",
        }
        info[n]["pi"] = pi_test(rows, info[n])

    by_head = {i["head"]: n for n, i in info.items()}
    for i in info.values():
        i["parent"] = by_head.get(i["base"])
        i["notes"] = []
        if i["base"] != "main" and i["parent"] is None:
            i["notes"].append(f"base `{i['base']}` has no open pull request")

    # Patch numbers: on main already, or added by more than one pull request.
    owners = {}
    for n, i in info.items():
        for p in i["patches"]:
            owners.setdefault(p, []).append(n)
    for n, i in info.items():
        for p in i["patches"]:
            others = [f"#{o}" for o in owners[p] if o != n and info[o]["parent"] != n
                      and i["parent"] != o]
            if p in on_main:
                i["notes"].append(f"{p[0]} {p[1]} is on main already")
            elif others:
                i["notes"].append(f"{p[0]} {p[1]} also in {', '.join(others)}")

    # Pairs that change the same files and conflict with each other.
    conflicts = []
    nums = sorted(info)
    for a_i, a in enumerate(nums):
        for b in nums[a_i + 1:]:
            if info[a]["parent"] == b or info[b]["parent"] == a:
                continue
            shared = set(info[a]["files"]) & set(info[b]["files"])
            if shared - COMMON:
                tree, files = merge_tree(info[a]["ref"], info[b]["ref"])
                if not tree:
                    conflicts.append((a, b, files))
    hot = {}
    for a, b, files in conflicts:
        info[a].setdefault("conflicts", []).append(b)
        info[b].setdefault("conflicts", []).append(a)
        for f in files:
            hot.setdefault(f, set()).update((a, b))

    # Order: ready ones first, then the rest; a parent always before its children.
    def rank(n):
        i = info[n]
        ready = (not i["draft"] and i["ci"] == "green" and i["main"] == "clean"
                 and i["pi"] is not None and i["pi"]["state"] == "passed")
        return (0 if ready else 1, i["draft"], n)

    order, seen = [], set()

    def visit(n):
        if n in seen:
            return
        seen.add(n)
        if info[n]["parent"]:
            visit(info[n]["parent"])
        order.append(n)

    for n in sorted(info, key=rank):
        visit(n)

    # Try the order: merge each into the result of the ones before it.
    current = git("rev-parse", "origin/main")
    for n in order:
        i = info[n]
        tree, _ = merge_tree(current, i["ref"])
        i["ready"] = rank(n)[0] == 0
        if tree:
            current = git("commit-tree", tree, "-p", current, "-p", i["ref"],
                          "-m", f"pr-order: #{n}")
            i["in_order"] = "clean"
        else:
            i["in_order"] = "conflict"

    if args.json:
        out = [{k: v for k, v in info[n].items() if k not in ("files", "ref")}
               for n in order]
        print(json.dumps(out, indent=2))
        return

    print("| # | PR | Branch | CI | Pi test | Into main | In this order | Notes |")
    print("|---|---|---|---|---|---|---|---|")
    for pos, n in enumerate(order, 1):
        i = info[n]
        branch = f"`{i['head']}`" + ("" if i["base"] == "main" else f" → `{i['base']}`")
        if i["draft"]:
            branch += " (draft)"
        pi = "—"
        if i["pi"]:
            t = i["pi"]
            pi = t["state"]
            when = ", ".join(x for x in (f"test {t['test']}" if t["test"] else "", t["date"]) if x)
            if when:
                pi += f" ({when})"
            if t["state"] != "queued" and t["queued"]:
                pi += f", test {', '.join(map(str, t['queued']))} queued"
        notes = list(i["notes"])
        c = sorted(i.get("conflicts", []))
        if len(c) > 4:
            notes.append(f"conflicts with {len(c)} pull requests")
        elif c:
            notes.append("conflicts with " + ", ".join(f"#{x}" for x in c))
        if i["patches"]:
            notes.insert(0, "adds " + ", ".join(f"{a} {num}" for a, num in i["patches"]))
        mark = "ready" if i["ready"] else ""
        print(f"| {pos} {mark} | #{n} | {branch} | {i['ci']} | {pi} | {i['main']} | "
              f"{i['in_order']} | {'; '.join(notes)} |")

    if hot:
        print("\nFiles where open pull requests conflict with each other:\n")
        for f, nums_ in sorted(hot.items(), key=lambda kv: (-len(kv[1]), kv[0])):
            print(f"- `{f}`: {', '.join(f'#{x}' for x in sorted(nums_))}")


if __name__ == "__main__":
    main()
