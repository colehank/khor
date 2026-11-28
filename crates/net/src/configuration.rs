//! User policy for the services an iroh endpoint can use.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkPolicy {
    pub khor_relays: bool,
    pub n0_relays: bool,
    pub khor_lookup: bool,
    pub n0_lookup: bool,
    pub n0_publish: bool,
    pub mdns: bool,
    pub peer_relay: bool,
    pub advertised_relays: bool,
    pub publish_direct: bool,
    pub public_dns_fallback: bool,
    pub pkarr: Vec<String>,
    pub dns_origins: Vec<String>,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            khor_relays: true,
            n0_relays: true,
            khor_lookup: true,
            n0_lookup: true,
            n0_publish: true,
            mdns: true,
            peer_relay: true,
            advertised_relays: true,
            publish_direct: false,
            public_dns_fallback: true,
            pkarr: Vec::new(),
            dns_origins: Vec::new(),
        }
    }
}

impl NetworkPolicy {
    pub fn self_hosted() -> Self {
        Self {
            khor_relays: false,
            n0_relays: false,
            khor_lookup: false,
            n0_lookup: false,
            n0_publish: false,
            peer_relay: false,
            advertised_relays: false,
            public_dns_fallback: false,
            ..Self::default()
        }
    }

    /// A legacy configuration may already publish direct addresses to its
    /// configured Pkarr registries and accept a peer's advertised relay.
    pub fn legacy(n0: bool) -> Self {
        Self {
            khor_lookup: false,
            n0_relays: n0,
            n0_lookup: n0,
            n0_publish: n0,
            publish_direct: true,
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        for registry in &self.pkarr {
            let url = url::Url::parse(registry)
                .map_err(|_| khor_catalog::msg::bad_network_registry(registry))?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(khor_catalog::msg::bad_network_registry(registry));
            }
        }
        for origin in &self.dns_origins {
            if origin.is_empty()
                || origin.trim_end_matches('.').split('.').any(|part| {
                    part.is_empty()
                        || part.len() > 63
                        || part.starts_with('-')
                        || part.ends_with('-')
                        || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                })
            {
                return Err(khor_catalog::msg::bad_network_dns_origin(origin));
            }
        }
        Ok(())
    }
}

/// Portable configuration. The relay's access token lives on each device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub version: u32,
    pub policy: NetworkPolicy,
    pub relays: Vec<ProfileRelay>,
    #[serde(default)]
    pub preferred: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRelay {
    pub url: String,
    #[serde(default)]
    pub qad: Option<u16>,
}

impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        use khor_catalog::msg;
        if self.version != 1 {
            return Err(msg::bad_network_configuration().to_string());
        }
        self.policy.validate()?;
        let mut urls = Vec::new();
        for relay in &self.relays {
            let url = relay_url(&relay.url)?;
            if relay.qad == Some(0) || urls.contains(&url) {
                return Err(msg::not_a_relay_url(&relay.url));
            }
            urls.push(url);
        }
        if let Some(preferred) = &self.preferred
            && !self.has_relay(&urls, &relay_url(preferred)?)
        {
            return Err(msg::preferred_not_a_relay(preferred));
        }
        Ok(())
    }

    fn has_relay(&self, custom: &[iroh::RelayUrl], url: &iroh::RelayUrl) -> bool {
        custom.contains(url)
            || self.policy.khor_relays
                && crate::endpoint::relay_map(&crate::endpoint::khor_tier())
                    .get(url)
                    .is_some()
            || self.policy.n0_relays && iroh::defaults::prod::default_relay_map().get(url).is_some()
    }
}

pub fn relay_url(raw: &str) -> Result<iroh::RelayUrl, String> {
    let invalid = || khor_catalog::msg::not_a_relay_url(raw);
    // Validate URL separately since RelayUrl may accept additional transports.
    let url = url::Url::parse(raw).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(khor_catalog::msg::bad_network_configuration().to_string());
    }
    if crate::endpoint::is_retired_relay(&url) {
        return Err(khor_catalog::msg::retired_relay_url(raw));
    }
    raw.parse().map_err(|_| invalid())
}

/// The operator's signed profile wire format. The signed bytes include a
/// domain separator and cover both the revision and the complete profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedProfile {
    pub revision: u64,
    pub profile: Profile,
    pub signature: String,
}

impl SignedProfile {
    pub fn sign(revision: u64, profile: Profile, key: &iroh::SecretKey) -> Result<Self, String> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        profile.validate()?;
        let bytes = Self::signing_bytes(revision, &profile)?;
        Ok(Self {
            revision,
            profile,
            signature: STANDARD.encode(key.sign(&bytes).to_bytes()),
        })
    }

    pub fn verify(&self, key: &iroh::PublicKey) -> Result<(), String> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        let invalid = || khor_catalog::msg::bad_network_profile_signature().to_string();
        let signature: [u8; 64] = STANDARD
            .decode(&self.signature)
            .map_err(|_| invalid())?
            .try_into()
            .map_err(|_| invalid())?;
        key.verify(
            &Self::signing_bytes(self.revision, &self.profile)?,
            &iroh::Signature::from_bytes(&signature),
        )
        .map_err(|_| invalid())?;
        self.profile.validate()
    }

    fn signing_bytes(revision: u64, profile: &Profile) -> Result<Vec<u8>, String> {
        if revision == 0 {
            return Err(khor_catalog::msg::bad_network_profile_revision().to_string());
        }
        let mut bytes = b"khor network profile v1\0".to_vec();
        bytes.extend(revision.to_be_bytes());
        bytes.extend(serde_json::to_vec(profile).map_err(|error| error.to_string())?);
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile {
            version: 1,
            policy: NetworkPolicy::self_hosted(),
            relays: vec![ProfileRelay {
                url: "https://relay.example.org:3341".into(),
                qad: Some(7842),
            }],
            preferred: None,
        }
    }

    #[test]
    fn signatures_cover_peer_routes_and_revision() {
        let key = iroh::SecretKey::generate();
        let original = SignedProfile::sign(2, profile(), &key).unwrap();
        let decoded: SignedProfile =
            serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
        decoded.verify(&key.public()).unwrap();
        assert!(
            decoded
                .verify(&iroh::SecretKey::generate().public())
                .is_err()
        );
        let mut changed = decoded.clone();
        changed.profile.policy.advertised_relays = true;
        assert!(changed.verify(&key.public()).is_err());
        let mut changed = decoded;
        changed.revision = 3;
        assert!(changed.verify(&key.public()).is_err());
    }

    #[test]
    fn incomplete_policy_and_inline_relay_key_are_refused() {
        let mut doc = serde_json::to_value(profile()).unwrap();
        doc["relays"][0]["token"] = "private-key".into();
        assert!(serde_json::from_value::<Profile>(doc).is_err());
        let mut doc = serde_json::to_value(profile()).unwrap();
        doc["policy"].as_object_mut().unwrap().remove("n0_lookup");
        assert!(serde_json::from_value::<Profile>(doc).is_err());

        let mut with_credentials = profile();
        with_credentials.relays[0].url = "https://relay.example.org/?token=private-key".into();
        assert!(with_credentials.validate().is_err());
    }
}
