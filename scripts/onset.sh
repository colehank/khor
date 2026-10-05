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
#                    [--release[=vX.Y.Z]] [--bin-on-target=PATH] [--dry-run]
#
# **Three places the binary can come from, one install after all three.**
#
# - Built here with zigbuild (the default, below).
# - `--release[=tag]`: fetched from the GitHub release (latest, or the
#   tag named) — the asset the `release` workflow built, checked against
#   the `.sha256` it published. This machine compiles nothing for Linux
#   then; cutting a release is what puts a build on the fleet, and the
#   Mac's disk stops paying for a target dir it does not run. `KHOR_MIRROR`
#   is honoured the way install.sh honours it. No `--musl` this way: the
#   release matrix has no static leg.
# - `--bin-on-target=PATH`: the binary already sits on the target, under
#   its `~/.local/bin` — put there by `scripts/build-on.sh`, which builds
#   natively on a Linux box whose glibc *is* the floor. Nothing is sent;
#   the file is renamed into the staged name and walks the same gate.
#
# `--dry-run` stops once the binary is ready and says what would be
# sent — the way to look at a release before shipping it anywhere.
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
SOURCE=build
RELEASE=latest
ONTARGET=""
DRY=no
for arg in "$@"; do
    case "$arg" in
        --uninstall) MODE=uninstall ;;
        --purge) PURGE=yes ;;
        --no-serve) SERVE=no ;;
        --musl) LIBC=musl ;;
        --release) SOURCE=release ;;
        --release=*) SOURCE=release; RELEASE="${arg#--release=}" ;;
        --bin-on-target=*) SOURCE=ontarget; ONTARGET="${arg#--bin-on-target=}" ;;
        --dry-run) DRY=yes ;;
        --print-serve-up) MODE=print-serve-up ;;
        -*) echo "unknown option: $arg" >&2; exit 2 ;;
        *) HOST="$arg" ;;
    esac
done
# **`khor-serve-up`, the login pull-up, is shipped too — and it is
# install.sh's own text, cut out of install.sh**, not a copy of it.
#
# onset never used to touch it, and on a shared home it is one file for
# every machine. turing's was install.sh's from 2026-08-21: it compared
# the whole `~/.khor/owner` to the hostname, the owner file grew a
# second line on 9-11, and from then on every login on turing — the
# machine that *owns* `$HOME` — started a resident for
# `.khor-hosts/turing`, a second identity for itself. Measured
# 2026-09-22: that resident had been up since 9-17 beside the real one,
# holding its web port. Every onset run since had upgraded the binary
# and left the script that starts it five weeks behind.
#
# `--print-serve-up` prints what would be sent and stops: the one way
# to test the text without a machine.
REPO=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
serve_up_text() {
    sed -n "/^cat > \"\$UP\" <<'SERVEUP'\$/,/^SERVEUP\$/p" "$REPO/scripts/install.sh" | sed '1d;$d'
}
if [ "$MODE" = print-serve-up ]; then
    serve_up_text
    exit 0
fi
[ -n "$HOST" ] || { echo "usage: onset.sh <ssh-host> [--musl] [--uninstall] [--purge] [--no-serve] [--release[=tag]] [--bin-on-target=PATH] [--dry-run]" >&2; exit 2; }
if [ "$SOURCE" = release ] && [ "$LIBC" = musl ]; then
    echo "onset: the release has no static build (release.yml says why); build --musl here instead" >&2
    exit 2
fi

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
# **Every remote script gets the shared half in front of it**, so
# "which store is mine" and "is this pid really khor" are answered the
# same way whether we are installing, starting or removing (onset-lib.sh
# carries why both questions are dangerous).
LIB="$REPO/scripts/onset-lib.sh"
remote() { cat "$LIB" "$1" | ssh_ "sh -s"; }

# ── uninstall ────────────────────────────────────────────
if [ "$MODE" = uninstall ]; then
    cat > "$STAGE/off.sh" <<'REMOTE'
set -eu
# **This machine's store only.** The loop used to walk both candidate
# roots, and on a shared home `$HOME/.khor/serve.pid` is *another
# machine's* pid file — so removing khor from hinton would read turing's
# number and kill whatever on hinton happens to hold it.
khor_stop_serve "$(khor_root)/.khor/serve.pid" || true
rm -f "$HOME/.local/bin/khor"
# The note that started all this. Deleted rather than updated: nothing
# reads it any more, and a stale pointer left lying about is exactly
# what it was.
rm -f "$HOME/.local/bin/.khor-root"
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

