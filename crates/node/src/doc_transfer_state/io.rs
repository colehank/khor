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
#[cfg(test)]
pub(super) static TEST_SERIAL: Mutex<()> = Mutex::new(());

pub(super) struct Gate {
    _local: MutexGuard<'static, ()>,
    _file: File,
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
}
