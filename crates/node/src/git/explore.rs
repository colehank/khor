use super::{
    BRANCH_BYTES, BRANCH_LIMIT, CommitFile, CommitSummary, SHOWN, WALK, diff_files, open_rel,
    summary_of,
};
use gix::bstr::ByteSlice;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};
use ts_rs::TS;

const DIR_ROWS: usize = 200;
const COMPARE_COMMITS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitExploreQuery {
    References,
    Resolve {
        reference: String,
    },
    Directory {
        commit: String,
        path: String,
        offset: u32,
    },
    File {
        commit: String,
        path: String,
    },
    Compare {
        base: String,
        head: String,
    },
    Merge {
        base: String,
        head: String,
    },
    Worktrees,
    Remotes,
    RemoteRefs {
        remote: String,
        access: Option<GitRemoteAccess>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct GitSnapshotEntry {
    pub name: String,
    pub kind: GitEntryKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GitEntryKind {
    Directory,
    File,
    Symlink,
    Submodule,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct GitWorktree {
    pub path: String,
    pub reference: Option<String>,
    pub commit: Option<String>,
    pub current: bool,
    pub available: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GitRemoteAccess {
    pub host: String,
    pub user: String,
    pub token: String,
}

impl std::fmt::Debug for GitRemoteAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GitRemoteAccess([redacted])")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct GitRemote {
    pub name: String,
    pub host: Option<String>,
    pub https: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct GitRemoteRef {
    pub reference: String,
    pub commit: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitExploreResult {
    References {
        current: Option<String>,
        refs: Vec<String>,
        more: bool,
    },
    Resolved {
        commit: String,
    },
    Directory {
        entries: Vec<GitSnapshotEntry>,
        next: Option<u32>,
        incomplete: bool,
    },
    File {
        entry: Option<GitEntryKind>,
        text: Option<String>,
        bytes: u32,
        too_large: bool,
    },
    Compare {
        ancestor: Option<String>,
        base_only: Vec<CommitSummary>,
        head_only: Vec<CommitSummary>,
        behind: u32,
        ahead: u32,
        commits_cut: bool,
        files: Vec<CommitFile>,
        total: u32,
        more: bool,
    },
    Merge {
        conflicts: Vec<String>,
        total: u32,
    },
    Worktrees {
        items: Vec<GitWorktree>,
        more: bool,
    },
    Remotes {
        items: Vec<GitRemote>,
    },
    RemoteRefs {
        refs: Vec<GitRemoteRef>,
        more: bool,
    },
}

pub(super) fn answer(dir: &Path, query: &GitExploreQuery) -> Result<GitExploreResult, String> {
    let (repo, relative) = open_rel(dir)?;
    match query {
        GitExploreQuery::References => references(&repo),
        GitExploreQuery::Resolve { reference } => Ok(GitExploreResult::Resolved {
            commit: resolve(&repo, reference)?.to_string(),
        }),
        GitExploreQuery::Directory {
            commit,
            path,
            offset,
        } => directory(&repo, commit, &scoped_path(&relative, path), *offset),
        GitExploreQuery::File { commit, path } => {
            file(&repo, commit, &scoped_path(&relative, path))
        }
        GitExploreQuery::Compare { base, head } => compare(&repo, base, head),
        GitExploreQuery::Merge { base, head } => merge(repo, base, head),
        GitExploreQuery::Worktrees => worktrees(&repo),
        GitExploreQuery::Remotes => remote::remotes(&repo),
        GitExploreQuery::RemoteRefs { remote, access } => {
            remote::remote_refs(&repo, remote, access.as_ref())
        }
    }
}

fn references(repo: &gix::Repository) -> Result<GitExploreResult, String> {
    let current = head_reference(repo)?;
    let platform = repo.references().map_err(|e| e.to_string())?;
    let mut refs = Vec::new();
    let mut bytes = 0;
    let mut more = false;
    'refs: for iter in [
        platform.local_branches(),
        platform.tags(),
        platform.remote_branches(),
    ] {
        for item in iter.map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            if item.try_id().is_none() {
                continue;
            }
            let Ok(name) = item.name().as_bstr().to_str() else {
                continue;
            };
            let size = serde_json::to_vec(name).map_err(|e| e.to_string())?.len() + 1;
            if refs.len() >= BRANCH_LIMIT || bytes + size > BRANCH_BYTES {
                more = true;
                break 'refs;
            }
            refs.push(name.to_owned());
            bytes += size;
        }
    }
    refs.sort_unstable();
    Ok(GitExploreResult::References {
        current,
        refs,
        more,
    })
}

fn head_reference(repo: &gix::Repository) -> Result<Option<String>, String> {
    Ok(repo
        .head_name()
        .map_err(|e| e.to_string())?
        .and_then(|name| name.as_bstr().to_str().ok().map(str::to_owned)))
}

pub(super) fn resolve(repo: &gix::Repository, input: &str) -> Result<gix::ObjectId, String> {
    let id = if input == "HEAD" {
        repo.head_id().map_err(|e| e.to_string())?.detach()
    } else if ["refs/heads/", "refs/remotes/", "refs/tags/"]
        .iter()
        .any(|prefix| input.starts_with(prefix))
    {
        let reference = repo.find_reference(input).map_err(|e| e.to_string())?;
        if reference.name().as_bstr() != input.as_bytes() {
            return Err("Git reference was not found".into());
        }
        reference
            .into_fully_peeled_id()
            .map_err(|e| e.to_string())?
            .detach()
    } else {
        exact_commit(repo, input)?
    };
    repo.find_commit(id).map_err(|e| e.to_string())?;
    Ok(id)
}

pub(super) fn exact_commit(repo: &gix::Repository, input: &str) -> Result<gix::ObjectId, String> {
    gix::ObjectId::from_hex(input.as_bytes())
        .ok()
        .filter(|id| id.kind() == repo.object_hash())
        .ok_or_else(|| "Expected a full Git commit ID".to_owned())
}

fn snapshot_tree<'repo>(
    repo: &'repo gix::Repository,
    commit: &str,
) -> Result<gix::Tree<'repo>, String> {
    repo.find_commit(exact_commit(repo, commit)?)
        .map_err(|e| e.to_string())?
        .tree()
        .map_err(|e| e.to_string())
}

fn scoped_path(root: &str, child: &str) -> String {
    match (root.is_empty(), child.is_empty()) {
        (true, _) => child.to_owned(),
        (_, true) => root.to_owned(),
        (false, false) => format!("{root}/{child}"),
    }
}

fn valid_component(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains('\0')
}

fn entry_at<'repo>(
    tree: &gix::Tree<'repo>,
    path: &str,
) -> Result<Option<gix::object::tree::Entry<'repo>>, String> {
    if !path.split('/').all(valid_component) {
        return Err("Expected a repository-relative Git path".into());
    }
    // Git uses slash separators on every platform. Use its raw components to
    // preserve names containing a literal backslash in a repository.
    tree.lookup_entry(path.split('/').map(str::as_bytes))
        .map_err(|e| e.to_string())
}

