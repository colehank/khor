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
        let effects = (material == "frost").then(|| {
            EffectsBuilder::new()
                .effect(Effect::UnderWindowBackground)
                .build()
        });
        let _ = window.set_effects(effects);
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
    // Linux transparency belongs to the compositor. Native blur is not
    // offered there; an imported frost preference becomes clear tint.
}

pub fn initial_script() -> String {
    let opacity = khor_node::config::get(&Node::root_from_env(), "window-opacity")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(100);
    format!(
        "window.__KHOR_WINDOW_SURFACE__ = true; window.__KHOR_WINDOW_PREF__ = {};",
        serde_json::json!({"material": material(), "opacity": opacity})
    )
}
