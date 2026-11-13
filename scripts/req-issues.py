#!/usr/bin/env python3
"""Turn the requirement files into Feature issues in redox-rpi (RMP, Storage).

Usage:
    scripts/req-issues.py [--trace docs/requirements/trace-v0.4.0.md]
                          [--milestone v0.4.0] [--only REQ-NET-1,...] [--apply]

Reads every `### REQ-…` entry in docs/requirements/*.md and prints, for each,
the issue it would create or update: title, labels, milestone, state and
body. Nothing is changed without --apply. With --apply it uses `gh api` with
the token in GH_TOKEN (the zehrer-xyz account):

- an entry listed in EXISTING goes into that issue: the title gets the ID and
  the requirement block is put above the existing body;
- an issue whose title already starts with the ID is updated, not duplicated;
- otherwise a new issue of type Feature is created.

An entry whose trace state is plain "verified" is closed as completed with
the milestone; every other entry stays open. Run it once per area with
--only first and check the result on GitHub.
"""

import argparse
import glob
import json
import os
import re
import subprocess
import sys

REPO = "redox-rpi/redox-rpi"
EXISTING = {"REQ-AUDIO-1": 250, "REQ-TIME-3": 51}
TOPIC = {
    "BOOT": "boot", "CON": "boot", "USB": "usb", "NET": "network",
    "TIME": "time", "SD": "sdcard", "GPIO": "gpio", "FW": "firmware",
    "AUDIO": "audio", "GFX": "graphics", "TCC": "tcc", "STAB": "stability",
    "BOARD": "zero2w", "REL": "ci",
}
REQ_DIR = os.path.join(os.path.dirname(__file__), "..", "docs", "requirements")
BEGIN, END = "<!-- requirement -->", "<!-- /requirement -->"


def gh(*args, data=None):
    cmd = ["gh", "api", *args]
    if data is not None:
        cmd += ["--input", "-"]
    out = subprocess.run(cmd, input=json.dumps(data) if data is not None else None,
                         capture_output=True, text=True, check=True).stdout
    return json.loads(out) if out.strip() else None


def entries():
    for path in sorted(glob.glob(os.path.join(REQ_DIR, "*.md"))):
        if os.path.basename(path) == "README.md" or "trace" in path:
            continue
        text = open(path).read()
        for m in re.finditer(r"^### (REQ-([A-Z]+)-\d+) (.*?)\n(.*?)(?=^### |\Z)",
                             text, re.M | re.S):
            yield m.group(1), m.group(2), m.group(3), m.group(4).strip()


def trace_states(path):
    states = {}
    for m in re.finditer(r"^\| \[(REQ-[A-Z]+-\d+)\].*\| ([^|]*) \|$", open(path).read(), re.M):
        states[m.group(1)] = m.group(2).strip()
    return states


def body_of(rid, block, state, trace_name):
    statement, _, rest = block.partition("\n\n| Field |")
    table, _, criteria = ("| Field |" + rest).partition("\n\nAcceptance criteria:\n\n")
    fields = dict(re.findall(r"^\| ([^|]+?) \| (.*) \|$", table, re.M))
    rationale = fields.pop("Rationale", "")
    rows = ["| Field | Value |", "|---|---|"]
    fields["Status"] = "approved"  # the script runs only after the maintainer approved the set
    for key in ("Status", "Kind", "Priority", "Level", "PBS", "Since", "Implemented by",
                "Verification"):
        if key in fields:
            rows.append(f"| {key} | {fields[key]} |")
    rows.insert(3, "| Source | the v0.4.0 set, docs/requirements (approved with its PR) |")
    done = state == "verified"
    checks = re.sub(r"^(\d+)\. ", lambda m: f"- [{'x' if done else ' '}] {m.group(1)}. ",
                    criteria.strip(), flags=re.M)
    return "\n".join([
        BEGIN,
        f"**Statement:** {' '.join(statement.split())}", "", *rows, "",
        f"**Rationale:** {rationale}", "",
        "### Acceptance criteria", "", checks, "",
        "### Trace", "",
        f"State on v0.4.0: {state} ({trace_name}). "
        "Format: docs/process/rmp.md, Storage.",
        END,
    ])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--trace", default=os.path.join(REQ_DIR, "trace-v0.4.0.md"))
    ap.add_argument("--milestone", default="v0.4.0")
    ap.add_argument("--only", default="")
    ap.add_argument("--apply", action="store_true")
    a = ap.parse_args()
    only = set(filter(None, a.only.split(",")))
    states = trace_states(a.trace)
    trace_name = os.path.basename(a.trace)

    by_title, milestone = {}, None
    if a.apply:
        for issue in gh("--paginate", f"repos/{REPO}/issues?state=all&per_page=100"):
            m = re.match(r"(REQ-[A-Z]+-\d+) ", issue["title"])
            if m and "pull_request" not in issue:
                by_title[m.group(1)] = issue["number"]
        for ms in gh(f"repos/{REPO}/milestones?state=all&per_page=100"):
            if ms["title"] == a.milestone:
                milestone = ms["number"]

    for rid, area, name, block in entries():
        if only and rid not in only:
            continue
        state = states.get(rid, "unknown")
        body = body_of(rid, block, state, trace_name)
        title = f"{rid} {name}"
        close = state == "verified"
        number = EXISTING.get(rid) or by_title.get(rid)
        print(f"== {title}\n   issue: {number or 'new'}, label topic:{TOPIC[area]}, "
              f"milestone {a.milestone}, {'closed as completed' if close else 'open'}")
        if not a.apply:
            print(body + "\n")
            continue
        if number:
            old = gh(f"repos/{REPO}/issues/{number}")
            rest = re.sub(re.escape(BEGIN) + ".*?" + re.escape(END), "", old["body"] or "",
                          flags=re.S).strip()
            old_title = re.sub(r"^REQ-[A-Z]+-\d+ ", "", old["title"])
            new_title = title if rid not in EXISTING else f"{rid} {old_title}"
            gh("-X", "PATCH", f"repos/{REPO}/issues/{number}",
               data={"title": new_title, "body": body + ("\n\n" + rest if rest else "")})
        else:
            issue = gh("-X", "POST", f"repos/{REPO}/issues",
                       data={"title": title, "body": body, "type": "Feature",
                             "labels": [f"topic:{TOPIC[area]}"], "milestone": milestone})
            number = issue["number"]
        if close:
            gh("-X", "PATCH", f"repos/{REPO}/issues/{number}",
               data={"state": "closed", "state_reason": "completed", "milestone": milestone})
        print(f"   -> #{number}")


if __name__ == "__main__":
    sys.exit(main())
