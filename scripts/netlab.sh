#!/bin/sh
# Run the network lab (docs/NETLAB.md): every scenario in
# `crates/cli/tests/netlab.rs`, on a Linux box, and print the results.
#
#   scripts/netlab.sh [--on <ssh-host>] [<test-name-filter>...]
#
# Off Linux it runs on `--on` (default turing): the working tree is
# rsynced to that box's local disk and this script runs there. The lab
# needs Linux network namespaces, entered unprivileged through a user
# namespace; nothing here asks for root.
#
# On the box, everything is the lab's own: source and target under
# /tmp/khor-netlab-*, never the NFS home and never the fleet's
# /tmp/khor-src-* or /tmp/khor-target-*; no install, no `khor` of the
# person's touched. `nft` and `conntrack` come from the distribution's
# packages unpacked into $NETLAB_TOOLS (apt-get download, no root) when
# the box has none.
#
# Output: the `[netlab-result]` lines, as a table, and the full log
# beside it (`/tmp/khor-netlab-run/<time>.log` on the box).
set -eu

HOST=turing
while [ $# -gt 0 ]; do
    case $1 in
        --on) HOST=$2; shift 2 ;;
        *) break ;;
    esac
done

if [ "$(uname -s)" != Linux ]; then
    REPO=$(cd "$(dirname "$0")/.." && pwd)
    rsync -a --delete --exclude /target --exclude node_modules --exclude .git \
        --exclude /apps/gui/dist "$REPO/" "$HOST:/tmp/khor-netlab-src/"
    exec ssh "$HOST" "sh /tmp/khor-netlab-src/scripts/netlab.sh $*"
fi

SRC=$(cd "$(dirname "$0")/.." && pwd)
TOOLS=${NETLAB_TOOLS:-$HOME/khor-netlab/tools}
export PATH="$TOOLS/bin:$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-/tmp/khor-netlab-target/khor}

# nft and conntrack, unpacked from the distribution's packages.
if ! command -v nft >/dev/null 2>&1; then
    command -v apt-get >/dev/null || { echo "netlab: no nft and no apt-get to fetch it" >&2; exit 1; }
    mkdir -p "$TOOLS/debs" "$TOOLS/bin"
    (cd "$TOOLS/debs" && apt-get download nftables libnftables1 conntrack >/dev/null)
    for d in "$TOOLS"/debs/*.deb; do dpkg -x "$d" "$TOOLS/root"; done
    for b in nft conntrack; do
        cat >"$TOOLS/bin/$b" <<EOF
#!/bin/sh
R=$TOOLS/root
LD_LIBRARY_PATH=\$R/usr/lib/x86_64-linux-gnu\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH} exec \$R/usr/sbin/$b "\$@"
EOF
        chmod +x "$TOOLS/bin/$b"
    done
fi
command -v tc >/dev/null || [ -x /usr/sbin/tc ] || { echo "netlab: no tc (iproute2)" >&2; exit 1; }
export PATH="$PATH:/usr/sbin:/sbin"

RUN=/tmp/khor-netlab-run
mkdir -p "$RUN"
LOG="$RUN/$(date +%Y%m%d-%H%M%S).log"
cd "$SRC"
# khor-web's build wants the face's index.html; the lab never serves the
# face, so a scratch copy without one gets a placeholder (the rsync
# above leaves apps/gui/dist alone).
if [ ! -f apps/gui/dist/index.html ]; then
    mkdir -p apps/gui/dist
    echo '<!doctype html><title>netlab: no face</title>' >apps/gui/dist/index.html
fi
echo "netlab: building on $(hostname) (target $CARGO_TARGET_DIR)"
nice -n 10 cargo test -p khor-cli --test netlab --no-run 2>&1 | grep -E '^(error|warning: unused)|Finished' || true
echo "netlab: running (log $LOG)"
# Scenarios run side by side: each lab is namespaces of its own.
# --nocapture: a green scenario's numbers are the point too.
nice -n 10 cargo test -p khor-cli --test netlab -- --test-threads 8 --nocapture "$@" >"$LOG" 2>&1 || true

echo
echo "| scenario | today | measured |"
echo "|---|---|---|"
grep '^\[netlab-result\]' "$LOG" | sed 's/^\[netlab-result\] /| /; s/$/ |/' | sort
echo
grep -E '^test result|^test .* (FAILED|ok)$' "$LOG" || true
