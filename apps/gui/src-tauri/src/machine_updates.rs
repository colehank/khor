use khor_gui_core::updates::{
    self, MachineUpdateJob, MachineUpdatePhase as Phase, MachineUpdateQueue, MachineUpdateSource,
    UpdateChannel,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Emitter, Manager};

const LIMIT: usize = 64;
const BYTES: u64 = 128 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Queue {
    schema: u8,
    revision: u32,
    jobs: Vec<MachineUpdateJob>,
}

impl Queue {
    fn restore(mut self) -> Result<Self, String> {
        if self.schema != 1 || self.jobs.len() > LIMIT {
            return Err("unsupported machine update queue".into());
        }
        let mut seen = std::collections::HashSet::new();
        for job in &mut self.jobs {
            if !valid_machine(&job.machine)
                || !seen.insert(job.machine.clone())
                || job.request.is_empty()
                || job.request.len() > 128
                || job.failure.as_ref().is_some_and(|s| s.len() > 1024)
                || job.target.as_ref().is_some_and(|s| s.len() > 256)
            {
                return Err("invalid machine update identity".into());
            }
            job.phase = match job.phase {
                Phase::Checking => Phase::Queued,
                Phase::Installing => Phase::Confirming,
                phase => phase,
            };
        }
        Ok(self)
    }

    fn update(&mut self, result: MachineUpdateJob) -> bool {
        let Some(job) = self
            .jobs
            .iter_mut()
            .find(|job| job.machine == result.machine && job.request == result.request)
        else {
            return false;
        };
        *job = result;
        true
    }
}

