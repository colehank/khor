//! The live link: serve loop, pairing, and sync rounds over iroh.
//!
//! Trust model (docs/NET.md): joining is the only gate. `Pair` is
//! answered for whoever holds an unburned token; everything else is
//! answered only for devices already in the table.

use std::collections::BTreeSet;
use std::fs;
use std::time::Duration;

use base64::Engine;
use khor_net::endpoint::{self, Ticket, ALPN};
use khor_sync::devices::DeviceInfo;
use khor_sync::store::Doc;
use khor_sync::{chat, wire};

use crate::proto::{self, Request, Response, MAX_FRAME};
use crate::{ipc, Node};

/// How often the serve loop syncs with everyone.
const SYNC_EVERY: Duration = Duration::from_secs(5);
/// Per-device budget for one sync visit; the far side may simply be off.
const DIAL_TIMEOUT: Duration = Duration::from_secs(10);

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

impl Node {
    /// Runs this device's listening half: answers connections, executes
    /// handed-off one-shot verbs, syncs with every known device every few
    /// seconds. Writes `endpoint.json` so one-shot verbs can find both
    /// the live endpoint and the hand-off port.
    pub async fn serve(self) -> Result<(), String> {
        let ep = std::sync::Arc::new(
            endpoint::bind(self.secret_key().clone(), self.relays())
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
            .map_err(|e| format!("开不了递话口: {e}"))?;
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

        // One task per connection: a client that vanishes without
        // closing must not block the accept loop until QUIC times out.
        let node = std::sync::Arc::new(self);
        let mut ticker = tokio::time::interval(SYNC_EVERY);
        loop {
            tokio::select! {
                incoming = ep.accept() => {
                    let Some(incoming) = incoming else { break };
                    let n = node.clone();
                    let e = ep.clone();
                    tokio::spawn(async move {
                        if let Ok(conn) = incoming.await {
                            let _ = n.handle(conn, &e).await;
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
                    // Off the accept loop: an unreachable device stalls a
                    // visit for DIAL_TIMEOUT, and a serve that goes deaf
                    // for that long fails everyone else. Skip when the
                    // previous pump still runs instead of piling up.
                    let n = node.clone();
                    let e = ep.clone();
                    tokio::spawn(async move {
                        let Ok(_g) = n.sync_gate.try_lock() else { return };
                        let _ = n.sync_with_all(&e).await;
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
            .map_err(|e| format!("读不完递话: {e}"))?;
        let reply = match proto::decode::<ipc::Handoff>(&bytes) {
            Ok(h) if h.cookie == cookie => self.run_handoff(ep, h.op).await,
            Ok(_) => ipc::Reply::Refused { why: "递话的暗号对不上,重读 endpoint.json 再来".into() },
            Err(e) => ipc::Reply::Refused { why: e },
        };
        stream
            .write_all(&proto::encode(&reply)?)
            .await
            .map_err(|e| format!("答不回去: {e}"))?;
        stream.shutdown().await.map_err(|e| format!("收不了尾: {e}"))?;
        Ok(())
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
            ipc::Op::SyncNow => {
                let _g = self.sync_gate.lock().await;
                ipc::Reply::Synced { outcomes: self.sync_with_all(ep).await }
            }
            ipc::Op::Accept { session } => {
                match self.accept_with(ep, &crate::SessionId(session)).await {
                    Ok(moved) => ipc::Reply::Accepted { moved },
                    Err(why) => ipc::Reply::Refused { why },
                }
            }
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
            return Some(Err(format!(
                "khor serve 在跑(pid {})但没开递话口,把它升级或先停掉",
                f.pid
            )));
        }
        Some(
            ipc::call(f.ipc_port, &f.ipc_cookie, op)
                .await
                .map_err(|e| format!("khor serve 在跑(pid {})但递不过去: {e}", f.pid)),
        )
    }

    async fn handle(&self, conn: iroh::endpoint::Connection, ep: &iroh::Endpoint) -> Result<(), String> {
        let remote = conn.remote_id().to_string();
        while let Ok((mut send, mut recv)) = conn.accept_bi().await {
            let bytes = recv
                .read_to_end(MAX_FRAME)
                .await
                .map_err(|e| format!("读不完请求: {e}"))?;
            let resp = match proto::decode::<Request>(&bytes) {
                Ok(req) => self.dispatch(&remote, req, ep).await,
                Err(e) => Response::Refused { why: e },
            };
            send.write_all(&proto::encode(&resp)?)
                .await
                .map_err(|e| format!("写不出应答: {e}"))?;
            send.finish().map_err(|e| format!("收不了尾: {e}"))?;
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
                    return Err("配对码不对,或已经被用过了".into());
                }
                // Burn before use: a token that pairs twice is a door
                // that never closes.
                fs::remove_file(&path).map_err(|e| format!("销不掉配对码: {e}"))?;
                let loaded = self.devices_loaded()?;
                loaded.doc.upsert(remote, &name, &addrs)?;
                let mut store = loaded.store;
                store.flush(&loaded.doc)?;
                Ok(Response::Paired {
                    name: self.name().to_owned(),
                    devices: B64.encode(loaded.doc.snapshot()?),
                })
            }
            Request::Sync { doc, have, changes } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err("这台设备不在设备表里,先配对".into());
                }
                let reply = if doc == "devices" {
                    let mut loaded = self.devices_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if doc == "seen" {
                    let mut loaded = self.seen_loaded()?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else if let Some(ch) = doc.strip_prefix("chat/") {
                    let dir = chat::channel_dir(self.root(), ch)
                        .ok_or_else(|| format!("频道名不合法: {ch:?}"))?;
                    let mut loaded = chat::open_channel(&dir, self.writer_peer())?;
                    wire::answer(&mut loaded.store, &loaded.doc, &have, &changes)?
                } else {
                    return Err(format!("不认识这种 doc: {doc}"));
                };
                Ok(Response::Synced {
                    version: reply.version,
                    changes: reply.changes,
                    items: reply.items as u64,
                })
            }
            Request::Fetch { digest, offset } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err("这台设备不在设备表里,先配对".into());
                }
                self.serve_slice(&digest, offset)
            }
            Request::Sessions => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err("这台设备不在设备表里,先配对".into());
                }
                Ok(Response::SessionRows { rows: self.reportable_rows()? })
            }
            Request::Act { session, action } => {
                if self.devices_loaded()?.doc.get(remote).is_none() {
                    return Err("这台设备不在设备表里,先配对".into());
                }
                match action.as_str() {
                    "accept" => {
                        let moved = self.accept_local(ep, &crate::SessionId(session)).await?;
                        Ok(Response::Acted { moved })
                    }
                    other => Err(format!("不认识的动作: {other}")),
                }
            }
        }
    }

    /// One slice of an offered payload. The offer's recorded size is the
    /// contract: a file that changed size since the offer is refused, not
    /// silently served (the digest would fail far away, much later).
    fn serve_slice(&self, digest: &str, offset: u64) -> Result<Response, String> {
        use std::io::{Read, Seek, SeekFrom};
        let mut offer = crate::transfer::load_offer(self.root(), digest)?
            .ok_or("没有这份文件的记录,可能出让方换了机器或删了它")?;
        let meta = fs::metadata(&offer.path)
            .map_err(|e| format!("出让的文件读不到了({}): {e}", offer.path.display()))?;
        if meta.len() != offer.size {
            return Err("出让的文件被动过(大小变了),让对方重新发一次".into());
        }
        if offset > offer.size {
            return Err(format!("起点越界: {offset} > {}", offer.size));
        }
        let mut f = fs::File::open(&offer.path)
            .map_err(|e| format!("出让的文件打不开: {e}"))?;
        f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        let want = (offer.size - offset).min(proto::SLICE) as usize;
        let mut buf = vec![0u8; want];
        f.read_exact(&mut buf).map_err(|e| format!("读不完这一片: {e}"))?;
        offer.started = true;
        offer.done = offset + want as u64 >= offer.size;
        crate::transfer::save_offer(self.root(), digest, &offer)?;
        Ok(Response::Slice { total: offer.size, bytes: serde_bytes::ByteBuf::from(buf) })
    }

    /// Approves a transfer and pulls its payload from home. Resumes from
    /// an existing partial; verifies the blake3 digest before the payload
    /// gets its real name. Returns bytes actually moved this run. Routed
    /// through the resident serve when one holds the key.
    pub async fn accept(&self, id: &crate::SessionId) -> Result<u64, String> {
        if let Some(reply) = self.via_serve(ipc::Op::Accept { session: id.0.clone() }).await {
            return match reply? {
                ipc::Reply::Accepted { moved } => Ok(moved),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(format!("serve 答非所问: {other:?}")),
            };
        }
        let ep = endpoint::bind(self.secret_key().clone(), self.relays())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = self.accept_with(&ep, id).await;
        ep.close().await;
        outcome
    }

    /// Accept on an endpoint the caller owns: pulls locally when this
    /// machine is the recipient, otherwise routes the action to the
    /// recipient's serve — 动作从哪台设备发都行 (docs/SESSION.md).
    async fn accept_with(&self, ep: &iroh::Endpoint, id: &crate::SessionId) -> Result<u64, String> {
        let Some(msg_id) = id.0.strip_prefix("transfer/") else {
            return Err(format!("没有这个传输: {}", id.0));
        };
        let (channel, _) = self.find_transfer(msg_id)?;
        if channel == self.name() {
            return self.accept_local(ep, id).await;
        }
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| format!("收文件的机器不在设备表里: {channel}"))?;
        let addr = endpoint::dial_addr(&target.id, &target.addrs, self.relays())
            .map_err(|e| e.to_string())?;
        let conn = tokio::time::timeout(DIAL_TIMEOUT, ep.connect(addr, ALPN))
            .await
            .map_err(|_| format!("连不上 {channel}(超时)——收下由那台机器执行,它得在线"))?
            .map_err(|e| format!("连不上 {channel}: {e}"))?;
        let resp = request(
            &conn,
            &Request::Act { session: id.0.clone(), action: "accept".into() },
        )
        .await?;
        match resp {
            Response::Acted { moved } => Ok(moved),
            Response::Refused { why } => Err(why),
            other => Err(format!("对面答非所问: {other:?}")),
        }
    }

    /// The pull itself, only ever on the recipient machine. Never
    /// re-routes an incoming Act — what lands wrong is refused, or two
    /// serves could bounce one forever.
    async fn accept_local(&self, ep: &iroh::Endpoint, id: &crate::SessionId) -> Result<u64, String> {
        use crate::transfer::{partial_path, payload_path, pulling_marker};
        let Some(msg_id) = id.0.strip_prefix("transfer/") else {
            return Err(format!("没有这个传输: {}", id.0));
        };
        let (channel, m) = self.find_transfer(msg_id)?;
        if channel != self.name() {
            return Err(format!("这份文件是发给 {channel} 的,这台机器不该收"));
        }
        let crate::MsgBody::Files(files) = &m.body else {
            return Err(format!("没有这个传输: {}", id.0));
        };
        let home = self
            .devices_loaded()?
            .doc
            .get(&m.from.id)
            .ok_or("出让方已不在设备表里")?;
        let dir = chat::channel_dir(self.root(), &channel)
            .ok_or_else(|| format!("频道名不合法: {channel:?}"))?;
        fs::create_dir_all(dir.join("files")).map_err(|e| format!("建不了 files 目录: {e}"))?;

        let outcome = async {
            let addr = endpoint::dial_addr(&home.id, &home.addrs, self.relays())
                .map_err(|e| e.to_string())?;
            let conn = tokio::time::timeout(DIAL_TIMEOUT, ep.connect(addr, ALPN))
                .await
                .map_err(|_| "连不上出让方(超时)".to_string())?
                .map_err(|e| format!("连不上出让方: {e}"))?;
            let mut moved = 0u64;
            for f in files {
                if payload_path(&dir, f).exists() {
                    continue;
                }
                let marker = pulling_marker(&dir, f);
                fs::write(&marker, format!("{}", std::process::id()))
                    .map_err(|e| format!("记不了拉取标记: {e}"))?;
                let pulled = pull_one(&conn, &dir, f).await;
                let _ = fs::remove_file(&marker);
                match pulled {
                    Ok(n) => moved += n,
                    Err(e) => {
                        // A digest mismatch poisons the partial; a broken
                        // link leaves it — it is the resume point.
                        if e.contains("校验") {
                            let _ = fs::remove_file(partial_path(&dir, f));
                        }
                        return Err(e);
                    }
                }
            }
            Ok(moved)
        }
        .await;
        if outcome.is_ok() {
            self.emit_row_of(&crate::transfer::TransferKind::session_id(&m.id))?;
        }
        outcome
    }

    /// Creates a one-time pairing ticket. Needs `khor serve` running —
    /// the ticket carries the live endpoint's addresses.
    pub fn invite(&self) -> Result<String, String> {
        let file = self.endpoint_file()?.ok_or(
            "khor serve 没在跑——先在这台机器起 khor serve,票里要带它的地址",
        )?;
        let token = fresh_hex()?;
        let dir = self.root().join(".khor").join("invites");
        fs::create_dir_all(&dir).map_err(|e| format!("建不了邀请目录: {e}"))?;
        fs::write(self.invite_path(&token)?, b"").map_err(|e| format!("存不了配对码: {e}"))?;
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
            return Err("配对码不对,或已经被用过了".into());
        }
        Ok(self.root().join(".khor").join("invites").join(token))
    }

