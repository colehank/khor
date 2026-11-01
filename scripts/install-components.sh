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
            exit 75
        }
        khor_download "$URL.sha256" "$STAGE/checksum" 512 || exit 75
        EXPECTED=$(khor_published_digest "$STAGE/checksum" "$ASSET.gz") || {
            echo "khor: invalid checksum record for $ASSET.gz" >&2; exit 1;
        }
    fi
    ACTUAL=$(khor_file_digest "$ARCHIVE")
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
            curl --disable -fsSL --connect-timeout 15 --max-time 300 "$_url" -o "$_destination"
        fi
    elif command -v wget >/dev/null 2>&1; then
        if [ -n "$_limit" ]; then
            khor_capture "$_destination" "$_limit" out wget -qT 15 -O - "$_url"
        else
            wget -qT 15 --tries=2 -O "$_destination" "$_url"
        fi
    else
        echo "khor: needs curl or wget to fetch a release" >&2; exit 1
    fi
)

khor_file_digest() (
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

khor_activate_release() (
    set -eu
    _program="$1" _bin="$2" _asset="$3" _version="$4"
    khor_valid_tag "$_version" || { echo "khor: activation needs an exact release tag" >&2; exit 1; }
    case "$_asset" in
        khor-linux-x86_64|khor-linux-aarch64|khor-macos-arm64|khor-macos-x86_64) : ;;
        *) echo "khor: unsupported installation platform" >&2; exit 1 ;;
    esac
    _digest=$(khor_file_digest "$_program")
    _platform="$_bin/.khor/$_asset"
    _relative="versions/$_version-$_digest/khor"
    _destination="$_platform/$_relative"
    mkdir -p "${_destination%/*}"
    _stage=$(mktemp -d "$_bin/.khor.activate.XXXXXXXX")
    trap 'rm -rf "$_stage"' EXIT
    cp "$_program" "$_stage/program"
    chmod 555 "$_stage/program"
    # link is an exclusive publish on NFS too. Concurrent publishers of the
    # same content reuse it; nobody writes through an existing executable.
    [ ! -d "$_destination" ] && [ ! -L "$_destination" ] || {
        echo "khor: immutable installation is not a regular file" >&2; exit 1;
    }
    if ! ln "$_stage/program" "$_destination" 2>/dev/null; then
        [ -f "$_destination" ] && [ "$(khor_file_digest "$_destination")" = "$_digest" ] || {
            echo "khor: immutable installation differs: $_destination" >&2; exit 1;
        }
    fi
    [ ! -d "$_platform/current" ] && [ ! -d "$_bin/khor" ] || {
        echo "khor: activation destination is a directory" >&2; exit 1;
    }
    khor_launcher > "$_stage/launcher"
    chmod 755 "$_stage/launcher"
    ln -s "$_relative" "$_stage/current"
    mv -f "$_stage/current" "$_platform/current"
    mv -f "$_stage/launcher" "$_bin/khor"
)

khor_launcher() (
    cat <<'KHOR_LAUNCHER'
#!/bin/sh
set -eu
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) asset=khor-linux-x86_64 ;;
    Linux-aarch64|Linux-arm64) asset=khor-linux-aarch64 ;;
    Darwin-arm64) asset=khor-macos-arm64 ;;
    Darwin-x86_64) asset=khor-macos-x86_64 ;;
    *) echo "khor: this platform has no installed build" >&2; exit 1 ;;
esac
bin=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
# A keeper can copy this launcher before switching from a direct binary
# installation. Its original install path still identifies the shared store.
if [ ! -d "$bin/.khor" ] && [ -n "${KHOR_SHIELD_FROM:-}" ]; then
    bin=$(CDPATH= cd -- "$(dirname -- "$KHOR_SHIELD_FROM")" && pwd -P)
fi
unset KHOR_SHIELD_FROM
KHOR_INSTALL_SOURCE="$bin/.khor/$asset/current"
export KHOR_INSTALL_SOURCE
[ -x "$KHOR_INSTALL_SOURCE" ] || { echo "khor: install $asset on this machine first" >&2; exit 1; }
exec "$KHOR_INSTALL_SOURCE" "$@"
KHOR_LAUNCHER
)

khor_install_release() (
    set -eu
    _repo="$1" _version="$2" _mirror="$3" _asset="$4" _bin="$5"
    _stage=$(mktemp -d "$_bin/.khor.install.XXXXXXXX")
    trap 'rm -rf "$_stage"' EXIT
    _preplaced="${KHOR_ASSET_FILE:-}"
    if [ -n "$_preplaced" ]; then
        khor_valid_tag "$_version" || { echo "khor: preplaced installation needs its release tag" >&2; exit 1; }
        cp "$_preplaced" "$_stage/preplaced.gz"
        KHOR_ASSET_FILE="$_stage/preplaced.gz"
        export KHOR_ASSET_FILE
        _url="$KHOR_ASSET_FILE"
    else
        _selected=$(khor_select_release "$_repo" "$_version" "$_mirror" "$_asset")
        _newline='
'
        _version="${_selected%%"$_newline"*}"
        _url="${_selected#*"$_newline"}"
    fi
    khor_fetch_verify_place "$_url" "$_stage" "$_asset" "$_version" "${_mirror:+mirror}"
    _reported=$("$_stage/khor" version) || { echo "khor: downloaded build cannot run on this machine" >&2; exit 1; }
    [ "$_reported" = "${_version#v}" ] || { echo "khor: downloaded build reports a different version" >&2; exit 1; }
    khor_activate_release "$_stage/khor" "$_bin" "$_asset" "$_version"
    if [ -n "$_preplaced" ] && [ ! "$_preplaced" -ef "$_bin/khor" ]; then
        rm -f "$_preplaced" || echo "khor: installed; could not remove preplaced archive" >&2
    fi
)

khor_publish_file() (
    set -eu
    _destination="$1" _mode="$2"
    [ ! -d "$_destination" ] || { echo "khor: publication target is a directory" >&2; exit 1; }
    _temporary=$(mktemp "${_destination%/*}/.khor-write.XXXXXXXX")
    trap 'rm -f "$_temporary"' EXIT
    cat > "$_temporary"
    chmod "$_mode" "$_temporary"
    mv -f "$_temporary" "$_destination"
)

khor_systemd_unit() (
    set -eu
    case "$1" in user) _target=default.target ;; system) _target=multi-user.target ;; *) exit 1 ;; esac
    cat <<'SERVICE'
[Unit]
Description=khor resident
After=network-online.target

[Service]
Type=exec
ExecStart="%h/.local/bin/khor" serve
Restart=on-failure
RestartSec=5s
# Session hosts deliberately outlive the resident. The keeper forwards TERM
# to its own worker; systemd must not sweep independent session hosts.
KillMode=process
TimeoutStopSec=30s

[Install]
SERVICE
    printf 'WantedBy=%s\n' "$_target"
)

