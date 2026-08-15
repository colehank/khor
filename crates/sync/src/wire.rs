//! Encoding for the live path: version vectors and increments as strings
//! a request/response frame can carry, for any [`Doc`]. When to send, to
//! whom, and how to retry live elsewhere.
//!
//! This is also the loro type boundary: callers get `String`s and never
//! import loro, so a loro version bump cannot leak into the wire shape.
//!
//! base64 rather than raw bytes because the payload rides inside JSON,
//! where raw bytes become a number array: +200% and unreadable in `--json`
//! output, versus +33% for base64.

use base64::Engine;
use khor_catalog::msg;
use loro::VersionVector;

use crate::store::{BlockStore, Doc};

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
pub fn version_b64<D: Doc>(doc: &D) -> String {
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
pub fn changes_since_b64<D: Doc>(doc: &D, theirs: &str) -> Result<String, String> {
    let vv = decode_version(theirs)?;
    if vv.includes_vv(&doc.version()) {
        return Ok(String::new());
    }
    let bytes = doc.changes_since(&vv)?;
    if bytes.len() > MAX_BYTES {
        return Err(msg::outbound_too_big(bytes.len() / 1024, MAX_BYTES / 1024));
    }
    Ok(B64.encode(bytes))
}

/// Merges what the far side sent. Empty = nothing for me, not an error.
/// Idempotent: the same segment twice equals once.
pub fn merge_b64<D: Doc>(doc: &D, changes: &str) -> Result<(), String> {
    if changes.is_empty() {
        return Ok(());
    }
    let bytes = B64
        .decode(changes)
        .map_err(msg::delta_not_base64)?;
    if bytes.len() > MAX_BYTES {
        return Err(msg::inbound_too_big(bytes.len() / 1024, MAX_BYTES / 1024));
    }
    doc.merge(&bytes)
}

/// base64 → version vector. Empty = empty vector = from the beginning.
fn decode_version(b64: &str) -> Result<VersionVector, String> {
    if b64.is_empty() {
        return Ok(VersionVector::default());
    }
    let bytes = B64
        .decode(b64)
        .map_err(msg::version_not_base64)?;
    VersionVector::decode(&bytes).map_err(msg::version_undecodable)
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
    /// Far side's item count after merging. For humans.
    pub items: usize,
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
    /// My item count after merging.
    pub items: usize,
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
    pub fn outgoing<D: Doc>(&self, doc: &D) -> Result<Outgoing, String> {
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
    pub fn absorb<D: Doc>(
        &mut self,
        store: &mut BlockStore,
        doc: &D,
        sent: &Outgoing,
        reply: Reply,
    ) -> Result<Round, String> {
        merge_b64(doc, &reply.changes)?;
        store.flush(doc)?;
        self.their = reply.version;
        Ok(Round {
            pushed: raw_len(&sent.changes),
            pulled: raw_len(&reply.changes),
            items: doc.items(),
        })
    }

    /// [`Self::outgoing`] + `send` + [`Self::absorb`] for synchronous
    /// transports. Async drivers use the two methods directly — `send` is
    /// a sync closure, an await cannot enter it. Extracted so the
    /// sequence exists once instead of being copied into every driver.
    pub fn round<D: Doc, F>(
        &mut self,
        store: &mut BlockStore,
        doc: &D,
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

/// The far side of one sync exchange: given the caller's version and
/// changes, compute what they lack **first**, then merge what they sent.
/// This is the one answering order every server dispatch must use; it
/// lives here so drivers are readers of the order, not re-implementors.
/// The store flushes right after the merge — same reason as
/// [`Peer::absorb`].
pub fn answer<D: Doc>(
    store: &mut BlockStore,
    doc: &D,
    have: &str,
    changes: &str,
) -> Result<Reply, String> {
    let out = changes_since_b64(doc, have)?;
    merge_b64(doc, changes)?;
    store.flush(doc)?;
    Ok(Reply {
        version: version_b64(doc),
        changes: out,
        items: doc.items(),
    })
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::chat::{open_channel, ChatDoc};
    use crate::testutil::{me, render, tmpdir};

    /// An in-memory far side riding [`answer`] — the same order the real
    /// server dispatch uses.
    struct Far {
        doc: ChatDoc,
        store: BlockStore,
        dir: std::path::PathBuf,
    }

    impl Far {
        fn new(tag: &str, peer: u64) -> Self {
            let dir = tmpdir(tag);
            let loaded = open_channel(&dir, peer).unwrap();
            Self { doc: loaded.doc, store: loaded.store, dir }
        }

        fn answer(&mut self, have: &str, changes: &str) -> Result<Reply, String> {
            answer(&mut self.store, &self.doc, have, changes)
        }
    }

    impl Drop for Far {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    /// Convergence, and the push happens on round two — removing the
    /// "first round never pushes" rule makes round one's `pushed`
    /// non-zero and this goes red on the spot.
    #[test]
    fn a_peer_pulls_first_then_pushes() {
        let dir = tmpdir("wire-round");
        let mut mine = open_channel(&dir, 0x51).unwrap();
        mine.doc.tell(&me("a"), "from this side").unwrap();
        mine.store.flush(&mine.doc).unwrap();

        let mut far = Far::new("wire-round-far", 0x52);
        far.doc.tell(&me("b"), "from that side").unwrap();

        let mut peer = Peer::new();
        let r1 = peer.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
        assert_eq!(r1.pushed, 0, "round one must not push (far version unknown yet)");
        assert!(r1.pulled > 0, "round one should pull the far line");
        assert_eq!(r1.items, 2, "two on my side now");
        assert_eq!(far.doc.items(), 1, "the far side has not got mine yet");

        let r2 = peer.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
        assert!(r2.pushed > 0, "round two should push mine");
        assert_eq!(far.doc.items(), 2, "the far side got it");
        assert_eq!(render(&mine.doc), render(&far.doc), "both sides verbatim identical");

        let _ = fs::remove_dir_all(&dir);
    }

    /// Control: a settled pair moves zero bytes in both directions.
    /// Blocks the full-resync-every-round implementation (green above,
    /// quietly hauling the whole history each time), and pins the
    /// version-inclusion emptiness criterion in `changes_since_b64` —
    /// the bytes-empty criterion goes red here because loro always
    /// exports a header block.
    #[test]
    fn a_settled_pair_moves_nothing() {
        let dir = tmpdir("wire-idle");
        let mut mine = open_channel(&dir, 0x61).unwrap();
        mine.doc.tell(&me("a"), "a line").unwrap();
        let mut far = Far::new("wire-idle-far", 0x62);

        let mut peer = Peer::new();
        for _ in 0..3 {
            peer.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
        }
        let r = peer.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
        assert_eq!((r.pushed, r.pulled), (0, 0), "steady state moves nothing either way, got {r:?}");
        assert_eq!(r.items, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    /// Remembering the far side's version saves a round trip — not
    /// bytes. This test replaced a claim of "forgetting resends the
    /// whole history", which it refuted on the spot: the amnesiac's
    /// first pull recovers the version and its second round computes the
    /// identical delta. Kept so the exaggerated version doesn't get
    /// written back and send someone optimizing a cost that doesn't
    /// exist.
    #[test]
    fn remembering_saves_a_round_trip_not_bytes() {
        let dir = tmpdir("wire-amnesia");
        let mut mine = open_channel(&dir, 0x71).unwrap();
        for i in 0..30 {
            mine.doc.tell(&me("a"), &format!("line {i}")).unwrap();
        }
        let mut far = Far::new("wire-amnesia-far", 0x72);

        // The one who remembers: pushes on round two, zero after.
        let mut good = Peer::new();
        good.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
        let push = good.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
        assert!(push.pushed > 0, "round two should push the 30 lines");

        // Say one more; the rememberer delivers in one round.
        mine.doc.tell(&me("a"), "a new line").unwrap();
        let one = good.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
        assert!(one.pushed > 0, "the far version is remembered, so this round pushes at once");
        assert_eq!(far.doc.items(), 31, "delivered in one round");

        // A fresh restart: same one new line — same length as the last,
        // so a byte difference can't be misread as anything but round
        // count.
        mine.doc.tell(&me("a"), "more lines").unwrap();
        let mut fresh = Peer::new();
        let a = fresh.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
        assert_eq!(a.pushed, 0, "the first round after a restart is pull-only");
        assert_eq!(far.doc.items(), 31, "so the far side has not got it this round");
        let b = fresh.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
        assert!(b.pushed > 0);
        assert_eq!(far.doc.items(), 32, "it lands on round two");

        // Same byte count — the factual half of "not bytes".
        assert_eq!(
            one.pushed, b.pushed,
            "remembered or not, the same bytes move ({} vs {}); only the round count differs",
            one.pushed, b.pushed
        );
        assert_eq!(render(&mine.doc), render(&far.doc));

        let _ = fs::remove_dir_all(&dir);
    }

    /// Over the cap, the error names both numbers and points at the road
    /// that works. Both directions guard — what the far side sends is
    /// not ours to control.
    #[test]
    fn an_oversized_payload_is_refused_by_name() {
        let doc = ChatDoc::new(0x81).unwrap();
        // Feed oversized bytes directly: far cheaper than building
        // 256 KiB of real history, and it hits the same gate.
        let huge = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            vec![0u8; MAX_BYTES + 1],
        );
        let e = merge_b64(&doc, &huge).unwrap_err();
        let cap = khor_catalog::msg::inbound_too_big((MAX_BYTES + 1) / 1024, MAX_BYTES / 1024);
        assert_eq!(e, cap, "the error must say the cap was exceeded");
        assert!(e.contains("ssh"), "and it must point at a viable route: {e}");

        // Control: the same path under the cap must pass this gate —
        // without it, a reject-everything implementation is green above.
        let ok = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            vec![0u8; 16],
        );
        let e2 = merge_b64(&doc, &ok).unwrap_err();
        let small_cap = khor_catalog::msg::inbound_too_big(16 / 1024, MAX_BYTES / 1024);
        assert_ne!(
            e2, small_cap,
            "16 bytes must not hit the cap (it fails as an invalid delta, a different matter)"
        );
    }

    /// The version vector is exempt from the cap: refusing it kills the
    /// whole path, and the cap is for content.
    #[test]
    fn the_version_vector_is_not_capped() {
        let doc = ChatDoc::new(0x91).unwrap();
        doc.tell(&me("a"), "a line").unwrap();
        let v = version_b64(&doc);
        assert!(!v.is_empty());
        // Empty = from the beginning, not an error — a freshly paired
        // device.
        assert!(changes_since_b64(&doc, "").is_ok());
        // Bad base64 must read as "not base64", not "sync broken".
        let e = changes_since_b64(&doc, "not base64!!").unwrap_err();
        assert!(e.contains("base64"), "{e}");
    }
}
