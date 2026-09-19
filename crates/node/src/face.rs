//! The web face's **installation** facts: which port it binds, where its
//! key lives, and how that key is read, minted and rotated.
//!
//! # Why these are here and not in `khor_web`
//!
//! They were in `khor_web` until 2026-09-19, which is where the server
//! that binds the port and checks the key lives — and that is exactly
//! the layer two other things now have to reach:
//!
//! - `khor doctor` reports whether the port is answering and the key is
//!   there ([`crate::doctor`]);
//! - 安全 shows the key and rotates it (`khor lock`'s page).
//!
//! Both are the node's, and **`khor_web` depends on this crate**, so
//! either asking it would be a cycle. What moved is only the part that
//! is about *this installation* — a file in this store, a port an
//! environment variable can move — and not the serving: `khor_web` keeps
//! `listen`, the request loop, and [`matches`]'s constant-time compare,
//! and reads these from here so there is still one spelling of each.
//!
//! # The key is khor's to hand out, never a password to choose
//!
//! 128 random bits from the same mint as the hand-off cookies
//! ([`crate::link::fresh_hex`]), `.khor/web.key`, mode 0600, **never in
//! a synced document**: every CRDT document here lands on every machine
//! by design, so a key that synced would mean pairing a laptop silently
//! handed it the keys to every face in the fleet.

use std::path::{Path, PathBuf};

use khor_catalog::msg;

/// Where the face listens unless `KHOR_WEB_PORT` says otherwise.
///
/// KHOR on a phone keypad. Far from the development cluster's own range
/// on purpose — 1430 is vite, 1431 the dev bridge, 1442/1443 the smoke
/// run — because a product port a development tool can shadow is a fault
/// nobody diagnoses twice.
pub const WEB_DEFAULT_PORT: u16 = 5467;

/// The port the face uses.
///
/// **One function rather than one line in each caller**: the resident
/// binds this port, `khor web` prints a link naming it and `khor doctor`
/// probes it, and any two of those disagreeing would produce a link that
/// is wrong in the one way nobody checks — it looks exactly like a link
/// that is right.
pub fn web_port() -> u16 {
    std::env::var("KHOR_WEB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(WEB_DEFAULT_PORT)
}

/// Where this machine keeps its key.
pub fn web_key_path(root: &Path) -> PathBuf {
    root.join(".khor").join("web.key")
}

/// The key as it is on disk right now, or `None` if this machine has no
/// face yet.
///
/// **Read on every request rather than cached.** It costs one 32-byte
/// read against a request whose own work is measured in milliseconds,
/// and it buys the thing the verb promises: `khor web --new` takes
/// effect on the next request, with no message to send to a running
/// serve and no window where the old link still opens.
pub fn web_key_read(root: &Path) -> Result<Option<String>, String> {
    let p = web_key_path(root);
    match std::fs::read_to_string(&p) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(msg::cant_read(p.display(), e)),
        Ok(text) => {
            check_permissions(&p)?;
            let key = text.trim();
            // **An empty file is not a key.** A truncated write leaves
            // one, and `khor_web::key::matches` is an honest comparison:
            // it would say yes to a request that presented nothing,
            // which is every request. Refusing here rather than in the
            // caller keeps the hole closed for callers that have not
            // been written yet.
            if key.is_empty() {
                return Err(msg::web_key_empty(p.display()));
            }
            Ok(Some(key.to_owned()))
        }
    }
}

/// The key, minting one if this machine has never had a face.
pub fn web_key_ensure(root: &Path) -> Result<String, String> {
    match web_key_read(root) {
        Ok(Some(key)) => Ok(key),
        Ok(None) => mint(root),
        Err(e) => Err(e),
    }
}

/// A new key, replacing whatever was there. Every link printed before
/// this call stops working — that is the whole point of the verb, so it
/// is not softened with a grace period: a key you can still use after
/// revoking it is not revoked.
pub fn web_key_rotate(root: &Path) -> Result<String, String> {
    mint(root)
}

fn mint(root: &Path) -> Result<String, String> {
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let key = crate::link::fresh_hex()?;
    crate::link::write_private(&web_key_path(root), key.as_bytes())?;
    Ok(key)
}

/// A loose key refuses to answer, the way a loose identity refuses to
/// start (`khor_net::identity`). On the machines this is for — shared
/// university boxes where every account can reach 127.0.0.1 — the mode
/// bits *are* the boundary between "my face" and "everyone's face", and
/// a key somebody else can read is one they can use silently, from
/// inside, forever.
fn check_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)
            .map_err(|e| msg::cant_read(path.display(), e))?
            .permissions()
            .mode()
            & 0o777;
        if mode & 0o077 != 0 {
            return Err(msg::web_key_too_open(
                format_args!("{mode:o}"),
                path.display(),
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
