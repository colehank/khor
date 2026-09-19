//! `khor lock`: the password on the faces (docs/MORE.md 安全).
//!
//! # What it locks, and what it deliberately does not
//!
//! **The screens.** The app's window and the web face — the two places
//! khor draws a conversation. A locked face draws a lock screen and
//! nothing else: the session tree is *unmounted*, not covered, so there
//! is no layer to peel back and nothing in the document to read.
//!
//! **Not the CLI.** Behind `khor` there is already somebody logged into
//! this machine; a prompt there would be theatre in front of a shell
//! that can read `.khor/` directly. The thing this guards is a screen
//! left open on a desk, which is a different threat and the only one a
//! screen lock can answer.
//!
//! # The PIN is the key every face has
//!
//! macOS can also unlock with Touch ID or the login password
//! (`src-tauri`'s `system_unlock`, the one door that is the app's
//! alone). It is an *extra*: a PIN must be set before the lock can be
//! turned on at all, because the web face a phone opens has no
//! LocalAuthentication to fall back to, and a lock whose only key lives
//! on one machine is a lock that shuts people out of the other faces.
//!
//! Stored as an **Argon2id hash** and never as the PIN — `.khor/` is a
//! directory on a shared box (docs/NET.md's own reason for the 0600
//! identity key), and the whole cost of hashing is paid once per unlock.
//!
//! # It is a local preference, not a synced document
//!
//! `.khor/lock.json`, beside `avatar.json` and the relay list, for the
//! reason those two are local: **the choice is made here and never
//! travels**. A synced lock would mean a machine somebody paired could
//! lock every other machine in the fleet, and a synced PIN hash would
//! put one on every disk in it.

use std::path::{Path, PathBuf};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Argon2, password_hash::rand_core::OsRng};
use khor_catalog::msg;

/// The shortest PIN khor will take.
///
/// **Characters, not digits.** 「PIN」 is what the ruling called it and
/// what the numeric field on a phone suggests, but refusing a longer
/// passphrase because it has letters in it would be refusing the
/// stronger answer. Six is the floor a four-digit habit has to clear.
pub const PIN_MIN: usize = 6;

/// The idle timeouts a face offers, in minutes. `0` is 永不.
///
/// **Here rather than in the two faces**, so the CLI and the page cannot
/// come to offer different ladders — and so `khor lock timeout 7` is
/// refused by name rather than quietly kept as a rung nothing draws.
pub const TIMEOUTS: &[u32] = &[0, 1, 5, 15, 60];

/// What a face needs to know about the lock. **Never the hash**: a
/// screen has no use for it, and a value that crosses a wire is a value
/// that ends up in a log.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LockState {
    /// Whether the faces lock at all.
    pub on: bool,
    /// Whether a PIN has been set. The lock cannot be turned on without
    /// one, so `on` implies this — but they are two facts, and the
    /// screen that offers 「设一个 PIN」 reads the second one.
    pub has_pin: bool,
    /// Minutes of idleness before a face locks itself; `0` is never.
    pub timeout_min: u32,
}

/// What is actually on disk. The hash lives here and goes no further.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct Stored {
    #[serde(default)]
    on: bool,
    #[serde(default)]
    pin: Option<String>,
    #[serde(default)]
    timeout_min: u32,
}

fn path(root: &Path) -> PathBuf {
    root.join(".khor").join("lock.json")
}

