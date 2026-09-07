//! The GUI's data layer: the same khor-node calls the CLI makes, shaped
//! into rows the frontend can render. Two skins share this module — the
//! tauri commands (apps/gui/src-tauri) and the dev bridge (`bridge`
//! bin) — so what the browser verifies is what the app ships. The LAN
//! listener a phone opens will be the third; [`api`] is the table it
//! will dispatch through, moved out of the bridge ahead of it (批⑦).
//!
//! No judgment lives here: words, ordering, unread all come from the
//! node. The GUI must not re-derive any of it (docs/UX.md 状态呈现).

pub mod acs;
pub mod agents;
pub mod api;
pub mod chat;
pub mod files;
pub mod recents;
pub mod term;
pub mod web;

use std::collections::HashMap;
use std::path::Path;

use khor_node::list::Arrange;
pub use khor_core::QuotaAnswer;
use khor_core::map::{self, Seat};
use khor_node::{
    Avatar, AvatarStyle, FaceShape, Node, SessionId, Usage, Variant, Vitals, PRESETS,
};
use serde::Serialize;
use ts_rs::TS;

/// One list row. `word` is the state *key* (`busy`…); the frontend looks
/// the display word up in the catalog at the last moment (docs/UX.md 文案).
#[derive(Debug, Clone, Serialize, TS)]
pub struct SessionRow {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub word: String,
    /// ms; `number` on the TS side — serde_json sends a number, and
    /// 2^53 outlives every timestamp and unread count in question.
    #[ts(type = "number")]
    pub at_ms: u64,
    #[ts(type = "number")]
    pub unread: u64,
    /// Present on reported rows: the offline axis, never a seventh word.
    pub source: Option<SourceTag>,
    /// Pinned somewhere in the network (`khor_sync::pins`). The frontend
    /// paints a mark from this and **does not sort by it** — the rows
    /// arrive in the order the node decided.
    pub pinned: bool,
    /// Old news the default view folds into a tail count
    /// (`khor_node::SessionView::stale` — dead, looked at, and dead
    /// past the one threshold). The frontend folds, it never judges:
    /// the row still arrives, search still reads it, nothing is
    /// deleted.
    pub stale: bool,
    /// The word has outlived its believability
    /// (`khor_node::SessionView::quiet`, 词的保鲜期批): grey the row
    /// and note how long the silence is — the word itself stands
    /// untouched, and the note's duration is the row's own `at_ms`
    /// read against now.
    pub quiet: bool,
    /// Whose session this is (`khor_core::Session::category`): `claude`,
    /// `codex`, `khor`, `shell`. `None` when nobody could place it, and
    /// the frontend must render that as its own group rather than
    /// picking a neighbour — **never inferred from the title here or
    /// there** (that is the whole reason it travels on the row).
    pub category: Option<String>,
    /// The group this row landed in, as `khor_node::list` keys them
    /// (`pin`, `cat:…`, `dev:…`, `state:…`, or empty when the mode does
    /// not group). The frontend starts a heading where this changes
    /// between neighbouring rows — **it never works out the grouping
    /// itself**, and the prefix tells it whether the rest is a word to
    /// look up or a name to print.
    pub group: String,
    /// The face of the machine this session lives on — derived here,
    /// never in the frontend (`khor_core::avatar`). `None` only when the
    /// home device is not in this table at all, and then the row draws a
    /// blank, **not an invented face**: a made-up face gets believed,
    /// and it is not that machine's.
    pub face: Option<Avatar>,
    /// This machine hosts the session's terminal (`Node::is_hosted`) — so
    /// a terminal can attach and paint it (docs/handoff 终端画屏). False
    /// for a discovered session, a `khor run`, or a remote one; those keep
    /// the transcript or the facts, not a live screen.
    pub hosted: bool,
    /// A terminal can be had here: hosted already, or something the
    /// attach bridge can stand a host up for on first open — a local
    /// discovered tmux session, or an agent sitting inside one (the
    /// row's own `Session::term`, via `Node::attach_tmux`). What the
    /// detail routes its terminal faces on — `hosted` alone would leave
    /// every one of those on the facts pane.
    pub attachable: bool,
    /// The row's preview line (`khor_core::Session::last`): the newest
    /// message, transcript utterance or terminal line — the kind judged
    /// it, this only paints it.
    pub last: Option<String>,
    /// The network exit this session borrows (device hex), when it does
    /// (docs/NET.md 借网 via 记号). No producer yet — lands with `--via`.
    pub via: Option<String>,
    /// The exit machine's face, looked up like `face` — the 行尾右下
    /// mini avatar. `None` with `via` set draws a blank, not an invented
    /// face (Avatar.tsx's rule).
    pub via_face: Option<Avatar>,
    /// Which machine this session lives on, by the name people call it.
    ///
    /// **Not the same fact as `source`.** `source` is the offline axis:
    /// it is set only on rows another device reported, so the rows that
    /// live here carry none — which is right for "how old is this
    /// report" and wrong for "which machine is this on". Filtering by
    /// machine needs the second question answered for *every* row,
    /// including this machine's own, or the axis is missing exactly the
    /// machine people reach for first and reads as broken.
    ///
    /// The device table's name, falling back to the id's hex — the same
    /// pair `list::place_of` builds for the device arrangement, so one
    /// machine is named the same in a chip and in a group heading.
    ///
    /// Derived here rather than in the frontend for the reason `face`
    /// is: the id→name table is the node's, and a screen that resolved
    /// it itself would be a second copy that can disagree.
    pub home: String,
    /// Where this session lives (`khor_core::Session::place`) — the
    /// project root, absolute, exactly as the group key spells it.
    ///
    /// **Absolute and unshortened**, though every screen prints it
    /// shortened: this is the value the `at:` filter compares against
    /// the group key, and a display string in that slot would be a
    /// second spelling of one fact. It is also what makes a row findable
    /// by the tree it is in.
    ///
    /// `None` for a row with no place — a conversation with another
    /// machine, or a session nobody recorded a directory for. The two
    /// are not distinguished here for the same reason the node does not
    /// distinguish them: the next move is the same.
    pub place: Option<String>,
    /// The session's process sits in a tmux session on its home
    /// machine (`khor_core::Session::mux`). **What the detail routes its
    /// default face on**, ahead of the kind: a conversation khor hosts
    /// that the person has also resumed in their own tmux is one `gui`
    /// row, and the terminal is where they are.
    pub mux: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct SourceTag {
    pub device: String,
    #[ts(type = "number")]
    pub age_ms: u64,
}

/// A machine's watched directories and how old that answer is — the
/// two axes `VitalsReading` keeps apart, for the rows of `Request::Watched`.
#[derive(Debug, Clone, Serialize, TS)]
pub struct WatchedReading {
    pub rows: Vec<khor_core::WatchedDir>,
    #[ts(type = "number")]
    pub age_ms: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DeviceRow {
    pub id: String,
    pub name: String,
    pub me: bool,
    /// This machine's face, in **its own** reported palette. See
    /// `khor_node::Node::face_of`.
    pub face: Option<Avatar>,
    /// Pinned, network-wide. Same rule as `SessionRow::pinned`: a mark to
    /// paint, never an ordering to redo.
    pub pinned: bool,
    /// What that machine is doing, and how long ago it said so. `None`
    /// when it has never been reached — see [`VitalsReading`].
    pub vitals: Option<VitalsReading>,
    /// Which road this machine is using to reach that one right now
    /// (`khor_core::Hop`), as its key — the word is looked up at paint
    /// time like every other.
    ///
    /// **Rides the row rather than arriving on its own**, so the whole
    /// table describes one instant: asked per machine it would be one
    /// reading per round trip, and two rows would be two moments.
    ///
    /// `None` for this machine, and that is the type doing the work: a
    /// machine does not reach itself over the network, so there is no
    /// reading rather than an unreadable one. Carried as a word every
    /// face had to remember to hide, the first face that forgot would
    /// print 说不出 on the row for the computer it is running on.
    pub hop: Option<String>,
    /// The machine this satellite speaks through, by name (附属机器批)
    /// — `None` for every ordinary machine. Where `hop` answers "which
    /// road", this answers "whose road": a satellite has none of its
    /// own, so faces print 「经 <sponsor>」 in the road's place.
    pub sponsor: Option<String>,
    /// The directories this machine is asked to keep an eye on — the
    /// setting, as kept on its row in the device table (2026-09-06
    /// 关注目录). What the editor on the card starts from.
    pub watch: Vec<String>,
    /// What the machine last said about them, with its own age — a
    /// record apart from `vitals` because the two requests fail apart
    /// (`Node::cache_peer_watched`). `None` when it never answered: an
    /// older khor refuses the op, and no rows is not the same fact as
    /// no directories.
    pub watched: Option<WatchedReading>,
    /// Where this machine sits when the network is drawn as a mandala
    /// (`khor_core::map`).
    ///
    /// **Derived here rather than by whoever paints it**, the same rule
    /// the face follows: a picture two ends work out separately is two
    /// pictures. It rides the row because a seat *is* a property of the
    /// placed machine — and because the seat of one depends on the whole
    /// set, which is a thing only the code building the whole set knows.
    pub seat: Seat,
}

/// A machine's reading plus its age — the two axes docs/SESSION.md keeps
/// apart, here for a machine instead of a session.
///
/// **The age is not decoration and it is not optional.** A number with no
/// age is read as the present, so a machine that has been off since
/// yesterday would show yesterday's CPU as if it were now. `age_ms` is
/// zero exactly when the reading was taken to answer this call, which is
/// the case for this machine and no other.
///
/// The row carries `None` when the machine has never answered at all.
/// That is a third state on purpose: "not asked yet" and "asked an hour
/// ago" send a person to look at different things.
#[derive(Debug, Clone, Serialize, TS)]
pub struct VitalsReading {
    pub vitals: Vitals,
    #[ts(type = "number")]
    pub age_ms: u64,
    /// The strain words for memory, disk and video memory, minted here
    /// so the screen paints a judgment instead of making one (the
    /// thresholds and their reasons live on `khor_core::Fill::strain`).
    /// CPU and GPU utilisation carry no
    /// word on purpose: a pinned core is a machine doing its job, a full
    /// disk is a machine about to stop doing it.
    pub mem_strain: Option<khor_core::Strain>,
    pub disk_strain: Option<khor_core::Strain>,
    /// Video memory is a `Fill` like the other two — it has a "full" to
    /// be near — and it was the one the gauges whispered about while it
    /// screamed (仪表环出声批's causing case: a 90% VRAM ring wearing the
    /// same grey as a 2% CPU). Minted from the same `Fill::strain`, so
    /// the thresholds stay one fact in one place.
    pub vram_strain: Option<khor_core::Strain>,
    /// One word per volume, index-parallel with `vitals.volumes` —
    /// minted here like the three above, from the same `Fill::strain`,
    /// so the per-volume rows the card grew on 2026-09-02 paint the
    /// same judgment the single disk ring always did.
    pub volume_strains: Vec<Option<khor_core::Strain>>,
}

impl VitalsReading {
    fn of(vitals: Vitals, age_ms: u64) -> VitalsReading {
        VitalsReading {
            mem_strain: vitals.mem.strain(),
            disk_strain: vitals.disk.and_then(|d| d.strain()),
            vram_strain: vitals.gpu.as_ref().and_then(|g| g.mem).and_then(|m| m.strain()),
            volume_strains: vitals.volumes.iter().map(|v| v.fill.strain()).collect(),
            vitals,
            age_ms,
        }
    }
}

#[cfg(test)]
mod vitals_reading_tests {
    use super::*;
    use khor_core::{Fill, Strain};

