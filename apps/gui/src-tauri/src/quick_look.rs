//! Read-only original-file previews. Download/version ownership stays in the
//! file handoff core; only this boundary knows the platform presenter.
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "macos")]
mod macos;

pub async fn open(view: tauri::Webview, file: khor_gui_core::file_handoff::ReadyFile) -> Result<bool, String> {
    #[cfg(target_os = "ios")]
    {
        ios::open(view, file).await
    }
    #[cfg(target_os = "macos")]
    {
        macos::open(view, file).await
    }
}
#[cfg(target_os = "macos")]
pub fn menu(id: &str) -> bool {
    macos::menu(id)
}
