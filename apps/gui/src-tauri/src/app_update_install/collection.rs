use super::*;

const RECORD_FILES: &[&str] = &[
    "receipt.json",
    "parent.lock",
    "worker.lock",
    "worker.ready",
    "worker.log",
    "healthy.json",
    "live.lock",
    "failed.json",
];

struct Backup {
    directory: PathBuf,
    receipt: Receipt,
    has_previous: bool,
    modified: std::time::SystemTime,
    _leases: Vec<File>,
}

pub(crate) fn collect(root: &Path, trust: &TrustRoot) -> Result<u64, String> {
    let program = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = std::fs::canonicalize(bundle_at(&program)?).map_err(|e| e.to_string())?;
    collect_for(root, &target, trust)
}

fn collect_for(root: &Path, target: &Path, trust: &TrustRoot) -> Result<u64, String> {
    let mut backups = Vec::new();
    for entry in std::fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .take(256)
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_name().to_string_lossy().starts_with("install-") {
            continue;
        }
        if let Ok(backup) = admit(&entry.path(), target, trust) {
            backups.push(backup);
        }
    }
    backups.sort_by_key(|backup| std::cmp::Reverse(backup.modified));
    let mut retained = false;
    let mut history = 0;
    let mut bytes = 0u64;
    for backup in backups {
        if backup.receipt.phase == Phase::Complete && backup.has_previous && !retained {
            retained = true;
            continue;
        }
        if backup.has_previous {
            let path = backup.directory.join("previous.app");
            let size = bundle_bytes(&path)?;
            std::fs::remove_dir_all(path).map_err(|e| e.to_string())?;
            bytes = bytes.saturating_add(size);
        }
        if matches!(backup.receipt.phase, Phase::Cancelled | Phase::RolledBack) {
            if let Ok(reclaimed) = failed::collect(&backup.directory, &backup.receipt) {
                bytes = bytes.saturating_add(reclaimed);
            }
        }
        File::open(&backup.directory)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        if !failed::pending(&backup.directory, &backup.receipt) {
            if history < 16 {
                history += 1;
            } else {
                retire(&backup.directory)?;
            }
        }
    }
    Ok(bytes)
}

