#!/bin/sh
# Build a development binary on a provisioned Linux builder without installing it.
# Usage: scripts/build-on.sh [<ssh-host>] --no-install
# Production deployment uses onset.sh and signed release artifacts.
set -eu

HOST=turing
INSTALL=yes
for arg in "$@"; do
    case "$arg" in
        --no-install) INSTALL=no ;;
        --no-serve) : ;;
        -*) echo "unknown option: $arg" >&2; exit 2 ;;
        *) HOST="$arg" ;;
    esac
done

[ "$INSTALL" = no ] || { echo "build-on: development binaries are not deployable releases; use --no-install, or onset.sh for a signed release" >&2; exit 2; }

REPO=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
say() { printf '%s\n' "$*"; }
ssh_() { eval "ssh ${KHOR_SSH_OPTS:-} -o BatchMode=yes '$HOST' \"\$@\""; }

# ── is this box a「here」 ───────────────────────────────────
# Three facts, asked of the box: the toolchain it carries, the floor it
# would bake in, and whether its cores are somebody else's right now.
FACTS=$(ssh_ 'set -u
cargo="$HOME/.cargo/bin/cargo"
[ -x "$cargo" ] && ver=$("$cargo" --version 2>/dev/null | cut -d" " -f2) || ver=""
glibc=$(ldd --version 2>/dev/null | head -1 | sed "s/.* //")
cores=$(nproc)
load=$(cut -d" " -f1 /proc/loadavg)
free=$(df -Pk /tmp | awk "NR==2{print int(\$4/1048576)}")
printf "%s %s %s %s %s %s\n" "$(uname -m)" "${ver:-none}" "${glibc:-?}" "$cores" "$load" "$free"' | tr -d '\r')
[ -n "$FACTS" ] || { echo "build-on: $HOST did not answer (ssh)" >&2; exit 1; }
set -- $FACTS
ARCH=$1 CARGO_VER=$2 GLIBC=$3 CORES=$4 LOAD=$5 FREE_GB=$6
say "build-on: $HOST is $ARCH, cargo $CARGO_VER, glibc $GLIBC, $CORES cores, load $LOAD, ${FREE_GB}G free on /tmp"

[ "$CARGO_VER" != none ] || { echo "build-on: $HOST has no ~/.cargo/bin/cargo — this script builds, it does not install toolchains" >&2; exit 1; }
case "$ARCH" in
    x86_64|aarch64) : ;;
    *) echo "build-on: no fleet target for $ARCH" >&2; exit 1 ;;
esac
# The floor onset pins (2.31 = Ubuntu 20.04). A box newer than it would
# build a binary that *runs here* and refuses to start on the fleet's
# older members — the failure that took fodor down, from the other side.
FLOOR=2.31
if [ "$GLIBC" != "$FLOOR" ]; then
    echo "build-on: $HOST's glibc is $GLIBC, and the fleet's floor is $FLOOR — a native build here would raise it." >&2
    echo "build-on: build on a $FLOOR box, or from a Mac with onset.sh (zig pins the floor)." >&2
    exit 1
fi
# Half the cores busy means somebody is using this machine; their job
# is not our build's to slow down. The number is the box's, so a
# 128-core node refuses at 64 and a 32-core one at 16.
if [ "${KHOR_BUILD_FORCE:-0}" != 1 ] && [ "$(printf '%s\n' "$LOAD $((CORES / 2))" | awk '{print ($1 > $2)}')" = 1 ]; then
    echo "build-on: $HOST is busy (load $LOAD on $CORES cores) — somebody is working there. KHOR_BUILD_FORCE=1 overrides." >&2
    exit 1
fi
[ "$FREE_GB" -ge 10 ] || { echo "build-on: only ${FREE_GB}G free on $HOST's /tmp; a target dir needs more" >&2; exit 1; }

# ── HEAD, exported ────────────────────────────────────────
HEAD=$(cd "$REPO" && git rev-parse --short HEAD)
DIRTY=$(cd "$REPO" && git status --porcelain | wc -l | tr -d ' ')
EXPORT="${TMPDIR:-/tmp}/khor-build-on.$$"
mkdir -p "$EXPORT"
trap 'rm -rf "$EXPORT"' EXIT
( cd "$REPO" && git archive --format=tar HEAD ) | tar -x -C "$EXPORT"
if [ "$DIRTY" != 0 ]; then
    say "build-on: shipping HEAD $HEAD; the working tree's $DIRTY uncommitted change(s) stay here"
