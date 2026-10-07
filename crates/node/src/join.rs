//! Adding a machine over ssh (添加机器; docs/NET.md 卫星 and 转正).
//!
//! One verb, two outcomes. By default the machine becomes a **full
//! member**: khor is put on it, its resident started at user level, and
//! it pairs with this machine — all from here, over the user's own ssh
//! road, the satellite module's rules (BatchMode, nothing stored, the
//! jump as `ProxyCommand`). When it cannot stay a member — its bin
//! directory is read-only, its resident does not outlive the session,
//! or it cannot call back over iroh in [`PAIR_WAIT`] — everything the
//! attempt put there is taken off again (「卫星身上不留东西」) and it is
//! adopted as a **satellite** instead. A flag skips straight to that.
//!
//! # What runs over there is `install.sh`, the one every install runs
//!
//! The script travels on ssh's stdin (`sh -s`), so nothing is written
//! there that the install itself would not write. Two passes, because
//! the person watches two steps: 放 khor (`KHOR_NO_SERVE`), then 启动
//! (`KHOR_SKIP_FETCH`). The version is this machine's own
//! ([`crate::VERSION`]), so a network never grows a second khor by
//! being added to; `KHOR_MIRROR` set here is passed along. When the far
//! machine cannot fetch, this one fetches (or, on the same platform,
//! sends its own program) and pushes the asset down the ssh channel.
//!
//! # What is never done
//!
//! No root is asked for, the user's `~/.ssh` is never written, the
//! pairing ticket travels on the pair command's stdin and is never
//! printed, logged, or put on a command line (a command line is in
//! `ps` on both machines).
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use khor_catalog::{cli, msg};

use crate::satellite::{self, Dest};
use crate::Node;

/// The install script, as shipped: what runs on the far machine.
const INSTALL_SH: &str = include_str!("../../../scripts/install.sh");

/// How long the new machine has to call back over iroh and pair. Past
/// it the machine is one khor cannot talk to directly, which is what a
/// satellite is for.
pub const PAIR_WAIT: Duration = Duration::from_secs(60);

/// How long the far side may take to say whether it reaches a relay
/// (round ROAD-4): ssh's login, then one GET to each relay at once,
/// each inside `doctor`'s three seconds.
pub const RELAY_ASK: Duration = Duration::from_secs(10);

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
    /// Taking off what the attempt put there, on the way to a satellite.
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
static RUNS: LazyLock<Mutex<Vec<(String, Arc<Mutex<Run>>)>>> = LazyLock::new(|| Mutex::new(Vec::new()));

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
pub(crate) fn start(node: Arc<Node>, run: Arc<Mutex<Run>>, to: Dest, name: Option<String>, satellite_only: bool) {
    std::thread::spawn(move || {
        let out = node.add_over_ssh(&to, name.as_deref(), satellite_only, None, &mut |s, m| {
            run.lock().unwrap_or_else(|p| p.into_inner()).marks.push((s, m));
        });
        run.lock().unwrap_or_else(|p| p.into_inner()).done = Some(out);
    });
}

/// What one look at the machine found, over one connection — the
/// before-picture the undo is measured against.
#[derive(Debug, Default)]
struct Look {
    asset: Option<&'static str>,
    tmux: bool,
    had: std::collections::HashSet<String>,
    bin_ok: bool,
}

const LOOK_SH: &str = r#"
B="$HOME/.local/bin"
uname -s; uname -m
command -v tmux >/dev/null 2>&1 && echo tmux
[ -e "$B/khor" ] && echo had-bin
[ -e "$B/khor-serve-up" ] && echo had-up
R=""; [ -x "$B/khor" ] && R=$(env -u KHOR_HOME "$B/khor" root 2>/dev/null)
[ -n "$R" ] || R="$HOME"
[ -e "$R/.khor" ] && echo had-store
[ -e "$HOME/.khor-hosts" ] && echo had-hosts
[ -e "$HOME/.profile" ] && echo had-profile
for d in Library Library/LaunchAgents .config .config/systemd .config/systemd/user; do
  [ -d "$HOME/$d" ] && echo "had-d:$d"
