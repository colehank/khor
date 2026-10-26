//! Strict Doc apply uses real Loro histories and owned filesystem/lock faults.
use khor_sync::{
    chat::ChatDoc,
    devices::DeviceDoc,
    pins::PinDoc,
    store::{self, ApplyError, ApplySpec, Doc, MergeReport},
};
use loro::VersionVector;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new(case: &str) -> Self {
        let base = std::env::var_os("KHOR_APPLY_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).unwrap();
        let dir = base.join(format!(
            "khor-apply-{case}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
    fn pile(&self) -> PathBuf {
        self.0.join("pile")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn spec(epoch: u32, base: VersionVector, target: VersionVector) -> ApplySpec {
    ApplySpec {
        expected_epoch: epoch,
        base_vv: base,
        target_vv: target,
    }
}
fn body(doc: &impl Doc) -> Vec<u8> {
    doc.changes_since(&VersionVector::default()).unwrap()
}
fn image(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    if !dir.exists() {
        return BTreeMap::new();
    }
    fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}
fn pieces(peer: u64) -> (PinDoc, Vec<u8>, Vec<u8>) {
    let d = PinDoc::new(peer).unwrap();
    d.set("dependency", true).unwrap();
    let first = d.version();
    let dependency = body(&d);
    d.set("pending-edit", true).unwrap();
    let edit = d.changes_since(&first).unwrap();
    let control = PinDoc::new(peer + 1).unwrap();
    assert!(!control.merge_report(&edit).unwrap().pending.is_empty());
    assert!(!control.pinned("pending-edit"));
    control.merge(&dependency).unwrap();
    assert!(control.pinned("pending-edit"));
    (d, dependency, edit)
}

#[test]
fn pending_bad_body_and_unmet_target_leave_live_and_disk_unchanged() {
    let s = Scratch::new("reject");
    let (source, _, edit) = pieces(0x4100);
    let mut loaded = store::load_pile::<PinDoc>(&s.pile(), 0x4200).unwrap();
    let empty = loaded.doc.version();
    let initial = image(&s.pile());
    let goal = spec(1, empty.clone(), source.version());
    assert_eq!(
        loaded.apply_checked(&goal, &edit).unwrap_err(),
        ApplyError::PendingImport
    );
    assert_eq!(loaded.doc.version(), empty);
    assert_eq!(image(&s.pile()), initial);
    assert!(matches!(
        loaded.apply_checked(&goal, b"not a Loro document"),
        Err(ApplyError::InvalidBody(_))
    ));
    let short = PinDoc::new(0x4300).unwrap();
    short.set("one", true).unwrap();
    let short_body = body(&short);
    short.set("two", true).unwrap();
    assert_eq!(
        loaded
            .apply_checked(&spec(1, empty.clone(), short.version()), &short_body)
            .unwrap_err(),
        ApplyError::MissingTarget
    );
    assert_eq!(loaded.doc.version(), empty);
    assert_eq!(image(&s.pile()), initial);
    assert!(
        !s.pile().exists(),
        "candidate rejection must not create a pile"
    );
}

#[test]
fn base_must_exist_both_live_and_on_disk_before_import() {
    let s = Scratch::new("base");
    let source = PinDoc::new(0x4400).unwrap();
    source.set("base", true).unwrap();
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x4401).unwrap();
    let goal = spec(0, source.version(), source.version());
    assert_eq!(
        loaded.apply_checked(&goal, &body(&source)).unwrap_err(),
        ApplyError::StaleBase
    );
    assert!(!loaded.doc.pinned("base"));
    loaded.doc.merge(&body(&source)).unwrap();
    assert_eq!(
        loaded.apply_checked(&goal, &body(&source)).unwrap_err(),
        ApplyError::StaleBase,
        "dirty live ops are not an advertised durable base"
    );
    assert!(!s.pile().exists());
    loaded.store.flush(&loaded.doc).unwrap();
    loaded.apply_checked(&goal, &body(&source)).unwrap();
}

#[test]
fn concurrent_base_target_and_dirty_local_ops_survive_flush_and_reopen() {
    let s = Scratch::new("concurrent");
    let mut loaded = store::load_pile::<PinDoc>(&s.pile(), 0x4500).unwrap();
    loaded.doc.set("local-base", true).unwrap();
    loaded.store.flush(&loaded.doc).unwrap();
    let base = loaded.doc.version();
    loaded.doc.set("local-dirty", true).unwrap();
    let before = loaded.doc.version();
    let remote = PinDoc::new(0x4501).unwrap();
    remote.set("remote", true).unwrap();
    assert!(!base.includes_vv(&remote.version()) && !remote.version().includes_vv(&base));
    let result = loaded
        .apply_checked(&spec(1, base, remote.version()), &body(&remote))
        .unwrap();
    assert!(result.version().includes_vv(&before));
    assert!(result.version().includes_vv(&remote.version()));
    assert!(result.block().unwrap().is_file());
    let actual = result.version().clone();
    drop(loaded);
    let reopened = store::load_pile::<PinDoc>(&s.pile(), 0x4502).unwrap();
    for key in ["local-base", "local-dirty", "remote"] {
        assert!(reopened.doc.pinned(key));
    }
    assert!(reopened.doc.version().includes_vv(&actual));
}

#[test]
fn duplicates_need_no_block_but_flush_new_local_dirty_ops() {
    let s = Scratch::new("duplicate");
    let source = PinDoc::new(0x4600).unwrap();
    source.set("remote", true).unwrap();
    let goal = spec(1, Default::default(), source.version());
    let bytes = body(&source);
    let mut loaded = store::load_pile::<PinDoc>(&s.pile(), 0x4601).unwrap();
    let first = loaded.apply_checked(&goal, &bytes).unwrap();
    assert!(first.block().is_some());
    assert_eq!(fs::read_to_string(s.pile().join(".epoch")).unwrap(), "1\n");
    let initial = image(&s.pile());
    let duplicate = loaded.apply_checked(&goal, &bytes).unwrap();
    assert!(duplicate.block().is_none());
    assert_eq!(image(&s.pile()), initial);
    loaded.doc.set("local-later", true).unwrap();
    assert!(
        loaded
            .apply_checked(&goal, &bytes)
            .unwrap()
            .block()
            .is_some()
    );
    drop(loaded);
    let reopened = store::load_pile::<PinDoc>(&s.pile(), 0x4602).unwrap();
    assert!(reopened.doc.pinned("local-later") && reopened.doc.pinned("remote"));
}

#[test]
fn strict_import_preserves_old_live_pending_and_can_unlock_it() {
    for unlock in [false, true] {
        let s = Scratch::new(if unlock { "unlock" } else { "preserve" });
        let (source, dependency, edit) = pieces(0x4700);
        let mut loaded = store::load::<PinDoc>(&s.pile(), 0x4702).unwrap();
        let report = loaded.doc.merge_report(&edit).unwrap();
        loaded.store.record_import(&report);
        let independent = PinDoc::new(0x4703).unwrap();
        independent.set("independent", true).unwrap();
        let target = if unlock {
            let d = PinDoc::new(0x4704).unwrap();
            d.merge(&dependency).unwrap();
            d
        } else {
            independent
        };
        let bytes = if unlock {
            dependency.clone()
        } else {
            body(&target)
        };
        let result = loaded
            .apply_checked(&spec(0, Default::default(), target.version()), &bytes)
            .unwrap();
        if unlock {
            assert!(result.version().includes_vv(&source.version()));
            assert!(
                !target.version().includes_vv(result.version()),
                "real live report may unlock ops beyond candidate/target"
            );
            assert!(result.report().applied.iter().any(|r| r.writer == 0x4700 && r.end == *source.version().get(&0x4700).unwrap()));
        } else {
            assert!(!loaded.doc.pinned("pending-edit"));
            assert!(loaded.store.compact(&loaded.doc).is_err());
            let report = loaded.doc.merge_report(&dependency).unwrap();
            loaded.store.record_import(&report);
            loaded.store.flush(&loaded.doc).unwrap();
        }
        assert!(loaded.doc.pinned("pending-edit"));
        drop(loaded);
        let reopened = store::load::<PinDoc>(&s.pile(), 0x4705).unwrap();
        assert!(reopened.doc.pinned("pending-edit"));
        assert!(reopened.doc.version().includes_vv(&source.version()));
    }
}

#[test]
fn rejected_candidate_does_not_destroy_old_live_pending() {
    let s = Scratch::new("reject-old");
    let (source, dependency, edit) = pieces(0x4800);
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x4802).unwrap();
    let report = loaded.doc.merge_report(&edit).unwrap();
    loaded.store.record_import(&report);
    assert!(
        loaded
            .apply_checked(&spec(0, Default::default(), source.version()), b"invalid")
            .is_err()
    );
    let report = loaded.doc.merge_report(&dependency).unwrap();
    loaded.store.record_import(&report);
    assert!(loaded.doc.pinned("pending-edit"));
    loaded.store.flush(&loaded.doc).unwrap();
}

#[test]
fn a_device_import_retires_the_current_pen_before_advancing_its_writer() {
    let s = Scratch::new("device-pen");
    fs::create_dir(s.pile()).unwrap();
    let mut loaded = store::load::<DeviceDoc>(&s.pile(), 0x4900).unwrap();
    let old = loaded.doc.peer_id();
    let source = DeviceDoc::new(old).unwrap();
    source.upsert("remote", "Remote", &[]).unwrap();
    let goal = spec(0, Default::default(), source.version());
    let result = loaded.apply_checked(&goal, &body(&source)).unwrap();
    assert_eq!(result.retired_pen(), Some(old));
    assert_eq!(loaded.store.retired(), Some(old));
    assert_eq!(loaded.doc.peer_id(), old, "live Doc must not be replaced");
    assert!(
        loaded
            .apply_checked(&goal, &body(&source))
            .unwrap()
            .retired_pen()
            .is_none()
    );
    source.upsert("remote-next", "Remote next", &[]).unwrap();
    assert!(
        loaded
            .apply_checked(
                &spec(0, Default::default(), source.version()),
                &body(&source)
            )
            .unwrap()
            .retired_pen()
            .is_none(),
        "an already retired writer need not retire a second time"
    );
    drop(loaded);
    let next = store::load::<DeviceDoc>(&s.pile(), 0x4901).unwrap();
    assert_ne!(next.doc.peer_id(), old);
    assert!(next.doc.version().includes_vv(&source.version()));
}

#[test]
fn pending_own_writer_is_retired_before_an_import_can_unlock_it() {
    let s = Scratch::new("pending-pen");
    fs::create_dir(s.pile()).unwrap();
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x4910).unwrap();
    let old = loaded.doc.peer_id();
    // Only the old live queue knows about own-writer ops: the candidate body
    // contains a foreign dependency and cannot predict that own advancement.
    let creator = PinDoc::new(0x4911).unwrap();
    creator.set("dependency", true).unwrap();
    let dependency = body(&creator);
    let source = PinDoc::new(old).unwrap();
    source.merge(&dependency).unwrap();
    source.set("pending-edit", true).unwrap();
    let edit = source.changes_since(&creator.version()).unwrap();
    let control = PinDoc::new(0x4912).unwrap();
    assert!(!control.merge_report(&edit).unwrap().pending.is_empty());
    assert!(!creator.version().includes_vv(&source.version()));
    let report = loaded.doc.merge_report(&edit).unwrap();
    loaded.store.record_import(&report);
    let pen = s.pile().join(".pen-0");
    let held = s.pile().join("held-pen");
    fs::rename(&pen, &held).unwrap();
    fs::create_dir(&pen).unwrap();
    assert!(matches!(
        loaded.apply_checked(&spec(0, Default::default(), creator.version()), &dependency),
        Err(ApplyError::CommitFailed(_))
    ));
    assert!(
        !loaded.doc.pinned("dependency") && !loaded.doc.pinned("pending-edit"),
        "registered own pending must force retirement before the foreign dependency enters live"
    );
    fs::remove_dir(&pen).unwrap();
    fs::rename(&held, &pen).unwrap();
    let result = loaded
        .apply_checked(&spec(0, Default::default(), creator.version()), &dependency)
        .unwrap();
    assert_eq!(result.retired_pen(), Some(old));
    assert!(loaded.doc.pinned("pending-edit"));
    assert!(result.version().includes_vv(&source.version()));
}

#[test]
fn spare_own_writer_advancement_is_rejected_before_live_import() {
    let s = Scratch::new("spare-own");
    let mut loaded = store::load_pile::<PinDoc>(&s.pile(), 0x4920).unwrap();
    let source = PinDoc::new(loaded.doc.peer_id()).unwrap();
    source.set("foreign-own-writer", true).unwrap();
    assert_eq!(
        loaded
            .apply_checked(
                &spec(1, Default::default(), source.version()),
                &body(&source)
            )
            .unwrap_err(),
        ApplyError::MissingWriterLease
    );
    assert!(!loaded.doc.pinned("foreign-own-writer"));
    assert!(s.pile().join(".pile").is_file());
    assert_eq!(fs::read_to_string(s.pile().join(".epoch")).unwrap(), "1\n");
    assert!(!s.pile().join(".pen-0").exists());
    assert!(!s.pile().join(".merged").exists());
    assert!(
        !fs::read_dir(s.pile()).unwrap().any(|e| e
            .unwrap()
            .path()
            .extension()
            .is_some_and(|x| x == "loro"))
    );
    // The strict boundary created only controls, so fresh reload can take a
    // persistent pen with no external mkdir or other hidden recovery action.
    drop(loaded);
    let mut fresh = store::load_pile::<PinDoc>(&s.pile(), 0x4920).unwrap();
    assert_ne!(fresh.doc.peer_id(), source.peer_id());
    fresh
        .apply_checked(
            &spec(1, Default::default(), source.version()),
            &body(&source),
        )
        .unwrap();
    assert!(fresh.doc.pinned("foreign-own-writer"));
}

#[test]
fn spare_registered_own_pending_is_rejected_even_for_a_foreign_body() {
    let s = Scratch::new("spare-pending");
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x4930).unwrap();
    let creator = PinDoc::new(0x4931).unwrap();
    creator.set("dependency", true).unwrap();
    let own = PinDoc::new(loaded.doc.peer_id()).unwrap();
    own.merge(&body(&creator)).unwrap();
    own.set("pending-edit", true).unwrap();
    let edit = own.changes_since(&creator.version()).unwrap();
    let report = loaded.doc.merge_report(&edit).unwrap();
    loaded.store.record_import(&report);
    assert_eq!(
        loaded
            .apply_checked(
                &spec(0, Default::default(), creator.version()),
                &body(&creator)
            )
            .unwrap_err(),
        ApplyError::MissingWriterLease
    );
    assert_eq!(loaded.doc.version(), VersionVector::default());
    assert!(s.pile().join(".pile").is_file());
    assert!(!s.pile().join(".epoch").exists());
    assert!(!s.pile().join(".merged").exists());
    assert!(
        !fs::read_dir(s.pile()).unwrap().any(|e| e
            .unwrap()
            .path()
            .extension()
            .is_some_and(|x| x == "loro"))
    );
    loaded.doc.merge(&body(&creator)).unwrap();
    assert!(
        loaded.doc.pinned("pending-edit"),
        "rejection must preserve the old queue"
    );
}

#[test]
fn pen_retirement_failure_precedes_live_import_and_preserves_the_held_lease() {
    let s = Scratch::new("pen-failure");
    fs::create_dir(s.pile()).unwrap();
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x4a00).unwrap();
    let old = loaded.doc.peer_id();
    let source = PinDoc::new(old).unwrap();
    source.set("foreign-own-writer", true).unwrap();
    let pen = s.pile().join(".pen-0");
    let held = s.pile().join("held-pen");
    fs::rename(&pen, &held).unwrap();
    fs::create_dir(&pen).unwrap();
    assert!(matches!(
        loaded.apply_checked(
            &spec(0, Default::default(), source.version()),
            &body(&source)
        ),
        Err(ApplyError::CommitFailed(_))
    ));
    assert!(!loaded.doc.pinned("foreign-own-writer"));
    assert_eq!(loaded.doc.version(), VersionVector::default());
    assert_eq!(loaded.store.retired(), None);
    assert!(
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&held)
            .unwrap()
            .try_lock()
            .is_err()
    );
    fs::remove_dir(&pen).unwrap();
    fs::rename(&held, &pen).unwrap();
    assert_eq!(
        loaded
            .apply_checked(
                &spec(0, Default::default(), source.version()),
                &body(&source)
            )
            .unwrap()
            .retired_pen(),
        Some(old)
    );
}