fn valid_machine(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

pub(crate) struct MachineUpdates {
    root: PathBuf,
    queue: Mutex<Option<Queue>>,
    active: AtomicBool,
}

impl MachineUpdates {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            queue: Mutex::new(None),
            active: AtomicBool::new(false),
        }
    }

    fn path(&self) -> PathBuf {
        self.root.join(".khor/machine-updates.json")
    }

    fn with_queue<T>(
        &self,
        action: impl FnOnce(&mut Queue) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut held = self
            .queue
            .lock()
            .map_err(|_| "machine update queue is unavailable")?;
        if held.is_none() {
            let queue = match File::open(self.path()) {
                Ok(file) => {
                    let mut bytes = Vec::new();
                    file.take(BYTES + 1)
                        .read_to_end(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    if bytes.len() as u64 > BYTES {
                        return Err("machine update queue exceeds its limit".into());
                    }
                    serde_json::from_slice::<Queue>(&bytes)
                        .map_err(|e| e.to_string())?
                        .restore()?
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Queue {
                    schema: 1,
                    revision: 0,
                    jobs: vec![],
                },
                Err(e) => return Err(e.to_string()),
            };
            *held = Some(queue);
        }
        action(held.as_mut().ok_or("machine update queue did not load")?)
    }

    fn save(&self, queue: &mut Queue) -> Result<(), String> {
        queue.revision = queue
            .revision
            .checked_add(1)
            .ok_or("machine update revision exhausted")?;
        let path = self.path();
        let parent = path.parent().ok_or("machine update queue has no parent")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec(&*queue).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > BYTES {
            return Err("machine update queue exceeds its limit".into());
        }
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(&path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> Result<MachineUpdateQueue, String> {
        self.with_queue(|queue| {
            Ok(MachineUpdateQueue {
                revision: queue.revision,
                jobs: queue.jobs.clone(),
            })
        })
    }

    pub(crate) fn status(&self) -> Result<Vec<MachineUpdateJob>, String> {
        self.with_queue(|queue| Ok(queue.jobs.clone()))
    }

    pub(crate) fn enqueue(
        &self,
        machines: Vec<String>,
        channel: Option<UpdateChannel>,
    ) -> Result<Vec<MachineUpdateJob>, String> {
        if machines.is_empty()
            || machines.len() > LIMIT
            || machines.iter().any(|m| !valid_machine(m))
        {
            return Err("select paired machine identities before updating".into());
        }
        self.with_queue(|queue| {
            let mut next = Queue {
                schema: 1,
                revision: queue.revision,
                jobs: queue.jobs.clone(),
            };
            for machine in machines {
                if next.jobs.iter().any(|j| {
                    j.machine == machine
                        && matches!(
                            j.phase,
                            Phase::Checking | Phase::Installing | Phase::Confirming
                        )
                }) {
                    continue;
                }
                next.jobs.retain(|j| j.machine != machine);
                next.jobs.push(MachineUpdateJob {
                    machine,
                    request: khor_node::link::fresh_hex()?,
                    source: None,
                    phase: Phase::Queued,
                    channel,
                    target: None,
                    failure: None,
                });
            }
            if next.jobs.len() > LIMIT {
                return Err("machine update queue is full".into());
            }
            self.save(&mut next)?;
            *queue = next;
            Ok(queue.jobs.clone())
        })
    }

    pub(crate) fn cancel_waiting(
        &self,
        machine: &str,
        request: &str,
    ) -> Result<Vec<MachineUpdateJob>, String> {
        self.with_queue(|queue| {
            let mut next = Queue {
                schema: 1,
                revision: queue.revision,
                jobs: queue.jobs.clone(),
            };
            next.jobs.retain(|j| {
                !(j.machine == machine
                    && j.request == request
                    && matches!(
                        j.phase,
                        Phase::Queued | Phase::Waiting | Phase::Complete | Phase::Failed
                    ))
            });
            self.save(&mut next)?;
            *queue = next;
            Ok(queue.jobs.clone())
        })
    }

    fn record(&self, app: &tauri::AppHandle, mut job: MachineUpdateJob) -> Result<(), String> {
        if let Some(failure) = &mut job.failure {
            let mut end = failure.len().min(1024);
            while !failure.is_char_boundary(end) {
                end -= 1;
            }
            failure.truncate(end);
        }
        let jobs = self.with_queue(|queue| {
            let mut next = Queue {
                schema: 1,
                revision: queue.revision,
                jobs: queue.jobs.clone(),
            };
            if !next.update(job) {
                return Err("machine update choice was replaced or cancelled".into());
            }
            self.save(&mut next)?;
            *queue = next;
            Ok(MachineUpdateQueue {
                revision: queue.revision,
                jobs: queue.jobs.clone(),
            })
        })?;
        let _ = app.emit("machine-updates:changed", jobs);
        Ok(())
    }

    pub(crate) fn kick(app: &tauri::AppHandle) {
        let manager = app.state::<Arc<Self>>().inner().clone();
        if manager
            .active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            struct Reset(Arc<MachineUpdates>);
            impl Drop for Reset {
                fn drop(&mut self) {
                    self.0.active.store(false, Ordering::SeqCst);
                }
            }
            let _reset = Reset(manager.clone());
            let Ok(jobs) = manager.status() else {
                return;
            };
            for job in jobs {
                if matches!(
                    job.phase,
                    Phase::Queued
                        | Phase::Waiting
                        | Phase::Checking
                        | Phase::Installing
                        | Phase::Confirming
                ) {
                    let _ = manager.execute(&app, job).await;
                }
            }
        });
    }

    async fn execute(
        &self,
        app: &tauri::AppHandle,
        mut job: MachineUpdateJob,
    ) -> Result<(), String> {
        // A cancelled or replaced choice cannot be revived by a late network reply.
        if !self
            .status()?
            .iter()
            .any(|j| j.machine == job.machine && j.request == job.request)
        {
            return Ok(());
        }
        let confirming = matches!(job.phase, Phase::Confirming | Phase::Installing);
        if !confirming {
            job.phase = Phase::Checking;
            self.record(app, job.clone())?;
        }
        let info = match updates::machine_installation(&self.root, &job.machine).await {
            Ok(info) => info,
            Err(why) => {
                job.phase = if confirming {
                    Phase::Confirming
                } else {
                    Phase::Waiting
                };
                job.failure = Some(why);
                return self.record(app, job);
            }
        };
        job.source = Some(info.source);
        if info.source != MachineUpdateSource::Cli {
            job.phase = Phase::Failed;
            job.failure = Some(
                match info.source {
                    MachineUpdateSource::App => {
                        "update from About in this machine's Khor application"
                    }
                    MachineUpdateSource::Development => {
                        "development builds are updated by their builder"
                    }
                    _ => "update through this machine's external installer",
                }
                .into(),
            );
            return self.record(app, job);
        }
        if confirming {
            job.phase = if info.failed == job.target && job.target.is_some() {
                Phase::Failed
            } else if info.selected == job.target && info.service_ready {
                Phase::Complete
            } else if info.selected == job.target {
                Phase::Confirming
            } else {
                Phase::Failed
            };
            if job.phase == Phase::Failed && job.failure.is_none() {
                job.failure =
                    Some("the requested version was not confirmed; check before retrying".into());
            }
            if job.phase == Phase::Complete {
                job.failure = None;
            }
            return self.record(app, job);
        }
        let planned =
            match updates::machine_update(&self.root, &job.machine, true, job.channel).await {
                Ok(plan) => plan,
                Err(why) => {
                    job.phase = Phase::Failed;
                    job.failure = Some(why);
                    return self.record(app, job);
                }
            };
        job.target = planned.available;
        if job.target.is_none() {
            job.phase = Phase::Complete;
            job.failure = None;
            return self.record(app, job);
        }
        job.phase = Phase::Installing;
        job.failure = None;
        self.record(app, job.clone())?;
        match updates::machine_update(&self.root, &job.machine, false, job.channel).await {
            Ok(result) => {
                if result.available.is_some() {
                    job.target = result.available;
                }
                job.phase = if result.enabled {
                    Phase::Confirming
                } else {
                    Phase::Complete
                };
            }
            Err(why) => {
                job.phase = Phase::Confirming;
                job.failure = Some(why);
            }
        }
        self.record(app, job)
    }
}

pub(crate) fn start(app: &tauri::AppHandle) {
    app.manage(Arc::new(MachineUpdates::new(
        khor_node::Node::root_from_env(),
    )));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            MachineUpdates::kick(&app);
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn job(phase: Phase) -> MachineUpdateJob {
        MachineUpdateJob {
            machine: "a".repeat(64),
            request: "owned-choice".into(),
            source: None,
            phase,
            channel: None,
            target: Some("v0.2.0".into()),
            failure: None,
        }
    }

    #[test]
    fn interrupted_installation_is_reconciled_and_never_replayed_as_a_fresh_install() {
        let restored = Queue {
            schema: 1,
            revision: 0,
            jobs: vec![job(Phase::Installing)],
        }
        .restore()
        .unwrap();
        assert_eq!(restored.jobs[0].phase, Phase::Confirming);
        let restored = Queue {
            schema: 1,
            revision: 0,
            jobs: vec![job(Phase::Checking)],
        }
        .restore()
        .unwrap();
        assert_eq!(restored.jobs[0].phase, Phase::Queued);
    }
    #[test]
    fn replaced_requests_reject_old_results_and_invalid_target_identities() {
        let mut queue = Queue {
            schema: 1,
            revision: 0,
            jobs: vec![job(Phase::Queued)],
        };
        let mut old = job(Phase::Complete);
        old.request = "retired-choice".into();
        assert!(!queue.update(old));
        assert_eq!(queue.jobs[0].phase, Phase::Queued);
        queue.jobs[0].machine = "renamable display title".into();
        assert!(queue.restore().is_err());
    }
    #[test]
    fn durable_choices_survive_restart_and_cancelling_waiting_does_not_cancel_an_install() {
        let root = tempfile::tempdir().unwrap();
        let manager = MachineUpdates::new(root.path().into());
        let jobs = manager.enqueue(vec!["a".repeat(64)], None).unwrap();
        let restored = MachineUpdates::new(root.path().into());
        assert_eq!(restored.status().unwrap()[0].request, jobs[0].request);
        let mut installing = jobs[0].clone();
        installing.phase = Phase::Installing;
        restored
            .with_queue(|queue| {
                queue.update(installing);
                restored.save(queue)
            })
            .unwrap();
        assert_eq!(
            restored
                .cancel_waiting(&jobs[0].machine, &jobs[0].request)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(restored.status().unwrap()[0].phase, Phase::Installing);
    }
}
