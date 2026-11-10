#!/usr/bin/env python3
"""Collect the quality KPIs of docs/process/qmp.md and write a Markdown page.

Usage:
    scripts/quality-kpis.py [--days 30] [--repo redox-rpi/redox-rpi]
                            [--rev origin/main] [--out docs/process/kpis.md]
                            [--json kpis.json]

Reads GitHub through the gh CLI (set GH_TOKEN, e.g.
GH_TOKEN=$(gh auth token --user zehrer-xyz)) and the repository at --rev
(patches/, git log), and the Pi test queue in --tests-repo (queue.md). Changes nothing on GitHub or in
the repository except the files it writes. Exits 1 only when a source
cannot be read.
"""

import argparse
import datetime
import json
import os
import re
import statistics
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Recurring and flaky bugs tracked in docs/topics/quality.md ("Known issues").
TRACKED = {
    28: "serial input drops characters",
    92: "network lost minutes after boot",
    217: "login without network",
    108: "pthread_exit ends with SIGKILL",
    79: "netstack finds no adapter",
    129: "devshd stops accepting",
    205: "kill() never reaches a pipe loop",
    207: "chaos test froze the Pi",
}
PR_RULE_SINCE = datetime.datetime(2026, 10, 29, tzinfo=datetime.timezone.utc)
NIGHTLY_ISSUE = "Nightly: main fails on the Pi"
CODE_PATHS = ("patches/", "tools/", "config/")
PATCH_PATHS = ("patches/", "config/")
DEFAULT_PATCH_DIRS = ("base", "kernel", "relibc", "redoxfs", "bootloader", "fdt")
PI_TEST_RE = re.compile(r"pi[- ]test|\b(?:tests?|rows?) #?\d{2,3}\b|test-plan|test-reports|real pi|on the pi\b", re.I)
ISSUE_REF_RE = re.compile(r"#\d+|/issues/\d+")


def sh(*args, cwd=ROOT):
    r = subprocess.run(args, cwd=cwd, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"{' '.join(args[:3])}: {r.stderr.strip()[:300]}")
    return r.stdout


def gh_json(*args):
    return json.loads(sh("gh", *args))


def ts(s):
    return datetime.datetime.fromisoformat(s.replace("Z", "+00:00"))


def graphql_all(query, repo, path, **variables):
    """Pages through a connection; path names it under repository."""
    owner, name = repo.split("/")
    out, cursor = [], None
    while True:
        args = ["api", "graphql", "-f", f"query={query}", "-F", f"owner={owner}", "-F", f"name={name}"]
        for k, v in variables.items():
            args += ["-F", f"{k}={v}"]
        if cursor:
            args += ["-F", f"cursor={cursor}"]
        conn = gh_json(*args)["data"]["repository"][path]
        out += conn["nodes"]
        if not conn["pageInfo"]["hasNextPage"]:
            return out
        cursor = conn["pageInfo"]["endCursor"]


ISSUES_Q = """
query($owner: String!, $name: String!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    issues(first: 100, after: $cursor, states: [OPEN, CLOSED],
           orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number title state createdAt updatedAt closedAt url
        issueType { name }
        assignees(first: 5) { nodes { login } }
        labels(first: 20) { nodes { name } }
        timelineItems(itemTypes: [REOPENED_EVENT], last: 5) { nodes { ... on ReopenedEvent { createdAt } } }
      }
    }
  }
}"""

REPOS_Q = """
query($owner: String!) {
  organization(login: $owner) {
    repositories(first: 100) { nodes { name isArchived hasIssuesEnabled } }
  }
}"""

PRS_Q = """
query($owner: String!, $name: String!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    pullRequests(first: 50, after: $cursor, states: [MERGED],
                 orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number title body createdAt mergedAt baseRefName
        closingIssuesReferences(first: 1) { totalCount }
        commits(last: 1) { nodes { commit { committedDate } } }
        files(first: 100) { nodes { path } }
      }
    }
  }
}"""


