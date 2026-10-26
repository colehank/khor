//! Current version metadata and receiving use real Doc/Loro/filesystem stages.
use khor_sync::{
    doc_transfer::{
        self, BodyError, Descriptor, DocumentVersion, Encoding, PrepareSpec, Prepared,
        ReceiveError, ReceiveSpec,
    },
    pins::PinDoc,
    store::{self, ApplyError, Doc},
    transfer::{CHUNK_BYTES, Error, MAX_METADATA_BYTES, Manifest, Progress, Sink, TransferStore},
};
use loro::{ExportMode, LoroDoc, VersionVector};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new(case: &str) -> Self {
        let base = std::env::var_os("KHOR_RECEIVE_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).unwrap();
        let dir = base.join(format!(
            "khor-receive-{case}-{}-{}",
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
    fn stage(&self, manifest: &Manifest) -> PathBuf {
        self.0
            .join("transfers")
            .join(manifest.id().unwrap().to_string())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn receive_spec() -> ReceiveSpec {
    ReceiveSpec {
        document: "pins".into(),
        source: [11; 32],
        recipient: [12; 32],
    }
}
fn manifest(doc: &impl Doc, base: VersionVector, encoding: Encoding, bytes: &[u8]) -> Manifest {
    Descriptor {
        document: "pins".into(),
        epoch: 1,
        encoding,
        base,
        target: doc.version(),
    }
    .bind(
        [11; 32],
        [12; 32],
        bytes.len() as u64,
        *blake3::hash(bytes).as_bytes(),
    )
    .unwrap()
}
fn stage(store: &TransferStore, manifest: &Manifest, bytes: &[u8]) -> Sink {
    let mut sink = store.receive(manifest.clone()).unwrap();
    for (i, chunk) in bytes.chunks(CHUNK_BYTES).enumerate() {
        sink.append((i * CHUNK_BYTES) as u64, chunk).unwrap();
    }
    assert!(matches!(
        sink.progress().unwrap(),
        Progress::VerifiedComplete { .. }
    ));
    sink
}
fn large_key() -> String {
    let mut state = 0xcaffeed123456789u64;
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
fn version_metadata_has_fixed_canonical_bytes_and_normalizes_zero_writers() {
    let value = DocumentVersion {
        epoch: 7,
        version: [(9, 3), (1, 2)].into_iter().collect(),
    };
    let expected=b"khor-version-v1\0\0\0\0\x07\0\0\0\x02\0\0\0\0\0\0\0\x01\0\0\0\x02\0\0\0\0\0\0\0\x09\0\0\0\x03";
    assert_eq!(value.encode().unwrap(), expected);
    assert_eq!(DocumentVersion::decode(expected).unwrap(), value);
    let normalized = DocumentVersion {
        epoch: 7,
        version: [(1, 2), (6, 0), (9, 3)].into_iter().collect(),
    };
    assert_eq!(normalized.encode().unwrap(), expected);
    assert_eq!(
        DocumentVersion {
            epoch: 8,
            ..value.clone()
        }
        .encode()
        .unwrap()[16..20],
        8u32.to_be_bytes()
    );
}

#[test]
fn malformed_noncanonical_and_oversized_version_metadata_is_rejected() {
    let value = DocumentVersion {
        epoch: 1,
        version: [(1, 2), (9, 3)].into_iter().collect(),
    };
    let valid = value.encode().unwrap();
    for length in 0..valid.len() {
        assert!(
            DocumentVersion::decode(&valid[..length]).is_err(),
            "accepted prefix {length}"
        );
    }
    let mut extra = valid.clone();
    extra.push(0);
    assert!(DocumentVersion::decode(&extra).is_err());
    for (at, bytes) in [
        (0, vec![b'X']),
        (20, u32::MAX.to_be_bytes().to_vec()),
        (32, 0u32.to_be_bytes().to_vec()),
        (32, u32::MAX.to_be_bytes().to_vec()),
    ] {
        let mut bad = valid.clone();
        bad[at..at + bytes.len()].copy_from_slice(&bytes);
        assert!(DocumentVersion::decode(&bad).is_err());
    }
    let mut duplicate = valid.clone();
    duplicate[36..44].copy_from_slice(&1u64.to_be_bytes());
    assert!(DocumentVersion::decode(&duplicate).is_err());
    let mut reversed = valid.clone();
    reversed[24..36].copy_from_slice(&valid[36..48]);
    reversed[36..48].copy_from_slice(&valid[24..36]);
    assert!(DocumentVersion::decode(&reversed).is_err());
    let mut negative = value.clone();
    negative.version.insert(4, -1);
    assert!(matches!(negative.encode(), Err(Error::InvalidManifest(_))));
    let count = (MAX_METADATA_BYTES - 24) / 12;
    let bounded = DocumentVersion {
        epoch: 1,
        version: (0..count as u64).map(|p| (p, 1)).collect(),
    };
    let bytes = bounded.encode().unwrap();
    assert!(bytes.len() <= MAX_METADATA_BYTES);
    assert_eq!(DocumentVersion::decode(&bytes).unwrap(), bounded);
    let mut overflow = bounded;
    overflow.version.insert(count as u64, 1);
    assert!(matches!(overflow.encode(), Err(Error::ResourceLimit(_))));
    assert!(matches!(
        DocumentVersion::decode(&vec![0; MAX_METADATA_BYTES + 1]),
        Err(Error::ResourceLimit(_))
    ));
}

#[test]
fn real_prepared_multichunk_stage_applies_after_source_drop_and_keeps_local_ops() {
    let big = large_key();
    for encoding in [Encoding::Updates, Encoding::Snapshot] {
        let sender = Scratch::new("sender");
        let recipient = Scratch::new("recipient");
        let outbound_store = sender.transfers();
        let inbound_store = recipient.transfers();
        let mut source = store::load_pile::<PinDoc>(&sender.pile(), 100).unwrap();
        source.doc.set("remote", true).unwrap();
        source.doc.set(&big, true).unwrap();
        let request = PrepareSpec {
            document: "pins".into(),
            source: [11; 32],
            recipient: [12; 32],
            receiver_epoch: 1,
            base: Default::default(),
            encoding,
        };
        let mut outbound =
            match doc_transfer::prepare(&mut source, &request, &outbound_store).unwrap() {
                Prepared::Transfer(value) => value,
                _ => panic!("expected prepared transfer"),
            };
        let fixed = outbound.descriptor().target.clone();
        let manifest = outbound.manifest().clone();
        assert!(manifest.total_bytes > CHUNK_BYTES as u64);
        drop(source);
        let mut sink = inbound_store.receive(manifest.clone()).unwrap();
        let mut receiver = store::load_pile::<PinDoc>(&recipient.pile(), 101).unwrap();
        receiver.doc.set("local-concurrent", true).unwrap();
        let local = receiver.doc.version();
        loop {
            let at = sink.next_offset();
            let chunk = outbound.read_chunk(at).unwrap();
            if chunk.is_empty() {
                break;
            }
            sink.append(at, &chunk).unwrap();
            if at == 0 {
                assert!(matches!(
                    doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
                    Err(ReceiveError::Incomplete { next_offset }) if next_offset == CHUNK_BYTES as u64
                ));
                assert_eq!(receiver.doc.version(), local);
                assert!(!receiver.doc.pinned("remote"));
                assert!(!recipient.pile().exists());
            }
        }
        let flushed =
            doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink).unwrap();
        assert_eq!(flushed.version(), &receiver.doc.version());
        assert_eq!(flushed.epoch(), 1);
        assert!(flushed.block().unwrap().is_file());
        assert!(
            receiver.doc.version().includes_vv(&fixed)
                && receiver.doc.version().includes_vv(&local)
        );
        assert!(
            receiver.doc.pinned("remote")
                && receiver.doc.pinned(&big)
                && receiver.doc.pinned("local-concurrent")
        );
        let actual = receiver.doc.version();
        drop(receiver);
        assert_eq!(
            store::load_pile::<PinDoc>(&recipient.pile(), 102)
                .unwrap()
                .doc
                .version(),
            actual
        );
        assert!(recipient.stage(&manifest).is_dir());
        assert!(matches!(inbound_store.cancel(&manifest), Err(Error::Busy)));
        drop(sink);
        assert!(matches!(
            inbound_store
                .receive(manifest.clone())
                .unwrap()
                .progress()
                .unwrap(),
            Progress::VerifiedComplete { .. }
        ));
        assert!(
            recipient.stage(&manifest).is_dir(),
            "successful local flush does not clear a stage before Node receipt"
        );
    }
}

#[test]
fn authorized_binding_and_complete_stage_are_required_before_any_merge() {
    let s = Scratch::new("binding");
    let store = s.transfers();
    let source = PinDoc::new(200).unwrap();
    source.set("remote", true).unwrap();
    let bytes = source.snapshot().unwrap();
    let manifest = manifest(&source, Default::default(), Encoding::Snapshot, &bytes);
    let mut sink = store.receive(manifest.clone()).unwrap();
    let mut receiver = store::load_pile::<PinDoc>(&s.pile(), 201).unwrap();
    let before = receiver.doc.version();
    let mut expected = receive_spec();
    expected.document = "seen".into();
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &expected, &mut sink),
        Err(ReceiveError::WrongDocument { .. })
    ));
    expected = receive_spec();
    expected.source = [99; 32];
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &expected, &mut sink),
        Err(ReceiveError::WrongSource)
    ));
    expected = receive_spec();
    expected.recipient = [99; 32];
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &expected, &mut sink),
        Err(ReceiveError::WrongRecipient)
    ));
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
        Err(ReceiveError::Incomplete { next_offset: 0 })
    ));
    assert_eq!(receiver.doc.version(), before);
    assert!(!s.pile().exists());
    assert!(s.stage(&manifest).is_dir());
    sink.append(0, &bytes).unwrap();
    assert!(doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink).is_ok());
}