# scp does not expand the remote shell's variables, so the home is
# asked for rather than spelled. Asked once, up here: every source
# below lands the binary under it.
ssh_ 'mkdir -p "$HOME/.local/bin"'
REMOTE_HOME=$(ssh_ 'printf %s "$HOME"' | tr -d '\r')
[ -n "$REMOTE_HOME" ] || { echo "onset: $HOST did not say where its home is" >&2; exit 1; }
STAGED="$REMOTE_HOME/.local/bin/.khor.onset$$"

case "$SOURCE" in
build)
    # ── built here, never there ──────────────────────────────
    # Compiling on the target is the thing this mode exists to avoid:
    # these boxes are shared (one carries 40 logged-in users), their
    # homes are on NFS, and a toolchain install is a gigabyte of
    # somebody else's disk. (`build-on.sh` is the deliberate exception,
    # for a box that already carries a toolchain and *is* the floor.)
    BIN="$REPO/target/$TARGET/release/khor"
    say "onset: building $ZIGTARGET (this machine)"
    ( cd "$REPO" && cargo zigbuild -q -p khor-cli --bin khor --target "$ZIGTARGET" --release )
    [ -f "$BIN" ] || { echo "onset: no binary at $BIN" >&2; exit 1; }
    ;;
release)
    # ── fetched from the release the workflow built ─────────
    case "$TARGET" in
        x86_64-unknown-linux-gnu) ASSET=khor-linux-x86_64 ;;
        aarch64-unknown-linux-gnu) ASSET=khor-linux-aarch64 ;;
        *) echo "onset: the release ships no $TARGET" >&2; exit 1 ;;
    esac
    GH_REPO="${KHOR_REPO:-colehank/khor}"
    if [ -n "${KHOR_MIRROR:-}" ]; then
        # The mirror carries one version, whatever its sync last pulled
        # (install.sh says the same); a tag asked for here is not on it.
        [ "$RELEASE" = latest ] || { echo "onset: a mirror has no tags; drop --release=$RELEASE or KHOR_MIRROR" >&2; exit 2; }
        URL="${KHOR_MIRROR%/}/$ASSET.gz"
    elif [ "$RELEASE" = latest ]; then
        URL="https://github.com/$GH_REPO/releases/latest/download/$ASSET.gz"
    else
        URL="https://github.com/$GH_REPO/releases/download/$RELEASE/$ASSET.gz"
    fi
    say "onset: fetching $ASSET ($RELEASE)"
    # The archive and the digest the workflow wrote beside it, both, and
    # the digest checked before the bytes are believed: a download that
    # came back short or as GitHub's HTML for a tag that is not there
    # must stop here, not on the target as「khor does not run」.
    curl -fsSL "$URL" -o "$STAGE/$ASSET.gz" || { echo "onset: could not fetch $URL" >&2; exit 1; }
    curl -fsSL "$URL.sha256" -o "$STAGE/$ASSET.gz.sha256" || { echo "onset: no digest beside $URL" >&2; exit 1; }
    ( cd "$STAGE" && { command -v sha256sum >/dev/null 2>&1 && sha256sum -c --quiet "$ASSET.gz.sha256" \
        || shasum -a 256 -c --quiet "$ASSET.gz.sha256"; } ) \
        || { echo "onset: $ASSET.gz does not match the digest the release published" >&2; exit 1; }
    BIN="$STAGE/khor"
    gzip -dc "$STAGE/$ASSET.gz" > "$BIN" || { echo "onset: $URL is not a gzip (wrong tag?)" >&2; exit 1; }
    case "$(head -c 4 "$BIN" | od -An -tx1 | tr -d ' \n')" in
        7f454c46) : ;;
        *) echo "onset: $URL did not answer with a program (wrong tag?)" >&2; exit 1 ;;
    esac
    say "onset: $ASSET verified ($(cut -c1-12 "$STAGE/$ASSET.gz.sha256")…)"
    ;;
