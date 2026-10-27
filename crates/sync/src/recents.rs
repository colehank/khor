//! Where this person has already been: one last-visit instant per
//! (kind, machine, place), replicated everywhere (docs/NET.md — the
//! same decision class as [`crate::seen`] and [`crate::titles`]).
//!
//! Shape: `recents : Map<"<kind>:<device_hex>:<place>", at_ms>`
//! (per-key LWW).
//!
//! # Why this replicates when the tabs do not
//!
//! A tab strip is what one screen currently has open, and it belongs to
//! that screen — the desk having four tabs open says nothing about what
//! the phone should be showing. Having *been* somewhere is the opposite:
//! a person remembers the directory they were digging through this
//! morning regardless of which machine they were sitting at, so the
//! suggestion list has to be there when they pick up the other device.
//! Memory travels, the screen's state stays with the screen.
//!
//! # The key carries a machine, for [`crate::dirpins`]'s reason
//!
//! `/Users/x/Desktop` exists on every Mac in the mesh and a page opened
//! through one machine's network is not the same thing as the same URL
//! opened through another's, so a placeless key would suggest going back
//! somewhere nobody has been. The id and not the name, because a rename
//! must not orphan the record. A device hex is fixed-width lowercase hex
//! and a kind is one of three words with no colon in any, so the first
//! two colons split the key no matter what the place contains
//! (`https://…`, a directory with a colon in its name).
//!
//! # Why the kind is in the key and not in a second document
//!
//! Directories and pages are two lists on two landings, never merged and
//! never shown together, so every read filters by kind anyway. One
//! document means one name on the wire and one best-effort round; two
//! would mean an older peer refusing twice for one feature, and the
//! second table would exist to hold the difference between two words.
//!
//! # Zero is the tombstone, and why the setter is not monotonic
//!
//! "Stop suggesting this" is a decision like any other and must travel,
//! or the device that dismissed a place keeps seeing it everywhere else
//! ([`crate::titles`] on why clearing writes a value rather than
//! deleting a key). So forgetting writes `0`, and readers treat `0` and
//! absent as one answer: not on the list.
//!
//! # Which session it was visited from rides beside it, in a second map
//!
//! UX-37 §1: the session's 附栏 lands on what *this session* touched —
//! a file or a page opened from its terminal, or from its 附栏 while it
//! was in front. That is `origins : Map<key, session id>` in the same
//! document, written with the visit and LWW like it. **A second map and
//! not a richer value** because the value is read by every build in the
//! fleet as an `i64`: a map in its place would read as 「never visited」
//! on all of them, and the list would empty network-wide. A build that
//! has never heard of `origins` merges it and reads past it. A visit
//! from nowhere in particular writes no origin and leaves the last one,
//! so a place stays in the slice of the session that last went there.
//!
//! That is also why [`RecentDoc::visit`] is a plain LWW write and not a
//! raise-only one like [`crate::seen::SeenDoc::mark`]. Raise-only would
//! refuse the tombstone, and — the expensive half — one device with a
//! skewed clock merging a timestamp from next year would make that place
//! permanently unwritable here: every real visit after it would be
//! "older" and silently do nothing, and the row would sit at the top of
//! the list forever with no way to move it. A recency list wobbling by a
//! merge is a list in a slightly wrong order; a key that can never be
//! written again is a row that cannot be fixed.

use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroValue, VersionVector};

use crate::glue;
use crate::store::Doc;

/// Location relative to the home directory (same reasoning as
/// `chat::REL_DIR`).
pub const REL_DIR: &str = ".khor/recents";

pub fn recents_dir(home: &Path) -> PathBuf {
    home.join(REL_DIR)
}

const RECENTS: &str = "recents";
/// Key → the session the visit came from (the module's head).
const ORIGINS: &str = "origins";

