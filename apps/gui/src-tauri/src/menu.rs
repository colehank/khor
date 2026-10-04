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
//! The items AppKit adds by itself, and the app menu's bold name, are not
//! built here: `Info.plist` (embedded by `main.rs`) and
//! `speak_at_launch` answer for those.
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
const ACTS: [&str; 23] = [
    "landing-1",
    "landing-2",
    "landing-3",
    "landing-4",
    "landing-5",
    "settings",
    "lock",
    "find",
    "find-list",
    "new",
    "new-tab",
    "import",
    "close-tab",
    "next-tab",
    "prev-tab",
    "toggle-list",
    // Clicked in the menu rather than pressed: the page runs these too
    // (the reload circle's act, 2026-10-03; roadmap 2.3's jumps).
    "reload-term",
    "prev-command",
    "next-command",
    // ⚠️ It was listed in the View menu and missing here, so choosing it
    // from the menu did nothing; only its key worked (UX-7).
    "command-history",
    // ⌘+ ⌘− ⌘0, the terminal's type size (item 22).
    "term-bigger",
    "term-smaller",
    "term-actual",
];

/// The five landings by the keys the page's strip orders them by
/// (`src/lib/nav-order.ts` `NavKey`), in its default order.
const LANDINGS: [&str; 5] = ["devices", "sessions", "quota", "workspaces", "browser"];

/// **The View menu follows the strip** (the audit fix plan F3.5; W03):
/// the page's order, mended the way the page mends it — unknown keys
/// dropped, each kept once, the missing ones appended in the default's
/// order — so the Nth item is the Nth glyph and ⌘N reaches it.
pub fn landing_order(order: &[String]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for k in order {
        if let Some(known) = LANDINGS.iter().find(|l| **l == k.as_str())
            && !out.contains(known)
        {
            out.push(known);
        }
    }
    for l in LANDINGS {
        if !out.contains(&l) {
            out.push(l);
        }
    }
    out
}

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
    /// The strip's landings, in its order (`nav-order.ts`).
    pub order: Vec<String>,
    /// The list beside the content: `Some(folded)` where there is one
    /// to fold (⌃⌘S), `None` where there is not.
    pub list: Option<bool>,
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
    sidebar: MenuItem<tauri::Wry>,
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
    /// The strip order the View menu was built in.
    #[cfg(target_os = "macos")]
    built_order: Mutex<Vec<&'static str>>,
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
    // ⌘T (UX-7): the landing's 「+」 where it makes a tab — a session, a
    // page. The page answers only there (`lib/shortcuts`).
    let new_tab = item("new-tab", m::new_tab(), Some("CmdOrCtrl+T"))?;
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
        &[&new, &new_tab, &import, &sep()?, &close_tab, &close_window],
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
            &sep()?,
            // ⌘F (UX-7): Mail's and Notes' 查找 — this column's search, or
            // the terminal's own in a terminal (`TerminalPane`).
            &item("find-list", m::find_list(), Some("CmdOrCtrl+F"))?,
        ],
    )?;

    let held = app.state::<Held>();
    let state = held.state.lock().map(|s| s.clone()).unwrap_or_default();
    let order = landing_order(&state.order);
    let name = |key: &str| match key {
        "sessions" => gui::sessions_tab(),
        "devices" => gui::devices_tab(),
        "workspaces" => gui::workspaces_tab(),
        "quota" => gui::accounts_tab(),
        _ => gui::browser_tab(),
    };
    // `landing-N` is the strip's Nth seat, which is what the page runs
    // for it (`go(N-1)`); the word is whichever landing sits there now.
    let mut landings = Vec::new();
    for (i, key) in order.iter().enumerate() {
        let id = format!("landing-{}", i + 1);
        let accel = format!("CmdOrCtrl+{}", i + 1);
        landings.push(item(&id, name(key), Some(accel.as_str()))?);
    }
    let find = item("find", m::find(), Some("CmdOrCtrl+K"))?;
    // ⌘R: the terminal in front joined again (2026-10-03, the reload
    // circle's act); the page answers it only while a terminal is open.
    let reload_term = item("reload-term", m::reload_term(), Some("CmdOrCtrl+R"))?;
    // ⌘↑ ⌘↓: the terminal in front to its previous or next prompt
    // (roadmap 2.3; VS Code's terminal binds the same keys to the same).
    let prev_command = item("prev-command", m::prev_command(), Some("CmdOrCtrl+Up"))?;
    let next_command = item("next-command", m::next_command(), Some("CmdOrCtrl+Down"))?;
    // ⌘⇧;: the machine's commands, searched (roadmap 2.6) — iTerm2's key
    // for its Command History.
    let command_history = item("command-history", m::command_history(), Some("CmdOrCtrl+Shift+Semicolon"))?;
    // ⌘+ ⌘− ⌘0: the terminal's type size a point at a time (item 22;
    // Terminal.app's View › Bigger, Smaller, Default Font Size).
    let term_bigger = item("term-bigger", m::term_bigger(), Some("CmdOrCtrl+Equal"))?;
    let term_smaller = item("term-smaller", m::term_smaller(), Some("CmdOrCtrl+Minus"))?;
    let term_actual = item("term-actual", m::term_actual(), Some("CmdOrCtrl+0"))?;
    // ⌃⌘S, the sidebar's key in Notes and Finder (F6.7).
    let sidebar = item("toggle-list", m::hide_sidebar(), Some("Ctrl+Cmd+S"))?;
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
            &sidebar,
            &find,
            &reload_term,
            &prev_command,
            &next_command,
            &command_history,
            &sep()?,
            &term_bigger,
            &term_smaller,
            &term_actual,
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

    let items = Items {
        new,
        import,
        close_tab,
        next,
        prev,
        lock,
        settings,
        find,
        sidebar,
        landings,
    };
    apply(&items, &state);
    if let Ok(mut o) = held.built_order.lock() {
        *o = order;
    }
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
    let _ = items.sidebar.set_enabled(walk && state.list.is_some());
    let _ = items.sidebar.set_text(if state.list == Some(true) {
        khor_catalog::menu::show_sidebar()
    } else {
        khor_catalog::menu::hide_sidebar()
    });
}

