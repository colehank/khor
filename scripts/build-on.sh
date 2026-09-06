#!/bin/sh
# Build khor **on** a Linux box that already carries a toolchain, and
# install what it built through `onset.sh --bin-on-target`.
#
#   scripts/build-on.sh [<ssh-host>] [--no-install] [--no-serve]
#
# `onset.sh`'s rule is「built here, never there」, and this is its one
# deliberate exception, for a box that is a better「here」than this Mac:
#
# - **Its glibc is the floor.** onset pins the fleet's floor at 2.31 by
#   asking zig for it; a machine whose own glibc *is* 2.31 produces that
#   floor natively, with no cross toolchain at all. That is checked
#   below, not assumed — a newer box would raise the floor for every
#   machine sharing the binary, silently, and the check refuses it.
# - **Its product lands where the fleet reads it.** On a cluster with
#   one NFS home, `~/.local/bin/khor` is the same file on every machine;
#   built on the mount's own side, the binary is already there and
#   nothing crosses the wire but the source.
# - **It has the cores and the disk** this Mac's 97%-full target volume
#   does not.
#
# What it costs, said up front: the box is shared. The load is read and
# a busy machine is refused unless `KHOR_BUILD_FORCE=1` — somebody's
# job waiting on the cores is a person waiting, not a number.
#
# **What ships is HEAD, never the working tree.** This tree usually has
# two or three lanes' half-finished edits in it at once, and the first
# run of this script shipped exactly that to three machines — a deleted
# component, a card mid-edit — because rsync sends what is on disk. So
# the source is `git archive HEAD` into a scratch dir, and the run says
# which commit it is shipping and how many files it left behind.
#
# **The face is built in that export, first, every time.**
# `crates/web/build.rs` compiles `apps/gui/dist` into the binary, and a
# cargo run that finds a stale dist ships a stale face (measured
# 2026-09-06: the deployed binary's web face lacked the strings the
# tree had, because the GUI was built *after* the CLI). The export has
# no node_modules of its own; the repo's is borrowed by symlink.
#
# Source and target dirs live on the box's **local** disk, never in the
# NFS home: rsync into an NFS tree is minutes, cargo's target dir on
# NFS is hours. `~/.cargo` (the toolchain and the registry cache) is
# whatever the box has — on this fleet that is the NFS home, shared by
# every node, which is fine for a cache and a compiler.
#
# `KHOR_SSH_OPTS` is passed to ssh and rsync verbatim, as onset does.
set -eu

HOST=turing
INSTALL=yes
SERVE_FLAG=""
for arg in "$@"; do
    case "$arg" in
        --no-install) INSTALL=no ;;
        --no-serve) SERVE_FLAG=--no-serve ;;
        -*) echo "unknown option: $arg" >&2; exit 2 ;;
        *) HOST="$arg" ;;
    esac
done

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
say "build-on: cargo build --release -p khor-cli on $HOST (target $TGT)"
ssh_ "cd '$SRC' && PATH=\"\$HOME/.cargo/bin:\$PATH\" CARGO_TARGET_DIR='$TGT' cargo build --release -p khor-cli --bin khor 2>&1 | grep -E '^(error|warning: unused|Finished|   Compiling khor)' | tail -5"
PRODUCT="$TGT/release/khor"
HASH=$(ssh_ "sha256sum '$PRODUCT' 2>/dev/null | cut -c1-24" | tr -d '\r')
[ -n "$HASH" ] || { echo "build-on: no product at $HOST:$PRODUCT" >&2; exit 1; }
say "build-on: built $PRODUCT (${HASH}…)"

if [ "$INSTALL" = no ]; then
    say "build-on: --no-install; the product is at $HOST:$PRODUCT"
    exit 0
fi

# ── into the fleet's file, through the same gate as every install ──
# Copied under `~/.local/bin` first (the NFS side, where the fleet's
# binary lives), then handed to onset, whose remote half asks it for
# its version before anything is replaced and restarts the resident.
STAGE_NAME=".khor.built.$$"
ssh_ "mkdir -p \"\$HOME/.local/bin\" && cp '$PRODUCT' \"\$HOME/.local/bin/$STAGE_NAME\""
say "build-on: staged as ~/.local/bin/$STAGE_NAME on $HOST; installing through onset.sh"
exec "$REPO/scripts/onset.sh" "$HOST" "--bin-on-target=\$HOME/.local/bin/$STAGE_NAME" $SERVE_FLAG