/// What sort of place a visit was to.
///
/// An enum rather than a free string, because an unrecognised kind is
/// not a harmless extra row: every read filters by kind, so a record
/// written under a misspelled one is stored, replicated, and never seen
/// again by anybody. The write would look like it took.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A directory on some machine's disk (the files landing).
    Dir,
    /// A web page, through some machine's network (the browser landing).
    Page,
    /// A file on some machine's disk, opened to be read (UX-37 §1: the
    /// session's 附栏 lists what the session touched, and a deliverable
    /// is a file). A build without this word skips its keys ([`split`]).
    File,
}

impl Kind {
    /// The spelling that goes in the key and on the wire. Stable: it is
    /// stored, so renaming a variant here orphans every record already
    /// written under the old word.
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Dir => "dir",
            Kind::Page => "page",
            Kind::File => "file",
        }
    }

    /// `None` for a word this version does not know — a record some
    /// other version wrote is skipped, never guessed at
    /// ([`crate::agents`]'s rule for keys).
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "dir" => Some(Kind::Dir),
            "page" => Some(Kind::Page),
            "file" => Some(Kind::File),
            _ => None,
        }
    }
}

/// The table's one key spelling. The split is [`split`]; nothing else
/// may take a key apart, or two readers disagree about what a row is.
pub fn key(kind: Kind, device_hex: &str, place: &str) -> String {
    format!("{}:{device_hex}:{place}", kind.as_str())
}

/// `(kind, device_hex, place)` back out of a key. `None` for a key some
/// other version wrote in a spelling this one does not know — including
/// a kind word this version has never heard of.
pub fn split(key: &str) -> Option<(Kind, &str, &str)> {
    let (kind, rest) = key.split_once(':')?;
    let (device, place) = rest.split_once(':')?;
    Some((Kind::parse(kind)?, device, place))
}

pub struct RecentDoc {
    inner: LoroDoc,
}

