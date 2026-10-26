//! Resume fixed outbound plans using only retained real filesystem stages.
use khor_sync::{
    doc_transfer::{
        self, BodyError, Descriptor, Encoding, Outbound, OutboundError, PrepareSpec, Prepared,
    },
    pins::PinDoc,
    store::{self, Doc},
    transfer::{CHUNK_BYTES, Cancelled, Error, Manifest, Progress, TransferStore},
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new(case: &str) -> Self {
        let base = std::env::var_os("KHOR_RESUME_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).unwrap();
        let dir = base.join(format!(
            "khor-resume-{case}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
    fn store(&self) -> TransferStore {
        TransferStore::open(self.0.join("transfers")).unwrap()
    }
    fn stage(&self, m: &Manifest) -> PathBuf {
        self.0.join("transfers").join(m.id().unwrap().to_string())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn manifest(doc: &impl Doc, encoding: Encoding, bytes: &[u8]) -> Manifest {
    Descriptor {
        document: "pins".into(),
        epoch: 1,
        encoding,
        base: Default::default(),
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
fn read(outbound: &mut Outbound) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let chunk = outbound.read_chunk(bytes.len() as u64).unwrap();
        if chunk.is_empty() {
            break;
        }
        bytes.extend(chunk);
    }
    assert_eq!(blake3::hash(&bytes).as_bytes(), &outbound.manifest().digest);
    bytes
}
fn large_key() -> String {
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
fn resumed_spool_keeps_the_original_target_after_source_reload_and_new_ops() {
    let big = large_key();
    for encoding in [Encoding::Updates, Encoding::Snapshot] {
        let s = Scratch::new("fixed");
        let transfers = s.store();
        let pile = s.0.join("pile");
        let mut source = store::load_pile::<PinDoc>(&pile, 100).unwrap();
        source.doc.set("original", true).unwrap();
        source.doc.set(&big, true).unwrap();
        let request = PrepareSpec {
            document: "pins".into(),
            source: [11; 32],
            recipient: [12; 32],
            receiver_epoch: 1,
            base: Default::default(),
            encoding,
        };
        let mut prepared = match doc_transfer::prepare(&mut source, &request, &transfers).unwrap() {
            Prepared::Transfer(value) => value,
            _ => panic!("expected transfer"),
        };
        let original = prepared.descriptor().clone();
        let meta = prepared.manifest().clone();
        let bytes = read(&mut prepared);
        assert!(bytes.len() > CHUNK_BYTES);
        drop(source);
        drop(prepared);
        drop(transfers);
        let mut later = store::load_pile::<PinDoc>(&pile, 101).unwrap();
        later.doc.set("later-local-op", true).unwrap();
        let later_version = later.doc.version();
        later.store.flush(&later.doc).unwrap();
        assert_ne!(later_version, original.target);
        drop(later);
        let reopened = s.store();
        let mut resumed = doc_transfer::resume(&reopened, &meta).unwrap().unwrap();
        assert_eq!(resumed.descriptor(), &original);
        assert_eq!(resumed.manifest(), &meta);
        assert_eq!(read(&mut resumed), bytes);
        // Same-root discard only releases this pinned plan, preserving another
        // already published opaque stage and its independent reservation.
        let other = Manifest {
            source: [1; 32],
            recipient: [2; 32],
            binding: b"other purpose".to_vec(),
            total_bytes: 3,
            digest: *blake3::hash(b"abc").as_bytes(),
        };
        let mut other_sink = reopened.receive(other.clone()).unwrap();
        other_sink.append(0, b"abc").unwrap();
        drop(other_sink);
        assert_eq!(resumed.discard().unwrap(), Cancelled::Removed);
        assert!(!s.stage(&meta).exists());
        assert!(s.stage(&other).is_dir());
        assert_eq!(reopened.retained_manifests().unwrap(), vec![other.clone()]);
        assert!(matches!(
            reopened
                .open_existing(&other)
                .unwrap()
                .unwrap()
                .progress()
                .unwrap(),
            Progress::VerifiedComplete { .. }
        ));
    }
}

#[test]
fn missing_and_noncanonical_plans_do_not_create_a_stage() {
    let s = Scratch::new("missing");
    let transfers = s.store();
    let source = PinDoc::new(200).unwrap();
    source.set("remote", true).unwrap();
    let bytes = source.snapshot().unwrap();
    let meta = manifest(&source, Encoding::Snapshot, &bytes);
    let mut invalid = meta.clone();
    invalid.binding.push(0);
    assert!(matches!(
        doc_transfer::resume(&transfers, &invalid),
        Err(OutboundError::Transfer(Error::InvalidManifest(_)))
    ));
    assert!(!s.0.join("transfers/gate").exists());
    assert!(doc_transfer::resume(&transfers, &meta).unwrap().is_none());
    assert_eq!(transfers.reserved_bytes().unwrap(), 0);
    assert!(transfers.retained_manifests().unwrap().is_empty());
    assert!(!s.stage(&meta).exists());
}

#[test]
fn partial_resume_reports_its_exact_manifest_and_keeps_confirmed_bytes() {
    let s = Scratch::new("partial");
    let transfers = s.store();
    let source = PinDoc::new(300).unwrap();
    source.set(&large_key(), true).unwrap();
    let bytes = source.snapshot().unwrap();
    assert!(bytes.len() > CHUNK_BYTES);
    let meta = manifest(&source, Encoding::Snapshot, &bytes);
    let mut sink = transfers.receive(meta.clone()).unwrap();
    sink.append(0, &bytes[..CHUNK_BYTES]).unwrap();
    drop(sink);
    let before = fs::read(s.stage(&meta).join("body")).unwrap();
    assert!(
        matches!(doc_transfer::resume(&transfers,&meta),Err(OutboundError::Staging {manifest,cause:Error::InvalidStage(_)}) if manifest==meta)
    );
    assert_eq!(fs::read(s.stage(&meta).join("body")).unwrap(), before);
    assert!(s.stage(&meta).is_dir());
    let mut sink = transfers.open_existing(&meta).unwrap().unwrap();
    assert_eq!(sink.next_offset(), CHUNK_BYTES as u64);
    let mut at = sink.next_offset() as usize;
    while at < bytes.len() {
        let end = (at + CHUNK_BYTES).min(bytes.len());
        sink.append(at as u64, &bytes[at..end]).unwrap();
        at = end;
    }
    drop(sink);
    let mut resumed = doc_transfer::resume(&transfers, &meta).unwrap().unwrap();
    assert_eq!(read(&mut resumed), bytes);
}

#[test]
fn wrong_mode_and_loro_checksum_errors_keep_typed_causes_and_staged_bytes() {
    let s = Scratch::new("invalid-body");
    let transfers = s.store();
    let source = PinDoc::new(400).unwrap();
    source.set("remote", true).unwrap();
    let snapshot = source.snapshot().unwrap();
    for corrupt in [false, true] {
        let mut bytes = snapshot.clone();
        let encoding = if corrupt {
            let last = bytes.len() - 1;
            bytes[last] ^= 1;
            Encoding::Snapshot
        } else {
            Encoding::Updates
        };
        let meta = manifest(&source, encoding, &bytes);
        let mut sink = transfers.receive(meta.clone()).unwrap();
        sink.append(0, &bytes).unwrap();
        drop(sink);
        match doc_transfer::resume(&transfers, &meta) {
            Err(OutboundError::StagingBody {
                manifest,
                cause: BodyError::EncodingMismatch { .. },
            }) if !corrupt => assert_eq!(manifest, meta),
            Err(OutboundError::StagingBody {
                manifest,
                cause: BodyError::InvalidLoro(_),
            }) if corrupt => assert_eq!(manifest, meta),
            _ => panic!("resume must preserve exact typed body failure"),
        }
        assert_eq!(fs::read(s.stage(&meta).join("body")).unwrap(), bytes);
        assert!(transfers.open_existing(&meta).unwrap().is_some());
    }
}

#[test]
fn corrupt_retained_core_bytes_and_busy_owner_are_not_hidden_or_deleted() {
    let s = Scratch::new("core-error");
    let transfers = s.store();
    let source = PinDoc::new(500).unwrap();
    source.set("remote", true).unwrap();
    let bytes = source.snapshot().unwrap();
    let meta = manifest(&source, Encoding::Snapshot, &bytes);
    let mut sink = transfers.receive(meta.clone()).unwrap();
    sink.append(0, &bytes).unwrap();
    assert!(
        matches!(doc_transfer::resume(&transfers,&meta),Err(OutboundError::Staging {manifest,cause:Error::Busy}) if manifest==meta)
    );
    drop(sink);
    let mut changed = bytes;
    let last = changed.len() - 1;
    changed[last] ^= 1;
    fs::write(s.stage(&meta).join("body"), &changed).unwrap();
    assert!(
        matches!(doc_transfer::resume(&transfers,&meta),Err(OutboundError::Staging {manifest,cause:Error::InvalidStage(_)}) if manifest==meta)
    );
    assert!(s.stage(&meta).is_dir());
    assert_eq!(fs::read(s.stage(&meta).join("body")).unwrap(), changed);
    assert_eq!(transfers.retained_manifests().unwrap(), vec![meta]);
}
