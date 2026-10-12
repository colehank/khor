//! The GUI's data layer: the same khor-node calls the CLI makes, shaped
//! into rows the frontend can render. Two skins share this module — the
//! tauri commands (apps/gui/src-tauri) and the dev bridge (`bridge`
//! bin) — so what the browser verifies is what the app ships. The LAN
//! listener a phone opens will be the third; [`api`] is the table it
//! will dispatch through, moved out of the bridge ahead of it (批⑦).
//!
//! No judgment lives here: words, ordering, unread all come from the
//! node. The GUI must not re-derive any of it (docs/UX.md 状态呈现).

pub mod acs;
pub mod add;
pub mod agents;
pub mod api;
pub mod chat;
pub mod code;
pub mod files;
pub mod git;
pub mod import;
pub mod miss;
mod landing;
pub mod qr;
pub mod recents;
pub mod rejoin;
pub mod tabs;
pub mod term;
pub mod web;
pub mod workspaces;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub use khor_core::QuotaAnswer;
use khor_core::map::{self, Seat};
use khor_node::list::Arrange;
use khor_node::{Avatar, AvatarStyle, FaceShape, Node, PRESETS, SessionId, Usage, Variant, Vitals};
use serde::Serialize;
use ts_rs::TS;

/// One list row. `word` is the state *key* (`busy`…); the frontend looks
/// the display word up in the catalog at the last moment (docs/UX.md 文案).
#[derive(Debug, Clone, Serialize, TS)]
pub struct SessionRow {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub word: String,
    /// ms; `number` on the TS side — serde_json sends a number, and
    /// 2^53 outlives every timestamp and unread count in question.
    #[ts(type = "number")]
    pub at_ms: u64,
    #[ts(type = "number")]
    pub unread: u64,
    /// Present on reported rows: the offline axis, never a seventh word.
    pub source: Option<SourceTag>,
    /// Pinned somewhere in the network (`khor_sync::pins`). The frontend
    /// paints a mark from this and **does not sort by it** — the rows
    /// arrive in the order the node decided.
    pub pinned: bool,
    /// On the list because it was imported (docs/SHELL.md 第八轮): 删除
    /// takes it off the list and nothing else.
    pub imported: bool,
    /// Old news the default view folds into a tail count
    /// (`khor_node::SessionView::stale` — dead, looked at, and dead
    /// past the one threshold). The frontend folds, it never judges:
    /// the row still arrives, search still reads it, nothing is
    /// deleted.
    pub stale: bool,
    /// The word has outlived its believability
    /// (`khor_node::SessionView::quiet`, 词的保鲜期批): grey the row
    /// and note how long the silence is — the word itself stands
    /// untouched, and the note's duration is the row's own `at_ms`
    /// read against now.
    pub quiet: bool,
    /// Whose session this is (`khor_core::Session::category`): `claude`,
    /// `codex`, `khor`, `shell`. `None` when nobody could place it, and
    /// the frontend must render that as its own group rather than
    /// picking a neighbour — **never inferred from the title here or
    /// there** (that is the whole reason it travels on the row).
    pub category: Option<String>,
    /// The group this row landed in, as `khor_node::list` keys them
    /// (`pin`, `cat:…`, `dev:…`, `state:…`, or empty when the mode does
    /// not group). The frontend starts a heading where this changes
    /// between neighbouring rows — **it never works out the grouping
    /// itself**, and the prefix tells it whether the rest is a word to
    /// look up or a name to print.
    pub group: String,
    /// The face of the machine this session lives on — derived here,
    /// never in the frontend (`khor_core::avatar`). `None` only when the
    /// home device is not in this table at all, and then the row draws a
    /// blank, **not an invented face**: a made-up face gets believed,
    /// and it is not that machine's.
    pub face: Option<Avatar>,
    /// This machine hosts the session's terminal (`Node::is_hosted`) — so
    /// a terminal can attach and paint it (docs/handoff 终端画屏). False
    /// for a discovered session, a `khor run`, or a remote one; those keep
    /// the transcript or the facts, not a live screen.
    pub hosted: bool,
    /// A terminal can be had here: hosted already, or something the
    /// attach bridge can stand a host up for on first open — a local
    /// discovered tmux session, or an agent sitting inside one (the
    /// row's own `Session::term`, via `Node::attach_tmux`). What the
    /// detail routes its terminal faces on — `hosted` alone would leave
    /// every one of those on the facts pane.
    pub attachable: bool,
    /// The row's preview line (`khor_core::Session::last`): the newest
    /// message, transcript utterance or terminal line — the kind judged
    /// it, this only paints it.
    pub last: Option<String>,
    /// What the session's shell is doing (`khor_core::Session::doing`,
    /// roadmap 2.1): the command running or last run, its status, the
    /// directory and title. The row's second line and the tab's name are
    /// made from it; the face only words it.
    pub doing: Option<khor_core::Doing>,
    /// The network exit this session borrows (device hex), when it does
    /// (docs/NET.md 借网 via 记号). No producer yet — lands with `--via`.
    pub via: Option<String>,
    /// The exit machine's face, looked up like `face` — the 行尾右下
    /// mini avatar. `None` with `via` set draws a blank, not an invented
    /// face (Avatar.tsx's rule).
    pub via_face: Option<Avatar>,
    /// Which machine this session lives on, by the name people call it.
    ///
    /// **Not the same fact as `source`.** `source` is the offline axis:
    /// it is set only on rows another device reported, so the rows that
    /// live here carry none — which is right for "how old is this
    /// report" and wrong for "which machine is this on". Filtering by
    /// machine needs the second question answered for *every* row,
    /// including this machine's own, or the axis is missing exactly the
    /// machine people reach for first and reads as broken.
    ///
    /// The device table's name, falling back to the id's hex — the same
    /// pair `list::place_of` builds for the device arrangement, so one
    /// machine is named the same in a chip and in a group heading.
    ///
    /// Derived here rather than in the frontend for the reason `face`
    /// is: the id→name table is the node's, and a screen that resolved
    /// it itself would be a second copy that can disagree.
    pub home: String,
    /// Where this session lives (`khor_core::Session::place`) — the
    /// project root, absolute, exactly as the group key spells it.
    ///
    /// **Absolute and unshortened**, though every screen prints it
    /// shortened: this is the value the `at:` filter compares against
    /// the group key, and a display string in that slot would be a
    /// second spelling of one fact. It is also what makes a row findable
    /// by the tree it is in.
    ///
    /// `None` for a row with no place — a conversation with another
    /// machine, or a session nobody recorded a directory for. The two
    /// are not distinguished here for the same reason the node does not
    /// distinguish them: the next move is the same.
    pub place: Option<String>,
    /// The session's process sits in a tmux session on its home
    /// machine (`khor_core::Session::mux`). **What the detail routes its
    /// default face on**, ahead of the kind: a conversation khor hosts
    /// that the person has also resumed in their own tmux is one `gui`
    /// row, and the terminal is where they are.
    pub mux: bool,
    /// **How a dead row ended** (`khor_node::live::Ending`, P5-SHELL item
    /// 3): the second line of a 失败 row names it — 「退出码 1」 or
    /// 「宿主被结束」. Read where the session lives, so only this
    /// machine's rows carry it; a reported row's is `None` and its line
    /// stays as it was.
    pub ended: Option<RowEnding>,
}

/// [`SessionRow::ended`].
#[derive(Debug, Clone, Serialize, TS)]
pub struct RowEnding {
    /// The exit its host recorded. `None`: the host was killed before
    /// its child ended, so nothing was recorded (宿主被结束).
    pub exit: Option<i32>,
}

