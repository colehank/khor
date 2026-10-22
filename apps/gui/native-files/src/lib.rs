//! Rust-only Android original-file opening. The renderer submits a
//! completed job ID to the app, never an arbitrary path to this plugin.
#[cfg(target_os = "android")]
struct Files<R: tauri::Runtime>(tauri::plugin::PluginHandle<R>);

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("khor-files")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                use tauri::Manager;
                let handle = _api.register_android_plugin("cloud.khor.files", "KhorFilesPlugin")?;
                _app.manage(Files(handle));
            }
            Ok(())
        })
        .build()
}
#[cfg(target_os = "android")]
pub fn open<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    path: &std::path::Path,
) -> Result<(), String> {
    use tauri::Manager;
    app.state::<Files<R>>()
        .0
        .run_mobile_plugin::<serde_json::Value>(
            "open",
            serde_json::json!({"path":path.to_string_lossy()}),
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}
