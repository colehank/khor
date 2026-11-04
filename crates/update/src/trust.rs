use crate::{Artifact, Channel, Component, Platform, ReleaseIdentity, ReleaseManifest, ReleaseTag};
use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use std::{fs::File, io::Read, path::Path};

const MANIFEST_LIMIT: usize = 256 * 1024;
const SIGNATURE_LIMIT: usize = 4096;

#[derive(Clone)]
pub struct TrustRoot {
    key: PublicKey,
    tauri_key: String,
}

impl TrustRoot {
    pub fn bundled() -> Result<Self, String> {
        let key = include_str!("../updater.pub").trim();
        if key.is_empty() {
            return Err("signed updates are not configured in this development build".into());
        }
        Self::from_tauri_base64(key)
    }
    pub fn from_tauri_base64(encoded: &str) -> Result<Self, String> {
        let bytes = STANDARD.decode(encoded.trim()).map_err(|e| e.to_string())?;
        let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
        let key = PublicKey::decode(text).map_err(|e| e.to_string())?;
        Ok(Self {
            key,
            tauri_key: encoded.trim().into(),
        })
    }

    pub fn tauri_key(&self) -> &str {
        &self.tauri_key
    }

    pub fn authenticate(&self, bytes: &[u8], signature: &str) -> Result<VerifiedRelease, String> {
        if bytes.len() > MANIFEST_LIMIT {
            return Err("release manifest exceeds its limit".into());
        }
        self.key
            .verify(bytes, &signature_from_base64(signature)?, false)
            .map_err(|e| e.to_string())?;
        let manifest: ReleaseManifest = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        manifest.validate()?;
        Ok(VerifiedRelease { manifest })
    }

    pub fn verify_file(&self, path: &Path, signature: &str) -> Result<(), String> {
        let signature = signature_from_base64(signature)?;
        let mut verifier = self
            .key
            .verify_stream(&signature)
            .map_err(|e| e.to_string())?;
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut buffer = [0; 64 * 1024];
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            verifier.update(&buffer[..count]);
        }
        verifier.finalize().map_err(|e| e.to_string())
    }
}

fn signature_from_base64(encoded: &str) -> Result<Signature, String> {
    if encoded.len() > SIGNATURE_LIMIT {
        return Err("release signature exceeds its limit".into());
    }
    let bytes = STANDARD.decode(encoded.trim()).map_err(|e| e.to_string())?;
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    Signature::decode(text).map_err(|e| e.to_string())
}

pub struct VerifiedRelease {
    manifest: ReleaseManifest,
}

impl VerifiedRelease {
    pub fn manifest(&self) -> &ReleaseManifest {
        &self.manifest
    }

    pub fn select(
        &self,
        component: Component,
        platform: Platform,
        current: &ReleaseIdentity,
        channel: Channel,
        exact: Option<&ReleaseTag>,
    ) -> Result<Option<Offer>, String> {
        let available = self.manifest.newer_than(current, channel)?;
        if let Some(exact) = exact {
            if exact != &self.manifest.identity.tag {
                return Err("release does not match the requested tag".into());
            }
        } else if !available {
            return Ok(None);
        }
        let artifact = self
            .manifest
            .artifact(component, platform)
            .ok_or("release has no asset for this component and platform")?;
        Ok(Some(Offer {
            identity: self.manifest.identity.clone(),
            build: self.manifest.build_identity(),
            artifact: artifact.clone(),
        }))
    }
}

#[derive(Clone)]
pub struct Offer {
    identity: ReleaseIdentity,
    build: ReleaseIdentity,
    artifact: Artifact,
}
impl Offer {
    pub fn build_identity(&self) -> &ReleaseIdentity {
        &self.build
    }
    pub fn identity(&self) -> &ReleaseIdentity {
        &self.identity
    }
    pub fn artifact(&self) -> &Artifact {
        &self.artifact
    }
}

#[cfg(test)]
pub(crate) fn fixture_offer(url: String) -> (TrustRoot, Offer) {
    let trust =
        TrustRoot::from_tauri_base64(include_str!("../tests/fixtures/public-key.txt")).unwrap();
    let mut release = trust
        .authenticate(
            include_bytes!("../tests/fixtures/release.json"),
            include_str!("../tests/fixtures/release.sig"),
        )
        .unwrap();
    release.manifest.artifacts[0].url = url;
    release.manifest.artifacts[0].platform = Platform::current().unwrap();
    let mut current = release.manifest.identity.clone();
    current.tag = "v0.1.0".to_owned().try_into().unwrap();
    let offer = release
        .select(
            Component::Cli,
            release.manifest.artifacts[0].platform,
            &current,
            Channel::Stable,
            None,
        )
        .unwrap()
        .unwrap();
    (trust, offer)
}

#[cfg(test)]
mod tests {
    use super::*;
    const KEY: &str = include_str!("../tests/fixtures/public-key.txt");
    const RELEASE: &[u8] = include_bytes!("../tests/fixtures/release.json");
    const SIGNATURE: &str = include_str!("../tests/fixtures/release.sig");

    #[test]
    fn official_tauri_signatures_and_bundled_root_are_compatible_with_stream_verification() {
        let trust = TrustRoot::bundled().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("root-challenge.txt");
        let bytes = include_bytes!("../tests/fixtures/root-challenge.txt");
        let signature = include_str!("../tests/fixtures/root-challenge.txt.sig");
        std::fs::write(&path, bytes).unwrap();
        trust.verify_file(&path, signature).unwrap();
        assert!(
            trust.authenticate(bytes, signature).is_err(),
            "a signing challenge is never a release manifest"
        );
        assert_ne!(trust.tauri_key(), KEY.trim());
    }
    #[test]
    fn only_authenticated_bytes_can_become_an_update_offer() {
        let trust = TrustRoot::from_tauri_base64(KEY).unwrap();
        let release = trust.authenticate(RELEASE, SIGNATURE).unwrap();
        let mut current = release.manifest().identity.clone();
        current.tag = "v0.1.0".to_owned().try_into().unwrap();
        let platform = release.manifest().artifacts[0].platform;
        assert!(
            release
                .select(Component::Cli, platform, &current, Channel::Stable, None)
                .unwrap()
                .is_some()
        );
        let mut changed = RELEASE.to_vec();
        changed[10] ^= 1;
        assert!(trust.authenticate(&changed, SIGNATURE).is_err());
        assert!(trust.authenticate(RELEASE, KEY).is_err());
        assert!(
            trust
                .authenticate(&vec![b' '; MANIFEST_LIMIT + 1], SIGNATURE)
                .is_err()
        );
        let wrong: ReleaseTag = "v9.0.0".to_owned().try_into().unwrap();
        assert!(
            release
                .select(
                    Component::Cli,
                    platform,
                    &current,
                    Channel::Stable,
                    Some(&wrong)
                )
                .is_err()
        );
    }

    #[test]
    fn large_asset_signatures_are_verified_as_streams() {
        let trust = TrustRoot::from_tauri_base64(KEY).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("asset.gz");
        std::fs::write(&file, include_bytes!("../tests/fixtures/asset.gz")).unwrap();
        let signature = include_str!("../tests/fixtures/asset.sig");
        trust.verify_file(&file, signature).unwrap();
        std::fs::write(&file, b"corrupt").unwrap();
        assert!(trust.verify_file(&file, signature).is_err());
    }
}