done
grep -qs 'HOME/.local/bin' "$HOME/.profile" && echo had-path
grep -qs khor-serve-up "$HOME/.profile" && echo had-up-profile
grep -qs khor-serve-up "$HOME/.bash_profile" && echo had-up-bash
[ -e "$HOME/Library/LaunchAgents/io.github.colehank.khor.plist" ] && echo had-agent
[ -e "$HOME/.config/systemd/user/khor-serve.service" ] && echo had-unit
if [ -d "$B" ]; then echo had-dir; [ -w "$B" ] && echo bin-ok
else P="$HOME/.local"; [ -d "$P" ] || P="$HOME"; [ -w "$P" ] && echo bin-ok; fi
exit 0
"#;

/// Takes off what an attempt put there, and only that: every `had-*`
/// the look saw is passed in as a variable and left alone.
const UNDO_SH: &str = r#"
B="$HOME/.local/bin"
R=""; [ -x "$B/khor" ] && R=$(env -u KHOR_HOME "$B/khor" root 2>/dev/null)
[ -n "$R" ] || R="$HOME"
if [ -z "$HAD_STORE" ]; then
  P=$(cat "$R/.khor/serve.pid" 2>/dev/null)
  [ -n "$P" ] && kill "$P" 2>/dev/null
  sleep 1
  [ -n "$P" ] && kill -9 "$P" 2>/dev/null
fi
if [ -z "$HAD_AGENT" ] && [ -e "$HOME/Library/LaunchAgents/io.github.colehank.khor.plist" ]; then
  launchctl bootout "gui/$(id -u)/io.github.colehank.khor" 2>/dev/null
  rm -f "$HOME/Library/LaunchAgents/io.github.colehank.khor.plist"
fi
if [ -z "$HAD_UNIT" ] && [ -e "$HOME/.config/systemd/user/khor-serve.service" ]; then
  systemctl --user disable khor-serve.service >/dev/null 2>&1
  rm -f "$HOME/.config/systemd/user/khor-serve.service"
