//! The macOS menu's undo/redo path. A keyboard key that CodeMirror
//! prevented already stops in WebKit; an actual menu selection (or an
//! accelerator WebKit forwarded) reaches here. Never emit a global event:
//! a remote webpage may have focus beside the trusted app webview.

use block2::RcBlock;
use objc2::{class, msg_send, rc::Retained, runtime::AnyObject, sel};
use objc2_foundation::NSString;
use tauri::{AppHandle, Manager};

const UNDO: &str = include_str!("native_edit.js");

/// Schedule through the actual focused window, without focusing another
/// window or looking at a stale document.activeElement in the main face.
pub fn run(app: &AppHandle, redo: bool) {
    let face = ["main", "settings"].into_iter().find_map(|label| {
        app.get_window(label)
            .filter(|window| window.is_focused().unwrap_or(false))
            .and_then(|_| app.get_webview(label))
    });
    if let Some(face) = face {
        let _ = face.with_webview(move |view| unsafe {
            perform(view.inner() as *mut AnyObject, redo);
        });
    } else {
        // Native dialogs, editor windows and remote-only windows keep
        // their responder chain. No main-window fallback or focus steal.
        let _ = app.run_on_main_thread(move || unsafe { fallback(redo, None) });
    }
}

/// Native main-thread entry, separated so the WK regression harness can
/// exercise this exact routing with real AppKit/WebKit objects.
pub(crate) unsafe fn perform(web: *mut AnyObject, redo: bool) {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let window: *mut AnyObject = msg_send![app, keyWindow];
        if window.is_null() {
            return;
        }
        let responder: *mut AnyObject = msg_send![window, firstResponder];
        let own: *mut AnyObject = msg_send![web, window];
        let is_view = !responder.is_null() && msg_send![responder, isKindOfClass: class!(NSView)];
        // A remote WK can be nested under the app's native view. Match
        // the nearest WK ancestor, not merely containment in the face.
        let mut ancestor = if is_view {
            responder
        } else {
            std::ptr::null_mut()
        };
        while !ancestor.is_null() && !msg_send![ancestor, isKindOfClass: class!(WKWebView)] {
            ancestor = msg_send![ancestor, superview];
        }
        let in_web = ancestor == web;
        if own != window || !in_web {
            fallback(redo, None);
            return;
        }
        // Retain both across the asynchronous JavaScript evaluation.
        // The callback may not send undo into a different field/window.
        let Some(window) = Retained::retain(window) else {
            return;
        };
        let Some(responder) = Retained::retain(responder) else {
            return;
        };
        let done = RcBlock::new(move |result: *mut AnyObject, error: *mut AnyObject| {
            let handled = error.is_null()
                && !result.is_null()
                && msg_send![result, isKindOfClass: class!(NSNumber)]
                && msg_send![result, boolValue];
            if !handled {
                fallback(redo, Some((&window, &responder)));
            }
        });
        let script = UNDO.replace(
            "__KHOR_HISTORY_INPUT__",
            if redo { "historyRedo" } else { "historyUndo" },
        );
        let _: () = msg_send![web, evaluateJavaScript: &*NSString::from_str(&script), completionHandler: &*done];
    }
}

unsafe fn fallback(redo: bool, original: Option<(&AnyObject, &AnyObject)>) {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        if let Some((window, responder)) = original {
            let now: *mut AnyObject = msg_send![app, keyWindow];
            if now != window as *const AnyObject as *mut AnyObject {
                return;
            }
            let first: *mut AnyObject = msg_send![now, firstResponder];
            if first != responder as *const AnyObject as *mut AnyObject {
                return;
            }
        }
        let action = if redo { sel!(redo:) } else { sel!(undo:) };
        let _: bool = msg_send![app, sendAction: action, to: std::ptr::null::<AnyObject>(), from: std::ptr::null::<AnyObject>()];
    }
}
