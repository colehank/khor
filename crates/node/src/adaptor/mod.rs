//! Vendor adaptors: turning an agent's own files into khor sessions.
//!
//! # Why the seam is cut here, and why it looks like this
//!
//! The ledger's trigger was "cut it when the second vendor arrives, from
//! two real implementations" (the [`crate::KindSurface`] precedent: that
//! trait was cut at the third kind, not guessed at the first). Codex is
//! the second vendor, so the cut is due. What the three real
//! implementations — claude-on-disk, codex-on-disk, and the claude hook
//! that predates both — actually share is narrower than it first looks:
//!
//! 1. They turn **vendor-specific evidence into a khor session id and one
//!    of the six words**, and they must agree on the id or the same
//!    session shows up twice ([`id_for`] is that agreement, and it is why
//!    the hook moved into [`claude`] instead of staying in `live.rs`).
//! 2. They must **refuse to guess**. That is why a mapping returns
//!    `Option<State>` and an unmapped sighting becomes no row at all
//!    plus a count ([`Sweep::unmapped`]) — a word we cannot derive must
//!    not land on its neighbour (docs/SESSION.md; a "waiting" claude
//!    painted 空闲 says "nothing is happening" while it sits on a prompt).
//!
//! What they do **not** share, and what the trait therefore refuses to
//! model: how they are triggered (the hook is pushed, disk is pulled),
//! whether a pid is knowable, and which words are reachable at all.
//! Claude's status file cannot say 完成 and codex's rollout cannot say
//! 待批; a trait with a method per word would have frozen both gaps into
//! ceremony. So [`Adaptor`] carries exactly one method, the pull, and the
//! push side stays a plain function in the vendor's own module.
//!
//! # The list is not a graveyard
//!
//! A discovered row must be backed by a **live process on this machine**.
//! The vendor's files say *what* a session is; a PID or an owned writer
//! lock establishes liveness. Codex daemons can host several loaded
//! threads without exposing their frontend PIDs. History alone is never
//! sufficient evidence for a row.
//!
//! The one exception is narrow and is the crash signal: a vendor file
//! that names its own pid can outlive that pid, and since the vendor
//! deletes the file on a clean exit, a surviving file means an ending
//! nobody recorded — 失败, by the same missing-ending rule `live.rs`
//! applies to its own registry. It counts only while the crash is still
//! fresh ([`CRASH_GRACE`]); a crash from days ago is history too.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use khor_core::{SessionId, State};

pub mod claude;
pub mod codex;
pub mod tmux;

/// How long a crashed session stays in the list.
///
/// A crash is worth showing to someone who stepped away for a meal, not
/// to someone returning from a week off — by then they have already
/// restarted or moved on, and a list that keeps one row per crash
/// forever is the graveyard this module refuses to be. The row holds no
/// badge either way (失败 sinks; docs/UX.md 角标可归零), so the cost of
/// the window being slightly wrong is a row too many or too few, never a
/// notification that cannot be cleared.
pub const CRASH_GRACE_MS: i64 = 24 * 60 * 60 * 1000;

/// A process may only be matched to a session that claims it if the two
/// agree on when it started: pid numbers get recycled, and a recycled pid
/// would otherwise resurrect a dead session as a live one. Claude writes
/// its status file a beat after the process itself starts (2-3s measured
/// across six live sessions on 2.1.226-2.1.233), so the tolerance is
/// generous in the direction that matters and still far below the gap
/// that a genuine pid reuse would show.
const START_TOLERANCE_MS: i64 = 60_000;

/// One vendor's sighting of one of its sessions, already reduced to what
/// the session surface needs (docs/SESSION.md 五问).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sighting {
    /// The vendor's own id for this session. This is the join key with
    /// hook-registered rows, so it must be the same string the vendor
    /// hands its hooks.
    pub vendor_session_id: String,
    pub title: String,
    /// Already one of the six. An evidence state this adaptor cannot map
    /// never reaches here — it is counted instead ([`Sweep::unmapped`]).
    pub word: State,
    /// When the word became true, by the vendor's own clock.
    pub at_ms: i64,
    /// Every **live** process this row accounts for.
    ///
    /// Not used to paint the row — liveness was already decided by the
    /// time a sighting exists. It is here so that a second source
    /// looking at the same machine can tell it is looking at a process
    /// somebody already listed: [`tmux`] reports whole multiplexer
    /// sessions, and one holding a claude nobody would want listed twice
    /// (docs/SESSION.md 同源去重).
    ///
    /// **Usually one, and the exception is the reason this is a list.**
    /// `claude -r` resumes a session into a second process while the
    /// first still runs: two processes, one session, one row — and the
    /// row has to account for *both*, or the tmux session holding the
    /// one that lost the merge shows up as a shell nobody could name.
    /// Found by running the check against this machine, where exactly
    /// that had happened (2026-08-16).
    ///
    /// Empty for a crashed row, or when the vendor proves liveness with
    /// a writer lock but exposes no PID. An empty list cannot identify a
    /// terminal host; it does not itself mean the session has died.
    pub pids: Vec<u32>,
    /// **The working directory this session is in**, when the vendor
    /// records one. Raw, exactly as it was read — the fold up to a
    /// project root is a judgment, and it is made once at the row
    /// (`khor_node::place`), not once per adaptor.
    ///
    /// `None` where the vendor keeps no such note. Never guessed from a
    /// title: a title is a name somebody chose and a directory is a
    /// fact, and the whole point of the field is that it is the second
    /// kind of thing.
    pub cwd: Option<String>,
}