fi
drop() { [ -f "$1" ] || return 0; grep -v "$2" "$1" > "$1.khor-undo" ; mv -f "$1.khor-undo" "$1"; }
[ -n "$HAD_PATH" ] || drop "$HOME/.profile" 'HOME/.local/bin:\$PATH'
[ -n "$HAD_UP_PROFILE" ] || drop "$HOME/.profile" 'khor-serve-up'
[ -n "$HAD_UP_BASH" ] || drop "$HOME/.bash_profile" 'khor-serve-up'
[ -n "$HAD_PROFILE" ] || [ -s "$HOME/.profile" ] || rm -f "$HOME/.profile"
[ -n "$HAD_BIN" ] || rm -f "$B/khor"
[ -n "$HAD_UP" ] || rm -f "$B/khor-serve-up"
rm -f "$B"/.khor.download.* "$B/.khor.push.gz"
[ -n "$HAD_DIR" ] || rmdir "$B" "$HOME/.local" 2>/dev/null
# The directories the keeper's file went into, deepest first; `rmdir`
# leaves any that hold something else.
for d in $NEW_DIRS; do rmdir "$HOME/$d" 2>/dev/null; done
if [ -z "$HAD_STORE" ]; then
  case "$R" in "$HOME"|"$HOME"/*) rm -rf "$R/.khor" ;; esac
  [ -n "$HAD_HOSTS" ] || rm -rf "$HOME/.khor-hosts"
fi
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

/// This machine's own asset, so a far machine of the same kind can be
/// sent the program that is running here.
fn own_asset() -> Option<&'static str> {
    let system = match std::env::consts::OS {
        "linux" => "Linux",
        "macos" => "Darwin",
        _ => return None,
    };
    // Rust calls Apple silicon `aarch64`; `uname -m` there says `arm64`.
    let machine = match (system, std::env::consts::ARCH) {
        ("Darwin", "aarch64") => "arm64",
        (_, arch) => arch,
    };
    asset_for(system, machine)
}

/// One ssh run with `stdin` fed and both outputs kept, bounded by
/// `wait` (killed past it — by its own pid).
fn run(to: &Dest, command: &str, stdin: &[u8], wait: Duration) -> Result<(bool, String, String), String> {
    let mut child = to
        .command("ssh")
        .args(["-o", "ConnectTimeout=8"])
        .arg(&to.host)
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(msg::ssh_wont_run)?;
    if let Some(mut w) = child.stdin.take() {
        // A far side that never reads (a refused login) closes the pipe:
        // its words are on stderr, not here.
        let _ = w.write_all(stdin);
    }
    let start = Instant::now();
    loop {
        if child.try_wait().map_err(msg::ssh_wont_run)?.is_some() {
            break;
        }
        if start.elapsed() > wait {
            let _ = child.kill();
            let _ = child.wait();
            return Ok((false, String::new(), msg::add_timed_out(wait.as_secs())));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let out = child.wait_with_output().map_err(msg::ssh_wont_run)?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// The environment the install script is started under over there:
/// this machine's version, and its mirror when it has one. Values are
/// single-quoted; the names are ours.
fn install_env(extra: &str) -> String {
    let mut e = format!("KHOR_VERSION='v{}'", crate::VERSION);
    if let Ok(m) = std::env::var("KHOR_MIRROR")
        && !m.is_empty()
        && !m.contains('\'')
    {
        e.push_str(&format!(" KHOR_MIRROR='{m}'"));
    }
    format!("{e} {extra} sh -s")
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
            !a.member || self.devices().is_ok_and(|all| all.iter().any(|d| d.id == a.id && self.logs_in_to(d)))
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
            Err(e) => {
                on(Step::Connect, Mark::Failed(e.clone()));
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
            Err(Stop::Fallback(why)) => {
                on(Step::Undo, Mark::Live);
                self.undo(to, &look)?;
                on(Step::Undo, Mark::Done(String::new()));
                self.add_satellite(to, &name, Some(why), on)
            }
        }
    }

    /// 转正: a satellite this machine sponsors becomes a full member
    /// through the same install; its satellite row goes and its name
    /// carries over to the new machine. When it cannot stay a member it
    /// stays the satellite it was, with nothing left behind.
    pub fn promote(
        &self,
        machine: &str,
        on: &mut dyn FnMut(Step, Mark),
    ) -> Result<Added, String> {
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
        let look = self.look(&to).inspect_err(|e| on(Step::Connect, Mark::Failed(e.clone())))?;
        on(Step::Connect, Mark::Done(String::new()));
        match self.become_member(&to, &name, &look, on) {
            Ok(added) => {
                self.forget_id(&row.id)?;
                Ok(added)
            }
            Err(Stop::Fatal(e)) => Err(e),
            Err(Stop::Fallback(why)) => {
                on(Step::Undo, Mark::Live);
                self.undo(&to, &look)?;
                on(Step::Undo, Mark::Done(String::new()));
                Err(msg::promote_stayed_satellite(why))
            }
        }
    }

    fn look(&self, to: &Dest) -> Result<Look, String> {
        let (ok, out, err) = run(to, "sh -s", LOOK_SH.as_bytes(), Duration::from_secs(30))?;
        if !ok {
            return Err(satellite::cant_reach(to, &err));
        }
        let mut lines = out.lines().map(str::trim);
        let system = lines.next().unwrap_or("").to_owned();
        let machine = lines.next().unwrap_or("").to_owned();
        let words: Vec<String> = lines.map(str::to_owned).collect();
        Ok(Look {
            asset: asset_for(&system, &machine),
            tmux: words.iter().any(|w| w == "tmux"),
            bin_ok: words.iter().any(|w| w == "bin-ok"),
            had: words.into_iter().filter(|w| w.starts_with("had-")).collect(),
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
        let (ok, _, err) = run(to, &install_env("KHOR_NO_SERVE=1"), INSTALL_SH.as_bytes(), wait).map_err(Stop::Fatal)?;
        let placed = if ok {
            cli::add_placed_fetched(crate::VERSION)
        } else {
            // It could not fetch for itself: this machine brings it.
            let pushed = self
                .push_asset(to, asset)
                .and_then(|()| {
                    let env = install_env("KHOR_NO_SERVE=1 KHOR_ASSET_FILE=\"$HOME/.local/bin/.khor.push.gz\"");
                    let (ok, _, err2) = run(to, &env, INSTALL_SH.as_bytes(), wait)?;
                    if ok { Ok(()) } else { Err(last_line(&err2)) }
                });
            if let Err(e) = pushed {
                let why = msg::add_why_cant_place(host, if e.is_empty() { last_line(&err) } else { e });
                on(Step::Place, Mark::Failed(why.clone()));
                return Err(Stop::Fallback(why));
            }
            cli::add_placed_pushed(crate::VERSION)
        };
        on(Step::Place, Mark::Done(placed));

        // 启动, then checked from a fresh connection: a resident that
        // dies with the session that started it is not one.
        on(Step::Start, Mark::Live);
        let (ok, out, err) =
            run(to, &install_env("KHOR_SKIP_FETCH=1"), INSTALL_SH.as_bytes(), wait).map_err(Stop::Fatal)?;
        let keeper = out
            .lines()
            .find_map(|l| l.trim().strip_prefix("khor: keeper "))
            .map(|k| keeper_words(k.trim()));
        let (alive, id) = if ok {
            let (_, out, _) = run(to, "sh -s", CHECK_SH.as_bytes(), Duration::from_secs(30)).map_err(Stop::Fatal)?;
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
        if name != host || !self.devices().is_ok_and(|d| d.iter().any(|r| r.id == id && r.shown() == name)) {
            self.rename_machine(&id, name).map_err(Stop::Fatal)?;
        }
        on(Step::Pair, Mark::Done(if carried { cli::add_paired_carried().to_owned() } else { String::new() }));
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

    /// Fetches (or, on the same platform, takes this machine's own) the
    /// asset and pushes it next to the far bin directory.
    fn push_asset(&self, to: &Dest, asset: &str) -> Result<(), String> {
        let tmp = self.root().join(".khor").join(format!(".add-{asset}.gz"));
        let made = if own_asset() == Some(asset) {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
            Command::new("gzip")
                .arg("-c")
                .arg(&exe)
                .stdout(f)
                .status()
                .map_err(|e| e.to_string())?
                .success()
        } else {
            let url = match std::env::var("KHOR_MIRROR") {
                Ok(m) if !m.is_empty() => format!("{}/{asset}.gz", m.trim_end_matches('/')),
                _ => format!(
                    "https://github.com/colehank/khor/releases/download/v{}/{asset}.gz",
                    crate::VERSION
                ),
            };
            Command::new("curl")
                .args(["-fsL", "--connect-timeout", "15", "-o"])
                .arg(&tmp)
                .arg(&url)
                .stderr(Stdio::null())
                .status()
                .map_err(|e| e.to_string())?
                .success()
        };
        let pushed = if made {
            let bytes = std::fs::read(&tmp).map_err(|e| e.to_string());
            bytes.and_then(|b| {
                let (ok, _, err) = run(
                    to,
                    "mkdir -p \"$HOME/.local/bin\" && cat > \"$HOME/.local/bin/.khor.push.gz\"",
                    &b,
                    Duration::from_secs(600),
                )?;
                if ok { Ok(()) } else { Err(last_line(&err)) }
            })
        } else {
            Err(msg::add_no_asset_here(asset))
        };
        let _ = std::fs::remove_file(&tmp);
        pushed
    }

    fn undo(&self, to: &Dest, look: &Look) -> Result<(), String> {
        let flag = |w: &str| if look.had.contains(w) { "1" } else { "" };
        let vars = format!(
            "HAD_BIN='{}' HAD_UP='{}' HAD_STORE='{}' HAD_HOSTS='{}' HAD_PATH='{}' HAD_UP_PROFILE='{}' HAD_UP_BASH='{}' HAD_AGENT='{}' HAD_UNIT='{}' HAD_DIR='{}' HAD_PROFILE='{}' NEW_DIRS='{}' sh -s",
            flag("had-bin"),
            flag("had-up"),
            flag("had-store"),
            flag("had-hosts"),
            flag("had-path"),
            flag("had-up-profile"),
            flag("had-up-bash"),
            flag("had-agent"),
            flag("had-unit"),
            flag("had-dir"),
            flag("had-profile"),
            // Created by the attempt, deepest first.
            ["Library/LaunchAgents", "Library", ".config/systemd/user", ".config/systemd", ".config"]
                .into_iter()
                .filter(|d| !look.had.contains(&format!("had-d:{d}")))
                .collect::<Vec<_>>()
                .join(" "),
        );
        let (ok, _, err) = run(to, &vars, UNDO_SH.as_bytes(), Duration::from_secs(60))?;
        if ok { Ok(()) } else { Err(satellite::cant_reach(to, &err)) }
    }

    /// **Pairs `id` over khor inside ssh** (round ROAD-3): this machine's
    /// resident carries it by `to` (`ipc::Op::CarryFor`), the far `khor
    /// pair -` goes through its own resident, which holds the link, and
    /// once the row is in this machine carries it for good
    /// (`DeviceDoc::set_carried`). Twice at most: the far end's first
    /// dial may leave before its link is up. `false` with no resident
    /// here, or when neither pairing arrives.
    pub fn pair_carried(&self, to: &Dest, id: &str) -> bool {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
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
        if !matches!(rt.block_on(self.via_serve(op)), Some(Ok(crate::ipc::Reply::Noted))) {
            return false;
        }
        for _ in 0..2 {
            let Ok(ticket) = self.invite() else { return false };
            let paired = run(
                to,
                "env -u KHOR_HOME \"$HOME/.local/bin/khor\" pair -",
                format!("{ticket}\n").as_bytes(),
                PAIR_WAIT,
            );
            drop(ticket);
            if matches!(paired, Ok((true, _, _))) && self.wait_for_row(id, Duration::from_secs(10)) {
                let me = self.device_str().to_owned();
                return self.devices_loaded().is_ok_and(|loaded| {
                    loaded.doc.set_carried(id, Some(&me), Some(&to.as_row())).is_ok() && {
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
    fn the_asset_table_is_install_shs() {
        for (s, m, a) in [
            ("Linux", "x86_64", "khor-linux-x86_64"),
            ("Linux", "aarch64", "khor-linux-aarch64"),
            ("Darwin", "arm64", "khor-macos-arm64"),
            ("Darwin", "x86_64", "khor-macos-x86_64"),
        ] {
            assert_eq!(asset_for(s, m), Some(a));
            assert!(INSTALL_SH.contains(&format!("ASSET={a}")), "install.sh names {a}");
        }
        assert_eq!(asset_for("FreeBSD", "amd64"), None);
    }

    #[test]
    fn install_sh_names_every_keeper_word_this_reads() {
        for k in ["none", "launchd", "systemd", "systemd-linger", "systemd-login", "login"] {
            assert!(INSTALL_SH.contains(&format!("KEEPER={k}")), "install.sh sets KEEPER={k}");
        }
        assert!(INSTALL_SH.contains("KHOR_SKIP_FETCH") && INSTALL_SH.contains("KHOR_ASSET_FILE"));
    }
}