impl RecentDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self {
            inner: glue::with_peer(peer)?,
        })
    }

    /// When this place was last visited; `0` = never, or forgotten. The
    /// two read the same on purpose — nothing a reader does differs
    /// between them, and the difference would have to travel.
    pub fn visited_at(&self, key: &str) -> i64 {
        match self
            .inner
            .get_map(RECENTS)
            .get(key)
            .and_then(|v| v.into_value().ok())
        {
            Some(LoroValue::I64(n)) => n,
            _ => 0,
        }
    }

    /// Records a visit. Revisiting overwrites the instant and adds no
    /// row: the list is places, not trips.
    ///
    /// A non-positive `at_ms` records nothing rather than writing the
    /// value it was handed. `0` is the tombstone, so a caller whose
    /// clock came back broken would otherwise **un-list the place it
    /// just went to**, network-wide — doing nothing costs one missing
    /// suggestion, doing what was asked does the opposite of what was
    /// meant.
    ///
    /// Unchanged means unwritten: this sits behind opening a tab, and
    /// rewriting the same instant inflates the version vector for
    /// nothing (`PinDoc::set`'s rule).
    pub fn visit(&self, key: &str, at_ms: i64) -> Result<(), String> {
        if at_ms <= 0 || self.visited_at(key) == at_ms {
            return Ok(());
        }
        self.write(key, at_ms)
    }

    /// [`Self::visit`], from a session (UX-37 §1): the origin is written
    /// beside the instant. `None` writes no origin and keeps the last
    /// one — a visit from the landing does not take a place out of the
    /// slice of the session that went there before.
    pub fn visit_from(&self, key: &str, at_ms: i64, session: Option<&str>) -> Result<(), String> {
        let Some(session) = session.filter(|s| !s.is_empty()) else {
            return self.visit(key, at_ms);
        };
        if at_ms <= 0 {
            return Ok(());
        }
        let same = self.visited_at(key) == at_ms && self.origin_of(key).as_deref() == Some(session);
        if same {
            return Ok(());
        }
        let map = self.inner.get_map(RECENTS);
        map.insert(key, at_ms).map_err(|e| e.to_string())?;
        self.inner
            .get_map(ORIGINS)
            .insert(key, session)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// The session this place was last visited from, if one was said.
    pub fn origin_of(&self, key: &str) -> Option<String> {
        match self
            .inner
            .get_map(ORIGINS)
            .get(key)
            .and_then(|v| v.into_value().ok())
        {
            Some(LoroValue::String(s)) if !s.is_empty() => Some(s.to_string()),
            _ => None,
        }
    }

    /// "Stop suggesting this." Writes the tombstone, which travels; the
    /// place comes back on its own if the person goes there again, which
    /// is why this is a value and not a deletion.
    pub fn forget(&self, key: &str) -> Result<(), String> {
        if self.visited_at(key) == 0 {
            return Ok(());
        }
        self.write(key, 0)
    }

    fn write(&self, key: &str, at_ms: i64) -> Result<(), String> {
        self.inner
            .get_map(RECENTS)
            .insert(key, at_ms)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every place still on the list, most recent first, as
    /// `(key, at_ms)`.
    ///
    /// Ties break on the key, ascending. Two places stamped in the same
    /// millisecond are ordinary — a machine that opens two tabs at once
    /// does it — and without a second axis the two would swap places
    /// between reads on the same data, which is a list that flickers
    /// while nothing changed.
    pub fn all(&self) -> Vec<(String, i64)> {
        let map = self.inner.get_map(RECENTS);
        let mut out: Vec<(String, i64)> = map
            .keys()
            .map(|k| (k.to_string(), self.visited_at(&k)))
            .filter(|(_, at)| *at > 0)
            .collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }
}

impl Doc for RecentDoc {
    fn open(peer: u64) -> Result<Self, String> {
        RecentDoc::new(peer)
    }

    fn peer_id(&self) -> u64 {
        self.inner.peer_id()
    }

    fn version(&self) -> VersionVector {
        self.inner.oplog_vv()
    }

    fn changes_since(&self, theirs: &VersionVector) -> Result<Vec<u8>, String> {
        glue::changes_since(&self.inner, theirs)
    }

    fn snapshot(&self) -> Result<Vec<u8>, String> {
        glue::snapshot(&self.inner)
    }

    fn merge_report(&self, bytes: &[u8]) -> Result<crate::store::MergeReport, String> {
        glue::merge_report(&self.inner, bytes)
    }

    /// Places on the list, not keys ever written: a forgotten place is
    /// not an item, and the sync line that prints this would otherwise
    /// count places the user dismissed.
    fn items(&self) -> usize {
        self.all().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A URL is full of colons and a directory may contain one; the
    /// split must land on the two boundaries the key defines, which is
    /// the whole reason the first two fields are fixed-form.
    #[test]
    fn the_key_splits_on_its_two_boundaries_not_inside_the_place() {
        let k = key(Kind::Page, "abc123", "https://example.com:8443/a?b=c:d");
        assert_eq!(
            split(&k),
            Some((Kind::Page, "abc123", "https://example.com:8443/a?b=c:d"))
        );
        let k = key(Kind::Dir, "abc123", "/tmp/odd:name/dir");
        assert_eq!(split(&k), Some((Kind::Dir, "abc123", "/tmp/odd:name/dir")));
    }

    /// **A visit from a session carries it, and travels with it**
    /// (UX-37 §1); a later visit from nowhere keeps it, a visit from
    /// another session takes it over, and the instant reads exactly as
    /// before — the value every build reads is still an `i64`.
    #[test]
    fn the_session_a_place_was_visited_from_rides_beside_it() {
        let a = RecentDoc::new(1).unwrap();
        let k = key(Kind::File, "abc123", "/home/x/out.md");
        a.visit_from(&k, 100, Some("tui/one")).unwrap();
        assert_eq!(a.visited_at(&k), 100);
        assert_eq!(a.origin_of(&k).as_deref(), Some("tui/one"));
        a.visit_from(&k, 200, None).unwrap();
        assert_eq!(a.visited_at(&k), 200, "control: the visit moved");
        assert_eq!(
            a.origin_of(&k).as_deref(),
            Some("tui/one"),
            "a visit from nowhere keeps the origin"
        );
        a.visit_from(&k, 300, Some("tui/two")).unwrap();
        assert_eq!(a.origin_of(&k).as_deref(), Some("tui/two"));
        let b = RecentDoc::new(2).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();
        assert_eq!(b.visited_at(&k), 300);
        assert_eq!(
            b.origin_of(&k).as_deref(),
            Some("tui/two"),
            "the origin travels with the visit"
        );
        let other = key(Kind::Page, "abc123", "https://example.com/");
        b.visit(&other, 50).unwrap();
        assert_eq!(
            b.origin_of(&other),
            None,
            "control: a plain visit says no session"
        );
    }

    /// A kind word this version does not know is skipped, never guessed
    /// at — the same rule the other tables' keys follow.
    #[test]
    fn a_kind_this_version_never_heard_of_is_skipped() {
        assert_eq!(split("gopher:abc123:/somewhere"), None);
        assert_eq!(
            split("dir:abc123"),
            None,
            "a key missing a boundary is not a key"
        );
    }

    /// **The list is places, not trips.** Going back to the same
    /// directory moves its instant and adds nothing — a counter would
    /// grow with every tab opened and the list would be a log.
    #[test]
    fn revisiting_a_place_only_moves_its_instant() {
        let d = RecentDoc::new(1).unwrap();
        let k = key(Kind::Dir, "d1e6", "/Users/x/proj");
        d.visit(&k, 1_000).unwrap();
        d.visit(&k, 2_000).unwrap();
        d.visit(&k, 3_000).unwrap();
        assert_eq!(d.visited_at(&k), 3_000, "the latest visit stands");
        assert_eq!(d.items(), 1, "three visits to one place is one place");

        // …and a second place is what makes it two.
        d.visit(&key(Kind::Dir, "d1e6", "/Users/x/other"), 1_500)
            .unwrap();
        assert_eq!(d.items(), 2);
    }

    /// Written out of order, read in order — an implementation that
    /// returned the map's own key order passes nothing here.
    #[test]
    fn the_list_reads_most_recent_first_however_it_was_written() {
        let d = RecentDoc::new(1).unwrap();
        let (a, b, c) = (
            key(Kind::Dir, "d1e6", "/a"),
            key(Kind::Dir, "d1e6", "/b"),
            key(Kind::Dir, "d1e6", "/c"),
        );
        d.visit(&b, 200).unwrap();
        d.visit(&a, 900).unwrap();
        d.visit(&c, 500).unwrap();
        let order: Vec<String> = d.all().into_iter().map(|(k, _)| k).collect();
        assert_eq!(order, vec![a, c, b], "time-descending, not insertion order");
    }

    /// Two places stamped in the same millisecond must not swap between
    /// reads: a list that reorders while nothing changed is a list
    /// nobody can point at.
    #[test]
    fn a_tie_reads_the_same_way_twice() {
        let d = RecentDoc::new(1).unwrap();
        d.visit(&key(Kind::Dir, "d1e6", "/z"), 400).unwrap();
        d.visit(&key(Kind::Dir, "d1e6", "/a"), 400).unwrap();
        assert_eq!(d.all(), d.all(), "the same data must read the same way");
        let first = d.all()[0].0.clone();
        assert!(
            first.ends_with("/a"),
            "ties break on the key, ascending: {first}"
        );
    }

    /// The promise: dig through a directory on the desk, the phone
    /// offers it. This is the whole reason it is a document and not a
    /// preference file.
    #[test]
    fn a_visit_on_one_device_shows_up_on_the_other_after_merge() {
        let desk = RecentDoc::new(1).unwrap();
        let phone = RecentDoc::new(2).unwrap();
        let k = key(Kind::Dir, "d1e6", "/Users/x/proj");
        desk.visit(&k, 4_242).unwrap();
        phone
            .merge(&desk.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(
            phone.visited_at(&k),
            4_242,
            "a visit on the desk must reach the phone"
        );
    }

    /// Dismissing has to travel too, or the device that dismissed a
    /// place keeps seeing it everywhere else — the reason forgetting
    /// writes `0` instead of deleting nothing.
    #[test]
    fn forgetting_travels_and_reads_as_not_on_the_list() {
        let a = RecentDoc::new(1).unwrap();
        let b = RecentDoc::new(2).unwrap();
        let k = key(Kind::Page, "d1e6", "https://news.example");
        a.visit(&k, 900).unwrap();
        b.merge(&a.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(b.visited_at(&k), 900, "control: it arrived on the list");

        b.forget(&k).unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        assert_eq!(
            a.visited_at(&k),
            0,
            "the dismissal must reach the other side too"
        );
        assert!(a.all().is_empty(), "a forgotten place is not on the list");
        assert_eq!(a.items(), 0, "…and is not an item");
    }

    /// Going back somewhere you dismissed puts it back. The tombstone
    /// says "stop suggesting this", not "this place is gone" — which is
    /// exactly what a raise-only setter would have made it.
    #[test]
    fn visiting_a_forgotten_place_again_brings_it_back() {
        let d = RecentDoc::new(1).unwrap();
        let k = key(Kind::Page, "d1e6", "https://news.example");
        d.visit(&k, 900).unwrap();
        d.forget(&k).unwrap();
        assert_eq!(d.visited_at(&k), 0, "control: it is off the list");
        d.visit(&k, 1_100).unwrap();
        assert_eq!(d.visited_at(&k), 1_100, "going back must put it back");
        assert_eq!(d.items(), 1);
    }

    /// A broken clock must not un-list the place it just went to. `0`
    /// is the tombstone, so writing what was handed over would do the
    /// exact opposite of recording a visit.
    #[test]
    fn a_non_positive_instant_records_nothing_rather_than_erasing() {
        let d = RecentDoc::new(1).unwrap();
        let k = key(Kind::Dir, "d1e6", "/Users/x/proj");
        d.visit(&k, 900).unwrap();
        let before = d.version();
        d.visit(&k, 0).unwrap();
        d.visit(&k, -1).unwrap();
        assert_eq!(d.visited_at(&k), 900, "the real visit must still stand");
        assert_eq!(d.version(), before, "…and nothing must have been written");
    }

    /// No-change writes grow nothing: this sits behind opening a tab,
    /// and re-opening the same one must not cost a block.
    #[test]
    fn writing_the_same_instant_or_forgetting_twice_writes_nothing() {
        let d = RecentDoc::new(1).unwrap();
        let k = key(Kind::Dir, "d1e6", "/Users/x/proj");
        d.visit(&k, 900).unwrap();
        let after_visit = d.version();
        d.visit(&k, 900).unwrap();
        assert_eq!(
            d.version(),
            after_visit,
            "the same instant must not create ops"
        );

        d.forget(&k).unwrap();
        let after_forget = d.version();
        d.forget(&k).unwrap();
        assert_eq!(
            d.version(),
            after_forget,
            "forgetting twice must not create ops"
        );

        let fresh = RecentDoc::new(3).unwrap();
        let empty = fresh.version();
        fresh.forget(&key(Kind::Dir, "d1e6", "/never")).unwrap();
        assert_eq!(
            fresh.version(),
            empty,
            "forgetting somewhere never visited writes nothing"
        );
    }

    /// Two devices disagreeing settle on one answer on both sides.
    /// Which one wins matters less than that they agree — a per-device
    /// store never does.
    #[test]
    fn a_concurrent_visit_and_dismissal_settle_the_same_way_on_both_sides() {
        let a = RecentDoc::new(1).unwrap();
        let b = RecentDoc::new(2).unwrap();
        let k = key(Kind::Page, "d1e6", "https://example.com");
        a.visit(&k, 500).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();

        a.visit(&k, 900).unwrap();
        b.forget(&k).unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        b.merge(&a.changes_since(&b.version()).unwrap()).unwrap();
        assert_eq!(
            a.visited_at(&k),
            b.visited_at(&k),
            "both sides must land on one answer"
        );
        let settled = a.visited_at(&k);
        assert!(
            settled == 900 || settled == 0,
            "the settled value must be one of the two: {settled}"
        );
    }
}
