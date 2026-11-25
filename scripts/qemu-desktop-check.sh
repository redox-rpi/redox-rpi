#!/usr/bin/env bash
# Desktop smoke test in QEMU raspi3b (#261): boot a desktop image, start the
# desktop from the console as a user would (`desktop`), log in at Orbital's
# login window (Enter, user `user` with the empty password), and check:
#
#   - no kernel panic and no Rust panic on the serial console;
#   - `orbital` and the launcher are still running about a minute later;
#   - the screen shows the desktop: a screenshot through the QEMU monitor
#     with the launcher bar along the bottom edge, many colours and less
#     than 85 % dark, unlike the text console before (all dark, two
#     colours). The session's welcome window (a dark terminal) may cover
#     much of a small screen, so the bar and the colours count most.
#
# Usage:
#   scripts/qemu-desktop-check.sh [--image <harddrive.img>] [--out <dir>] [--timeout <s>]
#
#   --image   default: the network-boot image of scripts/build-image.sh, which
#             must have been built with the desktop experiments
#             (rpi-fb orbital orbital-apps desktop, as in config/visit-images.conf)
#   --out     where the results go (default images/desktop-check): report.md,
#             console.png (before), desktop.png (after the login; .ppm
#             without ffmpeg), serial.log
#   --timeout for the whole run (default 900)
#
# Exit status: 0 pass, 1 a check failed, 2 the run itself failed (no boot).
# QEMU's framebuffer is the firmware mailbox model, so this checks Orbital,
# rpi-fbd, the launcher and the session on top of it, not HDMI.

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img"
out="$ROOT/images/desktop-check"
timeout=900
while [ $# -gt 0 ]; do
    case "$1" in
        --image) image="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --timeout) timeout="$2"; shift 2 ;;
        -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
