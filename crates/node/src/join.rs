//! Adding a machine over ssh (添加机器; docs/NET.md 卫星 and 转正).
//!
//! One verb, two outcomes. By default the machine becomes a **full
//! member**: khor is put on it, its resident started at user level, and
//! it pairs with this machine — all from here, over the user's own ssh
//! road, the satellite module's rules (BatchMode, nothing stored, the
//! jump as `ProxyCommand`). When it cannot stay a member — its bin
//! directory is read-only, its resident does not outlive the session,
//! or it cannot call back over iroh in [`PAIR_WAIT`] — it can be adopted
//! as a **satellite** instead. Verified installation and node identity remain
//! available for retry; a failed pairing never deletes shared install files.
//! A flag skips installation and adds the SSH host directly.
//!
//! # What runs over there is `install.sh`, the one every install runs
//!
//! The script travels on ssh's stdin (`sh -s`), so nothing is written
//! there that the install itself would not write. Two passes, because
//! the person watches two steps: 放 khor (`KHOR_NO_SERVE`), then 启动
//! (`KHOR_SKIP_FETCH`). The version is this machine's own
//! ([`crate::VERSION`]), so a network never grows a second khor by
//! being added to; `KHOR_MIRROR` set here is passed along. When the far
//! machine cannot fetch, this one fetches the target platform's same published
//! archive, verifies it, and pushes the exact bytes down the SSH channel.
//!
//! # What is never done
//!
//! No root is asked for, the user's `~/.ssh` is never written, the
//! pairing ticket travels on the pair command's stdin and is never
//! printed, logged, or put on a command line (a command line is in
//! `ps` on both machines).
use std::io::Read;
#[cfg(test)]
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use khor_catalog::{cli, msg};
use sha2::{Digest, Sha256};

use crate::Node;
use crate::satellite::{self, Dest};

/// The install script, as shipped: what runs on the far machine.
const INSTALL_SH: &str = include_str!("../../../scripts/install.sh");

// The receiving shell owns its stage until it publishes the exact path.
// An installer consumes only that stage; concurrent pushes never share a file.
const PUSH_ARCHIVE_SH: &str = r#"
set -eu
B="$1"
umask 077
mkdir -p "$B"
S=$(mktemp "$B/.khor.push.XXXXXXXX")
trap 'rm -f "$S"' EXIT
cat > "$S"
printf 'KHOR_PUSH_ASSET_V1\t%s\n' "$S"
trap - EXIT
"#;

struct PushedArchive {
    path: String,
    digest: String,
}

fn pushed_archive_path(receipt: &str) -> Result<String, String> {
    let path = receipt
        .strip_prefix("KHOR_PUSH_ASSET_V1\t")
        .and_then(|value| value.strip_suffix('\n'))
        .filter(|value| {
            value.starts_with('/') && value.len() <= 8192 && !value.contains(['\n', '\r', '\0'])
        })
        .ok_or("remote archive transfer did not return one exact path")?;
    let valid_name = path
        .rsplit('/')
        .next()
        .and_then(|name| name.strip_prefix(".khor.push."))
        .is_some_and(|nonce| {
            nonce.len() == 8 && nonce.bytes().all(|byte| byte.is_ascii_alphanumeric())
        });
    if !valid_name {
        return Err("remote archive transfer returned an invalid stage path".into());
    }
    Ok(path.to_owned())
}

/// How long the new machine has to call back over iroh and pair. Past
/// it the machine is one khor cannot talk to directly, which is what a
/// satellite is for.
pub const PAIR_WAIT: Duration = Duration::from_secs(60);

/// How long the far side may take to say whether it reaches a relay
/// (round ROAD-4): ssh's login, then one GET to each relay at once,
/// each inside `doctor`'s three seconds.
pub const RELAY_ASK: Duration = Duration::from_secs(10);
const DOWNLOAD_UNAVAILABLE: i32 = 75;
const MAX_ARCHIVE_BYTES: usize = 512 * 1024 * 1024;

/// **`khor _relay-check`, the far side of [`Node::far_reaches_a_relay`]**:
/// whether this machine reaches any of `urls`, by the same GET and the
/// same proxy rule as iroh's own relay connection
/// (`doctor::any_relay_answers`).
pub async fn reaches_a_relay(urls: Vec<String>) -> bool {
    crate::doctor::any_relay_answers(urls).await
}

/// The steps the person watches, in order (the GUI draws one line each).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Connect,
    Place,
    Start,
    Pair,
    /// Reserved wire value from the former destructive rollback path.
    Undo,
    Satellite,
}

/// Where a step is: under way, done (with what it did), or failed (why).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    Live,
    Done(String),
    Failed(String),
    /// The step failed because the target would not let this machine
    /// log in: the sentence, and which refusal it was (the face offers
    /// the way on — upload a key, a password, a passphrase).
    Refused(satellite::Refusal, String),
}