#[test]
fn ledger_failure_returns_no_receipt_and_retains_the_written_block_for_retry() {
    let s = Scratch::new("ledger-failure");
    fs::create_dir(s.pile()).unwrap();
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x4b00).unwrap();
    fs::create_dir(s.pile().join(".merged")).unwrap();
    let source = PinDoc::new(0x4b01).unwrap();
    source.set("remote", true).unwrap();
    let goal = spec(0, Default::default(), source.version());
    assert!(matches!(
        loaded.apply_checked(&goal, &body(&source)),
        Err(ApplyError::CommitFailed(_))
    ));
    assert!(
        loaded.doc.pinned("remote"),
        "commit failure must not imply rollback"
    );
    let blocks: Vec<_> = fs::read_dir(s.pile())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "loro"))
        .collect();
    assert_eq!(blocks.len(), 1);
    let raw = PinDoc::new(0x4b02).unwrap();
    raw.merge(&fs::read(&blocks[0]).unwrap()).unwrap();
    assert!(raw.pinned("remote"));
    fs::remove_dir(s.pile().join(".merged")).unwrap();
    assert!(
        loaded
            .apply_checked(&goal, &body(&source))
            .unwrap()
            .version()
            .includes_vv(&source.version())
    );
    drop(loaded);
    assert!(
        store::load::<PinDoc>(&s.pile(), 0x4b03)
            .unwrap()
            .doc
            .pinned("remote")
    );
}

