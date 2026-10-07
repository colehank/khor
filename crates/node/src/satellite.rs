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

/// Where a machine is reached over ssh: the destination as the user
/// names it (a Host alias or user@host), and the jump in front of it
/// when there is one.
///
/// **The jump rides every command as `-J`, never the user's config**
/// (docs/NET.md 卫星 经由): khor does not write `~/.ssh`, so a jump the
/// person picked in khor lives on the satellite's row next to its
/// destination and is handed to each ssh and scp this module runs.
/// `Display` is the destination alone — what every sentence names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dest {
    pub host: String,
    pub jump: Option<String>,
}

impl Dest {
    pub fn new(host: impl Into<String>, jump: Option<&str>) -> Dest {
        Dest {
            host: host.into(),
            jump: jump.filter(|j| !j.is_empty()).map(str::to_owned),
        }
    }

    /// A satellite row's road: its destination and its jump, `None`
    /// when the row has no destination on record.
    pub fn of(d: &khor_sync::devices::DeviceInfo) -> Option<Dest> {
        Some(Dest::new(d.ssh_host.clone()?, d.ssh_jump.as_deref()))
    }

    /// The flags every ssh and scp this module runs shares, before the
    /// destination: the test door's config, BatchMode, and the jump.
    ///
    /// **The jump is spelled as the `ProxyCommand` that `-J` stands
    /// for**, `ssh -W %h:%p <jump>`, because `-J` starts that second ssh
    /// without our `-o`s: measured on a scratch sshd, a jump that wanted
    /// a password was asked for one three times, and from a terminal it
    /// would have prompted. Spelled out, the jump gets BatchMode and the
    /// same timeout as the hop behind it.
    pub fn road(&self) -> Vec<String> {
        let mut v = Vec::new();
        let mut cfg = String::new();
        if let Some(dir) = ssh_dir() {
            let path = dir.join("config").display().to_string();
            cfg = format!("-F {} ", sh_quote(&path));
            v.extend(["-F".to_owned(), path]);
        }
        v.extend(["-o".to_owned(), "BatchMode=yes".to_owned()]);
        if let Some(j) = &self.jump {
            v.push("-o".to_owned());
            v.push(format!(
                "ProxyCommand=ssh {cfg}-o BatchMode=yes -o ConnectTimeout=8 -W %h:%p {}",
                sh_quote(j)
            ));
        }
        v
    }
}

impl std::fmt::Display for Dest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.host)
    }
}

/// Which part of an ssh road broke, read off ssh's own words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Broken {
    /// This machine could not get to the jump, or the jump refused it.
    ToJump,
    /// The jump was reached and could not open the way on to the target.
    JumpToTarget,
    /// The target answered and takes only a password (or keys that are
    /// not this machine's): BatchMode stopped at its door.
    PasswordOnly,
}

/// Reads which segment failed from what ssh printed — the texts below
/// are OpenSSH's, measured on scratch sshds (OpenSSH 10.3, 2026-10-08):
///
/// - jump down: the inner ssh says `ssh: connect to host … port …` or
///   `Could not resolve hostname`, and the outer one `Connection closed
///   by UNKNOWN port 65535` (its proxy, not a host);
/// - jump refuses this key: `Permission denied (…)` **and** the outer's
///   UNKNOWN line;
/// - jump up, target not: `channel 0: open failed: …` and `stdio
///   forwarding failed`, from the jump's `-W`;
/// - target refuses the key: `Permission denied (publickey,password)`
///   with no UNKNOWN line — over a jump or not.
///
/// `None` is "ssh said something else": the caller passes its words on
/// unread (a banner timeout over a proxy cannot say which side hung).
pub fn broken(to: &Dest, said: &str) -> Option<Broken> {
    let via_proxy = said.contains("UNKNOWN port 65535");
    let denied = said.contains("Permission denied");
    if to.jump.is_some() {
        if said.contains("stdio forwarding failed") || said.contains("open failed") {
            return Some(Broken::JumpToTarget);
        }
        if via_proxy
            && (denied
                || said.contains("ssh: connect to host")
                || said.contains("Could not resolve hostname")
                || said.contains("Host key verification failed"))
        {
            return Some(Broken::ToJump);
        }
    }
    if denied && !via_proxy {
        return Some(Broken::PasswordOnly);
    }
    None
}

/// The sentence for an ssh that failed: the broken segment named when
/// ssh's words say which, else ssh's words as they are. ssh's last
/// meaningful line rides along, never a password prompt — BatchMode.
pub(crate) fn cant_reach(to: &Dest, said: &str) -> String {
    let said = said.trim();
    // The proxy's own close is noise once the segment is named.
    let words: Vec<&str> = said
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains("UNKNOWN port 65535"))
        .collect();
    // The jump's own refusal is the line that says why (`channel 0:
    // open failed: connect failed: Connection refused`), not the
    // `stdio forwarding failed` after it.
    let last = words
        .iter()
        .find(|l| l.contains("open failed"))
        .or(words.last())
        .copied()
        .unwrap_or(said);
    match (broken(to, said), &to.jump) {
        (Some(Broken::ToJump), Some(j)) => msg::sat_to_jump_broken(j, last),
        (Some(Broken::JumpToTarget), Some(j)) => msg::sat_jump_to_target_broken(j, &to.host, last),
        (Some(Broken::PasswordOnly), _) => msg::sat_password_only(&to.host, copy_id_line(to)),
        _ => msg::sat_cant_reach(to, said),
    }
}

/// What a person runs once to stop a host asking for a password: the
/// GUI's action opens a terminal running exactly this (argv, so no shell
/// ever sees the names), and the sentence quotes it.
pub fn copy_id_argv(to: &Dest) -> Vec<String> {
    let mut v = vec!["ssh-copy-id".to_owned()];
    if let Some(dir) = ssh_dir() {
        v.extend(["-F".to_owned(), dir.join("config").display().to_string()]);
    }
    if let Some(j) = &to.jump {
        v.extend(["-o".to_owned(), format!("ProxyJump={j}")]);
    }
    v.push(to.host.clone());
    v
}

