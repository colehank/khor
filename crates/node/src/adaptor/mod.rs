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
//! The vendor's files say *what* a session is; the process table says
//! *whether it still is*. History files are history: this machine holds
//! 83 codex rollouts and one running codex, and that means one row.
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
    /// Empty for a crashed row: the pid its file names is gone, so
    /// nothing running can be inside it.
    pub pids: Vec<u32>,
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
}

impl Findings {
    fn absorb(&mut self, vendor: &'static str, other: Sweep) {
        self.rows.extend(other.rows.into_iter().map(|sighting| Found {
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
    /// Re-run `cargo test -p khor-node --test cost -- --ignored
    /// --nocapture` if the list ever feels slow, or on a machine with far
    /// more processes than 803; the cost scales with that count, which is
    /// why it is reported next to the times.
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
            .filter(|(_, p)| p.name == codex::PROCESS_NAME)
            .map(|(pid, _)| Pid::from_u32(*pid))
            .collect();
        if !wanted.is_empty() {
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&wanted),
                true,
                ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
            );
            for pid in wanted {
                if let (Some(p), Some(slot)) =
                    (sys.process(pid), by_pid.get_mut(&pid.as_u32()))
                {
                    slot.cwd = p.cwd().map(Path::to_path_buf);
                }
            }
        }
        Procs { by_pid }
    }

    /// A hand-built table, for tests and for the fixture trees.
    pub fn of(procs: impl IntoIterator<Item = (u32, Proc)>) -> Procs {
        Procs { by_pid: procs.into_iter().collect() }
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

    /// Every running process with exactly this name. Exact, not a
    /// substring: this machine runs both `codex` and
    /// `codex-code-mode-host`, and only one of them is an agent session.
    pub fn named<'a>(&'a self, name: &str) -> Vec<(u32, &'a Proc)> {
        self.by_pid
            .iter()
            .filter(|(_, p)| p.name == name)
            .map(|(pid, p)| (*pid, p))
            .collect()
    }
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
}

impl Discovery {
    /// Reads nothing. The default for [`crate::live::LiveKind`], so that
    /// every registry test stays closed and reading the machine's real
    /// vendor directories is an explicit decision made in one place.
    pub fn empty() -> Discovery {
        Discovery { adaptors: Vec::new(), tmux: None, procs: None }
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
        match &self.procs {
            Some(procs) => self.sweep_with(procs),
            None => self.sweep_with(&Procs::snapshot()),
        }
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
fn is_real_home(path: &Path) -> bool {
    let Some(home) = std::env::home_dir() else {
        return false;
    };
    let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    real(path) == real(&home)
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

    fn proc_at(name: &str, started_ms: i64) -> Proc {
        Proc { name: name.into(), started_ms, cwd: None, ppid: None }
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
        assert!(procs.alive_since(4242, 2_002_000).is_some(), "2s late is the normal case");
        assert!(
            procs.alive_since(4242, 1_000_000).is_none(),
            "a process that started 1000s earlier is a different one wearing the number"
        );
        assert!(procs.alive_since(9999, 2_000_000).is_none(), "not running at all");
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
        let found: Vec<u32> = procs.named("codex").into_iter().map(|(pid, _)| pid).collect();
        assert_eq!(found, vec![1]);
    }
}
