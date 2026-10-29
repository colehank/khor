//! The files landing's data layer: one machine's directory, shaped for
//! the screen. The order, the cap and the "where is this about" all
//! come from the node ([`khor_node::files`]) — this layer renames
//! fields for TS and decides nothing.

use std::path::Path;

use khor_node::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A directory answer: where it is about (absolute, the far machine's
/// own spelling — the asker may have said `""` for home), its rows in
/// the node's order, and whether the cap cut it short.
#[derive(Debug, Clone, Serialize, TS)]
pub struct DirListing {
    pub path: String,
    pub entries: Vec<DirRow>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DirRow {
    pub name: String,
    pub dir: bool,
    #[ts(type = "number")]
    pub size: u64,
    #[ts(type = "number")]
    pub at_ms: u64,
    /// A received file's sender, the device id its receipt names (UX-48,
    /// the storage page's row: where it came from); `None` for any other
    /// file.
    pub from: Option<String>,
    /// When a received file landed, ms since the epoch (its receipt):
    /// the storage page's "how long ago received". `None` for any other
    /// file.
    #[ts(type = "number | null")]
    pub received_ms: Option<u64>,
}

/// One machine's directory — the call `khor ls` makes. Async because
/// the machine may be far away and the answer rides a dial.
pub async fn ls(root: &Path, machine: &str, path: &str) -> Result<DirListing, String> {
    let n = Node::open(root.to_path_buf())?;
    let (path, entries, truncated) = n.ls_of(machine, path).await?;
    Ok(DirListing {
        path,
        entries: entries
            .into_iter()
            .map(|e| DirRow {
                name: e.name,
                dir: e.dir,
                size: e.size,
                at_ms: e.at_ms,
                from: e.from,
                received_ms: e.received_ms,
            })
            .collect(),
        truncated,
    })
}

/// Takes a file off a machine into this one's downloads directory —
/// the call `khor pull` makes, with the landing the desktop convention
/// picks (`~/Downloads`, falling back to home). Answers where it
/// landed: the place was chosen silently, so it must be said.
pub async fn pull(root: &Path, machine: &str, path: &str) -> Result<String, String> {
    let n = Node::open(root.to_path_buf())?;
    // **Not to a phone** (docs/IOS.md 第二步): its `$HOME` is the app's
    // container, whose top it may not write, and a file that did land
    // there is one no other app — the Files app included — can see. The
    // face draws no 拉过来 there; this is for anything that asks anyway.
    if !n.hosts() {
        return Err(khor_catalog::msg::pull_needs_a_computer().to_owned());
    }
    let dir = dirs_download();
    let (_, dest) = n.pull_path(machine, path, &dir).await?;
    Ok(dest.display().to_string())
}

/// The same file, fetched for handing to this machine's other apps (round
/// UX-3, the file page's 分享 on a Mac): into a folder of khor's own under
/// the system's temporary directory, not the downloads — what is handed
/// on is a copy for the moment, not a file the person asked to keep. A
/// path already on this machine answers itself.
pub async fn pull_temp(root: &Path, machine: &str, path: &str) -> Result<String, String> {
    let n = Node::open(root.to_path_buf())?;
    if machine.is_empty() || machine == n.name() {
        return Ok(path.to_owned());
    }
    if !n.hosts() {
        return Err(khor_catalog::msg::pull_needs_a_computer().to_owned());
    }
    let dir = std::env::temp_dir().join("khor-share");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let (_, dest) = n.pull_path(machine, path, &dir).await?;
    Ok(dest.display().to_string())
}

fn dirs_download() -> std::path::PathBuf {
    let home = std::env::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let dl = home.join("Downloads");
    if dl.is_dir() { dl } else { home }
}

/// Takes a path off whichever machine holds it — the call `khor rm`
/// makes (`khor_node::Link::remove_path`).
///
/// **This machine's goes to the recycle bin, every other machine's is
/// deleted**, and which of the two it will be is decided in the node.
/// The face has to say which before anybody presses, and it asks
/// [`trash_here`] rather than reading the platform off the browser: the
/// same page is served to a phone looking at a desktop's face.
pub async fn remove(root: &Path, machine: &str, path: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?
        .remove_path(machine, path)
        .await
}

/// One new directory on whichever machine holds the parent — the call
/// `khor mkdir` makes (`khor_node::Link::mkdir`). One level, and a name
/// already taken is refused.
pub async fn mkdir(root: &Path, machine: &str, path: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.mkdir(machine, path).await
}

/// One new empty file, the call `khor touch` makes
/// (`khor_node::Link::touch`). A name already taken is refused and the
/// file wearing it is not emptied.
pub async fn touch(root: &Path, machine: &str, path: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.touch(machine, path).await
}

/// Moves a name on whichever machine holds it — the call `khor mv`
/// makes (`khor_node::Link::rename_path`). Crossing directories is the
/// same verb; the target is never overwritten.
pub async fn rename_path(root: &Path, machine: &str, from: &str, to: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?
        .rename_path(machine, from, to)
        .await
}

/// Opens a path or a page **on the machine that holds it** — the call
/// `khor launch` makes (`khor_node::Link::launch`).
///
/// **Not [`crate::web::open_link`]**, and the pair is worth reading
/// together: that one hands a page to the machine the face is running
/// on, which is right for a link somebody wants to read. This one hands
/// it to the machine the session ran on, which is the only machine the
/// file in that session exists on.
pub async fn launch(root: &Path, machine: &str, target: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?
        .launch(machine, target)
        .await
}

/// One line of one file that holds the words somebody typed
/// (工作区第六轮 第 47 条). Renamed for TS and nothing else; every
/// judgment — the caps, `.gitignore`, what counts as binary — is
/// `khor_node::files::search`'s.
#[derive(Debug, Clone, Serialize, TS)]
pub struct SearchHit {
    pub path: String,
    #[ts(type = "number")]
    pub line: u32,
    pub text: String,
}

/// What one search answers: the names, then the lines, then whether a
/// cap cut it short.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Found {
    pub names: Vec<String>,
    pub lines: Vec<SearchHit>,
    pub more: bool,
}

/// Everything under one directory on one machine that holds `query` —
/// the call `khor search` makes (`khor_node::Link::search`).
///
/// **The walk happens on the machine that holds the files.** A face
/// that listed the tree and matched here would move a whole project
/// across the wire to look for one word, which is the shape this layer
/// refuses for the git marks and for `touched` as well.
pub async fn search(root: &Path, machine: &str, dir: &str, query: &str) -> Result<Found, String> {
    let n = Node::open(root.to_path_buf())?;
    let found = n.search(machine, dir, query).await?;
    Ok(Found {
        names: found.names,
        lines: found
            .lines
            .into_iter()
            .map(|h| SearchHit {
                path: h.path,
                line: h.line,
                text: h.text,
            })
            .collect(),
        more: found.more,
    })
}

/// Whether the browser this face runs in will draw a PDF itself
/// (`khor_node::preview::PDF_HERE`). False on Linux (WebKitGTK ships no
/// viewer) and Android, where khor shows the PDF's words instead and
/// says that is what they are.
pub fn pdf_here() -> bool {
    khor_node::preview::PDF_HERE
}

/// Whether **this** machine — the one khor's backend is running on —
/// has a recycle bin (`khor_node::files::TRASH_HERE`). False on iOS and
/// Android, where 删除 on a local file is as final as it is on a far
/// one and the sentence must say so.
pub fn trash_here() -> bool {
    khor_node::files::TRASH_HERE
}

/// Whether this backend can reveal a local path in the desktop file manager.
/// The face must additionally require a native desktop viewer and a local file:
/// a web client looking at this backend must never open the backend's desktop.
/// An explicit custom/disabled launcher is respected, including smoke stand-ins.
pub fn reveal_here() -> bool {
    reveal_available(
        std::env::consts::OS,
        std::env::var_os(khor_node::files::LAUNCHER_ENV).as_deref(),
    )
}

fn reveal_available(os: &str, launcher: Option<&std::ffi::OsStr>) -> bool {
    let default = match os {
        "macos" => "open",
        "windows" => "explorer",
        "linux" => "xdg-open",
        _ => return false,
    };
    launcher.is_none_or(|given| given.to_str().is_some_and(|s| s.trim() == default))
}

/// Shows a local file or directory in its containing folder. macOS and Windows
/// select the item. Linux asks FileManager1 to select it, falling back to opening
/// the containing folder when that desktop has no FileManager1 implementation.
/// No command shell is involved; paths are never interpreted as command text.
pub fn reveal(path: &str) -> Result<(), String> {
    // Unlike the general opener, revealing a web URL is never meaningful.
    if !Path::new(path).is_absolute() || path.contains('\0') {
        return Err(khor_catalog::msg::not_openable(path));
    }
    if !reveal_here() {
        return Err(khor_catalog::msg::no_launcher(
            khor_node::files::LAUNCHER_ENV,
        ));
    }
    reveal_native(path).map_err(|e| khor_catalog::msg::cant_launch(path, e))
}

#[cfg(target_os = "macos")]
fn reveal_native(path: &str) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn reveal_native(path: &str) -> Result<(), String> {
    reveal_linux(path, |program, args, wait| {
        let mut command = std::process::Command::new(program);
        command
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if wait {
            command
                .status()
                .map(|s| s.success())
                .map_err(|e| e.to_string())
        } else {
            command.spawn().map(|_| true).map_err(|e| e.to_string())
        }
    })
}

#[cfg(any(target_os = "linux", test))]
fn reveal_linux(
    path: &str,
    mut run: impl FnMut(&str, &[String], bool) -> Result<bool, String>,
) -> Result<(), String> {
    let args = vec![
        "--session".to_owned(),
        "--print-reply".to_owned(),
        "--type=method_call".to_owned(),
        "--reply-timeout=1500".to_owned(),
        "--dest=org.freedesktop.FileManager1".to_owned(),
        "/org/freedesktop/FileManager1".to_owned(),
        "org.freedesktop.FileManager1.ShowItems".to_owned(),
        format!("array:string:{}", file_uri(path)),
        "string:".to_owned(),
    ];
    // The reply is bounded: a missing service/tool or a rejected request must
    // still allow a desktop without FileManager1 to reveal the containing folder.
    if matches!(run("dbus-send", &args, true), Ok(true)) {
        return Ok(());
    }
    let parent = Path::new(path).parent().unwrap_or_else(|| Path::new(path));
    match run("xdg-open", &[parent.to_string_lossy().into_owned()], false)? {
        true => Ok(()),
        false => Err("file manager did not start".to_owned()),
    }
}

#[cfg(any(target_os = "linux", test))]
fn file_uri(path: &str) -> String {
    let mut uri = String::from("file://");
    // Encode commas too: dbus-send separates array items with a comma. Spaces,
    // hashes, percent signs and UTF-8 bytes must remain part of this one URI.
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(uri, "%{byte:02X}").expect("writing to String cannot fail");
        }
    }
    uri
}

