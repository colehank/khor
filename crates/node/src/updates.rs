use crate::runtime_image;
use khor_runtime::Contract;
pub use khor_update::{BuildIdentity, Channel, DownloadProgress, ReleaseIdentity, ReleaseTag};
use khor_update::{Component, Download, Feed, Offer, Platform, TrustRoot};
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::Path};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramInfo {
    pub schema: u8,
    pub build: BuildIdentity,
    pub platform: Platform,
    pub component: Component,
    pub contract: Contract,
}

pub fn program_info() -> Result<ProgramInfo, String> {
    let program = crate::self_exe()?;
    let component = if program
        .file_name()
        .is_some_and(|name| name == "khor-gui" || name == "khor-gui.exe")
    {
        Component::DesktopApp
    } else {
        Component::Cli
    };
    Ok(ProgramInfo {
        schema: 1,
        build: khor_update::build_identity()?,
        platform: Platform::current()?,
        component,
        contract: runtime_image::release_contract()?,
    })
}

pub fn inspect(program: &Path) -> Result<ProgramInfo, String> {
    let bytes = runtime_image::probe(program, &[OsStr::new("_release_info")])?;
    let info: ProgramInfo = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if info.schema != 1 {
        return Err("unsupported executable release identity".into());
    }
    Ok(info)
}

pub fn verify_program(program: &Path, offer: &Offer) -> Result<(), String> {
    let info = inspect(program)?;
    if info.build.release.as_ref() != Some(offer.build_identity())
        || info.platform != offer.artifact().platform
        || info.component != offer.artifact().component
        || info.contract != offer.artifact().contract
    {
        return Err("program identity does not match its signed release".into());
    }
    Ok(())
}
pub fn update_trust() -> Result<khor_update::TrustRoot, String> {
    TrustRoot::bundled()
}

pub(crate) fn recover_current_installation() -> Result<(), String> {
    #[cfg(unix)]
    {
        static CHECKED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
        return CHECKED
            .get_or_init(|| {
                let source = crate::installed_exe()?;
                let Some(installation) =
                    khor_update::CliInstallation::detect(&source, Platform::current()?)?
                else {
                    return Ok(());
                };
                installation.recover(|program, receipt| {
                    let info = inspect(program)?;
                    if info.component != Component::Cli
                        || info.platform != receipt.platform
                        || info.build.release.as_ref() != Some(&receipt.build)
                    {
                        return Err(
                            "activated program disagrees with its installation record".into()
                        );
                    }
                    Ok(())
                })?;
                Ok(())
            })
            .clone();
    }
    #[cfg(not(unix))]
    Ok(())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeRequest {
    pub check_only: bool,
    pub channel: Option<Channel>,
    pub version: Option<ReleaseTag>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeOutcome {
    pub current: ReleaseIdentity,
    pub target: Option<ReleaseIdentity>,
    pub installed: bool,
}

#[cfg(unix)]
pub async fn upgrade_local(
    request: UpgradeRequest,
    trust: TrustRoot,
    progress: impl FnMut(DownloadProgress),
) -> Result<UpgradeOutcome, String> {
    let source = crate::installed_exe()?;
    let platform = Platform::current()?;
    let installation = khor_update::CliInstallation::detect(&source, platform)?.ok_or(
        "this CLI is managed outside Khor; update it with its package manager or installer",
    )?;
    let info = inspect(&source)?;
    if info.component != Component::Cli {
        return Err("this installation is not the CLI".into());
    }
    let current = info
        .build
        .release
        .ok_or("install a signed release with full version identity before using upgrade")?;
    let current = installation.adopted_identity(&current)?;
    let channel = request
        .channel
        .or(installation.receipt()?.map(|receipt| receipt.channel))
        .unwrap_or_else(|| current.tag.channel());
    let feed = Feed::github(trust.clone())?;
    let Some(release) = feed
        .check(&current, channel, request.version.as_ref())
        .await?
    else {
        return Ok(UpgradeOutcome {
            current,
            target: None,
            installed: false,
        });
    };
    let Some(offer) = release.select(
        Component::Cli,
        platform,
        &current,
        channel,
        request.version.as_ref(),
    )?
    else {
        return Ok(UpgradeOutcome {
            current,
            target: None,
            installed: false,
        });
    };
    let target = Some(offer.identity().clone());
    if request.check_only {
        return Ok(UpgradeOutcome {
            current,
            target,
            installed: false,
        });
    }
    let download = Download::fetch(offer, &trust, &installation.staging(), progress).await?;
    installation.install(
        &download,
        channel,
        |program, previous, offer| {
            verify_program(program, offer)?;
            if let Some(previous) = previous {
                let output = runtime_image::probe(
                    program,
                    &[OsStr::new("_runtime_check"), previous.as_os_str()],
                )?;
                if output != b"compatible\n" {
                    return Err("activation compatibility was not confirmed".into());
                }
            }
            Ok(())
        },
        verify_program,
    )?;
    Ok(UpgradeOutcome {
        current,
        target,
        installed: true,
    })
}

#[cfg(not(unix))]
pub async fn upgrade_local(
    _request: UpgradeRequest,
    _trust: TrustRoot,
    _progress: impl FnMut(DownloadProgress),
) -> Result<UpgradeOutcome, String> {
    Err("this CLI uses its external installer for updates".into())
}