/// **Unreadable and unparseable both fall back to「no lock」**, the same
/// way a garbled `avatar.json` falls back to the factory face — and the
/// direction is chosen rather than inherited. A lock that defaulted to
/// *on* with no readable hash would be a machine nobody can unlock,
/// mended only by deleting a file from a terminal, which is precisely
/// the face this feature is not allowed to wear.
fn stored(root: &Path) -> Stored {
    std::fs::read_to_string(path(root))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(root: &Path, s: &Stored) -> Result<(), String> {
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let text = serde_json::to_vec(s).map_err(|e| e.to_string())?;
    // 0600 like every other secret in this directory: the hash is not a
    // key, but it is a thing worth nobody else's while to grind.
    crate::link::write_private(&path(root), &text)
}

fn seen(s: &Stored) -> LockState {
    LockState {
        // **Read as off when there is no PIN**, whatever the file says.
        // The only way to reach that state is editing the file by hand,
        // and honouring it would lock a machine with no key to it.
        on: s.on && s.pin.is_some(),
        has_pin: s.pin.is_some(),
        timeout_min: s.timeout_min,
    }
}

/// How this machine's faces are set to lock.
pub fn state(root: &Path) -> LockState {
    seen(&stored(root))
}

/// Turns the lock on or off.
///
/// **Refused by name when there is no PIN**, rather than quietly
/// staying off: a switch that turns itself back off is a screen lying
/// about its own state (`Node::set_relays` refuses the same shape for
/// the same reason).
pub fn set_on(root: &Path, on: bool) -> Result<LockState, String> {
    let mut s = stored(root);
    if on && s.pin.is_none() {
        return Err(msg::lock_needs_a_pin().to_owned());
    }
    s.on = on;
    save(root, &s)?;
    Ok(seen(&s))
}

/// Sets the PIN, or clears it when handed an empty one.
///
/// **Clearing turns the lock off in the same move.** Splitting those is
/// how a machine ends up claiming to be locked with no key to it — the
/// same judgment `Node::set_avatar_style` makes about persisting and
/// reporting together.
pub fn set_pin(root: &Path, pin: &str) -> Result<LockState, String> {
    let mut s = stored(root);
    if pin.is_empty() {
        s.pin = None;
        s.on = false;
        save(root, &s)?;
        return Ok(seen(&s));
    }
    if pin.chars().count() < PIN_MIN {
        return Err(msg::lock_pin_too_short(PIN_MIN));
    }
    let salt = SaltString::generate(&mut OsRng);
    s.pin = Some(
        Argon2::default()
            .hash_password(pin.as_bytes(), &salt)
            .map_err(msg::lock_cant_hash)?
            .to_string(),
    );
    save(root, &s)?;
    Ok(seen(&s))
}

/// Whether this is the PIN.
///
/// **`Ok(false)` and `Err` are different answers.** A wrong PIN is an
/// ordinary event a face says 「不对」 to; a hash this build cannot parse
/// is khor being broken, and folding it into「wrong」 would have somebody
/// retyping a correct PIN forever.
pub fn check_pin(root: &Path, pin: &str) -> Result<bool, String> {
    let s = stored(root);
    let Some(stored_hash) = s.pin else {
        return Ok(false);
    };
    let parsed = PasswordHash::new(&stored_hash).map_err(msg::lock_cant_hash)?;
    Ok(Argon2::default()
        .verify_password(pin.as_bytes(), &parsed)
        .is_ok())
}

/// Sets the idle timeout, in minutes. `0` is never.
///
/// Refused by name for a rung that is not on the ladder — a number the
/// faces do not draw would be a setting somebody could reach from the
/// CLI and never see again (`TIMEOUTS`).
pub fn set_timeout(root: &Path, minutes: u32) -> Result<LockState, String> {
    if !TIMEOUTS.contains(&minutes) {
        return Err(msg::lock_no_such_timeout(
            minutes,
            TIMEOUTS
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(khor_catalog::cli::name_separator()),
        ));
    }
    let mut s = stored(root);
    s.timeout_min = minutes;
    save(root, &s)?;
    Ok(seen(&s))
}

/// Throws this machine's identity away and mints a new one — **the same
/// thing as replacing the computer**, as far as the network is
/// concerned (docs/MORE.md 重置身份).
///
/// Every paired machine goes on knowing the old id and will not know
/// the new one: this side stops being reachable, and pairing has to be
/// done again from a machine that still has a table. Nothing is sent to
/// anyone, because there is no longer a key to sign it with.
///
/// **The machine's own name is the confirmation**, and it is checked
/// here rather than on a screen: a CLI flag and a dialog would then be
/// two rules about the same irreversible act, and the one that drifted
/// would be the one nobody ran.
pub fn reset_identity(root: &Path, name_typed: &str, name: &str) -> Result<(), String> {
    if name_typed.trim() != name {
        return Err(msg::reset_wants_the_machine_name(name));
    }
    let key = crate::doctor::identity_path(root);
    match std::fs::remove_file(&key) {
        Ok(()) => Ok(()),
        // Already gone is the state this asks for. Answering an error
        // would send somebody looking for a file that is not there.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(msg::cant_write(key.display(), e)),
    }
}
