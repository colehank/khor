use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use super::{Batch, Coverage, CoverageMode, Scope};

const OWNERS: usize = 128;
const SCOPES: usize = 64;
const SOURCES: usize = 256;
const INIT_RUNNING: usize = 4;
const INIT_QUEUED: usize = 64;
const PATH_BYTES: usize = 64 * 1024;
const LOG_ENTRIES: usize = 1024;
const LOG_BYTES: usize = 256 * 1024;
const IDLE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Ticket {
    pub source: u64,
    pub generation: u64,
}

#[derive(Debug, Clone)]
pub(super) struct Start {
    pub ticket: Ticket,
    pub machine: String,
    pub scope: Scope,
}

#[derive(Debug, Default)]
pub(super) struct Effects {
    pub start: Vec<Start>,
    pub stop: Vec<u64>,
}

struct Owner {
    machine: String,
    sources: Vec<u64>,
    expires: Instant,
    waiting: bool,
}

struct Source {
    machine: String,
    scope: Scope,
    references: usize,
    generation: u64,
    mode: CoverageMode,
    reason: Option<String>,
}

#[derive(serde::Serialize)]
struct Change {
    source: u64,
    dirs: Vec<String>,
    reset: bool,
    workspace: bool,
}

struct Record {
    seq: u64,
    bytes: usize,
    change: Change,
}

#[derive(Default)]
struct Log {
    seq: u64,
    floor: u64,
    bytes: usize,
    records: VecDeque<Record>,
}

/// Pure resident-owned accounting. Source startup and handles stay in watch.rs.
pub(super) struct Owned {
    epoch: String,
    next_source: u64,
    owners: HashMap<String, Owner>,
    sources: HashMap<u64, Source>,
    queued: VecDeque<Ticket>,
    running: HashSet<Ticket>,
    logs: HashMap<String, Log>,
}

impl Owned {
    pub fn new(epoch: String) -> Self {
        Self {
            epoch,
            next_source: 0,
            owners: HashMap::new(),
            sources: HashMap::new(),
            queued: VecDeque::new(),
            running: HashSet::new(),
            logs: HashMap::new(),
        }
    }

    pub fn begin(
        &mut self,
        id: String,
        machine: String,
        scopes: Vec<Scope>,
        now: Instant,
    ) -> Result<Effects, String> {
        validate(&scopes)?;
        if self.owners.len() == OWNERS || self.owners.contains_key(&id) {
            return Err("Directory watch owner budget exceeded".into());
        }
        let existing: Vec<_> = scopes
            .iter()
            .map(|scope| {
                self.sources
                    .iter()
                    .find(|(_, source)| source.machine == machine && &source.scope == scope)
                    .map(|(id, _)| *id)
            })
            .collect();
        let additional = existing.iter().filter(|id| id.is_none()).count();
        let available = INIT_RUNNING - self.running.len() + INIT_QUEUED - self.queued.len();
        if self.sources.len() + additional > SOURCES || additional > available {
            return Err("Directory watch initialization budget exceeded".into());
        }
        self.next_source
            .checked_add(additional as u64)
            .ok_or("Directory watch source IDs exhausted")?;
        let mut owned = Vec::with_capacity(scopes.len());
        for (scope, existing) in scopes.into_iter().zip(existing) {
            let source = if let Some(id) = existing {
                self.sources
                    .get_mut(&id)
                    .expect("reserved source")
                    .references += 1;
                id
            } else {
                self.next_source += 1;
                let id = self.next_source;
                self.sources.insert(
                    id,
                    Source {
                        machine: machine.clone(),
                        scope,
                        references: 1,
                        generation: 1,
                        mode: CoverageMode::Pending,
                        reason: None,
                    },
                );
                self.queued.push_back(Ticket {
                    source: id,
                    generation: 1,
                });
                id
            };
            owned.push(source);
        }
        self.logs.entry(machine.clone()).or_default();
        self.owners.insert(
            id,
            Owner {
                machine,
                sources: owned,
                expires: now + IDLE,
                waiting: false,
            },
        );
        Ok(Effects {
            start: self.start_queued(),
            stop: Vec::new(),
        })
    }

