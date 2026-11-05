use crate::updater_package::DesktopPackage;
use khor_gui_core::updates::{
    AppUpdateStatus, UpdateChannel, UpdatePhase, UpdatePreferences, UpdateSource,
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
            failure: preference_failure,
            blocked: Vec::new(),
            preferences,
        };
        Ok(Self {
            started: std::sync::atomic::AtomicBool::new(false),
            build,
            root,
            data: Mutex::new(Data {
                status,
                selected: None,
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
            Err(error) => self.failed(app, error),
        }
    }

    async fn check_inner(&self, app: &AppHandle) -> Result<AppUpdateStatus, String> {
        let current = self.current()?;
        let trust = TrustRoot::bundled()?;
        let channel = self.channel()?;
        let platform = Platform::current()?;
        let staging = self.store()?;
        // A cached signed selection is useful after a restart even when discovery is temporarily offline.
        if let Ok(Some((release, offer))) =
            staging.restore(&trust, &current, Component::DesktopApp, platform, channel)
        {
            let ready = staging.is_ready(&trust, &offer).unwrap_or(false);
            self.change(app, |data| {
                data.status.available = Some(offer.identity().tag.as_str().into());
                data.status.notes = release.manifest().notes.clone();
                data.status.phase = if ready {
                    UpdatePhase::Ready
                } else {
                    UpdatePhase::Available
                };
                data.selected = Some(Selection {
                    release,
                    package: None,
                });
            })?;
        }
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
            });
        };
        let offer = release
            .select(Component::DesktopApp, platform, &current, channel, None)?
            .ok_or("selected release has no application update")?;
        staging.remember(&release)?;
        collect_cache(staging.clone()).await;
        let ready = staging.is_ready(&trust, &offer).unwrap_or(false);
        let package = DesktopPackage::check(app, offer, trust).await?;
        self.change(app, |data| {
            data.status.available = Some(package.offer().identity().tag.as_str().into());
            data.status.notes = release.manifest().notes.clone();
            data.status.total = Some(package.offer().artifact().bytes);
            data.status.received = if ready {
                package.offer().artifact().bytes
            } else {
                0
            };
            data.status.checked_at = Some(jiff::Timestamp::now().to_string());
            data.status.phase = if ready {
                UpdatePhase::Ready
            } else {
                UpdatePhase::Available
            };
            data.selected = Some(Selection {
                release,
                package: Some(package),
            });
        })
    }

    pub(crate) async fn download(&self, app: &AppHandle) -> Result<AppUpdateStatus, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "an update operation is already running")?;
        let result = self.download_inner(app).await;
        match result {
            Ok(status) => Ok(status),
            Err(error) => self.failed(app, error),
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
        let current = self.current()?;
        let trust = TrustRoot::bundled()?;
        let channel = self.channel()?;
        let offer = release
            .select(
                Component::DesktopApp,
                Platform::current()?,
                &current,
                channel,
                None,
            )?
            .ok_or("no newer application update")?;
        let staging = self.store()?;
        let path = staging.archive(&offer);
        let package = DesktopPackage::check(app, offer, trust.clone()).await?;
        if !staging.is_ready(&trust, package.offer()).unwrap_or(false) {
            self.change(app, |data| {
                data.status.phase = UpdatePhase::Downloading;
                data.status.failure = None;
                data.status.received = 0;
                data.status.total = Some(package.offer().artifact().bytes);
            })?;
            let mut last = Instant::now() - Duration::from_secs(1);
            package
                .download(&path, |progress| {
                    if last.elapsed() < Duration::from_millis(100)
                        && progress.received != progress.total
                    {
                        return;
                    }
                    last = Instant::now();
                    let _ = self.change(app, |data| {
                        data.status.received = progress.received;
                    });
                })
                .await?;
        }
        self.change(app, |data| {
            data.status.phase = UpdatePhase::Ready;
            data.status.failure = None;
            data.status.received = package.offer().artifact().bytes;
            data.selected = Some(Selection {
                release,
                package: Some(package),
            });
        })
    }

    fn failed(&self, app: &AppHandle, error: String) -> Result<AppUpdateStatus, String> {
        self.change(app, |data| {
            data.status.phase = UpdatePhase::Failed;
            data.status.failure = Some(error);
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
            Err(error) => self.failed(app, error),
        }
    }

    #[cfg(target_os = "macos")]
    async fn install_inner(
        &self,
        app: &AppHandle,
        automatic: bool,
    ) -> Result<AppUpdateStatus, String> {
        let current = self.current()?;
        let trust = TrustRoot::bundled()?;
        let release = self
            .data
            .lock()
            .map_err(|_| "update state is unavailable")?
            .selected
            .as_ref()
            .map(|selection| selection.release.clone())
            .ok_or("download an update first")?;
        let offer = release
            .select(
                Component::DesktopApp,
                Platform::current()?,
                &current,
                self.channel()?,
                None,
            )?
            .ok_or("no newer application update")?;
        let staging = self.store()?;
        let path = staging.archive(&offer);
        if !staging.is_ready(&trust, &offer)? {
            return Err("download the complete application update first".into());
        }
        let package = self
            .data
            .lock()
            .map_err(|_| "update state is unavailable")?
            .selected
            .as_mut()
            .and_then(|selection| selection.package.take());
        let package = match package {
            Some(package) => package,
            None => DesktopPackage::check(app, offer.clone(), trust.clone()).await?,
        };
        self.change(app, |data| {
            data.status.phase = UpdatePhase::Verifying;
            data.status.failure = None;
            data.status.blocked.clear();
        })?;
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
        let admission = app
            .state::<std::sync::Arc<crate::update_admission::UpdateAdmission>>()
            .inner()
            .clone();
        let _permit = match admission.prepare(app, automatic).await {
            Ok(permit) => permit,
            Err(blocked) => {
                return self.change(app, |data| {
                    data.status.phase = UpdatePhase::Ready;
                    data.status.blocked = blocked;
                });
            }
        };
        self.change(app, |data| {
            data.status.phase = UpdatePhase::Installing;
        })?;
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
        let status = self.change(app, |data| {
            data.status.phase = UpdatePhase::AwaitingHealth;
        })?;
        app.exit(0);
        Ok(status)
    }
    fn restore_cached(&self, app: &AppHandle) -> Result<AppUpdateStatus, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "an update operation is already running")?;
        if self.status()?.source != UpdateSource::Direct {
            return self.status();
        }
        let trust = TrustRoot::bundled()?;
        let current = self.current()?;
        let staging = self.store()?;
        if let Ok(Some((release, offer))) = staging.restore(
            &trust,
            &current,
            Component::DesktopApp,
            Platform::current()?,
            self.channel()?,
        ) {
            let ready = staging.is_ready(&trust, &offer).unwrap_or(false);
            return self.change(app, |data| {
                data.status.available = Some(offer.identity().tag.as_str().into());
                data.status.notes = release.manifest().notes.clone();
                data.status.phase = if ready {
                    UpdatePhase::Ready
                } else {
                    UpdatePhase::Available
                };
                data.status.total = Some(offer.artifact().bytes);
                data.status.received = if ready { offer.artifact().bytes } else { 0 };
                data.selected = Some(Selection {
                    release,
                    package: None,
                });
            });
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
        let updates = app.state::<AppUpdates>();
        updates.maintain().await;
        let Ok(restored) = updates.restore_cached(&app) else {
            return;
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
                let _ = updates.install(&app, true).await;
                return;
            }
        }
        if !restored.preferences.automatic_check {
            return;
        }
        match updates.check(&app).await {
            Ok(status)
                if status.phase == UpdatePhase::Available
                    && status.preferences.automatic_download =>
            {
                let _ = updates.download(&app).await;
            }
            _ => {}
        }
    });
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
