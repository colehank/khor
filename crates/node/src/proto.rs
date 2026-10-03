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
    ///
    /// `roster` is the asker's newest signed roster, empty when it has
    /// none (docs/CONNECT.md 身份): the issuer settles whose root the
    /// network is before it burns the token (`khor_net::trust::settle`).
    ///
    /// **A tail field, and an older issuer refuses the whole frame** —
    /// by the rule at the head of this file it is the consumer's error
    /// branch that decides whether that is acceptable: `Node::pair_with`
    /// answers `Response::Refused`'s sentence, which the CLI prints and
    /// the join dialog paints as its error. A person reads a refusal,
    /// never a success; nothing is merged and no row is written. That is
    /// the 2026-08-17 rule's intended break (证书一上全网一起升).
    Pair {
        token: String,
        name: String,
        addrs: Vec<String>,
        #[serde(default)]
        roster: serde_bytes::ByteBuf,
    },
    /// One sync exchange over a named doc: `devices`, or `chat/<channel>`.
    Sync {
        doc: String,
        have: String,
        changes: String,
    },
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
    Open {
        kind: String,
        title: String,
        cwd: String,
        cmd: Vec<String>,
        cols: u16,
        rows: u16,
    },
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
    /// The directories that machine's row asks it to watch, each with
    /// the volume it is on (2026-09-06 关注目录). **A new op, not a sixth
    /// field on `Vitals`**: the head of this file measures what a
    /// tail-append costs an older peer (the whole reading), and an op it
    /// never heard of costs it nothing — it answers `Refused`, and the
    /// asker paints no rows. The list itself travels on the device
    /// table; only the machine named can say which volume a path is on.
    Watched,
    /// [`Request::FetchPath`], off a satellite: the sponsor stages the
    /// file down over scp on the first slice and serves the rest from
    /// the staging copy — one ssh per pull, not one per slice. Answers
    /// the ordinary [`Response::PathSlice`], whose change contract
    /// (total/at_ms moving = start over) rides the staged copy.
    SatFetchPath {
        sat: String,
        path: String,
        offset: u64,
    },
    /// [`Request::Remove`], off a satellite: the sponsor takes the path
    /// off that machine's disk over ssh (`crate::satellite::remove`).
    ///
    /// A new variant for [`Request::SatLs`]'s reason.
    SatRemove { sat: String, path: String },
    /// [`Request::Mkdir`], on a satellite: the sponsor makes the
    /// directory over ssh (`crate::satellite::mkdir`).
    ///
    /// A new variant for [`Request::SatLs`]'s reason.
    SatMkdir { sat: String, path: String },
    /// A new empty file on a satellite (`crate::satellite::touch`).
    ///
    /// **Its own variant rather than [`Request::SatWrite`] with no
    /// bytes**, which is how the plain-peer road spells the same thing.
    /// `SatWrite` pins the write to a sha256 of what is already there
    /// and answers 「it moved」 when there is no file at all — right for
    /// a save and exactly wrong for a new name. It also needs
    /// `sha256sum` on the far machine, which a new name has nothing to
    /// feed.
    SatTouch { sat: String, path: String },
    /// [`Request::RenamePath`], on a satellite: the sponsor moves the
    /// name over ssh (`crate::satellite::rename_path`).
    ///
    /// A new variant for [`Request::SatLs`]'s reason.
    SatRename {
        sat: String,
        from: String,
        to: String,
    },
    /// [`Request::Launch`], on a satellite: the sponsor runs that
    /// machine's opener over ssh (`crate::satellite::launch`).
    ///
    /// A new variant for [`Request::SatLs`]'s reason. **A satellite is
    /// very often a box nobody is sitting at**, and an opener there
    /// fails for want of a display — that refusal is the satellite's
    /// own sentence and travels back as it came, because 「it has no
    /// screen」 is a fact about that machine and not about khor.
    SatLaunch { sat: String, target: String },
    /// [`Request::Touched`], off a satellite: the sponsor walks that
    /// machine's directory over ssh and answers the ordinary
    /// [`Response::Touched`] (`crate::satellite::touched`).
    ///
    /// **A new variant for [`Request::SatLs`]'s reason**: a `sat` tail
    /// on `Touched` would lengthen that frame under every older peer
    /// for the ordinary case too. These words only ever go to a
    /// sponsor, and a machine that sponsors anything runs code that
    /// knows them.
    ///
    /// **Walked over there.** The mtimes are on the satellite's disk;
    /// a sponsor that answered from its own would date somebody else's
    /// files.
    SatTouched { sat: String, path: String },
    /// [`Request::GitStatus`], off a satellite: the sponsor runs that
    /// machine's own `git` over ssh and answers the ordinary
    /// [`Response::GitStatus`] (`crate::satellite::git_status`).
    ///
    /// A new variant for [`Request::SatLs`]'s reason.
    SatGitStatus { sat: String, root: String },
    /// [`Request::GitDiff`], off a satellite (`crate::satellite::
    /// git_diff`). A new variant for [`Request::SatLs`]'s reason.
    SatGitDiff { sat: String, path: String },
    /// [`Request::WritePath`], off a satellite (工作区批7).
    ///
    /// `was` is the same blake3 digest every other save is pinned to,
    /// and it stays blake3 across this wire: the sponsor is the one who
    /// can turn it into something the far machine can check, because it
    /// holds the copy those bytes came off
    /// (`crate::satellite::write_file`). A new variant for
    /// [`Request::SatLs`]'s reason — the sponsor's answer is the
    /// ordinary [`Response::Wrote`].
    SatWrite {
        sat: String,
        path: String,
        was: String,
        bytes: serde_bytes::ByteBuf,
    },
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
    OpenAgent {
        dir: String,
        title: String,
        form: String,
        agent: String,
    },
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
    /// (`msg::load_predates()`) rather than painting an empty table: an
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
    /// Takes a path off that machine's disk (`crate::files::remove`).
    /// `path` is absolute over there; a directory goes whole.
    ///
    /// **Never to the recycle bin.** A bin is somewhere a person can go
    /// and look, and the person is not sitting at that machine — a file
    /// put in another machine's bin is neither gone nor findable. So
    /// this deletes, and the face says so before anybody presses
    /// (`khor_catalog::gui::file_remove_far`), which is also why the
    /// 「不再提醒」 tick cannot silence it.
    ///
    /// Answered as [`Response::Removed`]; a khor that predates it
    /// answers [`Response::Refused`], which the asker shows as it came
    /// — the same road every refusal on this screen takes.
    Remove { path: String },
    /// Makes one directory on that machine (`crate::files::mkdir`).
    /// `path` is absolute over there.
    ///
    /// **One directory, and the name must be free.** Not `mkdir -p`:
    /// the caller is a person typing a name into a row of a tree they
    /// are looking at, so every parent already exists, and a spelling
    /// mistake that silently grows a chain of directories is the thing
    /// a refusal is for. A name already taken is refused rather than
    /// accepted quietly — 「it is there either way」 is true and is not
    /// what somebody pressing 新建目录 asked (`crate::files::remove`'s
    /// lesson, from the other end).
    ///
    /// Answered as [`Response::Made`]; a khor that predates it answers
    /// [`Response::Refused`], shown as it came.
    Mkdir { path: String },
    /// Moves a name on that machine (`crate::files::rename_path`).
    /// Both paths are absolute over there; crossing directories is the
    /// same verb, which is why this is spelled `mv` and not `rename`.
    ///
    /// **The target is never overwritten.** `std::fs::rename` replaces
    /// an existing file silently on unix, which would make one
    /// mistyped name eat another file — so the name being free is
    /// checked first and a taken one is refused. There is no `-f`:
    /// the second spelling of a verb is where a refusal goes to die
    /// (`crate::files::remove`).
    ///
    /// Answered as [`Response::Renamed`].
    RenamePath { from: String, to: String },
    /// Opens a path or a page **on that machine**, with that machine's
    /// own opener (`crate::files::launch`).
    ///
    /// **The machine running the session is the one that opens it**
    /// (the user, 2026-09-17: 都在宿主机器上打开). A face can be a
    /// browser on a phone; the file named in a conversation lives on
    /// the machine the conversation ran on, and opening it anywhere
    /// else opens nothing or, worse, something else with the same name.
    ///
    /// `target` is an absolute path over there or an `http(s)` address.
    /// Nothing else: what reaches here came out of a model's own words,
    /// so the schemes are a whitelist rather than a check for the
    /// obviously bad (`khor_gui_core::web::open_link`, same judgment).
    ///
    /// Answered as [`Response::Launched`]; a machine with no opener
    /// refuses by name rather than reporting a window nobody saw.
    Launch { target: String },
    /// When a directory on that machine was last changed: the newest
    /// file mtime under `path` (`crate::workspace::touched`). `path` is
    /// absolute over there.
    ///
    /// **Asked per workspace row, and the machine that holds the files
    /// is the only one that can answer.** A workspace list spans the
    /// mesh, and every row's date is a walk of somebody else's disk.
    ///
    /// Answered as [`Response::Touched`]; a khor that predates it
    /// answers [`Response::Refused`], and the row simply shows no time.
    /// **That is the consumer's error branch and it is deliberate**:
    /// unlike git marks, a missing date paints nothing rather than
    /// something false, and the row is still there to open.
    Touched { path: String },
    /// What differs from HEAD across one checkout (`crate::git::status`)
    /// — the marks the file tree wears and the branch its head says.
    /// `root` is the checkout itself, absolute on the answering machine.
    ///
    /// **One question per workspace, not per directory.** The tree
    /// walks and the answer does not change while it does; asking again
    /// on every step would make a person's own walking look like a
    /// repository churning. The cost of the whole answer was measured
    /// on this repo — 54 ms, 35 rows — and a per-step question would
    /// not have been cheaper, only more often.
    ///
    /// Answered as [`Response::GitStatus`]; a khor that predates it
    /// answers [`Response::Refused`], which the asker says out loud
    /// (`msg::git_predates()`) in the one place that has room for it —
    /// where the branch would have been. **Never painted as「clean」**:
    /// a tree with no marks and a tree khor could not read look the
    /// same, and only one of them is safe to edit.
    GitStatus { root: String },
    /// Where one file differs from HEAD (`crate::git::diff`), in the
    /// coordinates of the file as it is now. `path` is absolute on the
    /// answering machine; **which checkout it belongs to is worked out
    /// over there**, by the same walk that decided it was a workspace,
    /// rather than sent along and trusted.
    ///
    /// Asked when a file is opened, because the gutter is part of
    /// looking at it (「打开是当前状态,git状态显示在左侧」— the user,
    /// 2026-09-11). Answered as [`Response::GitDiff`]; refused the same
    /// way and with the same sentence as [`Request::GitStatus`], which
    /// is one key rather than two because the two ops shipped together
    /// and no khor has one without the other.
    GitDiff { path: String },
    /// Writes a whole file on the answering machine
    /// (`crate::files::write_whole`) — what saving an edit is.
    ///
    /// **`was` is the digest of what the writer started from, and it is
    /// the whole of the safety here.** A mismatch refuses and writes
    /// nothing, so two people editing one file end with one of them
    /// told rather than one of them silently gone. `""` means the
    /// writer expects no file at that path at all.
    ///
    /// **Whole file, never a range.** A range would need the two ends
    /// to agree on what the file was between them, which is the thing
    /// this op refuses to assume; and the file view only offers to save
    /// what fits in one slice, so the whole of it is already in hand.
    /// [`SLICE`] is the ceiling, worn at both ends.
    ///
    /// Answered as [`Response::Wrote`]; a khor that predates it answers
    /// [`Response::Refused`], which the asker says out loud
    /// (`msg::write_predates()`). **Never swallowed**: a save that
    /// quietly did nothing is the worst answer this file could give.
    WritePath {
        path: String,
        was: String,
        bytes: serde_bytes::ByteBuf,
    },
    /// Whether that machine can run VS Code's web face, and which
    /// version (`crate::code::version`). **A probe with no side effect**
    /// — nothing is started, nothing is downloaded — so it is safe to
    /// ask the moment a workspace opens, which is when the answer
    /// decides whether a button is drawn at all.
    ///
    /// Answered as [`Response::CodeHere`]; a khor that predates it
    /// answers [`Response::Refused`], which the asker paints as **no
    /// button**. That is the one place in this file where a refusal may
    /// be swallowed, and it is because the two readings agree: a
    /// machine that cannot say and a machine that has no editor are
    /// both machines khor must not offer to open one on.
    CodeHere,
    /// Starts VS Code's web face on `root` over there, or hands back the
    /// one already running, as `(port, token)`
    /// (`crate::code::serve`).
    ///
    /// **The consent happened before this frame was built.** The first
    /// run fetches ~640 MB from Microsoft and accepts their server
    /// licence, which is a thing a person says yes to — this op is what
    /// carries out the yes, and it has no way to ask.
    ///
    /// `port` is a loopback port **on the answering machine**; getting
    /// bytes to it is [`Request::Reach`]'s job done over again for a
    /// thing that is not a session, which is why the asker builds its
    /// own pipe rather than this op returning an address.
    CodeServe { root: String },
    /// Stops the web face on `root` over there, if one is running.
    /// **Answers the same whether there was one or not** — the caller
    /// wanted it gone, and a refusal would only name a race they cannot
    /// act on.
    CodeStop { root: String },
    /// Everything under `dir` on that machine that holds `query` — in a
    /// name or in a line (`crate::files::search`, 工作区第六轮 第 47 条).
    /// `dir` is absolute over there.
    ///
    /// **The walk happens where the files are.** The other road is to
    /// list the tree across the wire and match here, which is every
    /// byte of somebody's project moving so that one word can be
    /// looked for — and khor already refuses that shape for git marks
    /// and for `touched`.
    ///
    /// Answered as [`Response::Found`]; a khor that predates it answers
    /// [`Response::Refused`], which the face shows as it came.
    Search { dir: String, query: String },
    /// [`Request::Search`], on a satellite: the sponsor greps over ssh
    /// (`crate::satellite::search`).
    ///
    /// A new variant for [`Request::SatLs`]'s reason.
    SatSearch {
        sat: String,
        dir: String,
        query: String,
    },
    /// Whether `path` is a checkout (`crate::git::is_checkout`) — one
    /// `stat`, asked of the machine holding the directory.
    ///
    /// **Its own question and not a field on [`Request::GitStatus`]'s
    /// answer** (docs/SHELL.md 第五轮 第 6 条): a workspace row wears a
    /// branch or a folder by it, and the status walk is the one that
    /// can take seconds on a big tree or a network disk, or fail. A
    /// corner that waited on the walk painted a slow repository as a
    /// folder.
    ///
    /// Answered as [`Response::Checkout`]; a khor that predates it
    /// answers [`Response::Refused`], and the asker falls back to what
    /// the status answer says.
    Checkout { path: String },
    /// [`Request::Checkout`], on a satellite: the sponsor looks over
    /// ssh (`crate::satellite::is_checkout`).
    ///
    /// A new variant for [`Request::SatLs`]'s reason.
    SatCheckout { sat: String, path: String },
    /// **A round's opening question: what differs?** (round NET-3.) Each
    /// document the asker syncs, with a digest of its version
    /// (`khor_sync::wire::version_canonical`, blake3, 16 bytes); each
    /// reading it keeps of the answering machine (`sessions`, `vitals`,
    /// `watched`, `days`, `quotas`, `usage`, `sat/<id>`), with a digest
    /// of the answer it last got (empty: none). The day reading is asked
    /// as `days@<minute>`, the last minute of the asker's copy, so the
    /// answer can be the minutes since ([`Response::DaySince`]).
    /// Answered as [`Response::Versions`].
    ///
    /// **A new variant, the request changing words** (this file's first
    /// rule): a khor that predates it refuses the frame, and the asker
    /// then syncs the way it always did — every document in two
    /// exchanges, every reading asked — so an older peer shows exactly
    /// what it showed before, at the old price.
    Versions {
        docs: Vec<(String, serde_bytes::ByteBuf)>,
        readings: Vec<(String, serde_bytes::ByteBuf)>,
    },
    /// [`Request::Load`], **with the stamp of the day the asker already
    /// holds** (round NET-4; 0: none). The table is answered every time
    /// — it is what moves — and the day only when its stamp differs
    /// (`history::CpuHistory::stamp`, which changes once a minute at
    /// most): a machine card open on a far machine asked for the whole
    /// day, five series of 288 buckets, every five seconds. Answered as
    /// [`Response::LoadSince`]. A new variant for this file's first rule:
    /// a khor that predates it refuses the frame, and the asker asks
    /// [`Request::Load`] instead.
    LoadSince {
        top: u32,
        by: khor_core::TopBy,
        desc: bool,
        day: u64,
    },
    /// What a file is now, without its bytes (`crate::files::stat`):
    /// asked by a face coming back to a window with the file open, which
    /// re-reads it only when the answer differs from what it holds
    /// (round NET-4). Answered as [`Response::Stat`]; a khor that
    /// predates it refuses, and the face reads the file as before.
    Stat { path: String },
    /// **Watch these directories and say when one changes** (round
    /// NET-4, `crate::watch`): asked on a stream of its own, which stays
    /// open — answered first with [`Response::Watching`] and then one
    /// [`Response::Changed`] per burst of changes, each as a length-
    /// prefixed frame, until the asker lets the stream go. A khor that
    /// predates it refuses, and so does one that cannot watch; the
    /// asker's faces then ask `Ls`, as before.
    Watch { dirs: Vec<String> },
    /// The commands run on that machine, searched (roadmap 2.6,
    /// `crate::commands::search_history`). Answered as
    /// [`Response::History`]. **A new variant, the request changing
    /// words**: a khor that predates it refuses the frame, which the asker
    /// reads as "that machine keeps no history".
    History { ask: crate::commands::HistoryAsk },
    /// One file's history (round UX-4, the file page's git 历史): its
    /// log, its blame, or a revision of it (`crate::git::GitAsk`).
    /// Answered as [`Response::Git`]. **One new word for the three
    /// questions**, so an older khor refuses it once, which the asker
    /// says as `msg::git_predates()`.
    Git { ask: crate::git::GitAsk },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Pairing done on the issuer's side; `devices` is its table snapshot
    /// (base64) — merging it is what makes one pairing join the whole
    /// network.
    ///
    /// `roster` is the issuer's newest roster **after** it settled and
    /// signed (empty in a network from before roots); the asker runs the
    /// same rule on it before merging anything.
    ///
    /// An older asker cannot decode the longer frame: its `pair_with`
    /// lands on the decode error and prints it — a refusal a person
    /// reads, not a half-joined table (nothing was merged). Same break,
    /// same reason as `Request::Pair`.
    Paired {
        name: String,
        devices: String,
        #[serde(default)]
        roster: serde_bytes::ByteBuf,
    },
    Synced {
        version: String,
        changes: String,
        items: u64,
    },
    Refused {
        why: String,
    },
    /// One slice. `total` rides on every slice so the fetcher can show
    /// progress without a separate stat round; end = offset + len == total.
    Slice {
        total: u64,
        bytes: serde_bytes::ByteBuf,
    },
    SessionRows {
        rows: Vec<khor_core::Session>,
    },
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
    Vitals {
        vitals: khor_core::Vitals,
    },
    /// What that machine has spent. See [`Request::Usage`].
    Usage {
        usage: khor_core::Usage,
    },
    /// One directory, already ordered — directories first, each half by
    /// name — because the screen paints and never re-sorts (docs/UX.md
    /// 状态呈现). `path` is where the answer is about, absolute: the
    /// asker may have said `""` (home), and without the expansion it
    /// cannot spell the way down. `truncated` is the no-silent-caps
    /// rule on the wire: a directory bigger than the cap says so
    /// instead of looking whole.
    Dir {
        path: String,
        entries: Vec<DirEntry>,
        truncated: bool,
    },
    /// One slice of a pathed file. **Not [`Response::Slice`]**: that
    /// shape is what every shipped fetcher already decodes, and a field
    /// added to it would break them mid-pull — a new variant only ever
    /// reaches the peer that asked for it. `total` and `at_ms` ride
    /// every slice as the change contract: a puller that sees either
    /// move between slices is reading two different files and must
    /// start over, because no digest guards this path.
    PathSlice {
        total: u64,
        at_ms: u64,
        bytes: serde_bytes::ByteBuf,
    },
    /// A session was opened on the answering machine; `session` is its
    /// id there. See [`Request::Open`].
    Opened {
        session: String,
    },
    /// Where that session's host listens on the answering machine, and
    /// the cookie its handshake wants. See [`Request::Reach`].
    Reached {
        port: u16,
        cookie: String,
    },
    /// A satellite's cached reading and how old it is on the sponsor's
    /// clock. The age travels so the asker can keep the reading's true
    /// birth rather than dressing it in the arrival's freshness
    /// ([`Request::SatVitals`]; 离线轴 — a stale number must look
    /// stale).
    SatVitals {
        vitals: khor_core::Vitals,
        age_ms: u64,
    },
    /// [`Request::Watched`]'s answer: one row per path on that machine's
    /// `watch` list, in the list's order, each in one of the three
    /// shapes `khor_core::WatchedDir` keeps apart.
    Watched {
        rows: Vec<khor_core::WatchedDir>,
    },
    /// [`Request::Days`]'s answer: the record exactly as the answering
    /// machine keeps it on disk (`Day::to_wire`), one encoding for the
    /// file and the wire. The receiver rolls it to its own now
    /// (`Day::from_wire`), so every minute since this answer reads as
    /// nobody's.
    Days {
        day: String,
    },
    /// [`Request::Places`]'s answer: `(path, shown, sessions, last_ms)`
    /// per place, newest first — `shown` is the path as that machine's
    /// own user reads it (`~` for *its* home), because whose home a
    /// path is under is a fact about the answering machine.
    Places {
        places: Vec<(String, String, u32, i64)>,
    },
    /// [`Request::Quotas`]'s answer: one reading per account whose key
    /// the answering machine holds, as it last read them.
    Quotas {
        readings: Vec<khor_core::AccountReading>,
    },
    /// [`Request::Load`]'s answer, taken when this frame was built.
    Load {
        load: khor_core::Load,
    },
    /// [`Request::Remove`]'s answer: it is off that disk. Carries
    /// nothing — 「gone」 has no fields, and a path echoed back would be
    /// the asker's own argument returning as if it were news.
    Removed,
    /// [`Request::Mkdir`]'s and [`Request::SatTouch`]'s answer: the
    /// name is there now. Carries nothing, for [`Response::Removed`]'s
    /// reason.
    ///
    /// **One answer for both** because both say the same thing and the
    /// asker's next move is the same: ask the tree again. The plain
    /// `touch` road has no variant of its own at all — it is a
    /// [`Request::WritePath`] with no bytes, and answers
    /// [`Response::Wrote`].
    Made,
    /// [`Request::RenamePath`]'s answer: the name moved. Carries
    /// nothing — both paths were the asker's own arguments.
    Renamed,
    /// [`Request::Launch`]'s answer: that machine handed it to its
    /// opener. **Never a promise that a window appeared** — the opener
    /// is a separate process and khor does not wait for it — only that
    /// the machine had one and it started.
    Launched,
    /// [`Request::Touched`]'s answer, ms since the epoch. `0` means
    /// there was nothing under that path to date — an empty directory,
    /// or one that machine could not read — which the asker paints as
    /// no time rather than as 1970.
    Touched {
        at_ms: i64,
    },
    /// [`Request::GitStatus`]'s answer. `branch` is short (`main`) and
    /// **empty when there is no name to say** — detached, or no commit
    /// yet — which the asker paints as nothing rather than inventing a
    /// word for it. `rows` is `(path relative to the root, mark)`,
    /// sorted by path: the screen paints and never re-sorts (docs/UX.md
    /// 状态呈现), and the walk that produced them runs three threads
    /// and is documented to have no order of its own. `more` is the
    /// no-silent-caps rule [`Response::Dir::truncated`] already carries.
    GitStatus {
        branch: String,
        rows: Vec<(String, String)>,
        more: bool,
        /// How far this branch is from its remote-tracking branch, and
        /// when that was last true (`crate::git::Status`).
        ///
        /// **Tail-appended, and the consumer's error branch is the
        /// reason that is allowed.** A khor old enough to know
        /// `GitStatus` but not these refuses the whole frame, and
        /// `WorkspacePane` paints a refusal as its own sentence where
        /// the marks would be — a different screen from success, not a
        /// default wearing the clothes of a fact. The defaults
        /// themselves are unsayable rather than reassuring: `0` ahead
        /// and `0` behind with `fetched_ms` at 0 is「never fetched」,
        /// which the face draws as nothing at all.
        #[serde(default)]
        ahead: u32,
        #[serde(default)]
        behind: u32,
        #[serde(default)]
        fetched_ms: i64,
    },
    /// [`Request::GitDiff`]'s answer, in file order. `more` means the
    /// list stopped early — same rule, same reason.
    GitDiff {
        hunks: Vec<Hunk>,
        more: bool,
    },
    /// [`Request::WritePath`]'s answer: the file's mtime after the
    /// rename, and the digest of what is now there — so a writer can go
    /// on editing without reading the file back, and the next save has
    /// something true to compare against.
    Wrote {
        at_ms: u64,
        digest: String,
    },
    /// [`Request::CodeHere`]'s answer. `version` is `code --version`'s
    /// first line, and **empty means there is no editor there** — not
    /// an error, just a machine without one. `ready` says the server
    /// `serve-web` would fetch is already unpacked over there
    /// (`crate::code::Editor`).
    ///
    /// **`ready` is tail-appended, and the consumer's error branch is
    /// the reason that is allowed here.** A khor old enough to know
    /// `CodeHere` but not this field refuses the whole frame, and
    /// `Node::code_here`'s caller paints a refusal as **no VS Code
    /// button at all** (`views/WorkspacePane`) — which is a different
    /// screen from success, not a default wearing the clothes of a
    /// fact. The default itself errs toward asking: `false` shows the
    /// consent rather than skipping it.
    CodeHere {
        version: String,
        #[serde(default)]
        ready: bool,
    },
    /// [`Request::CodeServe`]'s answer: a loopback port on the
    /// answering machine and the connection token every request to it
    /// must carry.
    CodeServing {
        port: u16,
        token: String,
    },
    /// [`Request::Search`]'s and [`Request::SatSearch`]'s answer: the
    /// names and the lines that hold the words, in the answering
    /// machine's own spelling (`crate::files::Found`).
    ///
    /// **One answer for both roads** — a satellite's grep and khor's
    /// own walk are two ways of asking one question, and a face that
    /// had to tell them apart would be a face deciding which machine
    /// somebody's project is on.
    Found {
        found: crate::files::Found,
    },
    /// [`Request::Checkout`]'s and [`Request::SatCheckout`]'s answer.
    Checkout {
        yes: bool,
    },
    /// [`Request::Versions`]'s answer: only what differs.
    ///
    /// - `docs`: each asked document whose version is not the asker's,
    ///   with this machine's version (`khor_sync::wire::version_b64`) —
    ///   the asker syncs those alone, each in one exchange
    ///   (`khor_sync::wire::Peer::knowing`). A document at the same
    ///   version is not named; a name this khor does not know is not
    ///   named either (it would refuse the sync).
    /// - `readings`: each asked reading whose answer is not the one the
    ///   asker holds, as that whole answer's frame (`proto::encode` of
    ///   the [`Response`] the reading's own request gets) — or, for a
    ///   `days@<minute>` kind, a [`Response::DaySince`] when the minute
    ///   is inside the record; empty bytes for a reading this machine
    ///   refused. A reading not named is the one the asker already holds.
    Versions {
        docs: Vec<(String, String)>,
        readings: Vec<(String, serde_bytes::ByteBuf)>,
    },
    /// A day reading as the minutes since the asker's copy ended
    /// (`crate::day::DaySince`), in [`Response::Versions`]'s `readings`
    /// for a `days@<minute>` kind; `full` is the digest the whole
    /// [`Response::Days`] frame would have had, which the asker checks
    /// its rebuilt copy against. Only ever inside a `Versions` answer, so
    /// no older khor meets it.
    DaySince {
        since: crate::day::DaySince,
        full: serde_bytes::ByteBuf,
    },
    /// [`Request::LoadSince`]'s answer: the table, the day's stamp, and
    /// the day itself only when that stamp is not the one asked with.
    LoadSince {
        top: Vec<khor_core::TopRow>,
        day: u64,
        cpu_day: Option<khor_core::CpuDay>,
    },
    /// [`Request::Stat`]'s answer: size, mtime, and the digest of the
    /// bytes when the file fits one slice (empty otherwise).
    Stat {
        total: u64,
        at_ms: u64,
        digest: String,
    },
    /// [`Request::Watch`]'s first frame: the directories it is watching
    /// (one that is gone, or not a directory, is left out).
    Watching { dirs: Vec<String> },
    /// One burst of changes on a [`Request::Watch`] stream: the watched
    /// directories whose listings changed.
    Changed { dirs: Vec<String> },
    /// The answer to [`Request::History`], newest first.
    History {
        hits: Vec<crate::commands::HistoryHit>,
    },
    /// The answer to [`Request::Git`], variant for variant.
    Git { answer: crate::git::GitAnswer },
}

