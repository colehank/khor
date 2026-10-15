//! A bounded directory snapshot and one atomic directory landing.
//! Paths on the wire are relative with `/` separators. Links and special
//! files are refused explicitly; a traversal never follows them elsewhere.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use khor_catalog::msg;
use serde::{Deserialize, Serialize};

pub const MAX_ENTRIES: usize = 20_000;
const MAX_PATH_BYTES: usize = 2 * 1024 * 1024;
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub path: String,
    pub dir: bool,
    pub size: u64,
    pub at_ms: u64,
    pub mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub entries: Vec<Entry>,
    pub total: u64,
}

/// The same spelling is checked for local web uploads and peer replies.
pub fn relative(path: &str) -> Result<PathBuf, String> {
    if path.is_empty()
        || path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(msg::tree_bad_path(path));
    }
    if path.split('/').count() > MAX_DEPTH {
        return Err(msg::tree_too_many(MAX_ENTRIES));
    }
    Ok(path.split('/').collect())
}

impl Manifest {
    pub fn checked(mut entries: Vec<Entry>) -> Result<Self, String> {
        if entries.len() > MAX_ENTRIES {
            return Err(msg::tree_too_many(MAX_ENTRIES));
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let mut seen = HashMap::new();
        let mut total = 0u64;
        let mut spelling = 0usize;
        for e in &entries {
            relative(&e.path)?;
            spelling = spelling.saturating_add(e.path.len());
            if spelling > MAX_PATH_BYTES || seen.insert(e.path.clone(), e.dir).is_some() {
                return Err(msg::tree_bad_path(&e.path));
            }
            if e.dir && e.size != 0 {
                return Err(msg::tree_bad_path(&e.path));
            }
            if let Some((parent, _)) = e.path.rsplit_once('/')
                && seen.get(parent) != Some(&true)
            {
                return Err(msg::tree_bad_path(&e.path));
            }
            total = total
                .checked_add(e.size)
                .ok_or_else(|| msg::tree_too_big(crate::drops::CAP >> 20))?;
            if total > crate::drops::CAP {
                return Err(msg::tree_too_big(crate::drops::CAP >> 20));
            }
        }
        Ok(Self { entries, total })
    }

    pub fn validate(&self) -> Result<(), String> {
        if Self::checked(self.entries.clone())? != *self {
            return Err(msg::tree_changed().into());
        }
        Ok(())
    }
}

pub fn scan(path: &Path) -> Result<Option<Manifest>, String> {
    if !path.is_absolute() {
        return Err(msg::path_not_absolute(path.display()));
    }
    let root = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if root.is_file() || (root.is_symlink() && std::fs::metadata(path).is_ok_and(|m| m.is_file())) {
        return Ok(None);
    }
    if !root.is_dir() {
        return Err(msg::tree_entry_unsupported(path.display()));
    }
    let mut todo = vec![PathBuf::new()];
    let mut entries = Vec::new();
    let mut total = 0u64;
    let mut spelling = 0usize;
    while let Some(dir) = todo.pop() {
        for entry in std::fs::read_dir(path.join(dir)).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let full = entry.path();
            let relative_path = full.strip_prefix(path).map_err(|e| e.to_string())?;
            let name = relative_path
                .components()
                .map(|c| {
                    c.as_os_str()
                        .to_str()
                        .ok_or_else(|| msg::tree_bad_path(full.display()))
                })
                .collect::<Result<Vec<_>, _>>()?
                .join("/");
            relative(&name)?;
            spelling = spelling.saturating_add(name.len());
            if entries.len() >= MAX_ENTRIES || spelling > MAX_PATH_BYTES {
                return Err(msg::tree_too_many(MAX_ENTRIES));
            }
            let meta = std::fs::symlink_metadata(&full).map_err(|e| e.to_string())?;
            if !meta.is_file() && !meta.is_dir() {
                return Err(msg::tree_entry_unsupported(full.display()));
            }
            let size = if meta.is_dir() { 0 } else { meta.len() };
            total = total.saturating_add(size);
            if total > crate::drops::CAP {
                return Err(msg::tree_too_big(crate::drops::CAP >> 20));
            }
            let at_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|t| t.as_millis() as u64)
                .unwrap_or(0);
            #[cfg(unix)]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode() & 0o777
            };
            #[cfg(not(unix))]
            let mode = 0;
            if meta.is_dir() {
                todo.push(relative_path.to_path_buf());
            }
            entries.push(Entry {
                path: name,
                dir: meta.is_dir(),
                size,
                at_ms,
                mode,
            });
        }
    }
    Manifest::checked(entries).map(Some)
}

/// Owns only a hidden, newly-created directory. Failure or cancellation
/// removes it; nothing about an existing destination is ever removed.
pub struct Landing {
    pub part: PathBuf,
    dest: PathBuf,
    published: bool,
}

