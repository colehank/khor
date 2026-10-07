//! The one password khor keeps: an ssh host the person chose to log in
//! to with a password (添加机器 认证方式 密码; docs/NET.md 添加机器).
//!
//! # Only the OS secret store of the machine that runs ssh
//!
//! The macOS Keychain, or the Secret Service on Linux through
//! `secret-tool` (which takes the secret on stdin, so it is never on a
//! command line). Never a synced document, the wire, a log or a ticket:
//! the device row says only that the road is a password road
//! (`ssh_password`). A machine with no store refuses the password road
//! in one sentence rather than keeping it anywhere else.
//!
//! # ssh asks khor, through `SSH_ASKPASS`
//!
//! A password road runs ssh with `SSH_ASKPASS` set to this very program
//! and `SSH_ASKPASS_REQUIRE=force` ([`crate::satellite::Dest::env`]). ssh
//! then starts it with the prompt as its one argument and
//! [`ASKPASS_KEY`] naming the host; every khor `main` calls
//! [`answer_askpass_if_asked`] first, which answers from the store and
//! exits. It answers only a password prompt, and only to an `ssh`
//! parent: a host-key question gets no answer (known_hosts decides
//! that), and a one-time code is out of scope, said once by the caller
//! from [`ASKPASS_NOT_A_PASSWORD`].
use std::path::PathBuf;

use khor_catalog::msg;

/// The environment variable that names whose password ssh is asking for.
/// Its presence is also what makes a khor start up as the askpass.
pub const ASKPASS_KEY: &str = "KHOR_ASKPASS";
/// What the askpass prints on stderr (ssh passes it through) when the
/// prompt is not a password: the caller's sign for a one-time code.
pub const ASKPASS_NOT_A_PASSWORD: &str = "khor askpass: not a password prompt";
/// What it prints when nothing is kept for the host.
pub const ASKPASS_NONE_KEPT: &str = "khor askpass: no password kept";

/// The store's name for khor's entries (Keychain service, Secret
/// Service attribute).
const SERVICE: &str = "khor-ssh";

/// The test door (`KHOR_SSH_DIR`'s sibling): a scratch directory holding
/// one 0600 file per key, so tests never read or write a real Keychain
/// or keyring. Production leaves it unset.
fn door() -> Option<PathBuf> {
    std::env::var_os("KHOR_SECRET_DIR").map(Into::into)
}

fn door_file(dir: &std::path::Path, key: &str) -> PathBuf {
    let name: String = key.bytes().map(|b| format!("{b:02x}")).collect();
    dir.join(name)
}

/// Keeps `password` for `key`, replacing what was there.
pub fn put(key: &str, password: &str) -> Result<(), String> {
    if let Some(dir) = door() {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::create_dir_all(&dir).map_err(|e| msg::secret_store_failed(e.to_string()))?;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(door_file(&dir, key))
            .map_err(|e| msg::secret_store_failed(e.to_string()))?;
        return f
            .write_all(password.as_bytes())
            .map_err(|e| msg::secret_store_failed(e.to_string()));
    }
    os::put(key, password)
}