fn admit(directory: &Path, target: &Path, trust: &TrustRoot) -> Result<Backup, String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(directory).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.is_symlink() || metadata.mode() & 0o077 != 0 {
        return Err("application backup is not private".into());
    }
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name() == "previous.app" {
            if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                return Err("application backup is not a directory".into());
            }
        } else if !RECORD_FILES.iter().any(|name| entry.file_name() == *name) {
            return Err("application update contains unknown files".into());
        } else {
            regular_file(&entry.path())?;
        }
    }
    regular_file(&directory.join("receipt.json"))?;
    let receipt = read_receipt(directory)?;
    if receipt.target != target
        || !matches!(
            receipt.phase,
            Phase::Preparing | Phase::Cancelled | Phase::Complete | Phase::RolledBack
        )
    {
        return Err(
            "application backup is not an inactive transaction for this installation".into(),
        );
    }
    trust.restore(&receipt.release)?;
    let mut leases = vec![vacant(&directory.join("parent.lock"))?];
    match std::fs::symlink_metadata(directory.join("worker.lock")) {
        Ok(_) => leases.push(vacant(&directory.join("worker.lock"))?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && receipt.phase == Phase::Preparing => {
        }
        Err(e) => return Err(e.to_string()),
    }
    match std::fs::symlink_metadata(directory.join("live.lock")) {
        Ok(_) => leases.push(vacant(&directory.join("live.lock"))?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    let has_previous = match std::fs::symlink_metadata(directory.join("previous.app")) {
        Ok(metadata) if metadata.is_dir() && !metadata.is_symlink() => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        _ => return Err("application backup is not an owned directory".into()),
    };
    Ok(Backup {
        directory: directory.to_path_buf(),
        receipt,
        has_previous,
        modified: std::fs::metadata(directory.join("receipt.json"))
            .and_then(|m| m.modified())
            .map_err(|e| e.to_string())?,
        _leases: leases,
    })
}

fn retire(directory: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !RECORD_FILES.iter().any(|name| entry.file_name() == *name) {
            return Err("application update contains unknown files".into());
        }
        regular_file(&entry.path())?;
        files.push(entry.path());
    }
    for path in files {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    std::fs::remove_dir(directory).map_err(|e| e.to_string())?;
    Ok(())
}

fn regular_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.is_symlink() || metadata.nlink() != 1 {
        return Err("application update contains an unexpected link or directory".into());
    }
    Ok(())
}

fn vacant(path: &Path) -> Result<File, String> {
    regular_file(path)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.try_lock()
        .map_err(|_| "application update is still occupied")?;
    Ok(file)
}

pub(super) fn live(directory: &Path) -> Result<File, String> {
    let path = directory.join("live.lock");
    match std::fs::symlink_metadata(&path) {
        Ok(_) => regular_file(&path)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.try_lock_shared()
        .map_err(|_| "application update metadata is being collected")?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, symlink};

    fn trust() -> TrustRoot {
        TrustRoot::from_tauri_base64(include_str!(
            "../../../../../crates/update/tests/fixtures/public-key.txt"
        ))
        .unwrap()
    }

    fn backup(root: &Path, name: &str, phase: Phase, modified: u64) -> PathBuf {
        let path = root.join(name);
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        std::fs::create_dir(path.join("previous.app")).unwrap();
        std::fs::write(path.join("previous.app/payload"), b"backup").unwrap();
        let receipt = Receipt {
            schema: 1,
            nonce: name.into(),
            target: root.join("Khor.app"),
            old: khor_update::build_identity().unwrap(),
            phase,
            release: trust()
                .authenticate(
                    include_bytes!("../../../../../crates/update/tests/fixtures/release.json"),
                    include_str!("../../../../../crates/update/tests/fixtures/release.sig"),
                )
                .unwrap()
                .signed()
                .clone(),
        };
        write_json(&path, "receipt.json", &receipt).unwrap();
        File::open(path.join("receipt.json"))
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(modified)),
            )
            .unwrap();
        File::create(path.join("parent.lock")).unwrap();
        File::create(path.join("worker.lock")).unwrap();
        path
    }

    #[test]
    fn retention_keeps_one_completed_backup_and_all_occupied_or_unfinished_transactions() {
        let root = tempfile::tempdir().unwrap();
        let old = backup(root.path(), "install-old", Phase::Complete, 1);
        let latest = backup(root.path(), "install-latest", Phase::Complete, 2);
        let busy = backup(root.path(), "install-busy", Phase::Complete, 3);
        let parent = vacant(&busy.join("parent.lock")).unwrap();
        let worker = vacant(&busy.join("worker.lock")).unwrap();
        let unfinished = backup(root.path(), "install-unfinished", Phase::Installing, 4);
        assert_eq!(
            collect_for(root.path(), &root.path().join("Khor.app"), &trust()).unwrap(),
            6
        );
        assert!(!old.join("previous.app").exists());
        for path in [&latest, &busy, &unfinished] {
            assert!(path.join("previous.app/payload").exists());
        }
        drop(parent);
        assert_eq!(
            collect_for(root.path(), &root.path().join("Khor.app"), &trust()).unwrap(),
            0
        );
        drop(worker);
        assert_eq!(
            collect_for(root.path(), &root.path().join("Khor.app"), &trust()).unwrap(),
            6
        );
        assert!(busy.join("previous.app/payload").exists());
        assert!(!latest.join("previous.app").exists());
        assert!(old.join("receipt.json").is_file());
    }

    #[test]
    fn interrupted_preparation_is_collectible_but_unknown_linked_and_foreign_backups_are_preserved()
    {
        let root = tempfile::tempdir().unwrap();
        let interrupted = backup(root.path(), "install-interrupted", Phase::Preparing, 1);
        std::fs::remove_file(interrupted.join("worker.lock")).unwrap();
        let unknown = backup(root.path(), "install-unknown", Phase::Cancelled, 2);
        std::fs::write(unknown.join("personal-file"), b"keep").unwrap();
        let foreign = backup(root.path(), "install-foreign", Phase::Cancelled, 3);
        let mut receipt = read_receipt(&foreign).unwrap();
        receipt.target = root.path().join("Another.app");
        write_json(&foreign, "receipt.json", &receipt).unwrap();
        let linked = backup(root.path(), "install-linked", Phase::Cancelled, 4);
        std::fs::remove_dir_all(linked.join("previous.app")).unwrap();
        symlink(unknown.join("previous.app"), linked.join("previous.app")).unwrap();
        let invalid = backup(root.path(), "install-invalid", Phase::Cancelled, 5);
        std::fs::write(invalid.join("receipt.json"), b"not authenticated").unwrap();
        assert_eq!(
            collect_for(root.path(), &root.path().join("Khor.app"), &trust()).unwrap(),
            6
        );
        assert!(!interrupted.join("previous.app").exists());
        for path in [&unknown, &foreign, &linked, &invalid] {
            assert!(path.join("previous.app/payload").exists());
        }
    }

    #[test]
    fn a_running_updated_app_keeps_its_backup_and_terminal_receipts_have_a_bounded_history() {
        let root = tempfile::tempdir().unwrap();
        let running = backup(root.path(), "install-running", Phase::Complete, 1);
        let lease = live(&running).unwrap();
        for n in 2..24 {
            backup(
                root.path(),
                &format!("install-old-{n}"),
                Phase::Cancelled,
                n,
            );
        }
        collect_for(root.path(), &root.path().join("Khor.app"), &trust()).unwrap();
        assert!(running.join("previous.app/payload").exists());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 17);
        drop(lease);
    }

    #[test]
    fn quarantined_app_collection_checks_provenance_and_actual_open_files() {
        let root = tempfile::tempdir().unwrap();
        let directory = backup(root.path(), "install-failed", Phase::RolledBack, 1);
        let receipt = read_receipt(&directory).unwrap();
        let failed_path = root.path().join(".khor-failed-install-failed");
        let program = failed_path.join("Contents/MacOS/khor-gui");
        std::fs::create_dir_all(program.parent().unwrap()).unwrap();
        std::fs::write(&program, b"failed").unwrap();
        failed::remember(&directory, &receipt).unwrap();
        let open = File::open(&program).unwrap();
        assert_eq!(
            collect_for(root.path(), &receipt.target, &trust()).unwrap(),
            6
        );
        assert!(program.exists());
        drop(open);
        assert_eq!(
            collect_for(root.path(), &receipt.target, &trust()).unwrap(),
            6
        );
        assert!(!failed_path.exists());

        std::fs::create_dir_all(program.parent().unwrap()).unwrap();
        std::fs::write(&program, b"old failed").unwrap();
        failed::remember(&directory, &receipt).unwrap();
        let replacement = root.path().join("new-file");
        std::fs::write(&replacement, b"unrelated replacement").unwrap();
        std::fs::rename(replacement, &program).unwrap();
        assert_eq!(
            collect_for(root.path(), &receipt.target, &trust()).unwrap(),
            0
        );
        assert_eq!(std::fs::read(program).unwrap(), b"unrelated replacement");
    }
}
