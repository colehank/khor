//! **The menu bar** (2026-09-28: 「像一个 Mac app」).
//!
//! Tauri's default menu was what khor wore until now: English, and with
//! nothing of khor's in it — so no shortcut was discoverable, which is how
//! Mac people learn them — and with ⌘W bound to 「Close Window」, which on
//! khor's one window stops the node the app runs. This builds a real one,
//! macOS only (a menu set on a Windows or Linux window becomes a row inside
//! the window, and the skeleton has no such row; there the page's own
//! keydown is the one road, `src/lib/shortcuts.ts`).
//!
//! # One act, one handler, two roads
//!
//! Every khor item's id is an act name from `src/lib/shortcuts.ts`
//! (`landing-1` … `close-tab`), and picking it emits that name to the
//! page (`shortcut`), which runs the **same** function its keydown runs.
//! A key is handled once because of how WKWebView orders the two: the
//! page sees a key equivalent first, and only a key the page did not
//! `preventDefault` is re-sent to the menu (`WebViewImpl::doneWithKeyEvent`).
//! The page prevents every key it runs — so a press in the window runs
//! there, and the accelerator here fires only where the page never saw the
//! key: focus in a browsing tab's own webview, or an editor window.
//! `npm run native` counts a press through the real WKWebView to hold this.
//!
//! # Words
//!
//! From the catalog's `[menu]` section (and the rail's five names from
//! `[gui]`), in the language the process speaks. A language change
//! rebuilds the menu (`follow_lang`), in 设置 or from `khor config`.
//!
//! # Enabled
//!
//! The page says what applies (`menu_state`): whether ⌘W has anything to
//! close, whether there are two tabs to step between, what the landing's
//! 「+」 is called, whether ⌘L can lock. 「新建…」 takes the 「+」's own
//! name, so the menu says what ⌘N will really do.

#[cfg(target_os = "macos")]
use std::sync::Mutex;

#[cfg(target_os = "macos")]
use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu, WINDOW_SUBMENU_ID};
use tauri::{AppHandle, Emitter, Manager};

/// The acts the page runs, by the ids their items carry. The same names
/// as `Act` in `src/lib/shortcuts.ts`.
const ACTS: [&str; 13] = [
    "landing-1",
    "landing-2",
    "landing-3",
    "landing-4",
    "landing-5",
    "settings",
    "lock",
    "find",
    "new",
    "import",
    "close-tab",
    "next-tab",
    "prev-tab",
];

/// What the page says applies right now.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct MenuState {
    /// ⌘W has something to close: a 「…」 page, or a front tab.
    pub close: bool,
    /// Two tabs or more to step between.
    pub step: bool,
    /// The landing's 「+」, by its own name; `None` where there is none.
    pub new: Option<String>,
    /// ⌘L would lock: a PIN is set and the lock is on.
    pub lock: bool,
    /// The face is locked — nothing that walks the window applies.
    pub locked: bool,
}

#[cfg(target_os = "macos")]
struct Items {
    new: MenuItem<tauri::Wry>,
    import: MenuItem<tauri::Wry>,
    close_tab: MenuItem<tauri::Wry>,
    next: MenuItem<tauri::Wry>,
    prev: MenuItem<tauri::Wry>,
    lock: MenuItem<tauri::Wry>,
    settings: MenuItem<tauri::Wry>,
    find: MenuItem<tauri::Wry>,
    landings: Vec<MenuItem<tauri::Wry>>,
}

/// The items the page's state lands on, the state itself (so a menu
/// rebuilt in another language comes up the same), and the language the
/// menu was built in.
#[derive(Default)]
pub struct Held {
    #[cfg(target_os = "macos")]
    items: Mutex<Option<Items>>,
    #[cfg(target_os = "macos")]
    state: Mutex<MenuState>,
    #[cfg(target_os = "macos")]
    built_in: Mutex<Option<khor_catalog::lang::Lang>>,
}

