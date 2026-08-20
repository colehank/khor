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
#   KHOR_MIRROR    base URL of a mirror carrying install.sh + the
#                  release assets, for machines that cannot reach
#                  GitHub at all (the no-DNS half of a fleet):
#                    curl -fsSL http://<mirror>/khor/install.sh \
#                      | KHOR_MIRROR=http://<mirror>/khor sh
#                  The mirror is plain static files — any box that can
#                  reach GitHub can host one (scripts/mirror-sync.sh).
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

# Compressed on the wire: the release ships `.gz` because the download
# is the slow half of an install (measured at ~95 KB/s from the machines
# this runs on — 8.9 MB instead of 29.7).
if [ -n "${KHOR_MIRROR:-}" ]; then
    # The mirror carries exactly one version (whatever its sync last
    # pulled), so KHOR_VERSION means nothing here — the checks below
    # (gzip, magic bytes) still stand between a bad mirror and $BIN_DIR.
    URL="${KHOR_MIRROR%/}/$ASSET.gz"
elif [ "$VERSION" = latest ]; then
    URL="https://github.com/$REPO/releases/latest/download/$ASSET.gz"
else
    URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET.gz"
fi

echo "khor: fetching $ASSET ($VERSION)"
mkdir -p "$BIN_DIR"
# Downloaded **next to its destination**, then renamed: `mv` is only
# atomic within one filesystem, and across two it degrades to
# copy-then-unlink, which can rewrite the bytes of a binary some other
# process is running. On a cluster that other process is a different
# machine's khor, reading the same NFS file.
TMP="$BIN_DIR/.khor.download.$$"
trap 'rm -f "$TMP" "$TMP.gz"' EXIT
# Downloaded then decompressed, never piped through gzip: a plain `sh`
# has no `pipefail`, so a failed download would reach gzip as an empty
# stream and the install would report a corrupt archive instead of a
# network that did not answer.
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$TMP.gz"
elif command -v wget >/dev/null 2>&1; then
    wget -qO "$TMP.gz" "$URL"
else
    echo "khor: needs curl or wget to fetch a release" >&2
    exit 1
fi
gzip -dc "$TMP.gz" > "$TMP" || { echo "khor: $URL is not a gzip (wrong tag?)" >&2; exit 1; }
rm -f "$TMP.gz"
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
R="$HOME"
if [ -f "$HOME/.khor/owner" ] && [ "$(cat "$HOME/.khor/owner")" != "$(hostname)" ]; then
    R="$HOME/.khor-hosts/$(hostname)"
fi
PIDF="$R/.khor/serve.pid"
mkdir -p "$R/.khor"
if [ -f "$PIDF" ] && kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    exit 0
fi
# setsid where it exists (Linux); launchd/nohup orphaning does the same
# job on macOS, which has no setsid.
if command -v setsid >/dev/null 2>&1; then
    KHOR_HOME="$R" setsid nohup "$BIN" serve >> "$R/.khor/serve.log" 2>&1 &
else
    KHOR_HOME="$R" nohup "$BIN" serve >> "$R/.khor/serve.log" 2>&1 &
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
    ;;
Linux)
    if command -v systemctl >/dev/null 2>&1 && [ "$(id -u)" = 0 ]; then
        cat > /etc/systemd/system/khor-serve.service <<UNIT
[Unit]
Description=khor serve (boot pull-up; the keeper inside handles the rest)
After=network-online.target

[Service]
Type=oneshot
Environment=HOME=$HOME
ExecStart=$UP

[Install]
WantedBy=multi-user.target
UNIT
        systemctl daemon-reload
        systemctl enable khor-serve.service >/dev/null 2>&1
        GUARDIAN="systemd (system unit khor-serve.service)"
    elif command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
        mkdir -p "$HOME/.config/systemd/user"
        cat > "$HOME/.config/systemd/user/khor-serve.service" <<UNIT
[Unit]
Description=khor serve (boot pull-up; the keeper inside handles the rest)

[Service]
Type=oneshot
ExecStart=$UP

[Install]
WantedBy=default.target
UNIT
        systemctl --user daemon-reload
        systemctl --user enable khor-serve.service >/dev/null 2>&1
        # Without lingering the unit only fires at login — same promise
        # as the .profile line below, just tidier. With it, at boot.
        loginctl enable-linger "$(id -un)" >/dev/null 2>&1 \
            && GUARDIAN="systemd (user unit, lingering on: starts at boot)" \
            || GUARDIAN="systemd (user unit: starts at your first login)"
    fi
    # Belt and suspenders for every non-root tier: the first person to
    # log in after a reboot becomes the restart button.
    if [ "$(id -u)" != 0 ] && ! grep -qs 'khor-serve-up' "$HOME/.profile" 2>/dev/null; then
        printf '[ -x "$HOME/.local/bin/khor-serve-up" ] && "$HOME/.local/bin/khor-serve-up" >/dev/null 2>&1 || true\n' >> "$HOME/.profile"
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
if kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    echo "khor: serve is up (pid $(cat "$PIDF"), store $ROOT)"
    echo "khor: reboots handled by $GUARDIAN"
else
    echo "khor: serve died on start — tail of $ROOT/.khor/serve.log:" >&2
    tail -5 "$ROOT/.khor/serve.log" >&2
    exit 1
fi
echo "khor: done. join the mesh with — khor pair <ticket>"
