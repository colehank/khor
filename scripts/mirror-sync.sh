#!/bin/sh
# Sync a khor install mirror from GitHub — run this on the one box that
# has real internet (a cheap cloud VM is plenty: the mirror is static
# files and a curl every few minutes). The fleet's no-DNS machines then
# install with:
#
#   curl -fsSL http://<mirror-ip>/khor/install.sh | KHOR_MIRROR=http://<mirror-ip>/khor sh
#
# Binaries are pinned to the latest release; install.sh is refreshed
# from main afterwards so installer fixes reach the mirror without
# waiting for a tag (the raw.githubusercontent worry in install.sh's
# header is about the *target* machines — this script runs where GitHub
# is reachable, or it would have nothing to sync at all). Every landing
# is tmp + mv: nobody downloads a half-written file.
set -eu
REPO="${KHOR_REPO:-colehank/khor}"
DIR="${KHOR_MIRROR_DIR:-/var/www/html/khor}"
API="https://api.github.com/repos/$REPO/releases/latest"
mkdir -p "$DIR"
TAG=$(curl -fsSL "$API" | grep -m1 '"tag_name"' | cut -d'"' -f4)
[ -n "$TAG" ] || { echo "khor-mirror: no tag from $API" >&2; exit 1; }
# install.sh refreshes on every run, unconditionally — its whole point
# is reaching the mirror without waiting for a tag, so it must not sit
# behind the same-tag early exit below (that exact bug shipped first:
# the mirror served a frozen installer while main moved on).
if curl -fsSL "https://raw.githubusercontent.com/$REPO/main/scripts/install.sh" -o "$DIR/.tmp.install.sh"; then
    mv -f "$DIR/.tmp.install.sh" "$DIR/install.sh"
fi
if [ -f "$DIR/VERSION" ] && [ "$(cat "$DIR/VERSION")" = "$TAG" ]; then
    exit 0
fi
curl -fsSL "$API" | grep '"browser_download_url"' | cut -d'"' -f4 | while read -r url; do
    name=$(basename "$url")
    curl -fsSL "$url" -o "$DIR/.tmp.$name"
    mv -f "$DIR/.tmp.$name" "$DIR/$name"
done
printf '%s\n' "$TAG" > "$DIR/VERSION"
echo "khor-mirror: synced $TAG"