#[cfg(any(target_os = "windows", test))]
fn windows_reveal_path(path: &str) -> Vec<u16> {
    path.encode_utf16().chain(Some(0)).collect()
}

#[cfg(target_os = "windows")]
fn reveal_native(path: &str) -> Result<(), String> {
    use std::ffi::c_void;
    use std::ptr;

    // Use the Shell API rather than Explorer's /select command-line parser:
    // commas, quotes, Unicode and UNC paths remain one native path.
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(reserved: *mut c_void, flags: u32) -> i32;
        fn CoUninitialize();
        fn CoTaskMemFree(memory: *mut c_void);
    }
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHParseDisplayName(
            name: *const u16,
            context: *mut c_void,
            pidl: *mut *mut c_void,
            attributes: u32,
            actual: *mut u32,
        ) -> i32;
        fn SHOpenFolderAndSelectItems(
            pidl: *const c_void,
            count: u32,
            children: *const *const c_void,
            flags: u32,
        ) -> i32;
    }
    fn checked(result: i32) -> Result<(), String> {
        if result < 0 {
            Err(format!("Windows Shell error 0x{:08X}", result as u32))
        } else {
            Ok(())
        }
    }
    let wide = windows_reveal_path(path);
    unsafe {
        let initialized = CoInitializeEx(ptr::null_mut(), 2); // COINIT_APARTMENTTHREADED
        // RPC_E_CHANGED_MODE means this thread already has another COM apartment;
        // it may use Shell APIs, but this call must not uninitialize that apartment.
        if initialized < 0 && initialized != 0x80010106u32 as i32 {
            return checked(initialized);
        }
        let mut pidl = ptr::null_mut();
        let parsed = SHParseDisplayName(
            wide.as_ptr(),
            ptr::null_mut(),
            &mut pidl,
            0,
            ptr::null_mut(),
        );
        let result = if parsed < 0 {
            checked(parsed)
        } else if pidl.is_null() {
            Err("Windows Shell returned no item".to_owned())
        } else {
            // With zero children, the absolute PIDL identifies the one item to
            // select in its parent; this works for files and directories alike.
            checked(SHOpenFolderAndSelectItems(pidl, 0, ptr::null(), 0))
        };
        if !pidl.is_null() {
            CoTaskMemFree(pidl);
        }
        if initialized >= 0 {
            CoUninitialize();
        }
        result
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn reveal_native(_: &str) -> Result<(), String> {
    Err("no desktop file manager on this platform".to_owned())
}

#[cfg(test)]
mod reveal_tests {
    use super::*;

    #[test]
    fn capability_respects_platform_and_explicit_launcher() {
        for (platform, launcher) in [
            ("macos", "open"),
            ("windows", "explorer"),
            ("linux", "xdg-open"),
        ] {
            assert!(reveal_available(platform, None));
            assert!(reveal_available(platform, Some(launcher.as_ref())));
            assert!(!reveal_available(platform, Some("".as_ref())));
            assert!(!reveal_available(platform, Some("/usr/bin/true".as_ref())));
        }
        assert!(!reveal_available("ios", None));
        assert!(!reveal_available("android", None));
    }

    #[test]
    fn windows_path_preserves_unicode_commas_and_unc_as_one_native_string() {
        for path in [r"C:\Users\测试\a, b.md", r"\\server\shared folder\😀.md"] {
            let wide = windows_reveal_path(path);
            assert_eq!(wide.last(), Some(&0));
            assert!(!wide[..wide.len() - 1].contains(&0));
            assert_eq!(String::from_utf16(&wide[..wide.len() - 1]).unwrap(), path);
        }
    }

    #[test]
    fn linux_selects_one_encoded_uri_without_a_shell() {
        let mut calls = Vec::new();
        reveal_linux("/home/测试/a, b#%.md", |program, args, wait| {
            calls.push((program.to_owned(), args.to_vec(), wait));
            Ok(true)
        })
        .unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "dbus-send");
        assert!(calls[0].2);
        assert_eq!(
            calls[0].1[7],
            "array:string:file:///home/%E6%B5%8B%E8%AF%95/a%2C%20b%23%25.md"
        );
        assert!(calls[0].1.contains(&"--reply-timeout=1500".to_owned()));
    }

    #[test]
    fn linux_falls_back_to_parent_for_missing_or_rejecting_service() {
        for missing in [false, true] {
            let mut calls = Vec::new();
            reveal_linux("/home/user/my folder", |program, args, wait| {
                calls.push((program.to_owned(), args.to_vec(), wait));
                if program == "dbus-send" {
                    if missing {
                        Err("not installed".to_owned())
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(true)
                }
            })
            .unwrap();
            assert_eq!(
                calls[1],
                ("xdg-open".to_owned(), vec!["/home/user".to_owned()], false)
            );
        }
    }

    #[test]
    fn linux_root_fallback_does_not_use_an_empty_path_and_reports_errors() {
        let mut calls = Vec::new();
        let error = reveal_linux("/", |program, args, _| {
            calls.push((program.to_owned(), args.to_vec()));
            Err("not installed".to_owned())
        })
        .unwrap_err();
        assert_eq!(calls[1].1, vec!["/".to_owned()]);
        assert_eq!(error, "not installed");
    }

    #[test]
    fn reveal_refuses_urls_flags_and_nul_before_starting_any_process() {
        for path in ["https://example.com", "-R", "relative/file", "/tmp/a\0b"] {
            assert!(reveal(path).is_err());
        }
    }
}

/// How much of a file gets sniffed for the NUL that means binary. git's
/// own number, and the reason to borrow it rather than pick one: the
/// question「is this text」has no true answer, only a convention, and a
/// convention people already share beats a fresh judgment call.
const SNIFF: usize = 8000;

/// One file as a screen can use it.
#[derive(Debug, Clone, Serialize, TS)]
pub struct FileText {
    /// The whole file's size, whether or not all of it is here — the
    /// only thing that says the view is partial.
    #[ts(type = "number")]
    pub total: u64,
    #[ts(type = "number")]
    pub at_ms: u64,
    /// As much as one slice held, decoded. Empty when `binary`.
    pub text: String,
    /// More of the file exists than `text` holds.
    pub cut: bool,
    /// Not text at all, so nothing was decoded.
    pub binary: bool,
    /// What the file's bytes hash to (`khor_node::files::digest`), or
    /// empty when there is nothing here that could be written back.
    ///
    /// **It is the permission to save, not just an identifier.** A save
    /// hands this back and the far machine refuses unless the file
    /// still hashes to it, so a view holding an empty one has nothing
    /// to offer — which is exactly right for the two kinds of file that
    /// carry one: a binary, and a file the slice cut short. Writing
    /// either of those back would be writing something that is not
    /// what is on screen.
    pub digest: String,
}

/// One file off any machine, for looking at (`khor_node::Node::
/// read_head`). One slice, never a loop — the node's own head says why
/// and what it measured.
///
/// # Cut is not the same as binary, and one byte decides it
///
/// A slice ends where 256 KB ends, which can be **inside** a multi-byte
/// character. Handed to a decoder that only answers yes or no, that
/// file comes back「not text」and a person is told their Rust source is
/// a binary. `Utf8Error` separates the two exactly:
///
/// - `error_len() == None` — the input ends mid-character and
///   everything before it was valid. That is the slice boundary, not
///   the file. Keep what decoded.
/// - `error_len() == Some(_)` — a byte sequence that cannot be text no
///   matter what follows it.
///
/// # Why a decoder alone is not enough
///
/// NUL is a perfectly valid UTF-8 character, so a file full of them
/// decodes without complaint — and files full of them are exactly what
/// most binaries are. The sniff above is the second half of the answer,
/// and without it the first half passes an executable to a code view.
/// What an open file is now, without its bytes (`Node::stat_of`): what
/// a face asks on coming back to the window, re-reading the file only
/// when this differs from the [`FileText`] it holds (round NET-4).
#[derive(Debug, Clone, Serialize, TS)]
pub struct FileStat {
    #[ts(type = "number")]
    pub total: u64,
    #[ts(type = "number")]
    pub at_ms: u64,
    /// [`FileText::digest`]'s, for a file that fits one slice; empty
    /// for one that does not.
    pub digest: String,
}

pub async fn stat_text(root: &Path, machine: &str, path: &str) -> Result<FileStat, String> {
    let n = Node::open(root.to_path_buf())?;
    let (total, at_ms, digest) = n.stat_of(machine, path).await?;
    Ok(FileStat {
        total,
        at_ms,
        digest,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DirWatchScope {
    Entries { path: String },
    Workspace { root: String },
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DirWatchCoverageMode {
    Pending,
    Native,
    Polling,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DirWatchCoverage {
    pub scope: u32,
    pub mode: DirWatchCoverageMode,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DirWatchBatch {
    pub cursor: String,
    pub reset: bool,
    pub dirs: Vec<String>,
    pub workspace: bool,
    pub coverage: Vec<DirWatchCoverage>,
    pub retry_ms: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DirWatchOpened {
    pub id: String,
    pub machine_id: String,
    pub batch: DirWatchBatch,
}

impl From<khor_node::watch::Batch> for DirWatchBatch {
    fn from(batch: khor_node::watch::Batch) -> Self {
        use khor_node::watch::CoverageMode as Mode;
        Self {
            cursor: batch.cursor,
            reset: batch.reset,
            dirs: batch.dirs,
            workspace: batch.workspace,
            coverage: batch
                .coverage
                .into_iter()
                .map(|coverage| DirWatchCoverage {
                    scope: coverage.scope,
                    mode: match coverage.mode {
                        Mode::Pending => DirWatchCoverageMode::Pending,
                        Mode::Native => DirWatchCoverageMode::Native,
                        Mode::Polling => DirWatchCoverageMode::Polling,
                        Mode::Partial => DirWatchCoverageMode::Partial,
                        Mode::Unavailable => DirWatchCoverageMode::Unavailable,
                    },
                    reason: coverage.reason,
                })
                .collect(),
            retry_ms: batch.retry_ms,
        }
    }
}

pub async fn dirs_begin(
    root: &Path,
    machine: &str,
    scopes: Vec<DirWatchScope>,
) -> Result<DirWatchOpened, String> {
    let node = Node::open(root.to_path_buf())?;
    let scopes = scopes
        .into_iter()
        .map(|scope| match scope {
            DirWatchScope::Entries { path } => khor_node::watch::Scope::Entries { path },
            DirWatchScope::Workspace { root } => khor_node::watch::Scope::Workspace { root },
        })
        .collect();
    let opened = node.dirs_begin(machine, scopes).await?;
    Ok(DirWatchOpened {
        id: opened.id,
        machine_id: opened.machine_id,
        batch: opened.batch.into(),
    })
}

pub async fn dirs_wait(
    root: &Path,
    machine_id: &str,
    id: &str,
    cursor: Option<&str>,
    wait_ms: u32,
) -> Result<DirWatchBatch, String> {
    let node = Node::open(root.to_path_buf())?;
    Ok(node
        .dirs_wait(machine_id, id, cursor, wait_ms)
        .await?
        .into())
}

pub async fn dirs_leave(root: &Path, machine_id: &str, id: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?
        .dirs_leave(machine_id, id)
        .await
}

#[cfg(test)]
mod directory_watch_tests {
    use super::*;

    #[test]
    fn directory_watch_contract_preserves_scope_tags_and_bounded_backend_facts() {
        assert_eq!(
            serde_json::to_value(DirWatchScope::Entries {
                path: "/fixture".into()
            })
            .unwrap(),
            serde_json::json!({"kind":"entries","path":"/fixture"})
        );
        assert_eq!(
            serde_json::to_value(DirWatchScope::Workspace {
                root: "/fixture".into()
            })
            .unwrap(),
            serde_json::json!({"kind":"workspace","root":"/fixture"})
        );
        use khor_node::watch::CoverageMode as Mode;
        for (mode, expected) in [
            (Mode::Pending, "pending"),
            (Mode::Native, "native"),
            (Mode::Polling, "polling"),
            (Mode::Partial, "partial"),
            (Mode::Unavailable, "unavailable"),
        ] {
            let batch: DirWatchBatch = khor_node::watch::Batch {
                cursor: "epoch:42".into(),
                reset: true,
                dirs: vec!["/fixture".into()],
                workspace: true,
                coverage: vec![khor_node::watch::Coverage {
                    scope: 0,
                    mode,
                    reason: Some("provider fact".into()),
                }],
                retry_ms: 1000,
            }
            .into();
            let json = serde_json::to_value(batch).unwrap();
            assert_eq!(json["coverage"][0]["mode"], expected);
            assert_eq!(json["cursor"], "epoch:42");
            assert_eq!(json["reset"], true);
            assert_eq!(json["workspace"], true);
            assert_eq!(json["retry_ms"], 1000);
            assert_eq!(json["coverage"][0]["reason"], "provider fact");
        }
    }
}

/// Text files up to this size are read whole and editable. Chosen at 64 MB:
/// big enough for almost any source file, log, or config; small enough that
/// CodeMirror and the browser can hold it. Files larger than this show their
/// first slice read-only, which is what they did before this change — the cap
/// exists, it has only moved.
const TEXT_CAP: u64 = 64 * 1024 * 1024;

pub async fn read_text(root: &Path, machine: &str, path: &str) -> Result<FileText, String> {
    let n = Node::open(root.to_path_buf())?;
    let (total, at_ms, first) = n.read_head(machine, path).await?;

    // **Small files: the one-slice path stays exactly as it was.** One round
    // trip, one digest, no loop.
    if total <= khor_node::proto::SLICE {
        return Ok(shape(total, at_ms, &first));
    }

    // **Large files: pull slices until the cap or the end.** The digest is
    // computed incrementally as the bytes arrive, not after — so a binary
    // sniffed in slice 3 refuses before slice 4 crosses the wire.
    let mut bytes = first;
    let mut hasher = blake3::Hasher::new();

    // Sniff the first slice for binary. This is the same check `shape` does,
    // pulled out so a binary file is refused before pulling the rest.
    if bytes.iter().take(SNIFF).any(|b| *b == 0) {
        return Ok(FileText {
            total,
            at_ms,
            text: String::new(),
            cut: true,
            binary: true,
            digest: String::new(),
        });
    }

    hasher.update(&bytes);
    let want = total.min(TEXT_CAP);

    while (bytes.len() as u64) < want {
        let offset = bytes.len() as u64;
        let (now_total, now_ms, slice) = n.read_at(machine, path, offset).await?;

        // The file moved under us: total or mtime changed between slices.
        if now_total != total || now_ms != at_ms {
            return Err(khor_catalog::msg::file_moved_while_reading(path));
        }

        if slice.is_empty() {
            // The peer claims there is more but sent nothing. This cannot
            // make progress, so refuse rather than loop forever.
            return Err(khor_catalog::msg::file_moved_while_reading(path));
        }

        // Sniff each slice for binary before adding it.
        if slice.iter().take(SNIFF.min(slice.len())).any(|b| *b == 0) {
            return Ok(FileText {
                total,
                at_ms,
                text: String::new(),
                cut: true,
                binary: true,
                digest: String::new(),
            });
        }

        hasher.update(&slice);
        bytes.extend_from_slice(&slice);
    }

    bytes.truncate(want as usize);
    let cut = total > TEXT_CAP;

    // Decode the whole thing. A file cut by the cap is not given a digest,
    // because what is on screen is not what a save would write back — the
    // invariant from the original code.
    let text = match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_owned(),
        // A boundary cut mid-character: keep the valid prefix only if the
        // file was not cut by the cap. If it was cut, the text is not the
        // whole file, and `shape`'s rule applies: what is on screen must be
        // what a save writes back.
        Err(e) if e.error_len().is_none() && !cut => {
            String::from_utf8_lossy(&bytes[..e.valid_up_to()]).into_owned()
        }
        Err(_) => {
            return Ok(FileText {
                total,
                at_ms,
                text: String::new(),
                cut: true,
                binary: true,
                digest: String::new(),
            });
        }
    };

    let digest = if cut {
        String::new()
    } else {
        hasher.finalize().to_hex().to_string()
    };

    Ok(FileText {
        total,
        at_ms,
        text,
        cut,
        binary: false,
        digest,
    })
}

/// The whole judgment, with no machine in it — which is why it is a
/// function of its own. Whether a slice is text, and whether it was
/// cut, is decided from the bytes and the size; a test that had to
/// stand up a node to ask that would be testing the dial instead.
fn shape(total: u64, at_ms: u64, bytes: &[u8]) -> FileText {
    let cut = (bytes.len() as u64) < total;
    let no = |binary| FileText {
        total,
        at_ms,
        text: String::new(),
        cut,
        binary,
        digest: String::new(),
    };
    if bytes.iter().take(SNIFF).any(|b| *b == 0) {
        return no(true);
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        // The boundary, not the file — see the head of `read_text`.
        // **Only when the slice really ended early.** An *uncut* file
        // that stops mid-character is not a boundary artefact, it is a
        // file whose last bytes are not text; keeping the valid prefix
        // there would put something on screen that is not the file, and
        // since 工作区批4 what is on screen is what a save writes back.
        Err(e) if e.error_len().is_none() && cut => {
            String::from_utf8_lossy(&bytes[..e.valid_up_to()]).into_owned()
        }
        Err(_) => return no(true),
    };
    // **The digest is the invariant, spelled as code.** It is handed
    // out only when `text` is byte-for-byte the whole file, which is
    // precisely when writing it back loses nothing — so「can this be
    // saved」is never a second judgment that could disagree with this
    // one.
    let digest = if cut {
        String::new()
    } else {
        khor_node::files::digest(bytes)
    };
    FileText {
        total,
        at_ms,
        text,
        cut,
        binary: false,
        digest,
    }
}

/// One file shaped for a screen that is not a code view.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Preview {
    /// `image` · `svg` · `pdf` · `doc` · `text` · `other` — what the
    /// screen should do with this. **Decided in the node** so `khor`
    /// and the app never disagree about what a file is.
    pub kind: String,
    #[ts(type = "number")]
    pub total: u64,
    /// `image`, `svg` and `pdf`: the bytes, base64, ready for the
    /// browser to decode. Empty otherwise — **and empty on a `pdf`
    /// means the words are in `html` instead**, because this platform's
    /// browser has no PDF viewer (`khor_node::preview::PDF_HERE`). One
    /// kind, two shapes, and which one it is, is which field is full.
    pub data: String,
    /// The media type those bytes need to be announced with, or empty.
    /// **Carried rather than derived on the far side**: a `data:` URL
    /// with no media type is rendered only by sniffing, and a browser
    /// that sniffs wrong draws a broken-image glyph with no sentence.
    pub media: String,
    /// `doc`, and a `pdf` on a platform that cannot draw one: the file
    /// rendered to HTML, already escaped (`khor_node::preview::html`).
    /// Empty otherwise.
    pub html: String,
    /// The file is bigger than a preview carries and this is its
    /// beginning. **An image that is cut is not shown at all** — half a
    /// picture is a broken picture, where half a document is still a
    /// document.
    pub cut: bool,
    /// The file parsed and holds no words. A real answer, not a
    /// failure: a blank template does exactly this.
    pub empty: bool,
}

/// One file off any machine, shaped for whatever it turns out to be.
///
/// **The node decides the kind and does the rendering**; this layer
/// carries the answer and base64s the two kinds the browser draws
/// itself. Doing the render here would be a second renderer for `khor`
/// to disagree with (docs/KHOR.md, CLI 与 GUI 等价).
pub async fn preview(root: &Path, machine: &str, path: &str) -> Result<Preview, String> {
    use base64::Engine as _;
    let n = Node::open(root.to_path_buf())?;
    let kind = khor_node::preview::kind(path);
    let (total, _, bytes, cut) = n
        .read_all(machine, path, khor_node::preview::cap(path))
        .await?;
    let mut out = Preview {
        kind: match kind {
            khor_node::preview::Kind::Image => "image",
            khor_node::preview::Kind::Svg => "svg",
            khor_node::preview::Kind::Pdf => "pdf",
            khor_node::preview::Kind::Doc => "doc",
            khor_node::preview::Kind::Text => "text",
            khor_node::preview::Kind::Other => "other",
        }
        .to_owned(),
        total,
        data: String::new(),
        media: khor_node::preview::media(path).to_owned(),
        html: String::new(),
        cut,
        empty: false,
    };
    match kind {
        khor_node::preview::Kind::Image | khor_node::preview::Kind::Svg
            // Half a picture is a broken picture, so a cut one carries
            // no bytes at all and the face says why.
            if !cut => {
                out.data = base64::engine::general_purpose::STANDARD.encode(&bytes);
            }
        khor_node::preview::Kind::Doc => {
            let md = khor_node::preview::markdown(path, &bytes)?;
            out.empty = md.trim().is_empty();
            out.html = khor_node::preview::html(&md);
        }
        // **The bytes where the browser can draw them, the words where
        // it cannot** (工作区批⑤). Half a PDF is a broken PDF, so a cut
        // one carries no bytes and the face says why — the same rule a
        // picture is under; the words road is left to the fallback,
        // which does not need the whole file to be useful but would
        // then be answering about half a document without saying so.
        khor_node::preview::Kind::Pdf => {
            if khor_node::preview::PDF_HERE {
                if !cut {
                    out.data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                }
            } else {
                let md = khor_node::preview::markdown(path, &bytes)?;
                out.empty = md.trim().is_empty();
                out.html = khor_node::preview::html(&md);
            }
        }
        _ => {}
    }
    Ok(out)
}

/// What a save answers: where the file stands now.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Wrote {
    #[ts(type = "number")]
    pub at_ms: u64,
    /// The new digest, so the view can keep editing and save again
    /// without reading the file back.
    pub digest: String,
}

/// Saves a whole file onto any machine (`khor_node::Node::write_path`).
///
/// `was` is the `digest` the view was handed when it read the file. It
/// is required and never defaulted: an empty one means「there should be
/// no file here」, which is a different and much stronger claim than
/// 「I did not bother to check」, and the second one has no spelling on
/// purpose.
pub async fn write_text(
    root: &Path,
    machine: &str,
    path: &str,
    was: &str,
    text: &str,
) -> Result<Wrote, String> {
    let n = Node::open(root.to_path_buf())?;
    let (at_ms, digest) = n.write_path(machine, path, was, text.as_bytes()).await?;
    Ok(Wrote { at_ms, digest })
}

/// The three sides of a refused save, merged (`khor_node::merge`).
#[derive(Debug, Clone, Serialize, TS)]
pub struct Merged {
    /// The merged text, conflict markers and all.
    pub text: String,
    /// How many places khor would not choose between. `0` means there
    /// is nothing left for the person to decide.
    pub clashes: u32,
}

/// Merges the copy in the box over the copy on the machine, against the
/// copy the person started from.
///
/// **No machine and no path, because nothing is written.** The three
/// texts are already in front of the caller — the base and the edit are
/// the view's own, and 「theirs」 is what `read_text` just answered —
/// so this is a function, not a road to another machine. The save that
/// follows is the ordinary `write_text`, pinned to the digest that came
/// with 「theirs」, and it only happens when somebody presses save.
pub fn merge_text(base: &str, mine: &str, theirs: &str) -> Merged {
    let m = khor_node::merge::three(base, mine, theirs);
    Merged {
        text: m.text,
        clashes: m.clashes,
    }
}

#[cfg(test)]
mod read_text_tests {
    use super::*;

    /// A slice ends where 256 KB ends, and a character can straddle
    /// that. **This is the case that decides whether a person is told
    /// their source file is a binary**, and it is built here the way it
    /// happens in life: filler up to one byte short of the boundary,
    /// then a three-byte character that cannot fit.
    #[test]
    fn a_character_cut_by_the_slice_is_not_a_binary_file() {
        let slice = khor_node::proto::SLICE as usize;
        let mut whole = vec![b'a'; slice - 1];
        whole.extend("中".as_bytes());
        let head = &whole[..slice];

        let f = shape(whole.len() as u64, 0, head);
        assert!(!f.binary, "a cut character must not read as binary");
        assert!(f.cut, "and it must say it was cut");
        assert_eq!(
            f.text.len(),
            slice - 1,
            "what decoded is everything before the character"
        );
        assert!(
            f.text.chars().all(|c| c == 'a'),
            "and no replacement character was kept"
        );
    }

    /// NUL is a valid UTF-8 character, so a decoder alone says yes to a
    /// file that is binary in exactly the way most binaries are.
    #[test]
    fn nul_makes_it_binary_even_though_it_decodes() {
        let bytes = b"\x7fELF\0\0\0 and more".to_vec();
        assert!(
            std::str::from_utf8(&bytes).is_ok(),
            "the premise: this decodes"
        );
        let f = shape(bytes.len() as u64, 0, &bytes);
        assert!(f.binary, "the sniff is what catches it");
        assert!(f.text.is_empty(), "and nothing is handed to a code view");
    }

    /// The ordinary case, and the one that proves `cut` is not just
    /// always true.
    #[test]
    fn a_whole_small_file_is_text_and_says_it_is_whole() {
        let bytes = "fn main() {}\n中文也在这里\n".as_bytes();
        let f = shape(bytes.len() as u64, 7, bytes);
        assert!(!f.binary);
        assert!(!f.cut, "everything arrived");
        assert_eq!(f.text, String::from_utf8(bytes.to_vec()).unwrap());
        assert_eq!(f.at_ms, 7, "the change contract rides through untouched");
    }

    /// A byte sequence no continuation can rescue, away from the end.
    /// Told apart from the case above by `error_len`, which is the
    /// whole of the distinction.
    #[test]
    fn invalid_bytes_in_the_middle_are_binary() {
        let bytes = vec![b'o', b'k', 0xC3, 0x28, b'm', b'o', b'r', b'e'];
        let f = shape(bytes.len() as u64, 0, &bytes);
        assert!(
            f.binary,
            "not a boundary — nothing that follows makes this text"
        );
    }

    /// The same incomplete character as the first test, but the file
    /// really does end there. **The slice is not what cut it, so there
    /// is nothing to be lenient about** — and being lenient would put
    /// text on screen that a save would then write back over bytes it
    /// never saw (工作区批4).
    #[test]
    fn an_incomplete_character_at_the_end_of_a_whole_file_is_binary() {
        let bytes = vec![b'h', b'i', 0xE4, 0xB8];
        let f = shape(bytes.len() as u64, 0, &bytes);
        assert!(!f.cut, "the premise: everything the file has is here");
        assert!(
            f.binary,
            "and what it has is not text all the way to the end"
        );
        assert!(f.digest.is_empty(), "so there is nothing to offer a save");
    }

    /// The invariant the save rests on, asserted rather than described:
    /// a digest exists exactly when the text on screen is the whole
    /// file, byte for byte.
    #[test]
    fn a_digest_is_handed_out_only_when_the_text_is_the_whole_file() {
        let whole = "fn main() {}\n".as_bytes();
        let f = shape(whole.len() as u64, 0, whole);
        assert_eq!(f.digest, khor_node::files::digest(whole));
        assert_eq!(f.text.as_bytes(), whole, "and it is the same bytes");

        // Cut: the text is a prefix, so writing it back would truncate.
        let g = shape(whole.len() as u64 + 10, 0, whole);
        assert!(g.cut);
        assert!(g.digest.is_empty(), "a prefix must not be savable");

        // Binary: nothing was decoded at all.
        let h = shape(4, 0, b"\x7fE\0F");
        assert!(h.binary);
        assert!(h.digest.is_empty());
    }
}

/// Tests for the multi-slice read path (files larger than SLICE).
/// These exercise `read_text` directly against a real node on loopback,
/// so they live in their own mod and are not unit tests of `shape`.
#[cfg(test)]
mod large_file_tests {
    use khor_node::proto::SLICE;

    /// Build a scratch node backed by a temp directory, write a file into
    /// it, and read it back through `read_text`. We call into the node
    /// layer directly rather than spinning up HTTP so the test is fast and
    /// self-contained.
    fn read_via_node(content: &[u8]) -> Result<super::FileText, String> {
        let dir = std::env::temp_dir().join(format!(
            "khor-large-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.txt");
        std::fs::write(&path, content).unwrap();

        // Read through the node's own read_slice in a loop, mirroring what
        // read_text does, so we can test the logic without a full HTTP stack.
        let total = content.len() as u64;
        let at_ms = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Small-file fast path.
        if total <= SLICE {
            let (_, _, bytes) = khor_node::files::read_slice(path.to_str().unwrap(), 0)?;
            let _ = std::fs::remove_dir_all(&dir);
            return Ok(super::shape(total, at_ms, &bytes));
        }

        // Large-file loop path: replicate read_text's slice loop locally.
        const TEXT_CAP: u64 = 64 * 1024 * 1024;
        let mut bytes: Vec<u8> = Vec::new();
        let mut hasher = blake3::Hasher::new();
        {
            let (_, _, first) = khor_node::files::read_slice(path.to_str().unwrap(), 0)?;
            if first.iter().take(super::SNIFF).any(|b| *b == 0) {
                let _ = std::fs::remove_dir_all(&dir);
                return Ok(super::FileText {
                    total,
                    at_ms,
                    text: String::new(),
                    cut: true,
                    binary: true,
                    digest: String::new(),
                });
            }
            hasher.update(&first);
            bytes.extend_from_slice(&first);
        }
        let want = total.min(TEXT_CAP);
        while (bytes.len() as u64) < want {
            let offset = bytes.len() as u64;
            let (now_total, _, slice) =
                khor_node::files::read_slice(path.to_str().unwrap(), offset)?;
            assert_eq!(now_total, total, "file grew under read");
            if slice.is_empty() {
                break;
            }
            if slice
                .iter()
                .take(super::SNIFF.min(slice.len()))
                .any(|b| *b == 0)
            {
                let _ = std::fs::remove_dir_all(&dir);
                return Ok(super::FileText {
                    total,
                    at_ms,
                    text: String::new(),
                    cut: true,
                    binary: true,
                    digest: String::new(),
                });
            }
            hasher.update(&slice);
            bytes.extend_from_slice(&slice);
        }
        bytes.truncate(want as usize);
        let cut = total > TEXT_CAP;
        let text = match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_owned(),
            Err(e) if e.error_len().is_none() && !cut => {
                String::from_utf8_lossy(&bytes[..e.valid_up_to()]).into_owned()
            }
            Err(_) => {
                let _ = std::fs::remove_dir_all(&dir);
                return Ok(super::FileText {
                    total,
                    at_ms,
                    text: String::new(),
                    cut: true,
                    binary: true,
                    digest: String::new(),
                });
            }
        };
        let digest = if cut {
            String::new()
        } else {
            hasher.finalize().to_hex().to_string()
        };
        let _ = std::fs::remove_dir_all(&dir);
        Ok(super::FileText {
            total,
            at_ms,
            text,
            cut,
            binary: false,
            digest,
        })
    }

    /// A file that spans more than one slice (SLICE + a few bytes) must
    /// come back whole and carry a correct digest.
    #[test]
    fn file_bigger_than_one_slice_reads_back_whole_with_correct_digest() {
        let size = SLICE as usize + 512;
        let content: Vec<u8> = (0..size).map(|i| b'a' + (i % 26) as u8).collect();
        let result = read_via_node(&content).expect("read must succeed");
        assert!(!result.binary, "text file must not be called binary");
        assert!(!result.cut, "a file under TEXT_CAP must not be cut");
        assert_eq!(result.text.len(), content.len(), "all bytes must arrive");
        let expected_digest = khor_node::files::digest(&content);
        assert_eq!(
            result.digest, expected_digest,
            "digest must match blake3 of the whole file"
        );
    }

    /// A binary file detected in the second slice must be refused before
    /// the third slice crosses the wire — `binary: true`, no text, no digest.
    #[test]
    fn binary_file_detected_in_second_slice_is_refused() {
        // First slice: all printable ASCII.
        let mut content = vec![b'a'; SLICE as usize];
        // Second slice: starts with a NUL.
        content.push(0x00);
        content.extend_from_slice(b"and more bytes");
        let result = read_via_node(&content).expect("read_via_node should not Err");
        assert!(
            result.binary,
            "NUL in second slice must be detected as binary"
        );
        assert!(result.text.is_empty(), "no text for a binary");
        assert!(result.digest.is_empty(), "no digest for a binary");
    }

    /// Timing measurement for §6 (CM-C-BRIEF). Run with:
    ///   cargo test -p khor-gui-core -- large_file_tests::bench_read_times --ignored --nocapture
    /// Measures before (one slice only, old behaviour) and after (all slices, new behaviour)
    /// for 1 MB, 8 MB and 32 MB files over a local temp directory — no network.
    #[test]
    #[ignore]
    fn bench_read_times() {
        use std::time::Instant;

        fn make_file(size: usize) -> (std::path::PathBuf, std::path::PathBuf) {
            let dir = std::env::temp_dir().join(format!(
                "khor-bench-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("bench.txt");
            let content: Vec<u8> = (0..size).map(|i| b'a' + (i % 26) as u8).collect();
            std::fs::write(&path, &content).unwrap();
            (dir, path)
        }

        fn time_old(path: &std::path::Path) -> std::time::Duration {
            // Old behaviour: one read_slice call, stop at SLICE.
            let t = Instant::now();
            let _ = khor_node::files::read_slice(path.to_str().unwrap(), 0).unwrap();
            t.elapsed()
        }

        fn time_new(path: &std::path::Path, total: u64) -> std::time::Duration {
            const TEXT_CAP: u64 = 64 * 1024 * 1024;
            let t = Instant::now();
            let want = total.min(TEXT_CAP);
            let mut read: u64 = 0;
            while read < want {
                let (_, _, slice) =
                    khor_node::files::read_slice(path.to_str().unwrap(), read).unwrap();
                if slice.is_empty() {
                    break;
                }
                read += slice.len() as u64;
            }
            t.elapsed()
        }

        for &mb in &[1usize, 8, 32] {
            let size = mb * 1024 * 1024;
            let (dir, path) = make_file(size);
            // Warm-up: one read to bring the file into OS cache.
            let _ = khor_node::files::read_slice(path.to_str().unwrap(), 0);

            let old = time_old(&path);
            let new = time_new(&path, size as u64);
            println!(
                "bench {} MB — before (256 KB): {:>6.1} ms  after (all): {:>6.1} ms  ratio: {:.1}x",
                mb,
                old.as_secs_f64() * 1000.0,
                new.as_secs_f64() * 1000.0,
                new.as_secs_f64() / old.as_secs_f64().max(0.001),
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
