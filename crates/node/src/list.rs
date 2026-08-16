//! List order: the one place the session list is arranged.
//!
//! Both faces call [`arrange`] and paint what comes back — the CLI
//! prints it, the app renders it, and neither owns a comparison
//! (docs/KHOR.md: CLI equals GUI; docs/UX.md: the list never re-derives
//! a judgment the library already made). A second sort anywhere is the
//! bug this module exists to prevent, because two sorts disagree the
//! day one of them is updated.
//!
//! # Pins are not a mode
//!
//! Pinned rows lead in **every** mode. A pin says "this one matters to
//! me", and a way of looking at the list is not a reason to stop
//! honouring it — a mode that buried a pinned row would make the pin
//! feel unreliable, which is worse than not having one.
//!
//! # Why group keys carry a prefix
//!
//! A group key is either a **word to look up** (`state:busy`,
//! `cat:claude`) or **data to print as-is** (`dev:mac-mini`), and the
//! reader has to know which without guessing. Un-prefixed keys would
//! make that a lookup-and-hope: a machine named `busy` would come out
//! as 忙碌. The prefix states the kind of thing the rest of the key is,
//! so a face can dispatch instead of falling back.

use khor_core::DeviceId;

use crate::SessionView;

/// How to arrange the list. Stable keys: they cross the CLI's flag and
/// the app's stored preference, so they are spelled once here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrange {
    /// Freshest first, no groups. **The default**, the way a messaging
    /// app opens: the thing you touched last is the thing you want.
    Recent,
    /// By whose session it is (`khor_core::Session::category`).
    Category,
    /// By the machine it lives on.
    Device,
    /// By state word, groups ordered by `State::rank` — who is waiting
    /// for you. The faces never enumerate the six words themselves.
    State,
}

impl Arrange {
    pub const ALL: [Arrange; 4] =
        [Arrange::Recent, Arrange::Category, Arrange::Device, Arrange::State];

    /// The stable key, which is also the CLI's flag value.
    pub const fn key(self) -> &'static str {
        match self {
            Arrange::Recent => "recent",
            Arrange::Category => "category",
            Arrange::Device => "device",
            Arrange::State => "state",
        }
    }

    /// An unknown key is not a mode. Callers say so by name rather than
    /// quietly arranging some other way.
    pub fn from_key(key: &str) -> Option<Arrange> {
        Arrange::ALL.into_iter().find(|a| a.key() == key)
    }
}

/// The group pinned rows land in, in every mode.
pub const GROUP_PINNED: &str = "pin";
/// Prefix for a category group; the rest is the category name.
pub const GROUP_CATEGORY: &str = "cat:";
/// Prefix for a machine group; the rest is that machine's name.
pub const GROUP_DEVICE: &str = "dev:";
/// Prefix for a state group; the rest is one of the six word keys.
pub const GROUP_STATE: &str = "state:";

/// One row and the group it belongs to.
#[derive(Debug, Clone)]
pub struct Arranged {
    pub view: SessionView,
    /// The group's stable key, or empty in a mode that does not group.
    pub group: String,
}

/// Arranges the list: pinned rows first, then this mode's groups.
///
/// Within any group, freshest first. That is the useful default in all
/// four modes — grouping answers "which of these", and time answers
/// "which one now" — and it means only the *grouping* changes between
/// modes, not two things at once.
///
/// `device_name` turns a row's home into the name people call that
/// machine. A machine the table has never heard of groups under its id
/// rather than under a blank: an unknown machine is still a machine, and
/// a nameless group would silently merge several of them into one.
pub fn arrange(
    views: Vec<SessionView>,
    mode: Arrange,
    device_name: &dyn Fn(&DeviceId) -> Option<String>,
) -> Vec<Arranged> {
    let mut out: Vec<(Arranged, Place)> = views
        .into_iter()
        .map(|view| {
            let (group, place) = place_of(&view, mode, device_name);
            (Arranged { view, group }, place)
        })
        .collect();
    out.sort_by(|(a, pa), (b, pb)| {
        pa.cmp(pb)
            .then_with(|| b.view.session.state.at.0.cmp(&a.view.session.state.at.0))
    });
    out.into_iter().map(|(row, _)| row).collect()
}

