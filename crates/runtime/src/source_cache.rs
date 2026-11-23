//! Reusing a published runtime image for a source program that has not
//! changed.
//!
//! # The cost this removes
//!
//! Opening a session runs `host_at(source)`: it inspects the source program
//! (which starts a small probe process), then publishes a managed image —
//! hashing the whole binary (130 MB here) and copying it. On this machine
//! that is ~600 ms of every session creation, and it repeats on **every**
//! launch because `Store::image_for_program` only recognises a program that
//! is *already* a managed image, which a build-tree executable never is.
//!
//! # What a hit claims, and what it does not
//!
//! A hit claims only this: **the source file is the same file object that
//! was published from**. It is checked against the operating system's own
//! identity for that file — canonical path, size, last-write time, and on
//! Windows the volume serial plus file index, on unix the device and inode.
//! It does **not** re-hash the bytes, and it does not claim the bytes were
//! re-verified: the image it returns was published from a full hash and
//! manifest check the first time, and the store still holds it.
//!
//! The trust analysis is worth stating, because "file metadata instead of a
//! trust proof" is exactly what an independent review warned against:
//!
//! - **Writing the file in place** keeps the file index but changes size
//!   and/or last-write time, so the identity changes and the cache misses.
//! - **Replacing the file** (an installer, a rebuild) gives a new file
//!   index/inode, so the identity changes and the cache misses.
//! - An attacker who can rewrite a file *and restore both its file index
//!   and its timestamps* needs raw-volume access; the outcome of that is
//!   the **old verified image** being run, never their bytes — the cached
//!   image is a copy that was verified before it was stored.
//!
//! So the failure mode of a hit is staleness (running the previous build
//! after an upgrade that somehow kept every identity field identical), not
//! execution of unverified code. That is why the cache is a **hint** whose
//! entries are cheap to discard: any mismatch, any missing image, any
//! unreadable file falls back to the ordinary inspect-and-publish path.
//!
//! The caller additionally compares the image's contract and version with
//! the running release before using it, so an upgrade is picked up even if
//! a source file's identity were unchanged.

use crate::{Contract, Image, Role, Store};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};

const VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Entry {
    size: u64,
    mtime_ns: u128,
    #[serde(default)]
    file_id: Option<u64>,
    #[serde(default)]
    volume: Option<u32>,
    image: String,
    role: Role,
    contract: Contract,
    version: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct CacheFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    entries: BTreeMap<String, Entry>,
}

/// The identity of a source file, as the operating system states it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Identity {
    key: String,
    size: u64,
    mtime_ns: u128,
    file_id: Option<u64>,
    volume: Option<u32>,
}

impl Identity {
    fn of(source: &Path) -> io::Result<Option<Self>> {
        let canonical = std::fs::canonicalize(source)?;
        let metadata = std::fs::metadata(&canonical)?;
        if !metadata.is_file() {
            return Ok(None);
        }
        let mtime_ns = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|since| since.as_nanos())
            .unwrap_or(0);
        let (file_id, volume) = file_identity(&canonical, &metadata);
        Ok(Some(Self {
            key: canonical.to_string_lossy().into_owned(),
            size: metadata.len(),
            mtime_ns,
            file_id,
            volume,
        }))
    }

    fn matches(&self, entry: &Entry) -> bool {
        self.size == entry.size
            && self.mtime_ns == entry.mtime_ns
            && self.file_id == entry.file_id
            && self.volume == entry.volume
    }
}

#[cfg(windows)]
fn file_identity(path: &Path, _metadata: &std::fs::Metadata) -> (Option<u64>, Option<u32>) {
    // **The volume serial and the file index are the operating system's own
    // identity for a file object**: replacing the file gives a new index,
    // which is what makes a rebuild/upgrade miss. std exposes them only
    // behind an unstable feature, so the handle is asked directly — the
    // same call this crate already makes elsewhere (`lease.rs`).
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };
    let Ok(file) = std::fs::File::open(path) else {
        return (None, None);
    };
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return (None, None);
    }
    let index = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    (Some(index), Some(info.dwVolumeSerialNumber))
}

#[cfg(unix)]
fn file_identity(_path: &Path, metadata: &std::fs::Metadata) -> (Option<u64>, Option<u32>) {
    use std::os::unix::fs::MetadataExt;
    (Some(metadata.ino()), Some(metadata.dev() as u32))
}

#[cfg(not(any(windows, unix)))]
fn file_identity(_path: &Path, _metadata: &std::fs::Metadata) -> (Option<u64>, Option<u32>) {
    (None, None)
}

/// The per-user cache of "this file was published as this image".
pub struct SourceCache {
    path: PathBuf,
}

impl SourceCache {
    /// The cache beside the runtime store (`<local dir>/sources.json`).
    pub fn local() -> io::Result<Self> {
        Ok(Self {
            path: crate::local_directory()?.join("sources.json"),
        })
    }

    /// A cache at a stated path, so the behaviour is testable.
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// The image published from this file, when the file has not changed
    /// and the image is still in the store.
    ///
    /// Every failure is a miss: an unreadable file, a corrupt cache, a
    /// missing image. The caller must be able to do the ordinary work.
    pub fn lookup(&self, source: &Path) -> io::Result<Option<Image>> {
        let Some(identity) = Identity::of(source)? else {
            return Ok(None);
        };
        let file = self.read();
        let Some(entry) = file.entries.get(&identity.key) else {
            return Ok(None);
        };
        if !identity.matches(entry) {
            return Ok(None);
        }
        match Store::local()?.hold(&entry.image) {
            Ok(image) => Ok(Some(image)),
            Err(_) => {
                // The image was collected or the store moved on: drop the
                // entry so the next lookup does not pay for it again.
                let _ = self.forget(&identity.key);
                Ok(None)
            }
        }
    }

