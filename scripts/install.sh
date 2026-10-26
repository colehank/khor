#!/bin/sh
# Install khor on the machine that runs this, from a published release.
#
#   curl -fsSL https://github.com/colehank/khor/releases/latest/download/install.sh | sh
#
# **Fetched from the release, not from `raw.githubusercontent.com`.**
# Measured from turing: raw timed out after 20s having received nothing,
# while `api.github.com` answered in 0.59s and a release download in
# 1.82s — so the one host the old one-liner depended on is exactly the
# one those machines cannot reach. The binary and the script now come
# down the same road, which is also the road that is tested every time
# anybody installs.
#
# The whole install story is: **the machine fetches one file and runs
# it.** No toolchain, no package manager, no root, and nothing that has
# to be cross-compiled by hand on somebody's laptop and pushed over ssh
# — that road ends in per-machine adaptation (a shared NFS home, a
# broken `.profile`, an sshd that throttles), which is a tax on every
# new machine forever.
#
#   KHOR_VERSION   a tag to install (default: the latest release)
#   KHOR_REPO      owner/name (default: the one below)
#   KHOR_NO_SERVE  set to skip starting the resident
#   KHOR_SKIP_FETCH  set to keep the khor already in ~/.local/bin and only
#                  do what comes after the fetch (`khor add` runs the
#                  install in two steps, 放 khor then 启动)
#   KHOR_ASSET_SHA256 exact compressed-byte SHA256 required with KHOR_ASSET_FILE
#   KHOR_ASSET_FILE  a release asset (.gz) already on this machine, used
#                  instead of downloading one — `khor add` pushes it over
#                  ssh when this machine cannot fetch for itself
#   KHOR_MIRROR    base URL of a mirror carrying install.sh + the
#                  release assets, for machines that cannot reach
#                  GitHub at all (the no-DNS half of a fleet):
#                    curl -fsSL http://<mirror>/khor/install.sh \
#                      | KHOR_MIRROR=http://<mirror>/khor sh
#                  The mirror is plain static files — any box that can
#                  reach GitHub can host one (scripts/mirror-sync.sh).

# KHOR_MIRROR_LAYOUT=versioned-v1
# Shared declarations are generated from scripts/install-components.sh.
# Run python3 scripts/sync-install-components.py --write after editing them.
# KHOR_INSTALL_DECLARATIONS_BEGIN
khor_fetch_verify_place() (
    set -eu
    URL="$1"
    BIN_DIR="$2"
    ASSET="$3"
    VERSION="$4"
    if [ -n "${KHOR_ASSET_FILE:-}" ]; then
        echo "khor: verifying preplaced archive ($ASSET)"
    else
        [ -z "${KHOR_ASSET_SHA256:-}" ] || { echo "khor: preplaced SHA256 requires KHOR_ASSET_FILE" >&2; exit 1; }
        echo "khor: fetching $ASSET ($VERSION)"
    fi
    STAGE=$(mktemp -d "$BIN_DIR/.khor.download.XXXXXXXX")
    trap 'rm -rf "$STAGE"' EXIT
    TMP="$STAGE/khor"
    ARCHIVE="$STAGE/asset.gz"
    if [ -n "${KHOR_ASSET_FILE:-}" ]; then
        URL="$KHOR_ASSET_FILE"
        EXPECTED="${KHOR_ASSET_SHA256:-}"
        case "$EXPECTED" in ''|*[!0-9a-f]*) echo "khor: preplaced archive needs its exact SHA256" >&2; exit 1 ;; esac
        [ "${#EXPECTED}" = 64 ] || { echo "khor: invalid preplaced SHA256" >&2; exit 1; }
        cp "$KHOR_ASSET_FILE" "$ARCHIVE"
    else
        khor_download "$URL" "$ARCHIVE" || {
            if [ "${5:-}" = mirror ]; then
                echo "khor: mirror needs a complete releases/$VERSION directory; legacy flat mirrors must migrate" >&2
            fi
            exit 1
        }
        khor_download "$URL.sha256" "$STAGE/checksum" 512
        EXPECTED=$(khor_published_digest "$STAGE/checksum" "$ASSET.gz") || {
            echo "khor: invalid checksum record for $ASSET.gz" >&2; exit 1;
        }
    fi
    ACTUAL=$(khor_archive_digest "$ARCHIVE")
    [ "$ACTUAL" = "$EXPECTED" ] || { echo "khor: archive does not match its selected SHA256" >&2; exit 1; }
    gzip -dc "$ARCHIVE" > "$TMP" || { echo "khor: $URL is not a gzip (wrong tag?)" >&2; exit 1; }
    case "$(head -c 4 "$TMP" | od -An -tx1 | tr -d ' \n')" in
        7f454c46|cffaedfe|cafebabe) : ;;
        *) echo "khor: $URL did not answer with a program (wrong tag?)" >&2; exit 1 ;;
    esac
    [ ! -d "$BIN_DIR/khor" ] || { echo "khor: destination is a directory" >&2; exit 1; }
    chmod +x "$TMP"
    mv -f "$TMP" "$BIN_DIR/khor"
    if [ -n "${KHOR_ASSET_FILE:-}" ]; then
        if [ "$KHOR_ASSET_FILE" -ef "$BIN_DIR/khor" ]; then
            : # A source path may now name the just-installed destination.
        else
            _same_file=$?
            if [ "$_same_file" = 1 ]; then
                rm -f "$KHOR_ASSET_FILE" || echo "khor: installed; could not remove preplaced archive" >&2
            else
                echo "khor: installed; retaining preplaced archive (file identity unavailable)" >&2
            fi
        fi
    fi
)

