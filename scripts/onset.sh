#!/bin/sh
# Put khor on a machine, from a machine that already has it.
#
# The one hard rule this encodes: **the target needs nothing.** No Rust,
# no package manager, no root, no network of its own. A machine that can
# be reached over ssh can join the mesh — the binary is built here,
# against a named glibc floor (2.31 = Ubuntu 20.04), so nothing has to
# be installed over there to run it.
#
#   scripts/onset.sh <ssh-host> [--musl] [--uninstall] [--purge] [--no-serve]
#
# **glibc by default, static only when asked.** A statically linked
# binary is the tidier idea — it runs on any kernel, no version floor —
# but it has no dynamic loader, so it can never `dlopen` a driver
# library. Measured on one machine at one moment: the glibc build read
# `GPU 0% / 2 卡 显存 43.0G / 48.0G`, the musl build reported no GPU at
# all (`vitals/gpu/nvidia.rs`). "Cannot ask" and "there is none" are the
# same empty field, so the choice is made here, where the target's
# hardware is still visible. `--musl` exists for boxes older than glibc
# 2.31, and refuses to go to a machine with an NVIDIA driver.
#
# `KHOR_SSH_OPTS` is passed to ssh and scp verbatim — some networks need
# a ProxyCommand to reach the box at all, and that belongs to the person
# running this, not to khor:
#
#   KHOR_SSH_OPTS="-o ProxyCommand='nc -X 5 -x 127.0.0.1:7897 %h %p'" \
#     scripts/onset.sh turing
#
# Idempotent: running it twice is running it once.
#
# **The remote half is a file, not a quoted string.** It grew one nested
# level too many the first time (a here-string inside ssh inside eval),
# and quoting bugs in an installer are the kind that half-install.
set -eu

HOST=""
MODE=install
PURGE=no
SERVE=yes
LIBC=gnu
for arg in "$@"; do
    case "$arg" in
        --uninstall) MODE=uninstall ;;
        --purge) PURGE=yes ;;
        --no-serve) SERVE=no ;;
        --musl) LIBC=musl ;;
        -*) echo "unknown option: $arg" >&2; exit 2 ;;
        *) HOST="$arg" ;;
    esac
done
[ -n "$HOST" ] || { echo "usage: onset.sh <ssh-host> [--musl] [--uninstall] [--purge] [--no-serve]" >&2; exit 2; }

REPO=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
STAGE="${TMPDIR:-/tmp}/khor-onset.$$"
mkdir -p "$STAGE"
trap 'ssh -O exit -o ControlPath="$STAGE/mux" "$HOST" 2>/dev/null; rm -rf "$STAGE"' EXIT

# **One TCP connection for the whole run.** A run asks the target half a
# dozen things (what it is, where its home is, install, start), and
# opening a session for each is how a shared box's sshd starts refusing:
# measured on a machine with 41 logged-in users, where the fourth run in
# a row came back `Connection closed by UNKNOWN port 65535` while a
# quieter neighbour answered fine. Multiplexing makes the count one, and
# the master retires on its own a minute later.
MUX="-o ControlMaster=auto -o ControlPath=$STAGE/mux -o ControlPersist=60"
# One quoting rule for both tools. `eval` because a ProxyCommand carries
# its own quotes.
ssh_() { eval "ssh $MUX ${KHOR_SSH_OPTS:-} -o BatchMode=yes '$HOST' \"\$@\""; }
scp_() { eval "scp $MUX ${KHOR_SSH_OPTS:-} -o BatchMode=yes -q \"\$1\" '$HOST':\"\$2\""; }
say() { printf '%s\n' "$*"; }

# **Never a login shell.** A login shell sources the target's
# `~/.profile`, and `.` is a POSIX *special builtin*: when it fails, a
# non-interactive shell exits on the spot. One box of this fleet carries
# a stale `. "/tmp/.../bin/env"` there — measured, and it killed an
# install right after the file copy, leaving a binary in /tmp and
# nothing else. Everything below uses absolute paths, so no PATH is
# needed to install; the PATH line written into `.profile` is for the
# *person* who logs in later.
remote() { ssh_ "sh -s" < "$1"; }

