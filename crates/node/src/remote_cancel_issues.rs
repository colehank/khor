//! Bounded error records retain the damaged source/receipt in place. No issue
//! contains an executable target, raw task history, endpoint or credential.
use super::*;
use std::{collections::HashSet, io::Write};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ChildIssue {
    version: u32,
    intent_id: String,
    child_key: String,
    code: String,
    resolved: bool,
}
fn child_path(root: &Path, intent: &Intent, child: &str) -> PathBuf {
    directory(root, &intent.run)
        .join("issues")
        .join(format!("{}.json", key(child)))
}
pub(super) fn child(
    node: &Node,
    intent: &Intent,
    child: &str,
    code: Option<&str>,
) -> Result<(), String> {
    let path = child_path(node.root(), intent, child);
    let prior = match read::<ChildIssue>(&path, 4096) {
        Ok(issue)
            if issue.version == 1
                && issue.intent_id == intent.intent_id
                && issue.child_key == child =>
        {
            Some(issue)
        }
        Ok(_) => return Ok(()),
        // A damaged issue also remains in place and unresolved. It must not
        // prevent another valid child from receiving its own cancellation.
        Err(_) if path.try_exists().map_err(|_| "ReadFailed")? => return Ok(()),
        Err(_) => None,
    };
    if code.is_none() && prior.is_none() {
        return Ok(());
    }
    let issue = ChildIssue {
        version: 1,
        intent_id: intent.intent_id.clone(),
        child_key: child.into(),
        code: code.unwrap_or("Resolved").into(),
        resolved: code.is_none(),
    };
    if prior.as_ref() == Some(&issue) {
        return Ok(());
    }
    let folder = directory(node.root(), &intent.run).join("issues");
    let entries = names(&folder, CHILD_LIMIT * 2)?;
    if !path.exists() && entries.len() >= CHILD_LIMIT {
        return Err("ResourceLimit".into());
    }
    write(&path, &issue).map_err(|(code, _)| code)
}
pub(super) fn children(node: &Node, intent: &Intent) -> Result<HashSet<String>, String> {
    let folder = directory(node.root(), &intent.run).join("issues");
    let entries = names(&folder, CHILD_LIMIT * 2)?;
    if entries.len() > CHILD_LIMIT {
        return Err("ResourceLimit".into());
    }
    let mut unresolved = HashSet::new();
    for name in entries {
        match read::<ChildIssue>(&folder.join(&name), 4096) {
            Ok(issue)
                if issue.version == 1
                    && issue.intent_id == intent.intent_id
                    && format!("{}.json", key(&issue.child_key)) == name =>
            {
                if !issue.resolved {
                    unresolved.insert(issue.child_key);
                }
            }
            _ => {
                unresolved.insert(format!("invalid-issue:{name}"));
            }
        }
    }
    Ok(unresolved)
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PendingIssue {
    version: u32,
    marker_hash: String,
    code: String,
    resolved: bool,
    attention_only: bool,
}
fn pending_issue(root: &Path, hash: &str) -> PathBuf {
    base(root).join(hash).join("pending-issue.json")
}
fn valid_pending_issue(issue: &PendingIssue, hash: &str) -> bool {
    issue.version == 1 && issue.marker_hash == hash
}
fn pending_record(
    root: &Path,
    hash: &str,
    code: &str,
    resolved: bool,
    attention_only: bool,
) -> Result<(), String> {
    let path = pending_issue(root, hash);
    let issue = PendingIssue {
        version: 1,
        marker_hash: hash.into(),
        code: code.into(),
        resolved,
        attention_only,
    };
    if read::<PendingIssue>(&path, 4096)
        .ok()
        .filter(|prior| valid_pending_issue(prior, hash))
        .as_ref()
        == Some(&issue)
    {
        return Ok(());
    }
    if path.try_exists().map_err(|_| "ReadFailed")?
        && !read::<PendingIssue>(&path, 4096).is_ok_and(|prior| valid_pending_issue(&prior, hash))
    {
        return Ok(());
    }
    write(&path, &issue).map_err(|(code, _)| code)
}
fn marker_hash(name: &str) -> Option<&str> {
    let hash = name.strip_suffix(".json")?;
    (hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    .then_some(hash)
}
fn marker_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| "ReadFailed")?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| "ReadFailed")?;
    if bytes.len() > 4096 {
        return Err("ResourceLimit".into());
    }
    Ok(bytes)
}
fn preserve_marker(root: &Path, hash: &str, bytes: &[u8]) -> Result<(), String> {
    let dir = base(root).join(hash);
    fs::create_dir_all(&dir).map_err(|_| "WriteFailed")?;
    let path = dir.join("pending-original.bin");
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(&path) {
        Ok(mut file) => {
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| "WriteFailed")?;
            #[cfg(unix)]
            File::open(&dir)
                .and_then(|f| f.sync_all())
                .map_err(|_| "SyncFailed")?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if marker_bytes(&path)? == bytes {
                Ok(())
            } else {
                Err("OriginalConflict".into())
            }
        }
        Err(_) => Err("WriteFailed".into()),
    }
}
/// One current marker, one exact hash lookup. Never enumerate intent history.
/// Metadata::Discover holds the same short metadata try-lock as Persist.
pub(super) fn discover_pending(node: &Node, name: &str) -> Result<(Option<String>, bool), String> {
    let Some(hash) = marker_hash(name) else {
        let issue_hash = key(&format!("invalid-pending:{name}"));
        pending_record(node.root(), &issue_hash, "InvalidPendingName", false, true)?;
        return Ok((None, true));
    };
    let marker = base(node.root()).join("pending").join(name);
    let direct = read::<String>(&marker, 4096).ok();
    if let Some(run) = direct.as_ref().filter(|run| {
        key(run) == hash && valid_run(node, run).is_ok() && load_intent(node, run).is_ok()
    }) {
        if pending_issue(node.root(), hash)
            .try_exists()
            .map_err(|_| "ReadFailed")?
        {
            pending_record(node.root(), hash, "Resolved", true, false)?;
        }
        return Ok((Some(run.clone()), false));
    }
    let intent = read::<Intent>(&base(node.root()).join(hash).join("intent.json"), 4096)
        .ok()
        .filter(|intent| {
            intent.version == 1
                && intent.execution_home == node.device()
                && key(&intent.run) == hash
                && valid_run(node, &intent.run).is_ok()
        });
    let Some(intent) = intent else {
        pending_record(node.root(), hash, "PendingIntentUnavailable", false, true)?;
        return Ok((None, true));
    };
    let _run = match gate(&directory(node.root(), &intent.run).join("run.lock")) {
        Ok(lock) => lock,
        Err(_) => {
            pending_record(node.root(), hash, "PendingReferenceDeferred", false, false)?;
            return Ok((Some(intent.run), true));
        }
    };
    let repaired = marker_bytes(&marker).and_then(|bytes| {
        preserve_marker(node.root(), hash, &bytes)?;
        // A second writer outside the protocol must not lose newer raw bytes.
        if marker_bytes(&marker)? != bytes {
            return Err("ReferenceChanged".into());
        }
        write(&marker, &intent.run).map_err(|(code, _)| code)
    });
    match repaired {
        Ok(()) => {
            pending_record(node.root(), hash, "Resolved", true, false)?;
            Ok((Some(intent.run), false))
        }
        Err(code) => {
            let prior = read::<PendingIssue>(&pending_issue(node.root(), hash), 4096)
                .ok()
                .filter(|issue| valid_pending_issue(issue, hash));
            let attention = prior.is_some_and(|issue| !issue.resolved && issue.attention_only);
            pending_record(node.root(), hash, &code, false, attention)?;
            Ok((if attention { None } else { Some(intent.run) }, true))
        }
    }
}
pub(super) fn pending_unresolved(node: &Node, run: &str) -> bool {
    let path = pending(node.root(), run);
    match path.try_exists() {
        Ok(true) => !read::<String>(&path, 4096).is_ok_and(|value| value == run),
        Err(_) => true,
        Ok(false) => {
            let issue = pending_issue(node.root(), &key(run));
            match issue.try_exists() {
                Ok(false) => false,
                Err(_) => true,
                Ok(true) => !read::<PendingIssue>(&issue, 4096)
                    .is_ok_and(|issue| valid_pending_issue(&issue, &key(run)) && issue.resolved),
            }
        }
    }
}
pub(super) fn retire_pending(node: &Node, intent: &Intent) -> Result<(), String> {
    let path = pending(node.root(), &intent.run);
    if !path.try_exists().map_err(|_| "ReadFailed")? {
        return Ok(());
    }
    if read::<String>(&path, 4096).is_ok_and(|run| run == intent.run) {
        fs::remove_file(path).map_err(|_| "WriteFailed")?;
    } else {
        // Keep oversized/conflicting bytes in place; an explicit Retry can
        // recheck them. Discovery must not respawn an attention-only run.
        pending_record(
            node.root(),
            &key(&intent.run),
            "PendingReferenceNeedsAttention",
            false,
            true,
        )?;
    }
    Ok(())
}
pub(super) fn before_persist(node: &Node, run: &str) -> Result<(), String> {
    let path = pending(node.root(), run);
    if !path.try_exists().map_err(|_| "ReadFailed")? {
        return Ok(());
    }
    if read::<String>(&path, 4096).is_ok_and(|value| value == run) {
        return Ok(());
    }
    let bytes = marker_bytes(&path)?;
    preserve_marker(node.root(), &key(run), &bytes)
}
pub(super) fn persisted(node: &Node, run: &str) -> Result<(), String> {
    let path = pending_issue(node.root(), &key(run));
    if path.try_exists().map_err(|_| "ReadFailed")? {
        pending_record(node.root(), &key(run), "Resolved", true, false)?;
    }
    Ok(())
}