khor_capture() (
    set -eu
    _destination="$1" _limit="$2" _mode="$3"
    shift 3
    _pipe="$_destination.pipe"
    mkfifo "$_pipe" || exit 1
    _producer=""
    trap '[ -z "$_producer" ] || wait "$_producer" 2>/dev/null || :; rm -f "$_pipe"' EXIT
    (
        if [ "$_mode" = both ]; then exec "$@" 2>&1; else exec "$@"; fi
    ) > "$_pipe" &
    _producer=$!
    dd if="$_pipe" of="$_destination" bs=1 count="$((_limit + 1))" 2>/dev/null || exit 1
    _status=0
    wait "$_producer" || _status=$?
    _producer=""
    [ "$(wc -c < "$_destination")" -le "$_limit" ] || {
        echo "khor: metadata exceeds $_limit bytes" >&2; exit 1;
    }
    exit "$_status"
)

khor_download() (
    set -eu
    _url="$1" _destination="$2" _limit="${3:-}"
    if command -v curl >/dev/null 2>&1; then
        if [ -n "$_limit" ]; then
            khor_capture "$_destination" "$_limit" out curl --disable -fsSL --connect-timeout 15 --max-time 30 "$_url"
        else
            curl --disable -fsSL --connect-timeout 15 "$_url" -o "$_destination"
        fi
    elif command -v wget >/dev/null 2>&1; then
        if [ -n "$_limit" ]; then
            khor_capture "$_destination" "$_limit" out wget -qT 15 -O - "$_url"
        else
            wget -qT 15 -O "$_destination" "$_url"
        fi
    else
        echo "khor: needs curl or wget to fetch a release" >&2; exit 1
    fi
)

khor_archive_digest() (
    set -eu
    if command -v sha256sum >/dev/null 2>&1; then
        _line=$(sha256sum "$1")
    elif command -v shasum >/dev/null 2>&1; then
        _line=$(shasum -a 256 "$1")
    else
        echo "khor: needs sha256sum or shasum to verify a release" >&2; exit 1
    fi
    _digest="${_line%% *}"
    case "$_digest" in ''|*[!0-9a-f]*) exit 1 ;; esac
    [ "${#_digest}" = 64 ] || exit 1
    printf '%s\n' "$_digest"
)

khor_published_digest() (
    set -eu
    [ "$(wc -c < "$1")" -le 512 ] || exit 1
    LC_ALL=C awk -v name="$2" '
        NR != 1 { bad = 1; exit }
        {
            digest = substr($0, 1, 64)
            if (length($0) != 66 + length(name) || digest ~ /[^0-9a-f]/ || substr($0,65,2) != "  " || substr($0,67) != name) bad = 1
        }
        END { if (bad || NR != 1 || length(digest) != 64) exit 1; print digest }
    ' "$1"
)