    /// Memory strained, disk fine, VRAM tight — pairwise asymmetric on
    /// purpose. A symmetric fixture would stay green with two
    /// assignments swapped, and a swap is the one mistake this
    /// constructor has room for. The words differ too (Critical vs
    /// Tight), so mem↔vram swapped also goes red.
    #[test]
    fn each_strain_word_comes_from_its_own_fill() {
        let v = Vitals {
            cpu_pct: 99.0, // pinned CPU carries no word, also on purpose
            cores: 8,
            mem: Fill { used: 96, total: 100 },
            disk: Some(Fill { used: 50, total: 100 }),
            gpu: Some(khor_core::Gpu {
                util_pct: 99.0, // pinned GPU carries no word either
                cards: 1,
                mem: Some(Fill { used: 91, total: 100 }),
            }),
            version: None,
            gpus: vec![],
            volumes: vec![
                khor_core::Volume {
                    mount: "/".to_owned(),
                    fill: Fill { used: 96, total: 100 },
                },
                khor_core::Volume {
                    mount: "/data".to_owned(),
                    fill: Fill { used: 50, total: 100 },
                },
            ],
            uptime_s: 0,
        };
        let r = VitalsReading::of(v, 0);
        assert_eq!(r.mem_strain, Some(Strain::Critical));
        assert_eq!(r.disk_strain, None, "a half-full disk says nothing");
        assert_eq!(r.vram_strain, Some(Strain::Tight), "a 91% VRAM is worth a glance");
        assert_eq!(
            r.volume_strains,
            vec![Some(Strain::Critical), None],
            "each volume wears its own word, in its own seat"
        );
    }

