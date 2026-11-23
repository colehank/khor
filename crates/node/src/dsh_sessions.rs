//! The conversations DeepSeek Harness already has on this machine.
//!
//! DSH stores one directory per session under a directory per project:
//!
//! ```text
//! ~/.dsh/sessions/<project>/<session>/session.v4.jsonl.zstd
//! ```
//!
//! The file is a sequence of zstd frames (one per flush), each holding JSON
//! lines. The **first line is the session header** — `{type:"session", id,
//! createdAt, cwd, parentSession?, origin?}` — and a later
//! `{"type":"session/title","data":{"title":…}}` line carries the name the
//! person sees. A subagent's session has `{"type":"subagent/descriptor",
//! "data":{"label":…}}` instead, which is the closest thing it has to a
//! title.
//!
//! So a listing decompresses the first frames and reads those lines: no
//! decoding of the directory name (which encodes a path with its own
//! escaping that nothing here should re-implement), and no parsing of the
//! message events — the transcript is a different job, and doing it badly
//! would be worse than not doing it.
//!
//! # Bounds
//!
//! Each file is read up to [`SCAN`] decompressed bytes, and the scan stops
//! as soon as a title from the provider (not the first-message fallback) is
//! found. Sixteen sessions on this machine cost a few tens of milliseconds;
//! one enormous session cannot make the roster slow.
//!
//! # Why `ruzstd`
//!
//! Decompression only, in Rust, no C toolchain: a build dependency that
//! needs a C compiler would put a toolchain requirement on every platform
//! khor ships to, for a read this crate does not otherwise need.

use std::io::Read;
use std::path::{Path, PathBuf};

/// How much decompressed text one session may contribute to the listing.
const SCAN: usize = 512 * 1024;

/// How much of a conversation is read at all. Four times the listing's
/// budget: a transcript is meant to be read, a name is not.
const WHOLE: usize = 4 * 1024 * 1024;

/// One DSH session, as its own events state it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct DshSession {
    /// `session-<uuid>` in newer stores, a bare uuid in older ones.
    pub id: String,
    /// The directory the session was recorded in — its project, for khor.
    pub cwd: String,
    pub created_at_ms: i64,
    /// What the person sees: the last `session/title` event, or a
    /// subagent's `descriptor.label`. `None` when the file has neither —
    /// a conversation that was started and never named.
    pub title: Option<String>,
    /// A subagent's session names the session it was delegated from.
    pub parent: Option<String>,
    /// `origin` when DSH wrote one (`"subagent"` today).
    pub origin: Option<String>,
    /// Where the file is, for a later reader.
    pub path: String,
}

/// The header line's fields this cares about. Unknown fields are ignored
/// — DSH's own format is the authority and it grows.
#[derive(serde::Deserialize)]
struct Header {
    #[serde(rename = "type")]
    kind: String,
    id: String,
    #[serde(rename = "createdAt", default)]
    created_at: i64,
    cwd: String,
    #[serde(rename = "parentSession", default)]
    parent_session: Option<String>,
    #[serde(default)]
    origin: Option<String>,
}

/// What one file's early events said.
#[derive(Default)]
struct Scan {
    header: Option<Header>,
    /// The last title seen, and whether a provider wrote it (better than
    /// the fallback title, which is just the first message's opening).
    title: Option<String>,
    titled_by_provider: bool,
    label: Option<String>,
}

impl Scan {
    /// The best name this session has: a provider title, then a subagent
    /// label, then DSH's fallback title.
    fn name(&self) -> Option<String> {
        if self.titled_by_provider {
            return self.title.clone();
        }
        self.label.clone().or_else(|| self.title.clone())
    }
}

/// Every session in this machine's DSH home, newest first.
///
/// `DSH_HOME` decides the home when it is set (the CLI honours it, so a
/// person running an isolated profile sees that profile); otherwise
/// `~/.dsh`. A missing home is an empty list, not an error: most machines
/// do not run DSH at all.
pub fn sessions() -> Result<Vec<DshSession>, String> {
    let Some(home) = home() else {
        return Ok(Vec::new());
    };
    sessions_in(&home)
}