    fn start_queued(&mut self) -> Vec<Start> {
        let mut ready = Vec::new();
        while self.running.len() < INIT_RUNNING {
            let Some(ticket) = self.queued.pop_front() else {
                break;
            };
            let Some(source) = self.sources.get(&ticket.source) else {
                continue;
            };
            if source.generation != ticket.generation {
                continue;
            }
            self.running.insert(ticket);
            ready.push(Start {
                ticket,
                machine: source.machine.clone(),
                scope: source.scope.clone(),
            });
        }
        ready
    }

    /// Even a closed owner's late worker releases its physical initialization slot.
    pub fn initialized(
        &mut self,
        ticket: Ticket,
        mode: CoverageMode,
        reason: Option<String>,
    ) -> Result<(bool, Effects), String> {
        if !self.running.remove(&ticket) {
            return Ok((false, Effects::default()));
        }
        let accepted = self
            .sources
            .get(&ticket.source)
            .is_some_and(|s| s.generation == ticket.generation);
        if accepted {
            let source = self
                .sources
                .get_mut(&ticket.source)
                .expect("accepted source");
            source.mode = mode;
            source.reason = reason.map(bounded_reason);
            self.changed(ticket.source, Vec::new(), true)?;
        }
        Ok((
            accepted,
            Effects {
                start: self.start_queued(),
                stop: Vec::new(),
            },
        ))
    }

    pub fn changed(
        &mut self,
        source: u64,
        mut dirs: Vec<String>,
        mut reset: bool,
    ) -> Result<(), String> {
        let Some(scope) = self.sources.get(&source) else {
            return Ok(());
        };
        let workspace = matches!(scope.scope, Scope::Workspace { .. });
        if serde_json::to_vec(&dirs).map_err(|e| e.to_string())?.len() > PATH_BYTES {
            dirs.clear();
            reset = true;
        }
        let change = Change {
            source,
            dirs,
            reset,
            workspace,
        };
        let bytes = serde_json::to_vec(&change)
            .map_err(|e| e.to_string())?
            .len();
        let log = self.logs.get_mut(&scope.machine).expect("source has a log");
        log.seq = log
            .seq
            .checked_add(1)
            .ok_or("Directory watch sequence exhausted")?;
        log.bytes += bytes;
        log.records.push_back(Record {
            seq: log.seq,
            bytes,
            change,
        });
        while log.records.len() > LOG_ENTRIES || log.bytes > LOG_BYTES {
            let removed = log.records.pop_front().expect("budget has a record");
            log.bytes -= removed.bytes;
            log.floor = removed.seq;
        }
        Ok(())
    }

    fn owner(&self, machine: &str, id: &str) -> Result<&Owner, String> {
        let owner = self
            .owners
            .get(id)
            .ok_or("Directory watch lease is closed")?;
        if owner.machine != machine {
            return Err("Directory watch machine does not match its lease".into());
        }
        Ok(owner)
    }

    pub fn enter_wait(&mut self, machine: &str, id: &str, now: Instant) -> Result<(), String> {
        self.owner(machine, id)?;
        let owner = self.owners.get_mut(id).expect("validated owner");
        if owner.waiting {
            return Err("Directory watch already has a pending wait".into());
        }
        owner.waiting = true;
        owner.expires = now + IDLE;
        Ok(())
    }

    pub fn finish_wait(&mut self, id: &str) {
        if let Some(owner) = self.owners.get_mut(id) {
            owner.waiting = false;
        }
    }