    /// No GPU, no vram word — the absent card must not read as anything.
    #[test]
    fn a_machine_without_a_gpu_says_nothing_about_vram() {
        let v = Vitals {
            cpu_pct: 1.0,
            cores: 8,
            mem: Fill { used: 10, total: 100 },
            disk: None,
            gpu: None,
            version: None,
            gpus: vec![],
            volumes: vec![],
            uptime_s: 0,
        };
        let r = VitalsReading::of(v, 0);
        assert_eq!(r.vram_strain, None);
        assert!(r.volume_strains.is_empty(), "no volumes, no words");
    }
}

/// One option on the variant or shape axis, already painted.
///
/// The face is **this machine's real face under that option**, derived by
/// `khor_node::Node::face_under` — the same function and the same seed
/// every row's face goes through. Letting the frontend build previews
/// would put a second painter in the one place a second painter costs
/// most: a preview is all the evidence anybody has before pressing.
#[derive(Debug, Clone, Serialize, TS)]
pub struct FaceChoice {
    /// The stable key (`Variant::key`, `FaceShape::key`). The word beside
    /// it is looked up in the catalog at paint time, never carried here.
    pub key: String,
    pub face: Avatar,
}

/// A factory palette, painted, and the five slots it would set.
///
/// It carries its colors because **a palette only ever reaches the
/// library as five colors** (`Node::restyle`). Pressing a factory button
/// and dragging one well then travel the same path, so there is no
/// second way for a palette to arrive that could behave differently.
#[derive(Debug, Clone, Serialize, TS)]
pub struct PaletteChoice {
    /// The palette's `id` (`khor_core::avatar::PRESETS`).
    pub key: String,
    pub colors: Vec<String>,
    pub face: Avatar,
}

/// What this machine is wearing, and every option painted as it would
/// look on it.
///
/// **One call, not one per option**, and each option's face derived with
/// the *other two axes left where they are* — so a swatch is literally
/// "what you get if you press this" rather than a face assembled out of
/// factory defaults that nobody would ever see.
#[derive(Debug, Clone, Serialize, TS)]
pub struct FaceChoices {
    /// The face as it stands — the same picture the rail and this
    /// machine's own device row are painting.
    pub face: Avatar,
    /// The five slots as they stand, `#rrggbb`.
    pub colors: Vec<String>,
    /// Which factory palette those five are, when they are one. `None`
    /// once a slot has been changed by hand, and then **no** palette is
    /// marked: a nearest match would light a button nobody pressed, and
    /// pressing that lit button would then look like a control that does
    /// nothing (docs/UX.md 做了但没变化).
    pub preset: Option<String>,
    pub variant: String,
    pub shape: String,
    pub palettes: Vec<PaletteChoice>,
    pub variants: Vec<FaceChoice>,
    pub shapes: Vec<FaceChoice>,
}

fn open(root: &Path) -> Result<Node, String> {
    Node::open(root.to_path_buf())
}

/// Everything the settings screen paints. The order of each axis is the
/// library's (`PRESETS` is ordered by provenance, `ALL` by the order a
/// chooser should offer them); this layer picks none of it.
pub fn face_choices(root: &Path) -> Result<FaceChoices, String> {
    let n = open(root)?;
    let now = n.avatar_style();
    Ok(FaceChoices {
        face: n.face_under(&now),
        colors: now.palette.colors().to_vec(),
        preset: now.palette.preset_id().map(str::to_owned),
        variant: now.variant.key().to_owned(),
        shape: now.shape.key().to_owned(),
        palettes: PRESETS
            .iter()
            .map(|p| PaletteChoice {
                key: p.id.to_owned(),
                colors: p.colors.iter().map(|c| c.to_string()).collect(),
                face: n.face_under(&AvatarStyle { palette: p.palette(), ..now.clone() }),
            })
            .collect(),
        variants: Variant::ALL
            .iter()
            .map(|v| FaceChoice {
                key: v.key().to_owned(),
                face: n.face_under(&AvatarStyle { variant: *v, ..now.clone() }),
            })
            .collect(),
        shapes: FaceShape::ALL
            .iter()
            .map(|s| FaceChoice {
                key: s.key().to_owned(),
                face: n.face_under(&AvatarStyle { shape: *s, ..now.clone() }),
            })
            .collect(),
    })
}

/// Changes what this machine wears — the call `khor face` makes. One
/// function behind the verb and behind every control, so the two faces
/// cannot come to mean different things by the same words.
pub fn restyle(
    root: &Path,
    colors: Option<Vec<String>>,
    variant: Option<String>,
    shape: Option<String>,
) -> Result<(), String> {
    open(root)?
        .restyle(colors.as_deref(), variant.as_deref(), shape.as_deref())
        .map(|_| ())
}

/// The list, arranged. `by` is a `khor_node::list::Arrange` key; an
/// unknown one is refused by name rather than quietly falling back to
/// the default — a typo'd preference would otherwise look like the
/// setting silently not working.
pub fn list_sessions(root: &Path, by: &str) -> Result<Vec<SessionRow>, String> {
    let mode = Arrange::from_key(by).ok_or_else(|| {
        let all: Vec<&str> = Arrange::ALL.iter().map(|a| a.key()).collect();
        khor_catalog::msg::not_a_way_to_arrange(by, all.join(khor_catalog::cli::NAME_SEPARATOR))
    })?;
    let n = open(root)?;
    // Derived once per list, not once per row: a face is a pure function
    // of (id, reported style), and one machine owns many rows.
    let faces = faces_by_id(&n)?;
    // The same id→name table `Node::sessions_arranged` hands the
    // arranger, built once here for the same reason the faces are.
    let machines: HashMap<String, String> =
        n.devices()?.into_iter().map(|d| (d.id, d.name)).collect();
    Ok(n.sessions_arranged(mode)?
        .into_iter()
        .map(|a| {
            let (v, group) = (a.view, a.group);
            let hosted = n.is_hosted(&v.session.id);
            // The row itself says whether its home can stand a terminal
            // up (`Session::term` — hosted, a discovered tmux session,
            // or an agent sitting inside one), and that answer **travels
            // with the row**: a session on another machine is reached,
            // not hosted here (`Node::reach` binds a local port onto its
            // host, and `term_open` connects to that like any address).
            //
            // A locality gate used to sit here, from when the terminal
            // could not travel. It outlived the reason for it, and what
            // it produced was a row that opened onto the bare facts pane
            // — which reads, correctly, as "clicking this does nothing".
            let attachable = hosted || v.session.term;
            SessionRow {
            attachable,
            home: machines
                .get(&v.session.home.hex())
                .cloned()
                .unwrap_or_else(|| v.session.home.hex()),
            face: faces.get(&v.session.home.hex()).cloned(),
            hosted,
            via_face: v.session.via.as_ref().and_then(|d| faces.get(d).cloned()),
            last: v.session.last,
            via: v.session.via,
            id: v.session.id.0,
            kind: v.session.kind.0,
            title: v.session.title,
            word: v.session.state.state.key().to_owned(),
            at_ms: v.session.state.at.0,
            unread: v.session.unread,
            source: v.source.map(|(device, age_ms)| SourceTag { device, age_ms }),
            pinned: v.pinned,
            stale: v.stale,
            quiet: v.quiet,
            category: v.session.category,
            place: v.session.place,
            mux: v.session.mux,
            group,
        }
        })
        .collect())
}

/// One directory the person works in, for the wizard's first question
/// (`khor_node::place`).
#[derive(Debug, Clone, Serialize, TS)]
pub struct PlaceRow {
    /// Absolute, and what gets sent back when it is picked.
    pub path: String,
    /// The same path as a person reads it, home written `~`. Shortened
    /// here rather than in the face: it depends on whose home this is,
    /// which is a fact about the machine and not about the screen.
    pub shown: String,
    /// How many recorded conversations place themselves here. Shown as
    /// a count and never as a total — `Node::places` says what it does
    /// and does not see.
    pub sessions: u32,
    #[ts(type = "number")]
    pub last_ms: i64,
}

/// How many places the wizard is offered. **A shortlist, not a
/// history**: it exists to save typing a path that is one click away,
/// and a person whose place is not in it types it in the box that is
/// still right there.
const PLACES_SHOWN: usize = 8;

/// One ring on the 额度 page: an account's reading, whose it is and how
/// old it is, with the strain decided by the node (`accounts::strain_of`)
/// so the face only paints.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AccountRing {
    pub reading: khor_core::AccountReading,
    /// How long ago the reading was carried here — 0 for this machine's
    /// own, which is read now.
    #[ts(type = "number")]
    pub age_ms: u64,
    /// The name of the machine holding the key (`reading.home` is its id).
    pub home_name: String,
    pub mine: bool,
    pub strain: Option<khor_core::Strain>,
    pub base_url: Option<String>,
    #[ts(type = "number | null")]
    pub budget: Option<f64>,
    /// The unit the person set on this account, or `None` where they
    /// have not — **for the edit form's value, not for display**.
    ///
    /// What a screen prints is `reading.currency`, which already has
    /// this applied (`Node::quota_board`, one place for both faces).
    /// The form needs the raw fact instead: an empty box there means
    /// 「use what the endpoint said」, and it may only be empty when
    /// that is actually true.
    pub unit: Option<String>,
    /// Floated to the top of the list, on every machine
    /// (`khor_sync::accounts::Account::pinned`). Carried here rather
    /// than read a second time by the face: the order is the doc's, and
    /// a screen sorting by its own copy of the rule is how two faces
    /// come to disagree about what is at the top.
    pub pinned: bool,
}

/// Every registered account's ring (`Node::quota_board`), home machines
/// named.
pub async fn quota_board(root: &Path) -> Result<Vec<AccountRing>, String> {
    let n = open(root)?;
    let me = n.device_str().to_owned();
    let names: std::collections::HashMap<String, String> =
        n.devices()?.into_iter().map(|d| (d.id, d.name)).collect();
    let accounts = n.accounts()?;
    Ok(n.quota_board()
        .await?
        .into_iter()
        .map(|(reading, age_ms)| {
            let account = accounts.iter().find(|a| a.id == reading.account);
            AccountRing {
                home_name: names.get(&reading.home).cloned().unwrap_or_else(|| reading.home.clone()),
                mine: reading.home == me,
                strain: khor_node::accounts::strain_of(reading.fill),
                base_url: account.and_then(|a| a.base_url.clone()),
                budget: account.and_then(|a| a.budget),
                unit: account.and_then(|a| a.unit.clone()),
                pinned: account.is_some_and(|a| a.pinned),
                age_ms,
                reading,
            }
        })
        .collect())
}

/// 登记 (`Node::register_account`): the provider word is parsed here so
/// the face and the CLI refuse an unknown one with the same sentence.
/// Answers the new account's id.
pub fn register_account(
    root: &Path,
    name: &str,
    provider: &str,
    base_url: Option<&str>,
    key: &str,
    budget: Option<f64>,
) -> Result<String, String> {
    let provider = khor_node::Provider::parse(provider)
        .ok_or_else(|| khor_catalog::msg::no_such_provider(provider))?;
    Ok(open(root)?.register_account(name, provider, base_url, key, budget)?.id)
}

pub fn remove_account(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.remove_account(id)
}

/// 置顶 / 取消置顶 (`Node::pin_account`). The flag rides the replicated
/// doc, so this pins on every machine.
pub fn pin_account(root: &Path, id: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_account(id, on)
}

/// 编辑 (`Node::update_account`): name, address, budget, unit, and
/// optionally a new key. The provider is not editable — see the node's
/// own head.
///
/// `unit` absent and `unit` empty are the same thing on the way in —
/// 「use what the endpoint said」 — and the node is where that is
/// settled, so this layer does not second-guess it.
pub fn update_account(
    root: &Path,
    id: &str,
    name: &str,
    base_url: Option<&str>,
    key: Option<&str>,
    budget: Option<f64>,
    unit: Option<&str>,
) -> Result<(), String> {
    open(root)?.update_account(id, name, base_url, key, budget, unit)
}

/// 常去的地方, newest first, **on the machine the wizard names**
/// (`khor_node::Node::places_on`; empty means this one). `shown` comes
/// from the answering machine — whose home a path is under is its
/// fact, not this screen's. An older khor over there answers with no
/// list, and the wizard says nothing where it has nothing.
pub async fn places(root: &Path, machine: &str) -> Result<Vec<PlaceRow>, String> {
    let n = open(root)?;
    Ok(n.places_on(machine)
        .await?
        .into_iter()
        .take(PLACES_SHOWN)
        .map(|(path, shown, sessions, last_ms)| PlaceRow { path, shown, sessions, last_ms })
        .collect())
}

/// What a machine is busy with — its process top and CPU day
/// (`khor_node::Node::load_on`; a name, or empty for this one). An
/// older khor over there is a refusal with its own sentence, which the
/// card prints where the table would be.
pub async fn load(
    root: &Path,
    machine: &str,
    top: u32,
    by: khor_core::TopBy,
    desc: bool,
) -> Result<khor_core::Load, String> {
    open(root)?.load_on(machine, top, by, desc).await
}

/// One cell of a day strip, in the shape the frontend can paint.
///
/// **Three cases and three TS shapes, on purpose.** The words are one
/// namespace and the two nothings are outside it: an externally tagged
/// enum lands in TypeScript as `{ word: string } | "unwatched" |
/// "absent"`, so `var(--state-${…})` cannot be reached for a nothing.
/// Spelled as one string in one namespace, `--state-unwatched` would
/// resolve to nothing and paint transparent — silently folding 不知道
/// into 没有这一行, which is the exact collapse `khor_core::DayCell`
/// exists to prevent, re-introduced in CSS.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DayMark {
    /// A state key (`busy`…), looked up like every other word.
    Word(String),
    Unwatched,
    Absent,
}

