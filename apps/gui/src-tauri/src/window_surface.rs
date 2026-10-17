//! Desktop background paint, independently of NSApp/UI theme.
//! The page draws the tint once; the native window supplies the desktop
//! material behind it and remains clear whenever that tint has alpha.
use khor_node::Node;
use tauri::{AppHandle, Manager};

pub fn material() -> &'static str {
    khor_node::config::get(&Node::root_from_env(), "window-material").unwrap_or("solid")
}

pub fn background(dark: bool) -> tauri::webview::Color {
    if material() == "solid" {
        super::ground(dark)
    } else {
        tauri::webview::Color(0, 0, 0, 0)
    }
}

pub fn apply(app: &AppHandle, material: &str) {
    let Some(window) = app.get_window("main") else {
        return;
    };
    let dark = super::chosen_dark(|| matches!(window.theme(), Ok(tauri::Theme::Dark)));
    let color = if material == "solid" {
        super::ground(dark)
    } else {
        tauri::webview::Color(0, 0, 0, 0)
    };
    let _ = window.set_background_color(Some(color));
    // WebviewWindowBuilder sets both grounds at launch. They must both
    // change at runtime too: clearing only NSWindow leaves WKWebView's
    // original opaque ground behind the page's translucent tint.
    if let Some(view) = app.get_webview("main") {
        let _ = view.set_background_color(Some(color));
    }
    #[cfg(target_os = "macos")]
    {
        use tauri::window::{Effect, EffectsBuilder};
        // Blur and tint are independent: NSVisualEffectView materials add
        // their own grey even when the page's background opacity is zero.
        // Prefer an untinted WindowServer blur; older systems retain the
        // supported system material if the optional symbols are unavailable.
        let _ = window.set_effects(None);
        let clean = mac_blur::apply(&window, material == "frost", material == "solid");
        if material == "frost" && !clean {
            let _ = window.set_effects(Some(
                EffectsBuilder::new()
                    .effect(Effect::UnderWindowBackground)
                    .build(),
            ));
        }
    }
    #[cfg(target_os = "windows")]
    {
        use tauri::window::{Effect, EffectsBuilder};
        // Tauri chooses the first supported material, in OS order.
        let effects = (material == "frost").then(|| {
            EffectsBuilder::new()
                .effects([Effect::Mica, Effect::Acrylic, Effect::Blur])
                .build()
        });
        let _ = window.set_effects(effects);
    }
    #[cfg(desktop)]
    sync_frame(app, true);
    // Linux transparency belongs to the compositor. Native blur is not
    // offered there; an imported frost preference becomes clear tint.
}

pub fn initial_script() -> String {
    let opacity = khor_node::config::get(&Node::root_from_env(), "window-opacity")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(100);
    format!(
        "window.__KHOR_WINDOW_SURFACE__ = true; window.__KHOR_WINDOW_PREF__ = {}; window.__KHOR_WINDOW_PLATFORM__ = {};",
        serde_json::json!({"material": material(), "opacity": opacity, "outline": khor_node::config::get(&Node::root_from_env(), "window-outline").unwrap_or("on") != "off"}),
        serde_json::json!(std::env::consts::OS)
    )
}

/// macOS only, called on the AppKit thread by setup / set_window_appearance.
/// These optional WindowServer symbols are resolved at runtime, never linked
/// as mandatory APIs. The radius affects the backdrop, not window/content alpha.
#[cfg(target_os = "macos")]
mod mac_blur {
    use std::sync::OnceLock;
    type Connection = unsafe extern "C" fn() -> i32;
    type Blur = unsafe extern "C" fn(i32, i32, usize) -> i32;
    struct Api {
        connection: Connection,
        blur: Blur,
    }
    static API: OnceLock<Option<Api>> = OnceLock::new();
    const RADIUS: usize = 25;

    pub fn apply(window: &tauri::Window, enabled: bool, shadow: bool) -> bool {
        use objc2::{msg_send, runtime::AnyObject};
        let Ok(ns) = window.ns_window() else {
            return false;
        };
        if ns.is_null() {
            return false;
        }
        // SAFETY: owned NSWindow, on the AppKit thread. A fully transparent
        // window's native shadow follows its glyphs and controls, producing
        // dark outlines. Keep the shadow only for an opaque window.
        unsafe {
            let _: () = msg_send![ns as *mut AnyObject, setHasShadow: shadow];
        }
        let Some(api) = API.get_or_init(|| unsafe {
            // SAFETY: AppKit loads CoreGraphics before a window is created.
            // Check both pointers before converting their verified C signatures.
            let connection = libc::dlsym(
                libc::RTLD_DEFAULT,
                c"CGSDefaultConnectionForThread".as_ptr(),
            );
            let blur = libc::dlsym(
                libc::RTLD_DEFAULT,
                c"CGSSetWindowBackgroundBlurRadius".as_ptr(),
            );
            if connection.is_null() || blur.is_null() {
                return None;
            }
            Some(Api {
                connection: std::mem::transmute::<*mut libc::c_void, Connection>(connection),
                blur: std::mem::transmute::<*mut libc::c_void, Blur>(blur),
            })
        }) else {
            return false;
        };
        // SAFETY: window owns this NSWindow; callers run on the AppKit thread.
        unsafe {
            let number: isize = msg_send![ns as *mut AnyObject, windowNumber];
            let connection = (api.connection)();
            if connection == 0 || number <= 0 || number > i32::MAX as isize {
                return false;
            }
            (api.blur)(connection, number as i32, if enabled { RADIUS } else { 0 }) == 0
        }
    }
}

/// Keep the fallback edge aligned with native full-screen/maximized shapes.
/// Only state changes cross the webview boundary, not every resize tick.
#[cfg(desktop)]
pub fn sync_frame(app: &AppHandle, force: bool) {
    use std::sync::atomic::{AtomicU8, Ordering};
    static LAST: AtomicU8 = AtomicU8::new(u8::MAX);
    let Some(window) = app.get_window("main") else { return; };
    let full = window.is_fullscreen().unwrap_or(false);
    let square = full || window.is_maximized().unwrap_or(false);
    let flags = u8::from(full) | (u8::from(square) << 1);
    if LAST.swap(flags, Ordering::Relaxed) == flags && !force { return; }
    if let Some(view) = app.get_webview("main") {
        let script = format!("document.documentElement.toggleAttribute('data-window-fullscreen',{full});document.documentElement.toggleAttribute('data-window-square',{square});");
        let _ = view.eval(script);
    }
}
