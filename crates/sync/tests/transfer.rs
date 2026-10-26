//! Q039 byte staging contract; no Doc import or Applied acknowledgement here.
use khor_sync::transfer::{
    CHUNK_BYTES, Cancelled, Error, MAX_BLOB_BYTES, MAX_METADATA_BYTES, MAX_RETAINED_STAGES,
    Manifest, Progress, TransferStore,
};
use std::{
    fs,
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let base = std::env::var_os("KHOR_TRANSFER_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).unwrap();
        let path = base.join(format!(
            "khor-transfer-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn store(&self) -> TransferStore {
        TransferStore::open(self.0.join("stages")).unwrap()
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
        binding: binding.to_vec(),
        total_bytes: bytes.len() as u64,
        digest: *blake3::hash(bytes).as_bytes(),
    }
}

#[test]
fn inline_and_chunked_bodies_use_the_same_verification_contract() {
    let scratch = Scratch::new();
    let store = scratch.store();
    for bytes in [
        Vec::new(),
        b"small seen/pins delta".to_vec(),
        vec![7; CHUNK_BYTES * 2 + 19],
    ] {
        let meta = manifest(format!("doc/{}", bytes.len()).as_bytes(), &bytes);
        let id = meta.id().unwrap();
        let mut sink = store.receive(meta.clone()).unwrap();
        assert!(sink.read_verified(0, &mut [0; 1]).is_err() || bytes.is_empty());
        let mut progress = sink.progress().unwrap();
        for (index, chunk) in bytes.chunks(CHUNK_BYTES).enumerate() {
            progress = sink.append((index * CHUNK_BYTES) as u64, chunk).unwrap();
        }
        assert_eq!(
            progress,
            Progress::VerifiedComplete {
                id,
                bytes: bytes.len() as u64
            }
        );
        let mut actual = Vec::new();
        let mut buffer = vec![0; CHUNK_BYTES];
        loop {
            let n = sink
                .read_verified(actual.len() as u64, &mut buffer)
                .unwrap();
            if n == 0 {
                break;
            }
            actual.extend_from_slice(&buffer[..n]);
        }
        assert_eq!(actual, bytes);
        drop(sink);
        let mut reopened = store.receive(meta).unwrap();
        assert_eq!(reopened.progress().unwrap(), progress);
        assert_eq!(reopened.append(bytes.len() as u64, &[]).unwrap(), progress);
    }
}

#[test]
fn every_manifest_field_binds_the_id_without_ambiguous_concatenation() {
    let a = manifest(b"devices\0epoch1\0base\0target", b"delta");
    let id = a.id().unwrap();
    assert_eq!(id.to_string().len(), 64);
    assert_eq!(id, a.clone().id().unwrap());
    for which in 0..5 {
        let mut changed = a.clone();
        match which {
            0 => changed.source[0] ^= 1,
            1 => changed.recipient[0] ^= 1,
            2 => changed.binding.push(0),
            3 => changed.total_bytes += 1,
            _ => changed.digest[0] ^= 1,
        }
        assert_ne!(changed.id().unwrap(), id);
    }
    let mut invalid = a.clone();
    invalid.binding = vec![0; MAX_METADATA_BYTES];
    assert!(matches!(invalid.id(), Err(Error::ResourceLimit(_))));
    invalid = a.clone();
    invalid.total_bytes = MAX_BLOB_BYTES + 1;
    assert!(matches!(invalid.id(), Err(Error::ResourceLimit(_))));
    invalid.binding.clear();
    assert!(matches!(invalid.id(), Err(Error::InvalidManifest(_))));
}

#[test]
fn resume_accepts_exact_duplicates_but_rejects_conflicts_and_future_offsets() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let bytes = vec![3; CHUNK_BYTES + 31];
    let meta = manifest(b"resume", &bytes);
    let mut sink = store.receive(meta.clone()).unwrap();
    assert!(matches!(
        sink.append(5, &[3]),
        Err(Error::OutOfOrder {
            expected: 0,
            received: 5
        })
    ));
    assert!(matches!(sink.append(0, &[3]), Err(Error::InvalidChunk(_))));
    let stored = Progress::Stored {
        next_offset: CHUNK_BYTES as u64,
    };
    assert_eq!(sink.append(0, &bytes[..CHUNK_BYTES]).unwrap(), stored);
    assert_eq!(sink.append(0, &bytes[..CHUNK_BYTES]).unwrap(), stored);
    let conflict = vec![9; CHUNK_BYTES];
    assert!(matches!(
        sink.append(0, &conflict),
        Err(Error::ConflictingDuplicate)
    ));
    drop(sink);
    let mut sink = store.receive(meta).unwrap();
    assert_eq!(sink.progress().unwrap(), stored);
    assert!(matches!(
        sink.append(CHUNK_BYTES as u64, &vec![3; CHUNK_BYTES + 1]),
        Err(Error::InvalidChunk(_))
    ));
    assert!(matches!(
        sink.append(CHUNK_BYTES as u64, &bytes[CHUNK_BYTES..])
            .unwrap(),
        Progress::VerifiedComplete { .. }
    ));
}

#[test]
fn restart_discards_only_unconfirmed_body_and_torn_checkpoint_tails() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let bytes = vec![4; CHUNK_BYTES + 37];
    let meta = manifest(b"tails", &bytes);
    let id = meta.id().unwrap();
    let mut sink = store.receive(meta.clone()).unwrap();
    sink.append(0, &bytes[..CHUNK_BYTES]).unwrap();
    drop(sink);
    let dir = scratch.0.join("stages").join(id.to_string());
    fs::OpenOptions::new()
        .append(true)
        .open(dir.join("body"))
        .unwrap()
        .write_all(b"not yet acknowledged")
        .unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(dir.join("offsets"))
        .unwrap()
        .write_all(b"torn record")
        .unwrap();
    let mut sink = store.receive(meta).unwrap();
    assert_eq!(sink.next_offset(), CHUNK_BYTES as u64);
    assert_eq!(
        fs::metadata(dir.join("body")).unwrap().len(),
        CHUNK_BYTES as u64
    );
    assert!(matches!(
        sink.append(CHUNK_BYTES as u64, &bytes[CHUNK_BYTES..])
            .unwrap(),
        Progress::VerifiedComplete { .. }
    ));
}

