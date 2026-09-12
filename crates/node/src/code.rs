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
    if bundle.is_file() { Some(bundle.to_path_buf()) } else { None }
}

/// `code --version`'s first line, or empty when there is no `code`.
///
/// **One spawn, ~70 ms measured on this machine**, which is why it is
/// asked when a workspace opens and not on the beat. The version is
/// what travels rather than a bare yes: `serve-web` arrived in 1.82,
/// and a machine older than that would answer the probe and then fail
/// the launch — which is a worse shape than saying so up front.
pub fn version() -> String {
    let Some(exe) = cli() else { return String::new() };
    let Ok(out) = std::process::Command::new(exe).arg("--version").output() else {
        return String::new();
    };
    if !out.status.success() {
        return String::new();
    }
    String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").trim().to_owned()
}

/// A `PATH` lookup with no shell in it. `which(1)` would be a second
/// process and a second set of rules about quoting.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(name)).find(|p| p.is_file())
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
    let exe = cli().ok_or_else(|| msg::NO_VSCODE.to_owned())?;
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
    *SERVING.lock().map_err(|e| e.to_string())? =
        Some(Serving { port, token: token.clone(), child });
    Ok((port, token))
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
    let out = child.stdout.take().ok_or_else(|| msg::NO_VSCODE_OUTPUT.to_owned())?;
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
        std::thread::spawn(move || {
            for _ in BufReader::new(err).lines().map_while(Result::ok) {}
        });
    }
    rx.recv_timeout(READY).map_err(|_| msg::VSCODE_NEVER_ANSWERED.to_owned())
}

/// The port inside `Web UI available at http://127.0.0.1:9123?tkn=…`.
/// A pure function so the one fragile thing in this module — reading
/// somebody else's sentence — is tested without starting a server.
fn port_in(line: &str) -> Option<u16> {
    let at = line.find("http://127.0.0.1:")? + "http://127.0.0.1:".len();
    let rest = &line[at..];
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
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
}