impl From<khor_node::live::Ending> for RowEnding {
    fn from(e: khor_node::live::Ending) -> Self {
        match e {
            khor_node::live::Ending::Exit(code) => RowEnding { exit: Some(code) },
            khor_node::live::Ending::HostGone => RowEnding { exit: None },
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct SourceTag {
    pub device: String,
    #[ts(type = "number")]
    pub age_ms: u64,
}

/// A machine's watched directories and how old that answer is — the
/// two axes `VitalsReading` keeps apart, for the rows of `Request::Watched`.
#[derive(Debug, Clone, Serialize, TS)]
pub struct WatchedReading {
    pub rows: Vec<khor_core::WatchedDir>,
    #[ts(type = "number")]
    pub age_ms: u64,
}

/// The relay screen's whole answer: what this machine uses, and which
/// part of it the user put there.
///
/// **Two lists rather than one, and the split is the point.** `in_use`
/// is what the machine binds with — the built-in tier, `KHOR_RELAY`, and
/// the user's own, merged. `mine` is the only part a control can remove.
/// A screen given just `in_use` shows rows whose delete button has to be
/// greyed out with no explanation; a screen given just `mine` is empty
/// on a fresh machine and says nothing about the relay the connection is
/// actually standing on.
#[derive(Debug, Clone, Serialize, TS)]
pub struct RelayChoices {
    pub mine: Vec<String>,
    /// Which of `mine` were entered with a key. **Whether, never what**:
    /// the screen says a relay is keyed and never holds the key, which
    /// is why a write names the relays to keep and carries a key only
    /// for the one being added ([`RelayWant`]).
    pub keyed: Vec<String>,
    pub in_use: Vec<String>,
    /// Whether `mine` is being used **instead of** the Khor tier rather
    /// than on top of it. When this is on, `in_use` is `mine` (plus
    /// `KHOR_RELAY`) — which is the only way the screen can show that
    /// the switch did something.
    pub only_mine: bool,
    /// 首选中继: the relay this machine homes on when reachable, or
    /// `None` for the nearest (`Node::preferred_relay`).
    pub preferred: Option<String>,
}

/// 直连占比 on the 网络 page (docs/CONNECT.md 度量与验收): of the dials
/// the resident made lately that settled on a road, how many went
/// direct — every far machine together.
///
/// **The two numbers, not the percentage**: the screen writes the share
/// and how many dials it is out of, and 「100%」 of one dial is not the
/// same reading as 100% of twenty.
#[derive(Debug, Clone, Serialize, TS)]
pub struct RoadShare {
    pub direct: u32,
    pub settled: u32,
    /// Why there is no reading, when there is none: `"no-serve"` (no
    /// resident here, so nothing has been dialling) or `"unanswered"`
    /// (a resident that did not answer the question). `None` when the
    /// resident answered — `settled` may still be 0 then, meaning it
    /// has not dialled anybody since it started.
    pub unread: Option<String>,
}

/// **This machine's connections**, for the 网络 page's 连接 group (round
/// NET-2): `khor_core::ConnectionsView` with names on the machines.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Connections {
    pub home_relay: Option<String>,
    pub home_connected: bool,
    pub home_error: Option<String>,
    /// 首选中继, or `None` for the nearest.
    pub preferred_relay: Option<String>,
    pub relays: Vec<RelayRow>,
    pub peers: Vec<PeerRow>,
    /// Why there is nothing to show: `"no-serve"` or `"unanswered"`.
    pub unread: Option<String>,
}

/// One relay and its measured round trip.
#[derive(Debug, Clone, Serialize, TS)]
pub struct RelayRow {
    pub url: String,
    #[ts(type = "number | null")]
    pub rtt_ms: Option<u64>,
}

/// One machine's connection, named.
#[derive(Debug, Clone, Serialize, TS)]
pub struct PeerRow {
    pub id: String,
    pub shown: String,
    /// `khor_core::Hop`'s key.
    pub hop: String,
    #[ts(type = "number | null")]
    pub heard_ms: Option<u64>,
    #[ts(type = "number | null")]
    pub rtt_ms: Option<u64>,
    pub last_failure: Option<String>,
    #[ts(type = "number | null")]
    pub failure_ago_ms: Option<u64>,
    #[ts(type = "number")]
    pub failed: u64,
    #[ts(type = "number")]
    pub unanswered: u64,
}

/// **Every machine's try at one address** (round ROAD-1, 全网谁够得着):
/// `khor_core::ReachView` with names on the machines and the key shown
/// as OpenSSH prints it.
#[derive(Debug, Clone, Serialize, TS)]
pub struct ReachReport {
    pub host: String,
    pub port: u16,
    /// This machine first, then the table's order; satellites left out.
    pub rows: Vec<ReachRow>,
    /// The entrance's device id: the lowest ms among the reached, this
    /// machine on a tie.
    pub entrance: Option<String>,
    /// Two machines' known_hosts hold different keys for the name: it
    /// means a different machine from each.
    pub disagree: bool,
    /// The shown name of the machine in the table whose id is that host
    /// key (a satellite), when one is.
    pub known_as: Option<String>,
}

/// One machine's try.
#[derive(Debug, Clone, Serialize, TS)]
pub struct ReachRow {
    pub id: String,
    pub shown: String,
    pub here: bool,
    pub outcome: khor_core::ReachOutcome,
    /// The TCP connect, whole ms.
    #[ts(type = "number | null")]
    pub ms: Option<u32>,
    /// The address its resolver gave and it tried.
    pub addr: Option<String>,
    /// The server's first line (`SSH-2.0-OpenSSH_9.6`).
    pub banner: Option<String>,
    /// `SHA256:<base64>`, as that machine's known_hosts holds it.
    pub host_key: Option<String>,
    /// Words for a failure, in this machine's language.
    pub why: Option<String>,
}

/// **The ProxyCommand through a khor machine** (round ROAD-1, 经由):
/// the ssh config line and the one-off ssh command, to show with a copy
/// button. khor never writes the user's ssh config.
#[derive(Debug, Clone, Serialize, TS)]
pub struct ProxyLine {
    /// `ProxyCommand '<khor>' pipe --via '<machine>' %h %p`
    pub config_line: String,
    /// `ssh -o 'ProxyCommand=…' [-p port] '<host>'`
    pub command: String,
}

/// **One line of the connection journal** (`khor_node::journal`, NET-STABLE
/// 第一步): what this machine's resident saw on its way to another one,
/// in this language's words.
#[derive(Debug, Clone, Serialize, TS)]
pub struct JournalLine {
    #[ts(type = "number")]
    pub at: u64,
    /// `HH:MM:SS`, local.
    pub when: String,
    /// The far machine's name, or 本机 for this machine's own events.
    pub machine: String,
    pub text: String,
    /// Something going wrong: a failed dial, a wire retired, the home
    /// relay down.
    pub trouble: bool,
}

/// The journal of one machine by id, this machine's own events
/// interleaved, over the last `minutes`; read off the files, so it
/// answers with no resident.
pub fn journal(root: &Path, machine: &str, minutes: u64) -> Result<Vec<JournalLine>, String> {
    Ok(open(root)?
        .journal(Some(machine), minutes)
        .into_iter()
        .map(|l| JournalLine {
            at: l.at,
            when: l.when,
            machine: l.machine,
            text: l.text,
            trouble: l.trouble,
        })
        .collect())
}

/// 报告: every machine's last `minutes` frozen into one file under this
/// home (`Node::journal_report`); answers its path.
pub fn journal_report(root: &Path, minutes: u64) -> Result<String, String> {
    Ok(open(root)?.journal_report(minutes)?.display().to_string())
}

/// [`ProxyLine`] for `host:port` through `machine`; `port` `None` is 22.
pub fn proxy_line(machine: &str, host: &str, port: Option<u16>) -> ProxyLine {
    let (config_line, command) =
        khor_node::pipe::proxy_line(machine, host, khor_node::reach::port_or_ssh(port));
    ProxyLine { config_line, command }
}

/// Every machine's try at `host:port` (`Node::reach_probe`); `port`
/// `None` is ssh's 22. With no resident it binds an endpoint of its own
/// and still asks everyone; an error is a probe that could not run at
/// all (no address given, the resident's hand-off failed).
pub async fn reach_probe(root: &Path, host: String, port: Option<u16>) -> Result<ReachReport, String> {
    let at = root.to_path_buf();
    let n = tokio::task::spawn_blocking(move || open(&at))
        .await
        .map_err(|e| e.to_string())??;
    let port = khor_node::reach::port_or_ssh(port);
    let devices = n.devices().unwrap_or_default();
    let shown = |id: &str| {
        devices
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.shown().to_owned())
            .unwrap_or_else(|| id.chars().take(8).collect())
    };
    let view = n.reach_probe(&host, port).await?;
    let known_as = view
        .rows
        .iter()
        .filter_map(|(_, s)| s.host_key.as_deref())
        .find_map(|k| devices.iter().find(|d| d.id == k).map(|d| d.shown().to_owned()));
    Ok(ReachReport {
        rows: view
            .rows
            .iter()
            .map(|(id, s)| ReachRow {
                shown: shown(id),
                here: id == n.device_str(),
                id: id.clone(),
                outcome: s.outcome,
                ms: s.ms,
                addr: s.addr.clone(),
                banner: s.banner.clone(),
                host_key: s.host_key.as_deref().map(khor_node::reach::shown_key),
                why: s.why.clone(),
            })
            .collect(),
        host: view.host,
        port: view.port,
        entrance: view.entrance,
        disagree: view.disagree,
        known_as,
    })
}

/// A face is on screen, or stopped being (`Node::face_is_here`): the
/// resident's pump beats every 5 s while one is and every 30 s while none
/// is, and gives a face that arrives a round at once (round NET-6). With
/// no resident there is nobody to tell, and that is not an error.
pub async fn face_here(root: &Path, visible: bool) -> Result<(), String> {
    let at = root.to_path_buf();
    let n = tokio::task::spawn_blocking(move || open(&at))
        .await
        .map_err(|e| e.to_string())??;
    let _ = n.face_is_here(visible).await;
    Ok(())
}

/// A machine's page was opened (`Node::visit`): the resident has a round
/// with it now, past its backoff (NET-STABLE 第二步). With no resident
/// there is nobody to tell, and that is not an error.
pub async fn visit(root: &Path, machine: &str) -> Result<(), String> {
    let at = root.to_path_buf();
    let n = tokio::task::spawn_blocking(move || open(&at))
        .await
        .map_err(|e| e.to_string())??;
    let _ = n.visit(machine).await;
    Ok(())
}

/// **立即重试** (NOTICE-BRIEF): a session reconnecting to `machine` is
/// tried at once — every rejoin's wait cut short ([`rejoin::kick`]) and
/// the machine dialled past its backoff ([`visit`], a person's act).
pub async fn rejoin_now(root: &Path, machine: &str) -> Result<(), String> {
    rejoin::kick();
    visit(root, machine).await
}

/// The connections, read off the resident (`Node::connections`).
pub async fn connections(root: &Path) -> Result<Connections, String> {
    let at = root.to_path_buf();
    let n = tokio::task::spawn_blocking(move || open(&at))
        .await
        .map_err(|e| e.to_string())??;
    let names: std::collections::HashMap<String, String> = n
        .devices()
        .unwrap_or_default()
        .into_iter()
        .map(|d| (d.id.clone(), d.shown().to_owned()))
        .collect();
    Ok(match n.connections().await {
        Ok(v) => Connections {
            home_relay: v.home_relay,
            home_connected: v.home_connected,
            home_error: v.home_error,
            preferred_relay: v.preferred_relay,
            relays: v
                .relays
                .into_iter()
                .map(|r| RelayRow {
                    url: r.url,
                    rtt_ms: r.rtt_ms,
                })
                .collect(),
            peers: v
                .peers
                .into_iter()
                .map(|p| PeerRow {
                    shown: names.get(&p.id).cloned().unwrap_or_else(|| p.id.chars().take(8).collect()),
                    id: p.id,
                    hop: p.hop.key().to_owned(),
                    heard_ms: p.heard_ms,
                    rtt_ms: p.rtt_ms,
                    last_failure: p.last_failure,
                    failure_ago_ms: p.failure_ago_ms,
                    failed: p.failed,
                    unanswered: p.unanswered,
                })
                .collect(),
            unread: None,
        },
        Err(why) => Connections {
            home_relay: None,
            home_connected: false,
            home_error: None,
            preferred_relay: n.preferred_relay(),
            relays: Vec::new(),
            peers: Vec::new(),
            unread: Some(
                match why {
                    khor_node::link::RoadsUnread::NoServe => "no-serve",
                    khor_node::link::RoadsUnread::Unanswered => "unanswered",
                }
                .to_owned(),
            ),
        },
    })
}

/// 首选中继: the relay to home on when reachable, or `None` for the
/// nearest. Takes effect on the resident's next bind.
pub fn set_preferred_relay(root: &Path, url: Option<String>) -> Result<(), String> {
    open(root)?.set_preferred_relay(url.as_deref())
}

/// One relay a write wants kept or added.
///
/// `token` is the key typed for **this** relay, and only the relay
/// being added carries one: `None` keeps whatever key is already stored
/// for that url (`Node::set_relays`).
#[derive(Debug, Clone, serde::Deserialize, TS)]
pub struct RelayWant {
    pub url: String,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DeviceRow {
    pub id: String,
    /// **The channel, not the name on the glass.** What the network
    /// files this machine under: the segment in `chat/<name>`, and the
    /// key `seen` and `pins` use for that window. Settled once per
    /// store (`Node::open_as`) and never edited, so a rename cannot
    /// strand a directory or a watermark.
    ///
    /// Anything a person reads comes from [`DeviceRow::shown`]. The
    /// split is deliberate and the failure directions are not
    /// symmetric: a screen that reaches for `name` by mistake shows the
    /// name khor used before renaming existed, while a key built from
    /// `shown` would move the moment somebody renamed the machine.
    pub name: String,
    /// The name to paint — the rename when there is one, else `name`.
    /// Resolved here rather than on the glass, so four screens cannot
    /// make four decisions (`DeviceInfo::shown`).
    pub shown: String,
    /// The rename itself, or `None` when nobody has renamed this
    /// machine. Only the editor needs it: it must open on what the
    /// person typed, and show an empty box on a machine wearing its own
    /// name rather than offering that name as something to edit.
    pub label: Option<String>,
    pub me: bool,
    /// Paused, **as this machine decided it**: on this machine's own row,
    /// `khor pause` (docs/MORE.md 暂停); on another machine's row, this
    /// machine has stopped talking to that one (SHELL.md 第五轮 第 3 条,
    /// `khor pause <机器>`). What another machine decided about its own
    /// talking is not a thing this one can know; what it sees is the
    /// road going quiet, which `hop` already says. Read off the files
    /// like `khor devices` reads them, so the rail, the card and the
    /// terminal are one answer.
    pub paused: bool,
    /// This machine's face, in **its own** reported palette. See
    /// `khor_node::Node::face_of`.
    pub face: Option<Avatar>,
    /// Pinned, network-wide. Same rule as `SessionRow::pinned`: a mark to
    /// paint, never an ordering to redo.
    pub pinned: bool,
    /// What that machine is doing, and how long ago it said so. `None`
    /// when it has never been reached — see [`VitalsReading`].
    pub vitals: Option<VitalsReading>,
    /// Which road this machine is using to reach that one right now
    /// (`khor_core::Hop`), as its key — the word is looked up at paint
    /// time like every other.
    ///
    /// **Rides the row rather than arriving on its own**, so the whole
    /// table describes one instant: asked per machine it would be one
    /// reading per round trip, and two rows would be two moments.
    ///
    /// `None` for this machine, and that is the type doing the work: a
    /// machine does not reach itself over the network, so there is no
    /// reading rather than an unreadable one. Carried as a word every
    /// face had to remember to hide, the first face that forgot would
    /// print 说不出 on the row for the computer it is running on.
    pub hop: Option<String>,
    /// How long since that machine was last heard on this one's wire to
    /// it, in ms (`khor_core::Link::heard_ms`) — liveness, on its own
    /// axis beside `hop`. A connected machine is heard every few seconds
    /// by keepalive alone, so this only says something once it stops;
    /// faces show it beside 未连接 as 「上次 N 分钟前」. `None` when it
    /// was never heard since the resident started, and for this machine.
    #[ts(type = "number | null")]
    pub heard_ms: Option<u64>,
    /// **The row's latency** (round NET-8, UX-15): the live wire's round
    /// trip in whole milliseconds, QUIC's own smoothed estimate
    /// (`khor_core::Link::rtt_ms`). `None` without a live wire — the row
    /// then says 无法获取 beside `heard_ms` — and for this machine, whose
    /// word is 本机.
    #[ts(type = "number | null")]
    pub rtt_ms: Option<u32>,
    /// **How long ago the attempt that failed was, while that machine is
    /// unreachable** (round NET-13, `khor_core::Link::down_ms`). `hop`
    /// `quiet` alone is idle — no live wire, nothing known to be wrong,
    /// and the next open dials it fine; only with this set is the
    /// machine unreachable, the row dimmed and 「无法获取」 said. `None`
    /// otherwise, from an older resident, and for this machine.
    #[ts(type = "number | null")]
    pub down_ms: Option<u64>,
    /// The last round trip measured to that machine, kept after its wire
    /// goes (`khor_core::Link::last_rtt_ms`): what an idle machine is
    /// drawn by, at half ink. `None` when never measured since the
    /// resident started, and for this machine.
    #[ts(type = "number | null")]
    pub last_rtt_ms: Option<u32>,
    /// **How long since the road `last_hop` says settled**, in ms (UX-48,
    /// `khor_core::Link::since_ms`): the row's bottom-right under the
    /// road word — the live wire's road, or with none live the road it last
    /// used. `None` when no wire to it ever settled since the resident
    /// started, and for this machine.
    #[ts(type = "number | null")]
    pub since_ms: Option<u64>,
    /// **The road the last settled wire was on**, live or gone, as its
    /// `[hop]` key (`direct` / `relay`, `khor_core::Link::last_hop`): what
    /// an idle machine's row says in place of "offline" (UX-48). `None` when no
    /// wire to it ever settled, from an older resident, and for this
    /// machine.
    pub last_hop: Option<String>,
    /// **Which relay, when `hop` is 中继** (round NET-8 addendum, UX-16's
    /// map): its host, and port when not the scheme's own
    /// (`relay.khor.cloud`, `101.200.80.77:3340`). `None` on a direct
    /// wire, without a live one, and for this machine.
    pub relay: Option<String>,
    /// **The machine in the middle, by name, when `hop` is `forwarded`**
    /// (the peer hop, NET-DIRECT; `khor_core::Link::via`): the row says
    /// 「通过 <via>」 and the machine page draws its two legs. `None`
    /// otherwise, from an older resident, and for this machine.
    pub via: Option<String>,
    /// The machine this satellite speaks through, by name (附属机器批)
    /// — `None` for every ordinary machine. Where `hop` answers "which
    /// road", this answers "whose road": a satellite has none of its
    /// own, so faces print 「通过 <sponsor>」 in the road's place.
    pub sponsor: Option<String>,
    /// An SSH 主机's jump host, as its row keeps it (`ssh_jump`, round
    /// join-a): the machine page draws it as the road's middle stop.
    /// `None` for a direct ssh road and for every khor 主机.
    pub ssh_jump: Option<String>,
    /// The directories this machine is asked to keep an eye on — the
    /// setting, as kept on its row in the device table (2026-09-06
    /// 关注目录). What the editor on the card starts from.
    pub watch: Vec<String>,
    /// What the machine last said about them, with its own age — a
    /// record apart from `vitals` because the two requests fail apart
    /// (`Node::cache_peer_watched`). `None` when it never answered: an
    /// older khor refuses the op, and no rows is not the same fact as
    /// no directories.
    pub watched: Option<WatchedReading>,
    /// Where files sent to it should land, as set on its row — `None`
    /// for the default (STORE-BRIEF phase 1). What 存储's location row
    /// shows and changes.
    pub receive_dir: Option<String>,
    /// Where they land now, absolute, as the machine itself resolved the
    /// setting — `None` from a khor that predates it. What 存储 lists.
    pub receive_at: Option<String>,
    /// The setting `receive_at` answers, as the machine last resolved it
    /// in full — `None` for the default. A `receive_dir` that differs is
    /// one the machine has not seen yet, and 存储 says so.
    pub receive_for: Option<String>,
    /// Why the setting fell back to the default, in the machine's own
    /// sentence; `None` when it took.
    pub receive_why: Option<String>,
    /// Where this machine sits when the network is drawn as a mandala
    /// (`khor_core::map`).
    ///
    /// **Derived here rather than by whoever paints it**, the same rule
    /// the face follows: a picture two ends work out separately is two
    /// pictures. It rides the row because a seat *is* a property of the
    /// placed machine — and because the seat of one depends on the whole
    /// set, which is a thing only the code building the whole set knows.
    pub seat: Seat,
    /// Whether this machine can host a session — `false` for a phone
    /// (docs/IOS.md 第二步). **The one fact every entry that needs a
    /// hosting machine reads**: new session there, VS Code there, the
    /// process table, the hooks; and on this machine's own row, the acts
    /// that need a computer under the face (a second window, an opener,
    /// a downloads folder). Said by the machine itself in the device
    /// table (`khor_sync::devices`' `hosts`), so it is the same answer on
    /// every machine looking; a machine that never said is a computer.
    pub hosts: bool,
    /// **Every road this machine reports** (round NET-12, the 我的 khor
    /// dial and its matrix): to whom, direct / by which relay / none, and
    /// the round trip. On this machine's own row they are its live wires,
    /// first-hand, as `rtt_ms` is; on another's, what that machine last
    /// wrote into the table — which moves only when a road changed kind
    /// and held, or its round trip moved by a quarter (`roadbook`). Empty
    /// for an older peer, which writes none: the face says 未知.
    pub roads: Vec<khor_node::Road>,
}

/// A machine's reading plus its age — the two axes docs/SESSION.md keeps
/// apart, here for a machine instead of a session.
///
/// **The age is not decoration and it is not optional.** A number with no
/// age is read as the present, so a machine that has been off since
/// yesterday would show yesterday's CPU as if it were now. This
/// machine's reading has an age too since 2026-09-25 (docs/SHELL.md
/// 第十六轮): it is the background sampler's last round, a few seconds
/// old at most, and no longer taken to answer the call.
///
/// The row carries `None` when the machine has never answered at all.
/// That is a third state on purpose: "not asked yet" and "asked an hour
/// ago" send a person to look at different things.
#[derive(Debug, Clone, Serialize, TS)]
pub struct VitalsReading {
    pub vitals: Vitals,
    #[ts(type = "number")]
    pub age_ms: u64,
    /// The strain words for memory, disk and video memory, minted here
    /// so the screen paints a judgment instead of making one (the
    /// thresholds and their reasons live on `khor_core::Fill::strain`).
    /// CPU and GPU utilisation carry no
    /// word on purpose: a pinned core is a machine doing its job, a full
    /// disk is a machine about to stop doing it.
    pub mem_strain: Option<khor_core::Strain>,
    pub disk_strain: Option<khor_core::Strain>,
    /// Video memory is a `Fill` like the other two — it has a "full" to
    /// be near — and it was the one the gauges whispered about while it
    /// screamed (仪表环出声批's causing case: a 90% VRAM ring wearing the
    /// same grey as a 2% CPU). Minted from the same `Fill::strain`, so
    /// the thresholds stay one fact in one place.
    pub vram_strain: Option<khor_core::Strain>,
    /// One word per volume, index-parallel with `vitals.volumes` —
    /// minted here like the three above, from the same `Fill::strain`,
    /// so the per-volume rows the card grew on 2026-09-02 paint the
    /// same judgment the single disk ring always did.
    pub volume_strains: Vec<Option<khor_core::Strain>>,
    /// Mount points whose numbers did not come in time this round — the
    /// row keeps its last numbers, or has none yet and is not drawn.
    /// Only ever filled for this machine (`khor_node::vitals::local`);
    /// a peer's reading crosses the wire without it.
    pub stale: Vec<String>,
}

impl VitalsReading {
    fn of(vitals: Vitals, age_ms: u64) -> VitalsReading {
        VitalsReading {
            stale: Vec::new(),
            mem_strain: vitals.mem.strain(),
            disk_strain: vitals.disk.and_then(|d| d.strain()),
            vram_strain: vitals
                .gpu
                .as_ref()
                .and_then(|g| g.mem)
                .and_then(|m| m.strain()),
            volume_strains: vitals.volumes.iter().map(|v| v.fill.strain()).collect(),
            vitals,
            age_ms,
        }
    }
}

#[cfg(test)]
mod vitals_reading_tests {
    use super::*;
    use khor_core::{Fill, Strain};

