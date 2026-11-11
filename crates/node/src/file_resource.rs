//! A browser file origin's source boundary. Resolve on the machine holding the
//! files, including symlinks; a relative URL never grants access to the rest of
//! that machine. The nearest declared workspace/git root supplies web-root URLs.
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Seek, SeekFrom},
    path::{Component, Path},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slice {
    pub entry: String,
    pub total: u64,
    pub at_ms: u64,
    pub bytes: serde_bytes::ByteBuf,
}

pub fn read(
    anchor: &str,
    resource: &str,
    offset: Option<u64>,
    workspaces: &[String],
) -> Result<Slice, String> {
    let anchor = Path::new(anchor);
    if !anchor.is_absolute() {
        return Err("preview path must be absolute".into());
    }
    let anchor = anchor.canonicalize().map_err(|e| e.to_string())?;
    if !anchor.is_file() {
        return Err("preview requires a regular file".into());
    }
    let parent = anchor.parent().ok_or("file has no parent")?;
    let git = parent.ancestors().find(|p| p.join(".git").exists());
    let declared = workspaces
        .iter()
        .filter_map(|w| Path::new(w).canonicalize().ok())
        .filter(|w| anchor.starts_with(w))
        .max_by_key(|w| w.components().count());
    let root = declared.as_deref().or(git).unwrap_or(parent);
    let entry = anchor
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .components()
        .map(|c| c.as_os_str().to_str().ok_or("non UTF-8 preview path"))
        .collect::<Result<Vec<_>, _>>()?
        .join("/");
    let target = if resource.is_empty() {
        anchor.clone()
    } else {
        let path = Path::new(resource);
        if path
                .components()
                .any(|p| !matches!(p, Component::Normal(_)))
        {
            return Err("resource escapes preview root".into());
        }
        root.join(path).canonicalize().map_err(|e| e.to_string())?
    };
    if !target.starts_with(root) {
        return Err("resource escapes preview root".into());
    }
    if !target.is_file() { return Err("resource is not a regular file".into()); }
    // Open once, then stat this handle, so headers and bytes name one file.
    let mut file = std::fs::File::open(&target).map_err(|e| e.to_string())?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("resource is not a regular file".into());
    }
    let total = meta.len();
    let at_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut bytes = Vec::new();
    if let Some(offset) = offset {
        if offset > total {
            return Err("resource offset out of range".into());
        }
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        bytes.resize((total - offset).min(crate::proto::SLICE) as usize, 0);
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    }
    Ok(Slice {
        entry,
        total,
        at_ms,
        bytes: bytes.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scope_ranges_and_non_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        std::fs::create_dir_all(root.join("pages")).unwrap();
        std::fs::create_dir(root.join(".git")).unwrap();
        let anchor = root.join("pages/index.html");
        std::fs::write(&anchor, "<script src='/app.js'></script>").unwrap();
        std::fs::write(root.join("app.js"), "hello").unwrap();
        let path = anchor.to_str().unwrap();
        let head = read(path, "", None, &[]).unwrap();
        assert_eq!(head.entry, "pages/index.html");
        assert!(head.bytes.is_empty());
        assert_eq!(
            read(path, "app.js", Some(2), &[]).unwrap().bytes.as_ref(),
            b"llo"
        );
        assert!(read(path, "../outside", None, &[]).is_err());
        assert!(read(path, "/etc/passwd", None, &[]).is_err());
        assert!(read(path, "pages", None, &[]).is_err());
        assert!(read(path, "app.js", Some(6), &[]).is_err());
        #[cfg(unix)]
        {
            let outside = dir.path().join("secret");
            std::fs::write(&outside, "secret").unwrap();
            std::os::unix::fs::symlink(outside, root.join("escape")).unwrap();
            assert!(read(path, "escape", None, &[]).is_err());
        }
    }
}