#[test]
fn confirmed_disk_corruption_and_wrong_complete_digest_cannot_be_verified() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let bytes = vec![5; CHUNK_BYTES + 9];
    let meta = manifest(b"corruption", &bytes);
    let id = meta.id().unwrap();
    let mut sink = store.receive(meta.clone()).unwrap();
    sink.append(0, &bytes[..CHUNK_BYTES]).unwrap();
    drop(sink);
    let mut body = fs::OpenOptions::new()
        .write(true)
        .open(scratch.0.join("stages").join(id.to_string()).join("body"))
        .unwrap();
    body.seek(SeekFrom::Start(3)).unwrap();
    body.write_all(&[0]).unwrap();
    drop(body);
    assert!(matches!(store.receive(meta), Err(Error::InvalidStage(_))));
    let mut bad = manifest(b"wrong digest", b"abc");
    bad.digest = [0; 32];
    let mut sink = store.receive(bad.clone()).unwrap();
    assert!(matches!(sink.append(0, b"abc"), Err(Error::DigestMismatch)));
    assert!(matches!(
        sink.read_verified(0, &mut [0; 3]),
        Err(Error::DigestMismatch)
    ));
    drop(sink);
    assert!(matches!(
        store.receive(bad).unwrap().progress(),
        Err(Error::DigestMismatch)
    ));
}

#[test]
fn live_owner_limit_and_retained_byte_reservations_are_distinct() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let a = manifest(b"a", b"one");
    let b = manifest(b"b", b"two");
    let c = manifest(b"c", b"three");
    let one = store.receive(a.clone()).unwrap();
    let two = store.receive(b).unwrap();
    assert!(matches!(store.receive(a.clone()), Err(Error::Busy)));
    assert!(matches!(
        store.receive(c.clone()),
        Err(Error::ResourceLimit(_))
    ));
    drop(one);
    let three = store.receive(c).unwrap();
    drop(three);
    drop(two);
    assert_eq!(store.reserved_bytes().unwrap(), 11);
    let reopened = store.receive(a).unwrap();
    drop(reopened);

    let scratch = Scratch::new();
    let store = scratch.store();
    for n in 0..2 {
        let mut large = manifest(&[n], &[]);
        large.total_bytes = MAX_BLOB_BYTES;
        drop(store.receive(large).unwrap());
    }
    assert_eq!(store.reserved_bytes().unwrap(), MAX_BLOB_BYTES * 2);
    assert!(matches!(
        store.receive(manifest(b"over quota", &[1])),
        Err(Error::ResourceLimit(_))
    ));
}

