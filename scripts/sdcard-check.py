#!/usr/bin/env python3
"""Check that SwiftFlash would find and accept a published SD card image.

    scripts/sdcard-check.py <sdcard dir> <family> [<file>] [--url <base>]

SwiftFlash (`sudo swiftflash redox:<family>`) reads index.json, takes the
newest image of the family and checks the download against its .sha256.
This script checks the same things on the image server's directory:

- index.json parses and has an entry for the image with file, family,
  version and date, and sha256 true;
- the image is the one SwiftFlash picks for the family: with <file>, it
  must be the newest entry of the family; without, the newest is checked;
- <file>.sha256 is in sha256sum format, names the file, and matches the
  file's real checksum;
- <file>.manifest.toml is there and is valid TOML.

With --url (the image server's /sdcard/ URL), also over HTTP: index.json
has the same entry, the image answers with the same size, and the .sha256
is the same as on disk.

Prints one line per check and exits 1 if any fails. Run by
scripts/publish-sdcard.sh after each image and by its --check.
"""

import hashlib
import json
import os
import sys
import tomllib
import urllib.request

failures = 0


def ok(text):
    print("  ok    " + text)


def fail(text):
    global failures
    failures += 1
    print("  FAIL  " + text)


def newest(entries):
    # SwiftFlash takes the newest build of a family: by date, then version.
    return max(entries, key=lambda e: (e.get("date", ""), e.get("version", "")))


def entry_ok(entry, family, where):
    missing = [k for k in ("file", "family", "version", "date") if not entry.get(k)]
    if missing:
        fail("%s: entry for %s lacks %s" % (where, entry.get("file"), ", ".join(missing)))
    elif entry["family"] != family:
        fail("%s: entry for %s has family %s" % (where, entry["file"], entry["family"]))
    elif not entry.get("sha256"):
        fail("%s: entry for %s says no .sha256" % (where, entry["file"]))
    else:
        ok("%s: %s, version %s, %s" % (where, entry["file"], entry["version"], entry["date"]))


def fetch(url, method="GET"):
    req = urllib.request.Request(url, method=method)
    return urllib.request.urlopen(req, timeout=30)


def main():
    args = sys.argv[1:]
    url = None
    if "--url" in args:
        i = args.index("--url")
        url = args[i + 1].rstrip("/")
        del args[i:i + 2]
    if len(args) not in (2, 3):
        sys.exit(__doc__.split("\n\n")[1])
    top, family = args[0], args[1]
    want = args[2] if len(args) == 3 else None

    print("check: swiftflash redox:%s" % family)
    try:
        with open(os.path.join(top, "index.json")) as f:
            index = json.load(f)
    except (OSError, ValueError) as e:
        fail("index.json: %s" % e)
        return summary()
    entries = [e for e in index.get("images", []) if e.get("family") == family]
    if not entries:
        fail("index.json: no image of family %s" % family)
        return summary()
    pick = newest(entries)
    if want and pick["file"] != want:
        mine = [e for e in entries if e["file"] == want]
        if not mine:
            fail("index.json: no entry for %s" % want)
            return summary()
        fail("swiftflash would take %s (date %s), not %s (date %s)"
             % (pick["file"], pick.get("date"), want, mine[0].get("date")))
        pick = mine[0]
    entry_ok(pick, family, "index.json")
    name = pick["file"]
    path = os.path.join(top, name)

    try:
        line = open(path + ".sha256").read().split()
        digest = hashlib.sha256()
        with open(path, "rb") as f:
            for block in iter(lambda: f.read(1 << 20), b""):
                digest.update(block)
        if len(line) != 2 or line[1].lstrip("*") != name:
            fail("%s.sha256 does not name %s in sha256sum format" % (name, name))
        elif line[0] != digest.hexdigest():
            fail("%s.sha256 says %s, the file is %s" % (name, line[0][:12], digest.hexdigest()[:12]))
        else:
            ok("checksum %s matches" % line[0][:12])
    except OSError as e:
        fail("checksum: %s" % e)

    try:
        with open(path + ".manifest.toml", "rb") as f:
            tomllib.load(f)
        ok("manifest %s.manifest.toml" % name)
    except OSError:
        fail("no %s.manifest.toml" % name)
    except tomllib.TOMLDecodeError as e:
        fail("%s.manifest.toml is not valid TOML: %s" % (name, e))

    if url:
        try:
            remote = json.load(fetch(url + "/index.json"))
            same = [e for e in remote.get("images", []) if e.get("file") == name]
            if same:
                entry_ok(same[0], family, "HTTP index.json")
            else:
                fail("HTTP index.json: no entry for %s" % name)
            size = int(fetch("%s/%s" % (url, name), "HEAD").headers.get("Content-Length", -1))
            if size == os.path.getsize(path):
                ok("HTTP image: %d bytes" % size)
            else:
                fail("HTTP image: %d bytes, on disk %d" % (size, os.path.getsize(path)))
            if fetch("%s/%s.sha256" % (url, name)).read().decode() == open(path + ".sha256").read():
                ok("HTTP .sha256 as on disk")
            else:
                fail("HTTP .sha256 differs from the one on disk")
        except (OSError, ValueError) as e:
            fail("HTTP %s: %s" % (url, e))

    return summary()


def summary():
    print("check: %s" % ("FAIL, %d problem(s)" % failures if failures else "pass"))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
