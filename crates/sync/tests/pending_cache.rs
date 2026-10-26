//! Pending CRDT increments must survive reads made before their dependencies.
//! Actual PinDoc exports and on-disk BlockStore loads; no replacement cache.
use base64::Engine;
use khor_sync::{
    pins::PinDoc,
    store::{self, Doc},
    wire,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const CREATE: &str = "tui/q039-pending-create";
const EDIT: &str = "tui/q039-pending-edit";
const EDIT_BLOCK: &str = "u-0000000000003901-00000000.loro";
const CREATE_BLOCK: &str = "u-0000000000003901-00000001.loro";

struct Scratch(PathBuf);
impl Scratch {
    fn new(case: &str) -> Self {
        let base = std::env::var_os("KHOR_PENDING_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).unwrap();
        let root = base.join(format!(
            "khor-pending-{case}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&root).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn pieces() -> (PinDoc, Vec<u8>, Vec<u8>) {
    let source = PinDoc::new(0x3901).unwrap();
    source.set(CREATE, true).unwrap();
    let after_create = source.version();
    // Capture the dependency before the edit exists. Re-exporting it later
    // would include the edit and could hide a lost pending increment.
    let creation_only = source.changes_since(&Default::default()).unwrap();
    source.set(EDIT, true).unwrap();
    let edit_only = source.changes_since(&after_create).unwrap();
    assert!(!creation_only.is_empty() && !edit_only.is_empty());

    let direct = PinDoc::new(0x3902).unwrap();
    direct
        .merge(&edit_only)
        .expect("ordinary merge must allow a pending batch");
    assert!(!direct.pinned(CREATE));
    assert!(!direct.pinned(EDIT));
    assert!(
        !direct.version().includes_vv(&source.version()),
        "an Ok import is not proof of target coverage"
    );
    direct.merge(&creation_only).unwrap();
    assert!(
        direct.pinned(CREATE) && direct.pinned(EDIT),
        "the dependency must replay the retained edit without resending it"
    );
    assert!(direct.version().includes_vv(&source.version()));
    (source, creation_only, edit_only)
}

fn assert_complete(dir: &Path, source: &PinDoc, loaded: &store::Loaded<PinDoc>) {
    eprintln!(
        "[Q039-PENDING] dir={} reads={:?} create={} edit={} target_covered={} broken={}",
        dir.display(),
        store::reads_of(dir),
        loaded.doc.pinned(CREATE),
        loaded.doc.pinned(EDIT),
        loaded.doc.version().includes_vv(&source.version()),
        loaded.broken.len()
    );
    assert!(
        loaded.doc.pinned(CREATE),
        "the dependency batch must be applied"
    );
    assert!(
        loaded.doc.pinned(EDIT),
        "a previously pending edit must survive cached reads and replay when its dependency arrives"
    );
    assert!(loaded.doc.version().includes_vv(&source.version()));
    assert!(
        loaded.broken.is_empty(),
        "legitimate pending blocks are not corrupt blocks"
    );
}

#[test]
fn a_cold_store_accepts_the_edit_before_its_dependency_in_one_read() {
    let scratch = Scratch::new("cold");
    let (source, creation, edit) = pieces();
    // Name ordering deliberately puts the edit before its dependency.
    fs::write(scratch.0.join(EDIT_BLOCK), edit).unwrap();
    fs::write(scratch.0.join(CREATE_BLOCK), creation).unwrap();
    let loaded = store::load::<PinDoc>(&scratch.0, 0x3903).unwrap();
    assert_complete(&scratch.0, &source, &loaded);
    drop(loaded);
    let reopened = store::load::<PinDoc>(&scratch.0, 0x3904).unwrap();
    assert_complete(&scratch.0, &source, &reopened);
}

#[test]
fn a_pending_edit_survives_cached_reads_before_its_dependency_arrives() {
    let scratch = Scratch::new("cached");
    let (source, creation, edit) = pieces();
    fs::write(scratch.0.join(EDIT_BLOCK), edit).unwrap();
    let first = store::load::<PinDoc>(&scratch.0, 0x3905).unwrap();
    assert!(!first.doc.pinned(EDIT));
    assert!(first.broken.is_empty());
    drop(first);
    let after_first = store::reads_of(&scratch.0);
    let warm = store::load::<PinDoc>(&scratch.0, 0x3906).unwrap();
    assert!(!warm.doc.pinned(EDIT));
    assert!(warm.broken.is_empty());
    drop(warm);
    eprintln!(
        "[Q039-PENDING] pending_before_dependency first_reads={after_first:?} warm_reads={:?} cache_hit={}",
        store::reads_of(&scratch.0),
        after_first == store::reads_of(&scratch.0)
    );
    // A correct implementation may avoid caching unresolved pending state, or
    // retain it faithfully. Cache-hit counts are diagnostic, not the contract.
    fs::write(scratch.0.join(CREATE_BLOCK), creation).unwrap();
    let after = store::load::<PinDoc>(&scratch.0, 0x3907).unwrap();
    assert_complete(&scratch.0, &source, &after);
}

#[test]
fn compaction_cannot_discard_an_edit_still_waiting_for_its_dependency() {
    let scratch = Scratch::new("compact");
    let (source, creation, edit) = pieces();
    let edit_path = scratch.0.join(EDIT_BLOCK);
    fs::write(&edit_path, edit).unwrap();
    let mut loaded = store::load::<PinDoc>(&scratch.0, 0x3908).unwrap();
    assert!(!loaded.doc.pinned(EDIT));
    assert!(loaded.broken.is_empty());
    let compacted = loaded.store.compact(&loaded.doc);
    eprintln!(
        "[Q039-PENDING] compaction={compacted:?} retained_edit_block={}",
        edit_path.exists()
    );
    drop(loaded);
    fs::write(scratch.0.join(CREATE_BLOCK), creation).unwrap();
    let after = store::load::<PinDoc>(&scratch.0, 0x3909).unwrap();
    assert_complete(&scratch.0, &source, &after);
    assert!(
        compacted.is_err(),
        "an unresolved import must defer compaction, not claim a complete snapshot"
    );
    assert!(
        edit_path.exists(),
        "the only copy of a pending edit must remain"
    );
}

#[test]
fn reports_distinguish_pending_edits_from_dependencies_that_apply_them() {
    let (source, creation, edit) = pieces();
    let dependency = PinDoc::new(0x3910).unwrap();
    dependency.merge(&creation).unwrap();
    let begin = dependency.version().get(&0x3901).copied().unwrap();
    let end = source.version().get(&0x3901).copied().unwrap();
    let receiver = PinDoc::new(0x3911).unwrap();
    let parked = receiver.merge_report(&edit).unwrap();
    assert!(parked.applied.is_empty());
    assert_eq!(
        parked.pending,
        vec![store::ImportSpan {
            writer: 0x3901,
            start: begin,
            end
        }]
    );
    assert!(!receiver.version().includes_vv(&source.version()));
    let replayed = receiver.merge_report(&creation).unwrap();
    assert!(replayed.pending.is_empty());
    assert_eq!(
        replayed.applied,
        vec![store::ImportSpan {
            writer: 0x3901,
            start: 0,
            end
        }]
    );
    assert!(receiver.pinned(CREATE) && receiver.pinned(EDIT));
}

fn live_imports_preserve_pending_until_dependencies_arrive(answering: bool) {
    let scratch = Scratch::new(if answering { "answer" } else { "absorb" });
    let (source, creation, edit) = pieces();
    let mut loaded = store::load::<PinDoc>(&scratch.0, 0x3912).unwrap();
    let mut peer = wire::Peer::new();
    for (index, bytes) in [&edit, &creation].into_iter().enumerate() {
        let changes = base64::engine::general_purpose::STANDARD.encode(bytes);
        if answering {
            wire::answer(&mut loaded.store, &loaded.doc, "", &changes).unwrap();
        } else {
            let sent = peer.outgoing(&mut loaded.store, &loaded.doc).unwrap();
            let reply = wire::Reply {
                version: wire::version_b64(&loaded.store, &source),
                changes,
                items: source.items(),
            };
            peer.absorb(&mut loaded.store, &loaded.doc, &sent, reply)
                .unwrap();
        }
        if index == 0 {
            assert!(!loaded.doc.pinned(EDIT));
            assert!(
                loaded.store.compact(&loaded.doc).is_err(),
                "a real live import must register pending before any compaction"
            );
        }
    }
    assert!(loaded.doc.pinned(CREATE) && loaded.doc.pinned(EDIT));
    loaded
        .store
        .compact(&loaded.doc)
        .expect("resolved pending must not permanently disable compaction");
    drop(loaded);
    let reopened = store::load::<PinDoc>(&scratch.0, 0x3913).unwrap();
    assert_complete(&scratch.0, &source, &reopened);
}

#[test]
fn wire_answer_registers_live_pending_before_compaction() {
    live_imports_preserve_pending_until_dependencies_arrive(true);
}

#[test]
fn wire_peer_absorb_registers_live_pending_before_compaction() {
    live_imports_preserve_pending_until_dependencies_arrive(false);
}