/// What an add ended as.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Added {
    pub id: String,
    pub name: String,
    /// A full member; `false` = a satellite of this machine.
    pub member: bool,
    /// Why it became a satellite when a member was asked for — one
    /// sentence, `None` when it is a member or a satellite was asked for.
    pub why: Option<String>,
    /// For a member: what brings its resident back after a reboot, as
    /// a sentence (it outlives logging out in every case it is kept).
    pub keeper: Option<String>,
    /// For a satellite: whether tmux is there (a terminal on it can be
    /// kept).
    pub tmux: bool,
}

/// An add another machine asked this one to run (通过 khor 主机;
/// `proto::Request::AddOverSsh`): its marks so far and, once it is over,
/// how it ended. Read by token, like a drop's job.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Run {
    pub marks: Vec<(Step, Mark)>,
    pub done: Option<Result<Added, String>>,
}

/// The runs this machine holds, by token. A finished run is kept for a
/// few more asks (the asker's last read), then the oldest go.
static RUNS: LazyLock<Mutex<Vec<(String, Arc<Mutex<Run>>)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// How many finished runs are kept for their askers.
const RUNS_KEPT: usize = 8;

/// The run `token` names, and whether this ask is the one that makes it.
pub(crate) fn run_for(token: &str) -> (Arc<Mutex<Run>>, bool) {
    let mut runs = RUNS.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((_, r)) = runs.iter().find(|(t, _)| t == token) {
        return (r.clone(), false);
    }
    let finished = runs
        .iter()
        .filter(|(_, r)| r.lock().unwrap_or_else(|p| p.into_inner()).done.is_some())
        .count();
    if finished >= RUNS_KEPT
        && let Some(i) = runs
            .iter()
            .position(|(_, r)| r.lock().unwrap_or_else(|p| p.into_inner()).done.is_some())
    {
        runs.remove(i);
    }
    let r = Arc::new(Mutex::new(Run::default()));
    runs.push((token.to_owned(), r.clone()));
    (r, true)
}

/// Starts `to`'s add on a thread of its own as the run `run` (this
/// machine the sponsor), so it outlives the ask that started it.
pub(crate) fn start(
    node: Arc<Node>,
    run: Arc<Mutex<Run>>,
    to: Dest,
    name: Option<String>,
    satellite_only: bool,
) {
    std::thread::spawn(move || {
        let out = node.add_over_ssh(&to, name.as_deref(), satellite_only, None, &mut |s, m| {
            run.lock()
                .unwrap_or_else(|p| p.into_inner())
                .marks
                .push((s, m));
        });
        run.lock().unwrap_or_else(|p| p.into_inner()).done = Some(out);
    });
}

fn refused_or_failed(refused: Option<satellite::Refusal>, why: &str) -> Mark {
    match refused {
        Some(r) => Mark::Refused(r, why.to_owned()),
        None => Mark::Failed(why.to_owned()),
    }
}

/// Platform and installation capabilities observed over one connection.
#[derive(Debug, Default)]
struct Look {
    asset: Option<&'static str>,
    tmux: bool,
    bin_ok: bool,
}

const LOOK_SH: &str = r#"
B="$HOME/.local/bin"
uname -s; uname -m
command -v tmux >/dev/null 2>&1 && echo tmux
if [ -d "$B" ]; then [ -w "$B" ] && echo bin-ok
else P="$HOME/.local"; [ -d "$P" ] || P="$HOME"; [ -w "$P" ] && echo bin-ok; fi
exit 0
"#;

/// Is the resident still up now that the session that started it has
/// closed — and which machine is it.
const CHECK_SH: &str = r#"
B="$HOME/.local/bin"
R=$(env -u KHOR_HOME "$B/khor" root 2>/dev/null) || R="$HOME"
P=$(cat "$R/.khor/serve.pid" 2>/dev/null)
EP="$R/.khor/endpoint.json"
if [ -n "$P" ] && kill -0 "$P" 2>/dev/null; then echo alive
elif [ -f "$EP" ] && grep -q '"app":true' "$EP"; then echo alive
else echo dead; fi
env -u KHOR_HOME "$B/khor" id 2>/dev/null | head -1
"#;

/// The release asset a machine runs, from its `uname -s` and `-m` —
/// install.sh's table, which is the one that decides.
fn asset_for(system: &str, machine: &str) -> Option<&'static str> {
    Some(match (system, machine) {
        ("Linux", "x86_64") => "khor-linux-x86_64",
        ("Linux", "aarch64" | "arm64") => "khor-linux-aarch64",
        ("Darwin", "arm64") => "khor-macos-arm64",
        ("Darwin", "x86_64") => "khor-macos-x86_64",
        _ => return None,
    })
}

#[path = "join_process.rs"]
mod process;

