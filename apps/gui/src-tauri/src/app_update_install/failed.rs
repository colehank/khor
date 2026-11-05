use super::*;
use std::{fs, os::unix::fs::MetadataExt};

#[derive(Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileIdentity {
    device: u64,
    inode: u64,
    bytes: u64,
    modified: i64,
    modified_nanos: i64,
}

impl FileIdentity {
    fn at(path: &Path) -> Result<Self, String> {
        let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if metadata.is_symlink() {
            return Err("failed application identity is a link".into());
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            bytes: metadata.len(),
            modified: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Claim {
    schema: u8,
    nonce: String,
    directory: FileIdentity,
    program: FileIdentity,
}

fn path(receipt: &Receipt) -> Result<PathBuf, String> {
    Ok(receipt
        .target
        .parent()
        .ok_or("application has no installation directory")?
        .join(format!(".khor-failed-{}", receipt.nonce)))
}

pub(super) fn remember(directory: &Path, receipt: &Receipt) -> Result<(), String> {
    let path = path(receipt)?;
    let claim = Claim {
        schema: 1,
        nonce: receipt.nonce.clone(),
        directory: FileIdentity::at(&path)?,
        program: FileIdentity::at(&path.join("Contents/MacOS/khor-gui"))?,
    };
    write_json(directory, "failed.json", &claim)
}

fn matches(directory: &Path, receipt: &Receipt) -> Result<PathBuf, String> {
    let claim_path = directory.join("failed.json");
    let metadata = fs::symlink_metadata(&claim_path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.is_symlink() || metadata.nlink() != 1 {
        return Err("failed application claim is not a regular file".into());
    }
    let claim: Claim = read_json(&claim_path)?;
    let path = path(receipt)?;
    if claim.schema != 1
        || claim.nonce != receipt.nonce
        || !fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_dir()
        || claim.directory != FileIdentity::at(&path)?
        || claim.program != FileIdentity::at(&path.join("Contents/MacOS/khor-gui"))?
    {
        return Err("failed application no longer matches its collection claim".into());
    }
    Ok(path)
}

pub(super) fn collect(directory: &Path, receipt: &Receipt) -> Result<u64, String> {
    let path = matches(directory, receipt)?;
    if !unoccupied(&path)? {
        return Ok(0);
    }
    matches(directory, receipt)?;
    let bytes = bundle_bytes(&path)?;
    fs::remove_dir_all(path).map_err(|e| e.to_string())?;
    fs::remove_file(directory.join("failed.json")).map_err(|e| e.to_string())?;
    Ok(bytes)
}

pub(super) fn pending(directory: &Path, receipt: &Receipt) -> bool {
    directory.join("failed.json").exists()
        || path(receipt).is_ok_and(|p| p.symlink_metadata().is_ok())
}

fn unoccupied(path: &Path) -> Result<bool, String> {
    let mut child = Command::new("/usr/sbin/lsof")
        .arg("-t")
        .arg("+D")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut occupied = Vec::new();
                if let Some(stdout) = child.stdout.take() {
                    stdout
                        .take(1)
                        .read_to_end(&mut occupied)
                        .map_err(|e| e.to_string())?;
                }
                let mut errors = Vec::new();
                if let Some(stderr) = child.stderr.take() {
                    stderr
                        .take(4097)
                        .read_to_end(&mut errors)
                        .map_err(|e| e.to_string())?;
                }
                return Ok(status.code() == Some(1) && occupied.is_empty() && errors.is_empty());
            }
            Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(false);
            }
        }
    }
}
