//! 添加机器 over SSH (lanes join-a … join-d): the add, 安装 khor on an
//! SSH host, and the two things the form offers around them — the ssh
//! config's hosts and a terminal running `ssh-copy-id`.
//!
//! An add takes seconds to minutes and the person watches it step by
//! step, so it runs like a drop: `add_start` starts it on a thread of
//! its own under a token the face made, and `add_read` answers every
//! step so far and, once it is over, how it ended. The face asks again
//! every half second; nothing is pushed.
//!
//! **The secret** (a password, or a key file's passphrase) comes in on
//! [`AddAsk`] and goes straight to `Node::add_over_ssh`, which keeps it
//! only in the OS secret store. It is never in an [`AddRun`], never
//! logged, and [`AddAsk`] has no `Debug` so no panic message can print
//! it. The app only: no web face serves these (`command_tables.rs`).

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use khor_node::join::{Added, Mark, Step};
use khor_node::satellite::{self, Dest, Refusal};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What the SSH form asks for.
#[derive(Clone, Deserialize, TS)]
pub struct AddAsk {
    /// As the person names it: a Host alias or `user@host`.
    pub host: String,
    /// `None` = 22 or what the ssh config says.
    pub port: Option<u16>,
    /// An ssh jump host (跳板机 · 通过 SSH 跳板机).
    pub jump: Option<String>,
    /// A khor machine to run the add on (跳板机 · 通过 khor 主机): it
    /// becomes the sponsor. Never with `jump` from the form.
    pub on: Option<String>,
    /// A key file's path, on the machine that runs ssh. Only the path is
    /// kept; the key is never read, copied or sent.
    pub identity: Option<String>,
    /// The password, or the key file's passphrase when `identity` is set.
    pub secret: Option<String>,
    /// 在 <主机> 上安装 khor: `false` adds it as an SSH 主机 directly.
    pub install: bool,
}

/// The steps the face draws, one line each, in the order they happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum AddStep {
    Connect,
    Place,
    Start,
    Pair,
    Undo,
    Satellite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum StepState {
    Live,
    Done,
    Failed,
}

/// Why a login was refused, so the face can offer the way on: `key` →
/// upload the public key or use a password; `locked` → ask the key
/// file's passphrase; `password` / `passphrase` → the kept one was wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum AddRefusal {
    Key,
    Locked,
    Password,
    Passphrase,
}

impl From<Refusal> for AddRefusal {
    fn from(r: Refusal) -> Self {
        match r {
            Refusal::Key => AddRefusal::Key,
            Refusal::Locked => AddRefusal::Locked,
            Refusal::Password => AddRefusal::Password,
            Refusal::Passphrase => AddRefusal::Passphrase,
        }
    }
}

/// One step as it stands now: a later mark of the same step replaces it.
#[derive(Debug, Clone, Serialize, TS)]
pub struct AddMark {
    pub step: AddStep,
    pub state: StepState,
    /// What it did, or why it failed; empty when there is nothing to say.
    pub note: String,
    pub refusal: Option<AddRefusal>,
}

/// How an add ended.
#[derive(Debug, Clone, Serialize, TS)]
pub struct AddEnd {
    /// The machine's id and name in the table, when it is in.
    pub id: Option<String>,
    pub name: Option<String>,
    /// A khor 主机; `false` = an SSH 主机.
    pub member: bool,
    /// Why it is an SSH 主机 although khor was to be installed.
    pub why: Option<String>,
    /// What brings a khor 主机's resident back after a reboot.
    pub keeper: Option<String>,
    /// The sentence, when the add failed.
    pub error: Option<String>,
    /// The refusal behind `error`, when it was a refused login.
    pub refusal: Option<AddRefusal>,
}

/// An add as the face reads it.
#[derive(Debug, Clone, Default, Serialize, TS)]
pub struct AddRun {
    pub marks: Vec<AddMark>,
    pub done: Option<AddEnd>,
}

/// The runs this app holds, by token; the oldest finished ones go.
static RUNS: LazyLock<Mutex<VecDeque<(String, AddRun)>>> = LazyLock::new(|| Mutex::new(VecDeque::new()));
const RUNS_KEPT: usize = 8;

fn with_run(token: &str, f: impl FnOnce(&mut AddRun)) {
    let mut runs = RUNS.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((_, r)) = runs.iter_mut().find(|(t, _)| t == token) {
        f(r);
    }
}

/// Makes the run `token`; `false` when it already exists (a second ask
/// with the same token starts nothing).
fn make_run(token: &str) -> bool {
    let mut runs = RUNS.lock().unwrap_or_else(|p| p.into_inner());
    if runs.iter().any(|(t, _)| t == token) {
        return false;
    }
    while runs.iter().filter(|(_, r)| r.done.is_some()).count() >= RUNS_KEPT {
        let Some(i) = runs.iter().position(|(_, r)| r.done.is_some()) else { break };
        runs.remove(i);
    }
    runs.push_back((token.to_owned(), AddRun::default()));
    true
}

fn note(token: &str, step: Step, mark: Mark) {
    let step = match step {
        Step::Connect => AddStep::Connect,
        Step::Place => AddStep::Place,
        Step::Start => AddStep::Start,
        Step::Pair => AddStep::Pair,
        Step::Undo => AddStep::Undo,
        Step::Satellite => AddStep::Satellite,
    };
    let (state, note, refusal) = match mark {
        Mark::Live => (StepState::Live, String::new(), None),
        Mark::Done(n) => (StepState::Done, n, None),
        Mark::Failed(n) => (StepState::Failed, n, None),
        Mark::Refused(r, n) => (StepState::Failed, n, Some(r.into())),
    };
    let m = AddMark { step, state, note, refusal };
    with_run(token, |r| match r.marks.iter_mut().find(|x| x.step == step) {
        Some(was) => *was = m,
        None => r.marks.push(m),
    });
}

