//! Durable receipts for explicit command submissions. Replication only
//! carries receipts: it never schedules work. A disk claim is written
//! before starting a command so a lost HTTP answer cannot run it twice.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use khor_catalog::msg;
use khor_node::{CommandRecord, Node};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
pub struct ConsoleCommand {
    pub word: String,
    pub aliases: Vec<String>,
    pub usage: String,
    pub reach: String,
    pub mode: String,
    pub parameters: Vec<String>,
}

/// Dispatch a terminal-owned stream before starting any window or server.
pub fn main_if_requested(start_web: fn(&Path)) -> Option<i32> {
    khor_commands::console_main_if_requested(start_web)
}

pub fn catalog() -> Vec<ConsoleCommand> {
    khor_commands::catalog()
        .into_iter()
        .map(|c| ConsoleCommand {
            word: c.word,
            aliases: c.aliases,
            usage: c.usage,
            reach: c.reach,
            mode: c.mode,
            parameters: c.parameters,
        })
        .collect()
}

#[derive(Clone, Serialize, Deserialize)]
struct Stored {
    fingerprint: String,
    pid: u32,
    process_started: i64,
    private: bool,
    record: CommandRecord,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn process_started() -> i64 {
    static START: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
    *START.get_or_init(|| {
        khor_node::adaptor::Procs::snapshot()
            .get(std::process::id())
            .map_or(0, |p| p.started_ms)
    })
}

fn safe_id(id: &str) -> bool {
    (16..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn path(root: &Path, id: &str) -> PathBuf {
    root.join(".khor/console").join(format!("{id}.json"))
}

fn lock(root: &Path, id: &str) -> Result<std::fs::File, String> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path(root, id).with_extension("lock"))
        .map_err(|e| e.to_string())?;
    file.lock().map_err(|e| e.to_string())?;
    Ok(file)
}

fn checkpoint(root: &Path, stored: &Stored) -> Result<(), String> {
    let _lock = lock(root, &stored.record.id)?;
    save(root, stored)?;
    publish(root, stored)
}

fn read(root: &Path, id: &str) -> Result<Stored, String> {
    if !safe_id(id) {
        return Err(msg::console_token_invalid().to_owned());
    }
    serde_json::from_slice(&fs::read(path(root, id)).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

fn save(root: &Path, stored: &Stored) -> Result<(), String> {
    let target = path(root, &stored.record.id);
    let tmp = target.with_extension("tmp");
    khor_node::link::write_private(
        &tmp,
        &serde_json::to_vec(stored).map_err(|e| e.to_string())?,
    )?;
    std::fs::File::open(&tmp)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    fs::rename(tmp, &target).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    std::fs::File::open(target.parent().expect("receipt directory"))
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn public_record(stored: &Stored) -> CommandRecord {
    let mut r = stored.record.clone();
    if stored.private {
        r.stdout = msg::console_private().to_owned();
        r.stderr.clear();
        r.data_json = None;
        // An error may echo an argument, such as a rejected credential.
        r.error = r.error.map(|_| msg::console_private().to_owned());
    }
    r
}

fn publish(root: &Path, stored: &Stored) -> Result<(), String> {
    Node::open(root.to_path_buf())?.record_command(&public_record(stored))?;
    Ok(())
}

/// The request token belongs to exactly this invocation. It is not an
/// execution queue: an interrupted operation requires a new submission.
pub fn run(root: &Path, line: &str, token: &str, input: Option<&str>) -> Result<String, String> {
    if !(16..=64).contains(&token.len()) || !safe_id(token) {
        return Err(msg::console_token_invalid().to_owned());
    }
    let argv = khor_commands::parse_line(line)?;
    let word = argv.first().ok_or_else(|| msg::console_unknown(""))?;
    if !khor_commands::catalog()
        .iter()
        .any(|c| c.word == *word || c.aliases.contains(word))
    {
        return Err(msg::console_unknown(word));
    }
    let node = Node::open(root.to_path_buf())?;
    let id = format!("cmd-{}-{token}", &node.device_str()[..16]);
    let fingerprint = blake3::hash(&serde_json::to_vec(&(line, input)).map_err(|e| e.to_string())?)
        .to_hex()
        .to_string();
    let dir = root.join(".khor/console");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let _submission_lock = lock(root, &id)?;
    if path(root, &id).try_exists().map_err(|e| e.to_string())? {
        let prior = read(root, &id)?;
        if prior.fingerprint != fingerprint {
            return Err(msg::console_token_conflict().to_owned());
        }
        publish(root, &prior)?;
        return Ok(id);
    }
    let private = input.is_some() || khor_commands::private_invocation(&argv);
    let record = CommandRecord {
        id: id.clone(),
        line: if private {
            format!("/{word} …")
        } else {
            line.to_owned()
        },
        owner: node.device_str().to_owned(),
        owner_name: node.shown_name().to_owned(),
        status: "running".to_owned(),
        started_at_ms: now(),
        ended_at_ms: None,
        stdout: String::new(),
        stderr: String::new(),
        data_json: None,
        session: None,
        error: None,
    };
    let mut stored = Stored {
        fingerprint,
        pid: std::process::id(),
        process_started: process_started(),
        private,
        record,
    };
    save(root, &stored)?;
    if let Err(e) = publish(root, &stored) {
        stored.record.status = "interrupted".to_owned();
        stored.record.ended_at_ms = Some(now());
        stored.record.error = Some(e.clone());
        save(root, &stored)?;
        return Err(e);
    }
    // A failed read receipt must not leave a claimed command unstarted.
    let _ = node.seen_snapshot(
        &khor_node::SessionId(khor_node::wide::BROADCAST.to_owned()),
        &[khor_node::SeenMessage {
            id: id.clone(),
            at_ms: stored.record.started_at_ms,
        }],
    );
    let recovery = stored.clone();
    let task_root = root.to_path_buf();
    let input = input.map(str::to_owned);
    let spawned = std::thread::Builder::new()
        .name("khor-command".to_owned())
        .spawn(move || {
            let root = task_root;
            let progress = std::sync::Arc::new(std::sync::Mutex::new((
                stored.clone(),
                std::time::Instant::now(),
            )));
            let progress_root = root.clone();
            let sink: khor_commands::ProgressSink = std::sync::Arc::new(move |event| {
                let Ok(mut progress) = progress.lock() else {
                    return;
                };
                let stream = if event.stream == "stderr" {
                    &mut progress.0.record.stderr
                } else {
                    &mut progress.0.record.stdout
                };
                stream.push_str(&event.text);
                *stream = bounded(std::mem::take(stream));
                if progress.1.elapsed() >= std::time::Duration::from_millis(500) {
                    let _ = checkpoint(&progress_root, &progress.0);
                    progress.1 = std::time::Instant::now();
                }
            });
            let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                khor_commands::execute_with(&root, &argv, input.as_deref(), Some(sink))
            }));
            match answer {
                Ok(out) => {
                    stored.record.status =
                        if out.exit_code == 0 { "done" } else { "failed" }.to_owned();
                    stored.record.stdout = bounded(out.stdout);
                    stored.record.stderr = bounded(out.stderr);
                    stored.record.data_json = out
                        .data
                        .and_then(|v| serde_json::to_string(&v).ok())
                        .filter(|s| s.len() <= OUTPUT_MAX);
                    stored.record.session = out.session;
                    stored.record.error = out.error;
                }
                Err(_) => {
                    stored.record.status = "interrupted".to_owned();
                    stored.record.error = Some(msg::console_interrupted().to_owned());
                }
            }
            stored.record.ended_at_ms = Some(now().max(stored.record.started_at_ms + 1));
            // Keep the completed answer first. A publish failure can be
            // healed by a later read without executing anything again.
            let _ = checkpoint(&root, &stored);
        });
    if let Err(e) = spawned {
        let mut failed = recovery;
        failed.record.status = "failed".to_owned();
        failed.record.error = Some(e.to_string());
        failed.record.ended_at_ms = Some(now());
        save(root, &failed)?;
        publish(root, &failed)?;
        return Err(e.to_string());
    }
    Ok(id)
}

const OUTPUT_MAX: usize = 128 * 1024;
fn bounded(mut value: String) -> String {
    if value.len() > OUTPUT_MAX {
        let mut end = OUTPUT_MAX;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value.truncate(end);
        value.push('\n');
        value.push_str(msg::console_output_cut());
    }
    value
}

/// Reconcile receipts owned by this node. Peers never retry them. A
/// restarted face reports an interrupted executor instead of an eternal
/// spinner; final receipts can heal a lost CRDT flush.
pub fn reconcile(root: &Path, node: &Node, messages: &mut [khor_node::Message]) {
    let mut procs = None;
    for m in messages {
        let Some(record) = m.command.as_ref() else {
            continue;
        };
        if record.owner != node.device_str() {
            continue;
        }
        if !safe_id(&record.id) {
            continue;
        }
        let Ok(_lock) = lock(root, &record.id) else {
            continue;
        };
        let Ok(mut stored) = read(root, &record.id) else {
            continue;
        };
        if stored.record.status == "running"
            && !(stored.pid == std::process::id() && stored.process_started == process_started())
        {
            let table = procs.get_or_insert_with(khor_node::adaptor::Procs::snapshot);
            if table
                .alive_since(stored.pid, stored.process_started)
                .is_none()
            {
                stored.record.status = "interrupted".to_owned();
                stored.record.error = Some(msg::console_interrupted().to_owned());
                stored.record.ended_at_ms = Some(now());
                let _ = save(root, &stored);
            }
        }
        let public = public_record(&stored);
        if public != *record {
            let _ = node.record_command(&public);
        }
        m.at = stored
            .record
            .ended_at_ms
            .unwrap_or(stored.record.started_at_ms);
        // Private output is read only on its executing node. The wire
        // always receives the redacted public receipt above.
        m.command = Some(stored.record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn scratch(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!(
            "khor-console-{tag}-{}-{}",
            std::process::id(),
            now()
        ));
        fs::create_dir_all(p.join(".khor")).unwrap();
        fs::write(
            p.join(".khor/relays.json"),
            r#"{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}"#,
        )
        .unwrap();
        Scratch(p)
    }
    fn finished(root: &Path, id: &str) -> Stored {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Ok(s) = read(root, id) {
                if s.record.status != "running" {
                    return s;
                }
            }
            assert!(
                std::time::Instant::now() < end,
                "command must finish within the deadline"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    #[test]
    fn a_retry_returns_the_original_receipt_and_never_repeats_a_write() {
        let root = scratch("once");
        let file = root.0.join("only-once.txt");
        let line = format!("/touch '{}'", file.display());
        let id = run(&root.0, &line, "one-submission-12345678", None).unwrap();
        let first = finished(&root.0, &id);
        assert_eq!(first.record.status, "done", "{:?}", first.record.error);
        assert!(file.is_file());
        assert_eq!(
            run(&root.0, &line, "one-submission-12345678", None).unwrap(),
            id
        );
        assert_eq!(read(&root.0, &id).unwrap().record, first.record);
        assert!(run(&root.0, "/version", "one-submission-12345678", None).is_err());
        let node = Node::open(root.0.clone()).unwrap();
        let log = node.log(khor_node::wide::BROADCAST).unwrap();
        assert_eq!(log.messages.iter().filter(|m| m.id == id).count(), 1);
    }
    #[test]
    fn a_dead_executor_is_reported_without_replaying_its_command() {
        let root = scratch("restart");
        let id = run(&root.0, "/version", "dead-executor-12345678", None).unwrap();
        let mut saved = finished(&root.0, &id);
        saved.pid = u32::MAX;
        saved.process_started = 0;
        saved.record.status = "running".into();
        saved.record.ended_at_ms = None;
        saved.record.line = "/touch should-never-exist".into();
        save(&root.0, &saved).unwrap();
        publish(&root.0, &saved).unwrap();
        let node = Node::open(root.0.clone()).unwrap();
        let mut log = node.log(khor_node::wide::BROADCAST).unwrap();
        reconcile(&root.0, &node, &mut log.messages);
        assert_eq!(read(&root.0, &id).unwrap().record.status, "interrupted");
        assert!(!root.0.join("should-never-exist").exists());
    }
    #[test]
    fn credentials_are_local_and_the_replicated_receipt_is_redacted() {
        let root = scratch("private");
        let id = run(
            &root.0,
            "/lock pin",
            "private-input-abcdefgh",
            Some("123456\n"),
        )
        .unwrap();
        let saved = finished(&root.0, &id);
        assert_eq!(saved.record.status, "done", "{:?}", saved.record.error);
        assert!(saved.private);
        let node = Node::open(root.0.clone()).unwrap();
        let log = node.log(khor_node::wide::BROADCAST).unwrap();
        let public = log
            .messages
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .command
            .as_ref()
            .unwrap();
        assert_eq!(public.line, "/lock …");
        assert_eq!(public.stdout, msg::console_private());
        assert!(!serde_json::to_string(public).unwrap().contains("123456"));
    }
    #[test]
    fn output_limits_preserve_utf8() {
        let s = bounded("é".repeat(OUTPUT_MAX));
        assert!(s.ends_with(msg::console_output_cut()));
        assert!(s.len() < OUTPUT_MAX + 200);
    }
    #[test]
    fn receipt_paths_cannot_escape_the_store() {
        assert!(safe_id("cmd-0123456789abcdef-0123456789abcdef"));
        for s in ["../0123456789012345", "/tmp/01234567890123", "short"] {
            assert!(!safe_id(s));
        }
    }
}