/// One place a file differs from HEAD, in the coordinates of the file
/// **as it is now** — because that is the copy on the screen, and a
/// gutter drawn in the old file's line numbers would point at the
/// wrong lines of the one a person is reading.
///
/// The three kinds a gutter draws are told apart by which half is
/// empty, with no fourth field able to disagree with them:
///
/// | kind | `lines` | `was` |
/// |---|---|---|
/// | inserted | > 0 | empty |
/// | deleted | 0 | non-empty |
/// | changed | > 0 | non-empty |
///
/// Every field required; whatever a later khor wants rides the tail
/// with a serde default, never the middle (this file's field
/// discipline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hunk {
    /// First line of the hunk in the current file, 1-based. When
    /// `lines` is 0 this is the line the deletion sits **after**, so a
    /// deletion at the very top of a file is 0. Same convention as
    /// `git diff -U0`'s `+` side, checked against it on 14 hunks of a
    /// real file rather than reasoned about.
    pub at: u32,
    /// How many current lines the hunk covers.
    pub lines: u32,
    /// What HEAD had in their place, one string per line, **without
    /// terminators** — a list already says where each line ends.
    pub was: Vec<String>,
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

impl Request {
    /// **Whether asking twice is the same as asking once** — what lets a
    /// request whose wire died under it be asked again on a fresh one
    /// without the person seeing the death (`link::Node::ask_on`).
    ///
    /// A reading changes nothing over there; a sync exchange is a CRDT
    /// merge, which takes the same bytes twice as once; a reach answers
    /// the port and cookie of a session that is already there. Anything
    /// that starts, ends, writes or moves something is not, even when
    /// the first attempt probably never arrived — "probably" is not
    /// good enough for a second shell or a second rename.
    pub fn is_reading(&self) -> bool {
        matches!(
            self,
            Request::Sync { .. }
                | Request::Fetch { .. }
                | Request::Sessions
                | Request::Usage
                | Request::Vitals
                | Request::Ls { .. }
                | Request::FetchPath { .. }
                | Request::Reach { .. }
                | Request::SatVitals { .. }
                | Request::SatLs { .. }
                | Request::Watched
                | Request::SatFetchPath { .. }
                | Request::SatTouched { .. }
                | Request::SatGitStatus { .. }
                | Request::SatGitDiff { .. }
                | Request::Days
                | Request::Places
                | Request::Quotas
                | Request::Load { .. }
                | Request::Touched { .. }
                | Request::GitStatus { .. }
                | Request::GitDiff { .. }
                | Request::CodeHere
                | Request::Search { .. }
                | Request::SatSearch { .. }
                | Request::Checkout { .. }
                | Request::SatCheckout { .. }
                | Request::Versions { .. }
                | Request::LoadSince { .. }
                | Request::Stat { .. }
                | Request::History { .. }
                | Request::Git { .. }
        )
    }

