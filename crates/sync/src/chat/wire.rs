//! Encoding for the live path: version vectors and increments as strings
//! a request/response frame can carry. When to send, to whom, and how to
//! retry live elsewhere.
//!
//! This is also the loro type boundary: callers get `String`s and never
//! import loro, so a loro version bump cannot leak into the wire shape.
//!
//! base64 rather than raw bytes because the payload rides inside JSON,
//! where raw bytes become a number array: +200% and unreadable in `--json`
//! output, versus +33% for base64.

use base64::Engine;
use loro::VersionVector;

use super::doc::ChatDoc;

/// Raw bytes (pre-base64) one sync may carry per direction. The control
/// stream is serial — a huge frame blocks keys and events queued behind
/// it — so this path carries increments only; a full history goes over
/// the file path, which has no per-frame cap. 256 KiB ≈ tens of thousands
/// of text messages; the number is tunable, the criterion is not.
pub const MAX_BYTES: usize = 256 * 1024;

const B64: base64::engine::general_purpose::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Where I am; the far side computes what to send me from it.
///
/// Not capped by [`MAX_BYTES`]: it grows with peers ever used (tens of KB
/// at worst) and refusing it kills the whole path — the cap is for
/// content, not metadata.
pub fn version_b64(doc: &ChatDoc) -> String {
    B64.encode(doc.version().encode())
}

/// What I have beyond the far side's reported version. `theirs` empty =
/// from the beginning — a freshly paired device's situation, and the trip
/// most likely to hit the cap.
///
/// Returns the **empty string** when there is nothing new, judged by
/// version-vector inclusion — loro exports a non-empty header block even
/// with zero new ops, so an emptiness check would keep the wire forever
/// "moving" and idempotence tests forever green for the wrong reason.
pub fn changes_since_b64(doc: &ChatDoc, theirs: &str) -> Result<String, String> {
    let vv = decode_version(theirs)?;
    if vv.includes_vv(&doc.version()) {
        return Ok(String::new());
    }
    let bytes = doc.changes_since(&vv)?;
    guard(bytes.len(), "要发出去的")?;
    Ok(B64.encode(bytes))
}

/// Merges what the far side sent. Empty = nothing for me, not an error.
/// Idempotent: the same segment twice equals once.
pub fn merge_b64(doc: &ChatDoc, changes: &str) -> Result<(), String> {
    if changes.is_empty() {
        return Ok(());
    }
    let bytes = B64
        .decode(changes)
        .map_err(|e| format!("这段增量不是 base64: {e}"))?;
    guard(bytes.len(), "收到的")?;
    doc.merge(&bytes)
}

/// base64 → version vector. Empty = empty vector = from the beginning.
fn decode_version(b64: &str) -> Result<VersionVector, String> {
    if b64.is_empty() {
        return Ok(VersionVector::default());
    }
    let bytes = B64
        .decode(b64)
        .map_err(|e| format!("这个版本号不是 base64: {e}"))?;
    VersionVector::decode(&bytes).map_err(|e| format!("解不出版本号: {e}"))
}

/// Reply fields, defined here so no caller re-parses JSON and quietly
/// forgets `version`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reply {
    /// Where the far side is after merging my segment; the next round's
    /// `changes` is computed from it.
    pub version: String,
    /// The segment I lack.
    pub changes: String,
    /// Far side's message count after merging. For humans.
    pub messages: usize,
}

/// One round's outcome, for logs and acceptance: "synced but nothing
/// moved" and "sync failed" must wear different faces (docs/UX.md), and
/// neither is an error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Round {
    /// Raw bytes pushed (pre-base64). 0 = they already had everything.
    pub pushed: usize,
    /// Raw bytes pulled.
    pub pulled: usize,
    /// My message count after merging.
    pub messages: usize,
}

