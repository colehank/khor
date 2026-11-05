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