    /// **Whether this may ride 0-RTT** (round NET-3): sent inside the
    /// handshake of a redial and answered before that handshake is done.
    /// Early data can be replayed by anyone who captured it (RFC 9001
    /// §9.2, RFC 8446 §8), so only what changes nothing over there and
    /// costs little to answer twice: a reading of memory, one directory,
    /// one file slice, one `git` or `stat`. Not a sync (it merges), not a
    /// payload slice (it marks the offer started), not a reach (it hands
    /// out a session's cookie), not a search (a tree walk is a replay
    /// worth making). Everything else waits for the handshake on both
    /// ends: the asker sends it after, and an answering machine that sees
    /// one arrive early holds it until the handshake completes.
    pub fn rides_early(&self) -> bool {
        matches!(
            self,
            Request::Sessions
                | Request::Usage
                | Request::Vitals
                | Request::Ls { .. }
                | Request::FetchPath { .. }
                | Request::SatVitals { .. }
                | Request::SatLs { .. }
                | Request::Watched
                | Request::SatFetchPath { .. }
                | Request::SatTouched { .. }
                | Request::SatGitStatus { .. }
                | Request::SatGitDiff { .. }
                | Request::Days
                | Request::Places
                | Request::Quotas
                | Request::Load { .. }
                | Request::Touched { .. }
                | Request::GitStatus { .. }
                | Request::GitDiff { .. }
                | Request::Checkout { .. }
                | Request::SatCheckout { .. }
                | Request::Versions { .. }
                | Request::LoadSince { .. }
                | Request::Stat { .. }
        )
    }

