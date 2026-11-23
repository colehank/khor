//! Release selection and installation. Runtime images and their leases remain
//! owned by khor-runtime; sessions remain owned by the node.
mod app_package;
mod build;
mod download;
mod feed;
#[cfg(unix)]
mod install;
mod package_stage;
mod release;
mod staging;
mod trust;

pub use app_package::AppCandidate;
pub use build::{BuildIdentity, build_identity};
pub use download::{Download, DownloadProgress};
pub use feed::Feed;
#[cfg(unix)]
pub use install::{CliInstallation, InstallReceipt, InstallState};
pub use package_stage::{PackageStage, collect_downloads};
pub use release::{
    Architecture, Artifact, Channel, Component, Digest, OperatingSystem, Platform, ReleaseIdentity,
    ReleaseManifest, ReleaseTag, SourceCommit,
};
pub use staging::Staging;
pub use trust::{Offer, SignedRelease, TrustRoot, VerifiedRelease};

/// The MSI owns the CLI and App in its registered installation directory.
#[cfg(windows)]
pub fn windows_installer_owns(executable: &std::path::Path) -> bool {
    use std::{ffi::OsString, path::PathBuf};
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    let Some(name) = executable.file_name().filter(|name| {
        name.eq_ignore_ascii_case("khor.exe") || name.eq_ignore_ascii_case("khor-gui.exe")
    }) else {
        return false;
    };
    let installed: std::io::Result<OsString> = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Khor")
        .and_then(|key| key.get_value("CliPath"));
    installed
        .ok()
        .and_then(|directory| PathBuf::from(directory).join(name).canonicalize().ok())
        .zip(executable.canonicalize().ok())
        .is_some_and(|(installed, running)| installed == running)
}