fn entry_kind(mode: gix::object::tree::EntryMode) -> GitEntryKind {
    use gix::object::tree::EntryKind::*;
    match mode.kind() {
        Tree => GitEntryKind::Directory,
        Blob | BlobExecutable => GitEntryKind::File,
        Link => GitEntryKind::Symlink,
        Commit => GitEntryKind::Submodule,
    }
}

fn directory(
    repo: &gix::Repository,
    commit: &str,
    path: &str,
    offset: u32,
) -> Result<GitExploreResult, String> {
    let root = snapshot_tree(repo, commit)?;
    let tree = if path.is_empty() {
        root
    } else {
        let entry = entry_at(&root, path)?.ok_or("Path is absent in this Git snapshot")?;
        if !entry.mode().is_tree() {
            return Err("Path is not a Git directory".into());
        }
        repo.find_tree(entry.object_id())
            .map_err(|e| e.to_string())?
    };
    let mut entries = Vec::new();
    let mut incomplete = false;
    let mut next = None;
    let mut bytes = 0;
    for (index, entry) in tree.iter().enumerate().skip(offset as usize) {
        let entry = entry.map_err(|e| e.to_string())?;
        let Ok(name) = entry.filename().to_str() else {
            incomplete = true;
            continue;
        };
        let size = serde_json::to_vec(name).map_err(|e| e.to_string())?.len() + 56;
        if !valid_component(name) || size > BRANCH_BYTES {
            incomplete = true;
            continue;
        }
        if entries.len() >= DIR_ROWS || bytes + size > BRANCH_BYTES {
            next = Some(u32::try_from(index).map_err(|_| "Git directory has too many entries")?);
            break;
        }
        entries.push(GitSnapshotEntry {
            name: name.to_owned(),
            kind: entry_kind(entry.mode()),
        });
        bytes += size;
    }
    Ok(GitExploreResult::Directory {
        entries,
        next,
        incomplete,
    })
}

