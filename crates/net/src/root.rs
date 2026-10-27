//! The user's root key: generate, keep, and carry as 24 words
//! (docs/CONNECT.md 身份：用户根钥匙).
//!
//! # What it is and what it is not
//!
//! It **signs** — a device certificate, a roster of the user's devices
//! (`crate::trust`) — and does nothing else. It never binds an endpoint
//! and never encrypts a byte; that stays the device key's job
//! (`crate::identity`). So the two keys fail differently: losing a
//! device key costs one machine, losing the root key costs the ability
//! to add or remove machines, and **leaking** it hands somebody the
//! power to certify their own machine into the network. Hence the file
//! discipline below, which is `identity`'s and one step stricter.
//!
//! # The file
//!
//! 32 raw bytes, owner-only **from the first byte** (created with mode
//! 0600, never written then chmod'ed — a readable window is a window).
//! A looser mode refuses to load, for `identity`'s reason: silently using
//! a key the next account can read is far worse than stopping. It never
//! enters a synced document, a wire frame or a log line; nothing in this
//! module formats the secret except [`to_words`], whose whole job is to.
//!
//! # The words
//!
//! BIP-39, English list, 24 words: 256 bits of key plus an 8-bit
//! checksum. The same 32 bytes round-trip exactly — the words are the
//! key, not a seed that derives one — so any standard BIP-39 tool reads
//! what khor wrote. A wrong word, a wrong count and a bad checksum are
//! three different refusals, because they have three different fixes.

use std::path::Path;

use anyhow::{Context, Result};
use khor_catalog::msg;

/// How many words carry a key: 256 bits of entropy is 24 BIP-39 words.
pub const WORDS: usize = 24;

/// A fresh root key.
pub fn generate() -> iroh::SecretKey {
    iroh::SecretKey::generate()
}

/// The root key at `path`, or `None` when this machine does not hold
/// one. A file that is there but too open, unreadable or the wrong size
/// is an error, never a `None`: "no key" and "a key I must not use" lead
/// to different next steps.
pub fn load(path: &Path) -> Result<Option<iroh::SecretKey>> {
    if !path.exists() {
        return Ok(None);
    }
    check_private(path)?;
    let raw = std::fs::read(path).with_context(|| msg::root_key_corrupt(path.display()))?;
    let bytes: [u8; 32] = raw
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!(msg::root_key_corrupt(path.display())))?;
    Ok(Some(iroh::SecretKey::from_bytes(&bytes)))
}

/// Writes the key owner-only from the first byte, replacing whatever was
/// there. The directory is made if missing.
pub fn save(path: &Path, key: &iroh::SecretKey) -> Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| msg::cant_make_dir_for(dir.display()))?;
    }
    // Removed first so the mode below applies: `mode` only takes effect
    // when the open creates the file.
    let _ = std::fs::remove_file(path);
    let mut opts = std::fs::OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts
        .open(path)
        .with_context(|| msg::cant_write_file(path.display()))?;
    f.write_all(&key.to_bytes())
        .with_context(|| msg::cant_write_file(path.display()))?;
    Ok(())
}

/// Writes `key` at `path` **only if no key is there yet**, and
/// atomically: two processes opening a fresh store at the same moment
/// (the app and its resident, the first time) end with one root between
/// them rather than a file one of them overwrote. Answers whether this
/// call was the one that wrote it — the other caller then loads what is
/// there.
///
/// Written whole to a private temporary name first and then linked into
/// place, so nobody ever reads a half-written key.
pub fn create(path: &Path, key: &iroh::SecretKey) -> Result<bool> {
    static TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = TICK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_extension(format!("new-{}-{n}", std::process::id()));
    save(&tmp, key)?;
    let won = match std::fs::hard_link(&tmp, path) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => false,
        // A filesystem without links: the same promise, one step weaker
        // (a rename replaces), so it is taken only where nothing is there.
        Err(_) if !path.exists() => {
            std::fs::rename(&tmp, path).with_context(|| msg::cant_write_file(path.display()))?;
            true
        }
        Err(_) => false,
    };
    let _ = std::fs::remove_file(&tmp);
    Ok(won)
}

/// The key as 24 words, space-separated. The one function in khor that
/// turns the secret into something printable — callers decide who sees
/// it, and only after a person asked.
pub fn to_words(key: &iroh::SecretKey) -> String {
    bip39::Mnemonic::from_entropy(&key.to_bytes())
        // 32 bytes is a valid BIP-39 entropy length by construction.
        .expect("32 bytes is 24 words")
        .to_string()
}

/// The key back from its words. Whitespace and case are forgiven (the
/// words are written by hand); anything else is refused by name.
pub fn from_words(text: &str) -> Result<iroh::SecretKey> {
    let words: Vec<String> = text.split_whitespace().map(|w| w.to_lowercase()).collect();
    if words.len() != WORDS {
        anyhow::bail!(msg::root_words_count(WORDS, words.len()));
    }
    let joined = words.join(" ");
    let m = bip39::Mnemonic::parse_in_normalized(bip39::Language::English, &joined).map_err(
        |e| match e {
            bip39::Error::UnknownWord(i) => anyhow::anyhow!(msg::root_words_unknown(
                i + 1,
                words.get(i).map(String::as_str).unwrap_or("")
            )),
            bip39::Error::BadWordCount(n) => anyhow::anyhow!(msg::root_words_count(WORDS, n)),
            _ => anyhow::anyhow!(msg::root_words_checksum()),
        },
    )?;
    let (entropy, len) = m.to_entropy_array();
    let bytes: [u8; 32] = entropy[..len]
        .try_into()
        .map_err(|_| anyhow::anyhow!(msg::root_words_count(WORDS, words.len())))?;
    Ok(iroh::SecretKey::from_bytes(&bytes))
}