else
    say "build-on: shipping HEAD $HEAD"
fi

# ── the face, in the export ───────────────────────────────
say "build-on: building the web face (apps/gui at $HEAD)"
ln -s "$REPO/apps/gui/node_modules" "$EXPORT/apps/gui/node_modules"
# `npm run build` runs `npm run gen` first, which runs cargo on the
# catalog and the bindings; pointed at the repo's target dir so it
# reuses what is already compiled instead of starting a second one.
( cd "$EXPORT" && CARGO_TARGET_DIR="$REPO/target" npm --prefix apps/gui run build >"$EXPORT/gui-build.log" 2>&1 ) \
    || { echo "build-on: the web face did not build at $HEAD:" >&2; tail -15 "$EXPORT/gui-build.log" >&2; exit 1; }
[ -f "$EXPORT/apps/gui/dist/index.html" ] || { echo "build-on: no apps/gui/dist after the build" >&2; exit 1; }
rm "$EXPORT/apps/gui/node_modules"

# Pi is part of the executable too. Compile for the remote architecture
# on this Mac with pinned Bun; the server needs neither Node nor npm.
case "$ARCH" in
    x86_64) AGENT_TARGET=x86_64-unknown-linux-gnu ;;
    aarch64) AGENT_TARGET=aarch64-unknown-linux-gnu ;;
esac
say "build-on: bundling Pi for $AGENT_TARGET"
ln -s "$REPO/apps/agent/node_modules" "$EXPORT/apps/agent/node_modules"
( cd "$EXPORT" && node apps/agent/build.mjs "$AGENT_TARGET" >"$EXPORT/agent-build.log" 2>&1 ) \
    || { echo "build-on: Pi did not bundle at $HEAD:" >&2; tail -20 "$EXPORT/agent-build.log" >&2; exit 1; }
rm "$EXPORT/apps/agent/node_modules"
[ -s "$EXPORT/apps/agent/dist/khor-agent" ] || { echo "build-on: no Pi bundle" >&2; exit 1; }

# ── the source, over ──────────────────────────────────────
# `$USER` on the far side is asked, not assumed: the login name there
# is not this Mac's.
RUSER=$(ssh_ 'printf %s "$USER"' | tr -d '\r')
SRC="/tmp/khor-src-$RUSER"
TGT="/tmp/khor-target-$RUSER"
say "build-on: syncing the tree to $HOST:$SRC"
ssh_ "mkdir -p '$SRC' '$TGT'"
# The export is already the tree minus `.git` and node_modules. Every
# workspace member stays — cargo loads each member's manifest whether
# or not it builds it, so leaving `apps/gui/core` or
# `apps/gui/src-tauri` behind is a missing-member error, not a saving.
eval "rsync -a --delete \
    --exclude '/target' --exclude '/gui-build.log' \
    -e \"ssh ${KHOR_SSH_OPTS:-} -o BatchMode=yes\" \
    '$EXPORT/' '$HOST:$SRC/'"

# ── the build, there ──────────────────────────────────────
# The whole workspace's dependencies compile the first time (minutes on
# many cores); every run after that is incremental against $TGT.
say "build-on: cargo build --profile local -p khor-cli on $HOST (target $TGT)"
ssh_ "cd '$SRC' && if PATH=\"\$HOME/.cargo/bin:\$PATH\" CARGO_INCREMENTAL=0 CARGO_TARGET_DIR='$TGT' KHOR_AGENT_BUNDLE='$SRC/apps/agent/dist/khor-agent' cargo build --profile local -p khor-cli --bin khor >'$TGT/build.log' 2>&1; then tail -8 '$TGT/build.log'; else tail -60 '$TGT/build.log'; exit 1; fi"
PRODUCT="$TGT/local/khor"
HASH=$(ssh_ "sha256sum '$PRODUCT' 2>/dev/null | cut -c1-24" | tr -d '\r')
[ -n "$HASH" ] || { echo "build-on: no product at $HOST:$PRODUCT" >&2; exit 1; }
say "build-on: built $PRODUCT (${HASH}…)"

say "build-on: --no-install; the development product is at $HOST:$PRODUCT"
