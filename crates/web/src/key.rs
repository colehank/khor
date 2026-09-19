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

/// Where this machine keeps its key.
///
/// **The whole of「where the key is and how it is minted」lives in
/// `khor_node::face`**, and the dependency graph is the reason: `khor
/// doctor` reports whether this file is there, 安全 shows the key and
/// rotates it, and both of those are the node's — while this crate
/// depends on the node rather than the other way round. What stays here
/// is the serving: the request loop above, and [`matches`] below.
pub fn path(root: &Path) -> PathBuf {
    khor_node::face::web_key_path(root)
}

/// The key, minting one if this machine has never had a face.
pub fn ensure(root: &Path) -> Result<String, String> {
    khor_node::face::web_key_ensure(root)
}

/// A new key, replacing whatever was there.
pub fn rotate(root: &Path) -> Result<String, String> {
    khor_node::face::web_key_rotate(root)
}

/// The key as it is on disk right now, or `None` if this machine has no
/// face yet. Read on every request rather than cached — see
/// `khor_node::face`.
pub fn read(root: &Path) -> Result<Option<String>, String> {
    khor_node::face::web_key_read(root)
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
