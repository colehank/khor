//! Is this the same physical machine as last time?
//!
//! One question, asked by the two places that must not be fooled by a
//! rename: [`crate::claim_store`] ("is this store mine") and
//! `Node::store_under` ("which store under a shared home is mine").
//!
//! # Why the hostname was the wrong thing to ask
//!
//! Both used to compare hostnames, and a hostname is a **label** — the
//! thing this batch exists to let people change. Renaming a box made
//! khor refuse to open its own store, and the refusal said 「共用 home
//! 的机器各要一份:给这台设 KHOR_HOME」, which is advice for a
//! different problem: following it gives one machine a second store and
//! a second identity. Measured 2026-09-11, exit 1 on a store whose
//! stamp and hostname disagreed.
//!
//! That is this repo's own judgment about proxies (CLAUDE.md 假绿 15):
//! the hostname was standing in for 「same machine」 and the day it
//! stopped standing for it, nothing said so.
//!
//! # What is asked instead
//!
//! The id the OS keeps for the box, which survives a rename:
//!
//! | | where |
//! |---|---|
//! | Linux | `/etc/machine-id`, else `/var/lib/dbus/machine-id` |
//! | macOS | `IOPlatformUUID` out of `ioreg` |
//!
//! **Hashed before it is written down.** Both are stable identifiers of
//! somebody's computer, and `~/.khor/owner` is a file that travels on a
//! shared mount and gets pasted into bug reports. The hash answers the
//! only question asked of it — 「the same one as before?」 — and answers
//! nothing else.
//!
//! **`None` is a real answer**, not a failure: a platform with no such
//! id, a container with the file missing, an `ioreg` that is not there.
//! The callers fall back to comparing names, which is exactly what they
//! did before this module existed — so a machine that cannot be
//! fingerprinted is no worse off than it was, and one that can is
//! rename-proof. Nothing here returns an error, because there is
//! nothing a caller could do with one.

use std::process::Command;

/// This machine's stable id, hashed. `None` when the platform will not
/// say — see the module head on why that is not an error.
///
/// # Asked once per process
///
/// On macOS the platform will only say through `ioreg`, which is a
/// child process — and `Node::root_from_env` reaches this on **every**
/// command a face sends, several a second while a window is open.
/// Measured 2026-09-21 on a desktop app that had been up four hours:
/// the fork was the hottest frame under the IPC handler, on the thread
/// the webview waits on. The answer is a fact about the hardware this
/// process is running on; it cannot change while the process lives, so
/// it is read once and kept.
pub fn fingerprint() -> Option<String> {
    static KEPT: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    KEPT.get_or_init(|| {
        ASKED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        raw_id().map(|raw| hashed(raw.trim()))
    })
    .clone()
}

/// How many times this process actually went and asked the platform —
/// what the test below reads, since the point of the cache is a fork
/// that does not happen and an answer alone cannot show that.
static ASKED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(target_os = "linux")]
fn raw_id() -> Option<String> {
    for p in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(s) = std::fs::read_to_string(p) {
            if !s.trim().is_empty() {
                return Some(s);
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn raw_id() -> Option<String> {
    let out = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    // `      "IOPlatformUUID" = "CAE7694A-…"` — the value is the second
    // quoted run on its line, so splitting on the quote is enough and
    // there is no format to keep in step with.
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.contains("IOPlatformUUID"))
        .and_then(|l| l.split('"').nth(3).map(str::to_owned))
        .filter(|s| !s.trim().is_empty())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn raw_id() -> Option<String> {
    let _ = Command::new("true");
    None
}

/// blake3, hex, first 32 chars — the hash this workspace already
/// carries, so this adds no dependency. Short because it is only ever
/// compared with itself; full width would just make the file wider.
fn hashed(raw: &str) -> String {
    // Domain-separated: the same box hashed for some other purpose one
    // day must not come out as the same string as this one.
    let mut h = blake3::Hasher::new();
    h.update(b"khor/machine/v1:");
    h.update(raw.as_bytes());
    h.finalize().to_hex()[..32].to_string()
}

#[cfg(test)]
mod tests {
    /// Two arms, and the second is the one that matters: a digest that
    /// ignored its input would pass the first.
    #[test]
    fn the_same_box_hashes_the_same_and_a_different_one_does_not() {
        let a = super::hashed("one-box");
        assert_eq!(a, super::hashed("one-box"), "the same id hashes the same");
        assert_ne!(
            a,
            super::hashed("another-box"),
            "a different id hashes differently"
        );
        assert_eq!(a.len(), 32, "the width written into the owner file");
        assert!(!a.contains("one-box"), "the raw id is not carried through");
    }

    /// Whatever this machine is, asking twice must not give two
    /// answers — a fingerprint that wobbled would lock somebody out of
    /// their own store on the second run rather than the first.
    #[test]
    fn asking_this_machine_twice_gives_one_answer() {
        assert_eq!(super::fingerprint(), super::fingerprint());
    }
}

#[cfg(test)]
mod asked_once {
    /// **The second ask costs nothing.** `Node::root_from_env` stands
    /// behind every command a face sends; an `ioreg` per command was a
    /// child process several times a second for as long as a window was
    /// open.
    #[test]
    fn the_platform_is_asked_once_however_often_khor_asks() {
        let first = super::fingerprint();
        for _ in 0..50 {
            assert_eq!(super::fingerprint(), first, "and the answer does not move");
        }
        assert_eq!(
            super::ASKED.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "fifty-one asks reached the platform once"
        );
    }
}
