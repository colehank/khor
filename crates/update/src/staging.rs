use crate::{
    Channel, Component, Offer, Platform, ReleaseIdentity, SignedRelease, TrustRoot, VerifiedRelease,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const RECORD_LIMIT: u64 = 384 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u8,
    release: SignedRelease,
}

/// The archive is replaceable cache. Only its authenticated manifest can restore an offer.
pub struct Staging {
    root: PathBuf,
    _lock: File,
}

impl Staging {
    pub fn open(root: PathBuf) -> Result<Self, String> {
        if !root.exists() {
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(&root).map_err(|e| e.to_string())?;
        }
        let metadata = fs::symlink_metadata(&root).map_err(|e| e.to_string())?;
        if !metadata.is_dir() || metadata.is_symlink() {
            return Err("update staging is not an owned directory".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            // RuntimeStore has already admitted the node-local parent; this child must stay private.
            if metadata.mode() & 0o077 != 0 {
                return Err("update staging is not private".into());
            }
        }
        let lock_path = root.join("owner.lock");
        reject_symlink(&lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|_| "another application instance is preparing an update".to_owned())?;
        Ok(Self { root, _lock: lock })
    }

    pub fn archive(&self, offer: &Offer) -> PathBuf {
        self.root
            .join(format!("{}.package", offer.artifact().sha256.as_str()))
    }

    pub fn remember(&self, release: &VerifiedRelease) -> Result<(), String> {
        let record = Record {
            schema: 1,
            release: release.signed().clone(),
        };
        let bytes = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > RECORD_LIMIT {
            return Err("update record exceeds its limit".into());
        }
        for artifact in &release.manifest().artifacts {
            self.write_record(
                &format!("{}.package.json", artifact.sha256.as_str()),
                &bytes,
            )?;
        }
        self.write_record("selected.json", &bytes)?;
        sync_directory(&self.root)
    }

    fn write_record(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let mut file = tempfile::NamedTempFile::new_in(&self.root).map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(self.root.join(name))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn collect(&self, trust: &TrustRoot) -> Result<u64, String> {
        let selected = match fs::symlink_metadata(self.root.join("selected.json")) {
            Ok(_) => Some(read_release(&self.root.join("selected.json"), trust)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        let mut bytes = crate::collect_downloads(&self.root, trust)?;
        for entry in fs::read_dir(&self.root)
            .map_err(|e| e.to_string())?
            .take(256)
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let Some(digest) = name
                .to_str()
                .and_then(|name| name.strip_suffix(".package.json"))
            else {
                continue;
            };
            if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            if selected.as_ref().is_some_and(|release| {
                release
                    .manifest()
                    .artifacts
                    .iter()
                    .any(|asset| asset.sha256.as_str() == digest)
            }) {
                continue;
            }
            let Ok(release) = read_release(&entry.path(), trust) else {
                continue;
            };
            if !release
                .manifest()
                .artifacts
                .iter()
                .any(|asset| asset.sha256.as_str() == digest)
            {
                continue;
            }
            let archive = self.root.join(format!("{digest}.package"));
            match fs::symlink_metadata(&archive) {
                Ok(_) => {
                    let Ok(metadata) = crate::package_stage::regular_file(&archive) else {
                        continue;
                    };
                    fs::remove_file(&archive).map_err(|e| e.to_string())?;
                    bytes = bytes.saturating_add(metadata.len());
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => continue,
            }
            fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
        }
        sync_directory(&self.root)?;
        Ok(bytes)
    }

    pub fn restore(
        &self,
        trust: &TrustRoot,
        current: &ReleaseIdentity,
        component: Component,
        platform: Platform,
        channel: Channel,
    ) -> Result<Option<(VerifiedRelease, Offer)>, String> {
        let path = self.root.join("selected.json");
        match fs::symlink_metadata(&path) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        }
        let release = read_release(&path, trust)?;
        let Some(offer) = release.select(component, platform, current, channel, None)? else {
            return Ok(None);
        };
        Ok(Some((release, offer)))
    }

    pub fn is_ready(&self, trust: &TrustRoot, offer: &Offer) -> Result<bool, String> {
        let path = self.archive(offer);
        reject_symlink(&path)?;
        if !path.exists() {
            return Ok(false);
        }
        trust.verify_asset(&path, offer.artifact())?;
        Ok(true)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn read_release(path: &Path, trust: &TrustRoot) -> Result<VerifiedRelease, String> {
    crate::package_stage::regular_file(path)?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(RECORD_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > RECORD_LIMIT {
        return Err("update record exceeds its limit".into());
    }
    let record: Record = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if record.schema != 1 {
        return Err("unsupported update record".into());
    }
    trust.restore(&record.release)
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_symlink() => {
            Err("update cache contains an unexpected link".into())
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (TrustRoot, VerifiedRelease, ReleaseIdentity, Platform) {
        let trust =
            TrustRoot::from_tauri_base64(include_str!("../tests/fixtures/public-key.txt")).unwrap();
        let release = trust
            .authenticate(
                include_bytes!("../tests/fixtures/release.json"),
                include_str!("../tests/fixtures/release.sig"),
            )
            .unwrap();
        let mut current = release.manifest().identity.clone();
        current.tag = "v0.1.0".to_owned().try_into().unwrap();
        let platform = release.manifest().artifacts[0].platform;
        (trust, release, current, platform)
    }
    #[test]
    fn a_new_process_restores_only_authenticated_metadata_and_reverifies_the_archive() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("staging");
        let (trust, release, current, platform) = fixture();
        let staging = Staging::open(root.clone()).unwrap();
        assert!(
            Staging::open(root.clone()).is_err(),
            "one updater owns the transaction"
        );
        staging.remember(&release).unwrap();
        let (_, offer) = staging
            .restore(&trust, &current, Component::Cli, platform, Channel::Stable)
            .unwrap()
            .unwrap();
        assert!(!staging.is_ready(&trust, &offer).unwrap());
        fs::write(
            staging.archive(&offer),
            include_bytes!("../tests/fixtures/asset.gz"),
        )
        .unwrap();
        drop(staging);
        let staging = Staging::open(root).unwrap();
        let (_, offer) = staging
            .restore(&trust, &current, Component::Cli, platform, Channel::Stable)
            .unwrap()
            .unwrap();
        assert!(staging.is_ready(&trust, &offer).unwrap());
        fs::write(
            staging.archive(&offer),
            b"interrupted or corrupted download",
        )
        .unwrap();
        assert!(staging.is_ready(&trust, &offer).is_err());
        let path = staging.root.join("selected.json");
        let bytes = fs::read_to_string(&path)
            .unwrap()
            .replace("v0.2.0", "v9.0.0");
        fs::write(path, bytes).unwrap();
        assert!(
            staging
                .restore(&trust, &current, Component::Cli, platform, Channel::Stable)
                .is_err()
        );
    }
    #[cfg(unix)]
    #[test]
    fn staging_cannot_follow_links_into_an_unrelated_installation() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("staging");
        symlink(directory.path(), &root).unwrap();
        assert!(Staging::open(root).is_err());
    }

    #[test]
    fn collection_retains_selected_bytes_and_reclaims_only_authenticated_old_archives() {
        let directory = tempfile::tempdir().unwrap();
        let (trust, release, current, platform) = fixture();
        let staging = Staging::open(directory.path().join("staging")).unwrap();
        staging.remember(&release).unwrap();
        let offer = release
            .select(Component::Cli, platform, &current, Channel::Stable, None)
            .unwrap()
            .unwrap();
        let archive = staging.archive(&offer);
        fs::write(&archive, b"cached payload").unwrap();
        let unknown = staging.root.join(format!("{}.package", "b".repeat(64)));
        fs::write(&unknown, b"unclaimed data").unwrap();
        fs::write(
            staging
                .root
                .join(format!("{}.package.json", "b".repeat(64))),
            b"untrusted",
        )
        .unwrap();
        assert_eq!(staging.collect(&trust).unwrap(), 0);
        assert!(archive.exists());
        fs::write(staging.root.join("selected.json"), b"interrupted selection").unwrap();
        assert!(staging.collect(&trust).is_err());
        assert!(archive.exists());
        fs::remove_file(staging.root.join("selected.json")).unwrap();
        assert_eq!(staging.collect(&trust).unwrap(), 14);
        assert!(!archive.exists());
        assert_eq!(fs::read(unknown).unwrap(), b"unclaimed data");
    }

    #[cfg(unix)]
    #[test]
    fn collection_never_follows_an_archive_link_into_user_data() {
        let directory = tempfile::tempdir().unwrap();
        let (trust, release, current, platform) = fixture();
        let staging = Staging::open(directory.path().join("staging")).unwrap();
        staging.remember(&release).unwrap();
        fs::remove_file(staging.root.join("selected.json")).unwrap();
        let offer = release
            .select(Component::Cli, platform, &current, Channel::Stable, None)
            .unwrap()
            .unwrap();
        let neighbour = directory.path().join("neighbour");
        fs::write(&neighbour, b"user data").unwrap();
        std::os::unix::fs::symlink(&neighbour, staging.archive(&offer)).unwrap();
        assert_eq!(staging.collect(&trust).unwrap(), 0);
        assert_eq!(fs::read(neighbour).unwrap(), b"user data");
        assert!(
            fs::symlink_metadata(staging.archive(&offer))
                .unwrap()
                .is_symlink()
        );
    }
}