fn file(repo: &gix::Repository, commit: &str, path: &str) -> Result<GitExploreResult, String> {
    let tree = snapshot_tree(repo, commit)?;
    let Some(entry) = entry_at(&tree, path)? else {
        return Ok(GitExploreResult::File {
            entry: None,
            text: None,
            bytes: 0,
            too_large: false,
        });
    };
    let kind = entry_kind(entry.mode());
    if matches!(kind, GitEntryKind::Directory) {
        return Err("Path is a Git directory".into());
    }
    if matches!(kind, GitEntryKind::Submodule) {
        return Ok(GitExploreResult::File {
            entry: Some(kind),
            text: Some(entry.object_id().to_string()),
            bytes: 0,
            too_large: false,
        });
    }
    let id = entry.object_id();
    let length = repo.find_header(id).map_err(|e| e.to_string())?.size();
    let bytes = u32::try_from(length).unwrap_or(u32::MAX);
    let text = if length <= SHOWN as u64 {
        let blob = repo.find_blob(id).map_err(|e| e.to_string())?;
        if blob.data.contains(&0) {
            None
        } else {
            std::str::from_utf8(&blob.data).ok().map(str::to_owned)
        }
    } else {
        None
    };
    Ok(GitExploreResult::File {
        entry: Some(kind),
        text,
        bytes,
        too_large: length > SHOWN as u64,
    })
}

fn unique(
    repo: &gix::Repository,
    from: gix::ObjectId,
    other: gix::ObjectId,
) -> Result<(Vec<CommitSummary>, u32, bool), String> {
    let mut rows = Vec::new();
    let mut count = 0;
    let walk = repo
        .rev_walk([from])
        .with_hidden([other])
        .all()
        .map_err(|e| e.to_string())?;
    for item in walk {
        let item = item.map_err(|e| e.to_string())?;
        if count as usize >= WALK {
            return Ok((rows, count, true));
        }
        if rows.len() < COMPARE_COMMITS {
            rows.push(summary_of(
                &repo.find_commit(item.id).map_err(|e| e.to_string())?,
            )?);
        }
        count += 1;
    }
    Ok((rows, count, false))
}

fn compare(repo: &gix::Repository, base: &str, head: &str) -> Result<GitExploreResult, String> {
    if repo.shallow_commits().map_err(|e| e.to_string())?.is_some() {
        return Err("Repository history is incomplete in a shallow checkout".into());
    }
    let base = resolve(repo, base)?;
    let head = resolve(repo, head)?;
    let ancestor = match repo.merge_base(base, head) {
        Ok(id) => Some(id.detach()),
        Err(gix::repository::merge_base::Error::NotFound { .. }) => None,
        Err(error) => return Err(error.to_string()),
    };
    let from = ancestor
        .map(|id| {
            repo.find_commit(id)
                .map_err(|e| e.to_string())?
                .tree()
                .map_err(|e| e.to_string())
        })
        .transpose()?;
    let to = repo
        .find_commit(head)
        .map_err(|e| e.to_string())?
        .tree()
        .map_err(|e| e.to_string())?;
    let (files, total, more) = diff_files(repo, from.as_ref(), &to)?;
    let (base_only, behind, base_cut) = unique(repo, base, head)?;
    let (head_only, ahead, head_cut) = unique(repo, head, base)?;
    Ok(GitExploreResult::Compare {
        ancestor: ancestor.map(|id| id.to_string()),
        base_only,
        head_only,
        behind,
        ahead,
        commits_cut: base_cut || head_cut,
        files,
        total,
        more,
    })
}