/// Transparent real-Loro document with a one-shot filesystem fault only on
/// the original live instance. Candidate constructors have no fault attached.
struct FaultDoc {
    inner: PinDoc,
    obstruct: std::cell::RefCell<Option<(PathBuf, PathBuf)>>,
}
impl Doc for FaultDoc {
    fn open(peer: u64) -> Result<Self, String> {
        Ok(Self {
            inner: PinDoc::new(peer)?,
            obstruct: Default::default(),
        })
    }
    fn peer_id(&self) -> u64 {
        self.inner.peer_id()
    }
    fn version(&self) -> VersionVector {
        self.inner.version()
    }
    fn changes_since(&self, vv: &VersionVector) -> Result<Vec<u8>, String> {
        if let Some((dir, backup)) = self.obstruct.borrow_mut().take() {
            fs::rename(&dir, backup).unwrap();
            fs::write(dir, b"owned path-type fault").unwrap();
        }
        self.inner.changes_since(vv)
    }
    fn snapshot(&self) -> Result<Vec<u8>, String> {
        self.inner.snapshot()
    }
    fn merge_report(&self, bytes: &[u8]) -> Result<MergeReport, String> {
        self.inner.merge_report(bytes)
    }
    fn items(&self) -> usize {
        self.inner.items()
    }
}

