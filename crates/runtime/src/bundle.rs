//! A desktop host retains the signed app, including its sealed Info.plist and resources.
//! Only regular files/directories belong to this payload; links and extra node types refuse.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub(crate) const NAME: &str = "Khor.app";
pub(crate) const PROGRAM: &str = "Khor.app/Contents/MacOS/khor-gui";

struct Entry {
    relative: PathBuf,
    directory: bool,
    executable: bool,
}

fn entries(root: &Path) -> io::Result<Vec<Entry>> {
    fn walk(root: &Path, relative: &Path, depth: usize, output: &mut Vec<Entry>) -> io::Result<()> {
        if depth > 32 || output.len() >= 4096 {
            return Err(io::Error::other(
                "runtime app bundle exceeds its entry limit",
            ));
        }
        let metadata = fs::symlink_metadata(root.join(relative))?;
        if metadata.is_symlink() || !(metadata.is_dir() || metadata.is_file()) {
            return Err(io::Error::other(
                "runtime app bundle contains a link or special file",
            ));
        }
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o111 != 0
        };
        #[cfg(not(unix))]
        let executable = false;
        if !relative.as_os_str().is_empty() {
            output.push(Entry {
                relative: relative.into(),
                directory: metadata.is_dir(),
                executable,
            });
        } else if !metadata.is_dir() {
            return Err(io::Error::other("runtime app bundle is not a directory"));
        }
        if metadata.is_dir() {
            let mut children =
                fs::read_dir(root.join(relative))?.collect::<io::Result<Vec<_>>>()?;
            children.sort_by_key(|entry| entry.file_name());
            for entry in children {
                walk(root, &relative.join(entry.file_name()), depth + 1, output)?;
            }
        }
        Ok(())
    }
    let mut output = Vec::new();
    walk(root, Path::new(""), 0, &mut output)?;
    Ok(output)
}

pub(crate) fn hash(root: &Path) -> io::Result<(String, u64)> {
    let entries = entries(root)?;
    let mut hash = blake3::Hasher::new();
    let mut bytes = 0u64;
    for entry in entries {
        let path = entry
            .relative
            .to_str()
            .ok_or_else(|| io::Error::other("runtime bundle path is not UTF-8"))?;
        hash.update(&(path.len() as u64).to_le_bytes());
        hash.update(path.as_bytes());
        hash.update(&[
            u8::from(entry.directory),
            u8::from(!entry.directory && entry.executable),
        ]);
        if !entry.directory {
            let (digest, size) = super::store::hash_file(&root.join(&entry.relative))?;
            hash.update(&size.to_le_bytes());
            hash.update(digest.as_bytes());
            bytes = bytes
                .checked_add(size)
                .ok_or_else(|| io::Error::other("runtime bundle size overflow"))?;
        }
    }
    Ok((hash.finalize().to_hex().to_string(), bytes))
}

pub(crate) fn bytes(root: &Path) -> io::Result<u64> {
    entries(root)?
        .iter()
        .filter(|entry| !entry.directory)
        .try_fold(0u64, |total, entry| {
            total
                .checked_add(fs::metadata(root.join(&entry.relative))?.len())
                .ok_or_else(|| io::Error::other("runtime bundle size overflow"))
        })
}

pub(crate) fn copy(source: &Path, destination: &Path) -> io::Result<()> {
    let entries = entries(source)?;
    fs::create_dir(destination)?;
    for entry in &entries {
        let path = destination.join(&entry.relative);
        if entry.directory {
            fs::create_dir(&path)?;
        } else {
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            io::copy(&mut File::open(source.join(&entry.relative))?, &mut output)?;
            output.flush()?;
            output.sync_all()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    &path,
                    fs::Permissions::from_mode(if entry.executable { 0o500 } else { 0o400 }),
                )?;
            }
        }
    }
    #[cfg(unix)]
    for entry in entries.iter().rev().filter(|entry| entry.directory) {
        File::open(destination.join(&entry.relative))?.sync_all()?;
    }
    #[cfg(unix)]
    File::open(destination)?.sync_all()?;
    Ok(())
}

/// Called only for an owned publication/tombstone after its top-level schema check.
pub(crate) fn remove(root: &Path) -> io::Result<()> {
    if !root.try_exists()? {
        return Ok(());
    }
    entries(root)?;
    fs::remove_dir_all(root)
}