/// Builds the menu and puts it on the app. Called once from `setup`, and
/// again whenever the language changes.
#[cfg(target_os = "macos")]
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    use khor_catalog::{gui, menu as m};
    let sep = || PredefinedMenuItem::separator(app);
    let item =
        |id: &str, text: &str, key: Option<&str>| MenuItem::with_id(app, id, text, true, key);

    let about = PredefinedMenuItem::about(
        app,
        Some(m::about()),
        Some(AboutMetadata {
            name: Some("khor".into()),
            version: Some(app.package_info().version.to_string()),
            ..Default::default()
        }),
    )?;
    let settings = item("settings", m::settings(), Some("CmdOrCtrl+,"))?;
    let lock = item("lock", m::lock(), Some("CmdOrCtrl+L"))?;
    let app_menu = Submenu::with_items(
        app,
        "khor",
        true,
        &[
            &about,
            &sep()?,
            &settings,
            &lock,
            &sep()?,
            &PredefinedMenuItem::services(app, Some(m::services()))?,
            &sep()?,
            &PredefinedMenuItem::hide(app, Some(m::hide()))?,
            &PredefinedMenuItem::hide_others(app, Some(m::hide_others()))?,
            &PredefinedMenuItem::show_all(app, Some(m::show_all()))?,
            &sep()?,
            &PredefinedMenuItem::quit(app, Some(m::quit()))?,
        ],
    )?;

    let new = item("new", m::new(), Some("CmdOrCtrl+N"))?;
    let import = item("import", m::import(), None)?;
    let close_tab = item("close-tab", m::close_tab(), Some("CmdOrCtrl+W"))?;
    // **Not the predefined 「Close Window」**: that one is welded to ⌘W,
    // which is the tab's here. Same act — the front window's close, which
    // on khor's own window is the red button's road through the draft
    // guard (`onQuitRequested`).
    let close_window = item("close-window", m::close_window(), Some("CmdOrCtrl+Shift+W"))?;
    let file = Submenu::with_items(
        app,
        m::file(),
        true,
        &[&new, &import, &sep()?, &close_tab, &close_window],
    )?;

    // **The Edit menu is load-bearing, not a courtesy.** WKWebView has no
    // key binding of its own for ⌘C / ⌘V / ⌘X / ⌘A / ⌘Z: they reach a text
    // field only as these items' actions (`copy:` and the rest) sent down
    // the responder chain. Without them a text field cannot paste.
    let edit = Submenu::with_items(
        app,
        m::edit(),
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(m::undo()))?,
            &PredefinedMenuItem::redo(app, Some(m::redo()))?,
            &sep()?,
            &PredefinedMenuItem::cut(app, Some(m::cut()))?,
            &PredefinedMenuItem::copy(app, Some(m::copy()))?,
            &PredefinedMenuItem::paste(app, Some(m::paste()))?,
            &PredefinedMenuItem::select_all(app, Some(m::select_all()))?,
        ],
    )?;

    let names = [
        gui::sessions_tab(),
        gui::devices_tab(),
        gui::workspaces_tab(),
        gui::accounts_tab(),
        gui::browser_tab(),
    ];
    let mut landings = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let id = format!("landing-{}", i + 1);
        let key = format!("CmdOrCtrl+{}", i + 1);
        landings.push(item(&id, name, Some(key.as_str()))?);
    }
    let find = item("find", m::find(), Some("CmdOrCtrl+K"))?;
    let view = Submenu::with_items(
        app,
        m::view(),
        true,
        &[
            &landings[0],
            &landings[1],
            &landings[2],
            &landings[3],
            &landings[4],
            &sep()?,
            &find,
            &sep()?,
            &PredefinedMenuItem::fullscreen(app, Some(m::fullscreen()))?,
        ],
    )?;

    // ⌃⇥ is shown and never matched here: muda writes Tab's key
    // equivalent as the glyph 「⇥」, which no key press produces. The page
    // runs ⌃⇥ (and ⇧⌘] ⇧⌘[, which are not listed) — Safari's arrangement,
    // where the item says ⌃⇥ and both spellings work.
    let prev = item("prev-tab", m::prev_tab(), Some("Ctrl+Shift+Tab"))?;
    let next = item("next-tab", m::next_tab(), Some("Ctrl+Tab"))?;
    // The window list: AppKit fills it in under the menu it is told is
    // the windows menu, which is what this id asks tauri to do.
    let window = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        m::window(),
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some(m::minimize()))?,
            &PredefinedMenuItem::maximize(app, Some(m::zoom()))?,
            &sep()?,
            &prev,
            &next,
            &sep()?,
            &PredefinedMenuItem::bring_all_to_front(app, Some(m::front()))?,
        ],
    )?;
    // No 帮助 menu: there is nothing behind one yet, and an empty menu is
    // a menu that teaches people not to open menus.

    let menu = Menu::with_items(app, &[&app_menu, &file, &edit, &view, &window])?;
    app.set_menu(menu)?;

    let held = app.state::<Held>();
    let items = Items {
        new,
        import,
        close_tab,
        next,
        prev,
        lock,
        settings,
        find,
        landings,
    };
    let state = held.state.lock().map(|s| s.clone()).unwrap_or_default();
    apply(&items, &state);
    if let Ok(mut slot) = held.items.lock() {
        *slot = Some(items);
    }
    if let Ok(mut lang) = held.built_in.lock() {
        *lang = Some(khor_catalog::lang::current());
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn install(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

/// Puts the page's state on the items.
#[cfg(target_os = "macos")]
fn apply(items: &Items, state: &MenuState) {
    let walk = !state.locked;
    let _ = items.new.set_enabled(walk && state.new.is_some());
    let text = match state.new.as_deref() {
        Some(named) => named,
        None => khor_catalog::menu::new(),
    };
    let _ = items.new.set_text(text);
    let _ = items.import.set_enabled(walk);
    let _ = items.close_tab.set_enabled(walk && state.close);
    let _ = items.next.set_enabled(walk && state.step);
    let _ = items.prev.set_enabled(walk && state.step);
    let _ = items.lock.set_enabled(walk && state.lock);
    let _ = items.settings.set_enabled(walk);
    let _ = items.find.set_enabled(walk);
    for l in &items.landings {
        let _ = l.set_enabled(walk);
    }
}

/// The page's word on what applies (`src/lib/shortcuts.ts`, on change).
#[cfg(target_os = "macos")]
pub fn set_state(app: &AppHandle, state: MenuState) {
    let held = app.state::<Held>();
    if let Ok(items) = held.items.lock()
        && let Some(items) = items.as_ref()
    {
        apply(items, &state);
    }
    if let Ok(mut s) = held.state.lock() {
        *s = state;
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_state(_app: &AppHandle, _state: MenuState) {}

/// Rebuilds the menu when the process has changed language since it was
/// built — after 设置 wrote `lang`, and when the window comes back to the
/// front (`khor config set lang` in a terminal is the other road).
#[cfg(target_os = "macos")]
pub fn follow_lang(app: &AppHandle) {
    let now = khor_catalog::lang::current();
    let was = app.state::<Held>().built_in.lock().ok().and_then(|l| *l);
    if was != Some(now)
        && let Err(e) = install(app)
    {
        eprintln!("khor: the menu bar did not rebuild in the new language: {e}");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn follow_lang(_app: &AppHandle) {}

/// A menu item was picked (or its accelerator fired where the page did
/// not take the key).
pub fn on_event(app: &AppHandle, id: &str) {
    let focused = app
        .webview_windows()
        .into_iter()
        .find(|(_, w)| w.is_focused().unwrap_or(false));
    // An editor window in front (`open_code_window`) is a window of its
    // own: its ⌘W and ⇧⌘W close it, and nothing of khor's page is asked.
    let editor = focused
        .as_ref()
        .is_some_and(|(label, _)| label.starts_with("code:"));
    if id == "close-window" || (id == "close-tab" && editor) {
        let target = focused
            .map(|(_, w)| w)
            .or_else(|| app.get_webview_window("main"));
        if let Some(w) = target {
            let _ = w.close();
        }
        return;
    }
    if !ACTS.contains(&id) {
        return;
    }
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    // Everything else is about the main window, which comes to the front
    // for it — ⌘1 from an editor goes to 会话 the way Mail's ⌘1 does.
    if editor {
        let _ = main.set_focus();
    }
    let _ = main.emit_to("main", "shortcut", id);
}