impl Landing {
    pub fn new(dir: &Path, name: &str) -> Result<Self, String> {
        if relative(name)?.components().count() != 1 {
            return Err(msg::tree_bad_path(name));
        }
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        for n in 1..10_000 {
            let name = if n == 1 {
                name.to_owned()
            } else {
                format!("{name}-{n}")
            };
            let dest = dir.join(&name);
            if dest.symlink_metadata().is_ok() {
                continue;
            }
            let part = dir.join(format!(".khor-pull-{name}"));
            match std::fs::create_dir(&part) {
                Ok(()) => {
                    return Ok(Self {
                        part,
                        dest,
                        published: false,
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        Err(msg::dest_exists(dir.join(name).display()))
    }

    pub fn publish(mut self) -> Result<PathBuf, String> {
        // Existing empty directories must not be replaced by rename either.
        if self.dest.symlink_metadata().is_ok() {
            return Err(msg::dest_exists(self.dest.display()));
        }
        rename_new(&self.part, &self.dest)?;
        self.published = true;
        Ok(self.dest.clone())
    }
}

impl Drop for Landing {
    fn drop(&mut self) {
        if !self.published {
            let _ = std::fs::remove_dir_all(&self.part);
        }
    }
}

fn rename_new(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "linux"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let from =
            std::ffi::CString::new(from.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        let to = std::ffi::CString::new(to.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        // SAFETY: valid, NUL-terminated paths; these calls refuse replacement.
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let result = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                from.as_ptr(),
                libc::AT_FDCWD,
                to.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        return if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().to_string())
        };
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "linux")))]
    std::fs::rename(from, to).map_err(|e| e.to_string())
}

pub fn permissions(root: &Path, manifest: &Manifest) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in manifest.entries.iter().rev() {
            if !entry.dir && entry.mode != 0 {
                std::fs::set_permissions(
                    root.join(relative(&entry.path)?),
                    std::fs::Permissions::from_mode(entry.mode & 0o777),
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }
    #[cfg(not(unix))]
    let _ = (root, manifest);
    Ok(())
}

pub fn copy_into(source: &Path, dir: &Path, job: &crate::drops::Job) -> Result<PathBuf, String> {
    let canonical = source.canonicalize().map_err(|e| e.to_string())?;
    if dir.canonicalize().is_ok_and(|d| d.starts_with(&canonical)) {
        return Err(msg::tree_copy_inside().into());
    }
    let manifest = scan(source)?.ok_or_else(|| msg::not_a_file_path(source.display()))?;
    let name = crate::drops::name_of(&source.display().to_string())?;
    let at = Landing::new(dir, &name)?;
    job.total.store(manifest.total, Ordering::Relaxed);
    for entry in &manifest.entries {
        if job.is_stopped() {
            return Err(crate::drops::cancelled());
        }
        let rel = relative(&entry.path)?;
        let dest = at.part.join(&rel);
        if entry.dir {
            std::fs::create_dir(&dest).map_err(|e| e.to_string())?;
            continue;
        }
        let mut input = std::fs::File::open(source.join(&rel)).map_err(|e| e.to_string())?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dest)
            .map_err(|e| e.to_string())?;
        let mut moved = 0u64;
        let mut buf = [0u8; 64 * 1024];
        loop {
            if job.is_stopped() {
                return Err(crate::drops::cancelled());
            }
            let n = input.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            moved += n as u64;
            if moved > entry.size {
                return Err(msg::tree_changed().into());
            }
            output.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            job.moved.fetch_add(n as u64, Ordering::Relaxed);
        }
        if moved != entry.size {
            return Err(msg::tree_changed().into());
        }
    }
    if job.is_stopped() {
        return Err(crate::drops::cancelled());
    }
    if scan(source)?.as_ref() != Some(&manifest) {
        return Err(msg::tree_changed().into());
    }
    permissions(&at.part, &manifest)?;
    at.publish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peer_paths_cannot_escape_or_alias_the_tree() {
        for bad in ["", "/a", "../a", "a/../b", "a//b", "a\\b", "C:/a", "a/."] {
            assert!(relative(bad).is_err(), "{bad}");
        }
        let entry = |path: &str, dir| Entry {
            path: path.into(),
            dir,
            size: 0,
            at_ms: 0,
            mode: 0,
        };
        assert!(Manifest::checked(vec![entry("a/b", false)]).is_err());
        assert!(Manifest::checked(vec![entry("a", false), entry("a/b", false)]).is_err());
        assert!(Manifest::checked(vec![entry("a", true), entry("a", true)]).is_err());
    }
    #[test]
    fn an_empty_tree_is_a_directory_and_cancelled_copy_cleans_its_part() {
        let root = crate::testdir::TempTree::fresh("tree-empty-cancel");
        let src = root.join("empty");
        std::fs::create_dir(&src).unwrap();
        assert_eq!(scan(&src).unwrap().unwrap().total, 0);
        let dest = root.join("receive");
        let job = crate::drops::Job::default();
        job.stopped.store(true, Ordering::Relaxed);
        assert!(copy_into(&src, &dest, &job).is_err());
        // Empty directories also honour cancellation before publishing.
        assert_eq!(std::fs::read_dir(dest).unwrap().count(), 0);
    }
    #[test]
    fn a_landing_never_overwrites_a_destination_created_while_copying() {
        let root = crate::testdir::TempTree::fresh("tree-no-overwrite");
        let at = Landing::new(&root, "data").unwrap();
        let part = at.part.clone();
        std::fs::create_dir(root.join("data")).unwrap();
        assert!(at.publish().is_err());
        assert!(root.join("data").is_dir());
        assert!(!part.exists());
    }
    #[cfg(unix)]
    #[test]
    fn a_tree_refuses_links_instead_of_following_them() {
        let root = crate::testdir::TempTree::fresh("tree-links");
        std::os::unix::fs::symlink("/", root.join("outside")).unwrap();
        assert!(scan(&root).is_err());
    }
}
