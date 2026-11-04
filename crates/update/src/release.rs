use khor_runtime::Contract;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Beta,
}
impl Channel {
    pub fn accepts(self, tag: &ReleaseTag) -> bool {
        self == Self::Beta || tag.version().pre.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ReleaseTag(String);
impl ReleaseTag {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn version(&self) -> Version {
        Version::parse(&self.0[1..]).expect("validated release tag")
    }
    pub fn channel(&self) -> Channel {
        if self.version().pre.is_empty() {
            Channel::Stable
        } else {
            Channel::Beta
        }
    }
}
impl TryFrom<String> for ReleaseTag {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let raw = value
            .strip_prefix('v')
            .ok_or("release tags must start with v")?;
        let version = Version::parse(raw).map_err(|error| error.to_string())?;
        if !version.build.is_empty() {
            return Err("release tags cannot contain build metadata".into());
        }
        if !version.pre.is_empty() {
            let candidate = version
                .pre
                .as_str()
                .strip_prefix("rc.")
                .ok_or("unsupported release channel")?;
            if candidate.parse::<u64>().is_err()
                || candidate.is_empty()
                || !candidate.bytes().all(|c| c.is_ascii_digit())
            {
                return Err("release candidates require an rc number".into());
            }
        }
        Ok(Self(value))
    }
}
impl From<ReleaseTag> for String {
    fn from(value: ReleaseTag) -> Self {
        value.0
    }
}

fn hex(value: &str, length: usize) -> Result<(), String> {
    if value.len() == length
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        Ok(())
    } else {
        Err(format!("expected {length} lowercase hexadecimal digits"))
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SourceCommit(String);
impl SourceCommit {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for SourceCommit {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        hex(&value, 40)?;
        Ok(Self(value))
    }
}
impl From<SourceCommit> for String {
    fn from(value: SourceCommit) -> Self {
        value.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Digest(String);
impl Digest {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Digest {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        hex(&value, 64)?;
        Ok(Self(value))
    }
}
impl From<Digest> for String {
    fn from(value: Digest) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatingSystem {
    Linux,
    Macos,
    Windows,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    X86_64,
    Aarch64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub os: OperatingSystem,
    pub arch: Architecture,
}
impl Platform {
    pub fn current() -> Result<Self, String> {
        let os = match std::env::consts::OS {
            "linux" => OperatingSystem::Linux,
            "macos" => OperatingSystem::Macos,
            "windows" => OperatingSystem::Windows,
            _ => return Err("this platform uses its application store for updates".into()),
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => Architecture::X86_64,
            "aarch64" => Architecture::Aarch64,
            _ => return Err("this CPU has no published update".into()),
        };
        Ok(Self { os, arch })
    }
    pub fn runtime_name(self) -> String {
        let os = match self.os {
            OperatingSystem::Linux => "linux",
            OperatingSystem::Macos => "macos",
            OperatingSystem::Windows => "windows",
        };
        let arch = match self.arch {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
        };
        format!("{os}-{arch}")
    }
    pub fn updater_target(self) -> String {
        let os = match self.os {
            OperatingSystem::Linux => "linux",
            OperatingSystem::Macos => "darwin",
            OperatingSystem::Windows => "windows",
        };
        let arch = match self.arch {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
        };
        format!("{os}-{arch}")
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Component {
    Cli,
    DesktopApp,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseIdentity {
    pub tag: ReleaseTag,
    pub source: SourceCommit,
    pub build: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub component: Component,
    pub platform: Platform,
    pub file: String,
    pub url: String,
    pub bytes: u64,
    pub unpacked_bytes: u64,
    pub sha256: Digest,
    pub signature: String,
    pub contract: Contract,
}
impl Artifact {
    pub fn required_space(&self) -> Result<u64, String> {
        self.unpacked_bytes
            .checked_mul(2)
            .and_then(|size| size.checked_add(self.bytes))
            .and_then(|size| size.checked_add(128 * 1024 * 1024))
            .ok_or_else(|| "release size overflow".into())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub schema: u8,
    pub identity: ReleaseIdentity,
    pub notes: String,
    pub artifacts: Vec<Artifact>,
}
impl ReleaseManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 || self.identity.build == 0 {
            return Err("unsupported release identity".into());
        }
        if self.notes.len() > 64 * 1024 || self.artifacts.is_empty() || self.artifacts.len() > 32 {
            return Err("invalid release manifest size".into());
        }
        let mut targets = BTreeSet::new();
        for artifact in &self.artifacts {
            if !targets.insert((artifact.component, artifact.platform)) {
                return Err("release contains a duplicated component target".into());
            }
            if artifact.file.is_empty()
                || artifact.file.len() > 200
                || artifact.file == "."
                || artifact.file == ".."
                || !artifact
                    .file
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
            {
                return Err("release asset is not a file name".into());
            }
            let url = reqwest::Url::parse(&artifact.url).map_err(|error| error.to_string())?;
            if !matches!(url.scheme(), "https" | "http")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err("release asset requires an HTTP URL without credentials".into());
            }
            if artifact.bytes == 0
                || artifact.unpacked_bytes == 0
                || artifact.signature.is_empty()
                || artifact.signature.len() > 4096
            {
                return Err("release asset is incomplete".into());
            }
            artifact.required_space()?;
        }
        Ok(())
    }
    pub fn artifact(&self, component: Component, platform: Platform) -> Option<&Artifact> {
        self.artifacts
            .iter()
            .find(|artifact| artifact.component == component && artifact.platform == platform)
    }
    pub fn newer_than(&self, current: &ReleaseIdentity, channel: Channel) -> Result<bool, String> {
        if self.identity.tag == current.tag && self.identity != *current {
            return Err("a published release tag changed source".into());
        }
        Ok(channel.accepts(&self.identity.tag)
            && self.identity.tag.version() > current.tag.version())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tag(value: &str) -> ReleaseTag {
        value.to_owned().try_into().unwrap()
    }
    fn identity(value: &str) -> ReleaseIdentity {
        ReleaseIdentity {
            tag: tag(value),
            source: "a".repeat(40).try_into().unwrap(),
            build: 1,
        }
    }
    fn manifest(value: &str) -> ReleaseManifest {
        ReleaseManifest {
            schema: 1,
            identity: identity(value),
            notes: "Release notes".into(),
            artifacts: vec![Artifact {
                component: Component::Cli,
                platform: Platform {
                    os: OperatingSystem::Macos,
                    arch: Architecture::Aarch64,
                },
                file: "khor-macos-arm64.gz".into(),
                url: "https://example.test/khor.gz".into(),
                bytes: 12,
                unpacked_bytes: 20,
                sha256: "b".repeat(64).try_into().unwrap(),
                signature: "fixture signature".into(),
                contract: Contract {
                    terminal: 5,
                    gui: "gui/2".into(),
                    node: "wire/1".into(),
                    store: 1,
                    journal: 1,
                    agent: 1,
                },
            }],
        }
    }
    #[test]
    fn candidates_and_channels_are_compared_as_semver() {
        assert!(
            manifest("v0.2.0-rc.10")
                .newer_than(&identity("v0.2.0-rc.2"), Channel::Beta)
                .unwrap()
        );
        assert!(
            !manifest("v0.2.0-rc.10")
                .newer_than(&identity("v0.1.0"), Channel::Stable)
                .unwrap()
        );
        assert!(
            manifest("v0.2.0")
                .newer_than(&identity("v0.2.0-rc.10"), Channel::Beta)
                .unwrap()
        );
        assert!(
            !manifest("v0.2.0")
                .newer_than(&identity("v0.3.0-rc.1"), Channel::Stable)
                .unwrap()
        );
        for invalid in [
            "0.2.0",
            "v0.2",
            "v0.2.0-beta.1",
            "v0.2.0-rc.foo",
            "v0.2.0-rc.01",
            "v0.2.0+build.1",
            "../v0.2.0",
        ] {
            assert!(
                ReleaseTag::try_from(invalid.to_owned()).is_err(),
                "{invalid}"
            );
        }
    }
    #[test]
    fn full_identity_is_not_the_apple_numeric_version() {
        let a = identity("v0.2.0-rc.1");
        let b = identity("v0.2.0-rc.2");
        assert_ne!(a, b);
        let mut changed = manifest("v0.2.0-rc.1");
        changed.identity.source = "c".repeat(40).try_into().unwrap();
        assert!(changed.newer_than(&a, Channel::Beta).is_err());
        assert_eq!(
            serde_json::from_str::<ReleaseIdentity>(&serde_json::to_string(&a).unwrap()).unwrap(),
            a
        );
    }
    #[test]
    fn a_manifest_has_one_selection_per_component_and_platform() {
        let release = manifest("v0.2.0");
        release.validate().unwrap();
        let mut duplicate = release.clone();
        duplicate.artifacts.push(release.artifacts[0].clone());
        assert!(duplicate.validate().is_err());
        for name in ["../khor", "/khor", "a\\khor", ".", ".."] {
            let mut bad = release.clone();
            bad.artifacts[0].file = name.into();
            assert!(bad.validate().is_err());
        }
        for url in ["file:///tmp/khor", "https://user:secret@example.test/khor"] {
            let mut bad = release.clone();
            bad.artifacts[0].url = url.into();
            assert!(bad.validate().is_err());
        }
        let mut lan = release.clone();
        lan.artifacts[0].url = "http://192.0.2.1/khor.gz".into();
        lan.validate().unwrap();
        assert!(
            release
                .artifact(
                    Component::Cli,
                    Platform {
                        os: OperatingSystem::Macos,
                        arch: Architecture::X86_64
                    }
                )
                .is_none()
        );
    }
    #[test]
    fn source_and_digest_fields_never_accept_abbreviations_or_path_text() {
        for value in ["abc", "../release", &"A".repeat(40)] {
            assert!(SourceCommit::try_from(value.to_owned()).is_err());
        }
        assert!(Digest::try_from("a".repeat(63)).is_err());
        let platform = Platform {
            os: OperatingSystem::Macos,
            arch: Architecture::Aarch64,
        };
        assert_eq!(platform.runtime_name(), "macos-aarch64");
        assert_eq!(platform.updater_target(), "darwin-aarch64");
    }
    #[test]
    fn capacity_budget_includes_archive_unpack_publication_and_reserve_and_cannot_overflow() {
        let mut release = manifest("v0.2.0");
        let asset = &mut release.artifacts[0];
        assert_eq!(asset.required_space().unwrap(), 12 + 40 + 128 * 1024 * 1024);
        asset.unpacked_bytes = u64::MAX;
        assert!(release.validate().is_err());
    }
}
