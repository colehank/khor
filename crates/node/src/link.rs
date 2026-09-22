//! The live link: serve loop, pairing, and sync rounds over iroh.
//!
//! Trust model (docs/NET.md): joining is the only gate. `Pair` is
//! answered for whoever holds an unburned token; everything else is
//! answered only for devices already in the table.

use std::collections::BTreeSet;
use std::fs;
use std::time::Duration;

use base64::Engine;
use khor_catalog::msg;
use khor_net::endpoint::{self, ALPN, Ticket};
use khor_sync::devices::DeviceInfo;
use khor_sync::store::Doc;
use khor_sync::{chat, wire};

use crate::proto::{self, MAX_FRAME, Request, Response};
use crate::{Node, ipc};

/// How often the serve loop syncs with everyone — and, on the same beat,
/// reaps borrows whose rows are gone.
///
/// **Public so a test can wait exactly one of these** rather than hold a
/// number of its own. A test that slept "about long enough" would be
/// carrying a constant only its author could check, and it would quietly
/// stop covering the tick the day this changes.
pub const SYNC_EVERY: Duration = Duration::from_secs(5);

/// How long every road-having device must have kept failing before the
/// serve concludes its own ears are broken. Wall clock, not rounds: a
/// round is not a fixed [`SYNC_EVERY`] — when everyone is unreachable
/// each visit waits out its dial budget, and the old "24 rounds ≈ two
/// minutes" arithmetic measured eight real minutes (2026-08-20). Two
/// minutes is long enough that a peer rebooting or a relay blipping
/// never trips it, short beside the 50 minutes a deaf endpoint
/// actually cost (2026-08-20: a healthy-looking serve went both-ways
/// unreachable while a fresh endpoint on the same box echoed through
/// the same relay in seconds; the cure was always a restart, so the
/// serve prescribes it to itself — the keeper stands it back up with
/// fresh ears).
/// How often a running serve re-reads 暂停 (docs/MORE.md). Half a
/// second: somebody pressing 暂停接入 and watching the rail cannot feel
/// it, and it is far under anything the network does. **A beat rather
/// than a notification**, because the file is the truth and a serve
/// started while paused has to obey it too — one path, whether the
/// state was set a second ago or last week.
const PAUSE_POLL: Duration = Duration::from_millis(500);

const DEAF_AFTER: Duration = Duration::from_secs(120);

/// The exit code of a serve that concluded it had gone deaf. Non-zero
/// on purpose: a clean exit would tell the keeper to stay down.
pub const DEAF_EXIT: i32 = 86;

/// The deafness ledger: remembers when the silence began — the first
/// of an unbroken run of sync rounds in which every device that HAS
/// roads failed. Devices without roads never count — they cannot be
/// dialed on the best of days — so a mesh of one, or a table of
/// one-shot pairings, never trips this.
struct DeafWatch {
    deaf_since: Option<std::time::Instant>,
}

impl DeafWatch {
    fn new() -> Self {
        Self { deaf_since: None }
    }

    /// Feeds one pump's outcomes; true means "conclude deafness".
    fn observe(&mut self, outcomes: &[(String, Result<String, String>)]) -> bool {
        self.observe_at(outcomes, std::time::Instant::now())
    }

    /// The clock comes in as an argument so the tests need not sleep.
    fn observe_at(
        &mut self,
        outcomes: &[(String, Result<String, String>)],
        now: std::time::Instant,
    ) -> bool {
        let mut candidates = 0u32;
        let mut failures = 0u32;
        for (_, verdict) in outcomes {
            match verdict {
                Err(why) if why == msg::no_roads_reported() => continue,
                Err(_) => {
                    candidates += 1;
                    failures += 1;
                }
                Ok(_) => candidates += 1,
            }
        }
        if candidates == 0 || failures < candidates {
            self.deaf_since = None;
            return false;
        }
        let since = *self.deaf_since.get_or_insert(now);
        now.duration_since(since) >= DEAF_AFTER
    }
}
/// Per-device budget for one sync visit; the far side may simply be off.
const DIAL_TIMEOUT: Duration = Duration::from_secs(10);

/// The same budget in whole seconds, for the one place a person is told
/// what a wait could cost before it starts (`khor accept` announcing the
/// round it is about to drive — `Node::accept`).
///
/// Here rather than at the CLI for the reason [`invite_window_minutes`]
/// is: a screen that names a number the code does not enforce is worse
/// than a screen that names none, and a copy of the figure at the far
/// end of the crate is exactly how the two drift apart.
pub fn dial_budget_seconds() -> u64 {
    DIAL_TIMEOUT.as_secs()
}

/// How long the direct roads get to themselves before the relay road is
/// opened alongside them (`Node::dial_with`).
///
/// **Long enough that a working direct path never involves the relay**,
/// which is the whole point of having one: a machine on the same LAN
/// answers in milliseconds, so the second road is never opened for it
/// and the relay carries nothing it does not have to.
///
/// Two earlier shapes were wrong in opposite directions, and both are
/// worth keeping written down:
///
/// - *Splitting `DIAL_TIMEOUT` into 4s + 6s* cut off a direct path that
///   was merely **slow** — a loaded box, a busy LAN — and pushed it onto
///   a relay it did not need.
/// - *Trying the roads one after the other, each with its own budget*
///   fixed that and put the cost on **every unreachable machine
///   instead**: a `sync` round pays it once per peer that is off, so a
///   mesh with two dark machines got a third slower at everything.
///
/// Running them side by side costs neither: a machine that is genuinely
/// off still fails in exactly `DIAL_TIMEOUT`, the same as before any of
/// this.
const RELAY_HEAD_START: Duration = Duration::from_millis(1500);

/// How long a pairing ticket can be used after it is minted.
///
/// A ticket used to be one-time only in the sense that it burned on use;
/// **nothing stopped one found later from being used at all**, and a
/// pairing ticket is a key to the whole mesh. So there are two bounds
/// and the number sits between them:
///
/// - **Long enough to carry a ticket from one machine to another**,
///   including being interrupted on the way: mint it at the desk, pick
///   up the phone, get distracted, finish. Anything under a couple of
///   minutes would make ordinary setup fail and teach people to re-mint
///   in a hurry rather than read what they are pasting.
/// - **Short enough that a ticket found later is already dead** — in a
///   chat log, a screenshot, a shell history, a note from this morning.
///   The ledger's bar was "not still alive half a day later"; a quarter
///   of an hour is well inside it.
///
/// Fifteen minutes. Re-minting costs one command on a machine the person
/// is already sitting at, which is why the cost of being too short is
/// small and the cost of being too long is a key nobody revoked.
///
/// **Enforced on the issuer's side only** (`Node::invite_is_fresh`), so
/// this is one clock's arithmetic and not two — the accepting machine's
/// clock never enters into it.
pub const INVITE_WINDOW_MS: i64 = 15 * 60 * 1000;

/// The same window in whole minutes — **the unit every word for it uses**
/// (`invite-window`, `invite-expired`), so the conversion belongs here
/// rather than at each place that says it.
///
/// It exists because three callers were each dividing by 60,000: the CLI
/// after minting, the refusal an expired ticket gets, and the app's
/// ticket dialog. Three divisions of one constant is three chances for a
/// screen to name a window the code does not enforce — and the app's is
/// the one that cannot be checked by reading the same file.
pub fn invite_window_minutes() -> u32 {
    // The constant is a positive literal one screen up; a future value
    // that broke that would be caught here rather than in a rendered
    // sentence.
    (INVITE_WINDOW_MS / 60_000) as u32
}

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// What one-shot CLI verbs need to know about the live endpoint. The
/// cookie makes it a key to the hand-off port — owner-only on disk.
#[derive(serde::Serialize, serde::Deserialize)]
struct EndpointFile {
    id: String,
    addrs: Vec<String>,
    pid: u32,
    #[serde(default)]
    relays: Vec<String>,
    #[serde(default)]
    ipc_port: u16,
    #[serde(default)]
    ipc_cookie: String,
}

/// How a [`Node::leave`] ended, per machine that was in the table.
///
/// **Two lists rather than a count**, because the two endings leave
/// different things to do and the names are what a person acts on: a
/// machine in `untold` is one somebody has to walk over to (or reach
/// another way) and run `khor forget` on, and saying 「3 of 5」 would
/// name none of them.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Left {
    /// Machines that took the news before this one emptied its table.
    pub told: Vec<String>,
    /// Machines that could not be reached. They go on holding a row for
    /// this machine and painting it offline until somebody over there
    /// shows it the door.
    pub untold: Vec<String>,
}

