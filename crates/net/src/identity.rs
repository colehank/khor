//! The machine's identity key: load and persist.
//!
//! This key is the credential other devices' pairing records trust — the
//! public key is the machine id (docs/NET.md 身份与配对). Two
//! consequences: a leak lets anyone impersonate this machine, so the
//! file must be 0600 and a looser mode refuses to start; regenerating
//! makes this a *different* machine — every paired device loses it, and
//! all they see is "connection timed out".

use std::path::Path;

use anyhow::{Context, Result};

/// Loads the identity, or generates one and writes it 0600.
pub fn load_or_create(path: &Path) -> Result<iroh::SecretKey> {
    if path.exists() {
        check_private_permissions(path)?;
        let raw = std::fs::read(path).context("读身份文件失败")?;
        let bytes: [u8; 32] = raw.as_slice().try_into().map_err(|_| {
            anyhow::anyhow!(
                "身份文件损坏(应为 32 字节),删掉它会重新生成——\n  \
                 注意重新生成等于换一台机器,已配对的设备需要重新配对:{}",
                path.display()
            )
        })?;
        return Ok(iroh::SecretKey::from_bytes(&bytes));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("创建目录失败: {}", dir.display()))?;
    }
    let key = iroh::SecretKey::generate();
    write_private(path, &key.to_bytes())?;
    Ok(key)
}

/// Writes a file readable by the owner only.
fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    std::fs::write(path, data).with_context(|| format!("写入失败: {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .context("设置文件权限失败")?;
    }
    Ok(())
}

/// A loose mode refuses to start: silently using a group- or
/// world-readable private key is far more dangerous than failing.
fn check_private_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)?.permissions().mode() & 0o777;
        anyhow::ensure!(
            mode & 0o077 == 0,
            "身份文件权限过于开放({:o}),别人能读走它冒充这台机器。\n  修:chmod 600 {}",
            mode,
            path.display()
        );
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reason this file exists: the identity must survive a restart,
    /// or every paired device goes dark.
    #[test]
    fn the_same_identity_comes_back_on_the_second_call() {
        let dir = std::env::temp_dir().join(format!("khor-id-{}", std::process::id()));
        let path = dir.join("identity.key");
        let _ = std::fs::remove_dir_all(&dir);
        let a = load_or_create(&path).unwrap();
        let b = load_or_create(&path).unwrap();
        assert_eq!(a.public(), b.public());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_world_readable_key_is_refused_rather_than_used() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("khor-id-perm-{}", std::process::id()));
        let path = dir.join("identity.key");
        let _ = std::fs::remove_dir_all(&dir);
        load_or_create(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let err = load_or_create(&path).unwrap_err().to_string();
        assert!(err.contains("权限过于开放"), "实际:{err}");
        // The error must say how to fix it, not just "no".
        assert!(err.contains("chmod 600"), "实际:{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
