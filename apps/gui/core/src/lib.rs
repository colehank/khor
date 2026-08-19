//! The GUI's data layer: the same khor-node calls the CLI makes, shaped
//! into rows the frontend can render. Two skins share this module — the
//! tauri commands (apps/gui/src-tauri) and the dev bridge (`bridge` bin)
//! — so what the browser verifies is what the app ships.
//!
//! No judgment lives here: words, ordering, unread all come from the
//! node. The GUI must not re-derive any of it (docs/UX.md 状态呈现).

pub mod chat;
pub mod files;
pub mod term;
pub mod web;

use std::collections::HashMap;
use std::path::Path;

use khor_node::list::Arrange;
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
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct SourceTag {
    pub device: String,
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
    /// The strain words for memory and disk, minted here so the screen
    /// paints a judgment instead of making one (the thresholds and their
    /// reasons live on `khor_core::Fill::strain`). CPU and GPU carry no
    /// word on purpose: a pinned core is a machine doing its job, a full
    /// disk is a machine about to stop doing it.
    pub mem_strain: Option<khor_core::Strain>,
    pub disk_strain: Option<khor_core::Strain>,
}

impl VitalsReading {
    fn of(vitals: Vitals, age_ms: u64) -> VitalsReading {
        VitalsReading {
            mem_strain: vitals.mem.strain(),
            disk_strain: vitals.disk.and_then(|d| d.strain()),
            vitals,
            age_ms,
        }
    }
}

#[cfg(test)]
mod vitals_reading_tests {
    use super::*;
    use khor_core::{Fill, Strain};

