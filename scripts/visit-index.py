#!/usr/bin/env python3
"""Write the index of the nightly visit images for a tester at the Pi.

Called by scripts/build-visit-images.sh after each image:

    scripts/visit-index.py <out dir> <config> <test-plan.md> [<sdcard dir>]

Writes <out dir>/index.html and <out dir>/README.txt (served as
http://homenodellm:8080/visit/): per image the result, build ID, date, how
to stage it for network boot, the SD image with its checksum or why there is
none, and the queued rows of docs/test-plan.md that name the image
("nightly visit `<name>`") with what they check.
"""

import html
import os
import re
import sys

HTTP = "http://homenodellm:8080"


def read(path):
    try:
        with open(path) as f:
            return f.read()
    except OSError:
        return ""


def kv(text):
    """key = value lines (visit.txt)."""
    out = {}
    for line in text.splitlines():
        key, sep, value = line.partition(" = ")
        if sep:
            out[key.strip()] = value.strip()
    return out


def images(conf):
    """(name, owner, sdcard, note) per config line; note is the # comment."""
    for line in read(conf).splitlines():
        line, _, note = line.partition("#")
        words = line.split()
        if len(words) < 2 or words[0] == "deadline":
            continue
        yield words[0], words[1], "--sdcard" in words, note.strip()


def status(out):
    """name -> (result, build ID, commit, ref, date) from status.txt."""
    res = {}
    for line in read(os.path.join(out, "status.txt")).splitlines():
        if line.startswith("#"):
            continue
        words = line.split()
        if len(words) >= 6:
            res[words[0]] = words[1:6]
    return res


def rows(plan, name):
    """Queue rows of the test plan that name this visit image."""
    queue = plan.split("\n## Queue", 1)[-1].split("\n## Done", 1)[0]
    mark = re.compile(r"nightly visit `%s`" % re.escape(name), re.I)
    found = []
    for line in queue.splitlines():
        if not line.startswith("| ") or not mark.search(line):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split(" | ")]
        if len(cells) >= 6:
            found.append({"nr": cells[0], "test": cells[1], "owner": cells[2],
                          "needs": cells[4], "status": cells[5]})
    return found


def md(text):
    """Markdown of a table cell to HTML: links, bold, code."""
    out = html.escape(text, quote=False)
    out = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)",
                 lambda m: '<a href="%s">%s</a>' % (html.escape(m.group(2)), m.group(1)), out)
    out = re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", out)
    out = re.sub(r"`([^`]+)`", r"<code>\1</code>", out)
    return out


def plain(text):
    text = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)", r"\1 (\2)", text)
    return text.replace("**", "").replace("`", "")


def state(e):
    if e["ok"]:
        return "OK"
    if e["build_id"]:
        return "last build %s, the older image below still works" % e["result"]
    return "FAILED: " + e["result"]


