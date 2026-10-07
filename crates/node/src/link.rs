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
use crate::{Node, ipc, wires};

/// **Every synced document by its wire name**, loaded, bound to
/// `$loaded` (a `&mut khor_sync::store::Loaded<_>`) for `$body` — one
/// list for the arms that answer a sync, compare versions and run a
/// round, so a document added to one is added to all. An unknown name is
/// `msg::unknown_doc`. In a function returning `Result<_, String>`.
macro_rules! with_doc {
    ($node:expr, $name:expr, |$loaded:ident| $body:expr) => {{
        let name: &str = $name;
        match name {
            "devices" => { let mut l = $node.devices_loaded()?; let $loaded = &mut l; $body }
            "seen" => { let mut l = $node.seen_loaded()?; let $loaded = &mut l; $body }
            "pins" => { let mut l = $node.pins_loaded()?; let $loaded = &mut l; $body }
            "roster" => { let mut l = $node.roster_loaded()?; let $loaded = &mut l; $body }
            "dirpins" => { let mut l = $node.dirpins_loaded()?; let $loaded = &mut l; $body }
            "workspaces" => { let mut l = $node.workspaces_loaded()?; let $loaded = &mut l; $body }
            "webpins" => { let mut l = $node.webpins_loaded()?; let $loaded = &mut l; $body }
            "recents" => { let mut l = $node.recents_loaded()?; let $loaded = &mut l; $body }
            "agents" => { let mut l = $node.agents_loaded()?; let $loaded = &mut l; $body }
            "accounts" => { let mut l = $node.accounts_loaded()?; let $loaded = &mut l; $body }
            "titles" => { let mut l = $node.titles_loaded()?; let $loaded = &mut l; $body }
            _ => match name.strip_prefix("chat/") {
                Some(ch) => {
                    let dir = chat::channel_dir($node.root(), ch)
                        .ok_or_else(|| msg::bad_channel_name(format_args!("{ch:?}")))?;
                    let mut l = chat::open_channel(&dir, $node.writer_peer())?;
                    let $loaded = &mut l;
                    $body
                }
                // A khor-wide conversation (`wide/<name>`, foundation B):
                // the same document shape, kept beside the channels.
                None => match chat::wide_dir($node.root(), name) {
                    Some(dir) => {
                        let mut l = chat::open_channel(&dir, $node.writer_peer())?;
                        let $loaded = &mut l;
                        $body
                    }
                    None => Err(msg::unknown_doc(name)),
                },
            },
        }
    }};
}

/// The documents a round syncs, in the order it syncs them; chat
/// channels follow (`Node::known_channels`), then the khor-wide
/// conversations (`Node::known_wide`, best effort like the tables after
/// the first three: a peer that predates them refuses them by name). The first three and every
/// channel must sync for the round to count; the rest are best effort —
/// a peer that predates a table refuses it by name, and that must not
/// cost the rounds behind it.
const ROUND_DOCS: [&str; 11] = [
    "devices", "seen", "pins", "roster", "dirpins", "workspaces", "webpins", "recents",
    "agents", "accounts", "titles",
];

/// Whether a failed sync of this document fails the round
/// (`ROUND_DOCS`).
fn doc_must_sync(name: &str) -> bool {
    matches!(name, "devices" | "seen" | "pins") || name.starts_with("chat/")
}

/// A document's version, as a [`Request::Versions`] digest: blake3 of
/// its canonical spelling, 16 bytes.
fn version_digest<D: Doc>(doc: &D) -> Vec<u8> {
    blake3::hash(&wire::version_canonical(doc)).as_bytes()[..16].to_vec()
}

/// A reading's answer frame, as a [`Request::Versions`] digest.
fn reading_digest(frame: &[u8]) -> Vec<u8> {
    blake3::hash(frame).as_bytes()[..16].to_vec()
}

/// The reading an asker names in [`Request::Versions`] to say it takes the
/// sessions reading as a change (`Response::SessionRowsSince`). Asks for
/// nothing: [`reading_request`] has no request for it.
const ROWS_BY_CHANGE: &str = "sessions+";

/// What changed between the rows last sent and these: the ids in order
/// **when the order changed** — empty when it is the one the asker holds
/// — and the rows that differ. `None` when an id repeats (no rebuild could
/// tell them apart), or when every row went (an empty order would read
/// as "the same").
///
/// ⚠️ **The order is not sent when it has not changed** (the term lane's
/// measurement, audit/term-idle-cost.md): every id on a desk of 109 rows
/// is 2.6 KB, and a busy session's change carried them on every round —
/// 33 KB a minute for one row's few hundred bytes.
fn rows_since(was: &[khor_core::Session], now: &[khor_core::Session]) -> Option<(Vec<String>, Vec<khor_core::Session>)> {
    let mut seen = std::collections::HashSet::new();
    if !now.iter().all(|r| seen.insert(r.id.0.as_str())) || (now.is_empty() && !was.is_empty()) {
        return None;
    }
    let same_order = was.len() == now.len() && was.iter().zip(now).all(|(a, b)| a.id == b.id);
    let was: std::collections::HashMap<&str, Vec<u8>> = was
        .iter()
        .filter_map(|r| Some((r.id.0.as_str(), proto::encode(r).ok()?)))
        .collect();
    let changed = now
        .iter()
        .filter(|r| was.get(r.id.0.as_str()).is_none_or(|b| proto::encode(*r).ok().as_ref() != Some(b)))
        .cloned()
        .collect();
    let order = if same_order {
        Vec::new()
    } else {
        now.iter().map(|r| r.id.0.clone()).collect()
    };
    Some((order, changed))
}

/// The rows rebuilt from the copy held and a [`rows_since`] change — in
/// the held order when `order` is empty; `None` when an id is in neither.
fn rows_rebuilt(
    held: &[khor_core::Session],
    order: &[String],
    changed: Vec<khor_core::Session>,
) -> Option<Vec<khor_core::Session>> {
    let held_order: Vec<String>;
    let order = if order.is_empty() {
        held_order = held.iter().map(|r| r.id.0.clone()).collect();
        &held_order
    } else {
        order
    };
    let mut by_id: std::collections::HashMap<String, khor_core::Session> =
        held.iter().map(|r| (r.id.0.clone(), r.clone())).collect();
    for r in changed {
        by_id.insert(r.id.0.clone(), r);
    }
    order.iter().map(|id| by_id.remove(id)).collect()
}

/// The request a reading's kind stands for in [`Request::Versions`].
fn reading_request(kind: &str) -> Option<Request> {
    Some(match kind {
        "sessions" => Request::Sessions,
        "vitals" => Request::Vitals,
        "watched" => Request::Watched,
        "days" => Request::Days,
        _ if kind.starts_with("days@") => Request::Days,
        "quotas" => Request::Quotas,
        "usage" => Request::Usage,
        "workspace_facts" => Request::WorkspaceFacts,
        _ => Request::SatVitals {
            sat: kind.strip_prefix("sat/")?.to_owned(),
        },
    })
}

/// **What a pull's part file holds bytes of** (round NET-4): the
/// machine and path it is pulled from, and the (size, mtime) every slice
/// was checked against. Beside the part file, so a pull of the same file
/// into the same place carries on from it (`Node::pull_path_with`).
#[derive(serde::Serialize, serde::Deserialize)]
struct PullNote {
    machine: String,
    sat: Option<String>,
    path: String,
    total: u64,
    at_ms: u64,
}

impl PullNote {
    fn path_of(part: &std::path::Path) -> std::path::PathBuf {
        let mut name = part.file_name().unwrap_or_default().to_os_string();
        name.push(".json");
        part.with_file_name(name)
    }

    fn read(path: &std::path::Path) -> Option<PullNote> {
        serde_json::from_slice(&fs::read(path).ok()?).ok()
    }

    fn write(&self, path: &std::path::Path) -> Result<(), String> {
        fs::write(path, serde_json::to_vec(self).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
}

/// A drop's progress as [`Response::DropFile`] carries it.
fn drop_answer(s: crate::drops::DropState) -> Response {
    Response::DropFile {
        moved: s.moved,
        total: s.total,
        dest: s.dest,
        done: s.done,
    }
}

/// What a [`Response::Versions`] carries: the documents whose versions
/// differ, with the far version, and the readings that changed.
type VersionsAnswer = (Vec<(String, String)>, Vec<(String, serde_bytes::ByteBuf)>);

/// A [`Response::Versions`], or `None` for a khor that predates the
/// question (it refuses the frame whole).
fn versions_answer(answer: Response) -> Result<Option<VersionsAnswer>, String> {
    match answer {
        Response::Versions { docs, readings } => Ok(Some((docs, readings))),
        Response::Refused { .. } => Ok(None),
        other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
    }
}

/// A reading last got from a machine: its frame's digest, and the answer
/// itself, written into the cache again on a round that says it has not
/// changed — so its age is the last round's, as before.
#[derive(Clone)]
pub(crate) struct HeldReading {
    digest: Vec<u8>,
    answer: Response,
}

/// How often the serve loop syncs with everyone — and, on the same beat,
/// reaps borrows whose rows are gone.
///
/// **Public so a test can wait exactly one of these** rather than hold a
/// number of its own. A test that slept "about long enough" would be
/// carrying a constant only its author could check, and it would quietly
/// stop covering the tick the day this changes.
pub const SYNC_EVERY: Duration = Duration::from_secs(5);

/// How often a machine that keeps refusing rounds is vouched to
/// (`Node::vouch_to`, round NET-10).
const VOUCH_EVERY: Duration = Duration::from_secs(5 * 60);

/// **The pump's beat while no face is on screen** (round NET-6, the
/// owner's ruling through the lead): a round every 30 s instead of every
/// [`SYNC_EVERY`]. Rounds are what keep other machines' rows fresh for a
/// person looking, and while nobody looks a quiet round still cost about
/// 1 KB per machine every 5 s (audit/net-5.md). A face coming on screen
/// gets a round at once (`ipc::Op::FaceHere`), and a pull to refresh is a
/// round at once already (`sync_now`), so a returning face never reads
/// rows thirty seconds old.
pub const QUIET_PACE: Duration = Duration::from_secs(30);

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

/// How far the wall clock may run ahead of the monotonic one between two
/// looks at the wires before it is read as a sleep. Both clocks advance
/// together while the process is merely stopped or busy; only a
/// suspended machine stops the monotonic clock (`CLOCK_UPTIME_RAW` on
/// macOS, `CLOCK_MONOTONIC` on Linux) and not the wall. Five seconds is
/// above any adjustment NTP slews in and below any sleep worth noticing.
const SLEPT_AFTER: Duration = Duration::from_secs(5);

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
                // A peer off the roster is skipped the same way: nothing
                // was dialed, so nothing was heard or not heard.
                Err(why) if why == msg::no_roads_reported() || why == msg::peer_not_in_roster() => {
                    continue;
                }
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
/// What the hearing test found (round NET-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hearing {
    /// A throwaway endpoint reached these ears through their own home
    /// relay: they work, and the silence is the peers' (all off, or
    /// refusing).
    Echoed,
    /// No echo, and a relay answers from this machine: the road out is
    /// up and these ears are not on it. The one case new ears cure.
    Unheard,
    /// No relay answers from here at all: this machine's own network is
    /// down, and new ears would be bound on the same nothing.
    NoRelay,
}

/// How long the echo may take: a relay round trip from either side and
/// a handshake, on the slowest road a relay path has been measured on.
const ECHO_WITHIN: Duration = Duration::from_secs(10);

/// **Whether these ears can be heard at all** (round NET-11), asked only
/// once everyone with a road has been silent for [`DEAF_AFTER`].
///
/// Silence from every peer used to be the whole verdict, and it is also
/// what a machine whose peers are simply off sees: lecun, 2026-10-05,
/// with hinton refusing, the phone away and turing's two serves knocking
/// each other off the relay, exited as deaf every two to three minutes,
/// and each restart wrote its row again with a new port. The owner's Mac
/// app, every peer off overnight, took new ears every two minutes.
///
/// The test is the cure's own observation turned round (批23: 「a fresh
/// endpoint on the same box echoed through the same relay in seconds」):
/// a throwaway key dials this endpoint's id through its own home relay,
/// with no direct address, so only the relay path can carry it. An echo
/// means the ears work; no echo with a relay answering means they do
/// not; no relay answering means the network is down and new ears would
/// not help either.
///
/// `homed_before` is whether these ears ever had a home relay. Ears that
/// never reached one (lecun: no road to the internet from its campus
/// network, its peers all on the LAN) are never deaf: the relay path was
/// never working, so it cannot have stopped, and new ears would only find
/// the same nothing — every 2.5 minutes, all day, on lecun.
pub async fn ears_heard(
    ep: &iroh::Endpoint,
    relay_map: &[khor_net::endpoint::Relay],
    n0: bool,
    relays: &[String],
    homed_before: bool,
) -> Hearing {
    let home: Vec<iroh::RelayUrl> = ep
        .addr()
        .addrs
        .iter()
        .filter_map(|t| match t {
            iroh::TransportAddr::Relay(u) => Some(u.clone()),
            _ => None,
        })
        .collect();
    if !home.is_empty()
        && let Ok(probe) = endpoint::bind_with(iroh::SecretKey::generate(), relay_map, n0).await
    {
        let addr = home
            .into_iter()
            .fold(iroh::EndpointAddr::new(ep.id()), |a, u| a.with_relay_url(u));
        let echoed = match tokio::time::timeout(ECHO_WITHIN, probe.connect(addr, ALPN)).await {
            Ok(Ok(conn)) => {
                conn.close(0u32.into(), b"echo");
                true
            }
            _ => false,
        };
        probe.close().await;
        if echoed {
            return Hearing::Echoed;
        }
    } else if !homed_before {
        return Hearing::NoRelay;
    }
    if crate::doctor::any_relay_answers(relays.to_vec()).await {
        Hearing::Unheard
    } else {
        Hearing::NoRelay
    }
}

fn has_home_relay(ep: &iroh::Endpoint) -> bool {
    ep.addr().addrs.iter().any(|t| matches!(t, iroh::TransportAddr::Relay(_)))
}

/// The shortest and the longest wait between two tries at new ears
/// (round NET-11): 4, 8, 16 … 30 minutes after the first.
const DEAF_WAIT_FIRST: Duration = Duration::from_secs(4 * 60);
const DEAF_WAIT_MOST: Duration = Duration::from_secs(30 * 60);

/// The wait after the `step`-th try in a row.
fn deaf_wait(step: u32) -> Duration {
    DEAF_WAIT_FIRST
        .saturating_mul(1u32 << step.saturating_sub(1).min(8))
        .min(DEAF_WAIT_MOST)
}

impl Node {
    /// Where the tries at new ears are counted: `<tries in a row>
    /// <when the last was, ms>`. A file, because a kept serve's try is
    /// its own exit and the count has to outlive it.
    fn deaf_file(&self) -> std::path::PathBuf {
        self.root().join(".khor").join("deaf-tries")
    }

    fn deaf_record(&self) -> Option<(u32, u64)> {
        let text = std::fs::read_to_string(self.deaf_file()).ok()?;
        let mut it = text.split_whitespace();
        Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    }

    /// Minutes since the last try and minutes left, while a try would
    /// come too soon after it.
    fn deaf_waits(&self, now: u64) -> Option<(u64, u64)> {
        let (step, at) = self.deaf_record()?;
        let until = at.saturating_add(deaf_wait(step).as_millis() as u64);
        (now < until).then(|| (now.saturating_sub(at) / 60_000, (until - now).div_ceil(60_000)))
    }

    fn deaf_tried(&self, now: u64) {
        let step = self.deaf_record().map_or(0, |(s, _)| s).saturating_add(1);
        let _ = std::fs::write(self.deaf_file(), format!("{step} {now}\n"));
    }

    /// A round went through or the ears echoed: the next silence starts
    /// from the beginning, and says why again.
    fn deaf_cleared(&self) {
        if self.deaf_file().exists() {
            let _ = std::fs::remove_file(self.deaf_file());
        }
        *self.deaf_said.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }

    /// One line per reason, not one per beat: lecun's log was the same
    /// two lines every 2.5 minutes, all day.
    fn say_once(&self, why: &'static str, line: String) {
        let mut said = self.deaf_said.lock().unwrap_or_else(|p| p.into_inner());
        if *said != Some(why) {
            *said = Some(why);
            eprintln!("{line}");
        }
    }

    /// Whether any machine reached this one with a request within `span`.
    fn reached_within(&self, span: Duration) -> bool {
        let at = self.reached_at.load(std::sync::atomic::Ordering::Relaxed);
        at != 0 && crate::now_ms().saturating_sub(at) < span.as_millis() as u64
    }
}

// ── fresh ears (docs/IOS.md 第三步) ─────────────────────────────────

/// What a resumable serve is asked from outside its loop.
enum Wake {
    /// New ears, then a round at once. `ears` answers when the new
    /// endpoint is the one answering, `synced` with the round's outcomes;
    /// either may have nobody listening (the deaf watch asks for none).
    Resume {
        ears: Option<tokio::sync::oneshot::Sender<Result<(), String>>>,
        synced: Option<tokio::sync::oneshot::Sender<Vec<(String, Result<String, String>)>>>,
    },
    /// Close the endpoint under the serve and leave it closed ([`deafen`]).
    Deafen {
        done: tokio::sync::oneshot::Sender<()>,
    },
    /// The network under this machine changed, or it woke from sleep
    /// ([`network_changed`]): tell iroh, judge every wire by whether it
    /// is heard right after, and visit everyone once that is known.
    Network,
    /// The app is going to the background ([`suspending`]): let every
    /// wire go now, closed, so the far machines read 未连接 at once
    /// instead of after their silence limit.
    Suspend {
        done: tokio::sync::oneshot::Sender<()>,
    },
}

type Doors = std::sync::Mutex<
    std::collections::HashMap<std::path::PathBuf, tokio::sync::mpsc::UnboundedSender<Wake>>,
>;

/// The resumable serves running in this process, by the home they hold.
///
/// **Process-global and keyed by home**, because what calls [`resume`]
/// is not the code that started the serve: on a phone it is the app's
/// lifecycle handler on the main thread, and the serve is a `Node` that
/// was moved into its own runtime on another thread long before. The
/// home is the one name both sides already have (`Node::root_from_env`).
fn doors() -> &'static Doors {
    static DOORS: std::sync::OnceLock<Doors> = std::sync::OnceLock::new();
    DOORS.get_or_init(Default::default)
}

/// A serve's place in [`doors`], given up when the serve ends — and
/// only its own: a later serve on the same home may have taken it.
struct Listed {
    root: std::path::PathBuf,
    door: tokio::sync::mpsc::UnboundedSender<Wake>,
}

impl Listed {
    fn new(root: &std::path::Path, door: tokio::sync::mpsc::UnboundedSender<Wake>) -> Self {
        doors()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(root.to_path_buf(), door.clone());
        Self {
            root: root.to_path_buf(),
            door,
        }
    }
}

impl Drop for Listed {
    fn drop(&mut self) {
        let mut doors = doors().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if doors.get(&self.root).is_some_and(|d| d.same_channel(&self.door)) {
            doors.remove(&self.root);
        }
    }
}

fn door(root: &std::path::Path) -> Option<tokio::sync::mpsc::UnboundedSender<Wake>> {
    doors()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(root)
        .cloned()
}

/// Why a [`Node::rejoin`] did not reach the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejoin {
    /// The far machine was not reached — a dial that failed, a request
    /// that never came back, the resident's own ears down. Worth asking
    /// again.
    Unreached(String),
    /// The far machine answered, and there is no session to attach to
    /// (it ended, or it left the table). The face's 「结束了」 is true.
    Gone(String),
}

/// A [`resume`] under way: first the new ears, then the round.
pub struct Resuming {
    ears: Option<tokio::sync::oneshot::Receiver<Result<(), String>>>,
    synced: tokio::sync::oneshot::Receiver<Vec<(String, Result<String, String>)>>,
}

impl Resuming {
    /// When the new endpoint is the one answering — the moment a face
    /// attached through the old one can reach again ([`Node::rejoin`]).
    /// Answers once; a second call answers `Ok` at once.
    pub async fn ears(&mut self) -> Result<(), String> {
        match self.ears.take() {
            Some(rx) => rx.await.unwrap_or_else(|_| Err(msg::serve_gone_midway().into())),
            None => Ok(()),
        }
    }

    /// The round that followed, per machine — the same outcomes a
    /// `khor sync` prints.
    pub async fn synced(mut self) -> Result<Vec<(String, Result<String, String>)>, String> {
        self.ears().await?;
        self.synced
            .await
            .map_err(|_| msg::serve_gone_midway().to_string())
    }
}

/// **Fresh ears for the resident on `root`**, in this process, without
/// a restart (docs/IOS.md 第三步): a new endpoint on the same key, the
/// old one and what rode it let go, the new roads published, and a sync
/// round at once. `None` when no resumable serve holds that home here.
///
/// The phone's app calls this on its way back to the foreground
/// (`applicationWillEnterForeground`); a laptop waking from sleep could
/// call it the same way, and nothing here is a phone's.
pub fn resume(root: &std::path::Path) -> Option<Resuming> {
    let (ears, ears_rx) = tokio::sync::oneshot::channel();
    let (synced, synced_rx) = tokio::sync::oneshot::channel();
    door(root)?
        .send(Wake::Resume {
            ears: Some(ears),
            synced: Some(synced),
        })
        .ok()?;
    Some(Resuming {
        ears: Some(ears_rx),
        synced: synced_rx,
    })
}

/// **The network under this machine changed, or it woke from sleep**:
/// the resident on `root` tells iroh (`Endpoint::network_change`), keeps
/// every wire that is heard within [`wires::AFTER_CHANGE`] and retires
/// the rest, then visits every machine. Answers whether a resident here
/// took it. The Mac app calls this on `NSWorkspaceDidWakeNotification`,
/// the phone on a path change; the resident also notices a sleep on its
/// own, by its wall clock jumping past its monotonic one.
pub fn network_changed(root: &std::path::Path) -> bool {
    door(root).is_some_and(|d| d.send(Wake::Network).is_ok())
}

/// **The app is going to the background**: the resident on `root` lets
/// every wire go, closed — the far machines learn at once rather than
/// after [`wires::SILENT_AFTER`], and nothing is left half-open for the
/// system to reclaim under it. The endpoint stays; [`resume`] gives it
/// new ears on the way back. Answers once the wires are let go.
pub async fn suspending(root: &std::path::Path) -> bool {
    let (done, done_rx) = tokio::sync::oneshot::channel();
    let Some(door) = door(root) else { return false };
    if door.send(Wake::Suspend { done }).is_err() {
        return false;
    }
    done_rx.await.is_ok()
}

