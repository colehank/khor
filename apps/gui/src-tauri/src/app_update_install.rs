use khor_update::{BuildIdentity, SignedRelease, TrustRoot, VerifiedRelease};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::OnceLock,
    time::{Duration, Instant},
};

static EXIT_LEASE: OnceLock<File> = OnceLock::new();

mod collection;
pub(crate) use collection::collect;

fn spawn_monitor(program: &Path, directory: &Path, output: File) -> Result<Child, String> {
    Command::new(program)
        .arg("_app_update_watch")
        .arg(directory)
        .current_dir("/")
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output.try_clone().map_err(|e| e.to_string())?))
        .stderr(Stdio::from(output))
        .env_remove("KHOR_RUNTIME_LEASE_FD")
        .env_remove("KHOR_APP_UPDATE_RECORD")
        .env_remove("KHOR_APP_UPDATE_TOKEN")
        .spawn()
        .map_err(|e| e.to_string())
}

pub(crate) fn boot_script() -> &'static str {
    if std::env::var_os("KHOR_APP_UPDATE_RECORD").is_some() {
        "window.__KHOR_UPDATE_BOOT__=true;"
    } else {
        ""
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
enum Phase {
    Preparing,
    Prepared,
    Installing,
    Installed,
    Cancelled,
    Complete,
    RolledBack,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: u8,
    nonce: String,
    target: PathBuf,
    old: BuildIdentity,
    release: SignedRelease,
    phase: Phase,
}

pub(crate) struct Installation {
    directory: PathBuf,
    receipt: Receipt,
    lease: Option<File>,
    worker: Child,
}

impl Installation {
    pub(crate) fn prepare(root: &Path, release: &VerifiedRelease) -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let target = bundle_at(&executable)?;
        let target = std::fs::canonicalize(target).map_err(|e| e.to_string())?;
        let parent = target
            .parent()
            .ok_or("application has no installation directory")?;
        // A rollback must be possible without a second unattended authorization prompt.
        let writable = tempfile::tempdir_in(parent)
            .map_err(|_| "the application folder is not writable; update through its installer")?;
        drop(writable);
        let old = khor_update::build_identity()?;
        let size = bundle_bytes(&target)?;
        let required = size
            .checked_mul(2)
            .ok_or("application backup size overflow")?;
        let required = required
            .checked_add(128 * 1024 * 1024)
            .ok_or("application backup size overflow")?;
        if khor_runtime::available_space(root).map_err(|e| e.to_string())? < required {
            return Err("not enough space to retain the previous application".into());
        }
        let directory = tempfile::Builder::new()
            .prefix("install-")
            .tempdir_in(root)
            .map_err(|e| e.to_string())?;
        let path = directory.path();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let nonce = path
            .file_name()
            .ok_or("installation has no identity")?
            .to_string_lossy()
            .into_owned();
        let mut receipt = Receipt {
            schema: 1,
            nonce,
            target,
            old,
            release: release.signed().clone(),
            phase: Phase::Preparing,
        };
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path.join("parent.lock"))
            .map_err(|e| e.to_string())?;
        lease.lock().map_err(|e| e.to_string())?;
        write_json(path, "receipt.json", &receipt)?;
        let backup = path.join("previous.app");
        copy_bundle(&receipt.target, &backup)?;
        let previous = backup.join("Contents/MacOS/khor-gui");
        if khor_node::updates::inspect(&previous)?.build != receipt.old {
            return Err("the previous application changed during its backup".into());
        }
        receipt.phase = Phase::Prepared;
        write_json(path, "receipt.json", &receipt)?;
        let output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path.join("worker.log"))
            .map_err(|e| e.to_string())?;
        let worker = spawn_monitor(&previous, path, output)?;
        let directory = directory.keep();
        let mut installation = Self {
            directory,
            receipt,
            lease: Some(lease),
            worker,
        };
        let until = Instant::now() + Duration::from_secs(10);
        while !installation.directory.join("worker.ready").is_file() {
            if installation
                .worker
                .try_wait()
                .map_err(|e| e.to_string())?
                .is_some()
                || Instant::now() > until
            {
                installation.cancel();
                return Err("application update monitor did not start".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(installation)
    }

    pub(crate) fn handoff(mut self) -> Result<(), String> {
        if EXIT_LEASE.get().is_some() {
            return Err("an application update is already exiting".into());
        }
        if self.worker.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Err("application update monitor stopped".into());
        }
        self.receipt.phase = Phase::Installed;
        write_json(&self.directory, "receipt.json", &self.receipt)?;
        EXIT_LEASE
            .set(
                self.lease
                    .take()
                    .ok_or("application exit lease was released")?,
            )
            .map_err(|_| "an application update is already exiting")?;
        Ok(())
    }

    pub(crate) fn cancel(&mut self) {
        self.receipt.phase = Phase::Cancelled;
        let _ = write_json(&self.directory, "receipt.json", &self.receipt);
    }

    pub(crate) fn begin(&mut self) -> Result<(), String> {
        self.receipt.phase = Phase::Installing;
        write_json(&self.directory, "receipt.json", &self.receipt)
    }

    pub(crate) fn restore_after_failure(&mut self) -> Result<(), String> {
        restore_previous(&self.directory, &self.receipt)?;
        write_json(
            self.directory
                .parent()
                .ok_or("installation has no staging directory")?,
            "deferred.json",
            &self.receipt.release,
        )?;
        self.cancel();
        Ok(())
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        if self.lease.is_some() && self.receipt.phase == Phase::Installing {
            if let Some(lease) = self.lease.take() {
                let _ = EXIT_LEASE.set(lease);
            }
        } else if self.lease.is_some() {
            self.cancel();
        }
    }
}

