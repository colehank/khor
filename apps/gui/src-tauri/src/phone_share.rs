//! One native original-file presenter, shared by browser downloads and
//! workspace files. UIKit owns the sheet; Rust keeps its file alive.
use objc2::{
    class, msg_send,
    rc::{Allocated, Retained},
    runtime::AnyObject,
};
use objc2_foundation::NSString;

thread_local! { static BUSY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

/// # Safety
/// Main thread; `view` must be an attached, live WKWebView.
pub unsafe fn show(
    view: &AnyObject,
    path: &std::path::Path,
    done: impl Fn(Result<bool, String>) + 'static,
) -> bool {
    if BUSY.with(|busy| busy.get()) {
        return false;
    }
    unsafe {
        let at = view as *const AnyObject as *mut AnyObject;
        let window: *mut AnyObject = msg_send![at, window];
        if window.is_null() {
            return false;
        }
        let mut top: *mut AnyObject = msg_send![window, rootViewController];
        if top.is_null() {
            return false;
        }
        loop {
            let next: *mut AnyObject = msg_send![top, presentedViewController];
            if next.is_null() {
                break;
            }
            top = next;
        }
        let text = NSString::from_str(&path.to_string_lossy());
        let url: Retained<AnyObject> =
            msg_send![class!(NSURL), fileURLWithPath: &*text, isDirectory: false];
        let items: Retained<AnyObject> = msg_send![class!(NSArray), arrayWithObject: &*url];
        let sheet: Allocated<AnyObject> = msg_send![class!(UIActivityViewController), alloc];
        let sheet: Retained<AnyObject> = msg_send![sheet, initWithActivityItems: &*items, applicationActivities: std::ptr::null::<AnyObject>()];
        let pop: *mut AnyObject = msg_send![&*sheet, popoverPresentationController];
        if !pop.is_null() {
            let bounds: objc2_foundation::NSRect = msg_send![at, bounds];
            let _: () = msg_send![pop, setSourceView: at];
            let _: () = msg_send![pop, setSourceRect: bounds];
        }
        let gone = block2::RcBlock::new(
            move |_kind: *mut AnyObject,
                  completed: objc2::runtime::Bool,
                  _items: *mut AnyObject,
                  error: *mut AnyObject| {
                BUSY.with(|busy| busy.set(false));
                if error.is_null() {
                    done(Ok(completed.as_bool()));
                } else {
                    let description: Retained<NSString> = msg_send![error, localizedDescription];
                    done(Err(description.to_string()));
                }
            },
        );
        let _: () = msg_send![&*sheet, setCompletionWithItemsHandler: &*gone];
        BUSY.with(|busy| busy.set(true));
        let _: () = msg_send![top, presentViewController: &*sheet, animated: true, completion: std::ptr::null::<AnyObject>()];
        true
    }
}

pub async fn open(view: tauri::Webview, path: std::path::PathBuf) -> Result<bool, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    view.with_webview(move |webview| {
        let tx = std::rc::Rc::new(std::cell::RefCell::new(Some(tx)));
        let reply = tx.clone();
        // SAFETY: with_webview executes on the main thread and provides
        // this live app WKWebView. Only a completed app-owned path enters.
        let shown = unsafe {
            let view = &*(webview.inner() as *mut AnyObject);
            show(view, &path, move |result| {
                if let Some(tx) = reply.borrow_mut().take() {
                    let _ = tx.send(result);
                }
            })
        };
        if !shown {
            if let Some(tx) = tx.borrow_mut().take() {
                let _ = tx.send(Err(khor_catalog::msg::file_open_unavailable().into()));
            }
        }
    })
    .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}
