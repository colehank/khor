use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use notify::{EventKind, RecursiveMode, Watcher};

use super::{Scope, changes_files};

const PATH_BYTES: usize = 64 * 1024;
const DIRECTORIES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Target {
    path: PathBuf,
    recursive: bool,
}

#[derive(Default)]
struct Raw {
    paths: BTreeMap<PathBuf, bool>,
    bytes: usize,
    reset: bool,
    error: Option<String>,
}

impl Raw {
    fn take(&mut self, event: notify::Result<notify::Event>) {
        let event = match event {
            Ok(event) if changes_files(event.kind) => event,
            Ok(_) => return,
            Err(error) => {
                self.reset = true;
                self.error = Some(error.to_string().chars().take(512).collect());
                self.paths.clear();
                self.bytes = 0;
                return;
            }
        };
        self.reset |= event.need_rescan();
        if self.reset {
            self.paths.clear();
            self.bytes = 0;
            return;
        }
        let replan = matches!(
            event.kind,
            EventKind::Any
                | EventKind::Create(
                    notify::event::CreateKind::Folder
                        | notify::event::CreateKind::Any
                        | notify::event::CreateKind::Other
                )
                | EventKind::Remove(
                    notify::event::RemoveKind::Folder
                        | notify::event::RemoveKind::Any
                        | notify::event::RemoveKind::Other
                )
                | EventKind::Modify(notify::event::ModifyKind::Name(_))
        );
        for path in event.paths {
            if let Some(prior) = self.paths.get_mut(&path) {
                *prior |= replan;
                continue;
            }
            self.bytes += path.as_os_str().as_encoded_bytes().len();
            if self.bytes > PATH_BYTES {
                self.reset = true;
                self.paths.clear();
                self.bytes = 0;
                return;
            }
            self.paths.insert(path, replan);
        }
    }
}

pub(super) struct Dirty {
    raw: Raw,
}

impl Dirty {
    pub fn error(&self) -> Option<&str> {
        self.raw.error.as_deref()
    }
}

pub(super) struct Change {
    pub dirs: Vec<String>,
    pub reset: bool,
    pub replan: bool,
}

/// Filesystem facts prepared by an initialization worker, not coverage state.
pub(super) struct Plan {
    asked: String,
    root: PathBuf,
    exists: bool,
    workspace: bool,
    manual: bool,
    control: Vec<PathBuf>,
    aliases: Vec<PathBuf>,
    targets: BTreeMap<Target, bool>,
    pub git: Option<crate::git::GitWatchPlan>,
    pub partial: Option<String>,
}

impl Plan {
    pub fn registered(&mut self) {
        self.targets.clear();
    }
    pub fn prepare(scope: &Scope) -> Result<Self, String> {
        let (asked, workspace) = match scope {
            Scope::Entries { path } => (path, false),
            Scope::Workspace { root } => (root, true),
        };
        let path = Path::new(asked);
        if !path.is_absolute() {
            return Err("Directory watch requires an absolute path on its holding machine".into());
        }
        let root = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let mut plan = Self {
            asked: asked.clone(),
            root,
            exists: path.is_dir(),
            workspace,
            manual: cfg!(target_os = "linux"),
            control: Vec::new(),
            aliases: Vec::new(),
            targets: BTreeMap::new(),
            git: None,
            partial: None,
        };
        for ancestor in path.ancestors() {
            if std::fs::symlink_metadata(ancestor).is_ok_and(|m| m.file_type().is_symlink()) {
                if plan.aliases.len() >= 64 {
                    plan.partial = Some("Directory watch alias depth exceeds its budget".into());
                    break;
                }
                if let (Some(parent), Some(name)) = (ancestor.parent(), ancestor.file_name()) {
                    plan.aliases.push(canonical_spelling(parent).join(name));
                    plan.add_nearest(parent, true)?;
                }
            }
        }
        if workspace {
            match crate::git::watch_plan(path) {
                Ok(git) => plan.git = git,
                Err(error) => plan.partial = Some(error),
            }
            if let Some(git) = &plan.git {
                plan.control = git
                    .directories
                    .iter()
                    .map(|dir| canonical_spelling(dir))
                    .collect();
            }
            for dir in plan.control.clone() {
                plan.add_nearest(&dir, true)?;
            }
        }
        if let Some(parent) = plan.root.parent().map(Path::to_owned) {
            plan.add_nearest(&parent, true)?;
        }
        if !plan.root.is_dir() {
            plan.partial = Some("The requested watch directory does not exist".into());
            let root = plan.root.clone();
            plan.add_nearest(&root, true)?;
        } else if !workspace {
            let root = plan.root.clone();
            plan.add_nearest(&root, true)?;
        } else if plan.manual {
            let root = plan.root.clone();
            plan.add_nearest(&root, true)?;
        } else {
            plan.targets.insert(
                Target {
                    path: plan.root.clone(),
                    recursive: true,
                },
                false,
            );
        }
        if crate::keeper::on_network_fs(path) {
            plan.partial = Some("This filesystem cannot report writes made by another host".into());
        }
        Ok(plan)
    }