    /// Memory strained, disk fine, VRAM tight — pairwise asymmetric on
    /// purpose. A symmetric fixture would stay green with two
    /// assignments swapped, and a swap is the one mistake this
    /// constructor has room for. The words differ too (Critical vs
    /// Tight), so mem↔vram swapped also goes red.
    #[test]
    fn each_strain_word_comes_from_its_own_fill() {
        let v = Vitals {
            cpu_pct: 99.0, // pinned CPU carries no word, also on purpose
            cores: 8,
            mem: Fill {
                used: 96,
                total: 100,
            },
            disk: Some(Fill {
                used: 50,
                total: 100,
            }),
            gpu: Some(khor_core::Gpu {
                util_pct: 99.0, // pinned GPU carries no word either
                cards: 1,
                mem: Some(Fill {
                    used: 91,
                    total: 100,
                }),
            }),
            version: None,
            gpus: vec![],
            volumes: vec![
                khor_core::Volume {
                    mount: "/".to_owned(),
                    fill: Fill {
                        used: 96,
                        total: 100,
                    },
                },
                khor_core::Volume {
                    mount: "/data".to_owned(),
                    fill: Fill {
                        used: 50,
                        total: 100,
                    },
                },
            ],
            uptime_s: 0,
            model: None,
            system: None,
        };
        let r = VitalsReading::of(v, 0);
        assert_eq!(r.mem_strain, Some(Strain::Critical));
        assert_eq!(r.disk_strain, None, "a half-full disk says nothing");
        assert_eq!(
            r.vram_strain,
            Some(Strain::Tight),
            "a 91% VRAM is worth a glance"
        );
        assert_eq!(
            r.volume_strains,
            vec![Some(Strain::Critical), None],
            "each volume wears its own word, in its own seat"
        );
    }

