//! tmux: the container, not a vendor.
//!
//! Every other module here reads one agent's files. This one reads a
//! **terminal multiplexer**, and the difference is the whole design:
//! claude and codex each own the sessions they report, while a tmux
//! session owns nothing — it holds whatever the user started in it,
//! which is often a session somebody else in this directory already
//! listed.
//!
//! # Why this is not an `Adaptor`
//!
//! It has no [`super::Adaptor`] impl and no trait of its own. The seam
//! rule this repo follows is to cut one at the **second** real
//! implementation, never at the first (`Adaptor` itself waited for
//! codex; [`crate::KindSurface`] waited for the third kind). There is
//! one multiplexer. A second — screen, zellij — is what would show which
//! parts of this are general, and until then a trait would be a guess
//! wearing ceremony.
//!
//! It also would not fit: [`super::Adaptor::sweep`] answers "which of my
//! sessions are alive", and this one cannot answer alone. Its rows are
//! only rows once nothing else claims what is inside them, so it hands
//! back [`Multiplexed`] — a candidate plus the pids it holds — and
//! `crate::live::LiveKind::rows` decides.
//!
//! # Calling the user's tmux is not a dependency on tmux
//!
//! Khor asks nothing of the user (docs/KHOR.md 零依赖). Running their
//! `tmux` binary does not break that: **no tmux, no rows, no error, no
//! count** — the list is exactly what it would have been. The rule being
//! kept is "khor never requires an install"; the command-line interface
//! of a program the user already chose to run is the same kind of public
//! surface as the files claude already chose to write, and this module
//! reads it in the same posture.
//!
//! **Where the binary is looked for is part of that promise.** A GUI
//! launched from the Finder inherits a PATH without `/opt/homebrew/bin`,
//! so PATH alone would make the app list fewer sessions than the
//! terminal does, with nothing on screen to say why. See [`CANDIDATES`].
//!
//! # One row is one tmux session
//!
//! Panes and windows are not rows (decided 2026-08-16; the terminal
//! screen batch owns pane detail). A session with three panes is one
//! line, and its word is the busiest thing in it.
//!
//! # What tmux can and cannot say
//!
//! Two of the six words, and it is worth naming the four it cannot:
//!
//! - **忙碌 / 空闲** — from the foreground command, below.
//! - **完成 and 待批 are unreachable**: tmux has no notion of a turn or
//!   of an approval, so there is nothing in it that could be either. Not
//!   a gap to fill later — a shell has no turns.
//! - **中断 is unreachable**: an error is a thing the program inside
//!   knows, and tmux does not read it.
//! - **失败 is unreachable, and this is the sharp one.** Every other
//!   source here can spell 失败 because a file outlived the process that
//!   wrote it ([`super`] module head, the missing-ending rule). tmux
//!   keeps no such file: when a session ends, it is gone from
//!   `list-panes` on the very next sweep and the row simply stops
//!   existing. **There is no crash to record because there is nothing
//!   left to record it with**, and inventing one from the row's
//!   disappearance would put 失败 on every session the user closed on
//!   purpose.
//!
//! ## 忙碌 vs 空闲, and the two kinds of pane
//!
//! `#{pane_current_command}` is the command of the pane tty's foreground
//! process group — the same fact `crate::host::host_main` polls with
//! `process_group_leader()` for the shells khor hosts itself, so this is
//! that judgment ported, not a new one.
//!
//! Reading it needs to know what the pane started as, and tmux answers
//! that too:
//!
//! - `pane_start_command` **non-empty** — `tmux new-session 'sleep 300'`
//!   — means the pane *is* that command. It is running (its pid is in
//!   the process table, or there would be no word at all): 忙碌.
//! - `pane_start_command` **empty** means the pane runs the user's login
//!   shell. Then the foreground command equals the shell's own name
//!   exactly while there is a prompt: 空闲, and anything else is 忙碌.
//!
//! Without that split the first case reads 空闲 while its command runs,
//! because the pane's own process is trivially its own foreground —
//! measured, not reasoned: `tmux new-session -d 'sleep 300'` reports
//! `pane_pid=18849, pane_current_command=sleep`, and `ps` says 18849 is
//! the `sleep`.
//!
//! A pane whose pid is not in the process table yields **no word**
//! rather than a guess; a session where no pane yields one is counted,
//! never shown (see [`Listing::unmapped`]).
//!
//! # Measured on this machine, 2026-08-16
//!
//! 9 tmux sessions on the user's default server, 817 processes, debug
//! build (`cargo test -p khor-node --test cost -- --ignored --nocapture
//! --test-threads=1`):
//!
//! - one `tmux list-panes -a`: **6.8 ms** median (6.4 min, 8.9 max),
//!   which is **0.14% of a core** at the app's five-second poll. It is
//!   the only cost in the session list that is a subprocess rather than
//!   a syscall, and it is roughly what the whole process-table snapshot
//!   costs beside it (5.1 ms) — so asking tmux doubles the cheap part
//!   of a list that already costs 13.6 ms, and no cache is warranted for
//!   the same reason `Procs::snapshot` has none.
//! - of the 9, **8 hold something already listed and are dropped**: six
//!   claude sessions, one codex, and one claude that had been resumed
//!   into a second process (see [`super::Sighting::pids`] — that last
//!   one is why the field is a list). The ninth is a bare shell and is
//!   the one row this module adds on this machine.
//!
//! That ratio is the argument for the whole duplicate rule: without it
//! this machine's list would grow eight rows that are all second copies
//! of rows already on it.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use khor_core::{SessionId, State};