struct RestoreDirectory {
    dir: PathBuf,
    backup: PathBuf,
}
impl RestoreDirectory {
    fn restore(&self) {
        if self.backup.exists() {
            if self.dir.is_file() {
                fs::remove_file(&self.dir).unwrap();
            }
            fs::rename(&self.backup, &self.dir).unwrap();
        }
    }
}
impl Drop for RestoreDirectory {
    fn drop(&mut self) {
        self.restore();
    }
}

#[test]
fn block_path_failure_is_a_commit_error_and_can_be_retried() {
    let s = Scratch::new("block-failure");
    fs::create_dir(s.pile()).unwrap();
    let mut loaded = store::load::<FaultDoc>(&s.pile(), 0x4c00).unwrap();
    let backup = s.0.join("backup");
    let restore = RestoreDirectory {
        dir: s.pile(),
        backup: backup.clone(),
    };
    *loaded.doc.obstruct.borrow_mut() = Some((s.pile(), backup.clone()));
    let source = PinDoc::new(0x4c01).unwrap();
    source.set("remote", true).unwrap();
    let goal = spec(0, Default::default(), source.version());
    assert!(matches!(
        loaded.apply_checked(&goal, &body(&source)),
        Err(ApplyError::CommitFailed(_))
    ));
    assert!(loaded.doc.inner.pinned("remote"));
    assert!(
        s.pile().is_file(),
        "the fault must occur during real flush export"
    );
    assert!(
        !fs::read_dir(&backup).unwrap().any(|e| e
            .unwrap()
            .path()
            .extension()
            .is_some_and(|x| x == "loro"))
    );
    restore.restore();
    assert!(
        loaded
            .apply_checked(&goal, &body(&source))
            .unwrap()
            .block()
            .is_some()
    );
    drop(loaded);
    assert!(
        store::load::<PinDoc>(&s.pile(), 0x4c02)
            .unwrap()
            .doc
            .pinned("remote")
    );
}

