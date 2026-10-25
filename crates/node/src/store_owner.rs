//! Store claims and read-only ownership checks. No Node is opened here.

/// What a store's `owner` file says. Two fields, either of which may
/// be missing: the machine's stable fingerprint ([`crate::hardware`])
/// and the name it went by when it claimed the store.
///
/// ```text
/// machine 3f1a…            ← the claim
/// name    turing           ← a note for whoever reads the file
/// ```
///
/// **A bare line is the old format**, a hostname and nothing else, and
/// it reads back as `name` alone. Every store on disk before
/// 2026-09-11 is one of those.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Owner {
    pub(crate) machine: Option<String>,
    pub(crate) name: Option<String>,
}

impl Owner {
    pub(crate) fn parse(text: &str) -> Owner {
        let text = text.trim();
        if text.is_empty() {
            return Owner::default();
        }
        if !text.contains('\n') && !text.starts_with("machine ") && !text.starts_with("name ") {
            // The old format: one bare hostname.
            return Owner {
                machine: None,
                name: Some(text.to_string()),
            };
        }
        let mut out = Owner::default();
        for line in text.lines() {
            let Some((k, v)) = line.trim().split_once(char::is_whitespace) else {
                continue;
            };
            let v = v.trim();
            if v.is_empty() {
                continue;
            }
            match k {
                "machine" => out.machine = Some(v.to_string()),
                "name" => out.name = Some(v.to_string()),
                _ => {}
            }
        }
        out
    }

    pub(crate) fn render(&self) -> String {
        let mut out = String::new();
        if let Some(m) = &self.machine {
            out.push_str("machine ");
            out.push_str(m);
            out.push('\n');
        }
        if let Some(n) = &self.name {
            out.push_str("name ");
            out.push_str(n);
            out.push('\n');
        }
        out
    }

    /// Whether this stamp is **this** machine's. `here` is the current
    /// hostname, `fp` this machine's fingerprint when it has one.
    ///
    /// The fingerprint decides whenever both sides have one, and only
    /// then does a rename become free. The name is the fallback for the
    /// two cases where it cannot: a store stamped before this existed,
    /// and a platform that will not name its hardware. **An empty stamp
    /// belongs to whoever asks** — that is how a half-made store from a
    /// crashed first run gets finished instead of becoming unopenable.
    pub(crate) fn is_mine(&self, here: &str, fp: Option<&str>) -> bool {
        match (&self.machine, fp) {
            (Some(theirs), Some(mine)) => theirs == mine,
            _ => match &self.name {
                Some(n) => n == here,
                None => true,
            },
        }
    }
}

#[derive(Debug)]
pub(crate) enum VerificationError {
    Io(std::io::Error),
    Invalid(&'static str),
    Foreign(String),
}

impl From<std::io::Error> for VerificationError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

// Opening a store may complete a blank claim left by a first run. Removing a
// store requires positive evidence, so malformed/ambiguous claims fail closed.
fn removal_claim(text: &str) -> Result<Owner, VerificationError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(VerificationError::Invalid("empty owner claim"));
    }
    let mut machine = false;
    let mut name = false;
    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        match fields.as_slice() {
            [legacy] if text.lines().count() == 1 && !matches!(*legacy, "machine" | "name") => {}
            ["machine", _] if !machine => machine = true,
            ["name", _] if !name => name = true,
            _ => return Err(VerificationError::Invalid("malformed owner claim")),
        }
    }
    let owner = Owner::parse(text);
    if machine != owner.machine.is_some() || (name && owner.name.is_none()) {
        return Err(VerificationError::Invalid("unsupported owner claim format"));
    }
    Ok(owner)
}

