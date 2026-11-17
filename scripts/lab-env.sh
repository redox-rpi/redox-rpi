# Lab values for scripts that talk to the build server, the test Pi or the
# image server. Host names and addresses are not in the repository: the
# admins keep them on the build server in
#   ${REDOX_LAB_ENV:-~/.config/redox-rpi/lab.env}
# as KEY=value lines (# starts a comment). A variable already set in the
# environment wins over the file.
#
#   REDOX_BUILD_SERVER  IPv4 address of the build server as the Pi sees it
#                       (U-Boot, boot.scr and pkg use it; names do not
#                       resolve everywhere)
#   REDOX_PI            IPv4 address of the test Pi (devshd, pi-test.py)
#   REDOX_HTTP_BASE     base URL of the image server for people, without a
#                       trailing slash, e.g. http://buildhost.lan:8080
#   REDOX_SHELLY_URL    status URL of the Pi's power plug (read only)
#
# Usage, in a bash script:
#   . "$ROOT/scripts/lab-env.sh"
#   lab_need REDOX_BUILD_SERVER REDOX_PI   # stops with a message if unset

lab_env_file="${REDOX_LAB_ENV:-$HOME/.config/redox-rpi/lab.env}"

# Sets every REDOX_* key from the file that is not already in the
# environment. Only KEY=value lines; the file is not run as a script.
lab_env_load() {
    [ -r "$lab_env_file" ] || return 0
    local line key value
    while IFS= read -r line || [ -n "$line" ]; do
        line="${line%%#*}"
        line="${line#"${line%%[![:space:]]*}"}"
        line="${line%"${line##*[![:space:]]}"}"
        [ -n "$line" ] || continue
        key="${line%%=*}"
        key="${key%"${key##*[![:space:]]}"}"
        value="${line#*=}"
        value="${value#"${value%%[![:space:]]*}"}"
        [[ "$key" =~ ^REDOX_[A-Z0-9_]+$ ]] || continue
        value="${value%\"}"; value="${value#\"}"
        [ -n "${!key:-}" ] || export "$key=$value"
    done < "$lab_env_file"
}

lab_need() {
    local key missing=()
    for key in "$@"; do
        [ -n "${!key:-}" ] || missing+=("$key")
    done
    if [ ${#missing[@]} -gt 0 ]; then
        echo "${0##*/}: ${missing[*]} not set. Set it in the environment or in $lab_env_file (scripts/lab-env.sh); the admins keep that file on the build server." >&2
        exit 1
    fi
}

lab_env_load