# ── uninstall ────────────────────────────────────────────
if [ "$MODE" = uninstall ]; then
    cat > "$STAGE/off.sh" <<'REMOTE'
set -eu
for root in "$HOME" "$HOME/.khor-hosts/$(hostname)"; do
    pidf="$root/.khor/serve.pid"
    if [ -f "$pidf" ] && kill -0 "$(cat "$pidf")" 2>/dev/null; then
        kill "$(cat "$pidf")" 2>/dev/null || true
        sleep 1
    fi
    rm -f "$pidf"
done
rm -f "$HOME/.local/bin/khor"
echo "onset: binary and resident removed"
REMOTE
    say "onset: removing khor from $HOST"
    remote "$STAGE/off.sh"
    if [ "$PURGE" = yes ]; then
        # Data is the one thing an uninstall must not take by default:
        # the device identity lives here, and a machine that loses it
        # rejoins the mesh as a stranger.
        cat > "$STAGE/purge.sh" <<'REMOTE'
set -eu
rm -rf "$HOME/.khor" "$HOME/.khor-hosts/$(hostname)"
echo "onset: store purged"
REMOTE
        remote "$STAGE/purge.sh"
    fi
    exit 0
fi

# ── what the target is ───────────────────────────────────
ARCH=$(ssh_ 'uname -m' | tr -d '\r')
# An empty answer is a connection that dropped, not a CPU nobody
# supports — and "no build for  yet" reads exactly like the latter.
[ -n "$ARCH" ] || { echo "onset: $HOST did not answer what it is (ssh)" >&2; exit 1; }
# The glibc floor is named, not inherited: zig builds against exactly
# 2.31 (Ubuntu 20.04), so the result runs on that and everything newer,
# and a machine older than it fails loudly at exec instead of subtly.
case "$ARCH-$LIBC" in
    x86_64-gnu) TARGET=x86_64-unknown-linux-gnu; ZIGTARGET=x86_64-unknown-linux-gnu.2.31 ;;
    x86_64-musl) TARGET=x86_64-unknown-linux-musl; ZIGTARGET=$TARGET ;;
    aarch64-gnu|arm64-gnu) TARGET=aarch64-unknown-linux-gnu; ZIGTARGET=aarch64-unknown-linux-gnu.2.31 ;;
    aarch64-musl|arm64-musl) TARGET=aarch64-unknown-linux-musl; ZIGTARGET=$TARGET ;;
    *) echo "onset: no build for $ARCH yet" >&2; exit 1 ;;
esac
say "onset: $HOST is $ARCH → $ZIGTARGET"

if [ "$LIBC" = musl ] && ssh_ 'command -v nvidia-smi >/dev/null 2>&1'; then
    echo "onset: $HOST has an NVIDIA driver, and a static build cannot load NVML —" >&2
    echo "       it would report no GPU on a machine that has one. Drop --musl." >&2
    exit 1
fi

# ── built here, never there ──────────────────────────────
# Compiling on the target is the thing this script exists to avoid:
# these boxes are shared (one carries 40 logged-in users), their homes
# are on NFS, and a toolchain install is a gigabyte of somebody else's
# disk.
BIN="$REPO/target/$TARGET/release/khor"
say "onset: building $ZIGTARGET (this machine)"
( cd "$REPO" && cargo zigbuild -q -p khor-cli --bin khor --target "$ZIGTARGET" --release )
[ -f "$BIN" ] || { echo "onset: no binary at $BIN" >&2; exit 1; }
# The one thing worth checking about the artefact is that it is the
# *asked-for* one: a stale file from the other libc under a target dir
# that looks right is exactly the kind of thing that installs fine and
# then behaves like the build nobody chose.
case "$LIBC-$(file "$BIN")" in
    musl-*statically\ linked*) : ;;
    gnu-*dynamically\ linked*) : ;;
    *) echo "onset: $BIN is not a $LIBC build" >&2; exit 1 ;;
esac

