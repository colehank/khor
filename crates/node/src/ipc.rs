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