pub(crate) fn main_if_requested() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("_app_update_watch")) {
        return None;
    }
    let result = match (args.next(), args.next()) {
        (Some(path), None) => watch(Path::new(&path)),
        _ => Err("application update monitor expects one transaction".into()),
    };
    Some(match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    })
}

fn watch(directory: &Path) -> Result<(), String> {
    validate_directory(directory)?;
    let worker = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("worker.lock"))
        .map_err(|e| e.to_string())?;
    worker
        .try_lock()
        .map_err(|_| "another monitor owns this application update")?;
    EXIT_LEASE
        .set(worker)
        .map_err(|_| "this monitor already holds a process lease")?;
    let parent = OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.join("parent.lock"))
        .map_err(|e| e.to_string())?;
    std::fs::write(directory.join("worker.ready"), b"ready\n").map_err(|e| e.to_string())?;
    loop {
        let receipt = read_receipt(directory)?;
        if receipt.phase == Phase::Cancelled {
            return Ok(());
        }
        match parent.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) => {
                std::thread::sleep(Duration::from_millis(200))
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let mut receipt = read_receipt(directory)?;
    if matches!(
        receipt.phase,
        Phase::Complete | Phase::Cancelled | Phase::RolledBack
    ) {
        return Ok(());
    }
    let release = TrustRoot::bundled()?.restore(&receipt.release)?;
    let program = receipt.target.join("Contents/MacOS/khor-gui");
    let result = if receipt.phase == Phase::Installed {
        launch_and_wait(&program, directory, &receipt, &release)
    } else {
        Err("application installation was interrupted".into())
    };
    if result.is_ok() {
        receipt.phase = Phase::Complete;
        write_json(directory, "receipt.json", &receipt)?;
        return Ok(());
    }
    eprintln!("update was not healthy: {}", result.unwrap_err());
    restore_previous(directory, &receipt)?;
    write_json(
        directory
            .parent()
            .ok_or("installation has no staging directory")?,
        "deferred.json",
        &receipt.release,
    )?;
    receipt.phase = Phase::RolledBack;
    write_json(directory, "receipt.json", &receipt)?;
    launch(&program, None)?;
    Ok(())
}

