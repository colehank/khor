use super::{
    BRANCH_BYTES, BRANCH_LIMIT, GitExploreResult, GitRemote, GitRemoteAccess, GitRemoteRef,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use gix::{
    bstr::ByteSlice,
    protocol::transport::client::blocking_io::http::{self, options::FollowRedirects},
    remote::Direction,
    url::Scheme,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub(super) fn remotes(repo: &gix::Repository) -> Result<GitExploreResult, String> {
    let items = repo
        .remote_names()
        .into_iter()
        .take(BRANCH_LIMIT)
        .filter_map(|name| {
            let name = name.to_str().ok()?;
            let remote = repo.find_remote(name).ok()?;
            let url = remote.url(Direction::Fetch)?;
            Some(GitRemote {
                name: name.to_owned(),
                host: url.host().map(str::to_owned),
                https: url.scheme == Scheme::Https,
            })
        })
        .collect();
    Ok(GitExploreResult::Remotes { items })
}

pub(super) fn remote_refs(
    repo: &gix::Repository,
    name: &str,
    access: Option<&GitRemoteAccess>,
) -> Result<GitExploreResult, String> {
    // A remote name is resolved from the repository, not used as a URL.
    if !repo
        .remote_names()
        .iter()
        .any(|remote| remote.as_slice() == name.as_bytes())
    {
        return Err("Git remote is no longer configured".into());
    }
    let options = gix::open::Options::default().config_overrides([
        "core.sshCommand=ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -o ConnectTimeout=8 -o ServerAliveInterval=5 -o ServerAliveCountMax=2",
        "ssh.variant=ssh",
    ]);
    let configured = gix::open_opts(repo.git_dir(), options)
        .map_err(|_| "Cannot read Git remote configuration")?;
    let remote = configured
        .find_remote(name)
        .map_err(|_| "Cannot read Git remote configuration")?;
    let url = remote
        .url(Direction::Fetch)
        .ok_or("No fetch URL for this Git remote")?;
    let http = matches!(url.scheme, Scheme::Https | Scheme::Http);
    if !http && url.scheme != Scheme::Ssh {
        return Err("This Git remote requires an HTTPS, HTTP or SSH URL".into());
    }
    let mut headers = Vec::new();
    if let Some(access) = access {
        if url.scheme != Scheme::Https
            || url.host() != Some(&access.host)
            || access.user.is_empty()
            || access.user.contains(':')
            || access.user.len() > 256
            || access.token.is_empty()
            || access.token.len() > 4096
        {
            return Err("Git remote address or credentials are invalid".into());
        }
        let basic = STANDARD.encode(format!("{}:{}", access.user, access.token));
        headers.push(format!("Authorization: Basic {basic}"));
    }
    // Require configured TLS/known hosts, use SSH agent when available, and
    // never call a Git credential or repository-defined external program.
    let mut connection = remote
        .connect(Direction::Fetch)
        .map_err(|_| "Cannot connect to the Git remote")?
        .with_credentials(|_| Ok(None));
    if http {
        let transport = http::Options {
            user_agent: Some("Khor".into()),
            follow_redirects: FollowRedirects::None,
            ssl_verify: true,
            extra_headers: headers,
            backend: Some(Arc::new(Mutex::new(http::reqwest::Options {
                configure_request: Some(Box::new(|request| {
                    *request.timeout_mut() = Some(Duration::from_secs(12));
                    Ok(())
                })),
            }))),
            ..Default::default()
        };
        connection = connection.with_transport_options(Box::new(transport));
    }
    let spec = gix::refspec::parse(
        "refs/heads/*:refs/khor/query/*".into(),
        gix::refspec::parse::Operation::Fetch,
    )
    .map_err(|_| "Cannot request remote Git branches")?
    .to_owned();
    let (map, _) = connection
        .ref_map(
            gix::progress::Discard,
            gix::remote::ref_map::Options {
                extra_refspecs: vec![spec],
                ..Default::default()
            },
        )
        .map_err(|_| "Git remote did not answer. Check its address and authentication.")?;
    let mut refs = Vec::new();
    let mut bytes = 0;
    let mut more = false;
    for reference in map.remote_refs {
        let (name, id, _) = reference.unpack();
        let (Ok(name), Some(id)) = (name.to_str(), id) else {
            continue;
        };
        if !name.starts_with("refs/heads/") || id.is_null() {
            continue;
        }
        let size = serde_json::to_vec(name).map_err(|e| e.to_string())?.len() + 130;
        if refs.len() >= BRANCH_LIMIT || bytes + size > BRANCH_BYTES {
            more = true;
            break;
        }
        refs.push(GitRemoteRef {
            reference: name.to_owned(),
            commit: id.to_string(),
            available: repo
                .find_commit(id.to_owned())
                .ok()
                .and_then(|commit| commit.tree_id().ok())
                .and_then(|tree| repo.try_find_header(tree).ok().flatten())
                .is_some(),
        });
        bytes += size;
    }
    refs.sort_unstable_by(|a, b| a.reference.cmp(&b.reference));
    Ok(GitExploreResult::RemoteRefs { refs, more })
}
