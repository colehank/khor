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
    /// A satellite's last reading, asked of its **sponsor** — the one
    /// machine that samples it over ssh (附属机器批; docs/NET.md 卫星).
    /// A cached answer with its age, never a fresh sample: the asker
    /// must not be able to make the sponsor dial out on demand, and the
    /// age is what keeps a stale reading honest (离线轴).
    ///
    /// A new variant rather than a field on [`Request::Vitals`]: that
    /// one is a unit variant, and growing it a body would change its
    /// bytes under every older peer. A khor that predates satellites
    /// answers [`Response::Refused`], read as "no reading" — the same
    /// fallback every tail-appended op leans on.
    SatVitals { sat: String },
    /// [`Request::Ls`], run on a satellite: the sponsor lists the
    /// directory over ssh and answers the ordinary [`Response::Dir`].
    /// A new variant, not a tail field on `Ls` — a `sat: None` tail
    /// would lengthen the frame under every older peer for the plain
    /// listings too, breaking exactly the case that has no satellite in
    /// it (the module head's first question: change the words instead).
    /// These words only ever go to a sponsor, and a machine that
    /// sponsors anything runs code that knows them.
    SatLs { sat: String, path: String },
    /// [`Request::FetchPath`], off a satellite: the sponsor stages the
    /// file down over scp on the first slice and serves the rest from
    /// the staging copy — one ssh per pull, not one per slice. Answers
    /// the ordinary [`Response::PathSlice`], whose change contract
    /// (total/at_ms moving = start over) rides the staged copy.
    SatFetchPath { sat: String, path: String, offset: u64 },
    /// The asked machine's own day record (`khor_node::day`): what
    /// *its* list said about each of *its* sessions over the last 24
    /// hours. A reported row used to paint 没在看 for its whole strip
    /// (远端日条批, 2026-09-01), because a peer's row is one report
    /// with an age and not a history — the history lives on the home
    /// machine, and this is the road it travels.
    ///
    /// Answered as [`Response::Days`]; a khor that predates it answers
    /// [`Response::Refused`], which the asker reads as "no record from
    /// there" and keeps painting 没在看. **Tail-appended**, like every
    /// op since the first.
    Days,
    /// Open an **agent** session on the asked machine — the wizard's four
    /// answers, resolved over there (docs/KHOR.md 发起: 在任意设备开
    /// session, now for conversations too). Where [`Request::Open`]
    /// carries a command and a terminal size and hosts a PTY, this
    /// carries a *name* and hosts whatever that name means on the
    /// answering machine: khor's own `claude` / `codex`, or an agent
    /// registered there — resolved by [`crate::Node::start_session`],
    /// the same call the wizard makes locally, so both machines refuse
    /// in one voice. A name resolved on the asking side would carry the
    /// asker's paths to a machine that has different ones.
    ///
    /// `dir` empty (or `~`) means that machine's home. `form` is
    /// `chat` / `term` / `agent`. Answered as [`Response::Opened`]; a
    /// khor that predates it answers [`Response::Refused`].
    OpenAgent { dir: String, title: String, form: String, agent: String },
    /// 常去的地方 on the asked machine (`crate::Node::places`): the
    /// wizard's first question, answered for wherever the session is
    /// about to be born rather than for the machine the screen is on.
    /// Answered as [`Response::Places`]; a khor that predates it answers
    /// [`Response::Refused`], read as "no list" — the box still takes a
    /// typed path.
    Places,
    /// The asked machine's readings of the API accounts **it holds the
    /// keys for** (`khor_node::accounts`): every other machine reads
    /// them from here rather than from the provider, so a key stays
    /// on one disk. Answered as [`Response::Quotas`]; a khor that
    /// predates it answers [`Response::Refused`], read as "no reading
    /// from there" and painted as such with the last one's age.
    Quotas,
    /// What the asked machine is busy with, now and over the last day
    /// (`khor_core::Load`): its `top` hungriest processes and the CPU
    /// day its serve has been recording (机器页批3, 2026-09-03). Asked
    /// while a person has that machine's card open, never on the beat —
    /// a process table nobody is looking at is a table nobody is
    /// paying for. Answered as [`Response::Load`]; a khor that predates
    /// it answers [`Response::Refused`], which the asker says out loud
    /// (`msg::LOAD_PREDATES`) rather than painting an empty table: an
    /// empty table reads as an idle machine.
    /// `by`/`desc` rank the table **over there**, before the cut: a
    /// shared box runs ten thousand processes, and the alternative to
    /// ranking at the source is either all of them on the wire or a cut
    /// ordered by the wrong column (`khor_core::TopBy`). Tail-appended
    /// with defaults, so a khor that predates them answers its own
    /// busiest-first order — which is exactly what the defaults name.
    Load {
        top: u32,
        #[serde(default)]
        by: khor_core::TopBy,
        #[serde(default)]
        desc: bool,
    },
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
    /// A satellite's cached reading and how old it is on the sponsor's
    /// clock. The age travels so the asker can keep the reading's true
    /// birth rather than dressing it in the arrival's freshness
    /// ([`Request::SatVitals`]; 离线轴 — a stale number must look
    /// stale).
    SatVitals { vitals: khor_core::Vitals, age_ms: u64 },
    /// [`Request::Days`]'s answer: the record exactly as the answering
    /// machine keeps it on disk (`Day::to_wire`), one encoding for the
    /// file and the wire. The receiver rolls it to its own now
    /// (`Day::from_wire`), so every minute since this answer reads as
    /// nobody's.
    Days { day: String },
    /// [`Request::Places`]'s answer: `(path, shown, sessions, last_ms)`
    /// per place, newest first — `shown` is the path as that machine's
    /// own user reads it (`~` for *its* home), because whose home a
    /// path is under is a fact about the answering machine.
    Places { places: Vec<(String, String, u32, i64)> },
    /// [`Request::Quotas`]'s answer: one reading per account whose key
    /// the answering machine holds, as it last read them.
    Quotas { readings: Vec<khor_core::AccountReading> },
    /// [`Request::Load`]'s answer, taken when this frame was built.
    Load { load: khor_core::Load },
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
            (Request::Days, &b"Days"[..]),
            (
                Request::OpenAgent {
                    dir: String::new(),
                    title: String::new(),
                    form: String::new(),
                    agent: String::new(),
                },
                &b"OpenAgent"[..],
            ),
            (Request::Places, &b"Places"[..]),
            (Request::Quotas, &b"Quotas"[..]),
            (Request::Load { top: 8, by: khor_core::TopBy::Busiest, desc: false }, &b"Load"[..]),
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

    /// The load answer crosses the wire with its gaps intact: a bucket
    /// nobody measured is `None` at both ends, never a zero — a zero
    /// would be the one reading the band must not invent
    /// (`khor_core::CpuDay`). The filled neighbours prove the decoder
    /// reads what is there before the hole is trusted to be a hole.
    #[test]
    fn a_load_frame_keeps_its_holes_as_holes() {
        let sent = khor_core::Load {
            top: vec![khor_core::TopRow {
                pid: 41,
                name: "khor".into(),
                user: "zgh".into(),
                cpu_pct: 250.5,
                mem: 12_345_678,
                gpu_pct: Some(37.0),
                gpu_mem: Some(1_073_741_824),
                run_secs: Some(86_400),
                arg: Some("serve".into()),
            }],
            cpu_day: khor_core::CpuDay {
                bucket_ms: 300_000,
                now_ms: 1_700_000_000_000,
                buckets: vec![Some(12.5), None, Some(80.0)],
                mem: vec![Some(40.0), None, Some(41.5)],
                gpu: vec![Some(7.0), None, None],
                disk: vec![Some(96.0), None, Some(96.0)],
            },
        };
        let bytes = encode(&Response::Load { load: sent.clone() }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Load { load } => {
                assert_eq!(load, sent);
                assert_eq!(load.cpu_day.buckets[1], None, "the hole came back as a hole");
                assert_eq!(load.cpu_day.buckets[0], Some(12.5), "…between readings that came back whole");
            }
            other => panic!("decoded wrong: {other:?}"),
        }
        // The asker's own words decode too, `top` and all — a request
        // whose count was lost would size the table from nothing.
        let ask = encode(&Request::Load { top: 8, by: khor_core::TopBy::Mem, desc: true }).unwrap();
        assert!(matches!(
            decode::<Request>(&ask),
            Ok(Request::Load { top: 8, by: khor_core::TopBy::Mem, desc: true })
        ));
    }

    /// The three fields the card's redesign added (2026-09-03: uptime,
    /// the memory day, a card's temperature) ride the tail of their
    /// structs with a default, and this is the frame that proves it: an
    /// older node's shape, encoded without them, decodes on a new node
    /// with the defaults — and the bytes are first shown to be shorter,
    /// so the test cannot pass by accident.
    #[test]
    fn a_frame_from_before_uptime_memory_day_and_temperature_still_decodes() {
        #[derive(serde::Serialize)]
        struct OldCard {
            name: String,
            util_pct: f32,
            mem: Option<khor_core::Fill>,
        }
        #[derive(serde::Serialize)]
        struct OldVitals {
            cpu_pct: f32,
            cores: u32,
            mem: khor_core::Fill,
            disk: Option<khor_core::Fill>,
            gpu: Option<khor_core::Gpu>,
            version: Option<String>,
            gpus: Vec<OldCard>,
            volumes: Vec<khor_core::Volume>,
        }
        #[derive(serde::Serialize)]
        struct OldDay {
            bucket_ms: u64,
            now_ms: u64,
            buckets: Vec<Option<f32>>,
        }
        // A day from the recorder that knew CPU and memory but not the
        // other two: its GPU and disk series come back **empty**, which
        // the chart draws as two rows instead of four flat zeros.
        #[derive(serde::Serialize)]
        struct DayBeforeGpu {
            bucket_ms: u64,
            now_ms: u64,
            buckets: Vec<Option<f32>>,
            mem: Vec<Option<f32>>,
        }
        let old = rmp_serde::to_vec(&OldVitals {
            cpu_pct: 1.0,
            cores: 2,
            mem: khor_core::Fill { used: 1, total: 2 },
            disk: None,
            gpu: None,
            version: None,
            gpus: vec![OldCard { name: "x".into(), util_pct: 3.0, mem: None }],
            volumes: vec![],
        })
        .unwrap();
        let fresh = rmp_serde::to_vec(&khor_core::Vitals {
            cpu_pct: 1.0,
            cores: 2,
            mem: khor_core::Fill { used: 1, total: 2 },
            disk: None,
            gpu: None,
            version: None,
            gpus: vec![khor_core::GpuCard { name: "x".into(), util_pct: 3.0, mem: None, temp_c: None }],
            volumes: vec![],
            uptime_s: 0,
        })
        .unwrap();
        assert!(old.len() < fresh.len(), "the old frame really is shorter: it lacks the new fields");
        let v: khor_core::Vitals = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(v.uptime_s, 0);
        assert_eq!(v.gpus[0].temp_c, None);
        let day = rmp_serde::to_vec(&OldDay { bucket_ms: 1, now_ms: 2, buckets: vec![Some(1.0)] }).unwrap();
        let d: khor_core::CpuDay = rmp_serde::from_slice(&day).unwrap();
        assert!(d.mem.is_empty(), "a day from before memory has no memory line");
        assert_eq!(d.buckets, vec![Some(1.0)]);
        let two = rmp_serde::to_vec(&DayBeforeGpu {
            bucket_ms: 1,
            now_ms: 2,
            buckets: vec![Some(1.0)],
            mem: vec![Some(2.0)],
        })
        .unwrap();
        let d2: khor_core::CpuDay = rmp_serde::from_slice(&two).unwrap();
        assert_eq!(d2.mem, vec![Some(2.0)], "the series it did record survive");
        assert!(d2.gpu.is_empty(), "and the two it never knew are absent, not a day of zeros");
        assert!(d2.disk.is_empty());
    }

    /// A process row from before the GPU columns decodes with them
    /// absent — **absent, not zero**: a `0%` there would say "this
    /// process is not on the GPU" about a machine whose khor was never
    /// able to look, and a zero in a column of numbers reads as a
    /// reading (`khor_core::TopRow` carries the ruling).
    ///
    /// The old bytes are shown to be shorter first, so a decoder that
    /// silently accepted anything could not pass this by accident.
    #[test]
    fn a_process_row_from_before_the_gpu_columns_still_decodes() {
        #[derive(serde::Serialize)]
        struct OldRow {
            pid: u32,
            name: String,
            user: String,
            cpu_pct: f32,
            mem: u64,
        }
        let old = rmp_serde::to_vec(&OldRow {
            pid: 7,
            name: "khor".into(),
            user: "zgh".into(),
            cpu_pct: 5.0,
            mem: 9,
        })
        .unwrap();
        let fresh = rmp_serde::to_vec(&khor_core::TopRow {
            pid: 7,
            name: "khor".into(),
            user: "zgh".into(),
            cpu_pct: 5.0,
            mem: 9,
            gpu_pct: None,
            gpu_mem: None,
            run_secs: None,
            arg: None,
        })
        .unwrap();
        assert!(old.len() < fresh.len(), "the old row really is shorter: it lacks the GPU columns");
        let r: khor_core::TopRow = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(r.pid, 7, "the fields that were always there survive the tail-append");
        assert_eq!(r.mem, 9);
        assert_eq!(r.gpu_pct, None, "a row from a khor that could not look says nothing, not zero");
        assert_eq!(r.gpu_mem, None);
    }

    /// A process row from before **how long it has been up** and **the
    /// command line** decodes with both absent (2026-09-04:「表格多个
    /// 时间列」 and the three `python3` rows on turing).
    ///
    /// **The old bytes are shown to lack the fields precisely, not by
    /// being shorter.** This encoding writes a struct as an array, and
    /// msgpack spells a short array's element count into its first byte
    /// (`0x90 | n`) — so the count is the closest thing there is to
    /// "the key is not in the bytes", and asserting it is what stops
    /// this test from quietly comparing today's row against itself.
    #[test]
    fn a_process_row_from_before_run_time_and_the_command_line_still_decodes() {
        #[derive(serde::Serialize)]
        struct RowBeforeTime {
            pid: u32,
            name: String,
            user: String,
            cpu_pct: f32,
            mem: u64,
            gpu_pct: Option<f32>,
            gpu_mem: Option<u64>,
        }
        let old = rmp_serde::to_vec(&RowBeforeTime {
            pid: 7,
            name: "python3".into(),
            user: "zzl-zgh".into(),
            cpu_pct: 5.0,
            mem: 9,
            gpu_pct: Some(44.0),
            gpu_mem: Some(21_700_000_000),
        })
        .unwrap();
        assert_eq!(old[0], 0x97, "the row a node sent yesterday is seven fields");
        let fresh = rmp_serde::to_vec(&khor_core::TopRow {
            pid: 7,
            name: "python3".into(),
            user: "zzl-zgh".into(),
            cpu_pct: 5.0,
            mem: 9,
            gpu_pct: Some(44.0),
            gpu_mem: Some(21_700_000_000),
            run_secs: Some(3),
            arg: Some("vllm.entrypoints".into()),
        })
        .unwrap();
        assert_eq!(fresh[0], 0x99, "and today's is nine — the two new ones are really on the wire");
        let r: khor_core::TopRow = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(r.pid, 7, "the fields that were always there survive the tail-append");
        assert_eq!(r.gpu_mem, Some(21_700_000_000), "including the last one before the new tail");
        assert_eq!(r.run_secs, None, "an age nobody sent is absent — a zero would read as 刚起的");
        assert_eq!(r.arg, None, "and an identifying word nobody sent is absent, not an empty name");
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

    /// The satellite pair crosses whole, age included — the age is the
    /// half a lazier shape would drop, and it is what keeps a relayed
    /// reading wearing its true birth (附属机器批, 离线轴).
    #[test]
    fn a_satellites_reading_crosses_with_its_age() {
        let ask = Request::SatVitals { sat: "ab".repeat(32) };
        let bytes = encode(&ask).unwrap();
        match decode::<Request>(&bytes).unwrap() {
            Request::SatVitals { sat } => assert_eq!(sat, "ab".repeat(32)),
            other => panic!("decoded wrong: {other:?}"),
        }
        let sent = khor_core::Vitals {
            cpu_pct: 7.0,
            cores: 4,
            mem: khor_core::Fill { used: 1, total: 2 },
            disk: None,
            gpu: None,
            version: None,
            gpus: vec![],
            volumes: vec![],
            uptime_s: 0,
        };
        let bytes = encode(&Response::SatVitals { vitals: sent.clone(), age_ms: 38_000 }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::SatVitals { vitals, age_ms } => {
                assert_eq!(vitals, sent);
                assert_eq!(age_ms, 38_000, "the age must survive, or staleness dresses as fresh");
            }
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
            gpus: vec![khor_core::GpuCard { name: "A".to_owned(), util_pct: 12.0, mem: None, temp_c: None }],
            volumes: vec![khor_core::Volume {
                mount: "/".to_owned(),
                fill: khor_core::Fill { used: 1, total: 2 },
            }],
            uptime_s: 12,
        };
        let bytes = encode(&Response::Vitals { vitals: sent.clone() }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Vitals { vitals } => assert_eq!(vitals, sent),
            other => panic!("decoded wrong: {other:?}"),
        }
    }
}