def row_id(row):
    m = re.match(r"#?(\d+)", row["test"])
    return f"#{m.group(1)}" if m else row["test"][:12]


def collect(args):
    now = datetime.datetime.now(datetime.timezone.utc)
    since = now - datetime.timedelta(days=args.days)
    k = {"generated": now.isoformat(timespec="minutes"), "days": args.days, "rev": args.rev,
         "tests_repo": args.tests_repo}

    # Issues of every repository of the organisation (RFC 0002 moves
    # issues to the repository they concern). Numbers in the main repository
    # stay plain (#N), others are "<repo>#N".
    org = args.repo.split("/")[0]
    repos = gh_json("api", "graphql", "-f", f"query={REPOS_Q}", "-F", f"owner={org}")
    repos = sorted(r["name"] for r in repos["data"]["organization"]["repositories"]["nodes"]
                   if r["hasIssuesEnabled"] and not r["isArchived"])
    main_name = args.repo.split("/")[1]
    all_issues = []
    for name in repos:
        for i in graphql_all(ISSUES_Q, f"{org}/{name}", "issues"):
            i["repo"] = name
            i["ref"] = f"#{i['number']}" if name == main_name else f"{name}#{i['number']}"
            i["labels"] = [label["name"] for label in i["labels"]["nodes"]]
            i["type"] = (i["issueType"] or {}).get("name") or "none"
            i["assignees"] = [a["login"] for a in i["assignees"]["nodes"]]
            all_issues.append(i)
    issues = [i for i in all_issues if i["repo"] == main_name]
    k["repos"] = repos
    open_all = [i for i in all_issues if i["state"] == "OPEN"]
    by_type = {}
    for i in open_all:
        by_type.setdefault(i["repo"], {}).setdefault(i["type"], 0)
        by_type[i["repo"]][i["type"]] += 1
    k["open_by_repo_type"] = by_type
    k["no_type"] = [i["ref"] for i in open_all if i["type"] == "none"]
    open_issues = [i for i in issues if i["state"] == "OPEN"]
    bugs = [i for i in open_all if i["type"] == "Bug" or "bug" in i["labels"]]
    sev = {}
    for b in bugs:
        s = next((label.split(":", 1)[1] for label in b["labels"] if label.startswith("severity:")),
                 "unclassified")
        sev.setdefault(s, []).append(b["ref"])
    k["open_bugs"] = {s: n for s, n in sorted(sev.items())}
    k["s1_older_7d"] = [b["ref"] for b in bugs if "severity:S1" in b["labels"]
                        and now - ts(b["createdAt"]) > datetime.timedelta(days=7)]
    k["unlabelled"] = sorted(i["number"] for i in open_issues
                             if not any(label.startswith("topic:") for label in i["labels"]))
    k["stale_in_progress"] = sorted(
        i["number"] for i in open_issues
        if "in-progress" in i["labels"] and now - ts(i["updatedAt"]) > datetime.timedelta(days=14))
    k["reopened"] = sorted(i["number"] for i in issues
                           if any(ts(e["createdAt"]) >= since for e in i["timelineItems"]["nodes"]))
    k["closed_but_tracked"] = sorted(n for n in TRACKED
                                     if any(i["number"] == n and i["state"] == "CLOSED" for i in issues))

    # Pull requests (merged into main in the window)
    prs = []
    for p in graphql_all(PRS_Q, args.repo, "pullRequests"):
        if p["baseRefName"] == "main" and p["mergedAt"] and ts(p["mergedAt"]) >= since - datetime.timedelta(days=14):
            p["files"] = [f["path"] for f in p["files"]["nodes"]]
            prs.append(p)
    prs.sort(key=lambda p: p["mergedAt"])
    window = [p for p in prs if ts(p["mergedAt"]) >= since]
    hours = [(ts(p["mergedAt"]) - ts(p["createdAt"])).total_seconds() / 3600 for p in window]
    k["prs_merged"] = len(window)
    if hours:
        hours.sort()
        k["lead_time_h"] = {"median": round(statistics.median(hours), 1),
                            "p90": round(hours[min(len(hours) - 1, int(len(hours) * 0.9))], 1)}
    k["prs_without_issue"] = [p["number"] for p in window
                              if not p["closingIssuesReferences"]["totalCount"]
                              and not ISSUE_REF_RE.search(p["title"] + " " + (p["body"] or ""))]
    k["code_prs_without_pi_test"] = [
        p["number"] for p in window
        if any(f.startswith(CODE_PATHS) for f in p["files"])
        and not PI_TEST_RE.search(p["title"] + " " + (p["body"] or ""))]
    # A PR is a stale-CI merge when its last commit (so its last CI run,
    # unless rerun by hand) is older than the merge of an earlier PR that
    # changed the same patch directory (patches/base, patches/kernel, …)
    # or config/: the two were never tested together, as #81 after #194.
    def areas(files):
        return {"/".join(f.split("/")[:2]) if f.startswith("patches/") else "config"
                for f in files if f.startswith(PATCH_PATHS)}
    stale, last_merge = [], {}
    for p in prs:
        commit = p["commits"]["nodes"][0]["commit"]["committedDate"] if p["commits"]["nodes"] else p["createdAt"]
        mine = areas(p["files"])
        if ts(p["mergedAt"]) >= since and any(a in last_merge and ts(commit) < last_merge[a] for a in mine):
            stale.append(p["number"])
        for a in mine:
            last_merge[a] = ts(p["mergedAt"])
    k["stale_ci_merges"] = stale

    # Main's CI: hours red
    runs = gh_json("run", "list", "-R", args.repo, "--workflow", "qemu-boot.yml", "--branch", "main",
                   "--limit", "300", "--json", "conclusion,createdAt,headSha,event")
    runs = sorted((r for r in runs if r["event"] == "push" and r["conclusion"] in ("success", "failure")),
                  key=lambda r: r["createdAt"])
    red, red_since, streaks = 0.0, None, []
    for r in runs:
        t = ts(r["createdAt"])
        if r["conclusion"] == "failure" and red_since is None:
            red_since, first = t, r["headSha"][:8]
        elif r["conclusion"] == "success" and red_since is not None:
            if t >= since:
                h = (t - max(red_since, since)).total_seconds() / 3600
                red += h
                streaks.append({"from": first, "start": red_since.isoformat(timespec="minutes"), "hours": round(h, 1)})
            red_since = None
    if red_since is not None:
        h = (now - max(red_since, since)).total_seconds() / 3600
        red += h
        streaks.append({"from": first, "start": red_since.isoformat(timespec="minutes"), "hours": round(h, 1),
                        "still_red": True})
    k["main_red_hours"] = round(red, 1)
    k["main_red_streaks"] = streaks

    # Nightly: passes are its commits ("Nightly: <ID> is the stable image"),
    # on main until 2026-11-09 and in the tests repository since; fails are
    # its reports on the issue "Nightly: main fails on the Pi".
    log = sh("git", "log", args.rev, f"--since={since.isoformat()}", "--format=%s").splitlines()
    page = 1
    while True:
        commits = gh_json("api", f"repos/{args.tests_repo}/commits?since={since.isoformat()}&per_page=100&page={page}")
        log += [c["commit"]["message"].split("\n")[0] for c in commits]
        if len(commits) < 100:
            break
        page += 1
    passes = {s for s in log if re.match(r"Nightly: \S+ is the stable image", s)}
    fails = 0
    found = [i for i in all_issues if i["title"] == NIGHTLY_ISSUE]
    for i in found:
        # update-stable.sh starts each failure report with "Build `<ID>`";
        # other comments are people's notes.
        v = gh_json("issue", "view", str(i["number"]), "-R", f"{org}/{i['repo']}", "--json", "body,comments,createdAt")
        posts = [(v["createdAt"], v["body"])] + [(c["createdAt"], c["body"]) for c in v["comments"]]
        fails += sum(1 for t, body in posts if ts(t) >= since and body.startswith("Build `"))
    k["nightly"] = {"pass": len(passes), "fail": fails}

    # Commits straight on main that change patches/ (P5): first-parent
    # commits that are neither a merge nor a squashed PR ("… (#N)").
    # Counted from PR_RULE_SINCE, when merging only by PR started.
    start = max(since, PR_RULE_SINCE)
    direct = sh("git", "log", args.rev, "--first-parent", "--no-merges", f"--since={start.isoformat()}",
                "--format=%h %s", "--", "patches/*.patch")
    k["direct_patch_commits"] = [line.split()[0] for line in direct.splitlines()
                                 if not re.search(r"\(#\d+\)$", line)]

    # Pi rows from the "Done" table of the queue (redox-rpi/pi-tests since
    # 2026-11-10; it holds the older rows too)
    plan = sh("gh", "api", f"repos/{args.tests_repo}/contents/queue.md",
              "-H", "Accept: application/vnd.github.raw+json")
    done = plan.split("\n## Done", 1)[1] if "\n## Done" in plan else ""
    rows = []
    for line in done.splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 4 or not re.match(r"\d{4}-\d\d-\d\d$", cells[0]):
            continue
        if datetime.date.fromisoformat(cells[0]) < since.date():
            continue
        result = " | ".join(cells[3:])
        m = re.search(r"\*\*([^*]+)\*\*", result)
        head = (m.group(1) if m else result).lower()
        verdict = "pass" if head.startswith("pass") else "fail" if head.startswith("fail") else "other"
        rows.append({"date": cells[0], "test": cells[1], "build": cells[2], "verdict": verdict,
                     "text": cells[1] + " " + result})
    k["pi_rows"] = {v: sum(1 for r in rows if r["verdict"] == v) for v in ("pass", "fail", "other")}
    sightings = {}
    for n in TRACKED:
        ref = re.compile(rf"#{n}\b|issues/{n}\b")
        sightings[n] = [row_id(r) for r in rows if ref.search(r["text"])]
    k["sightings"] = {str(n): s for n, s in sightings.items()}
    flaky_rows = {r["test"] for r in rows for n in TRACKED if re.search(rf"#{n}\b|issues/{n}\b", r["text"])}
    k["flaky_rows"] = len(flaky_rows)
    k["dirty_rows"] = [row_id(r) for r in rows if "-dirty" in r["build"]]
    # Pi rows per topic: the topic labels of the issues a row names.
    topic_of = {i["number"]: [label[6:] for label in i["labels"] if label.startswith("topic:")] for i in issues}
    all_topics = sorted({t for ts_ in topic_of.values() for t in ts_})
    per_topic = {t: 0 for t in all_topics}
    for r in rows:
        found = set()
        # Row and issue numbers share the "#N" form; the row's own number
        # is left out, other row numbers in the text can still count.
        text = r["text"].replace(row_id(r), "", 1)
        for n in re.findall(r"#(\d+)|issues/(\d+)", text):
            found.update(topic_of.get(int(n[0] or n[1]), []))
        for t in found:
            per_topic[t] = per_topic.get(t, 0) + 1
    k["pi_rows_per_topic"] = per_topic

    # Host tests: our Rust crates under tools/ with at least one #[test]
    crates = [d for d in sh("git", "ls-tree", "--name-only", f"{args.rev}:tools").split()
              if f"tools/{d}/Cargo.toml" in sh("git", "ls-tree", "-r", "--name-only", args.rev, f"tools/{d}")]
    tested = {}
    for d in crates:
        n = 0
        for f in sh("git", "ls-tree", "-r", "--name-only", args.rev, f"tools/{d}").split():
            if f.endswith(".rs"):
                n += sh("git", "show", f"{args.rev}:{f}").count("#[test]")
        tested[d] = n
    k["host_tests"] = tested

    # Requirements (docs/requirements/*.md, RMP): one "### REQ-<AREA>-<n>"
    # entry each, with a field table. Until they become Feature issues
    # (RFC 0002) they are read from --req-rev.
    req_rev = args.req_rev or args.rev
    k["req_rev"] = req_rev
    names = sh("git", "ls-tree", "--name-only", f"{req_rev}:docs/requirements") \
        if subprocess.run(["git", "cat-file", "-e", f"{req_rev}:docs/requirements"], cwd=ROOT,
                         stderr=subprocess.DEVNULL).returncode == 0 else ""
    reqs = []
    for f in names.split():
        if not f.endswith(".md") or f in ("README.md", "trace.md"):
            continue
        cur = None
        for line in sh("git", "show", f"{req_rev}:docs/requirements/{f}").splitlines():
            m = re.match(r"### (REQ-[A-Z]+-\d+)\s+(.*)", line)
            if m:
                cur = {"id": m.group(1), "name": m.group(2), "file": f}
                reqs.append(cur)
                continue
            m = re.match(r"\| (Priority|Status|Level|Verification|Implemented by) \| (.*) \|$", line)
            if cur and m:
                cur[m.group(1).lower()] = m.group(2)
    if reqs:
        def prio(r):
            return r.get("priority", "?").split()[0].strip("`")
        ver = lambda r: r.get("verification", "")
        known = {r["id"] for r in reqs}
        named = set(re.findall(r"REQ-[A-Z]+-\d+", plan))
        k["requirements"] = {
            "total": len(reqs),
            "by_priority": {p_: sum(1 for r in reqs if prio(r) == p_) for p_ in sorted({prio(r) for r in reqs})},
            "gap": [r["id"] for r in reqs if "gap" in ver(r).lower()],
            "gap_must": [r["id"] for r in reqs if "gap" in ver(r).lower() and prio(r) == "must"],
            "no_test": [r["id"] for r in reqs if not re.search(r"\btest\b|\brow\b|checklist", ver(r))],
            "failing": [r["id"] for r in reqs if re.search(r"\bfails?\b", ver(r))],
            "person_only": [r["id"] for r in reqs if ver(r).startswith("test, person")],
            "dirty_only": [r["id"] for r in reqs if "allow-dirty" in ver(r)],
            "no_issue": [r["id"] for r in reqs
                         if not re.search(r"#\d+|upstream|before the project", r.get("implemented by", ""), re.I)],
            "rows_naming": sorted(named & known),
            "unknown_in_rows": sorted(named - known),
        }

    # Patches whose header names an issue
    total = named = 0
    for d in DEFAULT_PATCH_DIRS:
        names = sh("git", "ls-tree", "--name-only", f"{args.rev}:patches/{d}").split()
        for f in names:
            if not f.endswith(".patch"):
                continue
            total += 1
            text = sh("git", "show", f"{args.rev}:patches/{d}/{f}")
            if ISSUE_REF_RE.search(text.split("\ndiff --git", 1)[0]):
                named += 1
    k["patches_with_issue"] = {"named": named, "total": total}
    return k


