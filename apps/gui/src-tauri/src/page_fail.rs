//! WebKit navigation/error adapter. The common page_navigation owner keeps
//! the lifecycle facts; native callbacks synchronously produce immutable
//! issued snapshots. NSError evidence is bounded without changing its URL,
//! domain or code. Mac hooks preserve wry's original commit/finish IMPs.

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;

/// Bounded raw failure evidence shared by the Mac and iOS adapters.
///
/// # Safety
/// `error` is a live NSError supplied by WebKit. This reads only its
/// standard scalar fields and one bounded underlying-error chain.
pub unsafe fn details(error: *mut AnyObject) -> Option<crate::page_navigation::Failure> {
    use khor_gui_core::web::{PageNavigationCause, PageNavigationError};
    if error.is_null() {
        return None;
    }
    unsafe fn cause(error: *mut AnyObject) -> PageNavigationCause {
        let code: isize = unsafe { msg_send![error, code] };
        let domain: Option<Retained<NSString>> = unsafe { msg_send![error, domain] };
        let description: Option<Retained<NSString>> =
            unsafe { msg_send![error, localizedDescription] };
        let info: *mut AnyObject = unsafe { msg_send![error, userInfo] };
        let mut url = String::new();
        if !info.is_null() {
            for name in ["NSErrorFailingURLStringKey", "NSErrorFailingURLKey"] {
                let key = NSString::from_str(name);
                let value: *mut AnyObject = unsafe { msg_send![info, objectForKey: &*key] };
                if !value.is_null() {
                    let value: Option<Retained<NSString>> =
                        unsafe { msg_send![value, description] };
                    url = value.map(|value| value.to_string()).unwrap_or_default();
                    break;
                }
            }
        }
        PageNavigationCause {
            domain: domain.map(|value| value.to_string()).unwrap_or_default(),
            code: code.to_string(),
            description: description
                .map(|value| value.to_string())
                .unwrap_or_default(),
            url,
        }
    }
    unsafe fn underlying(error: *mut AnyObject) -> *mut AnyObject {
        let info: *mut AnyObject = unsafe { msg_send![error, userInfo] };
        if info.is_null() {
            return std::ptr::null_mut();
        }
        let key = NSString::from_str("NSUnderlyingError");
        unsafe { msg_send![info, objectForKey: &*key] }
    }
    let first = unsafe { cause(error) };
    let target = first.url.clone();
    let mut result = PageNavigationError {
        domain: first.domain,
        code: first.code,
        description: first.description,
        url: first.url,
        underlying: Vec::new(),
        truncated: false,
    };
    let mut seen = vec![error as usize];
    let mut next = unsafe { underlying(error) };
    while !next.is_null() && result.underlying.len() < 3 {
        let is_error: bool = unsafe { msg_send![next, isKindOfClass: objc2::class!(NSError)] };
        if !is_error {
            result.truncated = true;
            break;
        }
        if seen.contains(&(next as usize)) {
            result.truncated = true;
            break;
        }
        seen.push(next as usize);
        result.underlying.push(unsafe { cause(next) });
        next = unsafe { underlying(next) };
    }
    result.truncated |= !next.is_null();
    Some(crate::page_navigation::Failure {
        error: crate::page_navigation::bounded_error(result),
        target,
    })
}