fn merge(repo: gix::Repository, base: &str, head: &str) -> Result<GitExploreResult, String> {
    let repo = gix::open_opts(repo.git_dir(), gix::open::Options::isolated())
        .map_err(|e| e.to_string())?
        .with_object_memory();
    if repo.shallow_commits().map_err(|e| e.to_string())?.is_some() {
        return Err("Repository history is incomplete in a shallow checkout".into());
    }
    let base = resolve(&repo, base)?;
    let head = resolve(&repo, head)?;
    // Repository-defined programs may mutate the worktree or allow untrusted
    // changes to escape the disposable object store.
    if !repo
        .merge_resource_cache(Default::default())
        .map_err(|e| e.to_string())?
        .drivers()
        .is_empty()
        || !gix::filter::Pipeline::options(&repo)
            .map_err(|e| e.to_string())?
            .drivers
            .is_empty()
    {
        return Err("Merge preview requires built-in merge drivers and file filters".into());
    }
    let options = repo.tree_merge_options().map_err(|e| e.to_string())?.into();
    let result = repo
        .merge_commits(base, head, Default::default(), options)
        .map_err(|e| e.to_string())?;
    let paths: BTreeSet<String> = result
        .tree_merge
        .conflicts
        .iter()
        .filter(|conflict| conflict.is_unresolved(gix::merge::tree::TreatAsUnresolved::git()))
        .flat_map(|conflict| [&conflict.ours, &conflict.theirs])
        .map(|change| change.location().to_str_lossy().into_owned())
        .collect();
    let total = u32::try_from(paths.len()).unwrap_or(u32::MAX);
    let mut bytes = 0;
    let conflicts = paths
        .into_iter()
        .take_while(|s| {
            bytes += s.len().saturating_mul(6) + 3;
            bytes <= BRANCH_BYTES
        })
        .take(BRANCH_LIMIT)
        .collect();
    Ok(GitExploreResult::Merge { conflicts, total })
}

fn worktrees(repo: &gix::Repository) -> Result<GitExploreResult, String> {
    let mut items = Vec::new();
    let mut more = false;
    let main = repo.main_repo().map_err(|e| e.to_string())?;
    let linked = repo.worktrees().map_err(|e| e.to_string())?;
    let canonical_git_dir = |checkout: &gix::Repository| {
        checkout
            .git_dir()
            .canonicalize()
            .unwrap_or_else(|_| checkout.git_dir().to_path_buf())
    };
    let current = canonical_git_dir(repo);
    let mut bytes = 0;
    for checkout in std::iter::once(Ok(main)).chain(
        linked
            .into_iter()
            .map(|item| item.into_repo_with_possibly_inaccessible_worktree()),
    ) {
        let checkout = checkout.map_err(|e| e.to_string())?;
        let Some(path) = checkout.workdir().and_then(Path::to_str) else {
            continue;
        };
        let reference = head_reference(&checkout)?;
        let commit = checkout.head_id().ok().map(|id| id.to_string());
        let size = path
            .len()
            .saturating_add(reference.as_ref().map_or(0, String::len))
            .saturating_mul(6)
            + 150;
        if bytes + size > BRANCH_BYTES || items.len() >= BRANCH_LIMIT {
            more = true;
            break;
        }
        items.push(GitWorktree {
            path: path.to_owned(),
            reference,
            commit,
            current: canonical_git_dir(&checkout) == current,
            available: checkout.workdir().is_some_and(Path::is_dir),
        });
        bytes += size;
    }
    items.sort_by(|a, b| b.current.cmp(&a.current).then_with(|| a.path.cmp(&b.path)));
    Ok(GitExploreResult::Worktrees { items, more })
}

