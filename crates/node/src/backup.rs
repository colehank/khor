//! Keeping the store out of the system's backups (docs/IOS.md 第二步).
//!
//! # Why
//!
//! On a phone the store sits in the app's `Library/Application Support`
//! (`Node::sandbox_root`), and iOS backs that directory up — to iCloud
//! and to a computer — **identity key and root key included**. Restoring
//! that backup onto a new phone brings the old phone's key with it, and
//! then one key answers from two devices: the copied-store case the
//! owner stamp refuses on a shared home (`claim_store`), arriving through
//! a door the stamp cannot see, because the stamp travels with the copy.
//!
//! # What
//!
//! `NSURLIsExcludedFromBackupKey` on the `.khor` directory — Apple's own
//! answer for files an app can make again and must not be restored
//! (QA1719), and a directory carries its contents with it. A restored
//! phone then starts with no store and mints a new identity, which is
//! what a new device is. Keeping the keys in the keychain
//! (`ThisDeviceOnly`) is the stronger form and is not this step.
//!
//! **Once per process**: the store is opened on every command a face
//! sends, and the mark is an extended attribute written to disk.
//!
//! Elsewhere this does nothing: a computer's backup is its owner's, and
//! a restored Mac keeps being the same machine (`hardware::fingerprint`
//! answers for that).

use std::path::Path;

/// Marks `dir` as never backed up. Best effort, said once on failure:
/// a store that cannot be marked still opens — refusing would cost the
/// phone its place in the network over a backup that may never be made.
pub fn keep_out(dir: &Path) {
    #[cfg(target_os = "ios")]
    {
        static MARKED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        MARKED.get_or_init(|| {
            if let Err(e) = ios::exclude(dir) {
                eprintln!("{}", khor_catalog::msg::cant_write(dir.display(), e));
            }
        });
    }
    #[cfg(not(target_os = "ios"))]
    let _ = dir;
}

#[cfg(target_os = "ios")]
mod ios {
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSNumber, NSString, NSURL, NSURLIsExcludedFromBackupKey};

    pub(super) fn exclude(dir: &std::path::Path) -> Result<(), String> {
        let url = NSURL::fileURLWithPath(&NSString::from_str(&dir.to_string_lossy()));
        let yes = NSNumber::numberWithBool(true);
        let value: &AnyObject = &yes;
        // SAFETY: the key is the documented `NSURLIsExcludedFromBackupKey`
        // and its value is the `NSNumber` boolean the documentation names.
        unsafe { url.setResourceValue_forKey_error(Some(value), NSURLIsExcludedFromBackupKey) }
            .map_err(|e| e.localizedDescription().to_string())
    }
}