#[test]
fn shared_stamp_missing_epoch_fails_closed_without_dropping_shared_lock() {
    let s = Scratch::new("shared-stamp");
    fs::create_dir(s.pile()).unwrap();
    let blocker = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(s.pile().join(".pile"))
        .unwrap();
    blocker.try_lock_shared().unwrap();
    let mut loaded = store::load_pile::<PinDoc>(&s.pile(), 0x4d00).unwrap();
    drop(blocker);
    let source = PinDoc::new(0x4d01).unwrap();
    source.set("remote", true).unwrap();
    let before = image(&s.pile());
    let start = Instant::now();
    assert_eq!(
        loaded
            .apply_checked(
                &spec(1, Default::default(), source.version()),
                &body(&source)
            )
            .unwrap_err(),
        ApplyError::NeedsReload
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(image(&s.pile()), before);
    assert!(!loaded.doc.pinned("remote"));
    assert!(
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(s.pile().join(".pile"))
            .unwrap()
            .try_lock()
            .is_err()
    );
}

#[test]
fn a_replaced_pile_inode_is_rejected_while_original_lock_stays_held() {
    let s = Scratch::new("inode");
    fs::create_dir(s.pile()).unwrap();
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x4e00).unwrap();
    let old = s.pile().join("old-pile");
    fs::rename(s.pile().join(".pile"), &old).unwrap();
    fs::write(s.pile().join(".pile"), b"").unwrap();
    let source = PinDoc::new(0x4e01).unwrap();
    source.set("remote", true).unwrap();
    assert_eq!(
        loaded
            .apply_checked(
                &spec(0, Default::default(), source.version()),
                &body(&source)
            )
            .unwrap_err(),
        ApplyError::FenceLost
    );
    assert!(!loaded.doc.pinned("remote"));
    assert!(
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(old)
            .unwrap()
            .try_lock()
            .is_err()
    );
}