    /// No GPU, no vram word — the absent card must not read as anything.
    #[test]
    fn a_machine_without_a_gpu_says_nothing_about_vram() {
        let v = Vitals {
            cpu_pct: 1.0,
            cores: 8,
            mem: Fill {
                used: 10,
                total: 100,
            },
            disk: None,
            gpu: None,
            version: None,
            gpus: vec![],
            volumes: vec![],
            uptime_s: 0,
            model: None,
            system: None,
        };
        let r = VitalsReading::of(v, 0);
        assert_eq!(r.vram_strain, None);
        assert!(r.volume_strains.is_empty(), "no volumes, no words");
    }
}

/// One option on the variant or shape axis, already painted.
///
/// The face is **this machine's real face under that option**, derived by
/// `khor_node::Node::face_under` — the same function and the same seed
/// every row's face goes through. Letting the frontend build previews
/// would put a second painter in the one place a second painter costs
/// most: a preview is all the evidence anybody has before pressing.
#[derive(Debug, Clone, Serialize, TS)]
pub struct FaceChoice {
    /// The stable key (`Variant::key`, `FaceShape::key`). The word beside
    /// it is looked up in the catalog at paint time, never carried here.
    pub key: String,
    pub face: Avatar,
}

/// A factory palette, painted, and the five slots it would set.
///
/// It carries its colors because **a palette only ever reaches the
/// library as five colors** (`Node::restyle`). Pressing a factory button
/// and dragging one well then travel the same path, so there is no
/// second way for a palette to arrive that could behave differently.
#[derive(Debug, Clone, Serialize, TS)]
pub struct PaletteChoice {
    /// The palette's `id` (`khor_core::avatar::PRESETS`).
    pub key: String,
    pub colors: Vec<String>,
    pub face: Avatar,
}

/// What this machine is wearing, and every option painted as it would
/// look on it.
///
/// **One call, not one per option**, and each option's face derived with
/// the *other two axes left where they are* — so a swatch is literally
/// "what you get if you press this" rather than a face assembled out of
/// factory defaults that nobody would ever see.
#[derive(Debug, Clone, Serialize, TS)]
pub struct FaceChoices {
    /// The face as it stands — the same picture the rail and this
    /// machine's own device row are painting.
    pub face: Avatar,
    /// The five slots as they stand, `#rrggbb`.
    pub colors: Vec<String>,
    /// Which factory palette those five are, when they are one. `None`
    /// once a slot has been changed by hand, and then **no** palette is
    /// marked: a nearest match would light a button nobody pressed, and
    /// pressing that lit button would then look like a control that does
    /// nothing (docs/UX.md 做了但没变化).
    pub preset: Option<String>,
    pub variant: String,
    pub shape: String,
    pub palettes: Vec<PaletteChoice>,
    pub variants: Vec<FaceChoice>,
    pub shapes: Vec<FaceChoice>,
}

fn open(root: &Path) -> Result<Node, String> {
    Node::open(root.to_path_buf())
}

/// Everything the editor paints, **for whichever machine is open in
/// it**. The order of each axis is the library's (`PRESETS` is ordered
/// by provenance, `ALL` by the order a chooser should offer them); this
/// layer picks none of it.
///
/// `machine` is a device id, and **empty means this one** — the shape
/// `files::launch` already uses for 「which machine」 in this crate. Every
/// swatch is painted under *that* machine's seed (`face_under_for`),
/// because a preview drawn with this machine's seed would show somebody
/// a set of faces none of which is the face they are about to give.
pub fn face_choices(root: &Path, machine: &str) -> Result<FaceChoices, String> {
    let n = open(root)?;
    let device = if machine.is_empty() {
        n.device_str().to_owned()
    } else {
        machine.to_owned()
    };
    let now = n.avatar_style_of(&device);
    // An id with no seed has no face to paint, and a chooser drawing the
    // factory default for it would be inviting somebody to restyle a
    // machine khor cannot render.
    let paint = |st: &AvatarStyle| {
        n.face_under_for(&device, st)
            .ok_or_else(|| khor_catalog::msg::no_such_machine(machine, String::new()))
    };
    Ok(FaceChoices {
        face: paint(&now)?,
        colors: now.palette.colors().to_vec(),
        preset: now.palette.preset_id().map(str::to_owned),
        variant: now.variant.key().to_owned(),
        shape: now.shape.key().to_owned(),
        palettes: PRESETS
            .iter()
            .map(|p| {
                Ok(PaletteChoice {
                    key: p.id.to_owned(),
                    colors: p.colors.iter().map(|c| c.to_string()).collect(),
                    face: paint(&AvatarStyle {
                        palette: p.palette(),
                        ..now.clone()
                    })?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        variants: Variant::ALL
            .iter()
            .map(|v| {
                Ok(FaceChoice {
                    key: v.key().to_owned(),
                    face: paint(&AvatarStyle {
                        variant: *v,
                        ..now.clone()
                    })?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        shapes: FaceShape::ALL
            .iter()
            .map(|sh| {
                Ok(FaceChoice {
                    key: sh.key().to_owned(),
                    face: paint(&AvatarStyle {
                        shape: *sh,
                        ..now.clone()
                    })?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

/// Changes what this machine wears — the call `khor face` makes. One
/// function behind the verb and behind every control, so the two faces
/// cannot come to mean different things by the same words.
/// `machine` is a device id and **empty is this one**, `face_choices`'
/// rule. The node resolves and refuses; nothing here decides anything.
pub fn restyle(
    root: &Path,
    machine: &str,
    colors: Option<Vec<String>>,
    variant: Option<String>,
    shape: Option<String>,
) -> Result<(), String> {
    let n = open(root)?;
    let device = if machine.is_empty() {
        n.device_str().to_owned()
    } else {
        machine.to_owned()
    };
    // The same call the CLI makes, so an axis left out stays where it is
    // and a value outside its ladder is refused by name rather than
    // defaulted — one judgment, not one per face.
    n.restyle_device(
        &device,
        colors.as_deref(),
        variant.as_deref(),
        shape.as_deref(),
    )
    .map(|_| ())
}

/// The list, arranged. `by` is a `khor_node::list::Arrange` key; an
/// unknown one is refused by name rather than quietly falling back to
/// the default — a typo'd preference would otherwise look like the
/// setting silently not working.
pub fn list_sessions(root: &Path, by: &str) -> Result<Vec<SessionRow>, String> {
    let mode = Arrange::from_key(by).ok_or_else(|| {
        let all: Vec<&str> = Arrange::ALL.iter().map(|a| a.key()).collect();
        khor_catalog::msg::not_a_way_to_arrange(by, all.join(khor_catalog::cli::name_separator()))
    })?;
    let n = open(root)?;
    // ⚠️ **One reading of the device table, three things derived from
    // it.** Until 2026-09-11 this line was three separate reads —
    // `faces_by_id` called `Node::devices`, the name table below called
    // it, and `sessions_arranged` called it a third time inside — under
    // a comment claiming the table was 「built once here」. It was: once
    // per *place that wanted it*. On the 2 s beat that is three reads of
    // one doc, twelve times a minute, for one list.
    //
    // A face and a name are both pure functions of a row in this table,
    // so they are derived here rather than per session row: one machine
    // owns many rows.
    let devices = n.devices()?;
    let faces: HashMap<String, Avatar> = devices
        .iter()
        .filter_map(|d| n.face_of(d).map(|f| (d.id.clone(), f)))
        .collect();
    let machines: BTreeMap<String, String> = devices.into_iter().map(|d| (d.id, d.name)).collect();
    Ok(n.sessions_arranged_with(mode, &machines)?
        .into_iter()
        .map(|a| {
            let (v, group) = (a.view, a.group);
            let hosted = n.is_hosted(&v.session.id);
            // The row itself says whether its home can stand a terminal
            // up (`Session::term` — hosted, a discovered tmux session,
            // or an agent sitting inside one), and that answer **travels
            // with the row**: a session on another machine is reached,
            // not hosted here (`Node::reach` binds a local port onto its
            // host, and `term_open` connects to that like any address).
            //
            // A locality gate used to sit here, from when the terminal
            // could not travel. It outlived the reason for it, and what
            // it produced was a row that opened onto the bare facts pane
            // — which reads, correctly, as "clicking this does nothing".
            let attachable = hosted || v.session.term;
            // Only a dead row of this machine's is asked: its registry
            // dir is here.
            let ended = (v.session.dead && v.source.is_none())
                .then(|| n.ending(&v.session.id))
                .flatten()
                .map(RowEnding::from);
            SessionRow {
                ended,
                attachable,
                home: machines
                    .get(&v.session.home.hex())
                    .cloned()
                    .unwrap_or_else(|| v.session.home.hex()),
                // **A khor-wide row wears no machine's face** (the
                // broadcast, `khor_node::wide`): it is every machine's,
                // and the machine that derived it is only where it was
                // read. A blank draws, with the kind's mark on it.
                face: if v.session.id.0.starts_with(khor_node::wide::WIDE) {
                    None
                } else {
                    faces.get(&v.session.home.hex()).cloned()
                },
                hosted,
                via_face: v.session.via.as_ref().and_then(|d| faces.get(d).cloned()),
                last: v.session.last,
                doing: v.session.doing,
                via: v.session.via,
                id: v.session.id.0,
                kind: v.session.kind.0,
                title: v.session.title,
                word: v.session.state.state.key().to_owned(),
                at_ms: v.session.state.at.0,
                unread: v.session.unread,
                source: v
                    .source
                    .map(|(device, age_ms)| SourceTag { device, age_ms }),
                pinned: v.pinned,
                stale: v.stale,
                quiet: v.quiet,
                imported: v.imported,
                category: v.session.category,
                place: v.session.place,
                mux: v.session.mux,
                group,
            }
        })
        .collect())
}

/// One hit from the global search (`Node::find`, docs/SHELL.md 第十轮).
/// Renamed for TS and nothing more — the matching and the order are the
/// node's, so ⌘K and `khor find` cannot come to disagree.
#[derive(Debug, Clone, Serialize, TS)]
pub struct FindHit {
    /// `session` / `machine` / `workspace` / `tab` / `page`.
    pub kind: String,
    /// What the face jumps by: a session id, a device id,
    /// `<device>:<path>`, a tab id, or a page key.
    pub id: String,
    pub label: String,
    pub note: Option<String>,
}

/// The whole list for one word, in the node's order.
pub fn find(root: &Path, word: &str) -> Result<Vec<FindHit>, String> {
    Ok(open(root)?
        .find(word)?
        .into_iter()
        .map(|h| FindHit {
            kind: h.kind,
            id: h.id,
            label: h.label,
            note: h.note,
        })
        .collect())
}

/// One ring on the 额度 page: an account's reading, whose it is and how
/// old it is, with the strain decided by the node (`accounts::strain_of`)
/// so the face only paints.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AccountRing {
    pub reading: khor_core::AccountReading,
    /// How long ago the reading was carried here — 0 for this machine's
    /// own, which is read now.
    #[ts(type = "number")]
    pub age_ms: u64,
    /// The name of the machine holding the key (`reading.home` is its id).
    pub home_name: String,
    pub mine: bool,
    pub strain: Option<khor_core::Strain>,
    pub base_url: Option<String>,
    #[ts(type = "number | null")]
    pub budget: Option<f64>,
    /// The unit the person set on this account, or `None` where they
    /// have not — **for the edit form's value, not for display**.
    ///
    /// What a screen prints is `reading.currency`, which already has
    /// this applied (`Node::quota_board`, one place for both faces).
    /// The form needs the raw fact instead: an empty box there means
    /// 「use what the endpoint said」, and it may only be empty when
    /// that is actually true.
    pub unit: Option<String>,
    /// Floated to the top of the list, on every machine
    /// (`khor_sync::accounts::Account::pinned`). Carried here rather
    /// than read a second time by the face: the order is the doc's, and
    /// a screen sorting by its own copy of the rule is how two faces
    /// come to disagree about what is at the top.
    pub pinned: bool,
    /// `bill` or `sub` (`khor_sync::accounts::AccountKind`), the word
    /// the row's 类型 axis is made of.
    ///
    /// **From the registration, not from the reading**, and that is
    /// what makes it usable: the account document replicates, so this
    /// is known for every row on every machine — including a row whose
    /// home has never answered. Read off the reading instead (「it has
    /// windows, so it must be a subscription」) it would be *inferred*,
    /// and a subscription whose login expired has no windows: the row
    /// would quietly change kind at the moment it broke, which is the
    /// moment a person most needs it to say what it is.
    pub kind: String,
}

/// Every registered account's ring (`Node::quota_board`), home machines
/// named.
pub async fn quota_board(root: &Path) -> Result<Vec<AccountRing>, String> {
    let n = open(root)?;
    // This machine's own login joins the list as a registration the
    // first time it is found (`Node::adopt_subscriptions`). A failure
    // here is not the board's: the list is still the list.
    let _ = n.adopt_subscriptions();
    let me = n.device_str().to_owned();
    let names: std::collections::HashMap<String, String> =
        n.devices()?.into_iter().map(|d| (d.id, d.name)).collect();
    let accounts = n.accounts()?;
    Ok(n.quota_board()
        .await?
        .into_iter()
        .map(|(reading, age_ms)| {
            let account = accounts.iter().find(|a| a.id == reading.account);
            AccountRing {
                home_name: names
                    .get(&reading.home)
                    .cloned()
                    .unwrap_or_else(|| reading.home.clone()),
                mine: reading.home == me,
                strain: khor_node::accounts::strain_of(reading.fill),
                base_url: account.and_then(|a| a.base_url.clone()),
                budget: account.and_then(|a| a.budget),
                unit: account.and_then(|a| a.unit.clone()),
                pinned: account.is_some_and(|a| a.pinned),
                kind: account
                    .map(|a| a.kind.word().to_owned())
                    .unwrap_or_else(|| khor_node::AccountKind::Bill.word().to_owned()),
                age_ms,
                reading,
            }
        })
        .collect())
}

/// 登记 (`Node::register_account`): the provider and kind words are
/// parsed here so the face and the CLI refuse an unknown one with the
/// same sentence. Answers the new account's id.
///
/// **`kind` is a word rather than a bool** for the reason it is a word
/// everywhere else: 「订阅 / 计费」 is an axis that will hold a third
/// value before it holds none, and a `bool` named `sub` would have to
/// be replaced rather than extended — while spelling it out costs one
/// `parse` that already refuses by name.
pub fn register_account(
    root: &Path,
    name: &str,
    provider: &str,
    kind: &str,
    base_url: Option<&str>,
    key: &str,
    budget: Option<f64>,
) -> Result<String, String> {
    let provider = khor_node::Provider::parse(provider)
        .ok_or_else(|| khor_catalog::msg::no_such_provider(provider))?;
    let kind =
        khor_node::AccountKind::parse(kind).ok_or_else(|| khor_catalog::msg::no_such_kind(kind))?;
    Ok(open(root)?
        .register_account(name, provider, kind, base_url, key, budget)?
        .id)
}

pub fn remove_account(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.remove_account(id)
}

/// 置顶 / 取消置顶 (`Node::pin_account`). The flag rides the replicated
/// doc, so this pins on every machine.
pub fn pin_account(root: &Path, id: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_account(id, on)
}

/// 改名 (`Node::rename_account`): the list row's double-click, the
/// session row's gesture (2026-09-30). The name alone, network-wide.
pub fn rename_account(root: &Path, id: &str, name: &str) -> Result<(), String> {
    open(root)?.rename_account(id, name)
}

/// 编辑 (`Node::update_account`): name, address, budget, unit, and
/// optionally a new key. The provider is not editable — see the node's
/// own head.
///
/// `unit` absent and `unit` empty are the same thing on the way in —
/// 「use what the endpoint said」 — and the node is where that is
/// settled, so this layer does not second-guess it.
pub fn update_account(
    root: &Path,
    id: &str,
    name: &str,
    base_url: Option<&str>,
    key: Option<&str>,
    budget: Option<f64>,
    unit: Option<&str>,
) -> Result<(), String> {
    open(root)?.update_account(id, name, base_url, key, budget, unit)
}

/// What a machine is busy with — its process top and CPU day
/// (`khor_node::Node::load_on`; a name, or empty for this one). An
/// older khor over there is a refusal with its own sentence, which the
/// card prints where the table would be.
pub async fn load(
    root: &Path,
    machine: &str,
    top: u32,
    by: khor_core::TopBy,
    desc: bool,
) -> Result<khor_core::Load, String> {
    open(root)?.load_on(machine, top, by, desc).await
}

/// One cell of a day strip, in the shape the frontend can paint.
///
/// **Three cases and three TS shapes, on purpose.** The words are one
/// namespace and the two nothings are outside it: an externally tagged
/// enum lands in TypeScript as `{ word: string } | "unwatched" |
/// "absent"`, so `var(--state-${…})` cannot be reached for a nothing.
/// Spelled as one string in one namespace, `--state-unwatched` would
/// resolve to nothing and paint transparent — silently folding 不知道
/// into 没有这一行, which is the exact collapse `khor_core::DayCell`
/// exists to prevent, re-introduced in CSS.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DayMark {
    /// A state key (`busy`…), looked up like every other word.
    Word(String),
    Unwatched,
    Absent,
}

/// A stretch of identical cells: what the face paints as one block, as
/// wide as `cells` out of the total it asked for.
#[derive(Debug, Clone, Serialize, TS)]
pub struct DayRun {
    pub mark: DayMark,
    #[ts(type = "number")]
    pub cells: u32,
}

/// Each row's last 24 hours, keyed by session id
/// (`khor_node::day`).
///
/// **Its own call on its own beat, not a field on [`SessionRow`].** The
/// row list is polled every two seconds; a day of cells hung on it would
/// be a log riding a heartbeat. Nothing here judges: `cells` is a pixel
/// count the screen measured, and every merge down to it happened in the
/// node (`khor_core::DayCell::rank`).
pub fn day_strips(
    root: &Path,
    cells: u32,
) -> Result<std::collections::BTreeMap<String, Vec<DayRun>>, String> {
    let n = open(root)?;
    Ok(n.day_strips(cells as usize)?
        .into_iter()
        .map(|(id, runs)| {
            let runs = runs
                .into_iter()
                .map(|r| DayRun {
                    mark: match r.cell {
                        khor_core::DayCell::Word(word) => DayMark::Word(word.key().to_owned()),
                        khor_core::DayCell::Unwatched => DayMark::Unwatched,
                        khor_core::DayCell::Absent => DayMark::Absent,
                    },
                    cells: r.cells,
                })
                .collect();
            (id, runs)
        })
        .collect())
}

/// Async only because of the road reading: it is the resident's answer,
/// fetched once for the whole table (`khor_node::Node::hops`).
///
/// **Nothing in here waits for a disk** (docs/SHELL.md 第十六轮): this
/// machine's reading and watched rows are the background sampler's last
/// round, and the file reads (the store, the caches) go on a blocking
/// thread rather than the async one the app polls this on every two
/// seconds.
pub async fn list_devices(root: &Path) -> Result<Vec<DeviceRow>, String> {
    let at = root.to_path_buf();
    let n = tokio::task::spawn_blocking(move || open(&at))
        .await
        .map_err(|e| e.to_string())??;
    // A resident that did not answer in time keeps its last reading
    // (`Node::links`); only with no reading at all is the road left
    // unsaid on every row — never a word for "the question failed".
    let links = n.links().await.unwrap_or_default();
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || rows_of_devices(&root, &n, links))
        .await
        .map_err(|e| e.to_string())?
}

fn rows_of_devices(
    root: &Path,
    n: &Node,
    links: Vec<(String, khor_node::Link)>,
) -> Result<Vec<DeviceRow>, String> {
    let me = n.device_str().to_owned();
    let all = n.devices()?;
    let mine = n.local_vitals();
    // The ring is as big as the network minus this machine, and the
    // seats are handed out in the order the node listed them — so a
    // machine keeps its seat for as long as the list keeps its shape,
    // and nothing here decides an order the node did not.
    let mut ring = map::ring(all.iter().filter(|d| d.id != me).count()).into_iter();
    // id → name for the satellite rows: their road column names the
    // sponsor they speak through, and a screen prints names.
    let names: std::collections::HashMap<String, String> =
        all.iter().map(|d| (d.id.clone(), d.name.clone())).collect();
    Ok(all
        .into_iter()
        .map(|d| DeviceRow {
            ssh_jump: d.ssh_jump.clone(),
            sponsor: d
                .sponsor
                .as_ref()
                .map(|sp| names.get(sp).cloned().unwrap_or_else(|| sp.clone())),
            seat: if d.id == me {
                map::CENTRE
            } else {
                // One seat per machine on the ring, in step with the
                // count above: `ring` was built from exactly this many.
                ring.next().unwrap_or(map::CENTRE)
            },
            me: d.id == me,
            paused: if d.id == me {
                khor_node::pause::is_on(root)
            } else {
                khor_node::pause::peer_on(root, &d.id)
            },
            hop: (d.id != me)
                .then(|| links.iter().find(|(id, _)| *id == d.id))
                .flatten()
                .map(|(_, l)| l.hop.key().to_owned()),
            heard_ms: (d.id != me)
                .then(|| links.iter().find(|(id, _)| *id == d.id))
                .flatten()
                .and_then(|(_, l)| l.heard_ms),
            rtt_ms: (d.id != me)
                .then(|| links.iter().find(|(id, _)| *id == d.id))
                .flatten()
                .and_then(|(_, l)| l.rtt_ms),
            relay: (d.id != me)
                .then(|| links.iter().find(|(id, _)| *id == d.id))
                .flatten()
                .and_then(|(_, l)| l.relay.clone()),
            via: (d.id != me)
                .then(|| links.iter().find(|(id, _)| *id == d.id))
                .flatten()
                .and_then(|(_, l)| l.via.as_ref())
                .map(|v| names.get(v).cloned().unwrap_or_else(|| v.clone())),
            down_ms: (d.id != me)
                .then(|| links.iter().find(|(id, _)| *id == d.id))
                .flatten()
                .and_then(|(_, l)| l.down_ms),
            last_rtt_ms: (d.id != me)
                .then(|| links.iter().find(|(id, _)| *id == d.id))
                .flatten()
                .and_then(|(_, l)| l.last_rtt_ms),
            since_ms: (d.id != me)
                .then(|| links.iter().find(|(id, _)| *id == d.id))
                .flatten()
                .and_then(|(_, l)| l.since_ms),
            last_hop: (d.id != me)
                .then(|| links.iter().find(|(id, _)| *id == d.id))
                .flatten()
                .and_then(|(_, l)| l.last_hop)
                .map(|h| h.key().to_owned()),
            // This machine's from its live wires, first-hand, the same
            // readings `rtt_ms` and `relay` are; another machine's as it
            // wrote them into the table (round NET-12).
            // Satellites through their sponsors (round ROAD-1): this
            // machine's ssh to its own read live, the rest composed.
            roads: if d.id == me {
                let mut own = own_roads(&links);
                own.extend(n.my_satellite_roads());
                n.with_satellite_roads(&d.id, own)
            } else {
                n.with_satellite_roads(&d.id, n.roads_of(&d))
            },
            face: n.face_of(&d),
            pinned: d.pinned,
            // The background sampler's last round for this machine,
            // the last visit for every other; `me` says which.
            vitals: if d.id == me {
                mine.clone().map(|r| {
                    let mut reading = VitalsReading::of(r.vitals, r.age_ms);
                    reading.stale = r.stale;
                    reading
                })
            } else {
                n.vitals_of(&d.id)
                    .map(|(vitals, age_ms)| VitalsReading::of(vitals, age_ms))
            },
            watched: n
                .watched_of(&d.id)
                .map(|(rows, age_ms)| WatchedReading { rows, age_ms }),
            shown: d.shown().to_owned(),
            // This machine's own answer comes from the node that is
            // asking — the same fact it wrote to the table at open,
            // without waiting for the write to be read back.
            hosts: if d.id == me { n.hosts() } else { d.hosts },
            watch: d.watch,
            receive_dir: d.receive_dir,
            receive_at: d.receive_at,
            receive_for: d.receive_for,
            receive_why: d.receive_why,
            label: d.label,
            id: d.id,
            name: d.name,
        })
        .collect())
}

/// This machine's roads, from the live wires: what the face draws for
/// its own edges, never this machine's own row read back from the table
/// (round NET-12, `khor_node::roadbook`).
fn own_roads(links: &[(String, khor_node::Link)]) -> Vec<khor_node::Road> {
    khor_node::roadbook::own_roads(links)
}

/// What every machine in the network has spent, by day and by vendor —
/// the call `khor usage` makes, with no window applied.
///
/// **No window, unlike the verb.** A terminal prints once and scrolls
/// away, so `khor usage` takes a number of days; a screen scrolls, so the
/// answer arrives whole and how far back somebody looks is how far they
/// scroll. Same node call either way; what differs is what a face can do
/// with it.
pub fn usage(root: &Path) -> Result<Usage, String> {
    open(root)?.usage_everywhere()
}

/// What this machine's Claude subscription has left.
///
/// **Never fails.** Every way this can go wrong is something the person
/// might act on — no login, an expired token, a cooldown with a stated
/// wait — so a reason travels as data beside the reading rather than as
/// an error that a screen would have to turn back into words. A panel
/// that just stayed empty during a ten-minute cooldown would be
/// indistinguishable from one that is broken.
///
/// **No `root` argument, unlike its neighbours here.** This is not a
/// fact about a khor installation; it is a fact about an *account*, read
/// through the login Claude Code already stored on this machine. The
/// same account seen from two machines is one quota, not two.
pub async fn quota() -> QuotaAnswer {
    match khor_node::quota::read().await {
        Ok(quota) => QuotaAnswer {
            quota: Some(quota),
            trouble: None,
        },
        Err(trouble) => QuotaAnswer {
            quota: None,
            trouble: Some(trouble),
        },
    }
}

/// Pins or unpins a session — the call `khor pin <session>` makes. One
/// function behind the verb and the button, so the two cannot drift.
pub fn pin_session(root: &Path, id: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_session(&SessionId(id.to_owned()), on)
}

/// Pins or unpins a machine — the call `khor pin -m <machine>` makes.
pub fn pin_device(root: &Path, machine: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_device(machine, on)
}

/// Sets a machine's watch list — the call `khor watch <machine> …`
/// makes, from the card's editor. The refusals (a relative path, a
/// `..`, a duplicate) are the node's and come back as its sentence;
/// answers the list as kept, which the editor repaints from.
pub fn set_watch(root: &Path, machine: &str, paths: &[String]) -> Result<Vec<String>, String> {
    open(root)?.set_watch(machine, paths)
}

/// Sets where files sent to a machine land (存储's location row); empty
/// takes it back to the default. The refusal is the node's sentence;
/// answers the setting as kept.
pub fn set_receive_dir(root: &Path, machine: &str, dir: &str) -> Result<String, String> {
    open(root)?.set_receive_dir(machine, dir)
}

/// Shows a machine the door — the call `khor forget <machine>` makes,
/// from the machine row's menu. The judgment is `Node::forget_device`'s
/// (a network-wide tombstone, and this machine refusing to forget
/// itself); this answers the name of the machine that left.
pub fn forget_device(root: &Path, machine: &str) -> Result<String, String> {
    open(root)?.forget_device(machine)
}

// Every known machine's face, keyed the way the device table keys
// machines. The judgment behind each one — its own reported palette,
// seeded by its id — belongs to the node; this only indexes them.
// A plain comment, not a doc comment: it belongs to the tombstone
// below, and as `///` it would document the next function instead.
/* ⚠️ **Tombstone: `faces_by_id` is gone, 2026-09-11.** It read the
device table to derive a face per machine, which is still what
happens — inline in `list_sessions`, its only caller, beside the two
other things derived from that same reading. As its own function it
held a hidden `Node::devices` call, and a caller who already had the
table had no way to say so. A helper that re-reads what the caller
just read is a helper that costs more than it saves. */

pub fn seen(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.seen(&SessionId(id.to_owned()))
}

/// Names a session, for everyone; an empty title takes the name back so
/// the row falls back to its derived title (`Node::rename_session`).
/// One function behind the double-click and any future verb, so the two
/// cannot drift.
pub fn rename_session(root: &Path, id: &str, title: &str) -> Result<(), String> {
    open(root)?.rename_session(&SessionId(id.to_owned()), title)
}

/// Renames a machine for everyone; an empty name takes the rename back
/// so the row wears the name the machine reports about itself again.
/// Answers the name the row now shows.
///
/// One function behind the double-click and any future verb — the same
/// rule `rename_session` keeps, and the reason the two gestures cannot
/// come to mean different things.
pub fn rename_machine(root: &Path, id: &str, name: &str) -> Result<String, String> {
    open(root)?.rename_machine(id, name)
}

/// This machine's relays, both halves (see [`RelayChoices`]).
pub fn relays(root: &Path) -> Result<RelayChoices, String> {
    let node = open(root)?;
    Ok(RelayChoices {
        mine: node.relay_choices(),
        keyed: node.relays_keyed(),
        in_use: node.relays().to_vec(),
        only_mine: node.relays_only_mine(),
        preferred: node.preferred_relay(),
    })
}

/// 直连占比, from the resident's own memory of its dials
/// (`Node::road_tallies`).
pub async fn road_share(root: &Path) -> Result<RoadShare, String> {
    let at = root.to_path_buf();
    let n = tokio::task::spawn_blocking(move || open(&at))
        .await
        .map_err(|e| e.to_string())??;
    Ok(match n.road_tallies().await {
        Ok(tallies) => RoadShare {
            direct: tallies.iter().map(|(_, t)| t.direct).sum(),
            settled: tallies.iter().map(|(_, t)| t.direct + t.relay).sum(),
            unread: None,
        },
        Err(why) => RoadShare {
            direct: 0,
            settled: 0,
            unread: Some(
                match why {
                    khor_node::link::RoadsUnread::NoServe => "no-serve",
                    khor_node::link::RoadsUnread::Unanswered => "unanswered",
                }
                .to_owned(),
            ),
        },
    })
}

/// Replaces the user's own relays and answers with both halves again,
/// so the screen paints what the node says rather than what it just
/// sent — the same rule the face settings screen runs on.
///
/// ⚠️ `in_use` here is what the **next** node will bind with. A running
/// `khor serve` and every connection already standing keep the list they
/// opened with; the screen has to say so, because nothing else will.
pub fn set_relays(
    root: &Path,
    wants: &[RelayWant],
    only_mine: bool,
) -> Result<RelayChoices, String> {
    let node = open(root)?;
    let entries: Vec<khor_node::Relay> = wants
        .iter()
        .map(|w| khor_node::Relay {
            url: w.url.clone(),
            token: w.token.clone(),
            ..khor_node::Relay::default()
        })
        .collect();
    let in_use = node.set_relays(&entries, only_mine)?;
    Ok(RelayChoices {
        mine: node.relay_choices(),
        keyed: node.relays_keyed(),
        in_use,
        // Read back rather than echoed: the node is allowed to refuse
        // the combination, and a screen painting what it sent would show
        // a switch that is on while the file says off.
        only_mine: node.relays_only_mine(),
        preferred: node.preferred_relay(),
    })
}

pub fn close_session(root: &Path, id: &str) -> Result<(), String> {
    // Routed: a row on another machine closes where it runs
    // (`Node::close_anywhere`). The app's own runtime is the caller's,
    // so this one is built here the way every blocking door in this
    // file is.
    let n = open(root)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?
        .block_on(n.close_anywhere(&SessionId(id.to_owned())))
}

/// **A message composed into the broadcast** (STORE-BRIEF phase 5,
/// `Node::post`): the typed form (`@机器:路径`, files on machines, words)
/// and this machine's files to go with it — the call `khor tell` and
/// `khor send` make. Answers the message id, or the node's refusal (a
/// target not reachable is named, and nothing was written).
pub async fn post(root: &Path, typed: &str, files: &[String]) -> Result<String, String> {
    let files: Vec<std::path::PathBuf> = files.iter().map(std::path::PathBuf::from).collect();
    open(root)?.post(typed, &files).await
}

/// Whether claude on **this** machine is set up to tell khor what it is
/// doing.
///
/// One bit, because the control it feeds has two states and no third.
/// `khor hooks` is where the per-event detail lives, including the case
/// this bit folds in: hooks recorded here that point at a *different*
/// khor read as not installed, which is right for a button — pressing
/// install is exactly what fixes them.
#[derive(Debug, Clone, Serialize, TS)]
pub struct HooksState {
    /// True when every event khor asks for names this binary.
    pub installed: bool,
}

fn hooks_state_of(n: &Node) -> Result<HooksState, String> {
    use khor_node::adaptor::claude::Installed;
    let report = n.hooks_report()?;
    Ok(HooksState {
        installed: report.events.iter().all(|(_, s)| *s == Installed::Here),
    })
}

pub fn hooks_state(root: &Path) -> Result<HooksState, String> {
    hooks_state_of(&open(root)?)
}

/// Adds them, and answers with the state afterwards.
///
/// **The new state comes back from the same call**, so the button cannot
/// spend a moment showing the old one — and it is re-read rather than
/// assumed, which is what makes a half-applied install visible instead
/// of a button that claims success because the call returned.
pub fn install_hooks(root: &Path) -> Result<HooksState, String> {
    let n = open(root)?;
    n.install_hooks()?;
    hooks_state_of(&n)
}

pub fn uninstall_hooks(root: &Path) -> Result<HooksState, String> {
    let n = open(root)?;
    n.uninstall_hooks()?;
    hooks_state_of(&n)
}

/// What this installation of khor is, for the identity card's foot
/// (docs/MORE.md 头像) and for 设置's 关于 section.
///
/// **The version is this binary's, never a claim about the network.**
/// A device row carries no version (`khor_sync::devices::DeviceInfo`),
/// and inventing one here would put a number on every machine's card
/// that nothing ever measured. What another machine is running is a
/// question `khor doctor` asks over the wire, not a field on a table.
///
/// `root` is the store this face is talking to — the answer to
/// 「这一份 khor 住在哪儿」, which on a machine with two homes
/// (`KHOR_HOME`) is the only way to tell them apart.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct About {
    pub version: String,
    pub root: String,
    /// Whether this machine is paused (docs/MORE.md 暂停). It rides on
    /// `About` rather than a call of its own for the reason the store
    /// size does — it is read the same way, off one file with no node
    /// opened — and that is also why a card can show it on a machine
    /// whose store will not open.
    pub paused: bool,
    /// How much of the disk the store is using — 设置's 存储 section
    /// (docs/MORE.md). It rides here rather than on a call of its own
    /// because it is the same kind of fact as `root` and is read the
    /// same way: a walk of `.khor/` with no node opened
    /// (`doctor::store_bytes`, which 诊断 already does).
    pub store_bytes: u64,
    /// Which of khor's interfaces are up on this machine — the desktop
    /// app, the web face (docs/MORE.md 头像页: 本机开着哪几张脸).
    ///
    /// **Here rather than in a call of its own**, `paused`'s reason:
    /// it is read the same way, off two files and a loopback port with
    /// no node opened, by the one page that asks for `About` at all.
    /// The CLI's equivalent is `khor faces`, which prints this same
    /// list from the same function.
    pub faces: Vec<FaceRow>,
}

/// One of khor's interfaces on this machine, as the identity card draws
/// it (docs/MORE.md 头像页).
///
/// **The word is looked up here, not in TS** — `DoctorRow`'s judgment:
/// the catalog has one entry per key (`khor_node::face::face_name`), and
/// a second match on the same keys over there would be one rename away
/// from a card with a row labelled `web`.
///
/// `said` is the whole right-hand side already assembled — 运行中 with
/// the port when there is one, 未运行 when there is not. Assembling it
/// on this side is what keeps a port from ever being printed beside
/// 未运行.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct FaceRow {
    pub key: String,
    pub name: String,
    pub up: bool,
    pub said: String,
}

/// Neither half opens a node: the version is a compile-time constant and
/// the root is read off the environment. **That is what makes this
/// callable from a machine whose store refuses to open** — which is the
/// one moment a person most needs to be told which store that is.
pub fn about(root: &Path) -> About {
    About {
        version: khor_node::VERSION.to_owned(),
        root: root.display().to_string(),
        paused: khor_node::pause::is_on(root),
        store_bytes: khor_node::doctor::store_bytes(root),
        faces: khor_node::face::faces(root)
            .into_iter()
            .map(|f| FaceRow {
                name: khor_node::face::face_name(&f.key).to_owned(),
                said: match (&f.at, f.up) {
                    (Some(at), true) => format!(
                        "{} · {}",
                        khor_catalog::cli::face_up(),
                        khor_catalog::cli::face_at(at)
                    ),
                    (_, true) => khor_catalog::cli::face_up().to_owned(),
                    _ => khor_catalog::cli::face_down().to_owned(),
                },
                key: f.key,
                up: f.up,
            })
            .collect(),
    }
}

/// One value a setting takes, with the word for it.
///
/// **The word is looked up here, not in TS** — the same judgment
/// `DoctorRow` makes: the catalog has one entry per value
/// (`khor_catalog::config`), and a second match on the same keys over
/// there would be one restyle away from a page with a button labelled
/// `files-click`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ConfigChoice {
    pub value: String,
    pub label: String,
}

/// One row of 设置 (docs/MORE.md), as the pages draw it.
///
/// `section` is the node's (`khor_node::config::section`) so the two
/// pages group these rows the way the table says rather than by a
/// grouping written a second time in TS.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ConfigRow {
    pub name: String,
    pub label: String,
    pub section: String,
    pub section_label: String,
    /// Which 「…」 page draws this row's section — `settings` or
    /// `security` (`khor_node::config::Section::page`). It rides on the
    /// row so neither page keeps a list of the sections it owns.
    pub page: String,
    pub value: String,
    pub choices: Vec<ConfigChoice>,
    /// The sentence under the row's name — **what the setting does**
    /// (components/SettingsGroup: 「note 写会发生什么」) — when the
    /// catalog has one (`<key>-note`, 2026-10-02). Most keys have none.
    pub note: Option<String>,
}

fn config_row(k: &khor_node::config::Key, value: &str) -> ConfigRow {
    ConfigRow {
        name: k.name.to_owned(),
        label: khor_catalog::config::word(k.name).to_owned(),
        section: k.section.to_owned(),
        section_label: khor_catalog::config::word(k.section).to_owned(),
        // A key whose section is not on the node's list would be drawn
        // by nobody, which is what the empty string says here — and
        // `khor_node`'s own gate refuses that table, so it cannot ship.
        page: khor_node::config::section(k.section)
            .map(|s| s.page)
            .unwrap_or_default()
            .to_owned(),
        value: value.to_owned(),
        choices: k
            .values
            .iter()
            .map(|v| ConfigChoice {
                value: (*v).to_owned(),
                label: khor_catalog::config::word(v).to_owned(),
            })
            .collect(),
        note: {
            // `word` echoes a name it does not know, so an echo is
            // 「this key has no note」 — `khor-node`'s gate holds the
            // other direction (a note's stem is a key).
            let name = format!("{}-note", k.name);
            let said = khor_catalog::config::word(&name);
            (said != name).then(|| said.to_owned())
        },
    }
}

/// Every setting and what it is set to. **No node is opened**: the
/// preferences file is read straight off the store, so a face can ask
/// this before it has drawn anything — which is what lets the theme be
/// stamped before the first paint (批 D2).
pub fn config_rows(root: &Path) -> Vec<ConfigRow> {
    khor_node::config::list(root)
        .into_iter()
        .map(|(k, v)| config_row(k, v))
        .collect()
}

/// Sets one setting and answers **the whole list again**.
///
/// The whole list rather than the one row, for `set_lock`'s reason one
/// step further: a page that patched its own copy would be holding an
/// answer nobody checked, and these rows are what two different pages
/// draw from.
pub fn set_config(root: &Path, name: &str, value: &str) -> Result<Vec<ConfigRow>, String> {
    khor_node::config::set(root, name, value)?;
    // **语言 takes effect without a restart** (docs/MORE.md 语言). Every
    // binary resolves this once at startup, which is right for a CLI
    // that lives for one command and wrong for an app somebody is
    // standing in front of: the rows this very call is about to answer
    // with would still be in the old language. The face re-stamps its
    // own side from the same key (`src/lang.ts`).
    if name == "lang" {
        khor_node::config::apply_lang(root);
    }
    Ok(config_rows(root))
}

/// One row of 诊断 (docs/MORE.md), as the page draws it.
///
/// **Not `khor_node::doctor::Check` itself**, and the reason is where
/// the words are: the node answers a key and a sentence, and the page
/// also needs the check's *name* — which is a catalog lookup the node
/// already owns (`doctor::name`). Doing it here means the two faces read
/// one answer; doing it in TS would be a second match on the same keys,
/// one restyle away from a page with a row called `launcher`.
///
/// `state` travels as its wire spelling (`ok` / `warn` / `fail` /
/// `unmeasured`) rather than as a boolean pair — 未量 is a state of its
/// own and must not be paintable as ok.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DoctorRow {
    pub key: String,
    pub name: String,
    pub state: String,
    pub said: String,
    /// Which of `khor_node::doctor::GROUPS` this row is filed under, and
    /// the word for it — the pair `ConfigRow` carries for a section, for
    /// the same reason: the page folds the healthy rows under three
    /// headings, and neither the key nor the word is its to decide.
    pub group: String,
    pub group_label: String,
    /// The key to hand back to `doctor_fix`, when khor can mend this.
    pub fix: Option<String>,
}

/// **A line a terminal noted about itself** (items 18 A and 33): on
/// stderr, the device console, and in the notes `khor doctor`'s
/// `terminal` row reads (`khor_node::doctor::note_face`). A store that
/// cannot be written loses the note, never the terminal.
pub fn face_note(root: &Path, line: &str) {
    eprintln!("face: {}", line.chars().take(400).collect::<String>());
    let _ = khor_node::doctor::note_face(root, line);
}

/// The whole checklist. Each check under its own timeout, concurrently —
/// see `khor_node::doctor`.
pub async fn doctor(root: &Path) -> Result<Vec<DoctorRow>, String> {
    let n = open(root)?;
    Ok(khor_node::doctor::doctor(&n)
        .await
        .into_iter()
        .map(|c| DoctorRow {
            name: khor_node::doctor::name(&c.key).to_owned(),
            key: c.key,
            state: c.state.key().to_owned(),
            said: c.said,
            group_label: khor_node::doctor::group_name(&c.group).to_owned(),
            group: c.group,
            fix: c.fix,
        })
        .collect())
}

/// Runs one check's remedy and answers what it did, in words. Refused by
/// name for a key with no remedy — a button that reported success at
/// doing nothing is the shape where somebody believes a thing was
/// mended.
pub fn doctor_fix(root: &Path, key: &str) -> Result<String, String> {
    khor_node::doctor::fix(&open(root)?, key)
}

/// 安全 (docs/MORE.md), as the page and the lock screen read it.
///
/// **The Argon2id hash is not on it** — a screen has no use for one, and
/// a value that crosses a wire is a value that ends up in a log.
/// `timeouts` rides along so the page's ladder is the node's rather than
/// a second copy of it (`khor_node::lock::TIMEOUTS`).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LockFacts {
    pub on: bool,
    pub has_pin: bool,
    pub timeout_min: u32,
    pub timeouts: Vec<u32>,
    /// The PIN floor, so the box can refuse before a round trip and say
    /// the same number the node would (`khor_node::lock::PIN_MIN`).
    pub pin_min: u32,
    /// Whether this machine has the system's own unlock behind the
    /// lock screen's second button (Touch ID / the login password,
    /// `system_unlock`). **A Mac's alone**: the command asks
    /// LocalAuthentication there and answers `false` everywhere else, so
    /// on a phone the button was drawn and did nothing when pressed
    /// (docs/IOS.md 第二步) — and so on a Linux or Windows desktop, the
    /// same lie one platform over. Read here because the lock screen is
    /// drawn before anything else is known, and this answer is one the
    /// face already has at that point.
    pub system_door: bool,
}

fn lock_facts(state: khor_node::lock::LockState) -> LockFacts {
    LockFacts {
        on: state.on,
        has_pin: state.has_pin,
        timeout_min: state.timeout_min,
        timeouts: khor_node::lock::TIMEOUTS.to_vec(),
        pin_min: khor_node::lock::PIN_MIN as u32,
        system_door: cfg!(target_os = "macos"),
    }
}

/// How this machine's faces are set to lock. **No node is opened**: the
/// lock file is read straight off the store, so a face can ask this
/// before it has drawn anything at all.
pub fn lock_state(root: &Path) -> LockFacts {
    lock_facts(khor_node::lock::state(root))
}

/// Turns the lock on or off. Refused by name with no PIN set.
pub fn set_lock(root: &Path, on: bool) -> Result<LockFacts, String> {
    khor_node::lock::set_on(root, on).map(lock_facts)
}

/// Sets the PIN, or clears it when handed an empty one — which turns
/// the lock off in the same move.
pub fn set_lock_pin(root: &Path, pin: &str) -> Result<LockFacts, String> {
    khor_node::lock::set_pin(root, pin).map(lock_facts)
}

pub fn set_lock_timeout(root: &Path, minutes: u32) -> Result<LockFacts, String> {
    khor_node::lock::set_timeout(root, minutes).map(lock_facts)
}

/// Whether this is the PIN. **`false` is not an error**: a wrong PIN is
/// an ordinary event the lock screen says 「不对」 to, and only khor
/// being unable to read its own record is a refusal.
pub fn unlock(root: &Path, pin: &str) -> Result<bool, String> {
    khor_node::lock::check_pin(root, pin)
}

/// The web face's key as it is on disk, or `None` on a machine that has
/// never raised a face. Shown masked; the page reveals it on a press.
pub fn web_key(root: &Path) -> Result<Option<String>, String> {
    khor_node::face::web_key_read(root)
}

/// A new key, replacing whatever was there — the same call `khor web
/// --new` makes. Every link printed before it stops working, which is
/// the whole point of the verb.
pub fn rotate_web_key(root: &Path) -> Result<String, String> {
    khor_node::face::web_key_rotate(root)
}

/// 重置身份: throws this machine's key away. The machine's own name is
/// the confirmation and the node is what checks it — a dialog with its
/// own rule would be a second rule about one irreversible act.
pub fn reset_identity(root: &Path, name_typed: &str) -> Result<(), String> {
    let name = open(root)?.name().to_owned();
    khor_node::lock::reset_identity(root, name_typed, &name)
}

/// A freshly minted ticket, and how long it is good for.
///
/// **The window travels with the ticket instead of being written into
/// the screen that shows it.** A dialog holding its own `15` would be a
/// second copy of a number only `khor_node::link::INVITE_WINDOW_MS`
/// enforces, and the day that constant moves the app would go on
/// promising the old one — a sentence nobody can catch by reading either
/// file alone, since neither would be wrong on its own.
///
/// Minutes, because that is the unit the words for it use
/// (`invite-window` on this side, `invite-expired` on the refusal), and
/// the conversion happens once in the library.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Ticket {
    pub token: String,
    pub minutes: u32,
    /// **The same token as a QR code** (2026-09-30, the owner: 「邀请
    /// 设备显示两个内容：现在的 token，和一个二维码」). Encoded from
    /// `token` right here, so the picture and the string cannot be two
    /// different invites. `None` only when the token is too long for any
    /// QR version — the string is still the whole invite then.
    pub qr: Option<qr::Qr>,
}

/// 接管 (批C): the chat face becomes the session's one body — the node
/// ends the terminal side and resumes the conversation (`Node::takeover`).
pub fn takeover(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.takeover(&SessionId(id.to_owned()))
}

/// 接管 the other way: a conversation khor holds moves into a terminal
/// (`Node::takeover_term`). Same row, same conversation — the form is
/// what changes hands.
pub fn takeover_term(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.takeover_term(&SessionId(id.to_owned()))
}

/// One registered ACP agent, for the wizard's 智能体 list (批⑥).
///
/// The command is spelled the way the person typed it rather than as
/// the launch JSON it is stored in: this is a label under a name, and
/// reading the JSON back would be reading an implementation.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentRow {
    pub name: String,
    pub command: String,
}

/// Every ACP agent this person has named (`khor_sync::agents`), sorted.
///
/// khor's own two are **not** in here: they are not registrations, they
/// have shims of their own, and a wizard that listed them from the same
/// place would have to invent entries nobody made. The face puts them
/// beside this list, which is what they are.
pub fn agents(root: &Path) -> Result<Vec<AgentRow>, String> {
    Ok(open(root)?
        .agents()?
        .into_iter()
        .map(|(name, spec)| AgentRow {
            name,
            command: spec.typed(),
        })
        .collect())
}

/// The wizard's door: a fresh agent session where it pointed, **on the
/// machine it named** (`machine` empty or this machine's own name is
/// here). The judgment — which agent, which form, which directory,
/// which title — lives in [`khor_node::Node::start_session`] and runs on
/// the machine that will host the session: here directly, or over
/// there through [`khor_node::Node::open_agent_on`], which carries the
/// four answers and lets that machine resolve them against its own
/// registrations and paths. Answers the new row's id, which every face
/// will call it by wherever it was born.
///
/// `token` is the face's create token (`khor_node`'s `once`, round
/// NET-7): the same token asked again — a retry after an answer that
/// never came — answers the session the first ask made. Only another
/// machine's create needs it; here there is no road to lose an answer on.
pub async fn open_session(
    root: &Path,
    dir: &str,
    title: &str,
    form: &str,
    agent: &str,
    machine: &str,
    token: Option<&str>,
) -> Result<String, String> {
    let n = open(root)?;
    if machine.is_empty() || machine == n.name() {
        return n.start_session(dir, title, form, agent).map(|id| id.0);
    }
    n.open_agent_on(machine, dir, title, form, agent, token)
        .await
        .map(|id| id.0)
}

/// One message in a machine's conversation, shaped for painting. The
/// words and the order are the node's (`Node::log_faced`); this only
/// flattens the enum so a TS switch cannot invent a fourth arm.
#[derive(Debug, Clone, Serialize, TS)]
pub struct ChatMsgRow {
    pub id: String,
    #[ts(type = "number")]
    pub at_ms: i64,
    /// The sender's name at the time of writing — history must not
    /// rewrite itself on a later rename (`khor_sync::chat::Sender`).
    pub from: String,
    pub from_me: bool,
    pub text: Option<String>,
    pub files: Option<Vec<ChatFileRow>>,
    /// A body kind this build cannot read, by its kind name — said out
    /// loud rather than rendered as blank text (`MsgBody::Unknown`'s
    /// rule).
    pub unknown: Option<String>,
    pub retracted: bool,
    pub edited: bool,
    /// The file message's transfer face on this machine, as a state
    /// key (`TransferKind::message_face`) — `None` is "not my seat",
    /// never "done". The frontend looks the word up and never derives
    /// one (docs/UX.md 状态呈现).
    pub word: Option<String>,
    /// A file message's move as its receiver last wrote it
    /// (`khor_sync::chat::Fate`), the same on every device: `moving`,
    /// `landed`, `failed`, `stopped`; `None` until the receiver starts it.
    pub step: Option<String>,
    /// Bytes on the receiver so far.
    #[ts(type = "number")]
    pub moved: i64,
    /// Where each file landed on the receiver; empty until it has.
    pub landed: Vec<String>,
    /// Why the move failed, in the receiver's sentence.
    pub why: Option<String>,
    /// The directory the sender chose on the receiver; `None` for its
    /// receive location.
    pub into: Option<String>,
    /// In the broadcast (`khor_node::wide`), the machines that have this
    /// file, out of `of` it was offered to; `None` elsewhere. In the
    /// broadcast `step`, `moved`, `landed` and `why` are **this
    /// machine's own** answer to the offer, and `None` in `step` on a
    /// machine that has not answered is 待接收.
    pub got: Option<u32>,
    pub of: Option<u32>,
    /// In the broadcast, the machine this one's copy was pulled from when
    /// that was not the sender but a machine that had it (BCAST-BRIEF's
    /// holder), by its shown name; `None` otherwise.
    pub via: Option<String>,
    /// The machines this message was pushed to (STORE-BRIEF phase 5), in
    /// the order named, each by **today's** name and with its own move.
    pub to: Vec<ChatTarget>,
    /// The words said with a files message; `None` otherwise.
    pub note: Option<String>,
    /// Whether this machine is one of `to`: its file comes by itself,
    /// nobody presses 接收.
    pub pushed: bool,
}

/// One machine a message was pushed to, as the bubble shows it.
#[derive(Debug, Clone, Serialize, TS)]
pub struct ChatTarget {
    /// Its device id — what the `@机器` filter keys on.
    pub device: String,
    /// Its name as shown today (a rename shows the new name).
    pub name: String,
    /// The directory named there; `None` for its receive location.
    pub path: Option<String>,
    /// Its move of the files: `moving`, `landed`, `failed`, `stopped`;
    /// `None` before it has started (or for a line).
    pub step: Option<String>,
    pub why: Option<String>,
    pub landed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ChatFileRow {
    pub name: String,
    #[ts(type = "number")]
    pub size: i64,
}

/// The whole conversation readout.
#[derive(Debug, Clone, Serialize, TS)]
pub struct MachineLog {
    pub messages: Vec<ChatMsgRow>,
    /// Blocks that would not read — a missing slice of conversation
    /// the screen must be able to mention (`chat::ChannelLog`).
    #[ts(type = "number")]
    pub broken: u64,
}

/// The broadcast's readout — what `khor log` prints, shaped for the
/// chat pane. Takes the row's id (`wide/all`). ⚠️ 墓碑 (STORE-BRIEF phase
/// 5): a machine's `chat/<机器>` conversation read here too; that session
/// kind is gone.
pub fn machine_log(root: &Path, id: &str) -> Result<MachineLog, String> {
    wide_log(&open(root)?, id)
}

/// The broadcast's readout (`khor_node::wide`), in the same shape: a
/// file's move is this machine's own answer to the offer, and the count
/// of the machines that have it rides beside.
fn wide_log(n: &khor_node::Node, id: &str) -> Result<MachineLog, String> {
    let (log, faces) = n.wide_log(id)?;
    let me = n.device_str().to_owned();
    let names: std::collections::HashMap<String, String> =
        n.devices()?.into_iter().map(|d| (d.id.clone(), d.shown().to_owned())).collect();
    let messages = log
        .messages
        .into_iter()
        .zip(faces)
        .map(|(m, face)| {
            let (text, files, unknown) = match m.body {
                khor_node::MsgBody::Text(t) => (Some(t), None, None),
                khor_node::MsgBody::Files(fs) => (
                    None,
                    Some(fs.into_iter().map(|f| ChatFileRow { name: f.name, size: f.size }).collect()),
                    None,
                ),
                khor_node::MsgBody::Unknown(k) => (None, None, Some(k)),
            };
            let here = face.as_ref().and_then(|f| f.here.clone());
            let named = |id: &str| names.get(id).cloned().unwrap_or_else(|| id.chars().take(8).collect());
            let to: Vec<ChatTarget> = m
                .to
                .iter()
                .map(|t| {
                    let fate = m.fates.get(&t.device);
                    ChatTarget {
                        device: t.device.clone(),
                        name: named(&t.device),
                        path: Some(t.path.clone()).filter(|p| !p.is_empty()),
                        step: fate.map(|f| f.step.key().to_owned()),
                        why: fate.map(|f| f.why.clone()).filter(|w| !w.is_empty()),
                        landed: fate.map(|f| f.landed.clone()).unwrap_or_default(),
                    }
                })
                .collect();
            let pushed = m.to.iter().any(|t| t.device == me);
            let via = here
                .as_ref()
                .filter(|f| !f.via.is_empty() && f.via != m.from.id)
                .map(|f| names.get(&f.via).cloned().unwrap_or_else(|| f.via.chars().take(8).collect()));
            ChatMsgRow {
                at_ms: m.at,
                from_me: m.from.id == me,
                from: m.from.name,
                id: m.id,
                text,
                files,
                unknown,
                retracted: m.retracted,
                edited: m.edited,
                word: None,
                step: here.as_ref().map(|f| f.step.key().to_owned()),
                moved: here.as_ref().map_or(0, |f| f.moved),
                landed: here.as_ref().map(|f| f.landed.clone()).unwrap_or_default(),
                why: here.as_ref().map(|f| f.why.clone()).filter(|w| !w.is_empty()),
                into: None,
                got: face.as_ref().map(|f| f.got),
                of: face.as_ref().map(|f| f.of),
                via,
                to,
                note: Some(m.note).filter(|n| !n.is_empty()),
                pushed,
            }
        })
        .collect();
    Ok(MachineLog {
        messages,
        broken: log.broken as u64,
    })
}

/// The directory on the receiving machine a raw upload names
/// (`into=`), empty for its receive location.
pub fn into_arg(query: &str) -> String {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| *k == "into")
        .and_then(|(_, v)| percent_decode(v))
        .unwrap_or_default()
}

/// The most bytes one browser upload may carry: 1 GiB.
///
/// One constant, one place — the LAN listener and the dev bridge both
/// enforce it, and two copies is how a file gets refused on one face
/// and taken on the other. The number: the upload crosses one LAN hop
/// and streams to disk (never through memory whole), so the cap is not
/// protecting RAM — it bounds how much disk a single unauthenticated-
/// by-mistake or fat-fingered POST can commit, while staying above a
/// phone video, the largest thing this pipe is for. Bigger things have
/// `khor send` with a real path on the machine that has them.
pub const UPLOAD_CAP_BYTES: u64 = 1 << 30;

/// Streams one uploaded file to the sender's own staging area and
/// answers the staged path, refusing past [`UPLOAD_CAP_BYTES`] with the
/// catalog's sentence.
///
/// The staged file is **the offer's source, not a temporary**: `send`
/// records its path and serves the far side's fetch from it (the chat
/// document carries only the summary — payload bytes exist nowhere else
/// on the sending machine), so it must outlive the transfer the way a
/// CLI send's real path does. What must NOT survive is a failed upload
/// or a failed send — the caller deletes the staged file on any error
/// after this returns, and this function deletes it on its own errors,
/// so no orphan marks a send that never happened.
///
/// The name lands under the same whitelist the receiving side applies
/// (`transfer::safe_name` is not exported; the cleaning here mirrors
/// it): the browser hands over whatever it likes, and a name is not a
/// path.
pub fn save_upload(
    root: &Path,
    name: &str,
    body: &mut dyn std::io::Read,
) -> Result<std::path::PathBuf, String> {
    use std::io::Write;
    let clean: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | '\0'))
        .collect();
    let clean = clean.trim_start_matches('.');
    let clean = if clean.is_empty() { "file" } else { clean };
    let dir = root.join(".khor").join("uploads");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Epoch-ms plus pid in the name: two uploads of one file must not
    // overwrite each other while the first is still being fetched.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = dir.join(format!("{stamp}-{}-{clean}", std::process::id()));
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut taken: u64 = 0;
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = match body.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let _ = std::fs::remove_file(&path);
                return Err(e.to_string());
            }
        };
        taken += n as u64;
        if taken > UPLOAD_CAP_BYTES {
            let _ = std::fs::remove_file(&path);
            return Err(khor_catalog::msg::upload_too_big(UPLOAD_CAP_BYTES >> 20));
        }
        if let Err(e) = f.write_all(&buf[..n]) {
            let _ = std::fs::remove_file(&path);
            return Err(e.to_string());
        }
    }
    Ok(path)
}

/// The upload route's two arguments off its query string,
/// percent-decoded — a file name is user text and arrives encoded.
/// Both required: an upload with no machine has nowhere to go, one
/// with no name would invent one. Here rather than in each server so
/// the web face and the dev bridge parse one contract.
/// A file for an agent conversation's query (`/chat_put?id=…&name=…`,
/// round CHAT): the conversation and the file's name, the body being the
/// bytes. `None` where either is missing.
/// `percent_decode`, for the app's headers (`chat_put`).
pub fn percent_decoded(v: &str) -> Option<String> {
    percent_decode(v)
}

pub fn chat_put_args(query: &str) -> Option<(String, String)> {
    let mut id = None;
    let mut name = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=')?;
        match k {
            "id" => id = Some(percent_decode(v)?),
            "name" => name = Some(percent_decode(v)?),
            _ => {}
        }
    }
    match (id, name) {
        (Some(i), Some(n)) if !i.is_empty() && !n.is_empty() => Some((i, n)),
        _ => None,
    }
}

