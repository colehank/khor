//! The web face's key: the one thing a browser must show to get data.
//!
//! **Called a key and not a token** because `Tokens` in this codebase is
//! already what an agent spends (`khor_core::Tokens`, the usage pane).
//! One word, two unrelated meanings, in a tree where "how many tokens"
//! is a real question somebody asks — the collision would be in every
//! grep forever.
//!
//! It is khor handing you a key, not you inventing a password. Nobody
//! chooses it, nobody can choose it badly, and it is 128 random bits
//! from the same mint as the hand-off cookies (`khor_node::link`).
//!
//! # It lives on the machine, not in the mesh
//!
//! `.khor/web.key`, mode 0600, beside `endpoint.json` — **never in a
//! synced document.** The ledger's rule is that copying a document must
//! not copy a secret, and every CRDT document here lands on every
//! machine in the network by design. A key that synced would mean
//! pairing a laptop silently handed it the keys to every face in the
//! fleet.

use std::path::{Path, PathBuf};

use khor_catalog::msg;

/// Where this machine keeps its key.
pub fn path(root: &Path) -> PathBuf {
    root.join(".khor").join("web.key")
}

/// The key, minting one if this machine has never had a face.
pub fn ensure(root: &Path) -> Result<String, String> {
    match read(root) {
        Ok(Some(key)) => Ok(key),
        Ok(None) => mint(root),
        Err(e) => Err(e),
    }
}

/// A new key, replacing whatever was there. Every link printed before
/// this call stops working — that is the whole point of the verb, so it
/// is not softened with a grace period: a key you can still use after
/// revoking it is not revoked.
pub fn rotate(root: &Path) -> Result<String, String> {
    mint(root)
}

/// The key as it is on disk right now, or `None` if this machine has no
/// face yet.
///
/// **Read on every request rather than cached.** It costs one 32-byte
/// read against a request whose own work is measured in milliseconds,
/// and it buys the thing the verb promises: `khor web --new` takes
/// effect on the next request, with no message to send to a running
/// serve and no window where the old link still opens.
pub fn read(root: &Path) -> Result<Option<String>, String> {
    let p = path(root);
    match std::fs::read_to_string(&p) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(msg::cant_read(p.display(), e)),
        Ok(text) => {
            check_permissions(&p)?;
            let key = text.trim();
            // **An empty file is not a key.** A truncated write leaves
            // one, and [`matches`] is an honest comparison: it would
            // say yes to a request that presented nothing, which is
            // every request. Refusing here rather than in the caller
            // keeps the hole closed for callers that have not been
            // written yet.
            if key.is_empty() {
                return Err(msg::web_key_empty(p.display()));
            }
            Ok(Some(key.to_owned()))
        }
    }
}

fn mint(root: &Path) -> Result<String, String> {
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let key = khor_node::link::fresh_hex()?;
    khor_node::link::write_private(&path(root), key.as_bytes())?;
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
            return Err(msg::web_key_too_open(format_args!("{mode:o}"), path.display()));
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Whether a presented key is the real one, without leaking where two
/// keys first differ.
///
/// The timing this hides is not plausibly measurable over a LAN against
/// 128 bits — it is here because the cheap version is four lines and
/// the argument for skipping it is the kind that ages badly. Length is
/// compared openly: the length of a key khor mints is not a secret.
pub fn matches(presented: &str, real: &str) -> bool {
    let (a, b) = (presented.as_bytes(), real.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |seen, (x, y)| seen | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn a_key_matches_only_itself() {
        assert!(matches("abc", "abc"));
        assert!(!matches("abc", "abd"));
        assert!(!matches("abc", "abcd"), "a prefix is not the key");
        assert!(!matches("abcd", "abc"), "nor is an extension of it");
        assert!(!matches("", "abc"));
        // **Two empty strings do match, and that is why `read` refuses
        // an empty file.** A truncated write leaves one; a request with
        // no key presents the other; this function is an honest
        // comparison and would let them meet. The guard belongs where
        // the key is loaded, not here — see `read`.
        assert!(matches("", ""));
    }
}