    pub fn still_current(&self) -> Result<bool, String> {
        let path = Path::new(&self.asked);
        let root = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let aliases: Vec<_> = path
            .ancestors()
            .filter(|path| {
                std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
            })
            .filter_map(|path| Some(canonical_spelling(path.parent()?).join(path.file_name()?)))
            .take(64)
            .collect();
        Ok(root == self.root
            && aliases == self.aliases
            && path.is_dir() == self.exists
            && (!self.workspace || crate::git::watch_plan(path)? == self.git))
    }

    #[cfg(test)]
    pub fn use_manual_tree(&mut self) {
        self.manual = true;
        self.targets.remove(&Target {
            path: self.root.clone(),
            recursive: true,
        });
        self.targets.insert(
            Target {
                path: self.root.clone(),
                recursive: false,
            },
            true,
        );
    }

    pub fn expand_tree(&mut self, install: impl FnMut(&Path) -> Result<(), String>) {
        if self.workspace && self.exists && self.manual {
            self.walk_directories(install);
        }
    }

    fn add_nearest(&mut self, wanted: &Path, critical: bool) -> Result<(), String> {
        let mut path = wanted;
        while !path.is_dir() {
            path = path
                .parent()
                .ok_or("No existing parent can observe the requested directory")?;
        }
        let target = Target {
            path: std::fs::canonicalize(path).map_err(|e| e.to_string())?,
            recursive: false,
        };
        self.targets
            .entry(target)
            .and_modify(|prior| *prior |= critical)
            .or_insert(critical);
        Ok(())
    }

    fn walk_directories(&mut self, mut install: impl FnMut(&Path) -> Result<(), String>) {
        let mut pending = vec![self.root.clone()];
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while let Some(path) = pending.pop() {
            if self.targets.len() >= DIRECTORIES || std::time::Instant::now() >= deadline {
                self.partial =
                    Some("Directory watch initialization reached its tree budget".into());
                return;
            }
            self.targets
                .entry(Target {
                    path: path.clone(),
                    recursive: false,
                })
                .or_insert(false);
            if let Err(error) = install(&path) {
                self.partial = Some(error);
                return;
            }
            let entries = match std::fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(error) => {
                    self.partial = Some(error.to_string());
                    continue;
                }
            };
            for entry in entries {
                if std::time::Instant::now() >= deadline {
                    self.partial =
                        Some("Directory watch initialization reached its tree budget".into());
                    return;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        self.partial = Some(error.to_string());
                        continue;
                    }
                };
                match entry.file_type() {
                    Ok(kind) if kind.is_dir() => {
                        if self.targets.len() + pending.len() >= DIRECTORIES {
                            self.partial = Some(
                                "Directory watch initialization reached its tree budget".into(),
                            );
                        } else {
                            pending.push(entry.path());
                        }
                    }
                    Ok(_) => {}
                    Err(error) => self.partial = Some(error.to_string()),
                }
            }
        }
    }

    pub fn change(&self, dirty: &Dirty) -> Option<Change> {
        if dirty.raw.reset {
            return Some(Change {
                dirs: Vec::new(),
                reset: true,
                replan: true,
            });
        }
        let mut dirs = BTreeSet::new();
        let mut changed = false;
        let mut reset = false;
        let mut replan = false;
        for (path, identity_changed) in &dirty.raw.paths {
            if *identity_changed
                && self
                    .aliases
                    .iter()
                    .any(|alias| alias == path || alias.starts_with(path))
            {
                changed = true;
                reset = true;
                replan = true;
                continue;
            }
            if *identity_changed && self.root.starts_with(path) && path != &self.root {
                changed = true;
                reset = true;
                replan = true;
                continue;
            }
            if path == &self.root || (self.workspace && path.starts_with(&self.root)) {
                changed = true;
                replan |=
                    *identity_changed && (!self.workspace || path == &self.root || self.manual);
                replan |= self.workspace
                    && (path == &self.root.join(".git")
                        || self
                            .control
                            .iter()
                            .filter(|dir| *dir != &self.root)
                            .any(|dir| {
                                path == dir
                                    || path.parent() == Some(dir.as_path())
                                    || dir.starts_with(path)
                            }));
                if self.workspace {
                    let parent = path.parent().unwrap_or(path);
                    let parent = parent.strip_prefix(&self.root).unwrap_or(Path::new(""));
                    let asked = if parent.as_os_str().is_empty() {
                        PathBuf::from(&self.asked)
                    } else {
                        Path::new(&self.asked).join(parent)
                    };
                    if let Some(parent) = asked.to_str() {
                        dirs.insert(parent.to_owned());
                    } else {
                        reset = true;
                    }
                } else {
                    dirs.insert(self.asked.clone());
                }
            } else if !self.workspace && path.parent() == Some(self.root.as_path()) {
                changed = true;
                dirs.insert(self.asked.clone());
            } else if self.workspace
                && self.control.iter().any(|dir| {
                    path == dir || path.parent() == Some(dir.as_path()) || dir.starts_with(path)
                })
            {
                changed = true;
                replan = true;
            }
        }
        changed.then(|| Change {
            dirs: if reset {
                Vec::new()
            } else {
                dirs.into_iter().collect()
            },
            reset,
            replan,
        })
    }
}

