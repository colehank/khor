use crate::updater_package::DesktopPackage;
use khor_gui_core::updates::{
    AppUpdateStatus, UpdateAction, UpdateChannel, UpdatePhase, UpdatePreferences, UpdateSource,
};
use khor_update::{
    BuildIdentity, Channel, Component, Feed, Platform, Staging, TrustRoot, VerifiedRelease,
};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

struct Selection {
    release: VerifiedRelease,
    package: Option<DesktopPackage>,
}

struct Data {
    status: AppUpdateStatus,
    selected: Option<Selection>,
    #[cfg(unix)]
    companions: Option<crate::app_companions::Plan>,
}

pub(crate) struct AppUpdates {
    started: std::sync::atomic::AtomicBool,
    build: BuildIdentity,
    root: PathBuf,
    data: Mutex<Data>,
    operation: tokio::sync::Mutex<()>,
    staging: Mutex<Option<Arc<Staging>>>,
}

impl AppUpdates {
    pub(crate) fn new() -> Result<Self, String> {
        let build = khor_update::build_identity()?;
        let root = khor_runtime::local_directory()
            .map_err(|e| e.to_string())?
            .join("app-updates");
        let default = if build
            .release
            .as_ref()
            .is_some_and(|id| id.tag.channel() == Channel::Beta)
        {
            UpdateChannel::Beta
        } else {
            UpdateChannel::Stable
        };
        let (preferences, preference_failure) = match read_preferences(&root) {
            Ok(preferences) => (preferences, None),
            Err(_) if !root.join("preferences.json").exists() => {
                (UpdatePreferences::new(default), None)
            }
            Err(error) => {
                let mut preferences = UpdatePreferences::new(default);
                preferences.automatic_check = false;
                preferences.automatic_download = false;
                (preferences, Some(error))
            }
        };
        let source = installation_source(&build)?;
        let status = AppUpdateStatus {
            source,
            phase: if preference_failure.is_some() {
                UpdatePhase::Failed
            } else {
                UpdatePhase::Idle
            },
            current: build
                .release
                .as_ref()
                .map(|id| id.tag.as_str().to_owned())
                .unwrap_or_else(|| build.version.clone()),
            built_from: build.release.as_ref().map(|id| id.tag.as_str().to_owned()),
            source_commit: build
                .source
                .as_ref()
                .map(|source| source.as_str().to_owned()),
            available: None,
            notes: String::new(),
            received: 0,
            total: None,
            checked_at: None,
            retry: preference_failure.as_ref().map(|_| UpdateAction::Check),
            failure: preference_failure,
            blocked: Vec::new(),
            preferences,
            companions: Vec::new(),
            restart_required: false,
        };
        Ok(Self {
            started: std::sync::atomic::AtomicBool::new(false),
            build,
            root,
            data: Mutex::new(Data {
                status,
                selected: None,
                #[cfg(unix)]
                companions: None,
            }),
            operation: tokio::sync::Mutex::new(()),
            staging: Mutex::new(None),
        })
    }

    pub(crate) fn status(&self) -> Result<AppUpdateStatus, String> {
        let mut status = self
            .data
            .lock()
            .map(|data| data.status.clone())
            .map_err(|_| "update state is unavailable".to_owned())?;
        if let Ok(current) = self.current() {
            status.current = current.tag.as_str().to_owned();
        }
        Ok(status)
    }

    fn change(
        &self,
        app: &AppHandle,
        update: impl FnOnce(&mut Data),
    ) -> Result<AppUpdateStatus, String> {
        let status = {
            let mut data = self
                .data
                .lock()
                .map_err(|_| "update state is unavailable")?;
            update(&mut data);
            if data.status.phase != UpdatePhase::Failed {
                data.status.retry = None;
                data.status.failure = None;
            }
            data.status.clone()
        };
        let _ = app.emit("app-update:changed", &status);
        Ok(status)
    }

    fn store(&self) -> Result<Arc<Staging>, String> {
        let mut held = self
            .staging
            .lock()
            .map_err(|_| "update staging is unavailable")?;
        if let Some(staging) = held.as_ref() {
            return Ok(staging.clone());
        }
        // Use the runtime store's existing local-filesystem admission before creating this child.
        let _ = khor_runtime::Store::local().map_err(|e| e.to_string())?;
        let staging = Arc::new(Staging::open(self.root.clone())?);
        *held = Some(staging.clone());
        Ok(staging)
    }