/// A picked file's query (`/term_drop_stage?name=…`, item 60 part 3):
/// its name, the body being the bytes. `None` without one.
pub fn drop_stage_args(query: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| *k == "name")
        .and_then(|(_, v)| percent_decode(v))
        .filter(|n| !n.is_empty())
}

/// The upload route's one argument off its query string: the file's
/// name, percent-decoded. `None` without one — a name is never invented.
pub fn upload_args(query: &str) -> Option<String> {
    drop_stage_args(query)
}

/// Minimal percent-decoding (RFC 3986): enough for what
/// `encodeURIComponent` produces, refusing malformed escapes rather
/// than guessing at them. No crate — a dozen lines is not a dependency.
fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'%' => {
                let hi = char::from(bytes.next()?).to_digit(16)?;
                let lo = char::from(bytes.next()?).to_digit(16)?;
                out.push((hi * 16 + lo) as u8);
            }
            b'+' => out.push(b' '),
            other => out.push(other),
        }
    }
    String::from_utf8(out).ok()
}

/// One browser upload for the broadcast's composer, staged: the bytes
/// streamed to this machine's staging area under the cap, the staged
/// path answered — the shared body behind the web face's and the dev
/// bridge's upload routes. The composer posts that path with its
/// message ([`post`]); the staged file is the offer's source from then
/// on. A refused post keeps it: the draft and its files stay for the
/// next try (a draft abandoned leaves its staged copy under
/// `.khor/uploads`).
pub fn stage_upload(root: &Path, name: &str, body: &mut dyn std::io::Read) -> Result<String, String> {
    save_upload(root, name, body).map(|p| p.display().to_string())
}