/// A stretch of identical cells: what the face paints as one block, as
/// wide as `cells` out of the total it asked for.
#[derive(Debug, Clone, Serialize, TS)]
pub struct DayRun {
    pub mark: DayMark,
    #[ts(type = "number")]
    pub cells: u32,
}

/// Each row's last 24 hours, keyed by session id
/// (`khor_node::day`).
///
/// **Its own call on its own beat, not a field on [`SessionRow`].** The
/// row list is polled every two seconds; a day of cells hung on it would
/// be a log riding a heartbeat. Nothing here judges: `cells` is a pixel
/// count the screen measured, and every merge down to it happened in the
/// node (`khor_core::DayCell::rank`).
pub fn day_strips(
    root: &Path,
    cells: u32,
) -> Result<std::collections::BTreeMap<String, Vec<DayRun>>, String> {
    let n = open(root)?;
    Ok(n.day_strips(cells as usize)?
        .into_iter()
        .map(|(id, runs)| {
            let runs = runs
                .into_iter()
                .map(|r| DayRun {
                    mark: match r.cell {
                        khor_core::DayCell::Word(word) => DayMark::Word(word.key().to_owned()),
                        khor_core::DayCell::Unwatched => DayMark::Unwatched,
                        khor_core::DayCell::Absent => DayMark::Absent,
                    },
                    cells: r.cells,
                })
                .collect();
            (id, runs)
        })
        .collect())
}