khor_valid_tag() (
    set -eu
    LC_ALL=C; export LC_ALL
    case "$1" in ''|*[!a-zA-Z0-9.+-]*) exit 1 ;; esac
    [ "${#1}" -le 128 ] || exit 1
    LC_ALL=C awk -v tag="$1" 'BEGIN {
        count = split(tag, plus, /\+/); if (count > 2) exit 1
        if (count == 2) { if (plus[2] == "") exit 1; n = split(plus[2], ids, /\./); for (i=1;i<=n;i++) if (ids[i] !~ /^[0-9A-Za-z-]+$/) exit 1 }
        tag = plus[1]; dash = index(tag, "-")
        if (dash) {
            pre = substr(tag, dash+1); tag = substr(tag, 1, dash-1)
            if (pre == "") exit 1
            n = split(pre, ids, /\./)
            for (i=1;i<=n;i++) if (ids[i] !~ /^[0-9A-Za-z-]+$/ || (ids[i] ~ /^[0-9]+$/ && length(ids[i]) > 1 && substr(ids[i],1,1) == "0")) exit 1
        }
        if (tag !~ /^v[0-9]+\.[0-9]+\.[0-9]+$/) exit 1
        split(substr(tag,2), nums, /\./)
        for (i=1;i<=3;i++) if (length(nums[i]) > 1 && substr(nums[i],1,1) == "0") exit 1
    }'
)

khor_descriptor_tag() (
    set -eu
    _file="$1"
    _size=$(wc -c < "$_file")
    [ "$_size" -le 256 ] || exit 1
    _allowed=$(LC_ALL=C tr -cd 'A-Za-z0-9.+\012-' < "$_file" | wc -c)
    [ "$_allowed" -eq "$_size" ] || exit 1
    awk 'NR != 1 { bad = 1 } END { exit (bad || NR != 1) }' "$_file"
    _tag=$(cat "$_file")
    khor_valid_tag "$_tag" || exit 1
    printf '%s\n' "$_tag"
)

khor_header_summary() (
    set -eu
    _file="$1" _mode="$2"
    [ "$(wc -c < "$_file")" -le 65536 ] || exit 1
    [ "$(LC_ALL=C tr -d '\011\012\015\040-\176' < "$_file" | wc -c)" -eq 0 ] || exit 1
    LC_ALL=C awk -v mode="$_mode" '
        length($0) > 8192 { bad = 1; exit }
        {
            line = $0; sub(/\r$/, "", line)
            if (mode == "wget") {
                if (line == "---response begin---") {
                    if (raw || active || final_seen) { bad = 1; exit }
                    raw = 1; raw_status = 0; raw_blocks++; next
                }
                if (line == "---response end---") {
                    if (!raw || active || !raw_status) { bad = 1; exit }
                    raw = 0; next
                }
                if (!raw) next
            }
            if (line ~ /^HTTP\/[0-9.]+ [0-9][0-9][0-9]([ \t]|$)/) {
                if (active || final_seen) { bad = 1; exit }
                split(line, fields, /[ \t]+/); status = fields[2]
                if (mode == "wget") raw_status++
                proxy = tolower(line) ~ /^http\/[0-9.]+ 200 connection established[ \t]*$/
                if (status >= 200 && !proxy) final_seen = 1
                location = ""; locations = 0; active = 1; seen = 1; next
            }
            if (line == "") { active = 0; next }
            if (!active || line !~ /^[^ \t:]+:/) { bad = 1; exit }
            if (active && tolower(line) ~ /^location:/) {
                if (++locations > 1) { bad = 1; exit }
                sub(/^[^:]*:[ \t]*/, "", line); location = line
            }
        }
        END {
            if (bad || !seen || active || raw || (mode == "wget" && !raw_blocks)) exit 1
            if (status >= 300 && status < 400 && locations != 1) exit 1
            printf "%s\n%s\n", status, location
        }
    ' "$_file"
)