#[test]
fn noncanonical_descriptor_is_rejected_without_touching_the_doc() {
    let s = Scratch::new("descriptor");
    let store = s.transfers();
    let source = PinDoc::new(250).unwrap();
    source.set("remote", true).unwrap();
    let bytes = source.snapshot().unwrap();
    let mut manifest = manifest(&source, Default::default(), Encoding::Snapshot, &bytes);
    manifest.binding.push(0);
    let mut sink = stage(&store, &manifest, &bytes);
    let mut receiver = store::load_pile::<PinDoc>(&s.pile(), 251).unwrap();
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
        Err(ReceiveError::Transfer(Error::InvalidManifest(_)))
    ));
    assert!(!s.pile().exists());
    assert!(!receiver.doc.pinned("remote"));
    assert!(s.stage(&manifest).is_dir());
}

#[test]
fn verified_body_is_rechecked_for_digest_checksum_and_complete_mode() {
    let s = Scratch::new("body");
    let store = s.transfers();
    let source = PinDoc::new(300).unwrap();
    source.set("remote", true).unwrap();
    let snapshot = source.snapshot().unwrap();
    let mut receiver = store::load_pile::<PinDoc>(&s.pile(), 301).unwrap();
    let valid = manifest(&source, Default::default(), Encoding::Snapshot, &snapshot);
    let mut sink = stage(&store, &valid, &snapshot);
    let path = s.stage(&valid).join("body");
    let mut changed = snapshot.clone();
    let last = changed.len() - 1;
    changed[last] ^= 1;
    fs::write(&path, &changed).unwrap();
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
        Err(ReceiveError::Body(BodyError::Transfer(
            Error::DigestMismatch
        )))
    ));
    drop(sink);
    store.cancel(&valid).unwrap();
    // A matching transfer digest does not replace the embedded Loro checksum.
    let corrupted = manifest(&source, Default::default(), Encoding::Snapshot, &changed);
    let mut sink = stage(&store, &corrupted, &changed);
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
        Err(ReceiveError::Body(BodyError::InvalidLoro(_)))
    ));
    drop(sink);
    store.cancel(&corrupted).unwrap();
    let wrong = manifest(&source, Default::default(), Encoding::Updates, &snapshot);
    let mut sink = stage(&store, &wrong, &snapshot);
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
        Err(ReceiveError::Body(BodyError::EncodingMismatch { .. }))
    ));
    drop(sink);
    store.cancel(&wrong).unwrap();
    let loro = LoroDoc::new();
    loro.set_peer_id(302).unwrap();
    loro.get_map("pins").insert("remote", true).unwrap();
    loro.commit();
    let shallow = loro
        .export(ExportMode::shallow_snapshot(&loro.state_frontiers()))
        .unwrap();
    let shallow_manifest = Descriptor {
        document: "pins".into(),
        epoch: 1,
        encoding: Encoding::Snapshot,
        base: Default::default(),
        target: loro.oplog_vv(),
    }
    .bind(
        [11; 32],
        [12; 32],
        shallow.len() as u64,
        *blake3::hash(&shallow).as_bytes(),
    )
    .unwrap();
    let mut sink = stage(&store, &shallow_manifest, &shallow);
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
        Err(ReceiveError::Body(BodyError::EncodingMismatch { .. }))
    ));
    assert!(!s.pile().exists());
    assert!(receiver.doc.version().is_empty());
    assert!(s.stage(&shallow_manifest).is_dir());
}

