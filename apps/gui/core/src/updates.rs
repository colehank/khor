use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    Beta,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateSource {
    Direct,
    Store,
    External,
    Development,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum UpdatePhase {
    Idle,
    Checking,
    Available,
    Downloading,
    Verifying,
    Ready,
    Installing,
    AwaitingHealth,
    Complete,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateBlocker {
    UnsavedFiles,
    Attachments,
    UnconfirmedSends,
    DraftStorage,
    NavigationStorage,
    WindowNotReady,
    InputInProgress,
    UserActive,
}

#[derive(Clone, Debug, Serialize, TS)]
pub struct UpdatePreparation {
    pub nonce: String,
    pub automatic: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct UpdatePreferences {
    pub channel: UpdateChannel,
    pub automatic_check: bool,
    pub automatic_download: bool,
    #[serde(default, alias = "install_on_exit")]
    pub install_on_start: bool,
}

impl UpdatePreferences {
    pub fn new(channel: UpdateChannel) -> Self {
        Self {
            channel,
            automatic_check: true,
            automatic_download: true,
            install_on_start: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, TS)]
pub struct AppUpdateStatus {
    pub source: UpdateSource,
    pub phase: UpdatePhase,
    pub current: String,
    pub built_from: Option<String>,
    pub source_commit: Option<String>,
    pub available: Option<String>,
    pub notes: String,
    pub received: u64,
    pub total: Option<u64>,
    pub checked_at: Option<String>,
    pub failure: Option<String>,
    pub blocked: Vec<UpdateBlocker>,
    pub preferences: UpdatePreferences,
    pub companions: Vec<CompanionUpdate>,
    pub restart_required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct CompanionUpdate {
    pub id: String,
    pub path: String,
    pub current: String,
    pub available: Option<String>,
    pub selected: bool,
    pub phase: UpdatePhase,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, Serialize, TS)]
pub struct RuntimeCacheUsage {
    pub retained_bytes: u64,
    pub active_old_bytes: u64,
    pub idle_bytes: u64,
    pub unknown_items: u64,
}

pub fn runtime_cache(clean: bool) -> Result<RuntimeCacheUsage, String> {
    let store = khor_runtime::Store::local().map_err(|e| e.to_string())?;
    if clean {
        store.collect_all().map_err(|e| e.to_string())?;
    }
    let usage = store.usage_all().map_err(|e| e.to_string())?;
    Ok(RuntimeCacheUsage {
        retained_bytes: usage.retained_bytes,
        active_old_bytes: usage.active_old_bytes,
        idle_bytes: usage.idle_bytes,
        unknown_items: usage.unknown_items,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum MachineUpdateSource {
    Cli,
    App,
    External,
    Development,
}

#[derive(Clone, Debug, Serialize, TS)]
pub struct MachineInstallation {
    pub source: MachineUpdateSource,
    pub running: String,
    pub selected: Option<String>,
    pub channel: Option<UpdateChannel>,
    pub service_ready: bool,
    pub failed: Option<String>,
}

#[derive(Clone, Debug, Serialize, TS)]
pub struct MachineUpdateResult {
    pub current: String,
    pub available: Option<String>,
    pub enabled: bool,
}

pub async fn machine_installation(
    root: &std::path::Path,
    machine: &str,
) -> Result<MachineInstallation, String> {
    use khor_node::updates::InstallationSource;
    let info = super::open_async(root)
        .await?
        .installation_on(machine)
        .await?;
    Ok(MachineInstallation {
        source: match info.source {
            InstallationSource::Cli => MachineUpdateSource::Cli,
            InstallationSource::App => MachineUpdateSource::App,
            InstallationSource::External => MachineUpdateSource::External,
            InstallationSource::Development => MachineUpdateSource::Development,
        },
        running: info
            .running
            .build
            .release
            .map(|r| r.tag.as_str().to_owned())
            .unwrap_or(info.running.build.version),
        selected: info.current.map(|r| r.tag.as_str().to_owned()),
        channel: info.channel.map(|channel| match channel {
            khor_node::updates::Channel::Stable => UpdateChannel::Stable,
            khor_node::updates::Channel::Beta => UpdateChannel::Beta,
        }),
        service_ready: info.service_ready,
        failed: info.failed.map(|r| r.tag.as_str().to_owned()),
    })
}

pub async fn machine_update(
    root: &std::path::Path,
    machine: &str,
    check_only: bool,
    channel: Option<UpdateChannel>,
) -> Result<MachineUpdateResult, String> {
    let outcome = super::open_async(root)
        .await?
        .upgrade_on(
            machine,
            khor_node::updates::UpgradeRequest {
                check_only,
                channel: channel.map(|channel| match channel {
                    UpdateChannel::Stable => khor_node::updates::Channel::Stable,
                    UpdateChannel::Beta => khor_node::updates::Channel::Beta,
                }),
                version: None,
            },
        )
        .await?;
    Ok(MachineUpdateResult {
        current: outcome.current.tag.as_str().to_owned(),
        available: outcome.target.map(|r| r.tag.as_str().to_owned()),
        enabled: outcome.installed,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum MachineUpdatePhase {
    Queued,
    Waiting,
    Checking,
    Installing,
    Confirming,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct MachineUpdateJob {
    pub machine: String,
    pub request: String,
    #[serde(default)]
    pub source: Option<MachineUpdateSource>,
    pub phase: MachineUpdatePhase,
    pub channel: Option<UpdateChannel>,
    pub target: Option<String>,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, Serialize, TS)]
pub struct MachineUpdateQueue {
    pub revision: u32,
    pub jobs: Vec<MachineUpdateJob>,
}
