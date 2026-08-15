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
use crate::Node;

/// How often the serve loop syncs with everyone.
const SYNC_EVERY: Duration = Duration::from_secs(5);
/// Per-device budget for one sync visit; the far side may simply be off.
const DIAL_TIMEOUT: Duration = Duration::from_secs(10);

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// What one-shot CLI verbs need to know about the live endpoint.
#[derive(serde::Serialize, serde::Deserialize)]
struct EndpointFile {
    id: String,
    addrs: Vec<String>,
    pid: u32,
}

impl Node {
    /// Runs this device's listening half: answers connections, syncs with
    /// every known device every few seconds. Writes `endpoint.json` so
    /// one-shot verbs (`invite`) can see the live endpoint.
    pub async fn serve(self) -> Result<(), String> {
        let ep = endpoint::bind(self.secret_key().clone())
            .await
            .map_err(|e| e.to_string())?;
        let addrs = endpoint::local_addrs(&ep);
        // Own dialing hints go into the table so the snapshot handed to a
        // pairing device already says how to reach us.
        {
            let loaded = self.devices_loaded()?;
            loaded.doc.upsert(self.device_str(), self.name(), &addrs)?;
            let mut store = loaded.store;
            store.flush(&loaded.doc)?;
        }
        let file = EndpointFile { id: self.device_str().to_owned(), addrs, pid: std::process::id() };
        fs::write(
            self.root().join(".khor").join("endpoint.json"),
            serde_json::to_vec(&file).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("写不了 endpoint.json: {e}"))?;

        // One task per connection: a client that vanishes without
        // closing must not block the accept loop until QUIC times out.
        let node = std::sync::Arc::new(self);
        let mut ticker = tokio::time::interval(SYNC_EVERY);
        loop {
            tokio::select! {
                incoming = ep.accept() => {
                    let Some(incoming) = incoming else { break };
                    let n = node.clone();
                    tokio::spawn(async move {
                        if let Ok(conn) = incoming.await {
                            let _ = n.handle(conn).await;
                        }
                    });
                }
                _ = ticker.tick() => {
                    let _ = node.sync_with_all(&ep).await;
                }
            }
        }
        Ok(())
    }

    async fn handle(&self, conn: iroh::endpoint::Connection) -> Result<(), String> {
        let remote = conn.remote_id().to_string();
        while let Ok((mut send, mut recv)) = conn.accept_bi().await {
            let bytes = recv
                .read_to_end(MAX_FRAME)
                .await
                .map_err(|e| format!("读不完请求: {e}"))?;
            let resp = match proto::decode::<Request>(&bytes) {
                Ok(req) => self.dispatch(&remote, req),
                Err(e) => Response::Refused { why: e },
            };
            send.write_all(&proto::encode(&resp)?)
                .await
                .map_err(|e| format!("写不出应答: {e}"))?;
            send.finish().map_err(|e| format!("收不了尾: {e}"))?;
        }
        Ok(())
    }

    fn dispatch(&self, remote: &str, req: Request) -> Response {
        match self.dispatch_inner(remote, req) {
            Ok(resp) => resp,
            Err(why) => Response::Refused { why },
        }
    }

