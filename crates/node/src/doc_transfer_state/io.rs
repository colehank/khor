//! Private bounded metadata publication. File sync + atomic rename supports
//! process restart; this is not a cross-platform power-loss transaction.
use super::Error;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, TryLockError},
};

pub(super) const MAX_IMAGE: usize = 32 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"khor-d01";
const HEADER: usize = 8 + 8 + 8 + 32;
static LOCAL: Mutex<()> = Mutex::new(());
static ROOTS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
#[cfg(test)]
pub(super) static TEST_SERIAL: Mutex<()> = Mutex::new(());

pub(super) struct Gate {
    _local: MutexGuard<'static, ()>,
    _file: File,
}

/// Separate from the metadata gate: settlement holds this before loading a
/// devices document, and may then publish its bounded metadata transaction.
pub(crate) struct RootGate {
    root: PathBuf,
    _file: Option<File>,
}
impl Drop for RootGate {
    fn drop(&mut self) {
        // Release the OS lease before admitting another in-process mutator.
        self._file.take();
        ROOTS
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain(|p| p != &self.root);
    }
}
pub(crate) fn root_gate(root: &Path) -> Result<RootGate, Error> {
    let mut roots = ROOTS.lock().unwrap_or_else(|p| p.into_inner());
    if roots.iter().any(|p| p == root) {
        return Err(Error::busy());
    }
    if roots.len() >= 64 {
        return Err("active trust mutation quota exceeded".into());
    }
    roots.push(root.into());
    drop(roots);
    let mut owned = RootGate {
        root: root.into(),
        _file: None,
    };
    let directory = root.join(".khor/trust-mutation");
    private_dir(&directory)?;
    for (count, entry) in fs::read_dir(&directory).map_err(error)?.enumerate() {
        let entry = entry.map_err(error)?;
        if count >= 2 || !matches!(entry.file_name().to_str(), Some("gate" | "anchor.next")) {
            return Err("unknown trust mutation entry; reconciliation required".into());
        }
        check_file(&entry.path())?;
    }
    let file = open(&directory.join("gate"), true)?;
    if file.metadata().map_err(error)?.len() != 0 {
        return Err("unexpected trust mutation gate bytes".into());
    }
    match file.try_lock() {
        Ok(()) => (),
        Err(fs::TryLockError::WouldBlock) => return Err(Error::busy()),
        Err(fs::TryLockError::Error(why)) => return Err(error(why).into()),
    }
    owned._file = Some(file);
    Ok(owned)
}

/// The public anchor is old or new after process interruption, never a
/// truncated identity. The private next file is recovered only for this exact
/// requested value; unknown bytes are preserved for explicit reconciliation.
pub(crate) fn publish_anchor(root: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() != 65 || bytes.last() != Some(&b'\n') {
        return Err("invalid public anchor publication".into());
    }
    let directory = root.join(".khor/trust-mutation");
    private_dir(&directory)?;
    let next = directory.join("anchor.next");
    let prefix = read_optional(&next)?;
    if let Some(prefix) = &prefix {
        if prefix.len() > bytes.len() || prefix.as_slice() != &bytes[..prefix.len()] {
            return Err("unknown anchor publication; reconciliation required".into());
        }
    }
    let mut file = if prefix.is_some() {
        open(&next, false)?
    } else {
        options().create_new(true).open(&next).map_err(error)?
    };
    file.write_all(bytes).map_err(error)?;
    file.sync_all().map_err(error)?;
    drop(file);
    check_file(&next)?;
    let target = root.join(".khor/root.pub");
    match fs::symlink_metadata(&target) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Err("unsafe public anchor target".into());
        }
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(error(e)),
    }
    fs::rename(next, target).map_err(error)
}

pub(super) fn has_publication(root: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(root.join("registry.next")) {
        Ok(_) => {
            check_file(&root.join("registry.next"))?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(error(e)),
    }
}

/// Atomic registry reads do not acquire/mutate a transfer owner. Before any
/// trust bootstrap, a critical intent must be recovered under both leases.
pub(super) fn peek(root: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(root) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(error(e)),
        Ok(metadata) => {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err("unsafe transfer metadata directory".into());
            }
            private(&metadata)?;
        }
    }
    for (count, entry) in fs::read_dir(root).map_err(error)?.enumerate() {
        let entry = entry.map_err(error)?;
        if count >= 3
            || !matches!(
                entry.file_name().to_str(),
                Some("gate" | "registry" | "registry.next")
            )
        {
            return Err("unrecognized transfer metadata file; reconciliation required".into());
        }
        check_file(&entry.path())?;
    }
    read_optional(&root.join("registry"))?
        .map(|bytes| decode(&bytes).map(|(_, body)| body.to_vec()))
        .transpose()
}

