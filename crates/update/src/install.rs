use crate::{Channel, Component, Download, Offer, Platform, ReleaseIdentity};
use khor_runtime::{Image, Limits, Role, Source, Store};
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
    origin: PathBuf,
    local: bool,
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
    pub build: ReleaseIdentity,
    pub channel: Channel,
    pub platform: Platform,
    pub program_digest: String,
    pub previous: Option<PathBuf>,
    pub state: InstallState,
    #[serde(default)]
    pub service_ready: bool,
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
        let (bin, local) = if let Some(hidden) = root
            .parent()
            .filter(|p| p.file_name().is_some_and(|n| n == ".khor"))
        {
            let Some(bin) = hidden.parent() else {
                return Ok(None);
            };
            (bin.to_path_buf(), false)
        } else {
            let Some(origin) = read_origin(root)? else {
                return Ok(None);
            };
            let expected = node_root(
                &origin,
                platform,
                &khor_runtime::local_directory().map_err(|e| e.to_string())?,
            )?;
            if fs::canonicalize(root).map_err(|e| e.to_string())?
                != fs::canonicalize(expected).map_err(|e| e.to_string())?
            {
                return Err("CLI selection belongs to another node or installation".into());
            }
            (origin, true)
        };
        if !is_launcher(&bin.join("khor"))? {
            return Ok(None);
        }
        let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        let install = Self {
            root,
            platform,
            origin: bin,
            local,
        };
        install.current()?;
        Ok(Some(install))
    }

    pub fn source(&self) -> PathBuf {
        self.root.join("current")
    }

    pub fn is_node_local(&self) -> bool {
        self.local
    }

    pub fn prepare(bin: &Path, platform: Platform, runtime: &Path) -> Result<Self, String> {
        let bin = fs::canonicalize(bin).map_err(|e| e.to_string())?;
        Store::open(runtime.to_path_buf(), Limits::default()).map_err(|e| e.to_string())?;
        let runtime = fs::canonicalize(runtime).map_err(|e| e.to_string())?;
        let root = node_root(&bin, platform, &runtime)?;
        let cli = runtime.join("cli");
        private_directory(&cli)?;
        private_directory(root.parent().ok_or("invalid node installation")?)?;
        private_directory(&root)?;
        let origin = root.join("origin.json");
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&origin)
        {
            Ok(mut file) => {
                file.write_all(&serde_json::to_vec(&bin).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                file.sync_all().map_err(|e| e.to_string())?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
        if read_origin(&root)?.as_ref() != Some(&bin) {
            return Err("node installation has a different owner".into());
        }
        let install = Self {
            root,
            platform,
            origin: bin,
            local: true,
        };
        install.images()?;
        Ok(install)
    }

    pub fn on_node(
        &self,
        runtime: &Path,
        bootstrap: impl FnOnce() -> Result<Image, String>,
    ) -> Result<Self, String> {
        let local = Self::prepare(&self.origin, self.platform, runtime)?;
        if local.current()?.is_some() {
            return Ok(local);
        }
        let _lock = local.lock()?;
        if local.current()?.is_none() {
            let previous = bootstrap()?;
            if previous.role() != Role::Cli {
                return Err("this installation is not the CLI".into());
            }
            let store = local.images()?;
            let image = store
                .publish(
                    Role::Cli,
                    previous.version(),
                    previous.contract().clone(),
                    previous.embedded().clone(),
                    Source::File(previous.path()),
                )
                .map_err(|e| e.to_string())?;
            local.point(
                image
                    .path()
                    .strip_prefix(&local.root)
                    .map_err(|e| e.to_string())?,
            )?;
        }
        drop(_lock);
        Ok(local)
    }

    pub fn selected_on_node(&self, runtime: &Path) -> Result<PathBuf, String> {
        if self.local {
            return Ok(self.source());
        }
        let root = node_root(&self.origin, self.platform, runtime)?;
        let local = Self {
            root,
            platform: self.platform,
            origin: self.origin.clone(),
            local: true,
        };
        if local.root.try_exists().map_err(|e| e.to_string())? {
            khor_runtime::validate_local_directory(&local.root).map_err(|e| e.to_string())?;
            if read_origin(&local.root)?.as_ref() != Some(&local.origin) {
                return Err("node selection belongs to another installation".into());
            }
            if local.current()?.is_some() {
                return Ok(local.source());
            }
        }
        Ok(self.source())
    }

    pub fn bootstrap_source(&self) -> Result<Option<PathBuf>, String> {
        let shared = self
            .origin
            .join(".khor")
            .join(asset_name(self.platform)?)
            .join("current");
        if shared.try_exists().map_err(|e| e.to_string())? {
            return Ok(Some(shared));
        }
        let entry = self.origin.join("khor");
        if !entry.try_exists().map_err(|e| e.to_string())? || is_launcher(&entry)? {
            return Ok(None);
        }
        Ok(Some(entry))
    }

    pub fn publish_launcher(&self) -> Result<(), String> {
        let mut stage = tempfile::NamedTempFile::new_in(&self.origin).map_err(|e| e.to_string())?;
        stage
            .write_all(launcher()?.as_bytes())
            .map_err(|e| e.to_string())?;
        stage
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
        stage.as_file().sync_all().map_err(|e| e.to_string())?;
        stage
            .persist(self.origin.join("khor"))
            .map_err(|e| e.to_string())?;
        sync(&self.origin)
    }

    fn images(&self) -> Result<Store, String> {
        if !self.local {
            return Err("adopt this CLI on the current node before updating it".into());
        }
        Store::open(self.root.join("versions"), Limits::default()).map_err(|e| e.to_string())
    }

    pub fn staging(&self) -> PathBuf {
        self.root.join("staging")
    }

    pub fn activation_ready(&self) -> Result<bool, String> {
        Ok(!self
            .receipt()?
            .is_some_and(|r| matches!(r.state, InstallState::Prepared | InstallState::Activating)))
    }

    pub fn adopted_identity(&self, compiled: &ReleaseIdentity) -> Result<ReleaseIdentity, String> {
        if let Some(receipt) = self.receipt()? {
            if receipt.state == InstallState::Complete
                && &receipt.build == compiled
                && self.current()?.as_ref().is_some_and(|path| {
                    hash(path).is_ok_and(|digest| digest == receipt.program_digest)
                })
            {
                return Ok(receipt.identity);
            }
        }
        Ok(compiled.clone())
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
                    || receipt.identity.source != receipt.build.source
                    || receipt.identity.build != receipt.build.build
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
        if let Some(mut receipt) = self.receipt()? {
            if receipt.state == InstallState::Complete
                && &receipt.identity == offer.identity()
                && &receipt.build == offer.build_identity()
                && receipt.program_digest == digest
                && previous
                    .as_ref()
                    .is_some_and(|path| hash(path).is_ok_and(|value| value == digest))
            {
                health(&self.source(), offer)?;
                receipt.channel = channel;
                self.record(&receipt)?;
                self.retain_selected()?;
                return Ok(receipt);
            }
        }
        let store = self.images()?;
        let image = store
            .stage_checked(
                Role::Cli,
                offer.identity().tag.as_str(),
                offer.artifact().contract.clone(),
                Default::default(),
                Source::File(&program),
                |_| Ok(()),
            )
            .map_err(|e| e.to_string())?;
        let relative = image
            .path()
            .strip_prefix(&self.root)
            .map_err(|e| e.to_string())?
            .to_path_buf();
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
            build: offer.build_identity().clone(),
            channel,
            platform: self.platform,
            program_digest: digest,
            previous: previous_relative,
            state: InstallState::Prepared,
            service_ready: false,
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
        store.select(&image).map_err(|e| e.to_string())?;
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
                InstallState::Prepared | InstallState::Activating | InstallState::Complete
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
        if receipt.state == InstallState::Complete && self.local {
            self.retain_selected()?;
            return Ok(Some(receipt));
        }
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
            self.retain_selected()?;
        }
        Ok(Some(receipt))
    }

    pub fn service_start_pending(&self, program: &Path) -> Result<bool, String> {
        let Some(receipt) = self.receipt()? else {
            return Ok(false);
        };
        if receipt.state != InstallState::Complete || receipt.service_ready {
            return Ok(false);
        }
        Ok(
            self.current()?.as_ref()
                == Some(&fs::canonicalize(program).map_err(|e| e.to_string())?),
        )
    }

    pub fn finish_service_start(&self, program: &Path, healthy: bool) -> Result<bool, String> {
        let _lock = self.lock()?;
        if !self.service_start_pending(program)? {
            return Ok(false);
        }
        let mut receipt = self.receipt()?.ok_or("CLI startup record disappeared")?;
        if healthy {
            receipt.service_ready = true;
        } else {
            if receipt.previous.is_none() {
                return Ok(false);
            }
            self.rollback(&receipt)?;
            receipt.state = InstallState::RolledBack;
        }
        self.record(&receipt)?;
        self.retain_selected()?;
        Ok(true)
    }

    fn retain_selected(&self) -> Result<(), String> {
        if let Some(program) = self.current()? {
            let image = Store::image_for_program(&program)
                .map_err(|e| e.to_string())?
                .ok_or("selected CLI has no owned runtime image")?;
            self.images()?.select(&image).map_err(|e| e.to_string())?;
        }
        Ok(())
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

fn node_root(bin: &Path, platform: Platform, runtime: &Path) -> Result<PathBuf, String> {
    use std::os::unix::ffi::OsStrExt;
    let id = format!("{:x}", Sha256::digest(bin.as_os_str().as_bytes()));
    Ok(runtime.join("cli").join(id).join(asset_name(platform)?))
}
fn read_origin(root: &Path) -> Result<Option<PathBuf>, String> {
    match File::open(root.join("origin.json")) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(16 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 16 * 1024 {
                return Err("CLI origin record is too large".into());
            }
            serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| e.to_string())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn private_directory(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::DirBuilderExt;
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    khor_runtime::validate_local_directory(path).map_err(|e| e.to_string())
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
fn is_launcher(path: &Path) -> Result<bool, String> {
    let mut bytes = Vec::new();
    match File::open(path) {
        Ok(file) => {
            file.take(256 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.to_string()),
    }
    Ok(bytes.len() <= 256 * 1024 && bytes.starts_with(b"#!/bin/sh\n# KHOR_CLI_LAUNCHER_V1\n"))
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
            let shared = CliInstallation::detect(&source, platform).unwrap().unwrap();
            let runtime = directory.path().join("runtime");
            let bootstrap = Store::open(runtime.clone(), Limits::default())
                .unwrap()
                .publish(
                    Role::Cli,
                    "previous",
                    Download::fixture().offer().artifact().contract.clone(),
                    Default::default(),
                    Source::File(&old),
                )
                .unwrap();
            let install = shared.on_node(&runtime, || Ok(bootstrap)).unwrap();
            let old = install.current().unwrap().unwrap();
            Self {
                _directory: directory,
                install,
                old,
                source,
            }
        }
    }
    #[test]
    fn failed_resident_startup_restores_previous_selection_but_late_results_cannot_rollback_it() {
        let fixture = Fixture::new();
        let install = &fixture.install;
        install
            .install(
                &Download::fixture(),
                Channel::Stable,
                |_, _, _| Ok(()),
                |_, _| Ok(()),
            )
            .unwrap();
        let candidate = install.current().unwrap().unwrap();
        assert!(install.service_start_pending(&candidate).unwrap());
        assert!(install.finish_service_start(&candidate, false).unwrap());
        assert_eq!(install.current().unwrap().unwrap(), fixture.old);
        assert_eq!(
            install.receipt().unwrap().unwrap().state,
            InstallState::RolledBack
        );
        assert!(!install.finish_service_start(&candidate, true).unwrap());
        assert!(!install.finish_service_start(&candidate, false).unwrap());
        assert_eq!(install.current().unwrap().unwrap(), fixture.old);
    }

    #[test]
    fn ready_resident_clears_pending_startup_and_later_failure_is_not_an_installation_rollback() {
        let fixture = Fixture::new();
        let install = &fixture.install;
        install
            .install(
                &Download::fixture(),
                Channel::Stable,
                |_, _, _| Ok(()),
                |_, _| Ok(()),
            )
            .unwrap();
        let candidate = install.current().unwrap().unwrap();
        assert!(install.finish_service_start(&candidate, true).unwrap());
        assert!(!install.service_start_pending(&candidate).unwrap());
        assert!(!install.finish_service_start(&candidate, false).unwrap());
        assert_eq!(install.current().unwrap().unwrap(), candidate);
        assert!(install.receipt().unwrap().unwrap().service_ready);
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
        assert_eq!(
            install.receipt().unwrap().unwrap().previous,
            first.previous,
            "reinstalling the same release must preserve the previous rollback selection"
        );
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
        assert!(!install.activation_ready().unwrap());
        let _active = install.lock().unwrap();
        let observed = install
            .recover(|_, _| panic!("live updater owns the health check"))
            .unwrap()
            .unwrap();
        assert_eq!(observed.state, InstallState::Activating);
        assert_eq!(observed.channel, Channel::Beta);
    }
    #[test]
    fn a_promoted_installation_remembers_the_publication_without_inventing_a_different_build() {
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
        receipt.build.tag = "v0.2.0-rc.4".to_owned().try_into().unwrap();
        install.record(&receipt).unwrap();
        assert_eq!(
            install
                .adopted_identity(&receipt.build)
                .unwrap()
                .tag
                .as_str(),
            "v0.2.0"
        );
        let mut unrelated = receipt.build.clone();
        unrelated.source = "b".repeat(40).try_into().unwrap();
        assert_eq!(install.adopted_identity(&unrelated).unwrap(), unrelated);
        let program = install.current().unwrap().unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(program, b"altered installation").unwrap();
        assert_eq!(
            install.adopted_identity(&receipt.build).unwrap(),
            receipt.build
        );
    }
    #[test]
    fn external_installations_and_escaping_pointers_are_never_adopted() {
        let fixture = Fixture::new();
        let bin = &fixture.install.origin;
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
    #[test]
    fn shared_installations_select_independent_local_versions_even_when_the_shared_home_goes_offline()
     {
        let fixture = Fixture::new();
        let shared = CliInstallation::detect(&fixture.source, fixture.install.platform)
            .unwrap()
            .unwrap();
        let other_runtime = fixture._directory.path().join("other-runtime");
        let image = Store::image_for_program(&fixture.old).unwrap().unwrap();
        let other = shared.on_node(&other_runtime, || Ok(image)).unwrap();
        let other_old = other.current().unwrap().unwrap();
        fixture
            .install
            .install(
                &Download::fixture(),
                Channel::Stable,
                |_, _, _| Ok(()),
                |_, _| Ok(()),
            )
            .unwrap();
        assert_eq!(other.current().unwrap().unwrap(), other_old);
        assert_eq!(fs::read(&fixture.source).unwrap(), b"previous program");
        let _updating = other.lock().unwrap();
        let before = Instant::now();
        other
            .on_node(&other_runtime, || {
                panic!("selected nodes must not re-adopt shared state")
            })
            .unwrap();
        assert!(before.elapsed() < Duration::from_secs(1));
        fs::rename(
            fixture.install.origin.join(".khor"),
            fixture.install.origin.join("offline-home"),
        )
        .unwrap();
        assert_eq!(
            fs::read(other.current().unwrap().unwrap()).unwrap(),
            b"previous program"
        );
        assert_eq!(
            fs::read(fixture.install.current().unwrap().unwrap()).unwrap(),
            b"Khor update fixture program\n"
        );
    }
}