    async fn maintain(&self) {
        if self
            .status()
            .is_ok_and(|status| status.source == UpdateSource::Direct)
        {
            if let Ok(_operation) = self.operation.try_lock() {
                if let Ok(staging) = self.store() {
                    collect_cache(staging).await;
                }
            }
        }
    }

    fn channel(&self) -> Result<Channel, String> {
        Ok(match self.status()?.preferences.channel {
            UpdateChannel::Stable => Channel::Stable,
            UpdateChannel::Beta => Channel::Beta,
        })
    }

    fn current(&self) -> Result<khor_update::ReleaseIdentity, String> {
        #[cfg(target_os = "macos")]
        if let Some(identity) = crate::app_update_install::adopted(&self.root, &self.build) {
            return Ok(identity);
        }
        self.build
            .release
            .clone()
            .ok_or_else(|| "development builds do not have a release identity".into())
    }

    pub(crate) async fn preferences(
        &self,
        app: &AppHandle,
        preferences: UpdatePreferences,
    ) -> Result<AppUpdateStatus, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "an update operation is already running")?;
        let staging = self.store()?;
        write_preferences(staging.root(), &preferences)?;
        self.change(app, |data| {
            if data.status.preferences.channel != preferences.channel {
                data.selected = None;
                #[cfg(unix)]
                {
                    data.companions = None;
                }
                data.status.companions.clear();
                data.status.restart_required = false;
                data.status.available = None;
                data.status.notes.clear();
                data.status.phase = UpdatePhase::Idle;
            }
            data.status.preferences = preferences;
        })
    }

    pub(crate) async fn check(&self, app: &AppHandle) -> Result<AppUpdateStatus, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "an update operation is already running")?;
        if self.status()?.source != UpdateSource::Direct {
            return self.status();
        }
        self.change(app, |data| {
            data.status.phase = UpdatePhase::Checking;
            data.status.failure = None;
        })?;
        let result = self.check_inner(app).await;
        match result {
            Ok(status) => Ok(status),
            Err(error) => self.failed(app, UpdateAction::Check, error),
        }
    }

    fn app_offer(&self, release: &VerifiedRelease) -> Result<Option<khor_update::Offer>, String> {
        release.select(
            Component::DesktopApp,
            Platform::current()?,
            &self.current()?,
            self.channel()?,
            None,
        )
    }

    async fn check_inner(&self, app: &AppHandle) -> Result<AppUpdateStatus, String> {
        let current = self.current()?;
        let trust = TrustRoot::bundled()?;
        let channel = self.channel()?;
        let staging = self.store()?;
        // Refresh does not depend on a previously selected offer still being usable.
        // Startup already restores cached downloads; a fresh check must be able to
        // replace an interrupted or obsolete selection.
        #[cfg(unix)]
        let plan = {
            let root = self.root.clone();
            crate::off_main(move || crate::app_companions::Plan::discover(&root))
                .await
                .map_err(|error| format!("local CLI discovery: {error}"))?
        };
        #[cfg(unix)]
        let current = plan.oldest(&current);
        let release = Feed::github(trust.clone())?
            .check(&current, channel, None)
            .await?;
        let Some(release) = release else {
            return self.change(app, |data| {
                data.selected = None;
                data.status.available = None;
                data.status.notes.clear();
                data.status.phase = UpdatePhase::Idle;
                data.status.checked_at = Some(jiff::Timestamp::now().to_string());
                #[cfg(unix)]
                {
                    data.status.companions = plan.statuses();
                    data.companions = Some(plan);
                }
            });
        };
        staging.remember(&release)?;
        collect_cache(staging).await;
        self.adopt_release(app, release, true).await
    }

    async fn adopt_release(
        &self,
        app: &AppHandle,
        release: VerifiedRelease,
        checked: bool,
    ) -> Result<AppUpdateStatus, String> {
        let trust = TrustRoot::bundled()?;
        let staging = self.store()?;
        let offer = self.app_offer(&release)?;
        let restart_required = offer.is_some();
        let mut ready = offer
            .as_ref()
            .map(|offer| staging.is_ready(&trust, offer))
            .transpose()?
            .unwrap_or(true);
        let mut total = offer.as_ref().map_or(0, |offer| offer.artifact().bytes);
        let mut changed = offer.is_some();
        #[cfg(unix)]
        let plan = {
            let root = self.root.clone();
            let selected = release.clone();
            let channel = self.channel()?;
            let plan = crate::off_main(move || {
                let mut plan = crate::app_companions::Plan::discover(&root)?;
                plan.select_release(&selected, channel)?;
                Ok(plan)
            })
            .await?;
            if let Some(cli) = plan.offer(&release, channel)? {
                changed = true;
                ready &= staging.is_ready(&trust, &cli)?;
                total = total
                    .checked_add(cli.artifact().bytes)
                    .ok_or("component download size overflow")?;
            }
            plan
        };
        let package = match offer {
            Some(offer) if checked => Some(
                DesktopPackage::check(app, offer, trust)
                    .await
                    .map_err(|error| format!("application update metadata: {error}"))?,
            ),
            _ => self
                .data
                .lock()
                .map_err(|_| "update state is unavailable")?
                .selected
                .as_mut()
                .filter(|selection| {
                    selection.release.manifest().identity == release.manifest().identity
                })
                .and_then(|selection| selection.package.take()),
        };
        self.change(app, |data| {
            data.status.available =
                changed.then(|| release.manifest().identity.tag.as_str().to_owned());
            data.status.restart_required = restart_required;
            data.status.notes = release.manifest().notes.clone();
            data.status.total = changed.then_some(total);
            data.status.received = if ready && changed { total } else { 0 };
            if checked {
                data.status.checked_at = Some(jiff::Timestamp::now().to_string());
            }
            data.status.phase = if !changed {
                UpdatePhase::Idle
            } else if ready {
                UpdatePhase::Ready
            } else {
                UpdatePhase::Available
            };
            #[cfg(unix)]
            {
                data.status.companions = plan.statuses();
                data.companions = Some(plan);
            }
            data.selected = Some(Selection { release, package });
        })
    }

    #[cfg(unix)]
    pub(crate) async fn select_companions(
        &self,
        app: &AppHandle,
        ids: Vec<String>,
    ) -> Result<AppUpdateStatus, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "an update operation is already running")?;
        let mut plan = self
            .data
            .lock()
            .map_err(|_| "update state is unavailable")?
            .companions
            .clone()
            .ok_or("check updates before choosing CLI installations")?;
        let plan = crate::off_main(move || {
            plan.select(&ids)?;
            Ok(plan)
        })
        .await?;
        let release = self
            .data
            .lock()
            .map_err(|_| "update state is unavailable")?
            .selected
            .as_ref()
            .map(|selection| selection.release.clone());
        if let Some(release) = release {
            return self.adopt_release(app, release, false).await;
        }
        self.change(app, |data| {
            data.status.companions = plan.statuses();
            data.companions = Some(plan);
        })
    }

    pub(crate) async fn download(&self, app: &AppHandle) -> Result<AppUpdateStatus, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "an update operation is already running")?;
        self.change(app, |data| {
            data.status.phase = UpdatePhase::Downloading;
            data.status.received = 0;
        })?;
        let result = self.download_inner(app).await;
        match result {
            Ok(status) => Ok(status),
            Err(error) => self.failed(app, UpdateAction::Download, error),
        }
    }

    async fn download_inner(&self, app: &AppHandle) -> Result<AppUpdateStatus, String> {
        let release = self
            .data
            .lock()
            .map_err(|_| "update state is unavailable")?
            .selected
            .as_ref()
            .map(|selected| selected.release.clone())
            .ok_or("check for updates first")?;
        let trust = TrustRoot::bundled()?;
        let staging = self.store()?;
        let mut received = 0;
        if let Some(offer) = self.app_offer(&release)? {
            let path = staging.archive(&offer);
            if !staging.is_ready(&trust, &offer)? {
                let package = DesktopPackage::check(app, offer.clone(), trust.clone()).await?;
                self.change(app, |data| {
                    data.status.phase = UpdatePhase::Downloading;
                    data.status.failure = None;
                    data.status.received = 0;
                })?;
                let mut last = Instant::now() - Duration::from_secs(1);
                package
                    .download(&path, |progress| {
                        if last.elapsed() >= Duration::from_millis(100)
                            || progress.received == progress.total
                        {
                            last = Instant::now();
                            let _ =
                                self.change(app, |data| data.status.received = progress.received);
                        }
                    })
                    .await?;
            }
            received = offer.artifact().bytes;
        }
        #[cfg(unix)]
        {
            let channel = self.channel()?;
            let cli = self
                .data
                .lock()
                .map_err(|_| "update state is unavailable")?
                .companions
                .as_ref()
                .map(|plan| plan.offer(&release, channel))
                .transpose()?
                .flatten();
            if let Some(offer) = cli {
                if !staging.is_ready(&trust, &offer)? {
                    self.change(app, |data| {
                        data.status.phase = UpdatePhase::Downloading;
                        data.status.failure = None;
                        data.status.received = received;
                    })?;
                    let mut last = Instant::now() - Duration::from_secs(1);
                    let download =
                        khor_update::Download::fetch(offer, &trust, staging.root(), |progress| {
                            if last.elapsed() >= Duration::from_millis(100)
                                || progress.received == progress.total
                            {
                                last = Instant::now();
                                let _ = self.change(app, |data| {
                                    data.status.received = received + progress.received
                                });
                            }
                        })
                        .await?;
                    let staging = staging.clone();
                    let trust = trust.clone();
                    crate::off_main(move || staging.cache(&download, &trust)).await?;
                }
            }
        }
        self.adopt_release(app, release, false).await
    }

    fn failed(
        &self,
        app: &AppHandle,
        action: UpdateAction,
        error: String,
    ) -> Result<AppUpdateStatus, String> {
        eprintln!("application update {action:?} failed: {error}");
        // LaunchServices normally discards stderr. Keep one bounded diagnostic in
        // the already admitted private staging directory, without blocking retry.
        if self.staging.lock().is_ok_and(|staging| staging.is_some()) {
            if let Err(record_error) = record_failure(&self.root, action, &error) {
                eprintln!("application update diagnostic unavailable: {record_error}");
            }
        }
        self.change(app, |data| {
            data.status.phase = UpdatePhase::Failed;
            data.status.failure = Some(error);
            data.status.retry = Some(action);
        })
    }

    #[cfg(target_os = "macos")]
    pub(crate) async fn install(
        &self,
        app: &AppHandle,
        automatic: bool,
    ) -> Result<AppUpdateStatus, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "an update operation is already running")?;
        let result = self.install_inner(app, automatic).await;
        match result {
            Ok(status) => Ok(status),
            Err(error) => self.failed(app, UpdateAction::Install, error),
        }
    }

    #[cfg(target_os = "macos")]
    async fn install_inner(
        &self,
        app: &AppHandle,
        automatic: bool,
    ) -> Result<AppUpdateStatus, String> {
        let trust = TrustRoot::bundled()?;
        let release = self
            .data
            .lock()
            .map_err(|_| "update state is unavailable")?
            .selected
            .as_ref()
            .map(|selection| selection.release.clone())
            .ok_or("download an update first")?;
        let staging = self.store()?;
        self.change(app, |data| {
            data.status.phase = UpdatePhase::Verifying;
            data.status.failure = None;
            data.status.blocked.clear();
        })?;
        let application = if let Some(offer) = self.app_offer(&release)? {
            let path = staging.archive(&offer);
            if !staging.is_ready(&trust, &offer)? {
                return Err("download the complete application update first".into());
            }
            let held = self
                .data
                .lock()
                .map_err(|_| "update state is unavailable")?
                .selected
                .as_mut()
                .and_then(|selection| selection.package.take());
            let package = match held {
                Some(package) => package,
                None => DesktopPackage::check(app, offer.clone(), trust.clone()).await?,
            };
            let candidate_path = path.clone();
            let candidate_parent = staging.root().to_path_buf();
            crate::off_main(move || {
                let candidate =
                    khor_update::AppCandidate::extract(&candidate_path, &offer, &candidate_parent)?;
                khor_node::updates::verify_program(candidate.program(), &offer)?;
                khor_node::updates::verify_activation(
                    candidate.program(),
                    &std::env::current_exe().map_err(|e| e.to_string())?,
                )?;
                Ok(())
            })
            .await?;
            Some((package, path))
        } else {
            None
        };
        let _permit = if application.is_some() {
            let admission = app
                .state::<std::sync::Arc<crate::update_admission::UpdateAdmission>>()
                .inner()
                .clone();
            match admission.prepare(app, automatic).await {
                Ok(permit) => Some(permit),
                Err(blocked) => {
                    return self.change(app, |data| {
                        data.status.phase = UpdatePhase::Ready;
                        data.status.blocked = blocked;
                    });
                }
            }
        } else {
            None
        };
        self.apply_companions(app, &release, automatic).await?;
        let Some((package, path)) = application else {
            return self.change(app, |data| {
                data.status.phase = UpdatePhase::Complete;
                data.status.available = None;
            });
        };
        self.change(app, |data| data.status.phase = UpdatePhase::Installing)?;
        let install_root = staging.root().to_path_buf();
        crate::off_main(move || {
            let mut installation =
                crate::app_update_install::Installation::prepare(&install_root, &release)?;
            installation.begin()?;
            if let Err(error) = package.install(&path) {
                installation.restore_after_failure()?;
                return Err(error);
            }
            installation.handoff()
        })
        .await?;
        let status = self.change(app, |data| data.status.phase = UpdatePhase::AwaitingHealth)?;
        app.exit(0);
        Ok(status)
    }

    #[cfg(target_os = "macos")]
    async fn apply_companions(
        &self,
        app: &AppHandle,
        release: &VerifiedRelease,
        automatic: bool,
    ) -> Result<(), String> {
        let plan = self
            .data
            .lock()
            .map_err(|_| "update state is unavailable")?
            .companions
            .clone()
            .ok_or("check the component update plan first")?;
        if automatic && plan.automatic_blocked() {
            return Err("retry the failed CLI update manually".into());
        }
        let channel = self.channel()?;
        let trust = TrustRoot::bundled()?;
        let staging = self.store()?;
        for entry in plan.pending() {
            let offer = release
                .select(
                    Component::Cli,
                    Platform::current()?,
                    &entry.cli.current,
                    channel,
                    None,
                )?
                .ok_or("CLI no longer has the planned update")?;
            if !staging.is_ready(&trust, &offer)? {
                return Err("download the complete CLI update first".into());
            }
            self.companion_progress(app, &entry.status.id, UpdatePhase::Installing, None)
                .await?;
            let result = khor_node::updates::upgrade_managed_cli(
                &entry.cli.source,
                &entry.cli.current,
                release,
                channel,
                &trust,
                &staging.archive(&offer),
            )
            .await
            .and_then(|current| {
                if current.current == release.manifest().identity {
                    Ok(current)
                } else {
                    Err("CLI did not retain the selected update".into())
                }
            });
            let phase = if result.is_ok() {
                UpdatePhase::Complete
            } else {
                UpdatePhase::Failed
            };
            self.companion_progress(app, &entry.status.id, phase, Some(result.clone()))
                .await?;
            result?;
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn companion_progress(
        &self,
        app: &AppHandle,
        id: &str,
        phase: UpdatePhase,
        result: Option<Result<khor_node::updates::ManagedCli, String>>,
    ) -> Result<(), String> {
        let mut plan = self
            .data
            .lock()
            .map_err(|_| "update state is unavailable")?
            .companions
            .clone()
            .ok_or("component update plan is unavailable")?;
        let id = id.to_owned();
        let plan = crate::off_main(move || {
            plan.progress(&id, phase, result.as_ref())?;
            Ok(plan)
        })
        .await?;
        self.change(app, |data| {
            data.status.companions = plan.statuses();
            data.companions = Some(plan);
            if phase == UpdatePhase::Installing {
                data.status.phase = UpdatePhase::Installing;
            }
        })?;
        Ok(())
    }

    async fn restore_cached(&self, app: &AppHandle) -> Result<AppUpdateStatus, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "an update operation is already running")?;
        if self.status()?.source != UpdateSource::Direct {
            return self.status();
        }
        let staging = self.store()?;
        let trust = TrustRoot::bundled()?;
        if let Some(release) = staging.selected_release(&trust)? {
            return self.adopt_release(app, release, false).await;
        }
        self.status()
    }
}

