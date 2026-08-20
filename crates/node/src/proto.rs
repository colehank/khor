//! Wire frames between nodes. msgpack: field order is wire order, new
//! fields go at the tail with serde defaults, `skip_serializing_if` is
//! banned — a mid-frame optional desyncs the array on every older end.
//!
//! **That rule buys one direction, not two, and the second half was
//! measured rather than assumed** (`a_tail_field_reaches_an_older_peer_
//! one_way_only`). A tail default lets *this* end read an older peer's
//! shorter frame. It does not let an older peer read this end's longer
//! one: it refuses the whole frame. So appending a field is safe for
//! the reader and a break for the writer, and the machine that suffers
//! is the one that has not upgraded — which is the ordinary state of a
//! network people update one machine at a time.
//!
//! **That one-way property is pinned once, for the whole tree**, in the
//! test named above. It is a fact about this codec and not about any
//! one frame, so a second copy of it on a second type would catch
//! nothing the first does not — and one fact written four times rots
//! at three of them.
//!
//! # Which of the three ways a shape change goes
//!
//! **First ask whether the *request* can change words.** A peer that
//! asks in new words has told you what it can read, and the answer
//! never reaches an old one at all — [`Response::PathSlice`] exists
//! instead of a field on [`Response::Slice`] for exactly that. It is
//! better than either choice below, and it is the one people skip
//! because appending is quicker to type.
//!
//! Otherwise the question is **not how much the new field matters**.
//! An older peer does not lose the field, it refuses the whole frame —
//! so what to weigh is the frame's absence, not the field's:
//!
//! > **When an older peer refuses this whole frame, is what it then
//! > shows any different from what it shows on success?**
//!
//! Answer it by walking every consumer of that request down to its
//! error branch and naming what that branch paints:
//!
//! - **Different** — a reading whose age keeps climbing, an empty
//!   state, a refusal a person reads — then a tail default is fine.
//!   The loss lands somewhere that already knows how to say "I cannot
//!   tell".
//! - **The same** — a default painted as fact, a swallowed error that
//!   leaves a stale value looking current, a count that quietly stays
//!   zero — then it needs a new variant. Appending there builds a
//!   silent failure, which is the shape most of this tree's rules
//!   exist to prevent.
//!
//! Note which way that cuts: a *request/response* failure is loud (the
//! verb reports it) and a *broadcast* failure is quiet, so sorting by
//! frame class would protect the one that protects itself. What saves
//! khor's broadcasts is not their class but that they ride a freshness
//! axis (docs/SESSION.md), which makes the silence visible — and that
//! is a property of the consumer, which is why the question asks about
//! the consumer.
//!
//! **Say the answer before appending: name the consumer's error branch
//! and what it paints. A sentence that will not come is not a small
//! thing to skip — it is the check not having happened.**
//!
//! And say it *beforehand*, because **this debt cannot be repaid, only
//! avoided**: a field removed later breaks the newer peers instead, so
//! an append has no retreat, only a second break.

use serde::{Deserialize, Serialize};

/// One frame's byte cap — above the sync payload cap
/// (`khor_sync::wire::MAX_BYTES` base64'd) with headroom.
pub const MAX_FRAME: usize = 512 * 1024;

