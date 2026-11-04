use khor_gui_core::updates::{AppUpdateStatus, UpdatePreferences};
use tauri::Manager;

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
