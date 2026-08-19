#!/bin/sh
# Install khor on the machine that runs this, from a published release.
#
#   curl -fsSL https://raw.githubusercontent.com/OWNER/khor/main/scripts/install.sh | sh
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

if [ "$VERSION" = latest ]; then
    URL="https://github.com/$REPO/releases/latest/download/$ASSET"
else
    URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET"
fi

echo "khor: fetching $ASSET ($VERSION)"
mkdir -p "$BIN_DIR"
# Downloaded **next to its destination**, then renamed: `mv` is only
# atomic within one filesystem, and across two it degrades to
# copy-then-unlink, which can rewrite the bytes of a binary some other
# process is running. On a cluster that other process is a different
# machine's khor, reading the same NFS file.
TMP="$BIN_DIR/.khor.download.$$"
trap 'rm -f "$TMP"' EXIT
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$TMP"
elif command -v wget >/dev/null 2>&1; then
    wget -qO "$TMP" "$URL"
else
    echo "khor: needs curl or wget to fetch a release" >&2
    exit 1
fi
# A release asset that is not a program is usually GitHub's HTML for a
# tag that does not exist, and `chmod +x` on it would install a page.
case "$(head -c 4 "$TMP" | od -An -tx1 | tr -d ' \n')" in
    7f454c46|cffaedfe|cafebabe) : ;;
    *) echo "khor: $URL did not answer with a program (wrong tag?)" >&2; exit 1 ;;
esac
chmod +x "$TMP"
mv -f "$TMP" "$BIN_DIR/khor"
trap - EXIT

if ! grep -qs 'HOME/.local/bin' "$HOME/.profile" 2>/dev/null; then
    printf '%s\n' 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.profile"
    echo "khor: added ~/.local/bin to PATH in ~/.profile"
fi

"$BIN_DIR/khor" id

# One store, one machine. A cluster mounts one home everywhere, so
# `~/.khor` can already belong to a different box; khor refuses to open
# somebody else's store, and this puts one of this machine's own in
# front of that refusal.
ROOT="$HOME"
if [ -f "$HOME/.khor/owner" ] && [ "$(cat "$HOME/.khor/owner")" != "$(hostname)" ]; then
    ROOT="$HOME/.khor-hosts/$(hostname)"
    mkdir -p "$ROOT/.khor"
    echo "khor: this home belongs to $(cat "$HOME/.khor/owner") — $(hostname) gets its own store at $ROOT"
    if ! grep -qs 'KHOR_HOME' "$HOME/.profile" 2>/dev/null; then
        printf 'export KHOR_HOME="$HOME/.khor-hosts/$(hostname)"\n' >> "$HOME/.profile"
    fi
fi

[ -n "${KHOR_NO_SERVE:-}" ] && exit 0

PIDF="$ROOT/.khor/serve.pid"
mkdir -p "$ROOT/.khor"
# Stopped by its own pid file, never by name: `pkill -f khor` on a
# shared machine is somebody else's outage.
if [ -f "$PIDF" ] && kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    kill "$(cat "$PIDF")" 2>/dev/null || true
    sleep 1
fi
KHOR_HOME="$ROOT" setsid nohup "$BIN_DIR/khor" serve >> "$ROOT/.khor/serve.log" 2>&1 &
echo $! > "$PIDF"
sleep 2
if kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    echo "khor: serve is up (pid $(cat "$PIDF"), store $ROOT)"
else
    echo "khor: serve died on start — tail of $ROOT/.khor/serve.log:" >&2
    tail -5 "$ROOT/.khor/serve.log" >&2
    exit 1
fi
echo "khor: done. join the mesh with — khor pair <ticket>"