def link(repo, n, kind="issues"):
    return f"[#{n}](https://github.com/{repo}/{kind}/{n})"


def links(repo, ns, kind="issues", most=12):
    if not ns:
        return "none"
    shown = ", ".join(link(repo, n, kind) for n in ns[:most])
    return shown + (f" and {len(ns) - most} more" if len(ns) > most else "")


def pct(a, b):
    return f"{100 * a / b:.0f} %" if b else "–"


def render(k, repo):
    ok, warn = "ok", "**check**"
    rows = []

    def row(name, value, good, note=""):
        rows.append(f"| {name} | {value} | {ok if good else warn} | {note} |")

    bugs = k["open_bugs"]
    s1 = k.get("s1_older_7d", [])
    types = sorted({t for v in k["open_by_repo_type"].values() for t in v})
    row("Open issues by repo and type",
        "; ".join(f"{r}: " + ", ".join(f"{t} {n}" for t, n in sorted(v.items()))
                  for r, v in sorted(k["open_by_repo_type"].items())),
        True, f"types in use: {', '.join(types)}; repos read: {', '.join(k['repos'])}")
    row("Open issues without a type", len(k["no_type"]), not k["no_type"],
        "Task/Bug/Feature not set yet" if len(k["no_type"]) > 12 else ", ".join(k["no_type"]))
    row("Open bugs (type Bug or label `bug`)",
        ", ".join(f"{s}: {len(n)}" for s, n in bugs.items()) or "0", not s1,
        ("no `severity:` labels yet; " if set(bugs) <= {"unclassified"} else "")
        + "; ".join(f"{sv}: {', '.join(n)}" for sv, n in bugs.items())
        + (f"; **S1 older than 7 days:** {', '.join(s1)}" if s1 else ""))
    row("Unlabelled open issues", len(k["unlabelled"]), not k["unlabelled"], links(repo, k["unlabelled"]))
    row("Stale `in-progress` (14 days)", len(k["stale_in_progress"]), not k["stale_in_progress"],
        links(repo, k["stale_in_progress"]))
    row("Reopened issues", len(k["reopened"]), True, links(repo, k["reopened"]))
    row("Tracked bugs closed on GitHub", len(k["closed_but_tracked"]), not k["closed_but_tracked"],
        links(repo, k["closed_but_tracked"]) + " (still seen: see quality.md)" if k["closed_but_tracked"] else "")
    lt = k.get("lead_time_h", {})
    row("PR lead time (median / p90)",
        f"{lt.get('median', '–')} h / {lt.get('p90', '–')} h over {k['prs_merged']} PRs",
        lt.get("median", 0) < 72)
    row("PRs without an issue (P1)", len(k["prs_without_issue"]), not k["prs_without_issue"],
        links(repo, k["prs_without_issue"], "pull"))
    row("Code PRs without a Pi test named (P2)", len(k["code_prs_without_pi_test"]),
        not k["code_prs_without_pi_test"], links(repo, k["code_prs_without_pi_test"], "pull"))
    row("Stale-CI merges (P3)", len(k["stale_ci_merges"]), not k["stale_ci_merges"],
        "last commit older than an earlier merge in the same patch directory: "
        + links(repo, k["stale_ci_merges"], "pull", most=8))
    row("Patch commits on main outside a PR (P5)", len(k["direct_patch_commits"]),
        not k["direct_patch_commits"], ", ".join(k["direct_patch_commits"][:10]))
    longest = max((s["hours"] for s in k["main_red_streaks"]), default=0)
    row("Main red hours", f"{k['main_red_hours']} h in {len(k['main_red_streaks'])} streaks",
        longest < 8, "; ".join(f"from {s['from']} {s['hours']} h" + (" (still red)" if s.get("still_red") else "")
                               for s in k["main_red_streaks"][-5:]))
    n = k["nightly"]
    row("Nightly pass rate", f"{pct(n['pass'], n['pass'] + n['fail'])} ({n['pass']} pass, {n['fail']} fail)",
        n["fail"] == 0)
    p = k["pi_rows"]
    allrows = sum(p.values())
    row("Pi pass rate", f"{pct(p['pass'], allrows)} ({p['pass']} pass, {p['fail']} fail, {p['other']} other)",
        True, "\"other\": partly, finding, diagnosis")
    row("Flaky rate", f"{pct(k['flaky_rows'], allrows)} ({k['flaky_rows']} of {allrows} rows)", True,
        "rows that name a tracked bug")
    pw = k["patches_with_issue"]
    row("Patches with an issue (P8)", f"{pct(pw['named'], pw['total'])} ({pw['named']} of {pw['total']})", True)
    row("Dirty results (P9)", len(k["dirty_rows"]), True, ", ".join(k["dirty_rows"][:10]))
    ht = k["host_tests"]
    with_tests = [d for d, n in ht.items() if n]
    row("Host tests (Rust crates in tools/)",
        f"{len(with_tests)} of {len(ht)} crates, {sum(ht.values())} tests", True,
        ", ".join(f"{d} {ht[d]}" for d in with_tests) + "; line coverage not measured yet")
    pt = k["pi_rows_per_topic"]
    row("Pi rows per topic", ", ".join(f"{t} {n}" for t, n in sorted(pt.items(), key=lambda x: -x[1]) if n),
        True, "rows naming an issue of the topic (heuristic); no rows: " + (", ".join(t for t, n in sorted(pt.items()) if not n) or "none"))
    row("Release checklist completion", "not measurable yet", True, "the checklist has no result column")
    rq = k.get("requirements")
    if rq:
        tested = rq["total"] - len(set(rq["gap"]) | set(rq["no_test"]))
        row("Test coverage per requirement",
            f"{pct(tested, rq['total'])} ({tested} of {rq['total']}; "
            + ", ".join(f"{p_} {n}" for p_, n in rq["by_priority"].items()) + ")",
            not rq["gap_must"], f"fully tested; from `{k['req_rev']}`; a **gap** in some criterion: "
            f"{', '.join(rq['gap']) or 'none'}; no test named: {', '.join(rq['no_test']) or 'none'}"
            + (f"; `must` with a gap: {', '.join(rq['gap_must'])}" if rq["gap_must"] else ""))
        row("Requirements failing or verified only by a person / a dirty build",
            f"{len(rq['failing'])} / {len(rq['person_only'])} / {len(rq['dirty_only'])}", not rq["failing"],
            "failing: " + (", ".join(rq["failing"]) or "none") + "; dirty only: " + (", ".join(rq["dirty_only"]) or "none"))
        row("Trace: requirements without an implementing issue (P11)", len(rq["no_issue"]), not rq["no_issue"],
            ", ".join(rq["no_issue"]))
        row("Pi rows naming a requirement", f"{len(rq['rows_naming'])} IDs", not rq["unknown_in_rows"],
            "unknown IDs in rows: " + (", ".join(rq["unknown_in_rows"]) or "none"))
    else:
        row("Test coverage per requirement", "–", True, f"no docs/requirements at `{k['req_rev']}`")

    sight = [f"| {link(repo, int(i))} {TRACKED[int(i)]} | {len(s)} | {', '.join(s[:8])} |"
             for i, s in k["sightings"].items()]
    return "\n".join([
        "# Quality KPIs",
        "",
        f"Generated by `scripts/quality-kpis.py` on {k['generated']} from `{k['rev']}`,",
        f"last {k['days']} days. Definitions and targets:",
        "[qmp.md](qmp.md#kpis). Do not edit by hand.",
        "",
        "| KPI | Value | Status | Details |",
        "|---|---|---|---|",
        *rows,
        "",
        "## Rows that name a tracked bug",
        "",
        f"Done rows in [queue.md](https://github.com/{k['tests_repo']}/blob/main/queue.md#done) that name the issue: sightings and repro attempts alike.",
        "",
        "| Bug | Rows | Which |",
        "|---|---|---|",
        *sight,
        "",
    ])


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--days", type=int, default=30)
    ap.add_argument("--repo", default="redox-rpi/redox-rpi")
    ap.add_argument("--rev", default="origin/main")
    ap.add_argument("--req-rev", help="revision to read docs/requirements from (default: --rev)")
    ap.add_argument("--tests-repo", default="redox-rpi/pi-tests", help="repository with queue.md")
    ap.add_argument("--out", default=os.path.join(ROOT, "docs", "process", "kpis.md"))
    ap.add_argument("--json")
    args = ap.parse_args()
    k = collect(args)
    with open(args.out, "w") as f:
        f.write(render(k, args.repo))
    if args.json:
        with open(args.json, "w") as f:
            json.dump(k, f, indent=1)
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
