#!/bin/sh
# Land pages, upload an isolated generation, then publish its verified pointer.
set -eu
HOST="${1:?usage: deploy/push.sh <ssh-host>}"
ROOT="${KHOR_SITE_ROOT:-/srv/khor-site}"
case "$ROOT" in /*) : ;; *) echo "site: KHOR_SITE_ROOT must be absolute" >&2; exit 1 ;; esac
case "$ROOT" in *'
'*) echo "site: invalid site root" >&2; exit 1 ;; esac
cd "$(dirname "$0")/.."
# Paths pass through rsync's remote shell; require literal argument transport.
rsync --help | grep -q -- '--protect-args' || { echo "site: rsync with --protect-args is required" >&2; exit 1; }
npm run build
KHOR_MIRROR_DIR="$PWD/.mirror" sh ../../scripts/mirror-sync.sh
. ../../scripts/install-components.sh
. ../../scripts/mirror-lib.sh
TAG=$(khor_descriptor_tag .mirror/VERSION)
RELEASE="$PWD/.mirror/releases/$TAG"
khor_validate_mirror_release "$RELEASE"
quote() { printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"; }
DL="$ROOT/dl"
REMOTE_STAGE=""
BUNDLE=$(mktemp "$PWD/.mirror/.publish.XXXXXXXX")
cleanup() {
    rm -f "$BUNDLE"
    if [ -n "$REMOTE_STAGE" ]; then
        ssh -- "$HOST" "rm -rf -- $(quote "$REMOTE_STAGE")" </dev/null >/dev/null 2>&1 || { echo "site: remote upload stage cleanup could not be confirmed" >&2; }
    fi
}
trap cleanup EXIT
rsync -az --protect-args --delete --exclude=/dl/ dist/ "$HOST:$ROOT/"
REMOTE_STAGE=$(ssh -- "$HOST" "umask 077; mkdir -p $(quote "$DL") && mktemp -d $(quote "$DL/.incoming.XXXXXXXX")")
case "$REMOTE_STAGE" in "$DL"/.incoming.*) NONCE="${REMOTE_STAGE#"$DL"/.incoming.}" ;; *) echo "site: invalid remote stage receipt" >&2; REMOTE_STAGE=""; exit 1 ;; esac
case "$NONCE" in ''|*[!a-zA-Z0-9]*) echo "site: invalid remote stage receipt" >&2; REMOTE_STAGE=""; exit 1 ;; esac
[ "${#NONCE}" = 8 ] || { echo "site: invalid remote stage receipt" >&2; REMOTE_STAGE=""; exit 1; }
rsync -az --protect-args "$RELEASE/" "$HOST:$REMOTE_STAGE/"
# The remote command receives our checked-in helpers, not a downloaded script.
cat ../../scripts/install-components.sh ../../scripts/mirror-lib.sh > "$BUNDLE"
cat >> "$BUNDLE" <<'PUBLISH'
set -eu
trap 'rm -rf "$2"' EXIT
khor_publish_mirror_release "$1" "$2" "$3"
PUBLISH
ssh -- "$HOST" "sh -s -- $(quote "$DL") $(quote "$REMOTE_STAGE") $(quote "$TAG")" < "$BUNDLE"
REMOTE_STAGE=""
