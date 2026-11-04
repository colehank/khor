use khor_core::State;
use std::collections::HashMap;
use std::fs::{File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime};

const MAX_TAIL: u64 = 2 * 1024 * 1024;
const MAX_FILES: usize = 128;

#[derive(PartialEq)]
struct Stamp {
    len: u64,
    modified: SystemTime,
    created: Option<SystemTime>,
    #[cfg(unix)]
    identity: (u64, u64),
}

impl Stamp {
    fn of(meta: &Metadata) -> Option<Self> {
        Some(Self {
            len: meta.len(),
            modified: meta.modified().ok()?,
            created: meta.created().ok(),
            #[cfg(unix)]
            identity: {
                use std::os::unix::fs::MetadataExt;
                (meta.dev(), meta.ino())
            },
        })
    }
}

struct Cached {
    stamp: Stamp,
    answer: Option<(State, i64)>,
    used: Instant,
}

#[derive(Default)]
struct Endings(HashMap<PathBuf, Cached>);

impl Endings {
    fn read(&mut self, path: &Path) -> Option<(State, i64)> {
        let mut file = File::open(path).ok()?;
        let stamp = Stamp::of(&file.metadata().ok()?)?;
        if let Some(cached) = self.0.get_mut(path).filter(|c| c.stamp == stamp) {
            cached.used = Instant::now();
            return cached.answer;
        }
        let from = stamp.len.saturating_sub(MAX_TAIL);
        file.seek(SeekFrom::Start(from)).ok()?;
        let mut bytes = Vec::new();
        (&mut file).take(stamp.len - from).read_to_end(&mut bytes).ok()?;
        let head = if from == 0 { 0 } else {
            bytes.iter().position(|&b| b == b'\n').map_or(bytes.len(), |i| i + 1)
        };
        let answer = ending(&bytes[head..]);
        if Stamp::of(&file.metadata().ok()?)? != stamp {
            return None;
        }
        if self.0.len() >= MAX_FILES && !self.0.contains_key(path) {
            let oldest = self.0.iter().min_by_key(|(_, c)| c.used).map(|(p, _)| p.clone());
            if let Some(oldest) = oldest { self.0.remove(&oldest); }
        }
        self.0.insert(path.to_owned(), Cached { stamp, answer, used: Instant::now() });
        answer
    }
}

fn ending(bytes: &[u8]) -> Option<(State, i64)> {
    let end = bytes.iter().rposition(|&b| b == b'\n')?;
    for line in bytes[..end].split(|&b| b == b'\n').rev() {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else { continue };
        if value["isSidechain"].as_bool() == Some(true) { continue; }
        match value["type"].as_str() {
            Some("user") => return None,
            Some("system") if value["subtype"] == "compact_boundary" => return None,
            Some("assistant") => {
                let word = if value["isApiErrorMessage"].as_bool() == Some(true) {
                    State::Errored
                } else if value["message"]["stop_reason"] == "end_turn" {
                    State::Done
                } else { return None };
                let at = value["timestamp"].as_str()?.parse::<jiff::Timestamp>().ok()?.as_millisecond();
                return Some((word, at));
            }
            _ => {}
        }
    }
    None
}

pub(super) fn last_ending(path: &Path) -> Option<(State, i64)> {
    static CACHE: OnceLock<Mutex<Endings>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Endings::default())).lock().ok()?.read(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }
    const AT: &str = "2026-11-04T12:00:00Z";
    fn record(kind: &str, rest: serde_json::Value) -> String {
        let mut value = serde_json::json!({"type":kind,"timestamp":AT});
        value.as_object_mut().unwrap().extend(rest.as_object().unwrap().clone());
        format!("{value}\n")
    }
    fn done() -> String { record("assistant", serde_json::json!({"message":{"stop_reason":"end_turn"}})) }
    #[test]
    fn native_completion_survives_bookkeeping_but_not_a_new_turn_or_cancel() {
        let complete = done();
        let at = AT.parse::<jiff::Timestamp>().unwrap().as_millisecond();
        assert_eq!(ending(complete.as_bytes()), Some((State::Done, at)));
        let bookkeeping = record("system", serde_json::json!({"subtype":"turn_duration"}));
        assert_eq!(ending(format!("{complete}{bookkeeping}").as_bytes()), Some((State::Done, at)));
        for newer in [
            record("user", serde_json::json!({"message":{"content":"next"}})),
            record("user", serde_json::json!({"message":{"content":"[Request interrupted by user]"}})),
            record("assistant", serde_json::json!({"message":{"stop_reason":"tool_use"}})),
            record("assistant", serde_json::json!({"message":{"content":[]}})),
            record("system", serde_json::json!({"subtype":"compact_boundary"})),
        ] {
            assert_eq!(ending(format!("{complete}{newer}").as_bytes()), None);
        }
        assert_eq!(ending(bookkeeping.as_bytes()), None);
        assert_eq!(ending(complete.trim_end().as_bytes()), None, "an incomplete line is revisited");
    }
    #[test]
    fn api_errors_are_not_completions_and_sidechains_do_not_own_the_parent() {
        let error = record("assistant", serde_json::json!({"isApiErrorMessage":true,"message":{"stop_reason":"end_turn"}}));
        assert_eq!(ending(error.as_bytes()).unwrap().0, State::Errored);
        let side = record("assistant", serde_json::json!({"isSidechain":true,"message":{"stop_reason":"end_turn"}}));
        assert_eq!(ending(side.as_bytes()), None);
    }
    #[test]
    fn large_history_is_bounded_and_appends_and_replacements_invalidate_the_cache() {
        use std::io::Write;
        let dir = Scratch(std::env::temp_dir().join(format!("khor-native-ending-{}", std::process::id())));
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join("history.jsonl");
        let mut file = File::create(&path).unwrap();
        file.set_len(1024 * 1024 * 1024).unwrap();
        file.seek(SeekFrom::End(0)).unwrap();
        writeln!(file, "\n{}", done()).unwrap();
        let mut cache = Endings::default();
        assert_eq!(cache.read(&path).unwrap().0, State::Done);
        assert_eq!(cache.read(&path).unwrap().0, State::Done);
        file.write_all(record("user", serde_json::json!({})).as_bytes()).unwrap();
        assert_eq!(cache.read(&path), None);
        std::fs::write(&path, done()).unwrap();
        assert_eq!(cache.read(&path).unwrap().0, State::Done);
        std::fs::write(&path, "{\n").unwrap();
        assert_eq!(cache.read(&path), None);
    }
}