/// What one sweep of one vendor found.
#[derive(Debug, Default, Clone)]
pub struct Sweep {
    pub rows: Vec<Sighting>,
    /// Live sessions this adaptor saw but could not read: an unknown
    /// status word, a file layout from a version it does not understand,
    /// a rollout it could not join. Never silently zero — this is the
    /// "适配器过时" signal, and it is the honest alternative to putting a
    /// guessed word on the row.
    pub unmapped: usize,
}

/// One sighting plus the category its row will carry.
///
/// **The adaptor has nowhere to write this.** A row's category is "whose
/// session is this", and the only honest answer is the name of whoever
/// recognised it — so it is attached here, out of reach of [`Sweep`],
/// rather than being a field an adaptor fills in and could fill in
/// wrongly. The type is what keeps "who recognised it" and "whose it is"
/// from drifting apart; a comment asking adaptors to agree would not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub category: &'static str,
    /// What shape of session this is (`khor_core::kind`). A vendor's is
    /// an agent TUI; a bare multiplexer session is a shell. Carried here
    /// rather than derived at the row, because the only party that knows
    /// is the one that recognised it.
    pub kind: &'static str,
    pub sighting: Sighting,
}

impl Found {
    /// The khor session id this row lands on.
    ///
    /// The one spelling. A vendor's rows come out `tui/…`, which is
    /// exactly what [`id_for`] mints for the hook path — that agreement
    /// is what keeps a hooked-and-discovered session to one row, and it
    /// holds here because a vendor's [`Found::kind`] is
    /// `khor_core::kind::TUI`.
    pub fn id(&self) -> SessionId {
        SessionId(format!(
            "{}/{}",
            self.kind,
            crate::live::clean_leaf(&self.sighting.vendor_session_id)
        ))
    }
}

/// A session found by something that **holds other processes** rather
/// than being one: a terminal multiplexer's session (see [`tmux`]).
///
/// It is kept apart from [`Findings::rows`] until one question is
/// answered — is anything inside it already a row? A tmux session
/// running claude is claude's row, and listing it twice would make one
/// running process look like two. The pids it holds travel with it so
/// that the answer is a set intersection wherever it is asked, rather
/// than a second walk of the process table (`crate::live::LiveKind::rows`
/// asks, because the registry's own pids count too).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Multiplexed {
    pub found: Found,
    /// Every live pid inside this session — its panes and everything
    /// descended from them.
    pub holds: Vec<u32>,
    /// What a pane is running, by tmux's own `pane_current_command`, when
    /// it is not the pane's shell (roadmap 2.1: a tmux row's activity when
    /// no shell integration marks reach khor).
    pub running: Option<String>,
    /// Each pane's own reading, for a row whose front pane khor knows
    /// (item 21: a tmux client's ledger names it) but which sent no marks.
    pub panes: Vec<PaneReading>,
}

/// One tmux pane as tmux reads it: what it runs when that is not its
/// shell, and where it stands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneReading {
    /// `%3`.
    pub id: String,
    pub running: Option<String>,
    pub cwd: Option<String>,
}

/// What one sweep of every vendor found, each row already attributed.
#[derive(Debug, Default, Clone)]
pub struct Findings {
    pub rows: Vec<Found>,
    /// Multiplexer sessions, still to be checked against what else is
    /// listed. See [`Multiplexed`].
    pub multiplexed: Vec<Multiplexed>,
    /// Summed across vendors; see [`Sweep::unmapped`].
    pub unmapped: usize,
    /// The process table this sweep was answered from — handed on so a
    /// reader asking **which host a sighting sits in** asks the same
    /// moment the sighting came from (`crate::live::LiveKind::rows`
    /// walks a khor host's child subtree with it). A second snapshot
    /// would be a second moment, and a process that started between the
    /// two would be alive in one answer and absent from the other.
    pub procs: Procs,
}

impl Findings {
    fn absorb(&mut self, vendor: &'static str, other: Sweep) {
        self.rows
            .extend(other.rows.into_iter().map(|sighting| Found {
                category: vendor,
                kind: khor_core::kind::TUI,
                sighting,
            }));
        self.unmapped += other.unmapped;
    }
}

/// One vendor's disk surface.
///
/// Deliberately one method. See the module head for what was left out
/// and why.
pub trait Adaptor: Send + Sync {
    /// The vendor's name, for diagnostics. Not part of the session id:
    /// a session's id must survive khor learning to read a second file
    /// from the same vendor.
    fn vendor(&self) -> &'static str;

    /// Every session of this vendor that a live process still backs,
    /// plus any that crashed recently enough to matter.
    fn sweep(&self, procs: &Procs) -> Sweep;
}

