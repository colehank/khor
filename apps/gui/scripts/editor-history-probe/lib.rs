extern crate self as tauri;
pub struct AppHandle;
pub struct Window;
pub struct Webview;
pub struct PlatformWebview;
pub trait Manager {
    fn get_window(&self, _: &str) -> Option<Window>;
    fn get_webview(&self, _: &str) -> Option<Webview>;
}
impl Manager for AppHandle {
    fn get_window(&self, _: &str) -> Option<Window> {
        None
    }
    fn get_webview(&self, _: &str) -> Option<Webview> {
        None
    }
}
impl AppHandle {
    pub fn run_on_main_thread(&self, _: impl FnOnce() + Send + 'static) -> Result<(), ()> {
        Ok(())
    }
}
impl Window {
    pub fn is_focused(&self) -> Result<bool, ()> {
        Ok(false)
    }
}
impl Webview {
    pub fn with_webview(&self, _: impl FnOnce(PlatformWebview) + Send + 'static) -> Result<(), ()> {
        Ok(())
    }
}
impl PlatformWebview {
    pub fn inner(&self) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }
}
#[path = "__KHOR_NATIVE_EDIT_SOURCE__"]
mod native_edit;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn khor_native_edit(web: *mut std::ffi::c_void, redo: bool) {
    unsafe {
        native_edit::perform(web.cast(), redo);
    }
}
