//! Local hand-off: a resident `serve` holds this key's one endpoint
//! (two live endpoints on one key knock each other off — learned in
//! production), so one-shot verbs never bind their own while it runs —
//! they pass the job over a loopback socket and the serve executes it.
//!
//! TCP on 127.0.0.1 rather than a unix socket: identical on every
//! platform and immune to the 104-byte socket-path cap. The port alone
//! gates nothing — any local process can connect — so every hand-off
//! carries a cookie that lives in the owner-only endpoint.json beside
//! the port.

use khor_catalog::msg;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::proto::{self, MAX_FRAME};

#[derive(Debug, Serialize, Deserialize)]
pub struct Handoff {
    pub cookie: String,
    pub op: Op,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Op {
    Pair {
        ticket: String,
    },
    SyncNow,
    /// A conversation file's move, asked of its receiver: `take` lands
    /// it now (a retry after a failure or a cancel), `stop` cancels it.
    Take {
        session: String,
        action: String,
    },
    /// List a machine's directory (`proto::Request::Ls` carried to the
    /// resident endpoint). Tail-appended, like everything after v1.
    Ls {
        machine: String,
        path: String,
    },
    /// Pull a pathed file whole (the slice loop runs in the serve, like
    /// Accept's does). `dir` is where it lands. Tail-appended.
    Pull {
        machine: String,
        path: String,
        dir: String,
    },
    /// Start a borrow of a machine's network (docs/NET.md 借网): the serve
    /// dials the tunnel on its own endpoint, binds a local proxy port, and
    /// registers a borrow session — all of which must happen in the
    /// process that holds the key, never in the verb. Tail-appended.
    Borrow {
        machine: String,
    },
    /// Open a session on another machine (`proto::Request::Open`).
    /// Tail-appended.
    OpenOn {
        machine: String,
        kind: String,
        title: String,
        cwd: String,
        cmd: Vec<String>,
        cols: u16,
        rows: u16,
        /// The create's token (`crate::once`), when the asker chose one.
        /// A tail default: a resident older than the field refuses the
        /// frame, which the face says as a refusal and a restarted
        /// resident (a binary swap restarts it) answers.
        #[serde(default)]
        token: Option<String>,
    },
    /// Where another machine's session listens, and a tunnel to it —
    /// the serve is the only process holding the endpoint, so the pipe
    /// is bound here and the verb is handed a local address the way a
    /// borrow hands one. Tail-appended.
    ReachOn {
        machine: String,
        session: String,
    },
    /// End a session on another machine (`Act` "close"). Tail-appended.
    CloseOn {
        machine: String,
        session: String,
    },
    /// Each known device's link as the resident holds it — the road its
    /// wire is on and how long since it was heard (`khor_core::Link`).
    /// **Only the resident can answer**: the wires are its, and a
    /// one-shot verb that bound its own endpoint would hold none and
    /// report the whole fleet as not connected. Tail-appended.
    Hops,
    /// Tail-appended: [`crate::proto::Request::OpenAgent`] through the
    /// resident — an agent session, by name, on that machine. Resolved
    /// over there; this side only carries the wizard's answers.
    OpenAgentOn {
        machine: String,
        dir: String,
        title: String,
        form: String,
        agent: String,
        /// As `OpenOn`'s.
        #[serde(default)]
        token: Option<String>,
    },
    /// Tail-appended: [`crate::proto::Request::Places`] through the
    /// resident — 常去的地方 on that machine.
    PlacesOn {
        machine: String,
    },
    /// Tail-appended: [`crate::proto::Request::Load`] through the
    /// resident — what that machine is busy with, `top` rows deep.
    LoadOn {
        machine: String,
        top: u32,
        #[serde(default)]
        by: khor_core::TopBy,
        #[serde(default)]
        desc: bool,
    },
    /// Tail-appended: a borrow that does **not** wear a session row
    /// (docs/NET.md 可达性, 一层不占会话列表). What a browsing tab asks
    /// for: the tab is its face and its lifetime, so a row of its own
    /// would be a session in the list that is not a session.
    ///
    /// A separate op rather than a flag on [`Op::Borrow`], because a
    /// field added inside an existing variant is a change in the middle
    /// of the wire and everything after v1 here is tail-appended.
    BorrowUnlisted {
        machine: String,
    },
    /// Tail-appended: ends an unlisted borrow by the handle
    /// [`Reply::Borrowing`] answered with. The listed kind is ended by
    /// closing its row instead, which is the row's whole job.
    ///
    /// **Dropping one already gone answers the same as dropping a live
    /// one.** The caller wanted it not to exist; a refusal would only
    /// tell it about a race it cannot do anything about.
    DropBorrow {
        session: String,
    },
    /// Tail-appended: one slice of a file, for looking at rather than
    /// taking (`Node::read_head`). Distinct from [`Op::Pull`], which
    /// loops every slice onto this disk; this one answers once and the
    /// bytes never touch a file.
    ReadHead {
        machine: String,
        path: String,
    },
    /// Tail-appended: [`crate::proto::Request::GitStatus`] through the
    /// resident — what differs from HEAD across one checkout.
    GitStatusOn {
        machine: String,
        root: String,
    },
    /// Tail-appended: [`crate::proto::Request::GitDiff`] through the
    /// resident — where one file differs from HEAD.
    GitDiffOn {
        machine: String,
        path: String,
    },
    /// Tail-appended: [`crate::proto::Request::WritePath`] through the
    /// resident — saving a file back to whichever machine holds it.
    WritePathOn {
        machine: String,
        path: String,
        was: String,
        bytes: serde_bytes::ByteBuf,
    },
    /// Tail-appended: whether that machine has VS Code.
    CodeHereOn {
        machine: String,
    },
    /// Tail-appended: start (or reuse) VS Code's web face over there and
    /// **stand a local pipe in front of it** — the reply is an address
    /// on *this* machine, because a loopback port on somebody else's is
    /// not something a browser here can point at.
    CodeOpenOn {
        machine: String,
        root: String,
    },
    /// Tail-appended: stop it, and drop this machine's pipe with it.
    CodeStopOn {
        machine: String,
        root: String,
    },
    /// Tail-appended: one slice of a file **at an offset**
    /// (工作区批6). [`Op::ReadHead`] is this with the offset fixed at
    /// zero; it stays because it shipped, and because「the first slice」
    /// is what a text view wants and it should not have to say so.
    ///
    /// **The loop that uses this lives in the asker**, not here: one
    /// reply is one slice, and a whole 4 MB image answered in one frame
    /// would be eight times `proto::MAX_FRAME`.
    ReadAt {
        machine: String,
        path: String,
        offset: u64,
    },
    /// Tail-appended: [`crate::proto::Request::Touched`] through the
    /// resident — when a directory on that machine last changed
    /// (工作区批①).
    TouchedOn {
        machine: String,
        path: String,
    },
    /// Tail-appended: [`crate::proto::Request::Remove`] through the
    /// resident — taking a path off whichever machine holds it
    /// (工作区批④).
    RemoveOn {
        machine: String,
        path: String,
    },
    /// Tail-appended: [`crate::proto::Request::Mkdir`] through the
    /// resident — one new directory on whichever machine holds the
    /// parent (工作区第五轮).
    MkdirOn {
        machine: String,
        path: String,
    },
    /// Tail-appended: a new empty file on whichever machine holds the
    /// directory — a [`crate::proto::Request::WritePath`] of no bytes
    /// everywhere but a satellite ([`crate::proto::Request::SatTouch`]).
    TouchOn {
        machine: String,
        path: String,
    },
    /// Tail-appended: [`crate::proto::Request::RenamePath`] through the
    /// resident — moving a name on whichever machine holds it.
    RenameOn {
        machine: String,
        from: String,
        to: String,
    },
    /// Tail-appended: [`crate::proto::Request::Launch`] through the
    /// resident — that machine's own opener, on that machine.
    LaunchOn {
        machine: String,
        target: String,
    },
    /// Tail-appended: [`crate::proto::Request::Search`] through the
    /// resident — names and lines under one directory on whichever
    /// machine holds it (工作区第六轮 第 47 条).
    SearchOn {
        machine: String,
        dir: String,
        query: String,
    },
    /// Tail-appended: [`crate::proto::Request::Checkout`] through the
    /// resident — whether a directory on that machine is a checkout
    /// (docs/SHELL.md 第五轮 第 6 条).
    CheckoutOn {
        machine: String,
        path: String,
    },
    /// How the resident's recent dials went, per far machine
    /// (`khor_core::RoadTally`, docs/CONNECT.md 度量与验收). **Only the
    /// resident can answer**, for the reason [`Op::Hops`] gives.
    ///
    /// A new op rather than a field on [`Reply::Hops`]: a serve older
    /// than this decodes the whole hand-off as garbage and answers
    /// [`Reply::Refused`], and the consumers' error branch paints that
    /// as 未量 / 读不到 (`doctor`'s `roads` row, the 网络 page's 直连占比
    /// line) — not as a count of zero, which a field defaulting to empty
    /// on an old serve would have painted as fact. Tail-appended.
    Roads,
    /// Tail-appended: [`Op::ReachOn`] for a face whose pipe just ended
    /// (`Node::rejoin`, docs/IOS.md 第三步) — the same reach, answered
    /// so the face can tell **「not reached, try again」**
    /// ([`Reply::Unreached`]) from **「reached, and the session is not
    /// there」** ([`Reply::Refused`]). The first is a phone back from its
    /// pocket, the second a terminal that really ended. A new op rather
    /// than a new answer to `ReachOn`: a CLI of another build asking
    /// that would not know the new answer, and nothing but the face
    /// asks this one.
    Rejoin {
        machine: String,
        session: String,
    },
    /// How the resident's wires have fared since it started
    /// (`khor_core::WireCounts`) — `khor doctor`'s account of where a
    /// word on a screen came from. Tail-appended.
    Wires,
    /// This machine's connections for the 网络 page
    /// (`khor_core::ConnectionsView`): home relay, relay round trips, and
    /// per machine its road, age, round trip and last failure.
    /// Tail-appended.
    Connections,
    /// Retire the resident's wires to one machine, so the next ask dials
    /// afresh; answered with the counters after. A diagnostic, and what
    /// `crates/cli/tests/redial.rs` measures a redial with (round NET-3).
    /// Tail-appended.
    LetGo { machine: String },
    /// [`crate::proto::Request::Stat`] through the resident: what a file
    /// on that machine is now, without its bytes. Tail-appended.
    StatOn { machine: String, path: String },
    /// Which of these directories on that machine changed since `since`
    /// (`crate::watch::Watches`), held up to `wait_ms` (at most
    /// [`WATCH_WAIT`]) for one to — 0 asks without waiting, which is what
    /// a tree coming back on screen does. Tail-appended.
    WatchOn {
        machine: String,
        dirs: Vec<String>,
        since: u64,
        wait_ms: u64,
    },
    /// A machine's commands, searched (roadmap 2.6); answered with
    /// [`Reply::History`]. Tail-appended.
    HistoryOn {
        machine: String,
        ask: crate::commands::HistoryAsk,
    },
    /// One file's history on a machine (round UX-4); answered with
    /// [`Reply::Git`]. Tail-appended.
    GitOn {
        machine: String,
        ask: crate::git::GitAsk,
    },
    /// A face is on screen (`visible`), or stopped being: the resident's
    /// pump beats every 5 s while one is and every 30 s while none is, and
    /// a face arriving gets a round at once (round NET-6,
    /// `link::QUIET_PACE`). Sent again every [`FACE_BEAT`] while visible.
    /// Answered [`Reply::Noted`]. Tail-appended.
    FaceHere { visible: bool },
    /// An `Act` on a session on another machine other than closing it
    /// (`Act` "prompt-up" / "prompt-down", item 21); answered with
    /// [`Reply::Accepted`]. Tail-appended.
    ActOn {
        machine: String,
        session: String,
        action: String,
    },
    /// A tmux row's front pane history on another machine (item 31);
    /// answered with [`Reply::PaneHistory`]. Tail-appended.
    PaneHistoryOn {
        machine: String,
        session: String,
        since: Option<crate::adaptor::tmux::HistoryAnchor>,
    },
    /// One step of a drop (item 60, `crate::drops`): the file `path` on
    /// machine `from` put where machine `to` reads it, as the job named
    /// by `token` — started by the first ask, read by every later one.
    /// Answered with [`Reply::DropFile`]. Tail-appended.
    DropOn {
        token: String,
        from: String,
        path: String,
        to: String,
    },
    /// [`Op::DropOn`] into `dir` on `to` (absolute there) in place of its
    /// drop directory (item 60 part 5). Answered with
    /// [`Reply::DropFile`]. Tail-appended.
    DropInto {
        token: String,
        from: String,
        path: String,
        to: String,
        dir: String,
    },
    /// **The drop named by `token` cancelled** (item 71): the machine
    /// `to`, which pulls it, is told to stop. Answered with
    /// [`Reply::Noted`]. Tail-appended.
    DropStop { token: String, to: String },
    /// **Every machine's try at `host:port`** (round ROAD-1, `khor
    /// reach`): the resident asks them all at once
    /// (`proto::Request::Probe`). Answered [`Reply::ReachProbe`].
    /// Tail-appended.
    ReachProbe { host: String, port: u16 },
    /// **A pipe to `target` (`host:port`) through `machine`'s network**
    /// (round ROAD-1, `khor pipe`): the resident dials the exit's stream
    /// and binds a loopback port that hands it to the one connection
    /// that says the cookie first (`crate::pipe`). Answered
    /// [`Reply::Piping`]. Tail-appended.
    PipeOn { machine: String, target: String },
    /// [`crate::proto::Request::AddOverSsh`] asked of `machine` by the
    /// resident. Answered with [`Reply::Adding`]. Tail-appended.
    AddOn {
        machine: String,
        token: String,
        host: String,
        port: Option<u16>,
        jump: Option<String>,
        identity: Option<String>,
        name: Option<String>,
        satellite_only: bool,
    },
    /// **khor inside ssh, this end** (round ROAD-2, `khor _carry`): the
    /// resident binds a loopback port that takes the one connection saying
    /// the cookie first and makes it the carried link to `carrier`
    /// (`crate::carry`). Answered [`Reply::Piping`]. Tail-appended.
    CarryIn { carrier: String },
    /// **The add flow's carry** (round ROAD-3, `Node::carry_for`): an ssh
    /// session to `id`, not paired yet, by this road, so its pairing can
    /// ride it. Answered [`Reply::Noted`] once it has held. Tail-appended.
    CarryFor {
        id: String,
        host: String,
        port: Option<u16>,
        jump: Option<String>,
        identity: Option<String>,
        password: bool,
        passphrase: bool,
    },
    /// **A machine's page was opened** (NET-STABLE 第二步): a round with
    /// it now, past its backoff — the restart a person would otherwise
    /// reach for. At most once a `link::SYNC_EVERY` per machine.
    /// Answered [`Reply::Noted`] at once; the round runs on. Tail-appended.
    Visit { machine: String },
    /// [`crate::proto::Request::Journal`] asked of `machine` (net-r):
    /// answered [`Reply::Journal`]. Tail-appended.
    JournalOn { machine: String, peer: Option<String>, since: u64 },
    /// **A message composed into the broadcast** (`Node::post`): the
    /// typed form and this machine's files, resolved, checked and written
    /// by the serve, which then pushes to each target on its own time.
    /// Answered [`Reply::Posted`] once the message is written, or
    /// refused before anything is. Tail-appended.
    Post { typed: String, files: Vec<String> },
    /// Literal console prose; only explicit slash submissions execute.
    Note { text: String, files: Vec<String> },
    RecognitionOn { machine: String, action: crate::recognition::Action },
}

/// How often a visible face says so ([`Op::FaceHere`]); a face not heard
/// from for three of these is taken to be gone.
pub const FACE_BEAT: std::time::Duration = std::time::Duration::from_secs(10);

/// How long a [`Op::WatchOn`] is held when nothing changes: under
/// [`DEADLINE_FAR`], which bounds it.
pub const WATCH_WAIT: std::time::Duration = std::time::Duration::from_secs(20);

/// **In-process at the serve**: the answer is in the serve's own memory
/// or on its own disk (`Hops` reads the endpoint's view of its roads,
/// measured at 20–75 µs, and carries its own 500 ms budget inside;
/// `DropBorrow` forgets a handle). Five seconds is two orders of
/// magnitude over the work and still short enough that a face waiting
/// on it recovers before a person gives up on the click.
pub const DEADLINE_LOCAL: std::time::Duration = std::time::Duration::from_secs(5);

/// **One question to one other machine**: a dial (≤ 10 s,
/// `link::DIAL_TIMEOUT`, relay included) and one request/response
/// whose answer fits in a frame — a listing, a slice of a file, a git
/// status, a rename. Thirty seconds is the dial three times over; past
/// that the far machine is not slow, it is not answering.
pub const DEADLINE_FAR: std::time::Duration = std::time::Duration::from_secs(30);

/// **Work the far machine has to go and do**: a search walks a tree, a
/// sync visits every paired machine in turn (10 s dial each), pairing
/// waits on the inviter, a borrow dials and binds a proxy, VS Code's
/// web face may have to be fetched on first use (tens of MB). Five
/// minutes covers the slow real cases measured on this fleet (a first
/// `serve-web` download on an NFS home took about two) with room left.
pub const DEADLINE_SLOW: std::time::Duration = std::time::Duration::from_secs(300);

/// **Bytes that move**: a whole file pulled, a session's files
/// accepted. Their length is the file's, not khor's, so this is a
/// backstop against a transfer that has stopped, not a speed limit —
/// an hour moves a few GB over the slowest road this fleet has used
/// (the relay). A transfer that needs longer is a transfer this
/// deadline will name in the serve log, which is the point.
pub const DEADLINE_TRANSFER: std::time::Duration = std::time::Duration::from_secs(3600);

impl Op {
    /// The op's name for the words that say it ran out of time — the
    /// variant's own, never translated (it names code, like a check key).
    pub fn name(&self) -> &'static str {
        self.table().0
    }