say "onset: sending $(wc -c < "$BIN" | tr -d ' ') bytes"
# **Landed next to its destination, not in /tmp.** `mv` is only atomic
# within one filesystem; across two it degrades to copy-then-unlink,
# which can rewrite the bytes of a binary some other process is running
# — and on a cluster that other process is a different machine's
# resident serve, reading the same NFS file. Same directory, real
# `rename(2)`, old inode left alone for whoever is still running it.
ssh_ 'mkdir -p "$HOME/.local/bin"'
# scp does not expand the remote shell's variables, so the home is
# asked for rather than spelled.
REMOTE_HOME=$(ssh_ 'printf %s "$HOME"' | tr -d '\r')
scp_ "$BIN" "$REMOTE_HOME/.local/bin/.khor.onset$$"

# ── install ──────────────────────────────────────────────
# The store's root is decided **on the target**, because only it knows
# whether its home is its own: a cluster mounts one NFS export on every
# machine, so `~/.khor` is literally the same directory on all of them
# (measured on this fleet). khor refuses to open another machine's store
# — this puts one of its own in front of that refusal.
cat > "$STAGE/on.sh" <<REMOTE
set -eu
src="$REMOTE_HOME/.local/bin/.khor.onset$$"
REMOTE
cat >> "$STAGE/on.sh" <<'REMOTE'
mkdir -p "$HOME/.local/bin"
chmod +x "$src"
# rename, not copy: upgrading over a *running* khor works because Linux
# keeps the open inode, and the next start picks up the new file.
mv -f "$src" "$HOME/.local/bin/khor"

root="$HOME"
owner=""
[ -f "$HOME/.khor/owner" ] && owner=$(cat "$HOME/.khor/owner")
if [ -n "$owner" ] && [ "$owner" != "$(hostname)" ]; then
    root="$HOME/.khor-hosts/$(hostname)"
    echo "onset: this home belongs to $owner — $(hostname) gets its own store at $root"
fi
mkdir -p "$root/.khor"
printf '%s\n' "$root" > "$HOME/.local/bin/.khor-root"

# The PATH line, and KHOR_HOME when it is not the default. Written once:
# appending on every run is how a .profile ends up with twelve copies.
if ! grep -qs 'HOME/.local/bin' "$HOME/.profile" 2>/dev/null; then
    printf '%s\n' 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.profile"
    echo "onset: PATH line added to ~/.profile"
fi
if [ "$root" != "$HOME" ] && ! grep -qs 'KHOR_HOME' "$HOME/.profile" 2>/dev/null; then
    printf 'export KHOR_HOME="$HOME/.khor-hosts/$(hostname)"\n' >> "$HOME/.profile"
    echo "onset: KHOR_HOME line added to ~/.profile"
fi

KHOR_HOME="$root" "$HOME/.local/bin/khor" id
REMOTE
remote "$STAGE/on.sh"

# ── the resident ─────────────────────────────────────────
if [ "$SERVE" = yes ]; then
    cat > "$STAGE/serve.sh" <<'REMOTE'
set -eu
root=$(cat "$HOME/.local/bin/.khor-root")
pidf="$root/.khor/serve.pid"
# Stopped by **its own pid file**, never by name: `pkill -f khor` on a
# shared machine is somebody else's outage.
if [ -f "$pidf" ] && kill -0 "$(cat "$pidf")" 2>/dev/null; then
    kill "$(cat "$pidf")" 2>/dev/null || true
    sleep 1
fi
# setsid: the resident must outlive this ssh session, and be nobody's
# child so a logout cannot take it along.
KHOR_HOME="$root" setsid nohup "$HOME/.local/bin/khor" serve >> "$root/.khor/serve.log" 2>&1 &
echo $! > "$pidf"
sleep 2
if kill -0 "$(cat "$pidf")" 2>/dev/null; then
    echo "onset: serve is up (pid $(cat "$pidf"), store $root)"
else
    echo "onset: serve died on start — tail of $root/.khor/serve.log:" >&2
    tail -5 "$root/.khor/serve.log" >&2
    exit 1
fi
REMOTE
    say "onset: (re)starting the resident"
    remote "$STAGE/serve.sh"
fi

say "onset: done. next — pair it:"
say "  khor invite                     # on a machine already in the mesh"
say "  ssh $HOST khor pair <ticket>"
