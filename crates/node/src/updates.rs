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

pub fn verify_activation(program: &Path, current: &Path) -> Result<(), String> {
    let reply = runtime_image::probe(
        program,
        &[OsStr::new("_runtime_check"), current.as_os_str()],
    )?;
    if reply != b"compatible\n" {
        return Err("activation compatibility was not confirmed".into());
    }
    Ok(())
}
pub fn update_trust() -> Result<khor_update::TrustRoot, String> {
    TrustRoot::bundled()
}

pub(crate) fn selected_cli_source(source: &Path) -> Result<std::path::PathBuf, String> {
    #[cfg(unix)]
    if let Some(installation) = khor_update::CliInstallation::detect(source, Platform::current()?)?
    {
        return installation
            .selected_on_node(&khor_runtime::local_directory().map_err(|e| e.to_string())?);
    }
    Ok(source.into())
}

pub(crate) fn cli_activation_ready(source: &Path) -> Result<bool, String> {
    #[cfg(unix)]
    if let Some(installation) = khor_update::CliInstallation::detect(source, Platform::current()?)?
    {
        return installation.activation_ready();
    }
    Ok(true)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallationSource {
    Cli,
    App,
    External,
    Development,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationInfo {
    pub source: InstallationSource,
    pub running: ProgramInfo,
    pub selected: Option<ProgramInfo>,
    pub current: Option<ReleaseIdentity>,
    pub channel: Option<Channel>,
    pub service_ready: bool,
    pub failed: Option<ReleaseIdentity>,
}

pub async fn installation_info() -> Result<InstallationInfo, String> {
    tokio::task::spawn_blocking(describe_installation)
        .await
        .map_err(|e| e.to_string())?
}

fn describe_installation() -> Result<InstallationInfo, String> {
    let running = program_info()?;
    let source = if running.build.release.is_none() {
        InstallationSource::Development
    } else if running.component == Component::DesktopApp {
        InstallationSource::App
    } else {
        InstallationSource::External
    };
    #[cfg(unix)]
    if source == InstallationSource::External
        && khor_update::CliInstallation::detect(&crate::installed_exe()?, Platform::current()?)?
            .is_some()
    {
        let (installation, selected, current, channel) = cli_installation()?;
        let receipt = installation.receipt()?;
        return Ok(InstallationInfo {
            source: InstallationSource::Cli,
            running,
            selected: Some(selected),
            current: Some(current),
            channel: Some(channel),
            service_ready: receipt
                .as_ref()
                .is_some_and(|r| r.service_ready && r.state == khor_update::InstallState::Complete),
            failed: receipt
                .filter(|r| r.state == khor_update::InstallState::RolledBack)
                .map(|r| r.identity),
        });
    }
    Ok(InstallationInfo {
        current: running.build.release.clone(),
        channel: running.build.release.as_ref().map(|r| r.tag.channel()),
        source,
        running,
        selected: None,
        service_ready: true,
        failed: None,
    })
}

#[cfg(unix)]
pub(crate) fn cli_installation() -> Result<
    (
        khor_update::CliInstallation,
        ProgramInfo,
        ReleaseIdentity,
        Channel,
    ),
    String,
> {
    let source = crate::installed_exe()?;
    let platform = Platform::current()?;
    let installation = khor_update::CliInstallation::detect(&source, platform)?.ok_or(
        "this CLI is managed outside Khor; update it with its package manager or installer",
    )?;
    let installation = installation.on_node(
        &khor_runtime::local_directory().map_err(|e| e.to_string())?,
        || {
            let source = runtime_image::host_at(&source)?;
            khor_runtime::Store::image_for_program(source.path())
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "CLI bootstrap did not produce an owned runtime".into())
        },
    )?;
    let source = installation.source();
    let info = inspect(&source)?;
    if info.component != Component::Cli {
        return Err("this installation is not the CLI".into());
    }
    let current = info
        .build
        .release
        .clone()
        .ok_or("install a signed release with full version identity before using upgrade")?;
    let current = installation.adopted_identity(&current)?;
    let channel = installation
        .receipt()?
        .map(|receipt| receipt.channel)
        .unwrap_or_else(|| current.tag.channel());
    Ok((installation, info, current, channel))
}

#[cfg(unix)]
pub(crate) fn install_cli(
    installation: &khor_update::CliInstallation,
    download: &Download,
    channel: Channel,
    expected: Option<&ReleaseIdentity>,
) -> Result<khor_update::InstallReceipt, String> {
    installation.install(
        download,
        channel,
        |program, previous, offer| {
            verify_program(program, offer)?;
            if let Some(expected) = expected {
                let previous = previous.ok_or("CLI installation changed while downloading")?;
                let actual = inspect(previous)?
                    .build
                    .release
                    .ok_or("existing CLI has no release identity")?;
                if &installation.adopted_identity(&actual)? != expected {
                    return Err("CLI version changed while downloading; check again".into());
                }
            }
            if let Some(previous) = previous {
                verify_activation(program, previous)?;
            }
            Ok(())
        },
        verify_program,
    )
}

#[cfg(unix)]
pub async fn upgrade_local(
    request: UpgradeRequest,
    trust: TrustRoot,
    progress: impl FnMut(DownloadProgress),
) -> Result<UpgradeOutcome, String> {
    let (installation, info, current, default_channel) = cli_installation()?;
    let platform = info.platform;
    let channel = request.channel.unwrap_or(default_channel);
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
    install_cli(&installation, &download, channel, Some(&current))?;
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
