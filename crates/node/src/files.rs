//! The files landing's serving half: what one paired machine shows
//! another of its disk. Listing only — the payload path is the
//! transfer kind's, and what 待批 guards there is bytes, not looking
//! (docs/NET.md 入网即全信).
//!
//! # The path contract
//!
//! Empty means the user's home directory — the browse has to start
//! somewhere, and home is where the things a person goes looking for
//! live. Anything else must be absolute: a relative path would be
//! relative to wherever the serve process happened to start, an answer
//! about a directory nobody named.
//!
//! # What a row says
//!
//! `metadata()` follows symlinks on purpose: browsing wants "can I
//! step in", not "how is it wired". A link whose target is gone still
//! gets a row (size 0, not a directory) — dropping it would make a
//! visible name unexplainable. Entries the walk cannot stat at all
//! keep the same shape for the same reason.

use khor_catalog::msg;

use crate::proto::DirEntry;

/// Cap per answer. A directory bigger than this answers its first cap
/// in order plus `truncated: true` — the no-silent-caps rule; the cap
/// itself exists because one answer is one frame.
pub const MOST_ENTRIES: usize = 4096;

/// The listing behind [`crate::proto::Request::Ls`]. Ordered here —
/// directories first, each half by case-folded name — so every face
/// paints the same order and none re-sorts.
pub fn list_dir(path: &str) -> Result<(Vec<DirEntry>, bool), String> {
    let dir = if path.is_empty() {
        std::env::home_dir().ok_or(msg::NO_HOME_DIR)?
    } else if std::path::Path::new(path).is_absolute() {
        std::path::PathBuf::from(path)
    } else {
        return Err(msg::path_not_absolute(path));
    };
    let rd = std::fs::read_dir(&dir).map_err(|e| msg::cant_list_dir(&dir.display(), e))?;
    let mut entries: Vec<DirEntry> = Vec::new();
    for e in rd.flatten() {
        let Some(name) = e.file_name().to_str().map(String::from) else {
            // A name that is not UTF-8 cannot ride the frame as itself;
            // a lossy copy would name a file that does not exist.
            continue;
        };
        let meta = std::fs::metadata(e.path());
        let (dir, size, at_ms) = match meta {
            Ok(m) => (
                m.is_dir(),
                if m.is_dir() { 0 } else { m.len() },
                m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            ),
            Err(_) => (false, 0, 0),
        };
        entries.push(DirEntry { name, dir, size, at_ms });
    }
    entries.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let truncated = entries.len() > MOST_ENTRIES;
    entries.truncate(MOST_ENTRIES);
    Ok((entries, truncated))
}

/// One slice of a file by absolute path, plus the change contract
/// (`proto::Response::PathSlice`): total size and mtime ride out so the
/// puller can notice the file moving under it — no digest guards this
/// path, so "did it change" is the whole of the integrity story.
pub fn read_slice(path: &str, offset: u64) -> Result<(u64, u64, Vec<u8>), String> {
    use std::io::{Read, Seek, SeekFrom};
    if !std::path::Path::new(path).is_absolute() {
        return Err(msg::path_not_absolute(path));
    }
    let meta = std::fs::metadata(path).map_err(|e| msg::cant_list_dir(&path, e))?;
    if meta.is_dir() {
        return Err(msg::not_a_file_path(path));
    }
    let total = meta.len();
    if offset > total {
        return Err(msg::offset_out_of_range(offset, total));
    }
    let at_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut f = std::fs::File::open(path).map_err(|e| msg::cant_list_dir(&path, e))?;
    f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    let want = (total - offset).min(crate::proto::SLICE) as usize;
    let mut buf = vec![0u8; want];
    f.read_exact(&mut buf).map_err(msg::cant_read_slice)?;
    Ok((total, at_ms, buf))
}

/// Where a pulled file lands and how it gets there: written beside its
/// destination under a dot-name, renamed only when whole — a crash
/// leaves a dotfile, never a truncated file wearing a real name. An
/// existing destination refuses up front: overwriting what a person
/// already has is the one irreversible act in this module.
pub struct Landing {
    pub part: std::path::PathBuf,
    pub dest: std::path::PathBuf,
}

pub fn landing(remote_path: &str, dir: &std::path::Path) -> Result<Landing, String> {
    let name = std::path::Path::new(remote_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| msg::not_a_file_path(remote_path))?;
    let dest = dir.join(name);
    if dest.exists() {
        return Err(msg::dest_exists(dest.display()));
    }
    Ok(Landing { part: dir.join(format!(".khor-pull-{name}")), dest })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("khor-files-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Directories first, each half by case-folded name — asymmetric
    /// fixture on purpose: a file sorting before a directory, or `B`
    /// before `a`, would satisfy a plain byte sort and fail here.
    #[test]
    fn directories_lead_and_names_fold_case() {
        let root = tmp("order");
        std::fs::write(root.join("apple"), b"x").unwrap();
        std::fs::write(root.join("Banana"), b"xy").unwrap();
        std::fs::create_dir(root.join("zoo")).unwrap();
        let (rows, truncated) = list_dir(root.to_str().unwrap()).unwrap();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["zoo", "apple", "Banana"]);
        assert!(rows[0].dir && !rows[1].dir);
        assert_eq!(rows[2].size, 2);
        assert!(!truncated);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Relative paths are refused, not resolved: an answer about
    /// "wherever serve started" is an answer about nothing.
    #[test]
    fn a_relative_path_is_refused_by_name() {
        assert!(list_dir("some/where").is_err());
    }

    /// Bigger than the cap says so — the difference between "that is
    /// everything" and "that is the first page" must be on the wire.
    #[test]
    fn a_directory_over_the_cap_admits_truncation() {
        let root = tmp("cap");
        for i in 0..(MOST_ENTRIES + 3) {
            std::fs::write(root.join(format!("f{i:05}")), b"").unwrap();
        }
        let (rows, truncated) = list_dir(root.to_str().unwrap()).unwrap();
        assert_eq!(rows.len(), MOST_ENTRIES);
        assert!(truncated);
        let _ = std::fs::remove_dir_all(&root);
    }
}
