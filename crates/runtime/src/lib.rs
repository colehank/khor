//! Immutable, node-local execution resources. The store owns files, never sessions.
mod lease;
mod local;
mod store;
#[cfg(test)]
mod tests;

pub use lease::Lease;
pub use local::{
    available as available_space, local_directory, validate as validate_local_directory,
};
pub use store::{Collection, Image, Limits, Source, Store, Usage};

pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

use serde::{Deserialize, Serialize};

/// Enter before opening user stores. Unmanaged executables keep their ordinary installation path.
pub fn enter_current() -> std::io::Result<()> {
    static CURRENT: std::sync::OnceLock<Result<Option<Lease>, String>> = std::sync::OnceLock::new();
    match CURRENT.get_or_init(|| {
        (|| {
            let lease = Store::lease_for_program(&std::env::current_exe()?)?;
            if let Some(lease) = &lease {
                lease.make_inherited_private()?;
            }
            Ok(lease)
        })()
        .map_err(|error: std::io::Error| error.to_string())
    }) {
        Ok(_) => Ok(()),
        Err(error) => Err(std::io::Error::other(error.clone())),
    }
}

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