/// The page's word on what applies (`src/lib/shortcuts.ts`, on change).
#[cfg(target_os = "macos")]
pub fn set_state(app: &AppHandle, state: MenuState) {
    let held = app.state::<Held>();
    // **A reorder under 外观 rebuilds the View menu**: its items' words
    // and keys are set when it is built.
    let reorder = held
        .built_order
        .lock()
        .map(|o| *o != landing_order(&state.order))
        .unwrap_or(false);
    if let Ok(mut s) = held.state.lock() {
        *s = state.clone();
    }
    if reorder {
        if let Err(e) = install(app) {
            eprintln!("khor: the menu bar did not rebuild in the new order: {e}");
        }
        return;
    }
    if let Ok(items) = held.items.lock()
        && let Some(items) = items.as_ref()
    {
        apply(items, &state);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_state(_app: &AppHandle, _state: MenuState) {}

/// **AppKit's own items speak khor's language** (2026-09-28).
///
/// 编辑 and 窗口 carry items khor never wrote — 自动填充, 开始听写…,
/// 表情与符号, 填充, 居中, 移动与调整大小 … — which AppKit adds, worded in
/// the localization it picked for this process: the first of the person's
/// languages that `Info.plist` declares (zh-Hans, en), English when none
/// is. khor's own words follow `khor config`'s `lang`, which can say
/// otherwise — an English khor on a Chinese Mac. So when the two would
/// differ, khor's language is put first in this process's
/// `AppleLanguages`, **in the argument domain**: in memory, this process
/// only. Never `defaults write`: the app's domain is named after the
/// process, and every khor-gui on the machine — the owner's, the native
/// gate's — would read it.
///
/// It is the switch macOS's own per-app language turns (系统设置 › 通用 ›
/// 语言与地区 › 应用程序), and it reaches what that one reaches: the
/// system's panels (the file picker, the Touch ID sheet), and the
/// webviews' `navigator.language` — a browsing tab asks sites for khor's
/// language first, and the face's `auto` (`src/lang.ts`) lands where the
/// Rust side's did. When the two already agree, nothing is touched.
///
/// **Once, before AppKit is up** (`run`, ahead of the builder).
/// CoreFoundation settles a bundle's language the first time it is asked
/// and keeps it: measured, a language switched later and a menu rebuilt
/// after it still got AppKit's items in the first one. So a language
/// changed while the app runs rebuilds khor's own items at once
/// (`follow_lang`), and AppKit's follow at the next launch.
#[cfg(target_os = "macos")]
pub fn speak_at_launch() {
    use khor_catalog::lang::Lang;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{
        NSArgumentDomain, NSArray, NSBundle, NSMutableDictionary, NSString, NSUserDefaults,
        ns_string,
    };
    // The two `Info.plist` declares, spelled as it spells them.
    let wanted = match khor_catalog::lang::current() {
        Lang::Zh => "zh-Hans",
        Lang::En => "en",
    };
    let defaults = NSUserDefaults::standardUserDefaults();
    let key = ns_string!("AppleLanguages");
    let now: Vec<Retained<AnyObject>> = defaults
        .arrayForKey(key)
        .map(|a| a.to_vec())
        .unwrap_or_default();
    // What AppKit would pick from them. ⚠️ **The person's languages are
    // handed in, never left to the default** (`forPreferences: nil`):
    // asked for them, CoreFoundation reads them once and keeps them for
    // the life of the process — the first build of this asked that way,
    // and AppKit's items stayed Chinese under an English khor while the
    // webview had already switched.
    let declared =
        NSArray::from_retained_slice(&[NSString::from_str("zh-Hans"), NSString::from_str("en")]);
    let theirs: Vec<Retained<NSString>> = now
        .iter()
        .filter_map(|l| l.clone().downcast::<NSString>().ok())
        .collect();
    let would = NSBundle::preferredLocalizationsFromArray_forPreferences(
        &declared,
        Some(&NSArray::from_retained_slice(&theirs)),
    );
    if would.firstObject().is_some_and(|l| l.to_string() == wanted) {
        return;
    }
    // khor's first, then the person's own order behind it, so what else
    // they read (a site with no English, say) still has somewhere to go.
    let mut order: Vec<Retained<AnyObject>> = vec![NSString::from_str(wanted).into()];
    order.extend(now);
    let langs = NSArray::from_retained_slice(&order);
    // SAFETY: a constant Foundation exports, read once.
    let domain = unsafe { NSArgumentDomain };
    // The argument domain holds the command line's `-Key value` pairs:
    // kept, with this one key added.
    let args =
        NSMutableDictionary::dictionaryWithDictionary(&defaults.volatileDomainForName(domain));
    args.insert(key, AsRef::<AnyObject>::as_ref(&*langs));
    defaults.removeVolatileDomainForName(domain);
    // SAFETY: `args` is the `NSString → object` dictionary a domain is.
    unsafe { defaults.setVolatileDomain_forName(&args, domain) };
}

#[cfg(not(target_os = "macos"))]
pub fn speak_at_launch() {}

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
///
/// **The main window is looked up as a window, never as a webview
/// window** (2026-10-03, the native walk). tauri's `get_webview_window`
/// answers `None` for a window holding more than its own webview, and
/// khor's window holds a browsing tab's page beside the face from the
/// first page opened until the tab closes — so every item of the menu
/// bar went quiet once 浏览器 had been used. The editor windows hold one
/// webview each and are still found the old way.
pub fn on_event(app: &AppHandle, id: &str) {
    let editor = app
        .webview_windows()
        .into_iter()
        .find(|(label, w)| (label.starts_with("code:") || label == "settings") && w.is_focused().unwrap_or(false))
        .map(|(_, w)| w);
    // An editor window in front (`open_code_window`), or the settings
    // window, is a window of its own: its ⌘W and ⇧⌘W close it, and
    // nothing of khor's page is asked.
    if id == "close-window" || (id == "close-tab" && editor.is_some()) {
        if let Some(w) = editor {
            let _ = w.close();
        } else if let Some(w) = app.get_window("main") {
            let _ = w.close();
        }
        return;
    }
    if !ACTS.contains(&id) {
        return;
    }
    // ⌘, opens the settings window from anywhere, the page not asked
    // (UX-10): it is a window of khor's own, not a page of the main one.
    if id == "settings" {
        if let Err(e) = crate::show_settings(app, None) {
            eprintln!("khor: the settings window did not open: {e}");
        }
        return;
    }
    // Everything else is about the main window, which comes to the front
    // for it — ⌘1 from an editor goes to 会话 the way Mail's ⌘1 does.
    if editor.is_some()
        && let Some(main) = app.get_window("main")
    {
        let _ = main.set_focus();
    }
    let _ = app.emit_to("main", "shortcut", id);
}

#[cfg(test)]
mod tests {
    use super::landing_order;

    /// The View menu's order is the strip's, mended as the page mends it.
    #[test]
    fn the_view_menu_follows_the_strip() {
        let own = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            landing_order(&[]),
            ["devices", "sessions", "quota", "workspaces", "browser"],
            "no word from the page is the default order"
        );
        assert_eq!(
            landing_order(&own(&["browser", "sessions", "nope", "sessions"])),
            ["browser", "sessions", "devices", "quota", "workspaces"],
            "unknown dropped, each once, the rest appended in the default's order"
        );
    }
}