/// The khor session id for a vendor's session id.
///
/// The hook path and the disk path both come through here; if they ever
/// disagree the same claude session appears twice, once registered and
/// once discovered. `tui` is the kind because that is what these are —
/// an agent TUI, whichever vendor built it (docs/SESSION.md 六词映射).
/// 16 random bytes worn as a proper v4 uuid — claude refuses
/// `--session-id` values that are not valid uuids, so the version and
/// variant nibbles are set rather than left to chance.
///
/// Shared because **both** ways of opening a session mint one: the shim
/// hands it to `--session-id` on the protocol side, and the terminal
/// form does the same so its row can wear the vendor's own id from
/// birth rather than waiting for a hook to say what it is.
pub fn uuid_v4() -> Result<String, String> {
    let hex = crate::link::fresh_hex()?;
    let b: Vec<char> = hex.chars().collect();
    let mut s = String::with_capacity(36);
    for (i, c) in b.iter().enumerate() {
        match i {
            8 | 12 | 16 | 20 => s.push('-'),
            _ => {}
        }
        match i {
            12 => s.push('4'),
            16 => s.push(['8', '9', 'a', 'b'][(c.to_digit(16).unwrap_or(0) % 4) as usize]),
            _ => s.push(*c),
        }
    }
    Ok(s)
}

pub fn id_for(vendor_session_id: &str) -> SessionId {
    SessionId(format!(
        "{}/{}",
        khor_core::kind::TUI,
        crate::live::clean_leaf(vendor_session_id)
    ))
}

// ── the process table, injectable ───────────────────────────

/// One running process, in the two respects an adaptor needs: is it the
/// program we are looking for, and did it start when the session says it
/// did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proc {
    pub name: String,
    /// Process start, ms since the epoch.
    pub started_ms: i64,
    pub cwd: Option<PathBuf>,
    /// Who started it. The only way to ask "is this process running
    /// **inside** that one" — which is how a multiplexer session and the
    /// agent in it are recognised as one thing (see [`Multiplexed`]).
    /// Reading the command line instead would be the guess
    /// [`crate::live::LiveKind::register`] refuses for the same reason.
    pub ppid: Option<u32>,
}

/// A snapshot of the process table.
///
/// A snapshot rather than a live query on purpose: one sweep asks about
/// many pids, and asking the OS once keeps a row's liveness and a row's
/// word from being answered by two different moments. It is also the
/// seam that lets every fixture test run without a real process
/// ([`Procs::of`]).
#[derive(Debug, Default, Clone)]
pub struct Procs {
    by_pid: BTreeMap<u32, Proc>,
    sessions: Sessions,
}

/// Where a process's POSIX session id comes from (see [`Procs::held_by`]).
#[derive(Debug, Default, Clone)]
enum Sessions {
    /// A hand-built table with no session ids: nothing is known, so
    /// nothing is excluded for its session.
    #[default]
    Unknown,
    /// The live process table: ask the OS at the moment of the question.
    Os,
    /// A hand-built table that names them.
    Given(BTreeMap<u32, i32>),
}

/// How many process-table scans this process has done. Read by the cost
/// measurement (`tests/cost.rs`) to answer how many scans **one** session
/// list costs — a question the call graph can be misread on, and was:
/// the ledger recorded a guess of "maybe several" for months.
static SNAPSHOTS: AtomicU64 = AtomicU64::new(0);

/// The running count of [`Procs::snapshot`] calls.
pub fn snapshots_taken() -> u64 {
    SNAPSHOTS.load(Ordering::Relaxed)
}