pub(super) fn gate(root: &Path) -> Result<Gate, Error> {
    let local = match LOCAL.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::Poisoned(error)) => error.into_inner(),
        Err(TryLockError::WouldBlock) => return Err(Error::busy()),
    };
    private_dir(root)?;
    let file = open(root.join("gate").as_path(), true)?;
    if file.metadata().map_err(error)?.len() != 0 {
        return Err("unexpected transfer metadata gate bytes".to_owned().into());
    }
    match file.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => return Err(Error::busy()),
        Err(fs::TryLockError::Error(why)) => return Err(error(why).into()),
    }
    Ok(Gate {
        _local: local,
        _file: file,
    })
}

pub(super) struct Journal {
    root: PathBuf,
    generation: u64,
}

impl Journal {
    pub(super) fn load(
        root: &Path,
        validate: impl Fn(&[u8]) -> Result<(), String>,
    ) -> Result<(Self, Option<Vec<u8>>), String> {
        // Only three recognized controls; unknown files are never GC targets.
        for (count, entry) in fs::read_dir(root).map_err(error)?.enumerate() {
            if count >= 3 {
                return Err("transfer metadata has unexpected entries".into());
            }
            let entry = entry.map_err(error)?;
            if !matches!(
                entry.file_name().to_str(),
                Some("gate" | "registry" | "registry.next")
            ) {
                return Err("unrecognized transfer metadata file; reconciliation required".into());
            }
            check_file(&entry.path())?;
        }
        let current = read_optional(&root.join("registry"))?;
        let (mut generation, mut body) = match current {
            None => (0, None),
            Some(bytes) => {
                let (generation, body) = decode(&bytes)?;
                validate(body)?;
                (generation, Some(body.to_vec()))
            }
        };
        let next = root.join("registry.next");
        if let Some(bytes) = read_optional(&next)? {
            let expected = generation
                .checked_add(1)
                .ok_or("metadata generation overflow")?;
            let mut prefix = Vec::from(MAGIC.as_slice());
            prefix.extend_from_slice(&expected.to_be_bytes());
            if bytes[..bytes.len().min(prefix.len())] != prefix[..bytes.len().min(prefix.len())] {
                return Err(
                    "unrecognized metadata publication; preserve for reconciliation".into(),
                );
            }
            let partial = if bytes.len() < 24 {
                if bytes.len() > 16 {
                    let mut partial_length = [0u8; 8];
                    partial_length[..bytes.len() - 16].copy_from_slice(&bytes[16..]);
                    if u64::from_be_bytes(partial_length) > MAX_IMAGE as u64 {
                        return Err(
                            "invalid partial metadata length; preserve for reconciliation".into(),
                        );
                    }
                }
                true
            } else {
                let length = u64::from_be_bytes(bytes[16..24].try_into().unwrap());
                if length > MAX_IMAGE as u64 {
                    return Err("metadata publication exceeds budget".into());
                }
                bytes.len() < HEADER + length as usize
            };
            if partial {
                // This exact, private, unpublished next-generation control has
                // never returned success. Keep the previously committed image.
                // Bad complete checksums/unknown generations are NOT removed.
                check_file(&next)?;
                fs::remove_file(&next).map_err(error)?;
            } else {
                let (next_generation, next_body) = decode(&bytes)?;
                if next_generation != expected {
                    return Err("unexpected metadata generation".into());
                }
                validate(next_body)?;
                open(&next, false)?.sync_all().map_err(error)?;
                fs::rename(&next, root.join("registry")).map_err(error)?;
                generation = next_generation;
                body = Some(next_body.to_vec());
            }
        }
        Ok((
            Self {
                root: root.to_path_buf(),
                generation,
            },
            body,
        ))
    }

