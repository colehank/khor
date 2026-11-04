use crate::{Artifact, Channel, Component, Platform, ReleaseIdentity, ReleaseManifest, ReleaseTag};
use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
        Ok(VerifiedRelease {
            manifest,
            signed: SignedRelease {
                document: std::str::from_utf8(bytes)
                    .map_err(|e| e.to_string())?
                    .to_owned(),
                signature: signature.trim().to_owned(),
            },
        })
    }

    pub fn restore(&self, signed: &SignedRelease) -> Result<VerifiedRelease, String> {
        self.authenticate(signed.document.as_bytes(), &signed.signature)
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

    pub fn verify_asset(&self, path: &Path, asset: &Artifact) -> Result<(), String> {
        let signature = signature_from_base64(&asset.signature)?;
        let mut verifier = self
            .key
            .verify_stream(&signature)
            .map_err(|e| e.to_string())?;
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut digest = Sha256::new();
        let mut count = 0u64;
        let mut bytes = [0; 64 * 1024];
        loop {
            let size = file.read(&mut bytes).map_err(|e| e.to_string())?;
            if size == 0 {
                break;
            }
            count = count
                .checked_add(size as u64)
                .ok_or("release size overflow")?;
            if count > asset.bytes {
                return Err("release asset exceeds its signed size".into());
            }
            verifier.update(&bytes[..size]);
            digest.update(&bytes[..size]);
        }
        if count != asset.bytes || format!("{:x}", digest.finalize()) != asset.sha256.as_str() {
            return Err("release asset disagrees with its signed size or digest".into());
        }
        verifier.finalize().map_err(|e| e.to_string())
    }

    pub fn verify_asset_bytes(&self, bytes: &[u8], asset: &Artifact) -> Result<(), String> {
        if bytes.len() as u64 != asset.bytes
            || format!("{:x}", Sha256::digest(bytes)) != asset.sha256.as_str()
        {
            return Err("release asset disagrees with its signed size or digest".into());
        }
        self.key
            .verify(bytes, &signature_from_base64(&asset.signature)?, false)
            .map_err(|e| e.to_string())
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedRelease {
    document: String,
    signature: String,
}

#[derive(Clone)]
pub struct VerifiedRelease {
    manifest: ReleaseManifest,
    signed: SignedRelease,
}

impl VerifiedRelease {
    pub fn signed(&self) -> &SignedRelease {
        &self.signed
    }
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
pub(crate) fn fixture_app_offer() -> Offer {
    let (_, mut offer) = fixture_offer("http://127.0.0.1/app.tar.gz".into());
    offer.artifact.component = Component::DesktopApp;
    offer.artifact.platform.os = crate::OperatingSystem::Macos;
    offer.artifact.unpacked_bytes = 4096;
    offer
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
    fn persisted_assets_require_the_signed_size_digest_and_signature_together() {
        let trust = TrustRoot::from_tauri_base64(KEY).unwrap();
        let release = trust.authenticate(RELEASE, SIGNATURE).unwrap();
        let asset = &release.manifest().artifacts[0];
        let bytes = include_bytes!("../tests/fixtures/asset.gz");
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cached-package");
        std::fs::write(&path, bytes).unwrap();
        trust.verify_asset(&path, asset).unwrap();
        trust.verify_asset_bytes(bytes, asset).unwrap();
        let mut corrupted = bytes.to_vec();
        corrupted[10] ^= 1;
        std::fs::write(&path, &corrupted).unwrap();
        assert!(trust.verify_asset(&path, asset).is_err());
        assert!(trust.verify_asset_bytes(&corrupted, asset).is_err());
        assert!(
            trust
                .verify_asset_bytes(&bytes[..bytes.len() - 1], asset)
                .is_err()
        );
        let mut changed = asset.clone();
        changed.signature = SIGNATURE.into();
        assert!(trust.verify_asset_bytes(bytes, &changed).is_err());
        changed = asset.clone();
        changed.bytes -= 1;
        assert!(trust.verify_asset(&path, &changed).is_err());
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
    fn restored_staging_reauthenticates_original_signed_bytes_before_reusing_an_offer() {
        let trust = TrustRoot::from_tauri_base64(KEY).unwrap();
        let release = trust.authenticate(RELEASE, SIGNATURE).unwrap();
        let raw = serde_json::to_vec(release.signed()).unwrap();
        let saved: SignedRelease = serde_json::from_slice(&raw).unwrap();
        assert_eq!(
            trust.restore(&saved).unwrap().manifest(),
            release.manifest()
        );
        let mut changed = saved;
        changed.document = changed.document.replace("v0.2.0", "v9.9.9");
        assert!(trust.restore(&changed).is_err());
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