#[test]
fn zero_length_stages_cannot_bypass_metadata_and_inode_budget() {
    let scratch = Scratch::new();
    let store = scratch.store();
    for index in 0..MAX_RETAINED_STAGES {
        drop(store.receive(manifest(&index.to_be_bytes(), &[])).unwrap());
    }
    assert_eq!(store.reserved_bytes().unwrap(), 0);
    assert!(matches!(
        store.receive(manifest(b"one too many", &[])),
        Err(Error::ResourceLimit(_))
    ));
}

#[test]
fn cancel_releases_bad_digest_and_corrupt_body_stages_by_their_exact_manifest() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let mut bad = manifest(b"bad digest cancellation", b"abc");
    bad.digest = [0; 32];
    let mut sink = store.receive(bad.clone()).unwrap();
    assert!(matches!(sink.append(0, b"abc"), Err(Error::DigestMismatch)));
    drop(sink);
    assert_eq!(store.reserved_bytes().unwrap(), 3);
    assert_eq!(store.cancel(&bad).unwrap(), Cancelled::Removed);
    assert_eq!(store.reserved_bytes().unwrap(), 0);
    assert_eq!(store.cancel(&bad).unwrap(), Cancelled::Absent);

    let bytes = vec![7; CHUNK_BYTES + 3];
    let meta = manifest(b"corrupt body cancellation", &bytes);
    let id = meta.id().unwrap();
    let mut sink = store.receive(meta.clone()).unwrap();
    sink.append(0, &bytes[..CHUNK_BYTES]).unwrap();
    drop(sink);
    let dir = scratch.0.join("stages").join(id.to_string());
    fs::write(dir.join("body"), b"damaged body").unwrap();
    fs::write(dir.join("offsets"), b"damaged checkpoint").unwrap();
    assert!(matches!(
        store.receive(meta.clone()),
        Err(Error::InvalidStage(_))
    ));
    assert_eq!(store.cancel(&meta).unwrap(), Cancelled::Removed);
    assert_eq!(store.reserved_bytes().unwrap(), 0);
    assert!(!dir.exists());
}

#[test]
fn cancel_cannot_remove_a_live_owner_or_a_different_manifest() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let meta = manifest(b"current owner", b"bytes");
    let id = meta.id().unwrap();
    let sink = store.receive(meta.clone()).unwrap();
    assert!(matches!(store.cancel(&meta), Err(Error::Busy)));
    drop(sink);
    let mut other = meta.clone();
    other.source[0] ^= 1;
    assert_eq!(store.cancel(&other).unwrap(), Cancelled::Absent);
    assert!(scratch.0.join("stages").join(id.to_string()).exists());
    assert_eq!(store.cancel(&meta).unwrap(), Cancelled::Removed);
    // A late cancellation must not race through a fresh owner's lease.
    let current = store.receive(meta.clone()).unwrap();
    assert!(matches!(store.cancel(&meta), Err(Error::Busy)));
    drop(current);

    let path = scratch
        .0
        .join("stages")
        .join(id.to_string())
        .join("manifest");
    let mut changed = fs::read(&path).unwrap();
    *changed.last_mut().unwrap() ^= 1;
    fs::write(&path, &changed).unwrap();
    assert!(matches!(store.cancel(&meta), Err(Error::InvalidStage(_))));
    assert_eq!(
        fs::read(path).unwrap(),
        changed,
        "mismatched manifest is not owned by this cancel"
    );
}