fn canonical_spelling(path: &Path) -> PathBuf {
    for ancestor in path.ancestors() {
        if let Ok(canonical) = std::fs::canonicalize(ancestor) {
            return canonical.join(path.strip_prefix(ancestor).expect("path ancestor"));
        }
    }
    path.to_path_buf()
}

pub(super) struct Installed {
    pub partial: Option<String>,
    pub reset_sources: BTreeSet<u64>,
    pub partial_sources: BTreeSet<u64>,
}

struct Handle {
    users: HashMap<u64, (bool, bool)>,
    recursive: bool,
    active: bool,
}

/// A resident's physical notify handles. Lease and coverage decisions stay in watch.rs.
pub(super) struct Native {
    identity: String,
    revision: Arc<AtomicU64>,
    watcher: notify::RecommendedWatcher,
    raw: Arc<Mutex<Raw>>,
    handles: BTreeMap<PathBuf, Handle>,
}

impl Native {
    pub fn revision(&self) -> Arc<AtomicU64> {
        self.revision.clone()
    }
    #[cfg(test)]
    pub fn has_pending(&self) -> bool {
        let raw = self.raw.lock().unwrap_or_else(|p| p.into_inner());
        !raw.paths.is_empty() || raw.reset
    }
    #[cfg(test)]
    pub fn watches_directory(&self, path: &Path) -> bool {
        self.handles.get(path).is_some_and(|handle| handle.active)
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    #[cfg(test)]
    pub fn handle_count(&self) -> usize {
        self.handles.len()
    }
    pub fn new() -> Result<(Self, tokio::sync::mpsc::Receiver<()>), String> {
        let identity = crate::link::fresh_hex()?;
        let revision = Arc::new(AtomicU64::new(0));
        let raw = Arc::new(Mutex::new(Raw::default()));
        let (wake, rx) = tokio::sync::mpsc::channel(1);
        let heard = raw.clone();
        let changed = revision.clone();
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            #[cfg(test)]
            if std::env::var_os("KHOR_TEST_WATCH_TRACE").is_some() {
                match &event {
                    Ok(event) => eprintln!(
                        "watch-native kind={:?} rescan={} paths={:?}",
                        event.kind,
                        event.need_rescan(),
                        event.paths.iter().take(16).collect::<Vec<_>>()
                    ),
                    Err(error) => eprintln!("watch-native error={error}"),
                }
            }
            if event.as_ref().is_ok_and(|event| !changes_files(event.kind)) {
                return;
            }
            let _ = changed.fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            });
            heard.lock().unwrap_or_else(|p| p.into_inner()).take(event);
            let _ = wake.try_send(());
        })
        .map_err(|e| e.to_string())?;
        Ok((
            Self {
                identity,
                revision,
                watcher,
                raw,
                handles: BTreeMap::new(),
            },
            rx,
        ))
    }

    pub fn install(&mut self, source: u64, plan: &Plan) -> Installed {
        self.install_targets(source, &plan.targets, plan.partial.clone())
    }

    pub fn install_directory(&mut self, source: u64, path: &Path) -> Installed {
        let targets = BTreeMap::from([(
            Target {
                path: path.to_owned(),
                recursive: false,
            },
            false,
        )]);
        self.install_targets(source, &targets, None)
    }

    fn install_targets(
        &mut self,
        source: u64,
        wanted: &BTreeMap<Target, bool>,
        partial: Option<String>,
    ) -> Installed {
        let mut installed = Installed {
            partial,
            reset_sources: BTreeSet::new(),
            partial_sources: BTreeSet::new(),
        };
        let mut targets: Vec<_> = wanted.iter().collect();
        targets.sort_by_key(|(_, critical)| !**critical);
        for (target, critical) in targets {
            if cfg!(target_os = "macos") && target.path.to_str().is_none() {
                installed.partial =
                    Some("This watcher cannot register a non-UTF-8 directory".into());
                continue;
            }
            if let Some(handle) = self.handles.get_mut(&target.path) {
                let previous = handle.users.get(&source).copied().unwrap_or_default();
                let desired = (previous.0 || *critical, previous.1 || target.recursive);
                let recursive = desired.1 || handle.users.values().any(|(_, recursive)| *recursive);
                if !handle.active || (recursive && !handle.recursive) {
                    installed.reset_sources.extend(handle.users.keys());
                    if let Err(error) = retired(self.watcher.unwatch(&target.path)) {
                        installed.partial = Some(error);
                        installed.partial_sources.extend(handle.users.keys());
                        handle.active = false;
                        handle.users.insert(source, desired);
                        continue;
                    }
                    handle.active = false;
                    let mode = if recursive {
                        RecursiveMode::Recursive
                    } else {
                        RecursiveMode::NonRecursive
                    };
                    match self.watcher.watch(&target.path, mode) {
                        Ok(()) => {
                            handle.recursive = recursive;
                            handle.active = true;
                        }
                        Err(error) => {
                            installed.partial = Some(error.to_string());
                            installed.partial_sources.extend(handle.users.keys());
                        }
                    }
                }
                handle.users.insert(source, desired);
                continue;
            }
            if self.handles.len() >= DIRECTORIES && *critical {
                let evict = self.handles.iter().find_map(|(target, handle)| {
                    (!handle.users.values().any(|(critical, _)| *critical)).then(|| target.clone())
                });
                if let Some(evict) = evict {
                    match retired(self.watcher.unwatch(&evict)) {
                        Ok(()) => {
                            let handle = self.handles.remove(&evict).expect("selected handle");
                            installed.partial_sources.extend(handle.users.keys());
                        }
                        Err(error) => installed.partial = Some(error),
                    }
                }
            }
            if self.handles.len() >= DIRECTORIES {
                installed.partial = Some("The resident directory watch pool is full".into());
                continue;
            }
            let mode = if target.recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            match self.watcher.watch(&target.path, mode) {
                Ok(()) => {
                    self.handles.insert(
                        target.path.clone(),
                        Handle {
                            users: HashMap::from([(source, (*critical, target.recursive))]),
                            recursive: target.recursive,
                            active: true,
                        },
                    );
                }
                Err(error) => installed.partial = Some(error.to_string()),
            }
        }
        installed
    }

    pub fn remove(&mut self, source: u64) -> Installed {
        let mut removed = Installed {
            partial: None,
            reset_sources: BTreeSet::new(),
            partial_sources: BTreeSet::new(),
        };
        self.handles.retain(|target, handle| {
            handle.users.remove(&source);
            if handle.users.is_empty() {
                match retired(self.watcher.unwatch(target)) {
                    Ok(()) => false,
                    Err(error) => {
                        removed.partial = Some(error);
                        true
                    }
                }
            } else {
                let recursive = handle.users.values().any(|(_, recursive)| *recursive);
                if !handle.active || handle.recursive != recursive {
                    removed.reset_sources.extend(handle.users.keys());
                    if let Err(error) = retired(self.watcher.unwatch(target)) {
                        removed.partial = Some(error);
                        removed.partial_sources.extend(handle.users.keys());
                        handle.active = false;
                        return true;
                    }
                    handle.active = false;
                    let mode = if recursive {
                        RecursiveMode::Recursive
                    } else {
                        RecursiveMode::NonRecursive
                    };
                    match self.watcher.watch(target, mode) {
                        Ok(()) => {
                            handle.recursive = recursive;
                            handle.active = true;
                        }
                        Err(_) => removed.partial_sources.extend(handle.users.keys()),
                    }
                }
                true
            }
        });
        removed
    }

    pub fn drain(&self) -> Dirty {
        Dirty {
            raw: std::mem::take(&mut *self.raw.lock().unwrap_or_else(|p| p.into_inner())),
        }
    }
}