/// A conversation file sent again after it failed or was cancelled —
/// `khor retry`, through the same node call (docs/KHOR.md equivalence).
/// Takes the message id the chip carries; the transfer session id is
/// spelled here because it is the node's address scheme, not the
/// screen's. Answers where the files landed.
pub async fn retry_transfer(root: &Path, msg: &str) -> Result<Vec<String>, String> {
    let n = open(root)?;
    let (_, landed) = n
        .take(&khor_node::transfer::TransferKind::session_id(msg))
        .await?;
    Ok(landed)
}

/// A conversation file's move cancelled, on its receiver: nothing
/// partial stays (the chip's ×).
pub async fn stop_transfer(root: &Path, msg: &str) -> Result<(), String> {
    open(root)?
        .stop_take(&khor_node::transfer::TransferKind::session_id(msg))
        .await
}

/// The machine card's 在这台开终端 — the GUI face of `open` / `open
/// --on` (docs/KHOR.md: the faces are equivalent because they are one
/// call, and these are those calls). **The fork is on the row, not on
/// the person**: `attach` set the precedent — 行在哪台机器上就接哪台,
/// 不用说 — so this machine's own row opens locally, the CLI's bare
/// `open`, and any other routes over, its `open --on`. The command is
/// left empty going over so the far machine's own shell answers, the
/// CLI's judgment verbatim; locally the shell is resolved here exactly
/// as the CLI resolves it. The size is a placeholder the terminal pane
/// corrects on attach (`term_resize`), because no tty exists here to
/// measure.
///
/// **The wizard's shell kind arrives here too** (2026-09-06: 新建会话
/// 分 留言/智能体/shell 三档): `title` and `cwd` are its two answers,
/// both optional — an empty title is named after the shell, an empty
/// directory is the home, which is what the bare verb always did. `~`
/// is spelled out here for this machine and by the far side for that
/// one (`Request::Open`'s handler), the same rule `start_session` keeps
/// for agents: a path is expanded on the disk it names. An empty
/// `machine` is this one, `open_session`'s reading of the same blank.
///
/// `size` is the terminal the face will show it in, when it knows: the
/// shell's first prompt is written for that width before any face has
/// attached, and zsh's prompt mark (`PROMPT_SP`: the reverse-video `%`
/// plus a line of spaces, erased by the prompt only when it fits) wraps
/// into a line of its own in a narrower face — the `%` above a fresh
/// shell's first prompt on the phone. `None` is the 80×24 of every
/// terminal that has not been told.
pub async fn open_term_on(
    root: &Path,
    machine: &str,
    title: &str,
    cwd: &str,
    size: Option<(u16, u16)>,
    token: Option<&str>,
) -> Result<String, String> {
    let size = size
        .map(|(c, r)| (c.clamp(20, 500), r.clamp(5, 200)))
        .unwrap_or((80, 24));
    let n = open(root)?;
    if machine.is_empty() || machine == n.name() {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let shell_name = std::path::Path::new(&shell)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| shell.clone());
        let title = if title.trim().is_empty() {
            shell_name
        } else {
            title.trim().to_owned()
        };
        let cwd = cwd.trim();
        if cwd.is_empty() {
            let id = n.open_persistent(khor_node::kind::SHELL, &title, &[shell], size)?;
            return Ok(id.0);
        }
        let home = std::env::home_dir();
        let path = match cwd {
            "~" => home.ok_or_else(|| khor_catalog::msg::no_such_dir("~"))?,
            d if d.starts_with("~/") => home
                .ok_or_else(|| khor_catalog::msg::no_such_dir(d))?
                .join(&d[2..]),
            d => std::path::PathBuf::from(d),
        };
        if !path.is_dir() {
            return Err(khor_catalog::msg::no_such_dir(path.display().to_string()));
        }
        let id = n.open_persistent_at(&path, khor_node::kind::SHELL, &title, &[shell], size)?;
        return Ok(id.0);
    }
    let title = if title.trim().is_empty() {
        "shell"
    } else {
        title.trim()
    };
    let id = n
        .open_on(
            machine,
            khor_node::kind::SHELL,
            title,
            cwd.trim(),
            &[],
            size,
            token,
        )
        .await?;
    Ok(id.0)
}