ontarget)
    # ── already on the target ────────────────────────────────
    # Under its `~/.local/bin`, so the rename below is a `rename(2)` on
    # one filesystem and not a copy across two (the same rule the scp
    # landing follows, for the same NFS reason).
    case "$ONTARGET" in
        "$REMOTE_HOME"/.local/bin/*|'$HOME'/.local/bin/*) : ;;
        *) echo "onset: --bin-on-target must name a file under $REMOTE_HOME/.local/bin (got $ONTARGET)" >&2; exit 1 ;;
    esac
    ONTARGET=$(printf '%s' "$ONTARGET" | sed "s|^\$HOME|$REMOTE_HOME|")
    BIN=""
    ;;
esac

if [ -n "$BIN" ]; then
    # The one thing worth checking about the artefact is that it is the
    # *asked-for* one: a stale file from the other libc under a target
    # dir that looks right is exactly the kind of thing that installs
    # fine and then behaves like the build nobody chose.
    case "$LIBC-$(file "$BIN")" in
        musl-*statically\ linked*) : ;;
        gnu-*dynamically\ linked*) : ;;
        *) echo "onset: $BIN is not a $LIBC build" >&2; exit 1 ;;
    esac
    if [ "$DRY" = yes ]; then
        say "onset: dry run — $BIN is ready ($(wc -c < "$BIN" | tr -d ' ') bytes, $(shasum -a 256 "$BIN" | cut -c1-12)…); nothing sent to $HOST"
        exit 0
    fi
    say "onset: sending $(wc -c < "$BIN" | tr -d ' ') bytes"
    # **Landed next to its destination, not in /tmp.** `mv` is only
    # atomic within one filesystem; across two it degrades to
    # copy-then-unlink, which can rewrite the bytes of a binary some
    # other process is running — and on a cluster that other process is
    # a different machine's resident serve, reading the same NFS file.
    # Same directory, real `rename(2)`, old inode left alone for whoever
    # is still running it.
    scp_ "$BIN" "$STAGED"
else
    if [ "$DRY" = yes ]; then
        say "onset: dry run — would install $ONTARGET on $HOST; nothing touched"
        exit 0
    fi
    say "onset: taking $ONTARGET as the staged binary"
    ssh_ "mv -f '$ONTARGET' '$STAGED'"
fi
# The pull-up script travels beside the binary and lands the same way.
STAGED_UP="$REMOTE_HOME/.local/bin/.khor-serve-up.onset$$"
serve_up_text > "$STAGE/khor-serve-up"
[ -s "$STAGE/khor-serve-up" ] || { echo "onset: could not cut khor-serve-up out of scripts/install.sh" >&2; exit 1; }
scp_ "$STAGE/khor-serve-up" "$STAGED_UP"

# ── install ──────────────────────────────────────────────
# The store's root is decided **on the target**, because only it knows
# whether its home is its own: a cluster mounts one NFS export on every
# machine, so `~/.khor` is literally the same directory on all of them
# (measured on this fleet). khor refuses to open another machine's store
# — this puts one of its own in front of that refusal.
cat > "$STAGE/on.sh" <<REMOTE
set -eu
src="$STAGED"
up="$STAGED_UP"
REMOTE
cat >> "$STAGE/on.sh" <<'REMOTE'
mkdir -p "$HOME/.local/bin"
chmod +x "$src"

# ⚠️ **Asked of the staged file, before anything is replaced.**
#
# The gate itself is the keeper's (`keeper.rs`: a new generation must
# answer `version` before it is allowed to serve — a 0-byte file wearing
# the binary's name took two machines down on 2026-08-19). An install
# has the same question and did not ask it: the binary was moved into
# place, the resident was started, `kill -0` found a pid for a moment,
# and the script said 「serve is up」.
#
# **And the order matters more than the check.** On a shared home
# `~/.local/bin/khor` is *one file for every machine on the mount* — so
# an install onto a box that cannot run this build used to overwrite the
# binary three healthy machines were running, and only then discover it
# does not work. Measured 2026-09-06 on fodor (Ubuntu 18.04, glibc
# 2.27, against a glibc 2.31 build): four `GLIBC_2.2x not found` lines,
# and by then the file was already swapped.
#
# `if !` because `set -e` would take the shell down before the message
# could be printed, and a bare exit code is the thing being replaced.
if ! ver=$("$src" version 2>"$src.err"); then
    echo "onset: khor does not run on $(hostname) — it never got as far as saying its version." >&2
    echo "onset: what the machine said:" >&2
    sed 's/^/onset:   /' "$src.err" >&2
    echo "onset: nothing was replaced and nothing was started." >&2
    rm -f "$src" "$src.err" "$up"
    exit 1
fi
rm -f "$src.err"
echo "onset: khor $ver runs here"

# rename, not copy: upgrading over a *running* khor works because Linux
# keeps the open inode, and the next start picks up the new file.
mv -f "$src" "$HOME/.local/bin/khor"
# And the script every login runs to pull the resident up — install.sh's
# current text, so it asks the binary just installed which store is
# this machine's instead of working it out the way it did in August.
chmod +x "$up"
mv -f "$up" "$HOME/.local/bin/khor-serve-up"

# **The third copy of this rule, and the one that cost a store.**
# Until 2026-09-11 this block compared `$HOME/.khor/owner` to the
# hostname itself. The owner file grew a `machine <fingerprint>` line
# that day, the comparison stopped matching, and a machine that owns its
# home was told it did not: it made `.khor-hosts/turing`, ran `khor id`
# there, and minted a **second identity for a machine that already had
# one** — the exact failure the fingerprint was added to prevent, walked
# into through the back door by a script.
#
# So it asks `khor_root`, which asks the binary that was just installed.
# One rule, one implementation, and the two moments with no khor to ask
# are handled there rather than here.
root=$(khor_root)
[ "$root" = "$HOME" ] || echo "onset: this home is somebody else's — $(hostname) uses $root"
mkdir -p "$root/.khor"
# **No pointer file.** It lived at `~/.local/bin/.khor-root`, which on a
# cluster with one shared home is one file for every machine — last
# writer wins, and everyone else reads a path that is not theirs. The
# root is asked for instead (`khor_root`).
rm -f "$HOME/.local/bin/.khor-root"

# The PATH line. Written once: appending on every run is how a .profile
# ends up with twelve copies.
if ! grep -qs 'HOME/.local/bin' "$HOME/.profile" 2>/dev/null; then
    printf '%s\n' 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.profile"
    echo "onset: PATH line added to ~/.profile"
fi
# **No KHOR_HOME line.** There used to be one here, written by whichever
# machine found the home already claimed — and on a shared home
# `~/.profile` is one file for the whole fleet, so the owner read it too
# and got sent to a store that was not its own. See
# `khor_drop_profile_line`, which takes an old one back out.
khor_drop_profile_line "$HOME/.profile"

# **No KHOR_HOME on the command line either** (round NET-11): khor works
# out its own room (`Node::store_under`), and the room `khor_root` printed
# above is that same answer. An explicit one is a second way to name the
# store, which is how turing came to run two serve trees on one room:
# this script's, with KHOR_HOME set, and the login pull-up's, without.
env -u KHOR_HOME "$HOME/.local/bin/khor" id
REMOTE
remote "$STAGE/on.sh"

# ── the resident ─────────────────────────────────────────
if [ "$SERVE" = yes ]; then
    cat > "$STAGE/serve.sh" <<'REMOTE'
set -eu
root=$(khor_root)
pidf="$root/.khor/serve.pid"
# Stopped by **its own pid file**, never by name: `pkill -f khor` on a
# shared machine is somebody else's outage. And never on the file's word
# alone — `khor_stop_serve` checks the pid really is khor's resident
# before killing, because a number in a file is a claim about the past.
if ! khor_stop_serve "$pidf"; then
    echo "onset: refusing to start on top of that — check $pidf by hand" >&2
    exit 1
fi
# setsid: the resident must outlive this ssh session, and be nobody's
# child so a logout cannot take it along.
# **Where the log ends before this start**, so what is read back
# afterwards is this run's own words and not last week's. A `tail -5`
# of a shared log is how an old error gets reported as a new one, and
# how a new one gets scrolled off by a healthy start.
was=0
[ -f "$root/.khor/serve.log" ] && was=$(wc -c < "$root/.khor/serve.log")
# Without KHOR_HOME, as the login pull-up starts it: one way to name the
# room, so a second start is seen as a second serve on the same store and
# stands down (`serves::beside`).
env -u KHOR_HOME setsid nohup "$HOME/.local/bin/khor" serve >> "$root/.khor/serve.log" 2>&1 &
echo $! > "$pidf"
# **Alive twice, seconds apart, not once at two seconds.** A serve that
# dies at load is a pid for a moment — long enough for one `kill -0` to
# find it, which is the whole reason this check was worth nothing.
alive=no
i=0
while [ "$i" -lt 8 ]; do
    sleep 1
    i=$((i + 1))
    if kill -0 "$(cat "$pidf")" 2>/dev/null; then
        [ "$i" -ge 4 ] && alive=yes && break
    else
        alive=no
        break
    fi
done
said=$(tail -c "+$((was + 1))" "$root/.khor/serve.log" 2>/dev/null || true)
if [ "$alive" = yes ]; then
    echo "onset: serve is up (pid $(cat "$pidf"), store $root)"
    [ -n "$said" ] && printf '%s\n' "$said" | sed 's/^/onset:   /'
else
    echo "onset: serve did not stay up on $(hostname). What it wrote this time:" >&2
    if [ -n "$said" ]; then
        printf '%s\n' "$said" | sed 's/^/onset:   /' >&2
    else
        echo "onset:   (nothing — it did not get far enough to write a line)" >&2
    fi
    exit 1
fi
REMOTE
    say "onset: (re)starting the resident"
    remote "$STAGE/serve.sh"
fi

say "onset: done. next — pair it:"
say "  khor invite                     # on a machine already in the mesh"
say "  ssh $HOST khor pair <ticket>"
