//! Release selection and installation. Runtime images and their leases remain
//! owned by khor-runtime; sessions remain owned by the node.
mod build;
mod download;
mod feed;
#[cfg(unix)]
mod install;
mod release;
mod trust;

pub use build::{BuildIdentity, build_identity};
pub use download::{Download, DownloadProgress};
pub use feed::Feed;
#[cfg(unix)]
pub use install::{CliInstallation, InstallReceipt, InstallState};
pub use release::{
    Architecture, Artifact, Channel, Component, Digest, OperatingSystem, Platform, ReleaseIdentity,
    ReleaseManifest, ReleaseTag, SourceCommit,
};
pub use trust::{Offer, TrustRoot, VerifiedRelease};