khor_latest_tag() (
    set -eu
    _repo="$1"
    _current="https://github.com/$_repo/releases/latest"
    _scratch=$(mktemp -d "${TMPDIR:-/tmp}/khor-release.XXXXXXXX")
    trap 'rm -rf "$_scratch"' EXIT
    _hops=0
    _remaining=65536
    _newline='
'
    if command -v curl >/dev/null 2>&1; then
        _tool=curl
    elif command -v wget >/dev/null 2>&1; then
        khor_capture "$_scratch/help" 65536 both wget --help || { echo "khor: cannot identify wget" >&2; exit 1; }
        grep -q 'GNU Wget' "$_scratch/help" && grep -q -- '--max-redirect' "$_scratch/help" && grep -q -- '--server-response' "$_scratch/help" && grep -q -- '--debug' "$_scratch/help" || {
            echo "khor: this wget cannot safely resolve latest; a supported GNU wget or curl is required" >&2; exit 1;
        }
        _tool=wget
    else
        echo "khor: needs curl or wget to select a release" >&2; exit 1
    fi
    while :; do
        [ "$_remaining" -gt 0 ] || { echo "khor: release response headers exceed their total budget" >&2; exit 1; }
        _status=0
        if [ "$_tool" = curl ]; then
            khor_capture "$_scratch/headers" "$_remaining" out curl --disable -fsSI --connect-timeout 15 --max-time 30 "$_current" || _status=$?
        else
            LC_ALL=C khor_capture "$_scratch/headers" "$_remaining" both wget --no-config --no-hsts --no-netrc --debug --server-response --spider --max-redirect=0 --tries=1 -T 15 "$_current" || _status=$?
        fi
        _used=$(wc -c < "$_scratch/headers")
        [ "$_used" -le "$_remaining" ] || { echo "khor: release response headers exceed their total budget" >&2; exit 1; }
        _remaining=$((_remaining - _used))
        _reply=$(khor_header_summary "$_scratch/headers" "$_tool") || { echo "khor: invalid or ambiguous release response headers" >&2; exit 1; }
        _code="${_reply%%"$_newline"*}"
        _location="${_reply#*"$_newline"}"
        case "$_code" in
            2??)
                [ "$_status" = 0 ] || { echo "khor: release metadata request failed" >&2; exit 1; }
                _prefix="https://github.com/$_repo/releases/tag/"
                case "$_current" in "$_prefix"*) _tag="${_current#"$_prefix"}" ;; *) echo "khor: latest did not resolve to a canonical release tag" >&2; exit 1 ;; esac
                _tag=$(printf '%s' "$_tag" | sed 's/%2[Bb]/+/g')
                khor_valid_tag "$_tag" || { echo "khor: latest returned an unsafe release tag" >&2; exit 1; }
                printf '%s\n' "$_tag"; exit
                ;;
            3??)
                if [ "$_status" != 0 ]; then
                    [ "$_tool" = wget ] && [ "$_status" = 8 ] && grep -qx '0 redirections exceeded\.' "$_scratch/headers" || {
                        echo "khor: release metadata request failed" >&2; exit 1;
                    }
                fi
                [ "$_hops" -lt 20 ] || { echo "khor: too many release redirects" >&2; exit 1; }
                _decoded_location=$(printf '%s' "$_location" | sed 's/%2[Bb]/+/g')
                case "$_decoded_location" in ''|*[!a-zA-Z0-9:/._~+\[\]-]*) echo "khor: unsafe release redirect" >&2; exit 1 ;; esac
                case "$_location" in
                    https://github.com/*) _next="$_location" ;;
                    //github.com/*) _next="https:$_location" ;;
                    /*) _next="https://github.com$_location" ;;
                    *://*|//*) echo "khor: untrusted release redirect" >&2; exit 1 ;;
                    *) _next="${_current%/*}/$_location" ;;
                esac
                case "$_next" in
                    *'/../'*|*'/./'*|*/..|*/.) echo "khor: unsafe release redirect path" >&2; exit 1 ;;
                    "https://github.com/$_repo/"*) : ;;
                    *) echo "khor: release redirect changed its repository" >&2; exit 1 ;;
                esac
                _current="$_next"; _hops=$((_hops + 1))
                ;;
            *) echo "khor: release metadata did not succeed" >&2; exit 1 ;;
        esac
    done
)

