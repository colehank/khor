//! VS Code on a machine: whether it is there, and a web face pointed at
//! one workspace (工作区批5).
//!
//! **Not a session, and the reason is not a preference.** A session in
//! khor is something that can answer 你怎么样 with one of six words
//! (docs/UX.md); an editor somebody left open cannot — it is neither
//! busy nor idle in any sense khor could report, and a row that always
//! said the same word would be a row nobody ever reads. So this has no
//! row, no state, and no place in the sessions landing. What it has is
//! a lifetime tied to the thing that opened it.
//!
//! **khor does not ship VS Code and does not want to.** This module
//! runs a `code` that is already on that machine. The one thing it
//! costs the person is the first run: `code serve-web` fetches its own
//! server from Microsoft, measured at 643 MB on this machine, and that
//! is a sentence a person has to see before it happens rather than
//! after (「第一个,第一次是允许的.联网也要说一下」— the user,
//! 2026-09-10). The consent belongs to the face; this module is the
//! half that would carry it out.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use khor_catalog::msg;

/// Where a mac keeps the CLI when nobody ran 「Install 'code' command in
/// PATH」. **Looked for because not finding it is the common case, not
/// the odd one**: VS Code on macOS installs as an app bundle and the
/// PATH shim is a separate thing a person has to go and ask for, so a
/// probe that only read PATH would tell most mac users they have no
/// editor while its icon is in their dock.
const MAC_BUNDLE: &str = "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code";

/// The `code` this machine can run, or `None`.
///
/// **Two places and no more.** PATH is where a Linux box and a
/// configured mac both have it; the bundle above is where an
/// unconfigured mac does. Windows' installer puts it on PATH, so it
/// falls out of the first branch. A `code` somewhere else entirely is a
/// machine khor reports as having none — wrong, but wrong in the
/// direction that offers nothing rather than the direction that offers
/// a button that fails.
pub fn cli() -> Option<PathBuf> {
    if which("code").is_some() {
        return Some(PathBuf::from("code"));
    }
    let bundle = Path::new(MAC_BUNDLE);
    if bundle.is_file() {
        Some(bundle.to_path_buf())
    } else {
        None
    }
}

/// What one machine can say about its editor without starting anything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Editor {
    /// `code --version`'s first line, or empty when there is no `code`.
    ///
    /// The version is what travels rather than a bare yes: `serve-web`
    /// arrived in 1.82, and a machine older than that would answer the
    /// probe and then fail the launch — which is a worse shape than
    /// saying so up front.
    pub version: String,
    /// Whether the server `code serve-web` would otherwise fetch is
    /// already sitting on that machine.
    ///
    /// **This is what the face's consent is actually about.** The
    /// sentence it shows says ~640 MB is about to come down from
    /// Microsoft; on a machine where it already came down that sentence
    /// is false, and it was being shown anyway — the face only
    /// remembered the answer in its own storage, so a fresh app profile
    /// asked again, in front of a server that had been on disk since
    /// 2026-09-11 (found by opening the desktop app, 2026-09-13).
    ///
    /// False is the safe reading, which is why nothing here guesses:
    /// a machine that cannot be asked gets the sentence and a person
    /// gets the choice.
    pub ready: bool,
}

