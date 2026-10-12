#!/bin/sh
# Build the site and land it on the machine that hosts it.
#
#   deploy/push.sh <ssh-host>
#
# The install mirror (/dl) is synced **here**, then carried over: the
# host is in the mainland and cannot be counted on to reach GitHub,
# which is the whole reason the mirror exists. scripts/mirror-sync.sh
# returns early when the release has not moved, so a push that only
# changes a page downloads nothing.
set -eu
HOST="${1:?usage: deploy/push.sh <ssh-host>}"
ROOT="${KHOR_SITE_ROOT:-/srv/khor-site}"
cd "$(dirname "$0")/.."
npm run build
KHOR_MIRROR_DIR="$PWD/.mirror" sh ../../scripts/mirror-sync.sh
rsync -az --delete --exclude /dl/ dist/ "$HOST:$ROOT/"
rsync -az --delete --exclude '.tmp.*' .mirror/ "$HOST:$ROOT/dl/"