khor_select_release() (
    set -eu
    LC_ALL=C; export LC_ALL
    _repo="$1" _version="$2" _mirror="$3" _asset="$4"
    _owner="${_repo%%/*}"; _name="${_repo#*/}"
    case "$_repo" in *[!a-zA-Z0-9_./-]*) echo "khor: invalid release repository" >&2; exit 1 ;; esac
    case "$_owner/$_name" in */*/*|./*|../*|*/.|*/..) echo "khor: invalid release repository" >&2; exit 1 ;; esac
    [ "$_repo" != "$_owner" ] && [ -n "$_owner" ] && [ -n "$_name" ] && [ "${#_owner}" -le 100 ] && [ "${#_name}" -le 100 ] || { echo "khor: invalid release repository" >&2; exit 1; }
    case "$_asset" in ''|*[!a-zA-Z0-9_-]*) echo "khor: invalid release asset" >&2; exit 1 ;; esac
    if [ -n "$_mirror" ]; then
        case "$_mirror" in http://*|https://*) : ;; *) echo "khor: mirror must use HTTP(S)" >&2; exit 1 ;; esac
        case "$_mirror" in *[!a-zA-Z0-9:/._~+\[\]-]*) echo "khor: invalid mirror URL" >&2; exit 1 ;; esac
        _authority="${_mirror#*://}"; _authority="${_authority%%/*}"
        [ -n "$_authority" ] || { echo "khor: empty mirror host" >&2; exit 1; }
        while [ "${_mirror%/}" != "$_mirror" ]; do _mirror="${_mirror%/}"; done
        if [ "$_version" = latest ]; then
            _scratch=$(mktemp -d "${TMPDIR:-/tmp}/khor-release.XXXXXXXX")
            trap 'rm -rf "$_scratch"' EXIT
            khor_download "$_mirror/VERSION" "$_scratch/VERSION" 256
            _version=$(khor_descriptor_tag "$_scratch/VERSION") || { echo "khor: invalid mirror VERSION" >&2; exit 1; }
        else
            khor_valid_tag "$_version" || { echo "khor: invalid release tag" >&2; exit 1; }
        fi
        _url="$_mirror/releases/$_version/$_asset.gz"
    elif [ "$_version" = latest ]; then
        _version=$(khor_latest_tag "$_repo")
        _url="https://github.com/$_repo/releases/download/$_version/$_asset.gz"
    else
        khor_valid_tag "$_version" || { echo "khor: invalid release tag" >&2; exit 1; }
        _url="https://github.com/$_repo/releases/download/$_version/$_asset.gz"
    fi
    printf '%s\n%s\n' "$_version" "$_url"
)

khor_fetch_release() (
    set -eu
    _repo="$1" _version="$2" _mirror="$3" _asset="$4" _bin="$5"
    if [ -n "${KHOR_ASSET_FILE:-}" ]; then
        _url="$KHOR_ASSET_FILE"
    else
        _selected=$(khor_select_release "$_repo" "$_version" "$_mirror" "$_asset")
        _newline='
'
        _version="${_selected%%"$_newline"*}"
        _url="${_selected#*"$_newline"}"
    fi
    khor_fetch_verify_place "$_url" "$_bin" "$_asset" "$_version" "${_mirror:+mirror}"
)

khor_install_main() {
set -eu

REPO="${KHOR_REPO:-colehank/khor}"
VERSION="${KHOR_VERSION:-latest}"
BIN_DIR="$HOME/.local/bin"

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) ASSET=khor-linux-x86_64 ;;
    Linux-aarch64|Linux-arm64) ASSET=khor-linux-aarch64 ;;
    Darwin-arm64) ASSET=khor-macos-arm64 ;;
    Darwin-x86_64) ASSET=khor-macos-x86_64 ;;
    *) echo "khor: no build for $(uname -s)-$(uname -m) yet" >&2; exit 1 ;;
esac


mkdir -p "$BIN_DIR"
if [ -n "${KHOR_SKIP_FETCH:-}" ] && [ -x "$BIN_DIR/khor" ]; then
    :
else
    khor_fetch_release "$REPO" "$VERSION" "${KHOR_MIRROR:-}" "$ASSET" "$BIN_DIR"
fi

if ! grep -qs 'HOME/.local/bin' "$HOME/.profile" 2>/dev/null; then
    printf '%s\n' 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.profile"
    echo "khor: added ~/.local/bin to PATH in ~/.profile"
fi

