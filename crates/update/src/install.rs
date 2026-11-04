use crate::{Channel, Component, Download, Offer, Platform, ReleaseIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub struct CliInstallation {
    root: PathBuf,
    platform: Platform,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallState {
    Prepared,
    Activating,
    Complete,
    RolledBack,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallReceipt {
    pub schema: u8,
    pub identity: ReleaseIdentity,
    pub channel: Channel,
    pub platform: Platform,
    pub program_digest: String,
    pub previous: Option<PathBuf>,
    pub state: InstallState,
}

impl CliInstallation {
    pub fn detect(source: &Path, platform: Platform) -> Result<Option<Self>, String> {
        if source.file_name().and_then(|s| s.to_str()) != Some("current") {
            return Ok(None);
        }
        let Some(root) = source.parent() else {
            return Ok(None);
        };
        if root.file_name().and_then(|s| s.to_str()) != Some(asset_name(platform)?) {
            return Ok(None);
        }
        let Some(hidden) = root
            .parent()
            .filter(|p| p.file_name().is_some_and(|name| name == ".khor"))
        else {
            return Ok(None);
        };
        let Some(bin) = hidden.parent() else {
            return Ok(None);
        };
        if fs::read(bin.join("khor")).ok().as_deref() != Some(launcher()?.as_bytes()) {
            return Ok(None);
        }
        let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        let install = Self { root, platform };
        install.current()?;
        Ok(Some(install))
    }

    pub fn staging(&self) -> PathBuf {
        self.root.join("staging")
    }

    pub fn current(&self) -> Result<Option<PathBuf>, String> {
        let current = self.root.join("current");
        let relative = match fs::read_link(&current) {
            Ok(path) => path,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        relative_program(&relative)?;
        let target = fs::canonicalize(self.root.join(&relative)).map_err(|e| e.to_string())?;
        if !target.starts_with(self.root.join("versions")) || !target.is_file() {
            return Err("managed CLI source is invalid".into());
        }
        Ok(Some(target))
    }

    pub fn receipt(&self) -> Result<Option<InstallReceipt>, String> {
        let path = self.root.join("update.json");
        match File::open(path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(16 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                if bytes.len() > 16 * 1024 {
                    return Err("installation record exceeds its limit".into());
                }
                let receipt: InstallReceipt =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                if receipt.schema != 1
                    || receipt.platform != self.platform
                    || crate::Digest::try_from(receipt.program_digest.clone()).is_err()
                {
                    return Err("installation record belongs to another platform or program".into());
                }
                if let Some(path) = &receipt.previous {
                    relative_program(path)?;
                }
                Ok(Some(receipt))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn install(
        &self,
        download: &Download,
        channel: Channel,
        check: impl Fn(&Path, Option<&Path>, &Offer) -> Result<(), String>,
        health: impl Fn(&Path, &Offer) -> Result<(), String>,
    ) -> Result<InstallReceipt, String> {
        let offer = download.offer();
        if offer.artifact().component != Component::Cli
            || offer.artifact().platform != self.platform
        {
            return Err("release asset belongs to another installation".into());
        }
        if khor_runtime::available_space(&self.root).map_err(|e| e.to_string())?
            < offer
                .artifact()
                .unpacked_bytes
                .saturating_add(128 * 1024 * 1024)
        {
            return Err("not enough free space to publish this release".into());
        }
        let program = download.unpack_cli()?;
        let digest = hash(&program)?;
        let _lock = self.lock()?;
        let previous = self.current()?;
        check(&program, previous.as_deref(), offer)?;
        let version = format!("{}-{}", offer.identity().tag.as_str(), digest);
        let relative = Path::new("versions").join(version).join("khor");
        let destination = self.root.join(&relative);
        let parent = destination.parent().ok_or("invalid CLI destination")?;
        directory(&self.root.join("versions"))?;
        directory(parent)?;
        match fs::symlink_metadata(&destination) {
            Ok(meta) => {
                if !meta.is_file() || hash(&destination)? != digest {
                    return Err("immutable CLI installation differs from its release".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut temporary =
                    tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
                std::io::copy(
                    &mut File::open(&program).map_err(|e| e.to_string())?,
                    &mut temporary,
                )
                .map_err(|e| e.to_string())?;
                temporary
                    .as_file()
                    .set_permissions(fs::Permissions::from_mode(0o555))
                    .map_err(|e| e.to_string())?;
                temporary.as_file().sync_all().map_err(|e| e.to_string())?;
                if hash(temporary.path())? != digest {
                    return Err("CLI changed during publication".into());
                }
                fs::hard_link(temporary.path(), &destination).map_err(|e| e.to_string())?;
                sync(parent)?;
            }
            Err(e) => return Err(e.to_string()),
        }
        let previous_relative = previous
            .as_ref()
            .map(|path| {
                path.strip_prefix(&self.root)
                    .map(Path::to_path_buf)
                    .map_err(|e| e.to_string())
            })
            .transpose()?;
        let mut receipt = InstallReceipt {
            schema: 1,
            identity: offer.identity().clone(),
            channel,
            platform: self.platform,
            program_digest: digest,
            previous: previous_relative,
            state: InstallState::Prepared,
        };
        self.record(&receipt)?;
        receipt.state = InstallState::Activating;
        self.record(&receipt)?;
        self.point(&relative)?;
        if let Err(error) = health(&self.root.join("current"), offer) {
            self.rollback(&receipt)?;
            receipt.state = InstallState::RolledBack;
            self.record(&receipt)?;
            return Err(format!(
                "update health check failed; previous CLI retained: {error}"
            ));
        }
        receipt.state = InstallState::Complete;
        self.record(&receipt)?;
        Ok(receipt)
    }

    pub fn recover(
        &self,
        health: impl Fn(&Path, &InstallReceipt) -> Result<(), String>,
    ) -> Result<Option<InstallReceipt>, String> {
        let observed = self.receipt()?;
        if !observed.as_ref().is_some_and(|receipt| {
            matches!(
                receipt.state,
                InstallState::Prepared | InstallState::Activating
            )
        }) {
            return Ok(observed);
        }
        let lock = self.open_lock()?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => return Ok(observed),
            Err(error) => return Err(error.to_string()),
        }
        let Some(mut receipt) = self.receipt()? else {
            return Ok(None);
        };
        if !matches!(
            receipt.state,
            InstallState::Prepared | InstallState::Activating
        ) {
            return Ok(Some(receipt));
        }
        if self
            .current()?
            .as_ref()
            .is_some_and(|path| hash(path).is_ok_and(|digest| digest == receipt.program_digest))
        {
            if let Err(error) = health(&self.root.join("current"), &receipt) {
                self.rollback(&receipt)?;
                receipt.state = InstallState::RolledBack;
                self.record(&receipt)?;
                return Err(format!(
                    "interrupted update failed its health check; previous CLI retained: {error}"
                ));
            }
            receipt.state = InstallState::Complete;
            self.record(&receipt)?;
        }
        Ok(Some(receipt))
    }

    fn lock(&self) -> Result<File, String> {
        let file = self.open_lock()?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(file),
                Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                Err(error) => return Err(format!("another CLI update is active: {error}")),
            }
        }
    }

    fn open_lock(&self) -> Result<File, String> {
        let path = self.root.join("update.lock");
        if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
            return Err("update lock is not a regular file".into());
        }
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|e| e.to_string())
    }

    fn point(&self, relative: &Path) -> Result<(), String> {
        relative_program(relative)?;
        let stage = tempfile::tempdir_in(&self.root).map_err(|e| e.to_string())?;
        let link = stage.path().join("current");
        symlink(relative, &link).map_err(|e| e.to_string())?;
        fs::rename(link, self.root.join("current")).map_err(|e| e.to_string())?;
        sync(&self.root)
    }
    fn rollback(&self, receipt: &InstallReceipt) -> Result<(), String> {
        if let Some(previous) = &receipt.previous {
            self.point(previous)
        } else {
            fs::remove_file(self.root.join("current")).map_err(|e| e.to_string())?;
            sync(&self.root)
        }
    }
    fn record(&self, receipt: &InstallReceipt) -> Result<(), String> {
        let mut temporary =
            tempfile::NamedTempFile::new_in(&self.root).map_err(|e| e.to_string())?;
        temporary
            .write_all(&serde_json::to_vec(receipt).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        temporary
            .persist(self.root.join("update.json"))
            .map_err(|e| e.to_string())?;
        sync(&self.root)
    }
}

fn directory(path: &Path) -> Result<(), String> {
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(e)
            if e.kind() == std::io::ErrorKind::AlreadyExists
                && fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && !m.is_symlink()) =>
        {
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}
fn sync(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())
}
fn hash(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut bytes = [0; 64 * 1024];
    loop {
        let count = file.read(&mut bytes).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&bytes[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn asset_name(platform: Platform) -> Result<&'static str, String> {
    use crate::{Architecture::*, OperatingSystem::*};
    match (platform.os, platform.arch) {
        (Macos, Aarch64) => Ok("khor-macos-arm64"),
        (Macos, X86_64) => Ok("khor-macos-x86_64"),
        (Linux, Aarch64) => Ok("khor-linux-aarch64"),
        (Linux, X86_64) => Ok("khor-linux-x86_64"),
        _ => Err("this CLI installation uses an external package manager".into()),
    }
}
fn launcher() -> Result<&'static str, String> {
    include_str!("../../../scripts/install-components.sh")
        .split_once("<<'KHOR_LAUNCHER'\n")
        .and_then(|(_, rest)| rest.split_once("KHOR_LAUNCHER\n"))
        .map(|(script, _)| script)
        .ok_or_else(|| "canonical CLI launcher is missing".into())
}
fn relative_program(path: &Path) -> Result<(), String> {
    let components: Vec<_> = path.components().collect();
    if components.len() == 3
        && components[0].as_os_str() == "versions"
        && components[2].as_os_str() == "khor"
        && components
            .iter()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
    {
        Ok(())
    } else {
        Err("managed CLI pointer leaves its installation".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _directory: tempfile::TempDir,
        install: CliInstallation,
        old: PathBuf,
        source: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let platform = Platform::current().unwrap();
            let bin = directory.path().join("bin");
            let root = bin.join(".khor").join(asset_name(platform).unwrap());
            let old = root.join("versions/old/khor");
            fs::create_dir_all(old.parent().unwrap()).unwrap();
            fs::write(&old, b"previous program").unwrap();
            fs::write(bin.join("khor"), launcher().unwrap()).unwrap();
            let source = root.join("current");
            symlink("versions/old/khor", &source).unwrap();
            let install = CliInstallation::detect(&source, platform).unwrap().unwrap();
            Self {
                _directory: directory,
                install,
                old,
                source,
            }
        }
    }
    #[test]
    fn activation_rejects_incompatibility_and_rolls_back_failed_health_without_touching_old_bytes()
    {
        let fixture = Fixture::new();
        let install = &fixture.install;
        assert!(
            install
                .install(
                    &Download::fixture(),
                    Channel::Stable,
                    |_, _, _| Err("incompatible storage".into()),
                    |_, _| Ok(())
                )
                .is_err()
        );
        assert_eq!(install.current().unwrap().unwrap(), fixture.old);
        assert!(install.receipt().unwrap().is_none());
        assert!(
            install
                .install(
                    &Download::fixture(),
                    Channel::Stable,
                    |_, _, _| Ok(()),
                    |_, _| Err("health failed".into())
                )
                .is_err()
        );
        assert_eq!(install.current().unwrap().unwrap(), fixture.old);
        assert_eq!(
            install.receipt().unwrap().unwrap().state,
            InstallState::RolledBack
        );
        assert_eq!(fs::read(&fixture.old).unwrap(), b"previous program");
        install
            .install(
                &Download::fixture(),
                Channel::Stable,
                |_, _, _| Ok(()),
                |_, _| Ok(()),
            )
            .unwrap();
        assert_ne!(install.current().unwrap().unwrap(), fixture.old);
        assert_eq!(fs::read(&fixture.old).unwrap(), b"previous program");
    }
    #[test]
    fn repeated_install_reuses_immutable_program_and_interrupted_activation_can_be_confirmed_or_rolled_back()
     {
        use std::os::unix::fs::MetadataExt;
        let fixture = Fixture::new();
        let install = &fixture.install;
        let first = install
            .install(
                &Download::fixture(),
                Channel::Stable,
                |_, _, _| Ok(()),
                |_, _| Ok(()),
            )
            .unwrap();
        let program = install.current().unwrap().unwrap();
        let inode = fs::metadata(&program).unwrap().ino();
        install
            .install(
                &Download::fixture(),
                Channel::Stable,
                |_, _, _| Ok(()),
                |_, _| Ok(()),
            )
            .unwrap();
        assert_eq!(fs::metadata(&program).unwrap().ino(), inode);
        let mut interrupted = first.clone();
        interrupted.state = InstallState::Activating;
        install.record(&interrupted).unwrap();
        assert_eq!(
            install.recover(|_, _| Ok(())).unwrap().unwrap().state,
            InstallState::Complete
        );
        install.record(&interrupted).unwrap();
        assert!(install.recover(|_, _| Err("unhealthy".into())).is_err());
        assert_eq!(install.current().unwrap().unwrap(), fixture.old);
        assert_eq!(
            install.receipt().unwrap().unwrap().state,
            InstallState::RolledBack
        );
    }
    #[test]
    fn startup_recovery_does_not_wait_for_or_take_over_an_active_updater() {
        let fixture = Fixture::new();
        let install = &fixture.install;
        let mut receipt = install
            .install(
                &Download::fixture(),
                Channel::Beta,
                |_, _, _| Ok(()),
                |_, _| Ok(()),
            )
            .unwrap();
        receipt.state = InstallState::Activating;
        install.record(&receipt).unwrap();
        let _active = install.lock().unwrap();
        let observed = install
            .recover(|_, _| panic!("live updater owns the health check"))
            .unwrap()
            .unwrap();
        assert_eq!(observed.state, InstallState::Activating);
        assert_eq!(observed.channel, Channel::Beta);
    }
    #[test]
    fn external_installations_and_escaping_pointers_are_never_adopted() {
        let fixture = Fixture::new();
        let bin = fixture.install.root.parent().unwrap().parent().unwrap();
        fs::write(
            bin.join("khor"),
            b"#!/bin/sh\nexec external-package-khor \"$@\"\n",
        )
        .unwrap();
        assert!(
            CliInstallation::detect(&fixture.source, fixture.install.platform)
                .unwrap()
                .is_none()
        );
        fs::write(bin.join("khor"), launcher().unwrap()).unwrap();
        fs::remove_file(&fixture.source).unwrap();
        symlink("../../../outside", &fixture.source).unwrap();
        assert!(CliInstallation::detect(&fixture.source, fixture.install.platform).is_err());
        assert!(fixture.old.is_file());
    }
}
