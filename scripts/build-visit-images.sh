#!/usr/bin/env bash
# Nightly "visit" images: the images in config/visit-images.conf for tests
# with a person at the Pi, built after the stable image so they are ready
# when the person comes (docs/topics/ci.md, "Nightly visit images").
#
# Runs on the build host as the systemd user unit redox-visit.service, which
# scripts/update-stable.sh starts when the stable image passed or nothing
# changed since it. Or by hand:
#
#   scripts/build-visit-images.sh [--force] [--only <name>] [--deadline <HH:MM>]
#
#   --force     build even if the newest image has the same commit and options
#   --only      build only this image
#   --deadline  instead of the deadline in config/visit-images.conf
#
# Builds in its own clone (VISIT_CLONE, default ~/redox-rpi-visit), never on
# the Pi or its serial console, and never touches /srv/tftp/redox or the
# stable image. The list of images comes from the config file of this
# script's checkout.
#
# Per image:
#   - commit: the commit of the stable image, or origin/<branch> for ref=
#   - skipped if the newest build has the same commit and options
#   - /srv/tftp/visit/<name>/<build ID>/: redox/ with the network-boot files
#     as scripts/netboot.sh writes them (copy them to /srv/tftp/redox/ to
#     stage the image), BUILD_ID and visit.txt; the link
#     /srv/tftp/visit/<name>/latest points to the newest
#   - with --sdcard: the card from scripts/make-sdcard.sh --local, published
#     by scripts/publish-sdcard.sh as family redox-rpi-visit-<name>:
#     /srv/tftp/sdcard/redox-rpi-visit-<name>-<build ID>.img.xz with .sha256,
#     .manifest.toml and redox-rpi-visit-<name>-latest.* links
#   - the last KEEP builds stay, older ones are removed
# Results per image in /srv/tftp/visit/status.txt, and for a tester at the
# Pi in /srv/tftp/visit/index.html and README.txt
# (http://homenodellm:8080/visit/, scripts/visit-index.py). A failed image
# does not stop the others.
#
# No release, no package repository, no GitHub API: the clone only fetches.

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONF="$ROOT/config/visit-images.conf"
CLONE="${VISIT_CLONE:-$HOME/redox-rpi-visit}"
URL=https://github.com/redox-rpi/redox-rpi.git
TFTP=/srv/tftp
OUT="$TFTP/visit"
SDCARD="$TFTP/sdcard"
STATUS="$OUT/status.txt"
SERVER=192.168.178.120
KEEP=3

force=0
only=""
deadline=""
while [ $# -gt 0 ]; do
    case "$1" in
        --force) force=1; shift ;;
        --only) only="$2"; shift 2 ;;
        --deadline) deadline="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
[ -n "$deadline" ] || deadline="$(sed -n 's/^deadline[[:space:]]\{1,\}\([0-9:]*\).*/\1/p' "$CONF")"
[ -n "$deadline" ] || deadline=07:00

log() { echo "$(date -u +%H:%M:%S) $*"; }

# Seconds left until the deadline today (UTC), 0 if it has passed.
seconds_left() {
    local end
    end="$(date -u -d "today $deadline" +%s)"
    echo $((end > $(date +%s) ? end - $(date +%s) : 0))
}

mkdir -p "$OUT"
exec 9> "$OUT/.lock"
flock -n 9 || { log "another build-visit-images.sh is running"; exit 0; }

# status.txt: a header and one line per image, rewritten after each image.
declare -A result=()
names=()
write_status() {
    {
        echo "# Nightly visit images, scripts/build-visit-images.sh"
        echo "# run $(date -u -d "@$started" '+%Y-%m-%d %H:%M') UTC, deadline $deadline UTC, stable image ${stable_id:-none}"
        echo "# <name> <result> <build ID> <commit> <ref> <date of the build>"
        for n in "${names[@]}"; do echo "$n ${result[$n]}"; done
    } > "$STATUS.tmp" && mv "$STATUS.tmp" "$STATUS"
    # The page for a tester: index.html and README.txt next to status.txt.
    "$ROOT/scripts/pi-tests-queue.sh" > "$OUT/.test-plan.md" 2>/dev/null || log "no queue from pi-tests"
    python3 "$ROOT/scripts/visit-index.py" "$OUT" "$CONF" "$OUT/.test-plan.md" "$SDCARD" ||
        log "visit-index.py failed"
}
started="$(date +%s)"