/// One finite local SSH command, including input and concurrent output drains.
/// Confirming local cleanup does not confirm remote execution has stopped.
fn run(
    to: &Dest,
    command: &str,
    stdin: &[u8],
    wait: Duration,
) -> Result<(bool, String, String), String> {
    let (code, stdout, stderr) = run_exit(to, command, stdin, wait)?;
    Ok((code == Some(0), stdout, stderr))
}

fn run_exit(
    to: &Dest,
    command: &str,
    stdin: &[u8],
    wait: Duration,
) -> Result<(Option<i32>, String, String), String> {
    let mut command_line = to.finite_ssh_command();
    command_line
        .args(["-o", "ConnectTimeout=8"])
        .arg(&to.host)
        .arg(command);
    let out = process::run(command_line, stdin, wait).map_err(msg::ssh_wont_run)?;
    let mut stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    if out.stderr_truncated {
        stderr.insert_str(0, "[SSH stderr truncated to its last 262144 bytes]\n");
    }
    if out.timed_out {
        if !stderr.is_empty() && !stderr.ends_with('\n') {
            stderr.push('\n');
        }
        stderr.push_str(&msg::add_timed_out(wait.as_secs()));
        return Ok((None, String::new(), stderr));
    }
    Ok((
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr,
    ))
}

#[cfg(all(test, unix))]
#[path = "join_process_tests.rs"]
mod process_tests;

fn archive_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn published_archive_digest(record: &[u8], asset: &str) -> Result<String, String> {
    let line = record.strip_suffix(b"\n").unwrap_or(record);
    let name = format!("{asset}.gz");
    if record.len() > 512
        || line.len() != 66 + name.len()
        || !line[..64]
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        || &line[64..66] != b"  "
        || &line[66..] != name.as_bytes()
    {
        return Err(
            "release checksum must contain one digest for the exact archive basename".into(),
        );
    }
    Ok(line[..64].iter().map(|byte| char::from(*byte)).collect())
}

fn release_asset_url(
    mirror: Option<&str>,
    repo: Option<&str>,
    version: &str,
    asset: &str,
) -> Result<String, String> {
    if !matches!(
        asset,
        "khor-linux-x86_64" | "khor-linux-aarch64" | "khor-macos-arm64" | "khor-macos-x86_64"
    ) || version.is_empty()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-+".contains(&byte))
    {
        return Err("invalid release asset or version".into());
    }
    if let Some(mirror) = mirror.filter(|value| !value.is_empty()) {
        let url =
            reqwest::Url::parse(mirror).map_err(|_| "invalid release mirror URL".to_owned())?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !mirror
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b":/._~+[]-".contains(&byte))
        {
            return Err("invalid release mirror URL".into());
        }
        return Ok(format!(
            "{}/releases/v{version}/{asset}.gz",
            url.as_str().trim_end_matches('/')
        ));
    }
    let repo = repo
        .filter(|value| !value.is_empty())
        .unwrap_or("colehank/khor");
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty()
                || part.len() > 100
                || matches!(*part, "." | "..")
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        })
    {
        return Err("invalid release repository".into());
    }
    Ok(format!(
        "https://github.com/{repo}/releases/download/v{version}/{asset}.gz"
    ))
}

fn fetch_archive_bytes(url: &str, limit: Option<usize>) -> Result<Vec<u8>, String> {
    let mut command = Command::new("curl");
    command.args(["--disable", "-fsSL", "--connect-timeout", "15", url]);
    command.args(["--max-time", if limit.is_some() { "30" } else { "300" }]);
    read_archive_output(command, Some(limit.unwrap_or(MAX_ARCHIVE_BYTES)))
}

fn read_archive_output(mut command: Command, limit: Option<usize>) -> Result<Vec<u8>, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let Some(mut stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("release download has no output pipe".into());
    };
    let mut bytes = Vec::new();
    let read = match limit {
        Some(limit) => stdout.take((limit + 1) as u64).read_to_end(&mut bytes),
        None => stdout.read_to_end(&mut bytes),
    };
    if read.is_err() || limit.is_some_and(|limit| bytes.len() > limit) {
        let _ = child.kill();
        let _ = child.wait();
        return Err("release download exceeds its byte limit or could not be read".into());
    }
    if !child.wait().map_err(|error| error.to_string())?.success() {
        return Err("could not fetch the selected release archive or checksum".into());
    }
    Ok(bytes)
}

/// The environment the install script is started under over there:
/// this machine's version, and its mirror when it has one. Values are
/// single-quoted; the names are ours.
fn install_env(extra: &str) -> String {
    let mut e = format!("KHOR_VERSION='v{}'", crate::VERSION);
    for name in ["KHOR_MIRROR", "KHOR_REPO"] {
        if let Ok(value) = std::env::var(name)
            && !value.is_empty()
        {
            e.push_str(&format!(" {name}={}", install_quoted_value(&value)));
        }
    }
    format!("{e} {extra} sh -s")
}

