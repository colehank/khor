use khor_gui_core::updates::{AppUpdateStatus, UpdatePreferences};
use tauri::{Emitter, Manager};

fn authorize(webview: &tauri::Webview) -> Result<(), String> {
    let label = webview.label();
    if label != "main" && label != "settings" && !label.starts_with("code:") {
        return Err("updates are only available from the application interface".into());
    }
    let url = webview.url().map_err(|e| e.to_string())?;
    let bundled = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || (matches!(url.scheme(), "http" | "https") && url.host_str() == Some("tauri.localhost"));
    #[cfg(dev)]
    let bundled = bundled
        || (url.scheme() == "http"
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))
            && url.port() == Some(1430));
    if !bundled {
        return Err("updates are only available from the bundled application interface".into());
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn app_update_runtime_cache(
    webview: tauri::Webview,
    clean: bool,
) -> Result<khor_gui_core::updates::RuntimeCacheUsage, String> {
    authorize(&webview)?;
    #[cfg(desktop)]
    return crate::off_main(move || khor_gui_core::updates::runtime_cache(clean)).await;
    #[cfg(mobile)]
    {
        let _ = clean;
        Err("runtime caches are managed by the host machine".into())
    }
}

#[tauri::command]
pub(crate) async fn app_update_prepared(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    nonce: String,
    blocked: Vec<khor_gui_core::updates::UpdateBlocker>,
) -> Result<(), String> {
    authorize(&webview)?;
    #[cfg(desktop)]
    return app
        .state::<std::sync::Arc<crate::update_admission::UpdateAdmission>>()
        .reply(webview.label(), &nonce, blocked);
    #[cfg(mobile)]
    {
        let _ = (app, nonce, blocked);
        Err("this application is updated through its store".into())
    }
}

#[tauri::command]
pub(crate) async fn app_update_status(
    app: tauri::AppHandle,
    webview: tauri::Webview,
) -> Result<AppUpdateStatus, String> {
    authorize(&webview)?;
    #[cfg(desktop)]
    return app.state::<crate::app_updates::AppUpdates>().status();
    #[cfg(mobile)]
    {
        let _ = app;
        Ok(store_status())
    }
}

#[tauri::command]
pub(crate) async fn app_update_check(
    app: tauri::AppHandle,
    webview: tauri::Webview,
) -> Result<AppUpdateStatus, String> {
    authorize(&webview)?;
    #[cfg(desktop)]
    return app
        .state::<crate::app_updates::AppUpdates>()
        .check(&app)
        .await;
    #[cfg(mobile)]
    {
        let _ = app;
        Ok(store_status())
    }
}

#[tauri::command]
pub(crate) async fn app_update_download(
    app: tauri::AppHandle,
    webview: tauri::Webview,
) -> Result<AppUpdateStatus, String> {
    authorize(&webview)?;
    #[cfg(desktop)]
    return app
        .state::<crate::app_updates::AppUpdates>()
        .download(&app)
        .await;
    #[cfg(mobile)]
    {
        let _ = app;
        Err("this application is updated through its store".into())
    }
}

#[tauri::command]
pub(crate) async fn app_update_preferences(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    preferences: UpdatePreferences,
) -> Result<AppUpdateStatus, String> {
    authorize(&webview)?;
    #[cfg(desktop)]
    return app
        .state::<crate::app_updates::AppUpdates>()
        .preferences(&app, preferences)
        .await;
    #[cfg(mobile)]
    {
        let _ = (app, preferences);
        Err("this application is updated through its store".into())
    }
}

#[tauri::command]
pub(crate) async fn app_update_install(
    app: tauri::AppHandle,
    webview: tauri::Webview,
) -> Result<AppUpdateStatus, String> {
    authorize(&webview)?;
    #[cfg(all(desktop, target_os = "macos"))]
    return app
        .state::<crate::app_updates::AppUpdates>()
        .install(&app, false)
        .await;
    #[cfg(not(all(desktop, target_os = "macos")))]
    {
        let _ = app;
        Err("this application is updated through its original installer".into())
    }
}

#[tauri::command]
pub(crate) async fn app_update_ui_ready(
    app: tauri::AppHandle,
    webview: tauri::Webview,
) -> Result<(), String> {
    authorize(&webview)?;
    if webview.label() != "main" {
        return Ok(());
    }
    #[cfg(all(desktop, target_os = "macos"))]
    crate::off_main(crate::app_update_install::ui_ready).await?;
    #[cfg(desktop)]
    crate::app_updates::start(&app);
    #[cfg(mobile)]
    let _ = app;
    Ok(())
}

#[cfg(mobile)]
fn store_status() -> AppUpdateStatus {
    use khor_gui_core::updates::{UpdateChannel, UpdatePhase, UpdateSource};
    AppUpdateStatus {
        source: UpdateSource::Store,
        phase: UpdatePhase::Idle,
        current: env!("CARGO_PKG_VERSION").into(),
        built_from: None,
        source_commit: None,
        available: None,
        notes: String::new(),
        received: 0,
        total: None,
        checked_at: None,
        failure: None,
        blocked: Vec::new(),
        preferences: UpdatePreferences::new(UpdateChannel::Stable),
    }
}

#[tauri::command]
pub(crate) async fn machine_update_status(
    webview: tauri::Webview,
    machine: String,
) -> Result<khor_gui_core::updates::MachineInstallation, String> {
    authorize(&webview)?;
    khor_gui_core::updates::machine_installation(&khor_node::Node::root_from_env(), &machine).await
}

#[tauri::command]
pub(crate) async fn machine_update_check(
    webview: tauri::Webview,
    machine: String,
    channel: Option<khor_gui_core::updates::UpdateChannel>,
) -> Result<khor_gui_core::updates::MachineUpdateResult, String> {
    authorize(&webview)?;
    khor_gui_core::updates::machine_update(
        &khor_node::Node::root_from_env(),
        &machine,
        true,
        channel,
    )
    .await
}

#[tauri::command]
pub(crate) async fn machine_update_queue(
    app: tauri::AppHandle,
    webview: tauri::Webview,
) -> Result<khor_gui_core::updates::MachineUpdateQueue, String> {
    authorize(&webview)?;
    app.state::<std::sync::Arc<crate::machine_updates::MachineUpdates>>()
        .snapshot()
}

#[tauri::command]
pub(crate) async fn machine_update_start(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    machines: Vec<String>,
    channel: Option<khor_gui_core::updates::UpdateChannel>,
) -> Result<khor_gui_core::updates::MachineUpdateQueue, String> {
    authorize(&webview)?;
    let manager = app.state::<std::sync::Arc<crate::machine_updates::MachineUpdates>>();
    manager.enqueue(machines, channel)?;
    let status = manager.snapshot()?;
    let _ = app.emit("machine-updates:changed", &status);
    crate::machine_updates::MachineUpdates::kick(&app);
    Ok(status)
}

#[tauri::command]
pub(crate) async fn machine_update_cancel(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    machine: String,
    request: String,
) -> Result<khor_gui_core::updates::MachineUpdateQueue, String> {
    authorize(&webview)?;
    let manager = app.state::<std::sync::Arc<crate::machine_updates::MachineUpdates>>();
    manager.cancel_waiting(&machine, &request)?;
    let status = manager.snapshot()?;
    let _ = app.emit("machine-updates:changed", &status);
    Ok(status)
}