impl Procs {
    /// What is running right now.
    ///
    /// Two passes on purpose. A working directory costs the OS a
    /// separate lookup per process, and this runs on every session list
    /// — so the sweep pass asks only for names and start times, and cwd
    /// is fetched afterwards for the handful of pids that are candidate
    /// agent processes.
    ///
    /// # What it costs, measured
    ///
    /// Was on the ledger as "never measured". Measured 2026-08-16 on the
    /// development Mac (10 cores, 16 days up, **803 processes** — a busy
    /// machine, so an upper-ish bound), **debug build**, with one real
    /// `codex` running so the cwd pass is included:
    ///
    /// - one snapshot: **4.9 ms** median warm, 5.7 ms cold
    /// - one whole `Node::sessions()`: **13.2 ms** median
    /// - **one scan per `sessions()`** — counted with
    ///   [`snapshots_taken`], not read off the call graph
    /// - at the GUI's poll rate: **0.26% of a core**
    ///
    /// So no cache. A cache here would buy a quarter of a percent of one
    /// core and pay for it with a staleness window on the one thing the
    /// list exists to be right about — whether a process is still alive.
    /// **That verdict still holds, and it is the only part of the
    /// paragraph above that does** — see the correction below.
    ///
    /// # ⚠️ The `sessions()` figure above is not this machine's, 2026-09-11
    ///
    /// The snapshot numbers are sound: re-measured in release, **764
    /// processes, 3.6 ms median**, and still **one scan per list**
    /// (`snapshots_taken`). The scan is not what the list costs.
    ///
    /// **`13.2 ms` for one `Node::sessions()` is a fixture's figure**,
    /// and `tests/cost.rs` still reports it — 7.9 ms in release today —
    /// because that test builds its own empty home. Timed inside a real
    /// one (this developer's, 26 rows, a dozen chat channels):
    ///
    /// ```text
    /// sessions() total   95–131 ms
    ///   ChatKind::rows    37–53 ms
    ///   LiveKind::rows    30–58 ms   (of which the sweep is ~12: 3.6 snapshot + 7.9 tmux)
    ///   pins + titles      6–11 ms
    /// ```
    ///
    /// **So the cost is reading documents and sweeping the registry, and
    /// it scales with how much a person has in their home — which is
    /// exactly what a fixture has none of.** At the app's 2 s beat that
    /// is ~6 % of a core for the list alone, and it is most of what
    /// khor-gui burns sitting still.
    ///
    /// ⚠️ **A cost test on a fixture answers a different question than
    /// the one it is read for**, and this one was read for months as
    /// 「the list is cheap」. It is cheap on an empty home. Re-run
    /// `cargo test -p khor-node --release --test cost -- --ignored
    /// --nocapture` for the snapshot's cost, which is genuinely
    /// home-independent; for the list's, time it against a home with
    /// somebody's actual history in it.
    ///
    /// **The ledger's other worry was wrong and is corrected here**: it
    /// recorded that one `sessions()` call "might run `rows()` several
    /// times". It runs it once — `sessions` walks three kinds and only
    /// the live one sweeps. The rest of the 13.2 ms is reading documents
    /// off disk, not scanning again.
    pub fn snapshot() -> Procs {
        use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

        SNAPSHOTS.fetch_add(1, Ordering::Relaxed);
        let mut sys = System::new();
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        let mut by_pid: BTreeMap<u32, Proc> = sys
            .processes()
            .iter()
            .map(|(pid, p)| {
                (
                    pid.as_u32(),
                    Proc {
                        name: p.name().to_string_lossy().to_string(),
                        started_ms: (p.start_time() as i64).saturating_mul(1000),
                        cwd: None,
                        ppid: p.parent().map(sysinfo::Pid::as_u32),
                    },
                )
            })
            .collect();

        let wanted: Vec<Pid> = by_pid
            .iter()
            .filter(|(_, p)| process_name_matches(&p.name, codex::PROCESS_NAME))
            .map(|(pid, _)| Pid::from_u32(*pid))
            .collect();
        if !wanted.is_empty() {
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&wanted),
                true,
                ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
            );
            for pid in wanted {
                if let (Some(p), Some(slot)) = (sys.process(pid), by_pid.get_mut(&pid.as_u32())) {
                    slot.cwd = p.cwd().map(Path::to_path_buf);
                }
            }
        }
        Procs {
            by_pid,
            sessions: Sessions::Os,
        }
    }

    /// A hand-built table, for tests and for the fixture trees.
    pub fn of(procs: impl IntoIterator<Item = (u32, Proc)>) -> Procs {
        Procs {
            by_pid: procs.into_iter().collect(),
            sessions: Sessions::Unknown,
        }
    }

    /// The same table with the POSIX session id of some of its pids, for
    /// fixtures that exercise [`Procs::held_by`].
    pub fn with_sessions(mut self, sids: impl IntoIterator<Item = (u32, i32)>) -> Procs {
        self.sessions = Sessions::Given(sids.into_iter().collect());
        self
    }

    /// `pid`'s POSIX session id, when it can be read.
    fn session_of(&self, pid: u32) -> Option<i32> {
        match &self.sessions {
            Sessions::Unknown => None,
            Sessions::Given(sids) => sids.get(&pid).copied(),
            #[cfg(unix)]
            Sessions::Os => {
                // SAFETY: getsid only reads the process table.
                let sid = unsafe { libc::getsid(pid as libc::pid_t) };
                (sid > 0).then_some(sid)
            }
            #[cfg(not(unix))]
            Sessions::Os => None,
        }
    }

    /// How many processes were seen. What [`Procs::snapshot`] costs
    /// scales with this, so the cost measurement reports it alongside
    /// the time — a duration without the size it was measured at cannot
    /// be compared against a later one.
    pub fn len(&self) -> usize {
        self.by_pid.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_pid.is_empty()
    }

    /// The process at `pid`, but only if it started when the caller
    /// expects — a recycled pid number is not the process that wrote the
    /// file (see [`START_TOLERANCE_MS`]).
    pub fn alive_since(&self, pid: u32, started_ms: i64) -> Option<&Proc> {
        let p = self.by_pid.get(&pid)?;
        ((p.started_ms - started_ms).abs() <= START_TOLERANCE_MS).then_some(p)
    }

    /// One process by pid, whether or not anyone vouches for its age.
    pub fn get(&self, pid: u32) -> Option<&Proc> {
        self.by_pid.get(&pid)
    }

    /// `roots` plus everything descended from them, in this snapshot.
    ///
    /// Walked downward from a children index rather than upward from
    /// every pid: a wedged parentage cycle (a pid whose chain loops)
    /// would spin an upward walk forever, and the process table is
    /// something khor reads, not something it controls. Downward, each
    /// pid is visited at most once because it is marked before it is
    /// expanded, so a cycle costs nothing.
    pub fn subtree(&self, roots: &[u32]) -> Vec<u32> {
        let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (pid, p) in &self.by_pid {
            if let Some(parent) = p.ppid {
                children.entry(parent).or_default().push(*pid);
            }
        }
        let mut out: Vec<u32> = Vec::new();
        let mut queue: Vec<u32> = roots.to_vec();
        while let Some(pid) = queue.pop() {
            if out.contains(&pid) {
                continue;
            }
            out.push(pid);
            if let Some(kids) = children.get(&pid) {
                queue.extend(kids);
            }
        }
        out.sort_unstable();
        out
    }

    /// **What a multiplexer's panes hold**: the [`Procs::subtree`] of the
    /// pane processes, less every process that left their POSIX session.
    ///
    /// Descent alone says too much (T-34, the terminal audit). A khor
    /// host started by a serve, bridge or app that itself runs in a pane
    /// descends from that pane, yet sits on a pty of its own; seated *in*
    /// the pane, its keys went into the pane. A process is in a pane when
    /// it shares the session of the pane's own process, whose controlling
    /// terminal is the pane's tty; one that called `setsid` (every khor
    /// host, every daemon) has left it, and so has everything it starts.
    /// This is the field's line: tmux names a pane's program by the pane
    /// tty's foreground group (`pane_current_command`, `tcgetpgrp` on
    /// `pane_tty`), never by descent. A pid whose session cannot be read
    /// (a fixture's, another user's) is kept, as descent alone would.
    pub fn held_by(&self, roots: &[u32]) -> Vec<u32> {
        let theirs: Vec<i32> = roots.iter().filter_map(|&p| self.session_of(p)).collect();
        let mut out = self.subtree(roots);
        if !theirs.is_empty() {
            out.retain(|&p| self.session_of(p).is_none_or(|s| theirs.contains(&s)));
        }
        out
    }

    /// Match an executable base name, including Windows' case-insensitive
    /// `.exe` spelling. Helper names such as `codex-code-mode-host` do not match.
    pub fn named<'a>(&'a self, name: &str) -> Vec<(u32, &'a Proc)> {
        self.by_pid
            .iter()
            .filter(|(_, p)| process_name_matches(&p.name, name))
            .map(|(pid, p)| (*pid, p))
            .collect()
    }
}

