//! **The connection journal** (NET-STABLE 第一步, 2026-10-08): what the
//! resident saw on its way to each other machine, in order — every dial
//! and the roads it offered, how it ended and after how long, where a
//! fresh wire settled, every wire retired and how long it had been quiet
//! when it was, the backoff a failure left behind — and this machine's
//! own events beside them: network changes, its addresses, its home
//! relay, new ears.
//!
//! # Why
//!
//! The owner: 「我们经常半天连不上，重启app又可以了」. The counters behind
//! `khor doctor` say *how many*; nothing said *what happened, in what
//! order*, after the moment had passed. A restart clears every in-memory
//! trace of it, so the journal is on disk.
//!
//! # Cost
//!
//! **Written on events only**: no timer of its own, no polling. A line is
//! appended where the event already happens (the dial's door, the wire
//! watch's retirement, iroh's own watchers for addresses and the home
//! relay). One small file per machine under `.khor/journal/`, plus
//! `here.jsonl` for this machine's own events; each is cut back to its
//! last [`KEEP`] lines when it reaches twice that, so a file never holds
//! more than about 2 × [`KEEP`] lines.
//!
//! **Only the resident writes** ([`crate::Node`]'s serve sets it on its
//! wires): two processes appending and cutting one file would tear it.
//! Anyone reads — the CLI's `khor journal` reads the files straight, so
//! it answers when the resident does not.
//!
//! # What it never holds
//!
//! Not synced (`.khor/` is this machine's), not backed up
//! (`backup::keep_out`). No relay tokens (they never leave
//! `relays.json`; [`scrub`] strips a `token=` that an error might echo),
//! no tickets, no host keys, no recovery words: only ids, addresses,
//! relay URLs, timings and iroh's error sentences.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use khor_catalog::msg;
use serde::{Deserialize, Serialize};

/// Lines kept per file after a cut.
pub const KEEP: usize = 300;

/// The file for this machine's own events.
const HERE: &str = "here";

/// Who wanted a dial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum By {
    /// The background pump, backoff honoured.
    Pump,
    /// A round asked for now (立即同步, a wake, a resume), backoff not.
    Forced,
    /// A request — a person's, or a face's reading.
    Ask,
    /// The wire to a middle machine, for a hop.
    Hop,
    /// A middle's dial onward, for the machine that asked it for a hop
    /// (net-dial B3): not this machine's own failure when it fails.
    Onward,
    /// A pairing dial.
    Pair,
}

tokio::task_local! {
    /// Who the dials in this task are for; [`By::Ask`] when unset.
    pub(crate) static BY: By;
}

/// The asker of a dial made now, in this task.
pub(crate) fn by_now() -> By {
    BY.try_with(|b| *b).unwrap_or(By::Ask)
}

/// Who said the network changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The Mac woke (`NSWorkspaceDidWakeNotification`).
    Wake,
    /// The phone's path monitor saw a new road.
    Path,
    /// The resident's wall clock ran ahead of its monotonic one.
    Clock,
    /// The home relay was down since `down_at` and is connected again
    /// ([`watch_ears`]): the dials that failed since may have failed
    /// only because it was.
    Relay { down_at: std::time::Instant },
    /// iroh's own network monitor moved this machine's interface
    /// addresses ([`watch_ears`]): a Wi-Fi switch, a VPN or proxy TUN, a
    /// cable — on a Mac that did not sleep, and on Linux.
    Addrs,
}

impl Change {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Change::Wake => "wake",
            Change::Path => "path",
            Change::Clock => "clock",
            Change::Relay { .. } => "relay",
            Change::Addrs => "addrs",
        }
    }
}

/// Which of a dial's attempts answered first (`Node::dial_with`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Won {
    /// The attempt carrying the direct (and carried) addresses.
    Direct,
    /// The attempt by identity alone, through the relays.
    Relay,
    /// The 0-RTT dial (`Node::ask_early`), one attempt.
    Early,
}

/// One thing that happened.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "e", rename_all = "kebab-case")]
pub enum What {
    Started,
    /// A dial this machine started. `failures` and `retry_s` are the
    /// backoff the outcome left (`wires::Wires::failed`).
    Dial {
        by: By,
        tunnel: bool,
        ms: u64,
        direct: u32,
        relays: Vec<String>,
        carried: bool,
        /// `None`: it failed.
        won: Option<Won>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(default)]
        failures: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retry_s: Option<u64>,
    },
    /// Where a fresh wire settled, [`crate::roads::SETTLE`] in.
    Settled {
        road: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        relay: Option<String>,
        rtt_ms: u64,
        paths: Vec<String>,
    },
    /// A wire this machine held, retired. `quiet_ms` is how long nothing
    /// had been heard on it — for a dead wire, how long its death took
    /// to notice.
    Retired {
        why: String,
        tunnel: bool,
        quiet_ms: u64,
        age_ms: u64,
    },
    /// The relay said it left (vendor/iroh patch 4).
    Departed,
    /// It opened a connection to this machine.
    Inbound,
    Network { source: String },
    Addrs { addrs: Vec<String> },
    Home {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        state: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Ears {
        ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// The app went to the background: every wire let go, no dials until
    /// it is back (net-r).
    Rest,
    /// Back in front, the same endpoint kept (net-r): its sockets rebound
    /// on their ports, its relay checked, every path pinged; `ms` until
    /// the round that follows had come back.
    Front { ms: u64 },
    Hop {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        via: Option<String>,
        tried: u32,
    },
    Carry {
        up: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        why: Option<String>,
    },
    /// A person's act that dials past the backoff (NET-STABLE 第二步):
    /// `face`, a face arriving (this machine's line); `page`, a machine's
    /// page opened (that machine's).
    Person { by: String },
    /// What iroh learnt of this machine from the relays that answer QUIC
    /// address discovery (NET-DIRECT): its public addresses, whether its
    /// NAT maps each destination to a port of its own (then a peer behind
    /// another such NAT cannot punch through on IPv4), and which relays
    /// answered. Written when the addresses or the mapping change.
    Learnt {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        v4: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        v6: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        varies: Option<bool>,
        from: Vec<String>,
    },
    /// The kernel sends this machine's internet traffic out from `src`, a
    /// fake-IP address (198.18.0.0/15): a proxy's TUN carries every UDP
    /// datagram and gives it a port of its own. `None`: it no longer does.
    Tun {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        src: Option<String>,
    },
    /// A path of a connection to the peer, for the life of the
    /// connection: `opened`, a direct path validated (a punch landed);
    /// `selected`, the path now carrying it.
    Path {
        change: String,
        road: String,
        addr: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rtt_ms: Option<u64>,
    },
    /// Path changes of a connection to the peer that iroh's event buffer
    /// dropped before they were read (vendor/iroh patch 21). The next
    /// `path` line is the selection as it stood then.
    PathsMissed { missed: u64 },
    /// Hole punching toward the peer (vendor/iroh patch 8): the
    /// candidates of the latest round, and how many rounds were started
    /// since the last such line — written when the candidates change, and
    /// at most every [`ROUND_EVERY`] while they do not.
    Round {
        local: Vec<String>,
        remote: Vec<String>,
        rounds: u32,
    },
    /// A document that must sync with the peer did not, and the round
    /// stopped there (NET-DIRECT: turing's and lecun's devices tables
    /// failed every round for days and nothing said so). Written when
    /// the failure is new or its words changed ([`sync_failed`]).
    Sync { doc: String, error: String },
    /// A pile the peer keeps in another epoch (`khor_sync::store::load_pile`):
    /// not merged, and the round went on. Written like [`What::Sync`].
    Apart { doc: String, error: String },
    /// The far side sent ops under the name this machine's copy of a pile
    /// was writing with, ops this machine never kept: the name is retired
    /// (`khor_sync::store::BlockStore::took_in`).
    PenRetired { doc: String, name: String },
}

/// One line of the journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Milliseconds since the epoch.
    pub at: u64,
    /// The far machine's id; `None` for this machine's own events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer: Option<String>,
    #[serde(flatten)]
    pub what: What,
}

