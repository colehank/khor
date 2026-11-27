//! Import and persistence for a device's portable network profile.

use std::path::Path;

use khor_catalog::msg;
use serde::{Deserialize, Serialize};

use crate::{NetworkProfile, Node, ProfileRelay, Relay, SignedProfile, chosen_relays};

const LIMIT: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSource {
    pub url: String,
    pub public_key: String,
    pub revision: u64,
    signature: String,
}

impl Node {
    pub fn network_profile_source(&self) -> Option<ProfileSource> {
        self.relay_plan.source.clone()
    }

    pub fn network_profile(&self) -> NetworkProfile {
        let plan = &self.relay_plan;
        let mut policy = self.network_policy();
        if !self.has_explicit_network_policy() {
            policy.peer_relay = self.hop_on();
        }
        NetworkProfile {
            version: 1,
            policy,
            relays: plan
                .mine
                .iter()
                .map(|r| ProfileRelay {
                    url: r.url.clone(),
                    qad: r.qad,
                })
                .collect(),
            preferred: plan.preferred.clone(),
        }
    }

    pub fn import_network_profile(&self, text: &str) -> Result<(), String> {
        let profile = decode(text.as_bytes())?;
        self.store_network_profile(profile, None)
    }

    fn store_network_profile(
        &self,
        profile: NetworkProfile,
        source: Option<ProfileSource>,
    ) -> Result<(), String> {
        profile.validate()?;
        let mut plan = chosen_relays(self.root());
        if plan.revision != self.relay_plan.revision {
            return Err(msg::network_profile_conflict().to_string());
        }
        if let (Some(new), Some(old)) = (&source, &plan.source)
            && new.public_key == old.public_key
        {
            if new.revision < old.revision
                || (new.revision == old.revision && new.signature != old.signature)
            {
                return Err(msg::bad_network_profile_revision().to_string());
            }
            if new == old {
                return Ok(());
            }
        }
        let mut relays = Vec::new();
        for relay in profile.relays {
            let url = khor_net::configuration::relay_url(&relay.url)?.to_string();
            let token = plan
                .mine
                .iter()
                .find(|r| {
                    r.url
                        .parse::<iroh::RelayUrl>()
                        .ok()
                        .is_some_and(|u| u.as_str() == url)
                })
                .and_then(|r| r.token.clone());
            relays.push(Relay {
                url,
                token,
                qad: relay.qad,
                pkarr: None,
            });
        }
        plan.mine = relays;
        // Keep the spelling already used by the UI for this configured URL.
        plan.only_mine = !profile.policy.khor_relays && !profile.policy.n0_relays;
        plan.policy = Some(profile.policy);
        let candidates = plan.relays();
        let public = khor_net::endpoint::public_relay_urls(
            &candidates,
            plan.resolved_policy().n0_relays,
            false,
        );
        plan.preferred = profile.preferred.and_then(|p| {
            let wanted = khor_net::configuration::relay_url(&p).ok()?;
            candidates
                .into_iter()
                .map(|r| r.url)
                .chain(public)
                .find(|r| r.parse::<iroh::RelayUrl>().ok().as_ref() == Some(&wanted))
        });
        plan.source = source;
        plan.save(self.root())
    }
}