stable_commit="$(sed -n 's/^commit = "\(.*\)"$/\1/p' "$TFTP/stable/manifest.toml" 2>/dev/null)"
stable_id="$(sed -n 's/^build_id = "\(.*\)"$/\1/p' "$TFTP/stable/manifest.toml" 2>/dev/null)"
log "stable image ${stable_id:-none} (${stable_commit:-no commit}), deadline $deadline UTC"

if [ ! -d "$CLONE/.git" ]; then
    log "cloning into $CLONE"
    git clone -q "$URL" "$CLONE" || { log "clone failed"; exit 1; }
fi
cd "$CLONE"
git fetch -q --tags --prune origin || { log "fetch failed"; exit 1; }

# Remove all but the newest KEEP entries matching a glob (newest first by
# modification time). Build IDs start with "v", the latest links do not.
prune() {
    local f
    # shellcheck disable=SC2012
    ls -1dt $1 2>/dev/null | tail -n +$((KEEP + 1)) | while read -r f; do
        log "removing old $f"
        rm -rf "$f"
    done
}

# Build one image; prints nothing, sets result[name]. Runs in the clone.
build_one() {
    local name="$1" owner="$2" ref="$3" clean="$4" sdcard="$5"; shift 5
    local options=("$@") commit key latest left build_id manifest dir work
    if [ -n "$ref" ]; then
        # Only branches of redox-rpi/redox-rpi: the clone's only remote.
        if ! git check-ref-format --branch "$ref" > /dev/null 2>&1 ||
            [ "$(git remote get-url origin)" != "$URL" ]; then
            log "$name: ref=$ref is not a branch of $URL"; result[$name]="failed - - $ref config"; return 1
        fi
        commit="$(git rev-parse -q --verify "origin/$ref^{commit}")" ||
            { log "$name: no branch origin/$ref"; result[$name]="failed - - $ref no-branch"; return 1; }
    else
        [ -n "$stable_commit" ] || { log "$name: no stable image"; result[$name]="failed - - stable no-stable"; return 1; }
        commit="$stable_commit"
    fi
    key="$commit ${sdcard:+--sdcard }${options[*]}"
    latest="$OUT/$name/latest"
    if [ "$force" = 0 ] && [ "$(sed -n 's/^key = //p' "$latest/visit.txt" 2>/dev/null)" = "$key" ]; then
        log "$name: newest build $(cat "$latest/BUILD_ID") has the same commit and options, nothing to do"
        result[$name]="unchanged $(cat "$latest/BUILD_ID") ${commit:0:9} ${ref:-stable} $(sed -n 's/^built = //p' "$latest/visit.txt")"
        return 0
    fi

    git checkout -q --detach "$commit" || { result[$name]="failed - ${commit:0:9} ${ref:-stable} checkout"; return 1; }
    if [ -n "$clean" ]; then
        # Only recipes, nothing else under upstream/redox/.
        case "$clean" in recipes/?*) ;; *) clean=/ ;; esac
        case "$clean" in /*|*..*|*//*) log "$name: clean= must be a path under upstream/redox/recipes/"; result[$name]="failed - ${commit:0:9} ${ref:-stable} config"; return 1 ;; esac
        rm -rf "upstream/redox/$clean"
    fi
    if [ ! -x build/u-boot/obj/tools/mkimage ]; then
        log "$name: building U-Boot for mkimage"
        mkdir -p build
        scripts/build-uboot.sh rpi > build/build-uboot.log 2>&1 ||
            { log "$name: build-uboot.sh failed, see $CLONE/build/build-uboot.log"; result[$name]="failed - ${commit:0:9} ${ref:-stable} u-boot"; return 1; }
    fi

    # Network-boot image, then the SD card image with the same options.
    for variant in netboot ${sdcard:+sdcard}; do
        left="$(seconds_left)"
        [ "$left" -gt 0 ] || { log "$name: deadline $deadline passed"; result[$name]="deadline - ${commit:0:9} ${ref:-stable} -"; return 1; }
        log "$name: building $variant of ${ref:-stable} ${commit:0:9} with ${options[*]} (${left}s left)"
        local extra=()
        [ "$variant" = sdcard ] && extra=(--sdcard)
        timeout --kill-after=120 "$left" scripts/build-image.sh "${extra[@]}" "${options[@]}"
        local rc=$?
        if [ "$rc" -ne 0 ]; then
            local why=build
            [ "$rc" = 124 ] || [ "$rc" = 137 ] && why=deadline
            log "$name: $variant build failed ($why, exit $rc)"
            result[$name]="$why - ${commit:0:9} ${ref:-stable} $variant"
            return 1
        fi
        manifest="$(readlink -f "build/manifests/$variant-latest.toml")"
        build_id="$(sed -n 's/^build_id = "\(.*\)"$/\1/p' "$manifest")"

        if [ "$variant" = netboot ]; then
            work="$CLONE/build/visit-tftp"
            rm -rf "$work"
            scripts/netboot.sh --server "$SERVER" --tftp-dir "$work" --dir redox \
                --image upstream/redox/build/aarch64/minimal/harddrive.img > /dev/null ||
                { result[$name]="failed $build_id ${commit:0:9} ${ref:-stable} netboot"; return 1; }
            dir="$OUT/$name/$build_id"
            rm -rf "$dir"
            mkdir -p "$dir"
            cp -rp "$work/redox" "$dir/"
            echo "$build_id" > "$dir/BUILD_ID"
            {
                echo "name = $name"
                echo "owner = $owner"
                echo "ref = ${ref:-stable image $stable_id}"
                echo "commit = $commit"
                echo "options = ${options[*]}"
                echo "built = $(date -u '+%Y-%m-%d %H:%M') UTC"
                echo "key = $key"
            } > "$dir/visit.txt"
            ln -sfn "$build_id" "$OUT/$name/latest"
            log "$name: $dir"
        else
            # A whole card (make-sdcard.sh --local), published by the one
            # script for the image server (docs/topics/ci.md, "Image server").
            scripts/make-sdcard.sh --local upstream/redox/build/aarch64/sdcard/harddrive.img \
                --family "redox-rpi-visit-$name" --out build/visit-sdcard.img > /dev/null &&
                "$ROOT/scripts/publish-sdcard.sh" build/visit-sdcard.img --family "redox-rpi-visit-$name" \
                    --manifest "$manifest" --owner "$owner" --visit "$name" --latest ||
                { result[$name]="failed $build_id ${commit:0:9} ${ref:-stable} sdcard-publish"; return 1; }
            rm -f build/visit-sdcard.img build/visit-sdcard.img.xz
            log "$name: $SDCARD/redox-rpi-visit-$name-$build_id.img.xz"
        fi
    done
    result[$name]="built $(cat "$latest/BUILD_ID") ${commit:0:9} ${ref:-stable} $(date -u '+%Y-%m-%dT%H:%M')Z"
    prune "$OUT/$name/v*"
    if [ -n "$sdcard" ]; then
        for ext in img.xz img.xz.sha256 img.xz.manifest.toml img.xz.meta.json; do
            prune "$SDCARD/redox-rpi-visit-$name-v*.$ext"
        done
        python3 "$ROOT/scripts/sdcard-index.py" "$SDCARD" "$OUT/.test-plan.md" > /dev/null
    fi
    return 0
}

failed=0
while read -r name owner rest; do
    case "$name" in ''|'#'*|deadline) continue ;; esac
    rest="${rest%%#*}"
    [ -n "$only" ] && [ "$name" != "$only" ] && continue
    ref="" clean="" sdcard=""
    options=()
    for word in $rest; do
        case "$word" in
            ref=*) ref="${word#ref=}" ;;
            clean=*) clean="${word#clean=}" ;;
            --sdcard) sdcard=1 ;;
            *) options+=("$word") ;;
        esac
    done
    names+=("$name")
    result[$name]="running - - ${ref:-stable} -"
    write_status
    # stdin is the config file: keep the build from reading it.
    build_one "$name" "$owner" "$ref" "$clean" "$sdcard" "${options[@]}" < /dev/null || failed=1
    write_status
done < "$CONF"
[ "${#names[@]}" -gt 0 ] || { log "no images in $CONF${only:+ named $only}"; write_status; }
log "done: $(for n in "${names[@]}"; do printf '%s %s; ' "$n" "${result[$n]%% *}"; done)"
exit "$failed"