    /// How long the serve may take over this op, end to end, before the
    /// asker stops waiting and the serve gives up on it (SHELL.md 第五轮
    /// 第 2 条). Chosen by **whether the work leaves the process** and
    /// how far it goes; each tier's reason is on its constant.
    pub fn deadline(&self) -> std::time::Duration {
        self.table().1
    }

    /// **Exhaustive on purpose**: an op added without a deadline must not
    /// compile, because the default it would otherwise get — none — is
    /// the bug this table exists to end.
    fn table(&self) -> (&'static str, std::time::Duration) {
        match self {
            Op::RecognitionOn { .. } => ("RecognitionOn", DEADLINE_FAR),
            Op::Hops => ("Hops", DEADLINE_LOCAL),
            Op::Roads => ("Roads", DEADLINE_LOCAL),
            Op::Wires => ("Wires", DEADLINE_LOCAL),
            Op::Connections => ("Connections", DEADLINE_LOCAL),
            Op::LetGo { .. } => ("LetGo", DEADLINE_LOCAL),
            Op::StatOn { .. } => ("StatOn", DEADLINE_FAR),
            Op::WatchOn { .. } => ("WatchOn", DEADLINE_FAR),
            Op::DropBorrow { .. } => ("DropBorrow", DEADLINE_LOCAL),
            Op::Ls { .. } => ("Ls", DEADLINE_FAR),
            Op::ReadHead { .. } => ("ReadHead", DEADLINE_FAR),
            Op::ReadAt { .. } => ("ReadAt", DEADLINE_FAR),
            Op::TouchedOn { .. } => ("TouchedOn", DEADLINE_FAR),
            Op::CheckoutOn { .. } => ("CheckoutOn", DEADLINE_FAR),
            Op::GitStatusOn { .. } => ("GitStatusOn", DEADLINE_FAR),
            Op::GitDiffOn { .. } => ("GitDiffOn", DEADLINE_FAR),
            Op::PlacesOn { .. } => ("PlacesOn", DEADLINE_FAR),
            Op::LoadOn { .. } => ("LoadOn", DEADLINE_FAR),
            Op::CodeHereOn { .. } => ("CodeHereOn", DEADLINE_FAR),
            Op::RemoveOn { .. } => ("RemoveOn", DEADLINE_FAR),
            Op::MkdirOn { .. } => ("MkdirOn", DEADLINE_FAR),
            Op::TouchOn { .. } => ("TouchOn", DEADLINE_FAR),
            Op::RenameOn { .. } => ("RenameOn", DEADLINE_FAR),
            Op::LaunchOn { .. } => ("LaunchOn", DEADLINE_FAR),
            Op::WritePathOn { .. } => ("WritePathOn", DEADLINE_FAR),
            Op::OpenOn { .. } => ("OpenOn", DEADLINE_FAR),
            Op::OpenAgentOn { .. } => ("OpenAgentOn", DEADLINE_FAR),
            Op::ReachOn { .. } => ("ReachOn", DEADLINE_FAR),
            Op::Rejoin { .. } => ("Rejoin", DEADLINE_FAR),
            Op::CloseOn { .. } => ("CloseOn", DEADLINE_FAR),
            Op::CodeStopOn { .. } => ("CodeStopOn", DEADLINE_FAR),
            Op::SearchOn { .. } => ("SearchOn", DEADLINE_SLOW),
            Op::HistoryOn { .. } => ("HistoryOn", DEADLINE_FAR),
            Op::GitOn { .. } => ("GitOn", DEADLINE_FAR),
            Op::FaceHere { .. } => ("FaceHere", DEADLINE_LOCAL),
            Op::Visit { .. } => ("Visit", DEADLINE_LOCAL),
            Op::JournalOn { .. } => ("JournalOn", DEADLINE_FAR),
            // Dials every target and may digest a far file.
            Op::Post { .. } => ("Post", DEADLINE_SLOW),
            Op::Note { .. } => ("Note", DEADLINE_SLOW),
            Op::ActOn { .. } => ("ActOn", DEADLINE_FAR),
            Op::PaneHistoryOn { .. } => ("PaneHistoryOn", DEADLINE_FAR),
            Op::DropOn { .. } => ("DropOn", DEADLINE_FAR),
            Op::DropInto { .. } => ("DropInto", DEADLINE_FAR),
            Op::DropStop { .. } => ("DropStop", DEADLINE_FAR),
            Op::ReachProbe { .. } => ("ReachProbe", DEADLINE_FAR),
            Op::PipeOn { .. } => ("PipeOn", DEADLINE_FAR),
            Op::AddOn { .. } => ("AddOn", DEADLINE_FAR),
            Op::CarryIn { .. } => ("CarryIn", DEADLINE_FAR),
            Op::CarryFor { .. } => ("CarryFor", DEADLINE_FAR),
            Op::SyncNow => ("SyncNow", DEADLINE_SLOW),
            Op::Pair { .. } => ("Pair", DEADLINE_SLOW),
            Op::Borrow { .. } => ("Borrow", DEADLINE_SLOW),
            Op::BorrowUnlisted { .. } => ("BorrowUnlisted", DEADLINE_SLOW),
            Op::CodeOpenOn { .. } => ("CodeOpenOn", DEADLINE_SLOW),
            Op::Pull { .. } => ("Pull", DEADLINE_TRANSFER),
            Op::Take { .. } => ("Take", DEADLINE_TRANSFER),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Reply {
    Paired {
        name: String,
    },
    Synced {
        outcomes: Vec<(String, Result<String, String>)>,
    },
    /// Bytes moved, and where each file landed — the serve is the one
    /// holding the endpoint, so it is the only end that learns the
    /// paths and this is the only way back to the verb that asked.
    /// `landed` is tail-appended: a resident serve from an older khor
    /// answers without it, and the caller reads that as *nobody said*
    /// (`proto::Response::Acted`).
    Accepted {
        moved: u64,
        #[serde(default)]
        landed: Vec<String>,
    },
    Refused {
        why: String,
    },
    /// Tail-appended with [`Op::Ls`].
    Dir {
        path: String,
        entries: Vec<crate::proto::DirEntry>,
        truncated: bool,
    },
    /// Tail-appended with [`Op::Pull`]: bytes moved, and where the file
    /// landed — the verb prints the landing because the default (the
    /// asker's cwd) is a place the serve never knew.
    Pulled {
        moved: u64,
        dest: String,
    },
    /// Tail-appended with [`Op::Borrow`]: the borrow session's id and the
    /// local address its proxy listens on — the verb prints the address
    /// because the caller points a browser at it.
    Borrowing {
        session: String,
        addr: String,
    },
    /// Tail-appended with [`Op::OpenOn`]: the id the session got **on
    /// that machine**, which is the id every face will call it by.
    OpenedOn {
        session: String,
    },
    /// Tail-appended with [`Op::ReachOn`]: a local address that pipes
    /// to that session's host, and the cookie its handshake wants. The
    /// address is this machine's; what is on the other end of it is the
    /// far host's socket (`tunnel`).
    Reaching {
        addr: String,
        cookie: String,
    },
    /// Tail-appended with [`Op::Hops`]: one link per device id.
    ///
    /// A list rather than a map because it crosses msgpack, and every
    /// device present in the table is present here — a machine missing
    /// from this list would be read as *absent*, and the caller could
    /// not tell that from *not connected*.
    Hops {
        by_device: Vec<(String, khor_core::Link)>,
    },
    /// Tail-appended with [`Op::PlacesOn`]: `(path, shown, sessions,
    /// last_ms)` per place, as that machine answered.
    Places {
        places: Vec<(String, String, u32, i64)>,
    },
    /// Tail-appended with [`Op::LoadOn`]: the machine's top and CPU
    /// day, as it answered.
    Load {
        load: khor_core::Load,
    },
    /// Tail-appended with [`Op::ReadHead`]: the file's whole size, its
    /// mtime, and as much of it as one slice holds. `bytes.len() <
    /// total` is the only thing that says it was cut.
    Head {
        total: u64,
        at_ms: u64,
        bytes: serde_bytes::ByteBuf,
    },
    /// Tail-appended with [`Op::StatOn`]: size, mtime, and the digest of
    /// a file that fits one slice.
    Stat {
        total: u64,
        at_ms: u64,
        digest: String,
    },
    /// Tail-appended with [`Op::WatchOn`].
    DirsChanged { changes: crate::watch::DirChanges },
    /// Tail-appended with [`Op::GitStatusOn`]: the branch, the marked
    /// paths, and whether the list was cut.
    GitStatus {
        branch: String,
        rows: Vec<(String, String)>,
        more: bool,
        /// How far the branch is from its remote, and when that was
        /// last true. Tail-appended for `Reply::CodeHere::ready`'s
        /// reason: the resident serve and the CLI talking to it are two
        /// builds on one machine and upgrade separately, and an older
        /// serve answering without them is a branch drawn with no
        /// ahead/behind, which is what a branch in step looks like too.
        #[serde(default)]
        ahead: u32,
        #[serde(default)]
        behind: u32,
        #[serde(default)]
        fetched_ms: i64,
    },
    /// Tail-appended with [`Op::GitDiffOn`]: one file's hunks, in file
    /// order, and whether the list was cut.
    GitDiff {
        hunks: Vec<crate::proto::Hunk>,
        more: bool,
    },
    /// Tail-appended with [`Op::WritePathOn`]: the mtime and digest the
    /// file has now.
    Wrote {
        at_ms: u64,
        digest: String,
    },
    /// Tail-appended with [`Op::CodeHereOn`]: empty means no editor.
    /// `ready` says that machine already has `serve-web`'s server, and
    /// is itself tail-appended — the resident serve and the CLI talking
    /// to it are two builds on one machine and upgrade separately, so
    /// an older serve answers without it and the face asks for consent
    /// it may not need rather than skipping one it does.
    CodeHere {
        version: String,
        #[serde(default)]
        ready: bool,
    },
    /// Tail-appended with [`Op::CodeOpenOn`]: where to point a browser
    /// **on this machine**, and the token the far server wants.
    CodeAt {
        addr: String,
        token: String,
    },
    /// Tail-appended with [`Op::TouchedOn`]: ms since the epoch, `0`
    /// where there was nothing to date.
    Touched {
        at_ms: i64,
    },
    /// Tail-appended with [`Op::RemoveOn`]: it is off that disk.
    Removed,
    /// Tail-appended with [`Op::MkdirOn`]: the directory is there now.
    Made,
    /// Tail-appended with [`Op::RenameOn`]: the name moved.
    Renamed,
    /// Tail-appended with [`Op::LaunchOn`]: that machine's opener took
    /// it. Not a promise that a window appeared ([`crate::proto::
    /// Response::Launched`]).
    Launched,
    /// Tail-appended with [`Op::SearchOn`]: what that machine found
    /// (`crate::files::Found`).
    Found {
        found: crate::files::Found,
    },
    /// Tail-appended with [`Op::CheckoutOn`].
    Checkout {
        yes: bool,
    },
    /// Tail-appended with [`Op::Roads`]: one tally per machine this
    /// resident has dialled since it started.
    Roads {
        by_device: Vec<(String, khor_core::RoadTally)>,
    },
    /// Tail-appended with [`Op::Rejoin`]: that machine was not reached —
    /// a dial that failed, ears that are down. Worth asking again; a
    /// refusal from the far side is still [`Reply::Refused`].
    Unreached {
        why: String,
    },
    /// Tail-appended with [`Op::Wires`].
    Wires {
        counts: khor_core::WireCounts,
    },
    /// Tail-appended with [`Op::Connections`].
    Connections {
        view: khor_core::ConnectionsView,
    },
    /// Tail-appended with [`Op::HistoryOn`].
    History {
        hits: Vec<crate::commands::HistoryHit>,
    },
    /// Tail-appended with [`Op::GitOn`].
    Git { answer: crate::git::GitAnswer },
    /// Tail-appended with [`Op::FaceHere`]: noted.
    Noted,
    /// Tail-appended with [`Op::PaneHistoryOn`].
    PaneHistory {
        history: crate::adaptor::tmux::PaneHistory,
    },
    /// Tail-appended with [`Op::DropOn`].
    DropFile { state: crate::drops::DropState },
    /// Tail-appended with [`Op::ReachProbe`].
    ReachProbe { view: khor_core::ReachView },
    /// Tail-appended with [`Op::PipeOn`]: where to connect, and the
    /// cookie to say first.
    Piping { addr: String, cookie: String },
    /// Tail-appended with [`Op::AddOn`].
    Adding { run: crate::join::Run },
    /// Tail-appended with [`Op::JournalOn`]: the far journal's lines.
    Journal { lines: Vec<String> },
    /// Tail-appended with [`Op::Post`]: the written message's id.
    Posted { id: String },
    Recognition { report: crate::recognition::Report },
}

/// One verb, one connection: write the frame, half-close, read the reply.
///
/// **Under the op's own deadline** ([`Op::deadline`]), end to end. It
/// had none (SHELL.md 第五轮 第 2 条), and the app's bridge serves one
/// request at a time — so one op stuck behind a far machine that had
/// stopped answering held every later click on every page. The serve
/// holds itself to the same deadline and names the op in its log
/// (`link::run_handoff_within`); this side is the asker's guarantee,
/// which does not depend on the serve being well.
pub async fn call(port: u16, cookie: &str, op: Op) -> Result<Reply, String> {
    call_refreshing(port, cookie, op, || None).await
}

/// An endpoint file can be replaced between reading its port and connecting
/// (notably when a suspended phone repairs its loopback listener). Retry only
/// a refused TCP connect: no operation bytes have been sent at that point.
/// Once connected, neither a lost reply nor a write failure is replayed.
pub(crate) async fn call_refreshing(
    port: u16,
    cookie: &str,
    op: Op,
    mut refresh: impl FnMut() -> Option<(u16, String)>,
) -> Result<Reply, String> {
    let (name, deadline) = (op.name(), op.deadline());
    let attempt = async {
        let until = tokio::time::Instant::now() + std::time::Duration::from_millis(500);
        let (mut port, mut cookie) = (port, cookie.to_owned());
        let stream = loop {
            match tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                Ok(stream) => break stream,
                Err(error) => {
                    if error.kind() != std::io::ErrorKind::ConnectionRefused
                        || tokio::time::Instant::now() >= until
                    {
                        return Err(msg::serve_unreachable(error));
                    }
                    let Some((next_port, next_cookie)) = refresh() else {
                        return Err(msg::serve_unreachable(error));
                    };
                    if next_port == 0 {
                        return Err(msg::serve_unreachable(error));
                    }
                    if next_port == port && next_cookie == cookie {
                        tokio::time::sleep_until(until.min(
                            tokio::time::Instant::now() + std::time::Duration::from_millis(50),
                        ))
                        .await;
                    }
                    (port, cookie) = (next_port, next_cookie);
                }
            }
        };
        exchange(stream, &cookie, op).await
    };
    match tokio::time::timeout(deadline, attempt).await {
        Ok(r) => r,
        Err(_) => Err(msg::serve_op_timed_out(name, deadline.as_secs())),
    }
}

async fn exchange(
    mut stream: tokio::net::TcpStream,
    cookie: &str,
    op: Op,
) -> Result<Reply, String> {
    let frame = proto::encode(&Handoff {
        cookie: cookie.to_owned(),
        op,
    })?;
    stream
        .write_all(&frame)
        .await
        .map_err(msg::handoff_failed)?;
    stream.shutdown().await.map_err(msg::cant_close_stream)?;
    let mut bytes = Vec::new();
    (&mut stream)
        .take(MAX_FRAME as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(msg::no_reply_from_serve)?;
    proto::decode(&bytes).map(heard)
}

/// **The resident's words, in this process's language** (round NET-9):
/// a verb and the resident it asks may speak differently (a terminal's
/// `LANG` against the app's setting), and a far refusal the resident
/// passes on was said again in the resident's. Each refusal and each
/// round's failure is said again here (`khor_catalog::reword`).
fn heard(reply: Reply) -> Reply {
    let again = |why: String| khor_catalog::reword(&why).unwrap_or(why);
    match reply {
        Reply::Refused { why } => Reply::Refused { why: again(why) },
        Reply::Unreached { why } => Reply::Unreached { why: again(why) },
        Reply::Synced { outcomes } => Reply::Synced {
            outcomes: outcomes
                .into_iter()
                .map(|(name, out)| (name, out.map_err(again)))
                .collect(),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A "serve" that takes the connection and then never says a word —
    /// the shape of an op wedged behind a far machine that stopped
    /// answering.
    async fn mute_serve() -> (u16, tokio::task::JoinHandle<()>) {
        let l = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = l.local_addr().unwrap().port();
        let h = tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((s, _)) = l.accept().await {
                held.push(s);
            }
        });
        (port, h)
    }

    /// A serve that answers every hand-off with an empty road table.
    async fn answering_serve() -> (u16, tokio::task::JoinHandle<()>) {
        let l = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = l.local_addr().unwrap().port();
        let h = tokio::spawn(async move {
            while let Ok((mut s, _)) = l.accept().await {
                let mut b = Vec::new();
                let _ = (&mut s).take(MAX_FRAME as u64).read_to_end(&mut b).await;
                let reply = proto::encode(&Reply::Hops { by_device: vec![] }).unwrap();
                let _ = s.write_all(&reply).await;
                let _ = s.shutdown().await;
            }
        });
        (port, h)
    }

    /// SHELL.md 第五轮 第 2 条: `ipc::call` had no deadline at all, and
    /// the bridge serves one request at a time — so one op stuck behind
    /// a far machine held every later click on every page. A stuck op
    /// must come back with words, and the next call must go through.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_stuck_op_comes_back_with_words_and_does_not_hold_the_next() {
        let (mute, _m) = mute_serve().await;
        let (live, _l) = answering_serve().await;
        let stuck = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            call(mute, "c", Op::Hops),
        )
        .await
        .expect("the call never came back: ipc::call has no deadline");
        let why = stuck.expect_err("a serve that never answered is not an answer");
        assert!(
            why.contains(Op::Hops.name()),
            "it names the op that ran out of time: {why}"
        );
        let next = call(live, "c", Op::Hops).await;
        assert!(
            matches!(next, Ok(Reply::Hops { .. })),
            "the next call is not held by the stuck one: {next:?}"
        );
    }

    #[tokio::test]
    async fn a_refused_old_port_refreshes_before_sending_exactly_once() {
        let live = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let live_port = live.local_addr().unwrap().port();
        let old = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let old_port = old.local_addr().unwrap().port();
        drop(old); // Fault injection: endpoint.json still names the closed listener.
        let server = tokio::spawn(async move {
            let (mut socket, _) = live.accept().await.unwrap();
            let mut bytes = Vec::new();
            socket.read_to_end(&mut bytes).await.unwrap();
            let request: Handoff = proto::decode(&bytes).unwrap();
            assert_eq!(request.cookie, "fresh-cookie");
            assert!(matches!(request.op, Op::Hops));
            socket
                .write_all(&proto::encode(&Reply::Hops { by_device: vec![] }).unwrap())
                .await
                .unwrap();
            socket.shutdown().await.unwrap();
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(80), live.accept())
                    .await
                    .is_err()
            );
        });
        let mut refreshes = 0;
        let reply = call_refreshing(old_port, "old-cookie", Op::Hops, || {
            refreshes += 1;
            Some((live_port, "fresh-cookie".into()))
        })
        .await;
        assert!(matches!(reply, Ok(Reply::Hops { .. })), "{reply:?}");
        assert_eq!(refreshes, 1);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn a_lost_reply_never_replays_an_operation() {
        let live = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = live.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut socket, _) = live.accept().await.unwrap();
            let mut bytes = Vec::new();
            socket.read_to_end(&mut bytes).await.unwrap();
            let request: Handoff = proto::decode(&bytes).unwrap();
            assert!(matches!(request.op, Op::Hops));
            drop(socket); // The operation ran, but its answer disappeared.
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(80), live.accept())
                    .await
                    .is_err()
            );
        });
        let mut refreshes = 0;
        let reply = call_refreshing(port, "cookie", Op::Hops, || {
            refreshes += 1;
            Some((port, "cookie".into()))
        })
        .await;
        assert!(reply.is_err());
        assert_eq!(
            refreshes, 0,
            "no endpoint refresh after any request bytes went out"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn a_refused_live_pid_has_a_bounded_connect_retry() {
        let dead = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = dead.local_addr().unwrap().port();
        drop(dead);
        let began = tokio::time::Instant::now();
        let reply = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            call_refreshing(port, "cookie", Op::Hops, || Some((port, "cookie".into()))),
        )
        .await
        .expect("connect retry escaped its budget");
        assert!(reply.is_err());
        assert!(began.elapsed() < std::time::Duration::from_secs(2));
    }

    /// Every tier is a real deadline and the in-process one is the
    /// shortest: a face waits on those.
    #[test]
    fn the_in_process_tier_is_the_shortest() {
        assert!(
            Op::Hops.deadline()
                <= Op::Ls {
                    machine: String::new(),
                    path: String::new()
                }
                .deadline()
        );
    }
}