    pub fn read(&self, machine: &str, id: &str, cursor: Option<&str>) -> Result<Batch, String> {
        let owner = self.owner(machine, id)?;
        let log = &self.logs[machine];
        let mut reset = cursor.is_none();
        let since = if let Some(cursor) = cursor {
            let (epoch, seq) = cursor
                .split_once(':')
                .ok_or("Invalid directory watch cursor")?;
            let seq = seq
                .parse::<u64>()
                .map_err(|_| "Invalid directory watch cursor")?;
            if epoch != self.epoch {
                reset = true;
                0
            } else if seq > log.seq {
                return Err("Directory watch cursor is ahead of its source".into());
            } else {
                reset |= seq < log.floor;
                seq
            }
        } else {
            0
        };
        let mut dirs = BTreeSet::new();
        let mut bytes = 2;
        let mut workspace = false;
        for record in log
            .records
            .iter()
            .filter(|r| r.seq > since && owner.sources.contains(&r.change.source))
        {
            reset |= record.change.reset;
            workspace |= record.change.workspace;
            if reset {
                break;
            }
            for dir in &record.change.dirs {
                if dirs.contains(dir) {
                    continue;
                }
                bytes += serde_json::to_vec(dir).map_err(|e| e.to_string())?.len()
                    + usize::from(!dirs.is_empty());
                if bytes > PATH_BYTES {
                    reset = true;
                    break;
                }
                dirs.insert(dir.clone());
            }
            if reset {
                break;
            }
        }
        if reset {
            dirs.clear();
            workspace = owner
                .sources
                .iter()
                .any(|id| matches!(self.sources[id].scope, Scope::Workspace { .. }));
        }
        let coverage = owner
            .sources
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let source = &self.sources[id];
                Coverage {
                    scope: index as u32,
                    mode: source.mode,
                    reason: source.reason.clone(),
                }
            })
            .collect();
        Ok(Batch {
            cursor: format!("{}:{}", self.epoch, log.seq),
            reset,
            dirs: dirs.into_iter().collect(),
            workspace,
            coverage,
            retry_ms: 0,
        })
    }

    pub fn leave(&mut self, machine: &str, id: &str) -> Result<Effects, String> {
        if !self.owners.contains_key(id) {
            return Ok(Effects::default());
        }
        self.owner(machine, id)?;
        let owner = self.owners.remove(id).expect("validated owner");
        let mut stop = Vec::new();
        for id in owner.sources {
            let source = self.sources.get_mut(&id).expect("owner has a source");
            source.references -= 1;
            if source.references == 0 {
                self.sources.remove(&id);
                stop.push(id);
            }
        }
        self.queued
            .retain(|ticket| self.sources.contains_key(&ticket.source));
        if !self.owners.values().any(|owner| owner.machine == machine) {
            self.logs.remove(machine);
        }
        Ok(Effects {
            start: self.start_queued(),
            stop,
        })
    }

    pub fn expire(&mut self, now: Instant) -> Effects {
        let expired: Vec<_> = self
            .owners
            .iter()
            .filter(|(_, o)| o.expires <= now)
            .map(|(id, o)| (id.clone(), o.machine.clone()))
            .collect();
        let mut effects = Effects::default();
        for (id, machine) in expired {
            let removed = self.leave(&machine, &id).expect("expiry owns this lease");
            effects.start.extend(removed.start);
            effects.stop.extend(removed.stop);
        }
        effects
    }

    pub fn next_expiry(&self) -> Option<Instant> {
        self.owners.values().map(|o| o.expires).min()
    }
}

fn bounded_reason(text: String) -> String {
    let mut out = String::new();
    for ch in text.chars().take(512) {
        out.push(ch);
        if serde_json::to_vec(&out).expect("a string serializes").len() > 512 {
            out.pop();
            break;
        }
    }
    out
}

