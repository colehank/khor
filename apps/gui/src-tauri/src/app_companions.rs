use khor_gui_core::updates::{CompanionUpdate, UpdatePhase};
use khor_node::updates::ManagedCli;
use khor_update::{
    Channel, Component, Offer, Platform, ReleaseIdentity, ReleaseManifest, VerifiedRelease,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u8,
    target: Option<ReleaseIdentity>,
    selected: BTreeSet<String>,
    outcomes: Vec<CompanionUpdate>,
}

#[derive(Clone)]
pub(crate) struct Entry {
    pub cli: ManagedCli,
    pub status: CompanionUpdate,
}

#[derive(Clone)]
pub(crate) struct Plan {
    root: PathBuf,
    previous: Record,
    entries: Vec<Entry>,
    target: Option<ReleaseIdentity>,
}

impl Plan {
    pub(crate) fn discover(root: &Path) -> Result<Self, String> {
        Self::from_installations(root, khor_node::updates::managed_clis()?)
    }

    fn from_installations(root: &Path, installed: Vec<ManagedCli>) -> Result<Self, String> {
        let previous = read_record(root)?;
        let entries = installed
            .into_iter()
            .map(|cli| {
                let id = cli
                    .source
                    .to_str()
                    .ok_or("CLI installation path cannot be displayed")?
                    .to_owned();
                let old = previous.outcomes.iter().find(|status| status.id == id);
                let current = cli.current.tag.as_str().to_owned();
                let status = CompanionUpdate {
                    selected: previous.selected.contains(&id),
                    id,
                    path: cli.launcher.to_string_lossy().into_owned(),
                    current,
                    available: None,
                    phase: if old.is_some() && previous.target.as_ref() == Some(&cli.current) {
                        UpdatePhase::Complete
                    } else {
                        UpdatePhase::Idle
                    },
                    failure: None,
                };
                Ok(Entry { cli, status })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let target = previous.target.clone();
        Ok(Self {
            root: root.to_path_buf(),
            previous,
            entries,
            target,
        })
    }

    pub(crate) fn oldest(&self, app: &ReleaseIdentity) -> ReleaseIdentity {
        self.entries
            .iter()
            .map(|entry| &entry.cli.current)
            .chain(std::iter::once(app))
            .min_by_key(|id| id.tag.version())
            .expect("application identity exists")
            .clone()
    }

    pub(crate) fn select_release(
        &mut self,
        release: &VerifiedRelease,
        channel: Channel,
    ) -> Result<(), String> {
        self.select_manifest(release.manifest(), channel)
    }

    fn select_manifest(
        &mut self,
        manifest: &ReleaseManifest,
        channel: Channel,
    ) -> Result<(), String> {
        let target = &manifest.identity;
        self.target = Some(target.clone());
        for entry in &mut self.entries {
            let available = manifest.newer_than(&entry.cli.current, channel)?
                && manifest
                    .artifact(Component::Cli, Platform::current()?)
                    .is_some();
            entry.status.available = available.then(|| target.tag.as_str().to_owned());
            if !available {
                continue;
            }
            entry.status.phase = UpdatePhase::Available;
            if self.previous.target.as_ref() == Some(target) {
                if let Some(old) = self
                    .previous
                    .outcomes
                    .iter()
                    .find(|s| s.id == entry.status.id)
                {
                    if matches!(old.phase, UpdatePhase::Failed | UpdatePhase::Installing) {
                        entry.status.phase = UpdatePhase::Failed;
                        entry.status.failure = Some(old.failure.clone().unwrap_or_else(|| {
                            "CLI update was interrupted; check and retry".into()
                        }));
                    }
                }
            }
            if entry.cli.failed.as_ref() == Some(target) {
                entry.status.phase = UpdatePhase::Failed;
                entry.status.failure = Some("CLI startup failed; previous version restored".into());
            }
        }
        Ok(())
    }

    pub(crate) fn statuses(&self) -> Vec<CompanionUpdate> {
        self.entries
            .iter()
            .map(|entry| entry.status.clone())
            .collect()
    }

    pub(crate) fn select(&mut self, ids: &[String]) -> Result<(), String> {
        let selected = ids.iter().cloned().collect::<BTreeSet<_>>();
        if selected.len() != ids.len()
            || ids.len() > 64
            || selected
                .iter()
                .any(|id| !self.entries.iter().any(|entry| &entry.status.id == id))
        {
            return Err("CLI choices differ from the reviewed installations; check again".into());
        }
        for entry in &mut self.entries {
            entry.status.selected = selected.contains(&entry.status.id);
        }
        self.persist()
    }

    pub(crate) fn pending(&self) -> Vec<Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.status.selected && entry.status.available.is_some())
            .cloned()
            .collect()
    }

    pub(crate) fn offer(
        &self,
        release: &VerifiedRelease,
        channel: Channel,
    ) -> Result<Option<Offer>, String> {
        let Some(entry) = self.pending().first().cloned() else {
            return Ok(None);
        };
        release.select(
            Component::Cli,
            Platform::current()?,
            &entry.cli.current,
            channel,
            None,
        )
    }

    pub(crate) fn automatic_blocked(&self) -> bool {
        self.pending()
            .iter()
            .any(|entry| entry.status.phase == UpdatePhase::Failed)
    }