pub(crate) fn start(app: &AppHandle) {
    if app
        .state::<AppUpdates>()
        .started
        .swap(true, std::sync::atomic::Ordering::AcqRel)
    {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        startup(&app).await;
        let updates = app.state::<AppUpdates>();
        if !updates
            .status()
            .is_ok_and(|status| status.source == UpdateSource::Direct)
        {
            return;
        }
        let mut beat = tokio::time::interval(Duration::from_secs(30));
        beat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        beat.tick().await;
        loop {
            beat.tick().await;
            updates.maintain().await;
        }
    });
}

async fn startup(app: &AppHandle) {
    let updates = app.state::<AppUpdates>();
    updates.maintain().await;
    let restored = match updates.restore_cached(app).await {
        Ok(status) => status,
        Err(error) => match updates.failed(app, UpdateAction::Check, error) {
            Ok(status) => status,
            Err(_) => return,
        },
    };
    #[cfg(target_os = "macos")]
    if restored.phase == UpdatePhase::Ready && restored.preferences.install_on_start {
        let allowed = updates
            .data
            .lock()
            .ok()
            .and_then(|data| {
                data.selected
                    .as_ref()
                    .map(|selected| selected.release.manifest().identity.clone())
            })
            .is_some_and(|identity| {
                crate::app_update_install::automatic_allowed(&updates.root, &identity)
            });
        if allowed {
            let _ = updates.install(app, true).await;
            return;
        }
    }
    if !restored.preferences.automatic_check {
        return;
    }
    match updates.check(app).await {
        Ok(status)
            if status.phase == UpdatePhase::Available && status.preferences.automatic_download =>
        {
            let _ = updates.download(app).await;
        }
        _ => {}
    }
}

