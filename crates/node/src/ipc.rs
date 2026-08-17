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
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Reply {
    Paired { name: String },
    Synced { outcomes: Vec<(String, Result<String, String>)> },
    Accepted { moved: u64 },
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