    pub(super) fn save(&mut self, body: &[u8]) -> Result<(), String> {
        if body.len() > MAX_IMAGE {
            return Err("document transfer metadata exceeds 32 MiB".into());
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or("metadata generation overflow")?;
        let next = self.root.join("registry.next");
        let mut options = options();
        let mut file = options.create_new(true).open(&next).map_err(error)?;
        let mut header = Vec::from(MAGIC.as_slice());
        header.extend_from_slice(&generation.to_be_bytes());
        header.extend_from_slice(&(body.len() as u64).to_be_bytes());
        let mut checksum = blake3::Hasher::new();
        checksum.update(&header);
        checksum.update(body);
        header.extend_from_slice(checksum.finalize().as_bytes());
        file.write_all(&header).map_err(error)?;
        file.write_all(body).map_err(error)?;
        file.sync_all().map_err(error)?;
        drop(file);
        check_file(&next)?;
        fs::rename(&next, self.root.join("registry")).map_err(error)?;
        self.generation = generation;
        Ok(())
    }
}

fn decode(bytes: &[u8]) -> Result<(u64, &[u8]), String> {
    if bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return Err("invalid transfer metadata header".into());
    }
    let generation = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
    let length = u64::from_be_bytes(bytes[16..24].try_into().unwrap());
    if generation == 0 || length > MAX_IMAGE as u64 || bytes.len() != HEADER + length as usize {
        return Err("invalid transfer metadata length or generation".into());
    }
    let body = &bytes[HEADER..];
    let mut checksum = blake3::Hasher::new();
    checksum.update(&bytes[..24]);
    checksum.update(body);
    if checksum.finalize().as_bytes() != &bytes[24..56] {
        return Err("transfer metadata checksum mismatch; preserve for reconciliation".into());
    }
    Ok((generation, body))
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(error(why)),
        Ok(_) => {}
    }
    let file = open(path, false)?;
    if file.metadata().map_err(error)?.len() > (MAX_IMAGE + HEADER) as u64 {
        return Err("transfer metadata file exceeds budget".into());
    }
    let mut bytes = Vec::new();
    file.take((MAX_IMAGE + HEADER + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() > MAX_IMAGE + HEADER {
        return Err("transfer metadata file grew past budget".into());
    }
    Ok(Some(bytes))
}

fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    options
}

fn open(path: &Path, create: bool) -> Result<File, String> {
    let file = options().create(create).open(path).map_err(error)?;
    let metadata = file.metadata().map_err(error)?;
    private(&metadata)?;
    if !metadata.is_file() {
        return Err("transfer metadata control is not a file".into());
    }
    check_file(path)?;
    Ok(file)
}

fn check_file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(error)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("unsafe transfer metadata file".into());
    }
    private(&metadata)
}

fn private_dir(path: &Path) -> Result<(), String> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => {}
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(why) => return Err(error(why)),
    }
    let metadata = fs::symlink_metadata(path).map_err(error)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("unsafe transfer metadata directory".into());
    }
    private(&metadata)
}