/// The writer: the resident's, one per home.
///
/// **[`Journal::note`] never touches the disk** (net-dial B1). It used to
/// append in the caller's thread, and its callers include the wire
/// watch's `tick` and every retirement, which write while they hold the
/// wires' `slots` lock. On turing, whose home is on NFS, 2026-10-20
/// 22:50:15 was the last line it wrote for 87 minutes: requests went
/// unanswered from that second while iroh still finished handshakes, and
/// five minutes later iroh went too — the shape of tokio's workers being
/// taken one by one by tasks parked behind one lock. A write that does
/// not return holds everything that waits on what it holds.
///
/// So a line is handed to a thread of the journal's own, through a
/// queue of [`QUEUE`] lines; a writer stuck on a file leaves the queue to
/// fill, and once it is full a line is dropped — a gap in the journal,
/// which it already promised to be rather than a dial that failed.
pub struct Journal {
    tx: std::sync::mpsc::SyncSender<(String, String)>,
    /// Lines handed to the writer, and lines it is done with (written or
    /// not): [`flush`] waits for the second to reach the first.
    sent: Arc<std::sync::atomic::AtomicU64>,
    done: Arc<std::sync::atomic::AtomicU64>,
}

/// Lines the writer may fall behind by before new ones are dropped:
/// about a megabyte, hours of an ordinary machine's events.
const QUEUE: usize = 4096;

/// Where a home's journal lives.
pub fn dir(root: &Path) -> PathBuf {
    root.join(".khor").join("journal")
}

/// The journals open in this process, by directory, so a reader here
/// sees what was noted before it asked ([`read`] waits for them).
static OPEN: Mutex<Vec<(PathBuf, std::sync::Weak<Journal>)>> = Mutex::new(Vec::new());

impl Journal {
    pub fn open(root: &Path) -> Arc<Journal> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<(String, String)>(QUEUE);
        let sent = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let done = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let dir = dir(root);
        {
            let (dir, done) = (dir.clone(), done.clone());
            // Ends when the journal does: the last sender gone.
            let _ = std::thread::Builder::new()
                .name("khor-journal".into())
                .spawn(move || {
                    let mut lines: HashMap<String, usize> = HashMap::new();
                    for (key, line) in rx {
                        append(&dir, &mut lines, key, &line);
                        done.fetch_add(1, std::sync::atomic::Ordering::Release);
                    }
                });
        }
        let j = Arc::new(Journal { tx, sent, done });
        let mut open = OPEN.lock().unwrap_or_else(|p| p.into_inner());
        open.retain(|(_, w)| w.strong_count() > 0);
        open.push((dir, Arc::downgrade(&j)));
        j
    }

    /// Appends one event; `peer` `None` is this machine's own. Never
    /// fails the caller and **never waits on the disk**: a journal that
    /// cannot be written is a journal with a gap, not a dial that failed,
    /// and not a lock held for as long as a file takes.
    pub fn note(&self, peer: Option<&str>, what: What) {
        let entry = Entry {
            at: crate::now_ms(),
            peer: peer.map(str::to_owned),
            what,
        };
        let Ok(mut line) = serde_json::to_string(&entry) else { return };
        line.push('\n');
        let key = peer.unwrap_or(HERE).to_owned();
        // A file name from an id: hex, so safe; anything else is refused.
        if !key.chars().all(|c| c.is_ascii_alphanumeric()) {
            return;
        }
        if self.tx.try_send((key, line)).is_ok() {
            self.sent.fetch_add(1, std::sync::atomic::Ordering::Release);
        }
    }

    /// Waits, up to `within`, for every line noted so far to be written.
    /// Answers whether they were.
    pub fn flush(&self, within: Duration) -> bool {
        let want = self.sent.load(std::sync::atomic::Ordering::Acquire);
        let until = std::time::Instant::now() + within;
        loop {
            if self.done.load(std::sync::atomic::Ordering::Acquire) >= want {
                return true;
            }
            if std::time::Instant::now() >= until {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

/// The writer's one step: `line` onto `key`'s file, and the file cut
/// back to its last [`KEEP`] lines once it holds twice that.
fn append(dir: &Path, lines: &mut HashMap<String, usize>, key: String, line: &str) {
    let path = dir.join(format!("{key}.jsonl"));
    let n = lines.entry(key).or_insert_with(|| {
        std::fs::read_to_string(&path)
            .map(|t| t.lines().count())
            .unwrap_or(0)
    });
    let _ = std::fs::create_dir_all(dir);
    let wrote = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
    if wrote.is_err() {
        return;
    }
    *n += 1;
    if *n >= 2 * KEEP {
        if let Ok(text) = std::fs::read_to_string(&path) {
            let all: Vec<&str> = text.lines().collect();
            let kept = all[all.len().saturating_sub(KEEP)..].join("\n") + "\n";
            let tmp = path.with_extension("jsonl.tmp");
            if std::fs::write(&tmp, kept).is_ok() && std::fs::rename(&tmp, &path).is_ok() {
                *n = KEEP.min(all.len());
            }
        }
    }
}

/// The lines this process noted for `root`'s journal, written before a
/// read of it — at most a second's wait, so a stuck writer costs a reader
/// a second and not the answer.
fn flush_open(root: &Path) {
    let d = dir(root);
    let open: Vec<Arc<Journal>> = OPEN
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .filter(|(at, _)| *at == d)
        .filter_map(|(_, w)| w.upgrade())
        .collect();
    for j in open {
        j.flush(Duration::from_secs(1));
    }
}

/// Every entry of `peer` (and this machine's own, interleaved) — or of
/// every machine when `peer` is `None` — at or after `since` (ms), oldest
/// first. Lines that do not parse are skipped: a torn last line after a
/// crash is not a reason to read nothing.
pub fn read(root: &Path, peer: Option<&str>, since: u64) -> Vec<Entry> {
    flush_open(root);
    let d = dir(root);
    let mut files: Vec<PathBuf> = match peer {
        Some(id) => vec![d.join(format!("{id}.jsonl")), d.join(format!("{HERE}.jsonl"))],
        None => std::fs::read_dir(&d)
            .map(|it| {
                it.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
                    .collect()
            })
            .unwrap_or_default(),
    };
    files.sort();
    let mut out: Vec<Entry> = files
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .flat_map(|t| {
            t.lines()
                .filter_map(|l| serde_json::from_str::<Entry>(l).ok())
                .collect::<Vec<_>>()
        })
        .filter(|e| e.at >= since)
        .collect();
    out.sort_by_key(|e| e.at);
    out
}

/// [`read`] as JSON lines, for another machine that asked
/// (`proto::Request::Journal`). A `peer` that is not an id names no file
/// and reads nothing: it comes off the wire, and a file name is made of it.
pub fn lines_since(root: &Path, peer: Option<&str>, since: u64) -> Vec<String> {
    if peer.is_some_and(|id| !id.chars().all(|c| c.is_ascii_alphanumeric())) {
        return Vec::new();
    }
    read(root, peer, since)
        .iter()
        .filter_map(|e| serde_json::to_string(e).ok())
        .collect()
}

/// Removes what must never leave this machine from a sentence some
/// library wrote: a `token=` query value, a bearer credential.
pub fn scrub(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        let hit = ["token=", "Bearer "]
            .iter()
            .filter_map(|k| rest.find(k).map(|i| (i, k.len())))
            .min();
        let Some((i, k)) = hit else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..i + k]);
        out.push('…');
        let after = &rest[i + k..];
        let end = after
            .find(|c: char| c == '&' || c == ',' || c == '"' || c == '\'' || c == ')' || c.is_whitespace())
            .unwrap_or(after.len());
        rest = &after[end..];
    }
}