#[test]
fn strict_epoch_distinguishes_invalid_io_missing_and_wrong_epoch() {
    let s = Scratch::new("epoch-errors");
    fs::create_dir(s.pile()).unwrap();
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x4f00).unwrap();
    let source = PinDoc::new(0x4f01).unwrap();
    source.set("remote", true).unwrap();
    let bytes = body(&source);
    let goal = spec(0, Default::default(), source.version());
    fs::write(s.pile().join(".epoch"), b"invalid").unwrap();
    assert_eq!(
        loaded.apply_checked(&goal, &bytes).unwrap_err(),
        ApplyError::InvalidEpoch
    );
    fs::write(s.pile().join(".epoch"), [0xff, 0xfe]).unwrap();
    assert_eq!(
        loaded.apply_checked(&goal, &bytes).unwrap_err(),
        ApplyError::InvalidEpoch
    );
    fs::remove_file(s.pile().join(".epoch")).unwrap();
    fs::create_dir(s.pile().join(".epoch")).unwrap();
    assert!(matches!(
        loaded.apply_checked(&goal, &bytes),
        Err(ApplyError::EpochIo(_))
    ));
    fs::remove_dir(s.pile().join(".epoch")).unwrap();
    assert!(matches!(
        loaded.apply_checked(&spec(1, Default::default(), source.version()), &bytes),
        Err(ApplyError::WrongEpoch { .. })
    ));
    loaded.apply_checked(&goal, &bytes).unwrap();
    assert!(!s.pile().join(".epoch").exists());
}

