//! Outbound contracts use real Loro documents, block flushes and byte stages.
use khor_sync::{
    doc_transfer::{self, Descriptor, Encoding, Outbound, OutboundError, PrepareSpec, Prepared},
    pins::PinDoc,
    store::{self, ApplySpec, Doc, MergeReport},
    transfer::{CHUNK_BYTES, Cancelled, Error, MAX_METADATA_BYTES, Progress, TransferStore},
};
use loro::{EncodedBlobMode, ExportMode, LoroDoc, VersionVector};
use std::{
    cell::{Cell, RefCell},
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new(case: &str) -> Self {
        let base = std::env::var_os("KHOR_OUTBOUND_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).unwrap();
        let dir = base.join(format!(
            "khor-outbound-{case}-{}-{}",
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
    fn transfers(&self) -> TransferStore {
        TransferStore::open(&self.0.join("transfers")).unwrap()
    }
    fn stage_count(&self) -> usize {
        fs::read_dir(self.0.join("transfers"))
            .unwrap()
            .filter(|entry| entry.as_ref().unwrap().file_name().len() == 64)
            .count()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn spec(base: VersionVector, encoding: Encoding) -> PrepareSpec {
    PrepareSpec {
        document: "pins".into(),
        source: [11; 32],
        recipient: [12; 32],
        receiver_epoch: 1,
        base,
        encoding,
    }
}
fn transfer(prepared: Prepared) -> Outbound {
    match prepared {
        Prepared::Transfer(outbound) => outbound,
        Prepared::UpToDate { .. } => panic!("expected transfer"),
    }
}
fn read(outbound: &mut Outbound) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let chunk = outbound.read_chunk(bytes.len() as u64).unwrap();
        if chunk.is_empty() {
            break;
        }
        assert_eq!(
            chunk.len(),
            (outbound.manifest().total_bytes - bytes.len() as u64).min(CHUNK_BYTES as u64) as usize
        );
        bytes.extend(chunk);
    }
    assert_eq!(blake3::hash(&bytes).as_bytes(), &outbound.manifest().digest);
    bytes
}
fn large_key() -> String {
    // Deterministic, high entropy keys keep the real compressed blob multi-chunk.
    let mut state = 0x12345678abcdef01u64;
    (0..900_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_"
                [(state & 63) as usize] as char
        })
        .collect()
}

#[test]
fn real_multichunk_exports_survive_loaded_drop_and_strictly_apply() {
    let big = large_key();
    for encoding in [Encoding::Updates, Encoding::Snapshot] {
        let s = Scratch::new("roundtrip");
        let store = s.transfers();
        let mut source = store::load_pile::<PinDoc>(&s.pile(), 100).unwrap();
        source.doc.set("first", true).unwrap();
        source.doc.set(&big, true).unwrap();
        let target = source.doc.version();
        let mut outbound = transfer(
            doc_transfer::prepare(&mut source, &spec(Default::default(), encoding), &store)
                .unwrap(),
        );
        assert!(outbound.manifest().total_bytes > CHUNK_BYTES as u64);
        assert_eq!(outbound.descriptor().target, target);
        source.doc.set("late-local", true).unwrap();
        source.store.flush(&source.doc).unwrap();
        drop(source);
        assert!(
            store::load_pile::<PinDoc>(&s.pile(), 101)
                .unwrap()
                .doc
                .pinned("late-local")
        );
        let bytes = read(&mut outbound);
        let meta = LoroDoc::decode_import_blob_meta(&bytes, true).unwrap();
        assert_eq!(
            meta.mode,
            match encoding {
                Encoding::Updates => EncodedBlobMode::Updates,
                Encoding::Snapshot => EncodedBlobMode::Snapshot,
            }
        );
        let mut receiver = store::load_pile::<PinDoc>(&s.0.join("receiver"), 102).unwrap();
        receiver
            .apply_checked(
                &ApplySpec {
                    expected_epoch: 1,
                    base_vv: Default::default(),
                    target_vv: target.clone(),
                },
                &bytes,
            )
            .unwrap();
        assert_eq!(receiver.doc.version(), target);
        assert!(receiver.doc.pinned("first") && receiver.doc.pinned(&big));
        assert!(!receiver.doc.pinned("late-local"));
        drop(receiver);
        assert_eq!(
            store::load_pile::<PinDoc>(&s.0.join("receiver"), 103)
                .unwrap()
                .doc
                .version(),
            target
        );
        assert!(matches!(
            outbound.read_chunk(1),
            Err(Error::InvalidChunk(_))
        ));
        assert!(matches!(
            outbound.read_chunk(u64::MAX),
            Err(Error::InvalidChunk(_))
        ));
        assert_eq!(outbound.discard().unwrap(), Cancelled::Removed);
        assert_eq!(s.stage_count(), 0);
    }
}

#[test]
fn concurrent_base_and_partial_metadata_preserve_the_full_fixed_target() {
    let s = Scratch::new("concurrent");
    let store = s.transfers();
    let common = PinDoc::new(200).unwrap();
    common.set("common", true).unwrap();
    let mut source = store::load_pile::<PinDoc>(&s.pile(), 201).unwrap();
    source.doc.merge(&common.snapshot().unwrap()).unwrap();
    source.doc.set("sender", true).unwrap();
    let mut receiver = store::load_pile::<PinDoc>(&s.0.join("receiver"), 202).unwrap();
    receiver.doc.merge(&common.snapshot().unwrap()).unwrap();
    receiver.doc.set("receiver", true).unwrap();
    receiver.store.flush(&receiver.doc).unwrap();
    let base = receiver.doc.version();
    let target = source.doc.version();
    assert!(!base.includes_vv(&target) && !target.includes_vv(&base));
    let mut outbound = transfer(
        doc_transfer::prepare(&mut source, &spec(base.clone(), Encoding::Updates), &store).unwrap(),
    );
    let bytes = read(&mut outbound);
    let meta = LoroDoc::decode_import_blob_meta(&bytes, true).unwrap();
    assert_ne!(
        meta.partial_end_vv, target,
        "partial metadata is not the complete captured target"
    );
    assert_eq!(outbound.descriptor().target, target);
    receiver
        .apply_checked(
            &ApplySpec {
                expected_epoch: 1,
                base_vv: base.clone(),
                target_vv: target.clone(),
            },
            &bytes,
        )
        .unwrap();
    for key in ["common", "sender", "receiver"] {
        assert!(receiver.doc.pinned(key));
    }
    assert!(
        receiver.doc.version().includes_vv(&base) && receiver.doc.version().includes_vv(&target)
    );
}

#[test]
fn epoch_and_metadata_fail_before_flush_and_up_to_date_never_stages() {
    let s = Scratch::new("admission");
    let store = s.transfers();
    let mut source = store::load_pile::<PinDoc>(&s.pile(), 300).unwrap();
    source.doc.set("dirty", true).unwrap();
    let mut request = spec(Default::default(), Encoding::Updates);
    request.receiver_epoch = 2;
    assert!(matches!(
        doc_transfer::prepare(&mut source, &request, &store),
        Err(OutboundError::WrongEpoch {
            receiver: 2,
            loaded: 1
        })
    ));
    assert!(!s.pile().exists());
    request.receiver_epoch = 1;
    request.base.insert(1, -1);
    assert!(matches!(
        doc_transfer::prepare(&mut source, &request, &store),
        Err(OutboundError::Transfer(Error::InvalidManifest(_)))
    ));
    assert!(!s.pile().exists());
    request.base = Default::default();
    request.document = "x".repeat(MAX_METADATA_BYTES);
    assert!(matches!(
        doc_transfer::prepare(&mut source, &request, &store),
        Err(OutboundError::Transfer(Error::ResourceLimit(_)))
    ));
    assert!(!s.pile().exists());
    request.document = "pins".into();
    request.base = source.doc.version();
    request.base.insert(999, 1);
    match doc_transfer::prepare(&mut source, &request, &store).unwrap() {
        Prepared::UpToDate { epoch, version } => {
            assert_eq!(epoch, 1);
            assert_eq!(version, source.doc.version());
        }
        Prepared::Transfer(_) => panic!("receiver already covers sender"),
    }
    assert_eq!(s.stage_count(), 0);
    drop(source);
    assert!(
        store::load_pile::<PinDoc>(&s.pile(), 301)
            .unwrap()
            .doc
            .pinned("dirty")
    );
}

#[test]
fn same_manifest_resumes_and_drop_releases_only_its_owner() {
    let s = Scratch::new("resume");
    let store = s.transfers();
    let mut source = store::load_pile::<PinDoc>(&s.pile(), 400).unwrap();
    source.doc.set(&large_key(), true).unwrap();
    let request = spec(Default::default(), Encoding::Updates);
    let bytes = source.doc.changes_since(&request.base).unwrap();
    assert!(bytes.len() > CHUNK_BYTES);
    let descriptor = Descriptor {
        document: request.document.clone(),
        epoch: 1,
        encoding: request.encoding,
        base: request.base.clone(),
        target: source.doc.version(),
    };
    let manifest = descriptor
        .bind(
            request.source,
            request.recipient,
            bytes.len() as u64,
            *blake3::hash(&bytes).as_bytes(),
        )
        .unwrap();
    let mut partial = store.receive(manifest.clone()).unwrap();
    partial.append(0, &bytes[..CHUNK_BYTES]).unwrap();
    drop(partial);
    let mut outbound = transfer(doc_transfer::prepare(&mut source, &request, &store).unwrap());
    assert_eq!(outbound.manifest(), &manifest);
    assert_eq!(read(&mut outbound), bytes);
    assert!(matches!(store.cancel(&manifest), Err(Error::Busy)));
    drop(outbound);
    let mut resumed = store.receive(manifest.clone()).unwrap();
    assert!(matches!(
        resumed.progress().unwrap(),
        Progress::VerifiedComplete { .. }
    ));
    let mut buffer = vec![0; CHUNK_BYTES];
    assert_eq!(resumed.read_verified(0, &mut buffer).unwrap(), CHUNK_BYTES);
    assert_eq!(buffer, &bytes[..CHUNK_BYTES]);
    drop(resumed);
    let mut other_request = request.clone();
    other_request.recipient = [13; 32];
    let other = transfer(doc_transfer::prepare(&mut source, &other_request, &store).unwrap());
    let other_manifest = other.manifest().clone();
    let outbound = transfer(doc_transfer::prepare(&mut source, &request, &store).unwrap());
    assert_eq!(outbound.discard().unwrap(), Cancelled::Removed);
    assert_eq!(s.stage_count(), 1);
    drop(other);
    let mut preserved = store.receive(other_manifest.clone()).unwrap();
    assert!(matches!(
        preserved.progress().unwrap(),
        Progress::VerifiedComplete { .. }
    ));
    assert!(preserved.read_verified(0, &mut buffer).unwrap() > 0);
    drop(preserved);
    assert_eq!(store.cancel(&other_manifest).unwrap(), Cancelled::Removed);
}

// Real Loro Doc with observable export boundaries and explicit malformed export
// modes. It does not replace the block store or transport implementation.
#[derive(Clone, Copy)]
enum Behavior {
    Normal,
    MutateFlush,
    MutateExport,
    Corrupt,
    WrongMode,
    Shallow,
}
struct ObservedDoc {
    inner: LoroDoc,
    pile: RefCell<Option<PathBuf>>,
    calls: Cell<usize>,
    persisted: RefCell<Vec<bool>>,
    behavior: Cell<Behavior>,
}
impl ObservedDoc {
    fn put(&self, key: &str) {
        self.inner.get_map("pins").insert(key, true).unwrap();
        self.inner.commit();
    }
    fn persisted(&self) -> bool {
        self.pile.borrow().as_ref().is_some_and(|dir| {
            fs::read_dir(dir)
                .into_iter()
                .flatten()
                .any(|e| e.unwrap().path().extension().is_some_and(|v| v == "loro"))
        })
    }
}
impl Doc for ObservedDoc {
    fn open(peer: u64) -> Result<Self, String> {
        let inner = LoroDoc::new();
        inner.set_peer_id(peer).map_err(|e| e.to_string())?;
        Ok(Self {
            inner,
            pile: RefCell::new(None),
            calls: Cell::new(0),
            persisted: RefCell::new(Vec::new()),
            behavior: Cell::new(Behavior::Normal),
        })
    }
    fn peer_id(&self) -> u64 {
        self.inner.peer_id()
    }
    fn version(&self) -> VersionVector {
        self.inner.oplog_vv()
    }
    fn changes_since(&self, vv: &VersionVector) -> Result<Vec<u8>, String> {
        let calls = self.calls.get() + 1;
        self.calls.set(calls);
        self.persisted.borrow_mut().push(self.persisted());
        let bytes = self
            .inner
            .export(ExportMode::updates(vv))
            .map_err(|e| e.to_string())?;
        match (self.behavior.get(), calls) {
            (Behavior::MutateFlush, 1) | (Behavior::MutateExport, 2) => self.put("racing-op"),
            (Behavior::Corrupt, 2) => {
                let mut bytes = bytes;
                let end = bytes.len() - 1;
                bytes[end] ^= 1;
                return Ok(bytes);
            }
            (Behavior::WrongMode, 2) => {
                return self
                    .inner
                    .export(ExportMode::Snapshot)
                    .map_err(|e| e.to_string());
            }
            _ => {}
        }
        Ok(bytes)
    }
    fn snapshot(&self) -> Result<Vec<u8>, String> {
        self.persisted.borrow_mut().push(self.persisted());
        let frontiers = self.inner.state_frontiers();
        self.inner
            .export(match self.behavior.get() {
                Behavior::Shallow => ExportMode::shallow_snapshot(&frontiers),
                _ => ExportMode::Snapshot,
            })
            .map_err(|e| e.to_string())
    }
    fn merge_report(&self, bytes: &[u8]) -> Result<MergeReport, String> {
        let result = self.inner.import(bytes).map_err(|e| e.to_string())?;
        Ok(MergeReport {
            applied: result
                .success
                .iter()
                .map(|(p, (s, e))| store::ImportSpan {
                    writer: *p,
                    start: *s,
                    end: *e,
                })
                .collect(),
            pending: result
                .pending
                .iter()
                .flat_map(|r| r.iter())
                .map(|(p, (s, e))| store::ImportSpan {
                    writer: *p,
                    start: *s,
                    end: *e,
                })
                .collect(),
        })
    }
    fn items(&self) -> usize {
        0
    }
}

#[test]
fn flush_really_precedes_both_export_modes_and_up_to_date_skips_export() {
    for encoding in [Encoding::Updates, Encoding::Snapshot] {
        let s = Scratch::new("flush-order");
        let store = s.transfers();
        let mut source = store::load_pile::<ObservedDoc>(&s.pile(), 500).unwrap();
        *source.doc.pile.borrow_mut() = Some(s.pile());
        source.doc.put("dirty");
        let outbound = transfer(
            doc_transfer::prepare(&mut source, &spec(Default::default(), encoding), &store)
                .unwrap(),
        );
        assert_eq!(*source.doc.persisted.borrow(), [false, true]);
        assert_eq!(outbound.descriptor().target, source.doc.version());
        source.doc.persisted.borrow_mut().clear();
        let version = source.doc.version();
        let prepared = doc_transfer::prepare(&mut source, &spec(version, encoding), &store);
        assert!(matches!(prepared, Ok(Prepared::UpToDate { .. })));
        assert!(source.doc.persisted.borrow().is_empty());
    }
}

#[test]
fn changing_versions_corrupt_bytes_and_wrong_or_shallow_modes_never_stage() {
    for behavior in [
        Behavior::MutateFlush,
        Behavior::MutateExport,
        Behavior::Corrupt,
        Behavior::WrongMode,
        Behavior::Shallow,
    ] {
        let s = Scratch::new("export-errors");
        let store = s.transfers();
        let mut source = store::load_pile::<ObservedDoc>(&s.pile(), 600).unwrap();
        source.doc.put("dirty");
        source.doc.behavior.set(behavior);
        let encoding = if matches!(behavior, Behavior::Shallow) {
            Encoding::Snapshot
        } else {
            Encoding::Updates
        };
        let result =
            doc_transfer::prepare(&mut source, &spec(Default::default(), encoding), &store);
        match behavior {
            Behavior::MutateFlush | Behavior::MutateExport => {
                assert!(matches!(result, Err(OutboundError::VersionChanged)))
            }
            Behavior::Corrupt => assert!(matches!(result, Err(OutboundError::InvalidBody(_)))),
            Behavior::WrongMode | Behavior::Shallow => assert!(matches!(
                result,
                Err(OutboundError::EncodingMismatch { .. })
            )),
            Behavior::Normal => unreachable!(),
        }
        assert_eq!(s.stage_count(), 0);
    }
}

#[test]
fn staging_errors_expose_the_exact_manifest_for_retry_or_cancel() {
    let s = Scratch::new("stage-error");
    let store = s.transfers();
    let mut source = store::load_pile::<PinDoc>(&s.pile(), 700).unwrap();
    source.doc.set("dirty", true).unwrap();
    let request = spec(Default::default(), Encoding::Updates);
    let outbound = transfer(doc_transfer::prepare(&mut source, &request, &store).unwrap());
    let manifest = outbound.manifest().clone();
    match doc_transfer::prepare(&mut source, &request, &store) {
        Err(OutboundError::Staging {
            manifest: failed,
            cause: Error::Busy,
        }) => assert_eq!(failed, manifest),
        _ => panic!("active stage must report Busy and its exact manifest"),
    }
    assert_eq!(s.stage_count(), 1);
    drop(outbound);
    let retried = transfer(doc_transfer::prepare(&mut source, &request, &store).unwrap());
    assert_eq!(retried.manifest(), &manifest);
    retried.discard().unwrap();
    assert_eq!(s.stage_count(), 0);
}

#[test]
fn real_flush_failure_never_creates_a_stage() {
    let s = Scratch::new("flush-failure");
    let store = s.transfers();
    let mut source = store::load_pile::<PinDoc>(&s.pile(), 800).unwrap();
    source.doc.set("dirty", true).unwrap();
    fs::write(s.pile(), b"blocks cannot be created under a file").unwrap();
    assert!(matches!(
        doc_transfer::prepare(
            &mut source,
            &spec(Default::default(), Encoding::Updates),
            &store
        ),
        Err(OutboundError::Flush(_))
    ));
    assert_eq!(s.stage_count(), 0);
}

#[test]
fn truncated_verified_body_fails_with_unexpected_eof() {
    let s = Scratch::new("truncated-read");
    let store = s.transfers();
    let mut source = store::load_pile::<PinDoc>(&s.pile(), 900).unwrap();
    source.doc.set("dirty", true).unwrap();
    let mut outbound = transfer(
        doc_transfer::prepare(
            &mut source,
            &spec(Default::default(), Encoding::Updates),
            &store,
        )
        .unwrap(),
    );
    let path =
        s.0.join("transfers")
            .join(outbound.manifest().id().unwrap().to_string())
            .join("body");
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_len(1)
        .unwrap();
    assert!(
        matches!(outbound.read_chunk(0), Err(Error::Io(why)) if why.kind() == std::io::ErrorKind::UnexpectedEof)
    );
    assert_eq!(outbound.discard().unwrap(), Cancelled::Removed);
}