fn road_word(road: &str) -> &'static str {
    match road {
        "direct" => msg::journal_road_direct(),
        "relay" => msg::journal_road_relay(),
        "carried" => msg::journal_road_carried(),
        _ => msg::journal_road_unknown(),
    }
}

/// The road's word for a [`khor_net::path::PathKind`], as stored.
pub(crate) fn road_key(kind: khor_net::path::PathKind) -> &'static str {
    use khor_net::path::PathKind;
    match kind {
        PathKind::Direct => "direct",
        PathKind::Relay => "relay",
        PathKind::Carried => "carried",
        PathKind::Unknown => "unknown",
    }
}

fn by_word(by: By) -> &'static str {
    match by {
        By::Pump => msg::journal_by_pump(),
        By::Forced => msg::journal_by_forced(),
        By::Ask => msg::journal_by_ask(),
        By::Hop => msg::journal_by_hop(),
        By::Onward => msg::journal_by_onward(),
        By::Pair => msg::journal_by_pair(),
    }
}

fn why_word(why: &str) -> &'static str {
    match why {
        "silent" => msg::journal_why_silent(),
        "unanswered" => msg::journal_why_unanswered(),
        "network" => msg::journal_why_network(),
        "paused" => msg::journal_why_paused(),
        "suspended" => msg::journal_why_suspended(),
        "departed" => msg::journal_why_departed(),
        "replaced" => msg::journal_why_replaced(),
        _ => msg::journal_why_closed(),
    }
}

/// One entry in this language's words, without its time or machine.
pub fn words(e: &Entry, name_of: &dyn Fn(&str) -> String) -> String {
    let secs = |ms: u64| ms.div_ceil(1000);
    match &e.what {
        What::Started => msg::journal_started().to_owned(),
        What::Dial {
            by,
            ms,
            direct,
            relays,
            carried,
            won,
            error,
            failures,
            retry_s,
            ..
        } => {
            let mut offered = Vec::new();
            if *direct > 0 {
                offered.push(msg::journal_offered_direct(direct));
            }
            if !relays.is_empty() {
                offered.push(msg::journal_offered_relays(relays.join("、")));
            }
            if *carried {
                offered.push(msg::journal_offered_carried().to_owned());
            }
            let tried = msg::journal_offered(offered.join("，"));
            match won {
                Some(w) => {
                    let won = match w {
                        Won::Direct => msg::journal_won_direct(),
                        Won::Relay => msg::journal_won_relay(),
                        Won::Early => msg::journal_won_early(),
                    };
                    format!("{}；{tried}", msg::journal_dial_ok(by_word(*by), ms, won))
                }
                None => {
                    let why = error.as_deref().map(scrub).unwrap_or_else(|| msg::journal_timed_out().to_owned());
                    let mut s = format!("{}；{tried}", msg::journal_dial_failed(by_word(*by), ms, why));
                    if let Some(r) = retry_s {
                        s.push_str("；");
                        s.push_str(&msg::journal_backoff(failures, r));
                    }
                    s
                }
            }
        }
        What::Settled { road, relay, rtt_ms, paths } => {
            let road = match relay {
                Some(r) => msg::journal_settled_relay(road_word(road), r),
                None => road_word(road).to_owned(),
            };
            let mut s = msg::journal_settled(road, rtt_ms);
            if !paths.is_empty() {
                s.push_str("；");
                s.push_str(&msg::journal_paths(paths.join("，")));
            }
            s
        }
        What::Retired { why, quiet_ms, age_ms, .. } => {
            // How long it was quiet in ms: 367 ms read 「1 秒」 rounded up
            // (2026-10-16), which said the opposite of what happened.
            msg::journal_retired(why_word(why), quiet_ms, secs(*age_ms))
        }
        What::Departed => msg::journal_departed().to_owned(),
        What::Inbound => msg::journal_inbound().to_owned(),
        What::Network { source } => msg::journal_network(match source.as_str() {
            "wake" => msg::journal_net_wake(),
            "path" => msg::journal_net_path(),
            "relay" => msg::journal_net_relay(),
            "addrs" => msg::journal_net_addrs(),
            _ => msg::journal_net_clock(),
        }),
        What::Addrs { addrs } if addrs.is_empty() => msg::journal_addrs_none().to_owned(),
        What::Addrs { addrs } => msg::journal_addrs(addrs.join("，")),
        What::Home { url: None, .. } => msg::journal_home_none().to_owned(),
        What::Home { url: Some(url), state, error } => {
            let state = match (state.as_str(), error) {
                ("connected", _) => msg::journal_home_connected().to_owned(),
                ("connecting", _) => msg::journal_home_connecting().to_owned(),
                (_, Some(why)) => msg::journal_home_down_why(scrub(why)),
                _ => msg::journal_home_down().to_owned(),
            };
            msg::journal_home(scrub(url), state)
        }
        What::Ears { ms, error: None } => msg::journal_ears(ms),
        What::Ears { error: Some(why), .. } => msg::journal_ears_failed(scrub(why)),
        What::Rest => msg::journal_rest().to_owned(),
        What::Front { ms } => msg::journal_front(ms),
        What::Hop { via: Some(via), .. } => msg::journal_hop_up(name_of(via)),
        What::Hop { via: None, tried } => msg::journal_hop_none(tried),
        What::Carry { up: true, .. } => msg::journal_carry_up().to_owned(),
        What::Carry { up: false, why } => msg::journal_carry_down(scrub(why.as_deref().unwrap_or(""))),
        What::Person { by } if by == "page" => msg::journal_person_page().to_owned(),
        What::Person { .. } => msg::journal_person_face().to_owned(),
        What::Learnt { v4: None, v6: None, .. } => msg::journal_learnt_none().to_owned(),
        What::Learnt { v4, v6, varies, from } => {
            let addrs: Vec<&str> = v4.iter().chain(v6.iter()).map(String::as_str).collect();
            let mut s = msg::journal_learnt(addrs.join("，"), from.join("、"));
            if *varies == Some(true) {
                s.push_str(msg::journal_learnt_varies());
            }
            s
        }
        What::Tun { src: Some(src) } => msg::journal_tun(src),
        What::Tun { src: None } => msg::journal_tun_off().to_owned(),
        What::Path { change, road, addr, rtt_ms } => {
            let mut s = if change == "opened" {
                msg::journal_path_opened(addr)
            } else if road == "carried" && addr == "ssh" {
                msg::journal_path_ssh().to_owned()
            } else if road == "carried" && !addr.is_empty() {
                msg::journal_path_hop(name_of(addr))
            } else {
                msg::journal_path_selected(road_word(road), addr)
            };
            if let Some(ms) = rtt_ms {
                s.push_str(&msg::journal_path_rtt(ms));
            }
            s
        }
        What::PathsMissed { missed } => msg::journal_paths_missed(missed),
        What::Round { local, remote, rounds } => {
            let none = || msg::journal_round_none().to_owned();
            let list = |v: &[String]| if v.is_empty() { none() } else { v.join("，") };
            msg::journal_round(rounds, list(local), list(remote))
        }
        What::Sync { doc, error } => msg::journal_sync_failed(doc, scrub(error)),
        What::Apart { doc, error } => msg::journal_sync_apart(doc, scrub(error)),
        What::PenRetired { doc, name } => msg::journal_pen_retired(doc, name),
    }
}

