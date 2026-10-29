//! Resident-owned directory observations. Leases own shared local notify or
//! authenticated remote streams; reads only consume bounded epoch/sequence logs.

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
mod native;
mod owned;

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use crate::proto::{self, Request, Response};
use owned::{Effects, Owned, Start, Ticket};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scope {
    Entries { path: String },
    Workspace { root: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageMode {
    Pending,
    Native,
    Polling,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Coverage {
    pub scope: u32,
    pub mode: CoverageMode,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Batch {
    pub cursor: String,
    pub reset: bool,
    pub dirs: Vec<String>,
    pub workspace: bool,
    pub coverage: Vec<Coverage>,
    pub retry_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Opened {
    pub id: String,
    pub machine_id: String,
    pub batch: Batch,
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn changes_files(kind: notify::EventKind) -> bool {
    !matches!(
        kind,
        notify::EventKind::Access(_)
            | notify::EventKind::Modify(notify::event::ModifyKind::Metadata(
                notify::event::MetadataKind::AccessTime
            ))
    )
}

pub const GATHER: Duration = Duration::from_millis(150);
const FRAME_BYTES: usize = 128 * 1024;

#[derive(Clone)]
pub(crate) enum Route {
    Here,
    Peer {
        node: Weak<crate::Node>,
        endpoint: iroh::Endpoint,
        target: crate::DeviceInfo,
    },
    Unavailable(String),
}

struct Job {
    ticket: Ticket,
    cancel: Arc<AtomicBool>,
    stop: tokio::sync::watch::Sender<bool>,
    active: bool,
    task: tokio::task::JoinHandle<()>,
}

#[derive(Default)]
struct Running {
    closed: bool,
    routes: HashMap<String, Route>,
    jobs: HashMap<u64, Job>,
    expiry: Option<tokio::task::JoinHandle<()>>,
    observer: Option<tokio::task::JoinHandle<()>>,
    retries: HashMap<u64, Instant>,
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    plans: HashMap<u64, Arc<native::Plan>>,
}

pub struct Watches {
    owned: Mutex<Owned>,
    running: Mutex<Running>,
    arrived: Arc<tokio::sync::Notify>,
    deadlines: Arc<tokio::sync::Notify>,
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    native: Arc<Mutex<Option<native::Native>>>,
    #[cfg(test)]
    test_manual_tree: AtomicBool,
    #[cfg(test)]
    test_walk_gate: Mutex<Option<WalkGate>>,
}

#[cfg(test)]
struct WalkGate {
    walked: tokio::sync::oneshot::Sender<()>,
    resume: std::sync::mpsc::Receiver<()>,
}

impl Watches {
    fn source(
        weak: Weak<Self>,
        start: Start,
        route: Route,
        cancel: Arc<AtomicBool>,
        stopped: tokio::sync::watch::Receiver<bool>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async move {
            match route {
                Route::Here => {
                    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
                    Self::local_source(weak, start, cancel).await;
                    #[cfg(not(any(
                        target_os = "macos",
                        target_os = "windows",
                        target_os = "linux"
                    )))]
                    Self::failed(
                        weak,
                        start.ticket,
                        false,
                        "This machine cannot observe directories".into(),
                    );
                }
                Route::Unavailable(reason) => Self::failed(weak, start.ticket, false, reason),
                Route::Peer {
                    node,
                    endpoint,
                    target,
                } => {
                    let ticket = start.ticket;
                    let mut established = false;
                    let result = tokio::select! {
                        _ = cancelled(stopped) => Ok(()),
                        result = Self::remote_source(&weak, start, node, endpoint, target, &mut established) => result,
                    };
                    if let Err(error) = result {
                        Self::failed(weak, ticket, established, error);
                    } else if let Some(watches) = weak.upgrade() {
                        if !established {
                            watches.completed(ticket, CoverageMode::Unavailable, None);
                        }
                        watches.finished(ticket);
                    }
                }
            }
        })
    }

    fn finished(&self, ticket: Ticket) {
        let mut running = self.running.lock().unwrap_or_else(|p| p.into_inner());
        if running
            .jobs
            .get(&ticket.source)
            .is_some_and(|job| job.ticket == ticket)
        {
            running.jobs.remove(&ticket.source);
        }
    }

    fn failed(weak: Weak<Self>, ticket: Ticket, established: bool, reason: String) {
        let Some(watches) = weak.upgrade() else {
            return;
        };
        if established {
            let _ = watches
                .owned
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .coverage(ticket.source, CoverageMode::Unavailable, Some(reason), true);
        } else {
            watches.completed(ticket, CoverageMode::Unavailable, Some(reason));
        }
        watches.finished(ticket);
        watches.arrived.notify_waiters();
    }

    async fn remote_source(
        weak: &Weak<Self>,
        start: Start,
        node: Weak<crate::Node>,
        endpoint: iroh::Endpoint,
        target: crate::DeviceInfo,
        established: &mut bool,
    ) -> Result<(), String> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        let connect = async {
            let node = node
                .upgrade()
                .ok_or("Directory watch resident stopped before dialing")?;
            let line = node
                .line_to(&endpoint, &target, crate::wires::Proto::Main)
                .await
                .map_err(|e| format!("Directory watch connection failed: {e:?}"))?;
            let conn = line.conn.clone();
            drop(node);
            let (mut send, recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
            send.write_all(&proto::encode(&Request::Watch { scope: start.scope })?)
                .await
                .map_err(|e| e.to_string())?;
            send.finish().map_err(|e| e.to_string())?;
            Ok::<_, String>((conn, recv))
        };
        let (_conn, mut recv) = tokio::time::timeout_at(deadline, connect)
            .await
            .map_err(|_| "Directory watch initialization timed out")??;
        let mut first = true;
        loop {
            let read = read_frame(&mut recv);
            let response = if *established {
                read.await?
            } else {
                tokio::time::timeout_at(deadline, read)
                    .await
                    .map_err(|_| "Directory watch initialization timed out")??
            };
            let batch = match response {
                Response::Watching { batch } if first => batch,
                Response::Changed { batch } if !first => batch,
                Response::Refused { why } => return Err(why),
                _ => return Err("Directory watch returned an unexpected frame".into()),
            };
            first = false;
            if serde_json::to_vec(&batch).map_err(|e| e.to_string())?.len() > FRAME_BYTES
                || batch.coverage.len() != 1
                || batch.coverage[0].scope != 0
            {
                return Err("Directory watch returned an invalid batch".into());
            }
            let Some(watches) = weak.upgrade() else {
                return Ok(());
            };
            if !watches
                .owned
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .current(start.ticket)
            {
                return Ok(());
            }
            let coverage = &batch.coverage[0];
            if !*established && coverage.mode != CoverageMode::Pending {
                if !watches.completed(start.ticket, coverage.mode, coverage.reason.clone()) {
                    return Ok(());
                }
                *established = true;
            } else if *established {
                watches
                    .owned
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .coverage(
                        start.ticket.source,
                        coverage.mode,
                        coverage.reason.clone(),
                        batch.reset,
                    )?;
            }
            if batch.reset || !batch.dirs.is_empty() || batch.workspace {
                watches
                    .owned
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .changed(start.ticket.source, batch.dirs, batch.reset)?;
            }
            watches.arrived.notify_waiters();
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    async fn local_source(weak: Weak<Self>, start: Start, cancel: Arc<AtomicBool>) {
        let Some(watches) = weak.upgrade() else {
            return;
        };
        let native = watches.native.clone();
        let ticket = start.ticket;
        #[cfg(test)]
        let manual = watches.test_manual_tree.load(Ordering::Acquire);
        #[cfg(test)]
        let mut gate = if matches!(&start.scope, Scope::Workspace { .. }) {
            watches.test_walk_gate.lock().unwrap().take()
        } else {
            None
        };
        drop(watches);
        let prepared = tokio::task::spawn_blocking(move || {
            let mut receiver = None;
            let mut changes = native::Installed {
                partial: None,
                reset_sources: Default::default(),
                partial_sources: Default::default(),
            };
            for attempt in 0..3 {
                if cancel.load(Ordering::Acquire) {
                    return Ok(None);
                }
                let mut plan = native::Plan::prepare(&start.scope)?;
                #[cfg(test)]
                if manual {
                    plan.use_manual_tree();
                }
                let (revision, baseline) = {
                    let mut pool = native.lock().unwrap_or_else(|p| p.into_inner());
                    if cancel.load(Ordering::Acquire) {
                        return Ok(None);
                    }
                    if pool.is_none() {
                        let (opened, rx) = native::Native::new()?;
                        receiver = Some((opened.identity().to_owned(), rx));
                        *pool = Some(opened);
                    }
                    let pool = pool.as_mut().expect("initialized notify pool");
                    let removed = pool.remove(ticket.source);
                    changes.reset_sources.extend(removed.reset_sources);
                    changes.partial_sources.extend(removed.partial_sources);
                    let installed = pool.install(ticket.source, &plan);
                    changes.partial = installed.partial;
                    changes.reset_sources.extend(installed.reset_sources);
                    changes.partial_sources.extend(installed.partial_sources);
                    let revision = pool.revision();
                    let baseline = revision.load(Ordering::Acquire);
                    (revision, baseline)
                };
                plan.expand_tree(|path| {
                    let mut pool = native.lock().unwrap_or_else(|p| p.into_inner());
                    if cancel.load(Ordering::Acquire) {
                        return Err("Directory watch initialization cancelled".into());
                    }
                    let pool = pool.as_mut().ok_or("Directory watch pool was retired")?;
                    let installed = pool.install_directory(ticket.source, path);
                    changes.reset_sources.extend(installed.reset_sources);
                    changes.partial_sources.extend(installed.partial_sources);
                    if let Some(error) = installed.partial {
                        return Err(error);
                    }
                    Ok(())
                });
                if let Some(error) = &plan.partial {
                    changes.partial = Some(error.clone());
                }
                #[cfg(test)]
                if let Some(gate) = gate.take() {
                    let _ = gate.walked.send(());
                    gate.resume
                        .recv_timeout(Duration::from_secs(5))
                        .map_err(|e| e.to_string())?;
                }
                if cancel.load(Ordering::Acquire) {
                    return Ok(None);
                }
                let stable = plan.still_current();
                if matches!(stable, Ok(false)) && attempt < 2 {
                    continue;
                }
                let retry = matches!(stable, Ok(false));
                if retry {
                    changes.partial =
                        Some("Git watch identity changed during initialization".into());
                }
                if let Err(error) = stable {
                    changes.partial = Some(error);
                }
                plan.registered();
                return Ok::<_, String>(Some((plan, changes, receiver, retry, revision, baseline)));
            }
            unreachable!("bounded initialization attempts return")
        })
        .await;
        let Some(watches) = weak.upgrade() else {
            return;
        };
        match prepared {
            Ok(Ok(Some((plan, material, receiver, retry, revision, baseline)))) => {
                let mut reason = material.partial.clone();
                watches.material_changed(material);
                if let Some((identity, rx)) = receiver {
                    watches.observe_native(identity, rx);
                }
                {
                    let mut running = watches.running.lock().unwrap_or_else(|p| p.into_inner());
                    running.plans.insert(ticket.source, Arc::new(plan));
                    if retry {
                        running
                            .retries
                            .insert(ticket.source, Instant::now() + Duration::from_secs(3));
                    }
                }
                if baseline == u64::MAX {
                    reason = Some("Directory watch native revision exhausted".into());
                } else if revision.load(Ordering::Acquire) != baseline {
                    let _ = watches
                        .owned
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .replan(ticket.source);
                }
                let mode = if reason.is_some() {
                    CoverageMode::Partial
                } else {
                    CoverageMode::Native
                };
                if !watches.completed(ticket, mode, reason) {
                    watches.stop_source(ticket.source);
                }
                watches.finished(ticket);
                watches.deadlines.notify_one();
            }
            Ok(Ok(None)) => {
                watches.completed(ticket, CoverageMode::Unavailable, None);
                watches.finished(ticket);
                watches.stop_source(ticket.source);
            }
            Ok(Err(error)) => Self::failed(weak, ticket, false, error),
            Err(error) => Self::failed(weak, ticket, false, error.to_string()),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    fn material_changed(&self, changes: native::Installed) {
        let mut owned = self.owned.lock().unwrap_or_else(|p| p.into_inner());
        for source in changes.reset_sources {
            let _ = owned.changed(source, Vec::new(), true);
        }
        for source in changes.partial_sources {
            let _ = owned.coverage(
                source,
                CoverageMode::Partial,
                Some("A shared directory watch needs reestablishment".into()),
                true,
            );
        }
        self.arrived.notify_waiters();
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    fn observe_native(self: &Arc<Self>, identity: String, mut rx: tokio::sync::mpsc::Receiver<()>) {
        let pool = self.native.lock().unwrap_or_else(|p| p.into_inner());
        if !pool
            .as_ref()
            .is_some_and(|native| native.identity() == identity)
        {
            return;
        }
        let weak = Arc::downgrade(self);
        let task = tokio::spawn(async move {
            while rx.recv().await.is_some() {
                tokio::time::sleep(GATHER).await;
                while rx.try_recv().is_ok() {}
                let Some(watches) = weak.upgrade() else {
                    return;
                };
                let dirty = {
                    let native = watches.native.lock().unwrap_or_else(|p| p.into_inner());
                    let Some(native) = native.as_ref() else {
                        return;
                    };
                    if native.identity() != identity {
                        return;
                    }
                    native.drain()
                };
                let plans: Vec<_> = watches
                    .running
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .plans
                    .iter()
                    .map(|(id, plan)| (*id, plan.clone()))
                    .collect();
                for (source, plan) in plans {
                    if let Some(change) = plan.change(&dirty) {
                        let effects = {
                            let mut owned = watches.owned.lock().unwrap_or_else(|p| p.into_inner());
                            let _ = owned.changed(source, change.dirs, change.reset);
                            if let Some(error) = dirty.error() {
                                let _ = owned.coverage(
                                    source,
                                    CoverageMode::Partial,
                                    Some(error.into()),
                                    true,
                                );
                            }
                            change.replan.then(|| owned.replan(source))
                        };
                        if let Some(Ok(effects)) = effects {
                            watches.apply(effects);
                        }
                    }
                }
                watches.arrived.notify_waiters();
            }
        });
        let mut running = self.running.lock().unwrap_or_else(|p| p.into_inner());
        if running.closed {
            task.abort();
        } else {
            running.observer = Some(task);
        }
    }

    pub(crate) fn new() -> Result<Self, String> {
        Ok(Self {
            owned: Mutex::new(Owned::new(crate::link::fresh_hex()?)),
            running: Mutex::new(Running::default()),
            arrived: Arc::new(tokio::sync::Notify::new()),
            deadlines: Arc::new(tokio::sync::Notify::new()),
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            native: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            test_manual_tree: AtomicBool::new(false),
            #[cfg(test)]
            test_walk_gate: Mutex::new(None),
        })
    }

    pub(crate) fn begin(
        self: &Arc<Self>,
        machine: String,
        scopes: Vec<Scope>,
        route: Route,
    ) -> Result<Opened, String> {
        let id = crate::link::fresh_hex()?;
        let effects = {
            let mut running = self.running.lock().unwrap_or_else(|p| p.into_inner());
            if running.closed {
                return Err("Directory watch resident has stopped".into());
            }
            let effects = self.owned.lock().unwrap_or_else(|p| p.into_inner()).begin(
                id.clone(),
                machine.clone(),
                scopes,
                Instant::now(),
            )?;
            running.routes.insert(machine.clone(), route);
            if running.expiry.is_none() {
                let weak = Arc::downgrade(self);
                let deadlines = self.deadlines.clone();
                running.expiry = Some(tokio::spawn(Self::expire_loop(weak, deadlines)));
            }
            effects
        };
        self.apply(effects);
        self.deadlines.notify_one();
        let batch = self
            .owned
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .read(&machine, &id, None)?;
        Ok(Opened {
            id,
            machine_id: machine,
            batch,
        })
    }

    pub(crate) async fn wait(
        self: &Arc<Self>,
        machine: &str,
        id: &str,
        cursor: Option<&str>,
        longest: Duration,
    ) -> Result<Batch, String> {
        self.owned
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .enter_wait(machine, id, Instant::now())?;
        let _waiting = Waiting {
            watches: Arc::downgrade(self),
            id: id.into(),
        };
        self.deadlines.notify_one();
        let deadline = tokio::time::Instant::now() + longest.min(Duration::from_secs(20));
        loop {
            let notified = self.arrived.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let mut batch = self
                .owned
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .read(machine, id, cursor)?;
            if batch.reset
                || cursor != Some(batch.cursor.as_str())
                || tokio::time::Instant::now() >= deadline
            {
                batch.retry_ms = if longest.is_zero() { 1000 } else { 0 };
                return Ok(batch);
            }
            let _ = tokio::time::timeout_at(deadline, notified).await;
        }
    }

    pub(crate) fn leave(self: &Arc<Self>, machine: &str, id: &str) -> Result<(), String> {
        let effects = self
            .owned
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .leave(machine, id)?;
        self.apply(effects);
        self.arrived.notify_waiters();
        self.deadlines.notify_one();
        Ok(())
    }

    fn apply(self: &Arc<Self>, effects: Effects) {
        {
            let running = self.running.lock().unwrap_or_else(|p| p.into_inner());
            for source in &effects.stop {
                if let Some(job) = running.jobs.get(source) {
                    job.cancel.store(true, Ordering::Release);
                    job.stop.send_replace(true);
                }
            }
        }
        for source in effects.stop {
            self.stop_source(source);
        }
        {
            let mut running = self.running.lock().unwrap_or_else(|p| p.into_inner());
            let owned = self.owned.lock().unwrap_or_else(|p| p.into_inner());
            for source in effects.retry {
                if owned.has_source(source) {
                    running
                        .retries
                        .insert(source, Instant::now() + Duration::from_secs(3));
                }
            }
        }
        self.deadlines.notify_one();
        for start in effects.start {
            let mut running = self.running.lock().unwrap_or_else(|p| p.into_inner());
            if running.closed
                || !self
                    .owned
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .current(start.ticket)
            {
                drop(running);
                self.completed(start.ticket, CoverageMode::Unavailable, None);
                continue;
            }
            let route = running
                .routes
                .get(&start.machine)
                .expect("source route")
                .clone();
            let cancel = Arc::new(AtomicBool::new(false));
            let (stop, stopped) = tokio::sync::watch::channel(false);
            let weak = Arc::downgrade(self);
            let cancelled = cancel.clone();
            let ticket = start.ticket;
            if let Some(previous) = running.jobs.remove(&ticket.source) {
                previous.cancel.store(true, Ordering::Release);
                previous.stop.send_replace(true);
                previous.task.abort();
            }
            let task =
                tokio::spawn(
                    async move { Self::source(weak, start, route, cancelled, stopped).await },
                );
            running.jobs.insert(
                ticket.source,
                Job {
                    ticket,
                    cancel,
                    stop,
                    active: false,
                    task,
                },
            );
        }
    }

    fn completed(
        self: &Arc<Self>,
        ticket: Ticket,
        mode: CoverageMode,
        reason: Option<String>,
    ) -> bool {
        let result = self
            .owned
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .initialized(ticket, mode, reason);
        let Ok((accepted, effects)) = result else {
            return false;
        };
        if accepted {
            if let Some(job) = self
                .running
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .jobs
                .get_mut(&ticket.source)
            {
                if job.ticket == ticket {
                    job.active = true;
                }
            }
        }
        self.apply(effects);
        self.arrived.notify_waiters();
        accepted
    }

    fn stop_source(&self, source: u64) {
        {
            let mut running = self.running.lock().unwrap_or_else(|p| p.into_inner());
            running.retries.remove(&source);
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            running.plans.remove(&source);
            if let Some(job) = running.jobs.get(&source) {
                job.cancel.store(true, Ordering::Release);
                job.stop.send_replace(true);
                if job.active {
                    job.task.abort();
                }
            }
            if running.jobs.get(&source).is_some_and(|job| job.active) {
                running.jobs.remove(&source);
            }
            let book = self.owned.lock().unwrap_or_else(|p| p.into_inner());
            running
                .routes
                .retain(|machine, _| book.has_machine(machine));
        }
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        {
            let multi_thread = tokio::runtime::Handle::try_current().is_ok_and(|handle| {
                handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread
            });
            if multi_thread {
                tokio::task::block_in_place(|| self.remove_native(source));
            } else {
                self.remove_native(source);
            }
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    fn remove_native(&self, source: u64) {
        let mut pool = self.native.lock().unwrap_or_else(|p| p.into_inner());
        if self
            .owned
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_empty()
        {
            if let Some(task) = self
                .running
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .observer
                .take()
            {
                task.abort();
            }
            pool.take();
        } else if let Some(native) = pool.as_mut() {
            self.material_changed(native.remove(source));
        }
    }

    async fn expire_loop(weak: Weak<Self>, deadlines: Arc<tokio::sync::Notify>) {
        loop {
            let notified = deadlines.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let next = {
                let Some(watches) = weak.upgrade() else {
                    return;
                };
                let now = Instant::now();
                let effects = watches
                    .owned
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .expire(now);
                watches.apply(effects);
                let due: Vec<_> = watches
                    .running
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .retries
                    .iter()
                    .filter(|(_, at)| **at <= now)
                    .map(|(id, _)| *id)
                    .collect();
                for source in due {
                    watches
                        .running
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .retries
                        .remove(&source);
                    let effects = watches
                        .owned
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .replan(source);
                    if let Ok(effects) = effects {
                        watches.apply(effects);
                    }
                }
                watches.arrived.notify_waiters();
                let owner = watches
                    .owned
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .next_expiry();
                let retry = watches
                    .running
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .retries
                    .values()
                    .copied()
                    .min();
                owner.into_iter().chain(retry).min()
            };
            match next {
                Some(at) => {
                    let _ = tokio::time::timeout_at(at.into(), notified).await;
                }
                None => notified.await,
            }
        }
    }

    pub(crate) async fn shutdown(self: &Arc<Self>) {
        let effects = {
            self.running
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .closed = true;
            self.owned.lock().unwrap_or_else(|p| p.into_inner()).close()
        };
        self.apply(effects);
        self.arrived.notify_waiters();
        let jobs = {
            let mut running = self.running.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(task) = running.expiry.take() {
                task.abort();
            }
            if let Some(task) = running.observer.take() {
                task.abort();
            }
            std::mem::take(&mut running.jobs)
        };
        for (_, job) in jobs {
            let _ = job.task.await;
        }
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        {
            self.native.lock().unwrap_or_else(|p| p.into_inner()).take();
        }
    }
}

impl Drop for Watches {
    fn drop(&mut self) {
        let running = self.running.get_mut().unwrap_or_else(|p| p.into_inner());
        if let Some(task) = running.expiry.take() {
            task.abort();
        }
        if let Some(task) = running.observer.take() {
            task.abort();
        }
        for (_, job) in running.jobs.drain() {
            job.cancel.store(true, Ordering::Release);
            job.stop.send_replace(true);
            if job.active {
                job.task.abort();
            }
        }
    }
}

struct Waiting {
    watches: Weak<Watches>,
    id: String,
}

impl Drop for Waiting {
    fn drop(&mut self) {
        if let Some(watches) = self.watches.upgrade() {
            watches
                .owned
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .finish_wait(&self.id);
        }
    }
}

async fn cancelled(mut stopped: tokio::sync::watch::Receiver<bool>) {
    while !*stopped.borrow_and_update() {
        if stopped.changed().await.is_err() {
            return;
        }
    }
}

async fn write_frame(
    send: &mut iroh::endpoint::SendStream,
    response: &Response,
) -> Result<(), String> {
    let body = proto::encode(response)?;
    if body.len() > FRAME_BYTES + 1024 {
        return Err("Directory watch response exceeds its byte budget".into());
    }
    send.write_all(&(body.len() as u32).to_be_bytes())
        .await
        .map_err(|e| e.to_string())?;
    send.write_all(&body).await.map_err(|e| e.to_string())
}

async fn read_frame(recv: &mut iroh::endpoint::RecvStream) -> Result<Response, String> {
    let mut len = [0u8; 4];
    recv.read_exact(&mut len).await.map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes(len) as usize;
    if len > FRAME_BYTES + 1024 {
        return Err("Directory watch frame exceeds its byte budget".into());
    }
    let mut body = vec![0u8; len];
    recv.read_exact(&mut body)
        .await
        .map_err(|e| e.to_string())?;
    proto::answer(&body)
}

pub(crate) struct Subscription {
    watches: Arc<Watches>,
    opened: Opened,
    node: Weak<crate::Node>,
    peer: String,
}

impl Subscription {
    pub(crate) fn local(
        node: &Arc<crate::Node>,
        peer: String,
        scope: Scope,
    ) -> Result<Self, String> {
        node.admit(&peer)?;
        let watches = node.watches.clone();
        let opened = watches.begin(node.device_str().into(), vec![scope], Route::Here)?;
        Ok(Self {
            watches,
            opened,
            node: Arc::downgrade(node),
            peer,
        })
    }

    pub(crate) async fn serve(self, mut send: iroh::endpoint::SendStream) {
        let result = async {
            write_frame(&mut send, &Response::Watching { batch: self.opened.batch.clone() }).await?;
            let mut cursor = self.opened.batch.cursor.clone();
            loop {
                let batch = tokio::select! {
                    _ = send.stopped() => return Ok::<(), String>(()),
                    batch = self.watches.wait(&self.opened.machine_id, &self.opened.id, Some(&cursor), Duration::from_secs(20)) => batch?,
                };
                let node = self.node.upgrade().ok_or("Directory watch resident stopped")?;
                node.admit(&self.peer)?;
                if node.paused() || crate::pause::peer_on(node.root(), &self.peer) { return Err("Directory watch peer is paused".into()); }
                drop(node);
                cursor = batch.cursor.clone();
                write_frame(&mut send, &Response::Changed { batch }).await?;
            }
        }.await;
        let _ = result;
        let _ = send.finish();
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let _ = self.watches.leave(&self.opened.machine_id, &self.opened.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    async fn native_ready(watches: &Arc<Watches>, opened: &Opened) -> Batch {
        let mut batch = opened.batch.clone();
        tokio::time::timeout(Duration::from_secs(5), async {
            while batch.coverage.iter().any(|c| {
                c.mode == CoverageMode::Pending
                    || c.reason.as_deref() == Some(owned::INITIALIZING_CHANGED)
            }) || !watches.running.lock().unwrap().retries.is_empty()
            {
                batch = watches
                    .wait(
                        &opened.machine_id,
                        &opened.id,
                        Some(&batch.cursor),
                        Duration::from_secs(2),
                    )
                    .await
                    .unwrap();
            }
        })
        .await
        .expect("native initialization deadline");
        assert!(
            batch
                .coverage
                .iter()
                .all(|c| c.mode == CoverageMode::Native),
            "{batch:?}"
        );
        batch
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    async fn changed(watches: &Arc<Watches>, opened: &Opened, batch: &Batch) -> Batch {
        let result = watches
            .wait(
                &opened.machine_id,
                &opened.id,
                Some(&batch.cursor),
                Duration::from_secs(3),
            )
            .await
            .unwrap();
        assert_ne!(
            result.cursor, batch.cursor,
            "a real filesystem write must invalidate"
        );
        result
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[test]
    fn read_access_is_not_a_file_change() {
        use notify::{
            EventKind,
            event::{AccessKind, AccessMode, MetadataKind, ModifyKind},
        };
        for kind in [
            EventKind::Access(AccessKind::Read),
            EventKind::Access(AccessKind::Open(AccessMode::Read)),
            EventKind::Access(AccessKind::Close(AccessMode::Read)),
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)),
        ] {
            assert!(!changes_files(kind), "{kind:?}");
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[test]
    fn writes_and_directory_identity_changes_still_invalidate() {
        use notify::{
            EventKind,
            event::{CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode},
        };
        for kind in [
            EventKind::Create(CreateKind::File),
            EventKind::Create(CreateKind::Folder),
            EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime)),
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            EventKind::Remove(RemoveKind::Folder),
        ] {
            assert!(changes_files(kind), "{kind:?}");
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn native_workspace_observes_atomic_save_nested_rename_and_outside_file_parent() {
        let checkout = crate::git::repository_history_tests::Checkout::new();
        let workspace = checkout.0.join("workspace");
        let outside = checkout.0.join("outside");
        std::fs::create_dir_all(workspace.join("ignored/deep")).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(checkout.0.join(".gitignore"), b"workspace/ignored/\n").unwrap();
        std::fs::write(workspace.join("draft.txt"), b"before").unwrap();
        let watches = Arc::new(Watches::new().unwrap());
        let opened = watches
            .begin(
                "local".into(),
                vec![
                    Scope::Workspace {
                        root: workspace.display().to_string(),
                    },
                    Scope::Entries {
                        path: outside.display().to_string(),
                    },
                ],
                Route::Here,
            )
            .unwrap();
        let mut batch = native_ready(&watches, &opened).await;
        std::fs::write(workspace.join("draft.tmp"), b"after").unwrap();
        std::fs::rename(workspace.join("draft.tmp"), workspace.join("draft.txt")).unwrap();
        batch = changed(&watches, &opened, &batch).await;
        assert!(batch.workspace);
        assert!(batch.reset || batch.dirs.contains(&workspace.display().to_string()));
        std::fs::write(
            workspace.join("ignored/deep/中文.txt"),
            b"observed despite ignore",
        )
        .unwrap();
        batch = changed(&watches, &opened, &batch).await;
        assert!(batch.workspace);
        assert!(
            batch.reset
                || batch
                    .dirs
                    .contains(&workspace.join("ignored/deep").display().to_string())
        );
        std::fs::rename(
            workspace.join("ignored/deep/中文.txt"),
            workspace.join("moved.txt"),
        )
        .unwrap();
        batch = changed(&watches, &opened, &batch).await;
        assert!(
            batch.reset
                || (batch.dirs.contains(&workspace.display().to_string())
                    && batch
                        .dirs
                        .contains(&workspace.join("ignored/deep").display().to_string()))
        );
        std::fs::write(outside.join("current.txt"), b"outside root").unwrap();
        batch = changed(&watches, &opened, &batch).await;
        assert!(batch.reset || batch.dirs.contains(&outside.display().to_string()));
        watches.shutdown().await;
        assert_eq!(watches.owned.lock().unwrap().counts(), (0, 0, 0, 0));
        assert!(watches.native.lock().unwrap().is_none());
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn native_readonly_file_and_git_reads_do_not_create_an_invalidation_loop() {
        let (checkout, _) = crate::git::repository_history_tests::linear(1);
        let path = checkout.0.join(".gitignore");
        std::fs::write(&path, b"ignored\n").unwrap();
        let watches = Arc::new(Watches::new().unwrap());
        let opened = watches
            .begin(
                "local".into(),
                vec![Scope::Workspace {
                    root: checkout.0.display().to_string(),
                }],
                Route::Here,
            )
            .unwrap();
        let batch = native_ready(&watches, &opened).await;
        for _ in 0..20 {
            std::fs::read(&path).unwrap();
            crate::git::watch_plan(&checkout.0).unwrap();
            gix::open(&checkout.0).unwrap().head().unwrap();
        }
        let quiet = watches
            .wait(
                "local",
                &opened.id,
                Some(&batch.cursor),
                Duration::from_millis(500),
            )
            .await
            .unwrap();
        assert_eq!(
            quiet.cursor, batch.cursor,
            "read-only access must not feed back into fresh-stat refreshes"
        );
        assert!(!quiet.reset && quiet.dirs.is_empty() && !quiet.workspace);
        watches.shutdown().await;
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn native_linked_worktree_covers_common_refs_packed_refs_and_detached_head() {
        let (checkout, marks) = crate::git::repository_history_tests::linear(2);
        let linked = checkout.0.join("linked");
        checkout.git(&[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "topic/deep",
            linked.to_str().unwrap(),
            &marks[&1],
        ]);
        let watches = Arc::new(Watches::new().unwrap());
        let opened = watches
            .begin(
                "local".into(),
                vec![Scope::Workspace {
                    root: linked.display().to_string(),
                }],
                Route::Here,
            )
            .unwrap();
        let mut batch = native_ready(&watches, &opened).await;
        checkout.git(&["update-ref", "refs/heads/topic/deep", &marks[&2]]);
        batch = changed(&watches, &opened, &batch).await;
        assert!(
            batch.workspace,
            "a common-dir ref outside the linked root invalidates metadata"
        );
        checkout.git(&["pack-refs", "--all"]);
        batch = changed(&watches, &opened, &batch).await;
        assert!(batch.workspace);
        checkout.git(&[
            "-C",
            linked.to_str().unwrap(),
            "switch",
            "--quiet",
            "--detach",
            &marks[&1],
        ]);
        batch = changed(&watches, &opened, &batch).await;
        assert!(batch.workspace);
        let reopened = Opened {
            id: opened.id.clone(),
            machine_id: opened.machine_id.clone(),
            batch,
        };
        native_ready(&watches, &reopened).await;
        assert_eq!(
            crate::git::watch_plan(&linked).unwrap().unwrap().reference,
            None
        );
        watches.shutdown().await;
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn native_idle_deadline_releases_the_last_handle_without_another_wait() {
        let checkout = crate::git::repository_history_tests::Checkout::new();
        let watches = Arc::new(Watches::new().unwrap());
        let opened = watches
            .begin(
                "local".into(),
                vec![Scope::Entries {
                    path: checkout.0.display().to_string(),
                }],
                Route::Here,
            )
            .unwrap();
        native_ready(&watches, &opened).await;
        assert!(
            watches
                .native
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .handle_count()
                > 0
        );
        watches
            .owned
            .lock()
            .unwrap()
            .set_expiry(&opened.id, Instant::now() + Duration::from_millis(40));
        watches.deadlines.notify_one();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                if watches.owned.lock().unwrap().counts().0 == 0
                    && watches
                        .native
                        .lock()
                        .unwrap()
                        .as_ref()
                        .map_or(0, |native| native.handle_count())
                        == 0
                {
                    break;
                }
            }
        })
        .await
        .expect("the resident deadline, not a subsequent client request, retires the handle");
        assert!(
            watches
                .wait("local", &opened.id, None, Duration::ZERO)
                .await
                .is_err()
        );
        watches.shutdown().await;
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn native_aliases_share_physical_handles_but_keep_independent_leases() {
        let checkout = crate::git::repository_history_tests::Checkout::new();
        let target = checkout.0.join("target");
        let alias = checkout.0.join("alias");
        std::fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, &alias).unwrap();
        let watches = Arc::new(Watches::new().unwrap());
        let first = watches
            .begin(
                "local".into(),
                vec![Scope::Entries {
                    path: target.display().to_string(),
                }],
                Route::Here,
            )
            .unwrap();
        native_ready(&watches, &first).await;
        let before = watches
            .native
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .handle_count();
        let second = watches
            .begin(
                "local".into(),
                vec![Scope::Entries {
                    path: alias.display().to_string(),
                }],
                Route::Here,
            )
            .unwrap();
        let batch = native_ready(&watches, &second).await;
        assert_eq!(
            watches
                .native
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .handle_count(),
            before
        );
        watches.leave("local", &first.id).unwrap();
        std::fs::write(target.join("new.txt"), b"alias kept").unwrap();
        let batch = changed(&watches, &second, &batch).await;
        assert_eq!(batch.dirs, vec![alias.display().to_string()]);
        watches.leave("local", &second.id).unwrap();
        assert!(watches.native.lock().unwrap().is_none());
        watches.shutdown().await;
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn native_held_wait_is_cancelled_by_leave_and_a_late_initialization_cannot_reopen() {
        let checkout = crate::git::repository_history_tests::Checkout::new();
        let watches = Arc::new(Watches::new().unwrap());
        let scopes = vec![Scope::Entries {
            path: checkout.0.display().to_string(),
        }];
        let opened = watches
            .begin("local".into(), scopes.clone(), Route::Here)
            .unwrap();
        let batch = native_ready(&watches, &opened).await;
        let held = {
            let watches = watches.clone();
            let id = opened.id.clone();
            tokio::spawn(async move {
                watches
                    .wait("local", &id, Some(&batch.cursor), Duration::from_secs(20))
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!held.is_finished());
        assert!(
            watches
                .wait("local", &opened.id, None, Duration::ZERO)
                .await
                .is_err(),
            "one wait per owner"
        );
        watches.leave("local", &opened.id).unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(1), held)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        for _ in 0..10 {
            let pending = watches
                .begin("local".into(), scopes.clone(), Route::Here)
                .unwrap();
            watches.leave("local", &pending.id).unwrap();
            assert!(
                watches
                    .wait("local", &pending.id, None, Duration::ZERO)
                    .await
                    .is_err()
            );
        }
        watches.shutdown().await;
        assert_eq!(watches.owned.lock().unwrap().counts(), (0, 0, 0, 0));
        assert!(watches.native.lock().unwrap().is_none());
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn native_manual_tree_rechecks_a_nested_creation_drained_before_the_new_plan_exists() {
        let checkout = crate::git::repository_history_tests::Checkout::new();
        let workspace = checkout.0.join("workspace");
        let deep = workspace.join("already/deep");
        let observer = checkout.0.join("observer");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::create_dir(&observer).unwrap();
        let watches = Arc::new(Watches::new().unwrap());
        let existing = watches
            .begin(
                "local".into(),
                vec![Scope::Entries {
                    path: observer.display().to_string(),
                }],
                Route::Here,
            )
            .unwrap();
        native_ready(&watches, &existing).await;
        watches.test_manual_tree.store(true, Ordering::Release);
        let (walked, reached) = tokio::sync::oneshot::channel();
        let (resume, blocked) = std::sync::mpsc::channel();
        *watches.test_walk_gate.lock().unwrap() = Some(WalkGate {
            walked,
            resume: blocked,
        });
        let opened = watches
            .begin(
                "local".into(),
                vec![Scope::Workspace {
                    root: workspace.display().to_string(),
                }],
                Route::Here,
            )
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), reached)
            .await
            .unwrap()
            .unwrap();
        assert!(
            watches
                .native
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .watches_directory(&deep),
            "the existing nested parent was registered before its children were enumerated"
        );
        assert_eq!(
            watches.running.lock().unwrap().plans.len(),
            1,
            "the new plan is not yet handed off"
        );
        let counter = watches.native.lock().unwrap().as_ref().unwrap().revision();
        let before = counter.load(Ordering::Acquire);
        let new = deep.join("new");
        std::fs::create_dir(&new).unwrap();
        std::fs::write(new.join("child.txt"), b"created in the initialization gap").unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                let drained = !watches
                    .native
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .has_pending();
                if counter.load(Ordering::Acquire) > before && drained {
                    break;
                }
            }
        })
        .await
        .expect("the already running observer drained a real nested-directory event");
        assert_eq!(watches.running.lock().unwrap().plans.len(), 1);
        assert_eq!(
            watches
                .owned
                .lock()
                .unwrap()
                .first_native_generation(&opened.id),
            None
        );
        resume.send(()).unwrap();
        let batch = native_ready(&watches, &opened).await;
        assert!(
            watches
                .owned
                .lock()
                .unwrap()
                .first_native_generation(&opened.id)
                .unwrap()
                >= 2,
            "the stale first walk cannot publish full coverage"
        );
        assert!(
            watches
                .native
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .watches_directory(&new),
            "a fresh walk registered the new nested parent"
        );
        std::fs::write(new.join("child.txt"), b"modified after native coverage").unwrap();
        let changed = changed(&watches, &opened, &batch).await;
        assert!(changed.workspace);
        assert!(changed.reset || changed.dirs.contains(&new.display().to_string()));
        watches.shutdown().await;
        assert_eq!(watches.owned.lock().unwrap().counts(), (0, 0, 0, 0));
        assert!(watches.native.lock().unwrap().is_none());
    }
}
