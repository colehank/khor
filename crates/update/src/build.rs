use crate::{ReleaseIdentity, ReleaseTag, SourceCommit};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildIdentity {
    pub version: String,
    pub source: Option<SourceCommit>,
    pub release: Option<ReleaseIdentity>,
}

pub fn build_identity() -> Result<BuildIdentity, String> {
    from_values(
        env!("CARGO_PKG_VERSION"),
        env!("KHOR_BUILD_SOURCE"),
        env!("KHOR_RELEASE_TAG"),
        env!("KHOR_RELEASE_BUILD"),
    )
}

fn from_values(
    version: &str,
    source: &str,
    tag: &str,
    build: &str,
) -> Result<BuildIdentity, String> {
    let source = if source.is_empty() {
        None
    } else {
        Some(SourceCommit::try_from(source.to_owned())?)
    };
    let release = if tag.is_empty() && build.is_empty() {
        None
    } else {
        let tag = ReleaseTag::try_from(tag.to_owned())?;
        let package = semver::Version::parse(version).map_err(|e| e.to_string())?;
        let full = tag.version();
        if (package.major, package.minor, package.patch) != (full.major, full.minor, full.patch) {
            return Err("release tag disagrees with the package version".into());
        }
        let build = build
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or("release build must be a positive integer")?;
        Some(ReleaseIdentity {
            tag,
            source: source.clone().ok_or("release source commit is missing")?,
            build,
        })
    };
    Ok(BuildIdentity {
        version: version.into(),
        source,
        release,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_apple_version_is_separate_from_complete_release_and_development_identity() {
        let source = "a".repeat(40);
        let dev = from_values("0.2.0", &source, "", "").unwrap();
        assert!(dev.release.is_none());
        let rc = from_values("0.2.0", &source, "v0.2.0-rc.12", "123").unwrap();
        assert_eq!(rc.version, "0.2.0");
        assert_eq!(rc.release.unwrap().tag.as_str(), "v0.2.0-rc.12");
        for (source, tag, build) in [
            ("", "v0.2.0", "123"),
            (&source[..], "v0.2.0", ""),
            (&source[..], "v0.3.0", "123"),
            (&source[..], "", "123"),
        ] {
            assert!(from_values("0.2.0", source, tag, build).is_err());
        }
    }
}