/// [`sessions`] against a stated home, so the scan is testable without
/// this machine's own DSH deciding the answer.
pub(crate) fn sessions_in(home: &Path) -> Result<Vec<DshSession>, String> {
    let root = home.join("sessions");
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for project in dirs(&root) {
        for session in dirs(&project) {
            // Newer stores name the file `session.v4.jsonl.zstd`; a
            // directory without one is not a session.
            let Some(file) = file_in(&session) else {
                continue;
            };
            let Ok(scan) = read_session(&file) else {
                // **A file this cannot read is skipped, not fatal.** One
                // corrupt or newer-format session must not hide the rest,
                // and there is no sentence a list could show per file that
                // would be worth more than the other fifteen rows.
                continue;
            };
            let Some(ref header) = scan.header else {
                continue;
            };
            if header.kind != "session" {
                continue;
            }
            out.push(DshSession {
                id: header.id.clone(),
                cwd: header.cwd.clone(),
                created_at_ms: header.created_at,
                title: scan.name(),
                parent: header.parent_session.clone(),
                origin: header.origin.clone(),
                path: file.display().to_string(),
            });
        }
    }
    out.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
    Ok(out)
}

/// The whole conversation behind one session id, decompressed.
///
/// Answers the text the transcript reader parses: every frame, up to
/// [`WHOLE`] decompressed bytes — a session longer than that is truncated
/// rather than refused, because half a conversation is still readable and
/// the alternative is nothing.
pub fn conversation(native_id: &str) -> Result<String, String> {
    let file = file_for(native_id)
        .ok_or_else(|| format!("this machine has no DSH session {native_id}"))?;
    transcript_at(&file)
}

/// Where one session's file is.
///
/// **The directory is named after the session id** (newer stores
/// `session-<uuid>`, older ones a bare uuid), so the usual lookup is one
/// path probe per project and needs no decompression at all. A store that
/// ever names directories differently falls back to matching the headers
/// — the same scan [`sessions`] does.
pub(crate) fn file_for(native_id: &str) -> Option<PathBuf> {
    if native_id.is_empty() || native_id.contains(['/', '\\']) || native_id.contains("..") {
        return None;
    }
    let home = home()?;
    let root = home.join("sessions");
    for project in dirs(&root) {
        if let Some(file) = file_in(&project.join(native_id)) {
            return Some(file);
        }
    }
    for project in dirs(&root) {
        for session in dirs(&project) {
            let Some(file) = file_in(&session) else {
                continue;
            };
            let Ok(scan) = read_session(&file) else {
                continue;
            };
            if scan
                .header
                .as_ref()
                .is_some_and(|header| header.id == native_id)
            {
                return Some(file);
            }
        }
    }
    None
}

/// One session file, decompressed. The reader's half of
/// [`conversation`], split out so a caller that already has the path (a
/// content source resolving by id) does not scan the store twice.
pub(crate) fn transcript_at(file: &Path) -> Result<String, String> {
    let bytes = decompress(file, WHOLE)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// The directory one session recorded, from its header frame alone.
pub(crate) fn cwd_of(file: &Path) -> Option<String> {
    read_session(file).ok()?.header.map(|header| header.cwd)
}

/// One session by id, for a caller that has the id and needs its facts
/// (an import naming a candidate): the same locator, one small read.
pub(crate) fn session_of(native_id: &str) -> Option<DshSession> {
    let file = file_for(native_id)?;
    let scan = read_session(&file).ok()?;
    let header = scan.header.as_ref()?;
    if header.kind != "session" {
        return None;
    }
    Some(DshSession {
        id: header.id.clone(),
        cwd: header.cwd.clone(),
        created_at_ms: header.created_at,
        title: scan.name(),
        parent: header.parent_session.clone(),
        origin: header.origin.clone(),
        path: file.display().to_string(),
    })
}

fn home() -> Option<PathBuf> {
    if let Some(told) = std::env::var_os("DSH_HOME") {
        return Some(PathBuf::from(told));
    }
    std::env::home_dir().map(|home| home.join(".dsh"))
}

fn dirs(path: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    read.filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

fn file_in(session: &Path) -> Option<PathBuf> {
    let read = std::fs::read_dir(session).ok()?;
    read.filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("session.") && name.ends_with(".jsonl.zstd"))
        })
}