fn retired(result: notify::Result<()>) -> Result<(), String> {
    match result {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind,
                notify::ErrorKind::WatchNotFound | notify::ErrorKind::PathNotFound
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_overflow_is_one_reset_and_reads_do_not_wake_it() {
        let mut raw = Raw::default();
        raw.take(Ok(notify::Event::new(EventKind::Access(
            notify::event::AccessKind::Read,
        ))
        .add_path("/root/a".into())));
        assert!(raw.paths.is_empty());
        assert!(!raw.reset);
        raw.take(Ok(notify::Event::new(EventKind::Create(
            notify::event::CreateKind::File,
        ))
        .add_path(PathBuf::from("x".repeat(PATH_BYTES + 1)))));
        assert!(raw.reset);
        assert_eq!(raw.bytes, 0);
        assert!(raw.paths.is_empty());
        raw.take(Ok(notify::Event::new(EventKind::Create(
            notify::event::CreateKind::File,
        ))
        .add_path("/root/later".into())));
        assert!(
            raw.paths.is_empty(),
            "an overflow cannot accumulate a second unbounded burst"
        );
    }

    #[test]
    fn aliases_keep_requested_spelling_and_both_rename_parents() {
        let plan = Plan {
            asked: "/alias/root".into(),
            root: "/real/root".into(),
            exists: true,
            workspace: true,
            manual: false,
            control: Vec::new(),
            aliases: Vec::new(),
            targets: BTreeMap::new(),
            git: None,
            partial: None,
        };
        let mut raw = Raw::default();
        raw.take(Ok(notify::Event::new(EventKind::Modify(
            notify::event::ModifyKind::Name(notify::event::RenameMode::Both),
        ))
        .add_path("/real/root/from/a.txt".into())
        .add_path("/real/root/to/a.txt".into())));
        let change = plan.change(&Dirty { raw }).unwrap();
        assert_eq!(change.dirs, vec!["/alias/root/from", "/alias/root/to"]);
        assert!(!change.reset);
        let mut raw = Raw::default();
        raw.take(Ok(notify::Event::new(EventKind::Create(
            notify::event::CreateKind::File,
        ))
        .add_path("/real/root/file.txt".into())));
        assert_eq!(
            plan.change(&Dirty { raw }).unwrap().dirs,
            vec!["/alias/root"],
            "root parent spelling must not gain a trailing separator"
        );
    }

    #[test]
    fn an_ancestor_removal_and_notify_rescan_reset_the_whole_scope() {
        let plan = Plan {
            asked: "/root/child".into(),
            root: "/root/child".into(),
            exists: true,
            workspace: false,
            manual: false,
            control: Vec::new(),
            aliases: Vec::new(),
            targets: BTreeMap::new(),
            git: None,
            partial: None,
        };
        let mut raw = Raw::default();
        raw.take(Ok(notify::Event::new(EventKind::Remove(
            notify::event::RemoveKind::Folder,
        ))
        .add_path("/root".into())));
        let change = plan.change(&Dirty { raw }).unwrap();
        assert!(change.reset && change.replan);
        let mut raw = Raw::default();
        raw.take(Err(notify::Error::generic("provider lost events")));
        let dirty = Dirty { raw };
        assert_eq!(dirty.error(), Some("provider lost events"));
        assert!(plan.change(&dirty).unwrap().reset);
    }
}
