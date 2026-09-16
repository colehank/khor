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

/// Runs a script on the satellite **as `sh`, reading it from stdin**.
///
/// **`ssh host "<script>"` is not run by `sh`.** It is handed to the
/// far user's *login* shell, and on a mac that is zsh — which, unlike
/// sh, treats a glob that matches nothing as an error and aborts the
/// line (`no matches found: …/*.jsonl`). The first real run of a script
/// through here died on exactly that, against a satellite whose home
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

/// When a directory on a satellite was last changed — the same question
/// [`crate::workspace::touched`] answers locally, and it has to be the
/// same answer: one workspace list shows rows from both kinds of
/// machine, and a column ordered by two different rules is a column
/// nobody can read.
///
/// **The skip list is spelled twice and must not drift.** There is no
/// way to hand a Rust constant to a shell without pasting it in, and a
/// script built by string-joining `workspace::SKIP` would be a script
/// nobody can read in the file. What holds the two together is
/// `a_satellites_skip_list_is_the_same_one_the_local_walk_uses` below.
///
/// **`stat -c %Y` or `stat -f %m`**: GNU and BSD spell the same
/// question differently and neither accepts the other's flag, so it is
/// tried in that order — the idiom [`STATUS_SH`] already uses.
///
/// **`-exec … +` and not one `stat` per file**: a project tree is
/// thousands of files and a satellite is reached over a link, not a
/// bus. The cap is `head` on the pipe rather than a count in the walk;
/// `find` dying of SIGPIPE when it closes is why the script ends
/// `exit 0`: **a script's last command's status is not a verdict** —
/// an empty glob leaves an `&&` that answered no, and the whole read
/// would come back as「ssh 够不着」with an empty reason (measured
/// 2026-09-13, on the places script this file used to carry). A
/// connection that really failed is ssh's own 255, which this cannot
/// overwrite.
///
/// Nothing is written on the satellite — no temp file, no marker — for
/// the reason the whole module is built this way: 「它身上不留东西」.
const TOUCHED_SH: &str = r#"
[ -d "$KHOR_ROOT" ] || { printf 'at=0\n'; exit 0; }
max=$(find "$KHOR_ROOT" \( -type d \( -name .git -o -name node_modules -o -name target \
  -o -name .venv -o -name __pycache__ -o -name dist -o -name build \) \) -prune \
  -o -type f -exec sh -c 'stat -c %Y "$@" 2>/dev/null || stat -f %m "$@" 2>/dev/null' _ {} + \
  2>/dev/null | head -n 20000 | sort -n | tail -1)
printf 'at=%s\n' "${max:-0}"
exit 0
"#;

/// Takes a path off a satellite's disk (工作区批④).
///
/// **Always outright, never to a recycle bin.** A satellite runs no
/// khor and khor leaves nothing on it (docs/NET.md 它身上不留东西), so
/// there is no bin khor could put something in and nothing that would
/// know how to get it back out. The face says 直接删 before anybody
/// presses, which is the same sentence every other machine's rows get.
///
/// **`test -e` first, and it tests the link rather than its target**
/// (`-e` follows, `-L` does not, so both are asked): `rm -rf` on
/// something that is not there succeeds silently, and 「it was not
/// there」 is a different answer from 「it is gone now」 — the one
/// khor's own `files::remove` refuses on, so this road must refuse on
/// it too or one machine would answer differently from another.
///
/// The missing case answers one word rather than a sentence, because
/// the sentence belongs to the catalog on this side: a satellite is not
/// where khor's wording lives.
const REMOVE_SH: &str = r#"
if [ ! -e "$KHOR_ROOT" ] && [ ! -L "$KHOR_ROOT" ]; then
  printf 'gone
'
  exit 0