mod remote;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{GitAnswer, GitAsk, repository_history_tests::Checkout};
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        time::Duration,
    };

    fn make_checkout() -> (Checkout, String) {
        let checkout = Checkout::new();
        checkout.git(&["config", "user.name", "Lab"]);
        checkout.git(&["config", "user.email", "lab@example.test"]);
        fs::write(checkout.0.join("README.md"), "shared base\n").unwrap();
        checkout.git(&["add", "README.md"]);
        checkout.git(&["commit", "-qm", "shared base"]);
        let base = id(&checkout);
        checkout.git(&["tag", "-am", "release", "v1.0.0"]);
        (checkout, base)
    }

    fn id(checkout: &Checkout) -> String {
        String::from_utf8(checkout.git(&["rev-parse", "HEAD"]))
            .unwrap()
            .trim()
            .to_owned()
    }

    #[test]
    fn browse_compare_and_merge_leave_both_checkouts_intact() {
        let (checkout, initial) = make_checkout();
        let scratch = tempfile::tempdir().unwrap();
        let linked = scratch.path().join("linked");
        checkout.git(&["worktree", "add", "-qb", "topic", linked.to_str().unwrap()]);
        fs::write(linked.join("README.md"), "topic edit\n").unwrap();
        fs::create_dir(linked.join("docs")).unwrap();
        fs::write(linked.join("docs/only.md"), "only in topic\n").unwrap();
        checkout.git(&["-C", linked.to_str().unwrap(), "add", "."]);
        checkout.git(&[
            "-C",
            linked.to_str().unwrap(),
            "commit",
            "-qm",
            "topic work",
        ]);
        fs::write(checkout.0.join("README.md"), "base edit\n").unwrap();
        checkout.git(&["commit", "-qam", "main work"]);
        fs::write(checkout.0.join("README.md"), "unsaved worktree edit\n").unwrap();

        let ask = |query| answer(&checkout.0, &query).unwrap();
        let GitExploreResult::Resolved { commit: topic } = ask(GitExploreQuery::Resolve {
            reference: "refs/heads/topic".into(),
        }) else {
            panic!()
        };
        let GitExploreResult::Resolved { commit: tag } = ask(GitExploreQuery::Resolve {
            reference: "refs/tags/v1.0.0".into(),
        }) else {
            panic!()
        };
        assert_eq!(tag, initial);
        let GitExploreResult::References { refs, .. } = ask(GitExploreQuery::References) else {
            panic!()
        };
        assert!(refs.iter().any(|reference| reference == "refs/tags/v1.0.0"));

        let GitExploreResult::Directory { entries, .. } = ask(GitExploreQuery::Directory {
            commit: topic.clone(),
            path: "docs".into(),
            offset: 0,
        }) else {
            panic!()
        };
        assert_eq!(entries[0].name, "only.md");
        let GitExploreResult::File { text, .. } = ask(GitExploreQuery::File {
            commit: topic.clone(),
            path: "docs/only.md".into(),
        }) else {
            panic!()
        };
        assert_eq!(text.as_deref(), Some("only in topic\n"));
        let GitExploreResult::File { entry: None, .. } = ask(GitExploreQuery::File {
            commit: tag.clone(),
            path: "docs/only.md".into(),
        }) else {
            panic!("missing file must stay missing")
        };
        assert!(
            answer(
                &checkout.0,
                &GitExploreQuery::File {
                    commit: topic.clone(),
                    path: "../README.md".into()
                }
            )
            .is_err()
        );

        let base = id(&checkout);
        let GitExploreResult::Compare {
            ancestor,
            files,
            ahead,
            behind,
            ..
        } = ask(GitExploreQuery::Compare {
            base: base.clone(),
            head: topic.clone(),
        })
        else {
            panic!()
        };
        assert_eq!(ancestor, Some(initial));
        assert_eq!((ahead, behind), (1, 1));
        assert!(
            files
                .iter()
                .any(|file| file.path == "docs/only.md" && file.status == "A")
        );
        assert!(files.iter().any(|file| file.path == "README.md"
            && file.before == "shared base\n"
            && file.after == "topic edit\n"));

        let objects = checkout.git(&["count-objects", "-v"]);
        let index_before = fs::read(checkout.0.join(".git/index")).unwrap();
        let GitExploreResult::Merge { conflicts, total } = ask(GitExploreQuery::Merge {
            base: base.clone(),
            head: topic.clone(),
        }) else {
            panic!()
        };
        assert_eq!(conflicts, ["README.md"]);
        assert_eq!(total, 1);
        let GitExploreResult::Merge { total: 0, .. } = ask(GitExploreQuery::Merge {
            base: tag,
            head: topic.clone(),
        }) else {
            panic!("fast-forward preview")
        };
        assert_eq!(checkout.git(&["count-objects", "-v"]), objects);
        assert_eq!(
            fs::read(checkout.0.join(".git/index")).unwrap(),
            index_before
        );
        assert_eq!(
            fs::read_to_string(checkout.0.join("README.md")).unwrap(),
            "unsaved worktree edit\n"
        );
        assert_eq!(id(&checkout), base);

        let GitExploreResult::Worktrees { items, .. } = ask(GitExploreQuery::Worktrees) else {
            panic!()
        };
        assert_eq!(items.len(), 2);
        assert!(items[0].current);
        assert!(items[1].available);
        assert_eq!(items[1].reference.as_deref(), Some("refs/heads/topic"));
        let GitExploreResult::Worktrees {
            items: from_linked, ..
        } = answer(&linked, &GitExploreQuery::Worktrees).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            from_linked[0].reference.as_deref(),
            Some("refs/heads/topic")
        );
        assert!(from_linked[0].current);

        let page = crate::git::answer(&GitAsk::SnapshotLog {
            dir: checkout.0.display().to_string(),
            commit: topic.clone(),
            cursor: None,
            limit: 1,
        })
        .unwrap();
        let GitAnswer::RepositoryLog {
            head,
            commits,
            next,
        } = page
        else {
            panic!()
        };
        assert_eq!(head.as_deref(), Some(topic.as_str()));
        assert_eq!(commits[0].subject, "topic work");
        assert!(
            crate::git::answer(&GitAsk::SnapshotLog {
                dir: checkout.0.display().to_string(),
                commit: base,
                cursor: next,
                limit: 1,
            })
            .is_err()
        );
    }

    #[test]
    fn git_exploration_crosses_the_node_wire() {
        use crate::proto::{Request, Response, decode, encode_request, encode_response};
        let ask = GitAsk::Explore {
            dir: "/checkout".into(),
            query: GitExploreQuery::Directory {
                commit: "c".repeat(40),
                path: "docs".into(),
                offset: 2,
            },
        };
        let bytes = encode_request(&Request::Git { ask: ask.clone() }).unwrap();
        let Request::Git { ask: decoded } = decode(&bytes).unwrap() else {
            panic!()
        };
        assert_eq!(decoded, ask);
        let reply = GitAnswer::Explore {
            result: GitExploreResult::File {
                entry: Some(GitEntryKind::File),
                text: Some("fixture".into()),
                bytes: 7,
                too_large: false,
            },
        };
        let bytes = encode_response(&Response::Git { answer: reply }).unwrap();
        let Response::Git {
            answer:
                GitAnswer::Explore {
                    result:
                        GitExploreResult::File {
                            entry: Some(GitEntryKind::File),
                            text,
                            ..
                        },
                },
        } = decode(&bytes).unwrap()
        else {
            panic!()
        };
        assert_eq!(text.as_deref(), Some("fixture"));
    }

    #[test]
    fn queries_live_http_branch_advertisement_without_fetching() {
        let (checkout, local) = make_checkout();
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/repository", server.local_addr().unwrap());
        checkout.git(&["remote", "add", "origin", &url]);
        let saved_config = fs::read(checkout.0.join(".git/config")).unwrap();
        let remote_only = "e".repeat(40);
        let served = remote_only.clone();
        let advertised = local.clone();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = server.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            assert!(request.starts_with(b"GET /repository/info/refs?service=git-upload-pack "));
            let pkt = |line: &str| format!("{:04x}{line}", line.len() + 4);
            let body = [
                pkt("# service=git-upload-pack\n"),
                "0000".into(),
                pkt(&format!(
                    "{advertised} HEAD\0symref=HEAD:refs/heads/release\n"
                )),
                pkt(&format!("{advertised} refs/heads/release\n")),
                pkt(&format!("{served} refs/heads/upcoming\n")),
                "0000".into(),
            ]
            .concat();
            write!(socket,
                "HTTP/1.1 200 OK\r\nContent-Type: application/x-git-upload-pack-advertisement\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body,
            ).unwrap();
        });
        let query = GitExploreQuery::RemoteRefs {
            remote: "origin".into(),
            access: None,
        };
        let GitExploreResult::RemoteRefs { refs, more } = answer(&checkout.0, &query).unwrap()
        else {
            panic!()
        };
        server.join().unwrap();
        assert!(!more);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].commit, local);
        assert!(refs[0].available);
        assert_eq!(refs[1].commit, remote_only);
        assert!(!refs[1].available);
        assert_eq!(
            fs::read(checkout.0.join(".git/config")).unwrap(),
            saved_config
        );
        assert!(checkout.git(&["for-each-ref", "refs/remotes"]).is_empty());
        assert!(
            answer(
                &checkout.0,
                &GitExploreQuery::RemoteRefs {
                    remote: url,
                    access: None
                }
            )
            .is_err()
        );

        let secret = GitRemoteAccess {
            host: "example.test".into(),
            user: "user".into(),
            token: "secret".into(),
        };
        assert!(!format!("{secret:?}").contains("secret"));
    }
}
