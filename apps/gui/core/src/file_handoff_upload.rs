//! Bounded renderer-produced files enter the same native save/share owner.
//! The only published path is a complete atomic commit, never the staging file.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::io::Write;
use std::sync::atomic::Ordering;
const LIMIT: u64 = 512 * 1024 * 1024;
pub(super) fn directory(root: &Path, id: &str) -> PathBuf {
    cache_root(root).join(format!("upload-{id}"))
}
pub fn begin(root: &Path, id: &str, name: &str) -> Result<(), String> {
    valid(id)?;
    if name.is_empty()
        || name.len() > 128
        || matches!(
            name,
            "." | ".." | "ready.json" | "ready.pending" | "body.part" | ".readers.lock"
        )
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
    {
        return Err("invalid exported file name".into());
    }
    let mut all = entries().lock().map_err(|error| error.to_string())?;
    if all.contains_key(&(root.to_path_buf(), id.into())) {
        return Err("export id already belongs to a writer".into());
    }
    if all
        .values()
        .filter(|entry| {
            matches!(entry.source, Source::Upload { .. })
                && !entry.job.is_stopped()
                && !entry.job.state().is_ok_and(|state| state.done)
        })
        .count()
        >= 16
    {
        return Err("too many active file writers".into());
    }
    let cache = cache_root(root);
    std::fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    let dir = directory(root, id);
    prune_cache(&cache, &dir);
    std::fs::create_dir(&dir).map_err(|error| error.to_string())?;
    if let Err(error) = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("body.part"))
    {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(error.to_string());
    }
    all.insert(
        (root.to_path_buf(), id.into()),
        Arc::new(Entry {
            source: Source::Upload { name: name.into() },
            job: Arc::new(Job::default()),
            stopped: tokio::sync::Notify::new(),
            commit: Mutex::new(()),
            started: Instant::now(),
        }),
    );
    Ok(())
}
pub fn append(root: &Path, id: &str, offset: u64, data: &str) -> Result<(), String> {
    if data.len() > 4 * 65536_usize.div_ceil(3) {
        return Err("file chunk exceeds its budget".into());
    }
    let bytes = STANDARD.decode(data).map_err(|error| error.to_string())?;
    if bytes.is_empty() || bytes.len() > 65536 {
        return Err("invalid file chunk".into());
    }
    let owner = entry(root, id)?;
    let _guard = owner.commit.lock().map_err(|error| error.to_string())?;
    if !matches!(owner.source, Source::Upload { .. })
        || owner.job.is_stopped()
        || owner.job.state().is_ok_and(|state| state.done)
    {
        return Err("file writer is no longer active".into());
    }
    let moved = owner.job.moved.load(Ordering::Relaxed);
    if offset != moved || bytes.len() as u64 > LIMIT.saturating_sub(moved) {
        return Err("file writer offset or size is invalid".into());
    }
    let mut file = OpenOptions::new()
        .append(true)
        .open(directory(root, id).join("body.part"))
        .map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() != moved {
        return Err("staged file changed".into());
    }
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    owner
        .job
        .moved
        .store(moved + bytes.len() as u64, Ordering::Relaxed);
    Ok(())
}
pub fn finish(root: &Path, id: &str, total: u64) -> Result<(), String> {
    let owner = entry(root, id)?;
    let _guard = owner.commit.lock().map_err(|error| error.to_string())?;
    let Source::Upload { name } = &owner.source else {
        return Err("not a file writer".into());
    };
    if owner.job.is_stopped()
        || owner.job.state().is_ok_and(|state| state.done)
        || total != owner.job.moved.load(Ordering::Relaxed)
        || total > LIMIT
    {
        return Err("incomplete file writer".into());
    }
    let dir = directory(root, id);
    let file = OpenOptions::new()
        .write(true)
        .open(dir.join("body.part"))
        .map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() != total {
        return Err("staged file changed".into());
    }
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    let path = dir.join(name);
    std::fs::rename(dir.join("body.part"), &path).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("ready.pending"), name).map_err(|error| error.to_string())?;
    std::fs::rename(dir.join("ready.pending"), dir.join("ready.json"))
        .map_err(|error| error.to_string())?;
    owner.job.total.store(total, Ordering::Relaxed);
    owner.job.finish(Ok(path.to_string_lossy().into_owned()));
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_and_cancelled_writers_never_publish_and_offsets_cannot_repeat() {
        let root = super::super::tests::Scratch::new();
        begin(&root.0, "writer-test", "conversation.md").unwrap();
        assert!(ready_path(&root.0, "writer-test").is_err());
        append(&root.0, "writer-test", 0, &STANDARD.encode(b"first")).unwrap();
        assert!(append(&root.0, "writer-test", 0, &STANDARD.encode(b"again")).is_err());
        assert!(finish(&root.0, "writer-test", 10).is_err());
        cancel(&root.0, "writer-test").unwrap();
        assert!(!directory(&root.0, "writer-test").exists());
        assert!(finish(&root.0, "writer-test", 5).is_err());
        begin(&root.0, "writer-complete", "picture.png").unwrap();
        append(&root.0, "writer-complete", 0, &STANDARD.encode(b"complete")).unwrap();
        finish(&root.0, "writer-complete", 8).unwrap();
        assert_eq!(
            std::fs::read(ready_path(&root.0, "writer-complete").unwrap()).unwrap(),
            b"complete"
        );
        assert!(append(&root.0, "writer-complete", 8, &STANDARD.encode(b"late")).is_err());
    }
}