/// User-initiated fetch. A pinned source can update only the profile it
/// started with. Local settings are read again after the network await.
pub async fn import_profile_url(root: &Path, raw: &str, key: Option<&str>) -> Result<(), String> {
    let old = chosen_relays(root);
    let invalid_url = || msg::bad_network_profile_source().to_string();
    let url = reqwest::Url::parse(raw).map_err(|_| invalid_url())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid_url());
    }
    let pinned = key
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<iroh::PublicKey>()
                .map_err(|_| msg::bad_network_profile_signature().to_string())
        })
        .transpose()?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| msg::bad_network_profile_download().to_string())?;
    let mut response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|_| msg::bad_network_profile_download().to_string())?;
    if !response.status().is_success() {
        return Err(msg::bad_network_profile_download().to_string());
    }
    let mut body = Vec::new();
    if response
        .content_length()
        .is_some_and(|len| len > LIMIT as u64)
    {
        return Err(msg::large_network_profile().to_string());
    }
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| msg::bad_network_profile_download().to_string())?
    {
        if body.len() + chunk.len() > LIMIT {
            return Err(msg::large_network_profile().to_string());
        }
        body.extend_from_slice(&chunk);
    }
    let (profile, source) = if let Some(pinned) = pinned {
        let signed: SignedProfile = decode(&body)?;
        signed.verify(&pinned)?;
        let source = ProfileSource {
            url: url.to_string(),
            public_key: pinned.to_string(),
            revision: signed.revision,
            signature: signed.signature,
        };
        (signed.profile, Some(source))
    } else {
        (decode(&body)?, None)
    };
    let root = root.to_owned();
    tokio::task::spawn_blocking(move || {
        let node = Node::open_foreground(root)?;
        if node.relay_plan.revision != old.revision {
            return Err(msg::network_profile_conflict().to_string());
        }
        node.store_network_profile(profile, source)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn refresh_profile(root: &Path) -> Result<(), String> {
    let source = chosen_relays(root)
        .source
        .ok_or_else(|| msg::no_network_profile_source().to_string())?;
    import_profile_url(root, &source.url, Some(&source.public_key)).await
}

fn decode<T: serde::de::DeserializeOwned>(text: &[u8]) -> Result<T, String> {
    if text.len() > LIMIT {
        return Err(msg::large_network_profile().to_string());
    }
    serde_json::from_slice(text).map_err(|_| msg::bad_network_configuration().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NetworkPolicy, relay_prefs_path};

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".khor")).unwrap();
        dir
    }

    #[test]
    fn malformed_file_uses_no_public_providers() {
        let dir = fixture();
        let path = relay_prefs_path(dir.path());
        assert!(chosen_relays(dir.path()).resolved_policy().n0_relays);
        std::fs::write(&path, b"{\"version\":1,").unwrap();
        let plan = chosen_relays(dir.path());
        assert!(plan.invalid);
        assert!(plan.relays().is_empty());
        assert_eq!(plan.resolved_policy(), NetworkPolicy::self_hosted());
    }

    #[test]
    fn legacy_file_keeps_own_registry_and_relay_key() {
        let dir = fixture();
        std::fs::write(
            relay_prefs_path(dir.path()),
            br#"{
            "relays": [{"url": "https://relay.test", "qad": 7842,
                        "pkarr": "https://lookup.test/pkarr/", "token": "secret"}],
            "only_mine": true
        }"#,
        )
        .unwrap();
        let plan = chosen_relays(dir.path());
        assert!(!plan.resolved_policy().n0_lookup);
        assert!(plan.resolved_policy().advertised_relays);
        assert_eq!(plan.editable_policy().pkarr, ["https://lookup.test/pkarr"]);
        assert_eq!(plan.relays().len(), 1);
        assert_eq!(plan.mine[0].token.as_deref(), Some("secret"));
    }

    #[test]
    fn portable_profile_keeps_key_only_for_matching_relay_and_rejects_rollback() {
        let dir = fixture();
        let root = dir.path();
        std::fs::write(
            relay_prefs_path(root),
            br#"{"relays": [
            {"url": "https://relay.test", "token": "keep"},
            {"url": "https://removed.test", "token": "dropped"}
        ]}"#,
        )
        .unwrap();
        let node = Node::open(root.to_path_buf()).unwrap();
        let mut profile = node.network_profile();
        profile.policy = NetworkPolicy::self_hosted();
        profile.relays.truncate(1);
        profile.relays[0].url = "https://relay.test/".into();
        let source = ProfileSource {
            url: "https://profile.test/network.json".into(),
            public_key: "test-pin".into(),
            revision: 2,
            signature: "test-signature".into(),
        };
        node.store_network_profile(profile.clone(), Some(source.clone()))
            .unwrap();
        drop(node);

        let node = Node::open(root.to_path_buf()).unwrap();
        assert!(!node.hop_on() && !node.carry_on());
        assert_eq!(node.relay_plan.mine.len(), 1);
        assert_eq!(node.relay_plan.mine[0].token.as_deref(), Some("keep"));
        let exported = serde_json::to_string(&node.network_profile()).unwrap();
        assert!(!exported.contains("keep") && !exported.contains("dropped"));

        let mut rollback = source;
        rollback.revision = 1;
        assert!(node.store_network_profile(profile, Some(rollback)).is_err());
        assert_eq!(chosen_relays(root).source.unwrap().revision, 2);
    }
}
