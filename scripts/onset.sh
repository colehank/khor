#!/bin/sh
# Install a signed release over SSH. The target downloads first; on a fetch
# failure, the controller transfers the original archive and signed metadata.
set -eu
HOST=""
MODE=install
PURGE=no
SERVE=yes
RELEASE=latest
DRY=no
INSTALL_OPTIONS=no
PRINT_UP=no
for arg in "$@"; do
    case "$arg" in
        --uninstall) MODE=uninstall ;;
        --purge) PURGE=yes ;;
        --no-serve) SERVE=no; INSTALL_OPTIONS=yes ;;
        --release) INSTALL_OPTIONS=yes ;;
        --release=*) RELEASE="${arg#--release=}"; INSTALL_OPTIONS=yes ;;
        --dry-run) DRY=yes ;;
        --print-serve-up) PRINT_UP=yes ;;
        --musl|--bin-on-target=*) echo "onset: installation requires a signed release; build development binaries separately" >&2; exit 2 ;;
        -*) echo "onset: unknown option: $arg" >&2; exit 2 ;;
        *)
            [ -z "$HOST" ] && [ -n "$arg" ] || { echo "onset: expected one ssh host" >&2; exit 2; }
            case "$arg" in *[!a-zA-Z0-9_.@:%+\[\]-]*) echo "onset: invalid ssh host" >&2; exit 2 ;; esac
            HOST="$arg" ;;
    esac
done
if [ "$MODE" = uninstall ] && { [ "$PRINT_UP" = yes ] || [ "$INSTALL_OPTIONS" = yes ]; }; then
    echo "onset: --uninstall cannot be combined with installation or print options" >&2; exit 2
fi
if [ "$PURGE" = yes ] && [ "$MODE" != uninstall ]; then
    echo "onset: --purge requires --uninstall" >&2; exit 2
fi
REPO=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if [ "$PRINT_UP" = yes ]; then
    sed -n "/^khor_publish_file \"\$UP\" 755 <<'SERVEUP'\$/,/^SERVEUP\$/p" "$REPO/scripts/install.sh" | sed '1d;$d'
    exit 0
fi
[ -n "$HOST" ] || { echo "usage: onset.sh <ssh-host> [--release[=tag]] [--no-serve] [--dry-run] [--uninstall [--purge]]" >&2; exit 2; }
if [ "$DRY" = yes ]; then
    printf 'onset: dry run — %s on %s (release %s); no connection made\n' "$MODE" "$HOST" "$RELEASE"
    [ "$PURGE" != yes ] || printf 'onset: would purge only the verified machine store; its path will be resolved on the target\n'
    exit 0
fi
STAGE=$(mktemp -d "${TMPDIR:-/tmp}/khor-onset.XXXXXXXX")
trap 'ssh -O exit -o ControlPath="$STAGE/mux" "$HOST" 2>/dev/null || :; rm -rf "$STAGE"' EXIT
ssh_() { eval "ssh ${KHOR_SSH_OPTS:-} -o ControlMaster=auto -o \"ControlPath=\$STAGE/mux\" -o ControlPersist=60 -o BatchMode=yes '$HOST' \"\$@\""; }
scp_() { eval "scp ${KHOR_SSH_OPTS:-} -o ControlMaster=auto -o \"ControlPath=\$STAGE/mux\" -o ControlPersist=60 -o BatchMode=yes -q \"\$1\" '$HOST':\"\$2\""; }
say() { printf '%s\n' "$*"; }
LIB="$REPO/scripts/onset-lib.sh"
remote() {
    cat "$LIB" "$REPO/scripts/install-components.sh" "$1" > "$STAGE/remote.sh" || return 1
    ssh_ "sh -s" < "$STAGE/remote.sh"
}
if [ "$MODE" = uninstall ]; then
    printf 'purge=%s\n' "$PURGE" > "$STAGE/off.sh"
    cat >> "$STAGE/off.sh" <<'REMOTE'
set -eu
# Resolve while the installed binary still exists. Reuse this exact store
# for stopping and purging; another machine may own the shared front store.
root=$(khor_uninstall_root)
if [ -d "$HOME/.local/bin/.khor" ] || grep -qF "# KHOR_CLI_LAUNCHER_V1" "$HOME/.local/bin/khor" 2>/dev/null; then
    echo "onset: this versioned installation may be shared; --uninstall cannot remove it for one host" >&2
    exit 1
