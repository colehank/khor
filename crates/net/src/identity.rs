//! The machine's identity key: load and persist.
//!
//! This key is the credential other devices' pairing records trust — the
//! public key is the machine id(docs/NET.md). Two
//! consequences: a leak lets anyone impersonate this machine, so the
//! file must be 0600 and a looser mode refuses to start; regenerating
//! makes this a *different* machine — every paired device loses it, and
//! all they see is "connection timed out".

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use khor_catalog::msg;

/// Loads the identity, or publishes a complete private key without replacing
/// another process's identity. All concurrent creators load the same winner.
pub fn load_or_create(path: &Path) -> Result<iroh::SecretKey> {
    match File::open(path) {
        Ok(file) => return read_private(file, path),
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(err).context(msg::identity_unreadable()),
    }

    let key = iroh::SecretKey::generate();
    let pending = prepare_private(path, &key.to_bytes())?;
    pending.publish(path)?;
    // Even a losing creator must use the installed identity, never its own
    // candidate. Do not retry creation if someone removes the winner here.
    read_private(File::open(path).context(msg::identity_unreadable())?, path)
}

fn read_private(file: File, path: &Path) -> Result<iroh::SecretKey> {
    check_private_permissions(&file, path)?;
    // A corrupt file must not cause an unbounded allocation, or be replaced.
    let mut raw = Vec::with_capacity(33);
    file.take(33)
        .read_to_end(&mut raw)
        .context(msg::identity_unreadable())?;
    let bytes: [u8; 32] = raw
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!(msg::identity_corrupt(path.display())))?;
    Ok(iroh::SecretKey::from_bytes(&bytes))
}

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// A complete candidate in the destination directory. Dropping it only removes
/// the temporary name; a published hard link remains intact.
struct PendingIdentity {
    path: PathBuf,
}

impl PendingIdentity {
    fn publish(&self, path: &Path) -> Result<()> {
        match std::fs::hard_link(&self.path, path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(()),
            Err(err) => Err(err).with_context(|| msg::cant_write_file(path.display())),
        }
        // No rename fallback: it could overwrite the winner. This applies on
        // Windows too; filesystems without hard links fail closed. Windows
        // access control inherits the parent directory's ACL, not Unix modes.
    }
}

impl Drop for PendingIdentity {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn prepare_private(path: &Path, data: &[u8; 32]) -> Result<PendingIdentity> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir).with_context(|| msg::cant_make_dir_for(dir.display()))?;
    let (pending, mut file) = loop {
        let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temp = dir.join(format!(
            ".khor-identity-{}-{serial}.tmp",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // Apply the private mode in open(), before any bytes exist.
            options.mode(0o600);
        }
        match options.open(&temp) {
            Ok(file) => break (PendingIdentity { path: temp }, file),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err).with_context(|| msg::cant_write_file(path.display())),
        }
    };
    // Sync the content before it becomes visible. This does not promise that
    // the new directory entry survives power loss (the directory is not synced).
    let written = file.write_all(data).and_then(|()| file.sync_all());
    // Close before cleanup or publication, including on Windows.
    drop(file);
    written.with_context(|| msg::cant_write_file(path.display()))?;
    Ok(pending)
}

/// Validate the opened inode, so permission checks and reads refer to the same
/// key even if the path changes. Loose keys are refused, never silently repaired.
fn check_private_permissions(file: &File, path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = file.metadata()?.permissions().mode() & 0o777;
        anyhow::ensure!(
            mode & 0o077 == 0,
            msg::identity_too_open(format_args!("{mode:o}"), path.display())
        );
    }
    #[cfg(not(unix))]
    let _ = (file, path);
    Ok(())
}

#[cfg(test)]
#[path = "identity/tests.rs"]
mod tests;
