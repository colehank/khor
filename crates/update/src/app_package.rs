use crate::{Component, Offer, OperatingSystem};
use std::{
    fs::File,
    path::{Component as Part, Path, PathBuf},
};

pub struct AppCandidate {
    directory: tempfile::TempDir,
    program: PathBuf,
}

impl AppCandidate {
    pub fn extract(path: &Path, offer: &Offer, parent: &Path) -> Result<Self, String> {
        if offer.artifact().component != Component::DesktopApp
            || offer.artifact().platform.os != OperatingSystem::Macos
        {
            return Err("this package is not a macOS application".into());
        }
        let directory = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
        let file = File::open(path).map_err(|e| e.to_string())?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        let mut root: Option<PathBuf> = None;
        let mut bytes = 0u64;
        let mut count = 0u64;
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            count += 1;
            if count > 100_000 {
                return Err("application archive contains too many entries".into());
            }
            let path = entry.path().map_err(|e| e.to_string())?.into_owned();
            if path
                .components()
                .any(|part| !matches!(part, Part::Normal(_) | Part::CurDir))
            {
                return Err("application archive escapes its bundle".into());
            }
            let first = path
                .components()
                .find_map(|part| {
                    if let Part::Normal(name) = part {
                        Some(PathBuf::from(name))
                    } else {
                        None
                    }
                })
                .ok_or("empty application archive path")?;
            if first.extension().is_none_or(|extension| extension != "app")
                || root.as_ref().is_some_and(|root| root != &first)
            {
                return Err("application archive must contain exactly one app bundle".into());
            }
            root.get_or_insert(first);
            let kind = entry.header().entry_type();
            if kind.is_file() {
                bytes = bytes
                    .checked_add(entry.size())
                    .ok_or("application size overflow")?;
                if bytes > offer.artifact().unpacked_bytes {
                    return Err("application exceeds its signed unpacked size".into());
                }
            } else if kind.is_symlink() || kind.is_hard_link() {
                let target = entry
                    .link_name()
                    .map_err(|e| e.to_string())?
                    .ok_or("application link has no target")?;
                let at = if kind.is_symlink() {
                    path.parent().unwrap_or(Path::new(""))
                } else {
                    Path::new("")
                };
                if !link_stays_in_bundle(at, &target, root.as_ref().unwrap()) {
                    return Err("application link escapes its bundle".into());
                }
            } else if !kind.is_dir() {
                return Err("unsupported application archive entry".into());
            }
            if !entry
                .unpack_in(directory.path())
                .map_err(|e| e.to_string())?
            {
                return Err("application archive escapes its staging directory".into());
            }
        }
        let root = root.ok_or("empty application archive")?;
        let program = directory.path().join(root).join("Contents/MacOS/khor-gui");
        if !std::fs::symlink_metadata(&program)
            .map_err(|e| e.to_string())?
            .is_file()
        {
            return Err("application has no regular Khor executable".into());
        }
        Ok(Self { directory, program })
    }
    pub fn program(&self) -> &Path {
        &self.program
    }
    pub fn directory(&self) -> &Path {
        self.directory.path()
    }
}

fn link_stays_in_bundle(at: &Path, target: &Path, root: &Path) -> bool {
    let mut normal = PathBuf::new();
    for part in at.join(target).components() {
        match part {
            Part::Normal(name) => normal.push(name),
            Part::CurDir => {}
            Part::ParentDir => {
                if !normal.pop() {
                    return false;
                }
            }
            _ => return false,
        }
    }
    normal.starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive(path: &Path, files: &[(&str, &[u8])], link: Option<(&str, &str)>) {
        let file = File::create(path).unwrap();
        let compressed = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(compressed);
        for (name, bytes) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            archive.append_data(&mut header, name, *bytes).unwrap();
        }
        if let Some((name, target)) = link {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_mode(0o777);
            header.set_link_name(target).unwrap();
            header.set_cksum();
            archive
                .append_data(&mut header, name, std::io::empty())
                .unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap();
    }
    #[test]
    fn bundle_preflight_extracts_one_application_and_bounds_its_resources() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("app.tar.gz");
        let offer = crate::trust::fixture_app_offer();
        archive(
            &package,
            &[
                ("Khor.app/Contents/MacOS/khor-gui", b"program"),
                ("Khor.app/Contents/Resources/ui.bin", b"resources"),
            ],
            None,
        );
        let candidate = AppCandidate::extract(&package, &offer, root.path()).unwrap();
        assert_eq!(std::fs::read(candidate.program()).unwrap(), b"program");
        assert!(candidate.directory().is_dir());
        drop(candidate);
        archive(
            &package,
            &[("Khor.app/Contents/MacOS/khor-gui", &vec![0; 4097])],
            None,
        );
        assert!(AppCandidate::extract(&package, &offer, root.path()).is_err());
        archive(
            &package,
            &[
                ("Khor.app/Contents/MacOS/khor-gui", b"program"),
                ("Other.app/Contents/Resources/ui.bin", b"other"),
            ],
            None,
        );
        assert!(AppCandidate::extract(&package, &offer, root.path()).is_err());
    }
    #[test]
    fn bundle_preflight_rejects_a_link_that_would_escape_the_sdk_install_root() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("app.tar.gz");
        let offer = crate::trust::fixture_app_offer();
        archive(
            &package,
            &[("Khor.app/Contents/MacOS/khor-gui", b"program")],
            Some(("Khor.app/Contents/Resources/escaped", "../../../outside")),
        );
        assert!(AppCandidate::extract(&package, &offer, root.path()).is_err());
        assert!(!root.path().join("outside").exists());
    }
    #[test]
    fn framework_links_can_use_relative_parents_but_never_escape_the_app() {
        let root = Path::new("Khor.app");
        assert!(link_stays_in_bundle(
            Path::new("Khor.app/Contents/Frameworks/Foo.framework"),
            Path::new("Versions/Current/Foo"),
            root
        ));
        assert!(link_stays_in_bundle(
            Path::new("Khor.app/Contents/Resources"),
            Path::new("../Frameworks"),
            root
        ));
        assert!(!link_stays_in_bundle(
            Path::new("Khor.app/Contents"),
            Path::new("../../outside"),
            root
        ));
        assert!(!link_stays_in_bundle(
            Path::new("Khor.app/Contents"),
            Path::new("/etc/passwd"),
            root
        ));
    }
}