# One store, one machine. A cluster mounts one home everywhere, so
# `~/.khor` can already belong to a different box; khor refuses to open
# somebody else's store, and this puts one of this machine's own in
# front of that refusal. **Before** the `khor id` below, not after: id
# opens a store too, and with a bare ssh environment (no KHOR_HOME) it
# used to hit the owner's store, get refused, and take the whole
# install down with set -e — 2026-08-20, hinton, the same script that
# had worked the day before from inside a khor session whose
# environment carried KHOR_HOME. Who runs a script decides what it
# sees, so the script now decides for itself first.
# **A copy of `khor_drop_profile_line` from scripts/onset-lib.sh**, and
# a copy on purpose: this file is fetched and run on its own, so it may
# not assume anything else is on the machine. That is the same reason
# the rest of the install story is one file.
#
# Takes back the line the installer used to append,
# `export KHOR_HOME="$HOME/.khor-hosts/$(hostname)"`. It was written
# only by a machine that had found the home already claimed — but
# `~/.profile` on a shared home is one file for the whole fleet, so the
# owner read it too and was sent to a store that was not its own.
# Measured 2026-09-11 on turing. khor works the rule out for itself now,
# and a stale line would override it.
#
# **Only that exact line.** A `KHOR_HOME` somebody set by hand is a
# deliberate override, and removing it would be this doing the very
# thing it exists to undo.
khor_drop_profile_line() {
    _prof="$1"
    [ -f "$_prof" ] || return 0
    _line='export KHOR_HOME="$HOME/.khor-hosts/$(hostname)"'
    grep -qxF "$_line" "$_prof" 2>/dev/null || return 0
    _tmp="$_prof.khor-$$"
    grep -vxF "$_line" "$_prof" > "$_tmp" 2>/dev/null || {
        rm -f "$_tmp"
        return 1
    }
    mv -f "$_tmp" "$_prof" && echo "khor: took the stale KHOR_HOME line back out of $_prof"
}

# The freshly-unpacked binary is the authority on which store is its
# own (`khor root`); this block only runs when it cannot answer, and
# reads the `name` line of the two-line owner file before falling back
# to the bare hostname older stores carry. See `khor_root` in
# scripts/onset-lib.sh for what reproducing the rule in sh cost.
ROOT=$("$BIN_DIR/khor" root 2>/dev/null) || ROOT=""
if [ -n "$ROOT" ]; then
    mkdir -p "$ROOT/.khor"
    [ "$ROOT" = "$HOME" ] || echo "khor: this home is somebody else's — $(hostname) uses $ROOT"
fi
OWNER=""
if [ -z "$ROOT" ] && [ -f "$HOME/.khor/owner" ]; then
    OWNER=$(sed -n 's/^name[[:space:]]\{1,\}//p' "$HOME/.khor/owner" | head -1)
    [ -n "$OWNER" ] || OWNER=$(head -1 "$HOME/.khor/owner")
    case "$OWNER" in machine\ *) OWNER="" ;; esac
fi
[ -n "$ROOT" ] || ROOT="$HOME"
if [ -n "$OWNER" ] && [ "$OWNER" != "$(hostname)" ]; then
    ROOT="$HOME/.khor-hosts/$(hostname)"
    mkdir -p "$ROOT/.khor"
    echo "khor: this home belongs to $OWNER — $(hostname) gets its own store at $ROOT"
fi
# **No KHOR_HOME line in ~/.profile.** khor works the rule out for
# itself now (`Node::root_from_env`), and on a shared home that file is
# one file for the whole fleet: the line read
# `KHOR_HOME="$HOME/.khor-hosts/$(hostname)"` for everybody, which is
# right for every machine except the one that owns `$HOME` — and that
# one reads the same file. So an old line is taken back out; leaving it
# would override the rule it was replaced by.
khor_drop_profile_line "$HOME/.profile"

# **Nor on the command line** (round NET-11): khor works out its own
# room, and `$ROOT` above is that same answer. Two ways to name one store
# is how turing came to run two serve trees on it.
env -u KHOR_HOME "$BIN_DIR/khor" id

[ -n "${KHOR_NO_SERVE:-}" ] && exit 0

