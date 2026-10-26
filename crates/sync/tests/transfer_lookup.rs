//! Lookup contracts exercise the same real TransferStore owners and journals.
use khor_sync::{
    doc_transfer::{Descriptor, Encoding},
    transfer::{CHUNK_BYTES, Error, MAX_BLOB_BYTES, Manifest, Progress, TransferStore},
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new(case: &str) -> Self {
        let base = std::env::var_os("KHOR_TRANSFER_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).unwrap();
        let dir = base.join(format!(
            "khor-lookup-{case}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        Self(dir)
    }
    fn store(&self) -> TransferStore {
        TransferStore::open(&self.0).unwrap()
    }
    fn stage(&self, m: &Manifest) -> PathBuf {
        self.0.join(m.id().unwrap().to_string())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn manifest(binding: &[u8], bytes: &[u8]) -> Manifest {
    Manifest {
        source: [1; 32],
        recipient: [2; 32],
        binding: binding.into(),
        total_bytes: bytes.len() as u64,
        digest: *blake3::hash(bytes).as_bytes(),
    }
}

#[test]
fn missing_lookup_does_not_create_reserve_or_occupy_an_owner() {
    let s = Scratch::new("missing");
    let store = s.store();
    let mut absent = manifest(b"missing", b"not staged");
    absent.total_bytes = MAX_BLOB_BYTES;
    assert!(store.open_existing(&absent).unwrap().is_none());
    assert!(!s.stage(&absent).exists());
    assert!(!s.0.join("owner-0").exists() && !s.0.join("owner-1").exists());
    assert_eq!(store.reserved_bytes().unwrap(), 0);
    assert!(store.retained_manifests().unwrap().is_empty());
    // Both real owner slots remain available; Missing still returns None while
    // both slots are occupied, because no owner is needed for that outcome.
    let first = store.receive(manifest(b"first", b"one")).unwrap();
    let second = store.receive(manifest(b"second", b"two")).unwrap();
    assert!(store.open_existing(&absent).unwrap().is_none());
    assert_eq!(store.retained_manifests().unwrap().len(), 2);
    assert_eq!(store.reserved_bytes().unwrap(), 6);
    drop((first, second));
}

#[test]
fn existing_partial_and_complete_stages_use_the_same_owner_and_recovery_path() {
    let s = Scratch::new("reopen");
    let store = s.store();
    let bytes = vec![7; CHUNK_BYTES + 9];
    let meta = manifest(b"partial", &bytes);
    let mut original = store.receive(meta.clone()).unwrap();
    original.append(0, &bytes[..CHUNK_BYTES]).unwrap();
    drop(original);
    let mut opened = store.open_existing(&meta).unwrap().unwrap();
    assert_eq!(opened.next_offset(), CHUNK_BYTES as u64);
    assert!(
        matches!(opened.progress().unwrap(),Progress::Stored {next_offset} if next_offset==CHUNK_BYTES as u64)
    );
    assert!(matches!(store.open_existing(&meta), Err(Error::Busy)));
    assert!(matches!(store.receive(meta.clone()), Err(Error::Busy)));
    opened
        .append(CHUNK_BYTES as u64, &bytes[CHUNK_BYTES..])
        .unwrap();
    assert!(matches!(
        opened.progress().unwrap(),
        Progress::VerifiedComplete { .. }
    ));
    drop(opened);
    let mut complete = store.open_existing(&meta).unwrap().unwrap();
    assert!(matches!(
        complete.progress().unwrap(),
        Progress::VerifiedComplete { .. }
    ));
    let mut tail = vec![0; 9];
    assert_eq!(
        complete
            .read_verified(CHUNK_BYTES as u64, &mut tail)
            .unwrap(),
        9
    );
    assert_eq!(tail, &bytes[CHUNK_BYTES..]);
}

#[test]
fn retained_metadata_includes_opaque_and_doc_purposes_without_deleting_stages() {
    let s = Scratch::new("retained");
    let store = s.store();
    let opaque = manifest(&[0xff, 0, 9], b"abc");
    let empty = manifest(b"another purpose", b"");
    let doc = Descriptor {
        document: "pins".into(),
        epoch: 1,
        encoding: Encoding::Updates,
        base: Default::default(),
        target: [(9, 1)].into_iter().collect(),
    }
    .bind([1; 32], [2; 32], 2, *blake3::hash(b"xy").as_bytes())
    .unwrap();
    let mut active = store.receive(opaque.clone()).unwrap();
    active.append(0, b"abc").unwrap();
    drop(store.receive(empty.clone()).unwrap());
    drop(store.receive(doc.clone()).unwrap());
    let values = store.retained_manifests().unwrap();
    assert_eq!(values.len(), 3);
    for expected in [&opaque, &empty, &doc] {
        assert!(values.contains(expected));
        assert!(s.stage(expected).is_dir());
    }
    assert_eq!(store.reserved_bytes().unwrap(), 5);
    assert!(matches!(
        active.progress().unwrap(),
        Progress::VerifiedComplete { .. }
    ));
    assert!(matches!(store.open_existing(&opaque), Err(Error::Busy)));
    drop(active);
    assert_eq!(store.retained_manifests().unwrap(), values);
}

#[test]
fn a_different_or_invalid_manifest_never_touches_another_retained_id() {
    let s = Scratch::new("wrong");
    let store = s.store();
    let meta = manifest(b"published", b"hello");
    let mut sink = store.receive(meta.clone()).unwrap();
    sink.append(0, b"hello").unwrap();
    drop(sink);
    let before =
        ["manifest", "body", "offsets"].map(|name| fs::read(s.stage(&meta).join(name)).unwrap());
    let mut wrong = meta.clone();
    wrong.recipient[0] ^= 1;
    assert!(store.open_existing(&wrong).unwrap().is_none());
    assert!(!s.stage(&wrong).exists());
    wrong.total_bytes = MAX_BLOB_BYTES + 1;
    assert!(matches!(
        store.open_existing(&wrong),
        Err(Error::ResourceLimit(_))
    ));
    let after =
        ["manifest", "body", "offsets"].map(|name| fs::read(s.stage(&meta).join(name)).unwrap());
    assert_eq!(before, after);
    assert_eq!(store.retained_manifests().unwrap(), vec![meta]);
}