    /// Memory strained, disk fine — asymmetric on purpose. A symmetric
    /// fixture (both strained or both fine) would stay green with the two
    /// assignments swapped, and a swap is the one mistake this three-line
    /// constructor has room for.
    #[test]
    fn each_strain_word_comes_from_its_own_fill() {
        let v = Vitals {
            cpu_pct: 99.0, // pinned CPU carries no word, also on purpose
            cores: 8,
            mem: Fill { used: 96, total: 100 },
            disk: Some(Fill { used: 50, total: 100 }),
            gpu: None,
        };
        let r = VitalsReading::of(v, 0);
        assert_eq!(r.mem_strain, Some(Strain::Critical));
        assert_eq!(r.disk_strain, None, "a half-full disk says nothing");
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
    Ok(n.sessions_arranged(mode)?
        .into_iter()
        .map(|a| {
            let (v, group) = (a.view, a.group);
            let hosted = n.is_hosted(&v.session.id);
            // The row itself says whether its home can stand a terminal
            // up (`Session::term` — hosted, a discovered tmux session,
            // or an agent sitting inside one); this end only adds the
            // locality gate, because the terminal does not travel
            // (remote attach is on the ledger).
            let attachable = hosted || (v.source.is_none() && v.session.term);
            SessionRow {
            attachable,
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
            category: v.session.category,
            group,
        }
        })
        .collect())
}

pub fn list_devices(root: &Path) -> Result<Vec<DeviceRow>, String> {
    let n = open(root)?;
    let me = n.device_str().to_owned();
    let all = n.devices()?;
    // The ring is as big as the network minus this machine, and the
    // seats are handed out in the order the node listed them — so a
    // machine keeps its seat for as long as the list keeps its shape,
    // and nothing here decides an order the node did not.
    let mut ring = map::ring(all.iter().filter(|d| d.id != me).count()).into_iter();
    Ok(all
        .into_iter()
        .map(|d| DeviceRow {
            seat: if d.id == me {
                map::CENTRE
            } else {
                // One seat per machine on the ring, in step with the
                // count above: `ring` was built from exactly this many.
                ring.next().unwrap_or(map::CENTRE)
            },
            me: d.id == me,
            face: n.face_of(&d),
            pinned: d.pinned,
            // Sampled for this machine, read from the last visit for
            // every other. Which one it was is readable from `age_ms`
            // alone, so no second field says it twice.
            vitals: n
                .vitals_of(&d.id)
                .map(|(vitals, age_ms)| VitalsReading::of(vitals, age_ms)),
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

/// Pins or unpins a session — the call `khor pin <session>` makes. One
/// function behind the verb and the button, so the two cannot drift.
pub fn pin_session(root: &Path, id: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_session(&SessionId(id.to_owned()), on)
}

/// Pins or unpins a machine — the call `khor pin -m <machine>` makes.
pub fn pin_device(root: &Path, machine: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_device(machine, on)
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

/// Opens a fresh claude session where the wizard pointed (会话身份批B:
/// 建 session 四字段 — 目录、智能体、形式、名字; claude only this
/// batch). The two forms are the user ruling's two channels: `chat`
/// hosts khor's own claude shim behind the GUI host (`_cagent` — zero
/// dependencies, the skin ruling), `term` hosts the claude TUI. Either
/// way the session is born standing in the chosen directory, and its
/// row id is claude's own uuid (chat) or a khor mint (term).
pub fn open_session(root: &Path, dir: &str, title: &str, form: &str) -> Result<String, String> {
    let n = open(root)?;
    let home = std::env::home_dir();
    let path = match dir.trim() {
        "" | "~" => home.clone().ok_or_else(|| khor_catalog::msg::no_such_dir("~"))?,
        d if d.starts_with("~/") => {
            home.clone().ok_or_else(|| khor_catalog::msg::no_such_dir(d))?.join(&d[2..])
        }
        d => std::path::PathBuf::from(d),
    };
    if !path.is_dir() {
        return Err(khor_catalog::msg::no_such_dir(&path.display().to_string()));
    }
    let title = if title.trim().is_empty() {
        path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "claude".to_owned())
    } else {
        title.trim().to_owned()
    };
    match form {
        "chat" => {
            let exe = std::env::current_exe().map_err(khor_catalog::msg::cant_find_self)?;
            let id = n.open_gui_at(
                &path,
                &title,
                &[exe.display().to_string(), "_cagent".into()],
                // The wizard's 智能体 field. One vendor today, and it
                // is still passed rather than assumed downstream: the
                // day there are two, this line is the one that changes.
                Some(khor_node::adaptor::claude::VENDOR),
                false,
            )?;
            Ok(id.0)
        }
        // 调度员 (docs/AGENT.md): the same conversation session, with
        // khor's own verbs in its hands. Not a new shape — the ruling
        // says so in its first line — so it differs from `chat` by one
        // flag and nothing else.
        "agent" => {
            let exe = std::env::current_exe().map_err(khor_catalog::msg::cant_find_self)?;
            let id = n.open_gui_at(
                &path,
                &title,
                &[exe.display().to_string(), "_cagent".into()],
                Some(khor_node::adaptor::claude::VENDOR),
                true,
            )?;
            Ok(id.0)
        }
        "term" => {
            // The TUI child resolves claude the same way the shim does,
            // env door included, so both forms honour one override.
            let claude = std::env::var("KHOR_CLAUDE").unwrap_or_else(|_| "claude".into());
            // **The session is named before it exists**, and the row
            // wears that name — the same trick the shim plays on the
            // protocol side. Without it this form produced the one row
            // khor could not take over: a khor-minted id names no
            // vendor file, so 接管 answered 「没有它的对话记录」 unless
            // a hook happened to be installed to fill the leaf in.
            let vendor_session = khor_node::adaptor::uuid_v4()?;
            let id = khor_node::adaptor::id_for(&vendor_session);
            let id = n.open_persistent_as(
                &id,
                &path,
                khor_node::kind::TUI,
                &title,
                &[claude, "--session-id".into(), vendor_session],
                (80, 24),
                Some(khor_node::adaptor::claude::VENDOR),
            )?;
            Ok(id.0)
        }
        // The wizard sends exactly the two above; anything else is a
        // programmer's error, not a person's.
        other => Err(format!("no such form: {other}")),
    }
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