/// Async only because of the road reading: it is the resident's answer,
/// fetched once for the whole table (`khor_node::Node::hops`).
pub async fn list_devices(root: &Path) -> Result<Vec<DeviceRow>, String> {
    let n = open(root)?;
    let me = n.device_str().to_owned();
    let all = n.devices()?;
    let hops = n.hops().await;
    // The ring is as big as the network minus this machine, and the
    // seats are handed out in the order the node listed them — so a
    // machine keeps its seat for as long as the list keeps its shape,
    // and nothing here decides an order the node did not.
    let mut ring = map::ring(all.iter().filter(|d| d.id != me).count()).into_iter();
    // id → name for the satellite rows: their road column names the
    // sponsor they speak through, and a screen prints names.
    let names: std::collections::HashMap<String, String> =
        all.iter().map(|d| (d.id.clone(), d.name.clone())).collect();
    Ok(all
        .into_iter()
        .map(|d| DeviceRow {
            sponsor: d
                .sponsor
                .as_ref()
                .map(|sp| names.get(sp).cloned().unwrap_or_else(|| sp.clone())),
            seat: if d.id == me {
                map::CENTRE
            } else {
                // One seat per machine on the ring, in step with the
                // count above: `ring` was built from exactly this many.
                ring.next().unwrap_or(map::CENTRE)
            },
            me: d.id == me,
            hop: (d.id != me).then(|| {
                hops.iter()
                    .find(|(id, _)| *id == d.id)
                    .map(|(_, h)| *h)
                    .unwrap_or(khor_node::Hop::Unknown)
                    .key()
                    .to_owned()
            }),
            face: n.face_of(&d),
            pinned: d.pinned,
            // Sampled for this machine, read from the last visit for
            // every other. Which one it was is readable from `age_ms`
            // alone, so no second field says it twice.
            vitals: n
                .vitals_of(&d.id)
                .map(|(vitals, age_ms)| VitalsReading::of(vitals, age_ms)),
            watched: n.watched_of(&d.id).map(|(rows, age_ms)| WatchedReading { rows, age_ms }),
            watch: d.watch,
            id: d.id,
            name: d.name,
        })
        .collect())
}

/// What every machine in the network has spent, by day and by vendor —
/// the call `khor usage` makes, with no window applied.
///
/// **No window, unlike the verb.** A terminal prints once and scrolls
/// away, so `khor usage` takes a number of days; a screen scrolls, so the
/// answer arrives whole and how far back somebody looks is how far they
/// scroll. Same node call either way; what differs is what a face can do
/// with it.
pub fn usage(root: &Path) -> Result<Usage, String> {
    open(root)?.usage_everywhere()
}

/// What this machine's Claude subscription has left.
///
/// **Never fails.** Every way this can go wrong is something the person
/// might act on — no login, an expired token, a cooldown with a stated
/// wait — so a reason travels as data beside the reading rather than as
/// an error that a screen would have to turn back into words. A panel
/// that just stayed empty during a ten-minute cooldown would be
/// indistinguishable from one that is broken.
///
/// **No `root` argument, unlike its neighbours here.** This is not a
/// fact about a khor installation; it is a fact about an *account*, read
/// through the login Claude Code already stored on this machine. The
/// same account seen from two machines is one quota, not two.
pub async fn quota() -> QuotaAnswer {
    match khor_node::quota::read().await {
        Ok(quota) => QuotaAnswer {
            quota: Some(quota),
            trouble: None,
        },
        Err(trouble) => QuotaAnswer {
            quota: None,
            trouble: Some(trouble),
        },
    }
}

/// Pins or unpins a session — the call `khor pin <session>` makes. One
/// function behind the verb and the button, so the two cannot drift.
pub fn pin_session(root: &Path, id: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_session(&SessionId(id.to_owned()), on)
}

/// Pins or unpins a machine — the call `khor pin -m <machine>` makes.
pub fn pin_device(root: &Path, machine: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_device(machine, on)
}

/// Sets a machine's watch list — the call `khor watch <machine> …`
/// makes, from the card's editor. The refusals (a relative path, a
/// `..`, a duplicate) are the node's and come back as its sentence;
/// answers the list as kept, which the editor repaints from.
pub fn set_watch(root: &Path, machine: &str, paths: &[String]) -> Result<Vec<String>, String> {
    open(root)?.set_watch(machine, paths)
}

/// Shows a machine the door — the call `khor forget <machine>` makes,
/// from the machine row's menu. The judgment is `Node::forget_device`'s
/// (a network-wide tombstone, and this machine refusing to forget
/// itself); this answers the name of the machine that left.
pub fn forget_device(root: &Path, machine: &str) -> Result<String, String> {
    open(root)?.forget_device(machine)
}

/// Every known machine's face, keyed the way the device table keys
/// machines. The judgment behind each one — its own reported palette,
/// seeded by its id — belongs to the node; this only indexes them.
fn faces_by_id(n: &Node) -> Result<HashMap<String, Avatar>, String> {
    Ok(n.devices()?
        .into_iter()
        .filter_map(|d| n.face_of(&d).map(|f| (d.id, f)))
        .collect())
}

pub fn seen(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.seen(&SessionId(id.to_owned()))
}

/// Names a session, for everyone; an empty title takes the name back so
/// the row falls back to its derived title (`Node::rename_session`).
/// One function behind the double-click and any future verb, so the two
/// cannot drift.
pub fn rename_session(root: &Path, id: &str, title: &str) -> Result<(), String> {
    open(root)?.rename_session(&SessionId(id.to_owned()), title)
}

pub fn close_session(root: &Path, id: &str) -> Result<(), String> {
    // Routed: a row on another machine closes where it runs
    // (`Node::close_anywhere`). The app's own runtime is the caller's,
    // so this one is built here the way every blocking door in this
    // file is.
    let n = open(root)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?
        .block_on(n.close_anywhere(&SessionId(id.to_owned())))
}

/// Leaves a line in a machine's window — the call `khor tell` makes.
pub fn tell(root: &Path, machine: &str, text: &str) -> Result<(), String> {
    open(root)?.tell(machine, text).map(|_| ())
}