fn install_quoted_value(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn last_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .next_back()
        .unwrap_or("")
        .to_owned()
}

impl Node {
    /// Adds the machine at `to` — a full member unless `satellite_only`,
    /// a satellite when it cannot be one (the module head). `name` is
    /// what the row is called; the host when `None`. `secret`, when
    /// given, is kept in this machine's secret store and ssh asks for it
    /// ([`crate::secret`]): the password, or — when `to` logs in with a
    /// key file ([`Dest::identity`]) — that key's passphrase. `on` hears every step as it starts
    /// and ends. Errors are sentences; one that is a password-only door
    /// reads as such ([`satellite::broken`]).
    ///
    /// **The secret stays only while something needs it**: a
    /// satellite is reached over ssh for as long as it is one, so its
    /// row says `ssh_password` / `ssh_passphrase` and the secret goes
    /// when the row does (`Node::forget_id`, and the sponsor's sweep for
    /// a row removed elsewhere, [`crate::secret::sweep`]); a member is
    /// reached over iroh, so the secret is dropped the moment the add
    /// succeeds, and so is one whose add failed — unless this machine
    /// now carries the member over that same ssh (round ROAD-4), which
    /// keeps it as a satellite's is kept.
    pub fn add_over_ssh(
        &self,
        to: &Dest,
        name: Option<&str>,
        satellite_only: bool,
        secret: Option<&str>,
        on: &mut dyn FnMut(Step, Mark),
    ) -> Result<Added, String> {
        let Some(secret) = secret else {
            return self.add_road(to, name, satellite_only, on);
        };
        let to = if to.identity.is_some() {
            to.clone().with_passphrase(true)
        } else {
            to.clone().with_password(true)
        };
        let key = to.secret_key();
        if let Err(e) = crate::secret::keep(self.root(), &key, secret, crate::now_ms()) {
            on(Step::Connect, Mark::Failed(e.clone()));
            return Err(e);
        }
        let added = self.add_road(&to, name, satellite_only, on);
        let logs_in = added.as_ref().is_ok_and(|a| {
            !a.member
                || self
                    .devices()
                    .is_ok_and(|all| all.iter().any(|d| d.id == a.id && self.logs_in_to(d)))
        });
        if !logs_in {
            crate::secret::drop_kept(self.root(), &key)?;
        }
        added
    }

    fn add_road(
        &self,
        to: &Dest,
        name: Option<&str>,
        satellite_only: bool,
        on: &mut dyn FnMut(Step, Mark),
    ) -> Result<Added, String> {
        let name = name.unwrap_or(&to.host).to_owned();
        on(Step::Connect, Mark::Live);
        let look = match self.look(to) {
            Ok(l) => l,
            Err((e, refused)) => {
                on(Step::Connect, refused_or_failed(refused, &e));
                return Err(e);
            }
        };
        on(Step::Connect, Mark::Done(String::new()));
        if satellite_only {
            return self.add_satellite(to, &name, None, on);
        }
        match self.become_member(to, &name, &look, on) {
            Ok(added) => Ok(added),
            Err(Stop::Fatal(e)) => Err(e),
            Err(Stop::Fallback(why)) => self.add_satellite(to, &name, Some(why), on),
        }
    }