/// Sync state toward one far side. Memory-only, deliberately: losing it
/// costs one extra round trip (the next first round degrades to
/// pull-only), while a *stale* persisted version would under-send — and
/// that is silent message loss. Convergence lives in the CRDT; this only
/// saves a round trip, not bytes (the amnesiac's second round computes
/// the same delta — `remembering_saves_a_round_trip_not_bytes`).
#[derive(Debug, Clone, Default)]
pub struct Peer {
    /// The `version` from the last reply. Empty = never met.
    their: String,
}

impl Peer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Where the far side is (base64). Empty = never met.
    pub fn their_version(&self) -> &str {
        &self.their
    }

    /// What to send this round. Touches no IO, so both sync and async
    /// transports can use it.
    ///
    /// The first round is pull-only: the delta needs the far side's
    /// version, which we don't have yet. Cost: my words arrive one round
    /// late, bounded at one; pushing everything instead can hit
    /// [`MAX_BYTES`], unbounded.
    pub fn outgoing(&self, doc: &ChatDoc) -> Result<Outgoing, String> {
        Ok(Outgoing {
            have: version_b64(doc),
            // An empty `their` means "from the beginning" to
            // changes_since_b64, so the first round must explicitly not
            // push — otherwise it pushes the full history.
            changes: if self.their.is_empty() {
                String::new()
            } else {
                changes_since_b64(doc, &self.their)?
            },
        })
    }

    /// Takes in a reply: merge, flush, then record their version. The
    /// order is load-bearing twice over. Flush immediately, because
    /// merged-but-unflushed data is gone on restart while the version
    /// vector remembers receiving it — it would never be re-pulled.
    /// Record `their` last, because any earlier `?` then leaves it
    /// untouched and the next round recomputes from the last known good
    /// version; recording first would mark bytes delivered that a failed
    /// flush lost, and they would never be pushed again.
    pub fn absorb(
        &mut self,
        store: &mut super::store::ChatStore,
        doc: &ChatDoc,
        sent: &Outgoing,
        reply: Reply,
    ) -> Result<Round, String> {
        merge_b64(doc, &reply.changes)?;
        store.flush(doc)?;
        self.their = reply.version;
        Ok(Round {
            pushed: raw_len(&sent.changes),
            pulled: raw_len(&reply.changes),
            messages: doc.messages().len(),
        })
    }

    /// [`Self::outgoing`] + `send` + [`Self::absorb`] for synchronous
    /// transports. Async drivers use the two methods directly — `send` is
    /// a sync closure, an await cannot enter it. Extracted so the
    /// sequence exists once instead of being copied into every driver.
    pub fn round<F>(
        &mut self,
        store: &mut super::store::ChatStore,
        doc: &ChatDoc,
        send: F,
    ) -> Result<Round, String>
    where
        F: FnOnce(&str, &str) -> Result<Reply, String>,
    {
        let out = self.outgoing(doc)?;
        let reply = send(&out.have, &out.changes)?;
        self.absorb(store, doc, &out, reply)
    }
}

/// The two strings one round sends.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outgoing {
    /// Where I am.
    pub have: String,
    /// What I have beyond them. Empty = this round is pull-only.
    pub changes: String,
}

impl Outgoing {
    /// `false` means my words haven't left yet — drivers use it to
    /// schedule an immediate extra round instead of waiting a full idle
    /// cycle.
    pub fn pushes(&self) -> bool {
        !self.changes.is_empty()
    }
}

/// Decoded size of a base64 string, without decoding: the number only
/// feeds logs and assertions.
fn raw_len(b64: &str) -> usize {
    b64.len() / 4 * 3
}

/// The over-limit error, naming both numbers: "too big" alone doesn't
/// tell a near miss from 10×, and the next step differs.
fn guard(len: usize, which: &str) -> Result<(), String> {
    if len <= MAX_BYTES {
        return Ok(());
    }
    Err(format!(
        "{which}这一段有 {} KiB,超过配对通道单帧上限 {} KiB。\
         控制流是串行的,这条路只运增量——整段历史请走 ssh 那条(按文件搬,没有单帧上限)",
        len / 1024,
        MAX_BYTES / 1024
    ))
}
