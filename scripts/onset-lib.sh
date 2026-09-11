# The remote half's two dangerous questions, answered in one place.
#
# `onset.sh` pastes this in front of every script it runs on the target,
# so the answers cannot drift between installing, starting and removing.
# POSIX sh: the targets are whatever `/bin/sh` is over there.

# **Which store on this machine is khor's.**
#
# Recomputed from what khor itself records, never read back from a note
# left on disk. The note was `~/.local/bin/.khor-root`, and on a cluster
# that mounts one home on every machine it is **one file for the whole
# fleet**: turing writes its root, hinton overwrites it with hinton's,
# and afterwards turing reads hinton's path out of its own home. Found
# 2026-09-04 on exactly that pair — the file said
# `…/.khor-hosts/hinton` while turing's own store is `$HOME`.
#
# A single install run never noticed, because it writes the note and
# reads it back seconds later. Everything that reads it *later* — the
# next `--uninstall`, the next restart — got the other machine's answer.
#
# The rule reproduced here is khor's own, not a third scheme: the shared
# home belongs to whichever machine claimed it (`$HOME/.khor/owner`),
# and every other machine gets `$HOME/.khor-hosts/<hostname>`.
# **The installed khor is asked first, and it is the authority.**
#
# This used to reproduce khor's rule in sh, and the day the `owner` file
# grew a second line (`machine <fingerprint>` above `name <hostname>`)
# the sh half read the whole file as a hostname, concluded the home
# belonged to somebody else, and sent onset to a directory nobody had
# made. Measured 2026-09-11 on turing: `cannot create
# .khor-hosts/turing/.khor/serve.pid: Directory nonexistent`, and the
# resident stayed down until it was noticed.
#
# **A rule written twice is a rule that will disagree with itself.** The
# fallback below is still the old comparison, because there are two
# moments with no khor to ask — a first install, and an uninstall after
# the binary is gone — and it is written to survive the new format by
# reading the `name` line when there is one.
khor_root() {
    if [ -x "$HOME/.local/bin/khor" ]; then
        _root=$("$HOME/.local/bin/khor" root 2>/dev/null) || _root=""
        if [ -n "$_root" ]; then
            printf '%s\n' "$_root"
            return 0
        fi
    fi
    _owner=""
    if [ -f "$HOME/.khor/owner" ]; then
        # The two-line format, then the bare hostname every store
        # written before 2026-09-11 is in.
        _owner=$(sed -n 's/^name[[:space:]]\{1,\}//p' "$HOME/.khor/owner" 2>/dev/null | head -1)
        [ -n "$_owner" ] || _owner=$(head -1 "$HOME/.khor/owner" 2>/dev/null)
        case "$_owner" in
        machine\ *) _owner="" ;;
        esac
    fi
    if [ -n "$_owner" ] && [ "$_owner" != "$(hostname)" ]; then
        printf '%s\n' "$HOME/.khor-hosts/$(hostname)"
    else
        printf '%s\n' "$HOME"
    fi
}

# **Is this pid actually khor's resident, right now?**
#
# A pid read out of a file is a claim about the past. Pids get reused,
# files go stale, and on a shared home the file may have been written by
# a different machine entirely — so the number can name a stranger's
# process that happens to be running here. `kill` does not ask.
#
# So nothing is killed on the strength of the file alone: the number is
# checked against the process table first, and a pid whose command line
# is not khor's is left alone and reported. **This is the half that
# matters even after the note is gone** — every remaining way for the
# pid to be wrong (reuse, a crash between write and start, a hand-edited
# file) ends in the same place.
khor_is_serve() {
    [ -n "$1" ] || return 1
    _cmd=$(ps -o args= -p "$1" 2>/dev/null) || return 1
    [ -n "$_cmd" ] || return 1
    # The resident runs from a copy the keeper made, so the path varies
    # (`/tmp/khor-shield-…/khor-<hash> serve`); what is stable is a
    # `khor` in the program and `serve` as the verb.
    case "$_cmd" in
    *khor*" serve") return 0 ;;
    *khor*" serve "*) return 0 ;;
    *) return 1 ;;
    esac
}

# Stops the resident named by `$1` (a pid file), and says what it did.
# Never kills what it did not recognise.
khor_stop_serve() {
    _pidf="$1"
    [ -f "$_pidf" ] || return 0
    _pid=$(cat "$_pidf" 2>/dev/null)
    if [ -z "$_pid" ]; then
        rm -f "$_pidf"
        return 0
    fi
    if ! kill -0 "$_pid" 2>/dev/null; then
        # Nobody home: the file outlived the process it named.
        rm -f "$_pidf"
        return 0
    fi
    if khor_is_serve "$_pid"; then
        kill "$_pid" 2>/dev/null || true
        sleep 1
        rm -f "$_pidf"
        return 0
    fi
    echo "onset: $_pidf says pid $_pid, but that pid is not khor serve — leaving it alone" >&2
    echo "onset:   it is: $(ps -o args= -p "$_pid" 2>/dev/null | head -1)" >&2
    return 1
}

# **Takes back the `KHOR_HOME` line the installer used to append.**
#
# The line was `export KHOR_HOME="$HOME/.khor-hosts/$(hostname)"`, and
# it was written only by a machine that had found the home already
# claimed — but `~/.profile` on a shared home is **one file for the
# whole fleet**, so the owner read it too and was sent to a store that
# was not its own. Measured 2026-09-11: `khor devices` in a turing shell
# listed one machine under a second identity; the same command with
# `KHOR_HOME=$HOME` listed four. khor works the rule out for itself now
# (`Node::root_from_env`), and a stale line would override it.
#
# **Only that exact line.** A `KHOR_HOME` somebody set by hand is a
# deliberate override — dual instances on one machine ride on it — and
# removing it would be this function doing the very thing it exists to
# undo.
khor_drop_profile_line() {
    _prof="$1"
    [ -f "$_prof" ] || return 0
    _line='export KHOR_HOME="$HOME/.khor-hosts/$(hostname)"'
    grep -qxF "$_line" "$_prof" 2>/dev/null || return 0
    _tmp="$_prof.khor-$$"
    grep -vxF "$_line" "$_prof" > "$_tmp" 2>/dev/null || {
        rm -f "$_tmp"
        return 1
    }
    mv -f "$_tmp" "$_prof" && echo "onset: took the stale KHOR_HOME line back out of $_prof"
}