impl Node {
    /// Runs this device's listening half: answers connections, executes
    /// handed-off one-shot verbs, syncs with every known device every few
    /// seconds. Writes `endpoint.json` so one-shot verbs can find both
    /// the live endpoint and the hand-off port.
    pub async fn serve(self) -> Result<(), String> {
        let ep = std::sync::Arc::new(
            // The one bind that serves this machine, and the only
            // caller that passes the exclusive switch — every other
            // `bind` in this file is a test standing up a throwaway
            // endpoint, and none of them has a relay file to read.
            endpoint::bind_with(
                self.secret_key().clone(),
                self.relays(),
                self.relays_only_mine(),
            )
            .await
            .map_err(|e| e.to_string())?,
        );
        let addrs = endpoint::local_addrs(&ep);
        // Own dialing hints go into the table so the snapshot handed to a
        // pairing device already says how to reach us — relay roads
        // included, or a peer behind a hostile network has no way in.
        {
            let mut roads = addrs.clone();
            roads.extend(self.relays().iter().cloned());
            let loaded = self.devices_loaded()?;
            loaded.doc.upsert(self.device_str(), self.name(), &roads)?;
            let mut store = loaded.store;
            store.flush(&loaded.doc)?;
        }
        let handoffs = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(msg::cant_open_handoff_port)?;
        let cookie = fresh_hex()?;
        let file = EndpointFile {
            id: self.device_str().to_owned(),
            addrs,
            pid: std::process::id(),
            relays: self.relays().to_vec(),
            ipc_port: handoffs.local_addr().map_err(|e| e.to_string())?.port(),
            ipc_cookie: cookie.clone(),
        };
        write_private(
            &self.root().join(".khor").join("endpoint.json"),
            &serde_json::to_vec(&file).map_err(|e| e.to_string())?,
        )?;

        // Borrow rows left by a previous serve point at ports this
        // process does not hold; clear them before answering anyone.
        self.sweep_stale_borrows();

        // Watch this runtime for a handler that blocks it (#87 / #73:
        // `crate::reactor` has why a wall clock could not tell that
        // from an ordinary wait). Started here rather than at open:
        // there is no reactor to stall until there is a serve.
        crate::reactor::watch();

        // One task per connection: a client that vanishes without
        // closing must not block the accept loop until QUIC times out.
        let node = std::sync::Arc::new(self);
        // The satellite sampler (附属机器批): its own task on its own
        // minute beat — a sample is an ssh dial, and the 5s pump must
        // not inherit that weight.
        {
            let n = node.clone();
            tokio::spawn(async move { n.satellite_sampler().await });
        }
        // The day strip's sampler (`crates/node/src/day.rs`), on the same
        // footing and for the same reason: one sample is a whole
        // `sessions()`, and it is synchronous — the two things the 5s
        // pump must not take on. **The record only exists while a
        // resident does**, which is not a gap in the design: every skin
        // embeds one (this file's own head, and `gui-core`'s), and the
        // minutes with nobody looking are the ones the strip paints as
        // 不知道 rather than inventing a word for.
        {
            let n = node.clone();
            tokio::spawn(async move { n.day_sampler().await });
        }
        // The CPU day's recorder (`crates/node/src/history.rs`): the
        // same footing again — one reading a minute into a file, so the
        // band under the CPU ring has a day behind it that survives
        // this process (its own test restarts to prove it).
        {
            let n = node.clone();
            tokio::spawn(async move { n.cpu_recorder().await });
        }
        // 暂停 (docs/MORE.md), read **off this loop**. The file stays the
        // one truth — `khor pause` writes it from another process and
        // nothing here caches a decision — but the accept arm may not
        // touch a disk: a `read_to_string` per connection put the
        // reactor over its second under load (2026-09-20,
        // `crates/node/tests/load.rs`, which is the gate that said so,
        // and this file's own rule about the loop). So a task of its
        // own does the reading, on a beat a person cannot feel, and the
        // two arms below consult a bit.
        let paused = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(node.paused()));
        {
            let flag = paused.clone();
            let root = node.root().clone();
            tokio::spawn(async move {
                let mut beat = tokio::time::interval(PAUSE_POLL);
                loop {
                    beat.tick().await;
                    let r = root.clone();
                    // `spawn_blocking` for the reason the read is not in
                    // the loop: it is a file, and this runtime carries
                    // the link.
                    if let Ok(now) =
                        tokio::task::spawn_blocking(move || crate::pause::is_on(&r)).await
                    {
                        flag.store(now, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            });
        }
        let deaf = std::sync::Arc::new(std::sync::Mutex::new(DeafWatch::new()));
        let mut ticker = tokio::time::interval(SYNC_EVERY);
        loop {
            tokio::select! {
                incoming = ep.accept() => {
                    let Some(incoming) = incoming else { break };
                    // Paused: dropped without an answer, so the far side
                    // sees what a machine that is not there looks like —
                    // which is the state somebody asked for.
                    if paused.load(std::sync::atomic::Ordering::Relaxed) { continue }
                    let n = node.clone();
                    let e = ep.clone();
                    tokio::spawn(async move {
                        if let Ok(conn) = incoming.await {
                            if conn.alpn() == endpoint::TUNNEL_ALPN {
                                let _ = n.serve_tunnel(conn).await;
                            } else {
                                let _ = n.handle(conn, &e).await;
                            }
                        }
                    });
                }
                accepted = handoffs.accept() => {
                    if let Ok((stream, _)) = accepted {
                        let n = node.clone();
                        let e = ep.clone();
                        let c = cookie.clone();
                        tokio::spawn(async move {
                            let _ = n.handle_handoff(stream, &e, &c).await;
                        });
                    }
                }
                _ = ticker.tick() => {
                    // A kept serve whose keeper is gone must leave: an
                    // orphan holds the endpoint key, so the guardian's
                    // next serve would knock it off (one live endpoint
                    // per key) and the pid file already points at a
                    // corpse. Whatever killed the keeper — 2026-08-20
                    // it died silently on an NFS binary swap (#76) —
                    // every path converges on "the shop stands empty
                    // for the next keeper". Kept serves only: a GUI or
                    // foreground serve is init's child by design.
                    if crate::keeper::is_inner() && unsafe { libc::getppid() } == 1 {
                        let stamp =
                            jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                        eprintln!("{}", msg::serve_orphaned(&stamp));
                        std::process::exit(1);
                    }
                    // Off the accept loop: an unreachable device stalls a
                    // visit for DIAL_TIMEOUT, and a serve that goes deaf
                    // for that long fails everyone else. Skip when the
                    // previous pump still runs instead of piling up.
                    // Paused: no pump, and **no hearing test either**.
                    // The deaf watch exits the process when everyone
                    // with a road has been silent for DEAF_AFTER, which
                    // is exactly what a pause looks like from in here —
                    // left running, it would restart a machine somebody
                    // deliberately quieted, over and over. The local
                    // handoff arm stays open: a paused machine is still
                    // a machine its own faces talk to.
                    if paused.load(std::sync::atomic::Ordering::Relaxed) { continue }
                    let n = node.clone();
                    let e = ep.clone();
                    let deaf = deaf.clone();
                    tokio::spawn(async move {
                        // Reap borrows whose rows were closed since the
                        // last tick, freeing their ports, before syncing.
                        n.reap_borrows().await;
                        // And bridge hosts whose tmux session now lists
                        // through its own agent row (the fold in
                        // `LiveKind::rows` hides them; this stops them).
                        n.live.reap_folded_bridges();
                        let Ok(_g) = n.sync_gate.try_lock() else { return };
                        let out = n.sync_with_all(&e).await;
                        // The serve's own hearing test: a long-lived
                        // endpoint can go silently, bidirectionally
                        // unreachable while the process looks healthy
                        // (批23). Every observed cure was a restart, so
                        // when everyone with a road has been dead for
                        // DEAF_AFTER straight, prescribe it: exit, and
                        // the keeper stands up a fresh endpoint.
                        if deaf.lock().expect("deaf watch poisoned").observe(&out) {
                            let stamp =
                                jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                            eprintln!("{}", msg::serve_gone_deaf(&stamp, DEAF_AFTER.as_secs()));
                            std::process::exit(DEAF_EXIT);
                        }
                        // **Tidy up after the sync, never before it.**
                        // Compaction rewrites a document that the pump
                        // above may have just added blocks to, so doing
                        // it second means one fewer round where the new
                        // blocks are the ones left uncompacted. It is
                        // also the cheap half: the scan is a handful of
                        // `read_dir`s and only a document past
                        // `store::PILE` is rewritten, so an ordinary
                        // tick does nothing at all.
                        //
                        // ⚠️ **The resident serve is the only process
                        // that may do this** (`Node::compact_piles`).
                        // Off the async thread because it is file work,
                        // and behind the same sync gate this tick
                        // already holds, so a rewrite and a pump never
                        // touch one document at once.
                        let c = n.clone();
                        let done = tokio::task::spawn_blocking(move || c.compact_piles())
                            .await
                            .unwrap_or_default();
                        for (dir, c) in done {
                            let stamp =
                                jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                            eprintln!(
                                "{stamp} compacted {} — {} blocks into {}",
                                dir.display(),
                                c.before,
                                c.after
                            );
                        }
                    });
                }
            }
        }
        Ok(())
    }

    async fn handle_handoff(
        &self,
        mut stream: tokio::net::TcpStream,
        ep: &iroh::Endpoint,
        cookie: &str,
    ) -> Result<(), String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut bytes = Vec::new();
        (&mut stream)
            .take(MAX_FRAME as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(msg::handoff_unreadable)?;
        let reply = match proto::decode::<ipc::Handoff>(&bytes) {
            Ok(h) if h.cookie == cookie => self.run_handoff_within(ep, h.op).await,
            Ok(_) => ipc::Reply::Refused {
                why: msg::handoff_wrong_cookie().into(),
            },
            Err(e) => ipc::Reply::Refused { why: e },
        };
        stream
            .write_all(&proto::encode(&reply)?)
            .await
            .map_err(msg::cant_reply)?;
        stream.shutdown().await.map_err(msg::cant_close_stream)?;
        Ok(())
    }

    /// [`Self::run_handoff`] under the op's deadline (SHELL.md 第五轮 第 2
    /// 条). **Half a second short of the asker's**, so the refusal still
    /// reaches a caller that is about to stop waiting, and **named in the
    /// log** — a stuck op is a fault in something this serve talks to,
    /// and the log is where somebody goes looking for which.
    async fn run_handoff_within(&self, ep: &iroh::Endpoint, op: ipc::Op) -> ipc::Reply {
        let (name, deadline) = (op.name(), op.deadline());
        let mine = deadline.saturating_sub(Duration::from_millis(500));
        match tokio::time::timeout(mine, self.run_handoff(ep, op)).await {
            Ok(reply) => reply,
            Err(_) => {
                let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                eprintln!("{}", msg::serve_op_late(&stamp, name, mine.as_secs()));
                ipc::Reply::Refused {
                    why: msg::serve_op_timed_out(name, deadline.as_secs()),
                }
            }
        }
    }

    async fn run_handoff(&self, ep: &iroh::Endpoint, op: ipc::Op) -> ipc::Reply {
        match op {
            ipc::Op::Pair { ticket } => {
                // Routed pairing reports the serve's own addresses: they
                // outlive this exchange, unlike a one-shot endpoint's.
                let mut roads = endpoint::local_addrs(ep);
                roads.extend(self.relays().iter().cloned());
                match self.pair_with(ep, &ticket, roads).await {
                    Ok(name) => ipc::Reply::Paired { name },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::Hops => {
                // Every device gets a row, including this machine and
                // the ones nothing has ever dialled: a device left out
                // would read as absent rather than as unread, and those
                // are different answers.
                let devices = self.devices().unwrap_or_default();
                let asks = devices
                    .iter()
                    .map(|d| {
                        let id = d.id.clone();
                        (d.id.clone(), async move {
                            endpoint::roads_in_use(ep, &id).await
                        })
                    })
                    .collect();
                let (by_device, late) = hops_within(asks, HOPS_BUDGET).await;
                // Said, not swallowed: 说不出 on the card is true, and this
                // line is where the *why* goes — a stalled endpoint is a
                // fault worth finding in the serve's log.
                if !late.is_empty() {
                    let names: Vec<&str> = devices
                        .iter()
                        .filter(|d| late.contains(&d.id))
                        .map(|d| d.name.as_str())
                        .collect();
                    let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                    eprintln!(
                        "{}",
                        msg::serve_hops_late(&stamp, HOPS_BUDGET.as_millis(), names.join(", "))
                    );
                }
                ipc::Reply::Hops { by_device }
            }
            ipc::Op::SyncNow => {
                let _g = self.sync_gate.lock().await;
                ipc::Reply::Synced {
                    outcomes: self.sync_with_all(ep).await,
                }
            }
            ipc::Op::Accept { session } => {
                match self.accept_with(ep, &crate::SessionId(session)).await {
                    Ok((moved, landed)) => ipc::Reply::Accepted { moved, landed },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::Ls { machine, path } => match self.ls_with(ep, &machine, &path).await {
                Ok((path, entries, truncated)) => ipc::Reply::Dir {
                    path,
                    entries,
                    truncated,
                },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::Pull { machine, path, dir } => {
                match self
                    .pull_path_with(ep, &machine, &path, std::path::Path::new(&dir))
                    .await
                {
                    Ok((moved, dest)) => ipc::Reply::Pulled {
                        moved,
                        dest: dest.display().to_string(),
                    },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::Borrow { machine } => match self.start_borrow(ep, &machine, true).await {
                Ok((id, addr)) => ipc::Reply::Borrowing {
                    session: id.0,
                    addr: addr.to_string(),
                },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::BorrowUnlisted { machine } => {
                match self.start_borrow(ep, &machine, false).await {
                    Ok((id, addr)) => ipc::Reply::Borrowing {
                        session: id.0,
                        addr: addr.to_string(),
                    },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::DropBorrow { session } => {
                // The answer does not carry whether there was one to
                // drop: see `Op::DropBorrow`.
                self.drop_hosted_borrow(&crate::SessionId(session)).await;
                ipc::Reply::Accepted {
                    moved: 0,
                    landed: Vec::new(),
                }
            }
            ipc::Op::OpenOn {
                machine,
                kind,
                title,
                cwd,
                cmd,
                cols,
                rows,
            } => {
                match self
                    .open_on_with(ep, &machine, &kind, &title, &cwd, &cmd, (cols, rows))
                    .await
                {
                    Ok(id) => ipc::Reply::OpenedOn { session: id.0 },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::CloseOn { machine, session } => {
                match self
                    .close_on_with(ep, &machine, &crate::SessionId(session))
                    .await
                {
                    // A close borrows this reply for its "done" — no
                    // bytes and no paths, which is what both empties say.
                    Ok(()) => ipc::Reply::Accepted {
                        moved: 0,
                        landed: Vec::new(),
                    },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::ReachOn { machine, session } => {
                match self
                    .reach_on_with(ep, &machine, &crate::SessionId(session))
                    .await
                {
                    Ok((addr, cookie)) => ipc::Reply::Reaching {
                        addr: addr.to_string(),
                        cookie,
                    },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::OpenAgentOn {
                machine,
                dir,
                title,
                form,
                agent,
            } => {
                match self
                    .open_agent_on_with(ep, &machine, &dir, &title, &form, &agent)
                    .await
                {
                    Ok(id) => ipc::Reply::OpenedOn { session: id.0 },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::PlacesOn { machine } => match self.places_on_with(ep, &machine).await {
                Ok(places) => ipc::Reply::Places { places },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::LoadOn {
                machine,
                top,
                by,
                desc,
            } => match self.load_on_with(ep, &machine, top, by, desc).await {
                Ok(load) => ipc::Reply::Load { load },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::TouchedOn { machine, path } => {
                match self.touched_with(ep, &machine, &path).await {
                    Ok(at_ms) => ipc::Reply::Touched { at_ms },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::RemoveOn { machine, path } => {
                match self.remove_with(ep, &machine, &path).await {
                    Ok(()) => ipc::Reply::Removed,
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::ReadHead { machine, path } => {
                match self.read_head_with(ep, &machine, &path).await {
                    Ok((total, at_ms, bytes)) => ipc::Reply::Head {
                        total,
                        at_ms,
                        bytes: serde_bytes::ByteBuf::from(bytes),
                    },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::GitStatusOn { machine, root } => {
                match self.git_status_with(ep, &machine, &root).await {
                    Ok(s) => ipc::Reply::GitStatus {
                        branch: s.branch,
                        rows: s.rows,
                        more: s.more,
                        ahead: s.ahead,
                        behind: s.behind,
                        fetched_ms: s.fetched_ms,
                    },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::GitDiffOn { machine, path } => {
                match self.git_diff_with(ep, &machine, &path).await {
                    Ok(d) => ipc::Reply::GitDiff {
                        hunks: d.hunks,
                        more: d.more,
                    },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::WritePathOn {
                machine,
                path,
                was,
                bytes,
            } => {
                match self
                    .write_path_with(ep, &machine, &path, &was, &bytes)
                    .await
                {
                    Ok((at_ms, digest)) => ipc::Reply::Wrote { at_ms, digest },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::CodeHereOn { machine } => match self.code_here_with(ep, &machine).await {
                Ok(e) => ipc::Reply::CodeHere {
                    version: e.version,
                    ready: e.ready,
                },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::CodeOpenOn { machine, .. } => match self.code_open_with(ep, &machine).await {
                Ok((addr, token)) => ipc::Reply::CodeAt {
                    addr: addr.to_string(),
                    token,
                },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::ReadAt {
                machine,
                path,
                offset,
            } => match self.read_at_with(ep, &machine, &path, offset).await {
                Ok((total, at_ms, bytes)) => ipc::Reply::Head {
                    total,
                    at_ms,
                    bytes: serde_bytes::ByteBuf::from(bytes),
                },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::CodeStopOn { machine, .. } => match self.code_stop_with(ep, &machine).await {
                Ok(()) => ipc::Reply::Accepted {
                    moved: 0,
                    landed: Vec::new(),
                },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::MkdirOn { machine, path } => {
                match self.mkdir_with(ep, &machine, &path).await {
                    Ok(()) => ipc::Reply::Made,
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::TouchOn { machine, path } => {
                match self.touch_with(ep, &machine, &path).await {
                    Ok(()) => ipc::Reply::Made,
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::RenameOn { machine, from, to } => {
                match self.rename_with(ep, &machine, &from, &to).await {
                    Ok(()) => ipc::Reply::Renamed,
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::LaunchOn { machine, target } => {
                match self.launch_with(ep, &machine, &target).await {
                    Ok(()) => ipc::Reply::Launched,
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::SearchOn {
                machine,
                dir,
                query,
            } => match self.search_with(ep, &machine, &dir, &query).await {
                Ok(found) => ipc::Reply::Found { found },
                Err(why) => ipc::Reply::Refused { why },
            },
        }
    }

    /// Opens a session on another machine (docs/KHOR.md 发起). The row
    /// is **that machine's**: it forks the process, it writes the
    /// registry entry, and the id that comes back is the id every face
    /// in the network will call it by. Asking this machine to open on
    /// itself is not an error — the sessions landing lists every
    /// machine, this one included — and then nothing touches the wire.
    // Eight arguments because eight things describe the session being
    // opened on the far machine. Bundling them into a struct would
    // reshape this call path for a lint's sake.
    #[allow(clippy::too_many_arguments)]
    async fn open_on_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        kind: &str,
        title: &str,
        cwd: &str,
        cmd: &[String],
        size: (u16, u16),
    ) -> Result<crate::SessionId, String> {
        let (channel, home) = self.resolve(machine)?;
        // A satellite's terminal is a host on its **sponsor** running
        // ssh (附属机器批): the host's persistence lives on the sponsor
        // as ever, and the WORK's persistence is borrowed from the
        // satellite's own tmux where it has one (`new-session -A`: the
        // connection can die and the next open walks back into the
        // same session — docs/SESSION.md 例外墓碑). Without tmux the
        // shell is as mortal as its connection, and the row says so by
        // ending when it ends — never dressed as more durable than it
        // is. An explicit command is run bare either way: the person
        // named what to run, and wrapping it would run something else.
        if let Some(sat) = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .filter(|d| d.sponsor.is_some())
        {
            let sponsor_id = sat.sponsor.clone().expect("filtered on sponsor");
            let host = sat
                .ssh_host
                .clone()
                .ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
            let sponsor_name = self
                .devices_loaded()?
                .doc
                .get(&sponsor_id)
                .map(|d| d.name)
                .ok_or_else(|| msg::machine_left_table(&channel))?;
            let mut ssh_cmd = crate::satellite::ssh_argv(&host);
            if !cmd.is_empty() {
                ssh_cmd.extend(cmd.iter().cloned());
            } else {
                // The tmux decision is made ON the satellite, at open
                // time, inside the same connection — the adoption probe
                // (`sat_tmux`) is display only (领队裁: cache, never
                // gate). A satellite that grew a tmux since adoption
                // gets its persistence on the next open, no re-adopt;
                // one that lost it falls back to a plain login shell
                // instead of a broken command. One ssh either way.
                ssh_cmd.push(
                    "command -v tmux >/dev/null 2>&1 && exec tmux new-session -A -s khor || exec \"$SHELL\" -l"
                        .to_owned(),
                );
            }
            let title = if title.is_empty() {
                channel.clone()
            } else {
                title.to_owned()
            };
            // The sponsor is an ordinary machine and the command is
            // ordinary ssh: one recursion, zero new protocol.
            return Box::pin(self.open_on_with(
                ep,
                &sponsor_name,
                kind,
                &title,
                "",
                &ssh_cmd,
                size,
            ))
            .await;
        }
        if home == self.device() {
            let cwd = if cwd.is_empty() {
                std::env::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"))
            } else {
                std::path::PathBuf::from(cwd)
            };
            let cmd = if cmd.is_empty() {
                vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())]
            } else {
                cmd.to_vec()
            };
            return self.open_persistent_at(&cwd, kind, title, &cmd, size);
        }
        let req = Request::Open {
            kind: kind.to_owned(),
            title: title.to_owned(),
            cwd: cwd.to_owned(),
            cmd: cmd.to_vec(),
            cols: size.0,
            rows: size.1,
        };
        match self.ask(ep, &channel, &req).await? {
            Response::Opened { session } => Ok(crate::SessionId(session)),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// A local address that pipes to another machine's session host, and
    /// the cookie that host's handshake wants — so the ordinary attach
    /// client can speak to a terminal on another computer without
    /// knowing there is a network under it.
    ///
    /// One pipe per session, kept: a second attach to the same session
    /// gets the address the first one bound. The far session's own id is
    /// the key, because that is what both ends call it.
    async fn reach_on_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        session: &crate::SessionId,
    ) -> Result<(std::net::SocketAddr, String), String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            let dir = self
                .live
                .dir_of(session)
                .ok_or_else(|| msg::not_a_session_id(&session.0))?;
            let hf = crate::host::read_host_file(&dir)?;
            let addr: std::net::SocketAddr = ([127, 0, 0, 1], hf.port).into();
            return Ok((addr, hf.cookie));
        }
        let (port, cookie) = match self
            .ask(
                ep,
                &channel,
                &Request::Reach {
                    session: session.0.clone(),
                },
            )
            .await?
        {
            Response::Reached { port, cookie } => (port, cookie),
            Response::Refused { why } => return Err(why),
            other => return Err(msg::peer_non_answer(format_args!("{other:?}"))),
        };
        {
            let mut held = self.reaches.lock().await;
            match held.get(session) {
                // Same live pipe, same far port: reuse it. One session,
                // one local port, however many faces attach.
                Some((addr, far, handle)) if !handle.is_finished() && *far == port => {
                    return Ok((*addr, cookie));
                }
                // Either the pipe is gone or it points somewhere that no
                // longer exists. Its listener has to go with it — a
                // stale one would keep answering on a port this call is
                // about to hand out a replacement for.
                Some((_, _, handle)) => {
                    handle.abort();
                    held.remove(session);
                }
                None => {}
            }
        }
        let borrow = self.tunnel_on(ep, machine).await?;
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| e.to_string())?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        let target = format!("127.0.0.1:{port}");
        let handle = tokio::spawn(async move {
            let _ = crate::tunnel::serve_fixed(std::sync::Arc::new(borrow), listener, target).await;
        });
        self.reaches
            .lock()
            .await
            .insert(session.clone(), (addr, port, handle));
        Ok((addr, cookie))
    }

    /// Resolve a machine name to a live connection on `ep`. The recipe
    /// every request-reply verb repeats — resolved twice on purpose, the
    /// second time against the table, because two reads can straddle a
    /// removal.
    ///
    /// **The connection is the one already open when there is one**
    /// (`Node::wires` carries the reasoning and the cost).
    ///
    /// Every caller of this is the same shape: dial, send one request,
    /// read one answer, walk away. That shape is what makes holding the
    /// connection safe — nobody here owns it past their own stream, and
    /// QUIC carries the next stream over the same pipe.
    ///
    /// **The dial happens outside the lock.** Two asks that arrive
    /// together may each dial and the second will replace the first in
    /// the map; the replaced one stays alive as long as its own asker
    /// holds it (the type is a handle, not the socket), so the cost of
    /// that race is one extra handshake, never a request cut off. The
    /// alternative — holding the map locked across a dial — would make
    /// one unreachable machine stall every ask to every other one.
    async fn dial_named(
        &self,
        ep: &iroh::Endpoint,
        channel: &str,
    ) -> Result<iroh::endpoint::Connection, String> {
        if let Some(open) = self.held_wire(channel).await {
            // 按机器暂停 (SHELL.md 第五轮 第 3 条) reaches a pipe that was
            // already open too: a pause that only stopped *new* dials
            // would keep talking to the machine on the old one.
            if crate::pause::peer_on(self.root(), &open.remote_id().to_string()) {
                self.wires.lock().await.remove(channel);
                open.close(0u32.into(), b"paused");
                return Err(msg::cant_reach_named(channel, msg::peer_paused_refusal()));
            }
            return Ok(open);
        }
        let target = self
            .devices_loaded()?
            .doc
            .by_name(channel)
            .ok_or_else(|| msg::machine_left_table(channel))?;
        let conn = self
            .dial(ep, &target.id, &target.addrs, ALPN)
            .await
            .map_err(|e| match e {
                DialFailure::TimedOut => msg::machine_unreachable_timeout(channel),
                DialFailure::Refused(why) => msg::cant_reach_named(channel, why),
            })?;
        self.wires
            .lock()
            .await
            .insert(channel.to_owned(), conn.clone());
        Ok(conn)
    }

    /// A look at the connections this node is holding — for tests and
    /// for whatever diagnostic verb wants it later. Taken **before**
    /// `serve` consumes the node: the map is shared, so a watch made
    /// here still sees what the serve does with it.
    pub fn wire_watch(&self) -> crate::WireWatch {
        crate::WireWatch::of(&self.wires)
    }

    /// One request to a named machine, on the pipe already open to it
    /// when there is one.
    ///
    /// **A failure retires the pipe.** `close_reason` catches a
    /// connection that was closed, and that is not the same as a
    /// connection whose far end is *gone*: kill a serve and its QUIC
    /// connection stays unclosed here until an idle timeout nobody
    /// wants to sit through — the stream opens, the answer never
    /// comes, and the card says 读不到应答 about a machine that came
    /// back seconds ago. Measured, the first time turing was upgraded
    /// under an open card. So the ask that fails is also the ask that
    /// tells this map its entry is stale.
    ///
    /// **It does not retry**, and that is the point of leaving it to
    /// the caller: `Open` on a fresh pipe after a timeout would be a
    /// second session on a machine that may well have started the
    /// first. Only a caller that knows its request is a *reading*
    /// (`load_on_with`) asks twice.
    async fn ask(
        &self,
        ep: &iroh::Endpoint,
        channel: &str,
        req: &Request,
    ) -> Result<Response, String> {
        let conn = self.dial_named(ep, channel).await?;
        let out = request(&conn, req).await;
        if out.is_err() {
            self.wires.lock().await.remove(channel);
        }
        out
    }

    /// The held connection to that machine, if one is held **and is
    /// still up**.
    ///
    /// A closed connection is dropped here rather than handed back: a
    /// pipe that reports a `close_reason` will fail every stream opened
    /// on it, and handing it out would turn one dead connection into an
    /// endless run of "machine unreachable" for a machine that is right
    /// there.
    async fn held_wire(&self, channel: &str) -> Option<iroh::endpoint::Connection> {
        let mut held = self.wires.lock().await;
        match held.get(channel) {
            Some(conn) if conn.close_reason().is_none() => Some(conn.clone()),
            Some(_) => {
                held.remove(channel);
                None
            }
            None => None,
        }
    }

    /// Routes an op to the resident serve when one holds this key.
    /// `None` = no live serve, take the direct path. `Some(Err)` = a
    /// serve is alive but unreachable — never fall back then: the key is
    /// taken and a second endpoint would knock both off.
    async fn via_serve(&self, op: ipc::Op) -> Option<Result<ipc::Reply, String>> {
        let f = match self.endpoint_file() {
            Ok(Some(f)) => f,
            _ => return None,
        };
        if f.ipc_port == 0 {
            return Some(Err(msg::serve_no_handoff_port(f.pid)));
        }
        Some(
            ipc::call(f.ipc_port, &f.ipc_cookie, op)
                .await
                .map_err(|e| msg::serve_handoff_failed(f.pid, e)),
        )
    }

    /// The exit side of a borrow tunnel (docs/NET.md 借网). One
    /// connection carries many streams — a lease is one connection shared
    /// by every consumer of this exit — so each accepted bi stream is one
    /// pipe to a target, spawned so a slow one does not block the next.
    ///
    /// Pairing is checked per stream, freshly: a device removed from the
    /// table mid-session must stop being answered, and reading the doc
    /// each time is what makes the removal bite. The verdict is handed to
    /// `serve_stream`, which puts it on the wire as the status byte.
    async fn serve_tunnel(&self, conn: iroh::endpoint::Connection) -> Result<(), String> {
        let remote = conn.remote_id().to_string();
        while let Ok((send, recv)) = conn.accept_bi().await {
            // A machine this one has paused is treated as a stranger
            // here, per stream, so a pause bites on a tunnel already up.
            let paired = !crate::pause::peer_on(self.root(), &remote)
                && self
                    .devices_loaded()
                    .map(|l| l.doc.get(&remote).is_some())
                    .unwrap_or(false);
            tokio::spawn(async move {
                let _ = crate::tunnel::serve_stream(send, recv, paired).await;
            });
        }
        Ok(())
    }

    async fn handle(
        &self,
        conn: iroh::endpoint::Connection,
        ep: &iroh::Endpoint,
    ) -> Result<(), String> {
        let remote = conn.remote_id().to_string();
        while let Ok((mut send, mut recv)) = conn.accept_bi().await {
            // 按机器暂停 (SHELL.md 第五轮 第 3 条): a machine this one has
            // paused gets no answer — the connection is closed, the way
            // a machine that is not there closes nothing. Asked per
            // request (a small file, off the accept loop) so a pause
            // bites on a connection that was already open.
            if crate::pause::peer_on(self.root(), &remote) {
                conn.close(0u32.into(), b"paused");
                return Ok(());
            }
            let bytes = recv
                .read_to_end(MAX_FRAME)
                .await
                .map_err(msg::request_unreadable)?;
            let resp = match proto::decode::<Request>(&bytes) {
                Ok(req) => self.dispatch(&remote, req, ep).await,
                Err(e) => Response::Refused { why: e },
            };
            send.write_all(&proto::encode(&resp)?)
                .await
                .map_err(msg::cant_write_reply)?;
            send.finish().map_err(msg::cant_close_stream)?;
        }
        Ok(())
    }

    async fn dispatch(&self, remote: &str, req: Request, ep: &iroh::Endpoint) -> Response {
        match self.dispatch_inner(remote, req, ep).await {
            Ok(resp) => resp,
            Err(why) => Response::Refused { why },
        }
    }

    async fn dispatch_inner(
        &self,
        remote: &str,
        req: Request,
        ep: &iroh::Endpoint,
    ) -> Result<Response, String> {
        match req {
            Request::Pair { token, name, addrs } => {
                let path = self.invite_path(&token)?;
                if !path.exists() {
                    return Err(msg::bad_invite().into());
                }
                // **Expired is its own answer, not "wrong code".** The
                // two have different fixes: a wrong or spent code means
                // check what you pasted, an expired one means ask for
                // another. Reading them as one word sends people to look
                // for a mistake they did not make (docs/UX.md 失败要说
                // 清是哪一种).
                if !Self::invite_is_fresh(&path) {
                    // It can never be used again, so it goes now rather
                    // than waiting for the next mint to sweep it.
                    let _ = fs::remove_file(&path);
                    return Err(msg::invite_expired(invite_window_minutes()));
                }
                // Burn before use: a token that pairs twice is a door
                // that never closes.
                fs::remove_file(&path).map_err(msg::cant_burn_invite)?;
                let loaded = self.devices_loaded()?;
                loaded.doc.upsert(remote, &name, &addrs)?;
                // Taking a machine back in is what pairing *is*, so a
                // ticket clears the tombstone. Deliberately here and not
                // on the far side: a machine that was shown the door
                // cannot let itself back in, it has to be handed a
                // ticket by somebody already inside.
                loaded.doc.set_gone(remote, false)?;
                let mut store = loaded.store;
                store.flush(&loaded.doc)?;
                Ok(Response::Paired {
                    name: self.name().to_owned(),
                    devices: B64.encode(loaded.doc.snapshot()?),
                })
            }
            Request::Sync { doc, have, changes } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let reply = if doc == "devices" {
                    let mut loaded = self.devices_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "seen" {
                    let mut loaded = self.seen_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "pins" {
                    let mut loaded = self.pins_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "dirpins" {
                    let mut loaded = self.dirpins_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "workspaces" {
                    let mut loaded = self.workspaces_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "webpins" {
                    let mut loaded = self.webpins_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "recents" {
                    let mut loaded = self.recents_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "agents" {
                    let mut loaded = self.agents_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "accounts" {
                    let mut loaded = self.accounts_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "titles" {
                    let mut loaded = self.titles_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if let Some(ch) = doc.strip_prefix("chat/") {
                    let dir = chat::channel_dir(self.root(), ch)
                        .ok_or_else(|| msg::bad_channel_name(format_args!("{ch:?}")))?;
                    let mut loaded = chat::open_channel(&dir, self.writer_peer())?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else {
                    return Err(msg::unknown_doc(doc));
                };
                Ok(Response::Synced {
                    version: reply.version,
                    changes: reply.changes,
                    items: reply.items as u64,
                })
            }
            Request::Fetch { digest, offset } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                self.serve_slice(&digest, offset)
            }
            Request::Sessions => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                Ok(Response::SessionRows {
                    rows: self.reportable_rows()?,
                })
            }
            Request::Days => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // A file read, off the reactor for the reason every
                // registry read is (the `Ls` arm's note).
                let root = self.root().to_path_buf();
                let day = tokio::task::spawn_blocking(move || crate::day::read(&root).to_wire())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Response::Days { day })
            }
            Request::OpenAgent {
                dir,
                title,
                form,
                agent,
            } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // The wizard's own call, on the machine that will run
                // it — off the reactor for the `Open` arm's reason, and
                // one more: it waits for the agent to answer
                // `session/new`, which is seconds on a cold start.
                let root = self.root().to_path_buf();
                let id = tokio::task::spawn_blocking(move || {
                    crate::Node::open(root)?.start_session(&dir, &title, &form, &agent)
                })
                .await
                .map_err(|e| e.to_string())??;
                Ok(Response::Opened { session: id.0 })
            }
            Request::Places => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Reads every vendor's records (the `Ls` arm's note).
                let root = self.root().to_path_buf();
                let places = tokio::task::spawn_blocking(move || {
                    Ok::<_, String>(crate::Node::open(root)?.places_shown())
                })
                .await
                .map_err(|e| e.to_string())??;
                Ok(Response::Places { places })
            }
            Request::Remove { path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Off the reactor: a directory going whole is a walk,
                // and `write` already holds that line for its own
                // reason. **Never the recycle bin over the wire** — a
                // bin on a machine nobody is sitting at is not a place
                // anything can be got back from (`Request::Remove`).
                let p = path.clone();
                tokio::task::spawn_blocking(move || crate::files::remove(&p, false))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Removed)
            }
            Request::Mkdir { path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Off the reactor with `Remove`, for `Remove`'s reason.
                tokio::task::spawn_blocking(move || crate::files::mkdir(&path))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Made)
            }
            Request::RenamePath { from, to } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                tokio::task::spawn_blocking(move || crate::files::rename_path(&from, &to))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Renamed)
            }
            Request::Launch { target } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Off the reactor: starting a process is a fork, and a
                // fork on the reactor is the one thing that stops every
                // other answer this machine owes.
                tokio::task::spawn_blocking(move || crate::files::launch(&target))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Launched)
            }
            Request::Search { dir, query } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Off the reactor with `Touched` below, for its reason
                // and more of it: this walks a project's files and
                // reads every text file in it.
                let found = tokio::task::spawn_blocking(move || crate::files::search(&dir, &query))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Found { found })
            }
            Request::Touched { path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Off the reactor: a walk of somebody's project tree is
                // thousands of stat() calls, and the cap is 20_000 of
                // them (`crate::workspace::MOST_ENTRIES`).
                let dir = std::path::PathBuf::from(path);
                let at_ms = tokio::task::spawn_blocking(move || crate::workspace::touched(&dir))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Response::Touched { at_ms })
            }
            Request::GitStatus { root } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Blocking on purpose and off the reactor: the walk is
                // three threads of stat() and object reads, measured at
                // 54 ms on this repo's 1525 files. Not long enough to
                // stream, far too long to hold the serve loop.
                let dir = std::path::PathBuf::from(root);
                let s = tokio::task::spawn_blocking(move || crate::git::status(&dir))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::GitStatus {
                    branch: s.branch,
                    rows: s.rows,
                    more: s.more,
                    ahead: s.ahead,
                    behind: s.behind,
                    fetched_ms: s.fetched_ms,
                })
            }
            Request::GitDiff { path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let file = std::path::PathBuf::from(path);
                let d = tokio::task::spawn_blocking(move || crate::git::diff(&file))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::GitDiff {
                    hunks: d.hunks,
                    more: d.more,
                })
            }
            Request::WritePath { path, was, bytes } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Blocking: a read of the file, a write of a temporary,
                // an fsync and a rename. The fsync is the one that can
                // take a while, and holding the serve loop through a
                // disk barrier is how one save makes every other
                // machine's next question look slow.
                let (at_ms, digest) = tokio::task::spawn_blocking(move || {
                    crate::files::write_whole(&path, &bytes, &was)
                })
                .await
                .map_err(|e| e.to_string())??;
                Ok(Response::Wrote { at_ms, digest })
            }
            Request::CodeHere => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // One spawn of `code --version`, ~70 ms measured. Off
                // the reactor for the same reason every other spawn in
                // this file is.
                let e = tokio::task::spawn_blocking(crate::code::probe)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Response::CodeHere {
                    version: e.version,
                    ready: e.ready,
                })
            }
            Request::CodeServe { .. } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // **This is the one op in this file that can take
                // minutes.** A first run fetches 643 MB from Microsoft
                // before it prints anything, and `code::serve` waits
                // ten minutes for that line. A blocking task is right:
                // the serve loop must not be one of the things waiting.
                let (port, token) = tokio::task::spawn_blocking(crate::code::serve)
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::CodeServing { port, token })
            }
            Request::CodeStop { .. } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                tokio::task::spawn_blocking(crate::code::stop)
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Acted {
                    moved: 0,
                    landed: Vec::new(),
                })
            }
            Request::Quotas => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Reads the providers when the cache has aged out —
                // seconds of HTTP, off the reactor for the `Vitals`
                // arm's reason (`accounts` module head).
                let root = self.root().to_path_buf();
                let readings = tokio::task::spawn_blocking(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| e.to_string())?;
                    rt.block_on(crate::Node::open(root)?.quota_readings())
                })
                .await
                .map_err(|e| e.to_string())??;
                Ok(Response::Quotas { readings })
            }
            Request::Load { top, by, desc } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // A process table and a file read, off the reactor for
                // the `Vitals` arm's reason (the first process-table
                // read in a process pays a CPU window).
                let root = self.root().to_path_buf();
                let load = tokio::task::spawn_blocking(move || {
                    crate::Node::load(&root, top as usize, by, desc)
                })
                .await
                .map_err(|e| e.to_string())?;
                Ok(Response::Load { load })
            }
            Request::Vitals => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Taken here, now — the answer is only ever about the
                // moment the asker gets it (`khor_core::Vitals`).
                //
                // **Off the reactor**, because sampling blocks: measured
                // at 1.9 s the first time in a process (the disk list
                // costs 1.2 s once) and ~215 ms after. Called inline it
                // stalled the QUIC connection this very reply travels on,
                // and the two real-connection avatar tests timed out at
                // twenty seconds — the same shape as the ledger's
                // "serve 里任何会等 DIAL_TIMEOUT 的事不能内联在 select
                // 循环", arriving through a sleep instead of a dial.
                let root = self.root().to_path_buf();
                let vitals = tokio::task::spawn_blocking(move || crate::vitals::sample(&root))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Response::Vitals { vitals })
            }
            Request::Watched => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // The list is this machine's own row in the table — the
                // asker wrote it there, and it arrived on the sync round
                // before this request. Sampled here, now, off the
                // reactor for the `Vitals` arm's reason (the disk list
                // blocks).
                let paths = self.watch_of(self.device_str());
                let rows = tokio::task::spawn_blocking(move || crate::vitals::watched(&paths))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Response::Watched { rows })
            }
            Request::Usage => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // **Off the reactor, for the reason vitals is** — the
                // rule this codebase settled on is "how long does it
                // block", not "is it dialling" (docs/handoff 坑节). A
                // first pass over this machine's transcripts is eighteen
                // seconds (`crate::usage::Meters::tally`), which would
                // stall the QUIC connection this very reply travels on.
                //
                // Cheap after that: measured at 0.11 s when something was
                // appended and under 10 ms when nothing was, so a peer
                // asking on every sync round costs this machine a couple
                // of percent of one core rather than a repeat of the
                // eighteen.
                let usage = self.usage_meters();
                let usage = tokio::task::spawn_blocking(move || usage.tally())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Response::Usage { usage })
            }
            Request::Act { session, action } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                match action.as_str() {
                    "accept" => {
                        let (moved, landed) =
                            self.accept_local(ep, &crate::SessionId(session)).await?;
                        Ok(Response::Acted { moved, landed })
                    }
                    // Closing is the third leg of 在任意设备开: a
                    // session you can open from anywhere and attach to
                    // from anywhere, but must walk to the machine to
                    // end, is a session the network only half holds.
                    // Run **here**, on the machine that owns the
                    // process — an Act is never re-routed (this enum's
                    // rule), so the asker's own close path stays the
                    // one that refuses rows that are not khor's.
                    "close" => {
                        let id = crate::SessionId(session);
                        let live = self.live.clone();
                        tokio::task::spawn_blocking(move || live.close_session(&id))
                            .await
                            .map_err(|e| e.to_string())??;
                        // Nothing moved and nothing landed: closing
                        // has no files, and an empty list is the frame's
                        // own word for "nobody said a path".
                        Ok(Response::Acted {
                            moved: 0,
                            landed: Vec::new(),
                        })
                    }
                    other => Err(msg::unknown_action(other)),
                }
            }
            Request::Ls { path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Off the reactor for the standing reason ("how long
                // does it block", 坑节): a directory on a network mount
                // can sit for seconds, and this reply's own QUIC
                // connection would sit with it.
                let (path, entries, truncated) =
                    tokio::task::spawn_blocking(move || crate::files::list_dir(&path))
                        .await
                        .map_err(|e| e.to_string())??;
                Ok(Response::Dir {
                    path: path.display().to_string(),
                    entries,
                    truncated,
                })
            }
            Request::Open {
                kind,
                title,
                cwd,
                cmd,
                cols,
                rows,
            } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Defaults are resolved **here**, on the machine that
                // will run it: the asker's home and the asker's `$SHELL`
                // are answers to a question about the wrong computer.
                // **`~` too** (2026-09-06, the wizard's shell kind sends
                // the directory a person typed): it names this
                // machine's home, which only this machine knows —
                // `start_session` keeps the same rule for agents.
                let home = std::env::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
                let cwd = match cwd.trim() {
                    "" | "~" => home,
                    d if d.starts_with("~/") => home.join(&d[2..]),
                    d => std::path::PathBuf::from(d),
                };
                let cmd = if cmd.is_empty() {
                    vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())]
                } else {
                    cmd
                };
                // Off the reactor for the reason every registry write
                // is (the `Ls` arm's note): these homes can be on a
                // network mount, and this one forks a process too.
                let live = self.live.clone();
                let category = Self::category_of_started(&kind);
                let id = tokio::task::spawn_blocking(move || {
                    let leaf = crate::link::fresh_leaf()?;
                    let id = crate::SessionId(format!("{kind}/{leaf}"));
                    live.register(&id, &kind, &title, None, category)?;
                    live.learn_place(&id, &cwd)?;
                    let dir = live
                        .dir_of(&id)
                        .ok_or_else(|| msg::not_a_session_id(&id.0))?;
                    crate::host::spawn_host_at(&cwd, &dir, &id, &cmd, (cols, rows))?;
                    Ok::<_, String>(id)
                })
                .await
                .map_err(|e| e.to_string())??;
                Ok(Response::Opened { session: id.0 })
            }
            Request::Reach { session } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let id = crate::SessionId(session);
                // The same door a click opens locally: a row with a
                // route but no host gets one stood up **on the machine
                // that owns it**, which is the only machine that can.
                // Without this every discovered tmux session on a Linux
                // box answered 宿主没了 — a sentence about a process
                // nobody had asked to exist yet.
                //
                // Off the reactor for the Open arm's reason and one
                // more: resolving the route re-runs the discovery sweep,
                // which shells out to `ps` and `tmux`.
                if !self.is_hosted(&id) {
                    let (live, bridging) = (self.live.clone(), id.clone());
                    tokio::task::spawn_blocking(move || live.attach_multiplexed(&bridging))
                        .await
                        .map_err(|e| e.to_string())??;
                }
                let dir = self
                    .live
                    .dir_of(&id)
                    .ok_or_else(|| msg::not_a_session_id(&id.0))?;
                let hf = crate::host::read_host_file(&dir)?;
                // **A host file outlives its host.** Without this the
                // asker gets an address whose far end is a closed port,
                // the pipe drops the connection, and the attach reports
                // 读不到帧: Connection reset — a sentence about a
                // socket, for a session that simply ended. Measured
                // (the probe's `sleep` ran out mid-test).
                if !crate::link::pid_alive(hf.host_pid) {
                    return Err(msg::host_gone().into());
                }
                Ok(Response::Reached {
                    port: hf.port,
                    cookie: hf.cookie,
                })
            }
            Request::SatVitals { sat } => {
                let doc = self.devices_loaded()?.doc;
                if doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Only a satellite this machine sponsors — answering
                // for someone else's would put a second, staler voice
                // on a row that already has its one speaker.
                let is_mine = doc
                    .get(&sat)
                    .and_then(|d| d.sponsor)
                    .is_some_and(|sp| sp == self.device_str());
                if !is_mine {
                    return Err(msg::not_my_satellite(&sat));
                }
                match self.vitals_of(&sat) {
                    Some((vitals, age_ms)) => Ok(Response::SatVitals { vitals, age_ms }),
                    // Not sampled yet: an honest gap, never zeros.
                    None => Err(msg::sat_not_sampled_yet().into()),
                }
            }
            Request::SatLs { sat, path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                // Off the reactor: an ssh dial plus a per-entry stat.
                let (at, entries, truncated) =
                    tokio::task::spawn_blocking(move || crate::satellite::ls(&host, &path))
                        .await
                        .map_err(|e| e.to_string())??;
                Ok(Response::Dir {
                    path: at,
                    entries,
                    truncated,
                })
            }
            Request::SatRemove { sat, path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                // Off the reactor with SatLs, for SatLs's reason.
                tokio::task::spawn_blocking(move || crate::satellite::remove(&host, &path))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Removed)
            }
            Request::SatTouch { sat, path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                tokio::task::spawn_blocking(move || crate::satellite::touch(&host, &path))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Made)
            }
            Request::SatMkdir { sat, path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                // Off the reactor with SatLs, for SatLs's reason.
                tokio::task::spawn_blocking(move || crate::satellite::mkdir(&host, &path))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Made)
            }
            Request::SatRename { sat, from, to } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                tokio::task::spawn_blocking(move || {
                    crate::satellite::rename_path(&host, &from, &to)
                })
                .await
                .map_err(|e| e.to_string())??;
                Ok(Response::Renamed)
            }
            Request::SatLaunch { sat, target } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                tokio::task::spawn_blocking(move || crate::satellite::launch(&host, &target))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Launched)
            }
            Request::SatSearch { sat, dir, query } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                // Off the reactor with SatLs, for SatLs's reason: one
                // ssh dial and a walk of that machine's disk.
                let found = tokio::task::spawn_blocking(move || {
                    crate::satellite::search(&host, &dir, &query)
                })
                .await
                .map_err(|e| e.to_string())??;
                Ok(Response::Found { found })
            }
            Request::SatTouched { sat, path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                // Off the reactor with SatLs, for SatLs's reason: one
                // ssh dial and a walk of that machine's disk.
                let at_ms =
                    tokio::task::spawn_blocking(move || crate::satellite::touched(&host, &path))
                        .await
                        .map_err(|e| e.to_string())??;
                Ok(Response::Touched { at_ms })
            }
            Request::SatGitStatus { sat, root } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                let st =
                    tokio::task::spawn_blocking(move || crate::satellite::git_status(&host, &root))
                        .await
                        .map_err(|e| e.to_string())??;
                Ok(Response::GitStatus {
                    branch: st.branch,
                    rows: st.rows,
                    more: st.more,
                    ahead: st.ahead,
                    behind: st.behind,
                    fetched_ms: st.fetched_ms,
                })
            }
            Request::SatGitDiff { sat, path } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                let d =
                    tokio::task::spawn_blocking(move || crate::satellite::git_diff(&host, &path))
                        .await
                        .map_err(|e| e.to_string())??;
                Ok(Response::GitDiff {
                    hunks: d.hunks,
                    more: d.more,
                })
            }
            Request::SatFetchPath { sat, path, offset } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                let (total, at_ms, bytes) = self.sat_slice(&sat, &host, &path, offset).await?;
                Ok(Response::PathSlice {
                    total,
                    at_ms,
                    bytes: serde_bytes::ByteBuf::from(bytes),
                })
            }
            Request::SatWrite {
                sat,
                path,
                was,
                bytes,
            } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                let host = self.my_satellite_host(&sat)?;
                let (at_ms, digest) = self.sat_write(&sat, &host, &path, &was, &bytes).await?;
                Ok(Response::Wrote { at_ms, digest })
            }
            Request::FetchPath { path, offset } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err(msg::not_paired().into());
                }
                // Off the reactor with Ls, for Ls's reason.
                let (total, at_ms, bytes) =
                    tokio::task::spawn_blocking(move || crate::files::read_slice(&path, offset))
                        .await
                        .map_err(|e| e.to_string())??;
                Ok(Response::PathSlice {
                    total,
                    at_ms,
                    bytes: serde_bytes::ByteBuf::from(bytes),
                })
            }
        }
    }

    /// The ssh destination for a satellite **this machine sponsors** —
    /// the one gate every satellite op passes: not in the table, not
    /// mine, or mine without a road each refuse by name. One row, one
    /// speaker (附属机器批).
    fn my_satellite_host(&self, sat: &str) -> Result<String, String> {
        let d = self
            .devices_loaded()?
            .doc
            .get(sat)
            .ok_or_else(|| msg::not_my_satellite(sat))?;
        if d.sponsor.as_deref() != Some(self.device_str()) {
            return Err(msg::not_my_satellite(sat));
        }
        d.ssh_host.ok_or_else(|| msg::sat_no_ssh_host(sat))
    }

    /// Where a satellite pull stages on the sponsor: one spot per
    /// (satellite, path), content-addressed so a re-pull lands on the
    /// same file and the previous copy is simply replaced.
    /// One slice of a satellite's file, off a copy staged on this
    /// machine.
    ///
    /// Staged once per pull, on the first slice: one scp per file, then
    /// the ordinary local slice loop — a per-slice ssh would make a
    /// 100MB pull four hundred dials. The staging name is
    /// content-addressed by `(sat, path)`, so a second pull of the same
    /// file re-stages over the same spot and `PathSlice`'s own change
    /// contract (total/at_ms moving = start over) covers the swap.
    ///
    /// **Two callers, and that is the point** (卫星工作区批): the
    /// sponsor answering [`Request::SatFetchPath`] for somebody else,
    /// and the sponsor reading its own satellite's file for its own
    /// screen. They were one arm and a copy of it until the second
    /// caller existed; a staging rule written twice is two rules for
    /// where a file lands.
    async fn sat_slice(
        &self,
        sat: &str,
        host: &str,
        path: &str,
        offset: u64,
    ) -> Result<(u64, u64, Vec<u8>), String> {
        let staged = self.sat_staging_path(sat, path)?;
        if offset == 0 || !staged.exists() {
            let (h, p, s) = (host.to_owned(), path.to_owned(), staged.clone());
            tokio::task::spawn_blocking(move || crate::satellite::pull_file(&h, &p, &s))
                .await
                .map_err(|e| e.to_string())??;
        }
        let staged_str = staged.display().to_string();
        tokio::task::spawn_blocking(move || crate::files::read_slice(&staged_str, offset))
            .await
            .map_err(|e| e.to_string())?
    }

    /// Saves a whole file onto a satellite, keeping the same promise
    /// every other save keeps (工作区批7).
    ///
    /// **The staged copy is what makes the two digests meet.** khor
    /// hands out blake3 and a satellite can only compute sha256, so
    /// the check would be impossible if it had to happen in one place.
    /// It does not: the sponsor already holds the copy this file was
    /// read off (`sat_slice` stages one per `(sat, path)`), and one
    /// pile of bytes has both digests. If the staged copy hashes to
    /// `was`, its sha256 is what the far machine must still see; the
    /// script over there compares and writes, or compares and does
    /// not.
    ///
    /// A staged copy that is missing or no longer hashes to `was` is
    /// not a refusal yet — it is re-pulled once, because another
    /// reader re-staging the same file is ordinary and has nothing to
    /// do with this save. Only a fresh copy that still does not hash
    /// to `was` means the file really moved.
    async fn sat_write(
        &self,
        sat: &str,
        host: &str,
        path: &str,
        was: &str,
        bytes: &[u8],
    ) -> Result<(u64, String), String> {
        // The wire's own ceiling, worn here too: a save is one frame
        // everywhere else in khor and a satellite is not the place to
        // invent a second rule.
        if bytes.len() as u64 > crate::proto::SLICE {
            return Err(msg::too_big_to_write(
                bytes.len() as u64,
                crate::proto::SLICE,
            ));
        }
        let staged = self.sat_staging_path(sat, path)?;
        let want = {
            let (h, p, sp, was) = (
                host.to_owned(),
                path.to_owned(),
                staged.clone(),
                was.to_owned(),
            );
            tokio::task::spawn_blocking(move || -> Result<String, String> {
                let mut now = fs::read(&sp).unwrap_or_default();
                if now.is_empty() || crate::files::digest(&now) != was {
                    crate::satellite::pull_file(&h, &p, &sp)?;
                    now = fs::read(&sp).map_err(|e| msg::cant_read(sp.display(), e))?;
                }
                if crate::files::digest(&now) != was {
                    return Err(msg::file_changed_under_you(&p));
                }
                Ok(crate::satellite::sha256_hex(&now))
            })
            .await
            .map_err(|e| e.to_string())??
        };
        let (h, p, mine) = (host.to_owned(), path.to_owned(), bytes.to_vec());
        let at_ms =
            tokio::task::spawn_blocking(move || crate::satellite::write_file(&h, &p, &want, &mine))
                .await
                .map_err(|e| e.to_string())??;
        // The staged copy is now a copy of something nobody holds any
        // more. Written through so the next read off this sponsor
        // shows what was just saved rather than what it replaced.
        let _ = fs::write(&staged, bytes);
        Ok((at_ms, crate::files::digest(bytes)))
    }

    /// Whether `channel` names a satellite, and if so its id together
    /// with **the sponsor to ask** — `None` for the sponsor meaning
    /// this machine is it and there is no wire to use.
    ///
    /// **One reading of「is this a satellite and whose」**, because the
    /// three roads that take it (workspaces, the file slice, git) were
    /// three copies of the same six lines and the fourth would have
    /// been a fourth.
    fn sat_and_sponsor(&self, channel: &str) -> Result<Option<(String, Option<String>)>, String> {
        let doc = self.devices_loaded()?.doc;
        let Some(target) = doc.by_name(channel) else {
            return Err(msg::machine_left_table(channel));
        };
        let Some(sp) = target.sponsor.clone() else {
            return Ok(None);
        };
        if sp == self.device_str() {
            return Ok(Some((target.id.clone(), None)));
        }
        let sponsor = doc
            .get(&sp)
            .ok_or_else(|| msg::machine_left_table(channel))?;
        Ok(Some((target.id.clone(), Some(sponsor.name.clone()))))
    }

    fn sat_staging_path(&self, sat: &str, path: &str) -> Result<std::path::PathBuf, String> {
        let dir = self.root().join(".khor").join("satpull");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let digest = blake3::hash(format!("{sat}\n{path}").as_bytes()).to_hex();
        Ok(dir.join(format!(
            "{}-{}",
            &sat[..8.min(sat.len())],
            &digest.as_str()[..16]
        )))
    }

    /// One slice of an offered payload. The offer's recorded size is the
    /// contract: a file that changed size since the offer is refused, not
    /// silently served (the digest would fail far away, much later).
    fn serve_slice(&self, digest: &str, offset: u64) -> Result<Response, String> {
        use std::io::{Read, Seek, SeekFrom};
        let mut offer =
            crate::transfer::load_offer(self.root(), digest)?.ok_or(msg::offer_lost())?;
        let meta = fs::metadata(&offer.path)
            .map_err(|e| msg::offered_file_unreadable(offer.path.display(), e))?;
        if meta.len() != offer.size {
            return Err(msg::offered_file_changed().into());
        }
        if offset > offer.size {
            return Err(msg::offset_out_of_range(offset, offer.size));
        }
        let mut f = fs::File::open(&offer.path).map_err(msg::offered_file_wont_open)?;
        f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        let want = (offer.size - offset).min(proto::SLICE) as usize;
        let mut buf = vec![0u8; want];
        f.read_exact(&mut buf).map_err(msg::cant_read_slice)?;
        offer.started = true;
        offer.done = offset + want as u64 >= offer.size;
        crate::transfer::save_offer(self.root(), digest, &offer)?;
        Ok(Response::Slice {
            total: offer.size,
            bytes: serde_bytes::ByteBuf::from(buf),
        })
    }

    /// Where a transfer's payloads land, one path per offered file.
    /// For saying: accept's answer must name the landing, or the next
    /// person greps the disk for it — the 2026-08-19 install incident
    /// started exactly there.
    pub fn transfer_landing(
        &self,
        id: &crate::SessionId,
    ) -> Result<Vec<std::path::PathBuf>, String> {
        let Some(msg_id) = id.0.strip_prefix("transfer/") else {
            return Err(msg::no_such_transfer(&id.0));
        };
        let (channel, m) = self.find_transfer(msg_id)?;
        let crate::MsgBody::Files(files) = &m.body else {
            return Err(msg::no_such_transfer(&id.0));
        };
        let dir = chat::channel_dir(self.root(), &channel)
            .ok_or_else(|| msg::bad_channel_name(format_args!("{channel:?}")))?;
        Ok(files
            .iter()
            .map(|f| crate::transfer::payload_path(&dir, f))
            .collect())
    }

    /// Approves a transfer and pulls its payload from home. Resumes from
    /// an existing partial; verifies the blake3 digest before the payload
    /// gets its real name. Routed through the resident serve when one
    /// holds the key.
    ///
    /// Returns bytes actually moved this run, and where the files are
    /// **on the machine that received them** — which is this one only
    /// when the transfer was addressed here. See
    /// [`crate::proto::Response::Acted`] for why the paths travel
    /// instead of being worked out at this end.
    pub async fn accept(&self, id: &crate::SessionId) -> Result<(u64, Vec<String>), String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::Accept {
                session: id.0.clone(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Accepted { moved, landed } => Ok((moved, landed)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self.accept_with(&ep, id).await;
        ep.close().await;
        outcome
    }

    /// Opens a borrow lease to a machine's network (docs/NET.md 借网):
    /// a live tunnel connection whose streams each reach some `host:port`
    /// the exit can reach. The returned [`Borrow`] owns the endpoint it
    /// dialed from, so it must be kept alive while any stream runs.
    ///
    /// Binds its own endpoint — right for a one-shot caller (the CLI verb,
    /// a test). The resident proxy reuses the serve's endpoint instead;
    /// binding a second one under this key would knock the serve off
    /// (endpoint.rs: one live endpoint per key).
    ///
    /// [`Borrow`]: crate::tunnel::Borrow
    pub async fn tunnel_to(&self, machine: &str) -> Result<crate::tunnel::Borrow, String> {
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let conn = self.dial_tunnel(&ep, machine).await?;
        Ok(crate::tunnel::Borrow::new(ep, conn))
    }

    /// Dials a borrow lease on an endpoint the caller keeps alive — the
    /// resident serve's own. Same dial as [`tunnel_to`], but the `Borrow`
    /// does not own the endpoint, because the serve outlives it.
    async fn tunnel_on(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
    ) -> Result<crate::tunnel::Borrow, String> {
        Ok(crate::tunnel::Borrow::on_shared(
            self.dial_tunnel(ep, machine).await?,
        ))
    }

    /// Resolves a machine and opens the tunnel connection to it on `ep`.
    /// The one recipe both borrow dialers share.
    async fn dial_tunnel(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
    ) -> Result<iroh::endpoint::Connection, String> {
        let (channel, home) = self.resolve(machine)?;
        // Borrowing your own network is nothing to borrow — this machine
        // *is* that network. Dialling your own key would knock the serve
        // off its endpoint and then time out; refuse it in words instead.
        // (Direct browsing from this machine, with no exit, is its own
        // thing and on the ledger.)
        if home == self.device() {
            return Err(msg::borrow_self().into());
        }
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        self.dial(ep, &target.id, &target.addrs, endpoint::TUNNEL_ALPN)
            .await
            .map_err(|e| match e {
                DialFailure::TimedOut => msg::machine_unreachable_timeout(&channel),
                DialFailure::Refused(why) => msg::cant_reach_named(&channel, why),
            })
    }

    /// Starts a borrow the serve hosts: dials the lease on `ep`, binds a
    /// local proxy port, and spawns the proxy task tracked so a later
    /// close can reap it. Returns the borrow's id and the proxy's address
    /// — the caller points a browser at the latter. Runs only inside the
    /// serve (it is the key's one endpoint).
    ///
    /// **`listed` decides whether it wears a session row**, and the rule
    /// is docs/NET.md 可达性 (2026-08-17 定形, 一层不占会话列表): the one
    /// borrow that is a session of its own is the one a person holds by
    /// hand (`khor borrow`), because it has no other row to show itself
    /// on and nowhere else to hang its lifetime. A browsing tab's borrow
    /// has both — the tab is its face and its lifetime — so it is
    /// unlisted, and the session list stays a list of sessions.
    ///
    /// The two are **not shared even on one machine**: a hand-held proxy
    /// and a browsing tab pointed at the same exit are two leases, so
    /// closing the row a person is looking at cannot pull the network out
    /// from under a page, and closing the last tab cannot take away a
    /// proxy somebody is holding by hand.
    pub async fn start_borrow(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        listed: bool,
    ) -> Result<(crate::SessionId, std::net::SocketAddr), String> {
        // **One borrow per exit machine, however many pages point at it.**
        // Before this, every open dialled a fresh lease, bound a fresh
        // port and wrote a fresh session row, so three pages at one
        // machine spent three of each and the list filled with borrows
        // nobody could tell apart. `reaches` has kept this shape for
        // attach pipes all along — same file, same reasoning, and its
        // field sits beside this one in `Node`.
        {
            let mut held = self.borrows.lock().await;
            let mine = held.iter().find_map(|(id, b)| {
                (b.machine == machine && b.listed == listed)
                    .then(|| (id.clone(), b.addr, b.task.is_finished()))
            });
            if let Some((id, addr, finished)) = mine {
                // Alive, and — if it is the kind that has a row — still
                // claimed. An unlisted borrow has no row, so asking
                // `claims` about it would answer "gone" about something
                // that was never there.
                if !finished && (!listed || self.live.claims(&id)) {
                    return Ok((id, addr));
                }
                // The task died, or a close from any process took the row
                // out from under it. Either way the port it holds is not
                // one to hand out again — drop it here rather than wait
                // for the serve's next tick to reap it.
                if let Some(gone) = held.remove(&id) {
                    gone.task.abort();
                }
            }
        }
        let borrow = self.tunnel_on(ep, machine).await?;
        let id = if listed {
            let id =
                self.open_ephemeral(khor_core::kind::BORROW, &msg::borrowing_title(machine))?;
            // The lease starts with no pipes, so the row opens 空闲 rather
            // than the 忙碌 a fresh registration writes.
            self.report_state(&id, khor_core::State::Idle)?;
            id
        } else {
            // **Minted, not registered.** An unlisted borrow needs a
            // handle to be closed by and nothing else; the same shape as
            // `open_ephemeral`'s id so nothing downstream has to tell
            // them apart.
            crate::SessionId(format!("{}/{}", khor_core::kind::BORROW, fresh_leaf()?))
        };
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| e.to_string())?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        // The activity callback rewrites the row as the lease crosses
        // between busy and idle. It owns a bare registry handle (no
        // discovery — it only writes this one row's state), so the proxy
        // task can reach the registry without borrowing the node.
        //
        // **An unlisted borrow gets a callback that does nothing**, and
        // that is not a stub: 忙碌 and 空闲 are words a *row* wears, and
        // this borrow does not have one. Writing them anyway would report
        // a state for a session that is not in the list.
        let activity: crate::tunnel::Activity = if listed {
            let by = crate::live::LiveKind::new(self.root().clone(), self.device());
            let sid = id.clone();
            std::sync::Arc::new(move |busy| {
                let word = if busy {
                    khor_core::State::Busy
                } else {
                    khor_core::State::Idle
                };
                let _ = by.report(&sid, word, crate::live::Source::Reported);
            })
        } else {
            std::sync::Arc::new(|_busy| {})
        };
        let task = tokio::spawn(async move {
            let _ =
                crate::tunnel::serve_proxy(std::sync::Arc::new(borrow), listener, activity).await;
        });
        let hosted = crate::HostedBorrow {
            machine: machine.to_owned(),
            addr,
            listed,
            task,
        };
        self.borrows.lock().await.insert(id.clone(), hosted);
        Ok((id, addr))
    }

    /// Ends an unlisted borrow by the handle it was answered with: aborts
    /// the proxy task, which frees the port. Answers whether there was
    /// one — **and a caller may ignore that**: dropping a borrow already
    /// gone is the outcome the caller wanted, not a failure.
    ///
    /// The listed kind is not ended here. It is ended by closing the row
    /// a person is looking at, and `reap_borrows` picks that up on the
    /// tick — which is the whole reason that kind has a row.
    pub(crate) async fn drop_hosted_borrow(&self, id: &crate::SessionId) -> bool {
        match self.borrows.lock().await.remove(id) {
            Some(gone) => {
                gone.task.abort();
                true
            }
            None => false,
        }
    }

    /// Aborts any hosted borrow whose session row is gone (a close from
    /// any process removed it) and drops finished ones, freeing the proxy
    /// port. Called on the serve's tick, so a close from another process
    /// is reaped within one sync period.
    ///
    /// **An unlisted borrow is only reaped when its task ends.** It has
    /// no row to lose, so `claims` says nothing about it — the thing that
    /// ends it is `drop_hosted_borrow`, asked for by the face that opened
    /// it. Testing it against `claims` would reap every one of them on
    /// the first tick.
    pub(crate) async fn reap_borrows(&self) {
        let mut held = self.borrows.lock().await;
        held.retain(|id, b| {
            if b.task.is_finished() || (b.listed && !self.live.claims(id)) {
                b.task.abort();
                false
            } else {
                true
            }
        });
    }

    /// Removes borrow rows left by a previous serve: their proxies died
    /// with that process, so the rows point at ports nobody holds. Run
    /// once at startup, before answering anyone (docs/handoff: 改了结构,
    /// 为旧结构服务的东西不会自己消失).
    pub(crate) fn sweep_stale_borrows(&self) {
        // Watermark is irrelevant here — we look only at kind and id.
        for row in self.live.rows(|_| 0) {
            if row.kind.0 == khor_core::kind::BORROW {
                let _ = self.live.close_session(&row.id);
            }
        }
    }

    /// Borrows a machine's network through the resident serve, **as a
    /// session of its own**: the serve hosts the proxy and the lease, and
    /// this returns the borrow session's id and the local proxy address.
    /// Requires a serve — the proxy is long-lived and must live in the
    /// process that holds the key, so unlike ls/pull there is no
    /// bind-your-own fallback.
    ///
    /// This is the hand-held one (`khor borrow`): a person is given a
    /// local port to point something at, and the row is how they find it
    /// again and how they end it. A face opening a page wants
    /// [`Node::borrow_unlisted`] instead — see docs/NET.md 可达性.
    pub async fn borrow(&self, machine: &str) -> Result<(String, String), String> {
        match self
            .via_serve(ipc::Op::Borrow {
                machine: machine.to_owned(),
            })
            .await
        {
            Some(reply) => match reply? {
                ipc::Reply::Borrowing { session, addr } => Ok((session, addr)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            },
            None => Err(msg::borrow_needs_serve().into()),
        }
    }

    /// Borrows a machine's network **without taking a row in the session
    /// list** (docs/NET.md 可达性, 一层不占会话列表). What a browsing tab
    /// asks for: the tab is already the face of it and already owns its
    /// lifetime, so a row would be a session in the list that nothing
    /// treats as a session.
    ///
    /// Answers the same pair as [`Node::borrow`], and the id in it is a
    /// handle rather than a row — [`Node::drop_borrow`] is what ends it,
    /// because there is no row to close.
    pub async fn borrow_unlisted(&self, machine: &str) -> Result<(String, String), String> {
        match self
            .via_serve(ipc::Op::BorrowUnlisted {
                machine: machine.to_owned(),
            })
            .await
        {
            Some(reply) => match reply? {
                ipc::Reply::Borrowing { session, addr } => Ok((session, addr)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            },
            None => Err(msg::borrow_needs_serve().into()),
        }
    }

    /// Ends an unlisted borrow by its handle. **Ending one that is
    /// already gone is not an error** — the caller wanted it not to
    /// exist, and that is the state either way.
    ///
    /// Answers `Ok(())` when there is no resident serve too, for the same
    /// reason: no serve means no hosted borrow, so there is nothing left
    /// holding a port. `borrow_unlisted` refuses in that case because
    /// there it is a real failure — it could not do the thing.
    pub async fn drop_borrow(&self, session: &str) -> Result<(), String> {
        match self
            .via_serve(ipc::Op::DropBorrow {
                session: session.to_owned(),
            })
            .await
        {
            Some(reply) => match reply? {
                ipc::Reply::Accepted { .. } => Ok(()),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            },
            None => Ok(()),
        }
    }

    /// Opens a session on `machine` (docs/KHOR.md 发起). Routed like
    /// every one-shot verb: through the resident serve when one holds
    /// this key, on a bound-and-closed endpoint otherwise — the process
    /// that opens it is not the one that runs it, so nothing here needs
    /// to outlive the call.
    pub async fn open_on(
        &self,
        machine: &str,
        kind: &str,
        title: &str,
        cwd: &str,
        cmd: &[String],
        size: (u16, u16),
    ) -> Result<crate::SessionId, String> {
        let op = ipc::Op::OpenOn {
            machine: machine.to_owned(),
            kind: kind.to_owned(),
            title: title.to_owned(),
            cwd: cwd.to_owned(),
            cmd: cmd.to_vec(),
            cols: size.0,
            rows: size.1,
        };
        if let Some(reply) = self.via_serve(op).await {
            return match reply? {
                ipc::Reply::OpenedOn { session } => Ok(crate::SessionId(session)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self
            .open_on_with(&ep, machine, kind, title, cwd, cmd, size)
            .await;
        ep.close().await;
        outcome
    }

    /// An agent session by name, on any machine — the wizard's four
    /// answers carried to where the session is born ([`Request::OpenAgent`]).
    /// Through the resident when there is one, like [`Node::open_on`].
    pub async fn open_agent_on(
        &self,
        machine: &str,
        dir: &str,
        title: &str,
        form: &str,
        agent: &str,
    ) -> Result<crate::SessionId, String> {
        let op = ipc::Op::OpenAgentOn {
            machine: machine.to_owned(),
            dir: dir.to_owned(),
            title: title.to_owned(),
            form: form.to_owned(),
            agent: agent.to_owned(),
        };
        if let Some(reply) = self.via_serve(op).await {
            return match reply? {
                ipc::Reply::OpenedOn { session } => Ok(crate::SessionId(session)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self
            .open_agent_on_with(&ep, machine, dir, title, form, agent)
            .await;
        ep.close().await;
        outcome
    }

    async fn open_agent_on_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        dir: &str,
        title: &str,
        form: &str,
        agent: &str,
    ) -> Result<crate::SessionId, String> {
        let (channel, home) = self.resolve(machine)?;
        // A satellite has no khor of its own to host a conversation:
        // what `open_on` reaches over ssh is a shell, and an agent
        // process khor speaks a protocol to has to run beside a khor.
        if self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .is_some_and(|d| d.sponsor.is_some())
        {
            return Err(msg::agent_not_on_a_satellite(&channel));
        }
        if home == self.device() {
            // Off the reactor: this waits for the agent's `session/new`.
            let root = self.root().to_path_buf();
            let (dir, title, form, agent) = (
                dir.to_owned(),
                title.to_owned(),
                form.to_owned(),
                agent.to_owned(),
            );
            return tokio::task::spawn_blocking(move || {
                crate::Node::open(root)?.start_session(&dir, &title, &form, &agent)
            })
            .await
            .map_err(|e| e.to_string())?;
        }
        let req = Request::OpenAgent {
            dir: dir.to_owned(),
            title: title.to_owned(),
            form: form.to_owned(),
            agent: agent.to_owned(),
        };
        match self.ask(ep, &channel, &req).await? {
            Response::Opened { session } => Ok(crate::SessionId(session)),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// 常去的地方 on any machine (`(path, shown, sessions, last_ms)`,
    /// newest first). This machine's answers without a resident; another
    /// machine's goes through one when there is one. An older khor over
    /// there answers with no list at all, which the caller treats as
    /// silence and not as an error — the wizard's box still takes a path.
    pub async fn places_on(
        &self,
        machine: &str,
    ) -> Result<Vec<(String, String, u32, i64)>, String> {
        if machine.is_empty() || machine == self.name() {
            return Ok(self.places_shown());
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::PlacesOn {
                machine: machine.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Places { places } => Ok(places),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self.places_on_with(&ep, machine).await;
        ep.close().await;
        outcome
    }

    /// What a machine is busy with (`khor_core::Load`), `top` rows deep.
    /// This machine's answers by sampling, without a resident; another
    /// machine's goes through one when there is one. **An older khor
    /// over there is said out loud**, not painted as an empty table:
    /// `Places` may fall silent because a wizard with no list still
    /// takes a path, but a card with no rows reads as an idle machine,
    /// which is the one thing this screen must never say by accident.
    pub async fn load_on(
        &self,
        machine: &str,
        top: u32,
        by: khor_core::TopBy,
        desc: bool,
    ) -> Result<khor_core::Load, String> {
        if machine.is_empty() || machine == self.name() {
            let root = self.root.clone();
            return tokio::task::spawn_blocking(move || Self::load(&root, top as usize, by, desc))
                .await
                .map_err(|e| e.to_string());
        }
        let op = ipc::Op::LoadOn {
            machine: machine.to_owned(),
            top,
            by,
            desc,
        };
        if let Some(reply) = self.via_serve(op).await {
            return match reply? {
                ipc::Reply::Load { load } => Ok(load),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self.load_on_with(&ep, machine, top, by, desc).await;
        ep.close().await;
        outcome
    }

    async fn load_on_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        top: u32,
        by: khor_core::TopBy,
        desc: bool,
    ) -> Result<khor_core::Load, String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            let root = self.root.clone();
            return tokio::task::spawn_blocking(move || Self::load(&root, top as usize, by, desc))
                .await
                .map_err(|e| e.to_string());
        }
        let req = Request::Load { top, by, desc };
        // **Asked twice, because this one is a reading.** The first ask
        // may land on a pipe whose far end went away without closing
        // it (`ask` says why that is not the same as a closed one) —
        // and by the time it fails, `ask` has retired that entry, so
        // the second ask dials. Safe to repeat precisely because it
        // reads and changes nothing; `Open` next door may not.
        let answer = match self.ask(ep, &channel, &req).await {
            Ok(a) => a,
            Err(_) => self.ask(ep, &channel, &req).await?,
        };
        match answer {
            Response::Load { load } => Ok(load),
            // **The far side gets to say why.** This used to read
            // `Refused { .. }` and answer `LOAD_PREDATES` for every
            // refusal, arguing that a machine this side just resolved
            // from its own table can only be refusing because its khor
            // predates the op. **The inference was right about the
            // cause and wrong about everything it went on to say.** On
            // 2026-09-04 ordering by PID on a remote printed 「那边的
            // khor 版本旧,还报不了进程和 CPU 走势」: the first half was
            // true (that khor's `TopBy` has no `Pid`), and the second
            // half was false in the same second — every other column on
            // that same machine answered, top and day chart included.
            // One word it could not read became a sentence about the
            // whole op, and the reader is sent to fix the wrong thing.
            //
            // A `why` that travelled all this way is the answer to the
            // question the sentence was guessing at, so it is what goes
            // on screen. The guess is kept only for a refusal that
            // arrived empty-handed, which is the one case where there
            // is nothing to say but what we can infer.
            Response::Refused { why } if why.trim().is_empty() => Err(msg::load_predates().into()),
            Response::Refused { why } => Err(msg::load_refused(why)),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Declares a directory on some machine a workspace, for everyone.
    /// `machine` empty is this one.
    ///
    /// **Adding is the one workspace verb that goes out on the wire**,
    /// and only to ask one question: is that a directory? The list
    /// itself is local ([`Node::workspaces`]) because it is a set of
    /// decisions, but a decision about a path nobody can stand in is a
    /// row that will never open, made at the one moment somebody is
    /// there to be told. A machine that cannot be reached refuses the
    /// add rather than taking it on faith — the alternative is a list
    /// that fills up with typos.
    ///
    /// This machine answers from its own filesystem; another machine
    /// answers through `Ls`, which is also the road a satellite's
    /// listing already takes through its sponsor ([`Node::ls_of`]), so
    /// all three kinds of machine are checked by one call.
    ///
    /// **Adding something already on the list changes nothing** — in
    /// particular it does not unpin it. `add` means 「be on the list」,
    /// and it is the same word whether or not it already was.
    pub async fn ws_add(&self, machine: &str, path: &str) -> Result<(), String> {
        if !std::path::Path::new(path).is_absolute() {
            return Err(msg::path_not_absolute(path));
        }
        let (_, home) = self.resolve_or_here(machine)?;
        // **Stored as the holding machine spells it, links resolved**
        // — `files::list_dir` says why. The key must match what a
        // session in that directory will record, or the row never
        // shows its own sessions.
        let real = if home == self.device() {
            if !std::path::Path::new(path).is_dir() {
                return Err(msg::ws_not_a_dir(path));
            }
            std::fs::canonicalize(path)
                .map_err(|e| msg::cant_list_dir(path, e))?
                .display()
                .to_string()
        } else {
            // That machine's own sentence about why it could not list
            // the path is the one to show — it already says whether the
            // path is a file, is missing, or the machine is unreachable
            // (the `load-*` pair's lesson, zh.toml).
            self.ls_of(machine, path).await?.0
        };
        let loaded = self.workspaces_loaded()?;
        let key = khor_sync::workspaces::key(&home.hex(), &real);
        if loaded.doc.state(&key) == khor_sync::workspaces::State::No {
            loaded.doc.set(&key, khor_sync::workspaces::State::Yes)?;
            let mut store = loaded.store;
            store.flush(&loaded.doc)?;
        }
        Ok(())
    }

    /// Takes a path off whichever machine holds it.
    ///
    /// **The recycle bin is only this machine's.** `trash` is asked for
    /// exactly where somebody could go and look in it — the machine
    /// khor's face is running on — and nowhere else: a file put in a
    /// far machine's bin is neither gone nor findable, which is the
    /// worst of both (`crate::proto::Request::Remove`). So this is the
    /// one verb whose meaning differs by machine, and the face says
    /// which one it is about to do before anybody presses.
    pub async fn remove_path(&self, machine: &str, path: &str) -> Result<(), String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::RemoveOn {
                machine: machine.to_owned(),
                path: path.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Removed => Ok(()),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.remove_with(&ep, machine, path).await;
        ep.close().await;
        out
    }

    async fn remove_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<(), String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let p = path.to_owned();
            // The one caller that asks for the bin.
            return tokio::task::spawn_blocking(move || crate::files::remove(&p, true))
                .await
                .map_err(|e| e.to_string())?;
        }
        // Resolve proved the name microseconds ago, but two reads of
        // one table can straddle a removal.
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        // **A satellite routes to its sponsor**, the road `Ls` and
        // `FetchPath` already take: nothing can dial a satellite, so
        // asking it directly is a frame that goes nowhere.
        if let Some(sp) = target.sponsor.clone() {
            let sat = target.id.clone();
            if sp == self.device_str() {
                let host = target
                    .ssh_host
                    .clone()
                    .ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
                let path = path.to_owned();
                return tokio::task::spawn_blocking(move || crate::satellite::remove(&host, &path))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            let sponsor = self
                .devices_loaded()?
                .doc
                .get(&sp)
                .ok_or_else(|| msg::machine_left_table(&channel))?;
            let req = Request::SatRemove {
                sat,
                path: path.to_owned(),
            };
            return match self.ask(ep, &sponsor.name, &req).await? {
                Response::Removed => Ok(()),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        match self
            .ask(
                ep,
                &channel,
                &Request::Remove {
                    path: path.to_owned(),
                },
            )
            .await?
        {
            Response::Removed => Ok(()),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// The ssh road to a satellite this machine sponsors, by the name
    /// the tables call it. **Asked again rather than carried down from
    /// [`Self::sat_and_sponsor`]**: that one answers whose satellite it
    /// is, this one answers how to reach it, and a row can lose its
    /// address without leaving the table.
    fn sponsored_host(&self, channel: &str) -> Result<String, String> {
        self.devices_loaded()?
            .doc
            .by_name(channel)
            .and_then(|d| d.ssh_host.clone())
            .ok_or_else(|| msg::sat_no_ssh_host(channel))
    }

    /// Makes one directory on whichever machine holds the parent
    /// (工作区第五轮). One directory, never a chain; a name already
    /// taken is refused (`crate::files::mkdir`).
    pub async fn mkdir(&self, machine: &str, path: &str) -> Result<(), String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::MkdirOn {
                machine: machine.to_owned(),
                path: path.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Made => Ok(()),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.mkdir_with(&ep, machine, path).await;
        ep.close().await;
        out
    }

    async fn mkdir_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<(), String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let p = path.to_owned();
            return tokio::task::spawn_blocking(move || crate::files::mkdir(&p))
                .await
                .map_err(|e| e.to_string())?;
        }
        // A satellite routes to its sponsor, `remove_with`'s road.
        if let Some((sat, sponsor)) = self.sat_and_sponsor(&channel)? {
            let Some(sponsor) = sponsor else {
                let host = self.sponsored_host(&channel)?;
                let p = path.to_owned();
                return tokio::task::spawn_blocking(move || crate::satellite::mkdir(&host, &p))
                    .await
                    .map_err(|e| e.to_string())?;
            };
            let req = Request::SatMkdir {
                sat,
                path: path.to_owned(),
            };
            return match self.ask(ep, &sponsor, &req).await? {
                Response::Made => Ok(()),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        match self
            .ask(
                ep,
                &channel,
                &Request::Mkdir {
                    path: path.to_owned(),
                },
            )
            .await?
        {
            Response::Made => Ok(()),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Makes an empty file on whichever machine holds the directory.
    ///
    /// **A write of no bytes claiming there is nothing there** — that is
    /// the whole of it on every road but one, and it is why a khor built
    /// before this verb existed already answers it correctly: an empty
    /// `was` is [`crate::files::write_whole`]'s 「there should be
    /// nothing here」, and a name already taken fails that claim. So
    /// there is no `Request::Touch` on the wire at all.
    ///
    /// The one road that is not a write is a satellite's
    /// ([`crate::proto::Request::SatTouch`] says why).
    pub async fn touch(&self, machine: &str, path: &str) -> Result<(), String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::TouchOn {
                machine: machine.to_owned(),
                path: path.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Made => Ok(()),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.touch_with(&ep, machine, path).await;
        ep.close().await;
        out
    }

    async fn touch_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<(), String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let p = path.to_owned();
            return tokio::task::spawn_blocking(move || crate::files::touch(&p))
                .await
                .map_err(|e| e.to_string())?;
        }
        if let Some((sat, sponsor)) = self.sat_and_sponsor(&channel)? {
            let Some(sponsor) = sponsor else {
                let host = self.sponsored_host(&channel)?;
                let p = path.to_owned();
                return tokio::task::spawn_blocking(move || crate::satellite::touch(&host, &p))
                    .await
                    .map_err(|e| e.to_string())?;
            };
            let req = Request::SatTouch {
                sat,
                path: path.to_owned(),
            };
            return match self.ask(ep, &sponsor, &req).await? {
                Response::Made => Ok(()),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        // Every ordinary machine: the write it has always answered.
        self.write_path_with(ep, &channel, path, "", b"")
            .await
            .map(|_| ())
    }

    /// Moves a name on whichever machine holds it — inside a directory
    /// or across two of them, which is one verb and is why it is spelled
    /// `mv`. The target is never overwritten
    /// (`crate::files::rename_path`).
    pub async fn rename_path(&self, machine: &str, from: &str, to: &str) -> Result<(), String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::RenameOn {
                machine: machine.to_owned(),
                from: from.to_owned(),
                to: to.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Renamed => Ok(()),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.rename_with(&ep, machine, from, to).await;
        ep.close().await;
        out
    }

    async fn rename_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        from: &str,
        to: &str,
    ) -> Result<(), String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let (a, b) = (from.to_owned(), to.to_owned());
            return tokio::task::spawn_blocking(move || crate::files::rename_path(&a, &b))
                .await
                .map_err(|e| e.to_string())?;
        }
        if let Some((sat, sponsor)) = self.sat_and_sponsor(&channel)? {
            let Some(sponsor) = sponsor else {
                let host = self.sponsored_host(&channel)?;
                let (a, b) = (from.to_owned(), to.to_owned());
                return tokio::task::spawn_blocking(move || {
                    crate::satellite::rename_path(&host, &a, &b)
                })
                .await
                .map_err(|e| e.to_string())?;
            };
            let req = Request::SatRename {
                sat,
                from: from.to_owned(),
                to: to.to_owned(),
            };
            return match self.ask(ep, &sponsor, &req).await? {
                Response::Renamed => Ok(()),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        let req = Request::RenamePath {
            from: from.to_owned(),
            to: to.to_owned(),
        };
        match self.ask(ep, &channel, &req).await? {
            Response::Renamed => Ok(()),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Opens a path or a page **on the machine that holds it**
    /// (工作区第五轮；the user: 都在宿主机器上打开).
    ///
    /// **Not `khor_gui_core::web::open_link`**, which opens on the
    /// machine the face is running on. That one is right for a link a
    /// person wants to read; this one is right for the file a session
    /// just wrote, which exists on the machine the session ran on and
    /// very often nowhere else. The face can be a browser on a phone,
    /// and a phone opening `/Users/…/main.rs` opens nothing.
    ///
    /// **`khor open` was not reused**: that verb starts a session on a
    /// machine, and two meanings on one word is how a person ends up
    /// starting an agent when they meant to look at a picture.
    pub async fn launch(&self, machine: &str, target: &str) -> Result<(), String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::LaunchOn {
                machine: machine.to_owned(),
                target: target.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Launched => Ok(()),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.launch_with(&ep, machine, target).await;
        ep.close().await;
        out
    }

    async fn launch_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        target: &str,
    ) -> Result<(), String> {
        // Refused here, before the road is chosen: the satellite road
        // hands the target to an opener khor never sees again.
        crate::files::openable(target)?;
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let t = target.to_owned();
            return tokio::task::spawn_blocking(move || crate::files::launch(&t))
                .await
                .map_err(|e| e.to_string())?;
        }
        if let Some((sat, sponsor)) = self.sat_and_sponsor(&channel)? {
            let Some(sponsor) = sponsor else {
                let host = self.sponsored_host(&channel)?;
                let t = target.to_owned();
                return tokio::task::spawn_blocking(move || crate::satellite::launch(&host, &t))
                    .await
                    .map_err(|e| e.to_string())?;
            };
            let req = Request::SatLaunch {
                sat,
                target: target.to_owned(),
            };
            return match self.ask(ep, &sponsor, &req).await? {
                Response::Launched => Ok(()),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        match self
            .ask(
                ep,
                &channel,
                &Request::Launch {
                    target: target.to_owned(),
                },
            )
            .await?
        {
            Response::Launched => Ok(()),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Everything under one directory on any machine that holds the
    /// words — in a name, or in a line (工作区第六轮 第 47 条).
    ///
    /// **The walk happens where the files are**, on every road: this
    /// machine walks its own disk, a peer walks its own, and a
    /// satellite's sponsor greps over ssh. Nothing about a project
    /// crosses the wire but the answer.
    pub async fn search(
        &self,
        machine: &str,
        dir: &str,
        query: &str,
    ) -> Result<crate::files::Found, String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::SearchOn {
                machine: machine.to_owned(),
                dir: dir.to_owned(),
                query: query.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Found { found } => Ok(found),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.search_with(&ep, machine, dir, query).await;
        ep.close().await;
        out
    }

    async fn search_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        dir: &str,
        query: &str,
    ) -> Result<crate::files::Found, String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let (d, q) = (dir.to_owned(), query.to_owned());
            return tokio::task::spawn_blocking(move || crate::files::search(&d, &q))
                .await
                .map_err(|e| e.to_string())?;
        }
        if let Some((sat, sponsor)) = self.sat_and_sponsor(&channel)? {
            let Some(sponsor) = sponsor else {
                let host = self.sponsored_host(&channel)?;
                let (d, q) = (dir.to_owned(), query.to_owned());
                return tokio::task::spawn_blocking(move || {
                    crate::satellite::search(&host, &d, &q)
                })
                .await
                .map_err(|e| e.to_string())?;
            };
            let req = Request::SatSearch {
                sat,
                dir: dir.to_owned(),
                query: query.to_owned(),
            };
            return match self.ask(ep, &sponsor, &req).await? {
                Response::Found { found } => Ok(found),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        let req = Request::Search {
            dir: dir.to_owned(),
            query: query.to_owned(),
        };
        match self.ask(ep, &channel, &req).await? {
            Response::Found { found } => Ok(found),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// When a directory on any machine was last changed — the newest
    /// file mtime under it (`crate::workspace::touched`).
    ///
    /// **One question per row, asked of the machine holding the
    /// files.** Nothing here is cached and nothing is on a beat: the
    /// landing asks when it draws and again when somebody comes back to
    /// the window, which is `WorkspacePane`'s rule for the git marks
    /// and for the same reason.
    pub async fn touched(&self, machine: &str, path: &str) -> Result<i64, String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::TouchedOn {
                machine: machine.to_owned(),
                path: path.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Touched { at_ms } => Ok(at_ms),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.touched_with(&ep, machine, path).await;
        ep.close().await;
        out
    }

    async fn touched_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<i64, String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let dir = std::path::PathBuf::from(path);
            return tokio::task::spawn_blocking(move || crate::workspace::touched(&dir))
                .await
                .map_err(|e| e.to_string());
        }
        // Resolve proved the name microseconds ago, but two reads of
        // one table can straddle a removal.
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        // **A satellite routes to its sponsor**, the road `Ls` and
        // `FetchPath` already take: nothing can dial a satellite — its
        // row carries no addresses on purpose — so asking it directly
        // is a frame that goes nowhere and comes back as「联系不上」
        // about a machine ssh can reach. On the sponsor itself there is
        // no wire to use at all.
        if let Some(sp) = target.sponsor.clone() {
            let sat = target.id.clone();
            if sp == self.device_str() {
                let host = target
                    .ssh_host
                    .clone()
                    .ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
                let path = path.to_owned();
                return tokio::task::spawn_blocking(move || {
                    crate::satellite::touched(&host, &path)
                })
                .await
                .map_err(|e| e.to_string())?;
            }
            let sponsor = self
                .devices_loaded()?
                .doc
                .get(&sp)
                .ok_or_else(|| msg::machine_left_table(&channel))?;
            let req = Request::SatTouched {
                sat,
                path: path.to_owned(),
            };
            return match self.ask(ep, &sponsor.name, &req).await? {
                Response::Touched { at_ms } => Ok(at_ms),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        match self
            .ask(
                ep,
                &channel,
                &Request::Touched {
                    path: path.to_owned(),
                },
            )
            .await?
        {
            Response::Touched { at_ms } => Ok(at_ms),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// The **first slice** of a file on any machine, for looking at it
    /// rather than taking it: `(total, at_ms, bytes)`.
    ///
    /// **One slice, never a loop.** `pull_path` loops because it is
    /// spending this machine's disk on a whole file; this is spending a
    /// screen on as much as fits, and one [`proto::SLICE`] is 256 KB.
    /// Measured on this repo's neighbours (1525 tracked files): 98% are
    /// under that, and of the 31 that are not, 27 are png/pdf/icns/mp4
    /// — files no code view was ever going to paint. Four text files in
    /// 1525 are cut, and `total` is what says so.
    ///
    /// So the caller never has to guess: `bytes.len() < total` means
    /// there is more, and what it does about that is a question about a
    /// screen, which is why the limit is not decided here.
    ///
    /// A satellite's file does not come through this road — the sponsor
    /// serves those off a staging copy with `SatFetchPath`, which is
    /// the account already in docs/handoff/001.md.
    pub async fn read_head(
        &self,
        machine: &str,
        path: &str,
    ) -> Result<(u64, u64, Vec<u8>), String> {
        if machine.is_empty() || machine == self.name() {
            return crate::files::read_slice(path, 0);
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::ReadHead {
                machine: machine.to_owned(),
                path: path.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Head {
                    total,
                    at_ms,
                    bytes,
                } => Ok((total, at_ms, bytes.into_vec())),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.read_head_with(&ep, machine, path).await;
        ep.close().await;
        out
    }

    /// **The first slice is `read_at_with`'s first slice** (工作区批7).
    /// This used to dial the named machine itself, which for a
    /// satellite is a row carrying no addresses on purpose — so
    /// `khor read` and `khor put` on a satellite's file answered
    /// 「这不是一个机器 id」 about a file ssh reads fine, while the same
    /// file opened in the app. One routing, not two.
    async fn read_head_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<(u64, u64, Vec<u8>), String> {
        self.read_at_with(ep, machine, path, 0).await
    }

    /// What differs from HEAD in one checkout on any machine
    /// (`crate::git::status`). `root` is that machine's spelling of the
    /// directory, which is what the workspace list handed over.
    ///
    /// **A refusal is an error here, never an empty list.** An empty
    /// `rows` means「nothing differs from HEAD」— a clean tree, safe to
    /// edit — and that is exactly the wrong thing to paint when khor
    /// could not read the repository at all.
    pub async fn git_status_on(
        &self,
        machine: &str,
        root: &str,
    ) -> Result<crate::git::Status, String> {
        if machine.is_empty() || machine == self.name() {
            return crate::git::status(std::path::Path::new(root));
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::GitStatusOn {
                machine: machine.to_owned(),
                root: root.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::GitStatus {
                    branch,
                    rows,
                    more,
                    ahead,
                    behind,
                    fetched_ms,
                } => Ok(crate::git::Status {
                    branch,
                    rows,
                    more,
                    ahead,
                    behind,
                    fetched_ms,
                }),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.git_status_with(&ep, machine, root).await;
        ep.close().await;
        out
    }

    async fn git_status_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        root: &str,
    ) -> Result<crate::git::Status, String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            return crate::git::status(std::path::Path::new(root));
        }
        // A satellite's repository is read by its sponsor over ssh
        // (卫星工作区批): nothing can dial the satellite itself, and
        // the ask below would spend thirteen seconds finding that out
        // before painting 联系不上 over a checkout ssh can read.
        if let Some((sat, sponsor)) = self.sat_and_sponsor(&channel)? {
            if sponsor.is_none() {
                let host = self.my_satellite_host(&sat)?;
                let (h, r) = (host, root.to_owned());
                return tokio::task::spawn_blocking(move || crate::satellite::git_status(&h, &r))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            let sponsor = sponsor.expect("checked");
            let req = Request::SatGitStatus {
                sat,
                root: root.to_owned(),
            };
            return match self.ask(ep, &sponsor, &req).await? {
                Response::GitStatus {
                    branch,
                    rows,
                    more,
                    ahead,
                    behind,
                    fetched_ms,
                } => Ok(crate::git::Status {
                    branch,
                    rows,
                    more,
                    ahead,
                    behind,
                    fetched_ms,
                }),
                Response::Refused { why } if why.trim().is_empty() => {
                    Err(msg::git_predates().into())
                }
                Response::Refused { why } => Err(msg::git_refused(why)),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        let req = Request::GitStatus {
            root: root.to_owned(),
        };
        match self.ask(ep, &channel, &req).await? {
            Response::GitStatus {
                branch,
                rows,
                more,
                ahead,
                behind,
                fetched_ms,
            } => Ok(crate::git::Status {
                branch,
                rows,
                more,
                ahead,
                behind,
                fetched_ms,
            }),
            // Wordless means the op is unknown over there; words mean
            // that machine answered and its answer is the one to show
            // (the `load-*` pair's measured lesson, zh.toml).
            Response::Refused { why } if why.trim().is_empty() => Err(msg::git_predates().into()),
            Response::Refused { why } => Err(msg::git_refused(why)),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Where one file differs from HEAD on any machine
    /// (`crate::git::diff`). Refuses out loud for [`Node::git_status_on`]'s
    /// reason: no hunks means the file matches HEAD.
    pub async fn git_diff_on(&self, machine: &str, path: &str) -> Result<crate::git::Diff, String> {
        if machine.is_empty() || machine == self.name() {
            return crate::git::diff(std::path::Path::new(path));
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::GitDiffOn {
                machine: machine.to_owned(),
                path: path.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::GitDiff { hunks, more } => Ok(crate::git::Diff { hunks, more }),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.git_diff_with(&ep, machine, path).await;
        ep.close().await;
        out
    }

    async fn git_diff_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<crate::git::Diff, String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            return crate::git::diff(std::path::Path::new(path));
        }
        if let Some((sat, sponsor)) = self.sat_and_sponsor(&channel)? {
            let Some(sponsor) = sponsor else {
                let host = self.my_satellite_host(&sat)?;
                let (h, p) = (host, path.to_owned());
                return tokio::task::spawn_blocking(move || crate::satellite::git_diff(&h, &p))
                    .await
                    .map_err(|e| e.to_string())?;
            };
            let req = Request::SatGitDiff {
                sat,
                path: path.to_owned(),
            };
            return match self.ask(ep, &sponsor, &req).await? {
                Response::GitDiff { hunks, more } => Ok(crate::git::Diff { hunks, more }),
                Response::Refused { why } if why.trim().is_empty() => {
                    Err(msg::git_predates().into())
                }
                Response::Refused { why } => Err(msg::git_refused(why)),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        let req = Request::GitDiff {
            path: path.to_owned(),
        };
        match self.ask(ep, &channel, &req).await? {
            Response::GitDiff { hunks, more } => Ok(crate::git::Diff { hunks, more }),
            Response::Refused { why } if why.trim().is_empty() => Err(msg::git_predates().into()),
            Response::Refused { why } => Err(msg::git_refused(why)),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Saves a whole file onto any machine (`crate::files::write_whole`).
    ///
    /// **A refusal is an error and is never softened.** Every other
    /// asker in this file has to decide whether an empty answer is a
    /// fact or a failure; this one does not get to — a save that
    /// answered「fine」while writing nothing is the single worst thing
    /// this module could do, so every path out of here either wrote the
    /// file or says why it did not.
    pub async fn write_path(
        &self,
        machine: &str,
        path: &str,
        was: &str,
        bytes: &[u8],
    ) -> Result<(u64, String), String> {
        if machine.is_empty() || machine == self.name() {
            return crate::files::write_whole(path, bytes, was);
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::WritePathOn {
                machine: machine.to_owned(),
                path: path.to_owned(),
                was: was.to_owned(),
                bytes: serde_bytes::ByteBuf::from(bytes.to_vec()),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Wrote { at_ms, digest } => Ok((at_ms, digest)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.write_path_with(&ep, machine, path, was, bytes).await;
        ep.close().await;
        out
    }

    async fn write_path_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
        was: &str,
        bytes: &[u8],
    ) -> Result<(u64, String), String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            return crate::files::write_whole(path, bytes, was);
        }
        // **A satellite's file can be saved, and the promise is kept on
        // the far side** (工作区批7). This road was refused on the
        // grounds that a sponsor pulling, hashing here and pushing back
        // leaves a window the size of somebody else's save. That was
        // the wrong thing to look at: the check does not have to be
        // simultaneous with the read, it has to be pinned to content.
        // `sat_write` sends the far machine a sha256 taken off the very
        // bytes the person read, and that machine compares it to its
        // own file before writing — so a window is just a window, and
        // anything that changed in it fails the comparison.
        if let Some((sat, sponsor)) = self.sat_and_sponsor(&channel)? {
            let Some(sponsor) = sponsor else {
                let host = self
                    .devices_loaded()?
                    .doc
                    .by_name(&channel)
                    .and_then(|d| d.ssh_host.clone())
                    .ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
                return self.sat_write(&sat, &host, path, was, bytes).await;
            };
            let req = Request::SatWrite {
                sat,
                path: path.to_owned(),
                was: was.to_owned(),
                bytes: serde_bytes::ByteBuf::from(bytes.to_vec()),
            };
            return match self.ask(ep, &sponsor, &req).await? {
                Response::Wrote { at_ms, digest } => Ok((at_ms, digest)),
                Response::Refused { why } if why.trim().is_empty() => {
                    Err(msg::write_predates().into())
                }
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        let req = Request::WritePath {
            path: path.to_owned(),
            was: was.to_owned(),
            bytes: serde_bytes::ByteBuf::from(bytes.to_vec()),
        };
        match self.ask(ep, &channel, &req).await? {
            Response::Wrote { at_ms, digest } => Ok((at_ms, digest)),
            // Wordless means the op is unknown over there. **Said as a
            // version fact and never as「saved」**: the file view's next
            // move depends on it, and an old khor that cannot write is
            // not the same as a file it refused.
            Response::Refused { why } if why.trim().is_empty() => Err(msg::write_predates().into()),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Whether a machine has VS Code, which version, and whether
    /// `serve-web`'s server is already unpacked there
    /// (`crate::code::Editor`; an empty version means none).
    ///
    /// **The one asker in this file whose refusal is swallowed by its
    /// consumer**, and the reason is written at
    /// [`crate::proto::Request::CodeHere`]: a machine that cannot
    /// answer and a machine with no editor are two facts with one next
    /// move, which is to draw no button. The error still comes back
    /// here — swallowing it is the face's decision, made where the
    /// button would have been.
    pub async fn code_here(&self, machine: &str) -> Result<crate::code::Editor, String> {
        if machine.is_empty() || machine == self.name() {
            return Ok(crate::code::probe());
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::CodeHereOn {
                machine: machine.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::CodeHere { version, ready } => {
                    Ok(crate::code::Editor { version, ready })
                }
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.code_here_with(&ep, machine).await;
        ep.close().await;
        out
    }

    async fn code_here_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
    ) -> Result<crate::code::Editor, String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            return Ok(crate::code::probe());
        }
        // **A satellite answers「no editor」without being asked**, and
        // the reason is what a satellite is, not what ssh can do
        // (docs/NET.md「它身上不留东西」). `ssh -L` is a tunnel and
        // `code serve-web` is a command, so this is buildable; what it
        // would cost is ~640 MB of Microsoft's server unpacked into
        // that machine's home and a long-lived process left running
        // there — the first thing khor would ever leave behind on a
        // machine somebody deliberately did not install it on. The
        // earlier reason given here (「needs a khor over there to hold
        // the tunnel」) was wrong and is corrected rather than kept.
        //
        // Asked the ordinary way it would also dial a machine with no
        // addresses and spend thirteen seconds finding out, once per
        // workspace opened, to arrive at this same answer.
        if self.sat_and_sponsor(&channel)?.is_some() {
            return Ok(crate::code::Editor::default());
        }
        match self.ask(ep, &channel, &Request::CodeHere).await? {
            Response::CodeHere { version, ready } => Ok(crate::code::Editor { version, ready }),
            // **No version arm.** An older khor refuses wordlessly and
            // an editor-less machine answers an empty string, and the
            // consumer does the same thing with both — so inventing a
            // sentence here would be inventing a difference.
            Response::Refused { .. } => Ok(crate::code::Editor::default()),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Opens this machine's or another's VS Code web face and hands
    /// back an address **on this machine** plus the token it wants.
    ///
    /// **The pipe is this side's, not a field in the answer.** The far
    /// machine can only say which of its own loopback ports the editor
    /// is on; a browser here cannot point at that, so this stands a
    /// local listener in front of a tunnel the same way
    /// [`Node::reach`] does for a terminal (docs/NET.md 出口是一层 —
    /// one tunnel, many consumers).
    ///
    /// **Needs a resident serve, even for this machine's own editor**,
    /// and that is the difference from every other verb here. The
    /// server is a child process that has to outlive the call: a
    /// one-shot `khor code` that started it would hand back a link and
    /// then exit, and on the next invocation nothing would remember the
    /// server — measured on 2026-09-12, two orphaned server trees and a
    /// third started for the same folder.
    pub async fn code_open(&self, machine: &str) -> Result<(String, String), String> {
        match self
            .via_serve(ipc::Op::CodeOpenOn {
                machine: machine.to_owned(),
                root: String::new(),
            })
            .await
        {
            Some(reply) => match reply? {
                ipc::Reply::CodeAt { addr, token } => Ok((addr, token)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            },
            None => Err(msg::reach_needs_serve().into()),
        }
    }

    async fn code_open_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
    ) -> Result<(std::net::SocketAddr, String), String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            // This machine's own editor needs no pipe at all — the
            // browser and the server are already on one loopback.
            let (port, token) = crate::code::serve()?;
            return Ok((([127, 0, 0, 1], port).into(), token));
        }
        let (port, token) = match self
            .ask(
                ep,
                &channel,
                &Request::CodeServe {
                    root: String::new(),
                },
            )
            .await?
        {
            Response::CodeServing { port, token } => (port, token),
            Response::Refused { why } if why.trim().is_empty() => {
                return Err(msg::code_predates().into());
            }
            Response::Refused { why } => return Err(why),
            other => return Err(msg::peer_non_answer(format_args!("{other:?}"))),
        };
        let key = machine.to_owned();
        {
            let mut held = self.codes.lock().await;
            match held.get(&key) {
                // **The far port is checked, like `reaches` checks
                // its own.** `code::serve` above either handed back the
                // server that was already running or started a new one
                // on a new port; a pipe aimed at the old number would
                // pipe into nothing and read as a dead editor.
                Some((addr, far, handle)) if !handle.is_finished() && *far == port => {
                    return Ok((*addr, token));
                }
                Some((_, _, handle)) => {
                    handle.abort();
                    held.remove(&key);
                }
                None => {}
            }
        }
        let borrow = self.tunnel_on(ep, machine).await?;
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| e.to_string())?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        let target = format!("127.0.0.1:{port}");
        let handle = tokio::spawn(async move {
            let _ = crate::tunnel::serve_fixed(std::sync::Arc::new(borrow), listener, target).await;
        });
        self.codes.lock().await.insert(key, (addr, port, handle));
        Ok((addr, token))
    }

    /// Stops a machine's web face and drops this side's pipe. Answers
    /// the same whether either existed.
    pub async fn code_stop(&self, machine: &str) -> Result<(), String> {
        match self
            .via_serve(ipc::Op::CodeStopOn {
                machine: machine.to_owned(),
                root: String::new(),
            })
            .await
        {
            Some(reply) => match reply? {
                ipc::Reply::Accepted { .. } => Ok(()),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            },
            None => Err(msg::reach_needs_serve().into()),
        }
    }

    async fn code_stop_with(&self, ep: &iroh::Endpoint, machine: &str) -> Result<(), String> {
        // This side first, and unconditionally: the pipe is this
        // machine's and giving it back must not depend on the far one
        // being reachable.
        if let Some((_, _, handle)) = self.codes.lock().await.remove(machine) {
            handle.abort();
        }
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            return crate::code::stop();
        }
        match self
            .ask(
                ep,
                &channel,
                &Request::CodeStop {
                    root: String::new(),
                },
            )
            .await?
        {
            Response::Acted { .. } => Ok(()),
            // Nothing to stop over there reads as done, for the same
            // reason `DropBorrow` does: the caller wanted it gone.
            Response::Refused { .. } => Ok(()),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// One slice of a file at `offset`, off any machine. The general
    /// case [`Node::read_head`] is the zero-offset instance of.
    pub async fn read_at(
        &self,
        machine: &str,
        path: &str,
        offset: u64,
    ) -> Result<(u64, u64, Vec<u8>), String> {
        if machine.is_empty() || machine == self.name() {
            return crate::files::read_slice(path, offset);
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::ReadAt {
                machine: machine.to_owned(),
                path: path.to_owned(),
                offset,
            })
            .await
        {
            return match reply? {
                ipc::Reply::Head {
                    total,
                    at_ms,
                    bytes,
                } => Ok((total, at_ms, bytes.into_vec())),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.read_at_with(&ep, machine, path, offset).await;
        ep.close().await;
        out
    }

    async fn read_at_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
        offset: u64,
    ) -> Result<(u64, u64, Vec<u8>), String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            return crate::files::read_slice(path, offset);
        }
        // A satellite's file comes off its sponsor's staged copy, the
        // road `pull_path` already takes (卫星工作区批). Without this
        // the file view dialled a machine with no addresses and painted
        // 联系不上 over a file ssh could read — so a satellite's
        // workspace opened, its tree walked, and every file in it was
        // unreadable.
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        if let Some(sp) = target.sponsor.clone() {
            let sat = target.id.clone();
            if sp == self.device_str() {
                let host = target
                    .ssh_host
                    .clone()
                    .ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
                return self.sat_slice(&sat, &host, path, offset).await;
            }
            let sponsor = self
                .devices_loaded()?
                .doc
                .get(&sp)
                .ok_or_else(|| msg::machine_left_table(&channel))?;
            let ask = Request::SatFetchPath {
                sat,
                path: path.to_owned(),
                offset,
            };
            return match self.ask(ep, &sponsor.name, &ask).await? {
                Response::PathSlice {
                    total,
                    at_ms,
                    bytes,
                } => Ok((total, at_ms, bytes.into_vec())),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        match self
            .ask(
                ep,
                &channel,
                &Request::FetchPath {
                    path: path.to_owned(),
                    offset,
                },
            )
            .await?
        {
            Response::PathSlice {
                total,
                at_ms,
                bytes,
            } => Ok((total, at_ms, bytes.into_vec())),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// A whole file, up to `cap` bytes, for looking at rather than
    /// taking (`Node::pull_path` is the one that spends a disk).
    ///
    /// Answers `(total, at_ms, bytes, cut)` — `cut` meaning the file is
    /// bigger than `cap` and what came back is its beginning.
    ///
    /// **Why a loop here and not a bigger frame.** One slice is
    /// [`crate::proto::SLICE`] and one frame is `MAX_FRAME`; a 4 MB
    /// image in one answer would be eight frames' worth. So the asker
    /// loops, and every reply on that road stays the size every shipped
    /// khor already decodes.
    ///
    /// **The change contract is honoured here, and `pull_path` does not
    /// honour it.** `PathSlice` carries `total` and `at_ms` on every
    /// slice precisely so a reader can notice the file moving under it
    /// — no digest guards this road. A file rewritten between slice 3
    /// and slice 4 would otherwise be assembled out of two different
    /// files and painted as one, which for an image is a picture that
    /// never existed. That `pull_path` skips this check is an older
    /// gap, on the ledger rather than copied.
    pub async fn read_all(
        &self,
        machine: &str,
        path: &str,
        cap: u64,
    ) -> Result<(u64, u64, Vec<u8>, bool), String> {
        let (total, at_ms, first) = self.read_at(machine, path, 0).await?;
        let want = total.min(cap);
        let mut bytes = first;
        if bytes.len() as u64 >= want {
            bytes.truncate(want as usize);
            return Ok((total, at_ms, bytes, total > cap));
        }
        while (bytes.len() as u64) < want {
            let (now_total, now_ms, more) = self.read_at(machine, path, bytes.len() as u64).await?;
            if now_total != total || now_ms != at_ms {
                return Err(msg::file_moved_while_reading(path));
            }
            if more.is_empty() {
                // The far side answered nothing while claiming there is
                // more. Refused rather than looped: a loop that cannot
                // make progress is the one failure a reader must not
                // turn into a hang.
                return Err(msg::file_moved_while_reading(path));
            }
            bytes.extend_from_slice(&more);
        }
        bytes.truncate(want as usize);
        Ok((total, at_ms, bytes, total > cap))
    }

    async fn places_on_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
    ) -> Result<Vec<(String, String, u32, i64)>, String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            return Ok(self.places_shown());
        }
        match self.ask(ep, &channel, &Request::Places).await? {
            Response::Places { places } => Ok(places),
            // A khor over there that predates the op: no list, not a
            // fault — the wizard says nothing where it has nothing.
            //
            // **This is the same swallow `load_on_with` was fixed for,
            // and it is still here on purpose** (2026-09-04). There the
            // `why` was dropped and every refusal became one sentence
            // about the far side's version; the measured cost was a
            // machine that refused *one unreadable word* being reported
            // as unable to do the whole op. Here the swallow is wider:
            // a refusal for any reason at all becomes 「没有常去的地
            // 方」 — a refusal translated into an absence, which is the
            // reading nobody can tell from the truth.
            //
            // Left alone because unswallowing it changes what the
            // wizard shows, and that is a product decision rather than
            // a bug fix. **If you are here because a machine's 常去的
            // 地方 came back empty and you cannot see why: this line is
            // why. It is not evidence the list is empty.**
            Response::Refused { .. } => Ok(Vec::new()),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Where to speak to a session's terminal, wherever it lives: a
    /// local address and the cookie behind it. **Needs the resident
    /// serve** for the same reason a borrow does — the pipe has to
    /// outlive the verb that asked for it, and only the resident process
    /// is still there afterwards. A session on this machine answers with
    /// its own host's port, and then there is no pipe at all.
    pub async fn reach(
        &self,
        machine: &str,
        session: &crate::SessionId,
    ) -> Result<(String, String), String> {
        match self
            .via_serve(ipc::Op::ReachOn {
                machine: machine.to_owned(),
                session: session.0.clone(),
            })
            .await
        {
            Some(reply) => match reply? {
                ipc::Reply::Reaching { addr, cookie } => Ok((addr, cookie)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            },
            None => Err(msg::reach_needs_serve().into()),
        }
    }

    /// Ends a session that lives on another machine (`Act` "close").
    /// The row's own machine runs it — the same rule every Act follows,
    /// so nothing bounces and the far side's refusals stay its own.
    pub async fn close_on(&self, machine: &str, session: &crate::SessionId) -> Result<(), String> {
        let op = ipc::Op::CloseOn {
            machine: machine.to_owned(),
            session: session.0.clone(),
        };
        if let Some(reply) = self.via_serve(op).await {
            return match reply? {
                ipc::Reply::Accepted { .. } => Ok(()),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self.close_on_with(&ep, machine, session).await;
        ep.close().await;
        outcome
    }

    async fn close_on_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        session: &crate::SessionId,
    ) -> Result<(), String> {
        let (channel, _) = self.resolve(machine)?;
        let req = Request::Act {
            session: session.0.clone(),
            action: "close".into(),
        };
        match self.ask(ep, &channel, &req).await? {
            Response::Acted { .. } => Ok(()),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// A machine's directory listing, for the files landing. Routed the
    /// way every one-shot verb is: through the resident serve when one
    /// holds this key, on a bound-and-closed endpoint otherwise.
    pub async fn ls_of(
        &self,
        machine: &str,
        path: &str,
    ) -> Result<(String, Vec<proto::DirEntry>, bool), String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::Ls {
                machine: machine.to_owned(),
                path: path.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Dir {
                    path,
                    entries,
                    truncated,
                } => Ok((path, entries, truncated)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self.ls_with(&ep, machine, path).await;
        ep.close().await;
        outcome
    }

    /// The listing on an endpoint the caller owns. The asked machine may
    /// be this one — the files landing lists every machine, this one
    /// included — and then the answer never touches the wire.
    async fn ls_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<(String, Vec<proto::DirEntry>, bool), String> {
        // `""` is this machine, the spelling every other verb takes
        // (`resolve_or_here`); through `resolve` alone it was refused as
        // 「这个机器名进不了路径」 — the directory picker's 本机 row
        // asked exactly that and drew nothing (2026-09-15).
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let path = path.to_owned();
            let (at, entries, truncated) =
                tokio::task::spawn_blocking(move || crate::files::list_dir(&path))
                    .await
                    .map_err(|e| e.to_string())??;
            return Ok((at.display().to_string(), entries, truncated));
        }
        // Resolve proved the name microseconds ago, but two reads of
        // one table can straddle a removal.
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        // A satellite's home is its sponsor (附属机器批): the listing
        // runs there, over ssh — the same "ask home" this function
        // already is, with one more hop written in the table. On the
        // sponsor itself no wire is needed at all.
        if let Some(sp) = target.sponsor.clone() {
            if sp == self.device_str() {
                let host = target
                    .ssh_host
                    .clone()
                    .ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
                let path = path.to_owned();
                return tokio::task::spawn_blocking(move || crate::satellite::ls(&host, &path))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            let sponsor = self
                .devices_loaded()?
                .doc
                .get(&sp)
                .ok_or_else(|| msg::machine_left_table(&channel))?;
            let conn = self
                .dial(ep, &sponsor.id, &sponsor.addrs, ALPN)
                .await
                .map_err(|e| match e {
                    DialFailure::TimedOut => msg::machine_unreachable_timeout(&sponsor.name),
                    DialFailure::Refused(why) => msg::cant_reach_named(&sponsor.name, why),
                })?;
            return match request(
                &conn,
                &Request::SatLs {
                    sat: target.id.clone(),
                    path: path.to_owned(),
                },
            )
            .await?
            {
                Response::Dir {
                    path,
                    entries,
                    truncated,
                } => Ok((path, entries, truncated)),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        let conn = self
            .dial(ep, &target.id, &target.addrs, ALPN)
            .await
            .map_err(|e| match e {
                DialFailure::TimedOut => msg::machine_unreachable_timeout(&channel),
                DialFailure::Refused(why) => msg::cant_reach_named(&channel, why),
            })?;
        match request(
            &conn,
            &Request::Ls {
                path: path.to_owned(),
            },
        )
        .await?
        {
            Response::Dir {
                path,
                entries,
                truncated,
            } => Ok((path, entries, truncated)),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Takes one file off a machine by path, into `dir` — the
    /// browse-then-take half of the files landing. Whole-file only (the
    /// rsync-shaped delta is on the ledger, waiting for the repeat-pull
    /// case to exist). Returns bytes moved and where the file landed.
    pub async fn pull_path(
        &self,
        machine: &str,
        path: &str,
        dir: &std::path::Path,
    ) -> Result<(u64, std::path::PathBuf), String> {
        // A relative dir crosses the ipc boundary as a bare string, and
        // the resident serve would resolve it in *its* cwd — the caller
        // would then read "落在 ./x" about a file that landed somewhere
        // else entirely (2026-08-19, the netprobe pull). The caller's
        // "." becomes absolute before it leaves this process.
        let dir = std::path::absolute(dir).map_err(|e| e.to_string())?;
        let dir = dir.as_path();
        if let Some(reply) = self
            .via_serve(ipc::Op::Pull {
                machine: machine.to_owned(),
                path: path.to_owned(),
                dir: dir.display().to_string(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Pulled { moved, dest } => Ok((moved, std::path::PathBuf::from(dest))),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self.pull_path_with(&ep, machine, path, dir).await;
        ep.close().await;
        outcome
    }

    /// The pull on an endpoint the caller owns. Asking this very
    /// machine is a plain copy through the same refusals — the files
    /// landing lists this machine too, and "download from myself" must
    /// not need a wire to work.
    async fn pull_path_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
        dir: &std::path::Path,
    ) -> Result<(u64, std::path::PathBuf), String> {
        use std::io::Write;
        let (channel, home) = self.resolve(machine)?;
        let fell = crate::files::landing(path, dir)?;
        if home == self.device() {
            let (path, part) = (path.to_owned(), fell.part.clone());
            let moved = tokio::task::spawn_blocking(move || -> Result<u64, String> {
                // Through read_slice rather than fs::copy so the local
                // pull refuses exactly what the remote one refuses
                // (relative paths, directories).
                let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
                let mut offset = 0u64;
                loop {
                    let (total, _, bytes) = crate::files::read_slice(&path, offset)?;
                    out.write_all(&bytes).map_err(|e| e.to_string())?;
                    offset += bytes.len() as u64;
                    if offset >= total {
                        return Ok(offset);
                    }
                }
            })
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
            return match moved {
                Ok(moved) => {
                    std::fs::rename(&fell.part, &fell.dest).map_err(|e| e.to_string())?;
                    Ok((moved, fell.dest))
                }
                Err(why) => {
                    let _ = std::fs::remove_file(&fell.part);
                    Err(why)
                }
            };
        }
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        // A satellite's pull routes to its sponsor (附属机器批): on the
        // sponsor itself one scp lands the file whole; from anywhere
        // else the slices come off the sponsor's staging copy
        // (`Request::SatFetchPath`).
        let (dial_to, sat) = match target.sponsor.clone() {
            Some(sp) if sp == self.device_str() => {
                let host = target
                    .ssh_host
                    .clone()
                    .ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
                let (h, p, part) = (host, path.to_owned(), fell.part.clone());
                let moved =
                    tokio::task::spawn_blocking(move || crate::satellite::pull_file(&h, &p, &part))
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r);
                return match moved {
                    Ok(moved) => {
                        std::fs::rename(&fell.part, &fell.dest).map_err(|e| e.to_string())?;
                        Ok((moved, fell.dest))
                    }
                    Err(why) => {
                        let _ = std::fs::remove_file(&fell.part);
                        Err(why)
                    }
                };
            }
            Some(sp) => {
                let sponsor = self
                    .devices_loaded()?
                    .doc
                    .get(&sp)
                    .ok_or_else(|| msg::machine_left_table(&channel))?;
                (sponsor, Some(target.id.clone()))
            }
            None => (target, None),
        };
        let conn = self
            .dial(ep, &dial_to.id, &dial_to.addrs, ALPN)
            .await
            .map_err(|e| match e {
                DialFailure::TimedOut => msg::machine_unreachable_timeout(&dial_to.name),
                DialFailure::Refused(why) => msg::cant_reach_named(&dial_to.name, why),
            })?;
        let mut out = std::fs::File::create(&fell.part).map_err(|e| e.to_string())?;
        // The change contract: the first slice's (total, mtime) must
        // hold for every slice after it — two readings that differ are
        // two files, and no digest guards this path (proto::PathSlice).
        let mut contract: Option<(u64, u64)> = None;
        let mut offset = 0u64;
        let outcome = loop {
            let req = match &sat {
                Some(s) => Request::SatFetchPath {
                    sat: s.clone(),
                    path: path.to_owned(),
                    offset,
                },
                None => Request::FetchPath {
                    path: path.to_owned(),
                    offset,
                },
            };
            let resp = request(&conn, &req).await;
            match resp {
                Ok(Response::PathSlice {
                    total,
                    at_ms,
                    bytes,
                }) => {
                    match contract {
                        None => contract = Some((total, at_ms)),
                        Some(c) if c != (total, at_ms) => {
                            break Err(msg::file_changed_mid_pull().into());
                        }
                        Some(_) => {}
                    }
                    if bytes.is_empty() && total > 0 {
                        break Err(msg::empty_slice().into());
                    }
                    if let Err(e) = out.write_all(&bytes) {
                        break Err(e.to_string());
                    }
                    offset += bytes.len() as u64;
                    if offset >= total {
                        break Ok(offset);
                    }
                }
                Ok(Response::Refused { why }) => break Err(why),
                Ok(other) => break Err(msg::peer_non_answer(format_args!("{other:?}"))),
                Err(e) => break Err(e),
            }
        };
        match outcome {
            Ok(moved) => {
                drop(out);
                std::fs::rename(&fell.part, &fell.dest).map_err(|e| e.to_string())?;
                Ok((moved, fell.dest))
            }
            Err(why) => {
                drop(out);
                let _ = std::fs::remove_file(&fell.part);
                Err(why)
            }
        }
    }

    /// The offer behind `transfer/<msg_id>`, driving the sync axis once
    /// when it is not here yet.
    ///
    /// The summary travels the CRDT (docs/NET.md 同步), so `khor send`
    /// over there and `khor accept` over here are a race: for a window
    /// the id is real and the summary has not landed. That window used
    /// to be refused with [`msg::no_such_transfer`] — the words for an
    /// id that does not exist — so the only way through was to guess
    /// which of the two had happened and try again ten seconds later
    /// (2026-08-26, twice, on the upgrade path that exists *because*
    /// ssh does not).
    ///
    /// **The wait is one sync round, not a number of seconds.** Nothing
    /// here is a threshold anybody picked: a complete round is exactly
    /// what it takes for whatever exists on a reachable machine to be
    /// here, and a second round only re-asks machines that just
    /// answered. The round's own budget is already written down —
    /// [`DIAL_TIMEOUT`] per device, and a device that reported no roads
    /// is not dialled at all.
    ///
    /// The round is walked here rather than by [`Self::sync_with_all`]
    /// for one reason: **it stops at the machine that had the offer.**
    /// The round is serial, so the case this exists for — the offerer
    /// is up, it wrote the summary a second ago — must not pay
    /// `DIAL_TIMEOUT` for every dark machine that happens to sort ahead
    /// of it. Only a miss walks the whole table, and a miss is
    /// precisely the case that needs the whole table walked (below).
    ///
    /// **The outcomes are what separate the two misses**, with evidence
    /// rather than wording: every peer answered and it is still absent —
    /// then it is absent. Somebody did not answer — then khor does not
    /// know, and painting 不知道 as 没有 is the thing this repo keeps
    /// re-learning not to do (docs/SESSION.md 离线不是第七个词).
    ///
    /// **Who sees the wait, and who does not.** The CLI says what it is
    /// about to wait for before it goes quiet (`cli::syncing-for-offer`,
    /// with the machine count and this budget in it). **The GUI shows
    /// nothing**: `khor_gui_core::accept_transfer` awaits this call and
    /// gets one answer at the end of it, and there is no channel out of
    /// here to say more — `NodeEvent` has no notice to carry, `watch()`
    /// has no subscriber outside this crate's own tests, and when a
    /// serve holds the key the wait runs in the serve process, whose
    /// stderr is nobody's screen. So a face that presses 收下 on an
    /// offer it has not replicated yet sits still for up to a round.
    /// Written down rather than left to be rediscovered as a hang: the
    /// remedy is a progress channel, which is a batch of its own and not
    /// this one.
    ///
    /// **What it deliberately does not do is check the id's shape.** A
    /// syntactic gate would be the cheapest of the three answers, but it
    /// would put the message-id format in a second place (`khor_sync`
    /// mints them) and would start refusing real ids the day that format
    /// moves — buying a fast refusal at the price of a wrong one.
    async fn transfer_after_sync(
        &self,
        ep: &iroh::Endpoint,
        id: &crate::SessionId,
        msg_id: &str,
    ) -> Result<(String, chat::Message), String> {
        if let Ok(found) = self.find_transfer(msg_id) {
            return Ok(found);
        }
        let mut outcomes: Vec<(String, Result<String, String>)> = Vec::new();
        {
            let _g = self.sync_gate.lock().await;
            for d in self.devices_loaded()?.doc.all() {
                if d.id == self.device_str() {
                    continue;
                }
                let verdict = self.sync_device(ep, &d).await;
                outcomes.push((d.name, verdict));
                if let Ok(found) = self.find_transfer(msg_id) {
                    return Ok(found);
                }
            }
        }
        // A mesh of one: there was no round to wait out, so nothing is
        // pending behind this and the plain refusal is the whole truth.
        if outcomes.is_empty() {
            return Err(msg::no_such_transfer(&id.0));
        }
        let dark: Vec<&str> = outcomes
            .iter()
            .filter(|(_, verdict)| verdict.is_err())
            .map(|(name, _)| name.as_str())
            .collect();
        if dark.is_empty() {
            return Err(msg::transfer_absent_after_sync(&id.0));
        }
        Err(msg::transfer_not_synced_yet(
            &id.0,
            dark.len(),
            dark.join("、"),
        ))
    }

    /// Accept on an endpoint the caller owns: pulls locally when this
    /// machine is the recipient, otherwise routes the action to the
    /// recipient's serve — 动作从哪台设备发都行 (docs/SESSION.md).
    async fn accept_with(
        &self,
        ep: &iroh::Endpoint,
        id: &crate::SessionId,
    ) -> Result<(u64, Vec<String>), String> {
        let Some(msg_id) = id.0.strip_prefix("transfer/") else {
            return Err(msg::no_such_transfer(&id.0));
        };
        // Which machine to route to is read off the offer, so the wait
        // has to happen before the fork, not inside one arm of it.
        let (channel, _) = self.transfer_after_sync(ep, id, msg_id).await?;
        if channel == self.name() {
            return self.accept_local(ep, id).await;
        }
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::recipient_not_in_table(&channel))?;
        let conn = self
            .dial(ep, &target.id, &target.addrs, ALPN)
            .await
            .map_err(|e| match e {
                DialFailure::TimedOut => msg::machine_unreachable_timeout(&channel),
                DialFailure::Refused(why) => msg::cant_reach_named(&channel, why),
            })?;
        let resp = request(
            &conn,
            &Request::Act {
                session: id.0.clone(),
                action: "accept".into(),
            },
        )
        .await?;
        match resp {
            Response::Acted { moved, landed } => Ok((moved, landed)),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// The pull itself, only ever on the recipient machine. Never
    /// re-routes an incoming Act — what lands wrong is refused, or two
    /// serves could bounce one forever.
    async fn accept_local(
        &self,
        ep: &iroh::Endpoint,
        id: &crate::SessionId,
    ) -> Result<(u64, Vec<String>), String> {
        use crate::transfer::{payload_path, pulling_marker};
        let Some(msg_id) = id.0.strip_prefix("transfer/") else {
            return Err(msg::no_such_transfer(&id.0));
        };
        // Also here, not only in `accept_with`: an incoming Act lands
        // straight on this one (`handle`), and the recipient of a routed
        // accept can be behind on the very offer it is being asked to
        // approve. Costs nothing on the way through — the lookup that
        // already succeeded upstream succeeds again before any dial.
        let (channel, m) = self.transfer_after_sync(ep, id, msg_id).await?;
        if channel != self.name() {
            return Err(msg::wrong_recipient(&channel));
        }
        let crate::MsgBody::Files(files) = &m.body else {
            return Err(msg::no_such_transfer(&id.0));
        };
        let home = self
            .devices_loaded()?
            .doc
            .get(&m.from.id)
            .ok_or(msg::offerer_left_table())?;
        let dir = chat::channel_dir(self.root(), &channel)
            .ok_or_else(|| msg::bad_channel_name(format_args!("{channel:?}")))?;
        fs::create_dir_all(dir.join("files")).map_err(msg::cant_make_files_dir)?;

        let outcome = async {
            let conn = self
                .dial(ep, &home.id, &home.addrs, ALPN)
                .await
                .map_err(|e| match e {
                    DialFailure::TimedOut => msg::offerer_unreachable_timeout().to_string(),
                    DialFailure::Refused(why) => msg::offerer_unreachable(why),
                })?;
            let mut moved = 0u64;
            // Where each file ends up, whether or not this run is what
            // put it there: the asker is pointing something at these
            // paths, and a file that was already here is no less
            // present for having arrived earlier.
            let mut landed = Vec::with_capacity(files.len());
            for f in files {
                let dest = payload_path(&dir, f);
                if dest.exists() {
                    landed.push(dest.display().to_string());
                    continue;
                }
                let marker = pulling_marker(&dir, f);
                fs::write(&marker, format!("{}", std::process::id()))
                    .map_err(msg::cant_mark_pulling)?;
                let pulled = pull_one(&conn, &dir, f).await;
                let _ = fs::remove_file(&marker);
                moved += pulled?;
                landed.push(dest.display().to_string());
            }
            Ok((moved, landed))
        }
        .await;
        if outcome.is_ok() {
            self.emit_row_of(&crate::transfer::TransferKind::session_id(&m.id))?;
        }
        outcome
    }

    /// Winds this machine's khor down: the serve (keeper and inner
    /// both) and every session host with a live recorded pid. Processes
    /// only — every file stays, and the next `khor serve` (or the
    /// guardian's next boot pull-up) brings it all back.
    ///
    /// Kills go by **recorded pid**, never by name: on a shared machine
    /// a name matches somebody else's khor too (账本: 后台进程怎么起就
    /// 怎么收). Returns (whether a serve was told to stop, how many
    /// session hosts were).
    pub fn quit(&self) -> Result<(bool, usize), String> {
        let dot = self.root().join(".khor");
        // The serve's front door is the keeper: its pid is in
        // serve.pid (written at its own birth), TERM forwards to the
        // inner, and a clean stop stays stopped — nothing resurrects
        // it but the guardian's next boot.
        let keeper = fs::read_to_string(dot.join("serve.pid"))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .filter(|&p| pid_running(p));
        let inner = self.endpoint_file()?.map(|e| e.pid);
        let served = keeper.is_some() || inner.is_some();
        if let Some(pid) = keeper {
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGTERM);
            }
        } else if let Some(pid) = inner {
            // A serve from before the pid file, or started by hand:
            // the endpoint names the inner, and its process group is
            // the keeper's session (setsid at birth groups exactly the
            // pair) — the group signal reaches both without guessing
            // anybody's ppid. TERM straight at the inner would only
            // teach the keeper to respawn it.
            unsafe {
                let group = libc::getpgid(pid as libc::pid_t);
                if group > 0 {
                    libc::kill(-group, libc::SIGTERM);
                } else {
                    libc::kill(pid as libc::pid_t, libc::SIGTERM);
                }
            }
        }
        if served {
            // Let both lives leave before sweeping hosts, so the
            // count below cannot race a keeper mid-handover.
            for _ in 0..30 {
                let keeper_gone = keeper.is_none_or(|p| !pid_running(p));
                let inner_gone = inner.is_none_or(|p| !pid_running(p));
                if keeper_gone && inner_gone {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        let mut hosts = 0usize;
        if let Ok(entries) = fs::read_dir(crate::live::sessions_dir(self.root())) {
            for entry in entries.flatten() {
                let Ok(hf) = crate::host::read_host_file(&entry.path()) else {
                    continue;
                };
                if !pid_running(hf.host_pid) {
                    continue;
                }
                hosts += 1;
                unsafe {
                    // The child's group first (the shell or agent under
                    // the pty), then the host — the order `close` uses.
                    libc::kill(-(hf.child_pid as libc::pid_t), libc::SIGTERM);
                    libc::kill(hf.host_pid as libc::pid_t, libc::SIGTERM);
                }
            }
        }
        Ok((served, hosts))
    }

    /// Creates a one-time pairing ticket. Needs `khor serve` running —
    /// the ticket carries the live endpoint's addresses.
    ///
    /// The ticket is also **one-time in time**: see [`INVITE_WINDOW_MS`].
    /// Minting sweeps the expired ones first, which is the whole of the
    /// cleanup story — khor has no timer to hang a sweep on, and the one
    /// moment somebody is certainly thinking about invites is the moment
    /// they ask for one.
    pub fn invite(&self) -> Result<String, String> {
        // **Before the serve check**, because it is the more specific
        // truth: a paused machine's serve is running and answering
        // locally, so 「serve 没在跑」 would send somebody to start one
        // that is already there (docs/MORE.md 暂停).
        if self.paused() {
            return Err(msg::paired_while_paused().to_owned());
        }
        let file = self
            .endpoint_file()?
            .ok_or(msg::serve_not_running_for_invite())?;
        let token = fresh_hex()?;
        let dir = self.root().join(".khor").join("invites");
        fs::create_dir_all(&dir).map_err(msg::cant_make_invites_dir)?;
        self.sweep_expired_invites(&dir);
        // The file **is** the mint instant. It used to be empty, and the
        // window is enforced from what is written here rather than from
        // anything in the ticket: one clock decides, and it is the
        // issuer's, so there is no skew between two machines to reason
        // about and nothing the holder of a ticket can edit.
        fs::write(self.invite_path(&token)?, crate::live::now_ms().to_string())
            .map_err(msg::cant_save_invite)?;
        Ticket {
            id: self.device_str().to_owned(),
            name: self.name().to_owned(),
            direct: file.addrs,
            relays: file.relays,
            token,
        }
        .encode()
        .map_err(|e| e.to_string())
    }

    fn invite_path(&self, token: &str) -> Result<std::path::PathBuf, String> {
        // The token lands in a filename; only our own hex survives.
        if token.is_empty() || token.len() > 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(msg::bad_invite().into());
        }
        Ok(self.root().join(".khor").join("invites").join(token))
    }

    /// Whether a token file is still inside its window.
    ///
    /// **An unreadable mint instant is expired**, and that covers the
    /// only compatibility case there is: a ticket minted by a khor from
    /// before this rule wrote an empty file, and "I cannot tell when
    /// this was minted" has exactly one safe reading. The cost is that
    /// upgrading invalidates a ticket in flight, and the fix is to type
    /// `khor invite` again.
    fn invite_is_fresh(path: &std::path::Path) -> bool {
        let Ok(text) = fs::read_to_string(path) else {
            return false;
        };
        let Ok(minted) = text.trim().parse::<i64>() else {
            return false;
        };
        crate::live::now_ms().saturating_sub(minted) <= INVITE_WINDOW_MS
    }

    /// Deletes every ticket that can no longer be used.
    ///
    /// Best effort on purpose: a file that will not go away is a ticket
    /// that is already refused, so failing here costs a stale byte, not
    /// a door left open. Nothing is reported, because nobody asked about
    /// old tickets — they asked for a new one.
    fn sweep_expired_invites(&self, dir: &std::path::Path) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let path = e.path();
            if path.is_file() && !Self::invite_is_fresh(&path) {
                let _ = fs::remove_file(&path);
            }
        }
    }

    /// Accepts a ticket: dial the issuer, burn the token, merge its
    /// device table. When this returns, **both** tables know both
    /// machines — pairing has no direction (docs/NET.md). Routed through
    /// the resident serve when one holds the key.
    pub async fn pair(&self, ticket: &str) -> Result<String, String> {
        // Refused before the ticket is read: a paused machine cannot
        // finish the dial either way, and 「票过期了」 would be an
        // answer about the wrong thing (docs/MORE.md 暂停).
        if self.paused() {
            return Err(msg::paired_while_paused().to_owned());
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::Pair {
                ticket: ticket.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Paired { name } => Ok(name),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        // A one-shot endpoint's addresses die with it — better none than
        // stale; the far side learns real ones through table sync.
        let outcome = self.pair_with(&ep, ticket, vec![]).await;
        // Close before returning on every path: a dropped-but-unclosed
        // endpoint leaves the far side's stream wait dangling until QUIC
        // times out.
        ep.close().await;
        outcome
    }

    /// The pairing itself, on an endpoint the caller owns and outlives.
    async fn pair_with(
        &self,
        ep: &iroh::Endpoint,
        ticket: &str,
        report_addrs: Vec<String>,
    ) -> Result<String, String> {
        let t = Ticket::decode(ticket).map_err(|e| e.to_string())?;
        // Own relays join the ticket's: the issuer may sit behind a relay
        // it did not think to advertise.
        let mut relays = t.relays.clone();
        relays.extend(self.relays().iter().cloned());
        let conn = self
            .dial_with(ep, &t.id, &t.direct, &relays, ALPN)
            .await
            .map_err(|e| match e {
                DialFailure::TimedOut => msg::inviter_unreachable_timeout().to_string(),
                DialFailure::Refused(why) => msg::inviter_unreachable(why),
            })?;
        let resp = request(
            &conn,
            &Request::Pair {
                token: t.token,
                name: self.name().to_owned(),
                addrs: report_addrs,
            },
        )
        .await?;
        match resp {
            Response::Paired { name, devices } => {
                let bytes = B64.decode(devices).map_err(msg::far_table_not_base64)?;
                let loaded = self.devices_loaded()?;
                loaded.doc.merge(&bytes)?;
                let mut store = loaded.store;
                store.flush(&loaded.doc)?;
                Ok(name)
            }
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// Takes **this** machine out of the network (docs/MORE.md 离开).
    ///
    /// # The other half of the refusal `forget` makes
    ///
    /// `Node::forget_device` refuses this machine by name, and its
    /// reason is that a machine which removed itself from the table
    /// would keep every row it holds while becoming a stranger to
    /// everybody else — 「not left the network, broken in a way with no
    /// word for it」. Leaving supplies the missing half: the table is
    /// emptied too, so what is left is a machine that is in no network,
    /// which is a state khor has a word for and can be undone by
    /// pairing.
    ///
    /// # Three steps, in this order
    ///
    /// 1. **Tell the network first**, with the same tombstone `forget`
    ///    writes and down the same road ([`DeviceDoc::set_gone`] has why
    ///    leaving must travel the way joining does). The bit goes onto
    ///    this machine's *own* row and is pushed while the connections
    ///    are still up — after step 3 there is nobody left to push to.
    /// 2. **Empty the table down to this machine** (`keep_only`), and
    ///    clear the tombstone off the row that stays: a machine is not
    ///    gone from its own table. The bit that was sent is the peers'
    ///    copy now, and it is the copy that matters.
    /// 3. **Drop what the others left here** — their cached session
    ///    reports, readings and watch lists, and their rows on the
    ///    workspace list. Sessions need no step of their own: every row
    ///    is derived through the device table, so they leave with it
    ///    (the same thing that happens to a forgotten machine's rows).
    ///    Workspaces do need one — those rows survive a machine leaving
    ///    the table on purpose, so that a row whose machine is gone can
    ///    still be taken off the list — so leaving takes them off.
    ///
    /// **This machine's own sessions, workspaces, identity and settings
    /// are untouched.** Leaving a network is not leaving khor.
    ///
    /// # It works with nobody reachable, and says so
    ///
    /// A machine that cannot dial anyone still leaves — the alternative
    /// is a network you can only leave with the network's permission.
    /// What it cannot do is make the others believe it, so the two
    /// endings are reported apart ([`Left`]) and said in two sentences,
    /// never folded into one: 「everybody knows」 and 「they will go on
    /// seeing you offline until somebody over there runs `khor forget`」
    /// are different situations with different things left to do.
    ///
    /// **The machine's own name is the confirmation**, checked here
    /// rather than on a screen — `lock::reset_identity`'s rule, for its
    /// reason: a flag on the verb and a dialog on the page would be two
    /// rules about one act, and the one that drifted would be the one
    /// nobody ran. Naming another machine is not a typo worth guessing
    /// at, it is somebody reaching for `khor forget`, so it is answered
    /// with that.
    pub async fn leave(&self, name_typed: &str) -> Result<Left, String> {
        // Resolved through the same door `forget` uses, so either name
        // works and an unknown one is refused with the table listed.
        let (_, home) = self.resolve(name_typed)?;
        let all = self.devices()?;
        if home != self.device() {
            // Named by what is on the glass, like every refusal that
            // lists machines (`Node::resolve`): the channel is not what
            // somebody would type next into `khor forget`.
            let shown = all
                .iter()
                .find(|d| d.id == home.hex())
                .map(|d| d.shown().to_owned())
                .unwrap_or_else(|| name_typed.trim().to_owned());
            return Err(msg::leave_names_another_machine(&shown));
        }
        let others: Vec<DeviceInfo> = all
            .into_iter()
            .filter(|d| d.id != self.device_str())
            .collect();
        if others.is_empty() {
            // Refused by name rather than answered with a cheerful
            // nothing: this machine is not in a network, and 「left」
            // would be a sentence about an act that did not happen.
            return Err(msg::leave_with_no_network().into());
        }

        // 1 — the tombstone, onto this machine's own row.
        {
            let loaded = self.devices_loaded()?;
            loaded.doc.set_gone(self.device_str(), true)?;
            let mut store = loaded.store;
            store.flush(&loaded.doc)?;
        }
        // …and out, while there is still a table to send it to. A sync
        // this machine could not start at all (no endpoint to bind) is
        // the same ending as one where nobody answered: nobody was
        // told, which is the sentence below.
        let outcomes = self.sync_now().await.unwrap_or_default();
        // **Walked over the table, not over the outcomes.** Every row
        // gets exactly one ending that way — a machine the sync never
        // visited at all was not told, which is an answer, while a
        // machine missing from this list would be a machine khor said
        // nothing about. And the names come out as `shown()`, one
        // spelling: `sync_with_all` reports the channel, and on a
        // renamed machine that is a word nobody can see on any screen.
        let mut left = Left::default();
        for d in &others {
            let heard = outcomes
                .iter()
                .any(|(name, verdict)| name == &d.name && verdict.is_ok());
            if heard {
                left.told.push(d.shown().to_owned());
            } else {
                left.untold.push(d.shown().to_owned());
            }
        }

        // 2 — the table, down to this machine.
        {
            let loaded = self.devices_loaded()?;
            loaded.doc.keep_only(self.device_str())?;
            loaded.doc.set_gone(self.device_str(), false)?;
            let mut store = loaded.store;
            store.flush(&loaded.doc)?;
        }

        // 3 — what the others left on this disk. Best effort, each on
        // its own: a cache that will not go away is a stale byte nobody
        // paints (every reader of these walks the device table first),
        // and failing the leave over one would leave a machine half out
        // of a network it has already been said goodbye to.
        for dir in self.peer_caches() {
            let _ = fs::remove_dir_all(dir);
        }
        for ws in self.workspaces().unwrap_or_default() {
            if ws.device != self.device_str() {
                let _ = self.ws_rm(&ws.device, &ws.path);
            }
        }
        Ok(left)
    }

    /// Which road this machine is using to reach each device it knows.
    ///
    /// **No fallback to a fresh endpoint, unlike every other verb here.**
    /// `sync_now` binds its own when no resident holds the key, because
    /// syncing from a new endpoint is still syncing. A *reading* taken
    /// from a new endpoint is not the same reading: it has spoken to
    /// nobody, so every machine would come back as "nothing flowing" —
    /// a confident, wrong, and completely plausible answer to the one
    /// question this exists for. With no resident, the honest answer is
    /// that khor cannot say, and that is a word (`khor_core::Hop`).
    pub async fn hops(&self) -> Vec<(String, khor_core::Hop)> {
        let every = |h: khor_core::Hop| {
            self.devices()
                .unwrap_or_default()
                .into_iter()
                .map(|d| (d.id, h))
                .collect::<Vec<_>>()
        };
        match self.via_serve(ipc::Op::Hops).await {
            Some(Ok(ipc::Reply::Hops { by_device })) => by_device,
            // **Three silences, not one.** No resident at all is the one
            // with a remedy in it — nothing here is holding an endpoint,
            // and starting `khor serve` fixes it. A resident that
            // answered something else is one too old to know the
            // question, which is a thing khor genuinely cannot say.
            // Reporting both as "cannot say" would hide the fixable one
            // behind the unfixable one.
            None => every(khor_core::Hop::NoServe),
            Some(_) => every(khor_core::Hop::Unknown),
        }
    }

    /// One sync visit to every known device, now. Returns per-device
    /// outcomes — "moved nothing" and "failed" wear different faces.
    /// Routed through the resident serve when one holds the key.
    pub async fn sync_now(&self) -> Result<Vec<(String, Result<String, String>)>, String> {
        if let Some(reply) = self.via_serve(ipc::Op::SyncNow).await {
            return match reply? {
                ipc::Reply::Synced { outcomes } => Ok(outcomes),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.sync_with_all(&ep).await;
        ep.close().await;
        Ok(out)
    }

    async fn sync_with_all(&self, ep: &iroh::Endpoint) -> Vec<(String, Result<String, String>)> {
        let list = self
            .devices_loaded()
            .map(|l| l.doc.all())
            .unwrap_or_default();
        let mut out = Vec::new();
        for d in list {
            if d.id == self.device_str() {
                continue;
            }
            let verdict = self.sync_device(ep, &d).await;
            out.push((d.name, verdict));
        }
        out
    }

    async fn sync_device(&self, ep: &iroh::Endpoint, d: &DeviceInfo) -> Result<String, String> {
        // A device that never reported a road (one-shot pairings) cannot
        // be dialed, only dial us — probing it burns a full DIAL_TIMEOUT
        // per pump for nothing.
        if d.addrs.is_empty() {
            return Err(msg::no_roads_reported().into());
        }
        let conn = self
            .dial(ep, &d.id, &d.addrs, ALPN)
            .await
            .map_err(|e| match e {
                DialFailure::TimedOut => msg::dial_timed_out().to_string(),
                DialFailure::Refused(why) => msg::cant_reach_plain(why),
            })?;

        let mut moved = 0usize;
        {
            let mut loaded = self.devices_loaded()?;
            moved += self.rounds(&conn, "devices", &mut loaded).await?;
        }
        {
            let mut loaded = self.seen_loaded()?;
            moved += self.rounds(&conn, "seen", &mut loaded).await?;
        }
        // Pins ride the same pump as the read watermark, and for the same
        // reason: both say something about a session that belongs to the
        // session rather than to the screen looking at it.
        {
            let mut loaded = self.pins_loaded()?;
            moved += self.rounds(&conn, "pins", &mut loaded).await?;
        }
        // Directory pins too — a shortcut chosen on the phone must be
        // there when the desk opens the same machine's disk. Best
        // effort, unlike the two above: a peer that predates the table
        // refuses the doc by name, and that refusal must not cost the
        // chat rounds behind it — it only means older shortcuts there.
        {
            let mut loaded = self.dirpins_loaded()?;
            if let Ok(n) = self.rounds(&conn, "dirpins", &mut loaded).await {
                moved += n;
            }
        }
        // The workspaces, on the same best-effort footing: a workspace
        // declared on the desk must be on the phone's list, and a peer
        // that predates the table refuses the doc by name — which must
        // not cost the rounds behind it. What it costs instead is a
        // shorter list over there, never a wrong one.
        {
            let mut loaded = self.workspaces_loaded()?;
            if let Ok(n) = self.rounds(&conn, "workspaces", &mut loaded).await {
                moved += n;
            }
        }
        // Web pins, same best-effort footing as directory pins and for
        // the same reason: an older peer refuses the doc by name, and that
        // must not cost the rounds behind it.
        {
            let mut loaded = self.webpins_loaded()?;
            if let Ok(n) = self.rounds(&conn, "webpins", &mut loaded).await {
                moved += n;
            }
        }
        // Where this person has been, on the same best-effort footing
        // and for the same reason: an older peer refuses the doc by
        // name, and that must not cost the rounds behind it. What it
        // costs instead is a phone whose suggestion list stops at what
        // that phone itself has opened — no wrong answer, a shorter one.
        {
            let mut loaded = self.recents_loaded()?;
            if let Ok(n) = self.rounds(&conn, "recents", &mut loaded).await {
                moved += n;
            }
        }
        // The agent registry (批⑥): an agent named on one machine is
        // offerable on every machine, which is the whole reason it is a
        // document. Best effort on the same footing as the two pin
        // tables above — a peer that predates the registry refuses the
        // doc by name, and that refusal must not cost the rounds behind
        // it; it only means that machine offers what it was told
        // locally.
        {
            let mut loaded = self.agents_loaded()?;
            if let Ok(n) = self.rounds(&conn, "agents", &mut loaded).await {
                moved += n;
            }
        }
        // Registered API accounts — descriptions only, the keys never
        // leave the machine that holds them (`khor_sync::accounts`).
        // Same best-effort footing: an older peer refuses the doc by
        // name and the rounds behind it still run.
        {
            let mut loaded = self.accounts_loaded()?;
            if let Ok(n) = self.rounds(&conn, "accounts", &mut loaded).await {
                moved += n;
            }
        }
        // Session names, on the same best-effort footing as the tables
        // above and for the same reason: a peer that predates the table
        // refuses the doc by name, and that refusal must not cost the
        // chat rounds behind it — it only means older names there.
        {
            let mut loaded = self.titles_loaded()?;
            if let Ok(n) = self.rounds(&conn, "titles", &mut loaded).await {
                moved += n;
            }
        }
        for ch in self.known_channels()? {
            let dir = chat::channel_dir(self.root(), &ch)
                .ok_or_else(|| msg::bad_channel_name(format_args!("{ch:?}")))?;
            let mut loaded = chat::open_channel(&dir, self.writer_peer())?;
            moved += self
                .rounds(&conn, &format!("chat/{ch}"), &mut loaded)
                .await?;
        }
        // Remember what the far side reports about its own sessions:
        // when it goes unreachable, its last word plus the report's age
        // is all the UI may honestly show. Best effort — a failure here
        // only means older information.
        if let Ok(Response::SessionRows { rows }) = request(&conn, &Request::Sessions).await {
            let _ = self.cache_peer_rows(&d.id, &d.name, &rows);
        }
        // And what it is doing right now, on the same visit. Kept apart
        // from the rows above rather than folded into one record: the two
        // requests fail independently, so one round losing vitals would
        // wipe a reading the rows knew nothing about, and one shared
        // timestamp would make a stale reading look as fresh as the rows
        // beside it. Best effort, same as the rows — a peer running a khor
        // that has no such op simply has no reading here.
        if let Ok(Response::Vitals { vitals }) = request(&conn, &Request::Vitals).await {
            let _ = self.cache_peer_vitals(&d.id, &vitals);
        }
        // Its watched directories, on the same visit and in a record of
        // their own (2026-09-06 关注目录): the sync round just above
        // carried our copy of its `watch` list over, so what comes back
        // is already about that list. An older khor refuses the op and
        // simply has no rows here — never empty ones.
        if let Ok(Response::Watched { rows }) = request(&conn, &Request::Watched).await {
            let _ = self.cache_peer_watched(&d.id, &rows);
        }
        // Its day, on the same beat as its rows: the strips under a
        // peer's rows are read from this, and a `Refused` from an older
        // khor leaves the last cached day standing, aging like a report.
        if let Ok(Response::Days { day }) = request(&conn, &Request::Days).await {
            let _ = self.cache_peer_day(&d.id, &day);
        }
        // Its accounts' readings, on the same beat: the 额度 page draws
        // every machine's rings from this, and an older khor's Refused
        // leaves the last carried answer standing, aging like a report.
        if let Ok(Response::Quotas { readings }) = request(&conn, &Request::Quotas).await {
            let _ = self.cache_peer_quotas(&d.id, &readings);
        }
        // …and the readings of every satellite this device sponsors, on
        // the same visit (附属机器批): satellites cannot be dialed, so
        // their numbers ride the road to the one machine that samples
        // them. Best effort per satellite; the age crosses too, so a
        // reading keeps its true birth instead of the arrival's
        // freshness. An older sponsor refuses the op, which reads as
        // "no reading" — the row stays on the offline axis.
        if let Ok(devs) = self.devices() {
            for sat in devs
                .iter()
                .filter(|s| s.sponsor.as_deref() == Some(d.id.as_str()))
            {
                if let Ok(Response::SatVitals { vitals, age_ms }) = request(
                    &conn,
                    &Request::SatVitals {
                        sat: sat.id.clone(),
                    },
                )
                .await
                {
                    let _ = self.cache_peer_vitals_aged(&sat.id, &vitals, age_ms);
                }
            }
        }
        // And what it has spent, on the same visit and kept in its own
        // record for the same reason. **Asked every round like the two
        // above, which is affordable because the answering side keeps its
        // answer**: a peer that has written nothing since the last round
        // replies from a cache it validated with one directory walk
        // (`crate::usage::Meters::tally` has the figures). Best effort —
        // a peer running a khor with no such op simply has no spending
        // here, which reads as "never asked" rather than as zero.
        if let Ok(Response::Usage { usage }) = request(&conn, &Request::Usage).await {
            let _ = self.cache_peer_usage(&d.id, &usage);
        }
        Ok(if moved == 0 {
            msg::nothing_to_move().into()
        } else {
            msg::moved_bytes(moved)
        })
    }

    /// Two wire rounds: the first is pull-only by design, the second
    /// pushes. Returns bytes moved.
    async fn rounds<D: Doc>(
        &self,
        conn: &iroh::endpoint::Connection,
        doc_name: &str,
        loaded: &mut khor_sync::store::Loaded<D>,
    ) -> Result<usize, String> {
        let mut peer = wire::Peer::new();
        let mut moved = 0usize;
        for _ in 0..2 {
            let out = peer.outgoing(&loaded.doc)?;
            let resp = request(
                conn,
                &Request::Sync {
                    doc: doc_name.to_owned(),
                    have: out.have.clone(),
                    changes: out.changes.clone(),
                },
            )
            .await?;
            let reply = match resp {
                Response::Synced {
                    version,
                    changes,
                    items,
                } => wire::Reply {
                    version,
                    changes,
                    items: items as usize,
                },
                Response::Refused { why } => return Err(why),
                other => return Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
            let round = peer.absorb(&mut loaded.store, &loaded.doc, &out, reply)?;
            moved += round.pushed + round.pulled;
        }
        Ok(moved)
    }

    /// Channels worth syncing: every machine's window, plus any channel
    /// directory already on disk.
    fn known_channels(&self) -> Result<BTreeSet<String>, String> {
        let mut set: BTreeSet<String> = self
            .devices_loaded()?
            .doc
            .all()
            .into_iter()
            .map(|d| d.name)
            .collect();
        let chat_root = self.root().join(chat::REL_DIR);
        if let Ok(rd) = fs::read_dir(chat_root) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if chat::valid_channel(&name) {
                    set.insert(name);
                }
            }
        }
        Ok(set)
    }

    /// The addresses a **live** serve says this machine has, or `None`
    /// when nothing is serving.
    ///
    /// Both halves of that sentence are the answer to a question
    /// somebody asks before printing a link to hand to another device:
    /// which address to print, and whether anything is listening at it
    /// (`khor web`). Read off `endpoint.json` rather than off the
    /// interfaces, so it is the same set khor already tells the network
    /// to dial — an address khor itself does not believe in is not one
    /// to put in front of a person.
    ///
    /// The ports are the endpoint's own and are dropped: a consumer is
    /// asking where this machine *is*, not where this one socket is.
    pub fn local_ips(&self) -> Result<Option<Vec<std::net::IpAddr>>, String> {
        let Some(file) = self.endpoint_file()? else {
            return Ok(None);
        };
        let mut ips: Vec<std::net::IpAddr> = Vec::new();
        for a in &file.addrs {
            if let Ok(sock) = a.parse::<std::net::SocketAddr>()
                && !ips.contains(&sock.ip())
            {
                ips.push(sock.ip());
            }
        }
        Ok(Some(ips))
    }

    /// The pid of the resident `khor serve` holding this key, or `None`
    /// when nothing is.
    ///
    /// **Half an answer on purpose.** A live pid is not a serve that
    /// answers — the socket can be wedged while the process is alive —
    /// and `khor doctor` reports the two apart (`doctor::check_serve`).
    /// This side only says whether anything claims the key.
    pub fn serve_pid(&self) -> Option<u32> {
        self.endpoint_file().ok().flatten().map(|f| f.pid)
    }

    fn endpoint_file(&self) -> Result<Option<EndpointFile>, String> {
        let path = self.root().join(".khor").join("endpoint.json");
        let Ok(text) = fs::read_to_string(&path) else {
            return Ok(None);
        };
        let file: EndpointFile = serde_json::from_str(&text).map_err(msg::endpoint_file_garbled)?;
        if pid_alive(file.pid) {
            Ok(Some(file))
        } else {
            Ok(None)
        }
    }
}

/// 16 random bytes as hex — invite tokens, hand-off cookies, session
/// leaves, the web face's key.
///
/// Public because the last of those is minted outside this crate
/// (`khor_web`), and a capability that has to be guessed in 2^128 tries
/// is one fact: a second implementation of it somewhere else is a
/// second chance to get the width wrong. `LEAF_HEX` below has the
/// birthday arithmetic that settled 128.
pub fn fresh_hex() -> Result<String, String> {
    let mut raw = [0u8; 16];
    getrandom::fill(&mut raw).map_err(msg::no_entropy)?;
    Ok(raw.iter().map(|b| format!("{b:02x}")).collect())
}

/// Hex digits in a session id khor mints for itself. 16 of them = **64
/// bits**.
///
/// # Why 64 and not the 8 digits this used to take
///
/// Session ids are the key of two network-wide tables that carry no
/// machine in the key (`khor_sync::seen`, `khor_sync::pins`), so the
/// whole scheme rests on ids being unique across the network, not just
/// on one machine. That module grades the four kinds of id, and this was
/// the one it graded weak.
///
/// The birthday bound: with `n` ids of `b` bits, a collision arrives with
/// probability about `n² / 2^(b+1)`.
///
/// - at the old **32 bits**: 10⁴ ids → ~1.2%, 10⁵ ids → effectively
///   certain. A few thousand `khor run`s across a handful of machines is
///   an ordinary year, so this was not a distant risk.
/// - at **64 bits**: 10⁶ ids → ~3·10⁻⁸. Far past anything this product
///   will mint.
///
/// 64 is chosen rather than merely sufficient because it is the width
/// `khor_sync::pins` already calls strong for `transfer/` ids, which ride
/// the loro peer id. Putting khor's own mint in the class that document
/// already blessed means the grading table has one answer instead of
/// three, and nobody has to re-derive whether "probably enough" still
/// holds.
///
/// # The typing cost, which is real and is not the deciding factor
///
/// `khor attach <session>` takes the id exactly, so this makes a
/// hand-typed id eight characters longer. Ids are read off `khor
/// sessions` and pasted, and the GUI never types one — and if hand-typing
/// ever becomes the path people actually take, the answer is resolving a
/// unique prefix, not a shorter id. A short id trades a permanent
/// correctness margin for a convenience that a lookup rule gives back for
/// free.
///
/// **Ids already minted are unaffected**: nothing parses or validates the
/// leaf's length, so an old 8-digit id keeps naming its session.
pub(crate) const LEAF_HEX: usize = 16;

/// A fresh session leaf — the part after `<kind>/`.
pub(crate) fn fresh_leaf() -> Result<String, String> {
    Ok(fresh_hex()?.chars().take(LEAF_HEX).collect())
}

/// Owner-only from the first byte: the hand-off cookie in endpoint.json
/// is a capability, and create-then-chmod would leave a readable window.
///
/// Public for the same reason as [`fresh_hex`] — the web face's key is
/// written outside this crate and must land with the same permissions
/// on the same machines, including the shared ones where 0600 is the
/// whole defence against the account next door.
pub fn write_private(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let _ = fs::remove_file(path);
    let mut opts = fs::OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts
        .open(path)
        .map_err(|e| msg::cant_write(path.display(), e))?;
    f.write_all(bytes).map_err(msg::cant_finish_write)
}

async fn request(conn: &iroh::endpoint::Connection, req: &Request) -> Result<Response, String> {
    let (mut send, mut recv) = conn.open_bi().await.map_err(msg::cant_open_stream)?;
    send.write_all(&proto::encode(req)?)
        .await
        .map_err(msg::cant_send)?;
    send.finish().map_err(msg::cant_close_stream)?;
    let bytes = recv.read_to_end(MAX_FRAME).await.map_err(msg::no_answer)?;
    proto::decode(&bytes)
}

/// Pulls one payload into its partial, slice by slice, and promotes it to
/// its real name only after the digest checks out. Returns bytes moved
/// this run — a resumed pull moves only the missing tail.
async fn pull_one(
    conn: &iroh::endpoint::Connection,
    dir: &std::path::Path,
    f: &khor_sync::chat::FileRef,
) -> Result<u64, String> {
    use std::io::{Read, Write};

    use crate::transfer::{partial_path, payload_path};
    let partial = partial_path(dir, f);
    // Resume: the final digest must cover the bytes already on disk, so
    // they run through the hasher before any new slice does.
    let mut hasher = blake3::Hasher::new();
    let mut offset = 0u64;
    if let Ok(mut existing) = fs::File::open(&partial) {
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = existing.read(&mut buf).map_err(msg::cant_read_partial)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            offset += n as u64;
        }
    }
    let mut out = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&partial)
        .map_err(msg::cant_open_partial)?;
    let expect = f.size.max(0) as u64;
    let mut moved = 0u64;
    while offset < expect {
        let resp = request(
            conn,
            &Request::Fetch {
                digest: f.digest.clone(),
                offset,
            },
        )
        .await?;
        let (total, bytes) = match resp {
            Response::Slice { total, bytes } => (total, bytes),
            Response::Refused { why } => return Err(why),
            other => return Err(msg::peer_non_answer(format_args!("{other:?}"))),
        };
        if total != expect {
            return Err(msg::size_mismatch(total, expect));
        }
        if bytes.is_empty() {
            return Err(msg::empty_slice().into());
        }
        out.write_all(&bytes).map_err(msg::cant_write_slice)?;
        hasher.update(&bytes);
        offset += bytes.len() as u64;
        moved += bytes.len() as u64;
    }
    out.sync_all().map_err(msg::cant_sync_disk)?;
    drop(out);
    let got = hasher.finalize().to_hex().to_string();
    if got != f.digest {
        // A mismatch poisons the partial — deleted here, at the only
        // place that knows. Any other failure leaves it: it is the
        // resume point.
        let _ = fs::remove_file(&partial);
        return Err(msg::digest_mismatch(
            &got[..8],
            f.digest.chars().take(8).collect::<String>(),
        ));
    }
    fs::rename(&partial, payload_path(dir, f)).map_err(msg::cant_rename)?;
    Ok(moved)
}

/// Whether that pid is still a live process.
///
/// `libc::kill(pid, 0)` rather than shelling out to `kill -0`: khor
/// compiles everything it needs in (docs/KHOR.md), and `kill` is a
/// program Windows does not have. Signal 0 sends nothing — it only runs
/// the permission and existence checks.
///
/// **`EPERM` counts as alive**, which is where this is deliberately more
/// accurate than the shell was. A process owned by another user exists
/// but refuses the signal; the shell reported that as failure, so khor
/// read "somebody else's process" as "dead" and settled the row on a
/// missing ending. Only `ESRCH` — no such process — is death.
/// Whether a pid is a process that is still **running** — a zombie is
/// not one, and [`pid_alive`] cannot tell the difference.
///
/// A signal probe answers "the pid is taken", and a child that has
/// exited keeps its pid until somebody waits for it. Khor spawns its
/// hosts detached and never waits, so inside the process that spawned
/// one, `kill(pid, 0)` says "alive" forever after it dies — which is how
/// a takeover that politely waits for the old host to go ended up
/// reporting 那边的进程没退 about a process that had exited a
/// millisecond earlier.
///
/// Reaps, when the pid is this process's own child. That is safe here
/// because khor's detached helpers are precisely the ones nobody waits
/// for; a child somebody does wait for (the PTY child, inside its own
/// host) is never asked about from the process holding its handle.
pub(crate) fn pid_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let mut status = 0;
        // WNOHANG: 0 = ours and still running, pid = ours and reaped
        // just now, -1 = not ours, and then the signal probe is all
        // there is to go on.
        if unsafe { libc::waitpid(pid as libc::pid_t, &mut status, libc::WNOHANG) }
            == pid as libc::pid_t
        {
            return false;
        }
    }
    pid_alive(pid)
}

pub(crate) fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        if unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
            return true;
        }
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(not(unix))]
    {
        // Windows bring-up owns this (on the ledger): the answer there is
        // OpenProcess/GetExitCodeProcess, not a signal. Saying "alive"
        // here would park every dead session on its last word forever,
        // and saying "dead" would bury every live one — so the honest
        // placeholder is the one that keeps a word standing until a real
        // implementation can pronounce it, which is what a missing pid
        // already means everywhere else in this file.
        let _ = pid;
        true
    }
}

/// Why a dial did not produce a connection. Kept apart from the sentence
/// a person reads: every caller says who it was trying to reach and what
/// for, and that is the half worth keeping at the call site.
enum DialFailure {
    TimedOut,
    Refused(String),
}

impl Node {
    /// One dial to a machine, **and the retry that finds it again after
    /// it restarts**.
    ///
    /// The direct addresses are a hint that expires: they are whatever
    /// that machine last wrote into the devices table, and a machine
    /// that restarts its serve binds new ports. So after an upgrade —
    /// which restarts serve by definition — every road in the table
    /// leads nowhere, while the relay, reached by identity rather than
    /// by port, still leads home. Handing iroh both spends the whole
    /// budget on the dead ones.
    ///
    /// Measured on turing: after its serve restarted, this end took
    /// upwards of ten minutes to find it again, and `khor sync` — the
    /// obvious cure — cannot help, because **syncing is itself a dial**.
    /// The fresh address lives inside the thing you need the fresh
    /// address to reach.
    ///
    /// **The two roads run side by side, the relay after a head start**
    /// (`RELAY_HEAD_START` carries the two shapes this had before and
    /// what each of them charged to the wrong machine). A machine that
    /// is off fails in exactly `DIAL_TIMEOUT` — the budget this always
    /// had — and a machine on the LAN never touches the relay at all.
    ///
    /// The relay attempt is a *retry* and not the only attempt, because
    /// the direct roads are what make a peer on the same LAN fast.
    async fn dial_with(
        &self,
        ep: &iroh::Endpoint,
        id: &str,
        direct: &[String],
        relays: &[String],
        alpn: &[u8],
    ) -> Result<iroh::endpoint::Connection, DialFailure> {
        // 按机器暂停: not dialled at all, and said so — the sync tick,
        // every verb and every tunnel come through here.
        if crate::pause::peer_on(self.root(), id) {
            return Err(DialFailure::Refused(msg::peer_paused_refusal().to_owned()));
        }
        let bare = |e| DialFailure::Refused(format!("{e}"));
        let mut roads = tokio::task::JoinSet::new();
        if !direct.is_empty() {
            let addr = endpoint::dial_addr(id, direct, relays).map_err(bare)?;
            let (ep, alpn) = (ep.clone(), alpn.to_vec());
            roads.spawn(async move { ep.connect(addr, &alpn).await });
        }
        // By identity alone. Nothing here can be stale: a machine's id
        // is the one thing about it that a restart does not change.
        {
            let addr = endpoint::dial_addr(id, &[], relays).map_err(bare)?;
            let (ep, alpn, wait) = (ep.clone(), alpn.to_vec(), !direct.is_empty());
            roads.spawn(async move {
                if wait {
                    tokio::time::sleep(RELAY_HEAD_START).await;
                }
                ep.connect(addr, &alpn).await
            });
        }

        let deadline = tokio::time::sleep(DIAL_TIMEOUT);
        tokio::pin!(deadline);
        // First road home wins; a road that fails only counts if every
        // other one does too, so one dead direct address cannot decide
        // the answer while the relay is still walking.
        let mut refused: Option<String> = None;
        loop {
            tokio::select! {
                () = &mut deadline => return Err(DialFailure::TimedOut),
                joined = roads.join_next() => match joined {
                    Some(Ok(Ok(conn))) => return Ok(conn),
                    Some(Ok(Err(e))) => refused = Some(e.to_string()),
                    Some(Err(_)) => {}
                    None => return Err(match refused {
                        Some(why) => DialFailure::Refused(why),
                        None => DialFailure::TimedOut,
                    }),
                },
            }
        }
    }

    /// `dial_with`, using this node's own relays — what every verb wants.
    async fn dial(
        &self,
        ep: &iroh::Endpoint,
        id: &str,
        direct: &[String],
        alpn: &[u8],
    ) -> Result<iroh::endpoint::Connection, DialFailure> {
        self.dial_with(ep, id, direct, self.relays(), alpn).await
    }
}

/// How long the road question may take, for the whole device table.
///
/// **Not borrowed from the dial budgets.** The serve's other waits
/// (`DIAL_TIMEOUT`, the tunnel's `CONNECT_TIMEOUT`, 10 s each) cover a
/// trip across a network; this one never leaves the process — it asks
/// iroh's in-memory map, which answered in 20-75 µs across a 40 s
/// probe on 2026-09-21. The ceiling that matters is the caller's: the
/// app asks for the device list on a 2 s beat, from a bridge that
/// serves one request at a time, so a wait anywhere near 2 s stacks the
/// next poll behind this one. Half a second is four orders over normal
/// and a quarter of that beat.
const HOPS_BUDGET: Duration = Duration::from_millis(500);

/// The road question for every device, one answer each, inside one
/// budget for the lot.
///
/// **A machine whose answer is late reads [`khor_core::Hop::Unknown`]
/// and is named in the second list** — never left out, and never
/// waited on past the budget. The asks run in order against one
/// deadline, and each is polled once even when the deadline has
/// passed, so an answer that is already there still counts: one stuck
/// machine costs the budget, not every machine after it.
async fn hops_within<F>(
    asks: Vec<(String, F)>,
    budget: Duration,
) -> (Vec<(String, khor_core::Hop)>, Vec<String>)
where
    F: std::future::Future<Output = Option<endpoint::RoadsInUse>>,
{
    let deadline = tokio::time::Instant::now() + budget;
    let mut by_device = Vec::new();
    let mut late = Vec::new();
    for (id, ask) in asks {
        let hop = match tokio::time::timeout_at(deadline, ask).await {
            Ok(Some(u)) => khor_core::Hop::of(u.direct, u.relay, u.other),
            Ok(None) => khor_core::Hop::Unknown,
            Err(_) => {
                late.push(id.clone());
                khor_core::Hop::Unknown
            }
        };
        by_device.push((id, hop));
    }
    (by_device, late)
}

#[cfg(test)]
mod hops {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;

    type Ask = Pin<Box<dyn Future<Output = Option<endpoint::RoadsInUse>> + Send>>;

    fn stuck() -> Ask {
        Box::pin(std::future::pending())
    }

    fn direct() -> Ask {
        Box::pin(std::future::ready(Some(endpoint::RoadsInUse {
            direct: true,
            relay: false,
            other: false,
        })))
    }

    /// **A road question that never comes back costs the budget, not the
    /// caller.** Measured 2026-09-21: iroh's per-remote actor spun in
    /// `open_path_on_all_conns` (upstream #4390), `remote_info` queued
    /// behind it, and `Op::Hops` took 52950 ms and then never answered —
    /// with the bridge's one request thread waiting on it, so the whole
    /// face froze. The stuck machine reads 说不出 and is named; the one
    /// after it still gets its real answer.
    #[tokio::test]
    async fn a_road_question_that_never_answers_costs_the_budget_not_the_caller() {
        let asks = vec![("stuck".to_owned(), stuck()), ("fine".to_owned(), direct())];
        let started = std::time::Instant::now();
        let Ok((by_device, late)) =
            tokio::time::timeout(Duration::from_secs(5), hops_within(asks, HOPS_BUDGET)).await
        else {
            panic!("the road question hung for 5 s on one machine that never answered");
        };
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "took {:?} — past the GUI's 2 s poll beat",
            started.elapsed()
        );
        assert_eq!(
            by_device,
            vec![
                ("stuck".to_owned(), khor_core::Hop::Unknown),
                ("fine".to_owned(), khor_core::Hop::Direct),
            ]
        );
        assert_eq!(
            late,
            vec!["stuck".to_owned()],
            "the late machine is named, not dropped"
        );
    }
}

#[cfg(test)]
mod deafness {
    //! The DeafWatch alone: pure verdicts on an injected clock — no
    //! sockets, no sleeping. The end-to-end half of 批23's verdict is
    //! clinical — the fix was deployed onto a serve that was actually
    //! deaf at the time (2026-08-20, Mac).

    use super::*;
    use std::time::Instant;

    fn all_dead() -> Vec<(String, Result<String, String>)> {
        vec![
            ("a".into(), Err("dial timed out".into())),
            ("b".into(), Err("refused".into())),
        ]
    }

    #[test]
    fn deafness_is_concluded_by_elapsed_time_and_not_before() {
        let mut w = DeafWatch::new();
        let t0 = Instant::now();
        assert!(
            !w.observe_at(&all_dead(), t0),
            "the first failing round starts the clock"
        );
        assert!(
            !w.observe_at(&all_dead(), t0 + DEAF_AFTER - Duration::from_secs(1)),
            "one second short of the budget is not deafness"
        );
        assert!(
            w.observe_at(&all_dead(), t0 + DEAF_AFTER),
            "the budget spent in silence is"
        );
    }

    #[test]
    fn one_answer_resets_the_clock() {
        let mut w = DeafWatch::new();
        let t0 = Instant::now();
        assert!(!w.observe_at(&all_dead(), t0));
        let mut mixed = all_dead();
        mixed.push(("c".into(), Ok("synced".into())));
        assert!(
            !w.observe_at(&mixed, t0 + DEAF_AFTER * 2),
            "an answered round is not deafness, however late it lands"
        );
        let t1 = t0 + DEAF_AFTER * 3;
        assert!(
            !w.observe_at(&all_dead(), t1),
            "the clock must restart after the answer"
        );
        assert!(!w.observe_at(&all_dead(), t1 + DEAF_AFTER - Duration::from_secs(1)));
        assert!(w.observe_at(&all_dead(), t1 + DEAF_AFTER));
    }

    #[test]
    fn few_but_slow_rounds_still_read_as_deafness() {
        // Rounds are not SYNC_EVERY apart when everyone is down: each
        // visit waits out its dial budget, so two rounds can span many
        // minutes. The verdict follows the wall clock, not the round
        // count — the retired 24-round rule measured eight minutes.
        let mut w = DeafWatch::new();
        let t0 = Instant::now();
        assert!(!w.observe_at(&all_dead(), t0));
        assert!(
            w.observe_at(&all_dead(), t0 + DEAF_AFTER * 4),
            "the second round already lands past the budget"
        );
    }

    #[test]
    fn roadless_devices_and_empty_meshes_never_read_as_deafness() {
        let mut w = DeafWatch::new();
        let roadless: Vec<(String, Result<String, String>)> =
            vec![("ghost".into(), Err(msg::no_roads_reported().into()))];
        let t0 = Instant::now();
        for i in 0..48u32 {
            let now = t0 + DEAF_AFTER * i;
            assert!(
                !w.observe_at(&roadless, now),
                "a device with no roads cannot count"
            );
            assert!(!w.observe_at(&[], now), "a mesh of one has nobody to miss");
        }
    }
}
