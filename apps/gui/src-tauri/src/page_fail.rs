//! **A page that cannot be opened says so** (UX-22, the owner 2026-10-05:
//! a page that fails to load said nothing). WebKit tells its navigation
//! delegate when a load fails — before anything arrived (a name nobody
//! answers for, a refused connection, a timeout, a bad certificate) or
//! after — and khor heard neither: wry's delegate implements neither
//! method and reports only a commit and a finish, and the phone's pages
//! had no delegate at all. So a failed load was a tab that stopped
//! loading and showed the page before it, or nothing.
//!
//! Here: what an `NSError` from either says, as the face's `page:fail`
//! word (`code|domain|description|url`); on the Mac, the two methods
//! added to wry's delegate class at run time — wry keeps every method it
//! has, and these two are ones it does not have — and the webview → tab
//! book they answer from. The phone's own delegate is `phone_page`'s.

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;

/// What a failed load's `NSError` says, or `None` for a load that was not
/// a failure anybody should hear of: one cancelled because another took
/// its place (NSURLErrorCancelled, a link pressed while the last one
/// loaded), and WebKit's 「frame load interrupted」 (102), which is a
/// download or a policy decision taking the navigation over.
///
/// # Safety
/// `error` is a live `NSError`.
pub unsafe fn word(error: *mut AnyObject) -> Option<String> {
    if error.is_null() {
        return None;
    }
    let code: isize = msg_send![error, code];
    let domain: Option<Retained<NSString>> = msg_send![error, domain];
    let domain = domain.map(|d| d.to_string()).unwrap_or_default();
    if (domain == "NSURLErrorDomain" && code == -999) || (domain == "WebKitErrorDomain" && code == 102) {
        return None;
    }
    let said: Option<Retained<NSString>> = msg_send![error, localizedDescription];
    let said = said.map(|s| s.to_string()).unwrap_or_default();
    let info: *mut AnyObject = msg_send![error, userInfo];
    let url = if info.is_null() {
        String::new()
    } else {
        // The string on the phone; the Mac's error carries only the
        // `NSURL` (measured 2026-10-06), whose description is the same
        // address.
        let key = NSString::from_str("NSErrorFailingURLStringKey");
        let mut at: *mut AnyObject = msg_send![info, objectForKey: &*key];
        if at.is_null() {
            let key = NSString::from_str("NSErrorFailingURLKey");
            at = msg_send![info, objectForKey: &*key];
        }
        if at.is_null() {
            String::new()
        } else {
            let s: Option<Retained<NSString>> = msg_send![at, description];
            s.map(|s| s.to_string()).unwrap_or_default()
        }
    };
    // `|` cannot be in a domain or a number; the description and the
    // address are the last two and the face splits at the first three.
    Some(format!("{code}|{domain}|{}|{url}", said.replace('|', "/")))
}

#[cfg(target_os = "macos")]
pub use mac::{forget, take_failed, watch};

#[cfg(target_os = "macos")]
mod mac {
    use std::collections::HashMap;
    use std::sync::{Mutex, Once, OnceLock};

    use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
    use objc2::{msg_send, sel};
    use tauri::{Emitter, Manager};

    use crate::PageWord;