    /// **How long the answer may take once the far machine has it** —
    /// a backstop, not the liveness check. A wire that has gone dead is
    /// caught in seconds whatever this says (`crate::wires::ACK_WITHIN`,
    /// `SILENT_AFTER`); this bounds a far machine that is alive and
    /// still working, so the number is the work's, per kind:
    ///
    /// - the default fifteen covers every answer read off memory or one
    ///   directory, with the dial (≤ 10 s) inside the asker's thirty;
    /// - anything over ssh (a satellite's) waits on a second network;
    /// - starting a session forks and waits for its host;
    /// - VS Code's web face may be downloaded on first use (ipc's
    ///   `DEADLINE_SLOW` has the measurement), and an accepted transfer
    ///   pulls whole files (`DEADLINE_TRANSFER`).
    pub fn budget(&self) -> std::time::Duration {
        use std::time::Duration;
        match self {
            Request::Act { action, .. } if action == "accept" => Duration::from_secs(3600),
            Request::CodeServe { .. } => Duration::from_secs(280),
            Request::Sync { .. }
            | Request::Fetch { .. }
            | Request::FetchPath { .. }
            | Request::SatFetchPath { .. }
            | Request::Pair { .. } => Duration::from_secs(30),
            Request::SatVitals { .. }
            | Request::SatLs { .. }
            | Request::SatRemove { .. }
            | Request::SatMkdir { .. }
            | Request::SatTouch { .. }
            | Request::SatRename { .. }
            | Request::SatLaunch { .. }
            | Request::SatTouched { .. }
            | Request::SatGitStatus { .. }
            | Request::SatGitDiff { .. }
            | Request::SatWrite { .. }
            | Request::SatSearch { .. }
            | Request::SatCheckout { .. } => Duration::from_secs(30),
            Request::Open { .. } | Request::OpenAgent { .. } => Duration::from_secs(25),
            _ => Duration::from_secs(15),
        }
    }
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
                assert_eq!(
                    (doc.as_str(), have.as_str(), changes.as_str()),
                    ("devices", "aGF2ZQ", "")
                );
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
        // The fields are never read on purpose: this enum stands in for
        // the shape an older peer knows, and the claim is about what
        // `Deserialize` accepts, not about the values.
        #[allow(dead_code)]
        #[derive(Debug, Deserialize)]
        enum OlderRequest {
            Pair {
                token: String,
                name: String,
                addrs: Vec<String>,
            },
            Sync {
                doc: String,
                have: String,
                changes: String,
            },
            Fetch {
                digest: String,
                offset: u64,
            },
            Sessions,
            Act {
                session: String,
                action: String,
            },
        }