fn process_name_matches(actual: &str, wanted: &str) -> bool {
    actual == wanted
        || actual.rsplit_once('.').is_some_and(|(base, extension)| {
            extension.eq_ignore_ascii_case("exe") && base.eq_ignore_ascii_case(wanted)
        })
}

// ── the set of adaptors this build knows ────────────────────

/// Every vendor khor can read, rooted at one pretend-home.
pub struct Discovery {
    adaptors: Vec<Box<dyn Adaptor>>,
    /// The terminal multiplexer, when this node is allowed to ask one.
    /// `None` everywhere a root path is the whole world — see
    /// [`Discovery::at`].
    tmux: Option<tmux::Tmux>,
    /// A fixed process table. `None` means "ask the OS", which is
    /// production; a test that pins it here has a closed world, since
    /// the root is already a parameter.
    procs: Option<Procs>,
    /// Fixed answers for [`Discovery::attached_in`], for tests.
    attached: Vec<(u32, SessionId, Option<String>)>,
}

impl Discovery {
    /// Reads nothing. The default for [`crate::live::LiveKind`], so that
    /// every registry test stays closed and reading the machine's real
    /// vendor directories is an explicit decision made in one place.
    pub fn empty() -> Discovery {
        Discovery {
            adaptors: Vec::new(),
            tmux: None,
            procs: None,
            attached: Vec::new(),
        }
    }

    /// Sweeps against this process table instead of the OS.
    pub fn with_procs(mut self, procs: Procs) -> Discovery {
        self.procs = Some(procs);
        self
    }

    /// Also ask this multiplexer. Explicit because a server is not
    /// something a root path can redirect (see [`Discovery::at`]).
    pub fn with_tmux(mut self, tmux: tmux::Tmux) -> Discovery {
        self.tmux = Some(tmux);
        self
    }

    /// Answers [`Discovery::attached`] with these instead: a shell's
    /// pid and the session row its terminal's tmux client shows.
    pub fn with_attached(mut self, attached: Vec<(u32, SessionId)>) -> Discovery {
        self.attached = attached.into_iter().map(|(p, s)| (p, s, None)).collect();
        self
    }

    /// [`Discovery::with_attached`] with the client's front pane (`%3`).
    pub fn with_attached_in(mut self, shell: u32, session: SessionId, pane: &str) -> Discovery {
        self.attached.push((shell, session, Some(pane.to_owned())));
        self
    }

    /// **The tmux session a shell's terminal is showing** (SESS-BRIEF
    /// 1b): `shell` is a khor host's child, and the answer is the row id
    /// of the session the tmux client in front of its terminal is
    /// attached to — only on the server this discovery watches, since a
    /// session id (`$3`) means nothing on another one.
    ///
    /// The lookup is [`tmux::client_in_front`] (TERM-BRIEF item 58): the
    /// client's server must be the watched one, and its session's row id
    /// is made as the sweep makes it ([`tmux::id_of`], from the session's
    /// creation time). Fixed answers ([`Discovery::with_attached`]) come
    /// first; with no server watched, nothing is asked.
    pub fn attached(&self, shell: u32) -> Option<SessionId> {
        self.attached_in(shell).map(|(session, _)| session)
    }

