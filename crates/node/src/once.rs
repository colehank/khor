//! **A session made at most once per token** (round NET-7, from ux's
//! first-run walk): the phone asked the Mac for a session, the Mac quit
//! before the answer came back, the phone said 「studio 没有响应」 — and
//! the session had been started. Its row arrived when the Mac was back,
//! by which time the person may have pressed 创建 again and had two.
//!
//! So a create carries a token the asker chose before asking
//! (`proto::Request::OpenOnce`, `OpenAgentOnce`), and the machine that
//! makes the session keeps which session each token made, **on disk**,
//! beside its registry: a retry after a lost answer — the same press,
//! or the asker's own second ask — is answered with the session the
//! first one made, across a restart of either end. Stripe's
//! `Idempotency-Key` (<https://docs.stripe.com/api/idempotent_requests>)
//! is the same arrangement for the same failure ("the
//! server … saves the resulting status code and body of the first
//! request … subsequent requests with the same key return the same
//! result").
//!
//! One token at a time per process (`lock`): two retries in flight wait
//! for each other, and the second finds what the first made.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// How long a token's record is kept. A retry is seconds to minutes
/// after the first ask; a day covers a machine that was off overnight.
const KEPT_MS: u64 = 24 * 60 * 60 * 1000;

/// A token is what a face makes (`crypto.randomUUID()`): letters, digits
/// and dashes, long enough not to meet another by chance, short enough
/// to be a file name everywhere.
pub(crate) fn valid(token: &str) -> bool {
    (16..=64).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn dir(root: &Path) -> PathBuf {
    root.join(".khor").join("opened")
}

/// The session this token made, if it made one.
pub(crate) fn made(root: &Path, token: &str) -> Option<String> {
    let id = std::fs::read_to_string(dir(root).join(token)).ok()?;
    let id = id.trim();
    (!id.is_empty()).then(|| id.to_owned())
}

/// Records what this token made, and forgets records older than
/// [`KEPT_MS`] while it is there.
pub(crate) fn note(root: &Path, token: &str, id: &str) -> Result<(), String> {
    let d = dir(root);
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now();
    if let Ok(rd) = std::fs::read_dir(&d) {
        for e in rd.flatten() {
            let old = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| now.duration_since(t).ok())
                .is_some_and(|age| age.as_millis() as u64 > KEPT_MS);
            if old {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    crate::live::write_whole(&d.join(token), id.as_bytes())
}

type Locks = Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>;
static LOCKS: std::sync::OnceLock<Locks> = std::sync::OnceLock::new();

/// Holds `token` for this process until the guard drops.
pub(crate) async fn lock(token: &str) -> tokio::sync::OwnedMutexGuard<()> {
    let one = LOCKS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .entry(token.to_owned())
        .or_default()
        .clone();
    one.lock_owned().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_a_safe_file_name_and_only_that() {
        assert!(valid("3f2b9c1e-8a7d-4e5f-9b0c-1d2e3f4a5b6c"));
        for bad in ["short", "../../etc/passwd-xxxxxxxx", "a b c d e f g h i j", &"x".repeat(65)] {
            assert!(!valid(bad), "{bad:?}");
        }
    }

    #[test]
    fn what_a_token_made_is_found_again() {
        let root = std::env::temp_dir().join(format!("khor-once-{}", std::process::id()));
        let token = "3f2b9c1e-8a7d-4e5f-9b0c-1d2e3f4a5b6c";
        assert_eq!(made(&root, token), None);
        note(&root, token, "shell/abc").unwrap();
        assert_eq!(made(&root, token).as_deref(), Some("shell/abc"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