/// Issues a one-time pairing ticket. It carries the live endpoint's
/// addresses, so it needs a resident serve — and both skins embed one
/// (the bridge and the app each start `serve` on their own thread), so
/// the GUI can issue a real ticket without a terminal.
pub fn invite(root: &Path) -> Result<Ticket, String> {
    let token = open(root)?.invite()?;
    Ok(Ticket {
        qr: qr::encode(&token),
        token,
        minutes: khor_node::link::invite_window_minutes(),
    })
}

/// Joins with someone's ticket. Async because `Node::pair` dials — and
/// because a resident serve holds this key's one endpoint, the call
/// usually hands off to it over loopback rather than dialing here.
pub async fn pair(root: &Path, ticket: &str) -> Result<String, String> {
    let node = open(root)?;
    node.pair(ticket).await
}

/// One machine's answer to a sync round (`khor sync`): whether it was
/// reached, and the node's own sentence either way.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SyncOutcome {
    /// The machine, by the name on the glass.
    pub machine: String,
    pub ok: bool,
    /// What was exchanged, or why it could not be.
    pub said: String,
}

/// **A sync round with every machine, now** (`khor sync`) — what pulling
/// a list down asks for (the redesign, 2026-10-02). Through the resident
/// serve when there is one (`Node::sync_now`), so the round is the one
/// the serve would have run on its own beat, only sooner. Async for
/// `pair`'s reason: it dials, and a machine that is away costs a dial's
/// timeout.
pub async fn sync_now(root: &Path) -> Result<Vec<SyncOutcome>, String> {
    let node = open(root)?;
    Ok(node
        .sync_now()
        .await?
        .into_iter()
        .map(|(machine, verdict)| match verdict {
            Ok(said) => SyncOutcome { machine, ok: true, said },
            Err(said) => SyncOutcome { machine, ok: false, said },
        })
        .collect())
}

