#!/usr/bin/env python3
"""Weekly refresh of the CI image mirror, release ci-image (#409).

static.redox-os.org answers GitHub's runners with 403, so CI boots Redox's
daily aarch64 server image from our release ci-image. This script, run on
the build host (which can reach Redox's server), copies the newest daily
there once a week; scripts/update-stable.sh calls it on Mondays.

  scripts/refresh-ci-image.py [--dry-run] [--keep 2]

Steps: find the newest redox_server_aarch64_*_livedisk.iso.zst on
static.redox-os.org; stop if ci-image already has it; download it, check it
against Redox's SHA256SUM; upload it; rewrite the assets SHA256SUM (all
kept images) and latest.txt (the newest name, which the workflows read);
delete images beyond --keep; put the date, source and SHA-256 into the
release notes. Nothing else of the release changes; it stays a
pre-release.

GitHub: a token with contents write access to redox-rpi/redox-rpi only, in
$GH_TOKEN or ~/.config/redox-rpi/ci-image-token (mode 600; never printed,
never on a command line). Without one the script says so and exits 0.
"""

import argparse
import datetime
import hashlib
import json
import os
import re
import sys
import urllib.request

REPO = "redox-rpi/redox-rpi"
TAG = "ci-image"
DAILY = "https://static.redox-os.org/img/aarch64"
NAME = re.compile(r"redox_server_aarch64_[0-9-]+_[0-9]+_livedisk\.iso\.zst")
TOKEN_FILE = os.path.expanduser("~/.config/redox-rpi/ci-image-token")
API = f"https://api.github.com/repos/{REPO}"


def token():
    t = os.environ.get("GH_TOKEN")
    if not t and os.path.exists(TOKEN_FILE):
        t = open(TOKEN_FILE).read().strip()
    return t


def api(tok, method, url, data=None, ctype="application/json"):
    body = data if isinstance(data, (bytes, type(None))) else json.dumps(data).encode()
    req = urllib.request.Request(url, data=body, method=method, headers={
        "Authorization": f"Bearer {tok}", "Accept": "application/vnd.github+json",
        "Content-Type": ctype, "User-Agent": "redox-rpi-ci-image"})
    with urllib.request.urlopen(req, timeout=600) as r:
        raw = r.read()
        return json.loads(raw) if raw else None


def fetch(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "redox-rpi-ci-image"}),
                                timeout=600) as r:
        return r.read()


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--keep", type=int, default=2)
    args = ap.parse_args()
    tok = token()
    if not tok:
        print(f"no token ($GH_TOKEN or {TOKEN_FILE}), ci-image not refreshed")
        return 0
    names = sorted(set(NAME.findall(fetch(f"{DAILY}/").decode())))
    if not names:
        print("no daily image listed on static.redox-os.org")
        return 1
    newest = names[-1]
    rel = api(tok, "GET", f"{API}/releases/tags/{TAG}")
    assets = {a["name"]: a for a in rel["assets"]}
    if newest in assets:
        print(f"ci-image already has {newest}")
        return 0
    sums = fetch(f"{DAILY}/SHA256SUM").decode()
    want = next((l.split()[0] for l in sums.splitlines() if l.endswith(newest)), None)
    if not want:
        print(f"{newest} is not in Redox's SHA256SUM")
        return 1
    data = fetch(f"{DAILY}/{newest}")
    got = hashlib.sha256(data).hexdigest()
    if got != want:
        print(f"{newest}: SHA-256 {got}, Redox's SHA256SUM says {want}")
        return 1
    images = sorted([n for n in assets if NAME.fullmatch(n)] + [newest])
    keep, drop = images[-args.keep:], images[:-args.keep]
    # SHA-256 of the kept images: the new one computed, the others from the
    # current SHA256SUM asset.
    old = {}
    if "SHA256SUM" in assets:
        for l in fetch(assets["SHA256SUM"]["browser_download_url"]).decode().splitlines():
            if len(l.split()) == 2:
                old[l.split()[1]] = l.split()[0]
    old[newest] = got
    sumfile = "".join(f"{old[n]}  {n}\n" for n in keep if n in old)
    date = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    notes = (f"Not a release. A mirror for the QEMU boot test in CI (#409): Redox's daily aarch64 "
             f"server image, because static.redox-os.org and gitlab.redox-os.org answer GitHub's "
             f"runners with 403.\n\n- Newest: `{newest}`, from {DAILY}/{newest}\n"
             f"- Copied {date} by scripts/refresh-ci-image.py on the build host, checked against "
             f"Redox's SHA256SUM\n- SHA-256: `{got}`\n- Kept: {', '.join(f'`{n}`' for n in keep)}\n"
             f"- latest.txt names the newest; the workflows read it\n"
             f"- Redox OS, MIT and others (https://gitlab.redox-os.org/redox-os/redox)\n\n"
             f"Refreshed weekly (Mondays, from scripts/update-stable.sh).")
    print(f"upload {newest} ({len(data)} bytes, {got}); keep {keep}; delete {drop}")
    if args.dry_run:
        return 0
    up = rel["upload_url"].split("{")[0]

    def upload(name, blob, ctype):
        if name in assets:
            api(tok, "DELETE", f"{API}/releases/assets/{assets[name]['id']}")
        api(tok, "POST", f"{up}?name={name}", blob, ctype)

    upload(newest, data, "application/zstd")
    upload("SHA256SUM", sumfile.encode(), "text/plain")
    upload("latest.txt", (newest + "\n").encode(), "text/plain")
    for n in drop:
        if n in assets:
            api(tok, "DELETE", f"{API}/releases/assets/{assets[n]['id']}")
    api(tok, "PATCH", f"{API}/releases/{rel['id']}", {"body": notes, "prerelease": True})
    print(f"ci-image: {newest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