/// Whether claude on **this** machine is set up to tell khor what it is
/// doing.
///
/// One bit, because the control it feeds has two states and no third.
/// `khor hooks` is where the per-event detail lives, including the case
/// this bit folds in: hooks recorded here that point at a *different*
/// khor read as not installed, which is right for a button — pressing
/// install is exactly what fixes them.
#[derive(Debug, Clone, Serialize, TS)]
pub struct HooksState {
    /// True when every event khor asks for names this binary.
    pub installed: bool,
}

fn hooks_state_of(n: &Node) -> Result<HooksState, String> {
    use khor_node::adaptor::claude::Installed;
    let report = n.hooks_report()?;
    Ok(HooksState { installed: report.events.iter().all(|(_, s)| *s == Installed::Here) })
}

pub fn hooks_state(root: &Path) -> Result<HooksState, String> {
    hooks_state_of(&open(root)?)
}

/// Adds them, and answers with the state afterwards.
///
/// **The new state comes back from the same call**, so the button cannot
/// spend a moment showing the old one — and it is re-read rather than
/// assumed, which is what makes a half-applied install visible instead
/// of a button that claims success because the call returned.
pub fn install_hooks(root: &Path) -> Result<HooksState, String> {
    let n = open(root)?;
    n.install_hooks()?;
    hooks_state_of(&n)
}

pub fn uninstall_hooks(root: &Path) -> Result<HooksState, String> {
    let n = open(root)?;
    n.uninstall_hooks()?;
    hooks_state_of(&n)
}

/// A freshly minted ticket, and how long it is good for.
///
/// **The window travels with the ticket instead of being written into
/// the screen that shows it.** A dialog holding its own `15` would be a
/// second copy of a number only `khor_node::link::INVITE_WINDOW_MS`
/// enforces, and the day that constant moves the app would go on
/// promising the old one — a sentence nobody can catch by reading either
/// file alone, since neither would be wrong on its own.
///
/// Minutes, because that is the unit the words for it use
/// (`invite-window` on this side, `invite-expired` on the refusal), and
/// the conversion happens once in the library.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Ticket {
    pub token: String,
    pub minutes: u32,
}

/// 接管 (批C): the chat face becomes the session's one body — the node
/// ends the terminal side and resumes the conversation (`Node::takeover`).
pub fn takeover(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.takeover(&SessionId(id.to_owned()))
}

/// 接管 the other way: a conversation khor holds moves into a terminal
/// (`Node::takeover_term`). Same row, same conversation — the form is
/// what changes hands.
pub fn takeover_term(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.takeover_term(&SessionId(id.to_owned()))
}

/// One registered ACP agent, for the wizard's 智能体 list (批⑥).
///
/// The command is spelled the way the person typed it rather than as
/// the launch JSON it is stored in: this is a label under a name, and
/// reading the JSON back would be reading an implementation.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentRow {
    pub name: String,
    pub command: String,
}

/// Every ACP agent this person has named (`khor_sync::agents`), sorted.
///
/// khor's own two are **not** in here: they are not registrations, they
/// have shims of their own, and a wizard that listed them from the same
/// place would have to invent entries nobody made. The face puts them
/// beside this list, which is what they are.
pub fn agents(root: &Path) -> Result<Vec<AgentRow>, String> {
    Ok(open(root)?
        .agents()?
        .into_iter()
        .map(|(name, spec)| AgentRow { name, command: spec.typed() })
        .collect())
}

/// The wizard's door: a fresh agent session where it pointed, **on the
/// machine it named** (`machine` empty or this machine's own name is
/// here). The judgment — which agent, which form, which directory,
/// which title — lives in [`khor_node::Node::start_session`] and runs on
/// the machine that will host the session: here directly, or over
/// there through [`khor_node::Node::open_agent_on`], which carries the
/// four answers and lets that machine resolve them against its own
/// registrations and paths. Answers the new row's id, which every face
/// will call it by wherever it was born.
pub async fn open_session(
    root: &Path,
    dir: &str,
    title: &str,
    form: &str,
    agent: &str,
    machine: &str,
) -> Result<String, String> {
    let n = open(root)?;
    if machine.is_empty() || machine == n.name() {
        return n.start_session(dir, title, form, agent).map(|id| id.0);
    }
    n.open_agent_on(machine, dir, title, form, agent).await.map(|id| id.0)
}