    fn dispatch_inner(&self, remote: &str, req: Request) -> Result<Response, String> {
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
    /// gets its real name. Returns bytes actually moved this run.
    pub async fn accept(&self, id: &crate::SessionId) -> Result<u64, String> {
        use crate::transfer::{partial_path, payload_path, pulling_marker};
        let Some(msg_id) = id.0.strip_prefix("transfer/") else {
            return Err(format!("没有这个传输: {}", id.0));
        };
        self.refuse_if_serving()?;
        let (channel, m) = self.find_transfer(msg_id)?;
        if channel != self.name() {
            return Err(format!("这份文件是发给 {channel} 的,在那台机器上收"));
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

        let ep = endpoint::bind(self.secret_key().clone())
            .await
            .map_err(|e| e.to_string())?;
        let outcome = async {
            let addr = endpoint::dial_addr(&home.id, &home.addrs, &[]).map_err(|e| e.to_string())?;
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
        ep.close().await;
        if outcome.is_ok() {
            self.emit_transfer_row(&channel, &m)?;
        }
        outcome
    }

    /// Creates a one-time pairing ticket. Needs `khor serve` running —
    /// the ticket carries the live endpoint's addresses.
    pub fn invite(&self) -> Result<String, String> {
        let file = self.endpoint_file()?.ok_or(
            "khor serve 没在跑——先在这台机器起 khor serve,票里要带它的地址",
        )?;
        let mut raw = [0u8; 16];
        getrandom::fill(&mut raw).map_err(|e| format!("取不到随机数: {e}"))?;
        let token: String = raw.iter().map(|b| format!("{b:02x}")).collect();
        let dir = self.root().join(".khor").join("invites");
        fs::create_dir_all(&dir).map_err(|e| format!("建不了邀请目录: {e}"))?;
        fs::write(self.invite_path(&token)?, b"").map_err(|e| format!("存不了配对码: {e}"))?;
        Ticket {
            id: self.device_str().to_owned(),
            name: self.name().to_owned(),
            direct: file.addrs,
            relays: vec![],
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
    /// machines — pairing has no direction (docs/NET.md).
    pub async fn pair(&self, ticket: &str) -> Result<String, String> {
        let t = Ticket::decode(ticket).map_err(|e| e.to_string())?;
        self.refuse_if_serving()?;
        let ep = endpoint::bind(self.secret_key().clone())
            .await
            .map_err(|e| e.to_string())?;
        let addr = endpoint::dial_addr(&t.id, &t.direct, &t.relays).map_err(|e| e.to_string())?;
        let conn = tokio::time::timeout(DIAL_TIMEOUT, ep.connect(addr, ALPN))
            .await
            .map_err(|_| "连不上出票的那台机器(超时)".to_string())?
            .map_err(|e| format!("连不上出票的那台机器: {e}"))?;
        let resp = request(
            &conn,
            &Request::Pair {
                token: t.token,
                name: self.name().to_owned(),
                // Not serving, so any address we report dies with this
                // process — better none than stale.
                addrs: vec![],
            },
        )
        .await?;
        // Close before returning on every path: a dropped-but-unclosed
        // endpoint leaves the far side's stream wait dangling until QUIC
        // times out.
        let outcome = match resp {
            Response::Paired { name, devices } => (|| {
                let bytes = B64
                    .decode(devices)
                    .map_err(|e| format!("对面的设备表不是 base64: {e}"))?;
                let loaded = self.devices_loaded()?;
                loaded.doc.merge(&bytes)?;
                let mut store = loaded.store;
                store.flush(&loaded.doc)?;
                Ok(name)
            })(),
            Response::Refused { why } => Err(why),
            other => Err(format!("对面答非所问: {other:?}")),
        };
        ep.close().await;
        outcome
    }

    /// One sync visit to every known device, now. Returns per-device
    /// outcomes — "moved nothing" and "failed" wear different faces.
    pub async fn sync_now(&self) -> Result<Vec<(String, Result<String, String>)>, String> {
        self.refuse_if_serving()?;
        let ep = endpoint::bind(self.secret_key().clone())
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
        let addr = endpoint::dial_addr(&d.id, &d.addrs, &[]).map_err(|e| e.to_string())?;
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

    /// One live endpoint per key: dialing while `khor serve` holds the
    /// key would knock both off (learned in production).
    fn refuse_if_serving(&self) -> Result<(), String> {
        match self.endpoint_file()? {
            Some(f) if f.pid != std::process::id() => Err(format!(
                "khor serve 正在跑(pid {}),它每 {} 秒同步一次;同一把钥匙同时只能有一个端点",
                f.pid,
                SYNC_EVERY.as_secs()
            )),
            _ => Ok(()),
        }
    }
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