        for (op, name) in [
            (Request::Vitals, &b"Vitals"[..]),
            (Request::Usage, &b"Usage"[..]),
            (
                Request::Ls {
                    path: String::new(),
                },
                &b"Ls"[..],
            ),
            (
                Request::FetchPath {
                    path: String::new(),
                    offset: 0,
                },
                &b"FetchPath"[..],
            ),
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
            (
                Request::Load {
                    top: 8,
                    by: khor_core::TopBy::Busiest,
                    desc: false,
                },
                &b"Load"[..],
            ),
            (Request::Watched, &b"Watched"[..]),
            (
                Request::Touched {
                    path: String::new(),
                },
                &b"Touched"[..],
            ),
            (
                Request::Remove {
                    path: String::new(),
                },
                &b"Remove"[..],
            ),
            (
                Request::Mkdir {
                    path: String::new(),
                },
                &b"Mkdir"[..],
            ),
            (
                Request::RenamePath {
                    from: String::new(),
                    to: String::new(),
                },
                &b"RenamePath"[..],
            ),
            (
                Request::Launch {
                    target: String::new(),
                },
                &b"Launch"[..],
            ),
            (
                Request::GitStatus {
                    root: String::new(),
                },
                &b"GitStatus"[..],
            ),
            (
                Request::GitDiff {
                    path: String::new(),
                },
                &b"GitDiff"[..],
            ),
            (
                Request::WritePath {
                    path: String::new(),
                    was: String::new(),
                    bytes: serde_bytes::ByteBuf::new(),
                },
                &b"WritePath"[..],
            ),
            (Request::CodeHere, &b"CodeHere"[..]),
            (
                Request::CodeServe {
                    root: String::new(),
                },
                &b"CodeServe"[..],
            ),
            (
                Request::CodeStop {
                    root: String::new(),
                },
                &b"CodeStop"[..],
            ),
            (
                Request::SatTouched {
                    sat: String::new(),
                    path: String::new(),
                },
                &b"SatTouched"[..],
            ),
            (
                Request::SatRemove {
                    sat: String::new(),
                    path: String::new(),
                },
                &b"SatRemove"[..],
            ),
            (
                Request::SatMkdir {
                    sat: String::new(),
                    path: String::new(),
                },
                &b"SatMkdir"[..],
            ),
            (
                Request::SatTouch {
                    sat: String::new(),
                    path: String::new(),
                },
                &b"SatTouch"[..],
            ),
            (
                Request::SatRename {
                    sat: String::new(),
                    from: String::new(),
                    to: String::new(),
                },
                &b"SatRename"[..],
            ),
            (
                Request::SatLaunch {
                    sat: String::new(),
                    target: String::new(),
                },
                &b"SatLaunch"[..],
            ),
            (
                Request::Search {
                    dir: String::new(),
                    query: String::new(),
                },
                &b"Search"[..],
            ),
            (
                Request::SatSearch {
                    sat: String::new(),
                    dir: String::new(),
                    query: String::new(),
                },
                &b"SatSearch"[..],
            ),
            (
                Request::Checkout {
                    path: String::new(),
                },
                &b"Checkout"[..],
            ),
            (
                Request::SatCheckout {
                    sat: String::new(),
                    path: String::new(),
                },
                &b"SatCheckout"[..],
            ),
            (
                Request::SatGitStatus {
                    sat: String::new(),
                    root: String::new(),
                },
                &b"SatGitStatus"[..],
            ),
            (
                Request::SatGitDiff {
                    sat: String::new(),
                    path: String::new(),
                },
                &b"SatGitDiff"[..],
            ),
            (
                Request::SatWrite {
                    sat: String::new(),
                    path: String::new(),
                    was: String::new(),
                    bytes: serde_bytes::ByteBuf::new(),
                },
                &b"SatWrite"[..],
            ),
            (
                Request::Versions {
                    docs: Vec::new(),
                    readings: Vec::new(),
                },
                &b"Versions"[..],
            ),
            (
                Request::LoadSince {
                    top: 8,
                    by: khor_core::TopBy::Busiest,
                    desc: false,
                    day: 0,
                },
                &b"LoadSince"[..],
            ),
            (
                Request::Stat {
                    path: String::new(),
                },
                &b"Stat"[..],
            ),
            (Request::Watch { dirs: Vec::new() }, &b"Watch"[..]),
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
        assert!(matches!(
            decode::<OlderRequest>(&theirs),
            Ok(OlderRequest::Sessions)
        ));
    }