[ -f "$image" ] || { echo "no image $image" >&2; exit 2; }
image="$(cd "$(dirname "$image")" && pwd)/$(basename "$image")"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
rm -f "$out"/report.md "$out"/*.ppm "$out"/*.png "$out"/serial.log
monitor="$out/monitor.sock"
log="$ROOT/images/qemu-test.log"

# Seconds after `desktop` before Enter at the login window, and after that
# before the screenshot. Orbital and the launcher need about 20 s each in
# QEMU on the build host.
LOGIN_WAIT="${LOGIN_WAIT:-40}"
DESKTOP_WAIT="${DESKTOP_WAIT:-45}"
hold=$((LOGIN_WAIT + 45 + DESKTOP_WAIT + 30))

mon() {
    python3 -c 'import socket, sys, time
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); time.sleep(0.3)
s.sendall((sys.argv[2] + "\n").encode()); time.sleep(1.5)' "$monitor" "$1"
}
shot() {
    mon "screendump $out/$1.ppm"
    sleep 1
    if [ -s "$out/$1.ppm" ] && command -v ffmpeg > /dev/null; then
        ffmpeg -loglevel error -y -i "$out/$1.ppm" "$out/$1.png" || true
    fi
}

# The serial session: log in, start the desktop, keep QEMU up while the
# keys and screenshots go in through the monitor, then list the processes.
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --monitor "$monitor" --timeout "$timeout" \
    --run "desktop" --run "sleep $hold" --run "ps" --run "free" > "$out/qemu.out" 2>&1 &
qpid=$!

start=$SECONDS
started=no
while kill -0 $qpid 2> /dev/null; do
    if [ "$started" = no ] && grep -aq "desktop; sleep $hold" "$log" 2> /dev/null; then
        started=yes
        sleep 5
        shot console
        sleep "$LOGIN_WAIT"
        # Enter at the login window (user `user`, empty password). Orbital
        # may still be starting, so press it three times: after the login
        # the extra ones go to the welcome terminal, which does no harm.
        for _ in 1 2 3; do
            mon "sendkey ret"
            sleep 15
        done
        sleep "$DESKTOP_WAIT"
        shot desktop
    fi
    [ $((SECONDS - start)) -lt $((timeout + 60)) ] || { kill $qpid; break; }
    sleep 2
done
wait $qpid
qstatus=$?
cp "$log" "$out/serial.log" 2> /dev/null || true

if [ "$started" = no ]; then
    { echo "## Desktop check (QEMU raspi3b): no run"; echo
      echo "The image did not reach the shell (qemu-raspi3b.sh exit $qstatus):"; echo
      echo '```'; tail -5 "$out/qemu.out"; echo '```'; } > "$out/report.md"
    cat "$out/report.md"
    exit 2
fi

python3 - "$out" "$image" "$qstatus" <<'EOF'
import os, re, sys

out, image, qstatus = sys.argv[1], sys.argv[2], int(sys.argv[3])
log = open(os.path.join(out, "serial.log"), "rb").read().decode("utf-8", "replace")
log = re.sub(r"\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07", "", log)

def ppm(path):
    """Width, height and RGB bytes of a binary PPM (P6, maxval 255)."""
    data = open(path, "rb").read()
    fields, pos = [], 0
    while len(fields) < 4:
        while data[pos:pos + 1].isspace():
            pos += 1
        if data[pos:pos + 1] == b"#":
            pos = data.index(b"\n", pos) + 1
            continue
        end = pos
        while not data[end:end + 1].isspace():
            end += 1
        fields.append(data[pos:end])
        pos = end
    assert fields[0] == b"P6" and fields[3] == b"255", "not a P6 PPM"
    w, h = int(fields[1]), int(fields[2])
    return w, h, data[pos + 1:pos + 1 + 3 * w * h]

def screen(path):
    """Numbers that tell Orbital's desktop from the text console."""
    w, h, rgb = ppm(path)
    dark = colours = 0
    seen = set()
    step = 4
    for y in range(0, h, step):
        row = y * w * 3
        for x in range(0, w, step):
            r, g, b = rgb[row + 3 * x:row + 3 * x + 3]
            if r + g + b < 48:
                dark += 1
            seen.add((r >> 4, g >> 4, b >> 4))
    samples = len(range(0, h, step)) * len(range(0, w, step))
    # The launcher bar (about 48 px high): the bottom rows are one colour
    # across the screen (apart from the icons), and a different one than
    # the rows just above it. On the login screen the wallpaper reaches
    # the bottom edge, so both bands have the same colour.
    def band(y0, y1):
        px = [tuple(rgb[(y * w + x) * 3:(y * w + x) * 3 + 3]) for y in range(y0, y1, 2) for x in range(0, w, 8)]
        common = max(set(px), key=px.count)
        return common, px.count(common) / len(px)
    bar_colour, bar_share = band(h - 6, h - 2)
    above_colour, _ = band(h - 60, h - 54)
    return {
        "size": f"{w}x{h}",
        "dark": dark / samples,
        "colours": len(seen),
        "bar_colour": bar_colour,
        "bar_share": bar_share,
        "above_colour": above_colour,
        "bar_contrast": sum(abs(a - b) for a, b in zip(bar_colour, above_colour)),
    }

checks = []
def check(name, ok, detail):
    checks.append((name, ok, detail))

panic = re.search(r"KERNEL PANIC|panicked at", log)
check("no panic", not panic, panic.group(0) if panic else "none on the serial console")

ps = log[log.rfind("desktop; sleep"):] if "desktop; sleep" in log else log
names = ["orbital", "launcher"]
running = {n: bool(re.search(r"\b" + n + r"\b", ps.split("RUN-DONE-42")[0] if "RUN-DONE-42" in ps else ps)) for n in names}
check("orbital and launcher running", all(running.values()),
      ", ".join(f"{n} {'yes' if v else 'NO'}" for n, v in running.items()) + " in `ps` after the wait")

shotfile = os.path.join(out, "desktop.ppm")
if os.path.exists(shotfile) and os.path.getsize(shotfile) > 0:
    s = screen(shotfile)
    c = screen(os.path.join(out, "console.ppm")) if os.path.exists(os.path.join(out, "console.ppm")) else None
    desktop = (s["dark"] < 0.85 and s["colours"] >= 12 and s["bar_share"] >= 0.5
               and sum(s["bar_colour"]) > 48 and s["bar_contrast"] >= 40)
    hexc = lambda c: "#" + "".join(f"{v:02x}" for v in c)
    detail = (f"{s['size']}, {s['dark']:.0%} dark, {s['colours']} colours, bottom bar "
              f"{hexc(s['bar_colour'])} on {s['bar_share']:.0%} of the edge, above it "
              f"{hexc(s['above_colour'])} (difference {s['bar_contrast']})")
    if c:
        detail += f" (console before: {c['dark']:.0%} dark, {c['colours']} colours)"
    check("screen shows the desktop", desktop, detail)
else:
    check("screen shows the desktop", False, "no screenshot")

mem = re.findall(r"^Mem:.*$", log, re.M)
ok = all(c[1] for c in checks)
lines = [f"## Desktop check (QEMU raspi3b): {'PASS' if ok else 'FAIL'}", "",
         f"Image `{image}`.", "", "| Check | Result | Detail |", "|---|---|---|"]
lines += [f"| {n} | {'pass' if o else '**FAIL**'} | {d} |" for n, o, d in checks]
if mem:
    lines += ["", f"`free` after the wait: `{mem[-1].strip()}`"]
shots = [f for f in ("desktop.png", "console.png") if os.path.exists(os.path.join(out, f))]
if shots:
    lines += ["", "Screenshots: " + ", ".join(f"`{f}`" for f in shots)]
open(os.path.join(out, "report.md"), "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
sys.exit(0 if ok else 1)
EOF