fi
khor_stop_serve "$root/.khor/serve.pid"
rm -f "$HOME/.local/bin/khor"
# The note that started all this. Deleted rather than updated: nothing
# reads it any more, and a stale pointer left lying about is exactly
# what it was.
rm -f "$HOME/.local/bin/.khor-root"
echo "onset: binary and resident removed"
if [ "$purge" = yes ]; then
    rm -rf "$root/.khor"
    printf 'onset: store purged: %s/.khor\n' "$root"
fi
REMOTE
    say "onset: removing khor from $HOST"
    remote "$STAGE/off.sh"
    exit 0
fi


. "$LIB"
khor_install_components "$REPO/scripts/install.sh" "$REPO/scripts/install-components.sh" > "$STAGE/components.sh"
. "$STAGE/components.sh"
GH_REPO="${KHOR_REPO:-colehank/khor}"
MIRROR="${KHOR_MIRROR:-}"
quote() { printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"; }
setting() { printf 'export %s=' "$1"; quote "$2"; printf '\n'; }
installer() {
    setting KHOR_VERSION "$RELEASE"
    setting KHOR_REPO "$GH_REPO"
    setting KHOR_MIRROR "$MIRROR"
    setting KHOR_NO_SERVE "$( [ "$SERVE" = yes ] || printf 1 )"
    # Do not inherit a previous session's transfer or runtime selection.
    printf '%s\n' 'unset KHOR_HOME KHOR_SKIP_FETCH KHOR_ASSET_FILE KHOR_ASSET_SHA256 KHOR_RELEASE_FILE KHOR_RELEASE_SIGNATURE_FILE KHOR_INSTALL_SOURCE KHOR_RUNTIME_INSTALL_SOURCE'
    cat "$REPO/scripts/install.sh"
}
installer > "$STAGE/install.sh"
status=0
ssh_ "sh -s" < "$STAGE/install.sh" || status=$?
[ "$status" != 0 ] || exit 0
[ "$status" = 75 ] || { echo "onset: target installation failed ($status); no replacement attempted" >&2; exit "$status"; }

platform=$(ssh_ 'printf "%s-%s" "$(uname -s)" "$(uname -m)"')
case "$platform" in
    Linux-x86_64) ASSET=khor-linux-x86_64 ;;
    Linux-aarch64|Linux-arm64) ASSET=khor-linux-aarch64 ;;
    Darwin-arm64) ASSET=khor-macos-arm64 ;;
    Darwin-x86_64) ASSET=khor-macos-x86_64 ;;
    *) echo "onset: unsupported target platform: $platform" >&2; exit 1 ;;
esac
selected=$(khor_select_release "$GH_REPO" "$RELEASE" "$MIRROR" "$ASSET")
newline='
'
RELEASE="${selected%%"$newline"*}"
url="${selected#*"$newline"}"
# Decompression here only validates the container; a target binary never runs here.
khor_fetch_verify_place "$url" "$STAGE" "$ASSET" "$RELEASE" "${MIRROR:+mirror}" "$STAGE/archive.gz"
metadata="https://github.com/$GH_REPO/releases/download/$RELEASE"
[ -z "$MIRROR" ] || metadata="${MIRROR%/}/releases/$RELEASE"
khor_download "$metadata/khor-release.json" "$STAGE/khor-release.json" 262144
khor_download "$metadata/khor-release.json.sig" "$STAGE/khor-release.json.sig" 4096
digest=$(khor_file_digest "$STAGE/archive.gz")
landing=$(ssh_ 'mktemp -d "${TMPDIR:-/tmp}/khor-onset.XXXXXXXX"')
case "$landing" in
    *'
'*) echo "onset: invalid target transfer directory" >&2; exit 1 ;;
    /*/khor-onset.????????) ;;
    *) echo "onset: invalid target transfer directory" >&2; exit 1 ;;
esac
cleanup() { ssh_ "rm -rf -- $(quote "$landing")" || :; }
trap 'cleanup; ssh -O exit -o ControlPath="$STAGE/mux" "$HOST" 2>/dev/null || :; rm -rf "$STAGE"' EXIT
for file in archive.gz khor-release.json khor-release.json.sig; do
    scp_ "$STAGE/$file" "$(quote "$landing/$file")"
done
{
    installer | sed '$d'
    # install.sh ends with its main invocation; use its existing native signed path.
    setting KHOR_ASSET_FILE "$landing/archive.gz"
    setting KHOR_ASSET_SHA256 "$digest"
    setting KHOR_RELEASE_FILE "$landing/khor-release.json"
    setting KHOR_RELEASE_SIGNATURE_FILE "$landing/khor-release.json.sig"
    printf '%s\n' 'khor_install_main "$@"'
} > "$STAGE/transferred.sh"
ssh_ "sh -s" < "$STAGE/transferred.sh"