    /// [`Discovery::attached`], and the pane in front of that client
    /// (`%3`) where tmux named one (SESS-BRIEF 2c).
    pub fn attached_in(&self, shell: u32) -> Option<(SessionId, Option<String>)> {
        if let Some((_, row, pane)) = self.attached.iter().find(|(pid, _, _)| *pid == shell) {
            return Some((row.clone(), pane.clone()));
        }
        let watched = self.tmux.as_ref()?;
        let client = tmux::client_in_front(shell)?;
        if !watched.same_server(&client.server()) {
            return None;
        }
        let created = watched.session_created(&client.session_id)?;
        let pane = (!client.pane_id.is_empty()).then_some(client.pane_id);
        Some((tmux::id_of(&client.session_id, created), pane))
    }
    /// Adaptors reading the vendor directories under `home`.
    ///
    /// The root is a parameter, not a constant, because otherwise no
    /// test of this module can be closed: the graveyard control group
    /// and the crash rule both need a tree nobody's real agent is
    /// writing to.
    ///
    /// **No multiplexer here, and that is the same rule.** A tmux server
    /// is reached through a socket, not through this home, so a fixture
    /// tree cannot redirect it — a test built on `at()` that swept the
    /// machine's tmux would be answering with whatever the user happens
    /// to have open. Production adds it in [`Discovery::for_root`]; a
    /// test that wants one names its own server ([`with_tmux`]).
    ///
    /// [`with_tmux`]: Discovery::with_tmux
    pub fn at(home: &Path) -> Discovery {
        Discovery {
            adaptors: vec![
                Box::new(claude::Claude::at(home.join(".claude"))),
                Box::new(codex::Codex::at(home.join(".codex"))),
            ],
            tmux: None,
            procs: None,
            attached: Vec::new(),
        }
    }

    /// The vendor directories belonging to the node rooted at `root`.
    ///
    /// **The vendor home follows khor's own root**, because they are the
    /// same machine's home: a node rooted at `~` reads `~/.claude`, and a
    /// node rooted anywhere else is a second instance — a test, or the
    /// other half of a dual-instance verification — which has no claim
    /// on the real user's agents. Without that, two instances on one
    /// laptop each discover the same claude sessions and each calls
    /// itself their home, which turns a cross-device check into two
    /// devices agreeing about rows neither learned from the other.
    ///
    /// `KHOR_VENDOR_HOME` overrides it, which is how a node deliberately
    /// rooted elsewhere still reads the real agent files — the flagship
    /// verification runs exactly that way — and how a fixture tree can be
    /// driven through the CLI end to end rather than only from Rust.
    /// **The multiplexer follows the same rule, and needs it stated
    /// separately.** A tmux server is reached through a socket, so
    /// pointing khor at a different root does not point it at a
    /// different tmux — every test that opens a node under a temp
    /// directory would otherwise sweep the user's real sessions and
    /// answer with whatever they happen to have open. So it is asked
    /// only when this node *is* the real home's node, which is the same
    /// sentence as above with "the real user's agents" replaced by "the
    /// real user's terminals".
    pub fn for_root(root: &Path) -> Discovery {
        let home = vendor_home(root);
        let discovery = Discovery::at(&home);
        // The test door, KHOR_NAME's precedent: a scratch home never
        // watches the user's real tmux server, but a test (the smoke)
        // needs a server of its own to be watched — naming a socket here
        // is opting a private server in, on any home.
        if let Ok(sock) = std::env::var("KHOR_TMUX_SOCKET") {
            return discovery.with_tmux(tmux::Tmux::on_socket(&sock));
        }
        if is_real_home(&home) {
            discovery.with_tmux(tmux::Tmux::default_server())
        } else {
            discovery
        }
    }

    /// The multiplexer this discovery watches, if any — the attach
    /// bridge runs its command through the very same server and binary
    /// the sweep listed sessions from.
    pub fn tmux(&self) -> Option<&tmux::Tmux> {
        self.tmux.as_ref()
    }

    /// One sweep of every vendor.
    pub fn sweep(&self) -> Findings {
        let procs = match &self.procs {
            Some(procs) => procs.clone(),
            None => Procs::snapshot(),
        };
        let mut found = self.sweep_with(&procs);
        found.procs = procs;
        found
    }

    /// One sweep against a given process table (tests inject theirs).
    ///
    /// This is the one place a row's category is decided, and it is
    /// decided by which adaptor produced the row — see [`Found`].
    pub fn sweep_with(&self, procs: &Procs) -> Findings {
        let mut all = Findings::default();
        for a in &self.adaptors {
            all.absorb(a.vendor(), a.sweep(procs));
        }
        if let Some(t) = &self.tmux {
            let listing = t.sweep(procs);
            all.multiplexed = listing.rows;
            all.unmapped += listing.unmapped;
        }
        all
    }
}

