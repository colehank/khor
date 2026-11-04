use crate::{Channel, ReleaseIdentity, ReleaseTag, TrustRoot, VerifiedRelease};
use reqwest::{Client, Response};
use serde::Deserialize;
use std::time::Duration;

pub struct Feed {
    client: Client,
    trust: TrustRoot,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

impl Feed {
    pub fn github(trust: TrustRoot) -> Result<Self, String> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("khor-update/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client, trust })
    }

    pub async fn check(
        &self,
        current: &ReleaseIdentity,
        channel: Channel,
        exact: Option<&ReleaseTag>,
    ) -> Result<Option<VerifiedRelease>, String> {
        let tag = match exact {
            Some(tag) => tag.clone(),
            None => {
                let tags = bounded(
                    self.client
                        .get("https://api.github.com/repos/colehank/khor/releases?per_page=100")
                        .send()
                        .await
                        .map_err(|e| e.to_string())?,
                    2 * 1024 * 1024,
                )
                .await?;
                let releases: Vec<GitHubRelease> =
                    serde_json::from_slice(&tags).map_err(|e| e.to_string())?;
                let Some(tag) = choose(releases, channel) else {
                    return Ok(None);
                };
                if tag.version() <= current.tag.version() {
                    return Ok(None);
                }
                tag
            }
        };
        let prefix = format!(
            "https://github.com/colehank/khor/releases/download/{}/khor-release.json",
            tag.as_str()
        );
        let bytes = bounded(
            self.client
                .get(&prefix)
                .send()
                .await
                .map_err(|e| e.to_string())?,
            256 * 1024,
        )
        .await?;
        let signature = bounded(
            self.client
                .get(format!("{prefix}.sig"))
                .send()
                .await
                .map_err(|e| e.to_string())?,
            4096,
        )
        .await?;
        let release = self.trust.authenticate(
            &bytes,
            std::str::from_utf8(&signature).map_err(|e| e.to_string())?,
        )?;
        if release.manifest().identity.tag != tag {
            return Err("signed release does not match the selected tag".into());
        }
        release.manifest().newer_than(current, channel)?;
        Ok(Some(release))
    }
}

fn choose(releases: Vec<GitHubRelease>, channel: Channel) -> Option<ReleaseTag> {
    releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let tag: ReleaseTag = release.tag_name.try_into().ok()?;
            if !channel.accepts(&tag) || (channel == Channel::Stable && release.prerelease) {
                return None;
            }
            Some(tag)
        })
        .max_by_key(ReleaseTag::version)
}

async fn bounded(response: Response, limit: usize) -> Result<Vec<u8>, String> {
    let mut response = response.error_for_status().map_err(|e| e.to_string())?;
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err("release response exceeds its byte limit".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err("release response exceeds its byte limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn releases() -> Vec<GitHubRelease> {
        [
            ("v0.2.0-rc.9", false, true),
            ("v0.2.0-rc.10", false, true),
            ("v0.1.0", false, false),
            ("v9.0.0", true, false),
            ("unsafe/tag", false, false),
        ]
        .into_iter()
        .map(|(tag_name, draft, prerelease)| GitHubRelease {
            tag_name: tag_name.into(),
            draft,
            prerelease,
        })
        .collect()
    }
    #[test]
    fn beta_discovery_does_not_depend_on_github_latest_or_lexical_order() {
        assert_eq!(
            choose(releases(), Channel::Beta).unwrap().as_str(),
            "v0.2.0-rc.10"
        );
        assert_eq!(
            choose(releases(), Channel::Stable).unwrap().as_str(),
            "v0.1.0"
        );
    }
}