#[cfg(target_os = "macos")]
pub use mac::{forget, take_failed, watch};

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::OnceLock;

    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, Imp, NSObject, Sel};
    use objc2::{ClassType, class, define_class, msg_send, sel};
    use objc2_foundation::NSString;
    use tauri::{Emitter, Manager};

    use crate::page_navigation::{Changed, ViewKey, with_pages};

    static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
    type NavigationCallback =
        unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject);
    struct Originals {
        class: usize,
        committed: Imp,
        finished: Imp,
    }
    static ORIGINALS: OnceLock<Originals> = OnceLock::new();

    // Native KVO handles only. Navigation, failure and loading facts live
    // exclusively in page_navigation::Pages.
    struct Observation {
        view: Retained<AnyObject>,
        key: ViewKey,
    }
    thread_local! {
        static OBSERVATIONS: RefCell<HashMap<String, Observation>> = RefCell::new(HashMap::new());
        static PROGRESS: Retained<ProgressWatch> = unsafe { msg_send![ProgressWatch::class(), new] };
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "KhorPageProgressWatch"]
        struct ProgressWatch;
        impl ProgressWatch {
            #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
            fn observe(&self, key: *mut AnyObject, view: *mut AnyObject, _change: *mut AnyObject, _context: *mut std::ffi::c_void) {
                if key.is_null() || view.is_null() { return; }
                let name = unsafe { (&*(key as *const NSString)).to_string() };
                if name != "estimatedProgress" { return; }
                let value: f64 = unsafe { msg_send![view, estimatedProgress] };
                // Mutate synchronously in this callback. Only the complete,
                // immutable revision snapshot crosses the IPC/event queue.
                tell(with_pages(|pages| pages.key(view as usize).and_then(|key| pages.progressed(key, value))).flatten());
            }
        }
    );

    fn tell(change: Option<Changed>) {
        if let (Some(app), Some(change)) = (APP.get(), change) {
            let _ = app.emit_to(change.face, "page:navigation", change.state);
        }
    }

    unsafe fn url_of(view: *mut AnyObject) -> String {
        let url: *mut AnyObject = unsafe { msg_send![view, URL] };
        if url.is_null() {
            return String::new();
        }
        let string: Option<Retained<NSString>> = unsafe { msg_send![url, absoluteString] };
        string.map(|value| value.to_string()).unwrap_or_default()
    }

    /// Construction happens on about:blank. Observe before loading the
    /// original target; otherwise the first provisional start is lost.
    pub async fn watch(
        made: &tauri::Webview,
        face: String,
        tab: String,
        target: String,
        exit: String,
        proxied: bool,
    ) -> Result<(), String> {
        let _ = APP.set(made.app_handle().clone());
        let (tx, rx) = tokio::sync::oneshot::channel();
        made.with_webview(move |native| {
            let view = native.inner() as *mut AnyObject;
            let result = unsafe { watch_on_main(view, face, &tab, &target, exit, proxied) };
            if result.is_err() {
                forget_on_main(&tab);
            }
            let _ = tx.send(result);
        })
        .map_err(|error| error.to_string())?;
        rx.await.map_err(|error| error.to_string())?
    }

    unsafe fn watch_on_main(
        view: *mut AnyObject,
        face: String,
        tab: &str,
        target: &str,
        exit: String,
        proxied: bool,
    ) -> Result<(), String> {
        forget_on_main(tab);
        unsafe {
            install(view)?;
        }
        let (key, opened) = with_pages(|pages| {
            pages.attach(
                view as usize,
                face,
                tab.to_owned(),
                target.to_owned(),
                exit,
                proxied,
            )
        })
        .flatten()
        .ok_or("page navigation identity unavailable")?;
        let retained = unsafe { Retained::retain(view) }.ok_or("page view unavailable")?;
        OBSERVATIONS.with(|observations| {
            observations.borrow_mut().insert(
                tab.to_owned(),
                Observation {
                    view: retained,
                    key,
                },
            )
        });
        PROGRESS.with(|watch| unsafe {
            let path = NSString::from_str("estimatedProgress");
            let _: () = msg_send![view, addObserver: &**watch, forKeyPath: &*path, options: 1usize, context: std::ptr::null_mut::<std::ffi::c_void>()];
        });
        // WebKit caches which delegate selectors exist when assigning it.
        let delegate: *mut AnyObject = unsafe { msg_send![view, navigationDelegate] };
        unsafe {
            let _: () = msg_send![view, setNavigationDelegate: std::ptr::null::<AnyObject>()];
            let _: () = msg_send![view, setNavigationDelegate: delegate];
        }
        tell(Some(opened));
        let target = NSString::from_str(target);
        let url: Option<Retained<AnyObject>> =
            unsafe { msg_send![class!(NSURL), URLWithString: &*target] };
        let url = url.ok_or("page target unavailable")?;
        let request: Retained<AnyObject> =
            unsafe { msg_send![class!(NSURLRequest), requestWithURL: &*url] };
        let navigation: *mut AnyObject = unsafe { msg_send![view, loadRequest: &*request] };
        if navigation.is_null() {
            return Err("page navigation did not start".into());
        }
        with_pages(|pages| pages.expect(key, navigation as usize));
        Ok(())
    }

    fn retire_on_main(tab: &str, key: ViewKey) {
        let observation = OBSERVATIONS.with(|observations| {
            let mut observations = observations.borrow_mut();
            if observations
                .get(tab)
                .is_some_and(|observation| observation.key == key)
            {
                observations.remove(tab)
            } else {
                None
            }
        });
        if let Some(observation) = observation {
            with_pages(|pages| pages.detach(key));
            PROGRESS.with(|watch| unsafe {
                let path = NSString::from_str("estimatedProgress");
                let _: () =
                    msg_send![&*observation.view, removeObserver: &**watch, forKeyPath: &*path];
            });
        }
    }

    // Only synchronous main-thread setup/error paths retire the current
    // observation by tab. Deferred close captures an exact key instead.
    fn forget_on_main(tab: &str) {
        let key = OBSERVATIONS
            .with(|observations| {
                observations
                    .borrow()
                    .get(tab)
                    .map(|observation| observation.key)
            })
            .or_else(|| with_pages(|pages| pages.key_for_tab(tab)).flatten());
        if let Some(key) = key {
            with_pages(|pages| pages.detach(key));
            retire_on_main(tab, key);
        }
    }

    pub fn forget(tab: &str) {
        let key = with_pages(|pages| {
            let key = pages.key_for_tab(tab)?;
            pages.detach(key);
            Some(key)
        })
        .flatten();
        if let (Some(app), Some(key)) = (APP.get(), key) {
            let tab = tab.to_owned();
            let _ = app.run_on_main_thread(move || retire_on_main(&tab, key));
        }
    }

    pub fn take_failed(tab: &str) -> Option<String> {
        crate::page_navigation::retry_target(tab)
    }

    /// Wrap only wry's two existing callbacks, preserving their original
    /// IMP (including pending-script injection). The added selectors do
    /// not replace authentication/download/navigation policy methods.
    unsafe fn install(view: *mut AnyObject) -> Result<(), String> {
        let delegate: *mut AnyObject = unsafe { msg_send![view, navigationDelegate] };
        if delegate.is_null() {
            return Err("page navigation delegate unavailable".into());
        }
        let class = unsafe { objc2::ffi::object_getClass(delegate) } as *mut AnyClass;
        if let Some(original) = ORIGINALS.get() {
            return if original.class == class as usize {
                Ok(())
            } else {
                Err("page navigation delegate changed".into())
            };
        }
        let original = |selector| unsafe {
            let method = objc2::ffi::class_getInstanceMethod(class, selector);
            if method.is_null() {
                None
            } else {
                objc2::ffi::method_getImplementation(method)
            }
        };
        let committed_original = original(sel!(webView:didCommitNavigation:))
            .ok_or("page commit callback unavailable")?;
        let finished_original = original(sel!(webView:didFinishNavigation:))
            .ok_or("page finish callback unavailable")?;
        for selector in [
            sel!(webView:didStartProvisionalNavigation:),
            sel!(webView:didFailProvisionalNavigation:withError:),
            sel!(webView:didFailNavigation:withError:),
        ] {
            if original(selector).is_some() {
                return Err("page navigation callback already owned".into());
            }
        }
        ORIGINALS
            .set(Originals {
                class: class as usize,
                committed: committed_original,
                finished: finished_original,
            })
            .map_err(|_| "page navigation observer already installed")?;
        unsafe {
            for (selector, callback) in [
                (
                    sel!(webView:didCommitNavigation:),
                    committed as NavigationCallback,
                ),
                (
                    sel!(webView:didFinishNavigation:),
                    finished as NavigationCallback,
                ),
            ] {
                let imp: Imp = std::mem::transmute::<NavigationCallback, Imp>(callback);
                objc2::ffi::class_replaceMethod(class, selector, imp, c"v@:@@".as_ptr());
            }
            let started: Imp = std::mem::transmute::<NavigationCallback, Imp>(started);
            objc2::ffi::class_addMethod(
                class,
                sel!(webView:didStartProvisionalNavigation:),
                started,
                c"v@:@@".as_ptr(),
            );
            let failed: Imp = std::mem::transmute::<
                unsafe extern "C-unwind" fn(
                    *mut AnyObject,
                    Sel,
                    *mut AnyObject,
                    *mut AnyObject,
                    *mut AnyObject,
                ),
                Imp,
            >(failed);
            for selector in [
                sel!(webView:didFailProvisionalNavigation:withError:),
                sel!(webView:didFailNavigation:withError:),
            ] {
                objc2::ffi::class_addMethod(class, selector, failed, c"v@:@@@".as_ptr());
            }
            let challenged: Imp = std::mem::transmute::<
                unsafe extern "C-unwind" fn(
                    *mut AnyObject,
                    Sel,
                    *mut AnyObject,
                    *mut AnyObject,
                    *mut block2::Block<dyn Fn(isize, *mut AnyObject)>,
                ),
                Imp,
            >(challenged);
            objc2::ffi::class_addMethod(
                class,
                sel!(webView:didReceiveAuthenticationChallenge:completionHandler:),
                challenged,
                c"v@:@@@?".as_ptr(),
            );
        }
        Ok(())
    }

    unsafe extern "C-unwind" fn started(
        _this: *mut AnyObject,
        _selector: Sel,
        view: *mut AnyObject,
        navigation: *mut AnyObject,
    ) {
        let target = unsafe { url_of(view) };
        tell(
            with_pages(|pages| {
                pages
                    .key(view as usize)
                    .and_then(|key| pages.started(key, navigation as usize, target))
            })
            .flatten(),
        );
    }

    unsafe extern "C-unwind" fn committed(
        this: *mut AnyObject,
        selector: Sel,
        view: *mut AnyObject,
        navigation: *mut AnyObject,
    ) {
        if let Some(original) = ORIGINALS.get() {
            let callback: NavigationCallback = unsafe { std::mem::transmute(original.committed) };
            unsafe {
                callback(this, selector, view, navigation);
            }
        }
        let url = unsafe { url_of(view) };
        if url == "about:blank" {
            return;
        }
        tell(
            with_pages(|pages| {
                pages
                    .key(view as usize)
                    .and_then(|key| pages.committed(key, navigation as usize, url))
            })
            .flatten(),
        );
    }

    unsafe extern "C-unwind" fn finished(
        this: *mut AnyObject,
        selector: Sel,
        view: *mut AnyObject,
        navigation: *mut AnyObject,
    ) {
        if let Some(original) = ORIGINALS.get() {
            let callback: NavigationCallback = unsafe { std::mem::transmute(original.finished) };
            unsafe {
                callback(this, selector, view, navigation);
            }
        }
        let url = unsafe { url_of(view) };
        if url == "about:blank" {
            return;
        }
        let title: Option<Retained<NSString>> = unsafe { msg_send![view, title] };
        let route_failed =
            title.is_some_and(|title| title.to_string() == khor_node::tunnel::PAGE_FAILED_TITLE);
        tell(
            with_pages(|pages| {
                pages.key(view as usize).and_then(|key| {
                    if route_failed {
                        pages.failed(
                            key,
                            navigation as usize,
                            crate::page_navigation::route_error(url),
                        )
                    } else {
                        pages.finished(key, navigation as usize, url)
                    }
                })
            })
            .flatten(),
        );
    }

    unsafe extern "C-unwind" fn failed(
        _this: *mut AnyObject,
        _selector: Sel,
        view: *mut AnyObject,
        navigation: *mut AnyObject,
        error: *mut AnyObject,
    ) {
        let Some(raw) = (unsafe { super::details(error) }) else {
            return;
        };
        let change = with_pages(|pages| {
            pages
                .key(view as usize)
                .and_then(|key| pages.failed(key, navigation as usize, raw))
        })
        .flatten();
        if let Some(change) = change.as_ref() {
            unsafe {
                crate::page_trust::offered(&change.state.tab, &change.state.exit, error);
            }
        }
        tell(change);
    }

    unsafe extern "C-unwind" fn challenged(
        _this: *mut AnyObject,
        _selector: Sel,
        view: *mut AnyObject,
        challenge: *mut AnyObject,
        done: *mut block2::Block<dyn Fn(isize, *mut AnyObject)>,
    ) {
        let exit = with_pages(|pages| {
            pages
                .key(view as usize)
                .and_then(|key| pages.state(key).map(|state| state.exit.clone()))
        })
        .flatten();
        unsafe {
            crate::page_trust::answer(exit.as_deref(), challenge, &*done);
        }
    }
}
