//! A selected original byte range, staged by the existing file-handoff owner.
//! No body parser, provider lookup or complete-record allocation lives here.
use crate::file_handoff::{self, FileHandoff};
use khor_node::{
    Node, SessionId,
    adaptor::{transcript_page::Cursor, transcript_window::INLINE_RECORD_BYTES},
    road::Job,
    session_content::{ReadBinding, remote::ClientRead},
};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
};

#[derive(PartialEq, Eq)]
pub(crate) struct Selection {
    binding: ReadBinding,
    cursor: Cursor,
    start: u64,
    end: u64,
}

pub fn start(
    root: &Path,
    transfer: &str,
    id: &str,
    binding: &str,
    cursor: &str,
    start: u64,
    end: u64,
) -> Result<FileHandoff, String> {
    if cursor.len() > 4096 {
        return Err("history cursor exceeds its byte budget".into());
    }
    let binding = ReadBinding::decode(binding, &SessionId(id.into()))?;
    let cursor: Cursor =
        serde_json::from_str(cursor).map_err(|error| format!("invalid history cursor: {error}"))?;
    let cursor = cursor.at_record_range(&binding.source_id(), start, end)?;
    file_handoff::start_history(
        root,
        transfer,
        Selection {
            binding,
            cursor,
            start,
            end,
        },
    )
}

fn check(job: &Job) -> Result<(), String> {
    if job.is_stopped() {
        Err(khor_node::drops::cancelled())
    } else {
        Ok(())
    }
}

/// Begin/claim/IO/cancel are one child operation. Cancellation during a lost
/// Begin ACK waits for that bounded transport, then cancels before any IO.
/// Once claimed, the same job notification cancels the real home-side lease.
async fn chunk(
    root: &Path,
    node: &Node,
    selection: &Selection,
    from: u64,
    job: &Job,
    stopped: &tokio::sync::Notify,
) -> Result<Vec<u8>, String> {
    check(job)?;
    let key = node.content_begin(&selection.binding).await?;
    let result = async {
        check(job)?;
        let request = ClientRead::claim(root, selection.binding.clone(), &key)?;
        let read = node.content_record(&request, selection.cursor.clone(), from);
        let result = tokio::select! {
            result = read => result,
            _ = stopped.notified() => Err(khor_node::drops::cancelled()),
        }?;
        check(job)?;
        let expected = (selection.end - from).min(INLINE_RECORD_BYTES as u64);
        let next = from
            .checked_add(result.bytes.len() as u64)
            .ok_or("invalid history export continuation")?;
        if result.offset != from
            || result.bytes.len() as u64 != expected
            || result.next != (next < selection.end).then_some(next)
        {
            return Err("invalid history export continuation".into());
        }
        Ok(result.bytes)
    }
    .await;
    // Every successful registration has exactly one cleanup, including failed
    // claims, source replacement and a cancellation which won the IO race.
    let cleanup = node.content_cancel(&selection.binding, &key).await;
    match (result, cleanup) {
        (Ok(bytes), Ok(())) => Ok(bytes),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(format!(
            "{error}; history cleanup failed: {}",
            cleanup.chars().take(512).collect::<String>()
        )),
    }
}

pub(crate) async fn prepare(
    root: &Path,
    selection: &Selection,
    token: &str,
    job: &Job,
    stopped: &tokio::sync::Notify,
) -> Result<PathBuf, String> {
    check(job)?;
    let node = Node::open(root.to_path_buf())?;
    check(job)?;
    let total = selection.end - selection.start;
    job.total.store(total, Ordering::Relaxed);
    // Jobs never reuse a cached result without checking its real source. A
    // fresh job stages only its selected range; the existing cache pruner owns it.
    let cache = file_handoff::cache_root(root);
    std::fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    let dir = cache.join(format!("history-{token}"));
    file_handoff::prune_cache(&cache, &dir);
    std::fs::create_dir(&dir).map_err(|error| error.to_string())?;
    let result = async {
        let landing = khor_node::files::landing("history-record.jsonl", &dir)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&landing.part)
            .map_err(|error| error.to_string())?;
        let mut from = selection.start;
        while from < selection.end {
            let bytes = chunk(root, &node, selection, from, job, stopped).await?;
            check(job)?;
            file.write_all(&bytes).map_err(|error| error.to_string())?;
            from += bytes.len() as u64;
            job.moved.store(from - selection.start, Ordering::Relaxed);
        }
        file.flush().map_err(|error| error.to_string())?;
        check(job)?;
        drop(file);
        std::fs::rename(&landing.part, &landing.dest).map_err(|error| error.to_string())?;
        check(job)?;
        let note = dir.join("ready.pending");
        std::fs::write(&note, b"history-record.jsonl").map_err(|error| error.to_string())?;
        std::fs::rename(note, dir.join("ready.json")).map_err(|error| error.to_string())?;
        check(job)?;
        Ok(landing.dest)
    }
    .await;
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    result
}

/// OS Save panels may approve replacement, but failed/cancelled transfers never
/// touch the destination. The complete sibling is renamed only after success.
pub fn save(root: &Path, id: &str, destination: &Path) -> Result<(), String> {
    let source = file_handoff::ready_path(root, id)?;
    file_handoff::save_ready(root, id, &source, destination)
}