/// Where this node's vendors live: khor's own root, unless
/// `KHOR_VENDOR_HOME` says otherwise.
///
/// One function because reading and **writing** must agree. Discovery
/// reads `<here>/.claude/sessions`; `claude::install_hooks` writes
/// `<here>/.claude/settings.json`. If those two ever answered
/// differently, `khor hooks install` would configure one claude while
/// the list watched another — and both would look fine.
pub fn vendor_home(root: &Path) -> PathBuf {
    std::env::var_os("KHOR_VENDOR_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.to_path_buf())
}

/// Whether this path is the account's own home directory.
///
/// Compared after canonicalising, because `$HOME` and a path someone
/// typed can name one directory in two spellings (a symlinked home, a
/// trailing slash) — and getting that wrong in the false direction is
/// the loud failure, not the quiet one: khor would simply not list the
/// user's terminals.
///
/// **The account's home is the user database's, not `$HOME`** (T-35, the
/// terminal audit and the TERM lane's own incident, 2026-10-03): a scratch
/// node is started with `HOME` pointed at its scratch directory as often
/// as with `KHOR_HOME`, and then `$HOME` *was* the vendor home — the
/// scratch node took itself for the account's own and watched the
/// owner's default tmux server, which let it seat a session on his pane.
/// What the system says the account's home is cannot be moved by an
/// environment variable, so a node rooted anywhere else — however its
/// `HOME` is set — watches no default server; `KHOR_TMUX_SOCKET` is the
/// one way to give it a server of its own.
fn is_real_home(path: &Path) -> bool {
    let Some(home) = account_home() else {
        return false;
    };
    let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    real(path) == real(&home)
}

/// The account's home directory as the user database records it
/// (`getpwuid_r`), never `$HOME`. Off unix, `$HOME`'s answer.
#[cfg(unix)]
fn account_home() -> Option<PathBuf> {
    use std::ffi::CStr;
    use std::os::unix::ffi::OsStrExt;
    let mut pw: libc::passwd = unsafe { std::mem::zeroed() };
    let mut out: *mut libc::passwd = std::ptr::null_mut();
    let mut buf = vec![0u8; 16 * 1024];
    // SAFETY: getpwuid_r writes into `pw` and `buf`, both owned here and
    // alive for the call; `out` is null or points at `pw`.
    let rc = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut pw,
            buf.as_mut_ptr().cast(),
            buf.len(),
            &mut out,
        )
    };
    if rc != 0 || out.is_null() || pw.pw_dir.is_null() {
        return None;
    }
    // SAFETY: a non-null pw_dir from getpwuid_r is a NUL-terminated
    // string inside `buf`.
    let dir = unsafe { CStr::from_ptr(pw.pw_dir) };
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(dir.to_bytes())))
}

#[cfg(not(unix))]
fn account_home() -> Option<PathBuf> {
    std::env::home_dir()
}

