# macOS editor menu regression

```sh
python3 scripts/editor-history-probe/run.py --out /Volumes/disk0/khor-editor-history-native-check
```

Requires macOS, a GUI session, Rust, Swift and installed GUI dependencies.
Off macOS it explicitly skips. Builds and evidence stay in the supplied external
directory. The launcher starts its own Vite server and removes only its generated
fixture/server. No khor node, user files or sessions are used. Keyboard events are
posted only to the probe's own PID; the probe briefly activates its own window.

The Rust static library imports the **actual production `native_edit.rs`** and its
embedded JavaScript. Real AppKit/WKWebView objects execute native responder checks,
nearest-WK ownership, JavaScript, asynchronous reply and fallback. A tiny Tauri
facade stubs only the outer label lookup/scheduling API, which is compiled separately
by `cargo check -p khor-gui --lib`. This is not an end-to-end Tauri executable test.

Checks include the old native `undo:` no-op as a control, CodeMirror menu undo/redo,
real Command-Z / Shift-Command-Z without duplicate menu dispatch, ordinary HTML
input fallback, a nested remote WK input, a native text field and a focus change
before an asynchronous fallback. Source hashes and pinned Cargo.lock dependency
versions are recorded beside results. The probe uses CodeMirror's real keyboard
and beforeinput handlers; no alternate history implementation is tested.