/// One machine waiting for the root holder (`khor pending`) — the node's
/// [`khor_node::PendingRow`], minted here for [`LeaveOutcome`]'s reason:
/// what crosses to a face is this crate's own type.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PendingRow {
    pub id: String,
    /// The name on the glass.
    pub name: String,
    /// `device`, `satellite` or `leave` — the face picks the word.
    pub kind: String,
}

/// What the devices list shows above its rows (docs/CONNECT.md 根不在场):
/// who waits, and **whether this machine can do anything about it** —
/// the holder gets a button per row, everyone else a sentence saying
/// where the button is. One answer, so the two cannot disagree.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Waiting {
    pub holds: bool,
    pub rows: Vec<PendingRow>,
}

/// Who waits for the root holder, and whether this machine is it.
pub fn waiting(root: &Path) -> Result<Waiting, String> {
    let node = open(root)?;
    Ok(Waiting {
        holds: node.holds_root(),
        rows: node
            .pending()?
            .into_iter()
            .map(|r| PendingRow {
                id: r.id,
                name: r.name,
                kind: r.kind,
            })
            .collect(),
    })
}

/// `khor approve` — the holder signs one waiting machine in (or its
/// removal through), pushes the roster out, and answers the list as it
/// now stands. Async for `pair`'s reason: the push dials.
pub async fn approve(root: &Path, machine: &str) -> Result<Waiting, String> {
    open(root)?.approve(machine).await?;
    waiting(root)
}

/// The root's 24 words, for 「…」 → 安全 after the person pressed
/// 「显示…」. **App-only** (`tests/command_tables.rs` has why).
///
/// **On demand only** (2026-10-01, the owner: 「根钥匙是不是只在共享的时候
/// 出现更友好」). The app used to ask at first start whether a root born
/// here had had its words seen (`root_unshown`) and mark them seen when
/// the prompt closed (`root_words_seen`); nothing opens on its own now, so
/// both questions went. Showing still clears the node's mark.
pub fn root_words(root: &Path) -> Result<String, String> {
    open(root)?.root_words()
}

/// How a leave ended, per machine that was in the table (docs/MORE.md
/// 离开). The node's [`khor_node::link::Left`], minted here for the
/// same reason [`LockFacts`] is minted here: what crosses to a face is
/// this crate's own type, so the node is free to carry more.
///
/// **Two lists rather than a count**, which is the node's judgment and
/// not this file's: a machine in `untold` is one somebody has to go and
/// run `khor forget` on, and a number names none of them.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LeaveOutcome {
    pub told: Vec<String>,
    pub untold: Vec<String>,
}

/// This machine leaves the network — the call `khor leave <本机>` makes,
/// from the identity card.
///
/// **The name is passed through, not checked here.** `Node::leave` is
/// what refuses a name that is not this machine's, and a second rule on
/// this side would be the drift `lock::reset_identity` names: two rules
/// about one irreversible act, and the one nobody runs is the one that
/// rots. The page still disables its button on an empty box, which is
/// about the button being live, not about the act.
pub async fn leave(root: &Path, name_typed: &str) -> Result<LeaveOutcome, String> {
    let left = open(root)?.leave(name_typed).await?;
    Ok(LeaveOutcome {
        told: left.told,
        untold: left.untold,
    })
}

/// Pauses or resumes this machine (docs/MORE.md 暂停) — the call
/// `khor pause` and `khor resume` make.
///
/// **Refusing the state it is already in is the node's rule**, not a
/// thing this side re-decides: a card that greyed the control out would
/// be the second rule `lock::reset_identity` warns about, and the face
/// wants the sentence anyway to say what happened.
pub fn set_paused(root: &Path, on: bool) -> Result<(), String> {
    khor_node::pause::set_on(root, on)
}

/// Pauses or resumes talking to one machine, by name (SHELL.md 第五轮
/// 第 3 条) — the row menu's 暂停 / 恢复, the call `khor pause <机器>`
/// makes. This machine's own name is [`set_paused`]. Answers the
/// sentence the face shows.
pub fn set_machine_paused(root: &Path, machine: &str, on: bool) -> Result<String, String> {
    open(root)?.set_machine_paused(machine, on)
}

#[cfg(test)]
mod wizard_agent_tests {
    use super::*;

    /// The wizard's door went async on 2026-09-02 (在哪台批 — it can
    /// dial another machine now) and these tests never followed: they
    /// sat red, unrun, until the 机器页 batch tripped over them. The
    /// runtime is built the way every blocking door in this file
    /// builds one.
    fn wait<T>(f: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    /// A scratch tree that **takes itself away however the test leaves**
    /// (`crates/node/src/tests.rs::Scratch` has the whole reasoning: a
    /// cleanup written at the end of a test is skipped by the assertion
    /// that fails, so a red run leaks a directory every time it is red).
    /// A copy per file, which is this repo's convention for test helpers —
    /// it costs no dev-dependency.
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;

        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    fn root(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-wizard-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }

    /// **The 智能体 answer is an open position, and it refuses by name.**
    ///
    /// The refusal used to be `wizard_unknown_agent` — a programmer's
    /// error, because the wizard could only send one of two words. Now
    /// anything can arrive, so it is a lookup, and the sentence has to
    /// be the registry's: naming what *is* registered is what turns a
    /// typo into a one-step fix instead of a second command.
    ///
    /// It is the same call the CLI makes (`Node::agent_or_refuse`) —
    /// asserted here rather than trusted, because "the faces are
    /// equivalent" is a claim about a shared function, and a copy would
    /// satisfy every other assertion in this file.
    #[test]
    fn a_name_nobody_registered_is_refused_the_way_the_command_line_refuses_it() {
        let r = root("unknown");
        let n = khor_node::Node::open(r.to_path_buf()).unwrap();
        n.register_agent("zed", &["some-acp-agent".to_owned()])
            .unwrap();

        let why =
            wait(open_session(&r, "~", "", "chat", "zedd", "", None)).expect_err("no such registration");
        assert_eq!(
            why,
            n.agent_or_refuse("zedd").unwrap_err(),
            "the wizard refuses in the registry's own words, not its own"
        );
        assert!(
            why.contains("zed"),
            "and the refusal names what is registered: {why}"
        );
    }

    /// **The two forms a registered agent cannot take say which wall
    /// they hit, and they are different walls.**
    ///
    /// Enumerated, and asserted to *differ from each other and from
    /// codex's* — the neighbouring-answer trap is the whole risk here:
    /// three refusals collapsed into one "只有对话形式" would be true
    /// of all of them and useful for none, and `is_err()` cannot see
    /// the difference.
    #[test]
    fn a_registered_agent_is_refused_the_two_forms_it_cannot_take() {
        let r = root("forms");
        let n = khor_node::Node::open(r.to_path_buf()).unwrap();
        n.register_agent("zed", &["some-acp-agent".to_owned()])
            .unwrap();

        let term =
            wait(open_session(&r, "~", "", "term", "zed", "", None)).expect_err("no terminal form");
        let sched =
            wait(open_session(&r, "~", "", "agent", "zed", "", None)).expect_err("no scheduler form");
        assert_eq!(term, khor_catalog::msg::wizard_registered_no_terminal());
        assert_eq!(sched, khor_catalog::msg::wizard_registered_no_scheduler());
        assert_ne!(term, sched, "starting it and arming it are different walls");
        assert_ne!(
            term,
            khor_catalog::msg::wizard_codex_chat_only(),
            "and neither of them is codex's refusal, which is about a different limit"
        );
    }

    /// khor's own two keep every form they had. The control for the
    /// pair above: a rule that refused `term` for everything would pass
    /// both of them.
    #[test]
    fn the_two_khor_ships_keep_the_forms_they_had() {
        let r = root("builtin");
        let _n = khor_node::Node::open(r.to_path_buf()).unwrap();
        // Refused for what it *is*, not for the form — codex is
        // chat-only and claude is not, which is the pre-批⑥ shape and
        // must survive the open position.
        assert_eq!(
            wait(open_session(&r, "~", "", "term", "codex", "", None))
                .expect_err("codex has no terminal form"),
            khor_catalog::msg::wizard_codex_chat_only()
        );
        // claude's `term` is not refused on the agent axis at all: it
        // gets as far as the directory, which is the next question.
        let claude_term = wait(open_session(
            &r,
            "/nonexistent/khor-no-such-dir",
            "",
            "term",
            "claude",
            "",
            None,
        ))
        .expect_err("that directory does not exist");
        assert_eq!(
            claude_term,
            khor_catalog::msg::no_such_dir("/nonexistent/khor-no-such-dir"),
            "claude+term is refused by the directory, never by the form"
        );
    }
}