fi
rm -rf "$KHOR_ROOT" || { printf 'kept
'; exit 0; }
printf 'done
'
exit 0
"#;

/// [`REMOVE_SH`] over the user's ssh road.
pub fn remove(host: &str, path: &str) -> Result<(), String> {
    let out = sh_over_ssh_in(host, path, REMOVE_SH)?;
    let said = out.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    match said {
        "done" => Ok(()),
        // Translated here, in khor's own words: the far side said which
        // of the two it was and this side is where sentences live.
        "gone" => Err(msg::cant_remove(path, "No such file or directory")),
        _ => Err(msg::cant_remove(path, said)),
    }
}

/// Makes one directory on a satellite (工作区第五轮).
///
/// **The same three answers khor's own `files::mkdir` gives**, in the
/// same order: a name already taken, one directory rather than a chain,
/// and the shell's own words for anything else. `mkdir` without `-p` is
/// already both of the first two — it refuses an existing name and it
/// refuses a missing parent — so the only thing this script adds is
/// telling those two apart, because 「那个名字已经有了」 is the one a
/// person can act on by typing a different name.
const MKDIR_SH: &str = r#"
if [ -e "$KHOR_ROOT" ] || [ -L "$KHOR_ROOT" ]; then
  printf 'taken
'
  exit 0
fi
mkdir "$KHOR_ROOT" 2>/dev/null || { printf 'no
'; exit 0; }
printf 'done
'
exit 0
"#;

/// [`MKDIR_SH`] over the user's ssh road.
pub fn mkdir(host: &str, path: &str) -> Result<(), String> {
    let out = sh_over_ssh_in(host, path, MKDIR_SH)?;
    let said = out.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    match said {
        "done" => Ok(()),
        // Translated here, in khor's own words (`remove`'s rule).
        "taken" => Err(msg::name_taken(path)),
        _ => Err(msg::cant_mkdir(path, "mkdir")),
    }
}

/// Makes an empty file on a satellite.
///
/// **Not a zero-byte [`write_file`].** That road hashes what is there
/// and refuses if it moved, which needs `sha256sum` or `shasum` on the
/// far machine — tools `sat_no_tools` exists to say may be missing. A
/// new name has nothing to hash, so this asks the one question that is
/// actually being asked: is the name free.
const TOUCH_SH: &str = r#"
if [ -e "$KHOR_ROOT" ] || [ -L "$KHOR_ROOT" ]; then
  printf 'taken
'
  exit 0
fi
: > "$KHOR_ROOT" 2>/dev/null || { printf 'no
'; exit 0; }
printf 'done
'
exit 0
"#;

/// [`TOUCH_SH`] over the user's ssh road.
pub fn touch(host: &str, path: &str) -> Result<(), String> {
    let out = sh_over_ssh_in(host, path, TOUCH_SH)?;
    let said = out.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    match said {
        "done" => Ok(()),
        "taken" => Err(msg::name_taken(path)),
        _ => Err(msg::cant_save_file(path, "touch")),
    }
}

/// Moves a name on a satellite.
///
/// **Two paths, so the assignments are written by this side** rather
/// than pasted into the script ([`sh_over_ssh_in`] knows one directory;
/// [`write_file`] is the two-argument template this copies).
///
/// `mv` on unix replaces an existing target without a word, which is the
/// whole reason the target is tested first — the same check khor's own
/// `files::rename_path` makes, so one machine does not answer
/// differently from another.
const RENAME_SH: &str = r#"
set -u
if [ ! -e "$KHOR_FROM" ] && [ ! -L "$KHOR_FROM" ]; then
  printf 'gone
'
  exit 0
fi
if [ -e "$KHOR_TO" ] || [ -L "$KHOR_TO" ]; then
  printf 'taken
'
  exit 0
fi
mv "$KHOR_FROM" "$KHOR_TO" 2>/dev/null || { printf 'no
'; exit 0; }
printf 'done
'
exit 0
"#;

/// [`RENAME_SH`] over the user's ssh road.
pub fn rename_path(host: &str, from: &str, to: &str) -> Result<(), String> {
    let script = format!(
        "KHOR_FROM={}\nKHOR_TO={}\n{RENAME_SH}",
        sh_quote(from),
        sh_quote(to),
    );
    let out = sh_over_ssh(host, &script)?;
    let said = out.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    match said {
        "done" => Ok(()),
        "gone" => Err(msg::cant_move_name(from, "No such file or directory")),
        "taken" => Err(msg::name_taken(to)),
        _ => Err(msg::cant_move_name(from, "mv")),
    }
}

/// Hands a path or a page to a satellite's own opener.
///
/// **The whitelist is kept on this side** — `files::launch` has already
/// refused everything but an absolute path and an `http(s)` address by
/// the time this runs — and the quoting is `sh_quote`'s, so the target
/// is one word to the far shell whatever is in it.
///
/// **A satellite very often has no screen**, and `xdg-open` on a box
/// with no display session fails saying so. That sentence is the far
/// machine's and travels back as it came; khor adds nothing to it,
/// because 「这台机器没人坐在前面」 is not something khor knows.
///
/// **`KHOR_LAUNCHER` is not consulted here and that is deliberate**: it
/// names the opener on the machine khor is *installed* on, and khor is
/// installed on nothing a satellite runs (docs/NET.md 它身上不留东西).
/// A satellite's opener is whichever of the two its desktop ships.
const LAUNCH_SH: &str = r#"
set -u
for c in open xdg-open; do
  if command -v "$c" >/dev/null 2>&1; then
    "$c" "$KHOR_TARGET" >/dev/null 2>&1 &
    printf 'done
'
    exit 0
  fi
done
printf 'none
'
exit 0
"#;

/// [`LAUNCH_SH`] over the user's ssh road.
pub fn launch(host: &str, target: &str) -> Result<(), String> {
    let script = format!("KHOR_TARGET={}\n{LAUNCH_SH}", sh_quote(target));
    let out = sh_over_ssh(host, &script)?;
    let said = out.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    match said {
        "done" => Ok(()),
        "none" => Err(msg::no_launcher(crate::files::LAUNCHER_ENV)),
        _ => Err(msg::cant_launch(target, said)),
    }
}

/// [`TOUCHED_SH`] over the user's ssh road, in ms since the epoch —
/// `0` where there was nothing to date.
pub fn touched(host: &str, path: &str) -> Result<i64, String> {
    Ok(parse_touched(&sh_over_ssh_in(host, path, TOUCHED_SH)?))
}

/// The `at=` line, in ms. **Split from the ssh for [`parse_vitals`]'s
/// reason**, and unreadable output answers `0` rather than guessing:
/// this orders a list, and a guessed date would reorder it.
fn parse_touched(text: &str) -> i64 {
    text.lines()
        .find_map(|l| l.strip_prefix("at="))
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(0)
        * 1000
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
/// **`--ignored`, added 2026-09-16**, because the in-process reader now
/// emits ignored paths too (`crate::git::IGNORED`) and the two readers
/// have to answer the same rows about the same repository — a tree read
/// through a satellite that greyed nothing would be the same screen
/// disagreeing with itself depending on which disk it came from. The
/// bare flag is `--ignored=traditional`, which collapses an ignored
/// directory to one row exactly as gix's `CollapseDirectory` does.
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
# `@{u}` is the branch's own remote-tracking ref, and the command says
# nothing at all when there is no upstream — which is the same silence
# a detached HEAD gives, and the same one the in-process reader answers
# `None` for.
git rev-list --left-right --count '@{u}...HEAD' 2>/dev/null |
  awk '{printf "behind=%s\nahead=%s\n", $1, $2}'
# `.git/FETCH_HEAD`'s mtime dates the pair above. git writes it on
# every fetch and touches it for nothing else; a repository that never
# fetched has no file, and the line is simply absent.
g=$(git rev-parse --git-dir 2>/dev/null)
if [ -n "$g" ] && [ -f "$g/FETCH_HEAD" ]; then
  printf 'fetched=%s\n' "$(stat -c %Y "$g/FETCH_HEAD" 2>/dev/null || stat -f %m "$g/FETCH_HEAD" 2>/dev/null || echo 0)"
fi
git status --porcelain=v1 --ignored -z 2>&1 | tr '\0' '\n' | sed 's/^/row=/'
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
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut fetched_ms = 0i64;
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
        if let Some(n) = line.strip_prefix("ahead=") {
            ahead = n.trim().parse().unwrap_or(0);
            continue;
        }
        if let Some(n) = line.strip_prefix("behind=") {
            behind = n.trim().parse().unwrap_or(0);
            continue;
        }
        if let Some(n) = line.strip_prefix("fetched=") {
            // Seconds over there, milliseconds everywhere in khor.
            fetched_ms = n.trim().parse::<i64>().unwrap_or(0) * 1000;
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
    Ok(crate::git::Status { branch, rows, more, ahead, behind, fetched_ms })
}

/// One porcelain row's two columns as one of khor's seven letters.
fn row_mark(x: char, y: char) -> Option<&'static str> {
    // `!!` is git's own spelling for an ignored path, and it is read
    // first for the same reason `?` is: neither column is a comparison,
    // so the pair means one thing whole (`crate::git::IGNORED`).
    if x == '!' || y == '!' {
        return Some(crate::git::IGNORED);
    }
    if x == '?' || y == '?' {
        return Some(crate::git::UNTRACKED);
    }
    // **Unmerged is read before the columns are read apart.** porcelain
    // v1 spells it as a `U` in either column *or* one of `DD`/`AA`/`UU`,
    // and those doubled letters are one fact about one file — folded
    // column by column they would come out as an ordinary deletion or
    // an ordinary addition, which is the reading that sends somebody to
    // commit a file git will not take (crate::git::CONFLICT).
    if x == 'U' || y == 'U' || (x == y && (x == 'D' || x == 'A')) {
        return Some(crate::git::CONFLICT);
    }
    let of = |c: char| match c {
        ' ' => None,
        // A copy makes a file that was not there, which is what `A`
        // already means; only the rename gets a letter of its own,
        // because only the rename leaves HEAD holding another path.
        'A' | 'C' => Some(crate::git::ADDED),
        'D' => Some(crate::git::DELETED),
        'R' => Some(crate::git::RENAMED),
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

/// Writes a whole file on the satellite, **and only if it still holds
/// what the reader read** (工作区批7).
///
/// # Why this can keep khor's save promise after all
///
/// The promise everywhere else in khor is `files::write_whole`'s: the
/// bytes you are replacing are still the bytes you read, or nothing is
/// written. This road was refused on the grounds that it could not keep
/// it — the digest khor hands out is blake3, nothing over there can
/// compute one, and a sponsor that pulled the file, hashed it here and
/// pushed it back would leave a window in the middle the size of
/// somebody else's save (docs/handoff/001.md, 卫星工作区批).
///
/// **The window was the wrong thing to look at.** The check does not
/// have to happen at the same moment as the read; it has to be *pinned
/// to content*. So the comparison moves to the machine that holds the
/// file: the sponsor computes, from the copy whose blake3 is the one
/// the person read, that copy's sha256; the script below compares the
/// file's own sha256 to it and writes only on a match. Whatever happens
/// in between, a file somebody else touched hashes differently and this
/// writes nothing. The two algorithms never have to meet — one copy of
/// the bytes carries both.
///
/// `want` is that sha256, in lower-case hex. The reply is either
/// `ok <mtime-seconds>` or one of the named refusals, never a partial
/// write: the content lands under a dot-name in the destination's own
/// directory and is renamed into place, which is what `write_whole`
/// does locally and for the same reason.
///
/// Base64 rather than a heredoc because a heredoc's terminator can
/// appear inside any file, and a delimiter chosen by luck is a file
/// truncated by luck. `base64 -d` is coreutils on Linux and BSD on
/// macOS; both take `-d`, and a machine without it answers `nob64`
/// rather than writing something wrong.
const WRITE_SH: &str = r#"
set -u
f=$KHOR_FILE
want=$KHOR_WANT
if [ ! -f "$f" ]; then echo "gone"; exit 0; fi
now=$(sha256sum "$f" 2>/dev/null | cut -d' ' -f1)
if [ -z "$now" ]; then now=$(shasum -a 256 "$f" 2>/dev/null | cut -d' ' -f1); fi
if [ -z "$now" ]; then echo "nosha"; exit 0; fi
if [ "$now" != "$want" ]; then echo "moved"; exit 0; fi
command -v base64 >/dev/null 2>&1 || { echo "nob64"; exit 0; }
tmp=$(dirname "$f")/.khor-save-$(basename "$f")
printf %s "$KHOR_B64" | base64 -d > "$tmp" || { rm -f "$tmp"; echo "nowrite"; exit 0; }
mv -f "$tmp" "$f" || { rm -f "$tmp"; echo "nowrite"; exit 0; }
at=$(stat -c %Y "$f" 2>/dev/null || stat -f %m "$f" 2>/dev/null)
echo "ok ${at:-0}"
exit 0
"#;

/// [`WRITE_SH`] over the user's ssh road. Answers the file's mtime in
/// milliseconds.
///
/// Every refusal is a sentence rather than a silence, because the one
/// thing a save must never do is answer「fine」 while writing nothing.
pub fn write_file(host: &str, path: &str, want_sha: &str, bytes: &[u8]) -> Result<u64, String> {
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    let script = format!(
        "KHOR_FILE={}
KHOR_WANT={}
KHOR_B64={}
{WRITE_SH}",
        sh_quote(path),
        sh_quote(want_sha),
        sh_quote(&b64),
    );
    let out = sh_over_ssh(host, &script)?;
    let said = out.trim();
    match said.split_once(' ') {
        Some(("ok", at)) => Ok(at.trim().parse::<u64>().unwrap_or(0) * 1000),
        _ => match said {
            "gone" | "moved" => Err(msg::file_changed_under_you(path)),
            "nosha" | "nob64" => Err(msg::sat_no_tools(host)),
            _ => Err(msg::cant_save_file(path, said)),
        },
    }
}

/// The sha256 of some bytes, in the spelling the satellite's own
/// `sha256sum` prints — the bridge between khor's digest and the one
/// the far machine can compute.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let mut h = sha2::Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
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
pwd -P
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

    /// **Two spellings of one skip list, held together.** The names
    /// live in `workspace::SKIP` for the local walk and are pasted into
    /// [`TOUCHED_SH`] for the satellite's; a name added to one and not
    /// the other is a column ordered by two rules, which is the one
    /// failure this pair can have.
    #[test]
    fn a_satellites_skip_list_is_the_same_one_the_local_walk_uses() {
        for name in crate::workspace::SKIP {
            assert!(
                TOUCHED_SH.contains(&format!("-name {name} ")),
                "{name} is skipped locally and not over ssh"
            );
        }
        assert_eq!(
            TOUCHED_SH.matches("-name ").count(),
            crate::workspace::SKIP.len(),
            "the script skips something the local walk does not"
        );
    }

    /// **The script run as `sh`, against a real tree, answering what
    /// the local walk answers.** No ssh: what is being checked is the
    /// script's portability and its arithmetic, and `sh` on this
    /// machine is the same `sh` the far side pipes it into
    /// (`sh_over_ssh` on why it is never the login shell).
    #[test]
    fn the_touched_script_answers_what_the_local_walk_answers() {
        // Takes itself away however this test leaves — an assertion
        // panics past a cleanup written at the end, and a machine's
        // temp directory should not grow one tree per run.
        struct Scratch(std::path::PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let scratch = Scratch(
            std::env::temp_dir().join(format!("khor-sat-touched-{}", std::process::id())),
        );
        let tmp = scratch.0.clone();
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src/deep")).unwrap();
        std::fs::create_dir_all(tmp.join("target/debug")).unwrap();
        std::fs::write(tmp.join("src/deep/one.rs"), "x").unwrap();
        std::fs::write(tmp.join("README.md"), "x").unwrap();
        // Newer than everything, and inside a skipped directory — the
        // arm that tells a walk from a build clock. **Dated by hand**,
        // because three files written in one second cannot tell a
        // pruned walk from an unpruned one.
        std::fs::write(tmp.join("target/debug/fresh"), "x").unwrap();
        let stamp = |p: std::path::PathBuf, secs: u64| {
            let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
            std::fs::File::open(p).unwrap().set_modified(t).unwrap();
        };
        stamp(tmp.join("README.md"), 1_000);
        stamp(tmp.join("src/deep/one.rs"), 2_000);
        stamp(tmp.join("target/debug/fresh"), 9_000);

        let script = format!("KHOR_ROOT={}\n{TOUCHED_SH}", sh_quote(&tmp.display().to_string()));
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("sh runs");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let said = parse_touched(&String::from_utf8_lossy(&out.stdout));

        // To the second, because the far side's `stat` answers seconds
        // and the local walk answers milliseconds.
        let here = crate::workspace::touched(&tmp) / 1000;
        assert_eq!(said / 1000, here, "the script and the local walk must date one tree alike");
        assert_eq!(here, 2_000, "…and both must have skipped the build output");

        let empty = tmp.join("src/deep/nothing");
        std::fs::create_dir_all(&empty).unwrap();
        let script = format!("KHOR_ROOT={}\n{TOUCHED_SH}", sh_quote(&empty.display().to_string()));
        let out = std::process::Command::new("sh").arg("-c").arg(&script).output().unwrap();
        assert_eq!(
            parse_touched(&String::from_utf8_lossy(&out.stdout)),
            0,
            "nothing to date answers 0, not 1970-plus-noise"
        );
    }

    /// Recorded from a real `git status --porcelain=v1 -z` on a
    /// checkout built to hold every shape at once, 2026-09-13, with the
    /// NULs already turned into newlines the way [`STATUS_SH`] leaves
    /// them. **The two `!!` rows joined it on 2026-09-16**, off the same
    /// checkout read again with `--ignored`: a file the repository's
    /// `.gitignore` covers and a whole directory it covers, which is the
    /// pair the tree greys (`crate::git::IGNORED`).
    const PORCELAIN: &str = "\
branch=master
row=A  added.txt
row=MM both.txt
row= D del.txt
row= M mod.txt
row=R  renamed.txt
row=ren.txt
row=UU clash.txt
row=DD gone-both-ways.txt
row=?? subdir/
row=?? untracked.txt
row=!! debug.log
row=!! build/
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
            // An ignored directory, collapsed and without its slash —
            // the same row gix's walk gives it, and grey rather than a
            // change (2026-09-16).
            ("build", "!"),
            // Unmerged in its two porcelain spellings (工作区批⑦). `DD`
            // folded column by column is an ordinary deletion, which is
            // the reading that tells somebody a file is gone when what
            // is true is that git will not take it either way.
            ("clash.txt", "C"),
            ("debug.log", "!"),
            ("del.txt", "D"),
            ("gone-both-ways.txt", "C"),
            ("mod.txt", "M"),
            ("renamed.txt", "R"),
            ("subdir", "U"),
            ("untracked.txt", "U"),
        ]);
    }

    #[test]
    fn an_ignored_path_is_never_folded_into_a_change() {
        // `fold` runs on every row, first insert included, so an
        // ignored path folded against itself has to come back ignored —
        // the loop's fall-through is `MODIFIED`, and that reading
        // painted every build directory as an edit nobody made.
        assert_eq!(crate::git::fold(crate::git::IGNORED, crate::git::IGNORED), crate::git::IGNORED);
        // …and it gives way to anything git is actually looking at: a
        // directory holding an edited file and a build output is 黄.
        assert_eq!(crate::git::fold(crate::git::IGNORED, crate::git::MODIFIED), crate::git::MODIFIED);
        assert_eq!(crate::git::fold(crate::git::CONFLICT, crate::git::IGNORED), crate::git::CONFLICT);
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
    fn the_pair_and_its_date_travel_together() {
        let st = parse_status(
            "branch=main\nbehind=2\nahead=5\nfetched=1789274278\nrow= M a.txt\n",
        )
        .expect("reads");
        assert_eq!((st.ahead, st.behind), (5, 2));
        // Seconds over there, milliseconds in khor — a date off by a
        // thousand reads as 1970 and the face would say the fetch was
        // fifty-six years ago.
        assert_eq!(st.fetched_ms, 1_789_274_278_000);
    }

    #[test]
    fn a_branch_with_no_upstream_has_no_pair_and_no_date() {
        // `git rev-list @{u}...` prints nothing at all without an
        // upstream, and a repository that never fetched has no
        // FETCH_HEAD — so both lines are simply absent, and 0/0 with a
        // date of 0 is what the face reads as「say nothing」.
        let st = parse_status("branch=main\nrow= M a.txt\n").expect("reads");
        assert_eq!((st.ahead, st.behind, st.fetched_ms), (0, 0, 0));
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