    /// Records that `image` was published from `source`, as it is now.
    pub fn remember(&self, source: &Path, image: &Image) -> io::Result<()> {
        let Some(identity) = Identity::of(source)? else {
            return Ok(());
        };
        let mut file = self.read();
        file.version = VERSION;
        file.entries.insert(
            identity.key,
            Entry {
                size: identity.size,
                mtime_ns: identity.mtime_ns,
                file_id: identity.file_id,
                volume: identity.volume,
                image: image.id()?,
                role: image.role(),
                contract: image.contract().clone(),
                version: image.version().to_owned(),
            },
        );
        self.write(&file)
    }

    /// Forgets one source path.
    pub fn forget(&self, key: &str) -> io::Result<()> {
        let mut file = self.read();
        if file.entries.remove(key).is_some() {
            self.write(&file)?;
        }
        Ok(())
    }

    /// The cache as written by [`Self::remember`] — the contract and
    /// version recorded with an entry, for callers that want to compare
    /// before loading the image.
    pub fn stated(&self, source: &Path) -> io::Result<Option<(Contract, String)>> {
        let Some(identity) = Identity::of(source)? else {
            return Ok(None);
        };
        let file = self.read();
        Ok(file
            .entries
            .get(&identity.key)
            .filter(|entry| identity.matches(entry))
            .map(|entry| (entry.contract.clone(), entry.version.clone())))
    }

    fn read(&self) -> CacheFile {
        let Ok(bytes) = std::fs::read(&self.path) else {
            return CacheFile::default();
        };
        if bytes.len() > 1024 * 1024 {
            return CacheFile::default();
        }
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    fn write(&self, file: &CacheFile) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = self
            .path
            .with_extension(format!("{}.tmp", std::process::id()));
        let bytes = serde_json::to_vec(file).map_err(io::Error::other)?;
        std::fs::write(&temporary, bytes)?;
        // Rename over the old file: a reader sees either cache, never half
        // of one. On Windows a rename onto an existing file fails only when
        // the target is open, which a read of this size does for
        // microseconds — and that failure is a miss, not an error.
        match std::fs::rename(&temporary, &self.path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = std::fs::remove_file(&temporary);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("khor-source-cache-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn scratch_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn entry(identity: &Identity) -> Entry {
        Entry {
            size: identity.size,
            mtime_ns: identity.mtime_ns,
            file_id: identity.file_id,
            volume: identity.volume,
            image: "generation".into(),
            role: Role::Cli,
            contract: Contract {
                terminal: 1,
                gui: "g".into(),
                node: "n".into(),
                store: 1,
                journal: 1,
                agent: 1,
            },
            version: "0.2.0".into(),
        }
    }

    /// A rebuild must miss when size or the last-write time changes.
    #[test]
    fn a_changed_file_does_not_match_its_entry() {
        let dir = scratch("changed");
        let source = scratch_file(&dir, "khor.exe", b"one");
        let old_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&source)
            .unwrap();
        file.set_modified(old_time).unwrap();
        drop(file);
        let identity = Identity::of(&source).unwrap().unwrap();
        let mut recorded = entry(&identity);
        assert!(identity.matches(&recorded));
        std::fs::write(&source, b"second").unwrap();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&source)
            .unwrap();
        file.set_modified(old_time + std::time::Duration::from_secs(2))
            .unwrap();
        drop(file);
        let changed = Identity::of(&source).unwrap().unwrap();
        assert!(!changed.matches(&recorded));
        // Check each field independently.
        assert_ne!(changed.size, recorded.size);
        recorded.size = changed.size;
        assert!(!changed.matches(&recorded));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The cache file round-trips, and a corrupt one is a miss rather than
    /// an error: this is a hint, not a store.
    #[test]
    fn the_cache_round_trips_and_survives_corruption() {
        let dir = scratch("roundtrip");
        let cache = SourceCache::at(dir.join("sources.json"));
        let source = scratch_file(&dir, "khor.exe", b"program");
        let identity = Identity::of(&source).unwrap().unwrap();
        let mut file = CacheFile::default();
        file.version = VERSION;
        file.entries.insert(identity.key.clone(), entry(&identity));
        cache.write(&file).unwrap();
        let read = cache.read();
        assert_eq!(read.version, VERSION);
        assert_eq!(read.entries.len(), 1);
        assert!(read.entries.contains_key(&identity.key));

        std::fs::write(dir.join("sources.json"), b"{not json").unwrap();
        assert!(cache.read().entries.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `stated` answers only for the identity it was recorded against.
    #[test]
    fn a_stated_entry_answers_only_when_the_file_still_matches() {
        let dir = scratch("stated");
        let cache = SourceCache::at(dir.join("sources.json"));
        let source = scratch_file(&dir, "khor.exe", b"program");
        let identity = Identity::of(&source).unwrap().unwrap();
        let mut file = CacheFile::default();
        file.entries.insert(identity.key.clone(), entry(&identity));
        cache.write(&file).unwrap();
        assert!(cache.stated(&source).unwrap().is_some());
        std::fs::write(&source, b"a longer program").unwrap();
        assert!(cache.stated(&source).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
