use crate::{Channel, ReleaseIdentity, ReleaseTag, TrustRoot, VerifiedRelease};
use reqwest::{Client, Response};
use serde::Deserialize;
use std::time::Duration;
use xml::reader::{EventReader, XmlEvent};

pub struct Feed {
    client: Client,
    trust: TrustRoot,
}

#[derive(Clone, Deserialize)]
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
                let Some(tag) = discover(
                    &self.client,
                    channel,
                    "https://github.com/colehank/khor/releases.atom",
                    "https://api.github.com/repos/colehank/khor/releases?per_page=100",
                    "https://github.com/colehank/khor/releases/latest",
                )
                .await?
                else {
                    return Ok(None);
                };
                if tag.version() <= current.tag.version() {
                    return Ok(None);
                }
                tag
            }
        };
        let release = self.release(&tag).await?;
        release.manifest().newer_than(current, channel)?;
        Ok(Some(release))
    }

    pub async fn release(&self, tag: &ReleaseTag) -> Result<VerifiedRelease, String> {
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
        if &release.manifest().identity.tag != tag {
            return Err("signed release does not match the selected tag".into());
        }
        Ok(release)
    }
}

// Discovery is only a hint. The chosen release and every downloaded byte
// still pass through the existing signed-manifest and Tauri verification.
// The public Atom feed avoids consuming a shared IP's anonymous API quota.
async fn discover(
    client: &Client,
    channel: Channel,
    atom: &str,
    api: &str,
    latest: &str,
) -> Result<Option<ReleaseTag>, String> {
    let syndication = async {
        let response = client
            .get(atom)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        atom_releases(&bounded(response, 2 * 1024 * 1024).await?)
    }
    .await;
    match &syndication {
        Ok(releases) => {
            if let Some(tag) = choose(releases.clone(), channel) {
                return Ok(Some(tag));
            }
        }
        Err(_) => {}
    }
    // The feed has a finite history. Resolve the public stable permalink
    // before falling back to API history, so normal stable discovery also
    // works from an IP whose anonymous API quota is exhausted.
    if channel == Channel::Stable {
        if let Ok(stable) = latest_stable(client, latest).await {
            return Ok(stable);
        }
    }
    let releases = async {
        let response = client
            .get(api)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let bytes = bounded(response, 2 * 1024 * 1024).await?;
        serde_json::from_slice::<Vec<GitHubRelease>>(&bytes).map_err(|error| error.to_string())
    }
    .await;
    match releases {
        Ok(releases) => Ok(choose(releases, channel)),
        Err(error) => Err(match syndication {
            Err(atom_error) => format!("release discovery unavailable: {atom_error}; {error}"),
            Ok(_) => format!("release channel history unavailable: {error}"),
        }),
    }
}

async fn latest_stable(client: &Client, latest: &str) -> Result<Option<ReleaseTag>, String> {
    let endpoint = reqwest::Url::parse(latest).map_err(|error| error.to_string())?;
    let response = client
        .head(endpoint.clone())
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response = response
        .error_for_status()
        .map_err(|error| error.to_string())?;
    if response.url().origin() != endpoint.origin() {
        return Err("stable release permalink left its publisher".into());
    }
    let tag = response
        .url()
        .path()
        .strip_prefix("/colehank/khor/releases/tag/")
        .ok_or("stable release permalink did not name a release")?;
    let tag = ReleaseTag::try_from(tag.to_owned())?;
    Ok((tag.channel() == Channel::Stable).then_some(tag))
}

