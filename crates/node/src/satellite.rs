//! Satellites: machines only ssh can reach, spoken for by a sponsor
//! (附属机器批; docs/NET.md 卫星).
//!
//! 卫星 = 看得见、够得着,但不是它自己在说话;行星 = 它自己在说话;
//! 转正 = 一条命令. A satellite sits in the device table like any
//! machine — id, name, a face — but its home is its **sponsor**: every
//! operation routes there and the sponsor executes it over ssh.
//!
//! # Only the user's own ssh road, never khor's
//!
//! Everything here shells out to `ssh` so the user's `~/.ssh/config`,
//! agent and known_hosts do all the deciding. khor stores no
//! credential, opens no socket of its own, and never calls
//! `ssh-keyscan`: the identity comes out of **known_hosts** — the trust
//! the user already extended — and a host they never connected to is
//! refused with directions, not probed behind their back.
//!
//! # The id is the host key's SHA256, as hex
//!
//! A SHA256 is exactly 32 bytes, so the fingerprint wears the same
//! shape as every device id (64 hex chars): `DeviceId` parses it, the
//! avatar mint seeds from it (a satellite has a face for free), and no
//! reader needs a second id grammar. It is not an iroh key — nothing
//! can dial it — and the row's empty `addrs` keeps every sync loop
//! walking past it, which is exactly right.
//!
//! # BatchMode everywhere
//!
//! Every ssh this module runs carries `-o BatchMode=yes`: a password
//! prompt inside a serve is a hang nobody can answer. A host that needs
//! interactive auth fails fast with ssh's own words, which the caller
//! wraps in directions.

use std::path::PathBuf;
use std::process::Command;

use khor_catalog::msg;

/// The test door (`KHOR_TMUX_SOCKET`'s precedent): a scratch ssh world
/// — a `config` and a `known_hosts` under one directory — so
/// verification never reads or writes the user's real `~/.ssh`. It has
/// to be an explicit door rather than a `HOME` override because OpenSSH
/// resolves `~` through the passwd database, not the environment
/// (measured: a HOME-overridden probe read the real config and sent the
/// alias to a proxy's fake-ip DNS). Production leaves it unset and
/// every call rides the user's own road, which is the whole point.
fn ssh_dir() -> Option<PathBuf> {
    std::env::var_os("KHOR_SSH_DIR").map(Into::into)
}

/// What one adoption probe learned over a single connection.
#[derive(Debug, Clone)]
pub struct Probe {
    /// `uname -s` (or "unknown" where uname is missing) — recorded for
    /// the report, not for branching: platform quirks are handled where
    /// they bite (the sampler), never pre-sorted here.
    pub uname: String,
    /// Whether tmux is on the satellite — what decides if a terminal
    /// there can be made persistent (docs/SESSION.md 例外墓碑).
    pub tmux: bool,
}

/// The flags every satellite ssh shares. `ConnectTimeout` because a
/// dead host must cost seconds, not a TCP stack's patience.
fn ssh_base(host: &str) -> Command {
    let mut c = Command::new("ssh");
    if let Some(dir) = ssh_dir() {
        c.arg("-F").arg(dir.join("config"));
    }
    c.arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=8")
        .arg(host);
    c
}

/// One connection, two answers: what it is, and whether tmux is there.
pub fn probe(host: &str) -> Result<Probe, String> {
    let out = ssh_base(host)
        .arg("uname -s 2>/dev/null || echo unknown; command -v tmux >/dev/null 2>&1 && echo tmux-yes || echo tmux-no")
        .output()
        .map_err(|e| msg::ssh_wont_run(e))?;
    if !out.status.success() {
        return Err(msg::sat_cant_reach(host, String::from_utf8_lossy(&out.stderr).trim()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let uname = lines.next().unwrap_or("unknown").trim().to_owned();
    let tmux = lines.next().is_some_and(|l| l.trim() == "tmux-yes");
    Ok(Probe { uname, tmux })
}

/// How often a sponsor samples each of its satellites. A minute, not
/// the 5s sync beat: every sample is an ssh connection and a one-second
/// on-host measurement, and a reading of a machine nobody khor runs on
/// does not need a heartbeat's cadence — the age rides the reading
/// anyway (docs/SESSION.md 离线: how fresh is its own axis).
pub const SAMPLE_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// One shell, both platforms, key=value lines out. The one-second sleep
/// inside is the CPU delta window (a utilisation is a rate; a single
/// glance at /proc/stat is a lifetime average). Every section tolerates
/// its own absence — a missing nvidia-smi prints nothing and the parser
/// reads nothing, the disk falling over leaves the reading without a
/// disk, never without a row.
const SAMPLE_SH: &str = r#"
U=$(uname -s)
echo cores=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)
case "$U" in
Linux)
  c1=$(head -1 /proc/stat); sleep 1; c2=$(head -1 /proc/stat)
  echo "$c1
$c2" | awk 'NR==1{for(i=2;i<=NF;i++)t1+=$i; i1=$5} NR==2{for(i=2;i<=NF;i++)t2+=$i; i2=$5; dt=t2-t1; if(dt>0) printf "cpu_pct=%.1f\n", (dt-(i2-i1))*100/dt}'
  awk '/MemTotal/{t=$2} /MemAvailable/{a=$2} END{printf "mem_total=%d\nmem_used=%d\n", t*1024, (t-a)*1024}' /proc/meminfo
  ;;
Darwin)
  top -l 2 -n 0 -s 1 2>/dev/null | awk '/CPU usage/{u=$3; s=$5} END{gsub("%","",u); gsub("%","",s); printf "cpu_pct=%.1f\n", u+s}'
  T=$(sysctl -n hw.memsize)
  P=$(sysctl -n hw.pagesize)
  vm_stat | awk -v t="$T" -v p="$P" '/Pages active/{a=$3} /Pages wired/{w=$4} /occupied by compressor/{c=$5} END{gsub("\\.","",a); gsub("\\.","",w); gsub("\\.","",c); printf "mem_total=%d\nmem_used=%d\n", t, (a+w+c)*p}'
  ;;