use super::{Found, Multiplexed, Proc, Procs, Sighting};

/// This source's name, which is also the category its rows carry —
/// **and those are deliberately not the same word.**
///
/// `tmux` is who recognised the session; `shell` is what the session is.
/// Everywhere else in this directory those coincide, because a vendor's
/// row belongs to that vendor. Here the row belongs to the user: they
/// opened a shell, and tmux is the furniture it sits in. A category of
/// "tmux" would file "my own shell" under the name of a program the
/// person may not think of as owning anything.
pub const VENDOR: &str = "tmux";

/// Where the binary might be, tried in order until one starts.
///
/// PATH first, because that is the user's own answer. Then the two
/// places package managers put it: khor's list must not be shorter in
/// the app than in the terminal, and the app inherits the Finder's PATH,
/// which has neither. `KHOR_TMUX` overrides the lot — that is how the
/// tests point at a binary and how an odd install is fixed without a
/// release.
const CANDIDATES: [&str; 4] =
    ["tmux", "/opt/homebrew/bin/tmux", "/usr/local/bin/tmux", "/usr/bin/tmux"];

/// How long one `list-panes` may take before khor walks away from it.
///
/// A wedged tmux server is a thing that happens, and the session list is
/// polled every 5 seconds by the app — one stuck call would freeze the
/// list instead of showing it without tmux rows. Generous against the
/// 6 ms this takes when it works, small against the poll it must not
/// eat.
const CALL_DEADLINE: Duration = Duration::from_millis(1000);

/// One line of the format below. Session fields repeat per pane; that is
/// tmux's shape, and it is cheaper than a second call to join against.
///
/// `session_name` is **last** on purpose: a session name may contain the
/// separator, and the last field takes the rest of the line. The start
/// command is asked for as a `1`/`0` rather than as text for the same
/// reason — it is a whole command line, quotes and pipes and all, and
/// only its emptiness is being asked about.
const FORMAT: &str = "#{session_id}|#{session_created}|#{session_activity}\
|#{pane_pid}|#{?pane_start_command,1,0}|#{pane_current_command}|#{session_name}";

/// What one look at a multiplexer found.
#[derive(Debug, Default, Clone)]
pub struct Listing {
    pub rows: Vec<Multiplexed>,
    /// Sessions tmux named that khor could not describe: a line in a
    /// shape this version does not parse, or a session whose every pane
    /// has left the process table.
    ///
    /// **Not the same thing as "no tmux".** A machine without tmux, or
    /// with tmux and no server, reports zero rows and zero here, because
    /// zero rows is then the truth. This counts only the case where tmux
    /// answered and khor could not read the answer — which is the
    /// "适配器过时" signal the CLI prints, said about a program instead
    /// of about a file.
    pub unmapped: usize,
}

/// One tmux server.
pub struct Tmux {
    /// `-L <name>`. `None` is the user's default server, which is the
    /// only one production ever touches; tests name their own so that
    /// they never see, and can never disturb, the user's sessions.
    socket: Option<String>,
    /// Where to look for the binary, in order. Resolved once at
    /// construction rather than per call: `KHOR_TMUX` read inside
    /// [`Tmux::sweep`] would make every test that sets it a hazard for
    /// every test running beside it.
    binaries: Vec<String>,
}

impl Tmux {
    /// The server `tmux` talks to with no arguments — the user's.
    pub fn default_server() -> Tmux {
        Tmux { socket: None, binaries: Tmux::binaries() }
    }

    /// A server of one's own (`tmux -L <name>`).
    pub fn on_socket(name: &str) -> Tmux {
        Tmux { socket: Some(name.to_owned()), binaries: Tmux::binaries() }
    }

    /// This one binary and no fallbacks. How the "there is no tmux on
    /// this machine" path is exercised without uninstalling tmux — it is
    /// the same path, [`Tmux::spawn`] finding nothing to start.
    pub fn at_binary(path: &str) -> Tmux {
        Tmux { socket: None, binaries: vec![path.to_owned()] }
    }