/// Whether an entry is something going wrong — what a reader's eye
/// should land on.
pub fn is_trouble(e: &Entry) -> bool {
    match &e.what {
        What::Dial { won, .. } => won.is_none(),
        What::Retired { why, .. } => !matches!(why.as_str(), "replaced" | "paused" | "suspended"),
        What::Home { state, .. } => state == "disconnected",
        What::Ears { error, .. } => error.is_some(),
        What::Hop { via, .. } => via.is_none(),
        What::Carry { up, .. } => !up,
        What::Departed => true,
        What::Sync { .. } | What::Apart { .. } | What::PenRetired { .. } => true,
        _ => false,
    }
}

/// `HH:MM:SS` of an entry, local time; the date too when it is not today.
pub fn when(at: u64) -> String {
    let Ok(ts) = jiff::Timestamp::from_millisecond(at as i64) else {
        return String::new();
    };
    let zoned = ts.to_zoned(jiff::tz::TimeZone::system());
    let today = jiff::Zoned::now().date();
    if zoned.date() == today {
        zoned.strftime("%H:%M:%S").to_string()
    } else {
        zoned.strftime("%m-%d %H:%M:%S").to_string()
    }
}

/// **This machine's side, as iroh sees it**, for as long as `ep` lives:
/// its direct addresses (they move when iroh's own network monitor sees
/// interfaces change — a Wi-Fi switch, a VPN or proxy TUN coming up —
/// which no khor code is told of) and its home relay's connection state.
/// Both are iroh's watchers: each line is written when iroh changes the
/// value, nothing is polled.
///
/// **And what the resident is told** (`changed`, NET-STABLE 第二步): the
/// home relay connected again after it was down is
/// [`Change::Relay`] — measured, a 240 s outage left both sides in their
/// backoff for 70 s after the relay was back, and a restart was the cure.
/// The interface addresses moving is [`Change::Addrs`], once they have
/// held still for [`ADDRS_SETTLE`] ([`Interfaces`]); the addresses iroh
/// learns from QAD or a port mapping are not interfaces and move on
/// their own, so they are not looked at.
pub(crate) fn watch_ears(
    ep: &Arc<iroh::Endpoint>,
    wires: Arc<crate::wires::Wires>,
    changed: impl Fn(Change) + Send + 'static,
) {
    use iroh::Watcher;
    let mut addrs = ep.watch_addr();
    let mut home = ep.home_relay_status();
    let mut direct = ep.watch_direct_addrs();
    let mut report = ep.net_report();
    let mut rounds = ep.holepunch_rounds();
    // Owned: the watch never keeps old ears alive.
    let closed = ep.closed();
    tokio::spawn(async move {
        tokio::pin!(closed);
        let ips = |a: &iroh::EndpointAddr| {
            let mut v: Vec<String> = a
                .addrs
                .iter()
                .filter_map(|t| match t {
                    iroh::TransportAddr::Ip(s) => Some(s.to_string()),
                    _ => None,
                })
                .collect();
            v.sort();
            v
        };
        let homes = |h: &[iroh::endpoint::RelayStatus]| -> Vec<(String, String, Option<String>)> {
            h.iter()
                .map(|r| {
                    let state = if r.is_connected() {
                        "connected"
                    } else if r.last_error().is_some() {
                        "disconnected"
                    } else {
                        "connecting"
                    };
                    (
                        scrub(r.url().as_str().trim_end_matches('/')),
                        state.to_owned(),
                        r.last_error().map(|e| scrub(&e.to_string())),
                    )
                })
                .collect()
        };
        let mut was_ips = ips(&addrs.get());
        let mut was_home: Vec<(String, String, Option<String>)> = Vec::new();
        // Since when the home relay has been down; `None` while it is up
        // or has never been.
        let mut down_at: Option<std::time::Instant> = None;
        let mut interfaces = Interfaces::new(interface_addrs(&direct.get(), &[]));
        let mut learnt = Learnt::default();
        let mut punching = Punching::default();
        let mut tun = tun_source();
        wires.note(None, What::Addrs { addrs: was_ips.clone() });
        if tun.is_some() {
            wires.note(None, What::Tun { src: tun.clone() });
        }
        for (url, state, error) in homes(&home.get()) {
            if state == "disconnected" {
                down_at.get_or_insert_with(std::time::Instant::now);
            }
            wires.note(None, What::Home { url: Some(url.clone()), state: state.clone(), error: error.clone() });
            was_home.push((url, state, error));
        }
        loop {
            let due = interfaces.due;
            tokio::select! {
                () = &mut closed => return,
                got = direct.updated() => {
                    let Ok(now) = got else { return };
                    let set = interface_addrs(&now, &interfaces.told);
                    interfaces.saw(set, std::time::Instant::now());
                }
                () = tokio::time::sleep_until(tokio::time::Instant::from_std(due.unwrap_or_else(std::time::Instant::now))), if due.is_some() => {
                    if interfaces.settled() {
                        changed(Change::Addrs);
                        let now = tun_source();
                        if now != tun {
                            wires.note(None, What::Tun { src: now.clone() });
                            tun = now;
                        }
                    }
                }
                got = report.updated() => {
                    let Ok(now) = got else { return };
                    if let Some(w) = now.as_ref().and_then(|r| learnt.saw(r)) {
                        wires.note(None, w);
                    }
                }
                got = rounds.recv() => match got {
                    Ok(round) => {
                        wires.counts.punch_rounds.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let peer = round.remote.to_string();
                        if let Some(w) = punching.saw(round, std::time::Instant::now()) {
                            wires.note(Some(&peer), w);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                },
                got = addrs.updated() => {
                    let Ok(now) = got else { return };
                    let now = ips(&now);
                    if now != was_ips {
                        wires.note(None, What::Addrs { addrs: now.clone() });
                        was_ips = now;
                    }
                }
                got = home.updated() => {
                    let Ok(now) = got else { return };
                    let now = homes(&now);
                    if now.is_empty() && !was_home.is_empty() {
                        wires.note(None, What::Home { url: None, state: "disconnected".into(), error: None });
                        down_at.get_or_insert_with(std::time::Instant::now);
                    }
                    // **Only up and down**: iroh's reconnect walks
                    // connecting → failed many times a second at the start
                    // of an outage (its backoff starts at 10 ms) — measured
                    // 56 lines for one 45 s cut. Connecting is not a change.
                    for (url, state, error) in &now {
                        if state == "connecting" {
                            // **Except straight from connected**: an
                            // established connection that drops is set
                            // disconnected and at once connecting again, and
                            // this watch sees only the latter — the aps1
                            // redials every few minutes went unwritten
                            // (audit/net-d.md g.4). It is a drop.
                            let was = was_home.iter().find(|(u, ..)| u == url).map(|(_, s, _)| s.as_str());
                            if was == Some("connected") {
                                let down = "disconnected".to_owned();
                                wires.note(None, What::Home { url: Some(url.clone()), state: down.clone(), error: error.clone() });
                                was_home.retain(|(u, ..)| u != url);
                                was_home.push((url.clone(), down, error.clone()));
                                down_at.get_or_insert_with(std::time::Instant::now);
                            }
                            continue;
                        }
                        let was = was_home.iter().find(|(u, ..)| u == url).map(|(_, s, _)| s.as_str());
                        if was != Some(state.as_str()) {
                            wires.note(None, What::Home { url: Some(url.clone()), state: state.clone(), error: error.clone() });
                            was_home.retain(|(u, ..)| u != url);
                            was_home.push((url.clone(), state.clone(), error.clone()));
                            if state == "connected" {
                                // Down, then up: not the first connect.
                                if let Some(at) = down_at.take() {
                                    changed(Change::Relay { down_at: at });
                                }
                            } else {
                                down_at.get_or_insert_with(std::time::Instant::now);
                            }
                        }
                    }
                    was_home.retain(|(u, ..)| now.iter().any(|(n, ..)| n == u));
                }
            }
        }
    });
}

/// **What iroh learnt of this machine's public side, told when it
/// changes** ([`watch_ears`]): the addresses' IPs and whether the NAT maps
/// per destination. Ports are left out of the comparison — behind a NAT
/// that maps per destination each fresh QAD connection gets a new one, and
/// a line every five minutes would say nothing new.
#[derive(Debug, Default)]
struct Learnt {
    told: Option<(Option<std::net::IpAddr>, Option<std::net::IpAddr>, Option<bool>)>,
}

impl Learnt {
    fn saw(&mut self, r: &iroh::unstable_net_report::NetReport) -> Option<What> {
        use iroh::unstable_net_report::Probe;
        let varies = r.mapping_varies_by_dest_ipv4.or(r.mapping_varies_by_dest_ipv6);
        let key = (
            r.global_v4.map(|a| std::net::IpAddr::V4(*a.ip())),
            r.global_v6.map(|a| std::net::IpAddr::V6(*a.ip())),
            varies,
        );
        if self.told.as_ref() == Some(&key) {
            return None;
        }
        self.told = Some(key);
        let mut from: Vec<String> = r
            .relay_latency
            .iter()
            .filter(|(p, ..)| matches!(p, Probe::QadIpv4 | Probe::QadIpv6))
            .map(|(_, url, _)| scrub(url.as_str().trim_end_matches('/')))
            .collect();
        from.dedup();
        Some(What::Learnt {
            v4: r.global_v4.map(|a| a.to_string()),
            v6: r.global_v6.map(|a| a.to_string()),
            varies,
            from,
        })
    }
}

/// The sync failures last written, per (machine, document).
static SYNC_FAILED: std::sync::LazyLock<Mutex<HashMap<(String, String), String>>> =
    std::sync::LazyLock::new(Default::default);

/// **A must-sync document failed with the peer** (`link::sync_what_changed`):
/// the line to write, or `None` when the same words were written last
/// time. `error` `None` is a success, which clears the memory so the next
/// failure is told again.
pub(crate) fn sync_failed(peer: &str, doc: &str, error: Option<&str>) -> Option<What> {
    let mut last = SYNC_FAILED.lock().unwrap_or_else(|p| p.into_inner());
    let key = (peer.to_owned(), doc.to_owned());
    let Some(error) = error else {
        last.remove(&key);
        return None;
    };
    if last.get(&key).map(String::as_str) == Some(error) {
        return None;
    }
    last.insert(key, error.to_owned());
    if khor_sync::wire::other_epoch(error) {
        return Some(What::Apart { doc: doc.to_owned(), error: error.to_owned() });
    }
    Some(What::Sync { doc: doc.to_owned(), error: error.to_owned() })
}

/// How often a round of hole punching with unchanged candidates is
/// written: iroh starts one every 60 s while a connection has no direct
/// path of 10 ms or less, and a line a minute per machine would push the
/// dials out of the file.
pub const ROUND_EVERY: Duration = Duration::from_secs(10 * 60);

/// The rounds of hole punching per machine, and which of them are written
/// ([`What::Round`]).
#[derive(Debug, Default)]
struct Punching {
    /// Per machine: the candidates last written, rounds since, written when.
    last: HashMap<iroh::EndpointId, (Vec<String>, Vec<String>, u32, std::time::Instant)>,
}

impl Punching {
    fn saw(&mut self, round: iroh::endpoint::HolepunchRound, now: std::time::Instant) -> Option<What> {
        let sorted = |v: &[std::net::SocketAddr]| {
            let mut v: Vec<String> = v.iter().map(|a| a.to_string()).collect();
            v.sort();
            v
        };
        let (local, remote) = (sorted(&round.local), sorted(&round.remote_candidates));
        let first = !self.last.contains_key(&round.remote);
        let entry = self.last.entry(round.remote).or_insert_with(|| (Vec::new(), Vec::new(), 0, now));
        entry.2 += 1;
        let fresh = first || entry.0 != local || entry.1 != remote;
        if !fresh && now.duration_since(entry.3) < ROUND_EVERY {
            return None;
        }
        let rounds = entry.2;
        *entry = (local.clone(), remote.clone(), 0, now);
        Some(What::Round { local, remote, rounds })
    }
}

/// The address this machine's internet traffic leaves from, when that is
/// a proxy TUN's fake-IP range (198.18.0.0/15); `None` otherwise. Asked of
/// the kernel by connecting a UDP socket, which sends nothing.
fn tun_source() -> Option<String> {
    // A patchbay lab's internet is that range (`endpoint::LAB_ENV`).
    if khor_net::endpoint::in_lab() {
        return None;
    }
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("1.1.1.1:443").ok()?;
    match s.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(v4) if v4.octets()[0] == 198 && matches!(v4.octets()[1], 18 | 19) => {
            Some(v4.to_string())
        }
        _ => None,
    }
}

/// How long the interface addresses hold still before their change is
/// told ([`watch_ears`]): a switch passes through several sets — the old
/// going, nothing, the new arriving — within a second or two.
const ADDRS_SETTLE: Duration = Duration::from_secs(1);

/// This machine's interface addresses among iroh's direct ones: bound
/// locally, loopback left out — iroh lists loopback only when there is
/// nothing else, which is no network rather than a new one.
///
/// **Judged by the addresses, not by their source**: an address told as an
/// interface's stays one for as long as it is among iroh's direct
/// addresses at all, whatever source iroh gives it now. A global IPv6 with
/// no NAT in front is both an interface's and what QAD reports, and iroh
/// listed it as one or the other report by report (vendor/iroh patch 20
/// fixes the listing); every flip was told as a network change, every
/// 40–60 s on turing (2026-10-16), and each was a round with every machine.
fn interface_addrs(
    all: &std::collections::BTreeSet<iroh::endpoint::DirectAddr>,
    told: &[std::net::SocketAddr],
) -> Vec<std::net::SocketAddr> {
    let mut v: Vec<std::net::SocketAddr> = all
        .iter()
        .filter(|d| (d.typ == iroh::endpoint::DirectAddrType::Local || told.contains(&d.addr)) && !d.addr.ip().is_loopback())
        .map(|d| d.addr)
        .collect();
    v.sort();
    v.dedup();
    v
}

/// **The interface addresses the resident was last told about, and a
/// newer set waiting to hold still** ([`watch_ears`]).
#[derive(Debug)]
struct Interfaces {
    told: Vec<std::net::SocketAddr>,
    seen: Vec<std::net::SocketAddr>,
    /// When `seen` will have held still for [`ADDRS_SETTLE`]; `None`
    /// while it is what was told.
    due: Option<std::time::Instant>,
}

impl Interfaces {
    fn new(now: Vec<std::net::SocketAddr>) -> Self {
        Interfaces {
            told: now.clone(),
            seen: now,
            due: None,
        }
    }

    /// iroh's set, at `at`: each new set starts the wait again, and one
    /// back to what was told ends it.
    fn saw(&mut self, set: Vec<std::net::SocketAddr>, at: std::time::Instant) {
        self.due = (set != self.told).then_some(at + ADDRS_SETTLE);
        self.seen = set;
    }

    /// The wait is over: answers whether to tell. **An empty set is kept
    /// and not told** — with no network there is nothing to dial — so the
    /// network coming back, even on the same addresses, is a change.
    fn settled(&mut self) -> bool {
        self.due = None;
        self.told = self.seen.clone();
        !self.told.is_empty()
    }
}

/// What a dial offered: how many direct addresses, which relays (by
/// URL, scrubbed), and whether a carried road.
pub(crate) fn offered(addr: &iroh::EndpointAddr) -> (u32, Vec<String>, bool) {
    let mut ips = 0u32;
    let mut relays = Vec::new();
    let mut carried = false;
    for t in &addr.addrs {
        match t {
            iroh::TransportAddr::Ip(_) => ips = ips.saturating_add(1),
            iroh::TransportAddr::Relay(u) => relays.push(scrub(u.as_str().trim_end_matches('/'))),
            _ => carried = true,
        }
    }
    (ips, relays, carried)
}

/// How long ago a wire was last heard, for [`What::Retired`].
pub(crate) fn ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// One entry as a reader shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Line {
    pub at: u64,
    /// `HH:MM:SS`, local ([`when`]).
    pub when: String,
    /// The far machine's id; `None` for this machine's own events.
    pub peer: Option<String>,
    /// Its name as the table has it, or the start of its id.
    pub machine: String,
    pub text: String,
    /// Something going wrong ([`is_trouble`]).
    pub trouble: bool,
}