/// Verify an existing claim without creating directories, identities, or a
/// refreshed stamp. This is deliberately stricter than first-run store opening.
pub(crate) fn verify(
    root: &std::path::Path,
    here: &str,
    fingerprint: Option<&str>,
) -> Result<(), VerificationError> {
    use std::path::Component;
    if !root.is_absolute()
        || root
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(VerificationError::Invalid(
            "store root must be an absolute path without traversal",
        ));
    }
    // A proof for a different path reached through a link is not authority to
    // remove this path. Check ancestors too, including a linked hosts directory.
    for path in root.ancestors() {
        let metadata = std::fs::symlink_metadata(path)?;
        if metadata.is_symlink() || !metadata.is_dir() {
            return Err(VerificationError::Invalid(
                "store path contains a symlink or non-directory",
            ));
        }
    }
    let store = root.join(".khor");
    let metadata = std::fs::symlink_metadata(&store)?;
    if metadata.is_symlink() || !metadata.is_dir() {
        return Err(VerificationError::Invalid("store must be a real directory"));
    }
    let claim = store.join("owner");
    let metadata = std::fs::symlink_metadata(&claim)?;
    if metadata.is_symlink() || !metadata.is_file() {
        return Err(VerificationError::Invalid("owner must be a regular file"));
    }
    let owner = removal_claim(&std::fs::read_to_string(claim)?)?;
    if owner.machine.is_some() && fingerprint.is_none() {
        return Err(VerificationError::Invalid(
            "cannot verify a machine claim without this machine's fingerprint",
        ));
    }
    if !owner.is_mine(here, fingerprint) {
        return Err(VerificationError::Foreign(
            owner.name.unwrap_or_else(|| "another machine".into()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            // macOS /var is a symlink. Test paths are real paths, as required by
            // the destructive-operation verifier, instead of hiding that check.
            let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "khor-owner-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn claim(&self, text: &str) {
            fs::create_dir_all(self.0.join(".khor")).unwrap();
            fs::write(self.0.join(".khor/owner"), text).unwrap();
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut out = BTreeMap::new();
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_symlink() {
                out.insert(
                    path.clone(),
                    fs::read_link(path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                );
            } else if metadata.is_dir() {
                out.insert(path.clone(), Vec::new());
                out.extend(snapshot(&path));
            } else {
                out.insert(path.clone(), fs::read(path).unwrap());
            }
        }
        out
    }
    fn unchanged(root: &Path, here: &str, fp: Option<&str>, expected: bool) {
        let before = snapshot(root);
        assert_eq!(verify(root, here, fp).is_ok(), expected);
        assert_eq!(
            snapshot(root),
            before,
            "verification must not mutate any store file"
        );
        assert!(
            !root.join(".khor/identity.key").exists(),
            "verification must not create an identity"
        );
    }

    #[test]
    fn same_name_is_not_same_machine() {
        let home = Scratch::new();
        home.claim("machine A\nname A\n");
        let room = home.0.join(".khor-hosts/B");
        fs::create_dir_all(room.join(".khor")).unwrap();
        fs::write(room.join(".khor/owner"), "machine C\nname B\n").unwrap();
        let before = snapshot(&home.0);
        unchanged(&room, "B", Some("B"), false);
        assert_eq!(
            snapshot(&home.0),
            before,
            "A's front store must also remain unchanged"
        );
    }
    #[test]
    fn fingerprint_survives_rename_without_refreshing_owner() {
        let root = Scratch::new();
        root.claim("machine B\nname old-B\n");
        unchanged(&root.0, "new-B", Some("B"), true);
        unchanged(&root.0, "old-B", None, false);
    }
    #[test]
    fn legacy_claims_use_the_existing_name_rule() {
        for text in ["B\n", "name B\n"] {
            let root = Scratch::new();
            root.claim(text);
            unchanged(&root.0, "B", None, true);
            unchanged(&root.0, "C", Some("B"), false);
        }
    }
    #[test]
    fn absent_empty_or_malformed_claims_never_become_owned() {
        let root = Scratch::new();
        unchanged(&root.0, "B", Some("B"), false);
        fs::create_dir(root.0.join(".khor")).unwrap();
        unchanged(&root.0, "B", Some("B"), false);
        for text in [
            "",
            "\n",
            "garbage words\n",
            "machine B\nmachine C\nname B\n",
            "name B\nname C\n",
            "machine 0\nmachine 0\nname B\n",
            "machine\n",
            "name\n",
            "machine B\nunknown field\n",
        ] {
            root.claim(text);
            unchanged(&root.0, "B", Some("B"), false);
        }
    }
    #[test]
    fn zero_fingerprint_is_still_a_machine_claim() {
        for fp in ["0", "00", "0.0"] {
            let root = Scratch::new();
            root.claim(&format!("machine {fp}\nname B\n"));
            unchanged(&root.0, "B", None, false);
            unchanged(&root.0, "B", Some("different"), false);
            unchanged(&root.0, "renamed", Some(fp), true);
        }
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_ownership_proofs() {
        use std::os::unix::fs::symlink;
        let root = Scratch::new();
        root.claim("machine B\nname B\n");
        let link = Scratch::new();
        symlink(&root.0, link.0.join("room")).unwrap();
        assert!(verify(&link.0.join("room"), "B", Some("B")).is_err());
        fs::create_dir_all(root.0.join("child/.khor")).unwrap();
        fs::write(root.0.join("child/.khor/owner"), "B\n").unwrap();
        assert!(verify(&link.0.join("room/child"), "B", Some("B")).is_err());
        symlink(root.0.join(".khor"), link.0.join(".khor")).unwrap();
        assert!(verify(&link.0, "B", Some("B")).is_err());
        fs::remove_file(link.0.join(".khor")).unwrap();
        fs::create_dir(link.0.join(".khor")).unwrap();
        symlink(root.0.join(".khor/owner"), link.0.join(".khor/owner")).unwrap();
        assert!(verify(&link.0, "B", Some("B")).is_err());
        assert_eq!(
            fs::read_to_string(root.0.join(".khor/owner")).unwrap(),
            "machine B\nname B\n"
        );
    }
}
