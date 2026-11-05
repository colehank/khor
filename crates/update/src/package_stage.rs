use crate::{Offer, SignedRelease, TrustRoot};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const RECORD_LIMIT: u64 = 384 * 1024;
const FILES: &[&str] = &["owner.json", "lease", "asset", "khor", "khor.exe"];

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    schema: u8,
    nonce: String,
    release: SignedRelease,
}

pub struct PackageStage {
    // Close the lease before TempDir removes its files, including on Windows.
    _lease: File,
    directory: tempfile::TempDir,
}

impl PackageStage {
    pub fn new(offer: &Offer, root: &Path) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let metadata = fs::symlink_metadata(root).map_err(|e| e.to_string())?;
        if !metadata.is_dir() || metadata.is_symlink() {
            return Err("download root is not a local directory".into());
        }
        let directory = tempfile::Builder::new()
            .prefix("download-")
            .tempdir_in(root)
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(directory.path().join("lease"))
            .map_err(|e| e.to_string())?;
        lease.lock().map_err(|e| e.to_string())?;
        let owner = Owner {
            schema: 1,
            nonce: directory
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            release: offer.signed().clone(),
        };
        let bytes = serde_json::to_vec(&owner).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > RECORD_LIMIT {
            return Err("download ownership record exceeds its limit".into());
        }
        let mut record = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.path().join("owner.json"))
            .map_err(|e| e.to_string())?;
        record.write_all(&bytes).map_err(|e| e.to_string())?;
        record.sync_all().map_err(|e| e.to_string())?;
        sync(directory.path())?;
        Ok(Self {
            _lease: lease,
            directory,
        })
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn archive(&self) -> PathBuf {
        self.path().join("asset")
    }
}

pub fn collect_downloads(root: &Path, trust: &TrustRoot) -> Result<u64, String> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() && !metadata.is_symlink() => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        _ => return Err("download root is not a local directory".into()),
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = 0u64;
    for entry in entries.take(256) {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_name().to_string_lossy().starts_with("download-")
            || !entry.file_type().map_err(|e| e.to_string())?.is_dir()
        {
            continue;
        }
        if let Ok(reclaimed) = collect_one(&entry.path(), trust) {
            bytes = bytes.saturating_add(reclaimed);
        }
    }
    Ok(bytes)
}

fn collect_one(path: &Path, trust: &TrustRoot) -> Result<u64, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err("download stage is not an owned directory".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o077 != 0 {
            return Err("download stage is not private".into());
        }
    }
    let mut bytes = 0u64;
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !FILES.iter().any(|name| entry.file_name() == *name) {
            return Err("download stage contains an unknown file".into());
        }
        bytes = bytes.saturating_add(regular_file(&entry.path())?.len());
    }
    regular_file(&path.join("owner.json"))?;
    let mut document = Vec::new();
    File::open(path.join("owner.json"))
        .map_err(|e| e.to_string())?
        .take(RECORD_LIMIT + 1)
        .read_to_end(&mut document)
        .map_err(|e| e.to_string())?;
    if document.len() as u64 > RECORD_LIMIT {
        return Err("download ownership record exceeds its limit".into());
    }
    let owner: Owner = serde_json::from_slice(&document).map_err(|e| e.to_string())?;
    if owner.schema != 1 || path.file_name().and_then(|n| n.to_str()) != Some(owner.nonce.as_str())
    {
        return Err("download stage has another identity".into());
    }
    trust.restore(&owner.release)?;
    regular_file(&path.join("lease"))?;
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path.join("lease"))
        .map_err(|e| e.to_string())?;
    lease
        .try_lock()
        .map_err(|_| "download stage is still occupied")?;
    for name in FILES
        .iter()
        .filter(|name| **name != "lease" && **name != "owner.json")
    {
        match fs::remove_file(path.join(name)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    fs::remove_file(path.join("owner.json")).map_err(|e| e.to_string())?;
    drop(lease);
    fs::remove_file(path.join("lease")).map_err(|e| e.to_string())?;
    fs::remove_dir(path).map_err(|e| e.to_string())?;
    Ok(bytes)
}

pub(crate) fn regular_file(path: &Path) -> Result<fs::Metadata, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.is_symlink() {
        return Err("update cache contains an unexpected link or directory".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err("update cache contains a multiply linked file".into());
        }
    }
    Ok(metadata)
}

pub(crate) fn sync(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_unknown_and_linked_downloads_survive_while_an_interrupted_owned_stage_is_reclaimed() {
        let root = tempfile::tempdir().unwrap();
        let (trust, offer) = crate::trust::fixture_offer("http://127.0.0.1/asset".into());
        let stage = PackageStage::new(&offer, root.path()).unwrap();
        fs::write(stage.archive(), b"interrupted bytes").unwrap();
        let path = stage.path().to_path_buf();
        assert_eq!(collect_downloads(root.path(), &trust).unwrap(), 0);
        assert!(path.join("asset").exists());
        let PackageStage { _lease, directory } = stage;
        let path = directory.keep();
        drop(_lease);
        let unknown = root.path().join("download-unowned");
        fs::create_dir(&unknown).unwrap();
        fs::write(unknown.join("asset"), b"unrelated").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&unknown, root.path().join("download-linked")).unwrap();
        assert!(
            collect_downloads(root.path(), &trust).unwrap() >= 17,
            "interrupted stage was refused: {:?}",
            collect_one(&path, &trust)
        );
        assert!(!path.exists());
        assert_eq!(fs::read(unknown.join("asset")).unwrap(), b"unrelated");
    }

    #[test]
    fn invalid_identity_and_unknown_files_are_not_adopted_for_collection() {
        let root = tempfile::tempdir().unwrap();
        let (trust, offer) = crate::trust::fixture_offer("http://127.0.0.1/asset".into());
        for invalid in [false, true] {
            let stage = PackageStage::new(&offer, root.path()).unwrap();
            if invalid {
                fs::write(stage.path().join("owner.json"), b"not authenticated").unwrap();
            } else {
                fs::write(stage.path().join("keep-me"), b"unrelated").unwrap();
            }
            let PackageStage { _lease, directory } = stage;
            let path = directory.keep();
            drop(_lease);
            assert_eq!(collect_downloads(root.path(), &trust).unwrap(), 0);
            assert!(path.exists());
        }
    }
}
