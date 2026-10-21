#![allow(unexpected_cfgs, dead_code)]
extern crate self as tauri;
use std::sync::{Mutex, OnceLock};
use std::ffi::{c_char, c_void, CStr, CString};

#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit_to<T: serde::Serialize>(&self, owner: impl AsRef<str>, event: &str, payload: T) -> Result<(), String>;
}
fn events() -> &'static Mutex<Vec<serde_json::Value>> {
    static EVENTS: OnceLock<Mutex<Vec<serde_json::Value>>> = OnceLock::new();
    EVENTS.get_or_init(Default::default)
}
impl Emitter for AppHandle {
    fn emit_to<T: serde::Serialize>(&self, owner: impl AsRef<str>, event: &str, payload: T) -> Result<(), String> {
        events().lock().unwrap().push(serde_json::json!({"owner":owner.as_ref(),"event":event,"payload":payload}));
        Ok(())
    }
}
mod menu_overlay;

unsafe fn text(ptr: *const c_char) -> String { unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned() }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_show(face: *mut c_void, html: *const c_char, id: *const c_char, above: bool, x: f64, y: f64, w: f64, height: f64, width: f64) -> bool {
    let request = menu_overlay::MenuRequest {
        id: unsafe { text(id) },
        at: menu_overlay::MenuSpot{x,y,w,view:height,across:width,above},
        rows: serde_json::json!({"groups":[{"candidates":[{"key":"m:local"},{"key":"m:remote"}]}],"at":0}),
        html: Some(unsafe { text(html) }),
    };
    match menu_overlay::ios::show(&AppHandle,face.cast(),"main",request) {
        Ok(took) => took,
        Err(error) => { eprintln!("probe show failed: {error}"); false }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_hide(id: *const c_char) { menu_overlay::ios::hide("main", &unsafe { text(id) }); }
#[unsafe(no_mangle)]
pub extern "C" fn probe_active() -> bool { menu_overlay::held().is_some() }
#[unsafe(no_mangle)]
pub extern "C" fn probe_events() -> *mut c_char {
    CString::new(serde_json::to_string(&*events().lock().unwrap()).unwrap()).unwrap().into_raw()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_free(ptr: *mut c_char) { if !ptr.is_null() { let _ = unsafe { CString::from_raw(ptr) }; } }
#[unsafe(no_mangle)]
pub extern "C" fn probe_clear() { events().lock().unwrap().clear(); }