fn atom_releases(bytes: &[u8]) -> Result<Vec<GitHubRelease>, String> {
    const ATOM: &str = "http://www.w3.org/2005/Atom";
    const PREFIX: &str = "https://github.com/colehank/khor/releases/tag/";
    let mut releases = Vec::new();
    let mut entry = false;
    let mut feed = false;
    for event in EventReader::new(bytes) {
        match event.map_err(|error| error.to_string())? {
            XmlEvent::StartElement {
                name, attributes, ..
            } if name.namespace.as_deref() == Some(ATOM) => match name.local_name.as_str() {
                "feed" => feed = true,
                "entry" if feed => entry = true,
                "link" if entry => {
                    let alternate = attributes
                        .iter()
                        .any(|a| a.name.local_name == "rel" && a.value == "alternate");
                    if !alternate {
                        continue;
                    }
                    let tag = attributes
                        .iter()
                        .find(|a| a.name.local_name == "href")
                        .and_then(|a| a.value.strip_prefix(PREFIX))
                        .and_then(|value| ReleaseTag::try_from(value.to_owned()).ok());
                    if let Some(tag) = tag {
                        releases.push(GitHubRelease {
                            tag_name: tag.as_str().into(),
                            draft: false,
                            prerelease: tag.channel() == Channel::Beta,
                        });
                    }
                }
                _ => {}
            },
            XmlEvent::EndElement { name }
                if name.namespace.as_deref() == Some(ATOM) && name.local_name == "entry" =>
            {
                entry = false
            }
            _ => {}
        }
    }
    if !feed {
        return Err("release discovery did not return an Atom feed".into());
    }
    Ok(releases)
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

    fn atom(tags: &[&str]) -> String {
        format!(
            "<feed xmlns=\"http://www.w3.org/2005/Atom\">{}</feed>",
            tags.iter().map(|tag| format!(
                "<entry><title>Untrusted display title</title><link rel=\"alternate\" href=\"https://github.com/colehank/khor/releases/tag/{tag}\"/></entry>"
            )).collect::<String>()
        )
    }

    #[test]
    fn atom_discovery_uses_canonical_release_links_and_semver_channel_order() {
        let bytes = atom(&["v0.2.0-rc.9", "v0.2.0-rc.11", "v0.1.0", "unsafe/tag"]);
        let releases = atom_releases(bytes.as_bytes()).unwrap();
        assert_eq!(
            choose(releases.clone(), Channel::Beta).unwrap().as_str(),
            "v0.2.0-rc.11"
        );
        assert_eq!(
            choose(releases, Channel::Stable).unwrap().as_str(),
            "v0.1.0"
        );
        let misleading = r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><title>v9.0.0</title><link rel="alternate" href="https://evil.example/releases/tag/v9.0.0"/><link rel="enclosure" href="https://github.com/colehank/khor/releases/tag/v8.0.0"/></entry></feed>"#;
        assert!(atom_releases(misleading.as_bytes()).unwrap().is_empty());
        assert!(atom_releases(b"<html/>").is_err());
        assert!(atom_releases(b"<feed xmlns=\"http://www.w3.org/2005/Atom\">").is_err());
    }

    fn discovery_server(
        responses: Vec<(&'static str, u16, String)>,
    ) -> (String, std::thread::JoinHandle<()>) {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            for (route, status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = [0; 8192];
                let read = stream.read(&mut request).unwrap();
                assert_eq!(
                    std::str::from_utf8(&request[..read])
                        .unwrap()
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1),
                    Some(route)
                );
                if status == 302 {
                    write!(stream, "HTTP/1.1 302 Found\r\nLocation: {body}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                } else {
                    write!(stream, "HTTP/1.1 {status} Result\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                }
            }
        });
        (address, worker)
    }

    #[tokio::test]
    async fn published_feed_never_needs_an_anonymous_api_quota() {
        // Any API request would fail this server's route assertion. Discovery
        // still finds the real RC channel when that API is inaccessible.
        let (address, worker) = discovery_server(vec![("/atom", 200, atom(&["v0.2.0-rc.11"]))]);
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let selected = discover(
            &client,
            Channel::Beta,
            &format!("{address}/atom"),
            &format!("{address}/api-rate-limited"),
            &format!("{address}/latest"),
        )
        .await
        .unwrap();
        assert_eq!(selected.unwrap().as_str(), "v0.2.0-rc.11");
        worker.join().unwrap();
    }

    #[tokio::test]
    async fn stable_discovery_uses_public_permalink_when_feed_only_contains_prereleases() {
        let (address, worker) = discovery_server(vec![
            ("/atom", 200, atom(&["v0.2.0-rc.11"])),
            ("/latest", 302, "/colehank/khor/releases/tag/v0.1.0".into()),
            ("/colehank/khor/releases/tag/v0.1.0", 200, String::new()),
        ]);
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let selected = discover(
            &client,
            Channel::Stable,
            &format!("{address}/atom"),
            &format!("{address}/api-rate-limited"),
            &format!("{address}/latest"),
        )
        .await
        .unwrap();
        assert_eq!(selected.unwrap().as_str(), "v0.1.0");
        worker.join().unwrap();
    }

    #[tokio::test]
    async fn a_prerelease_only_repository_has_no_stable_update_without_needing_api() {
        let (address, worker) = discovery_server(vec![
            ("/atom", 200, atom(&["v0.2.0-rc.11"])),
            ("/latest", 404, String::new()),
        ]);
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        assert!(
            discover(
                &client,
                Channel::Stable,
                &format!("{address}/atom"),
                &format!("{address}/api-rate-limited"),
                &format!("{address}/latest")
            )
            .await
            .unwrap()
            .is_none()
        );
        worker.join().unwrap();
    }
}