    pub(crate) fn progress(
        &mut self,
        id: &str,
        phase: UpdatePhase,
        result: Option<&Result<ManagedCli, String>>,
    ) -> Result<(), String> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.status.id == id)
            .ok_or("CLI left the update plan")?;
        entry.status.phase = phase;
        entry.status.failure = None;
        if let Some(result) = result {
            match result {
                Ok(cli) => {
                    entry.cli = cli.clone();
                    entry.status.current = cli.current.tag.as_str().to_owned();
                    entry.status.available = None;
                }
                Err(error) => entry.status.failure = Some(error.clone()),
            }
        }
        self.persist()
    }

    fn persist(&self) -> Result<(), String> {
        let record = Record {
            schema: 1,
            target: self.target.clone(),
            selected: self
                .entries
                .iter()
                .filter(|entry| entry.status.selected)
                .map(|entry| entry.status.id.clone())
                .collect(),
            outcomes: self.statuses(),
        };
        let mut temporary =
            tempfile::NamedTempFile::new_in(&self.root).map_err(|e| e.to_string())?;
        temporary
            .write_all(&serde_json::to_vec(&record).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        temporary
            .persist(self.root.join("components.json"))
            .map_err(|e| e.to_string())?;
        File::open(&self.root)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())
    }
}

fn read_record(root: &Path) -> Result<Record, String> {
    let mut bytes = Vec::new();
    let file = match File::open(root.join("components.json")) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Record::default()),
        Err(error) => return Err(error.to_string()),
    };
    file.take(256 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 256 * 1024 {
        return Err("component update plan exceeds its limit".into());
    }
    let record: Record = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if record.schema != 1 || record.selected.len() > 64 || record.outcomes.len() > 64 {
        return Err("unsupported component update plan".into());
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, ReleaseManifest, Vec<ManagedCli>) {
        let directory = tempfile::tempdir().unwrap();
        let mut manifest: ReleaseManifest = serde_json::from_slice(include_bytes!(
            "../../../../crates/update/tests/fixtures/release.json"
        ))
        .unwrap();
        manifest.artifacts[0].platform = Platform::current().unwrap();
        let mut old = manifest.identity.clone();
        old.tag = "v0.1.0".to_owned().try_into().unwrap();
        let entries = ["one", "two"]
            .map(|name| ManagedCli {
                source: directory.path().join(name).join("current"),
                launcher: directory.path().join(name).join("bin/khor"),
                current: old.clone(),
                failed: None,
            })
            .to_vec();
        (directory, manifest, entries)
    }

    #[test]
    fn reviewed_choices_and_partial_results_survive_restart_without_replaying_successes() {
        let (directory, manifest, mut installed) = fixture();
        let mut plan = Plan::from_installations(directory.path(), installed.clone()).unwrap();
        plan.select_manifest(&manifest, Channel::Stable).unwrap();
        assert!(
            plan.pending().is_empty(),
            "finding a CLI must not authorize taking it over"
        );
        let ids = plan
            .statuses()
            .iter()
            .map(|s| s.id.clone())
            .collect::<Vec<_>>();
        assert!(
            plan.select(&["unregistered-package-manager-installation".into()])
                .is_err()
        );
        assert!(!directory.path().join("components.json").exists());
        plan.select(&ids).unwrap();
        plan.progress(&ids[0], UpdatePhase::Installing, None)
            .unwrap();
        installed[0].current = manifest.identity.clone();
        plan.progress(
            &ids[0],
            UpdatePhase::Complete,
            Some(&Ok(installed[0].clone())),
        )
        .unwrap();
        plan.progress(
            &ids[1],
            UpdatePhase::Failed,
            Some(&Err("fixture failed before activation".into())),
        )
        .unwrap();
        drop(plan);
        let mut restored = Plan::from_installations(directory.path(), installed).unwrap();
        restored
            .select_manifest(&manifest, Channel::Stable)
            .unwrap();
        assert_eq!(restored.pending().len(), 1);
        assert_eq!(restored.pending()[0].status.id, ids[1]);
        assert_eq!(restored.statuses()[0].phase, UpdatePhase::Complete);
        assert!(
            restored.automatic_blocked(),
            "a failed component needs a manual retry"
        );
    }

    #[test]
    fn interrupted_component_is_reconciled_with_the_actual_selected_version() {
        let (directory, manifest, mut installed) = fixture();
        let mut plan = Plan::from_installations(directory.path(), installed.clone()).unwrap();
        plan.select_manifest(&manifest, Channel::Stable).unwrap();
        let id = plan.statuses()[0].id.clone();
        plan.select(&[id.clone()]).unwrap();
        plan.progress(&id, UpdatePhase::Installing, None).unwrap();
        installed[0].current = manifest.identity.clone();
        let mut restored = Plan::from_installations(directory.path(), installed).unwrap();
        restored
            .select_manifest(&manifest, Channel::Stable)
            .unwrap();
        assert_eq!(restored.statuses()[0].phase, UpdatePhase::Complete);
        assert!(restored.pending().is_empty());
        std::fs::write(directory.path().join("components.json"), b"broken record").unwrap();
        assert!(Plan::from_installations(directory.path(), Vec::new()).is_err());
    }
}
