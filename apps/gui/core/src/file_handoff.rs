//! Whole original files staged for this device's other apps. Preview bytes
//! never cross this boundary. The existing node file road streams to disk.
use khor_node::{Node, road::Job};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, TS)]
pub struct FileHandoff {
    #[ts(type = "number")]
    pub moved: u64,
    #[ts(type = "number")]
    pub total: u64,
    pub ready: bool,
    pub cancelled: bool,
    pub error: Option<String>,
}
struct Entry {
    machine: String,
    path: String,
    job: Arc<Job>,
    started: Instant,
}
fn entries() -> &'static Mutex<HashMap<(PathBuf, String), Arc<Entry>>> {
    static ALL: OnceLock<Mutex<HashMap<(PathBuf, String), Arc<Entry>>>> = OnceLock::new();
    ALL.get_or_init(Default::default)
}
fn valid(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("invalid file handoff id".into());
    }
    Ok(())
}
fn entry(root: &Path, id: &str) -> Result<Arc<Entry>, String> {
    valid(id)?;
    entries()
        .lock()
        .map_err(|e| e.to_string())?
        .get(&(root.to_path_buf(), id.to_owned()))
        .cloned()
        .ok_or_else(|| "file handoff is no longer available".into())
}
fn snapshot(job: &Job) -> FileHandoff {
    let cancelled = job.is_stopped();
    match job.state() {
        Ok(s) => FileHandoff {
            moved: s.moved,
            total: s.total,
            ready: s.done && !cancelled,
            cancelled,
            error: None,
        },
        Err(error) => FileHandoff {
            moved: job.moved.load(std::sync::atomic::Ordering::Relaxed),
            total: job.total.load(std::sync::atomic::Ordering::Relaxed),
            ready: false,
            cancelled,
            error: (!cancelled).then_some(error),
        },
    }
}
pub fn start(root: &Path, id: &str, machine: &str, path: &str) -> Result<FileHandoff, String> {
    valid(id)?;
    let key = (root.to_path_buf(), id.to_owned());
    let mut all = entries().lock().map_err(|e| e.to_string())?;
    all.retain(|_, e| {
        e.started.elapsed() < Duration::from_secs(600)
            || matches!(e.job.state(),Ok(s) if !s.done&&!e.job.is_stopped())
    });
    if let Some(e) = all.get(&key) {
        if e.machine != machine || e.path != path {
            return Err("file handoff id belongs to another file".into());
        }
        return Ok(snapshot(&e.job));
    }
    let e = Arc::new(Entry {
        machine: machine.into(),
        path: path.into(),
        job: Arc::new(Job::default()),
        started: Instant::now(),
    });
    all.insert(key, e.clone());
    drop(all);
    let root = root.to_path_buf();
    let token = id.to_owned();
    let worker = e.clone();
    std::thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())
            .and_then(|rt| {
                rt.block_on(prepare(
                    &root,
                    &worker.machine,
                    &worker.path,
                    &token,
                    &worker.job,
                ))
            });
        worker.job.finish(result.map(|p| p.display().to_string()));
    });
    Ok(snapshot(&e.job))
}
pub fn state(root: &Path, id: &str) -> Result<FileHandoff, String> {
    Ok(snapshot(&entry(root, id)?.job))
}
pub fn cancel(root: &Path, id: &str) -> Result<(), String> {
    entry(root, id)?
        .job
        .stopped
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
/// Only a completed, app-owned job can ask the native adapter to open a
/// path. The renderer cannot use this command to name an arbitrary file.
pub fn ready_path(root: &Path, id: &str) -> Result<PathBuf, String> {
    let e = entry(root, id)?;
    if e.job.is_stopped() {
        return Err(khor_node::drops::cancelled());
    }
    let s = e.job.state()?;
    if !s.done {
        return Err("file is still being downloaded".into());
    }
    let path = PathBuf::from(s.dest);
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let base = cache_root(root).canonicalize().map_err(|e| e.to_string())?;
    if !path.starts_with(base) || !path.is_file() {
        return Err(khor_catalog::msg::file_cache_missing().into());
    }
    Ok(path)
}
fn cache_root(root: &Path) -> PathBuf {
    let key = blake3::hash(root.as_os_str().to_string_lossy().as_bytes())
        .to_hex()
        .to_string();
    #[cfg(target_os = "android")]
    let temporary = root.join(".khor/cache");
    #[cfg(not(target_os = "android"))]
    let temporary = std::env::temp_dir();
    temporary.join("khor-file-handoff").join(key)
}
async fn prepare(
    root: &Path,
    machine: &str,
    path: &str,
    token: &str,
    job: &Job,
) -> Result<PathBuf, String> {
    use std::sync::atomic::Ordering;
    if job.is_stopped() {
        return Err(khor_node::drops::cancelled());
    }
    let n = Node::open(root.to_path_buf())?;
    let machine = if machine.is_empty() {
        n.name()
    } else {
        machine
    };
    if job.is_stopped() {
        return Err(khor_node::drops::cancelled());
    }
    let stamp = n.stat_of(machine, path).await?;
    job.total.store(stamp.0, Ordering::Relaxed);
    let version =
        blake3::hash(&serde_json::to_vec(&(machine, path, &stamp)).map_err(|e| e.to_string())?)
            .to_hex()
            .to_string();
    let dir = cache_root(root).join(version);
    prune_cache(&cache_root(root), &dir);
    let manifest = dir.join("ready.json");
    if let Ok(name) = std::fs::read_to_string(&manifest) {
        let p = dir.join(name);
        if let (Ok(p), Ok(base)) = (p.canonicalize(), dir.canonicalize()) {
            if p.starts_with(&base)
                && p.is_file()
                && std::fs::metadata(&p)
                    .map(|m| m.len() == stamp.0)
                    .unwrap_or(false)
            {
                if job.is_stopped() {
                    return Err(khor_node::drops::cancelled());
                }
                let _ = std::fs::File::open(&manifest).and_then(|f| {
                    f.set_times(
                        std::fs::FileTimes::new().set_modified(std::time::SystemTime::now()),
                    )
                });
                job.moved.store(stamp.0, Ordering::Relaxed);
                return Ok(p);
            }
        }
    }
    let work = dir.join(token);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let result = async {
        let landed = n.pull_path_progress(machine, path, &work, job).await?;
        if job.is_stopped() {
            return Err(khor_node::drops::cancelled());
        }
        if n.stat_of(machine, path).await? != stamp {
            return Err(khor_catalog::msg::file_changed_mid_pull().into());
        }
        let relative = landed.strip_prefix(&dir).map_err(|e| e.to_string())?;
        let original = landed.canonicalize().map_err(|e| e.to_string())?;
        let note = dir.join(format!("ready-{token}.json"));
        std::fs::write(&note, relative.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        std::fs::rename(note, manifest).map_err(|e| e.to_string())?;
        Ok(original)
    }
    .await;
    if result.is_err() {
        let _ = std::fs::remove_dir_all(work);
    }
    result
}

/// Cache is disposable, never a receive folder. Age and size are soft
/// bounds: a file just handed to another app stays alive for an hour,
/// even if that single original exceeds the budget.
fn prune_cache(root: &Path, current: &Path) {
    fn size(path: &Path) -> u64 {
        let Ok(entries) = std::fs::read_dir(path) else {
            return 0;
        };
        entries
            .flatten()
            .map(|e| match e.file_type() {
                Ok(t) if t.is_dir() => size(&e.path()),
                Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
                _ => 0,
            })
            .sum()
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut dirs: Vec<_> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| {
            let path = e.path();
            let age = std::fs::metadata(path.join("ready.json"))
                .or_else(|_| e.metadata())
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .unwrap_or_default();
            let bytes = size(&path);
            (path, age, bytes)
        })
        .collect();
    dirs.sort_by_key(|(_, age, _)| std::cmp::Reverse(*age));
    let mut total: u64 = dirs.iter().map(|(_, _, bytes)| *bytes).sum();
    for (path, age, bytes) in dirs {
        let completed = path.join("ready.json").is_file();
        if path != current
            && age > Duration::from_secs(3600)
            && (age > Duration::from_secs(7 * 86400) || (completed && total > 512 * 1024 * 1024))
            && std::fs::remove_dir_all(path).is_ok()
        {
            total = total.saturating_sub(bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let nonce = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!(
                "khor-handoff-{}-{nonce}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(cache_root(&self.0));
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn original_bytes_cache_version_and_cancel() {
        let root = Scratch::new();
        let phone = Node::open_hosting(root.0.clone(), "handoff-test", false).unwrap();
        assert!(!phone.hosts());
        let source = root.0.join("large-original.pptx");
        let mut out = std::fs::File::create(&source).unwrap();
        // Exceeds the text-preview cap, contains non-text bytes, and is
        // streamed from a machine which does not accept normal downloads.
        let chunk: Vec<u8> = (0..1024 * 1024).map(|i| (i % 251) as u8).collect();
        let mut expected = blake3::Hasher::new();
        for _ in 0..65 {
            out.write_all(&chunk).unwrap();
            expected.update(&chunk);
        }
        drop(out);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let job = Job::default();
        let staging = root.0.join("phone-staging");
        std::fs::create_dir(&staging).unwrap();
        let phone_copy = rt
            .block_on(phone.pull_path_progress(
                phone.name(),
                source.to_str().unwrap(),
                &staging,
                &Job::default(),
            ))
            .unwrap();
        assert_eq!(
            std::fs::metadata(phone_copy).unwrap().len(),
            65 * 1024 * 1024
        );
        let dest = rt
            .block_on(prepare(
                &root.0,
                "",
                source.to_str().unwrap(),
                "original",
                &job,
            ))
            .unwrap();
        assert_eq!(
            job.moved.load(std::sync::atomic::Ordering::Relaxed),
            65 * 1024 * 1024
        );
        let mut digest = blake3::Hasher::new();
        let mut input = std::fs::File::open(&dest).unwrap();
        std::io::copy(&mut input, &mut digest).unwrap();
        assert_eq!(digest.finalize(), expected.finalize());
        let cached = rt
            .block_on(prepare(
                &root.0,
                "",
                source.to_str().unwrap(),
                "cached",
                &Job::default(),
            ))
            .unwrap();
        assert_eq!(cached, dest);
        std::fs::write(&source, b"new original").unwrap();
        let changed = rt
            .block_on(prepare(
                &root.0,
                "",
                source.to_str().unwrap(),
                "changed",
                &Job::default(),
            ))
            .unwrap();
        assert_ne!(changed, dest);
        assert_eq!(std::fs::read(changed).unwrap(), b"new original");
        start(&root.0, "public", "", source.to_str().unwrap()).unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while !state(&root.0, "public").unwrap().ready {
            assert!(Instant::now() < until, "public job did not finish");
            assert!(state(&root.0, "public").unwrap().error.is_none());
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            std::fs::read(ready_path(&root.0, "public").unwrap()).unwrap(),
            b"new original"
        );
        let stopped = Job::default();
        stopped
            .stopped
            .store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(
            rt.block_on(prepare(
                &root.0,
                "",
                source.to_str().unwrap(),
                "stopped",
                &stopped
            ))
            .is_err()
        );
        assert!(ready_path(&root.0, "unknown").is_err());
        assert!(start(&root.0, "../escape", "", source.to_str().unwrap()).is_err());
    }
    #[test]
    fn cancelled_transfer_removes_its_partial_copy() {
        let root = Scratch::new();
        Node::open_as(root.0.clone(), "handoff-cancel").unwrap();
        let source = root.0.join("cancelled.bin");
        std::fs::File::create(&source)
            .unwrap()
            .set_len(256 * 1024 * 1024)
            .unwrap();
        start(&root.0, "live", "", source.to_str().unwrap()).unwrap();
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            let s = state(&root.0, "live").unwrap();
            assert!(Instant::now() < until, "transfer never published progress");
            assert!(s.error.is_none(), "{:?}", s.error);
            assert!(
                !s.ready,
                "transfer finished before cancellation could be exercised"
            );
            if s.moved > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        cancel(&root.0, "live").unwrap();
        while entry(&root.0, "live").unwrap().job.state().is_ok() {
            assert!(Instant::now() < until, "cancel did not stop the transfer");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(state(&root.0, "live").unwrap().cancelled);
        assert!(ready_path(&root.0, "live").is_err());
        for version in std::fs::read_dir(cache_root(&root.0)).unwrap().flatten() {
            assert!(
                !version.path().join("live").exists(),
                "cancelled copy still occupies disk"
            );
        }
    }
    #[test]
    fn ids_cannot_be_rebound_or_open_partial_files() {
        let root = Scratch::new();
        Node::open_as(root.0.clone(), "handoff-id").unwrap();
        start(&root.0, "same", "", "missing").unwrap();
        assert!(start(&root.0, "same", "", "different").is_err());
        cancel(&root.0, "same").unwrap();
        assert!(state(&root.0, "same").unwrap().cancelled);
        assert!(ready_path(&root.0, "same").is_err());
        let until = Instant::now() + Duration::from_secs(10);
        while entry(&root.0, "same").unwrap().job.state().is_ok() {
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