#[test]
fn cancel_frees_quota_without_touching_another_retained_stage() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let mut one = manifest(b"large one", &[]);
    one.total_bytes = MAX_BLOB_BYTES;
    let mut two = manifest(b"large two", &[]);
    two.total_bytes = MAX_BLOB_BYTES;
    drop(store.receive(one.clone()).unwrap());
    drop(store.receive(two.clone()).unwrap());
    let other = scratch.0.join("stages").join(two.id().unwrap().to_string());
    let other_before = fs::read(other.join("manifest")).unwrap();
    let mut three = manifest(b"large three", &[]);
    three.total_bytes = MAX_BLOB_BYTES;
    assert!(matches!(
        store.receive(three.clone()),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(store.cancel(&one).unwrap(), Cancelled::Removed);
    assert_eq!(store.reserved_bytes().unwrap(), MAX_BLOB_BYTES);
    let new_owner = store.receive(three).unwrap();
    assert_eq!(store.reserved_bytes().unwrap(), MAX_BLOB_BYTES * 2);
    assert_eq!(fs::read(other.join("manifest")).unwrap(), other_before);
    assert!(other.join("body").is_file());
    drop(new_owner);
}

#[cfg(unix)]
#[test]
fn private_files_and_static_symlink_refusal_preserve_the_other_target() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let scratch = Scratch::new();
    let store = scratch.store();
    let meta = manifest(b"private", b"bytes");
    let id = meta.id().unwrap();
    drop(store.receive(meta.clone()).unwrap());
    let root = scratch.0.join("stages");
    let dir = root.join(id.to_string());
    assert_eq!(
        fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for name in ["manifest", "body", "offsets"] {
        assert_eq!(
            fs::metadata(dir.join(name)).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let other = scratch.0.join("other");
    fs::write(&other, b"keep me").unwrap();
    fs::remove_file(dir.join("body")).unwrap();
    symlink(&other, dir.join("body")).unwrap();
    assert!(matches!(store.receive(meta), Err(Error::InvalidStage(_))));
    assert_eq!(fs::read(&other).unwrap(), b"keep me");
    let linked = scratch.0.join("linked-root");
    symlink(&root, &linked).unwrap();
    assert!(matches!(
        TransferStore::open(linked),
        Err(Error::InvalidStage(_))
    ));
}

struct Worker(Option<Child>);
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn kernel_owner_lease_survives_a_second_process_and_releases_after_its_exit() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let log = fs::File::create(scratch.0.join("worker.log")).unwrap();
    let mut worker = Worker(Some(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "transfer_owner_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("KHOR_TRANSFER_WORKER_ROOT", &scratch.0)
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap(),
    ));
    let started = Instant::now();
    while !scratch.0.join("ready").exists() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "owned worker deadline"
        );
        assert!(
            worker.0.as_mut().unwrap().try_wait().unwrap().is_none(),
            "worker must actually retain its lease"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let bytes = vec![8; CHUNK_BYTES + 11];
    let meta = manifest(b"child lease", &bytes);
    assert!(matches!(store.receive(meta.clone()), Err(Error::Busy)));
    assert!(matches!(store.cancel(&meta), Err(Error::Busy)));
    let second = store.receive(manifest(b"parent lease", &[9])).unwrap();
    assert!(matches!(
        store.receive(manifest(b"third lease", &[9])),
        Err(Error::ResourceLimit(_))
    ));
    worker.0.as_mut().unwrap().kill().unwrap();
    worker.0.as_mut().unwrap().wait().unwrap();
    worker.0 = None;
    drop(second);
    let mut resumed = store.receive(meta).unwrap();
    assert_eq!(resumed.next_offset(), CHUNK_BYTES as u64);
    assert!(matches!(
        resumed
            .append(CHUNK_BYTES as u64, &bytes[CHUNK_BYTES..])
            .unwrap(),
        Progress::VerifiedComplete { .. }
    ));
}

#[test]
#[ignore = "actually launched by the kernel lease parent"]
fn transfer_owner_worker() {
    let root =
        PathBuf::from(std::env::var_os("KHOR_TRANSFER_WORKER_ROOT").expect("owned parent root"));
    let store = TransferStore::open(root.join("stages")).unwrap();
    let bytes = vec![8; CHUNK_BYTES + 11];
    let mut sink = store.receive(manifest(b"child lease", &bytes)).unwrap();
    sink.append(0, &bytes[..CHUNK_BYTES]).unwrap();
    fs::write(
        root.join("ready"),
        b"lease retained after a real persisted chunk",
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(20));
    drop(sink);
}
