#!/usr/bin/env python3
"""Write index.json for the SD card images on the image server.

    scripts/sdcard-index.py <sdcard dir> [<test-plan.md>]

SwiftFlash (github.com/zehrer/SwiftFlashCLI, docs/build-server.md) reads it
instead of guessing from file names: per published image file, family,
version and build date. Our own fields, which SwiftFlash ignores: board,
commit, build_id, experiments, owner, and "rows", the queued tests in
docs/test-plan.md that name the image.

Only real files count: the -latest symlinks are for people, not for
SwiftFlash, which finds the newest build by itself. Sources of the fields,
first match wins: <file>.meta.json (written by scripts/publish-sdcard.sh),
the image's manifest (<stem>.manifest.toml or <file>.manifest.toml), the
file name, the file's modification time. Written atomically and only when it changes; run
by scripts/publish-sdcard.sh after each image and by the systemd user unit
redox-sdcard-index.path whenever the directory changes (so an image copied in
by hand is not hidden from SwiftFlash, which reads only index.json when there
is one); safe to run by hand.
"""

import datetime
import json
import os
import re
import sys
import tomllib

# The version rule of build-server.md: v<major>.<minor>… or YYYYMMDD….
NAME = re.compile(r"^(?P<family>.+?)-(?P<version>v\d+\.\d+.*|\d{8}.*)\.(?:img|iso)(?:\.xz)?$")
# Older names with only a commit hash as version.
HASH = re.compile(r"^(?P<family>.+)-(?P<version>[0-9a-f]{7,12})\.(?:img|iso)(?:\.xz)?$")
IMAGE = re.compile(r"\.(?:img|iso)(?:\.xz)?$")


def iso(ts):
    return datetime.datetime.fromtimestamp(ts, datetime.timezone.utc) \
        .isoformat(timespec="seconds").replace("+00:00", "Z")


def manifest_for(path):
    stem = IMAGE.sub("", path)
    for cand in (stem + ".manifest.toml", stem + ".img.manifest.toml", path + ".manifest.toml"):
        try:
            with open(cand, "rb") as f:
                return tomllib.load(f)
        except (OSError, tomllib.TOMLDecodeError):
            continue
    return {}


def board(name, manifest):
    text = (manifest.get("config", "") + " " + name).lower()
    if "zero2w" in text:
        return "zero2w"
    if "raspi3bp" in text or "3bp" in text or "pi3" in text:
        return "rpi3"
    return None


def queue_rows(plan):
    queue = plan.split("\n## Queue", 1)[-1].split("\n## Done", 1)[0]
    rows = []
    for line in queue.splitlines():
        if not line.startswith("| "):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split(" | ")]
        if len(cells) >= 6 and cells[0].isdigit():
            rows.append((line, cells))
    return rows


def plain(text, limit=300):
    text = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)", r"\1", text).replace("**", "").replace("`", "")
    return text if len(text) <= limit else text[:limit - 1] + "…"


def main():
    top = sys.argv[1]
    plan = ""
    if len(sys.argv) > 2:
        try:
            with open(sys.argv[2]) as f:
                plan = f.read()
        except OSError:
            pass
    rows = queue_rows(plan)
    images = []
    for name in sorted(os.listdir(top)):
        path = os.path.join(top, name)
        if not IMAGE.search(name) or os.path.islink(path) or not os.path.isfile(path):
            continue
        try:
            with open(path + ".meta.json") as f:
                meta = json.load(f)
        except (OSError, ValueError):
            meta = {}
        manifest = manifest_for(path)
        match = NAME.match(name) or HASH.match(name)
        entry = {"file": name}
        family = meta.get("family") or (match and match["family"])
        version = meta.get("version") or (match and match["version"])
        if family:
            entry["family"] = family
        if version:
            entry["version"] = version
        entry["date"] = meta.get("date") or manifest.get("built") or iso(os.stat(path).st_mtime)
        extra = {
            "board": meta.get("board") or board(name, manifest),
            "commit": meta.get("commit") or (manifest.get("commit") or "")[:12] or None,
            "build_id": meta.get("build_id") or manifest.get("build_id"),
            "experiments": meta.get("experiments") or manifest.get("experiments"),
            "owner": meta.get("owner"),
            "sha256": os.path.exists(path + ".sha256") or None,
        }
        entry.update({k: v for k, v in extra.items() if v})
        # "redox-rpi" alone (the release family) is in every row's links.
        names = [n for n in (family if family != "redox-rpi" else None,
                             meta.get("visit") and "nightly visit `%s`" % meta["visit"]) if n]
        found = [{"row": int(c[0]), "test": plain(c[1]), "needs": plain(c[4], 120), "status": plain(c[5], 80)}
                 for line, c in rows
                 if name in line or any(re.search(r"(?<![\w/-])%s(?![\w/-])" % re.escape(n), line) for n in names)]
        if found:
            entry["rows"] = found
        images.append(entry)

    text = json.dumps({"images": images}, indent=2, ensure_ascii=False) + "\n"
    path = os.path.join(top, "index.json")
    try:
        with open(path) as f:
            if f.read() == text:
                # No write: redox-sdcard-index.path would start this again.
                print("index.json: %d images, unchanged" % len(images))
                return
    except OSError:
        pass
    tmp = os.path.join(top, ".index.json.tmp")
    with open(tmp, "w") as f:
        f.write(text)
    os.replace(tmp, path)
    print("index.json: %d images" % len(images))


if __name__ == "__main__":
    main()
