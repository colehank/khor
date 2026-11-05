use super::{inspect, install_cli, recover_cli};
use khor_update::{
    Channel, CliInstallation, Component, Download, Platform, ReleaseIdentity, TrustRoot,
    VerifiedRelease,
};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ManagedCli {
    pub source: PathBuf,
    pub launcher: PathBuf,
    pub current: ReleaseIdentity,
    pub failed: Option<ReleaseIdentity>,
}

pub fn managed_clis() -> Result<Vec<ManagedCli>, String> {
    let store = khor_runtime::Store::local().map_err(|e| e.to_string())?;
    let mut found = Vec::new();
    for child in store.cli_stores().map_err(|e| e.to_string())? {
        let source = child
            .directory()
            .parent()
            .ok_or("CLI store has no installation")?
            .join("current");
        if !source.try_exists().map_err(|e| e.to_string())? {
            continue;
        }
        let Some(installation) = CliInstallation::detect(&source, Platform::current()?)? else {
            continue;
        };
        recover_cli(&installation)?;
        if !installation.activation_ready()? {
            return Err(
                "a registered CLI update is still active; check again after it finishes".into(),
            );
        }
        found.push(describe(&installation)?);
        if found.len() > 64 {
            return Err("too many local CLI installations to review together".into());
        }
    }
    found.sort_by(|a, b| a.launcher.cmp(&b.launcher));
    Ok(found)
}

fn describe(installation: &CliInstallation) -> Result<ManagedCli, String> {
    let source = installation.source();
    let info = inspect(&source)?;
    if info.component != Component::Cli || info.platform != Platform::current()? {
        return Err("registered CLI has a different component or platform".into());
    }
    let compiled = info
        .build
        .release
        .ok_or("registered CLI has no release identity")?;
    let current = installation.adopted_identity(&compiled)?;
    let failed = installation
        .receipt()?
        .filter(|r| r.state == khor_update::InstallState::RolledBack)
        .map(|r| r.identity);
    Ok(ManagedCli {
        source,
        launcher: installation.launcher(),
        current,
        failed,
    })
}

pub async fn upgrade_managed_cli(
    source: &Path,
    expected: &ReleaseIdentity,
    release: &VerifiedRelease,
    channel: Channel,
    trust: &TrustRoot,
    archive: &Path,
) -> Result<ManagedCli, String> {
    let source = source.to_path_buf();
    let lookup = source.clone();
    let (installation, current) = tokio::task::spawn_blocking(move || {
        let owned = managed_clis()?
            .into_iter()
            .find(|cli| cli.source == lookup)
            .ok_or("this CLI is not registered to this machine")?;
        let installation = CliInstallation::detect(&lookup, Platform::current()?)?
            .ok_or("this CLI is no longer managed by Khor")?;
        recover_cli(&installation)?;
        let current = describe(&installation)?;
        if current.launcher != owned.launcher {
            return Err("CLI installation owner changed".into());
        }
        Ok::<_, String>((installation, current))
    })
    .await
    .map_err(|e| e.to_string())??;
    if current.current != *expected {
        return Err("CLI version changed; check updates again".into());
    }
    let Some(offer) = release.select(
        Component::Cli,
        Platform::current()?,
        &current.current,
        channel,
        None,
    )?
    else {
        return Ok(current);
    };
    let expected = expected.clone();
    let trust = trust.clone();
    let archive = archive.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let download = Download::import(offer, &trust, &archive, &installation.staging())?;
        install_cli(&installation, &download, channel, Some(&expected))?;
        describe(&installation)
    })
    .await
    .map_err(|e| e.to_string())?
}
