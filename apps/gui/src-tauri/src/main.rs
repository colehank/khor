// **Windows is always the GUI subsystem**, release and debug alike. The
// old `not(debug_assertions)` gate made a debug build a console
// application, which is how a build handed to a person opened a console
// window — the two artefacts on this machine disagreed (release: no
// console but a doubled title bar, debug: one bar and a console), and
// that read as a window that "flips" between two shapes. `diagnostics`
// covers what the gate used to buy: stderr goes to a log file, and
// `KHOR_GUI_CONSOLE=1` allocates a console for the times a terminal is
// genuinely wanted.
#![cfg_attr(windows, windows_subsystem = "windows")]

// **The bare binary's bundle facts** (2026-09-28, 「菜单栏」 in
// docs/design-system/guidelines/30-behavior.md). The app is run as this
// executable, not as a `.app`, and a process with no Info.plist is named
// by AppKit after its file — the app menu's bold title read 「khor-gui」 —
// and declares no language, so the items AppKit adds by itself (自动填充,
// 开始听写…, 填充, 居中 …) came out in English on a Chinese desktop. A
// bare Mach-O carries its Info.plist in the `__TEXT,__info_plist` section,
// which is what this puts there: `../Info.plist`, a name and two
// localizations.
//
// ⚠️ **Never a `CFBundleIdentifier` in that file.** WebKit, CFNetwork and
// the defaults system name their folders after `bundleIdentifier`, and
// after the process name only when there is none: the page's localStorage,
// cookies and caches live under `khor-gui` today (~/Library/WebKit/
// khor-gui, ~/Library/Caches/khor-gui, ~/Library/HTTPStorages/khor-gui…),
// and an identifier would start the app on a new, empty page store —
// measured on a scratch binary, which came up under the identifier's
// folders. The same goes for renaming the executable.
//
// **Not under `dev`**: a dev build (`--no-default-features`) gets one from
// tauri itself, from the same `Info.plist` plus the config's
// `bundle.macOS.bundleName`, and two would not link (`_EMBED_INFO_PLIST`
// is defined once on purpose).
#[cfg(all(target_os = "macos", not(dev)))]
embed_plist::embed_info_plist!("../Info.plist");

fn main() {
    // First, before anything can write: a GUI-subsystem process has no
    // streams, and every diagnostic in this crate speaks on stderr.
    khor_gui::diagnostics::attach();
    // ssh asking for a kept password (`khor_node::secret`): the app is
    // its own askpass, answered before any window.
    khor_node::secret::answer_askpass_if_asked();
    khor_gui::run()
}