fn copy_id_line(to: &Dest) -> String {
    match &to.jump {
        Some(j) => format!("ssh-copy-id -o ProxyJump={j} {}", to.host),
        None => format!("ssh-copy-id {}", to.host),
    }
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
fn ssh_base(host: &Dest) -> Command {
    let mut c = Command::new("ssh");
    c.args(host.road());
    c.arg("-o")
        .arg("ConnectTimeout=8")
        .arg(&host.host);
    c
}

/// One connection, two answers: what it is, and whether tmux is there.
pub fn probe(host: &Dest) -> Result<Probe, String> {
    let out = ssh_base(host)
        .arg("uname -s 2>/dev/null || echo unknown; command -v tmux >/dev/null 2>&1 && echo tmux-yes || echo tmux-no")
        .output()
        .map_err(msg::ssh_wont_run)?;
    if !out.status.success() {
        return Err(cant_reach(host, &String::from_utf8_lossy(&out.stderr)));
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
fn sh_over_ssh(host: &Dest, script: &str) -> Result<String, String> {
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
        return Err(cant_reach(host, &String::from_utf8_lossy(&out.stderr)));
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
pub fn remove(host: &Dest, path: &str) -> Result<(), String> {
    let out = sh_over_ssh_in(host, path, REMOVE_SH)?;
    let said = out
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
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
pub fn mkdir(host: &Dest, path: &str) -> Result<(), String> {
    let out = sh_over_ssh_in(host, path, MKDIR_SH)?;
    let said = out
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
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
pub fn touch(host: &Dest, path: &str) -> Result<(), String> {
    let out = sh_over_ssh_in(host, path, TOUCH_SH)?;
    let said = out
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
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

/// [`RENAME_SH`] with its two paths in front of it ([`search_script`]'s
/// reason).
fn rename_script(from: &str, to: &str) -> String {
    format!(
        "KHOR_FROM={}\nKHOR_TO={}\n{RENAME_SH}",
        sh_quote(from),
        sh_quote(to),
    )
}

/// [`RENAME_SH`] over the user's ssh road.
pub fn rename_path(host: &Dest, from: &str, to: &str) -> Result<(), String> {
    let out = sh_over_ssh(host, &rename_script(from, to))?;
    let said = out
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
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

/// [`LAUNCH_SH`] with its target in front of it ([`search_script`]'s
/// reason).
fn launch_script(target: &str) -> String {
    format!("KHOR_TARGET={}\n{LAUNCH_SH}", sh_quote(target))
}

/// [`LAUNCH_SH`] over the user's ssh road.
pub fn launch(host: &Dest, target: &str) -> Result<(), String> {
    let out = sh_over_ssh(host, &launch_script(target))?;
    let said = out
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    match said {
        "done" => Ok(()),
        "none" => Err(msg::no_launcher(crate::files::LAUNCHER_ENV)),
        _ => Err(msg::cant_launch(target, said)),
    }
}

/// Names and lines under one directory on a satellite (工作区第六轮
/// 第 47 条).
///
/// **Two commands, one for each half of the answer**, because the two
/// halves are two questions: `find` walks names and `grep -rnI` walks
/// text, and a single tool that did both would be one of them doing the
/// other badly. Both are piped through `head`, so the caps are the far
/// machine's own work rather than this side reading a million rows and
/// throwing them away.
///
/// **`grep -F` and `find | grep -F`, never `-iname`.** khor's own walk
/// matches a plain substring (`crate::files::search`), and `-iname`
/// would read a `*` in somebody's query as a glob — the same repository
/// answering two different tables depending on which road it was asked
/// over, which is the thing every satellite script here exists to
/// avoid.
///
/// **`.gitignore` is read by asking git, in a checkout** (工作区第七轮
/// 第 55 条). The local walk honours it and this road had nothing that
/// could: reimplementing git's exclude rules in `sh` is the
/// almost-right that goes wrong on the one project that needs it, so
/// the rules are not reimplemented — `git ls-files -co
/// --exclude-standard` and `git grep --untracked --exclude-standard`
/// **are** those rules, and every satellite with a checkout on it has
/// the tool that owns them. Both list what is tracked plus what is
/// merely untracked, which is what a person sees in the tree.
///
/// Outside a checkout there is nothing to ask and the walk is `find` +
/// `grep -rI`, which is what this whole script used to be. A directory
/// with a `.gitignore` and no `.git` is therefore answered differently
/// here than by the local walk (`crate::files::search` reads the file
/// regardless) — the one place the two roads still part, and it is
/// written down rather than hidden.
///
/// **Names are matched relative to `$KHOR_DIR`** (工作区第七轮 第 56
/// 条): git prints them that way already, and `find`'s `./` lead is cut
/// so the other branch agrees with it.
const SEARCH_SH: &str = r#"
set -u
cd "$KHOR_DIR" || exit 3
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git ls-files -co --exclude-standard -- . 2>/dev/null |
    grep -i -F -e "$KHOR_Q" 2>/dev/null |
    head -n "$KHOR_NAMES" |
    sed 's/^/name=/'
  git --no-pager grep -n -I -i -F --no-color --untracked --exclude-standard -e "$KHOR_Q" -- . 2>/dev/null |
    head -n "$KHOR_LINES" |
    sed 's/^/row=/'
  exit 0
fi
find . -type f -not -path '*/.git/*' 2>/dev/null |
  sed 's|^\./||' |
  grep -i -F -e "$KHOR_Q" 2>/dev/null |
  head -n "$KHOR_NAMES" |
  sed 's/^/name=/'
grep -rnI -i -F --exclude-dir=.git -e "$KHOR_Q" . 2>/dev/null |
  head -n "$KHOR_LINES" |
  sed 's/^/row=/'
exit 0
"#;

/// [`SEARCH_SH`] with its arguments in front of it.
///
/// **Split out so that verification runs the shipped text.** The one
/// thing a test of a shell script must not do is retype it, and the
/// assignments are as much of the script as the body is — a test that
/// wrote its own `KHOR_DIR=` would be checking a script khor does not
/// send (工作区第七轮 第 55 条). Every script below that takes
/// arguments is composed by a function of its own for the same reason.
fn search_script(dir: &str, query: &str) -> String {
    let names = crate::files::SEARCH_NAMES;
    let lines = crate::files::SEARCH_LINES;
    format!(
        "KHOR_DIR={}\nKHOR_Q={}\nKHOR_NAMES={names}\nKHOR_LINES={lines}\n{SEARCH_SH}",
        sh_quote(dir),
        sh_quote(query),
    )
}

/// [`SEARCH_SH`] over the user's ssh road, in [`crate::files::Found`]'s
/// shape.
pub fn search(host: &Dest, dir: &str, query: &str) -> Result<crate::files::Found, String> {
    Ok(parse_found(
        &sh_over_ssh(host, &search_script(dir, query))?,
        dir,
    ))
}

/// `name=` and `row=` lines into an answer, with the relative paths the
/// far shell printed made absolute again.
///
/// **A row is `path:line:text` and the path is what the `find` printed**
/// — `./src/a.rs` — so the two colons that matter are the first one
/// followed by digits. A path with a colon in it would be read one
/// segment short; it is the one shape this parse cannot tell apart, and
/// it is left that way rather than adding a second output format for a
/// name nobody on a unix machine has typed on purpose.
fn parse_found(text: &str, dir: &str) -> crate::files::Found {
    let root = dir.trim_end_matches('/');
    let absolute = |rel: &str| format!("{root}/{}", rel.trim_start_matches("./"));
    let mut out = crate::files::Found::default();
    for line in text.lines() {
        if let Some(rel) = line.strip_prefix("name=") {
            out.names.push(absolute(rel.trim_end()));
            continue;
        }
        let Some(row) = line.strip_prefix("row=") else {
            continue;
        };
        // The first `:` whose next field is a number: everything before
        // it is the path, everything after that field is the line.
        let mut cut = 0;
        while let Some(at) = row[cut..].find(':') {
            let at = cut + at;
            let rest = &row[at + 1..];
            let Some(end) = rest.find(':') else { break };
            if let Ok(n) = rest[..end].parse::<u32>() {
                out.lines.push(crate::files::Hit {
                    path: absolute(&row[..at]),
                    line: n,
                    text: rest[end + 1..].to_owned(),
                });
                break;
            }
            cut = at + 1;
        }
    }
    // The caps are `head`'s, so a list that came back exactly full is a
    // list that was cut — the no-silent-caps rule, read off the shape
    // of the answer because the far shell has no other way to say it.
    out.more = out.names.len() >= crate::files::SEARCH_NAMES
        || out.lines.len() >= crate::files::SEARCH_LINES;
    out
}

/// [`TOUCHED_SH`] over the user's ssh road, in ms since the epoch —
/// `0` where there was nothing to date.
pub fn touched(host: &Dest, path: &str) -> Result<i64, String> {
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

/// Whether the directory is a checkout, on a satellite: the same one
/// marker `crate::git::is_checkout` reads, looked at by the far shell.
/// `-e` because a linked worktree's `.git` is a file.
const CHECKOUT_SH: &str = r#"
cd "$KHOR_ROOT" || exit 3
if [ -e .git ]; then echo yes=1; else echo yes=0; fi
exit 0
"#;

/// [`CHECKOUT_SH`] over the user's ssh road. **Output that says neither
/// is a refusal**, not a `false`: 「not a repository」 is a claim, and a
/// shell that answered something else did not make it.
pub fn is_checkout(host: &Dest, path: &str) -> Result<bool, String> {
    parse_checkout(&sh_over_ssh_in(host, path, CHECKOUT_SH)?)
}

fn parse_checkout(text: &str) -> Result<bool, String> {
    match text
        .lines()
        .find_map(|l| l.strip_prefix("yes="))
        .map(str::trim)
    {
        Some("1") => Ok(true),
        Some("0") => Ok(false),
        _ => Err(msg::peer_non_answer(text.trim())),
    }
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
pub fn git_status(host: &Dest, root: &str) -> Result<crate::git::Status, String> {
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
        let Some(row) = line.strip_prefix("row=") else {
            continue;
        };
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
        *slot = crate::git::fold(slot, mark);
    }
    let rows = marks.into_iter().map(|(p, m)| (p, m.to_owned())).collect();
    Ok(crate::git::Status {
        branch,
        rows,
        more,
        ahead,
        behind,
        fetched_ms,
    })
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
fn sh_over_ssh_in(host: &Dest, root: &str, script: &str) -> Result<String, String> {
    sh_over_ssh(host, &rooted(root, script))
}

/// That assignment and that script, as one text ([`search_script`]'s
/// reason).
fn rooted(root: &str, script: &str) -> String {
    format!("KHOR_ROOT={}\n{script}", sh_quote(root))
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
pub fn git_diff(host: &Dest, path: &str) -> Result<crate::git::Diff, String> {
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
        let Some(body) = line.strip_prefix("d=") else {
            continue;
        };
        if let Some(head) = body.strip_prefix("@@ ") {
            if hunks.len() >= crate::git::HUNKS || was_bytes >= crate::git::WAS {
                more = true;
                break;
            }
            let Some((at, lines)) = plus_side(head) else {
                continue;
            };
            hunks.push(crate::proto::Hunk {
                at,
                lines,
                was: Vec::new(),
            });
            continue;
        }
        let Some(old) = body.strip_prefix('-') else {
            continue;
        };
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
pub fn sample(host: &Dest) -> Result<khor_core::Vitals, String> {
    let out = ssh_base(host)
        .arg(SAMPLE_SH)
        .output()
        .map_err(msg::ssh_wont_run)?;
    if !out.status.success() {
        return Err(cant_reach(host, &String::from_utf8_lossy(&out.stderr)));
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
        Some(khor_core::Fill {
            used: get(used)? as u64,
            total: get(total)? as u64,
        })
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
            (total > 0 && crate::vitals::keeps_volume(&mount)).then_some(khor_core::Volume {
                mount,
                fill: khor_core::Fill { used, total },
            })
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
        // A satellite runs no khor, and its script does not ask.
        model: None,
        system: None,
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
pub fn ssh_argv(host: &Dest) -> Vec<String> {
    let mut v = vec!["ssh".to_owned()];
    v.extend(host.road());
    v.extend(["-tt".to_owned(), host.host.clone()]);
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
pub fn write_file(host: &Dest, path: &str, want_sha: &str, bytes: &[u8]) -> Result<u64, String> {
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
pub fn ls(host: &Dest, path: &str) -> Result<(String, Vec<crate::proto::DirEntry>, bool), String> {
    let target = if path.is_empty() {
        "\"$HOME\"".to_owned()
    } else {
        sh_quote(path)
    };
    let script = format!(
        r#"cd {target} 2>/dev/null || {{ echo KHOR-NO-DIR; exit 0; }}
pwd -P
find . -mindepth 1 -maxdepth 1 2>/dev/null | while IFS= read -r f; do
  f=${{f#./}}
  if [ -d "$f" ]; then echo "d/$f"; else echo "f $(wc -c <"$f" 2>/dev/null | tr -d ' 	') $f"; fi
done"#
    );
    let out = ssh_base(host)
        .arg(script)
        .output()
        .map_err(msg::ssh_wont_run)?;
    if !out.status.success() {
        return Err(cant_reach(host, &String::from_utf8_lossy(&out.stderr)));
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
            entries.push(crate::proto::DirEntry {
                name: name.to_owned(),
                size: 0,
                dir: true,
                at_ms: 0,
                from: None,
                received_ms: None,
            });
        } else if let Some(rest) = l.strip_prefix("f ")
            && let Some((size, name)) = rest.split_once(' ')
        {
            entries.push(crate::proto::DirEntry {
                name: name.to_owned(),
                size: size.parse().unwrap_or(0),
                dir: false,
                at_ms: 0,
                from: None,
                received_ms: None,
            });
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
pub fn pull_file(host: &Dest, path: &str, dest: &std::path::Path) -> Result<u64, String> {
    let out = Command::new("scp")
        .args(host.road())
        .arg("-q")
        .arg(format!("{}:{path}", host.host))
        .arg(dest)
        .output()
        .map_err(msg::ssh_wont_run)?;
    if !out.status.success() {
        return Err(cant_reach(host, &String::from_utf8_lossy(&out.stderr)));
    }
    std::fs::metadata(dest)
        .map(|m| m.len())
        .map_err(|e| e.to_string())
}

/// Pushes one local file into the satellite's home directory — the
/// sponsor-side half of `khor send <satellite> <file>`.
pub fn push_file(host: &Dest, local: &std::path::Path) -> Result<u64, String> {
    let size = std::fs::metadata(local)
        .map(|m| m.len())
        .map_err(|e| e.to_string())?;
    let out = Command::new("scp")
        .args(host.road())
        .arg("-q")
        .arg(local)
        .arg(format!("{}:", host.host))
        .output()
        .map_err(msg::ssh_wont_run)?;
    if !out.status.success() {
        return Err(cant_reach(host, &String::from_utf8_lossy(&out.stderr)));
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
    if let Some(real) = canonical_hostname(host)
        && real != host
        && let Some(id) = known_host_id(&real)?
    {
        return Ok(id);
    }
    Err(msg::sat_not_in_known_hosts(host))
}

/// One name's known_hosts entry, fingerprinted — `None` when the file
/// has never seen it.
pub(crate) fn known_host_id(name: &str) -> Result<Option<String>, String> {
    let mut find = Command::new("ssh-keygen");
    if let Some(dir) = ssh_dir() {
        find.arg("-f").arg(dir.join("known_hosts"));
    }
    let found = find
        .arg("-F")
        .arg(name)
        .output()
        .map_err(msg::ssh_wont_run)?;
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
            .map_err(msg::ssh_wont_run)?;
        c.stdin
            .take()
            .ok_or_else(|| msg::ssh_wont_run("no stdin"))?
            .write_all(keys.as_bytes())
            .map_err(msg::ssh_wont_run)?;
        let out = c.wait_with_output().map_err(msg::ssh_wont_run)?;
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    // First fingerprint line wins: `256 SHA256:<b64> name (ED25519)`.
    // known_hosts may hold several key types for one host; any of them
    // identifies it, and "the first" is stable because -F preserves
    // file order.
    let Some(b64) = printed.lines().find_map(|l| {
        l.split_whitespace()
            .nth(1)?
            .strip_prefix("SHA256:")
            .map(str::to_owned)
    }) else {
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

    /// **A sponsor's road to its satellite** (round ROAD-1): `ssh` while
    /// the last sample is fresh, `none` once it is not, and through the
    /// sponsor from anywhere else — on a scratch store, no ssh at all.
    #[test]
    fn a_sponsors_road_to_its_satellite_follows_the_last_sample() {
        let dir = std::env::temp_dir().join(format!("khor-sat-road-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let n = crate::Node::open_as(dir.clone(), "sponsor").unwrap();
        let me = n.device_str().to_owned();
        let sat = "ab".repeat(32);
        {
            let loaded = n.devices_loaded().unwrap();
            loaded.doc.upsert(&sat, "box", &[]).unwrap();
            loaded.doc.set_satellite(&sat, &me, "box", None, false).unwrap();
            let mut store = loaded.store;
            store.flush(&loaded.doc).unwrap();
        }
        assert_eq!(n.satellite_road(&sat), None, "never sampled: no reading");
        n.cache_peer_vitals_aged(&sat, &parse_vitals(""), 1_000).unwrap();
        assert_eq!(n.satellite_road(&sat), Some(khor_core::RoadKind::Ssh));
        let mine = n.my_satellite_roads();
        assert_eq!(mine.len(), 1);
        assert_eq!((mine[0].to.as_str(), mine[0].road), (sat.as_str(), khor_core::RoadKind::Ssh));
        n.cache_peer_vitals_aged(&sat, &parse_vitals(""), crate::roadbook::SAT_FRESH.as_millis() as u64 + 1_000)
            .unwrap();
        assert_eq!(n.satellite_road(&sat), Some(khor_core::RoadKind::None), "a stale sample is a broken ssh");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// ssh's own words, as measured on scratch sshds (OpenSSH 10.3),
    /// each read as the segment that broke.
    #[test]
    fn sshs_words_name_the_broken_segment() {
        let via = Dest::new("gpu-box", Some("bastion"));
        let direct = Dest::new("gpu-box", None);
        let jump_down = "ssh: connect to host 127.0.0.1 port 22499: Connection refused\r\nConnection closed by UNKNOWN port 65535\r\n";
        let on_down = "channel 0: open failed: connect failed: Connection refused\r\nstdio forwarding failed\r\nConnection closed by UNKNOWN port 65535\r\n";
        let jump_denies = "zgh@127.0.0.1: Permission denied (publickey,password).\r\nConnection closed by UNKNOWN port 65535\r\n";
        let target_denies = "zgh@127.0.0.1: Permission denied (publickey,password).\r\n";
        assert_eq!(broken(&via, jump_down), Some(Broken::ToJump));
        assert_eq!(broken(&via, on_down), Some(Broken::JumpToTarget));
        assert_eq!(broken(&via, jump_denies), Some(Broken::ToJump));
        assert_eq!(broken(&via, target_denies), Some(Broken::PasswordOnly));
        assert_eq!(broken(&direct, target_denies), Some(Broken::PasswordOnly));
        assert_eq!(broken(&direct, "ssh: connect to host 127.0.0.1 port 22: Connection refused"), None);
        // A banner timeout over a proxy cannot say which side hung.
        assert_eq!(broken(&via, "Connection timed out during banner exchange\r\nConnection to UNKNOWN port 65535 timed out"), None);
        // The jump's refusal is the line quoted, not the noise after it.
        assert!(cant_reach(&via, on_down).contains("channel 0: open failed: connect failed: Connection refused"));
    }

    /// The jump rides every command as an explicit ProxyCommand with
    /// BatchMode (the `-J` spelling would let the jump prompt).
    #[test]
    fn a_jump_is_a_batch_mode_proxy_command() {
        let road = Dest::new("gpu-box", Some("bastion")).road();
        let proxy = road.iter().find(|a| a.starts_with("ProxyCommand=")).expect("a proxy");
        assert!(proxy.contains("-o BatchMode=yes") && proxy.ends_with("-W %h:%p 'bastion'"), "{proxy}");
        assert!(!Dest::new("gpu-box", Some("")).road().iter().any(|a| a.starts_with("ProxyCommand=")));
    }

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
                mem: Some(khor_core::Fill {
                    used: 10,
                    total: 100
                })
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
        assert_eq!(
            names,
            ["NVIDIA RTX A5000", ""],
            "the off-scale card is dropped whole"
        );
        assert_eq!(
            v.gpus[0].mem,
            Some(khor_core::Fill {
                used: 1048576,
                total: 2097152
            })
        );
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
        assert_eq!(
            sha256_b64_to_hex("not-base64!"),
            None,
            "garbage mints nothing"
        );
        let short = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD_NO_PAD.encode([1u8; 16])
        };
        assert_eq!(
            sha256_b64_to_hex(&short),
            None,
            "a wrong-sized digest mints nothing"
        );
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
    /// **The far shell and the local reader answer one question alike**
    /// — a checkout, a linked worktree (its `.git` is a file), a plain
    /// directory — and output that says neither is a refusal.
    #[test]
    fn the_checkout_script_answers_what_the_local_look_answers() {
        struct Scratch(std::path::PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let scratch =
            Scratch(std::env::temp_dir().join(format!("khor-sat-checkout-{}", std::process::id())));
        let tmp = scratch.0.clone();
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("repo/.git")).unwrap();
        std::fs::create_dir_all(tmp.join("linked")).unwrap();
        std::fs::write(tmp.join("linked/.git"), "gitdir: /elsewhere\n").unwrap();
        std::fs::create_dir_all(tmp.join("plain")).unwrap();
        for (dir, want) in [("repo", true), ("linked", true), ("plain", false)] {
            let at = tmp.join(dir);
            let script = format!(
                "KHOR_ROOT={}\n{CHECKOUT_SH}",
                sh_quote(&at.display().to_string())
            );
            let out = std::process::Command::new("sh")
                .arg("-c")
                .arg(&script)
                .output()
                .expect("sh runs");
            let said = parse_checkout(&String::from_utf8_lossy(&out.stdout));
            assert_eq!(said, Ok(want), "{dir}: the script");
            assert_eq!(crate::git::is_checkout(&at), want, "{dir}: the local look");
        }
        assert!(parse_checkout("sh: cd: nope").is_err());
    }

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
        let scratch =
            Scratch(std::env::temp_dir().join(format!("khor-sat-touched-{}", std::process::id())));
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

        let script = format!(
            "KHOR_ROOT={}\n{TOUCHED_SH}",
            sh_quote(&tmp.display().to_string())
        );
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("sh runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let said = parse_touched(&String::from_utf8_lossy(&out.stdout));

        // To the second, because the far side's `stat` answers seconds
        // and the local walk answers milliseconds.
        let here = crate::workspace::touched(&tmp) / 1000;
        assert_eq!(
            said / 1000,
            here,
            "the script and the local walk must date one tree alike"
        );
        assert_eq!(here, 2_000, "…and both must have skipped the build output");

        let empty = tmp.join("src/deep/nothing");
        std::fs::create_dir_all(&empty).unwrap();
        let script = format!(
            "KHOR_ROOT={}\n{TOUCHED_SH}",
            sh_quote(&empty.display().to_string())
        );
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .output()
            .unwrap();
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
        let got: Vec<(&str, &str)> = st
            .rows
            .iter()
            .map(|(p, m)| (p.as_str(), m.as_str()))
            .collect();
        // Checked against `khor git status` on the same checkout, which
        // reads it in-process through gix: the two printed the same
        // eight lines byte for byte.
        assert_eq!(
            got,
            [
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
            ]
        );
    }

    #[test]
    fn an_ignored_path_is_never_folded_into_a_change() {
        // `fold` runs on every row, first insert included, so an
        // ignored path folded against itself has to come back ignored —
        // the loop's fall-through is `MODIFIED`, and that reading
        // painted every build directory as an edit nobody made.
        assert_eq!(
            crate::git::fold(crate::git::IGNORED, crate::git::IGNORED),
            crate::git::IGNORED
        );
        // …and it gives way to anything git is actually looking at: a
        // directory holding an edited file and a build output is 黄.
        assert_eq!(
            crate::git::fold(crate::git::IGNORED, crate::git::MODIFIED),
            crate::git::MODIFIED
        );
        assert_eq!(
            crate::git::fold(crate::git::CONFLICT, crate::git::IGNORED),
            crate::git::CONFLICT
        );
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
        let st = parse_status("branch=main\nbehind=2\nahead=5\nfetched=1789274278\nrow= M a.txt\n")
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
        let shape: Vec<(u32, u32, usize)> = d
            .hunks
            .iter()
            .map(|h| (h.at, h.lines, h.was.len()))
            .collect();
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
        assert!(
            d.hunks
                .iter()
                .all(|h| h.was.iter().all(|w| !w.starts_with("--")))
        );
    }

    #[test]
    fn a_file_git_says_nothing_about_has_no_hunks() {
        assert!(parse_diff("").hunks.is_empty());
        assert!(
            parse_diff("d=fatal: not a git repository\n")
                .hunks
                .is_empty()
        );
    }

    /// What [`SEARCH_SH`] prints, parsed back into the same shape khor's
    /// own walk answers (工作区第六轮 第 47 条). **The two colons are
    /// the fragile part**: `grep -n` writes `path:line:text` and the
    /// text may hold colons of its own, so the line number is the first
    /// field that parses as one — and a path printed relative by the
    /// far shell comes back absolute.
    #[test]
    fn a_satellites_grep_reads_back_as_names_and_lines() {
        let text = "\
name=./src/one.rs
name=./README.md
row=./src/one.rs:2:    let keeper = 1;
row=./notes.md:11:see http://example.test/x for more
";
        let found = parse_found(text, "/home/peer/w/paper/");
        assert_eq!(
            found.names,
            vec![
                "/home/peer/w/paper/src/one.rs".to_owned(),
                "/home/peer/w/paper/README.md".to_owned(),
            ]
        );
        assert_eq!(
            found.lines,
            vec![
                crate::files::Hit {
                    path: "/home/peer/w/paper/src/one.rs".to_owned(),
                    line: 2,
                    text: "    let keeper = 1;".to_owned(),
                },
                crate::files::Hit {
                    path: "/home/peer/w/paper/notes.md".to_owned(),
                    line: 11,
                    // The address keeps both of its colons: only the
                    // first field that is a number is the line.
                    text: "see http://example.test/x for more".to_owned(),
                },
            ]
        );
        assert!(!found.more, "neither list reached its cap");
    }

    /// A list that came back exactly full was cut by `head`, and the
    /// far shell has no other way to say so — the no-silent-caps rule
    /// read off the shape of the answer.
    #[test]
    fn a_full_list_is_reported_as_cut() {
        let mut text = String::new();
        for n in 0..crate::files::SEARCH_LINES {
            text.push_str(&format!("row=./a.txt:{}:keeper\n", n + 1));
        }
        let found = parse_found(&text, "/w");
        assert_eq!(found.lines.len(), crate::files::SEARCH_LINES);
        assert!(found.more);
    }
}

/// The five scripts, run by the local `sh` (工作区第七轮 第 55 条).
///
/// # Why this is verification and not a rehearsal
///
/// Every one of these scripts had been read and none had been run. The
/// ssh half is verified — `sh_over_ssh` has carried `git status` since
/// the satellite workspace batch — so what was left unchecked is the
/// text itself: a quoting slip, a `head` in the wrong pipe, an `exit`
/// that answers `no` where the far machine said `taken`. A satellite is
/// not needed to find any of those. **The same script and the same
/// assignments** go into the local shell (`search_script`, `rooted`,
/// the two beside them) — the composition is shared code rather than
/// retyped here, which is the one thing a test of a shell script must
/// not do.
///
/// What this does **not** verify is the road: `ssh`, the far machine's
/// `sh` being a different `sh`, a `find` without `-not -path`. Those
/// stay in the 还欠什么 column where they were.
#[cfg(test)]
mod script_tests {
    use super::*;

    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;

        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Scratch {
        /// That path as the scripts take it: absolute, in this
        /// machine's own spelling.
        fn at(&self, rel: &str) -> String {
            self.0.join(rel).to_str().expect("utf-8 scratch").to_owned()
        }

        fn spelled(&self) -> String {
            self.0.to_str().expect("utf-8 scratch").to_owned()
        }
    }

    fn scratch(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-sat-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("a scratch directory");
        Scratch(p)
    }

    /// One script down a local `sh`'s stdin, its stdout back — the
    /// shape [`sh_over_ssh`] has with the `ssh` taken out of it.
    ///
    /// **Never the user's own git config.** `git init` and `git grep`
    /// below read whatever `~/.gitconfig` says, and a person with a
    /// `core.excludesFile` of their own would make a green run into a
    /// red one on somebody else's machine. The two overrides point the
    /// child at nothing; the user's files are neither read nor written
    /// (the ruling's own 别碰用户的 git 配置).
    fn run_here(script: &str, path: Option<&str>) -> String {
        use std::io::Write;
        // **`/bin/sh` by path, because `PATH` is one of the things
        // under test**: the opener case below hands the script a
        // `PATH` of its own, and a shell looked up through that `PATH`
        // is a shell that cannot be found.
        let mut cmd = Command::new("/bin/sh");
        cmd.env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if let Some(p) = path {
            cmd.env("PATH", p);
        }
        let mut child = cmd.spawn().expect("a shell on this machine");
        child
            .stdin
            .take()
            .expect("the shell's stdin")
            .write_all(script.as_bytes())
            .expect("the script down the pipe");
        let out = child.wait_with_output().expect("the shell to finish");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// The first word the script printed — what every one of the three
    /// name verbs reads (`mkdir`, `touch`, `rename_path` all take the
    /// first non-empty line).
    fn said(text: &str) -> String {
        text.lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("")
            .to_owned()
    }

    /// `done` and the directory is there; `taken` and nothing was
    /// touched. **The second half is the one worth a test**: `mkdir`
    /// without `-p` refuses an existing name on its own, and the script
    /// exists to tell that refusal apart from every other one.
    #[test]
    fn mkdir_makes_a_name_and_refuses_one_that_is_taken() {
        let dir = scratch("mkdir");
        let fresh = dir.at("made");
        assert_eq!(said(&run_here(&rooted(&fresh, MKDIR_SH), None)), "done");
        assert!(std::path::Path::new(&fresh).is_dir());
        assert_eq!(said(&run_here(&rooted(&fresh, MKDIR_SH), None)), "taken");

        // A file's name is taken too — the test is on the name, not on
        // the kind, which is what `[ -e ]` says and what a person
        // retyping a name needs to hear.
        let file = dir.at("already.txt");
        std::fs::write(&file, "x").unwrap();
        assert_eq!(said(&run_here(&rooted(&file, MKDIR_SH), None)), "taken");

        // …and a missing parent is the shell's own `no`, which khor
        // turns into 「做不了」 rather than 「名字已经有了」.
        let deep = dir.at("nowhere/under/it");
        assert_eq!(said(&run_here(&rooted(&deep, MKDIR_SH), None)), "no");
        assert!(!std::path::Path::new(&deep).exists());
    }

    /// `done` and the file is there and empty; `taken` and the bytes
    /// that were in it are still in it — **the half that matters**,
    /// because `: >` truncates, and a touch that emptied somebody's
    /// file would be the worst answer this script could give.
    #[test]
    fn touch_makes_an_empty_file_and_never_empties_one() {
        let dir = scratch("touch");
        let fresh = dir.at("new.txt");
        assert_eq!(said(&run_here(&rooted(&fresh, TOUCH_SH), None)), "done");
        assert_eq!(std::fs::read_to_string(&fresh).unwrap(), "");

        std::fs::write(&fresh, "somebody's work\n").unwrap();
        assert_eq!(said(&run_here(&rooted(&fresh, TOUCH_SH), None)), "taken");
        assert_eq!(
            std::fs::read_to_string(&fresh).unwrap(),
            "somebody's work\n",
            "the refusal has to happen before the truncation, not after"
        );
    }

    /// Moves a name; refuses to replace one; says `gone` when there was
    /// nothing to move. **`mv` on unix replaces without a word**, which
    /// is the whole reason the target is tested first.
    #[test]
    fn mv_moves_a_name_and_refuses_to_replace_one() {
        let dir = scratch("mv");
        let from = dir.at("one.txt");
        let to = dir.at("two.txt");
        std::fs::write(&from, "keeper\n").unwrap();
        assert_eq!(said(&run_here(&rename_script(&from, &to), None)), "done");
        assert!(!std::path::Path::new(&from).exists());
        assert_eq!(std::fs::read_to_string(&to).unwrap(), "keeper\n");

        // The source is gone now, and that is its own answer.
        assert_eq!(said(&run_here(&rename_script(&from, &to), None)), "gone");

        // …and a target that is there is refused with the bytes it had.
        let other = dir.at("three.txt");
        std::fs::write(&other, "mine\n").unwrap();
        assert_eq!(said(&run_here(&rename_script(&to, &other), None)), "taken");
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "mine\n");
        assert_eq!(std::fs::read_to_string(&to).unwrap(), "keeper\n");
    }

    /// `done` where the desktop has an opener, `none` where it has not.
    ///
    /// **The opener is `/usr/bin/true` on a `PATH` of this test's own**
    /// — 绝不真弹窗. The script looks the two openers up with
    /// `command -v`, so a directory holding one file named `open` is
    /// the whole of a desktop as far as it is concerned, and an empty
    /// one is a headless satellite. (`KHOR_LAUNCHER` is deliberately
    /// not consulted by this script — see [`LAUNCH_SH`] — so the
    /// environment variable is not what is set here; the opener it
    /// names is.)
    #[test]
    fn launch_answers_done_with_an_opener_and_none_without_one() {
        let dir = scratch("launch");
        let bin = dir.0.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let truth = std::path::Path::new("/usr/bin/true");
        assert!(truth.exists(), "this machine has no /usr/bin/true");
        std::os::unix::fs::symlink(truth, bin.join("open")).unwrap();
        let bare = dir.0.join("bare");
        std::fs::create_dir_all(&bare).unwrap();

        let target = dir.at("a.txt");
        std::fs::write(&target, "x").unwrap();
        assert_eq!(
            said(&run_here(
                &launch_script(&target),
                Some(bin.to_str().unwrap())
            )),
            "done"
        );
        assert_eq!(
            said(&run_here(
                &launch_script(&target),
                Some(bare.to_str().unwrap())
            )),
            "none",
            "a machine with no screen has no opener, and that is a sentence khor prints"
        );
    }

    /// A little project, the same shape `files::search`'s own tests
    /// use: a readme, a source file, a build output that `.gitignore`
    /// covers, and a file of bytes.
    fn project(tag: &str) -> Scratch {
        let dir = scratch(tag);
        std::fs::create_dir_all(dir.0.join("src")).unwrap();
        std::fs::create_dir_all(dir.0.join("build")).unwrap();
        std::fs::write(dir.0.join(".gitignore"), "build/\n").unwrap();
        std::fs::write(
            dir.0.join("README.md"),
            "# a project\nit has a keeper in it\n",
        )
        .unwrap();
        std::fs::write(
            dir.0.join("src/one.rs"),
            "fn main() {\n    let keeper = 1;\n}\n",
        )
        .unwrap();
        std::fs::write(dir.0.join("build/built.rs"), "let keeper = 2;\n").unwrap();
        std::fs::write(dir.0.join("src/shot.png"), [0x89, b'P', 0, b'N', b'G']).unwrap();
        dir
    }

    /// Outside a checkout: `find` and `grep -rI`, and the output parses
    /// back into the shape khor's own walk answers.
    ///
    /// **`build/built.rs` is in this answer and that is right**: there
    /// is no git here to ask, and the `.gitignore` sitting beside it is
    /// a file the shell has no rules for. The next test is the one
    /// where it goes away.
    #[test]
    fn search_outside_a_checkout_walks_with_find_and_grep() {
        let dir = project("plain");
        let out = run_here(&search_script(&dir.spelled(), "keeper"), None);
        let found = parse_found(&out, &dir.spelled());
        let rows: Vec<(String, u32)> = found
            .lines
            .iter()
            .map(|h| (under(&dir, &h.path), h.line))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("README.md".to_owned(), 2),
                ("build/built.rs".to_owned(), 1),
                ("src/one.rs".to_owned(), 2),
            ],
            "no git to ask, so the ignored copy is in the answer: {out}"
        );
        assert!(
            found.names.is_empty(),
            "no path here holds the word: {:?}",
            found.names
        );

        // …and the name half, which is the other list.
        let named = run_here(&search_script(&dir.spelled(), "one.rs"), None);
        let hit = parse_found(&named, &dir.spelled());
        assert_eq!(
            hit.names.iter().map(|p| under(&dir, p)).collect::<Vec<_>>(),
            vec!["src/one.rs".to_owned()],
            "{named}"
        );
        assert!(hit.lines.is_empty(), "{:?}", hit.lines);
    }

    /// In a checkout: git's own exclude rules, which is what
    /// 「尊重 `.gitignore`」 means on this road (第 55 条).
    #[test]
    fn search_in_a_checkout_leaves_out_what_gitignore_covers() {
        let dir = project("repo");
        git_here(&dir);
        let out = run_here(&search_script(&dir.spelled(), "keeper"), None);
        let found = parse_found(&out, &dir.spelled());
        let rows: Vec<(String, u32)> = found
            .lines
            .iter()
            .map(|h| (under(&dir, &h.path), h.line))
            .collect();
        assert_eq!(
            rows,
            vec![("README.md".to_owned(), 2), ("src/one.rs".to_owned(), 2)],
            "the copy under build/ is the one that must not be here: {out}"
        );

        // The name half honours the same rules — `git ls-files -co
        // --exclude-standard` is the one list, asked twice.
        let named = run_here(&search_script(&dir.spelled(), "built"), None);
        assert!(
            parse_found(&named, &dir.spelled()).names.is_empty(),
            "an ignored name came back by its name: {named}"
        );
    }

    /// The name half matches **the path under the directory**, never
    /// the whole of it (工作区第七轮 第 56 条) — the same claim
    /// `files::search`'s own test makes about the local walk, made here
    /// because the two roads have to answer one table.
    #[test]
    fn search_names_are_matched_relative_to_the_directory() {
        let dir = project("under");
        // The scratch path itself holds this word, because it is built
        // out of it: `khor-sat-under-<pid>`. Nothing **inside** the
        // project does.
        let out = run_here(&search_script(&dir.spelled(), "khor-sat"), None);
        let found = parse_found(&out, &dir.spelled());
        assert!(
            found.names.is_empty(),
            "the directories above a project are not part of what is being searched: {:?}",
            found.names
        );
        // **`./` is not part of a relative path either**, which is the
        // half of this that `find` can get wrong: it prints its walk
        // led by the directory it was handed, and two characters
        // nobody typed are two characters every query is matched
        // against.
        let led = run_here(&search_script(&dir.spelled(), "./"), None);
        assert!(
            parse_found(&led, &dir.spelled()).names.is_empty(),
            "the walk's own lead came back as a name hit: {led}"
        );

        // …and the same in a checkout, where git prints the relative
        // path and the `./` cut does not apply.
        git_here(&dir);
        let again = run_here(&search_script(&dir.spelled(), "khor-sat"), None);
        assert!(
            parse_found(&again, &dir.spelled()).names.is_empty(),
            "{again}"
        );
        let git_led = run_here(&search_script(&dir.spelled(), "./"), None);
        assert!(
            parse_found(&git_led, &dir.spelled()).names.is_empty(),
            "{git_led}"
        );
    }

    /// `git init` in a scratch directory, with the identity given on
    /// the command line. **The user's own git config is never read or
    /// written** (`run_here` points the child at nothing, and this
    /// carries the same two overrides).
    fn git_here(dir: &Scratch) {
        let out = Command::new("git")
            .current_dir(&dir.0)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "init",
                "--quiet",
            ])
            .output()
            .expect("git on this machine");
        assert!(
            out.status.success(),
            "git init: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// One answered path, spelled relative to the scratch directory —
    /// the absolute half is this machine's temp directory and says
    /// nothing about the script.
    fn under(dir: &Scratch, path: &str) -> String {
        path.trim_start_matches(&dir.spelled())
            .trim_start_matches('/')
            .to_owned()
    }
}