async fn collect_cache(staging: Arc<Staging>) {
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let trust = TrustRoot::bundled()?;
        staging.collect(&trust)?;
        #[cfg(target_os = "macos")]
        crate::app_update_install::collect(staging.root(), &trust)?;
        Ok(())
    })
    .await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => eprintln!("update cache maintenance deferred: {error}"),
        Err(error) => eprintln!("update cache maintenance stopped: {error}"),
    }
}

fn record_failure(root: &Path, action: UpdateAction, error: &str) -> Result<(), String> {
    let record = serde_json::json!({
        "action": action,
        "at": jiff::Timestamp::now().to_string(),
        "error": error.chars().take(4096).collect::<String>(),
    });
    let mut file = tempfile::NamedTempFile::new_in(root).map_err(|error| error.to_string())?;
    serde_json::to_writer(file.as_file_mut(), &record).map_err(|error| error.to_string())?;
    file.as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    file.persist(root.join("last-failure.json"))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn installation_source(build: &BuildIdentity) -> Result<UpdateSource, String> {
    if build.release.is_none() {
        return Ok(UpdateSource::Development);
    }
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    if executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .is_some_and(|path| path.extension().is_some_and(|extension| extension == "app"))
    {
        return Ok(UpdateSource::Direct);
    }
    // An externally owned executable is never silently taken over by the SDK.
    Ok(UpdateSource::External)
}

fn read_preferences(root: &Path) -> Result<UpdatePreferences, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(root.join("preferences.json"))
        .map_err(|e| e.to_string())?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4096 {
        return Err("update preferences exceed their limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn write_preferences(root: &Path, preferences: &UpdatePreferences) -> Result<(), String> {
    let bytes = serde_json::to_vec(preferences).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(root.join("preferences.json"))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_failure_keeps_one_private_bounded_diagnostic_and_retry_stage() {
        let root = tempfile::tempdir().unwrap();
        record_failure(root.path(), UpdateAction::Check, "metadata unavailable").unwrap();
        record_failure(root.path(), UpdateAction::Install, &"错".repeat(5000)).unwrap();
        let path = root.path().join("last-failure.json");
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(record["action"], "install");
        assert_eq!(record["error"].as_str().unwrap().chars().count(), 4096);
        assert!(
            record["at"]
                .as_str()
                .unwrap()
                .parse::<jiff::Timestamp>()
                .is_ok()
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o077,
                0
            );
        }
    }
}
