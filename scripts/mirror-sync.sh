#!/bin/sh
# Publish a complete immutable release, then atomically select it with VERSION.
# Existing releases and legacy flat assets are retained for pinned readers.
set -eu
REPO="${KHOR_REPO:-colehank/khor}"
DIR="${KHOR_MIRROR_DIR:-/var/www/html/khor}"
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$HERE/onset-lib.sh"
khor_install_components "$HERE/install.sh" "$HERE/install-components.sh" >/dev/null
. "$HERE/install-components.sh"
. "$HERE/mirror-lib.sh"
mkdir -p "$DIR"
mkdir "$DIR/.sync.lock" 2>/dev/null || { echo "khor-mirror: another writer holds the publication lock" >&2; exit 1; }
STAGE=""
trap '[ -z "$STAGE" ] || rm -rf "$STAGE"; rmdir "$DIR/.sync.lock"' EXIT
STAGE=$(mktemp -d "$DIR/.release.XXXXXXXX")
SELECTED=$(khor_select_release "$REPO" "${KHOR_VERSION:-latest}" "" khor-linux-x86_64)
TAG=$(printf '%s\n' "$SELECTED" | head -n 1)
ENCODED_TAG=$(printf '%s' "$TAG" | sed 's/+/%2B/g')
BASE="https://github.com/$REPO/releases/download/$ENCODED_TAG"
for ASSET in khor-linux-x86_64 khor-linux-aarch64 khor-macos-arm64 khor-macos-x86_64; do
    khor_download "$BASE/$ASSET.gz" "$STAGE/$ASSET.gz"
    khor_download "$BASE/$ASSET.gz.sha256" "$STAGE/$ASSET.gz.sha256" 512
done
khor_download "$BASE/install.sh" "$STAGE/install.sh" 262144
# Only publish installers that implement the matching immutable layout. This
# checks declarations; it never executes the downloaded installer or helpers.
grep -qx '# KHOR_MIRROR_LAYOUT=versioned-v1' "$STAGE/install.sh" || {
    echo "khor-mirror: release installer predates the versioned mirror contract" >&2; exit 1;
}
/bin/sh -n "$STAGE/install.sh"
khor_commit_mirror_release "$DIR" "$STAGE" "$TAG"
echo "khor-mirror: synced $TAG"