/// The header and the name, from the first [`SCAN`] decompressed bytes.
fn read_session(file: &Path) -> Result<Scan, String> {
    let text = decompress(file, SCAN)?;
    Ok(scan(&String::from_utf8_lossy(&text), &mut |scan| {
        scan.titled_by_provider
    }))
}

/// Reads lines out of `text` into a [`Scan`], calling `stop` after each one
/// to ask whether enough was read.
///
/// Split from decompression so the reading rules are testable on plain
/// text: which title wins, and when a scan may stop, are the decisions
/// here — not the zstd frames.
fn scan(text: &str, stop: &mut impl FnMut(&Scan) -> bool) -> Scan {
    let mut found = Scan::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match event.get("type").and_then(|t| t.as_str()) {
            Some("session") if found.header.is_none() => {
                if let Ok(header) = serde_json::from_value::<Header>(event) {
                    found.header = Some(header);
                }
            }
            Some("session/title") => {
                if let Some(title) = event.pointer("/data/title").and_then(|t| t.as_str()) {
                    let provider = event
                        .pointer("/data/source/kind")
                        .and_then(|k| k.as_str())
                        .is_some_and(|kind| kind == "provider");
                    // **The last title is the current one**, and a provider
                    // title beats DSH's first-message fallback even when the
                    // fallback was written later.
                    if provider {
                        found.titled_by_provider = true;
                        found.title = Some(title.to_owned());
                    } else if found.title.is_none() {
                        found.title = Some(title.to_owned());
                    }
                }
            }
            Some("subagent/descriptor") => {
                if let Some(label) = event.pointer("/data/label").and_then(|l| l.as_str()) {
                    found.label = Some(label.to_owned());
                }
            }
            _ => {}
        }
        if stop(&found) {
            break;
        }
    }
    found
}

/// All frames, up to `limit` decompressed bytes.
///
/// The file is a sequence of frames; reading one stops at its end, so this
/// walks the file frame by frame and asks the decoder how far it got.
fn decompress(file: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(file).map_err(|error| error.to_string())?;
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() && out.len() < limit {
        let cursor = std::io::Cursor::new(&bytes[at..]);
        let mut decoder =
            ruzstd::decoding::StreamingDecoder::new(cursor).map_err(|error| error.to_string())?;
        let mut buffer = [0u8; 32 * 1024];
        loop {
            match decoder.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    out.extend_from_slice(&buffer[..count]);
                    if out.len() >= limit {
                        break;
                    }
                }
                // A frame that ends mid-way is the last one most of the
                // time: keep what was read and stop.
                Err(_) => break,
            }
        }
        let used = decoder.into_inner().position() as usize;
        if used == 0 {
            break;
        }
        at += used;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = r#"{"type":"session","version":4,"id":"session-0b5f9684","createdAt":1794729585272,"cwd":"F:\\#Gong\\#AI4S\\khor","parentSession":"session-1d61","isSeeded":false,"origin":"subagent","agentPreset":"standard"}"#;
    const FALLBACK_TITLE: &str = r#"{"type":"session/title","seq":14,"time":1,"data":{"title":"You are a talented Windows","messageSeqs":[8],"source":{"kind":"fallback"}}}"#;
    const PROVIDER_TITLE: &str = r#"{"type":"session/title","seq":18,"time":2,"data":{"title":"Clarifying questions for agent-delete safety design","messageSeqs":[8],"source":{"kind":"provider","provider":"session-title-all-prompts-llm"}}}"#;
    const LABEL: &str = r#"{"type":"subagent/descriptor","seq":0,"time":1,"data":{"version":3,"mode":"continuable","label":"Design doc buildability check"}}"#;

    fn scan_all(text: &str) -> Scan {
        scan(text, &mut |_| false)
    }

    /// The header is parsed from its fields, not by position: DSH adds
    /// fields over time and a reader that assumed an order would break on
    /// the next release.
    #[test]
    fn a_header_is_read_from_its_fields() {
        let header = scan_all(HEADER).header.expect("a real header shape");
        assert_eq!(header.kind, "session");
        assert_eq!(header.id, "session-0b5f9684");
        assert_eq!(header.created_at, 1794729585272);
        assert_eq!(header.cwd, r"F:\#Gong\#AI4S\khor");
        assert_eq!(header.parent_session.as_deref(), Some("session-1d61"));
        assert_eq!(header.origin.as_deref(), Some("subagent"));
    }

    /// A provider title is the name; it wins over the fallback even though
    /// the scan sees the fallback first.
    #[test]
    fn the_provider_title_is_the_name() {
        let found = scan_all(&format!("{HEADER}\n{FALLBACK_TITLE}\n{PROVIDER_TITLE}"));
        assert_eq!(
            found.name().as_deref(),
            Some("Clarifying questions for agent-delete safety design")
        );
    }

    /// A subagent has no title, and its descriptor's label is the closest
    /// thing it has to one.
    #[test]
    fn a_subagents_label_stands_in_for_a_title() {
        let found = scan_all(&format!("{HEADER}\n{LABEL}\n{FALLBACK_TITLE}"));
        assert_eq!(
            found.name().as_deref(),
            Some("Design doc buildability check")
        );
    }

    /// A conversation that was started and never named has no name — the
    /// scan says so rather than inventing one from the id.
    #[test]
    fn an_unnamed_session_has_no_name() {
        assert!(scan_all(HEADER).name().is_none());
    }

    /// The scan may stop early, and the stop is asked after every event.
    #[test]
    fn the_scan_stops_when_asked() {
        let mut seen = 0;
        let found = scan(
            &format!("{HEADER}\n{PROVIDER_TITLE}\n{FALLBACK_TITLE}"),
            &mut |scan| {
                seen += 1;
                scan.titled_by_provider
            },
        );
        assert_eq!(
            seen, 2,
            "asked once per event, and stopped at the provider title"
        );
        assert_eq!(
            found.name().as_deref(),
            Some("Clarifying questions for agent-delete safety design")
        );
    }
}