#[test]
fn a_chat_missing_epoch_remains_epoch_zero() {
    let s = Scratch::new("chat-zero");
    let mut loaded = store::load::<ChatDoc>(&s.pile(), 0x5000).unwrap();
    let source = ChatDoc::new(0x5001).unwrap();
    source.set_home("real-chat-home").unwrap();
    let result = loaded
        .apply_checked(
            &spec(0, Default::default(), source.version()),
            &source.snapshot().unwrap(),
        )
        .unwrap();
    assert_eq!(result.epoch(), 0);
    assert!(!s.pile().join(".epoch").exists());
    assert_eq!(loaded.doc.home().as_deref(), Some("real-chat-home"));
    drop(loaded);
    let reopened = store::load::<ChatDoc>(&s.pile(), 0x5002).unwrap();
    assert_eq!(reopened.doc.home().as_deref(), Some("real-chat-home"));
}

#[test]
fn absent_pile_initialization_rejects_blocks_that_appeared_before_the_fence() {
    let s = Scratch::new("init-conflict");
    let mut loaded = store::load_pile::<PinDoc>(&s.pile(), 0x5010).unwrap();
    let source = PinDoc::new(0x5011).unwrap();
    source.set("remote", true).unwrap();
    fs::create_dir(s.pile()).unwrap();
    let path = s.pile().join("u-0000000000005011-00000000.loro");
    let bytes = body(&source);
    fs::write(&path, &bytes).unwrap();
    assert_eq!(
        loaded
            .apply_checked(&spec(1, Default::default(), source.version()), &bytes)
            .unwrap_err(),
        ApplyError::NeedsReload
    );
    assert!(!loaded.doc.pinned("remote"));
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert!(!s.pile().join(".epoch").exists());
}

#[test]
fn candidate_rejection_preserves_existing_pen_epoch_ledger_and_blocks() {
    let s = Scratch::new("existing-reject");
    fs::create_dir(s.pile()).unwrap();
    fs::write(s.pile().join(".epoch"), b"1\n").unwrap();
    let mut loaded = store::load_pile::<PinDoc>(&s.pile(), 0x5020).unwrap();
    loaded.doc.set("local", true).unwrap();
    loaded.store.flush(&loaded.doc).unwrap();
    let before = loaded.doc.version();
    let disk = image(&s.pile());
    let (source, _, edit) = pieces(0x5021);
    assert_eq!(
        loaded
            .apply_checked(&spec(1, before.clone(), source.version()), &edit)
            .unwrap_err(),
        ApplyError::PendingImport
    );
    assert_eq!(loaded.doc.version(), before);
    assert_eq!(image(&s.pile()), disk);
    assert!(loaded.doc.pinned("local"));
}