# ---- the one way a serve actually starts, everywhere -----------------
# khor-serve-up is idempotent (a live pid file means "already up", so
# running it twice is running it once) and host-aware (the owner-file
# dance above, replayed at start time — a shared NFS home runs this
# same file on every host). Every guardian below is only a way to get
# this script run at boot; the keeper inside `khor serve` does the rest.
UP="$BIN_DIR/khor-serve-up"
cat > "$UP" <<'SERVEUP'
#!/bin/sh
# Start this host's khor serve unless it is already running.
set -u
BIN="$(dirname "$0")/khor"
# **Asked, not computed.** This was the fourth copy of one rule, and the
# day the `owner` file grew a line the copies stopped agreeing — see
# `khor_root` in scripts/onset-lib.sh for what that cost. The binary
# sits next to this script, so there is always one to ask.
R=$("$BIN" root 2>/dev/null) || R=""
[ -n "$R" ] || R="$HOME"
PIDF="$R/.khor/serve.pid"
mkdir -p "$R/.khor"
if [ -f "$PIDF" ] && kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    exit 0
fi
# **The app is this machine's resident when it runs** (its endpoint file
# says `"app":true`): a second one on the same key knocks both off the
# relay. `khor serve` refuses too; this saves starting it to hear so.
EP="$R/.khor/endpoint.json"
if [ -f "$EP" ] && grep -q '"app":true' "$EP"; then
    APP_PID=$(sed -n 's/.*"pid":\([0-9]*\).*/\1/p' "$EP")
    if [ -n "$APP_PID" ] && kill -0 "$APP_PID" 2>/dev/null; then
        exit 0
    fi
fi
# setsid where it exists (Linux); launchd/nohup orphaning does the same
# job on macOS, which has no setsid.
# Without KHOR_HOME (round NET-11): the serve finds the room `$R` names
# by itself, one way to name it for every start.
if command -v setsid >/dev/null 2>&1; then
    env -u KHOR_HOME setsid nohup "$BIN" serve >> "$R/.khor/serve.log" 2>&1 &
else
    # Not `nohup` on macOS: under a non-interactive ssh (`khor add`) it
    # refuses with 「can't detach from console」 and nothing starts
    # (measured 2026-10-08). Ignoring HUP is all it would have done here;
    # the output is redirected anyway, and `exec` keeps the pid the one
    # written below.
    ( trap '' HUP; exec env -u KHOR_HOME "$BIN" serve ) < /dev/null >> "$R/.khor/serve.log" 2>&1 &
fi
echo $! > "$PIDF"
SERVEUP
chmod +x "$UP"

# ---- a guardian, so the serve outlives a reboot ----------------------
# The keeper (inside `khor serve`) already survives crashes and swaps
# generations; the one thing it cannot survive is the machine itself
# rebooting. Pick the best boot-hook the current privileges allow —
# detected, not asked (defaults over settings).
GUARDIAN="nothing — run khor-serve-up after a reboot"
# The same answer as one word, for `khor add` to read (it says it in the
# reader's language): none | launchd | systemd | systemd-linger |
# systemd-login | login.
KEEPER=none
GUARDIAN_SET=""
case "$(uname -s)" in
Darwin)
    PL="$HOME/Library/LaunchAgents/io.github.colehank.khor.plist"
    mkdir -p "$(dirname "$PL")"
    cat > "$PL" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>Label</key><string>io.github.colehank.khor</string>
    <key>ProgramArguments</key><array><string>$UP</string></array>
    <key>RunAtLoad</key><true/>
    <key>AbandonProcessGroup</key><true/>
</dict></plist>
PLIST
    launchctl bootstrap "gui/$(id -u)" "$PL" 2>/dev/null || true
    GUARDIAN="launchd (LaunchAgent, no root needed)"
    KEEPER=launchd
    ;;
Linux)
    if command -v systemctl >/dev/null 2>&1 && [ "$(id -u)" = 0 ]; then
        cat > /etc/systemd/system/khor-serve.service <<UNIT
[Unit]
Description=khor serve (boot pull-up; the keeper inside handles the rest)
After=network-online.target

[Service]
# forking + PIDFile so systemd adopts the keeper that khor-serve-up
# leaves behind. Type=oneshot swept the cgroup on deactivation and
# silently killed it — caught on a real reboot (2026-08-20 11:16:04,
# "Deactivated successfully" and the keeper was gone). launchd needed
# the same medicine, spelled AbandonProcessGroup.
Type=forking
PIDFile=$ROOT/.khor/serve.pid
Environment=HOME=$HOME
ExecStart=$UP