/// One message in a machine's conversation, shaped for painting. The
/// words and the order are the node's (`Node::log_faced`); this only
/// flattens the enum so a TS switch cannot invent a fourth arm.
#[derive(Debug, Clone, Serialize, TS)]
pub struct ChatMsgRow {
    pub id: String,
    #[ts(type = "number")]
    pub at_ms: i64,
    /// The sender's name at the time of writing — history must not
    /// rewrite itself on a later rename (`khor_sync::chat::Sender`).
    pub from: String,
    pub from_me: bool,
    pub text: Option<String>,
    pub files: Option<Vec<ChatFileRow>>,
    /// A body kind this build cannot read, by its kind name — said out
    /// loud rather than rendered as blank text (`MsgBody::Unknown`'s
    /// rule).
    pub unknown: Option<String>,
    pub retracted: bool,
    pub edited: bool,
    /// The file message's transfer face on this machine, as a state
    /// key (`TransferKind::message_face`) — `None` is "not my seat",
    /// never "done". The frontend looks the word up and never derives
    /// one (docs/UX.md 状态呈现).
    pub word: Option<String>,
    /// The sender's neutral account of the offer, as a delivery key
    /// (`TransferKind::Delivery`: waiting/fetching/served — 发端中性词批).
    /// Not a state word: it never joins the six and never wears their
    /// colors. At most one of `word` and `delivery` is present — the
    /// seats are exclusive.
    pub delivery: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ChatFileRow {
    pub name: String,
    #[ts(type = "number")]
    pub size: i64,
}

/// The whole conversation readout.
#[derive(Debug, Clone, Serialize, TS)]
pub struct MachineLog {
    pub messages: Vec<ChatMsgRow>,
    /// Blocks that would not read — a missing slice of conversation
    /// the screen must be able to mention (`chat::ChannelLog`).
    #[ts(type = "number")]
    pub broken: u64,
}

/// A machine conversation's readout — what `khor log <机器>` prints,
/// shaped for the chat pane. Takes the row's id (`chat/<channel>`) and
/// peels the prefix here: the caller holds a row, and which string is
/// the channel is this layer's knowledge, not the screen's.
pub fn machine_log(root: &Path, id: &str) -> Result<MachineLog, String> {
    let n = open(root)?;
    let machine = id.strip_prefix("chat/").unwrap_or(id);
    let (log, faces) = n.log_faced(machine)?;
    let me = n.device_str().to_owned();
    let messages = log
        .messages
        .into_iter()
        .zip(faces)
        .map(|(m, (face, delivery))| {
            let (text, files, unknown) = match m.body {
                khor_node::MsgBody::Text(t) => (Some(t), None, None),
                khor_node::MsgBody::Files(fs) => (
                    None,
                    Some(
                        fs.into_iter()
                            .map(|f| ChatFileRow { name: f.name, size: f.size })
                            .collect(),
                    ),
                    None,
                ),
                khor_node::MsgBody::Unknown(k) => (None, None, Some(k)),
            };
            ChatMsgRow {
                at_ms: m.at,
                from_me: m.from.id == me,
                from: m.from.name,
                id: m.id,
                text,
                files,
                unknown,
                retracted: m.retracted,
                edited: m.edited,
                word: face.map(|s| s.key().to_owned()),
                delivery: delivery.map(|d| d.key().to_owned()),
            }
        })
        .collect();
    Ok(MachineLog { messages, broken: log.broken as u64 })
}

/// Sends a file the sender names by path — `khor send <machine> <file>`
/// through the same node call (docs/KHOR.md equivalence). The tauri
/// skin reaches this after its native picker answered with a real local
/// path; the browser faces reach it after [`save_upload`] turned bytes
/// into one. Answers the transfer session id, the CLI's own deliverable.
pub fn send_path(root: &Path, machine: &str, path: &Path) -> Result<String, String> {
    Ok(open(root)?.send(machine, path)?.0)
}

/// The most bytes one browser upload may carry: 1 GiB.
///
/// One constant, one place — the LAN listener and the dev bridge both
/// enforce it, and two copies is how a file gets refused on one face
/// and taken on the other. The number: the upload crosses one LAN hop
/// and streams to disk (never through memory whole), so the cap is not
/// protecting RAM — it bounds how much disk a single unauthenticated-
/// by-mistake or fat-fingered POST can commit, while staying above a
/// phone video, the largest thing this pipe is for. Bigger things have
/// `khor send` with a real path on the machine that has them.
pub const UPLOAD_CAP_BYTES: u64 = 1 << 30;

/// Streams one uploaded file to the sender's own staging area and
/// answers the staged path, refusing past [`UPLOAD_CAP_BYTES`] with the
/// catalog's sentence.
///
/// The staged file is **the offer's source, not a temporary**: `send`
/// records its path and serves the far side's fetch from it (the chat
/// document carries only the summary — payload bytes exist nowhere else
/// on the sending machine), so it must outlive the transfer the way a
/// CLI send's real path does. What must NOT survive is a failed upload
/// or a failed send — the caller deletes the staged file on any error
/// after this returns, and this function deletes it on its own errors,
/// so no orphan marks a send that never happened.
///
/// The name lands under the same whitelist the receiving side applies
/// (`transfer::safe_name` is not exported; the cleaning here mirrors
/// it): the browser hands over whatever it likes, and a name is not a
/// path.
pub fn save_upload(
    root: &Path,
    name: &str,
    body: &mut dyn std::io::Read,
) -> Result<std::path::PathBuf, String> {
    use std::io::Write;
    let clean: String = name.chars().filter(|c| !matches!(c, '/' | '\\' | '\0')).collect();
    let clean = clean.trim_start_matches('.');
    let clean = if clean.is_empty() { "file" } else { clean };
    let dir = root.join(".khor").join("uploads");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Epoch-ms plus pid in the name: two uploads of one file must not
    // overwrite each other while the first is still being fetched.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = dir.join(format!("{stamp}-{}-{clean}", std::process::id()));
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut taken: u64 = 0;
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = match body.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let _ = std::fs::remove_file(&path);
                return Err(e.to_string());
            }
        };
        taken += n as u64;
        if taken > UPLOAD_CAP_BYTES {
            let _ = std::fs::remove_file(&path);
            return Err(khor_catalog::msg::upload_too_big(UPLOAD_CAP_BYTES >> 20));
        }
        if let Err(e) = f.write_all(&buf[..n]) {
            let _ = std::fs::remove_file(&path);
            return Err(e.to_string());
        }
    }
    Ok(path)
}

/// The upload route's two arguments off its query string,
/// percent-decoded — a file name is user text and arrives encoded.
/// Both required: an upload with no machine has nowhere to go, one
/// with no name would invent one. Here rather than in each server so
/// the web face and the dev bridge parse one contract.
pub fn upload_args(query: &str) -> Option<(String, String)> {
    let mut machine = None;
    let mut name = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=')?;
        match k {
            "machine" => machine = Some(percent_decode(v)?),
            "name" => name = Some(percent_decode(v)?),
            _ => {}
        }
    }
    match (machine, name) {
        (Some(m), Some(n)) if !m.is_empty() && !n.is_empty() => Some((m, n)),
        _ => None,
    }
}

/// Minimal percent-decoding (RFC 3986): enough for what
/// `encodeURIComponent` produces, refusing malformed escapes rather
/// than guessing at them. No crate — a dozen lines is not a dependency.
fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'%' => {
                let hi = char::from(bytes.next()?).to_digit(16)?;
                let lo = char::from(bytes.next()?).to_digit(16)?;
                out.push((hi * 16 + lo) as u8);
            }
            b'+' => out.push(b' '),
            other => out.push(other),
        }
    }
    String::from_utf8(out).ok()
}

/// One browser upload, end to end: stage the bytes, then send the
/// staged path — the shared body behind the web face's and the dev
/// bridge's upload routes, so the two cannot come to enforce different
/// rules. On a failed send the staged file goes too (the module's
/// no-orphan rule); on success it stays, because it is what the far
/// side's fetch reads.
pub fn send_upload(
    root: &Path,
    machine: &str,
    name: &str,
    body: &mut dyn std::io::Read,
) -> Result<String, String> {
    let staged = save_upload(root, name, body)?;
    match send_path(root, machine, &staged) {
        Ok(id) => Ok(id),
        Err(e) => {
            let _ = std::fs::remove_file(&staged);
            Err(e)
        }
    }
}

/// The waiting file message's 收下 — `khor accept`, through the same
/// node call (docs/KHOR.md equivalence). Takes the message id the chip
/// carries; the transfer session id is spelled here because it is the
/// node's address scheme, not the screen's. Answers where the files
/// landed, the CLI's own deliverable.
pub async fn accept_transfer(root: &Path, msg: &str) -> Result<Vec<String>, String> {
    let n = open(root)?;
    let (_, landed) = n
        .accept(&khor_node::transfer::TransferKind::session_id(msg))
        .await?;
    Ok(landed)
}