/// [`Editor`] for this machine.
///
/// **One spawn, ~70 ms measured on this machine**, which is why it is
/// asked when a workspace opens and not on the beat. Both facts come
/// out of that one spawn: the server directory is named after the
/// commit, which is `code --version`'s second line, so asking whether
/// the server is there means having asked the version already.
pub fn probe() -> Editor {
    let Some(exe) = cli() else {
        return Editor::default();
    };
    let Ok(out) = std::process::Command::new(exe).arg("--version").output() else {
        return Editor::default();
    };
    if !out.status.success() {
        return Editor::default();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let version = lines.next().unwrap_or("").trim().to_owned();
    let commit = lines.next().unwrap_or("").trim();
    Editor {
        ready: !version.is_empty() && server_is_here(commit),
        version,
    }
}

/// Whether `serve-web`'s server for `commit` is already unpacked.
///
/// **Named by commit, not by version**, because that is how the CLI
/// names it: `~/.vscode/cli/serve-web/<commit>`. A machine that
/// upgraded VS Code has a directory there for the old commit and none
/// for the new one, and the download ahead is real again — which is the
/// answer a person needs and the one a version check would get wrong.
///
/// `VSCODE_CLI_DATA_DIR` is honoured because the CLI honours it; a
/// machine that moved its data directory would otherwise be told it has
/// nothing while the server sits where it was put.
///
/// The probe is for `node`, the one file the server cannot run without,
/// rather than for the directory: an interrupted fetch leaves the
/// directory behind, and a directory is not a server.
fn server_is_here(commit: &str) -> bool {
    let base = match std::env::var_os("VSCODE_CLI_DATA_DIR") {
        Some(d) => PathBuf::from(d),
        None => match std::env::home_dir() {
            Some(h) => h.join(".vscode").join("cli"),
            None => return false,
        },
    };
    server_under(&base, commit)
}

/// The judgment half of [`server_is_here`], with the CLI's data
/// directory handed in.
///
/// **Split so it can be tested against a directory built for the
/// purpose.** The other half reads the environment and the home, which
/// a test can only change for the whole process — and a process-wide
/// change is a test that fails when another one runs beside it.
fn server_under(base: &Path, commit: &str) -> bool {
    if commit.is_empty() {
        return false;
    }
    base.join("serve-web").join(commit).join("node").is_file()
}

/// A `PATH` lookup with no shell in it. `which(1)` would be a second
/// process and a second set of rules about quoting.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// The one running web face, if any.
///
/// **One per machine, not one per workspace.** The first spelling keyed
/// this by directory and started a server for each; measured, each one
/// is a `node` process with its own extension host, and a person with
/// four projects open would be paying for four copies of VS Code to
/// look at four folders. The web face can open any folder it is asked
/// for in the URL (`?folder=`), which is what it is built for — so the
/// server is the machine's and the workspace is the link's.
struct Serving {
    port: u16,
    token: String,
    child: std::process::Child,
}

static SERVING: Mutex<Option<Serving>> = Mutex::new(None);

/// How long to wait for `code serve-web` to say where it is listening.
/// **Warm start measured at 1.5 s on this machine**; a cold one has to
/// fetch 643 MB first, and on a slow link that is minutes — so this is
/// not a timeout on the download, it is a timeout on a server that is
/// never going to answer.
const READY: std::time::Duration = std::time::Duration::from_secs(600);

/// Starts this machine's web face, or hands back the one already
/// running. Answers `(port, token)`.
///
/// **`--port 0` and then read the port back**, rather than picking one
/// here: a port khor chose could be taken between the choosing and the
/// binding, and `serve-web` prints the one it actually got.
pub fn serve() -> Result<(u16, String), String> {
    let exe = cli().ok_or_else(|| msg::no_vscode().to_owned())?;
    // Before the running-server shortcut, not after it: the machine
    // scope is watched live by the page, so a server already up gets
    // the key on the next press too, rather than on its next restart.
    follow_the_system_theme();
    {
        let mut held = SERVING.lock().map_err(|e| e.to_string())?;
        if let Some(s) = held.as_mut() {
            // Still alive? `try_wait` is the only honest answer — a
            // handle whose process exited an hour ago looks exactly
            // like one that is serving.
            match s.child.try_wait() {
                Ok(None) => return Ok((s.port, s.token.clone())),
                _ => *held = None,
            }
        }
    }
    let token = crate::link::fresh_hex()?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("serve-web")
        .args(["--port", "0"])
        .args(["--connection-token", &token])
        // Without this the server stops and asks on stdin, which
        // nobody is reading. **The terms are Microsoft's and the
        // person's to accept**, which is why the face asks before this
        // is ever called and why the sentence it asks with names them.
        .arg("--accept-server-license-terms")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    group(&mut cmd);
    let mut child = cmd.spawn().map_err(msg::cant_start_vscode)?;
    let port = match read_port(&mut child) {
        Ok(p) => p,
        Err(e) => {
            end(&mut child);
            return Err(e);
        }
    };
    *SERVING.lock().map_err(|e| e.to_string())? = Some(Serving {
        port,
        token: token.clone(),
        child,
    });
    Ok((port, token))
}

/// The setting VS Code reads to follow the operating system's light or
/// dark, and the file it reads it out of.
///
/// **khor follows the system and the web editor does not** (工作区第六轮
/// 第 49 条:「VSCODE没有遵循app的主题, 也就是深色.反之亦然」— the user).
/// The app's own window has `prefers-color-scheme` and turns with the
/// desktop; `window.autoDetectColorScheme` is off by default in VS
/// Code, so the editor stays light beside a dark khor and dark beside a
/// light one. Setting it on both ends is what makes them one screen —
/// and it is set on **that machine's** settings, because that is where
/// the editor is running.
const FOLLOW_KEY: &str = "window.autoDetectColorScheme";

/// Where the web editor reads a setting **from the server side**:
/// `--server-data-dir` (default `~/.vscode-server`) and
/// `data/Machine/settings.json` under it — the *machine* scope.
///
/// **Not `data/User/settings.json`, and that was a wrong turn on the
/// first try (2026-09-17).** VS Code for the Web — which is what
/// `serve-web` serves — keeps its *user* settings in the browser
/// (an IndexedDB named `vscode-web-db`, per origin), so a key written
/// to the server's `User` file is never read by the page: a fresh
/// dark browser against a server with that file still came up light.
/// The machine scope is the one the page asks the server for, and a
/// fresh dark browser came up dark and a fresh light one light with
/// the same key there. Measured, not read off a help text.
fn theme_settings() -> Option<PathBuf> {
    Some(
        std::env::home_dir()?
            .join(".vscode-server")
            .join("data")
            .join("Machine")
            .join("settings.json"),
    )
}

/// Writes [`FOLLOW_KEY`] into that file — **only where the key is not
/// there already**.
///
/// **A key somebody has answered is their answer.** Somebody who turned
/// this off turned it off; khor filling in a blank is a different act
/// from khor overruling a choice, and only the first is khor's to make.
/// So the test is 「is the key present」, not 「is it true」.
///
/// **Nothing here can stop the editor from starting.** Every way this
/// can fail — no home, a settings file with a comment in it (VS Code's
/// own format allows them and `serde_json` does not), a directory that
/// cannot be written — leaves the file exactly as it was and the
/// launch going ahead. The cost of failing is that the editor comes up
/// in the other scheme, which is where it was before this existed; the
/// cost of being loud about it would be a refusal in front of an editor
/// somebody asked for.
///
/// It is the same reach `khor hooks` takes into claude's settings
/// (`crate::adaptor::claude::install_hooks`): parse, touch one key,
/// write the rest back as it was. `serde_json`'s `preserve_order` is
/// why the rest really is as it was — without it a round trip would
/// re-alphabetise somebody's configuration and call the diff khor's.
fn follow_the_system_theme() {
    let Some(path) = theme_settings() else { return };
    follow_theme_in(&path);
}

/// The judgment half of [`follow_the_system_theme`], with the file
/// handed in.
///
/// **Split so it can be tested against a file built for the purpose**,
/// exactly as [`server_under`] is: the other half reads the home
/// directory, which a test can only change for the whole process — and
/// a process-wide change is a test that fails when another one runs
/// beside it.
fn follow_theme_in(path: &Path) {
    let mut settings = match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => serde_json::json!({}),
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(v) => v,
            // Not ours to repair. A file khor cannot parse is one
            // somebody is in the middle of editing, or one with the
            // comments VS Code allows — and rewriting it from khor's
            // idea of what it held would destroy the only copy.
            Err(_) => return,
        },
        Err(_) => serde_json::json!({}),
    };
    let Some(root) = settings.as_object_mut() else {
        return;
    };
    if root.contains_key(FOLLOW_KEY) {
        return;
    }
    root.insert(FOLLOW_KEY.to_owned(), serde_json::Value::Bool(true));
    let Ok(text) = serde_json::to_string_pretty(&settings) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, format!("{text}\n"));
}

