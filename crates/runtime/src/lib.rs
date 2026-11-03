//! Immutable, node-local execution resources. The store owns files, never sessions.
mod lease;
mod local;
mod store;
#[cfg(test)]
mod tests;

pub use lease::Lease;
pub use local::local_directory;
pub use store::{Collection, Image, Limits, Source, Store};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Cli,
    DesktopHost,
    Agent,
}

impl Role {
    fn executable(self) -> &'static str {
        match (self, cfg!(windows)) {
            (Self::Cli, false) => "khor",
            (Self::Cli, true) => "khor.exe",
            (Self::DesktopHost, false) => "khor-gui",
            (Self::DesktopHost, true) => "khor-gui.exe",
            (Self::Agent, false) => "khor-agent",
            (Self::Agent, true) => "khor-agent.exe",
        }
    }
}

/// Independent compatibility identifiers; application SemVer is not a protocol.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub terminal: u32,
    pub gui: String,
    pub node: String,
    pub store: u32,
    pub journal: u8,
    pub agent: u32,
}

impl Contract {
    pub fn accepts(&self, other: &Self) -> bool {
        self == other
    }
}