impl crate::Node {
    /// The journal of one machine (with this machine's own events
    /// interleaved), or of all when `peer` is `None`, over the last
    /// `minutes`.
    pub fn journal(&self, peer: Option<&str>, minutes: u64) -> Vec<Line> {
        let since = crate::now_ms().saturating_sub(minutes.saturating_mul(60_000));
        self.journal_lines(read(self.root(), peer, since), msg::journal_here())
    }

    /// **Another machine's journal, read from here** (net-r): both ends of
    /// a reconnect side by side — what this machine saw, and what the far
    /// one did meanwhile (its new ears, its own dials). `peer` narrows to
    /// that machine's file and the far one's own events, which are named
    /// by the far machine's name, not 「本机」.
    pub async fn journal_on(&self, machine: &str, peer: Option<&str>, minutes: u64) -> Result<Vec<Line>, String> {
        let since = crate::now_ms().saturating_sub(minutes.saturating_mul(60_000));
        let lines = self.journal_lines_of(machine, peer.map(str::to_owned), since).await?;
        let entries = lines.iter().filter_map(|l| serde_json::from_str::<Entry>(l).ok()).collect();
        let id = self.device_by_name(machine)?;
        let there = self
            .devices()
            .unwrap_or_default()
            .into_iter()
            .find(|d| d.id == id)
            .map_or_else(|| machine.to_owned(), |d| d.shown().to_owned());
        Ok(self.journal_lines(entries, &there))
    }