/// Puts the child in a process group of its own.
///
/// **Because `code` is a shell script, not the server.** On this
/// machine PATH's `code` is `bash /usr/local/bin/code`, which execs the
/// app bundle's launcher, which spawns `code-tunnel`, which spawns
/// `node`. Killing the handle khor holds kills the first of those and
/// orphans the rest — measured on 2026-09-12, two full server trees
/// left running after two `--stop`s that both reported success. A group
/// is the only handle that names the whole tree.
#[cfg(unix)]
fn group(cmd: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(not(unix))]
fn group(_cmd: &mut std::process::Command) {}

/// Ends a child and everything it spawned.
#[cfg(unix)]
fn end(child: &mut std::process::Child) {
    // Negative pid is the group. SIGTERM first because the server
    // writes state on the way out; the handle's own `kill` (SIGKILL to
    // the leader alone) is the fallback for a group that is already
    // gone, where it is a no-op.
    unsafe { libc::kill(-(child.id() as i32), libc::SIGTERM) };
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(not(unix))]
fn end(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Reads `serve-web`'s stdout until it says where it is listening.
///
/// **Parsed out of a sentence, and that is a real dependency on
/// somebody else's output.** There is no other channel: the CLI has no
/// `--print-port` and no json mode. What makes it survivable is that a
/// changed sentence fails *loudly* here — the launch times out and says
/// so — rather than handing back a port nobody is on.
fn read_port(child: &mut std::process::Child) -> Result<u16, String> {
    use std::io::{BufRead, BufReader};
    let out = child
        .stdout
        .take()
        .ok_or_else(|| msg::no_vscode_output().to_owned())?;
    let err = child.stderr.take();
    let (tx, rx) = std::sync::mpsc::channel();
    // ⚠️ **Both pipes are drained for the life of the child, not until
    // the port is found.** The first version returned from this thread
    // the moment it had the number, which dropped the reader and closed
    // the pipe — and `serve-web` logs every connection, so its first
    // write after that got EPIPE. What that looked like from outside
    // was a server holding the port (`lsof` said LISTEN) and answering
    // every request with an empty reply. Measured 2026-09-12; it cost
    // an hour because a listening socket looks like a working server.
    //
    // The lines are thrown away rather than kept: what khor needs from
    // this stream is one number, and holding a growing log of somebody
    // else's editor is a leak with no reader.
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(port) = port_in(&line) {
                let _ = tx.send(port);
            }
        }
    });
    if let Some(err) = err {
        std::thread::spawn(
            move || {
                for _ in BufReader::new(err).lines().map_while(Result::ok) {}
            },
        );
    }
    rx.recv_timeout(READY)
        .map_err(|_| msg::vscode_never_answered().to_owned())
}