def main():
    out, conf, plan_file = sys.argv[1:4]
    sdcard = sys.argv[4] if len(sys.argv) > 4 else "/srv/tftp/sdcard"
    plan = read(plan_file)
    st = status(out)
    header = [l[2:] for l in read(os.path.join(out, "status.txt")).splitlines()[1:2]]
    entries = []
    for name, owner, has_sd, note in images(conf):
        latest = os.path.join(out, name, "latest")
        info = kv(read(os.path.join(latest, "visit.txt")))
        build_id = read(os.path.join(latest, "BUILD_ID")).strip()
        result = st.get(name, ["not built yet", "-", "-", "-", "-"])
        ok = result[0] in ("built", "unchanged")
        sd = None
        if has_sd:
            base = "redox-rpi-visit-%s-latest.img.xz" % name
            target = os.path.realpath(os.path.join(sdcard, base))
            if os.path.exists(target):
                sd = {"file": os.path.basename(target),
                      "sha256": read(target + ".sha256").split(" ")[0]}
        entries.append({"name": name, "owner": owner, "ok": ok, "result": result[0],
                        "build_id": build_id, "info": info, "has_sd": has_sd, "sd": sd,
                        "note": note, "rows": rows(plan, name)})

    # README.txt
    lines = ["Nightly visit images: test images for a person at the Pi", ""]
    lines += header + [""]
    for e in entries:
        lines.append("== %s (%s) ==" % (e["name"], state(e)))
        if e["build_id"]:
            lines.append("newest image:  %s, built %s" % (e["build_id"], e["info"].get("built", "?")))
            lines.append("from:          %s, commit %s" % (e["info"].get("ref", "?"), e["info"].get("commit", "?")[:9]))
            lines.append("options:       %s" % e["info"].get("options", ""))
            lines.append("network boot:  cp -p /srv/tftp/visit/%s/latest/redox/* /srv/tftp/redox/" % e["name"])
        else:
            lines.append("no image yet")
        if e["sd"]:
            lines.append("SD card:       %s/sdcard/%s" % (HTTP, e["sd"]["file"]))
            lines.append("sha256:        %s" % e["sd"]["sha256"])
        else:
            lines.append("SD card:       none (%s)" % (e["note"] or ("not built yet" if e["has_sd"] else "not in the config")))
        lines.append("owner topic:   %s" % e["owner"])
        if e["rows"]:
            for r in e["rows"]:
                lines.append("test %s (%s, %s): %s" % (r["nr"], plain(r["owner"]), plain(r["status"]), plain(r["test"])))
                lines.append("  needs: %s" % plain(r["needs"]))
        else:
            lines.append("no queued test names \"nightly visit `%s`\" in docs/test-plan.md" % e["name"])
        lines.append("")

    # index.html
    h = ['<!doctype html><meta charset="utf-8">',
         '<meta name="viewport" content="width=device-width, initial-scale=1">',
         "<title>Visit images</title>",
         "<style>body{font:15px/1.45 system-ui,sans-serif;max-width:60rem;margin:1rem auto;padding:0 1rem}"
         "code,pre{background:#eee;padding:0 .2em;overflow-wrap:anywhere}pre{padding:.4em;white-space:pre-wrap}"
         ".ok{color:#070}.bad{color:#b00}section{border-top:1px solid #ccc;margin-top:1.2rem}"
         "td{vertical-align:top;padding:.2em .5em}</style>",
         "<h1>Nightly visit images</h1>",
         "<p>Test images for a person at the Pi, built every night after the stable image "
         "(docs/topics/ci.md, &ldquo;Nightly visit images&rdquo;).</p>"]
    h += ["<p><small>%s</small></p>" % html.escape(x) for x in header]
    for e in entries:
        cls, word = ("ok" if e["ok"] else "bad"), state(e)
        h.append('<section><h2>%s <span class="%s">%s</span></h2>' % (html.escape(e["name"]), cls, html.escape(word)))
        if e["build_id"]:
            h.append("<p>Newest image <code>%s</code>, built %s from %s, commit <code>%s</code>.<br>Options: <code>%s</code></p>" % (
                html.escape(e["build_id"]), html.escape(e["info"].get("built", "?")),
                html.escape(e["info"].get("ref", "?")), html.escape(e["info"].get("commit", "?")[:9]),
                html.escape(e["info"].get("options", ""))))
            h.append("<p>Network boot: stage it on the build host, then reset the Pi</p><pre>cp -p /srv/tftp/visit/%s/latest/redox/* /srv/tftp/redox/</pre>" % html.escape(e["name"]))
        else:
            h.append("<p>No image yet.</p>")
        if e["sd"]:
            url = "%s/sdcard/%s" % (HTTP, e["sd"]["file"])
            h.append('<p>SD card: <a href="%s">%s</a><br>sha256 <code>%s</code></p>' % (
                html.escape(url), html.escape(e["sd"]["file"]), html.escape(e["sd"]["sha256"])))
        else:
            h.append("<p>SD card: none (%s)</p>" % html.escape(e["note"] or ("not built yet" if e["has_sd"] else "not in the config")))
        if e["rows"]:
            h.append("<h3>Tests waiting for this image</h3><table>")
            for r in e["rows"]:
                h.append("<tr><td><b>%s</b></td><td>%s<br><small>Owner: %s. Needs: %s. Status: %s</small></td></tr>" % (
                    html.escape(r["nr"]), md(r["test"]), md(r["owner"]), md(r["needs"]), md(r["status"])))
            h.append("</table>")
        else:
            h.append("<p>No queued test in docs/test-plan.md names <code>nightly visit `%s`</code>.</p>" % html.escape(e["name"]))
        h.append("<p><small>Owner topic: %s</small></p></section>" % html.escape(e["owner"]))

    for fname, text in (("README.txt", "\n".join(lines)), ("index.html", "\n".join(h) + "\n")):
        tmp = os.path.join(out, fname + ".tmp")
        with open(tmp, "w") as f:
            f.write(text)
        os.replace(tmp, os.path.join(out, fname))


if __name__ == "__main__":
    main()