    /// The watched answer crosses the wire in all three of its shapes —
    /// a directory on a listed volume, a missing one, one on a
    /// filesystem khor does not list — and comes back the same. The
    /// third shape is the one that matters: `None` with `missing: false`
    /// is what stops a path on `/proc` wearing the root's numbers, and a
    /// decoder that flattened it to either neighbour would pass a test
    /// that only sent the first two.
    #[test]
    fn a_watched_answer_keeps_its_three_shapes_on_the_wire() {
        let rows = vec![
            khor_core::WatchedDir {
                path: "/data/models".into(),
                missing: false,
                volume: Some(khor_core::Volume {
                    mount: "/data".into(),
                    fill: khor_core::Fill {
                        used: 40,
                        total: 100,
                    },
                }),
            },
            khor_core::WatchedDir {
                path: "/nope".into(),
                missing: true,
                volume: None,
            },
            khor_core::WatchedDir {
                path: "/proc/1".into(),
                missing: false,
                volume: None,
            },
        ];
        let bytes = encode(&Response::Watched { rows: rows.clone() }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Watched { rows: back } => assert_eq!(back, rows),
            other => panic!("decoded wrong: {other:?}"),
        }
        let ask = encode(&Request::Watched).unwrap();
        assert!(matches!(decode::<Request>(&ask), Ok(Request::Watched)));
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
                ppid: None,
                status: Some("run".into()),
                cpu_ms: Some(1_500),
                io_bytes: Some(4_096),
            }],
            cpu_day: khor_core::CpuDay {
                bucket_ms: 300_000,
                now_ms: 1_700_000_000_000,
                buckets: vec![Some(12.5), None, Some(80.0)],
                mem: vec![Some(40.0), None, Some(41.5)],
                gpu: vec![Some(7.0), None, None],
                disk: vec![Some(96.0), None, Some(96.0)],
                vram: vec![Some(90.0), None, Some(12.0)],
            },
        };
        let bytes = encode(&Response::Load { load: sent.clone() }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Load { load } => {
                assert_eq!(load, sent);
                assert_eq!(
                    load.cpu_day.buckets[1], None,
                    "the hole came back as a hole"
                );
                assert_eq!(
                    load.cpu_day.buckets[0],
                    Some(12.5),
                    "…between readings that came back whole"
                );
            }
            other => panic!("decoded wrong: {other:?}"),
        }
        // The asker's own words decode too, `top` and all — a request
        // whose count was lost would size the table from nothing.
        let ask = encode(&Request::Load {
            top: 8,
            by: khor_core::TopBy::Mem,
            desc: true,
        })
        .unwrap();
        assert!(matches!(
            decode::<Request>(&ask),
            Ok(Request::Load {
                top: 8,
                by: khor_core::TopBy::Mem,
                desc: true
            })
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
            gpus: vec![OldCard {
                name: "x".into(),
                util_pct: 3.0,
                mem: None,
            }],
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
            gpus: vec![khor_core::GpuCard {
                name: "x".into(),
                util_pct: 3.0,
                mem: None,
                temp_c: None,
            }],
            volumes: vec![],
            uptime_s: 0,
            model: None,
            system: None,
        })
        .unwrap();
        assert!(
            old.len() < fresh.len(),
            "the old frame really is shorter: it lacks the new fields"
        );
        let v: khor_core::Vitals = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(v.uptime_s, 0);
        assert_eq!(
            v.model, None,
            "a peer that predates the model field reads as one that could not say"
        );
        assert_eq!(v.gpus[0].temp_c, None);
        let day = rmp_serde::to_vec(&OldDay {
            bucket_ms: 1,
            now_ms: 2,
            buckets: vec![Some(1.0)],
        })
        .unwrap();
        let d: khor_core::CpuDay = rmp_serde::from_slice(&day).unwrap();
        assert!(
            d.mem.is_empty(),
            "a day from before memory has no memory line"
        );
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
        assert!(
            d2.gpu.is_empty(),
            "and the ones it never knew are absent, not a day of zeros"
        );
        assert!(d2.disk.is_empty());
        assert!(d2.vram.is_empty());
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
            ppid: None,
            status: None,
            cpu_ms: None,
            io_bytes: None,
        })
        .unwrap();
        assert!(
            old.len() < fresh.len(),
            "the old row really is shorter: it lacks the GPU columns"
        );
        let r: khor_core::TopRow = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(
            r.pid, 7,
            "the fields that were always there survive the tail-append"
        );
        assert_eq!(r.mem, 9);
        assert_eq!(
            r.gpu_pct, None,
            "a row from a khor that could not look says nothing, not zero"
        );
        assert_eq!(r.gpu_mem, None);
    }

    /// A day from before the cards' memory was recorded decodes with
    /// that series **empty**, and the four before it intact.
    ///
    /// Empty rather than a day of zeros is what lets the face draw four
    /// rows on a machine that never recorded a fifth — and it is the
    /// same shape as the honest answer on an Apple Silicon machine,
    /// where the cards' bytes are already the memory row
    /// (`khor_core::CpuDay::vram`).
    ///
    /// **The old bytes are shown to lack the field precisely**: this
    /// encoding writes a struct as an array and msgpack spells a short
    /// array's element count into its first byte (`0x90 | n`), so six
    /// fields against today's seven is the closest thing there is to
    /// 「字节里确实没有那个键」.
    #[test]
    fn a_day_from_before_the_cards_memory_still_decodes() {
        #[derive(serde::Serialize)]
        struct DayBeforeVram {
            bucket_ms: u64,
            now_ms: u64,
            buckets: Vec<Option<f32>>,
            mem: Vec<Option<f32>>,
            gpu: Vec<Option<f32>>,
            disk: Vec<Option<f32>>,
        }
        let old = rmp_serde::to_vec(&DayBeforeVram {
            bucket_ms: 300_000,
            now_ms: 7,
            buckets: vec![Some(1.0)],
            mem: vec![Some(2.0)],
            gpu: vec![Some(3.0)],
            disk: vec![Some(4.0)],
        })
        .unwrap();
        assert_eq!(old[0], 0x96, "the day a node sent yesterday is six fields");
        let fresh = rmp_serde::to_vec(&khor_core::CpuDay {
            bucket_ms: 300_000,
            now_ms: 7,
            buckets: vec![Some(1.0)],
            mem: vec![Some(2.0)],
            gpu: vec![Some(3.0)],
            disk: vec![Some(4.0)],
            vram: vec![Some(5.0)],
        })
        .unwrap();
        assert_eq!(
            fresh[0], 0x97,
            "and today's is seven — the new one is really on the wire"
        );
        let d: khor_core::CpuDay = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(
            d.disk,
            vec![Some(4.0)],
            "the series it did record survive, the last one included"
        );
        assert!(
            d.vram.is_empty(),
            "and the one it never knew is absent, not a day of zeros"
        );
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
        assert_eq!(
            old[0], 0x97,
            "the row a node sent yesterday is seven fields"
        );
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
            ppid: None,
            status: Some("sleep".into()),
            cpu_ms: Some(90_000),
            io_bytes: Some(1_048_576),
        })
        .unwrap();
        assert_eq!(
            fresh[0], 0x9d,
            "and today's is thirteen — the new ones are really on the wire"
        );
        let r: khor_core::TopRow = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(
            r.pid, 7,
            "the fields that were always there survive the tail-append"
        );
        assert_eq!(
            r.gpu_mem,
            Some(21_700_000_000),
            "including the last one before the new tail"
        );
        assert_eq!(
            r.run_secs, None,
            "an age nobody sent is absent — a zero would read as 刚起的"
        );
        assert_eq!(
            r.arg, None,
            "and an identifying word nobody sent is absent, not an empty name"
        );
    }

    /// A process row from before **the parent pid** decodes with it
    /// absent — and absent has to stay absent, because **pid 0 is a
    /// real parent** on every platform khor runs on.
    ///
    /// The face folds ten `tokio-rt-worker serve` rows into one on the
    /// strength of them being siblings (2026-09-05); a row from an
    /// older node has no parent to compare, and the fold must leave
    /// those rows alone rather than treat them all as children of 0 and
    /// join a machine's whole process table into one line.
    ///
    /// Same shape as the two tests above it: the array's element count
    /// is asserted on both sides, because that is the closest msgpack
    /// comes to saying 「this key is not in the bytes」 — without it
    /// this would be today's row compared against itself.
    #[test]
    fn a_process_row_from_before_the_parent_pid_still_decodes() {
        #[derive(serde::Serialize)]
        struct RowBeforeParent {
            pid: u32,
            name: String,
            user: String,
            cpu_pct: f32,
            mem: u64,
            gpu_pct: Option<f32>,
            gpu_mem: Option<u64>,
            run_secs: Option<u64>,
            arg: Option<String>,
        }
        let old = rmp_serde::to_vec(&RowBeforeParent {
            pid: 2_661_641,
            name: "tokio-rt-worker".into(),
            user: "zgh".into(),
            cpu_pct: 21.0,
            mem: 171_100_000,
            gpu_pct: None,
            gpu_mem: None,
            run_secs: Some(79_200),
            arg: Some("serve".into()),
        })
        .unwrap();
        assert_eq!(
            old[0], 0x99,
            "the row a node sent before the parent is nine fields"
        );
        let fresh = rmp_serde::to_vec(&khor_core::TopRow {
            pid: 2_661_641,
            name: "tokio-rt-worker".into(),
            user: "zgh".into(),
            cpu_pct: 21.0,
            mem: 171_100_000,
            gpu_pct: None,
            gpu_mem: None,
            run_secs: Some(79_200),
            arg: Some("serve".into()),
            ppid: Some(2_661_640),
            status: Some("sleep".into()),
            cpu_ms: Some(3_600_000),
            io_bytes: None,
        })
        .unwrap();
        assert_eq!(fresh[0], 0x9d, "and today's is thirteen");
        let r: khor_core::TopRow = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(
            r.pid, 2_661_641,
            "the fields that were always there survive the tail-append"
        );
        assert_eq!(
            r.arg.as_deref(),
            Some("serve"),
            "including the last one before the new tail"
        );
        assert_eq!(
            r.ppid, None,
            "a parent nobody sent is absent — 0 is somebody's real parent"
        );
    }

    /// A process row from before **状态 / CPU时间 / 读写** decodes with
    /// all three absent (2026-09-05:「列太少了,不饱满」).
    ///
    /// Three appends in one round because the fleet pays **once** for a
    /// round of them: a tail-appended field costs an old node nothing,
    /// but the `TopBy` words that sort these columns are a break, so
    /// the two remote machines get reinstalled either way — and one
    /// reinstall for three columns is the whole reason they landed
    /// together.
    ///
    /// Same shape as the tests above: the element count is asserted on
    /// both sides, because that is the closest msgpack comes to saying
    /// 「this key is not in the bytes」. Without it this compares
    /// today's row against itself and passes on a build that appended
    /// nothing.
    ///
    /// **The third assertion is the one with a fact behind it.**
    /// `io_bytes` absent must not decode to `0`: zero is what the
    /// *library* hands back for a process khor is not allowed to read
    /// (`top::io_readable`), so a default of zero here would mean the
    /// same cell says 「没读到」 and 「一个字节都没动」 with the same
    /// bytes on the wire.
    #[test]
    fn a_process_row_from_before_the_status_and_the_two_totals_still_decodes() {
        #[derive(serde::Serialize)]
        struct RowBeforeStatus {
            pid: u32,
            name: String,
            user: String,
            cpu_pct: f32,
            mem: u64,
            gpu_pct: Option<f32>,
            gpu_mem: Option<u64>,
            run_secs: Option<u64>,
            arg: Option<String>,
            ppid: Option<u32>,
        }
        let old = rmp_serde::to_vec(&RowBeforeStatus {
            pid: 73_443,
            name: "khor".into(),
            user: "zgh".into(),
            cpu_pct: 1.5,
            mem: 41_000_000,
            gpu_pct: None,
            gpu_mem: None,
            run_secs: Some(3_600),
            arg: Some("serve".into()),
            ppid: Some(1),
        })
        .unwrap();
        assert_eq!(
            old[0], 0x9a,
            "the row a node sent before these three is ten fields"
        );
        let fresh = rmp_serde::to_vec(&khor_core::TopRow {
            pid: 73_443,
            name: "khor".into(),
            user: "zgh".into(),
            cpu_pct: 1.5,
            mem: 41_000_000,
            gpu_pct: None,
            gpu_mem: None,
            run_secs: Some(3_600),
            arg: Some("serve".into()),
            ppid: Some(1),
            status: Some("run".into()),
            cpu_ms: Some(54_000),
            io_bytes: Some(8_388_608),
        })
        .unwrap();
        assert_eq!(
            fresh[0], 0x9d,
            "and today's is thirteen — the three are really on the wire"
        );
        let r: khor_core::TopRow = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(
            r.pid, 73_443,
            "the fields that were always there survive the tail-append"
        );
        assert_eq!(
            r.ppid,
            Some(1),
            "including the last one before the new tail"
        );
        assert_eq!(
            r.status, None,
            "a status nobody sent is absent — never the word 运行"
        );
        assert_eq!(
            r.cpu_ms, None,
            "an accumulated time nobody sent is absent, not 「烧了 0 毫秒」"
        );
        assert_eq!(
            r.io_bytes, None,
            "and an unsent disk total is absent, not 「一个字节都没动」"
        );
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
            Paired {
                name: String,
                devices: String,
            },
            Synced {
                version: String,
                changes: String,
                items: u64,
            },
            Refused {
                why: String,
            },
            Slice {
                total: u64,
                bytes: serde_bytes::ByteBuf,
            },
            SessionRows {
                rows: Vec<khor_core::Session>,
            },
            Acted {
                moved: u64,
            },
        }

        let old = encode(&OlderResponse::Acted { moved: 7 }).unwrap();
        let new = encode(&Response::Acted {
            moved: 7,
            landed: Vec::new(),
        })
        .unwrap();
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
                assert!(
                    landed.is_empty(),
                    "a peer that said nothing must read as nothing said"
                );
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
        let tokens = khor_core::Tokens {
            input: 1,
            cached_input: 2,
            cache_write: 3,
            output: 4,
        };
        let sent = khor_core::Usage {
            days: vec![khor_core::UsageDay {
                day: "2026-08-17".to_owned(),
                category: "claude".to_owned(),
                tokens,
            }],
            unreadable: 5,
            // **Both a named model and the unnamed one**, because the
            // absence is a value here rather than a missing field, and
            // a codec that dropped `None` on the floor would arrive
            // looking like a shorter list rather than like an error.
            models: vec![
                khor_core::UsageModelDay {
                    day: "2026-08-17".to_owned(),
                    category: "claude".to_owned(),
                    model: Some("a-model".to_owned()),
                    tokens,
                },
                khor_core::UsageModelDay {
                    day: "2026-08-17".to_owned(),
                    category: "claude".to_owned(),
                    model: None,
                    tokens: khor_core::Tokens::default(),
                },
            ],
            today: Some("2026-08-17".to_owned()),
        };
        let bytes = encode(&Response::Usage {
            usage: sent.clone(),
        })
        .unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Usage { usage } => assert_eq!(usage, sent),
            other => panic!("decoded wrong: {other:?}"),
        }
    }

    /// **A spending answer from before the model breakdown still
    /// decodes, and says so by being empty rather than by being wrong.**
    ///
    /// The tail-default half of this codec's one-way property, on the
    /// frame that just grew two fields. `models` empty beside a
    /// non-empty `days` is the sentence `khor_core::UsageModelDay`
    /// reserves for exactly this peer — 「it cannot tell」 — and `today`
    /// is `None` rather than a date nobody stated.
    ///
    /// **The old shape is spelled out here rather than trusted**: this
    /// encoding is positional, so "the field is absent" is a frame that
    /// is *shorter*, and the length is asserted for the same reason the
    /// vitals version asserts it — a struct that quietly still carried
    /// the fields would round-trip and prove nothing.
    #[test]
    fn a_spending_answer_from_before_the_model_breakdown_still_decodes() {
        #[derive(serde::Serialize)]
        struct OldUsage {
            days: Vec<khor_core::UsageDay>,
            unreadable: u64,
        }
        let day = khor_core::UsageDay {
            day: "2026-08-17".to_owned(),
            category: "claude".to_owned(),
            tokens: khor_core::Tokens {
                input: 1,
                cached_input: 2,
                cache_write: 3,
                output: 4,
            },
        };
        let old = rmp_serde::to_vec(&OldUsage {
            days: vec![day.clone()],
            unreadable: 5,
        })
        .unwrap();
        let fresh = rmp_serde::to_vec(&khor_core::Usage {
            days: vec![day.clone()],
            unreadable: 5,
            models: vec![],
            today: None,
        })
        .unwrap();
        assert!(
            old.len() < fresh.len(),
            "the old frame really is shorter: it lacks the new fields"
        );

        let u: khor_core::Usage = rmp_serde::from_slice(&old).unwrap();
        assert_eq!(u.days, vec![day], "what it did record survives whole");
        assert_eq!(u.unreadable, 5);
        assert!(
            u.models.is_empty(),
            "a peer that cannot cut by model says so by an empty list, not by a wrong one"
        );
        assert_eq!(
            u.today, None,
            "and it states no day rather than a default one"
        );
    }

    /// The satellite pair crosses whole, age included — the age is the
    /// half a lazier shape would drop, and it is what keeps a relayed
    /// reading wearing its true birth (附属机器批, 离线轴).
    #[test]
    fn a_satellites_reading_crosses_with_its_age() {
        let ask = Request::SatVitals {
            sat: "ab".repeat(32),
        };
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
            model: None,
            system: None,
        };
        let bytes = encode(&Response::SatVitals {
            vitals: sent.clone(),
            age_ms: 38_000,
        })
        .unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::SatVitals { vitals, age_ms } => {
                assert_eq!(vitals, sent);
                assert_eq!(
                    age_ms, 38_000,
                    "the age must survive, or staleness dresses as fresh"
                );
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
            gpu: Some(khor_core::Gpu {
                util_pct: 12.0,
                cards: 1,
                mem: None,
            }),
            version: Some("9.9.9".to_owned()),
            gpus: vec![khor_core::GpuCard {
                name: "A".to_owned(),
                util_pct: 12.0,
                mem: None,
                temp_c: None,
            }],
            volumes: vec![khor_core::Volume {
                mount: "/".to_owned(),
                fill: khor_core::Fill { used: 1, total: 2 },
            }],
            uptime_s: 12,
            model: None,
            system: None,
        };
        let bytes = encode(&Response::Vitals {
            vitals: sent.clone(),
        })
        .unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Vitals { vitals } => assert_eq!(vitals, sent),
            other => panic!("decoded wrong: {other:?}"),
        }
    }
    /// Roadmap 2.6's question and answer cross the wire whole, under
    /// their own name — an older khor sees a word it does not know and
    /// refuses, never a neighbour's shape.
    #[test]
    fn a_history_question_and_answer_round_trip() {
        let ask = crate::commands::HistoryAsk {
            text: "cargo test".into(),
            dir: "/w/khor".into(),
            failed: true,
        };
        let bytes = encode(&Request::History { ask: ask.clone() }).unwrap();
        assert!(bytes.windows(7).any(|w| w == b"History"), "the name is on the wire");
        match decode::<Request>(&bytes).unwrap() {
            Request::History { ask: got } => assert_eq!(got, ask),
            other => panic!("decoded wrong: {other:?}"),
        }
        let hit = crate::commands::HistoryHit {
            session: "shell/a".into(),
            text: "cargo test".into(),
            cwd: Some("/w/khor".into()),
            started_ms: 1,
            ended_ms: Some(2),
            exit: Some(101),
            runs: 3,
        };
        let bytes = encode(&Response::History { hits: vec![hit.clone()] }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::History { hits } => assert_eq!(hits, [hit]),
            other => panic!("decoded wrong: {other:?}"),
        }
    }

    /// UX-4's one word for a file's history crosses whole, the question
    /// and each answer under their own names.
    #[test]
    fn a_git_history_question_and_answer_round_trip() {
        let ask = crate::git::GitAsk::Show {
            path: "/w/khor/Cargo.toml".into(),
            rev: "0".repeat(40),
        };
        let bytes = encode(&Request::Git { ask: ask.clone() }).unwrap();
        assert!(bytes.windows(4).any(|w| w == b"Show"), "the question's name is on the wire");
        match decode::<Request>(&bytes).unwrap() {
            Request::Git { ask: got } => assert_eq!(got, ask),
            other => panic!("decoded wrong: {other:?}"),
        }
        let commit = crate::git::Commit {
            id: "a".repeat(40),
            author: "colehank".into(),
            when_ms: 1,
            subject: "One line".into(),
        };
        let answer = crate::git::GitAnswer::Blame {
            commits: vec![commit.clone()],
            runs: vec![crate::git::BlameRun { start: 0, len: 3, commit: 0 }],
        };
        let bytes = encode(&Response::Git { answer }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Git {
                answer: crate::git::GitAnswer::Blame { commits, runs },
            } => {
                assert_eq!(commits, [commit]);
                assert_eq!(runs.len(), 1);
            }
            other => panic!("decoded wrong: {other:?}"),
        }
    }
}