fn validate(scopes: &[Scope]) -> Result<(), String> {
    if scopes.is_empty() || scopes.len() > SCOPES {
        return Err("Directory watch scope budget exceeded".into());
    }
    let mut workspace = 0;
    let mut bytes = 0;
    let mut seen = HashSet::new();
    for scope in scopes {
        let path = match scope {
            Scope::Entries { path } => path,
            Scope::Workspace { root } => {
                workspace += 1;
                root
            }
        };
        if path.is_empty() || path.contains('\0') || !seen.insert(scope) {
            return Err("Invalid directory watch scope".into());
        }
        bytes += path.len();
    }
    if workspace > 1 || bytes > PATH_BYTES {
        return Err("Directory watch scope budget exceeded".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> Scope {
        Scope::Entries { path: path.into() }
    }
    fn begin(
        book: &mut Owned,
        id: &str,
        machine: &str,
        scopes: Vec<Scope>,
        now: Instant,
    ) -> Effects {
        book.begin(id.into(), machine.into(), scopes, now).unwrap()
    }
    fn ready(book: &mut Owned, starts: Vec<Start>, mode: CoverageMode, reason: Option<String>) {
        let mut waiting: VecDeque<_> = starts.into();
        while let Some(start) = waiting.pop_front() {
            let (accepted, effects) = book
                .initialized(start.ticket, mode, reason.clone())
                .unwrap();
            assert!(accepted);
            waiting.extend(effects.start);
        }
    }

    #[test]
    fn leases_share_sources_but_leave_never_closes_another_owner() {
        let now = Instant::now();
        let mut book = Owned::new("epoch".into());
        let a = begin(&mut book, "A", "machine", vec![entry("/project")], now);
        let b = begin(&mut book, "B", "machine", vec![entry("/project")], now);
        assert_eq!(a.start.len(), 1);
        assert!(b.start.is_empty());
        assert_eq!(a.start[0].machine, "machine");
        assert_eq!(a.start[0].scope, entry("/project"));
        ready(&mut book, a.start, CoverageMode::Native, None);
        assert!(book.leave("other-machine", "A").is_err());
        assert!(book.leave("machine", "A").unwrap().stop.is_empty());
        assert_eq!(
            book.read("machine", "B", None).unwrap().coverage[0].mode,
            CoverageMode::Native
        );
        assert_eq!(book.leave("machine", "B").unwrap().stop.len(), 1);
        assert!(book.read("machine", "B", None).is_err());
        assert!(book.leave("machine", "B").unwrap().stop.is_empty());
        assert!(book.owners.is_empty());
        assert!(book.sources.is_empty());
        assert!(book.logs.is_empty());
    }

    #[test]
    fn local_and_remote_routes_are_pinned_without_resolving_display_names() {
        let now = Instant::now();
        let mut book = Owned::new("epoch".into());
        let a = begin(&mut book, "A", "local-id", vec![entry("/same")], now);
        let b = begin(&mut book, "B", "remote-id", vec![entry("/same")], now);
        assert_ne!(a.start[0].ticket.source, b.start[0].ticket.source);
        assert!(book.read("renamed-display-label", "A", None).is_err());
        assert!(book.enter_wait("remote-id", "A", now).is_err());
        book.enter_wait("local-id", "A", now).unwrap();
        assert!(book.enter_wait("local-id", "A", now).is_err());
        book.finish_wait("A");
        book.enter_wait("local-id", "A", now).unwrap();
        assert!(book.read("local-id", "A", None).is_ok());
    }

    #[test]
    fn initialization_budget_is_transactional_and_closed_workers_cannot_reopen() {
        let now = Instant::now();
        let mut book = Owned::new("epoch".into());
        let first = begin(
            &mut book,
            "A",
            "machine",
            (0..4).map(|i| entry(&format!("/first/{i}"))).collect(),
            now,
        );
        assert_eq!(first.start.len(), 4);
        let next = begin(
            &mut book,
            "B",
            "machine",
            (0..64).map(|i| entry(&format!("/next/{i}"))).collect(),
            now,
        );
        assert!(next.start.is_empty());
        assert_eq!(book.queued.len(), 64);
        assert!(
            book.begin("C".into(), "machine".into(), vec![entry("/overflow")], now)
                .is_err()
        );
        assert_eq!(book.owners.len(), 2);
        assert_eq!(book.sources.len(), 68);
        book.leave("machine", "A").unwrap();
        assert_eq!(
            book.running.len(),
            4,
            "blocked workers still hold their actual initialization slots"
        );
        for start in first.start {
            let (accepted, effects) = book
                .initialized(start.ticket, CoverageMode::Native, None)
                .unwrap();
            assert!(!accepted);
            assert_eq!(effects.start.len(), 1);
        }
        assert_eq!(book.running.len(), 4);
        assert_eq!(book.queued.len(), 60);
        assert!(!book.owners.contains_key("A"));
    }

    #[test]
    fn idle_expiry_releases_without_a_later_wait_and_respects_renewed_owner() {
        let now = Instant::now();
        let mut book = Owned::new("epoch".into());
        let a = begin(&mut book, "A", "machine", vec![entry("/project")], now);
        ready(&mut book, a.start, CoverageMode::Native, None);
        begin(&mut book, "B", "machine", vec![entry("/project")], now);
        assert_eq!(book.next_expiry(), Some(now + IDLE));
        book.enter_wait("machine", "B", now + Duration::from_secs(30))
            .unwrap();
        book.finish_wait("B");
        assert!(book.expire(now + IDLE).stop.is_empty());
        assert!(book.read("machine", "A", None).is_err());
        assert!(book.read("machine", "B", None).is_ok());
        assert_eq!(book.expire(now + Duration::from_secs(90)).stop.len(), 1);
        assert_eq!(book.next_expiry(), None);
        assert!(book.sources.is_empty());
    }

    #[test]
    fn native_coverage_has_a_new_reset_fence_then_reads_only_later_changes() {
        let now = Instant::now();
        let mut book = Owned::new("epoch".into());
        let starts = begin(
            &mut book,
            "A",
            "machine",
            vec![Scope::Workspace {
                root: "/project".into(),
            }],
            now,
        )
        .start;
        let source = starts[0].ticket.source;
        let pending = book.read("machine", "A", None).unwrap();
        assert!(pending.reset);
        assert_eq!(pending.coverage[0].mode, CoverageMode::Pending);
        ready(&mut book, starts, CoverageMode::Native, None);
        let established = book.read("machine", "A", Some(&pending.cursor)).unwrap();
        assert!(established.reset && established.workspace);
        assert_eq!(established.coverage[0].mode, CoverageMode::Native);
        assert!(
            !book
                .read("machine", "A", Some(&established.cursor))
                .unwrap()
                .reset
        );
        book.changed(source, vec!["/project/src".into()], false)
            .unwrap();
        let changed = book
            .read("machine", "A", Some(&established.cursor))
            .unwrap();
        assert_eq!(changed.dirs, vec!["/project/src"]);
        assert!(changed.workspace);
        assert!(!changed.reset);
        assert!(
            book.read("machine", "A", Some("old-epoch:1000000"))
                .unwrap()
                .reset
        );
        assert!(book.read("machine", "A", Some("epoch:1000000")).is_err());
    }

    #[test]
    fn log_bytes_and_entries_are_bounded_and_old_cursors_reset() {
        let now = Instant::now();
        let mut book = Owned::new("epoch".into());
        let starts = begin(&mut book, "A", "machine", vec![entry("/project")], now).start;
        let source = starts[0].ticket.source;
        ready(&mut book, starts, CoverageMode::Native, None);
        let before = book.read("machine", "A", None).unwrap().cursor;
        for n in 0..1100 {
            book.changed(
                source,
                vec![format!("/project/{n}/{}", "x".repeat(1000))],
                false,
            )
            .unwrap();
        }
        let log = &book.logs["machine"];
        assert!(log.bytes <= LOG_BYTES);
        assert!(log.records.len() <= LOG_ENTRIES);
        assert!(log.floor > 1);
        assert!(book.read("machine", "A", Some(&before)).unwrap().reset);
    }

    #[test]
    fn event_overflow_and_escaped_coverage_reasons_stay_within_wire_budgets() {
        let now = Instant::now();
        let mut book = Owned::new("epoch".into());
        let starts = begin(
            &mut book,
            "A",
            "machine",
            (0..64).map(|i| entry(&format!("/project/{i}"))).collect(),
            now,
        )
        .start;
        let source = starts[0].ticket.source;
        ready(
            &mut book,
            starts,
            CoverageMode::Partial,
            Some("\u{1}".repeat(512)),
        );
        let before = book.read("machine", "A", None).unwrap().cursor;
        book.changed(source, vec!["\u{1}".repeat(20_000)], false)
            .unwrap();
        let batch = book.read("machine", "A", Some(&before)).unwrap();
        assert!(batch.reset);
        assert!(batch.dirs.is_empty());
        assert!(serde_json::to_vec(&batch).unwrap().len() <= 128 * 1024);
        assert!(
            batch
                .coverage
                .iter()
                .all(|c| serde_json::to_vec(&c.reason).unwrap().len() <= 512)
        );
    }

    #[test]
    fn owner_and_scope_limits_reject_without_partially_registering() {
        let now = Instant::now();
        let mut book = Owned::new("epoch".into());
        for scopes in [
            vec![],
            (0..65).map(|i| entry(&format!("/{i}"))).collect(),
            vec![entry("/same"), entry("/same")],
            vec![entry(&"x".repeat(PATH_BYTES + 1))],
            vec![
                Scope::Workspace { root: "/a".into() },
                Scope::Workspace { root: "/b".into() },
            ],
        ] {
            assert!(
                book.begin("invalid".into(), "machine".into(), scopes, now)
                    .is_err()
            );
            assert!(book.owners.is_empty());
            assert!(book.sources.is_empty());
        }
        for n in 0..OWNERS {
            begin(
                &mut book,
                &format!("{n}"),
                "machine",
                vec![entry("/shared")],
                now,
            );
        }
        assert!(
            book.begin(
                "overflow".into(),
                "machine".into(),
                vec![entry("/shared")],
                now
            )
            .is_err()
        );
        assert_eq!(book.owners.len(), OWNERS);
        assert_eq!(book.sources.len(), 1);
    }
}