    /// 转正: a satellite this machine sponsors becomes a full member
    /// through the same install; its satellite row goes and its name
    /// carries over to the new machine. When it cannot stay a member it
    /// stays the satellite it was; verified installation files remain reusable.
    pub fn promote(&self, machine: &str, on: &mut dyn FnMut(Step, Mark)) -> Result<Added, String> {
        let (channel, _) = self.resolve(machine)?;
        let row = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::not_my_satellite(&channel))?;
        if row.sponsor.as_deref() != Some(self.device_str()) {
            return Err(msg::not_my_satellite(&channel));
        }
        let to = Dest::of(&row).ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
        let name = row.shown().to_owned();
        on(Step::Connect, Mark::Live);
        let look = self.look(&to).map_err(|(e, refused)| {
            on(Step::Connect, refused_or_failed(refused, &e));
            e
        })?;
        on(Step::Connect, Mark::Done(String::new()));
        match self.become_member(&to, &name, &look, on) {
            Ok(added) => {
                self.forget_id(&row.id)?;
                Ok(added)
            }
            Err(Stop::Fatal(e)) => Err(e),
            Err(Stop::Fallback(why)) => Err(msg::promote_stayed_satellite(why)),
        }
    }

    /// The first look, over one connection; a failure says whether it
    /// was a refused login ([`satellite::refusal`]).
    fn look(&self, to: &Dest) -> Result<Look, (String, Option<satellite::Refusal>)> {
        let (ok, out, err) =
            run(to, "sh -s", LOOK_SH.as_bytes(), Duration::from_secs(30)).map_err(|e| (e, None))?;
        if !ok {
            return Err((
                satellite::cant_reach(to, &err),
                satellite::refusal(to, &err),
            ));
        }
        let mut lines = out.lines().map(str::trim);
        let system = lines.next().unwrap_or("").to_owned();
        let machine = lines.next().unwrap_or("").to_owned();
        let words: Vec<String> = lines.map(str::to_owned).collect();
        Ok(Look {
            asset: asset_for(&system, &machine),
            tmux: words.iter().any(|w| w == "tmux"),
            bin_ok: words.iter().any(|w| w == "bin-ok"),
        })
    }

    fn become_member(
        &self,
        to: &Dest,
        name: &str,
        look: &Look,
        on: &mut dyn FnMut(Step, Mark),
    ) -> Result<Added, Stop> {
        let host = &to.host;
        // 放 khor.
        on(Step::Place, Mark::Live);
        if !look.bin_ok {
            let why = msg::add_why_bin_ro(host);
            on(Step::Place, Mark::Failed(why.clone()));
            return Err(Stop::Fallback(why));
        }
        let Some(asset) = look.asset else {
            let why = msg::add_why_no_build(host);
            on(Step::Place, Mark::Failed(why.clone()));
            return Err(Stop::Fallback(why));
        };
        let wait = Duration::from_secs(300);
        let (code, _, err) = run_exit(
            to,
            &install_env("KHOR_NO_SERVE=1"),
            INSTALL_SH.as_bytes(),
            wait,
        )
        .map_err(Stop::Fatal)?;
        let placed = if code == Some(0) {
            cli::add_placed_fetched(crate::VERSION)
        } else if code == Some(DOWNLOAD_UNAVAILABLE) {
            // It could not fetch for itself: this machine brings it.
            let pushed = self.push_asset(to, asset).and_then(|archive| {
                let env = install_env(&format!(
                    "KHOR_NO_SERVE=1 KHOR_ASSET_FILE={} KHOR_ASSET_SHA256={}",
                    install_quoted_value(&archive.path),
                    install_quoted_value(&archive.digest),
                ));
                let result = run(to, &env, INSTALL_SH.as_bytes(), wait);
                if !matches!(&result, Ok((true, _, _))) {
                    let cleanup = run(
                        to,
                        &format!("rm -f -- {}", install_quoted_value(&archive.path)),
                        &[],
                        Duration::from_secs(30),
                    );
                    if !matches!(cleanup, Ok((true, _, _))) {
                        let why = match &result {
                            Err(why) => why.clone(),
                            Ok((_, _, stderr)) => last_line(stderr),
                        };
                        return Err(format!(
                            "{why}; remote archive cleanup could not be confirmed"
                        ));
                    }
                }
                let (ok, _, err2) = result?;
                if ok { Ok(()) } else { Err(last_line(&err2)) }
            });
            if let Err(e) = pushed {
                let why =
                    msg::add_why_cant_place(host, if e.is_empty() { last_line(&err) } else { e });
                on(Step::Place, Mark::Failed(why.clone()));
                return Err(Stop::Fallback(why));
            }
            cli::add_placed_pushed(crate::VERSION)
        } else {
            let why = msg::add_why_cant_place(host, last_line(&err));
            on(Step::Place, Mark::Failed(why.clone()));
            return Err(if code.is_none() || code == Some(255) {
                Stop::Fatal(why)
            } else {
                Stop::Fallback(why)
            });
        };
        on(Step::Place, Mark::Done(placed));

        // 启动, then checked from a fresh connection: a resident that
        // dies with the session that started it is not one.
        on(Step::Start, Mark::Live);
        let (ok, out, err) = run(
            to,
            &install_env("KHOR_SKIP_FETCH=1"),
            INSTALL_SH.as_bytes(),
            wait,
        )
        .map_err(Stop::Fatal)?;
        let keeper = out
            .lines()
            .find_map(|l| l.trim().strip_prefix("khor: keeper "))
            .map(|k| keeper_words(k.trim()));
        let (alive, id) = if ok {
            let (_, out, _) = run(to, "sh -s", CHECK_SH.as_bytes(), Duration::from_secs(30))
                .map_err(Stop::Fatal)?;
            let mut lines = out.lines();
            let alive = lines.next().is_some_and(|l| l.trim() == "alive");
            let id = lines
                .next()
                .and_then(|l| l.split_whitespace().next())
                .map(str::to_owned);
            (alive, id)
        } else {
            (false, None)
        };
        let Some(id) = id.filter(|_| alive) else {
            let why = if ok {
                msg::add_why_no_resident(host)
            } else {
                msg::add_why_start_failed(host, last_line(&err))
            };
            on(Step::Start, Mark::Failed(why.clone()));
            return Err(Stop::Fallback(why));
        };
        on(Step::Start, Mark::Done(keeper.clone().unwrap_or_default()));

        // 配对: the ticket goes down the pair command's stdin — unless
        // khor inside ssh is on and the far side reaches none of this
        // machine's relays (round ROAD-4): that pairing cannot arrive, so
        // it is not waited out ([`PAIR_WAIT`]) before the carry.
        on(Step::Pair, Mark::Live);
        let no_relay = self.carry_on() && self.far_reaches_a_relay(to) == Some(false);
        let joined = !no_relay && {
            let ticket = self.invite().map_err(Stop::Fatal)?;
            let (ok, _, _) = run(
                to,
                "env -u KHOR_HOME \"$HOME/.local/bin/khor\" pair -",
                format!("{ticket}\n").as_bytes(),
                PAIR_WAIT,
            )
            .map_err(Stop::Fatal)?;
            drop(ticket);
            ok && self.wait_for_row(&id, Duration::from_secs(10))
        };
        // **khor inside ssh before SSH 主机** (round ROAD-3): khor runs
        // there but cannot connect back over iroh, so this machine carries
        // it over the ssh it already holds and the pairing rides that.
        let carried = !joined && self.carry_on() && self.pair_carried(to, &id);
        if !joined && !carried {
            let why = if no_relay {
                msg::add_why_no_relay(host)
            } else {
                msg::add_why_no_pair(host, PAIR_WAIT.as_secs())
            };
            on(Step::Pair, Mark::Failed(why.clone()));
            return Err(Stop::Fallback(why));
        }
        if name != host
            || !self
                .devices()
                .is_ok_and(|d| d.iter().any(|r| r.id == id && r.shown() == name))
        {
            self.rename_machine(&id, name).map_err(Stop::Fatal)?;
        }
        on(
            Step::Pair,
            Mark::Done(if carried {
                cli::add_paired_carried().to_owned()
            } else {
                String::new()
            }),
        );
        Ok(Added {
            id,
            name: name.to_owned(),
            member: true,
            why: None,
            keeper,
            tmux: look.tmux,
        })
    }

    fn add_satellite(
        &self,
        to: &Dest,
        name: &str,
        why: Option<String>,
        on: &mut dyn FnMut(Step, Mark),
    ) -> Result<Added, String> {
        on(Step::Satellite, Mark::Live);
        match self.adopt_satellite(to, Some(name)) {
            Ok((id, name, tmux)) => {
                on(Step::Satellite, Mark::Done(String::new()));
                Ok(Added {
                    id,
                    name,
                    member: false,
                    why,
                    keeper: None,
                    tmux,
                })
            }
            Err(e) => {
                on(Step::Satellite, Mark::Failed(e.clone()));
                Err(e)
            }
        }
    }

    /// Fetch the target's published archive and send those exact verified bytes.
    fn push_asset(&self, to: &Dest, asset: &str) -> Result<PushedArchive, String> {
        let mirror = std::env::var("KHOR_MIRROR").ok();
        let repo = std::env::var("KHOR_REPO").ok();
        let url = release_asset_url(mirror.as_deref(), repo.as_deref(), crate::VERSION, asset)?;
        let bytes = fetch_archive_bytes(&url, None)?;
        let record = fetch_archive_bytes(&format!("{url}.sha256"), Some(512))?;
        let expected = published_archive_digest(&record, asset)?;
        if archive_digest(&bytes) != expected {
            return Err("release archive does not match its selected SHA256".into());
        }
        let digest = archive_digest(&bytes);
        let command = format!(
            "sh -c {} sh \"$HOME/.local/bin\"",
            install_quoted_value(PUSH_ARCHIVE_SH)
        );
        let uncertain = |why: String| {
            format!(
                "{why}; remote transfer was not confirmed and an unclaimed temporary archive may remain"
            )
        };
        let (ok, receipt, err) =
            run(to, &command, &bytes, Duration::from_secs(600)).map_err(uncertain)?;
        if !ok {
            return Err(uncertain(last_line(&err)));
        }
        Ok(PushedArchive {
            path: pushed_archive_path(&receipt).map_err(uncertain)?,
            digest,
        })
    }

    /// **Pairs `id` over khor inside ssh** (round ROAD-3): this machine's
    /// resident carries it by `to` (`ipc::Op::CarryFor`), the far `khor
    /// pair -` goes through its own resident, which holds the link, and
    /// once the row is in this machine carries it for good
    /// (`DeviceDoc::set_carried`). Twice at most: the far end's first
    /// dial may leave before its link is up. `false` with no resident
    /// here, or when neither pairing arrives.
    pub fn pair_carried(&self, to: &Dest, id: &str) -> bool {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return false;
        };
        let op = crate::ipc::Op::CarryFor {
            id: id.to_owned(),
            host: to.host.clone(),
            port: to.port,
            jump: to.jump.clone(),
            identity: to.identity.clone(),
            password: to.password,
            passphrase: to.passphrase,
        };
        if !matches!(
            rt.block_on(self.via_serve(op)),
            Some(Ok(crate::ipc::Reply::Noted))
        ) {
            return false;
        }
        for _ in 0..2 {
            let Ok(ticket) = self.invite() else {
                return false;
            };
            let paired = run(
                to,
                "env -u KHOR_HOME \"$HOME/.local/bin/khor\" pair -",
                format!("{ticket}\n").as_bytes(),
                PAIR_WAIT,
            );
            drop(ticket);
            if matches!(paired, Ok((true, _, _))) && self.wait_for_row(id, Duration::from_secs(10))
            {
                let me = self.device_str().to_owned();
                return self.devices_loaded().is_ok_and(|loaded| {
                    loaded
                        .doc
                        .set_carried(id, Some(&me), Some(&to.as_row()))
                        .is_ok()
                        && {
                            let mut store = loaded.store;
                            store.flush(&loaded.doc).is_ok()
                        }
                });
            }
        }
        false
    }

    /// **Whether the far side reaches any relay this machine is on**
    /// (round ROAD-4), asked over the add's own ssh: `khor _relay-check`
    /// there sends one GET to each at once ([`reaches_a_relay`]). The
    /// plain pairing dials this machine through its relay, so a far side
    /// that reaches none of them does not arrive. `None` when it could
    /// not say: no relay here, no answer within [`RELAY_ASK`], or a khor
    /// there without the verb.
    pub fn far_reaches_a_relay(&self, to: &Dest) -> Option<bool> {
        if self.relays().is_empty() {
            return None;
        }
        let urls: String = self.relays().iter().map(|u| format!("{u}\n")).collect();
        let (ok, out, _) = run(
            to,
            "env -u KHOR_HOME \"$HOME/.local/bin/khor\" _relay-check",
            urls.as_bytes(),
            RELAY_ASK,
        )
        .ok()?;
        match (ok, out.trim()) {
            (true, "yes") => Some(true),
            (true, "no") => Some(false),
            _ => None,
        }
    }

    /// Whether `id` shows up in this machine's table within `wait` — the
    /// pair returned on the far side; this side's serve writes the row.
    fn wait_for_row(&self, id: &str, wait: Duration) -> bool {
        let start = Instant::now();
        loop {
            if self.devices().is_ok_and(|d| d.iter().any(|r| r.id == id)) {
                return true;
            }
            if start.elapsed() > wait {
                return false;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}

/// Why a member attempt stopped: give up altogether, or go satellite.
enum Stop {
    Fatal(String),
    Fallback(String),
}

/// install.sh's keeper word, said in the reader's language.
fn keeper_words(k: &str) -> String {
    match k {
        "launchd" => cli::keeper_launchd(),
        "systemd" | "systemd-linger" => cli::keeper_boot(),
        "systemd-login" | "login" => cli::keeper_login(),
        _ => cli::keeper_none(),
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_checksum_names_exactly_one_compressed_asset() {
        let asset = "khor-linux-x86_64";
        let digest = archive_digest(b"abc");
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let record = format!("{digest}  {asset}.gz");
        assert_eq!(
            published_archive_digest(record.as_bytes(), asset).unwrap(),
            digest
        );
        assert!(published_archive_digest(format!("{record}\n").as_bytes(), asset).is_ok());
        for record in [
            String::new(),
            format!("{digest}  other.gz\n"),
            format!("{digest}  ./{asset}.gz\n"),
            format!("{digest} *{asset}.gz\n"),
            format!("{record}\n\n"),
            format!("{record}\r\n"),
            format!("{record}\n{record}\n"),
            format!("{}  {asset}.gz\n", digest.to_uppercase()),
        ] {
            assert!(
                published_archive_digest(record.as_bytes(), asset).is_err(),
                "{record:?}"
            );
        }
    }

    #[test]
    fn pushed_archive_uses_one_explicit_release_directory() {
        let asset = "khor-linux-x86_64";
        assert_eq!(
            release_asset_url(Some("http://127.0.0.1:1234/base/"), None, "1.2.3", asset).unwrap(),
            "http://127.0.0.1:1234/base/releases/v1.2.3/khor-linux-x86_64.gz"
        );
        assert_eq!(
            release_asset_url(None, Some("owner/repo"), "1.2.3", asset).unwrap(),
            "https://github.com/owner/repo/releases/download/v1.2.3/khor-linux-x86_64.gz"
        );
        for mirror in [
            "file:///tmp/release",
            "https://user:pass@example.test",
            "https://example.test/?v=2",
            "https://example.test/#tag",
        ] {
            assert!(release_asset_url(Some(mirror), None, "1.2.3", asset).is_err());
        }
        assert!(release_asset_url(None, Some("../repo"), "1.2.3", asset).is_err());
        assert!(release_asset_url(None, None, "1.2.3/other", asset).is_err());
    }

    #[test]
    fn installer_configuration_values_are_literal_shell_words() {
        assert_eq!(install_quoted_value("a'b"), "'a'\\''b'");
        assert_eq!(install_quoted_value("$(touch never)"), "'$(touch never)'");
    }

    #[test]
    #[cfg(unix)]
    fn archive_output_checks_producer_status_and_bounds_metadata() {
        let mut exact = Command::new("/usr/bin/printf");
        exact.args(["%s", &"x".repeat(512)]);
        assert_eq!(
            read_archive_output(exact, Some(512)).unwrap(),
            vec![b'x'; 512]
        );

        let mut oversized = Command::new("/usr/bin/printf");
        oversized.args(["%s", &"x".repeat(100_000)]);
        assert!(read_archive_output(oversized, Some(512)).is_err());

        let mut failed = Command::new("/bin/sh");
        failed.args(["-c", "printf partial; exit 7"]);
        assert!(read_archive_output(failed, Some(512)).is_err());
    }

    #[test]
    fn archive_transfer_receipts_identify_one_owned_path() {
        assert_eq!(
            pushed_archive_path(
                "KHOR_PUSH_ASSET_V1\t/tmp/path with 'quotes'/.khor.push.A1b2C3d4\n"
            )
            .unwrap(),
            "/tmp/path with 'quotes'/.khor.push.A1b2C3d4"
        );
        for receipt in [
            "",
            "banner\nKHOR_PUSH_ASSET_V1\t/tmp/.khor.push.A1b2C3d4\n",
            "KHOR_PUSH_ASSET_V1\trelative/.khor.push.A1b2C3d4\n",
            "KHOR_PUSH_ASSET_V1\t/tmp/khor\n",
            "KHOR_PUSH_ASSET_V1\t/tmp/.khor.push.A1b2C3d4\nextra\n",
            "KHOR_PUSH_ASSET_V1\t/tmp/.khor.push.A1b2C3d4\r\n",
        ] {
            assert!(pushed_archive_path(receipt).is_err(), "{receipt:?}");
        }
    }

    #[test]
    #[cfg(unix)]
    fn concurrent_archive_transfers_keep_separate_owned_stages() {
        let dir = std::env::temp_dir().join(format!(
            "khor-archive-stage-{}",
            crate::link::fresh_hex().unwrap()
        ));
        std::fs::create_dir(&dir).unwrap();
        struct Scratch(std::path::PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _scratch = Scratch(dir.clone());
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers: Vec<_> = [b"first archive".as_slice(), b"second archive".as_slice()]
            .into_iter()
            .map(|payload| {
                let dir = dir.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let mut child = Command::new("/bin/sh")
                        .args(["-c", PUSH_ARCHIVE_SH, "archive-stage"])
                        .arg(dir)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .unwrap();
                    child.stdin.take().unwrap().write_all(payload).unwrap();
                    let output = child.wait_with_output().unwrap();
                    assert!(
                        output.status.success(),
                        "{}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                    let path =
                        pushed_archive_path(std::str::from_utf8(&output.stdout).unwrap()).unwrap();
                    assert_eq!(std::fs::read(&path).unwrap(), payload);
                    (path, payload)
                })
            })
            .collect();
        let stages: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        assert_ne!(stages[0].0, stages[1].0);
        std::fs::remove_file(&stages[0].0).unwrap();
        assert_eq!(std::fs::read(&stages[1].0).unwrap(), stages[1].1);

        let failed = format!("cat() {{ printf partial; return 7; }}\n{PUSH_ARCHIVE_SH}");
        let output = Command::new("/bin/sh")
            .args(["-c", &failed, "archive-stage"])
            .arg(&dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            output.stdout.is_empty(),
            "a partial stage cannot publish a receipt"
        );
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "failure cleans only its stage"
        );
    }

    #[test]
    fn the_asset_table_is_install_shs() {
        for (s, m, a) in [
            ("Linux", "x86_64", "khor-linux-x86_64"),
            ("Linux", "aarch64", "khor-linux-aarch64"),
            ("Darwin", "arm64", "khor-macos-arm64"),
            ("Darwin", "x86_64", "khor-macos-x86_64"),
        ] {
            assert_eq!(asset_for(s, m), Some(a));
            assert!(
                INSTALL_SH.contains(&format!("ASSET={a}")),
                "install.sh names {a}"
            );
        }
        assert_eq!(asset_for("FreeBSD", "amd64"), None);
    }

    #[test]
    fn install_sh_names_every_keeper_word_this_reads() {
        for k in [
            "none",
            "launchd",
            "systemd",
            "systemd-linger",
            "systemd-login",
            "login",
        ] {
            assert!(
                INSTALL_SH.contains(&format!("KEEPER={k}")),
                "install.sh sets KEEPER={k}"
            );
        }
        assert!(INSTALL_SH.contains("KHOR_SKIP_FETCH") && INSTALL_SH.contains("KHOR_ASSET_FILE"));
    }
}