[Install]
WantedBy=multi-user.target
UNIT
        systemctl daemon-reload
        systemctl enable khor-serve.service >/dev/null 2>&1
        GUARDIAN="systemd (system unit khor-serve.service)"
        KEEPER=systemd
        GUARDIAN_SET=1
    elif command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
        mkdir -p "$HOME/.config/systemd/user"
        cat > "$HOME/.config/systemd/user/khor-serve.service" <<UNIT
[Unit]
Description=khor serve (boot pull-up; the keeper inside handles the rest)

[Service]
# Same forking+PIDFile medicine as the system unit (oneshot sweeps the
# keeper with the cgroup). One caveat this file cannot fix: on a shared
# NFS home this unit file is shared between hosts while PIDFile is this
# host's — whichever host installed last wins the path, and the
# .profile line below covers the other.
Type=forking
PIDFile=$ROOT/.khor/serve.pid
ExecStart=$UP

[Install]
WantedBy=default.target
UNIT
        systemctl --user daemon-reload
        systemctl --user enable khor-serve.service >/dev/null 2>&1
        # Without lingering the unit only fires at login — same promise
        # as the .profile line below, just tidier. With it, at boot.
        loginctl enable-linger "$(id -un)" >/dev/null 2>&1 \
            && { GUARDIAN="systemd (user unit, lingering on: starts at boot)"; KEEPER=systemd-linger; } \
            || { GUARDIAN="systemd (user unit: starts at your first login)"; KEEPER=systemd-login; }
        GUARDIAN_SET=1
    fi
    # Belt and suspenders for every non-root tier: the first person to
    # log in after a reboot becomes the restart button. Written to BOTH
    # files when .bash_profile exists — bash logins read .bash_profile
    # and then never look at .profile, so a line only in .profile is a
    # line bash users never run (hinton, 2026-08-20: LOGIN-PULLUP-FAILED
    # for exactly this reason).
    if [ "$(id -u)" != 0 ]; then
        for RC in "$HOME/.profile" "$HOME/.bash_profile"; do
            [ "$RC" = "$HOME/.bash_profile" ] && [ ! -f "$RC" ] && continue
            if ! grep -qs 'khor-serve-up' "$RC" 2>/dev/null; then
                printf '[ -x "$HOME/.local/bin/khor-serve-up" ] && "$HOME/.local/bin/khor-serve-up" >/dev/null 2>&1 || true\n' >> "$RC"
            fi
        done
        if [ -z "${GUARDIAN_SET:-}" ]; then
            GUARDIAN="your first login (.profile/.bash_profile line)"
            KEEPER=login
        fi
    fi
    ;;
esac

PIDF="$ROOT/.khor/serve.pid"
mkdir -p "$ROOT/.khor"
# Stopped by its own pid file, never by name: `pkill -f khor` on a
# shared machine is somebody else's outage.
if [ -f "$PIDF" ] && kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    kill "$(cat "$PIDF")" 2>/dev/null || true
    sleep 1
fi
"$UP"
sleep 2
EP="$ROOT/.khor/endpoint.json"
if [ -f "$EP" ] && grep -q '"app":true' "$EP"; then
    APP_PID=$(sed -n 's/.*"pid":\([0-9]*\).*/\1/p' "$EP")
    if [ -n "$APP_PID" ] && kill -0 "$APP_PID" 2>/dev/null; then
        echo "khor: the khor app is this machine's resident (pid $APP_PID); no serve started beside it"
        echo "khor: done."
        exit 0
    fi
fi
if kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    echo "khor: serve is up (pid $(cat "$PIDF"), store $ROOT)"
    echo "khor: reboots handled by $GUARDIAN"
    echo "khor: keeper $KEEPER"
else
    echo "khor: serve died on start — tail of $ROOT/.khor/serve.log:" >&2
    tail -5 "$ROOT/.khor/serve.log" >&2
    exit 1
fi
echo "khor: done. join the mesh with — khor pair <ticket>"
}
# KHOR_INSTALL_DECLARATIONS_END
khor_install_main "$@"