    /// The command a khor host runs to show this server's session
    /// `target` without disturbing whoever is already looking at it:
    /// a **grouped** session (`new-session -t`) gets its own client and
    /// its own size, so attaching from the app never squeezes the user's
    /// real tmux client to the smaller window — plain `attach` does
    /// exactly that. `destroy-unattached` cleans the grouped session up
    /// the moment our client detaches (the host dying detaches it), so
    /// nothing lingers in the user's session list.
    ///
    /// Runs through the same binary and socket the sweep used — an
    /// attach against a different server than the one that listed the
    /// session would "succeed" against the wrong world.
    pub fn grouped_attach_argv(&self, target: &str) -> Vec<String> {
        let mut argv = vec![self.binaries.first().cloned().unwrap_or_else(|| "tmux".into())];
        if let Some(sock) = &self.socket {
            argv.push("-L".into());
            argv.push(sock.clone());
        }
        for a in ["new-session", "-t", target, ";", "set-option", "destroy-unattached", "on"] {
            argv.push(a.into());
        }
        argv
    }

    fn binaries() -> Vec<String> {
        match std::env::var("KHOR_TMUX") {
            Ok(p) if !p.is_empty() => vec![p],
            _ => CANDIDATES.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    /// Every session on this server, each with the pids it holds.
    pub fn sweep(&self, procs: &Procs) -> Listing {
        let mut listing = Listing::default();
        // No tmux, no server, or a server that would not answer in time:
        // zero rows and nothing counted. Khor never asked for tmux.
        let Some(text) = self.list_panes() else {
            return listing;
        };
        // Panes arrive grouped by nothing in particular, so the session
        // is assembled by id and the order of first sight is kept —
        // tmux lists in its own order and khor has no better one.
        let mut order: Vec<String> = Vec::new();
        let mut sessions: std::collections::BTreeMap<String, Gathering> =
            std::collections::BTreeMap::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let Some(pane) = Pane::parse(line) else {
                // A line in a shape this version does not understand.
                // Counted against the session it names — except that its
                // name is exactly what did not parse, so it is counted
                // on its own.
                listing.unmapped += 1;
                continue;
            };
            let entry = sessions.entry(pane.session_id.clone()).or_insert_with(|| {
                order.push(pane.session_id.clone());
                Gathering::new(&pane)
            });
            entry.absorb(&pane, procs);
        }
        for id in order {
            let Some(g) = sessions.remove(&id) else { continue };
            match g.word {
                Some(word) => listing.rows.push(g.into_row(word, procs)),
                // Every pane gone from the process table: tmux still
                // names the session, khor cannot say a thing about it.
                None => listing.unmapped += 1,
            }
        }
        listing
    }

    /// The raw `list-panes` output, or `None` for every way of not
    /// getting one — binary absent, no server, non-zero exit, a wedged
    /// call. They are one outcome here because they have one meaning:
    /// khor has no tmux rows to add.
    fn list_panes(&self) -> Option<String> {
        let mut child = self.spawn()?;
        let deadline = Instant::now() + CALL_DEADLINE;
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => break,
                Ok(Some(_)) => return None,
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(_) => return None,
            }
        }
        // Read after the exit rather than during it, which is safe only
        // because the answer is small: one line per pane, ~55 bytes.
        // Beyond a pipe buffer (~1100 panes) tmux would block writing,
        // never exit, and be killed at the deadline — no tmux rows on a
        // machine with a thousand panes, which is a boundary worth
        // writing down and not worth a reader thread.
        let mut out = String::new();
        child.stdout.take()?.read_to_string(&mut out).ok()?;
        Some(out)
    }

    /// Starts `tmux list-panes` at the first candidate that exists.
    ///
    /// Only a missing binary moves on to the next one. A tmux that
    /// started and failed has answered — "no server running" is an
    /// answer — and trying `/usr/bin/tmux` after it would be asking a
    /// different program the same question.
    fn spawn(&self) -> Option<std::process::Child> {
        for binary in &self.binaries {
            let mut c = Command::new(binary);
            if let Some(sock) = &self.socket {
                c.arg("-L").arg(sock);
            }
            c.arg("list-panes")
                .arg("-a")
                .arg("-F")
                .arg(FORMAT)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            match c.spawn() {
                Ok(child) => return Some(child),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => return None,
            }
        }
        None
    }
}

/// One pane, as tmux describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pane {
    session_id: String,
    session_name: String,
    created_secs: i64,
    activity_secs: i64,
    pid: u32,
    /// tmux was given a command to run in this pane, rather than the
    /// user's shell.
    started_with_command: bool,
    current_command: String,
}

impl Pane {
    fn parse(line: &str) -> Option<Pane> {
        let f: Vec<&str> = line.splitn(7, '|').collect();
        let [id, created, activity, pid, started, current, name] = f[..] else {
            return None;
        };
        Some(Pane {
            session_id: id.to_owned(),
            session_name: name.to_owned(),
            created_secs: created.parse().ok()?,
            activity_secs: activity.parse().ok()?,
            pid: pid.parse().ok()?,
            started_with_command: started == "1",
            current_command: current.to_owned(),
        })
    }
}

/// A session under construction, as its panes arrive.
struct Gathering {
    session_id: String,
    title: String,
    created_secs: i64,
    activity_secs: i64,
    pane_pids: Vec<u32>,
    /// The busiest word any pane has given so far. `None` while no pane
    /// has given one.
    word: Option<State>,
}

