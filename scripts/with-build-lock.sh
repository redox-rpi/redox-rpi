#!/usr/bin/env bash
# Run a command while holding the build host lock, so it does not run at the
# same time as a build in this tree (docs/test-plan.md). For jobs that use
# the build output, e.g. a QEMU test of the image just built; a
# scripts/build-image.sh started inside does not lock again.
#
# Usage:
#   scripts/with-build-lock.sh <command> [args]...

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$ROOT/build"
exec 9> "$ROOT/build/.build-image.lock"
if ! flock -n 9; then
    echo "another build is running in $ROOT, waiting for it" >&2
    flock 9
fi
export REDOX_RPI_BUILD_LOCK=1
exec "$@"