/// One payload slice per request: stays under [`MAX_FRAME`] and makes
/// resume-by-offset free. Room is left for the msgpack envelope.
pub const SLICE: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Join: burn the one-time token, record me, hand me the network.
    Pair { token: String, name: String, addrs: Vec<String> },
    /// One sync exchange over a named doc: `devices`, or `chat/<channel>`.
    Sync { doc: String, have: String, changes: String },
    /// One slice of an offered payload, content-addressed. The offerer
    /// serves bytes only after the far user approved the pull — the
    /// approval IS this request arriving (docs/SESSION.md 传输).
    Fetch { digest: String, offset: u64 },
    /// The rows only the asked device can derive (its transfer faces;
    /// live kinds later). Chat rows are excluded — every device derives
    /// those from the CRDT itself, and duplicates would collide.
    Sessions,
    /// Run an action on a session this device executes for. Handlers
    /// never re-route an incoming Act — what is not theirs to run is
    /// refused, or two serves could bounce one forever.
    Act { session: String, action: String },
    /// What that machine has spent, by day and by vendor
    /// (`khor_core::Usage`).
    ///
    /// An op rather than a replicated document, for a reason the vitals
    /// op does not share: this is **derived from files khor only reads**,
    /// so it is re-derivable at any moment on the machine that has them
    /// and meaningless to copy anywhere else. What the asker keeps is a
    /// cache with the moment it arrived, so an unreachable machine shows
    /// its last answer and how old it is rather than a current-looking
    /// number.
    ///
    /// A khor that predates it answers [`Response::Refused`] — the name
    /// is on the wire, so an unknown op is a refusal and never a misread
    /// neighbour.
    Usage,
    /// What that machine is doing right now (CPU, memory, disk).
    ///
    /// An op rather than a field in the device document, because a
    /// reading is true for seconds — see `khor_core::Vitals`. A khor that
    /// predates it answers [`Response::Refused`] (the name is on the
    /// wire, so an unknown op is a refusal and not a misread neighbour),
    /// which the asking side treats as "no vitals" and nothing else.
    Vitals,
    /// One directory of the asked machine, for the files landing.
    /// `path` must be absolute; empty means that machine's home
    /// directory. Answered to any paired device without a permission
    /// step on purpose — this is one person's network, and what 待批
    /// guards is a payload's bytes, never the looking (docs/NET.md
    /// 入网即全信). A khor that predates it answers
    /// [`Response::Refused`], read as "that machine cannot list".
    Ls { path: String },
    /// One slice of a file named by absolute path — the browse-then-
    /// take pull, which has no offer to be content-addressed by
    /// ([`Request::Fetch`]'s path). No permission step here either,
    /// and it is not an exception to 待批全量: that gate guards the
    /// **receiving** side of an offer someone else pushed at it, and
    /// a pull is the puller spending its own disk on its own ask.
    FetchPath { path: String, offset: u64 },
    /// Open a session **on the asked machine** (docs/KHOR.md 发起:
    /// 在任意设备开 session). The row is that machine's from birth —
    /// it runs the process, it owns the registry entry, and the asker
    /// learns the id from the answer and then sees the row arrive the
    /// ordinary way ([`Request::Sessions`]).
    ///
    /// `cwd` empty means that machine's home. `cmd` empty means its
    /// login shell. Kept a separate op rather than an [`Request::Act`]
    /// action because Act names a session that already exists, and this
    /// one is what makes one.
    ///
    /// A khor that predates it answers [`Response::Refused`].
    Open { kind: String, title: String, cwd: String, cmd: Vec<String>, cols: u16, rows: u16 },
    /// Where a session's terminal host is listening on the asked
    /// machine, so a tunnel stream can carry the terminal itself
    /// (docs/NET.md 借网's pipe, pointed at a loopback port instead of
    /// the wider network).
    ///
    /// **Handing the cookie to a paired device is the pairing
    /// doctrine, not a hole in it**: 入网即全信 — the same rule that
    /// lets [`Request::Ls`] list a home directory without an approval
    /// step. The cookie is what keeps *unpaired* processes on that
    /// machine out of the socket, and it never leaves the pair.
    Reach { session: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Pairing done on the issuer's side; `devices` is its table snapshot
    /// (base64) — merging it is what makes one pairing join the whole
    /// network.
    Paired { name: String, devices: String },
    Synced { version: String, changes: String, items: u64 },
    Refused { why: String },
    /// One slice. `total` rides on every slice so the fetcher can show
    /// progress without a separate stat round; end = offset + len == total.
    Slice { total: u64, bytes: serde_bytes::ByteBuf },
    SessionRows { rows: Vec<khor_core::Session> },
    /// An Act ran to completion; for accept, bytes moved and where each
    /// file landed **on the machine that ran it** — absolute, in the
    /// offer's order.
    ///
    /// `landed` names every file in the offer, including one already
    /// present that moved no bytes: the asker wants somewhere to point,
    /// and "already here" is a reason to have a path rather than to
    /// lack one. It cannot be worked out at the asking end — the name
    /// on disk carries a digest prefix and the directory is the far
    /// machine's — so an empty one means *nobody said*, which is what
    /// an older khor and a close both send, and a caller that needs a
    /// path must say so out loud instead of guessing one that does not
    /// exist.
    Acted {
        moved: u64,
        #[serde(default)]
        landed: Vec<String>,
    },
    /// One reading, taken when this frame was built.
    Vitals { vitals: khor_core::Vitals },
    /// What that machine has spent. See [`Request::Usage`].
    Usage { usage: khor_core::Usage },
    /// One directory, already ordered — directories first, each half by
    /// name — because the screen paints and never re-sorts (docs/UX.md
    /// 状态呈现). `path` is where the answer is about, absolute: the
    /// asker may have said `""` (home), and without the expansion it
    /// cannot spell the way down. `truncated` is the no-silent-caps
    /// rule on the wire: a directory bigger than the cap says so
    /// instead of looking whole.
    Dir { path: String, entries: Vec<DirEntry>, truncated: bool },
    /// One slice of a pathed file. **Not [`Response::Slice`]**: that
    /// shape is what every shipped fetcher already decodes, and a field
    /// added to it would break them mid-pull — a new variant only ever
    /// reaches the peer that asked for it. `total` and `at_ms` ride
    /// every slice as the change contract: a puller that sees either
    /// move between slices is reading two different files and must
    /// start over, because no digest guards this path.
    PathSlice { total: u64, at_ms: u64, bytes: serde_bytes::ByteBuf },
    /// A session was opened on the answering machine; `session` is its
    /// id there. See [`Request::Open`].
    Opened { session: String },
    /// Where that session's host listens on the answering machine, and
    /// the cookie its handshake wants. See [`Request::Reach`].
    Reached { port: u16, cookie: String },
}

