//! The same trusted frontend menu, in a keyless desktop window.

use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize};

use super::{MenuRequest, MenuSnapshot, MenuSpot};

const MENU: &str = "menu";

fn measured() -> &'static std::sync::Mutex<Option<(String, f64)>> {
    static HEIGHT: std::sync::OnceLock<std::sync::Mutex<Option<(String, f64)>>> =
        std::sync::OnceLock::new();
    HEIGHT.get_or_init(Default::default)
}

pub fn hang(app: &tauri::AppHandle) {
    let built = tauri::WebviewWindowBuilder::new(
        app,
        MENU,
        tauri::WebviewUrl::App("index.html?face=menu".into()),
    )
    .title("")
    .decorations(false)
    .resizable(false)
    .skip_taskbar(true)
    .focusable(false)
    .accept_first_mouse(true)
    .shadow(true)
    .always_on_top(true)
    .visible(false)
    .inner_size(1.0, 1.0)
    .build();
    if let Err(e) = built {
        eprintln!("khor: the overlay window did not build, menus stay in the page: {e}");
    }
}

fn trusted_renderer(window: &tauri::Window) -> Result<(), String> {
    if window.label() != MENU {
        return Err("this window is not the menu renderer".into());
    }
    Ok(())
}

/// CSS pixels become screen physical pixels exactly once. Screen bounds can
/// have negative origins on a monitor to the left of the main display.
fn place(
    owner: &tauri::Window,
    at: &MenuSpot,
    height: f64,
) -> Result<(i32, i32, u32, u32), String> {
    let scale = owner.scale_factor().map_err(|e| e.to_string())?;
    if !scale.is_finite() || scale <= 0.0 {
        return Err("invalid window scale".into());
    }
    let frame = owner.outer_size().map_err(|e| e.to_string())?;
    let origin = owner.outer_position().map_err(|e| e.to_string())?;
    let side = ((f64::from(frame.width) - at.across * scale) / 2.0).max(0.0);
    let down = ((f64::from(frame.height) - at.view * scale) - side).max(0.0);
    let mut w = (at.w * scale).max(1.0);
    let mut h = (height * scale).max(1.0);
    let mut x = f64::from(origin.x) + side + at.x * scale;
    let anchor = f64::from(origin.y) + down + at.y * scale;
    let mut y = if at.above { anchor - h } else { anchor };
    if let Some(monitor) = owner.current_monitor().map_err(|e| e.to_string())? {
        let pos = monitor.position();
        let size = monitor.size();
        w = w.min(f64::from(size.width));
        h = h.min(f64::from(size.height));
        x = x.clamp(
            f64::from(pos.x),
            f64::from(pos.x) + f64::from(size.width) - w,
        );
        y = y.clamp(
            f64::from(pos.y),
            f64::from(pos.y) + f64::from(size.height) - h,
        );
    }
    Ok((
        x.round() as i32,
        y.round() as i32,
        w.round() as u32,
        h.round() as u32,
    ))
}

fn position(app: &tauri::AppHandle, height: f64) -> Result<(), String> {
    let Some(held) = super::held() else {
        return Ok(());
    };
    let Some(owner) = app.get_window(&held.owner) else {
        return Ok(());
    };
    let Some(menu) = app.get_webview_window(MENU) else {
        return Ok(());
    };
    let (x, y, w, h) = place(&owner, &held.request.at, height)?;
    menu.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    menu.set_size(PhysicalSize::new(w, h))
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn show(window: &tauri::Window, request: &MenuRequest) -> Result<bool, String> {
    request.validate()?;
    let app = window.app_handle();
    let Some(menu) = app.get_webview_window(MENU) else {
        return Ok(false);
    };
    let replaced = super::held().is_some_and(|held| held.request.id != request.id);
    super::claim(window.label(), request)?;
    if replaced {
        menu.hide().map_err(|e| e.to_string())?;
    }
    // The renderer reports its actual height before showing; no estimate or
    // stale native width is read back from the asynchronous window server.
    app.emit_to(MENU, "menu:rows", request.snapshot())
        .map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn hide(window: &tauri::Window, id: &str) -> Result<(), String> {
    if !super::dismiss(window.label(), id) {
        return Ok(());
    }
    if let Some(menu) = window.app_handle().get_webview_window(MENU) {
        menu.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn size(window: &tauri::Window, id: &str, h: f64) -> Result<(), String> {
    trusted_renderer(window)?;
    if !super::accepts_size(id, h) {
        return Ok(());
    }
    if let Ok(mut measured) = measured().lock() {
        *measured = Some((id.into(), h));
    }
    position(window.app_handle(), h)?;
    let focused = super::held()
        .and_then(|held| window.app_handle().get_window(&held.owner))
        .is_some_and(|owner| owner.is_focused().unwrap_or(false));
    // A measurement can arrive after the owner lost focus. It must not
    // resurrect an always-on-top window over another application.
    if focused && let Some(menu) = window.app_handle().get_webview_window(MENU) {
        menu.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn rows(window: &tauri::Window) -> Result<Option<MenuSnapshot>, String> {
    trusted_renderer(window)?;
    Ok(super::snapshot())
}

pub fn pick(window: &tauri::Window, id: &str, key: &str) -> Result<(), String> {
    trusted_renderer(window)?;
    let Some(owner) = super::pick_owner(id, key) else {
        return Ok(());
    };
    window
        .app_handle()
        .emit_to(
            owner,
            "menu:pick",
            serde_json::json!({ "id": id, "key": key }),
        )
        .map_err(|e| e.to_string())
}

pub fn owner_focus(window: &tauri::Window, focused: bool) {
    let Some(held) = super::held().filter(|held| held.owner == window.label()) else {
        return;
    };
    let ready = measured()
        .lock()
        .is_ok_and(|m| m.as_ref().is_some_and(|(id, _)| id == &held.request.id));
    if let Some(menu) = window.app_handle().get_webview_window(MENU) {
        let _ = if focused && ready {
            menu.show()
        } else {
            menu.hide()
        };
    }
}

pub fn owner_moved(window: &tauri::Window) {
    let Some(held) = super::held().filter(|held| held.owner == window.label()) else {
        return;
    };
    let height = measured().lock().ok().and_then(|m| {
        m.as_ref()
            .filter(|(id, _)| id == &held.request.id)
            .map(|(_, h)| *h)
    });
    if let Some(h) = height {
        let _ = position(window.app_handle(), h);
    }
}

pub fn close(app: &tauri::AppHandle) {
    if let Some(menu) = app.get_webview_window(MENU) {
        let _ = menu.close();
    }
}