impl Gathering {
    fn new(first: &Pane) -> Gathering {
        Gathering {
            session_id: first.session_id.clone(),
            title: first.session_name.clone(),
            created_secs: first.created_secs,
            activity_secs: first.activity_secs,
            pane_pids: Vec::new(),
            word: None,
        }
    }

    fn absorb(&mut self, pane: &Pane, procs: &Procs) {
        self.pane_pids.push(pane.pid);
        self.activity_secs = self.activity_secs.max(pane.activity_secs);
        // 忙碌 wins: a session with one pane at a prompt and one running
        // a build is a session with something running in it. Reducing
        // the other way would let the idle pane hide the work.
        match word_of(pane, procs) {
            Some(State::Busy) => self.word = Some(State::Busy),
            Some(w) if self.word.is_none() => self.word = Some(w),
            _ => {}
        }
    }

    fn into_row(self, word: State, procs: &Procs) -> Multiplexed {
        Multiplexed {
            found: Found {
                category: khor_core::category::SHELL,
                kind: khor_core::kind::SHELL,
                sighting: Sighting {
                    vendor_session_id: leaf_of(&self.session_id, self.created_secs),
                    title: self.title,
                    word,
                    // tmux's own clock for the session: when it last saw
                    // anything happen in it. Not the instant the word
                    // flipped — tmux does not record that — but the
                    // closest thing it has, and the only one that does
                    // not come from khor's own clock.
                    at_ms: self.activity_secs.saturating_mul(1000),
                    // A multiplexer session is not a process, so it
                    // accounts for none. What it *holds* is the answer
                    // to the question these pids would have been asked,
                    // and that is carried below.
                    pids: Vec::new(),
                },
            },
            holds: procs.subtree(&self.pane_pids),
        }
    }
}

/// The word one pane is worth. `None` = tmux named a pane whose process
/// khor cannot find, so there is nothing to read a word off.
fn word_of(pane: &Pane, procs: &Procs) -> Option<State> {
    let p: &Proc = procs.get(pane.pid)?;
    if pane.started_with_command {
        return Some(State::Busy);
    }
    Some(if same_program(&pane.current_command, &p.name) {
        State::Idle
    } else {
        State::Busy
    })
}

/// Whether two names are the same program.
///
/// A login shell wears a leading `-` by convention and two programs
/// disagree about whether to show it: `ps` prints `-zsh` for every pane
/// on this machine, and if the process table ever answered that way
/// while tmux said `zsh`, **every shell pane would read 忙碌 forever**.
///
/// **Not what this machine does, and that is worth writing down rather
/// than implying.** Measured 2026-08-16 across all nine panes on the
/// user's server: sysinfo reports the executable name, `zsh`, with no
/// dash, matching tmux exactly. So this normalisation guards a
/// convention that exists in the neighbourhood rather than a case
/// observed here — kept because it costs one call and the failure it
/// would prevent is silent and total.
fn same_program(a: &str, b: &str) -> bool {
    a.trim_start_matches('-') == b.trim_start_matches('-')
}

/// The stable part of a tmux session's identity.
///
/// **Not the name.** A tmux session can be renamed and usually is, while
/// a pin (`khor_sync::pins`) is a fact about the session itself — keyed
/// by a name it would follow the name to whatever session took it.
///
/// `$120` alone would not do either: tmux numbers sessions from zero
/// again every time its server restarts, so a pin left on `$0` would
/// land on an unrelated session weeks later. The creation instant closes
/// that, costs one field khor already asks for, and never changes for a
/// session that lives.
fn leaf_of(session_id: &str, created_secs: i64) -> String {
    format!("{created_secs}-{}", session_id.trim_start_matches('$'))
}

/// Whether a khor session leaf is one [`leaf_of`] minted — the one place
/// that may take the format apart (a second parser is how two readers
/// drift). `khor open`'s leaves are pure hex with no dash
/// (`link::fresh_leaf`), so the two mints cannot collide.
pub fn is_tmux_leaf(leaf: &str) -> bool {
    tmux_target_of(leaf).is_some()
}

/// The tmux target (`$<id>`) back out of a leaf, or `None` for a leaf
/// some other mint produced.
pub fn tmux_target_of(leaf: &str) -> Option<String> {
    let (created, id) = leaf.split_once('-')?;
    if created.is_empty() || id.is_empty() {
        return None;
    }
    if !created.bytes().all(|b| b.is_ascii_digit()) || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(format!("${id}"))
}

/// The khor session id for a tmux session.
///
/// Deliberately **not** [`super::id_for`], which mints `tui/…`: that
/// function exists so a vendor's hook and its files agree on one string,
/// and tmux has no hook to agree with. A tmux session is a shell
/// (`khor_core::kind::SHELL`), which is also what stops `khor close`
/// from thinking it started it.
pub fn id_of(session_id: &str, created_secs: i64) -> SessionId {
    SessionId(format!(
        "{}/{}",
        khor_core::kind::SHELL,
        crate::live::clean_leaf(&leaf_of(session_id, created_secs))
    ))
}