#[test]
fn strict_apply_epoch_base_and_pending_errors_stay_typed_and_keep_the_stage() {
    let s = Scratch::new("apply-errors");
    let store = s.transfers();
    let source = PinDoc::new(400).unwrap();
    source.set("dependency", true).unwrap();
    let first = source.version();
    source.set("pending-edit", true).unwrap();
    let edit = source.changes_since(&first).unwrap();
    let all = source.snapshot().unwrap();
    let mut receiver = store::load_pile::<PinDoc>(&s.pile(), 401).unwrap();
    let mut epoch = manifest(&source, Default::default(), Encoding::Snapshot, &all);
    let mut descriptor = Descriptor::from_manifest(&epoch).unwrap();
    descriptor.epoch = 2;
    epoch = descriptor
        .bind(
            epoch.source,
            epoch.recipient,
            epoch.total_bytes,
            epoch.digest,
        )
        .unwrap();
    let mut sink = stage(&store, &epoch, &all);
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
        Err(ReceiveError::Apply(ApplyError::WrongEpoch {
            expected: 2,
            loaded: 1
        }))
    ));
    assert!(s.stage(&epoch).is_dir());
    drop(sink);
    store.cancel(&epoch).unwrap();
    let base = manifest(&source, first.clone(), Encoding::Snapshot, &all);
    let mut sink = stage(&store, &base, &all);
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
        Err(ReceiveError::Apply(ApplyError::StaleBase))
    ));
    assert!(s.stage(&base).is_dir());
    drop(sink);
    store.cancel(&base).unwrap();
    let pending = manifest(&source, Default::default(), Encoding::Updates, &edit);
    let mut sink = stage(&store, &pending, &edit);
    assert!(matches!(
        doc_transfer::apply_staged(&mut receiver, &receive_spec(), &mut sink),
        Err(ReceiveError::Apply(ApplyError::PendingImport))
    ));
    assert!(s.stage(&pending).is_dir());
    assert!(!receiver.doc.pinned("dependency") && !receiver.doc.pinned("pending-edit"));
    assert!(!s.pile().exists());
}

#[test]
fn truncated_verified_stage_reports_eof_without_importing() {
    let s = Scratch::new("eof");
    let store = s.transfers();
    let source = PinDoc::new(500).unwrap();
    source.set("remote", true).unwrap();
    let bytes = source.snapshot().unwrap();
    let manifest = manifest(&source, Default::default(), Encoding::Snapshot, &bytes);
    let mut sink = stage(&store, &manifest, &bytes);
    let mut receiver = store::load_pile::<PinDoc>(&s.pile(), 501).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(s.stage(&manifest).join("body"))
        .unwrap()
        .set_len(1)
        .unwrap();
    assert!(
        matches!(doc_transfer::apply_staged(&mut receiver,&receive_spec(),&mut sink),Err(ReceiveError::Transfer(Error::Io(why))) if why.kind()==std::io::ErrorKind::UnexpectedEof)
    );
    assert!(!receiver.doc.pinned("remote"));
    assert!(!s.pile().exists());
    assert!(s.stage(&manifest).is_dir());
}