/// End-to-end against **this machine's own DSH store**: an import, the row
/// it makes, and the content the reader resolves. Ignored by default —
/// CI has no DSH store — and run with:
///
/// ```text
/// KHOR_DSH_E2E=1 cargo test -p khor-node --lib dsh_observation_e2e -- --ignored --nocapture
/// ```
#[cfg(test)]
mod dsh_observation_e2e {
    use super::*;

    #[test]
    #[ignore = "needs a real DSH store on this machine"]
    fn a_conversation_imports_as_an_observation_and_resolves_to_a_readable_transcript() {
        if std::env::var_os("KHOR_DSH_E2E").is_none() {
            return;
        }
        let root = std::env::temp_dir().join(format!("khor-dsh-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let node = crate::Node::open(root.clone()).expect("node");
        let sessions = sessions().expect("this machine's DSH store");
        let session = sessions
            .iter()
            .find(|session| session.title.is_some())
            .expect("a named conversation");
        let candidate = format!("{}{}", crate::DSH_CANDIDATE, session.id);
        let row = node.import(&candidate).expect("import the candidate");

        // The row is the observation: its own title, its project, and the
        // category the content road keys on.
        let views = node.every_session().expect("every session");
        let view = views
            .into_iter()
            .find(|view| view.session.id == row)
            .expect("the imported row is on the list");
        assert_eq!(Some(view.session.title.clone()), session.title.clone());
        assert_eq!(view.session.place.as_deref(), Some(session.cwd.as_str()));
        assert_eq!(view.session.category.as_deref(), Some("dsh"));

        // The content road answers Available, and the snapshot it wrote
        // parses into lines with the conversation's own speaker.
        assert!(matches!(
            node.session_content(&row, None).expect("content"),
            crate::session_content::Availability::Available { .. }
        ));
        let snapshot = root
            .join(".khor")
            .join("cache")
            .join("dsh")
            .join(format!("{}.jsonl", crate::live::clean_leaf(&session.id)));
        let text = std::fs::read_to_string(&snapshot).expect("decompressed snapshot");
        let lines = crate::adaptor::dsh::lines_of(&text);
        assert!(!lines.is_empty(), "the snapshot has no readable lines");
        assert!(
            lines
                .iter()
                .any(|line| line.who == "user" || line.who == "assistant"),
            "the transcript has a speaker"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