/// Closes the endpoint under the resumable serve on `root` and leaves
/// it closed — **what a suspension does to a phone's ears**, done on
/// purpose so a test can prove [`resume`] cures it (`tests/resume.rs`).
/// Answers whether there was such a serve. Not for anything else: a
/// serve deafened this way hears nobody until it is resumed or its deaf
/// watch runs out.
#[doc(hidden)]
pub async fn deafen(root: &std::path::Path) -> bool {
    let (done, done_rx) = tokio::sync::oneshot::channel();
    let Some(door) = door(root) else { return false };
    if door.send(Wake::Deafen { done }).is_err() {
        return false;
    }
    done_rx.await.is_ok()
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

/// **Slices asked at once** when a file is pulled whole — a transfer, a
/// preview (round NET-2, piece 6). One at a time, a pull moved one
/// `SLICE` per round trip: 256 KiB per 300 ms is 0.85 MB/s however wide
/// the road. Eight is 2 MiB in flight, about one round trip's worth of a
/// 50 Mbit/s road at 300 ms, and well inside the 32 streams a far
/// machine answers at once (`STREAMS_AT_ONCE`).
const PULL_AHEAD: u64 = 8;

/// How often a drop's asker asks how far it got (`Node::drop_file`):
/// often enough for a progress line to move, and each ask is one small
/// frame.
pub const DROP_ASK_EVERY: Duration = Duration::from_millis(250);

/// How often a routed add's asker reads the run again
/// ([`Node::add_on`]): a step is seconds long, a mark is a line.
pub const ADD_ASK_EVERY: Duration = Duration::from_millis(500);

/// How often a drop's asker looks whether it was cancelled between its
/// asks (`Node::drop_file_until`, item 71): the order to stop goes out
/// within this of the ×, not a whole ask later.
const DROP_STOP_LOOK: Duration = Duration::from_millis(50);

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

/// How many requests from one connection are answered at once. Enough
/// that a slow one (a search, a big listing, a satellite's ssh) never
/// holds a quick one behind it; a bound so one machine asking without
/// pause cannot take every thread this one has. QUIC's own stream limit
/// (100 open bidirectional streams) is what holds the asker beyond it.
const STREAMS_AT_ONCE: usize = 32;

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
    /// The resident is an app (a resumable serve — the desktop app or the
    /// phone), not a kept `khor serve`. `khor quit` stops a kept serve; it
    /// never signals an app, whose window a person closes themselves.
    #[serde(default)]
    app: bool,
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
        self.serve_as(false).await
    }

    /// [`Self::serve`], for a resident that is **resumed rather than
    /// restarted** — a phone's (docs/IOS.md 第三步).
    ///
    /// iOS suspends an app seconds after it leaves the screen, and what
    /// it holds goes deaf: the relay's connection, the peers' QUIC
    /// connections, on a real device the sockets themselves. A desktop
    /// serve that went deaf exits ([`DEAF_EXIT`]) and its keeper stands
    /// up a new one; **nothing restarts a phone app**, and the system
    /// reads an exit as a crash. So this serve gets new ears in place,
    /// from [`resume`] (the app calls it on its way back to the
    /// foreground) or from its own deaf watch, and differs from `serve`
    /// in exactly these four ways, none of which a desktop sees:
    ///
    /// 1. it answers [`resume`]: a fresh endpoint on the same key, the
    ///    old one closed and everything held on it forgotten, the new
    ///    roads published, and a sync round **at once** rather than at
    ///    the next tick ([`Self::fresh_ears`]);
    /// 2. two minutes of deafness ask for the same, instead of
    ///    [`DEAF_EXIT`];
    /// 3. an endpoint that closes underneath it leaves it deaf, waiting
    ///    for one of those two, instead of ending it;
    /// 4. a hand-off listener that stops accepting (iOS reclaims a
    ///    listening socket of a suspended app) is bound again, instead of
    ///    being asked again in a loop that never yields.
    pub async fn serve_resumable(self) -> Result<(), String> {
        self.serve_as(true).await
    }

    /// The one bind that serves this machine: this key, these relays.
    /// Also what [`Self::fresh_ears`] binds again.
    async fn bind_ears(&self) -> Result<std::sync::Arc<iroh::Endpoint>, String> {
        let preferred = self.preferred_relay();
        let bind = |ports| {
            endpoint::bind_pinned(
                self.secret_key().clone(),
                self.relay_map(),
                self.relays_n0(),
                preferred.as_deref(),
                ports,
                self.carrying(),
            )
        };
        // **The ports these ears had last time** (round NET-11), so a
        // restart leaves this machine's row as it was. Taken — by the old
        // ears still closing, or by somebody else — any port, as before.
        let kept = self.kept_ports();
        let ep = match kept {
            Some(ports) => match bind(ports).await {
                Ok(ep) => ep,
                Err(_) => bind((0, 0)).await.map_err(|e| e.to_string())?,
            },
            None => bind((0, 0)).await.map_err(|e| e.to_string())?,
        };
        self.keep_ports(&ep);
        Ok(std::sync::Arc::new(ep))
    }

    /// `.khor/ports`: `<v4> <v6>`, the UDP ports the last ears bound.
    fn ports_file(&self) -> std::path::PathBuf {
        self.root().join(".khor").join("ports")
    }

    fn kept_ports(&self) -> Option<(u16, u16)> {
        let text = fs::read_to_string(self.ports_file()).ok()?;
        let mut it = text.split_whitespace();
        Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    }

    fn keep_ports(&self, ep: &iroh::Endpoint) {
        let (mut v4, mut v6) = (0, 0);
        for s in ep.bound_sockets() {
            match s {
                std::net::SocketAddr::V4(a) => v4 = a.port(),
                std::net::SocketAddr::V6(a) => v6 = a.port(),
            }
        }
        if v4 != 0 && self.kept_ports() != Some((v4, v6)) {
            let _ = fs::write(self.ports_file(), format!("{v4} {v6}\n"));
        }
    }

    /// The endpoint the resident answers on from now (`Node::ears`).
    fn set_ears(&self, ep: &std::sync::Arc<iroh::Endpoint>) {
        *self.ears.lock().unwrap_or_else(|p| p.into_inner()) = Some(ep.clone());
        // New ears have homed nowhere yet (round NET-11, `ears_heard`).
        self.ears_homed.store(false, std::sync::atomic::Ordering::Relaxed);
        // **What the relays say about who left** (round NET-5,
        // `wires::Wires::departed`), for as long as these ears live: the
        // subscription ends when the endpoint does.
        let mut left = ep.relay_departures();
        let wires = self.wires.clone();
        tokio::spawn(async move {
            loop {
                match left.recv().await {
                    Ok(id) => wires.departed(&id.to_string()),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    /// The endpoint the resident answers on now, for a task that outlives
    /// the call that started it. `None` on a node that is not serving.
    pub(crate) fn current_ears(&self) -> Option<std::sync::Arc<iroh::Endpoint>> {
        self.ears.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// The resident's own handle, when this node is the one serving.
    pub(crate) fn me(&self) -> Option<std::sync::Arc<Node>> {
        self.me.get().and_then(std::sync::Weak::upgrade)
    }

    /// Own dialing hints go into the table so the snapshot handed to a
    /// pairing device already says how to reach us — relay roads
    /// included, or a peer behind a hostile network has no way in.
    /// Written again after fresh ears: a new endpoint has new ports.
    fn publish_roads(&self, ep: &iroh::Endpoint) -> Result<Vec<String>, String> {
        let addrs = endpoint::local_addrs(ep);
        let mut roads = addrs.clone();
        roads.extend(self.relays().iter().cloned());
        let loaded = self.devices_loaded()?;
        loaded.doc.upsert(self.device_str(), self.name(), &roads)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(addrs)
    }

    /// `endpoint.json`: where one-shot verbs find the live endpoint and
    /// the hand-off port. Written again whenever either changes.
    fn write_endpoint_file(
        &self,
        addrs: Vec<String>,
        handoffs: &tokio::net::TcpListener,
        cookie: &str,
    ) -> Result<(), String> {
        let file = EndpointFile {
            id: self.device_str().to_owned(),
            addrs,
            pid: std::process::id(),
            relays: self.relays().to_vec(),
            ipc_port: handoffs.local_addr().map_err(|e| e.to_string())?.port(),
            ipc_cookie: cookie.to_owned(),
            app: self.app_resident.load(std::sync::atomic::Ordering::Relaxed),
        };
        write_private(
            &self.root().join(".khor").join("endpoint.json"),
            &serde_json::to_vec(&file).map_err(|e| e.to_string())?,
        )
    }

    /// **Fresh ears**: a new endpoint on the same key, and everything
    /// this node held on the old one let go (docs/IOS.md 第三步).
    ///
    /// Rebinding rather than asking iroh to notice: `network_change` only
    /// acts when the interfaces differ, and a phone back from its pocket
    /// is usually on the very network it left — with a relay connection
    /// and peers' QUIC connections that died while it was frozen, and on
    /// a real device sockets the system took back. A new endpoint is the
    /// cure the desktop's [`DEAF_EXIT`] buys with a restart, without one.
    ///
    /// What is let go, because it rode the old endpoint: the held
    /// connections (`wires`) and the pipes in front of far sessions and
    /// far editors (`reaches`, `codes`). A face attached through one of
    /// those pipes sees its socket end and attaches again through
    /// [`Self::rejoin`] — the same reach, on the new endpoint, and the
    /// far host pours its ring to a fresh client as it always does.
    /// Hosted borrows are left alone: a phone has none (docs/IOS.md 第二步).
    async fn fresh_ears(&self) -> Result<std::sync::Arc<iroh::Endpoint>, String> {
        let ep = tokio::time::timeout(DIAL_TIMEOUT, self.bind_ears())
            .await
            .map_err(|_| msg::dial_timed_out().to_string())??;
        self.wires.clear(wires::Retired::NewEars);
        for (_, (_, _, handle)) in self.reaches.lock().await.drain() {
            handle.abort();
        }
        for (_, (_, _, handle)) in self.codes.lock().await.drain() {
            handle.abort();
        }
        Ok(ep)
    }

    async fn serve_as(self, resumable: bool) -> Result<(), String> {
        self.app_resident
            .store(resumable, std::sync::atomic::Ordering::Relaxed);
        let mut ep = self.bind_ears().await?;
        let addrs = self.publish_roads(&ep)?;
        let mut handoffs = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(msg::cant_open_handoff_port)?;
        let cookie = fresh_hex()?;
        self.write_endpoint_file(addrs, &handoffs, &cookie)?;

        // Borrow rows left by a previous serve point at ports this
        // process does not hold; clear them before answering anyone.
        self.sweep_stale_borrows();

        // The trust beat, once before answering anyone: a holder whose
        // roster aged while it was off signs it again now, and a network
        // from before roots takes one up if it has arrived. Best effort
        // (`Node::trust_tick`).
        let _ = self.trust_tick();

        // Watch this runtime for a handler that blocks it (#87 / #73:
        // `crate::reactor` has why a wall clock could not tell that
        // from an ordinary wait). Started here rather than at open:
        // there is no reactor to stall until there is a serve.
        crate::reactor::watch();

        // This machine's readings, taken in the background from here on
        // (docs/SHELL.md 第十六轮): a peer's ask, the app's device table
        // and the CPU day all read what it last wrote down.
        crate::vitals::local::start(self.root());

        // One task per connection: a client that vanishes without
        // closing must not block the accept loop until QUIC times out.
        let node = std::sync::Arc::new(self);
        // The resident's handle on itself and on its current ears, for
        // the tasks it spawns that outlive one call (`Node::me`).
        let _ = node.me.set(std::sync::Arc::downgrade(&node));
        node.set_ears(&ep);
        // Conversation files that arrived while no resident ran start now.
        node.take_arrived();
        // The satellite sampler (附属机器批): its own task on its own
        // minute beat — a sample is an ssh dial, and the 5s pump must
        // not inherit that weight.
        {
            let n = node.clone();
            tokio::spawn(async move { n.satellite_sampler().await });
        }
        // The carrier (round ROAD-2, khor inside ssh): an ssh session up to
        // every member this machine carries.
        if node.carry_on() && node.carrying().is_some() {
            let n = node.clone();
            tokio::spawn(async move { n.carrier().await });
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
                // A process that was stopped (a phone in the background:
                // `tokio`'s clock keeps running while nothing here does)
                // owes one read on its way back, not one per half second
                // it was away — the default would fire them all at once.
                beat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
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
        // [`resume`]'s door into this loop, and the deaf watch's own. Only
        // a resumable serve is listed where [`resume`] looks; the sender
        // kept here is also how the watch asks this loop for new ears,
        // and how the wire watch below reports a wake.
        let (wake, mut woken) = tokio::sync::mpsc::unbounded_channel::<Wake>();
        let _listed = resumable.then(|| Listed::new(node.root(), wake.clone()));
        // **The wire watch** (`crate::wires`): once a second, what each
        // wire has heard, which road it is on, and the silent ones
        // retired. The same beat notices a sleep: the monotonic clock
        // stops while a Mac or a Linux box is suspended and the wall clock
        // does not, so a wall clock that ran ahead of it is a wake —
        // treated like a network change, because after a sleep it often
        // is one, and every wire has to prove it is still alive either way.
        {
            let n = node.clone();
            let wake = wake.clone();
            tokio::spawn(async move {
                let mut beat = tokio::time::interval(wires::TICK);
                beat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                let mut mono = std::time::Instant::now();
                let mut wall = std::time::SystemTime::now();
                loop {
                    beat.tick().await;
                    n.wires.tick();
                    let ran = mono.elapsed();
                    let walked = wall.elapsed().unwrap_or_default();
                    if walked > ran + SLEPT_AFTER {
                        let _ = wake.send(Wake::Network);
                    }
                    mono = std::time::Instant::now();
                    wall = std::time::SystemTime::now();
                }
            });
        }
        let mut ticker = tokio::time::interval(SYNC_EVERY);
        // When the pump last ran a round, for the quiet pace.
        let mut last_round: Option<std::time::Instant> = None;
        // This machine's workspace facts, kept from here on (round NET-8).
        // A phone hosts no workspace.
        if node.hosts() {
            *node.wsfacts.lock().unwrap_or_else(|p| p.into_inner()) =
                Some(crate::wsfacts::WsFacts::start(node.root(), node.device_str()));
        }
        // One pump on the way back from a stop, not one per five seconds
        // away (the pause beat above has the reason).
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Whether the endpoint is still one to accept on: an endpoint
        // that closed answers `None` at once and forever, so a resumable
        // serve stops asking it until it has a new one.
        let mut hearing = true;
        // **Gone to the background** (`suspending`) until `resume`: no
        // pump and no visits. iOS gives an app some seconds before it
        // freezes it, and a pump left running in them redialled every
        // machine the suspension had just let go (round NET-5: 2 dials
        // and 42 KB in 20 s), on wires the freeze then left half-open.
        let resting = node.resting.clone();
        // A resume in flight, and everyone waiting on it; the bind's
        // result comes back to this loop on `bound`.
        type Waiting = (
            Option<tokio::sync::oneshot::Sender<Result<(), String>>>,
            Option<tokio::sync::oneshot::Sender<Vec<(String, Result<String, String>)>>>,
        );
        let mut resuming: Option<Vec<Waiting>> = None;
        let (bound_tx, mut bound_rx) = tokio::sync::mpsc::unbounded_channel::<(
            Result<std::sync::Arc<iroh::Endpoint>, String>,
            std::time::Instant,
        )>();
        loop {
            tokio::select! {
                incoming = ep.accept(), if hearing => {
                    let Some(incoming) = incoming else {
                        // The endpoint closed under this serve. A serve
                        // somebody restarts ends here, as it always has;
                        // a resumable one stays deaf until `resume` or its
                        // own watch gives it new ears.
                        if !resumable { break }
                        hearing = false;
                        let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                        eprintln!("{}", msg::serve_ears_closed(&stamp));
                        continue;
                    };
                    // Paused: dropped without an answer, so the far side
                    // sees what a machine that is not there looks like —
                    // which is the state somebody asked for.
                    if paused.load(std::sync::atomic::Ordering::Relaxed) { continue }
                    if resting.load(std::sync::atomic::Ordering::Relaxed) { continue }
                    let n = node.clone();
                    let e = ep.clone();
                    let resting = resting.clone();
                    tokio::spawn(async move {
                        let Ok(accepting) = incoming.accept() else { return };
                        // **Early data is taken** (round NET-3): a machine
                        // redialling with a ticket from an earlier wire
                        // sends its first request inside the handshake,
                        // and a reading is answered at once — one round
                        // trip instead of two. What may not ride early is
                        // held until the handshake completes
                        // (`Request::rides_early`, in `handle`).
                        let early = accepting.into_0rtt();
                        let (done_tx, done) = tokio::sync::watch::channel(None::<bool>);
                        // **Completed means open.** iroh's
                        // `handshake_completed` on an incoming early
                        // connection also resolves `Ok` when the connection
                        // is lost before the handshake ever finished (it
                        // drops noq's accepted flag) — measured: a client
                        // that never finished resolved it the moment the
                        // connection closed. So a connection already closed
                        // when it resolves did not complete, and what waits
                        // for the handshake is dropped.
                        let completed = {
                            let early = early.clone();
                            async move {
                                early
                                    .handshake_completed()
                                    .await
                                    .ok()
                                    .filter(|conn| conn.close_reason().is_none())
                            }
                        };
                        let once_done = move |n: std::sync::Arc<Node>, e: std::sync::Arc<iroh::Endpoint>, conn: &iroh::endpoint::Connection| {
                            n.wires.serving(conn);
                            if resting.load(std::sync::atomic::Ordering::Relaxed) {
                                return;
                            }
                            // **A machine that dials in is back**: one this
                            // node holds no wire to — a phone returning to
                            // the front, a laptop waking — is visited now,
                            // not at the next beat or after its backoff.
                            let remote = conn.remote_id().to_string();
                            if conn.alpn() == endpoint::ALPN
                                && n.wires.live(&remote, wires::Proto::Main).is_none()
                            {
                                tokio::spawn(async move { n.visit_now(&e, &remote).await });
                            }
                        };
                        let early_from = early
                            .remote_id()
                            .ok()
                            .filter(|_| early.alpn().as_deref() == Some(endpoint::ALPN));
                        let Some(remote) = early_from else {
                            // A tunnel, or a machine not known until the
                            // handshake says who it is: after it, as before.
                            let Some(conn) = completed.await else { return };
                            once_done(n.clone(), e.clone(), &conn);
                            let remote = conn.remote_id().to_string();
                            let _ = done_tx.send(Some(true));
                            if conn.alpn() == endpoint::TUNNEL_ALPN {
                                let _ = n.serve_tunnel(conn, e).await;
                            } else {
                                let _ = n.handle(conn, remote, e, done).await;
                            }
                            return;
                        };
                        {
                            let (n, e) = (n.clone(), e.clone());
                            tokio::spawn(async move {
                                let conn = completed.await;
                                let _ = done_tx.send(Some(conn.is_some()));
                                if let Some(conn) = conn {
                                    once_done(n, e, &conn);
                                }
                            });
                        }
                        let _ = n.handle(early, remote.to_string(), e, done).await;
                    });
                }
                accepted = handoffs.accept() => {
                    match accepted {
                        Ok((stream, _)) => {
                            let n = node.clone();
                            let e = ep.clone();
                            let c = cookie.clone();
                            tokio::spawn(async move {
                                let _ = n.handle_handoff(stream, &e, &c).await;
                            });
                        }
                        // **A listener iOS took back.** A suspended app's
                        // listening socket can come back defunct, and then
                        // every accept fails at once: asked again in this
                        // loop, it is a core spinning on an error. Bound
                        // again on a fresh port, written where the verbs
                        // look, and given a beat so a failing bind cannot
                        // spin either.
                        Err(_) if resumable => {
                            if let Ok(fresh) = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await {
                                handoffs = fresh;
                                let _ = node.write_endpoint_file(endpoint::local_addrs(&ep), &handoffs, &cookie);
                            }
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        Err(_) => {}
                    }
                }
                Some(asked) = woken.recv() => match asked {
                    // The test's stand-in for what a suspension does to a
                    // phone's endpoint: closed under the serve, nothing
                    // heard, nothing sent (`deafen`).
                    Wake::Deafen { done } => {
                        ep.close().await;
                        hearing = false;
                        let _ = done.send(());
                    }
                    Wake::Suspend { done } => {
                        resting.store(true, std::sync::atomic::Ordering::Relaxed);
                        node.wires.clear(wires::Retired::Suspended);
                        node.wires.let_go_served(wires::Retired::Suspended);
                        let _ = done.send(());
                    }
                    // A network change while backgrounded is the resume's
                    // to deal with: it gives new ears and a round.
                    Wake::Network if resting.load(std::sync::atomic::Ordering::Relaxed) => {}
                    Wake::Network => {
                        node.wires.network_changed();
                        let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                        eprintln!("{}", msg::serve_network_changed(&stamp));
                        let n = node.clone();
                        let e = ep.clone();
                        let deaf = deaf.clone();
                        tokio::spawn(async move {
                            e.network_change().await;
                            // Once the wires that are going to answer have
                            // answered, and the rest are retired: a visit
                            // to everyone, the backed-off included — the
                            // network they failed on is not this one.
                            // The judging is per wire on its own patience;
                            // the floor is when the fast ones are known.
                            tokio::time::sleep(wires::Patience::for_rtt(Duration::ZERO).after_change + wires::TICK).await;
                            let out = n.sync_peers(&e, true).await;
                            let heard = if n.self_admitted() { out.as_slice() } else { &[] };
                            let _ = deaf.lock().expect("deaf watch poisoned").observe(heard);
                        });
                    }
                    Wake::Resume { ears, synced } => {
                        resting.store(false, std::sync::atomic::Ordering::Relaxed);
                        // **Off this loop**: binding waits on the network,
                        // and the arms above — the app's own status
                        // questions among them — must not wait behind it.
                        // The old ears keep answering meanwhile. A second
                        // resume while one is binding joins it: two binds
                        // on one key knock each other off the relay.
                        if let Some(waiting) = resuming.as_mut() {
                            waiting.push((ears, synced));
                            continue;
                        }
                        resuming = Some(vec![(ears, synced)]);
                        let n = node.clone();
                        let back = bound_tx.clone();
                        tokio::spawn(async move {
                            let begun = std::time::Instant::now();
                            let fresh = n.fresh_ears().await;
                            let _ = back.send((fresh, begun));
                        });
                    }
                },
                Some((fresh, begun)) = bound_rx.recv() => {
                    let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                    let waiting = resuming.take().unwrap_or_default();
                    let mut synced_to = Vec::new();
                    match fresh {
                        Ok(fresh) => {
                            let old = std::mem::replace(&mut ep, fresh);
                            node.set_ears(&ep);
                            hearing = true;
                            // Closed off the loop: a graceful close waits
                            // on its connections, and the new ears are
                            // already the ones answering.
                            tokio::spawn(async move { old.close().await });
                            match node.publish_roads(&ep) {
                                Ok(addrs) => {
                                    let _ = node.write_endpoint_file(addrs, &handoffs, &cookie);
                                }
                                Err(e) => eprintln!("{e}"),
                            }
                            // The round below is this interval's.
                            ticker.reset();
                            eprintln!(
                                "{}",
                                msg::serve_fresh_ears(&stamp, begun.elapsed().as_millis())
                            );
                            for (ears, synced) in waiting {
                                if let Some(ears) = ears {
                                    let _ = ears.send(Ok(()));
                                }
                                synced_to.extend(synced);
                            }
                        }
                        Err(why) => {
                            eprintln!("{}", msg::serve_fresh_ears_failed(&stamp, &why));
                            for (ears, synced) in waiting {
                                if let Some(ears) = ears {
                                    let _ = ears.send(Err(why.clone()));
                                }
                                synced_to.extend(synced);
                            }
                        }
                    }
                    // **A round at once**, not at the next tick: what the
                    // others wrote while this one was away is the first
                    // thing a person coming back looks at. Every machine,
                    // the backed-off included: the network they failed on
                    // may not be this one.
                    let n = node.clone();
                    let e = ep.clone();
                    let deaf = deaf.clone();
                    tokio::spawn(async move {
                        let out = n.sync_peers(&e, true).await;
                        let _ = n.trust_tick();
                        let heard = if n.self_admitted() { out.as_slice() } else { &[] };
                        let _ = deaf.lock().expect("deaf watch poisoned").observe(heard);
                        for synced in synced_to {
                            let _ = synced.send(out.clone());
                        }
                    });
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
                    // ⚠️ **Never in an app** (round NET-FIX): an app run
                    // from a shell inside a khor terminal inherits the
                    // kept serve's marker, and once that shell went away
                    // its parent was launchd — so the window exited on
                    // its own, with the sentence on a terminal nobody
                    // had any more.
                    if !resumable && crate::keeper::is_inner() && unsafe { libc::getppid() } == 1 {
                        let stamp =
                            jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                        eprintln!("{}", msg::serve_orphaned(&stamp));
                        std::process::exit(1);
                    }
                    // Paused: no pump, and **no hearing test either**.
                    // The deaf watch concludes deafness when everyone
                    // with a road has been silent for DEAF_AFTER, which
                    // is exactly what a pause looks like from in here —
                    // left running, it would replace the ears of a
                    // machine somebody deliberately quieted, over and
                    // over. The local handoff arm stays open: a paused
                    // machine is still a machine its own faces talk to.
                    if paused.load(std::sync::atomic::Ordering::Relaxed) { continue }
                    // **No face on screen: a round every QUIET_PACE**
                    // (round NET-6). The tidying below still runs every
                    // beat; only the visits wait.
                    let round = node.face_here()
                        || last_round.is_none_or(|at| at.elapsed() >= QUIET_PACE - SYNC_EVERY / 2);
                    if round {
                        last_round = Some(std::time::Instant::now());
                        node.wires.counts.pump_rounds.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    let n = node.clone();
                    let e = ep.clone();
                    let deaf = deaf.clone();
                    let wake = wake.clone();
                    tokio::spawn(async move {
                        // Reap borrows whose rows were closed since the
                        // last tick, freeing their ports, before syncing.
                        n.reap_borrows().await;
                        // And bridge hosts whose tmux session now lists
                        // through its own agent row (the fold in
                        // `LiveKind::rows` hides them; this stops them).
                        n.live.reap_folded_bridges();
                        // The workspaces this machine keeps facts for:
                        // the table's, as of this beat (round NET-8).
                        n.reconcile_workspace_facts();
                        // **Tidy up before the visits start**, at the one
                        // moment of a beat when no round is likely to be
                        // running: compaction rewrites a document, so it
                        // waits for the store to be free of rounds and
                        // never makes one wait (`try_write`). An ordinary
                        // tick does nothing here at all — the scan is a
                        // handful of `read_dir`s, and only a document past
                        // `store::PILE` is rewritten.
                        //
                        // ⚠️ **The resident serve is the only process
                        // that may do this** (`Node::compact_piles`).
                        if let Ok(store) = n.store_gate.try_write() {
                            let c = n.clone();
                            let done = tokio::task::spawn_blocking(move || c.compact_piles())
                                .await
                                .unwrap_or_default();
                            drop(store);
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
                        }
                        // What this machine's roads are, written into the
                        // table only when one changed and held (round
                        // NET-12, `roadbook`). Not while backgrounded: a
                        // phone that let its wires go has not lost its roads.
                        if !n.resting.load(std::sync::atomic::Ordering::Relaxed) {
                            let book = n.clone();
                            let _ = tokio::task::spawn_blocking(move || book.note_roads()).await;
                        }
                        if !round { return }
                        // Every machine at once, each on its own wire and
                        // budget; one still in a round from an earlier
                        // beat is joined rather than visited twice, and
                        // one backing off after failed dials is left alone
                        // until its wait is over (`Node::sync_peers`).
                        let out = n.sync_peers(&e, false).await;
                        // …and the trust beat on the same visit: the
                        // roster that just arrived may name this machine,
                        // and the holder's may be due for a new date.
                        let _ = n.trust_tick();
                        // The serve's own hearing test: a long-lived
                        // endpoint can go silently, bidirectionally
                        // unreachable while the process looks healthy
                        // (批23). Every observed cure was new ears, so
                        // when everyone with a road has been dead for
                        // DEAF_AFTER straight, prescribe them.
                        // A machine waiting for approval (or revoked) is
                        // refused by everyone on purpose; that silence is
                        // an answer, not deafness, and replacing ears over
                        // it every two minutes would fix nothing.
                        let heard = if n.self_admitted() { out.as_slice() } else { &[] };
                        // A round that went through clears the backoff and
                        // the once-said reason: the next silence is new.
                        if heard.iter().any(|(_, r)| r.is_ok()) {
                            n.deaf_cleared();
                        }
                        if has_home_relay(&e) {
                            n.ears_homed.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                        if deaf.lock().expect("deaf watch poisoned").observe(heard) {
                            // Claimed before the test, so the beats behind
                            // this one do not ask again; the next look is
                            // another DEAF_AFTER of silence away.
                            *deaf.lock().expect("deaf watch poisoned") = DeafWatch::new();
                            let stamp =
                                jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                            // **Silence is not deafness** (round NET-11).
                            // Not while any machine has reached this one
                            // lately; not while these ears echo through
                            // their relay; not on a machine whose relay
                            // path never worked, or none answers now.
                            if n.reached_within(DEAF_AFTER) {
                                n.say_once("reached", msg::serve_silent_reached(&stamp, DEAF_AFTER.as_secs()));
                                return;
                            }
                            let homed = n.ears_homed.load(std::sync::atomic::Ordering::Relaxed);
                            match ears_heard(&e, n.relay_map(), n.relays_n0(), n.relays(), homed).await {
                                Hearing::Echoed => {
                                    n.deaf_cleared();
                                    n.say_once("echoed", msg::serve_silent_echoed(&stamp));
                                    return;
                                }
                                Hearing::NoRelay => {
                                    n.say_once("no-relay", msg::serve_silent_no_relay(&stamp));
                                    return;
                                }
                                Hearing::Unheard => {}
                            }
                            // **And then not every two minutes**: 4, 8, 16,
                            // 30 minutes between tries, kept on disk so a
                            // kept serve's restart does not reset it.
                            let now = crate::now_ms();
                            if let Some((ago, left)) = n.deaf_waits(now) {
                                n.say_once("waits", msg::serve_deaf_waits(&stamp, ago, left));
                                return;
                            }
                            n.deaf_tried(now);
                            // **New ears in place**, for every resumable
                            // serve — the phone, and the app on a desktop
                            // too (round NET-FIX: the Mac app used to exit
                            // here, taking its window with it). Only a
                            // serve with a keeper behind it still exits,
                            // and the keeper stands up a fresh process.
                            if resumable {
                                eprintln!("{}", msg::serve_deaf_fresh_ears(&stamp, DEAF_AFTER.as_secs()));
                                *deaf.lock().expect("deaf watch poisoned") = DeafWatch::new();
                                let _ = wake.send(Wake::Resume { ears: None, synced: None });
                            } else {
                                eprintln!("{}", msg::serve_gone_deaf(&stamp, DEAF_AFTER.as_secs()));
                                std::process::exit(DEAF_EXIT);
                            }
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
                self.wires
                    .counts
                    .ops_late
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
            ipc::Op::Hops => ipc::Reply::Hops {
                by_device: self.links_now(),
            },
            ipc::Op::Roads => ipc::Reply::Roads {
                by_device: crate::roads::tally(&self.roads),
            },
            ipc::Op::Wires => ipc::Reply::Wires {
                counts: self.wires.counts_now(),
            },
            ipc::Op::FaceHere { visible } => {
                let was = self.face_here();
                *self.face_seen.lock().unwrap_or_else(|p| p.into_inner()) =
                    visible.then(std::time::Instant::now);
                // A face arriving gets the round it would otherwise wait up
                // to QUIET_PACE for.
                if visible && !was {
                    self.nudge(ep);
                }
                ipc::Reply::Noted
            }
            ipc::Op::LetGo { machine } => match self.resolve(&machine) {
                Ok((_, home)) => {
                    self.wires.let_go(&home.hex());
                    ipc::Reply::Wires {
                        counts: self.wires.counts_now(),
                    }
                }
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::Connections => ipc::Reply::Connections {
                view: self.connections_now(ep).await,
            },
            // **Never queues behind a pump already running**: a machine
            // in a round is joined, not visited twice, and every other
            // machine is visited now, in parallel, the backed-off ones
            // included — somebody asked (`Node::sync_peers`).
            ipc::Op::SyncNow => ipc::Reply::Synced {
                outcomes: self.sync_peers(ep, true).await,
            },
            ipc::Op::Take { session, action } => {
                match self.take_with(ep, &crate::SessionId(session), &action).await {
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
                token,
            } => {
                match self
                    .open_on_with(ep, &machine, &kind, &title, &cwd, &cmd, (cols, rows), token.as_deref())
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
            ipc::Op::Rejoin { machine, session } => {
                match self
                    .reach_or_why(ep, &machine, &crate::SessionId(session), true)
                    .await
                {
                    Ok((addr, cookie)) => ipc::Reply::Reaching {
                        addr: addr.to_string(),
                        cookie,
                    },
                    Err(Rejoin::Unreached(why)) => ipc::Reply::Unreached { why },
                    Err(Rejoin::Gone(why)) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::OpenAgentOn {
                machine,
                dir,
                title,
                form,
                agent,
                token,
            } => {
                match self
                    .open_agent_on_with(ep, &machine, &dir, &title, &form, &agent, token.as_deref())
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
            ipc::Op::CheckoutOn { machine, path } => {
                match self.checkout_with(ep, &machine, &path).await {
                    Ok(yes) => ipc::Reply::Checkout { yes },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::RemoveOn { machine, path } => {
                match self.remove_with(ep, &machine, &path).await {
                    Ok(()) => ipc::Reply::Removed,
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::WatchOn {
                machine,
                dirs,
                since,
                wait_ms,
            } => match self
                .watch_with(ep, &machine, &dirs, since, Duration::from_millis(wait_ms).min(ipc::WATCH_WAIT))
                .await
            {
                Ok(changes) => ipc::Reply::DirsChanged { changes },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::CarryIn { carrier } => match self.carry_hosted(&carrier).await {
                Ok((addr, cookie)) => ipc::Reply::Piping { addr, cookie },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::PipeOn { machine, target } => match self.pipe_hosted(ep, &machine, &target).await {
                Ok((addr, cookie)) => ipc::Reply::Piping { addr, cookie },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::ReachProbe { host, port } => match self.reach_probe_with(ep, &host, port).await {
                Ok(view) => ipc::Reply::ReachProbe { view },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::StatOn { machine, path } => match self.stat_with(ep, &machine, &path).await {
                Ok((total, at_ms, digest)) => ipc::Reply::Stat {
                    total,
                    at_ms,
                    digest,
                },
                Err(why) => ipc::Reply::Refused { why },
            },
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
            ipc::Op::GitOn { machine, ask } => match self.git_with(ep, &machine, &ask).await {
                Ok(answer) => ipc::Reply::Git { answer },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::HistoryOn { machine, ask } => match self.history_with(ep, &machine, &ask).await {
                Ok(hits) => ipc::Reply::History { hits },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::ActOn {
                machine,
                session,
                action,
            } => match self.act_on_moved(ep, &machine, &session, &action).await {
                Ok(moved) => ipc::Reply::Accepted {
                    moved,
                    landed: Vec::new(),
                },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::PaneHistoryOn { machine, session, since } => {
                match self.pane_history_with(ep, &machine, &session, since).await {
                    Ok(history) => ipc::Reply::PaneHistory { history },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
            ipc::Op::DropOn {
                token,
                from,
                path,
                to,
            } => match self.drop_step_with(ep, &token, &from, &path, &to, None).await {
                Ok(state) => ipc::Reply::DropFile { state },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::DropInto {
                token,
                from,
                path,
                to,
                dir,
            } => match self.drop_step_with(ep, &token, &from, &path, &to, Some(&dir)).await {
                Ok(state) => ipc::Reply::DropFile { state },
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::DropStop { token, to } => match self.drop_stop_with(ep, &token, &to).await {
                Ok(()) => ipc::Reply::Noted,
                Err(why) => ipc::Reply::Refused { why },
            },
            ipc::Op::AddOn {
                machine,
                token,
                host,
                port,
                jump,
                name,
                satellite_only,
            } => {
                let req = Request::AddOverSsh {
                    token,
                    host,
                    port,
                    jump,
                    name,
                    satellite_only,
                };
                match self.add_ask_with(ep, &machine, &req).await {
                    Ok(run) => ipc::Reply::Adding { run },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
        }
    }

    /// A session started **here** for another machine's
    /// [`Request::Open`] (or `OpenOnce`, which names the id): registered,
    /// its host spawned, and put on the list for every machine.
    async fn open_here(
        &self,
        id: Option<crate::SessionId>,
        kind: String,
        title: String,
        cwd: String,
        cmd: Vec<String>,
        (cols, rows): (u16, u16),
    ) -> Result<crate::SessionId, String> {
        // A phone hosts nothing (docs/IOS.md); an asker that read its row
        // already refused, this is for one that did not.
        self.refuse_unless_hosting()?;
        // Defaults are resolved **here**, on the machine that will run
        // it: the asker's home and the asker's `$SHELL` are answers to a
        // question about the wrong computer. **`~` too** (2026-09-06, the
        // wizard's shell kind sends the directory a person typed): it
        // names this machine's home, which only this machine knows —
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
        // Off the reactor for the reason every registry write is (the
        // `Ls` arm's note): these homes can be on a network mount, and
        // this one forks a process too.
        let live = self.live.clone();
        let category = Self::category_of_started(&kind);
        let id = tokio::task::spawn_blocking(move || {
            let id = match id {
                Some(id) => id,
                None => crate::SessionId(format!("{kind}/{}", crate::link::fresh_leaf()?)),
            };
            live.register(&id, &kind, &title, None, category)?;
            live.learn_place(&id, &cwd)?;
            let dir = live.dir_of(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
            crate::host::spawn_host_at(&cwd, &dir, &id, &cmd, (cols, rows))?;
            Ok::<_, String>(id)
        })
        .await
        .map_err(|e| e.to_string())??;
        // Opened here for another machine, and on the list for all of
        // them (`khor_sync::roster`).
        self.roster_new(&id)?;
        Ok(id)
    }

    /// Asks `channel` to open a session, **at most once when `once`
    /// carries a token** (`crate::once`): then the ask is
    /// [`Request::OpenOnce`]-shaped, and because a second ask cannot
    /// make a second session it counts as a reading
    /// (`Request::is_reading`) — a wire that died under it is asked again
    /// on a fresh one inside the same wait, the case it exists for. A
    /// khor that predates the new words refuses them by name; that one is
    /// asked the old way.
    async fn ask_open(
        &self,
        ep: &iroh::Endpoint,
        channel: &str,
        plain: Request,
        once: Option<(&str, Request)>,
    ) -> Result<crate::SessionId, String> {
        let answer = match once {
            Some((variant, req)) => {
                match self.ask(ep, channel, &req).await? {
                    Response::Refused { why } if why.contains(variant) => self.ask(ep, channel, &plain).await?,
                    other => other,
                }
            }
            None => self.ask(ep, channel, &plain).await?,
        };
        match answer {
            Response::Opened { session } => Ok(crate::SessionId(session)),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
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
        once: Option<&str>,
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
            let host = crate::satellite::Dest::of(&sat).ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
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
                once,
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
        let once = once.map(|token| {
            (
                "OpenOnce",
                Request::OpenOnce {
                    token: token.to_owned(),
                    kind: kind.to_owned(),
                    title: title.to_owned(),
                    cwd: cwd.to_owned(),
                    cmd: cmd.to_vec(),
                    cols: size.0,
                    rows: size.1,
                },
            )
        });
        self.ask_open(ep, &channel, req, once).await
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
        self.reach_or_why(ep, machine, session, false)
            .await
            .map_err(|e| match e {
                Rejoin::Unreached(why) | Rejoin::Gone(why) => why,
            })
    }

    /// [`Self::reach_on_with`], saying which of two things a failure
    /// was ([`Rejoin`]): **the far machine was not reached**, or **it
    /// was, and the session is not there to attach to**. Only the first
    /// is worth asking again.
    ///
    /// `again`: the face asking is one whose pipe just ended, so the pipe
    /// held for this session is not handed back even when its listener
    /// still runs — the connection under it is what ended. Any other
    /// face on that pipe ended with it and asks again the same way.
    async fn reach_or_why(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        session: &crate::SessionId,
        again: bool,
    ) -> Result<(std::net::SocketAddr, String), Rejoin> {
        let (channel, home) = self.resolve(machine).map_err(Rejoin::Gone)?;
        if home == self.device() {
            // This machine's own row: the same answer a far asker gets
            // (`LiveKind::terminal_address`) — and none on a machine
            // that hosts nothing, which has no host file to read.
            self.refuse_unless_hosting().map_err(Rejoin::Gone)?;
            let (live, bridging) = (self.live.clone(), session.clone());
            let hf = tokio::task::spawn_blocking(move || live.terminal_address(&bridging))
                .await
                .map_err(|e| Rejoin::Gone(e.to_string()))?
                .map_err(Rejoin::Gone)?;
            let addr: std::net::SocketAddr = ([127, 0, 0, 1], hf.port).into();
            return Ok((addr, hf.cookie));
        }
        // `ask`, in its two halves: a dial that fails, or a request that
        // never comes back, is the far machine not reached; an answer
        // that refuses is the far machine speaking.
        let d = self
            .devices_loaded()
            .map_err(Rejoin::Gone)?
            .doc
            .by_name(&channel)
            .ok_or_else(|| Rejoin::Gone(msg::machine_left_table(&channel)))?;
        // **A face asking again starts on fresh wires** where the held ones
        // have gone quiet: its pipe just ended, or a person pressed reload
        // because the screen stopped moving — either way a wire that missed
        // a keepalive is the likely cause, and the new attachment should not
        // spend the ack wait finding that out.
        if again {
            self.wires.retire_if_quiet(&d.id, wires::Proto::Main);
            self.wires.retire_if_quiet(&d.id, wires::Proto::Tunnel);
        }
        let answer = self
            .ask_on(
                ep,
                &d,
                &Request::Reach {
                    session: session.0.clone(),
                },
            )
            .await
            .map_err(Rejoin::Unreached)?;
        let (port, cookie) = match answer {
            Response::Reached { port, cookie } => (port, cookie),
            Response::Refused { why } => return Err(Rejoin::Gone(why)),
            other => return Err(Rejoin::Gone(msg::peer_non_answer(format_args!("{other:?}")))),
        };
        {
            let mut held = self.reaches.lock().await;
            match held.get(session) {
                // Same live pipe, same far port: reuse it. One session,
                // one local port, however many faces attach.
                Some((addr, far, handle)) if !again && !handle.is_finished() && *far == port => {
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
        let borrow = self.tunnel_on(ep, machine).await.map_err(Rejoin::Unreached)?;
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| Rejoin::Unreached(e.to_string()))?;
        let addr = listener
            .local_addr()
            .map_err(|e| Rejoin::Unreached(e.to_string()))?;
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

    /// **The wire to a machine for `proto`**: the one held, or one
    /// dialled now and held from here on (`crate::wires` has the
    /// lifecycle). Dials to one machine are one at a time — a second
    /// asker waits for the first dial instead of racing it — and an asker
    /// for another machine never waits on this one at all.
    ///
    /// 按机器暂停 (SHELL.md 第五轮 第 3 条) reaches a wire that was
    /// already held too: a pause that only stopped *new* dials would keep
    /// talking to the machine on the old one.
    pub(crate) async fn line_to(
        &self,
        ep: &iroh::Endpoint,
        d: &DeviceInfo,
        proto: wires::Proto,
    ) -> Result<Line, DialFailure> {
        self.line_to_by(ep, d, proto, None).await
    }

    /// [`Node::line_to`], a dial it has to make finished `by` then — what
    /// a caller with a deadline of its own passes, so the dial's time out
    /// is said in the dial's words before the caller's deadline says
    /// something vaguer (`Node::ask_on`).
    pub(crate) async fn line_to_by(
        &self,
        ep: &iroh::Endpoint,
        d: &DeviceInfo,
        proto: wires::Proto,
        by: Option<std::time::Instant>,
    ) -> Result<Line, DialFailure> {
        self.line_to_inner(ep, d, proto, by, true).await
    }

    /// [`Node::line_to_by`]; `may_hop` false for the wires a hop itself
    /// rides (`crate::hop`), so a hop never asks for another.
    pub(crate) async fn line_to_inner(
        &self,
        ep: &iroh::Endpoint,
        d: &DeviceInfo,
        proto: wires::Proto,
        by: Option<std::time::Instant>,
        may_hop: bool,
    ) -> Result<Line, DialFailure> {
        if crate::pause::peer_on(self.root(), &d.id) {
            self.wires.retire_peer(&d.id, wires::Retired::Paused);
            return Err(DialFailure::Refused(msg::peer_paused_refusal().to_owned()));
        }
        let line = |conn| Line {
            id: d.id.clone(),
            name: d.name.clone(),
            proto,
            conn,
        };
        if let Some(conn) = self.wires.live(&d.id, proto) {
            return Ok(line(conn));
        }
        let lock = self.wires.dial_lock(&d.id, proto);
        let _one = lock.lock().await;
        if let Some(conn) = self.wires.live(&d.id, proto) {
            return Ok(line(conn));
        }
        // The peer hop (round ROAD-3): a road through a third machine,
        // ready before the dial so it races from the first packet.
        if may_hop {
            self.hop_ahead(ep, d).await;
        }
        match self.dial(ep, &d.id, &d.addrs, proto.alpn(), by).await {
            Ok(conn) => {
                self.wires.adopt(&d.id, proto, conn.clone());
                Ok(line(conn))
            }
            Err(e) => {
                let words = match &e {
                    DialFailure::TimedOut => msg::machine_unreachable_timeout(&d.name),
                    DialFailure::Refused(why) => msg::cant_reach_named(&d.name, why),
                };
                self.wires.failed(&d.id, proto, words);
                Err(e)
            }
        }
    }

    /// A look at the wires this node is holding — for tests and for
    /// whatever diagnostic verb wants it later. Taken **before** `serve`
    /// consumes the node: the registry is shared, so a watch made here
    /// still sees what the serve does with it.
    pub fn wire_watch(&self) -> crate::WireWatch {
        let names = self
            .devices()
            .unwrap_or_default()
            .into_iter()
            .map(|d| (d.id, d.name))
            .collect();
        crate::WireWatch::of(&self.wires, names)
    }

    /// One request on a held wire, watched for life (`exchange`): what
    /// it hears keeps the wire's liveness, and a wire that never
    /// answered is retired — **also when the caller stops waiting
    /// first** (an ipc deadline cancelling the op; `RetireUnheard`).
    /// That second half is audit/net.md F7: a cancelled ask used to
    /// skip the retirement, so a dead wire was handed out again for
    /// 100 s of 29.5 s hangs.
    pub(crate) async fn exchange_on(&self, line: &Line, req: &Request) -> Result<Response, Miss> {
        let mut guard = RetireUnheard::new(&self.wires, line);
        let budget = wires::budget_for(req.budget(), &line.conn);
        let out = exchange(&line.conn, &line.name, req, budget).await;
        guard.disarm();
        match &out {
            Ok(_) => self.wires.heard(&line.id, line.proto, &line.conn),
            Err(Miss::Unreached(_)) => {
                let why = if line.conn.close_reason().is_some() {
                    wires::Retired::Closed
                } else {
                    wires::Retired::Unanswered
                };
                self.wires.retire(&line.id, line.proto, &line.conn, why);
            }
            Err(Miss::Late(_)) => {
                self.wires
                    .counts
                    .answers_late
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            Err(Miss::Garbled(_)) => {}
        }
        out
    }

    /// **One request to a machine, on the wire held to it.**
    ///
    /// A reading whose wire turns out dead is asked once more on a fresh
    /// one, so a machine that restarted under a held wire costs the
    /// person one [`crate::wires::ACK_WITHIN`], not an error. Anything
    /// else is not asked twice: `Open` on a fresh wire after a death
    /// could be a second session on a machine that may well have started
    /// the first ([`Request::is_reading`]).
    pub(crate) async fn ask_on(
        &self,
        ep: &iroh::Endpoint,
        d: &DeviceInfo,
        req: &Request,
    ) -> Result<Response, String> {
        // **Answer inside the face's own wait** (round NET-5): a face asks
        // through the resident, which gives up after `ipc::DEADLINE_FAR`
        // and says only that it stopped waiting. A dead wire's ack wait
        // plus the redial behind a slow road could run past that (29.5 s
        // at 600 ms / 5 % loss), so the redial is cut to what is left,
        // and the face reads 「无法连接」 rather than 「未在 30 秒内完成」.
        let by = std::time::Instant::now() + ipc::DEADLINE_FAR - Duration::from_secs(2);
        match self.ask_early(ep, d, req).await {
            Early::Answered(resp) => return Ok(resp),
            Early::Failed(e) => return Err(unreached_words(&d.name, e)),
            Early::NotTaken => {}
        }
        let line = self
            .line_to(ep, d, wires::Proto::Main)
            .await
            .map_err(|e| unreached_words(&d.name, e))?;
        match self.exchange_on(&line, req).await {
            Err(Miss::Unreached(_)) if req.is_reading() => {
                self.wires
                    .counts
                    .reads_retried
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let line = self
                    .line_to_by(ep, d, wires::Proto::Main, Some(by))
                    .await
                    .map_err(|e| unreached_words(&d.name, e))?;
                self.exchange_on(&line, req).await.map_err(Miss::words)
            }
            other => other.map_err(Miss::words),
        }
    }

    /// **A redial that carries its first question** (round NET-3): with
    /// no wire held to `d` and a request that may ride early data
    /// (`Request::rides_early`), the dial resumes the TLS session of an
    /// earlier wire and sends the request inside the handshake — the
    /// answer comes back in one round trip instead of the handshake's
    /// plus the request's. The wire is then held as any other.
    ///
    /// `NotTaken` when there is a wire, the request may not ride early,
    /// or there is no session to resume (a first dial to that machine
    /// since this resident started, or the far side rejected the early
    /// data); the wire this made, if any, is held, and the ask goes on
    /// as usual over it.
    async fn ask_early(&self, ep: &iroh::Endpoint, d: &DeviceInfo, req: &Request) -> Early {
        if !req.rides_early()
            || crate::pause::peer_on(self.root(), &d.id)
            || self.wires.live(&d.id, wires::Proto::Main).is_some()
        {
            return Early::NotTaken;
        }
        let lock = self.wires.dial_lock(&d.id, wires::Proto::Main);
        let _one = lock.lock().await;
        if self.wires.live(&d.id, wires::Proto::Main).is_some() {
            return Early::NotTaken;
        }
        self.hop_ahead(ep, d).await;
        let Ok(mut addr) = endpoint::dial_addr(&d.id, &d.addrs, self.relays()) else {
            return Early::NotTaken;
        };
        // A road this machine carries itself, as `dial_with` adds it.
        addr.addrs.extend(self.carried_addr(&d.id));
        let Ok(frame) = proto::encode(req) else {
            return Early::NotTaken;
        };
        let mut within = DIAL_TIMEOUT
            .max(self.wires.last_rtt(&d.id).saturating_mul(12))
            .min(Duration::from_secs(60));
        // As `dial`: short for a machine its relay just said left.
        if self.wires.gone_lately(&d.id) {
            within = within.min(
                Duration::from_secs(3).max(self.wires.last_rtt(&d.id).saturating_mul(6)),
            );
        }
        let started = std::time::Instant::now();
        let attempt = async {
            let connecting = ep
                .connect_with_opts(addr, ALPN, Default::default())
                .await
                .map_err(|e| DialFailure::Refused(e.to_string()))?;
            let early = match connecting.into_0rtt() {
                Ok(early) => early,
                // Nothing to resume: an ordinary handshake, then the ask
                // as usual on the wire it made.
                Err(connecting) => {
                    let conn = connecting
                        .await
                        .map_err(|e| DialFailure::Refused(e.to_string()))?;
                    return Ok((conn, None));
                }
            };
            // The question goes out with the handshake; its answer is
            // waited for apart, so the wire is handed to every other ask
            // the moment the handshake is done, not when this answer is.
            let asked = {
                let (early, frame, budget) = (early.clone(), frame.clone(), req.budget());
                tokio::spawn(async move {
                    let (mut send, mut recv) = early.open_bi().await.ok()?;
                    send.write_all(&frame).await.ok()?;
                    send.finish().ok()?;
                    let bytes = tokio::time::timeout(budget, recv.read_to_end(MAX_FRAME))
                        .await
                        .ok()?
                        .ok()?;
                    proto::answer(&bytes).ok()
                })
            };
            match early
                .handshake_completed()
                .await
                .map_err(|e| DialFailure::Refused(e.to_string()))?
            {
                iroh::endpoint::ZeroRttStatus::Accepted(conn) => Ok((conn, Some(asked))),
                // Rejected: the far side kept nothing of it; asked again
                // on the wire, which is up.
                iroh::endpoint::ZeroRttStatus::Rejected(conn) => {
                    asked.abort();
                    Ok((conn, None))
                }
            }
        };
        match tokio::time::timeout(within, attempt).await {
            Ok(Ok((conn, asked))) => {
                crate::roads::replied(&self.roads, &d.id, started.elapsed());
                crate::roads::watch(&self.roads, &d.id, &conn);
                self.wires.adopt(&d.id, wires::Proto::Main, conn.clone());
                drop(_one);
                match asked {
                    Some(asked) => match asked.await.ok().flatten() {
                        Some(resp) => {
                            self.wires.heard(&d.id, wires::Proto::Main, &conn);
                            Early::Answered(resp)
                        }
                        // No answer to the early question: asked again on
                        // the wire, which is up — a reading, so twice is
                        // the same as once.
                        None => Early::NotTaken,
                    },
                    None => Early::NotTaken,
                }
            }
            Ok(Err(e)) => {
                self.wires.failed(&d.id, wires::Proto::Main, msg::cant_reach_named(&d.name, match &e {
                    DialFailure::Refused(why) => why,
                    DialFailure::TimedOut => "",
                }));
                Early::Failed(e)
            }
            Err(_) => {
                self.wires.failed(&d.id, wires::Proto::Main, msg::machine_unreachable_timeout(&d.name));
                Early::Failed(DialFailure::TimedOut)
            }
        }
    }

    /// [`Self::ask_on`] by the name a person typed.
    async fn ask(
        &self,
        ep: &iroh::Endpoint,
        channel: &str,
        req: &Request,
    ) -> Result<Response, String> {
        let d = self
            .devices_loaded()?
            .doc
            .by_name(channel)
            .ok_or_else(|| msg::machine_left_table(channel))?;
        self.ask_on(ep, &d, req).await
    }

    /// Routes an op to the resident serve when one holds this key.
    /// `None` = no live serve, take the direct path. `Some(Err)` = a
    /// serve is alive but unreachable — never fall back then: the key is
    /// taken and a second endpoint would knock both off.
    pub(crate) async fn via_serve(&self, op: ipc::Op) -> Option<Result<ipc::Reply, String>> {
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
    async fn serve_tunnel(
        self: std::sync::Arc<Self>,
        conn: iroh::endpoint::Connection,
        ep: std::sync::Arc<iroh::Endpoint>,
    ) -> Result<(), String> {
        let remote = conn.remote_id().to_string();
        while let Ok((send, mut recv)) = conn.accept_bi().await {
            // A machine this one has paused is treated as a stranger
            // here, per stream, so a pause bites on a tunnel already up.
            let paired =
                !crate::pause::peer_on(self.root(), &remote) && self.admit(&remote).is_ok();
            let (node, ep, remote) = (self.clone(), ep.clone(), remote.clone());
            tokio::spawn(async move {
                let Ok(target) = crate::tunnel::read_target(&mut recv).await else { return };
                // A hop's ask rides the exit's stream (round ROAD-3).
                match crate::hop::Ask::of(&target) {
                    Some(ask) => node.serve_hop(ep, &remote, ask, send, recv, paired).await,
                    None => {
                        let _ = crate::tunnel::serve_target(send, recv, &target, paired).await;
                    }
                }
            });
        }
        Ok(())
    }

    /// Answers the requests a far machine sends on one connection —
    /// **each stream in its own task**, at most [`STREAMS_AT_ONCE`] at a
    /// time, and the next one waits for a seat rather than for the one in
    /// front of it.
    ///
    /// ⚠️ 墓碑 (2026-10-03, audit/net.md F5): this answered one stream at
    /// a time, and since every ask from one machine rides one wire, a
    /// five-second search held a `git status` behind it for 4.7 s (alone:
    /// 2–13 ms). Nothing here relies on order: a sync round is sequential
    /// on the asking side, each request carries all it needs, and the
    /// gate (`admit`) and pause are asked per request.
    ///
    /// `done` says whether the handshake has completed (`Some(true)`), or
    /// failed: a request that arrived as early data and may not ride it
    /// (`Request::rides_early`) waits for `Some(true)` and is dropped on
    /// anything else — a replay of captured early data never completes a
    /// handshake, so it never reaches anything that changes something.
    async fn handle<T: iroh::endpoint::ConnectionState>(
        self: std::sync::Arc<Self>,
        conn: iroh::endpoint::Connection<T>,
        remote: String,
        ep: std::sync::Arc<iroh::Endpoint>,
        done: tokio::sync::watch::Receiver<Option<bool>>,
    ) -> Result<(), String> {
        let seats = std::sync::Arc::new(tokio::sync::Semaphore::new(STREAMS_AT_ONCE));
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
            let Ok(seat) = seats.clone().acquire_owned().await else {
                return Ok(());
            };
            // A machine reached this one: these ears work (round NET-11).
            self.reached_at.store(crate::now_ms(), std::sync::atomic::Ordering::Relaxed);
            let (node, ep, remote, mut done) = (self.clone(), ep.clone(), remote.clone(), done.clone());
            tokio::spawn(async move {
                let _seat = seat;
                let early = recv.is_0rtt();
                let Ok(bytes) = recv.read_to_end(MAX_FRAME).await else {
                    return;
                };
                // **A watch keeps its stream** (`crate::watch`): answered
                // as it goes, not once — through the same gate as every
                // other request.
                if let Ok(Request::Watch { dirs }) = proto::decode::<Request>(&bytes) {
                    if early && *done.borrow() != Some(true) {
                        while done.borrow().is_none() {
                            if done.changed().await.is_err() {
                                return;
                            }
                        }
                        if *done.borrow() != Some(true) {
                            return;
                        }
                    }
                    match node.admit(&remote) {
                        Ok(()) => crate::watch::serve(dirs, send).await,
                        Err(why) => {
                            if let Ok(frame) = proto::encode(&Response::Refused { why }) {
                                let mut out = (frame.len() as u32).to_be_bytes().to_vec();
                                out.extend_from_slice(&frame);
                                let _ = send.write_all(&out).await;
                                let _ = send.finish();
                            }
                        }
                    }
                    return;
                }
                let resp = match proto::decode::<Request>(&bytes) {
                    Ok(req) => {
                        if early && !req.rides_early() {
                            while done.borrow().is_none() {
                                if done.changed().await.is_err() {
                                    return;
                                }
                            }
                            if *done.borrow() != Some(true) {
                                return;
                            }
                        }
                        node.dispatch(&remote, req, &ep).await
                    }
                    Err(e) => Response::Refused { why: e },
                };
                let Ok(frame) = proto::encode(&resp) else {
                    return;
                };
                if send.write_all(&frame).await.is_ok() {
                    let _ = send.finish();
                }
            });
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
        // **One gate, before any arm** (docs/CONNECT.md 名单是信任的唯一
        // 出处): in the table, and on the roster. `Pair` is the one
        // request a stranger may make — the token is its admission. It
        // used to be a table check repeated at the head of every arm;
        // an arm added without it would have been a door with no lock.
        if !matches!(req, Request::Pair { .. } | Request::Vouch { .. })
            && let Err(e) = self.admit(remote)
        {
            // **A stranger knocking is a reason to round now** (round
            // NET-6): most often it is a machine paired a moment ago
            // through another, whose row this one has not been sent yet
            // — and with no face here the next round is QUIET_PACE away.
            self.nudge(ep);
            return Err(e);
        }
        match req {
            Request::Pair {
                token,
                name,
                addrs,
                roster,
            } => {
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
                let loaded = self.devices_loaded()?;
                // Whose root this network is, **before the token burns**:
                // two networks that each have a root are refused here, and
                // the ticket is still good once the other side has left
                // its own (`khor_net::trust::settle`).
                self.settle_with(&loaded.doc, &roster, remote, false)?;
                // Burn before use: a token that pairs twice is a door
                // that never closes.
                fs::remove_file(&path).map_err(msg::cant_burn_invite)?;
                loaded.doc.upsert(remote, &name, &addrs)?;
                // Taking a machine back in is what pairing *is*, so a
                // ticket clears the tombstone. Deliberately here and not
                // on the far side: a machine that was shown the door
                // cannot let itself back in, it has to be handed a
                // ticket by somebody already inside.
                loaded.doc.set_gone(remote, false)?;
                // Pairing is done when the new machine has a certificate
                // (CONNECT.md 身份): signed now if this machine holds the
                // root, waiting in `khor pending` otherwise.
                self.enroll_after_pair(
                    &loaded.doc,
                    &[(remote, &name), (self.device_str(), self.name())],
                )?;
                let mut store = loaded.store;
                store.flush(&loaded.doc)?;
                // Everybody else learns of the new machine now, not at
                // the next quiet beat (the stranger's knock above is the
                // other half).
                self.nudge(ep);
                Ok(Response::Paired {
                    name: self.name().to_owned(),
                    devices: B64.encode(loaded.doc.snapshot()?),
                    roster: serde_bytes::ByteBuf::from(self.roster_offer_in(&loaded.doc)),
                })
            }
            // The other door a stranger may knock on (round NET-10): the
            // root's signature is its admission, as the token is `Pair`'s.
            Request::Vouch { name, addrs, roster } => {
                self.take_vouch(remote, &name, &addrs, &roster)?;
                // Its row is here now: the round that brings the rest
                // starts from this side too, not at the next quiet beat.
                self.nudge(ep);
                Ok(Response::Vouched)
            }
            Request::Sync { doc, have, changes } => {
                let reply = with_doc!(self, doc.as_str(), |loaded| wire::answer(
                    &mut loaded.store,
                    &loaded.doc,
                    &have,
                    &changes
                ))?;
                if doc == "devices" && !changes.is_empty() {
                    self.follow_after_sync();
                }
                if !changes.is_empty() && self.is_own_chat(&doc) {
                    self.take_arrived();
                }
                Ok(Response::Synced {
                    version: reply.version,
                    changes: reply.changes,
                    items: reply.items as u64,
                })
            }
            Request::Fetch { digest, offset } => {
                self.serve_slice(&remote, &digest, offset)
            }
            Request::Versions { docs, readings } => {
                self.answer_versions(remote, docs, readings, ep).await
            }
            Request::WorkspaceFacts => Ok(Response::WorkspaceFacts {
                facts: self
                    .wsfacts
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .as_ref()
                    .map(|k| k.facts())
                    .unwrap_or_default(),
            }),
            Request::Sessions => {
                Ok(Response::SessionRows {
                    rows: self.reportable_rows()?,
                })
            }
            Request::Days => {
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
                // Off the reactor with `Remove`, for `Remove`'s reason.
                tokio::task::spawn_blocking(move || crate::files::mkdir(&path))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Made)
            }
            Request::RenamePath { from, to } => {
                tokio::task::spawn_blocking(move || crate::files::rename_path(&from, &to))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Renamed)
            }
            Request::Launch { target } => {
                // Nothing is opened on a phone (docs/IOS.md): its opener
                // is not a program khor may start.
                if !self.hosts() {
                    return Err(msg::launch_needs_a_computer(self.name()));
                }
                // Off the reactor: starting a process is a fork, and a
                // fork on the reactor is the one thing that stops every
                // other answer this machine owes.
                tokio::task::spawn_blocking(move || crate::files::launch(&target))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Launched)
            }
            Request::Git { ask } => {
                // Off the reactor: a log walks up to `git::WALK` commits
                // and a blame diffs the file's whole history.
                let answer = tokio::task::spawn_blocking(move || crate::git::answer(&ask))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Git { answer })
            }
            // A tmux row's front pane history (item 31): tmux runs here.
            Request::PaneHistory { session, since } => {
                let id = crate::SessionId(session);
                let live = self.live.clone();
                let history = tokio::task::spawn_blocking(move || live.tmux_history(&id, since.as_ref()))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::PaneHistory { history })
            }
            Request::History { ask } => {
                // Off the reactor: up to `HISTORY_MAX` of lines read and
                // matched.
                let file = crate::commands::history_file(self.root());
                let hits = tokio::task::spawn_blocking(move || crate::commands::search_history(&file, &ask))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Response::History { hits })
            }
            Request::Search { dir, query } => {
                // Off the reactor with `Touched` below, for its reason
                // and more of it: this walks a project's files and
                // reads every text file in it.
                let found = tokio::task::spawn_blocking(move || crate::files::search(&dir, &query))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Found { found })
            }
            Request::Touched { path } => {
                // Off the reactor: a walk of somebody's project tree is
                // thousands of stat() calls, and the cap is 20_000 of
                // them (`crate::workspace::MOST_ENTRIES`).
                let dir = std::path::PathBuf::from(path);
                let at_ms = tokio::task::spawn_blocking(move || crate::workspace::touched(&dir))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Response::Touched { at_ms })
            }
            Request::Checkout { path } => {
                // Off the reactor anyway: one stat, but on a network
                // disk one stat is the call that hangs.
                let dir = std::path::PathBuf::from(path);
                let yes = tokio::task::spawn_blocking(move || crate::git::is_checkout(&dir))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Response::Checkout { yes })
            }
            Request::GitStatus { root } => {
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
                tokio::task::spawn_blocking(crate::code::stop)
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Acted {
                    moved: 0,
                    landed: Vec::new(),
                })
            }
            Request::Quotas => {
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
            // A watch is a stream, answered in `handle` before this; one
            // that arrives here came by a road that answers once.
            Request::Watch { .. } => Err(msg::watch_unavailable().into()),
            Request::Stat { path } => {
                // One `stat` and at most one slice off the disk, off the
                // reactor for the `Ls` arm's reason.
                let (total, at_ms, digest) =
                    tokio::task::spawn_blocking(move || crate::files::stat(&path))
                        .await
                        .map_err(|e| e.to_string())??;
                Ok(Response::Stat {
                    total,
                    at_ms,
                    digest,
                })
            }
            Request::LoadSince { top, by, desc, day } => {
                let root = self.root().to_path_buf();
                let (top, day, cpu_day) = tokio::task::spawn_blocking(move || {
                    crate::Node::load_since(&root, top as usize, by, desc, day)
                })
                .await
                .map_err(|e| e.to_string())?;
                Ok(Response::LoadSince { top, day, cpu_day })
            }
            Request::Vitals => {
                // The background sampler's last round, at most one beat
                // old (`vitals::local`, docs/SHELL.md 第十六轮) — no
                // longer taken to answer, because a sleeping disk made
                // the taking ten seconds long.
                //
                // **Off the reactor** all the same: before the first
                // round this waits for it, and a serve that has not got
                // one within a few seconds takes one itself — still
                // bounded, each volume by its own limit. Called inline a
                // sample stalled the QUIC connection this very reply
                // travels on, and the two real-connection avatar tests
                // timed out at twenty seconds (the ledger's "serve 里任何
                // 会等 DIAL_TIMEOUT 的事不能内联在 select 循环").
                let root = self.root().to_path_buf();
                let vitals = tokio::task::spawn_blocking(move || {
                    crate::vitals::local::reading_within(&root, std::time::Duration::from_secs(3))
                        .map(|r| r.vitals)
                        .unwrap_or_else(|| crate::vitals::sample(&root))
                })
                .await
                .map_err(|e| e.to_string())?;
                Ok(Response::Vitals { vitals })
            }
            Request::Watched => {
                // The list is this machine's own row in the table — the
                // asker wrote it there, and it arrived on the sync round
                // before this request. Read from the background
                // sampler, which a new list wakes for a round; off the
                // reactor for the `Vitals` arm's reason.
                let paths = self.watch_of(self.device_str());
                let root = self.root().to_path_buf();
                let rows = tokio::task::spawn_blocking(move || {
                    crate::vitals::local::watched_within(
                        &root,
                        &paths,
                        std::time::Duration::from_secs(3),
                    )
                    .filter(|(rows, _)| rows.len() == paths.len())
                    .map(|(rows, _)| rows)
                    .unwrap_or_else(|| crate::vitals::watched(&paths))
                })
                .await
                .map_err(|e| e.to_string())?;
                Ok(Response::Watched { rows })
            }
            Request::Usage => {
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
                match action.as_str() {
                    "take" | "stop" => {
                        let (moved, landed) =
                            self.take_local(ep, &crate::SessionId(session), &action).await?;
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
                    // A tmux row's prompts, in the pane its client has in
                    // front (item 21): tmux runs on this machine.
                    "prompt-up" | "prompt-down" => {
                        let id = crate::SessionId(session);
                        let up = action == "prompt-up";
                        let live = self.live.clone();
                        tokio::task::spawn_blocking(move || live.jump_prompt(&id, up))
                            .await
                            .map_err(|e| e.to_string())??;
                        Ok(Response::Acted {
                            moved: 0,
                            landed: Vec::new(),
                        })
                    }
                    // The wheel over a shell's own tmux client (item
                    // 58): `moved` 1 when tmux took it.
                    other if other.starts_with("wheel:") => {
                        let rows: i32 = other["wheel:".len()..].parse().map_err(|_| msg::unknown_action(other))?;
                        let id = crate::SessionId(session);
                        let live = self.live.clone();
                        let took = tokio::task::spawn_blocking(move || live.tmux_wheel(&id, rows))
                            .await
                            .map_err(|e| e.to_string())??;
                        Ok(Response::Acted {
                            moved: took as u64,
                            landed: Vec::new(),
                        })
                    }
                    other => Err(msg::unknown_action(other)),
                }
            }
            Request::Ls { path } => {
                // Off the reactor for the standing reason ("how long
                // does it block", 坑节): a directory on a network mount
                // can sit for seconds, and this reply's own QUIC
                // connection would sit with it.
                let root = self.root().to_path_buf();
                let (path, entries, truncated) =
                    tokio::task::spawn_blocking(move || listed_with_arrivals(&root, &path))
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
                let id = self.open_here(None, kind, title, cwd, cmd, (cols, rows)).await?;
                Ok(Response::Opened { session: id.0 })
            }
            Request::OpenOnce {
                token,
                kind,
                title,
                cwd,
                cmd,
                cols,
                rows,
            } => {
                // **Made at most once per token** (`crate::once`). The id
                // is chosen and written down *before* the session is
                // registered, so whatever moment this machine stopped at,
                // a retry finds the id — and makes the session under it
                // if it was never made.
                if !crate::once::valid(&token) {
                    return Err(msg::create_token_bad().into());
                }
                let _held = crate::once::lock(&token).await;
                let root = self.root().to_path_buf();
                let id = match crate::once::made(&root, &token) {
                    Some(id) => crate::SessionId(id),
                    None => {
                        let id = crate::SessionId(format!("{kind}/{}", crate::link::fresh_leaf()?));
                        crate::once::note(&root, &token, &id.0)?;
                        id
                    }
                };
                if self.live.dir_of(&id).is_some_and(|d| d.join("meta.json").exists()) {
                    return Ok(Response::Opened { session: id.0 });
                }
                let id = self.open_here(Some(id), kind, title, cwd, cmd, (cols, rows)).await?;
                Ok(Response::Opened { session: id.0 })
            }
            Request::OpenAgentOnce {
                token,
                dir,
                title,
                form,
                agent,
            } => {
                // As `OpenOnce`, but the id is the agent's to give
                // (`adaptor::id_for` its own session), so it is written
                // down the moment it is known: a machine stopped between
                // the agent's start and that line makes a second one on
                // the retry. Seconds wide, against the whole create.
                if !crate::once::valid(&token) {
                    return Err(msg::create_token_bad().into());
                }
                let _held = crate::once::lock(&token).await;
                let root = self.root().to_path_buf();
                if let Some(id) = crate::once::made(&root, &token) {
                    return Ok(Response::Opened { session: id });
                }
                let id = tokio::task::spawn_blocking(move || {
                    crate::Node::open(root)?.start_session(&dir, &title, &form, &agent)
                })
                .await
                .map_err(|e| e.to_string())??;
                crate::once::note(self.root(), &token, &id.0)?;
                Ok(Response::Opened { session: id.0 })
            }
            Request::Reach { session } => {
                let id = crate::SessionId(session);
                // The same door a click opens locally: a row with a
                // route but no host gets one stood up **on the machine
                // that owns it**, which is the only machine that can.
                // Without this every discovered tmux session on a Linux
                // box answered 宿主没了 — a sentence about a process
                // nobody had asked to exist yet.
                //
                //
                // **The seat names the file** (`LiveKind::terminal_address`):
                // an agent in the person's tmux is reached through a
                // viewer, whose address is not in `host.json`.
                //
                // A phone hosts nothing, so it has no host file to read
                // and nothing to bridge (docs/IOS.md); an asker that read
                // its row did not ask.
                //
                // Off the reactor for the Open arm's reason and one
                // more: resolving the route re-runs the discovery sweep,
                // which shells out to `ps` and `tmux`.
                self.refuse_unless_hosting()?;
                let (live, bridging) = (self.live.clone(), id.clone());
                let hf = tokio::task::spawn_blocking(move || live.terminal_address(&bridging))
                    .await
                    .map_err(|e| e.to_string())??;
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
                let host = self.my_satellite_host(&sat)?;
                // Off the reactor with SatLs, for SatLs's reason.
                tokio::task::spawn_blocking(move || crate::satellite::remove(&host, &path))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Removed)
            }
            Request::SatTouch { sat, path } => {
                let host = self.my_satellite_host(&sat)?;
                tokio::task::spawn_blocking(move || crate::satellite::touch(&host, &path))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Made)
            }
            Request::SatMkdir { sat, path } => {
                let host = self.my_satellite_host(&sat)?;
                // Off the reactor with SatLs, for SatLs's reason.
                tokio::task::spawn_blocking(move || crate::satellite::mkdir(&host, &path))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Made)
            }
            Request::SatRename { sat, from, to } => {
                let host = self.my_satellite_host(&sat)?;
                tokio::task::spawn_blocking(move || {
                    crate::satellite::rename_path(&host, &from, &to)
                })
                .await
                .map_err(|e| e.to_string())??;
                Ok(Response::Renamed)
            }
            Request::SatLaunch { sat, target } => {
                let host = self.my_satellite_host(&sat)?;
                tokio::task::spawn_blocking(move || crate::satellite::launch(&host, &target))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(Response::Launched)
            }
            Request::SatSearch { sat, dir, query } => {
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
                let host = self.my_satellite_host(&sat)?;
                // Off the reactor with SatLs, for SatLs's reason: one
                // ssh dial and a walk of that machine's disk.
                let at_ms =
                    tokio::task::spawn_blocking(move || crate::satellite::touched(&host, &path))
                        .await
                        .map_err(|e| e.to_string())??;
                Ok(Response::Touched { at_ms })
            }
            Request::SatCheckout { sat, path } => {
                let host = self.my_satellite_host(&sat)?;
                let yes = tokio::task::spawn_blocking(move || {
                    crate::satellite::is_checkout(&host, &path)
                })
                .await
                .map_err(|e| e.to_string())??;
                Ok(Response::Checkout { yes })
            }
            Request::SatGitStatus { sat, root } => {
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
                let host = self.my_satellite_host(&sat)?;
                let (at_ms, digest) = self.sat_write(&sat, &host, &path, &was, &bytes).await?;
                Ok(Response::Wrote { at_ms, digest })
            }
            Request::DropFile { token, from, path } => {
                // The machine that has the file, by the id the asker
                // named; this one when it is this one — then nothing
                // moves, as on the asker's side.
                if from == self.device_str() {
                    return Ok(drop_answer(crate::drops::DropState::there(&path)));
                }
                let channel = self
                    .devices_loaded()?
                    .doc
                    .get(&from)
                    .map(|d| d.name)
                    .ok_or_else(|| msg::machine_left_table(&from))?;
                self.drop_here(ep, &token, &channel, &path, None).await.map(drop_answer)
            }
            Request::DropFileInto { token, from, path, dir } => {
                // Into the asker's directory: a file already here is
                // copied there (item 60 part 5).
                if from == self.device_str() {
                    return self.copy_here(&token, &path, &dir).await.map(drop_answer);
                }
                let channel = self
                    .devices_loaded()?
                    .doc
                    .get(&from)
                    .map(|d| d.name)
                    .ok_or_else(|| msg::machine_left_table(&from))?;
                self.drop_here(ep, &token, &channel, &path, Some(&dir)).await.map(drop_answer)
            }
            Request::DropStop { token } => {
                // The asker's cancel (item 71): the job by its token, here.
                if !crate::once::valid(&token) {
                    return Err(msg::create_token_bad().into());
                }
                crate::drops::stop(&token);
                Ok(Response::DropStopped)
            }
            Request::Probe { host, port } => Ok(Response::Probed {
                seen: crate::reach::probe_here(&host, port).await,
            }),
            Request::AddOverSsh {
                token,
                host,
                port,
                jump,
                name,
                satellite_only,
            } => {
                // An add another machine asked this one to run as the
                // sponsor (通过 khor 主机), on a thread of its own so it
                // outlives this ask; every ask answers its marks so far.
                if !crate::once::valid(&token) {
                    return Err(msg::create_token_bad().into());
                }
                let (run, fresh) = crate::join::run_for(&token);
                if fresh {
                    let node = self.me().ok_or_else(|| msg::add_on_needs_serve(self.name()))?;
                    let to = crate::satellite::Dest::new(host, jump.as_deref()).on_port(port);
                    crate::join::start(node, run.clone(), to, name, satellite_only);
                }
                let run = run.lock().unwrap_or_else(|p| p.into_inner()).clone();
                Ok(Response::Adding { run })
            }
            Request::FetchPath { path, offset } => {
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
    fn my_satellite_host(&self, sat: &str) -> Result<crate::satellite::Dest, String> {
        let d = self
            .devices_loaded()?
            .doc
            .get(sat)
            .ok_or_else(|| msg::not_my_satellite(sat))?;
        if d.sponsor.as_deref() != Some(self.device_str()) {
            return Err(msg::not_my_satellite(sat));
        }
        // A satellite the roster revoked is not driven, by its sponsor or
        // on anybody's behalf (docs/CONNECT.md 卫星也进名单). One still
        // waiting for approval is: its sponsor already holds the ssh road,
        // so driving it grants nothing new.
        if self.standing_of(sat) == crate::Standing::Revoked {
            return Err(msg::sat_revoked(d.shown()));
        }
        crate::satellite::Dest::of(&d).ok_or_else(|| msg::sat_no_ssh_host(sat))
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
        host: &crate::satellite::Dest,
        path: &str,
        offset: u64,
    ) -> Result<(u64, u64, Vec<u8>), String> {
        let staged = self.sat_staging_path(sat, path)?;
        if offset == 0 || !staged.exists() {
            let (h, p, s) = (host.clone(), path.to_owned(), staged.clone());
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
        host: &crate::satellite::Dest,
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
                host.clone(),
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
        let (h, p, mine) = (host.clone(), path.to_owned(), bytes.to_vec());
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
    ///
    /// `remote` is the device fetching: what it fetched is kept as its
    /// own (`transfer::Offer::to`), never as the offer's.
    fn serve_slice(&self, remote: &str, digest: &str, offset: u64) -> Result<Response, String> {
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
        // Written only when it changes: slices are asked several at once
        // (`PULL_AHEAD`), and a middle slice rewriting what it loaded
        // could undo the last slice's `done`.
        let last = offset + want as u64 >= offer.size;
        if offer.note_fetch(remote, last) {
            // Read again under one lock: two receivers fetching at once
            // each write the record, and one must not write back what it
            // loaded before the other's mark.
            static WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());
            let _held = WRITE.lock().unwrap_or_else(|p| p.into_inner());
            let mut now = crate::transfer::load_offer(self.root(), digest)?.unwrap_or_else(|| offer.clone());
            if now.note_fetch(remote, last) {
                crate::transfer::save_offer(self.root(), digest, &now)?;
            }
        }
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
        let dir = chat::board_dir(self.root(), &channel)
            .ok_or_else(|| msg::bad_channel_name(format_args!("{channel:?}")))?;
        // Found by digest (`road::landed`); a file not landed yet answers
        // the conversation folder an older khor used, which is not there.
        Ok(files
            .iter()
            .map(|f| crate::road::landed(self.root(), &dir, f).unwrap_or_else(|| crate::transfer::payload_path(&dir, f)))
            .collect())
    }

    /// **A conversation file landed now** (STORE-BRIEF phase 2): its
    /// receiver pulls it at once into the directory the message names
    /// (its receive location when none) — the same move the receiver
    /// starts by itself when the message arrives. Asked here, it is a
    /// retry: a move that failed or was cancelled starts again (from the
    /// bytes a failure kept), one running is waited for, one landed is
    /// answered. Asked from any machine; routed to the receiver, through
    /// the resident serve when one holds the key.
    ///
    /// Returns bytes actually moved this run, and where the files are
    /// **on the machine that received them** — which is this one only
    /// when the transfer was addressed here. See
    /// [`crate::proto::Response::Acted`] for why the paths travel
    /// instead of being worked out at this end.
    pub async fn take(&self, id: &crate::SessionId) -> Result<(u64, Vec<String>), String> {
        self.take_act(id, "take").await
    }

    /// Cancels a conversation file's move on its receiver: the pull ends
    /// before its next slice and nothing partial stays. Asked from any
    /// machine.
    pub async fn stop_take(&self, id: &crate::SessionId) -> Result<(), String> {
        self.take_act(id, "stop").await.map(|_| ())
    }

    async fn take_act(&self, id: &crate::SessionId, action: &str) -> Result<(u64, Vec<String>), String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::Take {
                session: id.0.clone(),
                action: action.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Accepted { moved, landed } => Ok((moved, landed)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self.take_with(&ep, id, action).await;
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let conn = self.dial_tunnel(&ep, machine).await?;
        Ok(crate::tunnel::Borrow::new(ep, conn))
    }

    /// Dials a borrow lease on an endpoint the caller keeps alive — the
    /// resident serve's own. Same dial as [`tunnel_to`], but the `Borrow`
    /// does not own the endpoint, because the serve outlives it.
    ///
    /// **In the resident, the lease is the exit's held tunnel wire**,
    /// fetched again for every pipe (`tunnel::Borrow::on_wire`): the wire
    /// is dialled once here, so an exit that cannot be reached is said
    /// now, and after that a wire retired under the lease is re-opened by
    /// the next pipe rather than ending it.
    async fn tunnel_on(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
    ) -> Result<crate::tunnel::Borrow, String> {
        let conn = self.dial_tunnel(ep, machine).await?;
        let Some(me) = self.me() else {
            return Ok(crate::tunnel::Borrow::on_shared(conn));
        };
        let machine = machine.to_owned();
        Ok(crate::tunnel::Borrow::on_wire(std::sync::Arc::new(move || {
            let (me, machine) = (me.clone(), machine.clone());
            Box::pin(async move {
                let ep = me
                    .current_ears()
                    .ok_or_else(|| msg::serve_gone_midway().to_string())?;
                me.dial_tunnel(&ep, &machine).await
            })
        })))
    }

    /// Resolves a machine and opens the tunnel connection to it on `ep`.
    /// The one recipe both borrow dialers share.
    pub(crate) async fn dial_tunnel(
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
        self.line_to(ep, &target, wires::Proto::Tunnel)
            .await
            .map(|line| line.conn)
            .map_err(|e| unreached_words(&channel, e))
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
    // The create's answers plus its token (`crate::once`); the same
    // reason as `open_on_with` for not bundling them.
    #[allow(clippy::too_many_arguments)]
    pub async fn open_on(
        &self,
        machine: &str,
        kind: &str,
        title: &str,
        cwd: &str,
        cmd: &[String],
        size: (u16, u16),
        once: Option<&str>,
    ) -> Result<crate::SessionId, String> {
        self.refuse_unless_it_hosts(machine)?;
        let op = ipc::Op::OpenOn {
            machine: machine.to_owned(),
            kind: kind.to_owned(),
            title: title.to_owned(),
            cwd: cwd.to_owned(),
            cmd: cmd.to_vec(),
            cols: size.0,
            rows: size.1,
            token: once.map(str::to_owned),
        };
        if let Some(reply) = self.via_serve(op).await {
            return match reply? {
                ipc::Reply::OpenedOn { session } => Ok(crate::SessionId(session)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self
            .open_on_with(&ep, machine, kind, title, cwd, cmd, size, once)
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
        once: Option<&str>,
    ) -> Result<crate::SessionId, String> {
        self.refuse_unless_it_hosts(machine)?;
        let op = ipc::Op::OpenAgentOn {
            machine: machine.to_owned(),
            dir: dir.to_owned(),
            title: title.to_owned(),
            form: form.to_owned(),
            agent: agent.to_owned(),
            token: once.map(str::to_owned),
        };
        if let Some(reply) = self.via_serve(op).await {
            return match reply? {
                ipc::Reply::OpenedOn { session } => Ok(crate::SessionId(session)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self
            .open_agent_on_with(&ep, machine, dir, title, form, agent, once)
            .await;
        ep.close().await;
        outcome
    }

    // The create's answers plus its token (`crate::once`); the same
    // reason as `open_on_with` for not bundling them.
    #[allow(clippy::too_many_arguments)]
    async fn open_agent_on_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        dir: &str,
        title: &str,
        form: &str,
        agent: &str,
        once: Option<&str>,
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
        let once = once.map(|token| {
            (
                "OpenAgentOnce",
                Request::OpenAgentOnce {
                    token: token.to_owned(),
                    dir: dir.to_owned(),
                    title: title.to_owned(),
                    form: form.to_owned(),
                    agent: agent.to_owned(),
                },
            )
        });
        self.ask_open(ep, &channel, req, once).await
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        // **The day only when it moved** (round NET-4): asked with the
        // stamp of the day held from the last answer; an unchanged day
        // is filled in from that. A refusal — an older khor that does
        // not know the question, or a column it cannot rank by — falls
        // through to the plain ask below, whose answer says which.
        let held = {
            let days = self.days_held.lock().unwrap_or_else(|p| p.into_inner());
            days.get(&home.hex()).cloned()
        };
        let since = Request::LoadSince {
            top,
            by,
            desc,
            day: held.as_ref().map(|(stamp, _)| *stamp).unwrap_or(0),
        };
        if let Ok(Response::LoadSince { top, day, cpu_day }) = self.ask(ep, &channel, &since).await {
            let cpu_day = match (cpu_day, held) {
                (Some(fresh), _) => Some(fresh),
                (None, Some((stamp, kept))) if stamp == day => Some(kept),
                _ => None,
            };
            if let Some(cpu_day) = cpu_day {
                self.days_held
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .insert(home.hex(), (day, cpu_day.clone()));
                return Ok(khor_core::Load { top, cpu_day });
            }
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
                let host = crate::satellite::Dest::of(&target).ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
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
    fn sponsored_host(&self, channel: &str) -> Result<crate::satellite::Dest, String> {
        self.devices_loaded()?
            .doc
            .by_name(channel)
            .and_then(|d| crate::satellite::Dest::of(&d))
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        // Nothing is opened on a phone (docs/IOS.md): not this one, and
        // not one the table says is a phone — said before a dial, in
        // words, rather than as 「设 KHOR_LAUNCHER」, which is advice for
        // a computer.
        let phone = if home == self.device() {
            !self.hosts()
        } else {
            self.devices()?
                .iter()
                .any(|d| d.name == channel && !d.hosts)
        };
        if phone {
            return Err(msg::launch_needs_a_computer(&channel));
        }
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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

    /// **The commands run on a machine, searched** (roadmap 2.6): read
    /// where the history is, as the search above, and only the hits cross
    /// the wire. A satellite keeps none (its shells are not khor's hosts),
    /// and a khor that predates the question answers that it has none.
    pub async fn history(
        &self,
        machine: &str,
        ask: &crate::commands::HistoryAsk,
    ) -> Result<Vec<crate::commands::HistoryHit>, String> {
        if let Some(reply) = self
            .via_serve(ipc::Op::HistoryOn {
                machine: machine.to_owned(),
                ask: ask.clone(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::History { hits } => Ok(hits),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.history_with(&ep, machine, ask).await;
        ep.close().await;
        out
    }

    async fn history_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        ask: &crate::commands::HistoryAsk,
    ) -> Result<Vec<crate::commands::HistoryHit>, String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let (file, ask) = (crate::commands::history_file(self.root()), ask.clone());
            return tokio::task::spawn_blocking(move || crate::commands::search_history(&file, &ask))
                .await
                .map_err(|e| e.to_string());
        }
        if self.sat_and_sponsor(&channel)?.is_some() {
            return Ok(Vec::new());
        }
        match self.ask(ep, &channel, &Request::History { ask: ask.clone() }).await? {
            Response::History { hits } => Ok(hits),
            // Older than the question: no history kept there.
            Response::Refused { .. } => Ok(Vec::new()),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// **One file's history on any machine** (round UX-4): its log, its
    /// blame, or a revision of it (`crate::git::answer`), asked of the
    /// machine holding the file — as [`Node::git_diff_on`] is.
    pub async fn git_on(&self, machine: &str, ask: &crate::git::GitAsk) -> Result<crate::git::GitAnswer, String> {
        if machine.is_empty() || machine == self.name() {
            let ask = ask.clone();
            return tokio::task::spawn_blocking(move || crate::git::answer(&ask))
                .await
                .map_err(|e| e.to_string())?;
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::GitOn {
                machine: machine.to_owned(),
                ask: ask.clone(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Git { answer } => Ok(answer),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.git_with(&ep, machine, ask).await;
        ep.close().await;
        out
    }

    async fn git_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        ask: &crate::git::GitAsk,
    ) -> Result<crate::git::GitAnswer, String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            let ask = ask.clone();
            return tokio::task::spawn_blocking(move || crate::git::answer(&ask))
                .await
                .map_err(|e| e.to_string())?;
        }
        // A satellite's git is read over ssh by its sponsor, a road only
        // the status and the gutter have; its history is said to be out
        // of reach rather than looking empty.
        if self.sat_and_sponsor(&channel)?.is_some() {
            return Err(msg::git_history_satellite().into());
        }
        match self.ask(ep, &channel, &Request::Git { ask: ask.clone() }).await? {
            Response::Git { answer } => Ok(answer),
            Response::Refused { why } if why.trim().is_empty() => Err(msg::git_predates().into()),
            Response::Refused { why } => Err(msg::git_refused(why)),
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.touched_with(&ep, machine, path).await;
        ep.close().await;
        out
    }

    /// Whether a directory on any machine is a checkout
    /// (`crate::git::is_checkout`).
    ///
    /// **Asked beside [`Node::git_status_on`], never after it** — the
    /// workspace row's corner is this answer (docs/SHELL.md 第五轮
    /// 第 6 条), and the status walk is the one that can be slow or
    /// fail. Routed exactly as [`Node::touched`] is, satellites
    /// included, because it is the same shape of question: one row,
    /// the machine holding the files.
    pub async fn checkout_on(&self, machine: &str, path: &str) -> Result<bool, String> {
        if machine.is_empty() || machine == self.name() {
            return Ok(crate::git::is_checkout(std::path::Path::new(path)));
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::CheckoutOn {
                machine: machine.to_owned(),
                path: path.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Checkout { yes } => Ok(yes),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.checkout_with(&ep, machine, path).await;
        ep.close().await;
        out
    }

    async fn checkout_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<bool, String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let dir = std::path::PathBuf::from(path);
            return tokio::task::spawn_blocking(move || crate::git::is_checkout(&dir))
                .await
                .map_err(|e| e.to_string());
        }
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        // A satellite routes to its sponsor, [`Node::touched`]'s road.
        if let Some(sp) = target.sponsor.clone() {
            let sat = target.id.clone();
            if sp == self.device_str() {
                let host = crate::satellite::Dest::of(&target).ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
                let path = path.to_owned();
                return tokio::task::spawn_blocking(move || {
                    crate::satellite::is_checkout(&host, &path)
                })
                .await
                .map_err(|e| e.to_string())?;
            }
            let sponsor = self
                .devices_loaded()?
                .doc
                .get(&sp)
                .ok_or_else(|| msg::machine_left_table(&channel))?;
            let req = Request::SatCheckout {
                sat,
                path: path.to_owned(),
            };
            return match self.ask(ep, &sponsor.name, &req).await? {
                Response::Checkout { yes } => Ok(yes),
                Response::Refused { why } => Err(why),
                other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
            };
        }
        match self
            .ask(
                ep,
                &channel,
                &Request::Checkout {
                    path: path.to_owned(),
                },
            )
            .await?
        {
            Response::Checkout { yes } => Ok(yes),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
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
                let host = crate::satellite::Dest::of(&target).ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
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
    /// **Which of these directories on `machine` changed since `since`**
    /// (round NET-4, `crate::watch`), held up to `wait` (at most
    /// `ipc::WATCH_WAIT`) for one to. Only a resident watches: with none, and for this machine
    /// itself or a satellite (a tree here is asked off this disk; a
    /// satellite has no watcher of khor's), the answer is `unavailable`,
    /// and the face asks the tree as before.
    pub async fn dirs_changed(
        &self,
        machine: &str,
        dirs: &[String],
        since: u64,
        wait: Duration,
    ) -> Result<crate::watch::DirChanges, String> {
        let unavailable = || crate::watch::DirChanges {
            seq: since,
            dirs: dirs.to_vec(),
            unavailable: true,
            unreported: Vec::new(),
        };
        if machine.is_empty() || machine == self.name() {
            return Ok(unavailable());
        }
        match self
            .via_serve(ipc::Op::WatchOn {
                machine: machine.to_owned(),
                dirs: dirs.to_vec(),
                since,
                wait_ms: wait.as_millis() as u64,
            })
            .await
        {
            Some(reply) => match reply? {
                ipc::Reply::DirsChanged { changes } => Ok(changes),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            },
            None => Ok(unavailable()),
        }
    }

    async fn watch_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        dirs: &[String],
        since: u64,
        wait: Duration,
    ) -> Result<crate::watch::DirChanges, String> {
        let (channel, home) = self.resolve(machine)?;
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        if home == self.device() || target.sponsor.is_some() {
            return Ok(crate::watch::DirChanges {
                seq: since,
                dirs: dirs.to_vec(),
                unavailable: true,
                unreported: Vec::new(),
            });
        }
        let line = self
            .line_to(ep, &target, wires::Proto::Main)
            .await
            .map_err(|e| unreached_words(&target.name, e))?;
        let watches = self.watches.clone();
        let id = target.id.clone();
        Ok(self
            .watches
            .wait(&target.id, dirs, since, wait, |n, fresh| {
                tokio::spawn(crate::watch::run(watches, id, n, line.conn.clone(), fresh))
            })
            .await)
    }

    /// What a file on any machine is now, without its bytes
    /// (`crate::files::stat`): `(size, mtime, digest)`, the digest empty
    /// for a file bigger than one slice.
    pub async fn stat_of(&self, machine: &str, path: &str) -> Result<(u64, u64, String), String> {
        if machine.is_empty() || machine == self.name() {
            return crate::files::stat(path);
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::StatOn {
                machine: machine.to_owned(),
                path: path.to_owned(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Stat {
                    total,
                    at_ms,
                    digest,
                } => Ok((total, at_ms, digest)),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.stat_with(&ep, machine, path).await;
        ep.close().await;
        out
    }

    async fn stat_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<(u64, u64, String), String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            return crate::files::stat(path);
        }
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        // A satellite's file is read off its sponsor's staged copy; there
        // is no cheaper question to ask over ssh, so the first slice is
        // the answer, and the digest is taken here.
        if target.sponsor.is_some() {
            let (total, at_ms, bytes) = self.read_at_with(ep, machine, path, 0).await?;
            let digest = if bytes.len() as u64 == total {
                crate::files::digest(&bytes)
            } else {
                String::new()
            };
            return Ok((total, at_ms, digest));
        }
        match self
            .ask(
                ep,
                &channel,
                &Request::Stat {
                    path: path.to_owned(),
                },
            )
            .await?
        {
            Response::Stat {
                total,
                at_ms,
                digest,
            } => Ok((total, at_ms, digest)),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
                    .and_then(|d| crate::satellite::Dest::of(&d))
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
                let host = crate::satellite::Dest::of(&target).ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
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
        // Up to `PULL_AHEAD` slices asked at once, appended in order.
        while (bytes.len() as u64) < want {
            let from = bytes.len() as u64;
            let offsets: Vec<u64> = (0..PULL_AHEAD)
                .map(|k| from + k * proto::SLICE)
                .take_while(|o| *o < want)
                .collect();
            let asks = offsets
                .iter()
                .map(|&at| self.read_at(machine, path, at))
                .collect();
            for (at, answer) in offsets.iter().zip(join_all(asks).await) {
                // A short slice: what was asked beyond it is asked again.
                if *at != bytes.len() as u64 {
                    break;
                }
                let (now_total, now_ms, more) = answer?;
                if now_total != total || now_ms != at_ms {
                    return Err(msg::file_moved_while_reading(path));
                }
                if more.is_empty() {
                    // The far side answered nothing while claiming there
                    // is more. Refused rather than looped: a loop that
                    // cannot make progress is the one failure a reader
                    // must not turn into a hang.
                    return Err(msg::file_moved_while_reading(path));
                }
                bytes.extend_from_slice(&more);
            }
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

    /// [`Self::reach`] for a face whose pipe to that session just ended
    /// (docs/IOS.md 第三步) — and the answer says whether to try again.
    ///
    /// **The same reach, not a second road**: the resident binds a pipe
    /// onto the far host on its own endpoint (a fresh one after
    /// [`resume`]), and the far host pours its ring to the new client as
    /// it does for every attacher — so what happened while the old pipe
    /// was dead is on the screen again. The difference is the answer:
    /// [`Rejoin::Unreached`] when the far machine was not reached (worth
    /// asking again, after the next [`resume`] or a beat), [`Rejoin::Gone`]
    /// when it answered and the session is not there.
    pub async fn rejoin(
        &self,
        machine: &str,
        session: &crate::SessionId,
    ) -> Result<(String, String), Rejoin> {
        let op = ipc::Op::Rejoin {
            machine: machine.to_owned(),
            session: session.0.clone(),
        };
        match self.via_serve(op).await {
            Some(Ok(ipc::Reply::Reaching { addr, cookie })) => Ok((addr, cookie)),
            Some(Ok(ipc::Reply::Unreached { why })) => Err(Rejoin::Unreached(why)),
            Some(Ok(ipc::Reply::Refused { why })) => Err(Rejoin::Gone(why)),
            Some(Ok(other)) => Err(Rejoin::Gone(msg::serve_non_answer(format_args!("{other:?}")))),
            // The resident did not answer the hand-off: its own ears are
            // what is down, which is the case for asking again.
            Some(Err(why)) => Err(Rejoin::Unreached(why)),
            None => Err(Rejoin::Unreached(msg::reach_needs_serve().into())),
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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

    /// **Moves between the shell prompts of a tmux row** (item 21,
    /// `LiveKind::jump_prompt`), on the machine the row lives on: `Act`
    /// "prompt-up" or "prompt-down" there, as closing is.
    pub async fn jump_prompt_anywhere(&self, id: &crate::SessionId, up: bool) -> Result<(), String> {
        if let Some(machine) = self.far_machine(id)? {
            let action = if up { "prompt-up" } else { "prompt-down" };
            let op = ipc::Op::ActOn {
                machine: machine.clone(),
                session: id.0.clone(),
                action: action.into(),
            };
            if let Some(reply) = self.via_serve(op).await {
                return match reply? {
                    ipc::Reply::Accepted { .. } => Ok(()),
                    ipc::Reply::Refused { why } => Err(why),
                    other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
                };
            }
            let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
                .await
                .map_err(|e| e.to_string())?;
            let outcome = self.act_on_with(&ep, &machine, &id.0, action).await;
            ep.close().await;
            return outcome;
        }
        let live = self.live.clone();
        let id = id.clone();
        tokio::task::spawn_blocking(move || live.jump_prompt(&id, up))
            .await
            .map_err(|e| e.to_string())?
    }

    /// **The wheel over a shell's own tmux client** (item 58,
    /// `LiveKind::tmux_wheel`), on the machine the row lives on: `Act`
    /// "wheel:<rows>" there. `Ok(false)` when no tmux client is in front,
    /// or a khor there predates the question: the face keeps the wheel.
    pub async fn tmux_wheel_anywhere(&self, id: &crate::SessionId, rows: i32) -> Result<bool, String> {
        if let Some(machine) = self.far_machine(id)? {
            let action = format!("wheel:{rows}");
            let op = ipc::Op::ActOn {
                machine: machine.clone(),
                session: id.0.clone(),
                action: action.clone(),
            };
            let reply = match self.via_serve(op).await {
                Some(reply) => match reply? {
                    ipc::Reply::Accepted { moved, .. } => Ok(moved > 0),
                    ipc::Reply::Refused { .. } => Ok(false),
                    other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
                },
                None => {
                    let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
                        .await
                        .map_err(|e| e.to_string())?;
                    let outcome = self.act_on_moved(&ep, &machine, &id.0, &action).await;
                    ep.close().await;
                    outcome.map(|moved| moved > 0).or(Ok(false))
                }
            };
            return reply;
        }
        let live = self.live.clone();
        let id = id.clone();
        tokio::task::spawn_blocking(move || live.tmux_wheel(&id, rows))
            .await
            .map_err(|e| e.to_string())?
    }

    /// **A tmux row's front pane history** (item 31,
    /// `LiveKind::tmux_history`), on the machine the row lives on: what a
    /// phone scrolls locally. A khor there that predates the question
    /// refuses it, and the face keeps tmux's own scrolling.
    pub async fn pane_history_anywhere(
        &self,
        id: &crate::SessionId,
        since: Option<crate::adaptor::tmux::HistoryAnchor>,
    ) -> Result<crate::adaptor::tmux::PaneHistory, String> {
        if let Some(machine) = self.far_machine(id)? {
            return self.pane_history(&machine, &id.0, since).await;
        }
        let live = self.live.clone();
        let id = id.clone();
        tokio::task::spawn_blocking(move || live.tmux_history(&id, since.as_ref()))
            .await
            .map_err(|e| e.to_string())?
    }

    /// [`Node::pane_history_anywhere`] asked of a named machine (`""` is
    /// this one), as [`Node::history`] is.
    pub async fn pane_history(
        &self,
        machine: &str,
        session: &str,
        since: Option<crate::adaptor::tmux::HistoryAnchor>,
    ) -> Result<crate::adaptor::tmux::PaneHistory, String> {
        let op = ipc::Op::PaneHistoryOn {
            machine: machine.to_owned(),
            session: session.to_owned(),
            since: since.clone(),
        };
        if let Some(reply) = self.via_serve(op).await {
            return match reply? {
                ipc::Reply::PaneHistory { history } => Ok(history),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.pane_history_with(&ep, machine, session, since).await;
        ep.close().await;
        out
    }

    async fn pane_history_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        session: &str,
        since: Option<crate::adaptor::tmux::HistoryAnchor>,
    ) -> Result<crate::adaptor::tmux::PaneHistory, String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let live = self.live.clone();
            let id = crate::SessionId(session.to_owned());
            return tokio::task::spawn_blocking(move || live.tmux_history(&id, since.as_ref()))
                .await
                .map_err(|e| e.to_string())?;
        }
        let req = Request::PaneHistory {
            session: session.to_owned(),
            since,
        };
        match self.ask(ep, &channel, &req).await? {
            Response::PaneHistory { history } => Ok(history),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    async fn act_on_with(&self, ep: &iroh::Endpoint, machine: &str, session: &str, action: &str) -> Result<(), String> {
        self.act_on_moved(ep, machine, session, action).await.map(|_| ())
    }

    /// [`Node::act_on_with`], keeping what the act says it moved (the
    /// wheel's "taken", item 58).
    async fn act_on_moved(&self, ep: &iroh::Endpoint, machine: &str, session: &str, action: &str) -> Result<u64, String> {
        let (channel, _) = self.resolve(machine)?;
        let req = Request::Act {
            session: session.to_owned(),
            action: action.to_owned(),
        };
        match self.ask(ep, &channel, &req).await? {
            Response::Acted { moved, .. } => Ok(moved),
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
            let (path, root) = (path.to_owned(), self.root().to_path_buf());
            let (at, entries, truncated) =
                tokio::task::spawn_blocking(move || listed_with_arrivals(&root, &path))
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
                let host = crate::satellite::Dest::of(&target).ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
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
            return match self
                .ask_on(
                    ep,
                    &sponsor,
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
        match self
            .ask_on(
                ep,
                &target,
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        let fell = crate::files::landing(path, dir)?;
        self.pull_into(ep, machine, path, fell, None, None, true).await
    }

    /// The pull into a landing already chosen. `job`, when given, is
    /// told the bytes as they land, and a job cancelled
    /// (`crate::road::stop`, item 71) stops before its next slice. The
    /// size is held to `cap` from the first slice; without `resume` a
    /// pull that stops for any reason leaves no part file behind — a
    /// drop is asked again by dropping again, not carried on
    /// (`crate::road::Parcel`).
    #[allow(clippy::too_many_arguments)]
    async fn pull_into(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
        fell: crate::files::Landing,
        job: Option<&crate::drops::Job>,
        cap: Option<u64>,
        resume: bool,
    ) -> Result<(u64, std::path::PathBuf), String> {
        use std::io::Write;
        let (channel, home) = self.resolve(machine)?;
        let stopped = job.map(|j| j.stopped.clone());
        let is_stopped = move || stopped.as_ref().is_some_and(|s| s.load(std::sync::atomic::Ordering::Relaxed));
        if home == self.device() {
            let (path, part, is_stopped) = (path.to_owned(), fell.part.clone(), is_stopped.clone());
            let moved = tokio::task::spawn_blocking(move || -> Result<u64, String> {
                // Through read_slice rather than fs::copy so the local
                // pull refuses exactly what the remote one refuses
                // (relative paths, directories).
                let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
                let mut offset = 0u64;
                loop {
                    if is_stopped() {
                        return Err(crate::drops::cancelled());
                    }
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
                let host = crate::satellite::Dest::of(&target).ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
                let (h, p, part) = (host, path.to_owned(), fell.part.clone());
                let moved =
                    tokio::task::spawn_blocking(move || crate::satellite::pull_file(&h, &p, &part))
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                        // scp says no size before it lands, so a drop's cap
                        // is held after — and a cancel, which one scp
                        // cannot hear.
                        .and_then(|moved| match cap {
                            Some(cap) if moved > cap => Err(msg::drop_too_big(cap >> 20, path)),
                            _ if is_stopped() => Err(crate::drops::cancelled()),
                            _ => Ok(moved),
                        });
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
        // The wire first, so a machine that cannot be reached is said
        // before a partial file is made for it. Each slice is then asked
        // on its own: a reading, so a wire that dies mid-file costs one
        // slice asked again on a fresh wire, not the whole pull.
        self.line_to(ep, &dial_to, wires::Proto::Main)
            .await
            .map_err(|e| unreached_words(&dial_to.name, e))?;
        // **A pull resumes from what it already has** (round NET-4), the
        // way an offer does from its `.partial`: the bytes so far stay in
        // the part file, and beside it what they are bytes *of* — which
        // machine, which path, and the (size, mtime) every slice was
        // checked against. A pull of the same file into the same place
        // carries on from there if the far file still answers to that
        // reading, and starts over if it does not.
        let note_path = PullNote::path_of(&fell.part);
        let held = PullNote::read(&note_path)
            .filter(|n| n.machine == dial_to.id && n.sat == sat && n.path == path)
            .and_then(|n| {
                let had = std::fs::metadata(&fell.part).ok()?.len();
                (had <= n.total).then_some((n, had))
            });
        let (mut out, mut contract, mut offset) = match held {
            Some((note, had)) => (
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(&fell.part)
                    .map_err(|e| e.to_string())?,
                Some((note.total, note.at_ms)),
                had,
            ),
            None => (
                std::fs::File::create(&fell.part).map_err(|e| e.to_string())?,
                None,
                0,
            ),
        };
        let resumed_from = offset;
        let ask = |at: u64| match &sat {
            Some(s) => Request::SatFetchPath {
                sat: s.clone(),
                path: path.to_owned(),
                offset: at,
            },
            None => Request::FetchPath {
                path: path.to_owned(),
                offset: at,
            },
        };
        // Why a pull stopped: the far machine not reached (the part file
        // stays, to be carried on from), or something the part cannot be
        // carried on from (it goes).
        enum Stop {
            Unreached(String),
            Spoiled(String),
        }
        let outcome: Result<u64, Stop> = 'pull: loop {
            // A drop cancelled (item 71): before the next slices are
            // asked, and its part file goes as a spoiled one does.
            if is_stopped() {
                break 'pull Err(Stop::Spoiled(crate::drops::cancelled()));
            }
            // The first slice alone until the file's (size, mtime) is
            // known; then up to `PULL_AHEAD` asked at once, written in
            // order (the offers' shape, `pull_one`).
            let total = contract.map(|(t, _)| t);
            let offsets: Vec<u64> = match total {
                None => vec![offset],
                Some(t) => (0..PULL_AHEAD)
                    .map(|k| offset + k * proto::SLICE)
                    .take_while(|o| *o < t)
                    .collect(),
            };
            if offsets.is_empty() {
                break Ok(offset);
            }
            let reqs: Vec<Request> = offsets.iter().map(|&at| ask(at)).collect();
            let asks = reqs.iter().map(|r| self.ask_on(ep, &dial_to, r)).collect();
            for (at, resp) in offsets.iter().zip(join_all(asks).await) {
                if *at != offset {
                    break;
                }
                match resp {
                    Ok(Response::PathSlice {
                        total,
                        at_ms,
                        bytes,
                    }) => {
                        match contract {
                            None => {
                                if let Some(cap) = cap
                                    && total > cap
                                {
                                    break 'pull Err(Stop::Spoiled(msg::drop_too_big(cap >> 20, path)));
                                }
                                if let Some(job) = job {
                                    job.total.store(total, std::sync::atomic::Ordering::Relaxed);
                                }
                                contract = Some((total, at_ms));
                                let _ = PullNote {
                                    machine: dial_to.id.clone(),
                                    sat: sat.clone(),
                                    path: path.to_owned(),
                                    total,
                                    at_ms,
                                }
                                .write(&note_path);
                            }
                            // The file changed since the bytes held were
                            // read: they are another file's. A resumed
                            // pull starts over; one under way says so.
                            Some(c) if c != (total, at_ms) => {
                                if offset == resumed_from && resumed_from > 0 {
                                    let _ = out.set_len(0);
                                    offset = 0;
                                    contract = None;
                                    continue 'pull;
                                }
                                break 'pull Err(Stop::Spoiled(msg::file_changed_mid_pull().into()));
                            }
                            Some(_) => {}
                        }
                        if bytes.is_empty() && total > 0 {
                            break 'pull Err(Stop::Spoiled(msg::empty_slice().into()));
                        }
                        if let Err(e) = out.write_all(&bytes) {
                            break 'pull Err(Stop::Spoiled(e.to_string()));
                        }
                        offset += bytes.len() as u64;
                        if let Some(job) = job {
                            job.moved.store(offset, std::sync::atomic::Ordering::Relaxed);
                        }
                        if offset >= total {
                            break 'pull Ok(offset);
                        }
                    }
                    Ok(Response::Refused { why }) => break 'pull Err(Stop::Spoiled(why)),
                    Ok(other) => {
                        break 'pull Err(Stop::Spoiled(msg::peer_non_answer(format_args!("{other:?}"))));
                    }
                    Err(e) => break 'pull Err(Stop::Unreached(e)),
                }
            }
        };
        drop(out);
        match outcome {
            Ok(total) => {
                std::fs::rename(&fell.part, &fell.dest).map_err(|e| e.to_string())?;
                let _ = std::fs::remove_file(&note_path);
                Ok((total - resumed_from.min(total), fell.dest))
            }
            Err(Stop::Unreached(why)) if resume => Err(why),
            Err(Stop::Unreached(why) | Stop::Spoiled(why)) => {
                let _ = std::fs::remove_file(&fell.part);
                let _ = std::fs::remove_file(&note_path);
                Err(why)
            }
        }
    }

    /// Where a file dropped onto a terminal lands on this machine: its
    /// receive directory (`Node::receive_dir`).
    pub fn drop_dir(&self) -> Result<std::path::PathBuf, String> {
        self.receive_dir()
    }

    /// **Puts a file where a session's machine can read it** (item 60,
    /// `crate::drops`) and answers its absolute path there. `from` is
    /// the machine that has the file and `to` the session's machine,
    /// each by name, the empty name meaning this machine; `path` is
    /// absolute on `from`. `on` is told how far it got after every ask.
    ///
    /// The same machine moves nothing. Otherwise the file lands in `to`'s
    /// drop directory, pulled there by `to` itself — this machine only
    /// orders it and asks after it, every [`DROP_ASK_EVERY`], until it
    /// has landed or failed.
    ///
    /// **`into`, a directory absolute on `to`** (item 60 part 5, 「放到当
    /// 前目录」, the shell's current directory): the file lands there in
    /// place of the drop directory, under the same naming, and a file
    /// already on `to` is copied there — unless it already sits in it.
    pub async fn drop_file(
        &self,
        from: &str,
        path: &str,
        to: &str,
        into: Option<&str>,
        on: impl FnMut(&crate::drops::DropState),
    ) -> Result<String, String> {
        self.drop_file_until(from, path, to, into, on, || false).await
    }

    /// [`Node::drop_file`], **cancelled when `stopped` says so** (item
    /// 71, the pill's ×): the machine that pulls is told to stop, by the
    /// job's token, and this answers `crate::drops::cancelled` at once —
    /// whatever that machine does with the order. A machine whose khor
    /// predates the order refuses it; its pull then lands quietly in its
    /// drop directory, and nothing of it is pasted here either.
    pub async fn drop_file_until(
        &self,
        from: &str,
        path: &str,
        to: &str,
        into: Option<&str>,
        mut on: impl FnMut(&crate::drops::DropState),
        stopped: impl Fn() -> bool,
    ) -> Result<String, String> {
        let token = fresh_hex()?;
        let op = || match into {
            None => ipc::Op::DropOn {
                token: token.clone(),
                from: from.to_owned(),
                path: path.to_owned(),
                to: to.to_owned(),
            },
            Some(dir) => ipc::Op::DropInto {
                token: token.clone(),
                from: from.to_owned(),
                path: path.to_owned(),
                to: to.to_owned(),
                dir: dir.to_owned(),
            },
        };
        let from_serve = |reply: Result<ipc::Reply, String>| match reply? {
            ipc::Reply::DropFile { state } => Ok(state),
            ipc::Reply::Refused { why } => Err(why),
            other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
        };
        // Through the resident when there is one: it holds the key's one
        // endpoint, and a pull this machine runs must outlive this call.
        // Else on an endpoint of this call's, kept for the whole drop.
        let first = async {
            match self.via_serve(op()).await {
                Some(reply) => (from_serve(reply), None),
                None => match endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0()).await {
                    Ok(e) => (self.drop_step_with(&e, &token, from, path, to, into).await, Some(e)),
                    Err(e) => (Err(e.to_string()), None),
                },
            }
        };
        // **A cancel during the first ask.** With no resident, a pull
        // that lands here runs inside that ask, in this process: its job
        // is told here, and the ask ends at the pull's next slice. It is
        // never dropped mid-pull — that would leave its part file.
        tokio::pin!(first);
        let (mut answer, ep) = loop {
            tokio::select! {
                a = &mut first => break a,
                () = tokio::time::sleep(DROP_STOP_LOOK) => {
                    if stopped() {
                        crate::drops::stop(&token);
                    }
                }
            }
        };
        let outcome = loop {
            match answer {
                Ok(state) => {
                    on(&state);
                    if state.done {
                        break Ok(state.dest);
                    }
                }
                Err(why) => break Err(why),
            }
            let next = tokio::time::Instant::now() + DROP_ASK_EVERY;
            while !stopped() && tokio::time::Instant::now() < next {
                tokio::time::sleep_until(next.min(tokio::time::Instant::now() + DROP_STOP_LOOK)).await;
            }
            if stopped() {
                break Err(crate::drops::cancelled());
            }
            answer = match &ep {
                Some(e) => self.drop_step_with(e, &token, from, path, to, into).await,
                None => match self.via_serve(op()).await {
                    Some(reply) => from_serve(reply),
                    None => Err(msg::drop_serve_gone().into()),
                },
            };
        };
        // **Cancelled: the machine that pulls is told** (item 71), by the
        // road the asks took. What it answers changes nothing here — an
        // older khor refuses the order, and its pull lands quietly.
        if stopped() && outcome.is_err() {
            let _ = match &ep {
                Some(e) => self.drop_stop_with(e, &token, to).await,
                None => match self.via_serve(ipc::Op::DropStop { token: token.clone(), to: to.to_owned() }).await {
                    Some(Ok(ipc::Reply::Noted)) => Ok(()),
                    Some(Ok(ipc::Reply::Refused { why })) => Err(why),
                    Some(Ok(other)) => Err(msg::serve_non_answer(format_args!("{other:?}"))),
                    Some(Err(why)) => Err(why),
                    None => Err(msg::drop_serve_gone().into()),
                },
            };
        }
        if let Some(ep) = ep {
            ep.close().await;
        }
        outcome
    }

    /// **The order to stop a drop** (item 71), on an endpoint the caller
    /// owns: the job `token` names on `to`, the machine that pulls. Here,
    /// it is told at once; another machine is asked (`Request::DropStop`).
    pub(crate) async fn drop_stop_with(&self, ep: &iroh::Endpoint, token: &str, to: &str) -> Result<(), String> {
        let (dst_channel, dst) = self.resolve_or_here(to)?;
        if dst == self.device() {
            crate::drops::stop(token);
            return Ok(());
        }
        match self
            .ask(
                ep,
                &dst_channel,
                &Request::DropStop {
                    token: token.to_owned(),
                },
            )
            .await?
        {
            Response::DropStopped => Ok(()),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// **Adds the machine at `to` from `machine`** (添加机器 通过 khor
    /// 主机, 2026-10-08 owner ruling b): that machine becomes the sponsor
    /// and runs the add over its own ssh road ([`crate::join`]); this
    /// one asks it every [`ADD_ASK_EVERY`] and hands each new mark to
    /// `on` as it comes. Key login only: a password is kept only on the
    /// machine that runs ssh and never crosses the wire, so a password
    /// road is refused here in one sentence.
    pub async fn add_on(
        &self,
        machine: &str,
        to: &crate::satellite::Dest,
        name: Option<&str>,
        satellite_only: bool,
        mut on: impl FnMut(crate::join::Step, crate::join::Mark),
    ) -> Result<crate::join::Added, String> {
        if to.password {
            return Err(msg::add_on_no_password(machine));
        }
        let req = Request::AddOverSsh {
            token: fresh_hex()?,
            host: to.host.clone(),
            port: to.port,
            jump: to.jump.clone(),
            name: name.map(str::to_owned),
            satellite_only,
        };
        let op = || match req.clone() {
            Request::AddOverSsh {
                token,
                host,
                port,
                jump,
                name,
                satellite_only,
            } => ipc::Op::AddOn {
                machine: machine.to_owned(),
                token,
                host,
                port,
                jump,
                name,
                satellite_only,
            },
            _ => unreachable!("built above"),
        };
        // Through the resident when there is one, else on an endpoint of
        // this call's, kept for the whole add.
        let mut ep: Option<iroh::Endpoint> = None;
        let mut seen = 0;
        let out = loop {
            let answer = match &ep {
                Some(e) => self.add_ask_with(e, machine, &req).await,
                None => match self.via_serve(op()).await {
                    Some(reply) => match reply {
                        Ok(ipc::Reply::Adding { run }) => Ok(run),
                        Ok(ipc::Reply::Refused { why }) => Err(why),
                        Ok(other) => Err(msg::serve_non_answer(format_args!("{other:?}"))),
                        Err(e) => Err(e),
                    },
                    None => {
                        let e = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
                            .await
                            .map_err(|e| e.to_string())?;
                        let a = self.add_ask_with(&e, machine, &req).await;
                        ep = Some(e);
                        a
                    }
                },
            };
            let run = match answer {
                Ok(run) => run,
                Err(why) => break Err(why),
            };
            for (s, m) in run.marks.iter().skip(seen) {
                on(*s, m.clone());
            }
            seen = run.marks.len();
            if let Some(done) = run.done {
                break done;
            }
            tokio::time::sleep(ADD_ASK_EVERY).await;
        };
        if let Some(e) = ep {
            e.close().await;
        }
        out
    }

    /// One ask of a routed add on an endpoint the caller owns.
    async fn add_ask_with(&self, ep: &iroh::Endpoint, machine: &str, req: &Request) -> Result<crate::join::Run, String> {
        let (channel, home) = self.resolve(machine)?;
        if home == self.device() {
            return Err(msg::add_on_self().to_owned());
        }
        match self.ask(ep, &channel, req).await? {
            Response::Adding { run } => Ok(run),
            Response::Refused { why } if why.contains("AddOverSsh") => Err(msg::add_on_predates(&channel)),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// One ask of a drop on an endpoint the caller owns: started by the
    /// first, read by every later one with the same token.
    async fn drop_step_with(
        &self,
        ep: &iroh::Endpoint,
        token: &str,
        from: &str,
        path: &str,
        to: &str,
        into: Option<&str>,
    ) -> Result<crate::drops::DropState, String> {
        let (src_channel, src) = self.resolve_or_here(from)?;
        let (dst_channel, dst) = self.resolve_or_here(to)?;
        if src == dst && into.is_none() {
            return Ok(crate::drops::DropState::there(path));
        }
        if dst == self.device() {
            return match into {
                Some(dir) if src == dst => self.copy_here(token, path, dir).await,
                _ => self.drop_here(ep, token, &src_channel, path, into).await,
            };
        }
        let req = match into {
            None => Request::DropFile {
                token: token.to_owned(),
                from: src.hex(),
                path: path.to_owned(),
            },
            Some(dir) => Request::DropFileInto {
                token: token.to_owned(),
                from: src.hex(),
                path: path.to_owned(),
                dir: dir.to_owned(),
            },
        };
        match self.ask(ep, &dst_channel, &req).await? {
            Response::DropFile {
                moved,
                total,
                dest,
                done,
            } => Ok(crate::drops::DropState {
                moved,
                total,
                dest,
                done,
            }),
            Response::Refused { why } if why.contains("DropFile") => Err(msg::drop_predates().into()),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// A drop that lands **here**: the job this token names, started on
    /// the first ask — a pull off `channel` into this machine's drop
    /// directory, on a task of its own in a resident so it outlives the
    /// ask (a one-shot runs it inside the first ask, which then answers
    /// the end).
    async fn drop_here(
        &self,
        ep: &iroh::Endpoint,
        token: &str,
        channel: &str,
        path: &str,
        into: Option<&str>,
    ) -> Result<crate::drops::DropState, String> {
        if !crate::once::valid(token) {
            return Err(msg::create_token_bad().into());
        }
        let dir = match into {
            Some(dir) => crate::road::Dest::Dir(crate::drops::named_dir(dir)?),
            None => crate::road::Dest::Receive,
        };
        let (job, fresh) = crate::drops::job(token);
        if fresh {
            match self.me() {
                Some(node) => {
                    let (ep, channel, path, job) = (ep.clone(), channel.to_owned(), path.to_owned(), job.clone());
                    tokio::spawn(async move { node.run_drop(&ep, &channel, &path, dir, &job).await });
                }
                None => self.run_drop(ep, channel, path, dir, &job).await,
            }
        }
        job.state()
    }

    /// A drop into `dir` of a file already on this machine (item 60 part
    /// 5): copied there under the drop's naming, as the job this token
    /// names, on a blocking thread — the file sits in `dir` already:
    /// nothing moves.
    async fn copy_here(&self, token: &str, path: &str, dir: &str) -> Result<crate::drops::DropState, String> {
        if !crate::once::valid(token) {
            return Err(msg::create_token_bad().into());
        }
        let dir = crate::drops::named_dir(dir)?;
        let (job, fresh) = crate::drops::job(token);
        if fresh {
            let (path, held) = (path.to_owned(), job.clone());
            let copied = tokio::task::spawn_blocking(move || crate::drops::copy_into(std::path::Path::new(&path), &dir, &held))
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
            job.finish(copied.map(|p| p.display().to_string()));
        }
        job.state()
    }

    /// One drop's pull, start to end, told to its job: the file road's
    /// direct mode (`crate::road`), capped, never carried on.
    async fn run_drop(&self, ep: &iroh::Endpoint, channel: &str, path: &str, into: crate::road::Dest, job: &crate::drops::Job) {
        let parcel = crate::road::Parcel {
            mode: crate::road::Mode::Direct,
            source: crate::road::Source::Path {
                machine: channel.to_owned(),
                path: path.to_owned(),
            },
            into,
            cap: Some(crate::drops::CAP),
            resume: false,
        };
        let landed = self.land(ep, &parcel, Some(job)).await;
        job.finish(landed.map(|l| l.path.display().to_string()));
    }

    /// **The file road's one landing** (`crate::road`, STORE-BRIEF
    /// foundation A): `parcel`'s bytes pulled onto this machine, into its
    /// receive directory or the directory named, told to `job` as they
    /// land. A path is pulled by slices (`FetchPath`) under the parcel's
    /// cap and resume; an offer by its digest (`Fetch`), verified before
    /// it gets its name and carried on from its partial. What lands in
    /// the receive directory is recorded by digest (`road::Receipt`).
    pub(crate) async fn land(
        &self,
        ep: &iroh::Endpoint,
        parcel: &crate::road::Parcel,
        job: Option<&crate::road::Job>,
    ) -> Result<crate::road::Landed, String> {
        use crate::road::{Dest, Source};
        let (dir, receive) = match &parcel.into {
            Dest::Receive => (self.receive_dir()?, true),
            Dest::Dir(d) => (d.clone(), false),
        };
        match &parcel.source {
            Source::Path { machine, path } => {
                let fell = crate::drops::landing(&dir, &crate::drops::name_of(path)?)?;
                let part = fell.part.clone();
                let out = self.pull_into(ep, machine, path, fell, job, parcel.cap, parcel.resume).await;
                let (moved, dest) = match out {
                    Ok(o) => o,
                    Err(why) => {
                        // Whatever stopped it, before the slices or during
                        // them: the part file reserved for it goes too,
                        // unless the parcel is carried on from.
                        if !parcel.resume {
                            let _ = std::fs::remove_file(PullNote::path_of(&part));
                            let _ = std::fs::remove_file(&part);
                        }
                        return Err(why);
                    }
                };
                let digest = if receive {
                    let (d, from) = (dest.clone(), self.resolve(machine).map(|(_, id)| id.hex()).unwrap_or_default());
                    let root = self.root().to_path_buf();
                    let name = crate::drops::name_of(path)?;
                    let mode = parcel.mode;
                    tokio::task::spawn_blocking(move || -> Option<String> {
                        let (digest, size) = crate::transfer::digest_file(&d).ok()?;
                        let r = crate::road::Receipt { digest: digest.clone(), name, path: d, size, from, at_ms: crate::road::now_ms(), mode };
                        crate::road::record(&root, &r).ok()?;
                        Some(digest)
                    })
                    .await
                    .ok()
                    .flatten()
                } else {
                    None
                };
                Ok(crate::road::Landed { moved, path: dest, digest })
            }
            Source::Offer { from, file } => {
                let home = self
                    .devices_loaded()?
                    .doc
                    .get(from)
                    .ok_or(msg::offerer_left_table())?;
                self.pull_offer(ep, &home, file, &dir, parcel.mode, job).await
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
    /// The round visits every machine in parallel (`Self::sync_peers`),
    /// so the offerer that is up answers in its own round's time whatever
    /// dark machines share the table with it.
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
        // One round with everyone, in parallel — so the case this exists
        // for (the offerer is up and wrote the summary a second ago) costs
        // that one machine's round, not a walk past every dark machine
        // that sorts ahead of it.
        let outcomes = self.sync_peers(ep, true).await;
        if let Ok(found) = self.find_transfer(msg_id) {
            return Ok(found);
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

    /// A take or a stop on an endpoint the caller owns: run here when
    /// this machine is the receiver, otherwise routed to the receiver's
    /// serve — 动作从哪台设备发都行 (docs/SESSION.md).
    async fn take_with(
        &self,
        ep: &iroh::Endpoint,
        id: &crate::SessionId,
        action: &str,
    ) -> Result<(u64, Vec<String>), String> {
        let Some(msg_id) = id.0.strip_prefix("transfer/") else {
            return Err(msg::no_such_transfer(&id.0));
        };
        // Which machine to route to is read off the message, so the wait
        // has to happen before the fork, not inside one arm of it.
        let (channel, _) = self.transfer_after_sync(ep, id, msg_id).await?;
        // A broadcast file is taken by whichever machine asks for it: each
        // one answers the offer for itself (STORE-BRIEF phase 3).
        if channel == self.name() || channel.starts_with(chat::WIDE) {
            return self.take_local(ep, id, action).await;
        }
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::recipient_not_in_table(&channel))?;
        let resp = self
            .ask_on(
                ep,
                &target,
                &Request::Act {
                    session: id.0.clone(),
                    action: action.to_owned(),
                },
            )
            .await?;
        match resp {
            Response::Acted { moved, landed } => Ok((moved, landed)),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    /// The take or the stop itself, only ever on the receiver. Never
    /// re-routes an incoming Act — what lands wrong is refused, or two
    /// serves could bounce one forever.
    async fn take_local(
        &self,
        ep: &iroh::Endpoint,
        id: &crate::SessionId,
        action: &str,
    ) -> Result<(u64, Vec<String>), String> {
        let Some(msg_id) = id.0.strip_prefix("transfer/") else {
            return Err(msg::no_such_transfer(&id.0));
        };
        // Also here, not only in `take_with`: an incoming Act lands
        // straight on this one (`handle`), and the receiver can be behind
        // on the very message it is asked about.
        let (channel, m) = self.transfer_after_sync(ep, id, msg_id).await?;
        let wide = channel.starts_with(chat::WIDE);
        if channel != self.name() && !wide {
            return Err(msg::wrong_recipient(&channel));
        }
        // The machine that offered it has the bytes already.
        if wide && m.from.id == self.device_str() {
            return Err(msg::own_broadcast_file().to_owned());
        }
        if action == "stop" {
            self.stop_sent(&channel, &m)?;
            return Ok((0, Vec::new()));
        }
        self.run_take(ep, &channel, &m, true).await
    }

    /// Whether a synced document is this machine's own conversation.
    fn is_own_chat(&self, doc: &str) -> bool {
        doc.strip_prefix("chat/")
            .is_some_and(|ch| self.resolve(self.name()).is_ok_and(|(own, _)| own == ch))
    }

    /// **The conversation files that arrived for this machine, pulled
    /// now** (STORE-BRIEF phase 2: 直接传，不用等接收): every file
    /// message in this machine's own conversation that another machine
    /// sent and whose move has not started is landed, each on a task of
    /// its own. Called by the resident when it starts and whenever a sync
    /// moves its conversation. A move that failed or was cancelled is
    /// not started again here — that is a person's retry.
    pub(crate) fn take_arrived(&self) {
        let (Some(n), Some(ep)) = (self.me(), self.current_ears()) else {
            return;
        };
        let Ok((channel, _)) = self.resolve(self.name()) else {
            return;
        };
        let Ok(log) = self.chat.log(&channel) else {
            return;
        };
        for m in log.messages {
            let new = matches!(&m.body, crate::MsgBody::Files(fs) if !fs.is_empty())
                && !m.retracted
                && m.fate.is_none()
                && m.from.id != self.device_str();
            if !new {
                continue;
            }
            let (n, ep, channel) = (n.clone(), ep.clone(), channel.clone());
            tokio::spawn(async move {
                if let Err(why) = n.run_take(&ep, &channel, &m, false).await {
                    eprintln!("{why}");
                }
            });
        }
    }

    /// One conversation file's move on its receiver, told to the
    /// message for every device to read (`khor_sync::chat::Fate`; in the
    /// broadcast, under this machine's own key, as an accepted offer) and to
    /// a job keyed by the message (`road::job`), so a second ask joins
    /// the running one instead of pulling twice. `asked`: a person asked
    /// (a retry, a delivery waiting on it) — an ended move starts again
    /// and a running one is waited for; otherwise (the move starting by
    /// itself) a move that has a job already is left alone.
    pub(crate) async fn run_take(
        &self,
        ep: &iroh::Endpoint,
        channel: &str,
        m: &chat::Message,
        asked: bool,
    ) -> Result<(u64, Vec<String>), String> {
        use khor_sync::chat::{Fate, Step};
        let crate::MsgBody::Files(files) = &m.body else {
            return Err(msg::no_such_transfer(&m.id));
        };
        let key = crate::road::sent_key(&m.id, self.device_str());
        let (mut job, fresh) = crate::road::job(&key);
        if !fresh {
            if !asked {
                return Ok((0, Vec::new()));
            }
            match job.state() {
                Ok(s) if s.done && s.dest.lines().all(|p| std::path::Path::new(p).is_file()) => {
                    return Ok((0, s.dest.lines().map(str::to_owned).collect()));
                }
                Ok(s) if !s.done => return wait_for(&job).await,
                // Ended without landing, or what landed is gone since: a
                // retry is a new move.
                _ => {
                    crate::road::forget_job(&key);
                    job = crate::road::job(&key).0;
                }
            }
        }
        // Landed already (a retry pressed after it arrived): the answer
        // as it stands, while every file is still where it landed.
        if let Some(f) = self.chat.fate_here(channel, m).filter(|f| f.step == Step::Landed && f.landed.len() == files.len())
            && f.landed.iter().all(|p| std::path::Path::new(p).is_file())
        {
            job.finish(Ok(f.landed.join("\n")));
            return Ok((0, f.landed.clone()));
        }
        let size: i64 = files.iter().map(|f| f.size.max(0)).sum();
        let tell = |step: Step, moved: i64, landed: Vec<String>, why: String| {
            let fate = Fate { step, moved, landed, why };
            if let Err(e) = self.chat.set_fate(channel, &m.id, &fate) {
                eprintln!("{e}");
            }
        };
        tell(Step::Moving, 0, Vec::new(), String::new());
        let pull = async {
            let into = if m.into.is_empty() {
                crate::road::Dest::Receive
            } else {
                let dir = crate::road::expand(self.root(), &m.into);
                if !dir.is_dir() {
                    return Err(msg::no_such_dir(dir.display()));
                }
                crate::road::Dest::Dir(dir)
            };
            let home = self
                .devices_loaded()?
                .doc
                .get(&m.from.id)
                .ok_or(msg::offerer_left_table())?;
            let mut moved = 0u64;
            let mut landed = Vec::with_capacity(files.len());
            for f in files {
                let parcel = crate::road::Parcel {
                    // A broadcast file waited for this machine's yes.
                    mode: if channel.starts_with(chat::WIDE) {
                        crate::road::Mode::Offer
                    } else {
                        crate::road::Mode::Direct
                    },
                    source: crate::road::Source::Offer {
                        from: home.id.clone(),
                        file: f.clone(),
                    },
                    into: into.clone(),
                    cap: None,
                    resume: true,
                };
                let got = self.land(ep, &parcel, Some(&job)).await?;
                moved += got.moved;
                landed.push(got.path.display().to_string());
            }
            Ok::<_, String>((moved, landed))
        };
        // The progress beat: the bytes on this machine, written to the
        // message at most once a second and only when the percent moved,
        // so a long pull costs the document at most a hundred small ops.
        tokio::pin!(pull);
        let mut beat = tokio::time::interval(Duration::from_secs(1));
        let mut said = 0i64;
        let outcome = loop {
            tokio::select! {
                out = &mut pull => break out,
                _ = beat.tick() => {
                    let now = job.moved.load(std::sync::atomic::Ordering::Relaxed) as i64;
                    if size > 0 && now * 100 / size != said * 100 / size {
                        said = now;
                        tell(Step::Moving, now, Vec::new(), String::new());
                    }
                }
            }
        };
        match &outcome {
            Ok((_, landed)) => {
                tell(Step::Landed, size, landed.clone(), String::new());
                job.finish(Ok(landed.join("\n")));
            }
            Err(_) if job.is_stopped() => {
                // A cancel keeps nothing: the bytes so far go with it.
                for f in files {
                    for p in [crate::road::partial_of(self.root(), &f.digest), crate::road::marker_of(self.root(), &f.digest)]
                        .into_iter()
                        .flatten()
                    {
                        let _ = fs::remove_file(p);
                    }
                }
                tell(Step::Stopped, 0, Vec::new(), String::new());
                job.finish(Err(crate::road::cancelled()));
            }
            Err(why) => {
                let moved = job.moved.load(std::sync::atomic::Ordering::Relaxed) as i64;
                tell(Step::Failed, moved, Vec::new(), why.clone());
                job.finish(Err(why.clone()));
            }
        }
        let _ = self.emit_row_of(&crate::transfer::TransferKind::session_id(&m.id));
        if channel.starts_with(chat::WIDE) {
            let _ = self.emit_row_of(&crate::SessionId(channel.to_owned()));
        }
        // Everyone else reads the new state at once, not at the next beat.
        self.nudge(ep);
        if job.is_stopped() && outcome.is_err() {
            return Err(crate::road::cancelled());
        }
        outcome
    }

    /// A cancel on the receiver: the running pull stops before its next
    /// slice and cleans up after itself (`run_take`); a move that has not
    /// started is marked cancelled so it does not start by itself.
    fn stop_sent(&self, channel: &str, m: &chat::Message) -> Result<(), String> {
        use khor_sync::chat::{Fate, Step};
        let key = crate::road::sent_key(&m.id, self.device_str());
        let (job, fresh) = crate::road::job(&key);
        let ended = job.state();
        if !fresh && ended.as_ref().is_ok_and(|s| !s.done) {
            job.stopped.store(true, std::sync::atomic::Ordering::Relaxed);
            return Ok(());
        }
        if fresh || ended.is_err() {
            job.stopped.store(true, std::sync::atomic::Ordering::Relaxed);
            job.finish(Err(crate::road::cancelled()));
        }
        if self.chat.fate_here(channel, m).is_none_or(|f| f.step != Step::Landed) {
            self.chat.set_fate(
                channel,
                &m.id,
                &Fate {
                    step: Step::Stopped,
                    moved: 0,
                    landed: Vec::new(),
                    why: String::new(),
                },
            )?;
        }
        Ok(())
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
        let endpoint = self.endpoint_file()?;
        // **An app is not this verb's to stop.** Its resident is the app
        // process itself: a TERM here was the whole window going, with
        // nothing on it to say why (round NET-FIX). The person closes it.
        if keeper.is_none()
            && let Some(e) = endpoint.as_ref().filter(|e| e.app && pid_running(e.pid))
        {
            return Err(msg::quit_app_resident(e.pid));
        }
        let inner = endpoint.map(|e| e.pid);
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

    /// The app's resident holding this home right now, by pid — `None`
    /// when the resident is a kept `khor serve`, or there is none. What
    /// keeps a second resident from starting beside the app (`khor serve`,
    /// `khor-serve-up`, the installer): two endpoints on one key knock
    /// each other off the relay (`endpoint::bind`).
    pub fn app_resident(&self) -> Option<u32> {
        self.endpoint_file()
            .ok()
            .flatten()
            .filter(|e| e.app && pid_running(e.pid))
            .map(|e| e.pid)
    }

    /// A kept `khor serve` holding this home, by its keeper's pid
    /// (`serve.pid`) — `None` when there is none. The app stops it on its
    /// way up (`src-tauri`'s `take_the_home`): one resident per home.
    pub fn kept_resident(&self) -> Option<u32> {
        fs::read_to_string(self.root().join(".khor").join("serve.pid"))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .filter(|&p| p != std::process::id() && pid_running(p))
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
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
        // **Asked beside the dial, read only if it times out** (UX-8):
        // whether any relay answers here is what tells 「no road out of
        // this machine」 — no network, or no relay it can reach — from
        // 「the machine that issued the ticket is not answering」, and the
        // two are checked in different places. Beside, so the answer is in
        // by the time the dial gives up rather than three seconds after.
        let probe = tokio::spawn(crate::doctor::any_relay_answers(relays.clone()));
        let dialled = self
            .dial_with(ep, &t.id, &t.direct, &relays, ALPN, DIAL_TIMEOUT)
            .await;
        let conn = match dialled {
            Ok(conn) => {
                probe.abort();
                conn
            }
            Err(DialFailure::TimedOut) => {
                return Err(if probe.await.unwrap_or(true) {
                    msg::inviter_unreachable_timeout().to_string()
                } else {
                    msg::no_relay_reached().to_string()
                });
            }
            Err(DialFailure::Refused(why)) => {
                probe.abort();
                return Err(msg::inviter_unreachable(why));
            }
        };
        let resp = request(
            &conn,
            &t.name,
            &Request::Pair {
                token: t.token,
                name: self.name().to_owned(),
                addrs: report_addrs,
                roster: serde_bytes::ByteBuf::from(self.roster_offer()),
            },
        )
        .await?;
        match resp {
            Response::Paired {
                name,
                devices,
                roster,
            } => {
                let bytes = B64.decode(devices).map_err(msg::far_table_not_base64)?;
                let mut loaded = self.devices_loaded()?;
                // The same rule the issuer just ran, from this side — so
                // both reach one verdict without a round trip to agree.
                self.settle_with(&loaded.doc, &roster, &t.id, true)?;
                loaded.doc.merge(&bytes)?;
                let signed = self.enroll_after_pair(
                    &loaded.doc,
                    &[(t.id.as_str(), name.as_str()), (self.device_str(), self.name())],
                )?;
                loaded.store.flush(&loaded.doc)?;
                // This side holds the root and just signed the issuer in
                // (a server that issued the ticket, CONNECT.md 新设备入网三
                // 路): it learns its certificate the way it learns
                // anything, one devices round over this same connection.
                // Best effort — the next sync carries it anyway.
                if signed {
                    let line = Line {
                        id: t.id.clone(),
                        name: name.clone(),
                        proto: wires::Proto::Main,
                        conn: conn.clone(),
                    };
                    let _ = self.rounds(&line, "devices", &mut loaded).await;
                }
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
        // spelling: `sync_peers` reports the channel, and on a
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

        // 2 — the table, down to this machine. The network's trust goes
        // with it (`Node::leave_trust`: a holder keeps the root, which is
        // the network's and not this machine's).
        {
            let loaded = self.devices_loaded()?;
            loaded.doc.keep_only(self.device_str())?;
            loaded.doc.set_gone(self.device_str(), false)?;
            self.leave_trust(&loaded.doc)?;
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

    /// **Each machine's link as the resident holds it**: the road its
    /// wire is on and how long since it was last heard
    /// (`khor_core::Link`).
    ///
    /// **No fallback to a fresh endpoint, unlike every other verb here.**
    /// `sync_now` binds its own when no resident holds the key, because
    /// syncing from a new endpoint is still syncing. A *reading* taken
    /// from a new endpoint is not the same reading: it holds no wire, so
    /// every machine would come back not connected — a confident, wrong,
    /// and completely plausible answer. With no resident the answer is
    /// [`khor_core::Hop::NoServe`], which has a remedy in it.
    ///
    /// **A resident that does not answer in time keeps the last reading**
    /// (aged by the time since), rather than painting every row with a
    /// word for "the question failed" (audit/net.md F4: that was half of
    /// the owner's 「unknown」). Only with no reading at all does this
    /// say [`RoadsUnread::Unanswered`], and a face then draws no word.
    pub async fn links(&self) -> Result<Vec<(String, khor_core::Link)>, RoadsUnread> {
        match self.via_serve(ipc::Op::Hops).await {
            Some(Ok(ipc::Reply::Hops { by_device })) => {
                last_links::keep(self.root(), &by_device);
                Ok(by_device)
            }
            None => Ok(self
                .devices()
                .unwrap_or_default()
                .into_iter()
                .filter(|d| d.id != self.device_str())
                .map(|d| {
                    (
                        d.id,
                        khor_core::Link {
                            hop: khor_core::Hop::NoServe,
                            heard_ms: None,
                            rtt_ms: None,
                            relay: None,
                            down_ms: None,
                            last_rtt_ms: None,
                            since_ms: None,
                            last_hop: None,
                            via: None,
                        },
                    )
                })
                .collect()),
            Some(_) => last_links::recall(self.root()).ok_or(RoadsUnread::Unanswered),
        }
    }

    /// The resident's own answer to [`Self::links`]: every machine in the
    /// table but this one, read off the wires.
    pub(crate) fn links_now(&self) -> Vec<(String, khor_core::Link)> {
        use std::sync::atomic::Ordering::Relaxed;
        self.devices()
            .unwrap_or_default()
            .into_iter()
            .filter(|d| d.id != self.device_str())
            .map(|d| {
                let r = self.wires.link(&d.id);
                let hop = match r.road {
                    Some(khor_net::path::PathKind::Direct) => {
                        self.wires.counts.read_direct.fetch_add(1, Relaxed);
                        khor_core::Hop::Direct
                    }
                    Some(khor_net::path::PathKind::Relay) => {
                        self.wires.counts.read_relay.fetch_add(1, Relaxed);
                        khor_core::Hop::Relay
                    }
                    Some(khor_net::path::PathKind::Carried) => self.carried_hop(&d.id),
                    _ => {
                        if r.down.is_some() {
                            self.wires.counts.read_down.fetch_add(1, Relaxed);
                        } else {
                            self.wires.counts.read_idle.fetch_add(1, Relaxed);
                        }
                        khor_core::Hop::Idle
                    }
                };
                // The road the last settled wire was on, live or gone: an
                // idle row says it (UX-48), dated by `since_ms`.
                let last_hop = r.last_road.and_then(|k| match k {
                    khor_net::path::PathKind::Direct => Some(khor_core::Hop::Direct),
                    khor_net::path::PathKind::Relay => Some(khor_core::Hop::Relay),
                    khor_net::path::PathKind::Carried => Some(self.carried_hop(&d.id)),
                    _ => None,
                });
                // The middle of a peer hop (round ROAD-3).
                let via = match d.id.parse().ok().and_then(|p| self.carry.carrier(&p)) {
                    Some(khor_net::carry::Carrier::Hop { via }) if hop == khor_core::Hop::Forwarded => {
                        Some(via.to_string())
                    }
                    _ => None,
                };
                let ms = |d: Duration| u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
                // Whole milliseconds, never 0 for a wire that has one: a
                // loopback wire's 0.3 ms reads as 1 ms, not as nothing.
                let whole = |t: Duration| (t.as_micros().div_ceil(1000)).min(u128::from(u32::MAX)) as u32;
                (
                    d.id,
                    khor_core::Link {
                        hop,
                        heard_ms: r.heard.map(ms),
                        rtt_ms: r.rtt.map(whole),
                        relay: r.relay,
                        down_ms: r.down.map(ms),
                        last_rtt_ms: r.last_rtt.map(whole),
                        since_ms: last_hop.and(r.since).map(ms),
                        last_hop,
                        via,
                    },
                )
            })
            .collect()
    }

    /// **This machine's connections, as the 网络 page shows them**
    /// (`khor_core::ConnectionsView`). Only the resident can answer — the
    /// wires and the home relay are its.
    pub async fn connections(&self) -> Result<khor_core::ConnectionsView, RoadsUnread> {
        match self.via_serve(ipc::Op::Connections).await {
            Some(Ok(ipc::Reply::Connections { view })) => Ok(view),
            None => Err(RoadsUnread::NoServe),
            Some(_) => Err(RoadsUnread::Unanswered),
        }
    }

    /// The resident's own answer to [`Self::connections`].
    async fn connections_now(&self, ep: &iroh::Endpoint) -> khor_core::ConnectionsView {
        use iroh::Watcher;
        let home = ep.home_relay_status().get().into_iter().next();
        let urls = self.relays().to_vec();
        let measured = join_all(
            urls.iter()
                .map(|u| async move { crate::doctor::relay_round_trip(u).await })
                .collect(),
        )
        .await;
        let relays = urls
            .into_iter()
            .zip(measured)
            .map(|(url, rtt_ms)| khor_core::RelayReading { url, rtt_ms })
            .collect();
        let links = self.links_now();
        let peers = links
            .into_iter()
            .map(|(id, link)| {
                let (rtt, failure, failed, unanswered) = self.wires.peer_view(&id);
                let ms = |d: Duration| u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
                khor_core::PeerConnection {
                    id,
                    hop: link.hop,
                    heard_ms: link.heard_ms,
                    rtt_ms: rtt.filter(|r| !r.is_zero()).map(ms),
                    failure_ago_ms: failure.as_ref().map(|(_, ago)| ms(*ago)),
                    last_failure: failure.map(|(w, _)| w),
                    failed,
                    unanswered,
                }
            })
            .collect();
        khor_core::ConnectionsView {
            home_connected: home.as_ref().is_some_and(|h| h.is_connected()),
            home_error: home
                .as_ref()
                .and_then(|h| h.last_error())
                .map(|e| e.to_string()),
            home_relay: home.map(|h| h.url().to_string()),
            preferred_relay: self.preferred_relay(),
            relays,
            peers,
        }
    }

    /// **A round now, out of turn, at most once a [`SYNC_EVERY`]** (round
    /// NET-6): for a face arriving, a machine just paired, and a stranger
    /// knocking — the three moments the quiet pace would otherwise make
    /// somebody wait up to [`QUIET_PACE`] for what a round brings. The
    /// limit keeps a stranger who knocks on every beat from buying more
    /// than the pace a face gets.
    pub(crate) fn nudge(&self, ep: &iroh::Endpoint) {
        use std::sync::atomic::Ordering::Relaxed;
        let now = crate::now_ms();
        let last = self.nudged.load(Relaxed);
        if now.saturating_sub(last) < SYNC_EVERY.as_millis() as u64
            || self.nudged.compare_exchange(last, now, Relaxed, Relaxed).is_err()
        {
            return;
        }
        if let Some(n) = self.me() {
            let e = ep.clone();
            tokio::spawn(async move {
                n.wires.counts.pump_rounds.fetch_add(1, Relaxed);
                let _ = n.sync_peers(&e, false).await;
            });
        }
    }

    /// The workspaces this machine keeps facts for, made the table's own
    /// (`crate::wsfacts`, round NET-8): the resident's tidy beat.
    pub(crate) fn reconcile_workspace_facts(&self) {
        let Some(keeper) = self.wsfacts.lock().unwrap_or_else(|p| p.into_inner()).clone() else {
            return;
        };
        let Ok(all) = self.workspaces() else { return };
        let mine: Vec<String> = all
            .into_iter()
            .filter(|w| w.device == self.device_str())
            .map(|w| w.path)
            .collect();
        keeper.reconcile(&mine);
    }

    /// One machine's workspace facts as this machine last knew them —
    /// its own resident's for this machine, the last round's for another
    /// (`crate::wsfacts`). Empty when there are none yet.
    pub fn workspace_facts_of(&self, device: &str) -> Vec<khor_core::WorkspaceFact> {
        crate::wsfacts::read(self.root(), device).map(|k| k.facts).unwrap_or_default()
    }

    /// **Whether a face is on screen** (`ipc::Op::FaceHere`): one said so
    /// within three of its beats.
    pub(crate) fn face_here(&self) -> bool {
        self.face_seen
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_some_and(|at| at.elapsed() < ipc::FACE_BEAT * 3)
    }

    /// A face says it is on screen, or stopped being (`ipc::Op::FaceHere`).
    /// Nothing to tell without a resident.
    pub async fn face_is_here(&self, visible: bool) -> Result<(), RoadsUnread> {
        match self.via_serve(ipc::Op::FaceHere { visible }).await {
            Some(Ok(ipc::Reply::Noted)) => Ok(()),
            None => Err(RoadsUnread::NoServe),
            Some(_) => Err(RoadsUnread::Unanswered),
        }
    }

    /// Retires the resident's wires to `machine` (`ipc::Op::LetGo`).
    pub async fn let_go_of(&self, machine: &str) -> Result<khor_core::WireCounts, RoadsUnread> {
        match self
            .via_serve(ipc::Op::LetGo {
                machine: machine.to_owned(),
            })
            .await
        {
            Some(Ok(ipc::Reply::Wires { counts })) => Ok(counts),
            None => Err(RoadsUnread::NoServe),
            Some(_) => Err(RoadsUnread::Unanswered),
        }
    }

    /// How the resident's wires have fared since it started
    /// (`khor_core::WireCounts`), for `khor doctor`. Only the resident
    /// can answer, for the reason [`Self::links`] gives.
    pub async fn wire_counts(&self) -> Result<khor_core::WireCounts, RoadsUnread> {
        match self.via_serve(ipc::Op::Wires).await {
            Some(Ok(ipc::Reply::Wires { counts })) => Ok(counts),
            None => Err(RoadsUnread::NoServe),
            Some(_) => Err(RoadsUnread::Unanswered),
        }
    }

    /// How the resident's recent dials went, per far machine
    /// (`khor_core::RoadTally`). Machines it has not dialled since it
    /// started are absent — that is「no reading」, not「zero」.
    ///
    /// **Only the resident can answer**, for the reason [`Node::hops`]
    /// gives: a one-shot verb has dialled nothing. `Err` says which of
    /// the two silences it is, because only one has a remedy: no serve
    /// at all ([`RoadsUnread::NoServe`]), or a serve that did not
    /// understand the question — one older than this op, which decodes
    /// it as garbage and answers `Refused` — or did not answer in time
    /// ([`RoadsUnread::Unanswered`]).
    pub async fn road_tallies(&self) -> Result<Vec<(String, khor_core::RoadTally)>, RoadsUnread> {
        match self.via_serve(ipc::Op::Roads).await {
            Some(Ok(ipc::Reply::Roads { by_device })) => Ok(by_device),
            None => Err(RoadsUnread::NoServe),
            Some(_) => Err(RoadsUnread::Unanswered),
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
        let ep = endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.sync_peers(&ep, true).await;
        ep.close().await;
        Ok(out)
    }

    /// **One visit to every machine, in parallel, each on its own wire
    /// and budget.** A machine already in a round — started by the
    /// pump's last beat, or by somebody else's 同步 — is joined, not
    /// visited twice. A machine whose dials keep failing is left alone
    /// until its backoff is over (`crate::wires`: 5 s doubling to a
    /// minute) unless `force`, which is a person asking, a wake, or a
    /// return from the background.
    ///
    /// ⚠️ 墓碑 (2026-10-03, audit/net.md F8): the round used to visit the
    /// machines one after another, each dark one costing its full 10 s
    /// dial before the next was asked, under one gate an explicit sync
    /// queued behind — a phone in someone's pocket cost the desk ten
    /// seconds of every round.
    pub(crate) async fn sync_peers(
        &self,
        ep: &iroh::Endpoint,
        force: bool,
    ) -> Vec<(String, Result<String, String>)> {
        // Backgrounded: a round already under way when the app went
        // stops here rather than redialling what the suspension let go.
        if self.resting.load(std::sync::atomic::Ordering::Relaxed) {
            return Vec::new();
        }
        let list: Vec<DeviceInfo> = self
            .devices_loaded()
            .map(|l| l.doc.all())
            .unwrap_or_default()
            .into_iter()
            .filter(|d| d.id != self.device_str())
            .collect();
        let visits = list
            .iter()
            .map(|d| async move { (d.name.clone(), self.round_with(ep, d, force).await) })
            .collect();
        join_all(visits).await
    }

    /// A round with one machine by id, now — backoff or not. Nothing for
    /// a key not in the table (a stranger pairing).
    async fn visit_now(&self, ep: &iroh::Endpoint, id: &str) {
        let Ok(loaded) = self.devices_loaded() else { return };
        let Some(d) = loaded.doc.get(id) else { return };
        drop(loaded);
        let _ = self.round_with(ep, &d, true).await;
    }

    /// One machine's round: skipped with a reason, joined, or run.
    async fn round_with(
        &self,
        ep: &iroh::Endpoint,
        d: &DeviceInfo,
        force: bool,
    ) -> Result<String, String> {
        if self.resting.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(msg::round_cancelled().into());
        }
        // A device that never reported a road (one-shot pairings) cannot
        // be dialed, only dial us — probing it burns a full DIAL_TIMEOUT
        // for nothing.
        if d.addrs.is_empty() {
            return Err(msg::no_roads_reported().into());
        }
        // Not on the roster (waiting, or revoked): nothing is pulled from
        // it or pushed to it. Its own requests are refused at the gate;
        // this is the same rule from the dialing side.
        if !self.standing_of(&d.id).admitted() {
            return Err(msg::peer_not_in_roster().into());
        }
        if !force && let Some(left) = self.wires.backing_off(&d.id) {
            return Err(msg::peer_backing_off(left.as_secs().max(1)));
        }
        // Its relay said it left: it will dial when it is back
        // (`wires::LEFT_ALONE_FOR`).
        if !force && self.wires.left_alone(&d.id) {
            return Err(msg::peer_left_its_relay().into());
        }
        let asked = std::time::Instant::now();
        loop {
            let joined = {
                let mut rounds = self.rounds.lock().unwrap_or_else(|p| p.into_inner());
                match rounds.get(&d.id) {
                    Some((began, running)) if running.borrow().is_none() => {
                        Ok((*began, running.clone()))
                    }
                    _ => {
                        let (tx, rx) = tokio::sync::watch::channel(None);
                        rounds.insert(d.id.clone(), (std::time::Instant::now(), rx));
                        Err(RoundDone(Some(tx)))
                    }
                }
            };
            match joined {
                Ok((began, mut running)) => {
                    let out = loop {
                        if let Some(out) = running.borrow().clone() {
                            break out;
                        }
                        if running.changed().await.is_err() {
                            break Err(msg::round_cancelled().into());
                        }
                    };
                    // **A person's 同步 carries what changed before they
                    // asked.** A round that began earlier may have read
                    // the store before the change (a machine paired a
                    // moment ago), so its end is where theirs starts,
                    // not their answer. The pump joins as before.
                    if force && began < asked {
                        continue;
                    }
                    return out;
                }
                Err(mut done) => {
                    let out = {
                        let _store = self.store_gate.read().await;
                        self.sync_device(ep, d).await
                    };
                    done.finish(out.clone());
                    return out;
                }
            }
        }
    }

    async fn sync_device(&self, ep: &iroh::Endpoint, d: &DeviceInfo) -> Result<String, String> {
        let unreached = |e| match e {
            DialFailure::TimedOut => msg::dial_timed_out().to_string(),
            DialFailure::Refused(why) => msg::cant_reach_plain(why),
        };
        // **The round's opening question rides a redial's handshake**
        // (`Node::ask_early`): it is a reading, so after a wake or a
        // dropped wire the round's first answer comes a round trip sooner.
        let (kinds, opening) = self.opening(d)?;
        let early = match self.ask_early(ep, d, &opening).await {
            Early::Answered(resp) => Some(resp),
            Early::Failed(e) => return Err(unreached(e)),
            Early::NotTaken => None,
        };
        let line = self
            .line_to(ep, d, wires::Proto::Main)
            .await
            .map_err(unreached)?;
        let moved = match self.sync_what_changed(&line, d, &kinds, &opening, early).await? {
            Some(moved) => moved,
            // Refused at the door by a machine this one's roster admits:
            // its table may have lost this machine (round NET-10).
            None if self.vouch_to(&line, d).await => {
                match self.sync_what_changed(&line, d, &kinds, &opening, None).await? {
                    Some(moved) => moved,
                    None => self.sync_everything(&line, d).await?,
                }
            }
            None => self.sync_everything(&line, d).await?,
        };
        Ok(if moved == 0 {
            msg::nothing_to_move().into()
        } else {
            msg::moved_bytes(moved)
        })
    }

    /// **Vouches for this machine to one that refused a round's
    /// opening** ([`Request::Vouch`], round NET-10), and answers whether
    /// it was taken. A machine on this side's roster that refuses every
    /// request is most often one whose table lost its rows, and it cannot
    /// bring them back by itself (`Node::take_vouch` has the case).
    ///
    /// At most once per [`VOUCH_EVERY`] for each machine: a refusal for
    /// any other reason — an older khor that does not know the opening,
    /// a revocation this side has not heard of — costs one small frame
    /// per interval, not one per round.
    async fn vouch_to(&self, line: &Line, d: &DeviceInfo) -> bool {
        {
            let mut last = self.vouched.lock().unwrap_or_else(|p| p.into_inner());
            if last.get(&d.id).is_some_and(|t| t.elapsed() < VOUCH_EVERY) {
                return false;
            }
            last.insert(d.id.clone(), std::time::Instant::now());
        }
        let Some((name, addrs, roster)) = self.vouch_offer() else {
            return false;
        };
        let ask = Request::Vouch {
            name,
            addrs,
            roster: serde_bytes::ByteBuf::from(roster),
        };
        matches!(self.exchange_on(line, &ask).await, Ok(Response::Vouched))
    }

    /// **A round by what changed** (round NET-3). One exchange first,
    /// [`Request::Versions`]: a digest of every document's version and
    /// of every reading this node holds of that machine. Then only the
    /// documents whose versions differ, one exchange each (the far
    /// version is known, so the first exchange already pushes), and the
    /// readings that changed came back in the answer itself. A quiet
    /// round is the one exchange.
    ///
    /// The shape is the one CouchDB (`_revs_diff`), Syncthing (index id
    /// and max sequence before an index update) and Automerge (heads
    /// before changes) share: say where you are, then send only the
    /// difference. Unlike theirs, nothing here is remembered between
    /// rounds — a version is a digest of the document itself, so a
    /// restart, a rejoin or a second asker cannot leave a stale marker
    /// that under-sends.
    ///
    /// `None` when that machine's khor predates the question: the round
    /// is then [`Node::sync_everything`], as before.
    async fn sync_what_changed(
        &self,
        line: &Line,
        d: &DeviceInfo,
        kinds: &[String],
        opening: &Request,
        early: Option<Response>,
    ) -> Result<Option<usize>, String> {
        let answer = match early {
            Some(answer) => answer,
            None => self.exchange_on(line, opening).await.map_err(Miss::words)?,
        };
        let Some((differ, changed)) = versions_answer(answer)? else {
            return Ok(None);
        };
        self.take_readings(line, d, kinds, changed).await;

        let mut moved = 0usize;
        for (name, theirs) in differ {
            let synced = with_doc!(self, name.as_str(), |loaded| self
                .rounds_from(line, &name, loaded, wire::Peer::knowing(theirs), 1)
                .await);
            match synced {
                Ok(n) => {
                    moved += n;
                    if name == "devices" && n > 0 {
                        self.follow_after_sync();
                    }
                    if n > 0 && self.is_own_chat(&name) {
                        self.take_arrived();
                    }
                }
                Err(why) if doc_must_sync(&name) => return Err(why),
                Err(_) => {}
            }
        }
        // **Readings that the documents just moved can change are asked
        // again.** Over there a row's word is derived from documents this
        // round carries — a watermark this machine just pushed is what
        // turns 完成 into 空闲 — and the readings above were taken before
        // it arrived. Only after a round that moved something: a quiet
        // round stays the one exchange.
        if moved > 0 {
            let again = self.versions_request(d, kinds, Vec::new());
            let answer = self.exchange_on(line, &again).await.map_err(Miss::words)?;
            if let Some((_, changed)) = versions_answer(answer)? {
                self.take_readings(line, d, kinds, changed).await;
            }
        }
        Ok(Some(moved))
    }

    /// A round's opening [`Request::Versions`] for `d`: every document's
    /// digest and every reading held, and the readings' kinds.
    fn opening(&self, d: &DeviceInfo) -> Result<(Vec<String>, Request), String> {
        let mut names: Vec<String> = ROUND_DOCS.iter().map(|n| (*n).to_owned()).collect();
        names.extend(self.known_channels()?.into_iter().map(|ch| format!("chat/{ch}")));
        names.extend(self.known_wide());
        let mut docs = Vec::with_capacity(names.len());
        for name in &names {
            let digest = with_doc!(self, name.as_str(), |loaded| Ok::<_, String>(version_digest(
                &loaded.doc
            )))?;
            docs.push((name.clone(), serde_bytes::ByteBuf::from(digest)));
        }
        let mut kinds: Vec<String> = ["sessions", "vitals", "watched", "days", "quotas", "usage"]
            .iter()
            .map(|k| (*k).to_owned())
            .collect();
        // **Asked only of a machine that holds a workspace** (round
        // NET-8): the table says which, on this disk. Named on every
        // round regardless, the reading's name and digest cost a phone
        // 0.5 KB a minute at idle for a machine with nothing to say.
        if self
            .workspaces_loaded()
            .map(|l| {
                l.doc
                    .all()
                    .into_iter()
                    .any(|(k, _)| khor_sync::workspaces::split(&k).is_some_and(|(dev, _)| dev == d.id))
            })
            .unwrap_or(false)
        {
            kinds.push("workspace_facts".to_owned());
        }
        if let Ok(devs) = self.devices() {
            kinds.extend(
                devs.iter()
                    .filter(|s| s.sponsor.as_deref() == Some(d.id.as_str()))
                    .map(|s| format!("sat/{}", s.id)),
            );
        }
        let opening = self.versions_request(d, &kinds, docs);
        Ok((kinds, opening))
    }

    /// A [`Request::Versions`]: these documents' digests and the digests
    /// of the readings held of `d`.
    fn versions_request(
        &self,
        d: &DeviceInfo,
        kinds: &[String],
        docs: Vec<(String, serde_bytes::ByteBuf)>,
    ) -> Request {
        let all = self.readings_held.lock().unwrap_or_else(|p| p.into_inner());
        let readings = kinds
            .iter()
            .map(|k| {
                let held = all.get(&(d.id.clone(), k.clone()));
                let digest = held.map(|h| h.digest.clone()).unwrap_or_default();
                // The day is asked as the minutes since the copy held.
                let asked = match held.map(|h| &h.answer) {
                    Some(Response::Days { day }) => match crate::day::Day::end_of_wire(day) {
                        Some(end) => format!("days@{end}"),
                        None => k.clone(),
                    },
                    _ => k.clone(),
                };
                (asked, serde_bytes::ByteBuf::from(digest))
            })
            .collect::<Vec<_>>();
        // **This asker takes the rows as a change** (round NET-6): an
        // answerer that knows the marker may send `SessionRowsSince`; one
        // that does not finds no request for it and skips it, answering
        // the rows whole as before.
        let mut readings = readings;
        if kinds.iter().any(|k| k == "sessions") {
            readings.push((ROWS_BY_CHANGE.to_owned(), serde_bytes::ByteBuf::new()));
        }
        Request::Versions { docs, readings }
    }

    /// The readings a [`Request::Versions`] answer carried, into the
    /// caches and the held set; every reading it did not name is the one
    /// already held, written again so its age is this round's.
    async fn take_readings(
        &self,
        line: &Line,
        d: &DeviceInfo,
        kinds: &[String],
        changed: Vec<(String, serde_bytes::ByteBuf)>,
    ) {
        let held: std::collections::HashMap<String, HeldReading> = {
            let all = self.readings_held.lock().unwrap_or_else(|p| p.into_inner());
            kinds
                .iter()
                .filter_map(|k| all.get(&(d.id.clone(), k.clone())).map(|h| (k.clone(), h.clone())))
                .collect()
        };
        let mut fresh: std::collections::HashMap<String, Option<HeldReading>> = Default::default();
        for (asked, frame) in changed {
            let kind = asked.split('@').next().unwrap_or_default().to_owned();
            if frame.is_empty() {
                fresh.insert(kind, None);
                continue;
            }
            let reading = match proto::decode::<Response>(&frame) {
                Ok(Response::SessionRowsSince { order, rows, full }) => {
                    // Rebuilt from the copy held and checked against the
                    // whole answer's digest; asked whole when they differ.
                    let rebuilt = match held.get(&kind).map(|h| &h.answer) {
                        Some(Response::SessionRows { rows: was }) => rows_rebuilt(was, &order, rows),
                        _ => None,
                    }
                    .map(|rows| Response::SessionRows { rows })
                    .and_then(|answer| {
                        let digest = reading_digest(&proto::encode(&answer).ok()?);
                        (digest[..] == full[..]).then_some(HeldReading { digest, answer })
                    });
                    match rebuilt {
                        Some(reading) => Some(reading),
                        None => match self.exchange_on(line, &Request::Sessions).await {
                            Ok(answer @ Response::SessionRows { .. }) => {
                                proto::encode(&answer).ok().map(|frame| HeldReading {
                                    digest: reading_digest(&frame),
                                    answer,
                                })
                            }
                            _ => None,
                        },
                    }
                }
                Ok(Response::DaySince { since, full }) => {
                    // Rebuilt from the copy held, and checked against the
                    // whole answer's digest; asked whole when they differ.
                    let rebuilt = match held.get(&kind).map(|h| &h.answer) {
                        Some(Response::Days { day }) => crate::day::apply_since(day, &since),
                        _ => None,
                    }
                    .map(|day| Response::Days { day })
                    .and_then(|answer| {
                        let digest = reading_digest(&proto::encode(&answer).ok()?);
                        (digest[..] == full[..]).then_some(HeldReading { digest, answer })
                    });
                    match rebuilt {
                        Some(reading) => Some(reading),
                        None => match self.exchange_on(line, &Request::Days).await {
                            Ok(answer @ Response::Days { .. }) => {
                                proto::encode(&answer).ok().map(|frame| HeldReading {
                                    digest: reading_digest(&frame),
                                    answer,
                                })
                            }
                            _ => None,
                        },
                    }
                }
                Ok(answer) => Some(HeldReading {
                    digest: reading_digest(&frame),
                    answer,
                }),
                Err(_) => None,
            };
            fresh.insert(kind, reading);
        }
        for kind in kinds {
            let now = match fresh.remove(kind) {
                Some(now) => now,
                // Not named: unchanged, the one already held.
                None => held.get(kind).cloned(),
            };
            let key = (d.id.clone(), kind.clone());
            let mut all = self.readings_held.lock().unwrap_or_else(|p| p.into_inner());
            match now {
                Some(reading) => {
                    self.keep_reading(d, kind, &reading.answer);
                    all.insert(key, reading);
                }
                None => {
                    all.remove(&key);
                }
            }
        }
    }

    /// One reading of a machine, into the cache its kind is read from —
    /// what [`Node::sync_everything`] does with each answer, one place.
    fn keep_reading(&self, d: &DeviceInfo, kind: &str, answer: &Response) {
        match answer {
            Response::SessionRows { rows } => {
                let _ = self.cache_peer_rows(&d.id, &d.name, rows);
            }
            Response::Vitals { vitals } => {
                let _ = self.cache_peer_vitals(&d.id, vitals);
            }
            Response::Watched { rows } => {
                let _ = self.cache_peer_watched(&d.id, rows);
            }
            Response::Days { day } => {
                let _ = self.cache_peer_day(&d.id, day);
            }
            Response::Quotas { readings } => {
                let _ = self.cache_peer_quotas(&d.id, readings);
            }
            Response::Usage { usage } => {
                let _ = self.cache_peer_usage(&d.id, usage);
            }
            Response::WorkspaceFacts { facts } => {
                crate::wsfacts::write(self.root(), &d.id, facts);
            }
            Response::SatVitals { vitals, age_ms } => {
                if let Some(sat) = kind.strip_prefix("sat/") {
                    let _ = self.cache_peer_vitals_aged(sat, vitals, *age_ms);
                }
            }
            _ => {}
        }
    }

    /// [`Request::Versions`], answered: only what differs.
    async fn answer_versions(
        &self,
        remote: &str,
        docs: Vec<(String, serde_bytes::ByteBuf)>,
        readings: Vec<(String, serde_bytes::ByteBuf)>,
        ep: &iroh::Endpoint,
    ) -> Result<Response, String> {
        let mut differ = Vec::new();
        for (name, theirs) in docs {
            let Ok((mine, version)) = with_doc!(self, name.as_str(), |loaded| Ok::<_, String>((
                version_digest(&loaded.doc),
                wire::version_b64(&loaded.doc)
            ))) else {
                continue;
            };
            if mine[..] != theirs[..] {
                differ.push((name, version));
            }
        }
        let by_change = readings.iter().any(|(k, _)| k == ROWS_BY_CHANGE);
        let mut changed = Vec::new();
        for (kind, held) in readings {
            let Some(req) = reading_request(&kind) else {
                continue;
            };
            let answer = Box::pin(self.dispatch(remote, req, ep)).await;
            let frame = match &answer {
                Response::Refused { .. } => Vec::new(),
                answer => proto::encode(answer)?,
            };
            // The rows this asker will hold after this answer, and the
            // ones it held before if they were the last sent it.
            let rows_before = match &answer {
                Response::SessionRows { rows } if kind == "sessions" && !frame.is_empty() => {
                    let mut sent = self.rows_sent.lock().unwrap_or_else(|p| p.into_inner());
                    sent.insert(remote.to_owned(), (reading_digest(&frame), rows.clone()))
                        .filter(|(digest, _)| digest[..] == held[..])
                        .map(|(_, rows)| rows)
                }
                _ => None,
            };
            if !frame.is_empty() && reading_digest(&frame)[..] == held[..] {
                continue;
            }
            // **Only what changed**, to an asker that takes it and holds
            // the copy last sent it — and only when that is smaller.
            let since = match (&answer, rows_before) {
                (Response::SessionRows { rows }, Some(was)) if by_change => rows_since(&was, rows)
                    .and_then(|(order, rows)| {
                        proto::encode(&Response::SessionRowsSince {
                            order,
                            rows,
                            full: serde_bytes::ByteBuf::from(reading_digest(&frame)),
                        })
                        .ok()
                    })
                    .filter(|since| since.len() < frame.len()),
                _ => None,
            };
            let frame = since.unwrap_or(frame);
            // The day, as the minutes since the asker's copy, when it
            // holds one this record still overlaps (`crate::day::DaySince`).
            let frame = match (&answer, kind.strip_prefix("days@").and_then(|m| m.parse().ok())) {
                (Response::Days { day }, Some(held_end)) => {
                    match crate::day::Day::from_wire(day, crate::live::now_ms()).and_then(|d| d.since(held_end)) {
                        Some(since) => proto::encode(&Response::DaySince {
                            since,
                            full: serde_bytes::ByteBuf::from(reading_digest(&frame)),
                        })?,
                        None => frame,
                    }
                }
                _ => frame,
            };
            changed.push((kind, serde_bytes::ByteBuf::from(frame)));
        }
        Ok(Response::Versions {
            docs: differ,
            readings: changed,
        })
    }

    /// **A round the way it ran before round NET-3**: every document in
    /// two exchanges, every reading asked. For a machine whose khor does
    /// not know [`Request::Versions`].
    async fn sync_everything(&self, line: &Line, d: &DeviceInfo) -> Result<usize, String> {
        let mut moved = 0usize;
        {
            let mut loaded = self.devices_loaded()?;
            moved += self.rounds(line, "devices", &mut loaded).await?;
        }
        self.follow_after_sync();
        {
            let mut loaded = self.seen_loaded()?;
            moved += self.rounds(line, "seen", &mut loaded).await?;
        }
        // Pins ride the same pump as the read watermark, and for the same
        // reason: both say something about a session that belongs to the
        // session rather than to the screen looking at it.
        {
            let mut loaded = self.pins_loaded()?;
            moved += self.rounds(line, "pins", &mut loaded).await?;
        }
        // What is on the session list (docs/SHELL.md 第八轮): an import
        // on one machine is a row on every machine. Best effort — an
        // older peer refuses the doc by name, and that must not cost the
        // rounds behind it; what it costs is that peer's list still
        // showing whatever it found on its own.
        {
            let mut loaded = self.roster_loaded()?;
            if let Ok(n) = self.rounds(line, "roster", &mut loaded).await {
                moved += n;
            }
        }
        // Directory pins too — a shortcut chosen on the phone must be
        // there when the desk opens the same machine's disk. Best
        // effort, unlike the two above: a peer that predates the table
        // refuses the doc by name, and that refusal must not cost the
        // chat rounds behind it — it only means older shortcuts there.
        {
            let mut loaded = self.dirpins_loaded()?;
            if let Ok(n) = self.rounds(line, "dirpins", &mut loaded).await {
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
            if let Ok(n) = self.rounds(line, "workspaces", &mut loaded).await {
                moved += n;
            }
        }
        // Web pins, same best-effort footing as directory pins and for
        // the same reason: an older peer refuses the doc by name, and that
        // must not cost the rounds behind it.
        {
            let mut loaded = self.webpins_loaded()?;
            if let Ok(n) = self.rounds(line, "webpins", &mut loaded).await {
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
            if let Ok(n) = self.rounds(line, "recents", &mut loaded).await {
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
            if let Ok(n) = self.rounds(line, "agents", &mut loaded).await {
                moved += n;
            }
        }
        // Registered API accounts — descriptions only, the keys never
        // leave the machine that holds them (`khor_sync::accounts`).
        // Same best-effort footing: an older peer refuses the doc by
        // name and the rounds behind it still run.
        {
            let mut loaded = self.accounts_loaded()?;
            if let Ok(n) = self.rounds(line, "accounts", &mut loaded).await {
                moved += n;
            }
        }
        // Session names, on the same best-effort footing as the tables
        // above and for the same reason: a peer that predates the table
        // refuses the doc by name, and that refusal must not cost the
        // chat rounds behind it — it only means older names there.
        {
            let mut loaded = self.titles_loaded()?;
            if let Ok(n) = self.rounds(line, "titles", &mut loaded).await {
                moved += n;
            }
        }
        for ch in self.known_channels()? {
            let dir = chat::channel_dir(self.root(), &ch)
                .ok_or_else(|| msg::bad_channel_name(format_args!("{ch:?}")))?;
            let mut loaded = chat::open_channel(&dir, self.writer_peer())?;
            moved += self
                .rounds(line, &format!("chat/{ch}"), &mut loaded)
                .await?;
        }
        for board in self.known_wide() {
            if let Some(dir) = chat::wide_dir(self.root(), &board) {
                let mut loaded = chat::open_channel(&dir, self.writer_peer())?;
                if let Ok(n) = self.rounds(line, &board, &mut loaded).await {
                    moved += n;
                }
            }
        }
        // Remember what the far side reports about its own sessions:
        // when it goes unreachable, its last word plus the report's age
        // is all the UI may honestly show. Best effort — a failure here
        // only means older information.
        if let Ok(Response::SessionRows { rows }) = self.exchange_on(line, &Request::Sessions).await {
            let _ = self.cache_peer_rows(&d.id, &d.name, &rows);
        }
        // And what it is doing right now, on the same visit. Kept apart
        // from the rows above rather than folded into one record: the two
        // requests fail independently, so one round losing vitals would
        // wipe a reading the rows knew nothing about, and one shared
        // timestamp would make a stale reading look as fresh as the rows
        // beside it. Best effort, same as the rows — a peer running a khor
        // that has no such op simply has no reading here.
        if let Ok(Response::Vitals { vitals }) = self.exchange_on(line, &Request::Vitals).await {
            let _ = self.cache_peer_vitals(&d.id, &vitals);
        }
        // Its watched directories, on the same visit and in a record of
        // their own (2026-09-06 关注目录): the sync round just above
        // carried our copy of its `watch` list over, so what comes back
        // is already about that list. An older khor refuses the op and
        // simply has no rows here — never empty ones.
        if let Ok(Response::Watched { rows }) = self.exchange_on(line, &Request::Watched).await {
            let _ = self.cache_peer_watched(&d.id, &rows);
        }
        // Its day, on the same beat as its rows: the strips under a
        // peer's rows are read from this, and a `Refused` from an older
        // khor leaves the last cached day standing, aging like a report.
        if let Ok(Response::Days { day }) = self.exchange_on(line, &Request::Days).await {
            let _ = self.cache_peer_day(&d.id, &day);
        }
        // Its accounts' readings, on the same beat: the 额度 page draws
        // every machine's rings from this, and an older khor's Refused
        // leaves the last carried answer standing, aging like a report.
        if let Ok(Response::Quotas { readings }) = self.exchange_on(line, &Request::Quotas).await {
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
                if let Ok(Response::SatVitals { vitals, age_ms }) = self.exchange_on(
                    line,
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
        if let Ok(Response::Usage { usage }) = self.exchange_on(line, &Request::Usage).await {
            let _ = self.cache_peer_usage(&d.id, &usage);
        }
        Ok(moved)
    }

    /// Two wire rounds: the first is pull-only by design, the second
    /// pushes. Returns bytes moved.
    async fn rounds<D: Doc>(
        &self,
        line: &Line,
        doc_name: &str,
        loaded: &mut khor_sync::store::Loaded<D>,
    ) -> Result<usize, String> {
        self.rounds_from(line, doc_name, loaded, wire::Peer::new(), 2).await
    }

    /// `times` wire rounds from what `peer` knows of the far side: two
    /// from nothing ([`Node::rounds`]), one from a version just told
    /// ([`Node::sync_what_changed`]). Returns bytes moved.
    async fn rounds_from<D: Doc>(
        &self,
        line: &Line,
        doc_name: &str,
        loaded: &mut khor_sync::store::Loaded<D>,
        mut peer: wire::Peer,
        times: usize,
    ) -> Result<usize, String> {
        let mut moved = 0usize;
        for _ in 0..times {
            let out = peer.outgoing(&loaded.doc)?;
            let resp = self
                .exchange_on(
                    line,
                    &Request::Sync {
                        doc: doc_name.to_owned(),
                        have: out.have.clone(),
                        changes: out.changes.clone(),
                    },
                )
                .await
                .map_err(Miss::words)?;
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

    /// The khor-wide conversations worth syncing, by board name
    /// (`wide/<name>`): the broadcast always — every machine keeps it,
    /// written to or not — plus any already on disk.
    pub(crate) fn known_wide(&self) -> BTreeSet<String> {
        let mut set = BTreeSet::from([format!("{}{}", chat::WIDE, chat::ALL)]);
        if let Ok(rd) = fs::read_dir(self.root().join(chat::WIDE_DIR)) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if chat::valid_channel(&name) {
                    set.insert(format!("{}{name}", chat::WIDE));
                }
            }
        }
        set
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

/// A machine's round in flight, for a second asker to wait on
/// (`Node::sync_peers`): when it began, and `None` while it runs, its
/// outcome once done.
pub(crate) type RoundWatch = (
    std::time::Instant,
    tokio::sync::watch::Receiver<Option<Result<String, String>>>,
);

/// The running half of a round: says how it ended, and says
/// "interrupted" if the one running it stopped waiting first, so a
/// joiner never waits on a round nobody is running.
struct RoundDone(Option<tokio::sync::watch::Sender<Option<Result<String, String>>>>);

impl RoundDone {
    fn finish(&mut self, out: Result<String, String>) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(Some(out));
        }
    }
}

impl Drop for RoundDone {
    fn drop(&mut self) {
        self.finish(Err(msg::round_cancelled().into()));
    }
}

/// Every future to completion, polled together on the caller's task —
/// the visits of one round borrow the node, so they cannot be spawned.
pub(crate) async fn join_all<F: std::future::Future>(futs: Vec<F>) -> Vec<F::Output> {
    let mut futs: Vec<std::pin::Pin<Box<F>>> = futs.into_iter().map(Box::pin).collect();
    let mut out: Vec<Option<F::Output>> = futs.iter().map(|_| None).collect();
    std::future::poll_fn(|cx| {
        let mut waiting = false;
        for (f, slot) in futs.iter_mut().zip(out.iter_mut()) {
            if slot.is_none() {
                match f.as_mut().poll(cx) {
                    std::task::Poll::Ready(v) => *slot = Some(v),
                    std::task::Poll::Pending => waiting = true,
                }
            }
        }
        if waiting {
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    })
    .await;
    out.into_iter().flatten().collect()
}

/// The last link reading each home's resident gave, and when — what
/// [`Node::links`] answers with when the resident does not answer in
/// time. Process-wide because the face opens a node per call.
mod last_links {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::{LazyLock, Mutex};
    use std::time::Instant;

    type Reading = Vec<(String, khor_core::Link)>;

    static LAST: LazyLock<Mutex<HashMap<PathBuf, (Instant, Reading)>>> =
        LazyLock::new(Default::default);

    pub(super) fn keep(root: &Path, reading: &Reading) {
        if let Ok(mut last) = LAST.lock() {
            last.insert(root.to_path_buf(), (Instant::now(), reading.clone()));
        }
    }

    /// The last reading, its ages moved on by the time since it was
    /// taken: a machine heard 2 s before a reading kept 10 s ago was
    /// heard 12 s ago.
    pub(super) fn recall(root: &Path) -> Option<Reading> {
        let last = LAST.lock().ok()?;
        let (at, reading) = last.get(root)?;
        let since = u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX);
        Some(
            reading
                .iter()
                .map(|(id, l)| {
                    (
                        id.clone(),
                        khor_core::Link {
                            hop: l.hop,
                            heard_ms: l.heard_ms.map(|h| h.saturating_add(since)),
                            // A recalled reading's round trip is a number
                            // about a wire that may be gone: not repeated.
                            rtt_ms: None,
                            relay: None,
                            down_ms: l.down_ms.map(|d| d.saturating_add(since)),
                            last_rtt_ms: l.last_rtt_ms.or(l.rtt_ms),
                            since_ms: l.since_ms.map(|s| s.saturating_add(since)),
                            last_hop: l.last_hop,
                            via: l.via.clone(),
                        },
                    )
                })
                .collect(),
        )
    }
}

/// A directory's listing with each received file's sender and landing
/// time from its receipt (`road::mark_arrivals`, UX-48's storage row).
/// **Blocking.**
fn listed_with_arrivals(root: &std::path::Path, path: &str) -> Result<(std::path::PathBuf, Vec<proto::DirEntry>, bool), String> {
    let (dir, mut entries, truncated) = crate::files::list_dir(path)?;
    crate::road::mark_arrivals(root, &dir, &mut entries);
    Ok((dir, entries, truncated))
}

/// A held wire to one machine, as a caller holds it: the connection, and
/// who and what it is to ([`Node::line_to`]).
pub(crate) struct Line {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) proto: wires::Proto,
    pub(crate) conn: iroh::endpoint::Connection,
}

/// What [`Node::ask_early`] came to.
enum Early {
    /// Answered inside the redial's handshake.
    Answered(Response),
    /// The redial itself failed: the far machine was not reached.
    Failed(DialFailure),
    /// Not tried, or tried without an answer: ask as usual.
    NotTaken,
}

/// Why one exchange produced no answer — three things, because only the
/// first may be asked again and only the first retires the wire.
#[derive(Debug)]
pub(crate) enum Miss {
    /// The wire is dead, or the far machine never acknowledged the
    /// request: nothing came back within [`wires::ACK_WITHIN`] of
    /// sending, or nothing for [`wires::SILENT_AFTER`] after that, or the
    /// connection closed under it.
    Unreached(String),
    /// The far machine is alive and acknowledged, and did not finish
    /// within the request's budget ([`Request::budget`]). The wire stays.
    Late(String),
    /// It answered, and the answer could not be read; or the request
    /// could not be written down at all.
    Garbled(String),
}

impl Miss {
    pub(crate) fn words(self) -> String {
        match self {
            Miss::Unreached(w) | Miss::Late(w) | Miss::Garbled(w) => w,
        }
    }
}

/// How often an exchange looks at whether its wire has been heard.
const LIVENESS_BEAT: Duration = Duration::from_millis(250);

/// **One request and its answer on one connection, watched for life.**
///
/// The far machine's QUIC stack acknowledges what it receives within a
/// round trip, and iroh's keepalive draws an ack every few seconds
/// besides, so a live wire is *heard* (`wires::heard_count`) soon after
/// anything is sent on it and every few seconds after that — whether or
/// not the answer is ready. That is what tells a dead wire from a slow
/// answer without a wire-format change:
///
/// - nothing heard within [`wires::ACK_WITHIN`] of sending, or nothing
///   for [`wires::SILENT_AFTER`] at any point after: **unreached** — a
///   machine that restarted, froze, slept or lost its network;
/// - heard, but no answer within `budget`: **late** — alive and still
///   working, and the wire is fine.
///
/// ⚠️ 墓碑 (2026-10-03, audit/net.md F6): this had no deadline at all. A
/// far machine frozen for 20 s held a `git status` for exactly 20 s, and
/// one restarted under a held wire held every ask until the ipc
/// deadline cancelled it, 29.5 s, three times running.
async fn exchange(
    conn: &iroh::endpoint::Connection,
    who: &str,
    req: &Request,
    budget: Duration,
) -> Result<Response, Miss> {
    let frame = proto::encode(req).map_err(Miss::Garbled)?;
    let sent = tokio::time::Instant::now();
    let mut count = wires::heard_count(conn);
    let mut heard: Option<tokio::time::Instant> = None;
    let lost = |e: String| {
        if conn.close_reason().is_some() {
            Miss::Unreached(msg::cant_reach_named(who, e))
        } else {
            Miss::Garbled(e)
        }
    };
    let work = async {
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| Miss::Unreached(msg::cant_reach_named(who, msg::cant_open_stream(e))))?;
        send.write_all(&frame)
            .await
            .map_err(|e| lost(msg::cant_send(e)))?;
        send.finish().map_err(|e| lost(msg::cant_close_stream(e)))?;
        let bytes = recv
            .read_to_end(MAX_FRAME)
            .await
            .map_err(|e| lost(msg::no_answer(e)))?;
        proto::answer(&bytes).map_err(Miss::Garbled)
    };
    tokio::pin!(work);
    let mut beat = tokio::time::interval(LIVENESS_BEAT);
    beat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            out = &mut work => return out,
            _ = beat.tick() => {
                let now = tokio::time::Instant::now();
                let seen = wires::heard_count(conn);
                if seen != count {
                    count = seen;
                    heard = Some(now);
                }
                // The road's own patience, read each beat: QUIC refines its
                // round trip as the answers come (`wires::Patience`).
                let patience = wires::Patience::of(conn);
                let quiet = match heard {
                    None => now.duration_since(sent) >= patience.ack,
                    Some(at) => now.duration_since(at) >= patience.silent,
                };
                if quiet {
                    return Err(Miss::Unreached(msg::machine_silent(who)));
                }
                if now.duration_since(sent) >= budget {
                    return Err(Miss::Late(msg::answer_late(who, budget.as_secs())));
                }
            }
        }
    }
}

/// [`exchange`] on a connection no wire holds (pairing: the far side is
/// not in the table yet), failures in words.
async fn request(
    conn: &iroh::endpoint::Connection,
    who: &str,
    req: &Request,
) -> Result<Response, String> {
    exchange(conn, who, req, req.budget())
        .await
        .map_err(Miss::words)
}

/// Retires a wire whose request was abandoned before an answer — an ipc
/// deadline cancelling the op, a face going away — **if nothing at all
/// was heard on it since the request went out** and that silence has
/// already lasted [`wires::ACK_WITHIN`]. Shorter than that is no
/// evidence either way, and the resident's tick catches a wire that
/// stays silent.
struct RetireUnheard<'a> {
    wires: &'a wires::Wires,
    line: &'a Line,
    count: u64,
    at: std::time::Instant,
    armed: bool,
}

impl<'a> RetireUnheard<'a> {
    fn new(wires: &'a wires::Wires, line: &'a Line) -> Self {
        Self {
            wires,
            line,
            count: wires::heard_count(&line.conn),
            at: std::time::Instant::now(),
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for RetireUnheard<'_> {
    fn drop(&mut self) {
        if self.armed
            && wires::heard_count(&self.line.conn) == self.count
            && self.at.elapsed() >= wires::Patience::of(&self.line.conn).ack
        {
            self.wires.retire(
                &self.line.id,
                self.line.proto,
                &self.line.conn,
                wires::Retired::Unanswered,
            );
        }
    }
}

/// A conversation file's running move, waited out: where it landed, or
/// why it did not.
async fn wait_for(job: &crate::road::Job) -> Result<(u64, Vec<String>), String> {
    loop {
        match job.state() {
            Ok(s) if s.done => return Ok((0, s.dest.lines().map(str::to_owned).collect())),
            Ok(_) => tokio::time::sleep(Duration::from_millis(100)).await,
            Err(why) => return Err(why),
        }
    }
}

/// A dial's failure as the sentence a person reads.
fn unreached_words(name: &str, e: DialFailure) -> String {
    match e {
        DialFailure::TimedOut => msg::machine_unreachable_timeout(name),
        DialFailure::Refused(why) => msg::cant_reach_named(name, why),
    }
}

impl Node {
    /// **An offered file, landed** (the file road's `Source::Offer`): a
    /// file already received whole is the answer as it is; otherwise its
    /// bytes are pulled by digest into a partial kept under the store
    /// (`road::partial_of`), carried on from when one is there, checked
    /// against the digest, and only then moved into `dir` under its own
    /// name (a taken one suffixed) with a receipt. Answers bytes moved
    /// this run — a resumed pull moves only the missing tail.
    async fn pull_offer(
        &self,
        ep: &iroh::Endpoint,
        home: &DeviceInfo,
        f: &khor_sync::chat::FileRef,
        dir: &std::path::Path,
        mode: crate::road::Mode,
        job: Option<&crate::road::Job>,
    ) -> Result<crate::road::Landed, String> {
        if let Some(r) = crate::road::receipt(self.root(), &f.digest) {
            if r.path.parent() == Some(dir) {
                return Ok(crate::road::Landed {
                    moved: 0,
                    path: r.path,
                    digest: Some(r.digest),
                });
            }
            // The same bytes are here already, elsewhere: copied into the
            // directory asked for, nothing pulled.
            let at = crate::drops::landing(dir, &crate::transfer::safe_name(&f.name))?;
            let dest = at.dest.clone();
            let copied = fs::copy(&r.path, &at.part).and_then(|_| fs::rename(&at.part, &at.dest));
            let _ = fs::remove_file(&at.part);
            copied.map_err(|e| msg::cant_read(r.path.display(), e))?;
            return Ok(crate::road::Landed {
                moved: 0,
                path: dest,
                digest: Some(r.digest),
            });
        }
        let not_digest = || msg::not_a_digest(format_args!("{:?}", f.digest));
        let partial = crate::road::partial_of(self.root(), &f.digest).ok_or_else(not_digest)?;
        let marker = crate::road::marker_of(self.root(), &f.digest).ok_or_else(not_digest)?;
        fs::create_dir_all(crate::road::received_dir(self.root())).map_err(msg::cant_make_files_dir)?;
        fs::write(&marker, format!("{}", std::process::id())).map_err(msg::cant_mark_pulling)?;
        let pulled = self.pull_one(ep, home, &partial, f, job).await;
        let _ = fs::remove_file(&marker);
        let moved = pulled?;
        let at = crate::drops::landing(dir, &crate::transfer::safe_name(&f.name))?;
        let dest = at.dest.clone();
        crate::road::settle(&partial, &at).map_err(msg::cant_rename)?;
        crate::road::record(
            self.root(),
            &crate::road::Receipt {
                digest: f.digest.clone(),
                name: f.name.clone(),
                path: dest.clone(),
                size: f.size.max(0) as u64,
                from: home.id.clone(),
                at_ms: crate::road::now_ms(),
                mode,
            },
        )?;
        if let Some(job) = job {
            job.finish(Ok(dest.display().to_string()));
        }
        Ok(crate::road::Landed {
            moved,
            path: dest,
            digest: Some(f.digest.clone()),
        })
    }

    /// Pulls one payload into `partial`, slice by slice, and answers once
    /// the bytes there match the digest — the caller gives it its name.
    /// A job is told the bytes as they land and stops the pull before its
    /// next slices when cancelled.
    async fn pull_one(
        &self,
        ep: &iroh::Endpoint,
        home: &DeviceInfo,
        partial: &std::path::Path,
        f: &khor_sync::chat::FileRef,
        job: Option<&crate::road::Job>,
    ) -> Result<u64, String> {
        use std::io::{Read, Write};
        // Resume: the final digest must cover the bytes already on disk, so
        // they run through the hasher before any new slice does.
        let mut hasher = blake3::Hasher::new();
        let mut offset = 0u64;
        if let Ok(mut existing) = fs::File::open(partial) {
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
            .open(partial)
            .map_err(msg::cant_open_partial)?;
        let expect = f.size.max(0) as u64;
        if let Some(job) = job {
            job.total.store(expect, std::sync::atomic::Ordering::Relaxed);
            job.moved.store(offset, std::sync::atomic::Ordering::Relaxed);
        }
        let mut moved = 0u64;
        // The first slice alone (it is the one that marks the offer
        // started over there), then up to `PULL_AHEAD` asked at once and
        // written in order.
        let mut ahead = 1;
        while offset < expect {
            if job.is_some_and(|j| j.is_stopped()) {
                return Err(crate::road::cancelled());
            }
            let offsets: Vec<u64> = (0..ahead)
                .map(|k| offset + k * proto::SLICE)
                .take_while(|o| *o < expect)
                .collect();
            let fetches: Vec<Request> = offsets
                .iter()
                .map(|&at| Request::Fetch {
                    digest: f.digest.clone(),
                    offset: at,
                })
                .collect();
            let asks = fetches.iter().map(|r| self.ask_on(ep, home, r)).collect();
            for (at, resp) in offsets.iter().zip(join_all(asks).await) {
                // A short slice moved the offset: what was asked beyond
                // it is asked again from there.
                if *at != offset {
                    break;
                }
                let (total, bytes) = match resp? {
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
                if let Some(job) = job {
                    job.moved.store(offset, std::sync::atomic::Ordering::Relaxed);
                }
            }
            ahead = PULL_AHEAD;
        }
        out.sync_all().map_err(msg::cant_sync_disk)?;
        drop(out);
        let got = hasher.finalize().to_hex().to_string();
        if got != f.digest {
            // A mismatch poisons the partial — deleted here, at the only
            // place that knows. Any other failure leaves it: it is the
            // resume point.
            let _ = fs::remove_file(partial);
            return Err(msg::digest_mismatch(
                &got[..8],
                f.digest.chars().take(8).collect::<String>(),
            ));
        }
        Ok(moved)
    }
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

/// Why [`Node::road_tallies`] has no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadsUnread {
    /// No resident on this machine: nothing here has been dialling.
    NoServe,
    /// A resident that refused the question or did not answer it.
    Unanswered,
}

/// Why a dial did not produce a connection. Kept apart from the sentence
/// a person reads: every caller says who it was trying to reach and what
/// for, and that is the half worth keeping at the call site.
pub(crate) enum DialFailure {
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
        within: Duration,
    ) -> Result<iroh::endpoint::Connection, DialFailure> {
        // 按机器暂停: not dialled at all, and said so — the sync tick,
        // every verb and every tunnel come through here.
        if crate::pause::peer_on(self.root(), id) {
            return Err(DialFailure::Refused(msg::peer_paused_refusal().to_owned()));
        }
        let bare = |e| DialFailure::Refused(format!("{e}"));
        // 首包时间 starts here, before either road is tried: what a
        // person waits through is the whole dial, head start included.
        let started = std::time::Instant::now();
        let mut roads = tokio::task::JoinSet::new();
        // A road this machine carries itself (round ROAD-2): a candidate
        // beside the direct ones, so it races them from the first packet
        // (`khor_net::carry::RoadOrder` then keeps the owner's order).
        let carried = self.carried_addr(id);
        let ahead = !direct.is_empty() || carried.is_some();
        if ahead {
            let mut addr = endpoint::dial_addr(id, direct, relays).map_err(bare)?;
            addr.addrs.extend(carried);
            let (ep, alpn) = (ep.clone(), alpn.to_vec());
            roads.spawn(async move { ep.connect(addr, &alpn).await });
        }
        // By identity alone. Nothing here can be stale: a machine's id
        // is the one thing about it that a restart does not change.
        {
            let addr = endpoint::dial_addr(id, &[], relays).map_err(bare)?;
            let (ep, alpn, wait) = (ep.clone(), alpn.to_vec(), ahead);
            roads.spawn(async move {
                if wait {
                    tokio::time::sleep(RELAY_HEAD_START).await;
                }
                ep.connect(addr, &alpn).await
            });
        }

        let deadline = tokio::time::sleep(within);
        tokio::pin!(deadline);
        // First road home wins; a road that fails only counts if every
        // other one does too, so one dead direct address cannot decide
        // the answer while the relay is still walking.
        let mut refused: Option<String> = None;
        loop {
            tokio::select! {
                () = &mut deadline => return Err(DialFailure::TimedOut),
                joined = roads.join_next() => match joined {
                    Some(Ok(Ok(conn))) => {
                        crate::roads::replied(&self.roads, id, started.elapsed());
                        crate::roads::watch(&self.roads, id, &conn);
                        return Ok(conn);
                    }
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
        by: Option<std::time::Instant>,
    ) -> Result<iroh::endpoint::Connection, DialFailure> {
        // A machine last seen behind a slow road is given that road's time
        // to answer a handshake (`wires::Patience`); one never measured, or
        // a fast one, the budget dials always had — and never past the
        // caller's own deadline, when it has one.
        let mut within = DIAL_TIMEOUT
            .max(self.wires.last_rtt(id).saturating_mul(12))
            .min(Duration::from_secs(60));
        // A machine its relay just said left, and that has not been heard
        // from since: a short dial — long enough for one that came straight
        // back on a new connection, not the full wait on one that is gone.
        if self.wires.gone_lately(id) {
            within = within.min(
                Duration::from_secs(3).max(self.wires.last_rtt(id).saturating_mul(6)),
            );
        }
        if let Some(by) = by {
            within = within.min(by.saturating_duration_since(std::time::Instant::now()));
        }
        self.dial_with(ep, id, direct, self.relays(), alpn, within).await
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

    /// Round NET-11: tries at new ears, once the hearing test says they
    /// are needed, come 4, 8, 16 and then every 30 minutes — never every
    /// DEAF_AFTER.
    #[test]
    fn tries_at_new_ears_back_off_to_half_an_hour() {
        let min = |m: u64| Duration::from_secs(m * 60);
        assert_eq!(
            [1, 2, 3, 4, 5, 40].map(super::deaf_wait),
            [min(4), min(8), min(16), min(30), min(30), min(30)]
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

#[cfg(test)]
mod rows_by_change {
    //! `SessionRowsSince` alone (round NET-6): what a change carries and
    //! that the rebuild is the whole, byte for byte.

    use super::*;
    use khor_core::{DeviceId, Session, SessionId, State};

    fn row(id: &str, command: &str) -> Session {
        Session {
            id: SessionId(id.into()),
            kind: khor_core::Kind(khor_core::kind::SHELL.into()),
            title: id.into(),
            home: DeviceId([1; 32]),
            state: khor_core::StateStamp {
                state: State::Busy,
                at: khor_core::Millis(1),
            },
            unread: 0,
            place: None,
            mux: false,
            doing: Some(khor_core::Doing {
                command: Some(command.into()),
                running: true,
                ..Default::default()
            }),
            dead: false,
            unbacked: false,
            category: None,
            last: None,
            via: None,
            term: true,
        }
    }

    /// A busy session's next command is one row on the wire, not six;
    /// the rebuild is the whole answer, digest for digest.
    #[test]
    fn a_busy_rows_next_command_carries_that_row_alone() {
        let was: Vec<Session> = (0..6).map(|k| row(&format!("shell/{k}"), "sleep 3600")).collect();
        let mut now = was.clone();
        now[5] = row("shell/5", "make test --shard 2");
        let (order, changed) = rows_since(&was, &now).unwrap();
        assert_eq!(changed.len(), 1, "only the busy row");
        assert!(order.is_empty(), "and no order: it is the one held");
        let whole = proto::encode(&Response::SessionRows { rows: now.clone() }).unwrap();
        let since = proto::encode(&Response::SessionRowsSince {
            order: order.clone(),
            rows: changed.clone(),
            full: serde_bytes::ByteBuf::from(reading_digest(&whole)),
        })
        .unwrap();
        assert!(since.len() * 2 < whole.len(), "the change {} B against the whole {} B", since.len(), whole.len());

        // On a desk of 109 rows (the term lane's), the change is still one
        // row's size: the ids are not on it.
        let many: Vec<Session> = (0..109).map(|k| row(&format!("shell/{k:016x}"), "sleep 3600")).collect();
        let mut busy = many.clone();
        busy[108] = row("shell/000000000000006c", "make test --shard 3");
        let (many_order, many_changed) = rows_since(&many, &busy).unwrap();
        let many_since = proto::encode(&Response::SessionRowsSince {
            order: many_order.clone(),
            rows: many_changed.clone(),
            full: serde_bytes::ByteBuf::from(vec![0; 16]),
        })
        .unwrap();
        assert!(many_since.len() < 600, "one busy row among 109 is {} B on the wire", many_since.len());
        assert_eq!(rows_rebuilt(&many, &many_order, many_changed).unwrap(), busy);
        let rebuilt = rows_rebuilt(&was, &order, changed).unwrap();
        let again = proto::encode(&Response::SessionRows { rows: rebuilt }).unwrap();
        assert_eq!(reading_digest(&again), reading_digest(&whole));
    }

    /// A row gone and a row new, in a new order; a repeated id is no change.
    #[test]
    fn rows_that_come_go_and_move_rebuild_in_order() {
        let was = vec![row("a", "x"), row("b", "y"), row("c", "z")];
        let now = vec![row("c", "z"), row("d", "w"), row("a", "x")];
        let (order, changed) = rows_since(&was, &now).unwrap();
        assert_eq!(changed.iter().map(|r| r.id.0.as_str()).collect::<Vec<_>>(), ["d"]);
        assert_eq!(rows_rebuilt(&was, &order, changed).unwrap(), now);
        assert!(rows_since(&was, &[row("a", "x"), row("a", "y")]).is_none());
        assert!(rows_since(&was, &[]).is_none(), "every row gone is sent whole");
        assert!(rows_rebuilt(&was, &["e".to_owned()], vec![]).is_none(), "an id in neither");
    }
}