fn end(token: &str, out: Result<Added, String>) {
    with_run(token, |r| {
        let refusal = r.marks.iter().rev().find_map(|m| m.refusal);
        r.done = Some(match out {
            Ok(a) => AddEnd {
                id: Some(a.id),
                name: Some(a.name),
                member: a.member,
                why: a.why,
                keeper: a.keeper,
                error: None,
                refusal: None,
            },
            Err(e) => AddEnd {
                id: None,
                name: None,
                member: false,
                why: None,
                keeper: None,
                error: Some(e),
                refusal,
            },
        });
    });
}

/// Starts the add `ask` as the run `token`. On this machine it is
/// `Node::add_over_ssh`; with [`AddAsk::on`] naming another khor machine
/// it is `Node::add_on`, run there (a key login only — the node refuses
/// a password or passphrase in one sentence, since it would have to
/// cross the wire).
pub fn add_start(root: &Path, token: String, ask: AddAsk) -> Result<(), String> {
    let host = ask.host.trim().to_owned();
    if host.is_empty() {
        return Err(khor_catalog::msg::add_need_host().into());
    }
    if !make_run(&token) {
        return Ok(());
    }
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let to = Dest::new(host, ask.jump.as_deref().filter(|j| !j.trim().is_empty()))
            .on_port(ask.port)
            .with_identity(ask.identity.as_deref().filter(|k| !k.trim().is_empty()));
        let on_step = |s: Step, m: Mark| note(&token, s, m);
        let out = (|| {
            let n = khor_node::Node::open(root)?;
            match ask.on.as_deref().filter(|m| !m.is_empty() && *m != n.name()) {
                Some(machine) => {
                    let to = match ask.secret {
                        Some(_) if to.identity.is_some() => to.with_passphrase(true),
                        Some(_) => to.with_password(true),
                        None => to,
                    };
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| e.to_string())?;
                    rt.block_on(n.add_on(machine, &to, None, !ask.install, on_step))
                }
                None => {
                    let mut on_step = on_step;
                    n.add_over_ssh(&to, None, !ask.install, ask.secret.as_deref(), &mut on_step)
                }
            }
        })();
        end(&token, out);
    });
    Ok(())
}

/// 安装 khor on an SSH 主机 this machine sponsors (`Node::promote`), as
/// the run `token`: the same steps as an add.
pub fn promote_start(root: &Path, token: String, machine: String) -> Result<(), String> {
    if !make_run(&token) {
        return Ok(());
    }
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let out = khor_node::Node::open(root).and_then(|n| n.promote(&machine, &mut |s, m| note(&token, s, m)));
        end(&token, out);
    });
    Ok(())
}

/// The run `token` as it stands; an unknown token is an error (the app
/// restarted under the face, or the run aged out).
pub fn add_read(token: &str) -> Result<AddRun, String> {
    let runs = RUNS.lock().unwrap_or_else(|p| p.into_inner());
    runs.iter()
        .find(|(t, _)| t == token)
        .map(|(_, r)| r.clone())
        .ok_or_else(|| khor_catalog::msg::add_lost().into())
}

/// The `Host` names of the person's ssh config, read only.
pub fn ssh_config_hosts() -> Vec<String> {
    satellite::config_hosts()
}

/// The login the form describes, for `ssh-copy-id`.
#[derive(Clone, Deserialize, TS)]
pub struct LoginAsk {
    pub host: String,
    pub port: Option<u16>,
    pub jump: Option<String>,
    pub identity: Option<String>,
}

/// 上传公钥: a terminal session on this machine running `ssh-copy-id`
/// for the form's host (`satellite::copy_id_argv`: port, jump and key
/// file included). ssh asks for the password in that terminal, so khor
/// never sees it; the shell stays when it is done. Answers the session.
pub fn open_copy_id(root: &Path, ask: LoginAsk) -> Result<String, String> {
    let host = ask.host.trim();
    if host.is_empty() {
        return Err(khor_catalog::msg::add_need_host().into());
    }
    let to = Dest::new(host, ask.jump.as_deref().filter(|j| !j.trim().is_empty()))
        .on_port(ask.port)
        .with_identity(ask.identity.as_deref().filter(|k| !k.trim().is_empty()));
    // The names ride as arguments, so no shell ever reads them.
    let mut argv = vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        r#""$@"; exec "${SHELL:-/bin/sh}" -l"#.to_owned(),
        "sh".to_owned(),
    ];
    argv.extend(satellite::copy_id_argv(&to));
    let n = khor_node::Node::open(root.to_path_buf())?;
    let id = n.open_persistent(khor_node::kind::SHELL, "ssh-copy-id", &argv, (80, 24))?;
    Ok(id.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A later mark of a step replaces the earlier one, and the end of a
    /// refused login carries its refusal.
    #[test]
    fn a_run_keeps_one_line_per_step_and_its_refusal() {
        let t = "t-one-line";
        assert!(make_run(t));
        assert!(!make_run(t));
        note(t, Step::Connect, Mark::Live);
        note(t, Step::Connect, Mark::Refused(Refusal::Key, "refused".into()));
        end(t, Err("refused".into()));
        let run = add_read(t).unwrap();
        assert_eq!(run.marks.len(), 1);
        assert_eq!(run.marks[0].state, StepState::Failed);
        assert_eq!(run.done.unwrap().refusal, Some(AddRefusal::Key));
    }
}