esac
df -k "$HOME" 2>/dev/null | awk 'NR==2{printf "disk_total=%d\ndisk_used=%d\n", ($3+$4)*1024, $3*1024}'
if command -v nvidia-smi >/dev/null 2>&1; then
  nvidia-smi --query-gpu=utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits 2>/dev/null | awk -F', *' '{n++; u+=$1; mu+=$2; mt+=$3} END{if(n>0) printf "gpu_pct=%.1f\ngpu_cards=%d\nvram_used=%d\nvram_total=%d\n", u/n, n, mu*1048576, mt*1048576}'
  nvidia-smi --query-gpu=utilization.gpu,memory.used,memory.total,name --format=csv,noheader,nounits 2>/dev/null | awk -F', *' '{printf "gpu_card=%s|%d|%d|%s\n", $1, $2*1048576, $3*1048576, $4}'
fi
df -kP 2>/dev/null | awk 'NR>1 && $1 ~ /^\// {printf "vol=%s|%d|%d\n", $6, $3*1024, ($3+$4)*1024}'
"#;

/// The satellite's own recorded working directories, rooted and
/// checked over there.
///
/// **One shell, and it has to be, because every question is about that
/// disk.** Which directory a transcript was written in, whether a
/// `.git` sits above it, whether the root is still there — a sponsor
/// can answer none of them from its own filesystem, and a sponsor that
/// tried would answer about *itself* and label the rows with the
/// satellite's name. So the rooting happens on the satellite and only
/// the merge happens here (`crate::place::merge`), which keeps one
/// definition of 工作区 across machines that reach khor and machines
/// that do not.
///
/// **The two vendors' record shapes are read the way the two adaptors
/// read them** (`adaptor::claude::Claude::places`,
/// `adaptor::codex::Codex::places`): claude files one directory per
/// project and the transcripts inside carry `cwd`; codex files rollouts
/// flat and each carries its own. Both are JSONL, so both are read by
/// pulling the first `"cwd"` out of a file — a real parser is not
/// available over there and is not needed: the field's value is a path,
/// and a path containing a `"` would have broken the vendors' own files
/// first.
///
/// Every section tolerates its own absence, the sampler's rule: a
/// satellite with no claude prints nothing for it and the parser reads
/// nothing, never a row of zeros.
///
/// **300 rollouts is `adaptor::codex::PLACES_SCAN`, not a number
/// chosen here.** The cap exists because codex files rollouts flat and
/// a long-lived home holds thousands; a different cap on this road
/// would make the same machine list different projects depending on
/// whether khor was running on it.
///
/// **What this road cannot see is khor's own live sessions**, which
/// the local list folds in (`Node::places`). A satellite's sessions are
/// hosted on its sponsor over ssh and their recorded place is the
/// sponsor's, so there is nothing here to fold in — the count is what
/// the vendors' files remember, which is what the row already says it
/// is (`workspaces::WorkspaceRow::sessions`).
const PLACES_SH: &str = r#"
emit() {
  cwd=$1; n=$2; at=$3
  [ -d "$cwd" ] || return 0
  root=$cwd; co=0; d=$cwd
  while [ "$d" != "/" ] && [ -n "$d" ]; do
    if [ "$d" != "$HOME" ] && [ -e "$d/.git" ]; then root=$d; co=1; break; fi
    p=$(dirname "$d"); [ "$p" = "$d" ] && break; d=$p
  done
  printf 'place=%s|%s|%s|%s\n' "$root" "$n" "$at" "$co"
}
printf 'home=%s\n' "$HOME"
mtime() { stat -c %Y "$1" 2>/dev/null || stat -f %m "$1" 2>/dev/null || echo 0; }
cwd_of() { sed -n 's/.*"cwd"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$1" 2>/dev/null | head -1; }
for d in "$HOME"/.claude/projects/*/; do
  [ -d "$d" ] || continue
  set -- "$d"*.jsonl
  [ -e "$1" ] || continue
  n=$#
  newest=$(ls -t "$d"*.jsonl 2>/dev/null | head -3)
  at=0; cwd=
  for f in $newest; do
    [ "$at" = 0 ] && at=$(mtime "$f")
    [ -z "$cwd" ] && cwd=$(cwd_of "$f")
  done
  [ -n "$cwd" ] && emit "$cwd" "$n" "$at"
done
for f in $(ls -t "$HOME"/.codex/sessions/*.jsonl "$HOME"/.codex/sessions/*/*.jsonl "$HOME"/.codex/sessions/*/*/*.jsonl "$HOME"/.codex/sessions/*/*/*/*.jsonl 2>/dev/null | head -300); do
  cwd=$(cwd_of "$f")
  [ -n "$cwd" ] && emit "$cwd" 1 "$(mtime "$f")"
done
# **The script's own status is not a verdict.** It is whatever its last
# line returned — an empty codex directory leaves a `[ -n "$cwd" ] &&`
# that answered no, and the whole read would have been reported as「ssh
# 够不着」with an empty reason (2026-09-13). A connection that really
# failed is ssh's own 255, which this cannot overwrite.
exit 0
"#;

/// Runs a script on the satellite **as `sh`, reading it from stdin**.
///
/// **`ssh host "<script>"` is not run by `sh`.** It is handed to the
/// far user's *login* shell, and on a mac that is zsh — which, unlike
/// sh, treats a glob that matches nothing as an error and aborts the
/// line (`no matches found: …/*.jsonl`). The first real run of
/// [`PLACES_SH`] died on exactly that, against a satellite whose home
/// held a claude project directory with no transcripts left in it
/// (2026-09-13). Every shell idiom in these scripts is POSIX sh, so the
/// fix is to say so rather than to write around one shell at a time.
///
/// Piped rather than quoted for the same reason quoting is the thing
/// that breaks: a script travelling as an argument is re-split by
/// whatever shell receives it, and a script travelling on stdin is
/// bytes.
fn sh_over_ssh(host: &str, script: &str) -> Result<String, String> {
    use std::io::Write;
    let mut child = ssh_base(host)
        .arg("sh")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(msg::ssh_wont_run)?;
    child
        .stdin
        .take()
        .ok_or_else(|| msg::sat_cant_reach(host, "no stdin"))?
        .write_all(script.as_bytes())
        .map_err(|e| msg::sat_cant_reach(host, e.to_string()))?;
    let out = child.wait_with_output().map_err(msg::ssh_wont_run)?;
    if !out.status.success() {
        return Err(msg::sat_cant_reach(host, String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// [`PLACES_SH`] over the user's ssh road, in the wire shape of
/// [`crate::proto::Response::Workspaces`]: `(path, shown, sessions,
/// last_ms)`, checkouts only.
///
/// Filtered here rather than by the caller because the filter and the
/// shortening are the same two facts the local list applies
/// (`Node::workspaces_shown`), and the shortening needs the home this
/// road is the only one that can read.
pub fn places(host: &str) -> Result<Vec<(String, String, u32, i64)>, String> {
    Ok(parse_places(&sh_over_ssh(host, PLACES_SH)?))
}

/// `place=` lines into places. **Split from the ssh for
/// [`parse_vitals`]'s reason**: recorded output can be fed to it, and a
/// satellite is not needed to prove that a truncated answer yields
/// fewer rows rather than a wrong one.
///
/// A line this cannot read is dropped rather than guessed at. The
/// fields are positional and the path comes first, so a path containing
/// the separator would shift the numbers — which is why it does not:
/// `rsplitn` reads the three fixed fields off the **end**, and
/// everything before them is the path, separators and all.
fn parse_places(text: &str) -> Vec<(String, String, u32, i64)> {
    let home = text.lines().find_map(|l| l.strip_prefix("home=")).unwrap_or("").trim();
    crate::place::merge(text.lines().filter_map(|line| {
        let body = line.strip_prefix("place=")?;
        let mut back = body.rsplitn(4, '|');
        let checkout = back.next()? == "1";
        let last_ms = back.next()?.trim().parse::<i64>().ok()? * 1000;
        let sessions = back.next()?.trim().parse::<u32>().ok()?;
        let path = back.next()?;
        if path.is_empty() {
            return None;
        }
        Some((path.to_owned(), sessions, last_ms, checkout))
    }))
    .into_iter()
    .filter(|p| p.checkout)
    .map(|p| {
        let shown = crate::place::shorten_under(&p.path, home);
        (p.path, shown, p.sessions, p.last_ms)
    })
    .collect()
}

/// `git status`, read on a satellite.
///
/// **The one place khor runs a git binary, and it is not a change of
/// mind.** `crate::git` reads repositories in-process precisely so the
/// user side installs nothing — and that argument is about khor's own
/// machines. A satellite runs no khor at all: there is no process over
/// there to link gix into, and the only thing that can read that disk
/// is a command. A satellite without git says so in git's own words
/// (「command not found」), which is a fact with a fix, unlike a
/// silence.
///
/// `--porcelain=v1 -z` because it is the shape git promises not to
/// change and the one that survives a newline in a path.
///
/// **No `-uall`**, and the first spelling had it. An untracked
/// directory is one row in the in-process reader — gix's walk collapses
/// it — and `-uall` made the same repository answer five rows over ssh
/// where it answered one locally, so a tree read through a satellite
/// marked five files where khor's own reader marked their directory
/// (found by running both against this checkout, 2026-09-13). git's
/// default collapses the same way; what it adds is a trailing `/` on
/// the directory, which this side takes back off.
const STATUS_SH: &str = r#"
cd "$KHOR_ROOT" || exit 3
b=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)
[ "$b" = HEAD ] && b=
printf 'branch=%s\n' "$b"
git status --porcelain=v1 -z 2>&1 | tr '\0' '\n' | sed 's/^/row=/'
exit 0
"#;

/// [`STATUS_SH`] over ssh, in [`crate::git::Status`]'s shape.
pub fn git_status(host: &str, root: &str) -> Result<crate::git::Status, String> {
    let text = sh_over_ssh_in(host, root, STATUS_SH)?;
    parse_status(&text)
}

/// `row=XY path` lines into marks, **by the same rule the in-process
/// reader uses**.
///
/// The two columns are the two comparisons `crate::git::status` folds:
/// the left is index against HEAD (gix's `Item::TreeIndex`), the right
/// is worktree against index (`Item::IndexWorktree`). So each column
/// becomes a mark on its own and the pair is folded with
/// [`crate::git::fold`] — not re-decided here, because a second
/// priority order would make one file wear two different letters
/// depending on which machine it is on.
fn parse_status(text: &str) -> Result<crate::git::Status, String> {
    let mut branch = String::new();
    let mut marks: std::collections::BTreeMap<String, &'static str> =
        std::collections::BTreeMap::new();
    let mut more = false;
    // **A rename is two fields, not two rows.** `-z` writes
    // `R<space><space>new\0old\0`, so the name it came from arrives
    // looking exactly like another row — and read as one it is a file
    // with the last of its letters bitten off wearing whatever its
    // first two characters happen to mean. `ren.txt` became a modified
    // `n.txt` that no repository has ever contained (found by running
    // both readers over one repo built to hold every shape,
    // 2026-09-13). The order is git's own and fixed: the field after a
    // rename is its old name.
    let mut skip_next = false;
    for line in text.lines() {
        if let Some(b) = line.strip_prefix("branch=") {
            branch = b.trim().to_owned();
            continue;
        }
        let Some(row) = line.strip_prefix("row=") else { continue };
        if std::mem::take(&mut skip_next) {
            continue;
        }
        if row.len() < 4 {
            continue;
        }
        let (flags, path) = row.split_at(2);
        // git marks an untracked *directory* with a trailing slash and
        // the in-process reader does not; the tree folds marks by path,
        // so one spelling has to win and it is khor's.
        let path = path.trim_start_matches(' ').trim_end_matches('/');
        if path.is_empty() {
            continue;
        }
        let mut flags = flags.chars();
        let (x, y) = (flags.next().unwrap_or(' '), flags.next().unwrap_or(' '));
        skip_next = x == 'R' || x == 'C' || y == 'R' || y == 'C';
        let Some(mark) = row_mark(x, y) else { continue };
        if marks.len() >= crate::git::ROWS && !marks.contains_key(path) {
            more = true;
            break;
        }
        let slot = marks.entry(path.to_owned()).or_insert(mark);
        *slot = crate::git::fold(*slot, mark);
    }
    let rows = marks.into_iter().map(|(p, m)| (p, m.to_owned())).collect();
    Ok(crate::git::Status { branch, rows, more })
}

/// One porcelain row's two columns as one of khor's four letters.
fn row_mark(x: char, y: char) -> Option<&'static str> {
    if x == '?' || y == '?' {
        return Some(crate::git::UNTRACKED);
    }
    let of = |c: char| match c {
        ' ' => None,
        'A' => Some(crate::git::ADDED),
        'D' => Some(crate::git::DELETED),
        _ => Some(crate::git::MODIFIED),
    };
    match (of(x), of(y)) {
        (Some(a), Some(b)) => Some(crate::git::fold(a, b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

/// [`sh_over_ssh`] with one directory named in `KHOR_ROOT`.
///
/// **The path travels as an environment value, not inside the script.**
/// A directory pasted into a shell script is a directory that can end
/// the string it is in; `ssh -o SendEnv` needs the far sshd's consent,
/// so the assignment is written by this side into the pipe, quoted
/// once, by code rather than by hand.
fn sh_over_ssh_in(host: &str, root: &str, script: &str) -> Result<String, String> {
    sh_over_ssh(host, &format!("KHOR_ROOT={}\n{script}", sh_quote(root)))
}

/// `git diff -U0` for one file, read on a satellite.
///
/// **`-U0` is not a choice here, it is the shape khor's hunk already
/// is**: `proto::Hunk`'s `at`/`lines` are documented as「same
/// convention as `git diff -U0`'s `+` side」and were checked against it
/// when the in-process reader was written. So the parsing below moves
/// the numbers across rather than deriving them again.
///
/// `HEAD --` rather than a bare `diff` so a file staged and then edited
/// answers about HEAD, which is what the in-process reader compares
/// against; `--no-color`, `--no-ext-diff` and `-M` off so a person's
/// `~/.gitconfig` cannot change what this parses.
const DIFF_SH: &str = r#"
cd "$(dirname "$KHOR_ROOT")" || exit 3
git --no-pager diff -U0 --no-color --no-ext-diff --no-renames HEAD -- "$KHOR_ROOT" 2>&1 | sed 's/^/d=/'
exit 0
"#;

/// [`DIFF_SH`] over ssh, in [`crate::git::Diff`]'s shape.
pub fn git_diff(host: &str, path: &str) -> Result<crate::git::Diff, String> {
    let text = sh_over_ssh_in(host, path, DIFF_SH)?;
    Ok(parse_diff(&text))
}

/// `d=` lines into hunks. Everything before the first `@@` is git's
/// own header and is skipped; a `-` line under a hunk is what HEAD had
/// there, without its terminator, which is the shape `Hunk::was`
/// already documents.
///
/// The caps are `crate::git`'s own, not new numbers: the same frame
/// carries this answer whichever reader filled it.
fn parse_diff(text: &str) -> crate::git::Diff {
    let mut hunks: Vec<crate::proto::Hunk> = Vec::new();
    let mut was_bytes = 0usize;
    let mut more = false;
    for line in text.lines() {
        let Some(body) = line.strip_prefix("d=") else { continue };
        if let Some(head) = body.strip_prefix("@@ ") {
            if hunks.len() >= crate::git::HUNKS || was_bytes >= crate::git::WAS {
                more = true;
                break;
            }
            let Some((at, lines)) = plus_side(head) else { continue };
            hunks.push(crate::proto::Hunk { at, lines, was: Vec::new() });
            continue;
        }
        let Some(old) = body.strip_prefix('-') else { continue };
        // `---` is the file header, which only appears before the first
        // `@@` — a hunk is already open by the time a `-` means a line.
        let Some(h) = hunks.last_mut() else { continue };
        was_bytes += old.len();
        if was_bytes >= crate::git::WAS {
            more = true;
            break;
        }
        h.was.push(old.trim_end_matches(['\n', '\r']).to_owned());
    }
    crate::git::Diff { hunks, more }
}

/// `-a,b +c,d @@ …` → `(c, d)`. A missing count is 1, which is git's
/// own shorthand and the reason a one-line hunk has no comma.
fn plus_side(head: &str) -> Option<(u32, u32)> {
    let plus = head.split_whitespace().find(|w| w.starts_with('+'))?;
    let mut parts = plus[1..].splitn(2, ',');
    let at = parts.next()?.parse::<u32>().ok()?;
    let lines = match parts.next() {
        Some(n) => n.parse::<u32>().ok()?,
        None => 1,
    };
    Some((at, lines))
}

/// One reading of the satellite, taken over the user's ssh road — the
/// same `khor_core::Vitals` shape every machine reports about itself,
/// with `version: None`: no khor runs there, and saying so is the row's
/// honest gap, never a guessed number.
pub fn sample(host: &str) -> Result<khor_core::Vitals, String> {
    let out = ssh_base(host)
        .arg(SAMPLE_SH)
        .output()
        .map_err(|e| msg::ssh_wont_run(e))?;
    if !out.status.success() {
        return Err(msg::sat_cant_reach(host, String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(parse_vitals(&String::from_utf8_lossy(&out.stdout)))
}

/// key=value lines into a reading. Split from the ssh so yesterday's
/// text can be fed to today's parser (the degrade test below): an
/// answer without the per-card and per-volume lines — truncated, or a
/// pre-2026-09-02 shape — yields empty lists and everything else
/// intact, never a guessed row.
///
/// Which way the volume lines can be wrong: `df -kP` is parsed by awk
/// column, so a mount point with a space in it arrives truncated at
/// the space — the numbers stay that volume's own (they sit in earlier
/// columns), only the name is short. Accepted: such mounts are rare on
/// the servers satellites are, and a short name is a display flaw
/// where a shifted number would be a lie.
fn parse_vitals(text: &str) -> khor_core::Vitals {
    let get = |key: &str| -> Option<f64> {
        text.lines()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix('=')?.trim().parse().ok())
    };
    let fill = |used: &str, total: &str| -> Option<khor_core::Fill> {
        Some(khor_core::Fill { used: get(used)? as u64, total: get(total)? as u64 })
    };
    let gpus: Vec<khor_core::GpuCard> = text
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("gpu_card=")?;
            let mut it = rest.splitn(4, '|');
            let util_pct: f32 = it.next()?.trim().parse().ok()?;
            let used: u64 = it.next()?.trim().parse().ok()?;
            let total: u64 = it.next()?.trim().parse().ok()?;
            // Off-scale is evidence of a wrong read; the card is
            // dropped whole (`vitals::gpu`'s clamp scar).
            if !(0.0..=100.0).contains(&util_pct) {
                return None;
            }
            Some(khor_core::GpuCard {
                name: it.next().unwrap_or("").trim().to_owned(),
                util_pct,
                mem: (total > 0).then_some(khor_core::Fill { used, total }),
                // The ssh probe reads utilisation and memory only; a
                // satellite's cards report no temperature.
                temp_c: None,
            })
        })
        .collect();
    let volumes: Vec<khor_core::Volume> = text
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("vol=")?;
            let mut it = rest.splitn(3, '|');
            let mount = it.next()?.trim().to_owned();
            let used: u64 = it.next()?.trim().parse().ok()?;
            let total: u64 = it.next()?.trim().parse().ok()?;
            (total > 0 && crate::vitals::keeps_volume(&mount))
                .then_some(khor_core::Volume { mount, fill: khor_core::Fill { used, total } })
        })
        .collect();
    // `/` as the home: no khor runs on a satellite, so there is no home
    // volume to lead with — the tidy pass still dedupes and drops the
    // sealed twin.
    let volumes = crate::vitals::tidy_volumes(volumes, std::path::Path::new("/"));
    khor_core::Vitals {
        cpu_pct: get("cpu_pct").unwrap_or(0.0) as f32,
        cores: get("cores").unwrap_or(1.0) as u32,
        mem: fill("mem_used", "mem_total").unwrap_or(khor_core::Fill { used: 0, total: 0 }),
        disk: fill("disk_used", "disk_total"),
        gpu: get("gpu_pct").map(|pct| khor_core::Gpu {
            util_pct: pct as f32,
            cards: get("gpu_cards").unwrap_or(1.0) as u32,
            mem: fill("vram_used", "vram_total"),
        }),
        version: None,
        gpus,
        volumes,
        uptime_s: get("uptime_s").unwrap_or(0.0) as u64,
    }
}

/// The argv prefix for an **interactive** ssh to the satellite — what a
/// terminal host on the sponsor runs. `-tt`, doubled: the far end is a
/// shell that wants a tty, and a single `-t` lets ssh decide stdin is
/// not terminal enough and skip the allocation — measured: the remote
/// tmux died with `open terminal failed: not a terminal` while the very
/// same command under `-tt` came up. BatchMode still, because the
/// no-password contract holds for terminals too and a hidden prompt
/// would hang the host, not ask anyone.
pub fn ssh_argv(host: &str) -> Vec<String> {
    let mut v = vec!["ssh".to_owned()];
    if let Some(dir) = ssh_dir() {
        v.push("-F".to_owned());
        v.push(dir.join("config").display().to_string());
    }
    v.extend(["-o".to_owned(), "BatchMode=yes".to_owned(), "-tt".to_owned(), host.to_owned()]);
    v
}

/// A string as one safely-quoted sh word: wrapped in single quotes,
/// with embedded single quotes spelled `'\''`. Paths cross into a
/// remote shell here, and quoting is the difference between a filename
/// and a command.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// One directory of the satellite, over ssh — the same answer shape as
/// the local listing (`crate::files::list_dir`): the expanded path,
/// entries sorted directories-first then case-folded, and whether the
/// cap cut it short. Empty `path` means the satellite's own home, the
/// `Ls` op's rule.
///
/// The line protocol is spelled to survive any filename short of a
/// newline: `d/<name>` and `f <bytes> <name>` — the name is everything
/// after the fixed prefix, spaces included. `find`, not a glob: the
/// remote command runs in whatever login shell the satellite has, and
/// zsh (the Mac default) errors a matchless glob instead of passing it
/// through — measured, `no matches found: ..?*` where a listing should
/// have been. `wc -c` per file costs a fork each, tolerable under the
/// same entry cap the local listing keeps.
pub fn ls(host: &str, path: &str) -> Result<(String, Vec<crate::proto::DirEntry>, bool), String> {
    let target = if path.is_empty() { "\"$HOME\"".to_owned() } else { sh_quote(path) };
    let script = format!(
        r#"cd {target} 2>/dev/null || {{ echo KHOR-NO-DIR; exit 0; }}
pwd
find . -mindepth 1 -maxdepth 1 2>/dev/null | while IFS= read -r f; do
  f=${{f#./}}
  if [ -d "$f" ]; then echo "d/$f"; else echo "f $(wc -c <"$f" 2>/dev/null | tr -d ' 	') $f"; fi
done"#
    );
    let out = ssh_base(host).arg(script).output().map_err(|e| msg::ssh_wont_run(e))?;
    if !out.status.success() {
        return Err(msg::sat_cant_reach(host, String::from_utf8_lossy(&out.stderr).trim()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    if first == "KHOR-NO-DIR" {
        return Err(msg::no_such_dir(path));
    }
    let at = first.to_owned();
    let mut entries: Vec<crate::proto::DirEntry> = Vec::new();
    for l in lines {
        if let Some(name) = l.strip_prefix("d/") {
            // at_ms 0 = "the machine could not say", the field's own
            // word for it: a portable mtime needs stat, whose flags
            // differ per platform, and the browse reads names first.
            entries.push(crate::proto::DirEntry { name: name.to_owned(), size: 0, dir: true, at_ms: 0 });
        } else if let Some(rest) = l.strip_prefix("f ") {
            if let Some((size, name)) = rest.split_once(' ') {
                entries.push(crate::proto::DirEntry {
                    name: name.to_owned(),
                    size: size.parse().unwrap_or(0),
                    dir: false,
                    at_ms: 0,
                });
            }
        }
    }
    // The local listing's order, re-stated: directories first, each
    // half case-folded — the screen paints and never re-sorts.
    entries.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let truncated = entries.len() > crate::files::MOST_ENTRIES;
    entries.truncate(crate::files::MOST_ENTRIES);
    Ok((at, entries, truncated))
}

/// Copies one satellite file down to `dest` on this machine, whole —
/// scp, riding the same config door every ssh here rides. The bytes
/// land where the caller says; slicing them onward to a third machine
/// is the caller's business (`SatFetchPath` stages through this).
pub fn pull_file(host: &str, path: &str, dest: &std::path::Path) -> Result<u64, String> {
    let mut c = Command::new("scp");
    if let Some(dir) = ssh_dir() {
        c.arg("-F").arg(dir.join("config"));
    }
    let out = c
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-q")
        .arg(format!("{host}:{path}"))
        .arg(dest)
        .output()
        .map_err(|e| msg::ssh_wont_run(e))?;
    if !out.status.success() {
        return Err(msg::sat_cant_reach(host, String::from_utf8_lossy(&out.stderr).trim()));
    }
    std::fs::metadata(dest).map(|m| m.len()).map_err(|e| e.to_string())
}

/// Pushes one local file into the satellite's home directory — the
/// sponsor-side half of `khor send <satellite> <file>`.
pub fn push_file(host: &str, local: &std::path::Path) -> Result<u64, String> {
    let size = std::fs::metadata(local).map(|m| m.len()).map_err(|e| e.to_string())?;
    let mut c = Command::new("scp");
    if let Some(dir) = ssh_dir() {
        c.arg("-F").arg(dir.join("config"));
    }
    let out = c
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-q")
        .arg(local)
        .arg(format!("{host}:"))
        .output()
        .map_err(|e| msg::ssh_wont_run(e))?;
    if !out.status.success() {
        return Err(msg::sat_cant_reach(host, String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(size)
}

/// The satellite's stable id: the SHA256 of its host key, hex — read
/// out of known_hosts (`ssh-keygen -F`), the module head's rule.
///
/// The alias the user types may not be what known_hosts is keyed by, so
/// a miss retries with the canonical hostname `ssh -G` resolves the
/// alias to — the same resolution the connection itself would use.
pub fn fingerprint_id(host: &str) -> Result<String, String> {
    if let Some(id) = known_host_id(host)? {
        return Ok(id);
    }
    if let Some(real) = canonical_hostname(host) {
        if real != host {
            if let Some(id) = known_host_id(&real)? {
                return Ok(id);
            }
        }
    }
    Err(msg::sat_not_in_known_hosts(host))
}

/// One name's known_hosts entry, fingerprinted — `None` when the file
/// has never seen it.
fn known_host_id(name: &str) -> Result<Option<String>, String> {
    let mut find = Command::new("ssh-keygen");
    if let Some(dir) = ssh_dir() {
        find.arg("-f").arg(dir.join("known_hosts"));
    }
    let found = find
        .arg("-F")
        .arg(name)
        .output()
        .map_err(|e| msg::ssh_wont_run(e))?;
    if !found.status.success() {
        return Ok(None);
    }
    // Key lines only; `-F` prefixes each match with a `# Host …` comment.
    let keys: String = String::from_utf8_lossy(&found.stdout)
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .map(|l| format!("{l}\n"))
        .collect();
    if keys.is_empty() {
        return Ok(None);
    }
    let printed = {
        use std::io::Write;
        let mut c = Command::new("ssh-keygen")
            .arg("-lf")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| msg::ssh_wont_run(e))?;
        c.stdin
            .take()
            .ok_or_else(|| msg::ssh_wont_run("no stdin"))?
            .write_all(keys.as_bytes())
            .map_err(|e| msg::ssh_wont_run(e))?;
        let out = c.wait_with_output().map_err(|e| msg::ssh_wont_run(e))?;
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    // First fingerprint line wins: `256 SHA256:<b64> name (ED25519)`.
    // known_hosts may hold several key types for one host; any of them
    // identifies it, and "the first" is stable because -F preserves
    // file order.
    let Some(b64) = printed
        .lines()
        .find_map(|l| l.split_whitespace().nth(1)?.strip_prefix("SHA256:").map(str::to_owned))
    else {
        return Ok(None);
    };
    Ok(sha256_b64_to_hex(&b64))
}

/// What known_hosts would key this alias under — the hostname the
/// user's config resolves it to, spelled the way ssh itself records it:
/// bare for port 22, `[host]:port` for any other port.
fn canonical_hostname(host: &str) -> Option<String> {
    let mut c = Command::new("ssh");
    if let Some(dir) = ssh_dir() {
        c.arg("-F").arg(dir.join("config"));
    }
    let out = c.arg("-G").arg(host).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let field = |name: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(name).map(|h| h.trim().to_owned()))
    };
    let hostname = field("hostname ")?;
    match field("port ").as_deref() {
        Some("22") | None => Some(hostname),
        Some(port) => Some(format!("[{hostname}]:{port}")),
    }
}

/// OpenSSH prints SHA256 fingerprints as unpadded base64 of the 32
/// digest bytes; the device table wants those bytes as hex. `None` on
/// anything that does not decode to exactly 32 bytes — a truncated or
/// foreign fingerprint must not mint a malformed id.
fn sha256_b64_to_hex(b64: &str) -> Option<String> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD_NO_PAD;
    let bytes = engine.decode(b64).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fingerprint-to-id math on a fixed digest: 32 bytes in, the
    /// device table's 64-hex shape out — and the two malformed shapes
    /// (bad base64, wrong length) mint nothing rather than a wrong id.
    /// Yesterday's script text — no per-card, no per-volume lines —
    /// parses into today's reading with both lists empty and the
    /// aggregate intact: the degrade path a truncated answer walks.
    #[test]
    fn an_old_scripts_text_parses_with_empty_cards_and_volumes() {
        let old = "cores=8\ncpu_pct=12.5\nmem_total=1000\nmem_used=400\ndisk_total=90\ndisk_used=30\ngpu_pct=40.0\ngpu_cards=2\nvram_used=10\nvram_total=100\n";
        let v = parse_vitals(old);
        assert!(v.gpus.is_empty() && v.volumes.is_empty());
        assert_eq!(v.cpu_pct, 12.5);
        assert_eq!(
            v.gpu,
            Some(khor_core::Gpu {
                util_pct: 40.0,
                cards: 2,
                mem: Some(khor_core::Fill { used: 10, total: 100 })
            })
        );
    }

    /// Today's lines parse per card and per volume; machinery mounts,
    /// off-scale figures and the sealed `/` twin are dropped at the
    /// door — the same rules the local sampler lives by.
    #[test]
    fn todays_text_parses_cards_and_volumes_and_drops_the_noise() {
        let text = "cores=8\ncpu_pct=1.0\nmem_total=10\nmem_used=1\ngpu_card=35.0|1048576|2097152|NVIDIA RTX A5000\ngpu_card=900|1|2|broken\ngpu_card=0.0|0|0|\nvol=/|30|90\nvol=/data|5|50\nvol=/proc|1|2\nvol=/System/Volumes/Data|3|9\n";
        let v = parse_vitals(text);
        let names: Vec<_> = v.gpus.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["NVIDIA RTX A5000", ""], "the off-scale card is dropped whole");
        assert_eq!(v.gpus[0].mem, Some(khor_core::Fill { used: 1048576, total: 2097152 }));
        assert_eq!(v.gpus[1].mem, None, "a zero total is no memory reading");
        let mounts: Vec<_> = v.volumes.iter().map(|m| m.mount.as_str()).collect();
        assert_eq!(
            mounts,
            ["/System/Volumes/Data", "/data"],
            "machinery and the sealed twin are gone"
        );
    }

    #[test]
    fn a_sha256_fingerprint_becomes_a_device_shaped_id() {
        let b64 = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD_NO_PAD.encode([0xabu8; 32])
        };
        let id = sha256_b64_to_hex(&b64).expect("a whole digest mints");
        assert_eq!(id.len(), 64, "the id must wear the device shape");
        assert_eq!(id, "ab".repeat(32));
        assert_eq!(sha256_b64_to_hex("not-base64!"), None, "garbage mints nothing");
        let short = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD_NO_PAD.encode([1u8; 16])
        };
        assert_eq!(sha256_b64_to_hex(&short), None, "a wrong-sized digest mints nothing");
    }

    /// Recorded from a real run of [`PLACES_SH`] on this machine,
    /// 2026-09-13. The nine checkouts it found there were the same nine
    /// `khor workspaces` printed, which is the only evidence this
    /// script implements the same rule as `place::fold` rather than a
    /// similar one; these four lines are the interesting corners of
    /// that run.
    const REAL: &str = "\
home=/Users/zgh
place=/Users/zgh/Desktop/working/khor|3|1787245509|1
place=/Users/zgh/Desktop/working/khor|4|1789274278|1
place=/Users/zgh|5|1788707864|0
place=/Volumes/disk0/workingdir/labsys|2|1789273465|1
";

    #[test]
    fn two_records_of_one_checkout_fold_into_one_row() {
        let rows = parse_places(REAL);
        let khor = rows
            .iter()
            .find(|(path, ..)| path == "/Users/zgh/Desktop/working/khor")
            .expect("the checkout with two records");
        assert_eq!(khor.2, 7, "the counts add");
        assert_eq!(khor.3, 1_789_274_278_000, "the newest wins");
    }

    #[test]
    fn a_directory_that_is_not_a_checkout_is_not_a_workspace() {
        // `~` itself is somewhere sessions ran and is not a project —
        // the same line `Node::workspaces_shown` draws locally.
        let rows = parse_places(REAL);
        assert_eq!(rows.len(), 2, "two checkouts out of three roots");
        assert!(rows.iter().all(|(path, ..)| path != "/Users/zgh"));
    }

    #[test]
    fn the_satellites_home_is_the_one_written_as_a_tilde() {
        // **Not this machine's home.** The sponsor's happens to differ
        // here, and a row shortened with the wrong one is wrong twice:
        // a `~` where the paths merely look alike, and a satellite's
        // own home spelled out in full.
        let rows = parse_places(REAL);
        let khor = rows.iter().find(|(path, ..)| path.ends_with("/khor")).expect("khor");
        assert_eq!(khor.1, "~/Desktop/working/khor");
        let labsys = rows.iter().find(|(path, ..)| path.ends_with("labsys")).expect("labsys");
        assert_eq!(labsys.1, "/Volumes/disk0/workingdir/labsys", "outside home, spelled out");
    }

    #[test]
    fn a_satellite_that_did_not_say_its_home_shortens_nothing() {
        let rows = parse_places("place=/Users/other/w|1|1789274278|1\n");
        assert_eq!(rows[0].1, "/Users/other/w");
    }

    #[test]
    fn newest_first_the_way_the_local_list_is_ordered() {
        let rows = parse_places(REAL);
        let order: Vec<&str> = rows.iter().map(|(path, ..)| path.as_str()).collect();
        assert_eq!(order, ["/Users/zgh/Desktop/working/khor", "/Volumes/disk0/workingdir/labsys"]);
    }

    #[test]
    fn seconds_become_milliseconds() {
        // The satellite's `stat` answers in seconds and every other
        // time in khor is a millisecond. A row off by a thousand sorts
        // to the bottom and reads as「六个月前」.
        let rows = parse_places("place=/w|1|1789274278|1\n");
        assert_eq!(rows[0].3, 1_789_274_278_000);
    }

    #[test]
    fn a_path_with_a_separator_in_it_keeps_its_numbers() {
        // The fields are positional, so a `|` in a directory name would
        // shift them — read off the end, it cannot.
        let rows = parse_places("place=/w/a|b/c|2|1789274278|1\n");
        assert_eq!(rows[0].0, "/w/a|b/c");
        assert_eq!(rows[0].2, 2);
    }

    #[test]
    fn a_truncated_answer_loses_rows_rather_than_inventing_them() {
        // What a connection dropped mid-write leaves. Each line stands
        // on its own, so the ones that arrived whole are still read and
        // the half-line is dropped — never a row with a guessed number
        // in it (`parse_vitals`'s rule).
        let rows = parse_places("place=/a|1|1789274278|1\nplace=/b|1|17892");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "/a");
    }

    #[test]
    fn a_satellite_with_no_records_answers_nothing_rather_than_a_row() {
        assert!(parse_places("").is_empty());
        assert!(parse_places("bash: line 1: stat: command not found\n").is_empty());
    }

    /// Recorded from a real `git status --porcelain=v1 -z` on a
    /// checkout built to hold every shape at once, 2026-09-13, with the
    /// NULs already turned into newlines the way [`STATUS_SH`] leaves
    /// them.
    const PORCELAIN: &str = "\
branch=master
row=A  added.txt
row=MM both.txt
row= D del.txt
row= M mod.txt
row=R  renamed.txt
row=ren.txt
row=?? subdir/
row=?? untracked.txt
";

    #[test]
    fn every_shape_wears_the_letter_the_in_process_reader_gives_it() {
        let st = parse_status(PORCELAIN).expect("reads");
        assert_eq!(st.branch, "master");
        let got: Vec<(&str, &str)> =
            st.rows.iter().map(|(p, m)| (p.as_str(), m.as_str())).collect();
        // Checked against `khor git status` on the same checkout, which
        // reads it in-process through gix: the two printed the same
        // eight lines byte for byte.
        assert_eq!(got, [
            ("added.txt", "A"),
            ("both.txt", "M"),
            ("del.txt", "D"),
            ("mod.txt", "M"),
            ("renamed.txt", "M"),
            ("subdir", "U"),
            ("untracked.txt", "U"),
        ]);
    }

    #[test]
    fn a_renames_old_name_is_not_a_file() {
        // `-z` writes `R  new\0old`, so the name it came from arrives
        // looking like another row. Read as one, `ren.txt` became a
        // modified `n.txt` — a path no repository has ever held.
        let st = parse_status(PORCELAIN).expect("reads");
        assert!(st.rows.iter().all(|(p, _)| p != "n.txt" && p != "ren.txt"));
    }

    #[test]
    fn an_untracked_directory_is_one_row_without_its_slash() {
        // git writes `subdir/`; gix's walk collapses the directory and
        // names it without one. The tree folds marks by path, so the
        // two spellings would be two different directories.
        let st = parse_status(PORCELAIN).expect("reads");
        assert!(st.rows.iter().any(|(p, m)| p == "subdir" && m == "U"));
    }

    #[test]
    fn a_detached_head_has_no_branch_rather_than_the_word_head() {
        let st = parse_status("branch=\nrow= M a.txt\n").expect("reads");
        assert_eq!(st.branch, "");
    }

    /// Recorded from a real `git diff -U0` over the same checkout: two
    /// deletions and one line added at the end.
    const DIFF: &str = "\
d=diff --git a/lines.txt b/lines.txt
d=--- a/lines.txt
d=+++ b/lines.txt
d=@@ -2 +1,0 @@ a
d=-b
d=@@ -4 +2,0 @@ c
d=-d
d=@@ -5,0 +4 @@ e
d=+f
";

    #[test]
    fn the_numbers_are_gits_own_plus_side() {
        let d = parse_diff(DIFF);
        let shape: Vec<(u32, u32, usize)> =
            d.hunks.iter().map(|h| (h.at, h.lines, h.was.len())).collect();
        // A deletion is `+c,0` and `at` is the line it sits after —
        // the convention `proto::Hunk` documents and the in-process
        // reader arrives at by subtracting one. Here it is simply what
        // git said. Checked against `khor git diff` on this file: the
        // same three hunks and the same two old lines.
        assert_eq!(shape, [(1, 0, 1), (2, 0, 1), (4, 1, 0)]);
        assert_eq!(d.hunks[0].was, ["b"]);
        assert_eq!(d.hunks[1].was, ["d"]);
        assert!(!d.more);
    }

    #[test]
    fn a_hunk_without_a_comma_covers_one_line() {
        // git's own shorthand: `+7` means `+7,1`.
        let d = parse_diff("d=@@ -7 +7 @@\nd=-old\n");
        assert_eq!((d.hunks[0].at, d.hunks[0].lines), (7, 1));
    }

    #[test]
    fn the_file_header_is_not_an_old_line() {
        // `---` starts with `-` and is not something HEAD had; it only
        // ever appears before the first `@@`, which is why no hunk is
        // open to take it.
        let d = parse_diff(DIFF);
        assert!(d.hunks.iter().all(|h| h.was.iter().all(|w| !w.starts_with("--"))));
    }

    #[test]
    fn a_file_git_says_nothing_about_has_no_hunks() {
        assert!(parse_diff("").hunks.is_empty());
        assert!(parse_diff("d=fatal: not a git repository\n").hunks.is_empty());
    }
}