/// Where a group sits: a band, then a name to sort within it. Bands keep
/// the "cannot say" groups out of the alphabet (see [`place_of`]).
type Place = (u8, String);

/// A row's group and where that group sits.
///
/// Computed together on purpose. Deriving the position from the group
/// key afterwards would mean parsing the key back out, and the two
/// facts that decide the position — is this machine named, is this
/// category known — are exactly what a key cannot carry once it is a
/// string.
///
/// **The two "cannot say" groups go last**, by one rule: a row nobody
/// could categorise, and a machine the table cannot name. Neither may
/// sit in alphabetical position — an empty category leads the alphabet
/// and a raw hex id does too (digits sort before letters), so the one
/// group that is an admission of ignorance would end up at the top of
/// the screen looking like the most important thing on it.
fn place_of(
    view: &SessionView,
    mode: Arrange,
    device_name: &dyn Fn(&DeviceId) -> Option<String>,
) -> (String, Place) {
    if view.pinned {
        return (GROUP_PINNED.to_owned(), (0, String::new()));
    }
    match mode {
        Arrange::Recent => (String::new(), (1, String::new())),
        Arrange::Category => match view.session.category.as_deref() {
            Some(name) if !name.is_empty() => {
                (format!("{GROUP_CATEGORY}{name}"), (1, name.to_owned()))
            }
            // Never folded into a neighbour's group (docs/SESSION.md
            // 认不出就不落词); the face names it "could not tell".
            _ => (GROUP_CATEGORY.to_owned(), (2, String::new())),
        },
        Arrange::Device => match device_name(&view.session.home) {
            Some(name) => (format!("{GROUP_DEVICE}{name}"), (1, name)),
            None => {
                let hex = view.session.home.hex();
                (format!("{GROUP_DEVICE}{hex}"), (2, hex))
            }
        },
        Arrange::State => {
            let word = view.session.state.state;
            (format!("{GROUP_STATE}{}", word.key()), (1 + word.rank(), String::new()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use khor_core::{Kind, Millis, Session, SessionId, State, StateStamp};

    fn view(id: &str, at: u64, word: State, category: Option<&str>, home: u8) -> SessionView {
        SessionView {
            session: Session {
                id: SessionId(id.to_owned()),
                kind: Kind("tui".to_owned()),
                title: id.to_owned(),
                home: DeviceId([home; 32]),
                state: StateStamp { state: word, at: Millis(at) },
                unread: 0,
                category: category.map(str::to_owned),
            },
            source: None,
            pinned: false,
        }
    }

    fn names(id: &DeviceId) -> Option<String> {
        match id.0[0] {
            1 => Some("alpha".to_owned()),
            2 => Some("beta".to_owned()),
            _ => None,
        }
    }

    fn sample() -> Vec<SessionView> {
        vec![
            view("a", 10, State::Idle, Some("claude"), 1),
            view("b", 40, State::Blocked, Some("codex"), 2),
            view("c", 30, State::Failed, Some("claude"), 2),
            view("d", 20, State::Busy, None, 1),
        ]
    }

    fn ids(rows: &[Arranged]) -> Vec<String> {
        rows.iter().map(|r| r.view.session.id.0.clone()).collect()
    }

    #[test]
    fn recent_is_freshest_first_and_groups_nothing() {
        let rows = arrange(sample(), Arrange::Recent, &names);
        assert_eq!(ids(&rows), vec!["b", "c", "d", "a"]);
        assert!(rows.iter().all(|r| r.group.is_empty()), "recent has no groups");
    }

    /// Groups by rank — who is waiting for you — and 失败 lands last
    /// even though it is the second freshest row. Enumerated rather than
    /// asserted as a property: "grouped somehow" is true of an
    /// implementation that puts everything in one group.
    #[test]
    fn state_groups_run_from_who_needs_you_to_what_is_over() {
        let rows = arrange(sample(), Arrange::State, &names);
        assert_eq!(ids(&rows), vec!["b", "d", "a", "c"]);
        assert_eq!(
            rows.iter().map(|r| r.group.as_str()).collect::<Vec<_>>(),
            vec!["state:blocked", "state:busy", "state:idle", "state:failed"],
        );
    }

    /// By category, alphabetically — with the row nobody could place in
    /// a group of its own, at the end. Folding it into a neighbour would
    /// be a confident wrong answer; putting it in alphabetical position
    /// would make an empty name look like a category.
    #[test]
    fn category_keeps_the_unplaceable_row_in_its_own_group_last() {
        let rows = arrange(sample(), Arrange::Category, &names);
        assert_eq!(ids(&rows), vec!["c", "a", "b", "d"]);
        assert_eq!(
            rows.iter().map(|r| r.group.as_str()).collect::<Vec<_>>(),
            vec!["cat:claude", "cat:claude", "cat:codex", "cat:"],
        );
    }

    /// By machine, alphabetically, freshest first inside each — and a
    /// machine the table cannot name groups under its **id**, kept last.
    ///
    /// Two things are pinned here that a weaker version would miss. An
    /// unnamed machine must not group under a blank, or every unknown
    /// machine merges into one. And it must not sort by that id in the
    /// alphabet either: the row added below is the freshest of all and
    /// its hex begins with a digit, so alphabetical placement would put
    /// the one group nobody can name at the very top.
    #[test]
    fn device_groups_by_machine_and_keeps_the_unnamed_one_last() {
        let mut rows = sample();
        rows.push(view("e", 50, State::Idle, Some("khor"), 9));
        let out = arrange(rows, Arrange::Device, &names);
        assert_eq!(ids(&out), vec!["d", "a", "b", "c", "e"]);
        let groups: Vec<String> = out.iter().map(|r| r.group.clone()).collect();
        assert_eq!(&groups[..4], &["dev:alpha", "dev:alpha", "dev:beta", "dev:beta"]);
        assert_eq!(groups[4], format!("{GROUP_DEVICE}{}", DeviceId([9; 32]).hex()));
    }

    /// **A pin leads in every mode.** The row picked here is the oldest
    /// and 失败 — last in recent order and last by rank — so nothing but
    /// the pin could have put it on top.
    #[test]
    fn a_pinned_row_leads_in_every_mode() {
        for mode in Arrange::ALL {
            let mut rows = sample();
            rows[2].pinned = true; // "c": 失败, and not the freshest
            let out = arrange(rows, mode, &names);
            assert_eq!(
                ids(&out)[0],
                "c",
                "{}: a pinned row must lead",
                mode.key()
            );
            assert_eq!(out[0].group, GROUP_PINNED);
            assert_eq!(out.len(), 4, "{}: no row lost", mode.key());
        }
    }

    /// Two pinned rows share the one group and are freshest-first inside
    /// it — pinning is not itself an ordering.
    #[test]
    fn pinned_rows_share_one_group_freshest_first() {
        let mut rows = sample();
        rows[0].pinned = true; // "a", at 10
        rows[1].pinned = true; // "b", at 40
        let out = arrange(rows, Arrange::State, &names);
        assert_eq!(ids(&out)[..2], ["b".to_owned(), "a".to_owned()]);
        assert_eq!(out[0].group, GROUP_PINNED);
        assert_eq!(out[1].group, GROUP_PINNED);
    }

    #[test]
    fn a_mode_key_round_trips_and_an_unknown_one_is_refused() {
        for mode in Arrange::ALL {
            assert_eq!(Arrange::from_key(mode.key()), Some(mode));
        }
        assert_eq!(Arrange::from_key("alphabetical"), None);
    }
}