/// A script that answers `list-panes` like tmux would, for tests that
/// need a multiplexer whose contents they chose.
///
/// Lives outside the test module because the rule it exists to test —
/// a session holding a listed process is not a second row — is decided
/// in `crate::live`, not here.
#[cfg(test)]
pub(crate) fn fake_tmux(tag: &str, output: &str) -> String {
    let dir =
        std::env::temp_dir().join(format!("khor-faketmux-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tmux");
    std::fs::write(&path, format!("#!/bin/sh\ncat <<'KHOREOF'\n{output}KHOREOF\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc_named(name: &str, ppid: Option<u32>) -> Proc {
        Proc { name: name.into(), started_ms: 0, cwd: None, ppid }
    }

    /// A test server of khor's own. Named after the process so two test
    /// binaries never meet, and **never without `-L`**: the user's own
    /// sessions are read-only scenery for this suite, and a missing
    /// socket argument here would kill them.
    struct Server {
        socket: String,
    }

    impl Server {
        fn start(tag: &str) -> Server {
            Server { socket: format!("khor-{tag}-{}", std::process::id()) }
        }

        fn tmux(&self, args: &[&str]) -> Option<String> {
            let out = Command::new("tmux")
                .arg("-L")
                .arg(&self.socket)
                .args(args)
                .stdin(Stdio::null())
                .output()
                .ok()?;
            String::from_utf8(out.stdout).ok()
        }

        fn at(&self) -> Tmux {
            Tmux::on_socket(&self.socket)
        }

        /// Blocks until tmux reports what the test set up, or gives up.
        ///
        /// **These tests had fixed sleeps first, and one lost a race.**
        /// On 2026-08-16 a full suite run that happened to overlap a
        /// compile saw `send-keys "sleep 300"` fail to become a running
        /// `sleep` inside 900 ms; the pane read 空闲 and the assertion
        /// failed — once, then passed eleven runs in a row, which is
        /// exactly the shape of failure this repo refuses to write off
        /// as "flaky, just re-run". A test that is only true on an idle
        /// machine is not testing khor.
        ///
        /// Waits on the same `list-panes` khor itself reads, so what it
        /// waits for is what the assertion will see.
        fn until(&self, ready: impl Fn(&str) -> bool) {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let seen = self
                    .tmux(&[
                        "list-panes",
                        "-a",
                        "-F",
                        "#{session_name}|#{pane_current_command}",
                    ])
                    .unwrap_or_default();
                if ready(&seen) || Instant::now() >= deadline {
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        /// Waits for one session's foreground command to be `want`.
        fn until_running(&self, session: &str, want: &str) {
            let line = format!("{session}|{want}");
            self.until(|seen| seen.lines().any(|l| l == line));
        }

        /// Waits for a session to exist at all. Deliberately not "waits
        /// for a `zsh` prompt": the shell here is whatever the machine
        /// running the suite uses, and naming one would make these tests
        /// quietly time out on a bash machine rather than fail on it.
        fn until_present(&self, session: &str) {
            let prefix = format!("{session}|");
            self.until(|seen| seen.lines().any(|l| l.starts_with(&prefix)));
        }

        /// Waits for a session to settle **at a prompt**.
        ///
        /// # The other half of the rule the batch above only did once
        ///
        /// `until_running` was added because "send keys, sleep 900ms,
        /// assert 忙碌" measured how busy the machine was rather than
        /// what khor read. **The idle side had the same hole and kept
        /// it**: `new-session -d` returns the moment tmux has the
        /// session, and a shell that is still running its startup files
        /// has a foreground command that is *not* the shell — which is
        /// [`word_of`]'s definition of 忙碌. So "a prompt is 空闲" was
        /// true only on a machine quiet enough for the shell to have
        /// finished. Observed once in a full-suite run 2026-08-17
        /// (`Some(Busy)` where 空闲 was expected) and green on five
        /// straight runs of the test alone, which is the signature.
        ///
        /// # Why sampling alone was not enough — the settled look-alike
        ///
        /// This wait first shipped as the sampling loop below on its
        /// own, and the same red came back with it compiled in (later
        /// the same day, same two values). Startup files run *external*
        /// commands with the shell back in the foreground **between**
        /// them, and a sample landing in such a gap is indistinguishable
        /// from a prompt; the sweep a few milliseconds later meets the
        /// next startup command and reads 忙碌. Traced live on this
        /// machine's real rc: the sampled condition held from 0.2s in,
        /// while the buffered keys below were not consumed until 0.9s.
        ///
        /// So the sample now has a precondition that is evidence rather
        /// than a look: keys typed at a pane sit in the pty's buffer,
        /// and an interactive shell reads them only once its startup
        /// files have finished — so the sentinel's product appearing on
        /// disk *is* "startup is over", by definition. The sampling
        /// loop still runs after it: the sentinel command itself, and
        /// any prompt hook that follows it, must also leave the
        /// foreground.
        ///
        /// Two boundaries, written down rather than implied. A startup
        /// file that reads stdin eats the sentinel (observed while
        /// building the red control for this fix: a `read -t` swallowed
        /// the buffered line), leaving only the deadline. And a prompt
        /// hook running external commands *after* the sentinel re-opens
        /// a far smaller gap of the same shape — never observed, and
        /// the first place to look if this test ever reds at these two
        /// values again.
        ///
        /// # Why it cannot be spelled as "wait for the shell"
        ///
        /// The same reason [`Server::until_present`] gives: the shell is
        /// whatever the machine running the suite uses. So this waits on
        /// **khor's own rule** instead — the pane's foreground command
        /// *is* the pane's own process ([`same_program`]) — which is also
        /// what makes it the right thing to wait for: it is the very
        /// comparison the assertion is about to make, read from the same
        /// `list-panes`.
        ///
        /// A pane khor cannot find a process for is not settled either:
        /// [`word_of`] answers `None` there, and a row with no word is
        /// not the row this waits for.
        fn until_at_a_prompt(&self, session: &str) {
            let marker = std::env::temp_dir()
                .join(format!("khor-prompt-{}-{session}", std::process::id()));
            let _ = std::fs::remove_file(&marker);
            self.tmux(&[
                "send-keys",
                "-t",
                session,
                &format!("touch '{}'", marker.display()),
                "Enter",
            ]);
            let rc_deadline = Instant::now() + Duration::from_secs(10);
            while !marker.exists() && Instant::now() < rc_deadline {
                std::thread::sleep(Duration::from_millis(25));
            }
            let _ = std::fs::remove_file(&marker);
            let prefix = format!("{session}|");
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let seen = self
                    .tmux(&[
                        "list-panes",
                        "-a",
                        "-F",
                        "#{session_name}|#{pane_pid}|#{pane_current_command}",
                    ])
                    .unwrap_or_default();
                let procs = Procs::snapshot();
                let mut panes = seen.lines().filter(|l| l.starts_with(&prefix)).peekable();
                // `all` over nothing is true, so the session has to be
                // there before its panes can be settled.
                let settled = panes.peek().is_some()
                    && panes.all(|l| {
                        let mut parts = l.split('|').skip(1);
                        let Some(pid) = parts.next().and_then(|p| p.parse::<u32>().ok()) else {
                            return false;
                        };
                        let Some(command) = parts.next() else { return false };
                        procs.get(pid).is_some_and(|p| same_program(command, &p.name))
                    });
                if settled || Instant::now() >= deadline {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    impl Drop for Server {
        fn drop(&mut self) {
            self.tmux(&["kill-server"]);
            // The socket file survives `kill-server`, and one per test
            // run accumulates in /tmp forever. Same discipline as
            // closing a dev server's port: what a test starts, it takes
            // away.
            let dir = std::env::var("TMUX_TMPDIR").unwrap_or_else(|_| "/tmp".into());
            #[cfg(unix)]
            let uid = unsafe { libc::getuid() };
            #[cfg(not(unix))]
            let uid = 0;
            let _ = std::fs::remove_file(
                std::path::Path::new(&dir).join(format!("tmux-{uid}")).join(&self.socket),
            );
        }
    }

    fn have_tmux() -> bool {
        Command::new("tmux")
            .arg("-V")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    /// **The promise, stated as a test**: a machine with no tmux gets no
    /// rows, no count, and no error. Written against a binary name that
    /// cannot exist rather than by uninstalling tmux, which is the same
    /// code path — [`Tmux::spawn`] returning `None`.
    #[test]
    fn no_tmux_is_no_rows_and_no_complaint() {
        let listing =
            Tmux::at_binary("/nonexistent/khor-test/tmux").sweep(&Procs::default());
        assert_eq!(listing.rows.len(), 0);
        assert_eq!(
            listing.unmapped, 0,
            "khor never asked for tmux; not having it is not khor being out of date"
        );
    }

    /// A tmux that runs and says "no server": also nothing, also not a
    /// complaint. Separated from the test above because they fail
    /// differently — one never starts, one exits non-zero — and one
    /// covering both would pass while the other path was broken.
    #[test]
    fn a_server_that_is_not_running_is_no_rows_and_no_complaint() {
        if !have_tmux() {
            return;
        }
        let quiet = Tmux::on_socket(&format!("khor-absent-{}", std::process::id()));
        let listing = quiet.sweep(&Procs::default());
        assert_eq!(listing.rows.len(), 0);
        assert_eq!(listing.unmapped, 0);
    }

    /// The flagship, on a real server of khor's own: a shell sitting at
    /// a prompt is 空闲, a shell running something is 忙碌, and a pane
    /// tmux was handed a command is 忙碌 too.
    ///
    /// The process table is the real one here, because the pids are
    /// real — this is the one test that proves the parse, the join and
    /// the word all line up against the program as shipped.
    #[test]
    fn a_real_server_gives_the_two_words_it_can() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("words");
        s.tmux(&["new-session", "-d", "-s", "idle"]);
        s.tmux(&["new-session", "-d", "-s", "cmd", "sleep 300"]);
        s.tmux(&["new-session", "-d", "-s", "working"]);
        s.until_present("working");
        s.tmux(&["send-keys", "-t", "working", "sleep 300", "Enter"]);
        s.until_running("working", "sleep");
        // …and the idle one has to have finished starting up, or "a
        // prompt is 空闲" is a claim about how quiet the machine is.
        s.until_at_a_prompt("idle");

        let procs = Procs::snapshot();
        let listing = s.at().sweep(&procs);
        let word = |name: &str| {
            listing
                .rows
                .iter()
                .find(|m| m.found.sighting.title == name)
                .map(|m| m.found.sighting.word)
        };
        assert_eq!(listing.unmapped, 0);
        assert_eq!(word("idle"), Some(State::Idle), "a prompt is 空闲");
        assert_eq!(word("working"), Some(State::Busy), "a command in a shell is 忙碌");
        assert_eq!(
            word("cmd"),
            Some(State::Busy),
            "a pane tmux started on a command is that command, and it is running"
        );
        for m in &listing.rows {
            assert_eq!(m.found.category, khor_core::category::SHELL);
            assert_eq!(m.found.kind, khor_core::kind::SHELL);
        }
    }

    /// A session holds its panes **and everything under them** — which
    /// is the only reason the duplicate rule can work, because an agent
    /// is a grandchild of the pane, not the pane.
    #[test]
    fn a_session_holds_what_is_running_inside_it() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("holds");
        s.tmux(&["new-session", "-d", "-s", "deep"]);
        s.until_present("deep");
        s.tmux(&["send-keys", "-t", "deep", "sleep 300", "Enter"]);
        s.until_running("deep", "sleep");

        let procs = Procs::snapshot();
        let listing = s.at().sweep(&procs);
        assert_eq!(listing.rows.len(), 1);
        let held = &listing.rows[0].holds;
        let pane_pid: u32 = s
            .tmux(&["list-panes", "-t", "deep", "-F", "#{pane_pid}"])
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(held.contains(&pane_pid), "the pane itself");
        assert!(
            held.iter().any(|p| procs.get(*p).is_some_and(|q| q.name == "sleep")),
            "and the sleep the shell started, which is a child of the pane, not the pane"
        );
    }

    /// Two panes, one session, one row — and the busy one decides.
    #[test]
    fn a_session_with_two_panes_is_one_row_wearing_the_busier_word() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("panes");
        s.tmux(&["new-session", "-d", "-s", "split"]);
        s.until_present("split");
        s.tmux(&["split-window", "-t", "split"]);
        s.until(|seen| seen.lines().filter(|l| l.starts_with("split|")).count() == 2);
        s.tmux(&["send-keys", "-t", "split.1", "sleep 300", "Enter"]);
        s.until(|seen| seen.lines().any(|l| l == "split|sleep"));

        let listing = s.at().sweep(&Procs::snapshot());
        assert_eq!(listing.rows.len(), 1, "panes are not rows");
        assert_eq!(
            listing.rows[0].found.sighting.word,
            State::Busy,
            "one pane at a prompt must not hide the one doing work"
        );
    }

    /// The word rule, against a table with no real processes in it, so
    /// that both branches are reachable on any machine.
    #[test]
    fn the_word_reads_the_foreground_and_refuses_when_there_is_none() {
        let pane = |pid: u32, started: bool, current: &str| Pane {
            session_id: "$1".into(),
            session_name: "s".into(),
            created_secs: 100,
            activity_secs: 100,
            pid,
            started_with_command: started,
            current_command: current.into(),
        };
        let procs = Procs::of([
            // Both spellings of the same shell. `zsh` is what sysinfo
            // actually reports here (measured, see `same_program`);
            // `-zsh` is the convention `ps` uses, and the two must not
            // read differently or a shell pane would sit at 忙碌 for the
            // life of the machine.
            (10, proc_named("zsh", Some(1))),
            (12, proc_named("-zsh", Some(1))),
            (11, proc_named("sleep", Some(1))),
        ]);
        assert_eq!(word_of(&pane(10, false, "zsh"), &procs), Some(State::Idle));
        assert_eq!(
            word_of(&pane(12, false, "zsh"), &procs),
            Some(State::Idle),
            "the login shell's dash is a convention, not a different program"
        );
        assert_eq!(word_of(&pane(10, false, "cargo"), &procs), Some(State::Busy));
        assert_eq!(
            word_of(&pane(11, true, "sleep"), &procs),
            Some(State::Busy),
            "a pane that IS the command must not read as its own prompt"
        );
        assert_eq!(
            word_of(&pane(99, false, "zsh"), &procs),
            None,
            "no process, no word — never a guessed one"
        );
    }

    /// The four words tmux must never spell. A property assertion
    /// ("the word is one of the six") is blind to this; enumerating the
    /// forbidden ones is not.
    #[test]
    fn tmux_never_spells_a_word_it_cannot_know() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("narrow");
        s.tmux(&["new-session", "-d", "-s", "one"]);
        s.tmux(&["new-session", "-d", "-s", "two", "sleep 300"]);
        s.until(|seen| {
            seen.lines().any(|l| l.starts_with("one|")) && seen.lines().any(|l| l == "two|sleep")
        });
        let listing = s.at().sweep(&Procs::snapshot());
        assert_eq!(listing.rows.len(), 2, "the control: it did find them");
        for m in &listing.rows {
            let w = m.found.sighting.word;
            assert!(
                matches!(w, State::Busy | State::Idle),
                "tmux has no turn, no approval, no error and no file that \
                 outlives a session — {w:?} would have to have been invented"
            );
        }
    }

    /// A session name may hold the separator, which is the whole reason
    /// it is the last field.
    #[test]
    fn the_name_takes_the_rest_of_the_line() {
        assert!(Pane::parse("$1|100|100|10|0|zsh").is_none(), "a field short");
        assert!(Pane::parse("$1|100|100|notapid|0|zsh|alpha").is_none());
        assert_eq!(
            Pane::parse("$1|100|100|10|0|zsh|has|pipes|in|it").unwrap().session_name,
            "has|pipes|in|it"
        );
    }

    /// A tmux khor cannot read is counted, and the session beside it
    /// still becomes a row — the "prove the finder was working when it
    /// declined" rule, and the only way to see the count at all.
    ///
    /// Driven through a stand-in binary rather than a real tmux: the
    /// case being tested is a **future** tmux whose output khor does not
    /// understand, and no installed tmux can be asked to be one. What is
    /// under test is khor's side of that day, which is exactly the code
    /// this reaches.
    #[test]
    fn output_this_version_cannot_read_is_counted_and_stops_nothing() {
        let fake = fake_tmux(
            "fmt",
            "$1|100|100|10|0|zsh|readable\n\
             $2|from-a-later-tmux\n\
             $3|200|200|11|0|zsh|also-readable\n",
        );
        let procs = Procs::of([
            (10, proc_named("-zsh", Some(1))),
            (11, proc_named("-zsh", Some(1))),
        ]);
        let listing = Tmux::at_binary(&fake).sweep(&procs);
        let mut titles: Vec<&str> =
            listing.rows.iter().map(|m| m.found.sighting.title.as_str()).collect();
        titles.sort_unstable();
        assert_eq!(
            titles,
            vec!["also-readable", "readable"],
            "the ones it understood are still rows"
        );
        assert_eq!(listing.unmapped, 1, "and the one it did not is counted, not dropped");
    }

    /// A session whose panes have all left the process table: tmux still
    /// names it, khor cannot say a word about it, so it is counted
    /// rather than shown with a guessed one.
    #[test]
    fn a_session_with_nothing_running_in_it_is_counted_not_guessed_at() {
        let fake = fake_tmux("gone", "$1|100|100|424242|0|zsh|ghost\n");
        let listing = Tmux::at_binary(&fake).sweep(&Procs::default());
        assert_eq!(listing.rows.len(), 0);
        assert_eq!(listing.unmapped, 1);

        // The control: the same line against a table that vouches for
        // the pane is an ordinary row, so the absence above came from
        // the missing process and not from the line being unreadable.
        let procs = Procs::of([(424242, proc_named("-zsh", Some(1)))]);
        let listing = Tmux::at_binary(&fake).sweep(&procs);
        assert_eq!(listing.rows.len(), 1);
        assert_eq!(listing.unmapped, 0);
    }

    /// The id is the session's, not the name's, and survives a rename.
    #[test]
    fn the_id_outlives_a_rename_and_a_server_that_starts_counting_again() {
        assert_eq!(id_of("$120", 1_786_245_338).0, "shell/1786245338-120");
        assert_eq!(
            id_of("$120", 1_786_245_338),
            id_of("$120", 1_786_245_338),
            "the same session is the same id however it is named now"
        );
        assert_ne!(
            id_of("$0", 1_786_245_338),
            id_of("$0", 1_786_300_000),
            "a fresh server counting from zero again is not the old session"
        );
    }

    /// The whole duplicate rule rests on parentage being in the
    /// snapshot at all. That sysinfo fills it in under
    /// `ProcessRefreshKind::nothing()` is an assumption, and an
    /// assumption load-bearing enough to be an assertion rather than a
    /// comment: without it `holds` would be only the pane pids, every
    /// agent would be missed, and every tmux session would show twice.
    #[test]
    fn the_process_table_knows_who_started_what() {
        let procs = Procs::snapshot();
        let me = std::process::id();
        assert!(
            procs.get(me).and_then(|p| p.ppid).is_some(),
            "this test process has a parent and the snapshot must know it"
        );
        assert!(
            procs.subtree(&[me]).contains(&me),
            "a subtree includes its own root, which is what lets a bare pane match"
        );
    }
}
