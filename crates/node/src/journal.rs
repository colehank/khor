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
}

impl Change {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Change::Wake => "wake",
            Change::Path => "path",
            Change::Clock => "clock",
            Change::Relay { .. } => "relay",
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
pub struct Journal {
    dir: PathBuf,
    /// Lines in each file as far as this writer knows, counted on first
    /// touch.
    lines: Mutex<HashMap<String, usize>>,
}

/// Where a home's journal lives.
pub fn dir(root: &Path) -> PathBuf {
    root.join(".khor").join("journal")
}

impl Journal {
    pub fn open(root: &Path) -> Arc<Journal> {
        Arc::new(Journal {
            dir: dir(root),
            lines: Mutex::new(HashMap::new()),
        })
    }

    /// Appends one event; `peer` `None` is this machine's own. Never
    /// fails the caller: a journal that cannot be written is a journal
    /// with a gap, not a dial that failed.
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
        let path = self.dir.join(format!("{key}.jsonl"));
        let mut lines = self.lines.lock().unwrap_or_else(|p| p.into_inner());
        let n = lines.entry(key).or_insert_with(|| {
            std::fs::read_to_string(&path)
                .map(|t| t.lines().count())
                .unwrap_or(0)
        });
        let _ = std::fs::create_dir_all(&self.dir);
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
}

/// Every entry of `peer` (and this machine's own, interleaved) — or of
/// every machine when `peer` is `None` — at or after `since` (ms), oldest
/// first. Lines that do not parse are skipped: a torn last line after a
/// crash is not a reason to read nothing.
pub fn read(root: &Path, peer: Option<&str>, since: u64) -> Vec<Entry> {
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
            msg::journal_retired(why_word(why), secs(*quiet_ms), secs(*age_ms))
        }
        What::Departed => msg::journal_departed().to_owned(),
        What::Inbound => msg::journal_inbound().to_owned(),
        What::Network { source } => msg::journal_network(match source.as_str() {
            "wake" => msg::journal_net_wake(),
            "path" => msg::journal_net_path(),
            "relay" => msg::journal_net_relay(),
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
        What::Hop { via: Some(via), .. } => msg::journal_hop_up(name_of(via)),
        What::Hop { via: None, tried } => msg::journal_hop_none(tried),
        What::Carry { up: true, .. } => msg::journal_carry_up().to_owned(),
        What::Carry { up: false, why } => msg::journal_carry_down(scrub(why.as_deref().unwrap_or(""))),
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
pub(crate) fn watch_ears(
    ep: &Arc<iroh::Endpoint>,
    wires: Arc<crate::wires::Wires>,
    changed: impl Fn(Change) + Send + 'static,
) {
    use iroh::Watcher;
    let mut addrs = ep.watch_addr();
    let mut home = ep.home_relay_status();
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
        wires.note(None, What::Addrs { addrs: was_ips.clone() });
        for (url, state, error) in homes(&home.get()) {
            if state == "disconnected" {
                down_at.get_or_insert_with(std::time::Instant::now);
            }
            wires.note(None, What::Home { url: Some(url.clone()), state: state.clone(), error: error.clone() });
            was_home.push((url, state, error));
        }
        loop {
            tokio::select! {
                () = &mut closed => return,
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
        let names: HashMap<String, String> = self
            .devices()
            .unwrap_or_default()
            .into_iter()
            .map(|d| (d.id, d.name))
            .collect();
        let name_of = |id: &str| {
            names
                .get(id)
                .cloned()
                .unwrap_or_else(|| id.chars().take(8).collect())
        };
        read(self.root(), peer, since)
            .into_iter()
            .map(|e| Line {
                at: e.at,
                when: when(e.at),
                machine: e.peer.as_deref().map_or_else(|| msg::journal_here().to_owned(), name_of),
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_token_never_survives_into_the_words() {
        assert_eq!(
            scrub("relay https://r.example/?token=s3cret&x=1 refused"),
            "relay https://r.example/?token=…&x=1 refused"
        );
        assert_eq!(scrub("Authorization: Bearer abc.def"), "Authorization: Bearer …");
        assert_eq!(scrub("nothing here"), "nothing here");
    }
}