fn launch_and_wait(
    program: &Path,
    directory: &Path,
    receipt: &Receipt,
    release: &VerifiedRelease,
) -> Result<(), String> {
    verify_installed(program, release)?;
    let mut child = launch(program, Some((directory, &receipt.nonce)))?;
    let until = Instant::now() + Duration::from_secs(90);
    loop {
        if let Ok(health) = read_json::<Health>(&directory.join("healthy.json")) {
            if health.nonce == receipt.nonce
                && health.build.release.as_ref() == Some(&release.manifest().build_identity())
            {
                write_json(
                    directory
                        .parent()
                        .ok_or("installation has no staging directory")?,
                    "adopted.json",
                    &receipt.release,
                )?;
                return Ok(());
            }
        }
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Err("updated application exited before restoring its interface".into());
        }
        if Instant::now() > until {
            // This is only the child this transaction launched, never an unrelated session host.
            let _ = child.kill();
            let _ = child.wait();
            return Err("updated application did not confirm its restored interface".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn launch(program: &Path, health: Option<(&Path, &str)>) -> Result<Child, String> {
    let mut command = Command::new(program);
    command
        .current_dir("/")
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env_remove("KHOR_RUNTIME_LEASE_FD")
        .env_remove("KHOR_APP_UPDATE_RECORD")
        .env_remove("KHOR_APP_UPDATE_TOKEN");
    if let Some((directory, nonce)) = health {
        command
            .env("KHOR_APP_UPDATE_RECORD", directory)
            .env("KHOR_APP_UPDATE_TOKEN", nonce);
    }
    command.spawn().map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
struct Health {
    nonce: String,
    build: BuildIdentity,
}

pub(crate) fn ui_ready() -> Result<(), String> {
    let Some(directory) = std::env::var_os("KHOR_APP_UPDATE_RECORD") else {
        return Ok(());
    };
    let directory = Path::new(&directory);
    validate_directory(directory)?;
    let receipt = read_receipt(directory)?;
    if std::env::var("KHOR_APP_UPDATE_TOKEN").ok().as_deref() != Some(&receipt.nonce) {
        return Err("application health confirmation belongs to another update".into());
    }
    let build = khor_update::build_identity()?;
    let release = TrustRoot::bundled()?.restore(&receipt.release)?;
    verify_installed(
        &std::env::current_exe().map_err(|e| e.to_string())?,
        &release,
    )?;
    if !matches!(receipt.phase, Phase::Installed | Phase::Complete)
        || build.release.as_ref() != Some(&release.manifest().build_identity())
        || std::fs::canonicalize(bundle_at(
            &std::env::current_exe().map_err(|e| e.to_string())?,
        )?)
        .map_err(|e| e.to_string())?
            != receipt.target
    {
        return Err("application health confirmation does not match the installed release".into());
    }
    write_json(
        directory
            .parent()
            .ok_or("installation has no staging directory")?,
        "adopted.json",
        &receipt.release,
    )?;
    write_json(
        directory,
        "healthy.json",
        &Health {
            nonce: receipt.nonce,
            build,
        },
    )
}

fn verify_installed(program: &Path, release: &VerifiedRelease) -> Result<(), String> {
    let identity = &release.manifest().identity;
    let offer = release
        .select(
            khor_update::Component::DesktopApp,
            khor_update::Platform::current()?,
            identity,
            identity.tag.channel(),
            Some(&identity.tag),
        )?
        .ok_or("installed release has no application for this platform")?;
    khor_node::updates::verify_program(program, &offer)
}

pub(crate) fn adopted(root: &Path, build: &BuildIdentity) -> Option<khor_update::ReleaseIdentity> {
    let signed = read_json::<SignedRelease>(&root.join("adopted.json")).ok()?;
    let release = TrustRoot::bundled().ok()?.restore(&signed).ok()?;
    (build.release.as_ref() == Some(&release.manifest().build_identity()))
        .then(|| release.manifest().identity.clone())
}

pub(crate) fn automatic_allowed(root: &Path, identity: &khor_update::ReleaseIdentity) -> bool {
    let path = root.join("deferred.json");
    if !path.exists() {
        return true;
    }
    let Some(release) = read_json::<SignedRelease>(&path)
        .ok()
        .and_then(|signed| TrustRoot::bundled().ok()?.restore(&signed).ok())
    else {
        return false;
    };
    release.manifest().identity != *identity
}

fn restore_previous(directory: &Path, receipt: &Receipt) -> Result<(), String> {
    let previous = directory.join("previous.app");
    if khor_node::updates::inspect(&previous.join("Contents/MacOS/khor-gui"))?.build != receipt.old
    {
        return Err("the previous application backup changed".into());
    }
    let parent = receipt
        .target
        .parent()
        .ok_or("application has no installation directory")?;
    if receipt.target.exists()
        && khor_node::updates::inspect(&receipt.target.join("Contents/MacOS/khor-gui"))
            .is_ok_and(|info| info.build == receipt.old)
    {
        return Ok(());
    }
    let staging = tempfile::Builder::new()
        .prefix(".khor-restore-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let restored = staging.path().join("previous.app");
    copy_bundle(&previous, &restored)?;
    let failed = parent.join(format!(".khor-failed-{}", receipt.nonce));
    if receipt.target.exists() {
        std::fs::rename(&receipt.target, &failed).map_err(|e| e.to_string())?;
    }
    if let Err(error) = std::fs::rename(&restored, &receipt.target) {
        if failed.exists() {
            let _ = std::fs::rename(&failed, &receipt.target);
        }
        return Err(error.to_string());
    }
    // Failed bytes stay named and recoverable; collection decides only after occupancy is known.
    Ok(())
}

fn bundle_at(executable: &Path) -> Result<&Path, String> {
    executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .filter(|path| path.extension().is_some_and(|extension| extension == "app"))
        .ok_or_else(|| "this executable is not an installed application bundle".into())
}

fn copy_bundle(from: &Path, to: &Path) -> Result<(), String> {
    let status = Command::new("/usr/bin/ditto")
        .arg(from)
        .arg(to)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("could not retain the previous application bundle".into());
    }
    Ok(())
}

fn bundle_bytes(root: &Path) -> Result<u64, String> {
    let mut bytes = 0u64;
    for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        let size = if entry.file_type().map_err(|e| e.to_string())?.is_symlink() {
            0
        } else if metadata.is_dir() {
            bundle_bytes(&entry.path())?
        } else {
            metadata.len()
        };
        bytes = bytes
            .checked_add(size)
            .ok_or("application backup size overflow")?;
    }
    Ok(bytes)
}

fn validate_directory(directory: &Path) -> Result<(), String> {
    let root = khor_runtime::local_directory()
        .map_err(|e| e.to_string())?
        .join("app-updates");
    if directory.parent() != Some(root.as_path())
        || !directory
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("install-"))
    {
        return Err("application update transaction is outside its local staging directory".into());
    }
    let metadata = std::fs::symlink_metadata(directory).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err("application update transaction is not an owned directory".into());
    }
    Ok(())
}

fn read_receipt(directory: &Path) -> Result<Receipt, String> {
    let receipt: Receipt = read_json(&directory.join("receipt.json"))?;
    if receipt.schema != 1
        || directory.file_name().and_then(|name| name.to_str()) != Some(receipt.nonce.as_str())
    {
        return Err("invalid application update transaction identity".into());
    }
    Ok(receipt)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(384 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 384 * 1024 {
        return Err("application update record exceeds its limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn write_json(root: &Path, name: &str, value: &impl Serialize) -> Result<(), String> {
    let mut file = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(root.join(name)).map_err(|e| e.to_string())?;
    File::open(root)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn update_monitor_has_an_independent_lifetime_group() {
        let root = tempfile::tempdir().unwrap();
        let program = root.path().join("monitor");
        let log = root.path().join("output");
        std::fs::write(&program, b"#!/bin/sh\n/bin/ps -o pgid= -p \"$$\"\n").unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = File::create(&log).unwrap();
        let mut worker = spawn_monitor(&program, root.path(), output).unwrap();
        let pid = worker.id();
        assert!(worker.wait().unwrap().success());
        let group: u32 = std::fs::read_to_string(log)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(
            group, pid,
            "the updater must outlive cleanup of the old App's process group"
        );
    }

    #[test]
    fn relaunched_application_has_an_independent_lifetime_group() {
        let root = tempfile::tempdir().unwrap();
        let program = root.path().join("application");
        std::fs::write(
            &program,
            b"#!/bin/sh\n/bin/ps -o pgid= -p \"$$\" > \"${0%/*}/group\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut application = launch(&program, None).unwrap();
        let pid = application.id();
        assert!(application.wait().unwrap().success());
        let group: u32 = std::fs::read_to_string(root.path().join("group"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(
            group, pid,
            "the restored App must outlive its completed update monitor"
        );
    }

    fn old_bundle(root: &Path) -> BuildIdentity {
        let mut info = khor_node::updates::program_info().unwrap();
        info.component = khor_update::Component::DesktopApp;
        let directory = root.join("Contents/MacOS");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("khor-gui");
        let json = serde_json::to_string(&info).unwrap();
        std::fs::write(
            &path,
            format!("#!/bin/sh\ncat <<'KHOR_UPDATE_INFO'\n{json}\nKHOR_UPDATE_INFO\n"),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::create_dir_all(root.join("Contents/Resources")).unwrap();
        std::fs::write(
            root.join("Contents/Resources/ui.bin"),
            b"previous interface",
        )
        .unwrap();
        info.build
    }
    fn receipt(target: PathBuf, old: BuildIdentity) -> Receipt {
        Receipt {
            schema: 1,
            nonce: "install-fixture".into(),
            target,
            old,
            release: serde_json::from_value(
                serde_json::json!({"document":"not used by backup restoration","signature":""}),
            )
            .unwrap(),
            phase: Phase::Installing,
        }
    }
    #[test]
    fn rollback_restores_the_complete_previous_bundle_and_preserves_failed_bytes_and_neighbours() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("install-fixture");
        std::fs::create_dir(&directory).unwrap();
        let old = old_bundle(&directory.join("previous.app"));
        let target = root.path().join("Khor.app");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("failed-build"), b"failed application").unwrap();
        std::fs::write(root.path().join("neighbour"), b"unrelated user data").unwrap();
        restore_previous(&directory, &receipt(target.clone(), old)).unwrap();
        assert_eq!(
            std::fs::read(target.join("Contents/Resources/ui.bin")).unwrap(),
            b"previous interface"
        );
        assert_eq!(
            std::fs::read(
                root.path()
                    .join(".khor-failed-install-fixture/failed-build")
            )
            .unwrap(),
            b"failed application"
        );
        assert_eq!(
            std::fs::read(root.path().join("neighbour")).unwrap(),
            b"unrelated user data"
        );
        restore_previous(
            &directory,
            &receipt(target.clone(), khor_update::build_identity().unwrap()),
        )
        .unwrap();
        assert_eq!(
            std::fs::read(target.join("Contents/Resources/ui.bin")).unwrap(),
            b"previous interface"
        );
    }
    #[test]
    fn a_mismatched_previous_program_cannot_overwrite_the_current_application() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("install-fixture");
        std::fs::create_dir(&directory).unwrap();
        let mut old = old_bundle(&directory.join("previous.app"));
        old.version = "different build".into();
        let target = root.path().join("Khor.app");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("keep"), b"current application").unwrap();
        assert!(restore_previous(&directory, &receipt(target.clone(), old)).is_err());
        assert_eq!(
            std::fs::read(target.join("keep")).unwrap(),
            b"current application"
        );
    }

    #[test]
    fn unknown_failed_update_metadata_cannot_enable_an_automatic_retry_loop() {
        let root = tempfile::tempdir().unwrap();
        let identity = khor_update::ReleaseIdentity {
            tag: "v0.2.0".to_owned().try_into().unwrap(),
            source: "a".repeat(40).try_into().unwrap(),
            build: 1,
        };
        assert!(automatic_allowed(root.path(), &identity));
        std::fs::write(
            root.path().join("deferred.json"),
            b"unverified or interrupted failure record",
        )
        .unwrap();
        assert!(!automatic_allowed(root.path(), &identity));
    }
}