    /// Accepts a ticket: dial the issuer, burn the token, merge its
    /// device table. When this returns, **both** tables know both
    /// machines — pairing has no direction (docs/NET.md). Routed through
    /// the resident serve when one holds the key.
    pub async fn pair(&self, ticket: &str) -> Result<String, String> {
        if let Some(reply) = self.via_serve(ipc::Op::Pair { ticket: ticket.to_owned() }).await {
            return match reply? {
                ipc::Reply::Paired { name } => Ok(name),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(format!("serve 答非所问: {other:?}")),
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
        let addr = endpoint::dial_addr(&t.id, &t.direct, &relays).map_err(|e| e.to_string())?;
        let conn = tokio::time::timeout(DIAL_TIMEOUT, ep.connect(addr, ALPN))
            .await
            .map_err(|_| "连不上出票的那台机器(超时)".to_string())?
            .map_err(|e| format!("连不上出票的那台机器: {e}"))?;
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
                let bytes = B64
                    .decode(devices)
                    .map_err(|e| format!("对面的设备表不是 base64: {e}"))?;
                let loaded = self.devices_loaded()?;
                loaded.doc.merge(&bytes)?;
                let mut store = loaded.store;
                store.flush(&loaded.doc)?;
                Ok(name)
            }
            Response::Refused { why } => Err(why),
            other => Err(format!("对面答非所问: {other:?}")),
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
                other => Err(format!("serve 答非所问: {other:?}")),
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
            return Err("它没报过任何地址,等它自己来同步".into());
        }
        let addr = endpoint::dial_addr(&d.id, &d.addrs, self.relays()).map_err(|e| e.to_string())?;
        let conn = tokio::time::timeout(DIAL_TIMEOUT, ep.connect(addr, ALPN))
            .await
            .map_err(|_| "连不上(超时)".to_string())?
            .map_err(|e| format!("连不上: {e}"))?;

        let mut moved = 0usize;
        {
            let mut loaded = self.devices_loaded()?;
            moved += self.rounds(&conn, "devices", &mut loaded).await?;
        }
        {
            let mut loaded = self.seen_loaded()?;
            moved += self.rounds(&conn, "seen", &mut loaded).await?;
        }
        for ch in self.known_channels()? {
            let dir = chat::channel_dir(self.root(), &ch)
                .ok_or_else(|| format!("频道名不合法: {ch:?}"))?;
            let mut loaded = chat::open_channel(&dir, self.writer_peer())?;
            moved += self.rounds(&conn, &format!("chat/{ch}"), &mut loaded).await?;
        }
        // Remember what the far side reports about its own sessions:
        // when it goes unreachable, its last word plus the report's age
        // is all the UI may honestly show. Best effort — a failure here
        // only means older information.
        if let Ok(Response::SessionRows { rows }) = request(&conn, &Request::Sessions).await {
            let _ = self.cache_peer_rows(&d.id, &d.name, &rows);
        }
        Ok(if moved == 0 { "无事,已是同步的".into() } else { format!("搬了 {moved} 字节") })
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
                Response::Synced { version, changes, items } => wire::Reply {
                    version,
                    changes,
                    items: items as usize,
                },
                Response::Refused { why } => return Err(why),
                other => return Err(format!("对面答非所问: {other:?}")),
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

    fn endpoint_file(&self) -> Result<Option<EndpointFile>, String> {
        let path = self.root().join(".khor").join("endpoint.json");
        let Ok(text) = fs::read_to_string(&path) else {
            return Ok(None);
        };
        let file: EndpointFile =
            serde_json::from_str(&text).map_err(|e| format!("endpoint.json 读不懂: {e}"))?;
        if pid_alive(file.pid) {
            Ok(Some(file))
        } else {
            Ok(None)
        }
    }

}

/// 16 random bytes as hex — invite tokens, hand-off cookies, session
/// leaves.
pub(crate) fn fresh_hex() -> Result<String, String> {
    let mut raw = [0u8; 16];
    getrandom::fill(&mut raw).map_err(|e| format!("取不到随机数: {e}"))?;
    Ok(raw.iter().map(|b| format!("{b:02x}")).collect())
}

/// Owner-only from the first byte: the hand-off cookie in endpoint.json
/// is a capability, and create-then-chmod would leave a readable window.
pub(crate) fn write_private(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
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
        .map_err(|e| format!("写不了 {}: {e}", path.display()))?;
    f.write_all(bytes).map_err(|e| format!("写不完: {e}"))
}

async fn request(
    conn: &iroh::endpoint::Connection,
    req: &Request,
) -> Result<Response, String> {
    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|e| format!("开不了流: {e}"))?;
    send.write_all(&proto::encode(req)?)
        .await
        .map_err(|e| format!("发不出去: {e}"))?;
    send.finish().map_err(|e| format!("收不了尾: {e}"))?;
    let bytes = recv
        .read_to_end(MAX_FRAME)
        .await
        .map_err(|e| format!("读不到应答: {e}"))?;
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
            let n = existing.read(&mut buf).map_err(|e| format!("读不了断点: {e}"))?;
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
        .map_err(|e| format!("开不了断点文件: {e}"))?;
    let expect = f.size.max(0) as u64;
    let mut moved = 0u64;
    while offset < expect {
        let resp = request(conn, &Request::Fetch { digest: f.digest.clone(), offset }).await?;
        let (total, bytes) = match resp {
            Response::Slice { total, bytes } => (total, bytes),
            Response::Refused { why } => return Err(why),
            other => return Err(format!("对面答非所问: {other:?}")),
        };
        if total != expect {
            return Err(format!("对面报的大小和摘要里的对不上({total} vs {expect})"));
        }
        if bytes.is_empty() {
            return Err("对面送了个空片".into());
        }
        out.write_all(&bytes).map_err(|e| format!("写不下这一片: {e}"))?;
        hasher.update(&bytes);
        offset += bytes.len() as u64;
        moved += bytes.len() as u64;
    }
    out.sync_all().map_err(|e| format!("落不了盘: {e}"))?;
    drop(out);
    let got = hasher.finalize().to_hex().to_string();
    if got != f.digest {
        return Err(format!(
            "内容校验对不上(拉到的 {}… ≠ 摘要说的 {}…)",
            &got[..8],
            f.digest.chars().take(8).collect::<String>()
        ));
    }
    fs::rename(&partial, payload_path(dir, f)).map_err(|e| format!("改不了名: {e}"))?;
    Ok(moved)
}

pub(crate) fn pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
