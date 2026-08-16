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

use khor_core::{SessionId, State};

pub mod claude;
pub mod codex;

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
}

impl Sighting {
    /// The khor session id this sighting lands on.
    pub fn id(&self) -> SessionId {
        id_for(&self.vendor_session_id)
    }
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
    pub sighting: Sighting,
}

/// What one sweep of every vendor found, each row already attributed.
#[derive(Debug, Default, Clone)]
pub struct Findings {
    pub rows: Vec<Found>,
    /// Summed across vendors; see [`Sweep::unmapped`].
    pub unmapped: usize,
}

impl Findings {
    fn absorb(&mut self, vendor: &'static str, other: Sweep) {
        self.rows.extend(
            other
                .rows
                .into_iter()
                .map(|sighting| Found { category: vendor, sighting }),
        );
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

impl Procs {
    /// What is running right now.
    ///
    /// Two passes on purpose. A working directory costs the OS a
    /// separate lookup per process, and this runs on every session list
    /// — so the sweep pass asks only for names and start times, and cwd
    /// is fetched afterwards for the handful of pids that are candidate
    /// agent processes.
    pub fn snapshot() -> Procs {
        use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

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

    /// The process at `pid`, but only if it started when the caller
    /// expects — a recycled pid number is not the process that wrote the
    /// file (see [`START_TOLERANCE_MS`]).
    pub fn alive_since(&self, pid: u32, started_ms: i64) -> Option<&Proc> {
        let p = self.by_pid.get(&pid)?;
        ((p.started_ms - started_ms).abs() <= START_TOLERANCE_MS).then_some(p)
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
        Discovery { adaptors: Vec::new(), procs: None }
    }

    /// Sweeps against this process table instead of the OS.
    pub fn with_procs(mut self, procs: Procs) -> Discovery {
        self.procs = Some(procs);
        self
    }
    /// Adaptors reading the vendor directories under `home`.
    ///
    /// The root is a parameter, not a constant, because otherwise no
    /// test of this module can be closed: the graveyard control group
    /// and the crash rule both need a tree nobody's real agent is
    /// writing to.
    pub fn at(home: &Path) -> Discovery {
        Discovery {
            adaptors: vec![
                Box::new(claude::Claude::at(home.join(".claude"))),
                Box::new(codex::Codex::at(home.join(".codex"))),
            ],
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
    pub fn for_root(root: &Path) -> Discovery {
        let home = std::env::var_os("KHOR_VENDOR_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.to_path_buf());
        Discovery::at(&home)
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
        all
    }
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
        Proc { name: name.into(), started_ms, cwd: None }
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
            (4001, Proc { name: "claude".into(), started_ms: 1_700_000_000_000, cwd: None }),
            (
                700,
                Proc {
                    name: codex::PROCESS_NAME.into(),
                    started_ms: 1_786_024_158_000,
                    cwd: Some(PathBuf::from("/w/alpha")),
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