/// `identity`'s rule, with this file's words: a key anybody else can read
/// refuses to load.
fn check_private(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)?.permissions().mode() & 0o777;
        anyhow::ensure!(
            mode & 0o077 == 0,
            msg::root_key_too_open(format_args!("{mode:o}"), path.display())
        );
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temp directory that goes away however the test leaves.
    struct Tmp(std::path::PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn tmp(tag: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("khor-root-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }

    /// The prefix of a parameterized message up to its first argument —
    /// the stable part to assert on.
    fn head(s: String) -> String {
        s.split('\u{0}').next().unwrap().to_owned()
    }

    #[test]
    fn words_round_trip_to_the_same_32_bytes() {
        for _ in 0..8 {
            let k = generate();
            let w = to_words(&k);
            assert_eq!(w.split(' ').count(), WORDS, "24 words: {w}");
            let back = from_words(&w).unwrap();
            assert_eq!(back.to_bytes(), k.to_bytes());
            assert_eq!(back.public(), k.public());
        }
    }

    /// Words are written by hand: extra spaces, line breaks and capitals
    /// are the handwriting, not a different key.
    #[test]
    fn words_forgive_spacing_and_case() {
        let k = generate();
        let w = to_words(&k);
        let messy = format!("  {}\n", w.to_uppercase().replace(' ', "   \n "));
        assert_eq!(from_words(&messy).unwrap().to_bytes(), k.to_bytes());
    }

    /// A known vector: 32 zero bytes are the standard's own test words,
    /// so what khor prints is what every BIP-39 tool prints.
    #[test]
    fn the_standard_vector_reads_back() {
        let zero = iroh::SecretKey::from_bytes(&[0u8; 32]);
        let w = to_words(&zero);
        assert!(w.starts_with("abandon abandon"), "{w}");
        assert!(w.ends_with(" art"), "{w}");
        assert_eq!(from_words(&w).unwrap().to_bytes(), [0u8; 32]);
    }

    #[test]
    fn a_wrong_count_is_refused_by_count() {
        let k = generate();
        let w = to_words(&k);
        let short: Vec<&str> = w.split(' ').take(12).collect();
        let err = from_words(&short.join(" ")).unwrap_err().to_string();
        assert_eq!(err, msg::root_words_count(WORDS, 12));
    }

    #[test]
    fn a_word_off_the_list_is_refused_by_position() {
        let k = generate();
        let mut w: Vec<String> = to_words(&k).split(' ').map(str::to_owned).collect();
        w[4] = "khorword".into();
        let err = from_words(&w.join(" ")).unwrap_err().to_string();
        assert_eq!(err, msg::root_words_unknown(5, "khorword"));
    }

    /// Two real words swapped: every word is on the list, so only the
    /// checksum can catch it — and it must, or the wrong key is restored
    /// without a word said.
    #[test]
    fn a_swapped_pair_is_refused_by_the_checksum() {
        let k = generate();
        let words: Vec<String> = to_words(&k).split(' ').map(str::to_owned).collect();
        // Find a pair whose swap changes the text (two equal words would
        // swap into the same phrase).
        let (i, j) = (0..WORDS)
            .flat_map(|i| (i + 1..WORDS).map(move |j| (i, j)))
            .find(|&(i, j)| words[i] != words[j])
            .unwrap();
        let mut swapped = words.clone();
        swapped.swap(i, j);
        match from_words(&swapped.join(" ")) {
            Err(e) => assert_eq!(e.to_string(), msg::root_words_checksum()),
            // One in 256 swaps keeps the checksum by chance; then it must
            // at least be a *different* key, never silently the same one.
            Ok(other) => assert_ne!(other.to_bytes(), k.to_bytes()),
        }
    }

    #[test]
    fn the_key_survives_a_save_and_a_load() {
        let t = tmp("save");
        let path = t.0.join(".khor").join("root.key");
        assert!(load(&path).unwrap().is_none(), "no file is no key");
        let k = generate();
        save(&path, &k).unwrap();
        assert_eq!(load(&path).unwrap().unwrap().to_bytes(), k.to_bytes());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "owner-only from the first byte");
        }
    }

    /// Two creators, one key: the second finds the first's and says so.
    #[test]
    fn create_writes_once_and_never_overwrites() {
        let t = tmp("create");
        let path = t.0.join("root.key");
        let (a, b) = (generate(), generate());
        assert!(create(&path, &a).unwrap());
        assert!(!create(&path, &b).unwrap());
        assert_eq!(load(&path).unwrap().unwrap().to_bytes(), a.to_bytes());
        let leftovers: Vec<_> = std::fs::read_dir(&t.0).unwrap().flatten().collect();
        assert_eq!(leftovers.len(), 1, "no temporary is left behind");
    }

    #[cfg(unix)]
    #[test]
    fn a_group_readable_root_key_is_refused_rather_than_used() {
        use std::os::unix::fs::PermissionsExt;
        let t = tmp("perm");
        let path = t.0.join("root.key");
        save(&path, &generate()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        let err = load(&path).unwrap_err().to_string();
        assert!(
            err.starts_with(&head(msg::root_key_too_open('\u{0}', '\u{0}'))),
            "got: {err}"
        );
        assert!(
            err.contains("chmod 600"),
            "it must say how to fix it: {err}"
        );
    }

    #[test]
    fn a_key_of_the_wrong_size_is_refused() {
        let t = tmp("size");
        let path = t.0.join("root.key");
        let k = generate();
        save(&path, &k).unwrap();
        // Truncate in place, keeping the private mode.
        let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(31).unwrap();
        let err = load(&path).unwrap_err().to_string();
        assert!(
            err.starts_with(&head(msg::root_key_corrupt('\u{0}'))),
            "got: {err}"
        );
    }
}