    /// Entries as a reader shows them, machines by the table's names;
    /// `here` names the events of the machine whose journal it is.
    fn journal_lines(&self, entries: Vec<Entry>, here: &str) -> Vec<Line> {
        let names: HashMap<String, String> = self
            .devices()
            .unwrap_or_default()
            .into_iter()
            // The name a person reads (the rename), not the channel: the
            // iPhone's channel is its hostname, 「localhost」.
            .map(|d| (d.shown().to_owned(), d.id))
            .map(|(shown, id)| (id, shown))
            .collect();
        let name_of = |id: &str| {
            names
                .get(id)
                .cloned()
                .unwrap_or_else(|| id.chars().take(8).collect())
        };
        entries
            .into_iter()
            .map(|e| Line {
                at: e.at,
                when: when(e.at),
                machine: e.peer.as_deref().map_or_else(|| here.to_owned(), name_of),
                text: words(&e, &name_of),
                trouble: is_trouble(&e),
                peer: e.peer,
            })
            .collect()
    }

    /// **Freezes the last `minutes` of every machine's journal into one
    /// file** the owner can hand over: `.khor/reports/connections-<time>.txt`
    /// under this home. A header says which machine, which version, which
    /// relays (URLs only); then one line per event. Nothing in it is a
    /// secret ([`scrub`] on every sentence a library wrote).
    pub fn journal_report(&self, minutes: u64) -> Result<PathBuf, String> {
        let lines = self.journal(None, minutes);
        let mut out = String::new();
        out.push_str(msg::journal_report_title());
        out.push('\n');
        out.push_str(&jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S %Z").to_string());
        out.push('\n');
        let short: String = self.device_str().chars().take(8).collect();
        out.push_str(&msg::journal_report_machine(self.shown_name(), short));
        out.push('\n');
        out.push_str(&msg::journal_report_version(format_args!(
            "{} {} {}",
            crate::VERSION,
            std::env::consts::OS,
            std::env::consts::ARCH
        )));
        out.push('\n');
        let relays: Vec<String> = self.relays().iter().map(|r| scrub(r)).collect();
        out.push_str(&msg::journal_report_relays(relays.join("，")));
        out.push('\n');
        out.push_str(&msg::journal_report_span(minutes, lines.len()));
        out.push_str("\n\n");
        let since = crate::now_ms().saturating_sub(minutes.saturating_mul(60_000));
        let verdicts = not_direct(&read(self.root(), None, since));
        if !verdicts.is_empty() {
            let names: HashMap<String, String> = self
                .devices()
                .unwrap_or_default()
                .into_iter()
                // The name a person reads (the rename), not the channel: the
                // iPhone's channel is its hostname, 「localhost」.
                .map(|d| (d.shown().to_owned(), d.id))
                .map(|(shown, id)| (id, shown))
                .collect();
            out.push_str(msg::journal_why_title());
            out.push('\n');
            for (peer, road, why) in &verdicts {
                let name = names.get(peer).cloned().unwrap_or_else(|| peer.chars().take(8).collect());
                let why: Vec<String> = why.iter().map(Why::words).collect();
                out.push_str(&msg::journal_why_line(name, road_word(road), why.join("；")));
                out.push('\n');
            }
            out.push('\n');
        }
        for l in &lines {
            let mark = if l.trouble { "!" } else { " " };
            out.push_str(&format!("{} {mark} {}\t{}\n", l.when, l.machine, l.text));
        }
        let dir = self.root().join(".khor").join("reports");
        std::fs::create_dir_all(&dir).map_err(|e| msg::journal_report_cant(e))?;
        let path = dir.join(format!(
            "connections-{}.txt",
            jiff::Zoned::now().strftime("%Y%m%d-%H%M%S")
        ));
        std::fs::write(&path, out).map_err(|e| msg::journal_report_cant(e))?;
        Ok(path)
    }
}

/// One reason a machine's connection is not direct ([`not_direct`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// No round of hole punching from this side was written.
    NoRound,
    /// It offers a public IPv6 address and this machine has none.
    V6Here,
    /// This machine has a public IPv6 address and it offers none.
    V6There,
    /// Neither has public IPv6, and this machine's NAT maps per destination.
    HardNat,
    /// The default route goes through a proxy's TUN.
    Tun,
    /// No relay told this machine its public address.
    NoPublic,
    /// This many rounds, and nothing above explains it: its candidates did
    /// not answer.
    Punched(u32),
}