/// One row of a listed directory. Every field required; whatever a
/// later khor wants to add rides the tail with a serde default, never
/// the middle (this file's field discipline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub dir: bool,
    /// Bytes; 0 for directories — a directory's byte size answers a
    /// question nobody browsing is asking.
    pub size: u64,
    /// Modified, ms since epoch. 0 when the machine could not say.
    pub at_ms: u64,
}

pub fn encode<T: Serialize>(t: &T) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec(t).map_err(khor_catalog::msg::cant_encode_frame)
}

pub fn decode<T: for<'a> Deserialize<'a>>(bytes: &[u8]) -> Result<T, String> {
    rmp_serde::from_slice(bytes).map_err(khor_catalog::msg::cant_decode_frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_round_trips_and_names_its_op() {
        let req = Request::Sync {
            doc: "devices".into(),
            have: "aGF2ZQ".into(),
            changes: String::new(),
        };
        let bytes = encode(&req).unwrap();
        // The op name itself is on the wire: renaming a variant is a
        // protocol change and must land here first, consciously.
        assert!(
            bytes.windows(4).any(|w| w == b"Sync"),
            "the frame must carry the op name"
        );
        let back: Request = decode(&bytes).unwrap();
        match back {
            Request::Sync { doc, have, changes } => {
                assert_eq!((doc.as_str(), have.as_str(), changes.as_str()), ("devices", "aGF2ZQ", ""));
            }
            other => panic!("decoded wrong: {other:?}"),
        }
    }

    /// **A new op must be unreadable to an older peer, not readable as a
    /// different one.** That is the whole safety of adding one: the op
    /// name travels, so a khor that never heard of `Vitals` fails to
    /// decode and answers `Refused`, which the caller ignores. Were ops
    /// numbered instead, appending would be safe and inserting would
    /// silently turn one op into its neighbour — the failure this asserts
    /// cannot happen.
    ///
    /// The older peer is spelled out here as its own enum rather than
    /// mimicked, because a decoder built from today's type would be
    /// testing today's type against itself.
    ///
    /// **Both ops added since are checked**, not just the newest: the
    /// enum below is the one khor shipped before either existed, so
    /// `Vitals` proves the rule held once and `Usage` proves it still
    /// holds — and an op added without touching this test would be the
    /// one nobody checked.
    #[test]
    fn an_op_an_older_peer_never_heard_of_is_refused_not_mistaken() {
        #[derive(Debug, Deserialize)]
        enum OlderRequest {
            Pair { token: String, name: String, addrs: Vec<String> },
            Sync { doc: String, have: String, changes: String },
            Fetch { digest: String, offset: u64 },
            Sessions,
            Act { session: String, action: String },
        }

        for (op, name) in [
            (Request::Vitals, &b"Vitals"[..]),
            (Request::Usage, &b"Usage"[..]),
            (Request::Ls { path: String::new() }, &b"Ls"[..]),
            (Request::FetchPath { path: String::new(), offset: 0 }, &b"FetchPath"[..]),
        ] {
            let bytes = encode(&op).unwrap();
            assert!(
                bytes.windows(name.len()).any(|w| w == name),
                "the op name must be on the wire, or this proves nothing about names"
            );
            let refused = decode::<OlderRequest>(&bytes);
            assert!(refused.is_err(), "an older peer read {op:?} as {refused:?}");
        }

        // …and the same peer's own ops still decode here, so the
        // assertion above is about the new name and not about the two
        // enums having drifted apart in some other way.
        let theirs = encode(&Request::Sessions).unwrap();
        assert!(matches!(decode::<Request>(&theirs), Ok(Request::Sessions)));
        assert!(matches!(decode::<OlderRequest>(&theirs), Ok(OlderRequest::Sessions)));
    }

    /// **A tail-appended field reaches an older peer one way only**, and
    /// this pins both halves — the half that is a guarantee and the half
    /// that is not.
    ///
    /// The rule at the top of this file says new fields ride the tail
    /// with a serde default. Measured, that buys exactly one direction:
    /// this end reads an older peer's frame and defaults what is
    /// missing. The other direction does not decode at all —
    /// `rmp_serde` answers `array had incorrect length, expected 1` and
    /// the whole frame is refused.
    ///
    /// **What that costs is worth saying plainly, because the act still
    /// happens.** An older asker tells a newer machine to accept: the
    /// files really move, and then the answer is unreadable to the one
    /// who asked. It is told the transfer failed, and it is wrong.
    /// Loud rather than silent — nothing is misread as something else —
    /// but not compatible, and the doc comments on `Vitals::disk`,
    /// `Vitals::gpu` and `Vitals::version` (three tail-appends already
    /// shipped) each say only the half that works.
    ///
    /// The way out, when that cost has to go, is written a few variants
    /// up: [`Response::PathSlice`] exists rather than a field on
    /// `Slice` precisely so a changed shape only ever reaches a peer
    /// that asked in the new words. That is a bigger move than a
    /// default, and this test is here so the choice is made with the
    /// measurement in hand rather than from the rule alone.
    ///
    /// **The template's first step does not transfer, and copying it
    /// would have made this test unfailable.** `the_avatar_nonce_…` in
    /// mandala proves its sample is genuinely old by asserting the
    /// field's *name* is absent from the bytes. This codec never writes
    /// field names — a struct is a msgpack array and position is the
    /// only thing carrying identity — so `!bytes.contains("landed")` is
    /// true of every frame this program can build, the new one
    /// included. What proves the sample old here is that its array is
    /// one element shorter, so that is what is asserted.
    #[test]
    fn a_tail_field_reaches_an_older_peer_one_way_only() {
        /// What `Acted` was before the landing paths — spelled out
        /// rather than borrowed, because a sample built from today's
        /// type is today's type tested against itself. The variants
        /// before it ride along so this enum is what khor shipped,
        /// whichever way the codec identifies a variant.
        #[derive(Debug, Serialize, Deserialize)]
        enum OlderResponse {
            Paired { name: String, devices: String },
            Synced { version: String, changes: String, items: u64 },
            Refused { why: String },
            Slice { total: u64, bytes: serde_bytes::ByteBuf },
            SessionRows { rows: Vec<khor_core::Session> },
            Acted { moved: u64 },
        }

        let old = encode(&OlderResponse::Acted { moved: 7 }).unwrap();
        let new = encode(&Response::Acted { moved: 7, landed: Vec::new() }).unwrap();
        assert!(
            old.len() < new.len(),
            "the old sample must be a shorter frame, or it is not the old shape at all"
        );

        // ① The guarantee: an older peer's frame decodes here, the
        // absent field reads as nothing said, and `moved` still lands
        // in `moved` — the part a desynced array would take out.
        match decode::<Response>(&old).expect("an older peer's frame must still decode") {
            Response::Acted { moved, landed } => {
                assert_eq!(moved, 7);
                assert!(landed.is_empty(), "a peer that said nothing must read as nothing said");
            }
            other => panic!("decoded wrong: {other:?}"),
        }

        // ② The half that is not a guarantee. Asserted rather than
        // assumed: the day this codec is made tolerant of a long array,
        // this line goes red and the rule at the top of the file can be
        // rewritten to promise both directions.
        let mine = encode(&Response::Acted {
            moved: 9,
            landed: vec!["/home/a/files/ab12-notes.txt".to_owned()],
        })
        .unwrap();
        let refused = decode::<OlderResponse>(&mine);
        assert!(
            refused.is_err(),
            "if an older peer can now read a longer frame, the tail rule promises more than it did: {refused:?}"
        );
        // And the same peer's own frame still decodes, so the failure
        // above is about the extra element and not about these two
        // enums having drifted apart in some other way.
        assert!(matches!(
            decode::<OlderResponse>(&old),
            Ok(OlderResponse::Acted { moved: 7 })
        ));
    }

    /// A spending answer survives the round trip whole, **including the
    /// count of what could not be read**.
    ///
    /// That field is the one most easily lost on the way: an answer whose
    /// rows all arrive looks complete, and the only thing saying otherwise
    /// is a number that is usually zero. So the frame carrying a non-zero
    /// one is sent here on purpose.
    #[test]
    fn a_spending_answer_crosses_inside_its_response() {
        let sent = khor_core::Usage {
            days: vec![khor_core::UsageDay {
                day: "2026-08-17".to_owned(),
                category: "claude".to_owned(),
                tokens: khor_core::Tokens {
                    input: 1,
                    cached_input: 2,
                    cache_write: 3,
                    output: 4,
                },
            }],
            unreadable: 5,
        };
        let bytes = encode(&Response::Usage { usage: sent.clone() }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Usage { usage } => assert_eq!(usage, sent),
            other => panic!("decoded wrong: {other:?}"),
        }
    }

    /// A reading survives the round trip whole. The frame is a response,
    /// so the guard against reordering `Vitals`' own fields lives with
    /// the struct (`khor_core`); what this covers is the envelope.
    #[test]
    fn a_reading_crosses_inside_its_response() {
        let sent = khor_core::Vitals {
            cpu_pct: 42.5,
            cores: 10,
            mem: khor_core::Fill { used: 3, total: 4 },
            disk: Some(khor_core::Fill { used: 5, total: 6 }),
            gpu: Some(khor_core::Gpu { util_pct: 12.0, cards: 1, mem: None }),
            version: Some("9.9.9".to_owned()),
        };
        let bytes = encode(&Response::Vitals { vitals: sent.clone() }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Vitals { vitals } => assert_eq!(vitals, sent),
            other => panic!("decoded wrong: {other:?}"),
        }
    }
}