/// Reads a whole file, or nothing. Every adaptor reads vendor files that
/// the vendor is writing concurrently; an unreadable or half-written one
/// is a moment, not an error.
pub(crate) fn read_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-35: the account's home is the user database's. A directory that
    /// is not it is never the real home — whatever `$HOME` says, which a
    /// scratch node sets to its scratch directory — and the account's own
    /// is.
    #[cfg(unix)]
    #[test]
    fn the_real_home_is_the_accounts_not_whatever_home_says() {
        let scratch = std::env::temp_dir().join("khor-not-a-home");
        assert!(!is_real_home(&scratch));
        let own = account_home().expect("the user database names a home");
        assert!(is_real_home(&own));
        // Run in a child with HOME pointed at the scratch dir, the way a
        // scratch node is started: it must still not take the scratch
        // dir for the real home.
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "adaptor::tests::scratch_home_probe",
                "--nocapture",
                "--ignored",
            ])
            .env("HOME", &scratch)
            .env("KHOR_PROBE_DIR", &scratch)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("probe-real-home=false"), "{text}");
    }

    /// The child half of the test above.
    #[cfg(unix)]
    #[test]
    #[ignore]
    fn scratch_home_probe() {
        if let Some(dir) = std::env::var_os("KHOR_PROBE_DIR") {
            println!(
                "probe-real-home={}",
                is_real_home(std::path::Path::new(&dir))
            );
        }
    }

    /// **The ancestry case (T-34).** A bridge started in a tmux pane
    /// stands up a khor host, which calls `setsid` and runs a shell on a
    /// pty of its own. Both descend from the pane; only what stayed in
    /// the pane's session is in the pane. Before, `holds` was the whole
    /// subtree, the host's shell was seated in the pane, and its keys
    /// went there.
    #[test]
    fn a_pane_holds_its_session_not_everything_descended_from_it() {
        let child = |name: &str, ppid: u32| Proc {
            name: name.into(),
            started_ms: 0,
            cwd: None,
            ppid: Some(ppid),
        };
        let procs = Procs::of([
            (100, child("zsh", 1)),
            (101, child("khor-gui", 100)),
            (102, child("khor-gui", 101)),
            (103, child("zsh", 102)),
            (104, child("claude", 100)),
            (105, child("ssh", 100)),
        ])
        // 102 is the host: it left the pane's session, and its shell 103
        // is in the host's. 105's session cannot be read.
        .with_sessions([(100, 100), (101, 100), (102, 102), (103, 102), (104, 100)]);

        assert_eq!(procs.subtree(&[100]), vec![100, 101, 102, 103, 104, 105]);
        assert_eq!(
            procs.held_by(&[100]),
            vec![100, 101, 104, 105],
            "the host and its shell left the pane; an unreadable session is kept"
        );
        assert_eq!(
            Procs::of([(100, child("zsh", 1)), (102, child("khor-gui", 100))]).held_by(&[100]),
            vec![100, 102],
            "with no session ids known, descent alone decides, as before"
        );
    }

    /// The same, on the real process table: a child that calls `setsid`
    /// (as every khor host does) is no longer held by this process,
    /// while a plain child still is.
    #[cfg(unix)]
    #[test]
    fn a_child_that_called_setsid_left_its_parents_pane() {
        use std::os::unix::process::CommandExt;
        let mut plain = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let mut away = std::process::Command::new("sleep");
        away.arg("30");
        // SAFETY: setsid is async-signal-safe.
        unsafe {
            away.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        let mut away = away.spawn().unwrap();
        let held = Procs::snapshot().held_by(&[std::process::id()]);
        let (plain_pid, away_pid) = (plain.id(), away.id());
        let _ = plain.kill();
        let _ = away.kill();
        let _ = plain.wait();
        let _ = away.wait();
        assert!(
            held.contains(&plain_pid),
            "a plain child stays in its parent's session"
        );
        assert!(
            !held.contains(&away_pid),
            "a child in a session of its own is not held"
        );
    }

    fn proc_at(name: &str, started_ms: i64) -> Proc {
        Proc {
            name: name.into(),
            started_ms,
            cwd: None,
            ppid: None,
        }
    }

    /// The id both paths mint must be the same string, or one session
    /// becomes two rows. This is the whole of judgement "同源去重" at the
    /// level where it can be stated exactly.
    #[test]
    fn the_hook_and_the_disk_mint_the_same_id_for_one_session() {
        let vendor_id = "a03df748-dd59-4ac3-adb5-ed83d9907611";
        let from_disk = id_for(vendor_id);
        // Spelled the way live.rs spells it for a hook payload, on
        // purpose: if that expression drifts, this fails rather than the
        // list quietly growing a duplicate.
        let from_hook = SessionId(format!(
            "{}/{}",
            khor_core::kind::TUI,
            crate::live::clean_leaf(vendor_id)
        ));
        assert_eq!(from_disk, from_hook);
        assert_eq!(from_disk.0, "tui/a03df748-dd59-4ac3-adb5");
    }

    /// A pid number that has been recycled must not resurrect the
    /// session that used to own it.
    #[test]
    fn a_recycled_pid_is_not_the_process_that_wrote_the_file() {
        let procs = Procs::of([(4242, proc_at("claude", 2_000_000))]);
        assert!(
            procs.alive_since(4242, 2_002_000).is_some(),
            "2s late is the normal case"
        );
        assert!(
            procs.alive_since(4242, 1_000_000).is_none(),
            "a process that started 1000s earlier is a different one wearing the number"
        );
        assert!(
            procs.alive_since(9999, 2_000_000).is_none(),
            "not running at all"
        );
    }

    /// **A row's category is the adaptor that recognised it.**
    ///
    /// Both vendors are swept in one call, from one home, so the two
    /// implementations that would otherwise pass cannot: stamping every
    /// row with a constant, and stamping every row with the first
    /// adaptor's name. That is the "全塞进同一格" trap — a property
    /// assertion ("every row has a category") is blind to it, so this
    /// enumerates instead.
    #[cfg(unix)]
    #[test]
    fn each_row_carries_the_name_of_the_adaptor_that_found_it() {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors");
        // One home wearing both vendors' directories.
        let home = std::env::temp_dir().join(format!("khor-both-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::os::unix::fs::symlink(fixtures.join("claude/.claude"), home.join(".claude")).unwrap();
        std::os::unix::fs::symlink(fixtures.join("codex/.codex"), home.join(".codex")).unwrap();

        // One live process per vendor, each vouching for its own fixture.
        let procs = Procs::of([
            (
                4001,
                Proc {
                    name: "claude".into(),
                    started_ms: 1_700_000_000_000,
                    cwd: None,
                    ppid: None,
                },
            ),
            (
                700,
                Proc {
                    name: codex::PROCESS_NAME.into(),
                    started_ms: 1_786_024_158_000,
                    cwd: Some(PathBuf::from("/w/alpha")),
                    ppid: None,
                },
            ),
        ]);
        let found = Discovery::at(&home).sweep_with(&procs);

        let mut seen: Vec<(&str, String)> = found
            .rows
            .iter()
            .map(|f| (f.category, f.sighting.title.clone()))
            .collect();
        seen.sort();
        assert_eq!(
            seen,
            vec![
                ("claude", "one-aa".to_owned()),
                ("codex", "alpha".to_owned()),
            ],
            "each vendor's own row must carry that vendor's name"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// Exact name match: this machine runs `codex-code-mode-host` beside
    /// `codex`, and a substring match would file the helper as a session.
    #[test]
    fn a_helper_process_is_not_the_agent_it_helps() {
        let procs = Procs::of([
            (1, proc_at("codex", 10)),
            (2, proc_at("codex-code-mode-host", 10)),
            (3, proc_at("codex-tui", 10)),
        ]);
        let found: Vec<u32> = procs
            .named("codex")
            .into_iter()
            .map(|(pid, _)| pid)
            .collect();
        assert_eq!(found, vec![1]);
    }
}
