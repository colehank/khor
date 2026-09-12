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
    Pair { ticket: String },
    SyncNow,
    Accept { session: String },
    /// List a machine's directory (`proto::Request::Ls` carried to the
    /// resident endpoint). Tail-appended, like everything after v1.
    Ls { machine: String, path: String },
    /// Pull a pathed file whole (the slice loop runs in the serve, like
    /// Accept's does). `dir` is where it lands. Tail-appended.
    Pull { machine: String, path: String, dir: String },
    /// Start a borrow of a machine's network (docs/NET.md 借网): the serve
    /// dials the tunnel on its own endpoint, binds a local proxy port, and
    /// registers a borrow session — all of which must happen in the
    /// process that holds the key, never in the verb. Tail-appended.
    Borrow { machine: String },
    /// Open a session on another machine (`proto::Request::Open`).
    /// Tail-appended.
    OpenOn { machine: String, kind: String, title: String, cwd: String, cmd: Vec<String>, cols: u16, rows: u16 },
    /// Where another machine's session listens, and a tunnel to it —
    /// the serve is the only process holding the endpoint, so the pipe
    /// is bound here and the verb is handed a local address the way a
    /// borrow hands one. Tail-appended.
    ReachOn { machine: String, session: String },
    /// End a session on another machine (`Act` "close"). Tail-appended.
    CloseOn { machine: String, session: String },
    /// Which road this machine is using to reach each known device
    /// (`khor_core::Hop`). **Only the resident can answer**: the reading
    /// belongs to the live endpoint's own view of its remotes, and a
    /// one-shot verb that bound its own endpoint would have talked to
    /// nobody and report "nothing flowing" for the whole fleet.
    /// Tail-appended.
    Hops,
    /// Tail-appended: [`crate::proto::Request::OpenAgent`] through the
    /// resident — an agent session, by name, on that machine. Resolved
    /// over there; this side only carries the wizard's answers.
    OpenAgentOn { machine: String, dir: String, title: String, form: String, agent: String },
    /// Tail-appended: [`crate::proto::Request::Places`] through the
    /// resident — 常去的地方 on that machine.
    PlacesOn { machine: String },
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
    BorrowUnlisted { machine: String },
    /// Tail-appended: ends an unlisted borrow by the handle
    /// [`Reply::Borrowing`] answered with. The listed kind is ended by
    /// closing its row instead, which is the row's whole job.
    ///
    /// **Dropping one already gone answers the same as dropping a live
    /// one.** The caller wanted it not to exist; a refusal would only
    /// tell it about a race it cannot do anything about.
    DropBorrow { session: String },
    /// Tail-appended: [`crate::proto::Request::Workspaces`] through
    /// the resident — the checkouts that machine has been worked in.
    WorkspacesOn { machine: String },
    /// Tail-appended: one slice of a file, for looking at rather than
    /// taking (`Node::read_head`). Distinct from [`Op::Pull`], which
    /// loops every slice onto this disk; this one answers once and the
    /// bytes never touch a file.
    ReadHead { machine: String, path: String },
    /// Tail-appended: [`crate::proto::Request::GitStatus`] through the
    /// resident — what differs from HEAD across one checkout.
    GitStatusOn { machine: String, root: String },
    /// Tail-appended: [`crate::proto::Request::GitDiff`] through the
    /// resident — where one file differs from HEAD.
    GitDiffOn { machine: String, path: String },
    /// Tail-appended: [`crate::proto::Request::WritePath`] through the
    /// resident — saving a file back to whichever machine holds it.
    WritePathOn { machine: String, path: String, was: String, bytes: serde_bytes::ByteBuf },
    /// Tail-appended: whether that machine has VS Code.
    CodeHereOn { machine: String },
    /// Tail-appended: start (or reuse) VS Code's web face over there and
    /// **stand a local pipe in front of it** — the reply is an address
    /// on *this* machine, because a loopback port on somebody else's is
    /// not something a browser here can point at.
    CodeOpenOn { machine: String, root: String },
    /// Tail-appended: stop it, and drop this machine's pipe with it.
    CodeStopOn { machine: String, root: String },
    /// Tail-appended: one slice of a file **at an offset**
    /// (工作区批6). [`Op::ReadHead`] is this with the offset fixed at
    /// zero; it stays because it shipped, and because「the first slice」
    /// is what a text view wants and it should not have to say so.
    ///
    /// **The loop that uses this lives in the asker**, not here: one
    /// reply is one slice, and a whole 4 MB image answered in one frame
    /// would be eight times `proto::MAX_FRAME`.
    ReadAt { machine: String, path: String, offset: u64 },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Reply {
    Paired { name: String },
    Synced { outcomes: Vec<(String, Result<String, String>)> },
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
    Refused { why: String },
    /// Tail-appended with [`Op::Ls`].
    Dir { path: String, entries: Vec<crate::proto::DirEntry>, truncated: bool },
    /// Tail-appended with [`Op::Pull`]: bytes moved, and where the file
    /// landed — the verb prints the landing because the default (the
    /// asker's cwd) is a place the serve never knew.
    Pulled { moved: u64, dest: String },
    /// Tail-appended with [`Op::Borrow`]: the borrow session's id and the
    /// local address its proxy listens on — the verb prints the address
    /// because the caller points a browser at it.
    Borrowing { session: String, addr: String },
    /// Tail-appended with [`Op::OpenOn`]: the id the session got **on
    /// that machine**, which is the id every face will call it by.
    OpenedOn { session: String },
    /// Tail-appended with [`Op::ReachOn`]: a local address that pipes
    /// to that session's host, and the cookie its handshake wants. The
    /// address is this machine's; what is on the other end of it is the
    /// far host's socket (`tunnel`).
    Reaching { addr: String, cookie: String },
    /// Tail-appended with [`Op::Hops`]: one reading per device id.
    ///
    /// A list rather than a map because it crosses msgpack, and every
    /// device present in the table is present here — a machine missing
    /// from this list would be read as *absent*, and the caller has no
    /// way to tell that from *no reading*, which is precisely the
    /// distinction `Hop` exists to keep (`Hop::Unknown`).
    Hops { by_device: Vec<(String, khor_core::Hop)> },
    /// Tail-appended with [`Op::PlacesOn`]: `(path, shown, sessions,
    /// last_ms)` per place, as that machine answered.
    Places { places: Vec<(String, String, u32, i64)> },
    /// Tail-appended with [`Op::LoadOn`]: the machine's top and CPU
    /// day, as it answered.
    Load { load: khor_core::Load },
    /// Tail-appended with [`Op::WorkspacesOn`]: the same four fields
    /// as [`Reply::Places`], filtered to checkouts over there.
    Workspaces { workspaces: Vec<(String, String, u32, i64)> },
    /// Tail-appended with [`Op::ReadHead`]: the file's whole size, its
    /// mtime, and as much of it as one slice holds. `bytes.len() <
    /// total` is the only thing that says it was cut.
    Head { total: u64, at_ms: u64, bytes: serde_bytes::ByteBuf },
    /// Tail-appended with [`Op::GitStatusOn`]: the branch, the marked
    /// paths, and whether the list was cut.
    GitStatus { branch: String, rows: Vec<(String, String)>, more: bool },
    /// Tail-appended with [`Op::GitDiffOn`]: one file's hunks, in file
    /// order, and whether the list was cut.
    GitDiff { hunks: Vec<crate::proto::Hunk>, more: bool },
    /// Tail-appended with [`Op::WritePathOn`]: the mtime and digest the
    /// file has now.
    Wrote { at_ms: u64, digest: String },
    /// Tail-appended with [`Op::CodeHereOn`]: empty means no editor.
    CodeHere { version: String },
    /// Tail-appended with [`Op::CodeOpenOn`]: where to point a browser
    /// **on this machine**, and the token the far server wants.
    CodeAt { addr: String, token: String },
}

/// One verb, one connection: write the frame, half-close, read the reply.
pub async fn call(port: u16, cookie: &str, op: Op) -> Result<Reply, String> {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(msg::serve_unreachable)?;
    let frame = proto::encode(&Handoff { cookie: cookie.to_owned(), op })?;
    stream
        .write_all(&frame)
        .await
        .map_err(msg::handoff_failed)?;
    stream
        .shutdown()
        .await
        .map_err(msg::cant_close_stream)?;
    let mut bytes = Vec::new();
    (&mut stream)
        .take(MAX_FRAME as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(msg::no_reply_from_serve)?;
    proto::decode(&bytes)
}
