#!/usr/bin/env python3
"""Code metrics for the coding rules (docs/process/coding-standards.md).

Usage: scripts/code-metrics.py [--json] <label>=<dir> [<label>=<dir> ...]

Reads the source files below each directory and prints, per label: code
lines (no blank or comment lines) per language, the comment/code ratio,
files over the size limit, functions over 60 lines, duplicated lines,
unit tests, `unsafe` uses, and REQ- and issue references. Generated and
vendor code is skipped (SKIP_DIRS). scripts/quality-kpis.py --code uses
measure() for the KPI dashboard (#648).

The numbers are rough on purpose: functions are found by a brace scan
(Rust, C) or by indentation (Python), and a duplicated line is any line of
a 6-line window that appears twice after whitespace is normalised (windows
with less than 120 characters are ignored). Docstrings count as code.
"""

import collections
import hashlib
import json
import os
import re
import sys

EXT = {".rs": "rust", ".c": "c", ".h": "c", ".sh": "shell", ".py": "python", ".S": "asm", ".s": "asm"}
SKIP_DIRS = {"target", ".git", "node_modules", "vendor", "upstream", "build", "broadcom"}
FILE_LIMIT = {"rust": 600, "c": 600, "shell": 400, "python": 400, "asm": 800}
FUNC_LIMIT = 60
WINDOW, WINDOW_CHARS = 6, 120
RUST_FN = re.compile(r"^\s*(pub(\([\w:]+\))?\s+)?(const\s+)?(async\s+)?(unsafe\s+)?(extern\s+\"C\"\s+)?fn\s+\w+")
C_FN = re.compile(r"^[A-Za-z_][\w\s\*]*\b\w+\s*\([^;]*$")
REQ_RE = re.compile(r"REQ-[A-Z]+-\d+")
ISSUE_RE = re.compile(r"#\d{2,4}\b")


def sources(root):
    """(path, language) of every source file below root, shell scripts
    without an extension included."""
    for d, dirs, files in os.walk(root):
        dirs[:] = sorted(x for x in dirs if x not in SKIP_DIRS)
        for f in sorted(files):
            path = os.path.join(d, f)
            lang = EXT.get(os.path.splitext(f)[1])
            if lang is None and "." not in f:
                try:
                    with open(path, "rb") as fh:
                        head = fh.read(64)
                except OSError:
                    continue
                lang = "shell" if head.startswith(b"#!") and b"sh" in head.split(b"\n")[0] else None
            if lang:
                yield path, lang


def is_comment(lang, line):
    s = line.strip()
    if lang in ("rust", "c", "asm"):
        return s.startswith(("//", "/*", "*"))
    return s.startswith("#") and not s.startswith("#!")


def brace_functions(lines, pattern):
    """Lengths of the functions that start at lines matching pattern."""
    out, i = [], 0
    while i < len(lines):
        if not pattern.match(lines[i]):
            i += 1
            continue
        depth, opened, j = 0, False, i
        while j < len(lines):
            depth += lines[j].count("{") - lines[j].count("}")
            opened = opened or "{" in lines[j]
            if (opened and depth <= 0) or (not opened and lines[j].rstrip().endswith(";")):
                break
            j += 1
        if opened:
            out.append(j - i + 1)
        i = j + 1
    return out


def indent_functions(lines):
    """Lengths of the Python functions, by indentation."""
    out = []
    for i, line in enumerate(lines):
        if not re.match(r"\s*def ", line):
            continue
        indent, j = len(line) - len(line.lstrip()), i + 1
        while j < len(lines) and (not lines[j].strip() or len(lines[j]) - len(lines[j].lstrip()) > indent):
            j += 1
        out.append(j - i)
    return out


def function_lengths(lang, lines):
    if lang == "rust":
        return brace_functions(lines, RUST_FN)
    if lang == "c":
        return brace_functions(lines, C_FN)
    if lang == "python":
        return indent_functions(lines)
    return []


def count_tests(lang, text):
    if lang == "rust":
        return text.count("#[test]")
    if lang == "python":
        return len(re.findall(r"^\s*def test_", text, re.M))
    return 0


def duplicated(windows):
    """Lines that belong to a 6-line window seen more than once."""
    dup = set()
    for places in windows.values():
        if len(places) > 1:
            dup.update((p, k + x) for p, k in places for x in range(WINDOW))
    return len(dup)


def measure(root):
    """The metrics of one directory, as a dict (see the module doc)."""
    langs = collections.defaultdict(collections.Counter)
    windows = collections.defaultdict(list)
    big, refs = [], collections.Counter()
    for path, lang in sources(root):
        try:
            with open(path, errors="replace") as fh:
                text = fh.read()
        except OSError:
            continue
        lines = text.split("\n")
        code = [line for line in lines if line.strip() and not is_comment(lang, line)]
        c = langs[lang]
        c["files"] += 1
        c["code"] += len(code)
        c["comment"] += sum(1 for line in lines if line.strip() and is_comment(lang, line))
        lengths = function_lengths(lang, lines)
        c["functions"] += len(lengths)
        c["long_functions"] += sum(1 for n in lengths if n > FUNC_LIMIT)
        if len(code) > FILE_LIMIT[lang]:
            big.append((os.path.relpath(path, root), len(code)))
        refs["req"] += len(REQ_RE.findall(text))
        refs["issue"] += len(ISSUE_RE.findall(text))
        refs["unsafe"] += len(re.findall(r"\bunsafe\b", text)) if lang == "rust" else 0
        refs["tests"] += count_tests(lang, text)
        norm = [re.sub(r"\s+", " ", line.strip()) for line in code]
        for k in range(len(norm) - WINDOW + 1):
            w = norm[k:k + WINDOW]
            if sum(map(len, w)) >= WINDOW_CHARS:
                windows[hashlib.md5("\n".join(w).encode()).hexdigest()].append((path, k))
    total = sum(c["code"] for c in langs.values())
    dup = duplicated(windows)
    return {
        "code": total,
        "by_language": {lang: dict(c) for lang, c in sorted(langs.items())},
        "duplicated": dup,
        "duplicated_pct": round(100 * dup / max(total, 1), 1),
        "over_limit": sorted(big, key=lambda x: -x[1]),
        "long_functions": sum(c["long_functions"] for c in langs.values()),
        "tests": refs["tests"],
        "unsafe": refs["unsafe"],
        "req_refs": refs["req"],
        "issue_refs": refs["issue"],
    }


def main():
    args = sys.argv[1:]
    as_json = "--json" in args
    pairs = [a.split("=", 1) for a in args if a != "--json"]
    if not pairs or any(len(p) != 2 for p in pairs):
        sys.exit(__doc__.split("\n\n")[1])
    result = {label: measure(path) for label, path in pairs}
    if as_json:
        print(json.dumps(result, indent=1))
        return
    for label, m in result.items():
        print(f"{label}: {m['code']} code lines, {m['duplicated_pct']} % duplicated, "
              f"{len(m['over_limit'])} files over the limit, {m['long_functions']} functions over "
              f"{FUNC_LIMIT} lines, {m['tests']} tests, {m['unsafe']} unsafe, {m['req_refs']} REQ- refs")


if __name__ == "__main__":
    main()