impl Why {
    fn words(&self) -> String {
        match self {
            Why::NoRound => msg::journal_why_no_round().to_owned(),
            Why::V6Here => msg::journal_why_v6_here().to_owned(),
            Why::V6There => msg::journal_why_v6_there().to_owned(),
            Why::HardNat => msg::journal_why_hard_nat().to_owned(),
            Why::Tun => msg::journal_why_tun().to_owned(),
            Why::NoPublic => msg::journal_why_no_public().to_owned(),
            Why::Punched(n) => msg::journal_why_punched(n),
        }
    }
}

/// Whether an address is a global unicast IPv6 one (2000::/3).
fn global_v6(addr: &str) -> bool {
    match addr.parse::<std::net::SocketAddr>() {
        Ok(std::net::SocketAddr::V6(a)) => a.ip().segments()[0] & 0xe000 == 0x2000,
        _ => false,
    }
}

/// **Per machine whose latest road is not direct, why** (NET-DIRECT, the
/// report's 「为什么不是直连」): read off the journal alone — this machine's
/// addresses, what the relays told it, the TUN, and that machine's rounds
/// of hole punching with the candidates it offered. Oldest first in, one
/// verdict per machine out, by id.
pub fn not_direct(entries: &[Entry]) -> Vec<(String, String, Vec<Why>)> {
    let mut here_v6 = false;
    let mut learnt_v4 = false;
    let mut varies = None;
    let mut tun = false;
    // Per machine: latest road, rounds counted, its latest candidates.
    let mut peers: std::collections::BTreeMap<String, (Option<String>, u32, Vec<String>)> =
        Default::default();
    for e in entries {
        match (&e.peer, &e.what) {
            (None, What::Addrs { addrs }) => here_v6 = addrs.iter().any(|a| global_v6(a)),
            (None, What::Learnt { v4, v6, varies: v, .. }) => {
                learnt_v4 = v4.is_some();
                here_v6 |= v6.is_some();
                varies = *v;
            }
            (None, What::Tun { src }) => tun = src.is_some(),
            (Some(p), What::Settled { road, .. }) => {
                peers.entry(p.clone()).or_default().0 = Some(road.clone());
            }
            (Some(p), What::Path { change, road, .. }) if change == "selected" => {
                peers.entry(p.clone()).or_default().0 = Some(road.clone());
            }
            (Some(p), What::Round { remote, rounds, .. }) => {
                let peer = peers.entry(p.clone()).or_default();
                peer.1 += rounds;
                peer.2 = remote.clone();
            }
            _ => {}
        }
    }
    peers
        .into_iter()
        .filter_map(|(peer, (road, rounds, remote))| {
            let road = road?;
            if road == "direct" {
                return None;
            }
            let mut why = Vec::new();
            if rounds == 0 {
                why.push(Why::NoRound);
            } else {
                let there_v6 = remote.iter().any(|a| global_v6(a));
                match (here_v6, there_v6) {
                    (false, true) => why.push(Why::V6Here),
                    (true, false) => why.push(Why::V6There),
                    (false, false) if varies == Some(true) => why.push(Why::HardNat),
                    _ => {}
                }
            }
            if tun {
                why.push(Why::Tun);
            }
            if !learnt_v4 {
                why.push(Why::NoPublic);
            }
            if why.is_empty() {
                why.push(Why::Punched(rounds));
            }
            Some((peer, road, why))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The report's 「为什么不是直连」, from MacLab's night of 10-09**: no
    /// IPv6 here, the iPhone offering Unicom v6, a NAT that maps per
    /// destination, the proxy's TUN — and turing, direct, left out.
    #[test]
    fn why_not_direct_reads_the_journal() {
        let at = |n: u64, peer: Option<&str>, what: What| Entry { at: n, peer: peer.map(str::to_owned), what };
        let entries = vec![
            at(1, None, What::Addrs { addrs: vec!["59.64.53.85:57876".into(), "198.18.0.1:57876".into()] }),
            at(2, None, What::Tun { src: Some("198.18.0.1".into()) }),
            at(3, None, What::Learnt {
                v4: Some("219.142.45.253:52498".into()),
                v6: None,
                varies: Some(true),
                from: vec!["https://aps1-1.relay.n0.iroh.link.".into()],
            }),
            at(4, Some("phone"), What::Settled { road: "relay".into(), relay: None, rtt_ms: 27, paths: vec![] }),
            at(5, Some("phone"), What::Round {
                local: vec!["59.64.53.85:57876".into()],
                remote: vec!["[2408:8409:18d0:5ff8::1]:61723".into(), "10.1.2.3:61723".into()],
                rounds: 3,
            }),
            at(6, Some("turing"), What::Settled { road: "relay".into(), relay: None, rtt_ms: 30, paths: vec![] }),
            at(7, Some("turing"), What::Path { change: "selected".into(), road: "direct".into(), addr: "172.16.185.103:35156".into(), rtt_ms: Some(2) }),
            at(8, Some("lecun"), What::Settled { road: "carried".into(), relay: None, rtt_ms: 2, paths: vec![] }),
        ];
        let v = not_direct(&entries);
        assert_eq!(
            v,
            vec![
                ("lecun".to_owned(), "carried".to_owned(), vec![Why::NoRound, Why::Tun]),
                ("phone".to_owned(), "relay".to_owned(), vec![Why::V6Here, Why::Tun]),
            ],
            "turing went direct and is not listed"
        );
        // Both with v6, nothing else wrong: the rounds are the answer.
        let entries = vec![
            at(1, None, What::Addrs { addrs: vec!["[2001:da8:207:e212::5]:35060".into()] }),
            at(2, None, What::Learnt { v4: Some("1.2.3.4:5".into()), v6: None, varies: Some(false), from: vec![] }),
            at(3, Some("p"), What::Settled { road: "relay".into(), relay: None, rtt_ms: 9, paths: vec![] }),
            at(4, Some("p"), What::Round { local: vec![], remote: vec!["[2408::1]:1".into()], rounds: 2 }),
            at(5, Some("p"), What::Round { local: vec![], remote: vec!["[2408::1]:1".into()], rounds: 5 }),
        ];
        assert_eq!(not_direct(&entries), vec![("p".to_owned(), "relay".to_owned(), vec![Why::Punched(7)])]);
    }

    /// Rounds with unchanged candidates are written once per
    /// [`ROUND_EVERY`], with the count since; new candidates at once.
    #[test]
    fn rounds_are_written_on_change_and_then_sparingly() {
        let id = iroh::SecretKey::generate().public();
        let a: std::net::SocketAddr = "1.2.3.4:5".parse().unwrap();
        let b: std::net::SocketAddr = "[2408::1]:6".parse().unwrap();
        let round = |remote: Vec<std::net::SocketAddr>| iroh::endpoint::HolepunchRound {
            remote: id,
            local: vec![a],
            remote_candidates: remote,
        };
        let t = std::time::Instant::now();
        let mut p = Punching::default();
        assert!(matches!(p.saw(round(vec![a]), t), Some(What::Round { rounds: 1, .. })), "the first is written");
        for i in 1..5 {
            assert_eq!(p.saw(round(vec![a]), t + Duration::from_secs(60 * i)), None, "unchanged within the wait");
        }
        assert!(matches!(p.saw(round(vec![a]), t + ROUND_EVERY), Some(What::Round { rounds: 5, .. })), "then with the count");
        assert!(matches!(p.saw(round(vec![a, b]), t + ROUND_EVERY), Some(What::Round { rounds: 1, .. })), "new candidates at once");
    }

    /// **A switch is told once, after it holds still** (NET-STABLE 第二步):
    /// the old address going, nothing, the new one arriving within the
    /// settle is one change; a set back to what was told is none; the
    /// network gone is not told, and back on the same address is.
    #[test]
    fn interface_addresses_are_told_once_they_hold_still() {
        let a: std::net::SocketAddr = "192.168.1.5:7000".parse().unwrap();
        let b: std::net::SocketAddr = "10.0.0.7:7000".parse().unwrap();
        let t = std::time::Instant::now();
        let ms = |n: u64| t + Duration::from_millis(n);
        let mut i = Interfaces::new(vec![a]);
        i.saw(vec![], ms(0));
        i.saw(vec![b], ms(300));
        assert_eq!(i.due, Some(ms(300) + ADDRS_SETTLE), "each set starts the wait again");
        assert!(i.settled(), "a new network is told");
        assert_eq!(i.due, None);
        i.saw(vec![a], ms(2000));
        i.saw(vec![b], ms(2200));
        assert_eq!(i.due, None, "back to what was told: nothing to tell");
        i.saw(vec![], ms(3000));
        assert!(!i.settled(), "no network is not told");
        i.saw(vec![b], ms(9000));
        assert!(i.due.is_some());
        assert!(i.settled(), "the network back on the same address is told");
    }

    /// An interface's address that iroh lists as learnt by QAD in one
    /// report and as an interface's in the next is no network change.
    #[test]
    fn an_address_that_changes_only_its_source_is_no_change() {
        use iroh::endpoint::{DirectAddr, DirectAddrType};
        let v6: std::net::SocketAddr = "[2001:db8::5]:7000".parse().unwrap();
        let v4: std::net::SocketAddr = "192.168.1.5:7000".parse().unwrap();
        let set = |typ| std::collections::BTreeSet::from([DirectAddr { addr: v4, typ: DirectAddrType::Local }, DirectAddr { addr: v6, typ }]);
        let told = interface_addrs(&set(DirectAddrType::Local), &[]);
        assert_eq!(told, vec![v4, v6]);
        assert_eq!(interface_addrs(&set(DirectAddrType::Qad), &told), told, "the source moved, the set did not");
        assert_eq!(interface_addrs(&set(DirectAddrType::Qad), &[]), vec![v4], "learnt only by QAD is no interface");
        let gone = std::collections::BTreeSet::from([DirectAddr { addr: v4, typ: DirectAddrType::Local }]);
        assert_eq!(interface_addrs(&gone, &told), vec![v4], "gone from the list is gone");
    }

    #[test]
    fn a_file_is_cut_back_to_its_last_lines_and_reads_in_order() {
        let root = crate::testdir::TempTree::fresh("journal-ring");
        let j = Journal::open(&root.path());
        for i in 0..(2 * KEEP + 5) {
            j.note(Some("abc"), What::Hop { via: None, tried: i as u32 });
        }
        j.note(None, What::Started);
        let all = read(&root.path(), Some("abc"), 0);
        let hops = all.iter().filter(|e| matches!(e.what, What::Hop { .. })).count();
        assert!(hops <= 2 * KEEP && hops >= KEEP, "kept {hops}");
        assert!(matches!(all.last().unwrap().what, What::Started));
        let last = all.iter().rev().find_map(|e| match e.what {
            What::Hop { tried, .. } => Some(tried),
            _ => None,
        });
        assert_eq!(last, Some((2 * KEEP + 4) as u32), "the newest line survives the cut");
    }

    /// **A file that never opens does not hold the one noting** (net-dial
    /// B1): a FIFO nobody reads is the stand-in for turing's NFS home,
    /// where an open or a write can stay out for as long as the server
    /// does. `note` used to append in its caller's thread — the wire
    /// watch's tick and every retirement among them, under the wires'
    /// lock — so that caller stayed out with it.
    #[test]
    fn a_journal_file_that_never_opens_does_not_hold_the_caller() {
        use std::os::unix::ffi::OsStrExt;
        let root = crate::testdir::TempTree::fresh("journal-stuck");
        let d = dir(&root.path());
        std::fs::create_dir_all(&d).unwrap();
        let fifo = std::ffi::CString::new(d.join("stuck.jsonl").as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0, "a FIFO to open");
        let j = Journal::open(&root.path());
        let (tx, rx) = std::sync::mpsc::channel();
        {
            let j = j.clone();
            std::thread::spawn(move || {
                let t = std::time::Instant::now();
                for _ in 0..3 {
                    j.note(Some("stuck"), What::Departed);
                }
                j.note(None, What::Started);
                let _ = tx.send(t.elapsed());
            });
        }
        let took = rx.recv_timeout(Duration::from_secs(3));
        assert!(
            took.is_ok_and(|t| t < Duration::from_millis(500)),
            "noting returned at once while the file would not open: {took:?}"
        );
        // The writer behind it is stuck on the FIFO's open for good; its
        // thread is this test's to leak.
        assert!(!j.flush(Duration::from_millis(50)), "and the line is still waiting, not lost in the caller");
    }

    #[test]
    fn a_token_never_survives_into_the_words() {
        assert_eq!(
            scrub("relay https://r.example/?token=s3cret&x=1 refused"),
            "relay https://r.example/?token=…&x=1 refused"
        );
        assert_eq!(scrub("Authorization: Bearer abc.def"), "Authorization: Bearer …");
        assert_eq!(scrub("nothing here"), "nothing here");
    }

    /// net-e: a carried path names what carries it — 「改为通过 turing」,
    /// 「改为经 SSH 转连」 — where it said 「改走转连：，往返 0 ms」.
    #[test]
    fn a_carried_path_says_what_carries_it() {
        let name_of = |id: &str| if id == "b1" { "turing".to_owned() } else { id.to_owned() };
        let e = |addr: &str, rtt_ms| Entry {
            at: 0,
            peer: Some("y".into()),
            what: What::Path { change: "selected".into(), road: "carried".into(), addr: addr.into(), rtt_ms },
        };
        assert_eq!(words(&e("b1", None), &name_of), msg::journal_path_hop("turing"));
        assert_eq!(words(&e("ssh", None), &name_of), msg::journal_path_ssh());
        assert_eq!(
            words(&e("b1", Some(3)), &name_of),
            format!("{}{}", msg::journal_path_hop("turing"), msg::journal_path_rtt(3))
        );
    }

    /// **A pile kept in another epoch is said once, as itself** (PEERS-BRIEF
    /// step 3): not 「the round stopped there」, because it did not, and
    /// not again on every round while the two stay apart. Another
    /// failure is still a failure.
    #[test]
    fn a_pile_in_another_epoch_is_one_line_of_its_own() {
        let apart = msg::pile_other_epoch("devices", 0, khor_sync::store::EPOCH);
        let w = sync_failed("epoch-peer", "devices", Some(&apart)).expect("the first time is written");
        assert!(matches!(&w, What::Apart { doc, .. } if doc == "devices"), "{w:?}");
        let e = Entry { at: 1, peer: Some("epoch-peer".into()), what: w };
        assert_eq!(words(&e, &|s| s.to_owned()), msg::journal_sync_apart("devices", &apart));
        assert!(sync_failed("epoch-peer", "devices", Some(&apart)).is_none(), "and not again");
        let other = msg::version_undecodable("x");
        assert!(matches!(sync_failed("epoch-peer", "devices", Some(&other)), Some(What::Sync { .. })));
    }
}