struct Worker {
    child: Child,
    release: PathBuf,
}
impl Worker {
    fn start(dir: &Path, epoch: &str) -> Self {
        let release = dir.join("release");
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "pile_lock_worker", "--ignored", "--nocapture"])
            .env("KHOR_APPLY_LOCK_WORKER", dir)
            .env("KHOR_APPLY_WORKER_EPOCH", epoch)
            .spawn()
            .unwrap();
        let mut worker = Self { child, release };
        let start = Instant::now();
        while !dir.join("ready").exists() {
            assert!(
                worker.child.try_wait().unwrap().is_none(),
                "owned lock worker exited before ready"
            );
            assert!(
                start.elapsed() < Duration::from_secs(3),
                "owned lock worker did not reach barrier"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        worker
    }
    fn finish(mut self) {
        fs::write(&self.release, b"release").unwrap();
        assert!(self.child.wait().unwrap().success());
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = fs::write(&self.release, b"release");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
#[ignore = "subprocess entry point; ordinary parent tests execute and reap it"]
fn pile_lock_worker() {
    let dir = PathBuf::from(std::env::var_os("KHOR_APPLY_LOCK_WORKER").expect("owned worker only"));
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join(".pile"))
        .unwrap();
    file.try_lock().unwrap();
    let epoch = std::env::var("KHOR_APPLY_WORKER_EPOCH").unwrap();
    if !epoch.is_empty() {
        fs::write(dir.join(".epoch"), epoch).unwrap();
    }
    fs::write(dir.join("ready"), b"ready").unwrap();
    let start = Instant::now();
    while !dir.join("release").exists() && start.elapsed() < Duration::from_secs(18) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(dir.join("release").exists(), "owned worker fuse expired");
}

#[test]
fn an_unfenced_loaded_rechecks_disk_epoch_after_an_owned_process_changes_it() {
    let s = Scratch::new("stale-epoch");
    let mut loaded = store::load_pile::<PinDoc>(&s.pile(), 0x5100).unwrap();
    fs::create_dir(s.pile()).unwrap();
    Worker::start(&s.pile(), "2\n").finish();
    let source = PinDoc::new(0x5101).unwrap();
    source.set("remote", true).unwrap();
    assert_eq!(
        loaded
            .apply_checked(
                &spec(1, Default::default(), source.version()),
                &body(&source)
            )
            .unwrap_err(),
        ApplyError::StaleEpoch {
            expected: 1,
            disk: 2
        }
    );
    assert!(!loaded.doc.pinned("remote"));
    assert_eq!(fs::read_to_string(s.pile().join(".epoch")).unwrap(), "2\n");
}

#[test]
fn a_real_pile_lock_timeout_never_falls_back_to_an_unfenced_import() {
    let s = Scratch::new("lock-timeout");
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x5200).unwrap();
    fs::create_dir(s.pile()).unwrap();
    let worker = Worker::start(&s.pile(), "");
    let source = PinDoc::new(0x5201).unwrap();
    source.set("remote", true).unwrap();
    let start = Instant::now();
    assert!(matches!(
        loaded.apply_checked(
            &spec(0, Default::default(), source.version()),
            &body(&source)
        ),
        Err(ApplyError::FenceUnavailable(_))
    ));
    let elapsed = start.elapsed();
    worker.finish();
    assert!(
        elapsed >= Duration::from_secs(9) && elapsed < Duration::from_secs(14),
        "bounded lock wait was {elapsed:?}"
    );
    assert!(!loaded.doc.pinned("remote"));
    assert!(!s.pile().join(".merged").exists());
}

#[test]
fn block_absorb_pending_must_not_be_discarded_by_compaction() {
    let s = Scratch::new("absorb-pending");
    let (source, dependency, edit) = pieces(0x5300);
    let mut loaded = store::load::<PinDoc>(&s.pile(), 0x5302).unwrap();
    let edit_name = "u-0000000000005300-00000001.loro";
    loaded.store.absorb(&loaded.doc, edit_name, &edit).unwrap();
    let compact = loaded.store.compact(&loaded.doc);
    let retained = s.pile().join(edit_name).exists();
    drop(loaded);
    fs::write(
        s.pile().join("u-0000000000005300-00000000.loro"),
        dependency,
    )
    .unwrap();
    let reopened = store::load::<PinDoc>(&s.pile(), 0x5303).unwrap();
    eprintln!(
        "[Q039-ABSORB] compact={compact:?} retained={retained} edit={} coverage={}",
        reopened.doc.pinned("pending-edit"),
        reopened.doc.version().includes_vv(&source.version())
    );
    assert!(
        reopened.doc.pinned("pending-edit"),
        "absorb must register pending before compaction deletes its only block"
    );
    assert!(reopened.doc.version().includes_vv(&source.version()));
    assert!(compact.is_err());
    assert!(retained);
}
