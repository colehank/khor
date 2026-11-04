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
}