/// The password kept for `key`, `None` when there is none.
pub fn get(key: &str) -> Result<Option<String>, String> {
    if let Some(dir) = door() {
        return match std::fs::read_to_string(door_file(&dir, key)) {
            Ok(p) => Ok(Some(p)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(msg::secret_store_failed(e.to_string())),
        };
    }
    os::get(key)
}

/// Drops the password kept for `key`; nothing kept is not an error.
pub fn forget(key: &str) -> Result<(), String> {
    if let Some(dir) = door() {
        return match std::fs::remove_file(door_file(&dir, key)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(msg::secret_store_failed(e.to_string())),
            _ => Ok(()),
        };
    }
    os::forget(key)
}

/// Whether ssh's prompt asks for a password (or a key's passphrase,
/// which is the same question to the person) — and not for a one-time
/// code or a host key's yes/no.
fn asks_password(prompt: &str) -> bool {
    let p = prompt.to_lowercase();
    let other = ["one-time", "verification", "code", "otp", "token", "yes/no"];
    (p.contains("password") || p.contains("passphrase")) && !other.iter().any(|o| p.contains(o))
}

/// When this process was started by ssh as its askpass, answers and
/// exits; otherwise returns at once. Every khor `main` calls it first.
pub fn answer_askpass_if_asked() {
    let Some(key) = std::env::var_os(ASKPASS_KEY) else {
        return;
    };
    let key = key.to_string_lossy().into_owned();
    let prompt = std::env::args().nth(1).unwrap_or_default();
    if !parent_is_ssh() {
        std::process::exit(1);
    }
    if !asks_password(&prompt) {
        // A host key's yes/no is known_hosts' business; ssh says so.
        if !prompt.contains("yes/no") {
            eprintln!("{ASKPASS_NOT_A_PASSWORD}");
        }
        std::process::exit(1);
    }
    match get(&key) {
        Ok(Some(p)) => {
            println!("{p}");
            std::process::exit(0);
        }
        _ => {
            eprintln!("{ASKPASS_NONE_KEPT}");
            std::process::exit(1);
        }
    }
}

/// The askpass answers only ssh: anything else that sets
/// [`ASKPASS_KEY`] and starts khor gets nothing out of the store.
fn parent_is_ssh() -> bool {
    let ppid = std::os::unix::process::parent_id();
    let Some(path) = exe_of(ppid) else {
        return false;
    };
    path.file_name().is_some_and(|n| n == "ssh")
}

#[cfg(target_os = "macos")]
fn exe_of(pid: u32) -> Option<PathBuf> {
    let mut buf = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: the buffer is as long as the size we pass.
    let n = unsafe { libc::proc_pidpath(pid as i32, buf.as_mut_ptr().cast(), buf.len() as u32) };
    if n <= 0 {
        return None;
    }
    buf.truncate(n as usize);
    Some(PathBuf::from(String::from_utf8(buf).ok()?))
}

#[cfg(not(target_os = "macos"))]
fn exe_of(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/exe")).ok()
}

#[cfg(target_os = "macos")]
mod os {
    use super::SERVICE;
    use khor_catalog::msg;
    use security_framework::passwords;

    /// `errSecItemNotFound`.
    const NOT_FOUND: i32 = -25300;

    pub fn put(key: &str, password: &str) -> Result<(), String> {
        passwords::set_generic_password(SERVICE, key, password.as_bytes())
            .map_err(|e| msg::secret_store_failed(e.to_string()))
    }

    pub fn get(key: &str) -> Result<Option<String>, String> {
        match passwords::get_generic_password(SERVICE, key) {
            Ok(b) => Ok(Some(String::from_utf8_lossy(&b).into_owned())),
            Err(e) if e.code() == NOT_FOUND => Ok(None),
            Err(e) => Err(msg::secret_store_failed(e.to_string())),
        }
    }

    pub fn forget(key: &str) -> Result<(), String> {
        match passwords::delete_generic_password(SERVICE, key) {
            Err(e) if e.code() != NOT_FOUND => Err(msg::secret_store_failed(e.to_string())),
            _ => Ok(()),
        }
    }
}

#[cfg(target_os = "linux")]
mod os {
    use std::io::Write;
    use std::process::{Command, Stdio};

    use super::SERVICE;
    use khor_catalog::msg;

    /// `secret-tool` with khor's two attributes; a machine without it
    /// (or without a Secret Service to talk to) has no store.
    fn tool(verb: &str, key: &str) -> Command {
        let mut c = Command::new("secret-tool");
        c.arg(verb);
        if verb == "store" {
            c.args(["--label", &format!("khor ssh {key}")]);
        }
        c.args(["service", SERVICE, "account", key]);
        c
    }

    pub fn put(key: &str, password: &str) -> Result<(), String> {
        let mut child = tool("store", key)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| msg::no_secret_store())?;
        if let Some(mut w) = child.stdin.take() {
            w.write_all(password.as_bytes())
                .map_err(|e| msg::secret_store_failed(e.to_string()))?;
        }
        let out = child
            .wait_with_output()
            .map_err(|e| msg::secret_store_failed(e.to_string()))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(msg::no_secret_store())
        }
    }

    pub fn get(key: &str) -> Result<Option<String>, String> {
        let out = tool("lookup", key)
            .stderr(Stdio::null())
            .output()
            .map_err(|_| msg::no_secret_store())?;
        // `lookup` answers 1 both for "nothing kept" and for "no
        // service"; either way there is nothing to give ssh.
        Ok(out
            .status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned()))
    }

    pub fn forget(key: &str) -> Result<(), String> {
        match tool("clear", key).stderr(Stdio::null()).status() {
            Ok(_) => Ok(()),
            Err(_) => Err(msg::no_secret_store()),
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod os {
    use khor_catalog::msg;

    pub fn put(_: &str, _: &str) -> Result<(), String> {
        Err(msg::no_secret_store())
    }

    pub fn get(_: &str) -> Result<Option<String>, String> {
        Err(msg::no_secret_store())
    }

    pub fn forget(_: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_password_prompt_is_answered() {
        for p in [
            "user@gpu-box's password: ",
            "(user@gpu-box) Password: ",
            "Enter passphrase for key '/tmp/k': ",
        ] {
            assert!(asks_password(p), "{p}");
        }
        for p in [
            "Verification code: ",
            "One-time password (OATH) for `user': ",
            "Are you sure you want to continue connecting (yes/no/[fingerprint])? ",
            "Allow use of key?",
        ] {
            assert!(!asks_password(p), "{p}");
        }
    }
}