/// The port inside `Web UI available at http://127.0.0.1:9123?tkn=…`.
/// A pure function so the one fragile thing in this module — reading
/// somebody else's sentence — is tested without starting a server.
fn port_in(line: &str) -> Option<u16> {
    let at = line.find("http://127.0.0.1:")? + "http://127.0.0.1:".len();
    let rest = &line[at..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Stops this machine's web face, if there is one. **Answers the same
/// whether there was one or not**: the caller wanted it gone, and a
/// refusal would only tell them about a race they cannot act on (the
/// rule `ipc::Op::DropBorrow` already follows).
pub fn stop() -> Result<(), String> {
    let mut held = SERVING.lock().map_err(|e| e.to_string())?;
    if let Some(mut s) = held.take() {
        end(&mut s.child);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_port_is_read_out_of_the_sentence_vscode_actually_prints() {
        // Copied from a real run, 2026-09-12, `code --version` 1.137.0.
        let line = "Web UI available at http://127.0.0.1:9123?tkn=aa2e3e6211284063f9d6efbebae168fd";
        assert_eq!(port_in(line), Some(9123));
    }

    #[test]
    fn a_sentence_without_a_port_is_none_rather_than_a_guess() {
        for line in [
            "* Visual Studio Code Server",
            "Web UI available at http://localhost/",
            "",
        ] {
            assert_eq!(port_in(line), None, "{line}");
        }
    }

    /// Builds what the CLI leaves behind after a finished fetch.
    fn unpacked(base: &Path, commit: &str) {
        let dir = base.join("serve-web").join(commit);
        std::fs::create_dir_all(&dir).expect("make server dir");
        std::fs::write(dir.join("node"), b"#!/bin/sh\n").expect("write node");
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("khor-code-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("make scratch");
        dir
    }

    #[test]
    fn a_finished_fetch_is_seen_and_a_different_commit_is_not() {
        let base = scratch("finished");
        unpacked(&base, "645f29cc3176500b4b5762ba887cf2a7f0ffdf2c");
        assert!(server_under(
            &base,
            "645f29cc3176500b4b5762ba887cf2a7f0ffdf2c"
        ));
        // **An upgraded VS Code is a download ahead again.** The old
        // commit's directory is still there and answers for nothing.
        assert!(!server_under(
            &base,
            "0000000000000000000000000000000000000000"
        ));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_directory_without_the_runtime_is_not_a_server() {
        // What an interrupted fetch leaves: the directory exists and
        // `serve-web` will go and get the rest. Reading that as「already
        // there」 would skip the sentence about the download that is
        // still coming.
        let base = scratch("half");
        std::fs::create_dir_all(base.join("serve-web").join("abc")).expect("make dir");
        assert!(!server_under(&base, "abc"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_machine_that_said_no_commit_is_not_ready() {
        // `code --version` with one line, or none: nothing to name a
        // directory with, so the answer is the asking one.
        let base = scratch("nocommit");
        assert!(!server_under(&base, ""));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 工作区第六轮 第 49 条, the half a person meets on a fresh
    /// machine: no settings file at all, so khor writes the one key and
    /// the editor comes up in the desktop's own scheme.
    #[test]
    fn a_machine_with_no_settings_gets_the_key() {
        let dir = scratch("theme-fresh");
        let at = dir.join("data").join("Machine").join("settings.json");
        follow_theme_in(&at);
        let text = std::fs::read_to_string(&at).expect("the file khor wrote");
        let v: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(v[FOLLOW_KEY], serde_json::Value::Bool(true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// …and the half the ruling is actually about: **a key somebody has
    /// answered is their answer.** `false` is a person saying「do not
    /// follow」, and khor overruling that would be khor deciding
    /// somebody's editor for them.
    #[test]
    fn a_key_already_answered_is_left_exactly_as_it_was() {
        let dir = scratch("theme-answered");
        std::fs::create_dir_all(&dir).expect("make scratch");
        let at = dir.join("settings.json");
        let was = "{\n  \"window.autoDetectColorScheme\": false,\n  \"editor.fontSize\": 13\n}\n";
        std::fs::write(&at, was).expect("write settings");
        follow_theme_in(&at);
        assert_eq!(
            std::fs::read_to_string(&at).expect("read back"),
            was,
            "khor rewrote a settings file it had nothing to add to"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The other keys survive, and so does their order — which is what
    /// `serde_json`'s `preserve_order` is in this tree for. A round
    /// trip that re-alphabetised somebody's configuration would be a
    /// diff that is all khor's and says nothing khor meant.
    #[test]
    fn everything_else_in_the_file_comes_back_in_its_own_order() {
        let dir = scratch("theme-order");
        std::fs::create_dir_all(&dir).expect("make scratch");
        let at = dir.join("settings.json");
        std::fs::write(
            &at,
            "{\"zebra\": 1, \"apple\": 2, \"workbench.colorTheme\": \"Default Dark+\"}",
        )
        .expect("write settings");
        follow_theme_in(&at);
        let text = std::fs::read_to_string(&at).expect("read back");
        let keys: Vec<&str> = text
            .lines()
            .filter_map(|l| l.trim().strip_prefix('"'))
            .filter_map(|l| l.split('"').next())
            .collect();
        assert_eq!(
            keys,
            vec!["zebra", "apple", "workbench.colorTheme", FOLLOW_KEY],
            "the keys came back in another order, or one of them did not come back"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file khor cannot parse is not khor's to repair. VS Code's own
    /// settings format allows comments and `serde_json` does not, so
    /// this is a real file on real machines — and the answer is to
    /// leave it alone and start the editor anyway.
    #[test]
    fn a_settings_file_with_comments_in_it_is_left_alone() {
        let dir = scratch("theme-jsonc");
        std::fs::create_dir_all(&dir).expect("make scratch");
        let at = dir.join("settings.json");
        let was = "{\n  // mine\n  \"editor.fontSize\": 13\n}\n";
        std::fs::write(&at, was).expect("write settings");
        follow_theme_in(&at);
        assert_eq!(std::fs::read_to_string(&at).expect("read back"), was);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
