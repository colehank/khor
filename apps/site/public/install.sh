#!/bin/sh
# khor.cloud's front door to the installer:
#
#   curl -fsSL https://khor.cloud/install.sh | sh
#
# The installer itself is scripts/install.sh in the repo; this only
# points it at this site's mirror (/dl, kept by scripts/mirror-sync.sh)
# so a machine that cannot reach GitHub installs all the same. Every
# variable install.sh reads passes straight through.
#
# Fetched to a file, then run: a plain `sh` has no `pipefail`, and a
# download that failed would reach `sh` as an empty script that exits 0.
set -eu
KHOR_MIRROR="${KHOR_MIRROR:-https://khor.cloud/dl}"
export KHOR_MIRROR
TMP="${TMPDIR:-/tmp}/khor-install.$$"
trap 'rm -f "$TMP"' EXIT
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$KHOR_MIRROR/install.sh" -o "$TMP"
elif command -v wget >/dev/null 2>&1; then
    wget -qO "$TMP" "$KHOR_MIRROR/install.sh"
else
    echo "khor: needs curl or wget to fetch a release" >&2
    exit 1
fi
sh "$TMP"