fn private(metadata: &fs::Metadata) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
            || (metadata.is_file() && metadata.nlink() != 1)
        {
            return Err("transfer metadata must be owner-only without hard links".into());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("transfer metadata reparse point".into());
        }
    }
    Ok(())
}
fn error(why: std::io::Error) -> String {
    format!("document transfer metadata: {why}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Outside(PathBuf);
    impl Drop for Outside {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "khor-doc-journal-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn image(generation: u64, body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::from(MAGIC.as_slice());
        bytes.extend_from_slice(&generation.to_be_bytes());
        bytes.extend_from_slice(&(body.len() as u64).to_be_bytes());
        let mut hash = blake3::Hasher::new();
        hash.update(&bytes);
        hash.update(body);
        bytes.extend_from_slice(hash.finalize().as_bytes());
        bytes.extend_from_slice(body);
        bytes
    }
    fn next(root: &Path, bytes: &[u8]) {
        let mut f = options()
            .create_new(true)
            .open(root.join("registry.next"))
            .unwrap();
        f.write_all(bytes).unwrap();
        f.sync_all().unwrap();
    }
    #[test]
    fn a_complete_unpublished_image_recovers_and_every_partial_prefix_keeps_the_committed_image() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        for length in 0..HEADER + 7 {
            let fixture = Fixture::new();
            let _gate = gate(&fixture.0).unwrap();
            let (mut journal, _) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
            journal.save(b"previous").unwrap();
            let candidate = image(2, b"current");
            next(&fixture.0, &candidate[..length]);
            let (_, body) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
            assert_eq!(body.as_deref(), Some(b"previous".as_slice()));
            assert!(!fixture.0.join("registry.next").exists());
        }
        let fixture = Fixture::new();
        let _gate = gate(&fixture.0).unwrap();
        let (mut journal, _) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
        journal.save(b"previous").unwrap();
        next(&fixture.0, &image(2, b"current"));
        let (_, body) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
        assert_eq!(body.as_deref(), Some(b"current".as_slice()));
    }
    #[test]
    fn complete_corruption_unknown_files_and_semantically_bad_publications_are_preserved() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        for case in 0..3 {
            let fixture = Fixture::new();
            let _gate = gate(&fixture.0).unwrap();
            let (mut journal, _) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
            journal.save(b"previous").unwrap();
            let previous = fs::read(fixture.0.join("registry")).unwrap();
            let mut bytes = image(2, b"current");
            match case {
                0 => bytes[24] ^= 1,
                1 => {
                    fs::write(fixture.0.join("unknown"), b"keep").unwrap();
                }
                _ => {}
            }
            next(&fixture.0, &bytes);
            assert!(
                Journal::load(&fixture.0, |body| if case == 2 && body == b"current" {
                    Err("semantic rejection".into())
                } else {
                    Ok(())
                })
                .is_err()
            );
            assert_eq!(fs::read(fixture.0.join("registry.next")).unwrap(), bytes);
            assert_eq!(fs::read(fixture.0.join("registry")).unwrap(), previous);
        }
    }
    #[cfg(unix)]
    #[test]
    fn metadata_links_never_touch_their_targets() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        use std::os::unix::fs::symlink;
        for hard in [false, true] {
            let fixture = Fixture::new();
            let _gate = gate(&fixture.0).unwrap();
            let target = fixture.0.with_extension("outside");
            let mut file = options().create_new(true).open(&target).unwrap();
            let _outside = Outside(target.clone());
            file.write_all(b"preserve outside target").unwrap();
            drop(file);
            let linked = fixture.0.join("registry.next");
            if hard {
                fs::hard_link(&target, &linked).unwrap();
            } else {
                symlink(&target, &linked).unwrap();
            }
            assert!(Journal::load(&fixture.0, |_| Ok(())).is_err());
            assert_eq!(fs::read(&target).unwrap(), b"preserve outside target");
            assert!(fs::symlink_metadata(&linked).is_ok());
            fs::remove_file(&linked).unwrap();
            fs::remove_file(&target).unwrap();
        }
    }
    #[test]
    fn ordinary_open_waits_for_a_same_root_control_lease_then_reuses_its_anchor() {
        use std::sync::mpsc;
        use std::time::Duration;

        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let fixture = Fixture::new();
        let first = crate::Node::open_hosting(fixture.0.clone(), "ordinary-open", false).unwrap();
        assert!(first.root_born());
        let anchor = fs::read(fixture.0.join(".khor/root.pub")).unwrap();
        assert!(!anchor.is_empty());
        drop(first);

        let control = root_gate(&fixture.0).unwrap();
        let root = fixture.0.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (proceed_tx, proceed_rx) = mpsc::channel();
        let (attempting_tx, attempting_rx) = mpsc::channel();
        let (returned_tx, returned_rx) = mpsc::channel();
        let handshake = Duration::from_secs(2);
        let worker = std::thread::spawn(move || {
            let _ = started_tx.send(());
            if proceed_rx.recv_timeout(handshake).is_err() {
                return Err("ordinary-open worker did not receive its start handshake".to_owned());
            }
            let _ = attempting_tx.send(());
            let opened =
                crate::Node::open_hosting(root.clone(), "ordinary-open", false).and_then(|node| {
                    let anchor =
                        fs::read(root.join(".khor/root.pub")).map_err(|error| error.to_string())?;
                    Ok((node.root_born(), anchor))
                });
            let _ = returned_tx.send(opened.clone());
            opened
        });

        let started = started_rx.recv_timeout(handshake).is_ok();
        let proceeded = started && proceed_tx.send(()).is_ok();
        drop(proceed_tx);
        let attempting = proceeded && attempting_rx.recv_timeout(handshake).is_ok();
        let early = if attempting {
            Some(returned_rx.recv_timeout(Duration::from_millis(100)))
        } else {
            None
        };
        // Always release the actual owner and join before any assertion can unwind.
        drop(control);
        let joined = worker.join();
        match &early {
            Some(Ok(Err(error))) => eprintln!("Q039_EARLY_OPEN_ERROR: {error}"),
            Some(Ok(Ok(_))) => eprintln!("Q039_EARLY_OPEN_RETURNED_OK"),
            Some(Err(mpsc::RecvTimeoutError::Timeout)) => {
                eprintln!("Q039_HELD_WINDOW_TIMEOUT_AFTER_ATTEMPT_HANDSHAKE");
            }
            Some(Err(mpsc::RecvTimeoutError::Disconnected)) => {
                eprintln!("Q039_OPEN_WORKER_DISCONNECTED");
            }
            None => eprintln!("Q039_OPEN_ATTEMPT_HANDSHAKE_MISSING"),
        }
        assert!(
            started && proceeded && attempting,
            "ordinary-open handshake did not complete"
        );
        assert!(
            matches!(early, Some(Err(mpsc::RecvTimeoutError::Timeout))),
            "ordinary Node open returned while the same-root control lease was held: {early:?}"
        );
        let (born, reopened_anchor) = joined
            .expect("ordinary-open worker panicked")
            .expect("ordinary Node open failed after the control lease was released");
        assert!(!born, "ordinary reopening must not create another root");
        assert_eq!(
            reopened_anchor, anchor,
            "ordinary reopening changed the trust anchor"
        );
    }
}