/// The machine card's 在这台开终端 — the GUI face of `open` / `open
/// --on` (docs/KHOR.md: the faces are equivalent because they are one
/// call, and these are those calls). **The fork is on the row, not on
/// the person**: `attach` set the precedent — 行在哪台机器上就接哪台,
/// 不用说 — so this machine's own row opens locally, the CLI's bare
/// `open`, and any other routes over, its `open --on`. The command is
/// left empty going over so the far machine's own shell answers, the
/// CLI's judgment verbatim; locally the shell is resolved here exactly
/// as the CLI resolves it. The size is a placeholder the terminal pane
/// corrects on attach (`term_resize`), because no tty exists here to
/// measure.
///
/// **The wizard's shell kind arrives here too** (2026-09-06: 新建会话
/// 分 留言/智能体/shell 三档): `title` and `cwd` are its two answers,
/// both optional — an empty title is named after the shell, an empty
/// directory is the home, which is what the bare verb always did. `~`
/// is spelled out here for this machine and by the far side for that
/// one (`Request::Open`'s handler), the same rule `start_session` keeps
/// for agents: a path is expanded on the disk it names. An empty
/// `machine` is this one, `open_session`'s reading of the same blank.
pub async fn open_term_on(root: &Path, machine: &str, title: &str, cwd: &str) -> Result<String, String> {
    let n = open(root)?;
    if machine.is_empty() || machine == n.name() {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let shell_name = std::path::Path::new(&shell)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| shell.clone());
        let title = if title.trim().is_empty() { shell_name } else { title.trim().to_owned() };
        let cwd = cwd.trim();
        if cwd.is_empty() {
            let id = n.open_persistent(khor_node::kind::SHELL, &title, &[shell], (80, 24))?;
            return Ok(id.0);
        }
        let home = std::env::home_dir();
        let path = match cwd {
            "~" => home.ok_or_else(|| khor_catalog::msg::no_such_dir("~"))?,
            d if d.starts_with("~/") => home.ok_or_else(|| khor_catalog::msg::no_such_dir(d))?.join(&d[2..]),
            d => std::path::PathBuf::from(d),
        };
        if !path.is_dir() {
            return Err(khor_catalog::msg::no_such_dir(&path.display().to_string()));
        }
        let id = n.open_persistent_at(&path, khor_node::kind::SHELL, &title, &[shell], (80, 24))?;
        return Ok(id.0);
    }
    let title = if title.trim().is_empty() { "shell" } else { title.trim() };
    let id = n
        .open_on(machine, khor_node::kind::SHELL, title, cwd.trim(), &[], (80, 24))
        .await?;
    Ok(id.0)
}


/// Issues a one-time pairing ticket. It carries the live endpoint's
/// addresses, so it needs a resident serve — and both skins embed one
/// (the bridge and the app each start `serve` on their own thread), so
/// the GUI can issue a real ticket without a terminal.
pub fn invite(root: &Path) -> Result<Ticket, String> {
    Ok(Ticket {
        token: open(root)?.invite()?,
        minutes: khor_node::link::invite_window_minutes(),
    })
}

/// Joins with someone's ticket. Async because `Node::pair` dials — and
/// because a resident serve holds this key's one endpoint, the call
/// usually hands off to it over loopback rather than dialing here.
pub async fn pair(root: &Path, ticket: &str) -> Result<String, String> {
    let node = open(root)?;
    node.pair(ticket).await
}

#[cfg(test)]
mod wizard_agent_tests {
    use super::*;

    /// The wizard's door went async on 2026-09-02 (在哪台批 — it can
    /// dial another machine now) and these tests never followed: they
    /// sat red, unrun, until the 机器页 batch tripped over them. The
    /// runtime is built the way every blocking door in this file
    /// builds one.
    fn wait<T>(f: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    fn root(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("khor-wizard-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// **The 智能体 answer is an open position, and it refuses by name.**
    ///
    /// The refusal used to be `wizard_unknown_agent` — a programmer's
    /// error, because the wizard could only send one of two words. Now
    /// anything can arrive, so it is a lookup, and the sentence has to
    /// be the registry's: naming what *is* registered is what turns a
    /// typo into a one-step fix instead of a second command.
    ///
    /// It is the same call the CLI makes (`Node::agent_or_refuse`) —
    /// asserted here rather than trusted, because "the faces are
    /// equivalent" is a claim about a shared function, and a copy would
    /// satisfy every other assertion in this file.
    #[test]
    fn a_name_nobody_registered_is_refused_the_way_the_command_line_refuses_it() {
        let r = root("unknown");
        let n = khor_node::Node::open(r.clone()).unwrap();
        n.register_agent("zed", &["some-acp-agent".to_owned()]).unwrap();

        let why = wait(open_session(&r, "~", "", "chat", "zedd", "")).expect_err("no such registration");
        assert_eq!(
            why,
            n.agent_or_refuse("zedd").unwrap_err(),
            "the wizard refuses in the registry's own words, not its own"
        );
        assert!(why.contains("zed"), "and the refusal names what is registered: {why}");
        let _ = std::fs::remove_dir_all(&r);
    }

    /// **The two forms a registered agent cannot take say which wall
    /// they hit, and they are different walls.**
    ///
    /// Enumerated, and asserted to *differ from each other and from
    /// codex's* — the neighbouring-answer trap is the whole risk here:
    /// three refusals collapsed into one "只有对话形式" would be true
    /// of all of them and useful for none, and `is_err()` cannot see
    /// the difference.
    #[test]
    fn a_registered_agent_is_refused_the_two_forms_it_cannot_take() {
        let r = root("forms");
        let n = khor_node::Node::open(r.clone()).unwrap();
        n.register_agent("zed", &["some-acp-agent".to_owned()]).unwrap();

        let term = wait(open_session(&r, "~", "", "term", "zed", "")).expect_err("no terminal form");
        let sched = wait(open_session(&r, "~", "", "agent", "zed", "")).expect_err("no scheduler form");
        assert_eq!(term, khor_catalog::msg::WIZARD_REGISTERED_NO_TERMINAL);
        assert_eq!(sched, khor_catalog::msg::WIZARD_REGISTERED_NO_SCHEDULER);
        assert_ne!(term, sched, "starting it and arming it are different walls");
        assert_ne!(
            term,
            khor_catalog::msg::WIZARD_CODEX_CHAT_ONLY,
            "and neither of them is codex's refusal, which is about a different limit"
        );
        let _ = std::fs::remove_dir_all(&r);
    }

    /// khor's own two keep every form they had. The control for the
    /// pair above: a rule that refused `term` for everything would pass
    /// both of them.
    #[test]
    fn the_two_khor_ships_keep_the_forms_they_had() {
        let r = root("builtin");
        let _n = khor_node::Node::open(r.clone()).unwrap();
        // Refused for what it *is*, not for the form — codex is
        // chat-only and claude is not, which is the pre-批⑥ shape and
        // must survive the open position.
        assert_eq!(
            wait(open_session(&r, "~", "", "term", "codex", "")).expect_err("codex has no terminal form"),
            khor_catalog::msg::WIZARD_CODEX_CHAT_ONLY
        );
        // claude's `term` is not refused on the agent axis at all: it
        // gets as far as the directory, which is the next question.
        let claude_term = wait(open_session(&r, "/nonexistent/khor-no-such-dir", "", "term", "claude", ""))
            .expect_err("that directory does not exist");
        assert_eq!(
            claude_term,
            khor_catalog::msg::no_such_dir("/nonexistent/khor-no-such-dir"),
            "claude+term is refused by the directory, never by the form"
        );
        let _ = std::fs::remove_dir_all(&r);
    }
}