    static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
    /// Each page's webview, by address, to the face that holds it and its
    /// tab; and where each tab last failed to go, for 重试.
    struct Book {
        views: HashMap<usize, (String, String)>,
        failed: HashMap<String, String>,
    }
    fn book() -> &'static Mutex<Book> {
        static B: OnceLock<Mutex<Book>> = OnceLock::new();
        B.get_or_init(|| Mutex::new(Book { views: HashMap::new(), failed: HashMap::new() }))
    }

    /// Hears `made`'s failed loads from now on, for `tab` on `face`.
    pub fn watch(made: &tauri::Webview, face: String, tab: String) {
        let _ = APP.set(made.app_handle().clone());
        let _ = made.with_webview(move |wv| {
            let view = wv.inner() as *mut AnyObject;
            // SAFETY: `inner` is the live WKWebView, on the main thread.
            unsafe {
                install(view);
                // **The delegate set again, so WebKit reads the class's
                // methods again**: WKWebView notes which delegate methods
                // exist when the delegate is set, and wry set it before
                // the two were added — the first page of a run heard no
                // failure (measured 2026-10-06, the scratch app).
                let delegate: *mut AnyObject = msg_send![view, navigationDelegate];
                if !delegate.is_null() {
                    let _: () = msg_send![view, setNavigationDelegate: std::ptr::null::<AnyObject>()];
                    let _: () = msg_send![view, setNavigationDelegate: delegate];
                }
            }
            if let Ok(mut b) = book().lock() {
                b.views.insert(view as usize, (face, tab));
            }
        });
    }

    /// The tab's page is gone.
    pub fn forget(tab: &str) {
        if let Ok(mut b) = book().lock() {
            b.views.retain(|_, (_, t)| t != tab);
            b.failed.remove(tab);
        }
    }

    /// Where the tab's last load failed to go, if it failed since 重试
    /// last asked: what 重试 goes to (a reload would load the page before
    /// it, the one that is still there).
    pub fn take_failed(tab: &str) -> Option<String> {
        book().lock().ok()?.failed.remove(tab).filter(|u| !u.is_empty())
    }

    /// Adds the two failure methods to the class of `view`'s navigation
    /// delegate, once: every page webview wears the same wry delegate
    /// class. **Taken off the live delegate, not looked up by name**:
    /// objc2 registers a `define_class!` class under its Rust path and
    /// crate version (`wry::…::WryNavigationDelegate0.55.1`), so
    /// `AnyClass::get("WryNavigationDelegate")` found nothing and the
    /// Mac heard no failure (measured 2026-10-06 on the scratch app).
    unsafe fn install(view: *mut AnyObject) {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let delegate: *mut AnyObject = unsafe { msg_send![view, navigationDelegate] };
            let cls = if delegate.is_null() { std::ptr::null() } else { unsafe { objc2::ffi::object_getClass(delegate) } };
            if cls.is_null() {
                eprintln!("khor: the page has no navigation delegate; a page that fails to load says nothing");
                return;
            }
            let cls = cls as *mut AnyClass;
            // SAFETY: `failed` has the C signature of both selectors
            // (self, _cmd, WKWebView *, WKNavigation *, NSError *) and the
            // type string says so; class_addMethod adds only where the
            // class has no such method, so wry's own are never replaced.
            unsafe {
                let imp: Imp = std::mem::transmute::<
                    unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject, *mut AnyObject),
                    Imp,
                >(failed);
                for s in [sel!(webView:didFailProvisionalNavigation:withError:), sel!(webView:didFailNavigation:withError:)] {
                    objc2::ffi::class_addMethod(cls, s, imp, c"v@:@@@".as_ptr());
                }
            }
        });
    }

    unsafe extern "C-unwind" fn failed(_this: *mut AnyObject, _cmd: Sel, view: *mut AnyObject, _nav: *mut AnyObject, error: *mut AnyObject) {
        let Some(text) = (unsafe { super::word(error) }) else { return };
        let Some(app) = APP.get() else { return };
        let Some((face, tab)) = book().lock().ok().and_then(|mut b| {
            let at = b.views.get(&(view as usize)).cloned()?;
            let url = text.splitn(4, '|').nth(3).unwrap_or_default().to_owned();
            b.failed.insert(at.1.clone(), url);
            Some(at)
        }) else {
            return;
        };
        // The load is over: the light goes out, then the word.
        let _ = app.emit_to(face.clone(), "page:load", PageWord { tab: tab.clone(), text: "finished".into() });
        let _ = app.emit_to(face, "page:fail", PageWord { tab, text });
    }
}