pub(crate) fn copy_to(
    source: &Path,
    destination: &Path,
    job: &Arc<Job>,
    commit: &std::sync::Mutex<()>,
    id: &str,
) -> Result<(), String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let parent = destination
        .parent()
        .ok_or("history export has no destination directory")?;
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let part = parent.join(format!(
        ".khor-history-{id}-{}-{serial}.part",
        std::process::id()
    ));
    let mut created = false;
    let result = (|| {
        check(job)?;
        let mut original = std::fs::File::open(source).map_err(|error| error.to_string())?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)
            .map_err(|error| error.to_string())?;
        created = true;
        let mut bytes = [0u8; INLINE_RECORD_BYTES];
        let mut moved = 0u64;
        let total = job.total.load(Ordering::Relaxed);
        loop {
            check(job)?;
            let count = original
                .read(&mut bytes)
                .map_err(|error| error.to_string())?;
            if count == 0 {
                break;
            }
            if count as u64 > total.saturating_sub(moved) {
                return Err("history export cache changed".into());
            }
            output
                .write_all(&bytes[..count])
                .map_err(|error| error.to_string())?;
            moved += count as u64;
        }
        if moved != total {
            return Err("history export cache changed".into());
        }
        output.flush().map_err(|error| error.to_string())?;
        drop(output);
        // Cancellation and destination replacement have one ordering point.
        // The lock covers only this final check/rename, never disk streaming.
        let _commit = commit.lock().map_err(|error| error.to_string())?;
        check(job)?;
        std::fs::rename(&part, destination).map_err(|error| error.to_string())?;
        Ok(())
    })();
    if result.is_err() && created {
        let _ = std::fs::remove_file(part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "khor-history-export-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(file_handoff::cache_root(&self.0));
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn selected_original_streams_without_parsing_and_rejects_replacement() {
        use khor_core::ObservationNamespace;
        use khor_node::{
            adaptor::{
                claude::Utterance,
                transcript_page::{Position, read_page},
            },
            session_content::ContentReference,
        };
        let root = Scratch::new();
        let node = Node::open(root.0.clone()).unwrap();
        let native = "01a17c1a-a542-7822-b01d-82a832a158e6";
        let path = root.0.join(format!(
            ".codex/sessions/2026/10/30/rollout-2026-10-30T12-00-00-{native}.jsonl"
        ));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Invalid JSON still has a complete original-byte export. Neither its
        // content nor split UTF-8 characters may be decoded/replaced here.
        let original = format!("not-json:{}\n", "中🙂".repeat(30000)).into_bytes();
        std::fs::write(&path, &original).unwrap();
        let binding = ReadBinding::new(
            SessionId("observe/export".into()),
            ContentReference {
                home: node.device(),
                namespace: ObservationNamespace::Native,
                agent: "codex".into(),
                native_id: native.into(),
            },
        )
        .unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let page = read_page(
            &mut file,
            &binding.source_id(),
            None,
            |_| Vec::<Utterance>::new(),
            &|| false,
        )
        .unwrap();
        let cursor = page.cursor_before(Position {
            start: 0,
            end: original.len() as u64,
            item: 0,
        });
        let selection = Selection {
            binding,
            cursor,
            start: 0,
            end: original.len() as u64,
        };
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let job = Job::default();
        assert!(
            !file_handoff::cache_root(&root.0).exists(),
            "first export has no prior handoff cache"
        );
        let landed = rt
            .block_on(prepare(
                &root.0,
                &selection,
                "first",
                &job,
                &tokio::sync::Notify::new(),
            ))
            .unwrap();
        assert_eq!(std::fs::read(&landed).unwrap(), original);
        assert_eq!(job.moved.load(Ordering::Relaxed), original.len() as u64);
        assert!(landed.parent().unwrap().join("ready.json").is_file());
        let replacement = path.with_extension("replacement");
        std::fs::write(&replacement, &original).unwrap();
        std::fs::rename(replacement, &path).unwrap();
        let error = rt
            .block_on(prepare(
                &root.0,
                &selection,
                "replaced",
                &Job::default(),
                &tokio::sync::Notify::new(),
            ))
            .unwrap_err();
        assert!(error.contains("changed"), "{error}");
        assert!(
            !file_handoff::cache_root(&root.0)
                .join("history-replaced")
                .exists()
        );
    }
    #[test]
    fn save_commits_only_a_complete_uncancelled_sibling() {
        let root = Scratch::new();
        let source = root.0.join("source");
        let destination = root.0.join("existing");
        let original = "中🙂".repeat(30000).into_bytes();
        std::fs::write(&source, &original).unwrap();
        std::fs::write(&destination, b"previous contents").unwrap();
        let job = Arc::new(Job::default());
        let commit = std::sync::Mutex::new(());
        job.total
            .store(original.len() as u64 + 1, Ordering::Relaxed);
        assert_eq!(
            copy_to(&source, &destination, &job, &commit, "exact").unwrap_err(),
            "history export cache changed"
        );
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous contents");
        job.total.store(original.len() as u64, Ordering::Relaxed);
        job.stopped.store(true, Ordering::Relaxed);
        assert!(copy_to(&source, &destination, &job, &commit, "exact").is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous contents");
        assert_eq!(
            std::fs::read_dir(&root.0).unwrap().count(),
            2,
            "failed/cancelled sibling files are removed"
        );
        job.stopped.store(false, Ordering::Relaxed);
        // Rename replacement is atomic on the current macOS/Linux targets.
        #[cfg(unix)]
        {
            copy_to(&source, &destination, &job, &commit, "exact").unwrap();
            assert_eq!(std::fs::read(destination).unwrap(), original);
        }
    }
}
