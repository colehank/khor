//! **A browsing tab's page on a phone** (2026-10-03, the owner: the
//! browser did nothing on the iPhone).
//!
//! # Why the phone drew nothing
//!
//! On a desktop a tab's page is a child webview of khor's window
//! (`open_page`, tauri's `add_child`), wired to the borrow's proxy and
//! laid over the tab body. tauri has no child webview on a phone —
//! `add_child` is `all(desktop, feature = "unstable")` — so the phone's
//! commands refused (`page-needs-desktop`) and the landing listed pages
//! over an empty body.
//!
//! # What stands in for it
//!
//! The same thing, built by hand: a `WKWebView` added beside khor's own
//! webview in its superview, framed over the tab body, its own data store
//! per exit machine with that machine's borrow as its proxy
//! (`WKWebsiteDataStore.proxyConfigurations`, iOS 17) and the page shim
//! injected — the desktop's three halves (proxy, shim, browser agent),
//! never one without the others. **Not an `iframe`**: a page in a frame
//! of khor's face cannot go through a proxy at all (a frame fetches with
//! its parent's network), and most sites refuse to be framed
//! (`X-Frame-Options`, `frame-ancestors`), which a frame cannot get past
//! without a rewriting proxy that breaks the pages it rewrites.
//!
//! What the face is told comes from polling the views (every 400 ms):
//! where each page is once a load ends (`page:went`), its title
//! (`page:title`) and whether it is loading (`page:load`) — the desktop's
//! three events; the loading is also observed, so it is told the moment
//! it changes (`load_heard`, UX-38 §2). A link that asks for a new window
//! is told as `page:popup`, and the face opens it as a new tab beside this
//! one — the desktop's answer (`on_new_window`), the UI delegate's
//! `createWebView` (`PageAsk`, UX-42 §15). ⚠️ 墓碑: a script made every
//! such link and `window.open` navigate this page instead.
//!
//! A load that fails is heard by a navigation delegate of khor's own
//! (`PageFail`, UX-22): the two failure methods, and the two policy
//! methods that turn what a page cannot show into a download (UX-42 §16)
//! — every other navigation is allowed, as it was with none. It is the
//! downloads' delegate too, and offers each file in the share sheet.
//!
//! Main thread only: every function here runs inside `with_webview` or
//! `run_on_main_thread`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, NSObject};
use objc2::{class, define_class, msg_send, ClassType};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use tauri::{Emitter, Manager};

use crate::{PageBox, PageWord};

struct Page {
    view: Retained<AnyObject>,
    /// The exit machine the view's data store is wired to (`""`: direct).
    exit: String,
    /// Its borrow's proxy, for a loopback address the page goes to on its
    /// own (`failed`).
    proxy: Option<String>,
    /// Where the page last was, as asked or as it arrived.
    url: String,
    /// What the face was last told, so a poll reports changes only.
    told_url: String,
    told_title: String,
    loading: bool,
    /// Whether the page could step back and forward when the face was
    /// last told (`page:steps`, round UX-3: the deck greys ← and draws →
    /// only where there is somewhere to go).
    told_steps: Option<(bool, bool)>,
    /// The view's scroll view, watched for `page:scroll` (round UX-4).
    scroll: Retained<AnyObject>,
    /// Existing WebKit scroll views: root plus native overflow scrollers.
    sheet_scrolls: Vec<Retained<AnyObject>>,
    /// Where it was last told to be, and when.
    told_y: f64,
    told_at: Option<std::time::Instant>,
    sheet: Option<SheetPan>,
}

thread_local! {
    static SHEET_CONFIG: RefCell<HashMap<String,(f64,f64)>> = RefCell::new(HashMap::new());
    static PAGES: RefCell<HashMap<String, Page>> = RefCell::new(HashMap::new());
    /// The one observer every page's scroll view reports to.
    static WATCH: Retained<ScrollWatch> = unsafe { msg_send![ScrollWatch::class(), new] };
    /// Every page's navigation delegate (WebKit holds it weakly).
    static FAIL: Retained<PageFail> = unsafe { msg_send![PageFail::class(), new] };
    /// Every page's UI delegate, for its alerts (also held weakly).
    static ASK: Retained<PageAsk> = unsafe { msg_send![PageAsk::class(), new] };
}

/// The app, for the scroll observer, which is called by UIKit with
/// nothing of khor's in hand.
static APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and this class
    // has no ivars and no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "KhorPageScrollWatch"]
    struct ScrollWatch;

    impl ScrollWatch {
        // Observe the existing scroll recognizer; do not replace WebKit's delegate.
        #[unsafe(method(sheetPan:))]
        fn sheet_pan(&self, rec: *mut AnyObject) {
            sheet_panned(rec);
        }

        // SAFETY: the signature is NSKeyValueObserving's.
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn observe_value(
            &self,
            key: *mut AnyObject,
            object: *mut AnyObject,
            change: *mut AnyObject,
            _context: *mut std::ffi::c_void,
        ) {
            // SAFETY: KVO hands the key path as an NSString.
            let loading = !key.is_null() && unsafe { (*(key as *const NSString)).to_string() } == "loading";
            if loading {
                // Heard on the beat after: a load begun from inside this
                // module (`open`, `step`) fires while the pages are held.
                // So the value is the change's own, taken now: read again
                // on that beat, a load quicker than it is `false` twice
                // and says nothing (UX-42, audit row 6).
                let view = object as usize;
                let now = new_bool(change);
                if let Some(app) = APP.get() {
                    let _ = app.run_on_main_thread(move || load_heard(view, now));
                }
            } else {
                scrolled(object);
            }
        }
    }
);

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and this class
    // has no ivars and no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "KhorPageFail"]
    struct PageFail;

    impl PageFail {
        // SAFETY: the signatures are WKNavigationDelegate's.
        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn failed_early(&self, view: *mut AnyObject, _nav: *mut AnyObject, error: *mut AnyObject) {
            failed(view, error);
        }

        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn failed_late(&self, view: *mut AnyObject, _nav: *mut AnyObject, error: *mut AnyObject) {
            failed(view, error);
        }

        // **A download is never silent** (UX-42 §16): what the page
        // cannot show, an attachment, and a link with `download` become a
        // download, and its file is offered in the share sheet. ⚠️ 墓碑:
        // with no policy methods WebKit showed what it could and dropped
        // the rest without a word.
        // A certificate passed for this run (UX-42 §14, `page_trust`).
        #[unsafe(method(webView:didReceiveAuthenticationChallenge:completionHandler:))]
        fn challenged(&self, view: *mut AnyObject, challenge: *mut AnyObject, done: &block2::Block<dyn Fn(isize, *mut AnyObject)>) {
            let exit = PAGES.with(|pages| {
                let pages = pages.try_borrow().ok()?;
                pages.values().find(|p| Retained::as_ptr(&p.view) as *mut AnyObject == view).map(|p| p.exit.clone())
            });
            // SAFETY: WebKit hands a live challenge and its handler.
            unsafe { crate::page_trust::answer(exit.as_deref(), challenge, done) };
        }

        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn decide_action(&self, _view: *mut AnyObject, action: *mut AnyObject, done: &block2::Block<dyn Fn(isize)>) {
            // SAFETY: a live WKNavigationAction; `shouldPerformDownload` is iOS 14.5's.
            let download = unsafe {
                let can: bool = msg_send![action, respondsToSelector: objc2::sel!(shouldPerformDownload)];
                can && { let d: bool = msg_send![action, shouldPerformDownload]; d }
            };
            // 2: WKNavigationActionPolicyDownload; 1: allow, as with no method.
            done.call((if download { 2 } else { 1 },));
        }

        #[unsafe(method(webView:decidePolicyForNavigationResponse:decisionHandler:))]
        fn decide_response(&self, _view: *mut AnyObject, response: *mut AnyObject, done: &block2::Block<dyn Fn(isize)>) {
            // 2: WKNavigationResponsePolicyDownload; 1: allow.
            done.call((if unsafe { downloads(response) } { 2 } else { 1 },));
        }

        #[unsafe(method(webView:navigationAction:didBecomeDownload:))]
        fn action_download(&self, _view: *mut AnyObject, _action: *mut AnyObject, download: *mut AnyObject) {
            // SAFETY: a live WKDownload; this object outlives it (`FAIL`).
            unsafe { let _: () = msg_send![download, setDelegate: self]; }
        }

        #[unsafe(method(webView:navigationResponse:didBecomeDownload:))]
        fn response_download(&self, _view: *mut AnyObject, _response: *mut AnyObject, download: *mut AnyObject) {
            // SAFETY: as above.
            unsafe { let _: () = msg_send![download, setDelegate: self]; }
        }

        // SAFETY: the signatures are WKDownloadDelegate's.
        #[unsafe(method(download:decideDestinationUsingResponse:suggestedFilename:completionHandler:))]
        fn download_to(&self, download: *mut AnyObject, _response: *mut AnyObject, name: *mut AnyObject, done: &block2::Block<dyn Fn(*mut AnyObject)>) {
            // SAFETY: WebKit hands an NSString.
            let name = if name.is_null() { String::new() } else { unsafe { (*(name as *const NSString)).to_string() } };
            match download_place(download, &name) {
                Some(path) => {
                    let text = NSString::from_str(&path.to_string_lossy());
                    // SAFETY: a file URL for a path that does not exist yet.
                    let url: Retained<AnyObject> = unsafe { msg_send![class!(NSURL), fileURLWithPath: &*text, isDirectory: false] };
                    done.call((Retained::as_ptr(&url) as *mut AnyObject,));
                }
                None => done.call((std::ptr::null_mut(),)),
            }
        }

        #[unsafe(method(downloadDidFinish:))]
        fn download_done(&self, download: *mut AnyObject) {
            download_over(download, true);
        }

        #[unsafe(method(download:didFailWithError:resumeData:))]
        fn download_failed(&self, download: *mut AnyObject, _error: *mut AnyObject, _resume: *mut AnyObject) {
            download_over(download, false);
        }
    }
);

/// Whether a response is a download rather than a page: the main frame's,
/// of a type WebKit cannot show, or sent as an attachment — Safari's rule.
///
/// # Safety
/// `response` is a live `WKNavigationResponse`.
unsafe fn downloads(response: *mut AnyObject) -> bool {
    if objc2::runtime::AnyClass::get(c"WKDownload").is_none() {
        return false;
    }
    let main: bool = unsafe { msg_send![response, isForMainFrame] };
    if !main {
        return false;
    }
    let show: bool = unsafe { msg_send![response, canShowMIMEType] };
    if !show {
        return true;
    }
    let r: *mut AnyObject = unsafe { msg_send![response, response] };
    let http = !r.is_null() && unsafe { msg_send![r, isKindOfClass: class!(NSHTTPURLResponse)] };
    if !http {
        return false;
    }
    let key = NSString::from_str("Content-Disposition");
    let v: Option<Retained<NSString>> = unsafe { msg_send![r, valueForHTTPHeaderField: &*key] };
    v.is_some_and(|v| v.to_string().trim_start().to_ascii_lowercase().starts_with("attachment"))
}

thread_local! {
    /// Each running download: its tab and where its file is going.
    static DOWNS: RefCell<HashMap<usize, (String, std::path::PathBuf)>> = RefCell::new(HashMap::new());
}

/// The tab whose page `view` is.
fn tab_of(view: *mut AnyObject) -> Option<String> {
    PAGES.with(|pages| {
        let pages = pages.try_borrow().ok()?;
        pages.iter().find(|(_, p)| Retained::as_ptr(&p.view) as *mut AnyObject == view).map(|(t, _)| t.clone())
    })
}

fn tell_download(tab: &str, text: String) {
    if let Some(app) = APP.get() {
        let _ = app.emit_to("main", "page:download", PageWord { tab: tab.to_owned(), text });
    }
}

/// Where a download's file goes: a folder of its own under the app's
/// temporary directory, under the name the site suggests (its last
/// component only). Told as `started|name`.
fn download_place(download: *mut AnyObject, suggested: &str) -> Option<std::path::PathBuf> {
    // SAFETY: a live WKDownload; `webView` is weak and may be gone.
    let view: *mut AnyObject = unsafe { msg_send![download, webView] };
    let tab = tab_of(view)?;
    let name = std::path::Path::new(suggested).file_name().and_then(|n| n.to_str()).filter(|n| !n.is_empty()).unwrap_or("download").to_owned();
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join("khor-downloads").join(N.fetch_add(1, Ordering::Relaxed).to_string());
    let _ = std::fs::remove_dir_all(&dir);
    if std::fs::create_dir_all(&dir).is_err() {
        tell_download(&tab, format!("failed|{name}"));
        return None;
    }
    let path = dir.join(&name);
    tell_download(&tab, format!("started|{name}"));
    DOWNS.with(|d| d.borrow_mut().insert(download as usize, (tab, path.clone())));
    Some(path)
}

/// A download ended: its file is offered in the share sheet (「存储到
/// "文件"」 is there, as Safari's downloads land), and the folder goes
/// once the sheet is done; a failure, or no screen to offer it on, is
/// said on the page's line.
fn download_over(download: *mut AnyObject, ok: bool) {
    let Some((tab, path)) = DOWNS.with(|d| d.borrow_mut().remove(&(download as usize))) else { return };
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let dir = path.parent().map(std::path::Path::to_path_buf);
    let shown = ok && {
        let view = PAGES.with(|p| p.borrow().get(&tab).map(|p| p.view.clone()));
        // SAFETY: main thread; a live view.
        view.is_some_and(|v| unsafe { share(&v, &path, dir.clone()) })
    };
    if shown {
        tell_download(&tab, format!("offered|{name}"));
    } else {
        if let Some(dir) = dir {
            let _ = std::fs::remove_dir_all(dir);
        }
        tell_download(&tab, format!("failed|{name}"));
    }
}

/// The view controller in front, over `view`'s window.
///
/// # Safety
/// Main thread; `view` is a live UIView.
unsafe fn front_controller(view: *mut AnyObject) -> *mut AnyObject {
    let window: *mut AnyObject = unsafe { msg_send![view, window] };
    let mut top: *mut AnyObject = if window.is_null() { std::ptr::null_mut() } else { unsafe { msg_send![window, rootViewController] } };
    while !top.is_null() {
        let next: *mut AnyObject = unsafe { msg_send![top, presentedViewController] };
        if next.is_null() {
            break;
        }
        top = next;
    }
    top
}

/// Browser downloads own their temporary directory until the native
/// sheet finishes. File handoffs use the same presenter with a cache.
unsafe fn share(view: &AnyObject, path: &std::path::Path, dir: Option<std::path::PathBuf>) -> bool {
    unsafe { crate::phone_share::show(view, path, move |_| {
        if let Some(dir) = &dir { let _ = std::fs::remove_dir_all(dir); }
    }) }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and this class
    // has no ivars and no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "KhorPageAsk"]
    struct PageAsk;

    impl PageAsk {
        // SAFETY: the signatures are WKUIDelegate's.
        #[unsafe(method(webView:runJavaScriptAlertPanelWithMessage:initiatedByFrame:completionHandler:))]
        fn alert(&self, view: *mut AnyObject, message: *mut AnyObject, _frame: *mut AnyObject, done: &block2::Block<dyn Fn()>) {
            let done = done.copy();
            ask(view, message, None, false, move |_, _| done.call(()));
        }

        #[unsafe(method(webView:runJavaScriptConfirmPanelWithMessage:initiatedByFrame:completionHandler:))]
        fn confirm(&self, view: *mut AnyObject, message: *mut AnyObject, _frame: *mut AnyObject, done: &block2::Block<dyn Fn(objc2::runtime::Bool)>) {
            let done = done.copy();
            ask(view, message, None, true, move |ok, _| done.call((objc2::runtime::Bool::new(ok),)));
        }

        #[unsafe(method(webView:runJavaScriptTextInputPanelWithPrompt:defaultText:initiatedByFrame:completionHandler:))]
        fn prompt(&self, view: *mut AnyObject, prompt: *mut AnyObject, text: *mut AnyObject, _frame: *mut AnyObject, done: &block2::Block<dyn Fn(*mut AnyObject)>) {
            let done = done.copy();
            ask(view, prompt, Some(text), true, move |ok, said: Option<Retained<AnyObject>>| {
                let said = if ok { said.as_deref().map_or(std::ptr::null_mut(), |s| s as *const AnyObject as *mut AnyObject) } else { std::ptr::null_mut() };
                done.call((said,));
            });
        }

        /// **A link that asks for a new window is a new tab** (UX-42 §15):
        /// the face mints it beside this one (`page:popup`), as the
        /// desktop's `on_new_window` does; no view is made here, so the
        /// page's own `window.open` answers null.
        #[unsafe(method(webView:createWebViewWithConfiguration:forNavigationAction:windowFeatures:))]
        fn new_window(&self, view: *mut AnyObject, _cfg: *mut AnyObject, action: *mut AnyObject, _features: *mut AnyObject) -> *mut AnyObject {
            popped(view, action);
            std::ptr::null_mut()
        }
    }
);

/// **A page's own alert, confirm and prompt are asked** (UX-38 audit:
/// with no UI delegate WebKit answered them itself — an alert never
/// showed, a confirm said false, a prompt null — and the page went on as
/// if the person had refused). The system's alert, over the page, as
/// Safari asks; the answer goes back exactly once, also when there is
/// nothing to show it on.
fn ask(
    view: *mut AnyObject,
    message: *mut AnyObject,
    field: Option<*mut AnyObject>,
    cancel: bool,
    answer: impl FnOnce(bool, Option<Retained<AnyObject>>) + 'static,
) {
    let answer = std::rc::Rc::new(RefCell::new(Some(answer)));
    // SAFETY: WebKit calls this on the main thread with a live view; every
    // selector is documented UIKit.
    unsafe {
        let top = front_controller(view);
        if top.is_null() {
            if let Some(a) = answer.borrow_mut().take() {
                a(false, None);
            }
            return;
        }
        // 1: UIAlertControllerStyleAlert.
        let alert: Retained<AnyObject> = msg_send![class!(UIAlertController), alertControllerWithTitle: std::ptr::null::<AnyObject>(), message: message, preferredStyle: 1isize];
        if let Some(text) = field {
            let text = if text.is_null() { None } else { Some(Retained::retain(text).unwrap()) };
            let fill = block2::RcBlock::new(move |f: *mut AnyObject| {
                if let Some(t) = &text {
                    let _: () = msg_send![f, setText: &**t];
                }
            });
            let _: () = msg_send![&*alert, addTextFieldWithConfigurationHandler: &*fill];
        }
        let weak: *mut AnyObject = Retained::as_ptr(&alert) as *mut AnyObject;
        let act = |title: String, style: isize, ok: bool| {
            let answer = answer.clone();
            let handler = block2::RcBlock::new(move |_a: *mut AnyObject| {
                let said = if ok {
                    let fields: *mut AnyObject = msg_send![weak, textFields];
                    let first: *mut AnyObject = if fields.is_null() { std::ptr::null_mut() } else { msg_send![fields, firstObject] };
                    if first.is_null() {
                        None
                    } else {
                        let t: Option<Retained<AnyObject>> = msg_send![first, text];
                        t
                    }
                } else {
                    None
                };
                if let Some(a) = answer.borrow_mut().take() {
                    a(ok, said);
                }
            });
            let title = NSString::from_str(&title);
            let action: Retained<AnyObject> = msg_send![class!(UIAlertAction), actionWithTitle: &*title, style: style, handler: &*handler];
            action
        };
        if cancel {
            // 1: UIAlertActionStyleCancel.
            let no = act(khor_catalog::msg::page_ask_cancel().into(), 1, false);
            let _: () = msg_send![&*alert, addAction: &*no];
        }
        let yes = act(khor_catalog::msg::page_ask_ok().into(), 0, true);
        let _: () = msg_send![&*alert, addAction: &*yes];
        let _: () = msg_send![top, presentViewController: &*alert, animated: true, completion: std::ptr::null::<AnyObject>()];
    }
}

/// The address a new-window action asks for, told to the face as the
/// opener tab's `page:popup`. A window opened blank (`about:blank`, to be
/// written into by its opener) has nowhere to go and is not told.
fn popped(view: *mut AnyObject, action: *mut AnyObject) {
    let Some(app) = APP.get() else { return };
    // SAFETY: WebKit hands a live WKNavigationAction on the main thread.
    let url = unsafe {
        let req: *mut AnyObject = msg_send![action, request];
        let u: *mut AnyObject = if req.is_null() { std::ptr::null_mut() } else { msg_send![req, URL] };
        let s: Option<Retained<NSString>> = if u.is_null() { None } else { msg_send![u, absoluteString] };
        s.map(|s| s.to_string()).unwrap_or_default()
    };
    if url.is_empty() || url == "about:blank" {
        return;
    }
    if let Some(tab) = tab_of(view) {
        let _ = app.emit_to("main", "page:popup", PageWord { tab, text: url });
    }
}

/// **A load that failed says why** (UX-22): `page:fail` with the error
/// (`page_fail::word`), and the light out at once rather than at the next
/// poll.
fn failed(view: *mut AnyObject, error: *mut AnyObject) {
    let Some(app) = APP.get() else { return };
    // SAFETY: WebKit hands a live NSError.
    let Some(text) = (unsafe { crate::page_fail::word(error) }) else { return };
    let found = PAGES.with(|pages| {
        let mut pages = pages.try_borrow_mut().ok()?;
        let (tab, page) = pages.iter_mut().find(|(_, p)| Retained::as_ptr(&p.view) as *mut AnyObject == view)?;
        page.loading = false;
        Some((tab.clone(), page.proxy.clone(), page.exit.clone()))
    });
    let Some((tab, proxy, exit)) = found else { return };
    // SAFETY: a live NSError.
    unsafe { crate::page_trust::offered(&tab, &exit, error) };
    let mut parts = text.splitn(4, '|');
    let (code, domain, _, url) = (parts.next(), parts.next(), parts.next(), parts.next().unwrap_or_default().to_owned());
    // **A loopback address the page went to on its own** (a link, a
    // redirect; UX-38 §1): refused because it went to the phone's own
    // port. Piped to the machine's, then asked again, once per port.
    if let (Some(proxy), Some((host, port))) = (&proxy, khor_gui_core::web::loopback_of(&url)) {
        if domain == Some("NSURLErrorDomain") && code == Some("-1004") && !khor_gui_core::web::loopback_held(port) {
            let (proxy, app, tab) = (proxy.clone(), app.clone(), tab.clone());
            tauri::async_runtime::spawn(async move {
                if khor_gui_core::web::hold_loopback(&host, port, &proxy).await.is_err() {
                    return;
                }
                let _ = app.run_on_main_thread(move || {
                    if let Some(page) = PAGES.with(|p| p.borrow().get(&tab).map(|p| p.view.clone())) {
                        // SAFETY: main thread, a live view.
                        unsafe { load(&page, &url) };
                    }
                });
            });
            return;
        }
    }
    // A CONNECT the machine could not carry through: WebKit asks eight
    // times and then says the connection was lost (measured: -1005 on
    // every status a proxy answers), which is the route's failure, not
    // the site's.
    let text = if proxy.is_some() && domain == Some("NSURLErrorDomain") && code == Some("-1005") && khor_gui_core::web::loopback_of(&url).is_none() {
        format!("route|khor||{url}")
    } else {
        text
    };
    let _ = app.emit_to("main", "page:load", PageWord { tab: tab.clone(), text: "finished".into() });
    let _ = app.emit_to("main", "page:fail", PageWord { tab, text });
}

/// **Whether the page is loading, as WebKit says it** (UX-38 §2): the
/// view's `loading` observed, so a load is `started` and `finished` the
/// moment it is — the deck's line runs exactly as long. ⚠️ 墓碑: only the
/// 400 ms poll told it, so a load quicker than a beat said nothing and a
/// new page's first load never said `started`; the face lit the line from
/// its own errand instead, and coming back to a tab ran it for the whole
/// ceiling with nothing loading (measured on the simulator: 30 s).
///
/// `now` is the value the change carried (`NSKeyValueChangeNewKey`), so a
/// load that began and ended inside one beat is still `started` then
/// `finished`, and the face's hold (`use-loading`'s `SETTLE_MS`) shows
/// its line briefly; `None` (no value in the change) reads the view.
fn load_heard(view: usize, now: Option<bool>) {
    let Some(app) = APP.get() else { return };
    let word = PAGES.with(|pages| {
        let mut pages = pages.try_borrow_mut().ok()?;
        let (tab, page) = pages.iter_mut().find(|(_, p)| Retained::as_ptr(&p.view) as usize == view)?;
        let loading: bool = now.unwrap_or_else(|| unsafe { msg_send![&*page.view, isLoading] });
        if loading == page.loading {
            return None;
        }
        page.loading = loading;
        Some(PageWord { tab: tab.clone(), text: if loading { "started" } else { "finished" }.into() })
    });
    if let Some(w) = word {
        let _ = app.emit_to("main", "page:load", w);
    }
}

unsafe extern "C" {
    /// Foundation's key for a KVO change's new value.
    static NSKeyValueChangeNewKey: *const AnyObject;
}

/// A KVO change's new value, as a bool (`isLoading` arrives boxed in an
/// `NSNumber`).
fn new_bool(change: *mut AnyObject) -> Option<bool> {
    if change.is_null() {
        return None;
    }
    // SAFETY: KVO hands the change as an NSDictionary, and the new value
    // of a BOOL property is an NSNumber (or NSNull, never for this one).
    unsafe {
        let v: *mut AnyObject = msg_send![change, objectForKey: NSKeyValueChangeNewKey];
        if v.is_null() {
            return None;
        }
        let number: bool = msg_send![v, isKindOfClass: class!(NSNumber)];
        number.then(|| msg_send![v, boolValue])
    }
}

/// UIKit owns webpage touches, so DOM bubbling cannot resize a session sheet.
struct SheetPan {
    h: f64,
    full: f64,
    last: Option<NSPoint>,
    owned: bool,
    locked: bool,
    offset: f64,
    start_h: f64,
    start_y: f64,
    vertical: Option<bool>,
    scroll: Option<Retained<AnyObject>>,
}
impl SheetPan {
    fn new(h: f64, full: f64) -> Self {
        Self {
            h,
            full,
            last: None,
            owned: false,
            locked: false,
            offset: 0.0,
            start_h: h,
            start_y: 0.0,
            vertical: None,
            scroll: None,
        }
    }
    /// Lifecycle cancellation must restore the same scroll view that drove
    /// this pan, including a site's inner overflow view.
    fn cancel(&mut self) -> bool {
        let owned = self.owned;
        if owned {
            self.h = self.start_h;
            if let Some(scroll) = self.scroll.as_ref() {
                let at: NSPoint = unsafe { msg_send![&**scroll, contentOffset] };
                let _: () = unsafe {
                    msg_send![&**scroll, setContentOffset: NSPoint::new(at.x, self.start_y)]
                };
            }
        }
        self.last = None;
        self.owned = false;
        self.locked = false;
        self.vertical = None;
        owned
    }
}
pub fn sheet(tab: &str, geometry: Option<(f64, f64)>) {
    SHEET_CONFIG.with(|c| {
        let mut c = c.borrow_mut();
        if let Some(g) = geometry {
            c.insert(tab.to_owned(), g);
        } else {
            c.remove(tab);
        }
    });
    PAGES.with(|p| {
        if let Some(page) = p.borrow_mut().get_mut(tab) {
            if let Some((h, full)) = geometry {
                if let Some(s) = page.sheet.as_mut() {
                    s.full = full;
                    if s.last.is_none() {
                        s.h = h;
                    }
                } else {
                    page.sheet = Some(SheetPan::new(h, full));
                }
            } else {
                if page.sheet.as_mut().is_some_and(SheetPan::cancel) {
                    if let Some(app) = APP.get() {
                        let _ = app.emit_to(
                            "main",
                            "page:sheet",
                            PageWord {
                                tab: tab.to_owned(),
                                text: "cancel,0,0".into(),
                            },
                        );
                    }
                }
                page.sheet = None;
            }
            watch_sheet_scrolls(tab, page);
        }
    });
}
/// Add targets and KVO to the existing recognizers; WebKit keeps their
/// delegates, failure relationships, long presses and horizontal gestures.
fn watch_sheet_scrolls(tab: &str, page: &mut Page) {
    unsafe fn collect(view: *mut AnyObject, found: &mut Vec<Retained<AnyObject>>) {
        if unsafe { msg_send![view, isKindOfClass: class!(UIScrollView)] } {
            found.push(unsafe { Retained::retain(view) }.unwrap());
        }
        let subs: Retained<AnyObject> = unsafe { msg_send![view, subviews] };
        let n: usize = unsafe { msg_send![&*subs, count] };
        for i in 0..n {
            let child: *mut AnyObject = unsafe { msg_send![&*subs, objectAtIndex: i] };
            unsafe { collect(child, found) };
        }
    }
    let mut found = vec![page.scroll.clone()];
    if page.sheet.is_some() {
        found.clear();
        unsafe { collect(Retained::as_ptr(&page.scroll) as *mut AnyObject, &mut found) };
    }
    if let Some(s) = page.sheet.as_mut() {
        if s.scroll.as_ref().is_some_and(|active| {
            !found
                .iter()
                .any(|v| Retained::as_ptr(v) == Retained::as_ptr(active))
        }) {
            // A site's navigation can remove the scrolling element during
            // a pan. End ownership before removing its observation.
            if s.cancel() {
                if let Some(app) = APP.get() {
                    let _ = app.emit_to(
                        "main",
                        "page:sheet",
                        PageWord {
                            tab: tab.to_owned(),
                            text: "cancel,0,0".into(),
                        },
                    );
                }
            }
            s.scroll = None;
        }
    }
    WATCH.with(|w| {
        let key = NSString::from_str("contentOffset");
        for old in &page.sheet_scrolls {
            if found.iter().any(|v| Retained::as_ptr(v) == Retained::as_ptr(old)) { continue; }
            unsafe {
                let pan: *mut AnyObject = msg_send![&**old, panGestureRecognizer];
                let _: () = msg_send![pan, removeTarget: &**w, action: objc2::sel!(sheetPan:)];
                let _: () = msg_send![&**old, removeObserver: &**w, forKeyPath: &*key];
            }
        }
        for view in &found {
            if page.sheet_scrolls.iter().any(|v| Retained::as_ptr(v) == Retained::as_ptr(view)) { continue; }
            unsafe {
                let pan: *mut AnyObject = msg_send![&**view, panGestureRecognizer];
                let _: () = msg_send![pan, addTarget: &**w, action: objc2::sel!(sheetPan:)];
                let _: () = msg_send![&**view, addObserver: &**w, forKeyPath: &*key, options: 1usize, context: std::ptr::null_mut::<std::ffi::c_void>()];
            }
        }
    });
    page.sheet_scrolls = found;
}

/// Pick the innermost vertical scroller under the touch. Ancestors can
/// recognize simultaneously, but only this source may drive one sheet pan.
unsafe fn sheet_scroll_at(page: &Page, rec: *mut AnyObject) -> *mut AnyObject {
    let point: NSPoint = unsafe { msg_send![rec, locationInView: &*page.view] };
    let mut hit: *mut AnyObject = unsafe {
        msg_send![&*page.view, hitTest: point, withEvent: std::ptr::null_mut::<AnyObject>()]
    };
    while !hit.is_null() && hit != Retained::as_ptr(&page.view) as *mut AnyObject {
        if page
            .sheet_scrolls
            .iter()
            .any(|v| Retained::as_ptr(v) as *mut AnyObject == hit)
        {
            let size: NSSize = unsafe { msg_send![hit, contentSize] };
            let bounds: NSRect = unsafe { msg_send![hit, bounds] };
            let enabled: bool = unsafe { msg_send![hit, isScrollEnabled] };
            let pan: *mut AnyObject = unsafe { msg_send![hit, panGestureRecognizer] };
            let state: isize = unsafe { msg_send![pan, state] };
            if enabled && (state == 1 || state == 2) && size.height > bounds.size.height + 1.0 {
                return hit;
            }
        }
        hit = unsafe { msg_send![hit, superview] };
    }
    Retained::as_ptr(&page.scroll) as *mut AnyObject
}

fn sheet_panned(rec: *mut AnyObject) {
    let Some(app) = APP.get() else { return };
    let mut word = None;
    PAGES.with(|p| {
        let Ok(mut pages) = p.try_borrow_mut() else {
            return;
        };
        let view: *mut AnyObject = unsafe { msg_send![rec, view] };
        for (tab, page) in pages.iter_mut() {
            if !page
                .sheet_scrolls
                .iter()
                .any(|v| Retained::as_ptr(v) as *mut AnyObject == view)
            {
                continue;
            }
            let state: isize = unsafe { msg_send![rec, state] };
            if state == 1 && unsafe { sheet_scroll_at(page, rec) } != view {
                return;
            }
            let Some(s) = page.sheet.as_mut() else { return };
            if state != 1
                && s.scroll
                    .as_ref()
                    .is_none_or(|v| Retained::as_ptr(v) as *mut AnyObject != view)
            {
                return;
            }
            let point: NSPoint =
                unsafe { msg_send![rec, translationInView: std::ptr::null_mut::<AnyObject>()] };
            let at: NSPoint = unsafe { msg_send![view, contentOffset] };
            if state == 1 {
                // Translation already includes the recognition threshold.
                // Consume it now instead of throwing away the first movement.
                s.last = Some(NSPoint::new(0.0, 0.0));
                s.owned = false;
                s.locked = false;
                s.vertical = None;
                s.scroll = Some(unsafe { Retained::retain(view) }.unwrap());
                s.offset = at.y;
                s.start_h = s.h;
                s.start_y = at.y;
            }
            let Some(last) = s.last else { return };
            let touches: usize = unsafe { msg_send![rec, numberOfTouches] };
            if state == 4 || state == 5 || touches > 1 {
                if s.cancel() {
                    word = Some(PageWord {
                        tab: tab.clone(),
                        text: "cancel,0,0".into(),
                    });
                }
                return;
            }
            let velocity: NSPoint =
                unsafe { msg_send![rec, velocityInView: std::ptr::null_mut::<AnyObject>()] };
            if state == 3 {
                if s.owned {
                    word = Some(PageWord {
                        tab: tab.clone(),
                        text: format!("end,{},{:.3}", s.h, velocity.y),
                    });
                }
                s.last = None;
                s.owned = false;
                return;
            }
            if state != 1 && state != 2 {
                return;
            }
            let inset: Insets = unsafe { msg_send![view, adjustedContentInset] };
            let top = -inset.top;
            let mut up = last.y - point.y;
            let began = !s.owned;
            if !s.owned {
                if point.x == 0.0 && point.y == 0.0 {
                    return;
                }
                let vertical = *s.vertical.get_or_insert(point.y.abs() > point.x.abs());
                s.last = Some(point);
                if !vertical {
                    return;
                }
                let Some(left) = crate::page_sheet::handoff(
                    s.h,
                    s.full,
                    up,
                    (s.offset - top).max(0.0),
                    (at.y - top).max(0.0),
                ) else {
                    s.offset = at.y;
                    return;
                };
                up = left;
                s.owned = true;
                s.locked = true;
                if up < 0.0 {
                    s.offset = top;
                }
            }
            let size: NSSize = unsafe { msg_send![view, contentSize] };
            let bounds: NSRect = unsafe { msg_send![view, bounds] };
            let max = (size.height - bounds.size.height + inset.bottom - top).max(0.0);
            let (h, y) = crate::page_sheet::travel(s.h, (s.offset - top).max(0.0), up, s.full, max);
            s.h = h;
            s.offset = y + top;
            s.last = Some(point);
            let _: () = unsafe { msg_send![view, setContentOffset: NSPoint::new(at.x, s.offset)] };
            word = Some(PageWord {
                tab: tab.clone(),
                text: format!(
                    "{},{},{:.3}",
                    if began { "start" } else { "move" },
                    h,
                    velocity.y
                ),
            });
            return;
        }
    });
    if let Some(word) = word {
        let _ = app.emit_to("main", "page:sheet", word);
    }
}

/// **Where the page is scrolled, as it moves** (round UX-4, the deck's
/// fold): `page:scroll` with `y,down` or `y,up`, at most once a frame and
/// only for a real move. Observed (KVO on `contentOffset`) rather than
/// polled, so the fold follows the finger rather than the 400 ms beat.
fn scrolled(view: *mut AnyObject) {
    let Some(app) = APP.get() else { return };
    let mut word: Option<PageWord> = None;
    PAGES.with(|pages| {
        // A change made while this module holds the pages (a frame set
        // inside `open`) is not a scroll anybody made.
        let Ok(mut pages) = pages.try_borrow_mut() else {
            return;
        };
        for (tab, page) in pages.iter_mut() {
            if !page
                .sheet_scrolls
                .iter()
                .any(|v| Retained::as_ptr(v) as *mut AnyObject == view)
            {
                continue;
            }
            if let Some(sheet) = page.sheet.as_ref().filter(|s| {
                s.scroll
                    .as_ref()
                    .is_some_and(|v| Retained::as_ptr(v) as *mut AnyObject == view)
            }) {
                let tracking: bool = unsafe { msg_send![view, isTracking] };
                let decelerating: bool = unsafe { msg_send![view, isDecelerating] };
                if sheet.owned
                    || (!tracking && decelerating && sheet.h < sheet.full - 1.0 && sheet.locked)
                {
                    let at: NSPoint = unsafe { msg_send![view, contentOffset] };
                    if (at.y - sheet.offset).abs() > 0.1 {
                        let _: () = unsafe {
                            msg_send![view,setContentOffset:NSPoint::new(at.x,sheet.offset)]
                        };
                    }
                    return;
                }
            }
            if Retained::as_ptr(&page.scroll) as *mut AnyObject != view {
                return;
            }
            // **Only a scroll somebody made** (UX-38 audit): the page's
            // inset set under the deck moves the offset too, and a fresh
            // page arrived with its deck folded (seen on the simulator).
            let moved: bool = unsafe {
                let t: bool = msg_send![&*page.scroll, isTracking];
                let d: bool = msg_send![&*page.scroll, isDecelerating];
                t || d
            };
            let at: NSPoint = unsafe { msg_send![&*page.scroll, contentOffset] };
            if !moved {
                page.told_y = at.y;
                return;
            }
            let dy = at.y - page.told_y;
            if dy.abs() < 1.0 {
                return;
            }
            let now = std::time::Instant::now();
            if page
                .told_at
                .is_some_and(|t| now.duration_since(t).as_millis() < 16)
            {
                return;
            }
            page.told_y = at.y;
            page.told_at = Some(now);
            let way = if dy > 0.0 { "down" } else { "up" };
            word = Some(PageWord {
                tab: tab.clone(),
                text: format!("{:.0},{way}", at.y),
            });
            return;
        }
    });
    if let Some(w) = word {
        let _ = app.emit_to("main", "page:scroll", w);
    }
}

static POLLING: AtomicBool = AtomicBool::new(false);

#[link(name = "Network", kind = "framework")]
unsafe extern "C" {
    fn nw_endpoint_create_host(
        hostname: *const std::ffi::c_char,
        port: *const std::ffi::c_char,
    ) -> *mut AnyObject;
    fn nw_proxy_config_create_http_connect(
        endpoint: *mut AnyObject,
        tls: *mut AnyObject,
    ) -> *mut AnyObject;
}

/// **A single image is centred and fitted, as Safari shows it** (UX-42,
/// audit row 18): WebKit's own image document on the phone lays the
/// picture at its natural size from the top left. Only that document —
/// an `image/*` response whose body is the one `<img>` — is touched; the
/// page's own ground is kept, and a pinch still zooms it.
const FIT_IMAGE: &str = "(function () { \
if (!/^image\\//.test(document.contentType)) return; \
var b = document.body; \
if (!b || b.children.length !== 1 || b.firstElementChild.tagName !== 'IMG') return; \
var at = document.head || document.documentElement; \
var m = document.createElement('meta'); m.name = 'viewport'; \
m.content = 'width=device-width, initial-scale=1'; at.appendChild(m); \
var s = document.createElement('style'); \
s.textContent = 'html, body { height: 100%; margin: 0; } \
body { display: flex; align-items: center; justify-content: center; } \
img { display: block; max-width: 100%; max-height: 100%; width: auto; height: auto; object-fit: contain; }'; \
at.appendChild(s); })();";

/// What to build a page with.
pub struct Wiring {
    /// `host:port` of the borrow's local proxy; `None` goes direct.
    pub proxy: Option<String>,
    pub agent: Option<String>,
}

/// The rectangle, in the superview's points, of a box the face measured
/// in its own CSS pixels (which are points: the face is never zoomed).
///
/// **Not corrected by the face's scroll offset.** The face is never
/// panned at rest (`lib/viewport` takes WebKit's pan back), and the offset
/// read while the keyboard rises is the pan *before* it is taken back —
/// measured on the simulator: the page landed a keyboard's height too
/// high and stayed there, because the face's box did not move again.
unsafe fn frame_of(face: *mut AnyObject, at: PageBox) -> NSRect {
    let base: NSRect = unsafe { msg_send![face, frame] };
    NSRect::new(
        NSPoint::new(base.origin.x + at.x, base.origin.y + at.y),
        NSSize::new(at.w.max(0.0), at.h.max(0.0)),
    )
}

unsafe fn load(view: &AnyObject, url: &str) {
    let text = NSString::from_str(url);
    let u: Option<Retained<AnyObject>> = unsafe { msg_send![class!(NSURL), URLWithString: &*text] };
    let Some(u) = u else { return };
    let req: Retained<AnyObject> = unsafe { msg_send![class!(NSURLRequest), requestWithURL: &*u] };
    let _: Option<Retained<AnyObject>> = unsafe { msg_send![view, loadRequest: &*req] };
}

unsafe fn add_script(controller: *mut AnyObject, source: &str) {
    let src = NSString::from_str(source);
    let a: Allocated<AnyObject> = unsafe { msg_send![class!(WKUserScript), alloc] };
    // 0: WKUserScriptInjectionTimeAtDocumentStart; every frame.
    let script: Retained<AnyObject> = unsafe {
        msg_send![a, initWithSource: &*src, injectionTime: 0isize, forMainFrameOnly: false]
    };
    let _: () = unsafe { msg_send![controller, addUserScript: &*script] };
}

/// A script run once the document is parsed, in the main frame only.
unsafe fn add_end_script(controller: *mut AnyObject, source: &str) {
    let src = NSString::from_str(source);
    let a: Allocated<AnyObject> = unsafe { msg_send![class!(WKUserScript), alloc] };
    // 1: WKUserScriptInjectionTimeAtDocumentEnd.
    let script: Retained<AnyObject> = unsafe {
        msg_send![a, initWithSource: &*src, injectionTime: 1isize, forMainFrameOnly: true]
    };
    let _: () = unsafe { msg_send![controller, addUserScript: &*script] };
}

/// A persistent store per exit machine: its cookies are that exit's,
/// and the face's own store never carries a proxy.
unsafe fn store_for(exit: &str) -> Option<Retained<AnyObject>> {
    // FNV-1a, spelled out (as `build_the_window`'s page store): std's
    // hasher may change between releases, and a changed id is a store
    // whose cookies are silently gone.
    let fnv = |seed: u64, bytes: &[u8]| {
        bytes.iter().fold(seed, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3))
    };
    let key = format!("khor-page:{exit}");
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&fnv(0xcbf2_9ce4_8422_2325, key.as_bytes()).to_le_bytes());
    b[8..].copy_from_slice(&fnv(0x6c62_272e_07bb_0142, key.as_bytes()).to_le_bytes());
    let id = format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    );
    let text = NSString::from_str(&id);
    let a: Allocated<AnyObject> = unsafe { msg_send![class!(NSUUID), alloc] };
    let uuid: Option<Retained<AnyObject>> = unsafe { msg_send![a, initWithUUIDString: &*text] };
    let uuid = uuid?;
    unsafe { msg_send![class!(WKWebsiteDataStore), dataStoreForIdentifier: &*uuid] }
}

/// Opens or re-points `tab`'s page. `Err` when the phone cannot carry a
/// proxy (before iOS 17), so the face says so instead of a blank body.
pub fn open(
    app: &tauri::AppHandle,
    face: *mut AnyObject,
    tab: &str,
    exit: &str,
    url: &str,
    at: PageBox,
    wiring: Wiring,
) -> Result<(), String> {
    start_polling(app);
    let _ = APP.set(app.clone());
    unsafe {
        let rect = frame_of(face, at);
        let reused = PAGES.with(|pages| {
            let mut pages = pages.borrow_mut();
            if let Some(page) = pages.get_mut(tab) {
                if page.exit == exit {
                    if page.url != url {
                        page.url = url.to_owned();
                        load(&page.view, url);
                    }
                    let _: () = msg_send![&*page.view, setFrame: rect];
                    // Shown only if the face still wants it: the person
                    // may have left while the borrow dialled
                    // (`page_wanted`).
                    let _: () = msg_send![&*page.view, setHidden: !crate::page_is_wanted(tab)];
                    return true;
                }
                unwatch(page);
                let _: () = msg_send![&*page.view, removeFromSuperview];
                pages.remove(tab);
            }
            false
        });
        if reused {
            deck_follows(tab);
    crate::menu_overlay::ios::raise();
            return Ok(());
        }
        let store = store_for(exit).ok_or_else(|| "no data store".to_string())?;
        if let Some(addr) = &wiring.proxy {
            let can: bool = msg_send![&*store, respondsToSelector: objc2::sel!(setProxyConfigurations:)];
            if !can {
                return Err(khor_catalog::msg::page_needs_desktop().into());
            }
            let (host, port) = addr.rsplit_once(':').ok_or_else(|| format!("not an address: {addr}"))?;
            let host = std::ffi::CString::new(host).map_err(|e| e.to_string())?;
            let port = std::ffi::CString::new(port).map_err(|e| e.to_string())?;
            let ep = nw_endpoint_create_host(host.as_ptr(), port.as_ptr());
            if ep.is_null() {
                return Err(format!("not an address: {addr}"));
            }
            let ep: Retained<AnyObject> = Retained::from_raw(ep).ok_or("endpoint")?;
            let pc = nw_proxy_config_create_http_connect(Retained::as_ptr(&ep) as *mut AnyObject, std::ptr::null_mut());
            let pc: Retained<AnyObject> = Retained::from_raw(pc).ok_or("proxy config")?;
            let list: Retained<AnyObject> = msg_send![class!(NSArray), arrayWithObject: &*pc];
            let _: () = msg_send![&*store, setProxyConfigurations: &*list];
        }
        let cfg: Retained<AnyObject> = msg_send![class!(WKWebViewConfiguration), new];
        let _: () = msg_send![&*cfg, setWebsiteDataStore: &*store];
        let controller: *mut AnyObject = msg_send![&*cfg, userContentController];
        if wiring.proxy.is_some() {
            // The proxy and the shim ride together (`open_page`).
            add_script(controller, khor_gui_core::web::PAGE_SHIM);
        }
        add_end_script(controller, FIT_IMAGE);
        let a: Allocated<AnyObject> = msg_send![class!(WKWebView), alloc];
        let view: Retained<AnyObject> = msg_send![a, initWithFrame: rect, configuration: &*cfg];
        if let Some(agent) = &wiring.agent {
            let ua = NSString::from_str(agent);
            let _: () = msg_send![&*view, setCustomUserAgent: &*ua];
        }
        // The page's own back and forward, as Safari's edge swipes.
        let _: () = msg_send![&*view, setAllowsBackForwardNavigationGestures: true];
        FAIL.with(|f| {
            let _: () = msg_send![&*view, setNavigationDelegate: &**f];
        });
        ASK.with(|a| {
            let _: () = msg_send![&*view, setUIDelegate: &**a];
        });
        // Attached hidden when the face stopped wanting it during the
        // dial (`page_wanted`); `show_page` brings it back.
        let _: () = msg_send![&*view, setHidden: !crate::page_is_wanted(tab)];
        let parent: *mut AnyObject = msg_send![face, superview];
        let _: () = msg_send![parent, addSubview: &*view];
            crate::menu_overlay::ios::raise();
        let scroll: Retained<AnyObject> = msg_send![&*view, scrollView];
        WATCH.with(|w| {
            let pan:*mut AnyObject=msg_send![&*scroll,panGestureRecognizer];
            let _:()=msg_send![pan,addTarget:&**w,action:objc2::sel!(sheetPan:)];
            let key = NSString::from_str("contentOffset");
            // 1: NSKeyValueObservingOptionNew.
            let _: () = msg_send![&*scroll, addObserver: &**w, forKeyPath: &*key, options: 1usize, context: std::ptr::null_mut::<std::ffi::c_void>()];
            let key = NSString::from_str("loading");
            let _: () = msg_send![&*view, addObserver: &**w, forKeyPath: &*key, options: 1usize, context: std::ptr::null_mut::<std::ffi::c_void>()];
        });
        load(&view, url);
        PAGES.with(|pages| {
            pages.borrow_mut().insert(
                tab.to_owned(),
                Page {
                    view,
                    exit: exit.to_owned(),
                    proxy: wiring.proxy.clone(),
                    url: url.to_owned(),
                    told_url: url.to_owned(),
                    told_title: String::new(),
                    // Not yet: the load just begun is heard as `started`
                    // (`load_heard`, after this insert).
                    loading: false,
                    told_steps: None,
                    sheet_scrolls: vec![scroll.clone()],
                    scroll,
                    told_y: 0.0,
                    told_at: None,
                    sheet: SHEET_CONFIG.with(|c|c.borrow().get(tab).copied()).map(|(h,f)|SheetPan::new(h,f)),
                },
            );
        });
    }
    Ok(())
}

pub fn place(face: *mut AnyObject, tab: &str, at: PageBox) {
    PAGES.with(|pages| {
        if let Some(page) = pages.borrow().get(tab) {
            unsafe {
                let rect = frame_of(face, at);
                let _: () = msg_send![&*page.view, setFrame: rect];
            }
        }
    });
    deck_follows(tab);
    crate::menu_overlay::ios::raise();
}

pub fn hide(tab: &str, hidden: bool) {
    let mut cancelled = false;
    PAGES.with(|pages| {
        if let Some(page) = pages.borrow_mut().get_mut(tab) {
            if hidden {
                if let Some(s) = page.sheet.as_mut() {
                    cancelled = s.cancel();
                }
            }
            unsafe { let _: () = msg_send![&*page.view, setHidden: hidden]; }
        }
    });
    if cancelled {
        if let Some(app) = APP.get() {
            let _ = app.emit_to("main", "page:sheet", PageWord { tab: tab.to_owned(), text: "cancel,0,0".into() });
        }
    }
    deck_follows(tab);
    crate::menu_overlay::ios::raise();
}

/// Stops watching a page's scroll view and its loading, before the page goes.
fn unwatch(page: &Page) {
    WATCH.with(|w| {
        let key = NSString::from_str("contentOffset");
        let load = NSString::from_str("loading");
        unsafe {
            for view in &page.sheet_scrolls {
                let pan: *mut AnyObject = msg_send![&**view, panGestureRecognizer];
                let _: () = msg_send![pan, removeTarget: &**w, action: objc2::sel!(sheetPan:)];
                let _: () = msg_send![&**view, removeObserver: &**w, forKeyPath: &*key];
            }
            let _: () = msg_send![&*page.view, removeObserver: &**w, forKeyPath: &*load];
        }
    });
}

pub fn close(tab: &str) {
    deck(tab, None);
    crate::page_trust::forget(tab);
    PAGES.with(|pages| {
        if let Some(page) = pages.borrow_mut().remove(tab) {
            unwatch(&page);
            unsafe {
                let _: () = msg_send![&*page.view, removeFromSuperview];
            }
        }
    });
}

/// **What is behind the folded capsule** (UX-29): the page's ground at
/// the point under the capsule's centre, `y` up from the page view's
/// foot, asked of the page itself — the nearest painted background up
/// from the element at that point, else the page's `theme-color` as a
/// hint — and told to the face as `page:ground` (`r,g,b`, a colour the
/// face can read, or empty: nothing known, a white page's ground).
pub fn ground(tab: &str, y: f64) {
    let Some(app) = APP.get().cloned() else { return };
    let view = PAGES.with(|p| p.borrow().get(tab).map(|p| p.view.clone()));
    let Some(view) = view else { return };
    let js = format!(
        "(function(){{var es=document.elementsFromPoint(innerWidth/2,innerHeight-{y:.1});\
         for(var i=0;i<es.length;i++){{for(var e=es[i];e;e=e.parentElement){{\
         var m=/rgba?\\(([\\d.]+),\\s*([\\d.]+),\\s*([\\d.]+)(?:,\\s*([\\d.]+))?\\)/.exec(getComputedStyle(e).backgroundColor);\
         if(m&&(m[4]===undefined||+m[4]>0.5))return m[1]+','+m[2]+','+m[3];}}}}\
         var t=document.querySelector('meta[name=theme-color]');return t&&t.content?t.content:'';}})()"
    );
    let tab = tab.to_owned();
    let done = block2::RcBlock::new(move |result: *mut AnyObject, _err: *mut AnyObject| {
        let text = if result.is_null() {
            String::new()
        } else {
            let s: Option<Retained<NSString>> = unsafe { msg_send![result, description] };
            s.map(|s| s.to_string()).unwrap_or_default()
        };
        let _ = app.emit_to("main", "page:ground", PageWord { tab: tab.clone(), text });
    });
    unsafe {
        let _: () = msg_send![&*view, evaluateJavaScript: &*NSString::from_str(&js), completionHandler: &*done];
    }
}

/// Main-thread snapshot, including an unchanged history whose last poll
/// event happened before the current DOM owner subscribed.
pub fn steps(tab: &str) -> Option<crate::page_history::PageSteps> {
    PAGES.with(|pages| pages.borrow().get(tab).map(|page| {
        // SAFETY: PAGES holds the live WKWebView and all callers run on main.
        unsafe { crate::page_history::webkit(&*page.view as *const AnyObject as *mut AnyObject) }
    }))
}

pub fn step(tab: &str, way: &str) {
    PAGES.with(|pages| {
        if let Some(page) = pages.borrow().get(tab) {
            unsafe {
                let _: Option<Retained<AnyObject>> = match way {
                    "back" => msg_send![&*page.view, goBack],
                    "forward" => msg_send![&*page.view, goForward],
                    // 重试 (UX-22): the address asked for, again — a
                    // failed load leaves the view on the page before it.
                    "retry" => {
                        load(&page.view, &page.url);
                        None
                    }
                    // 「仍然访问」 (UX-42 §14): the certificate passed
                    // for its host for this run, and the address again.
                    "pass" => {
                        if let Some(url) = crate::page_trust::pass(tab) {
                            load(&page.view, &url);
                        }
                        None
                    }
                    // The pill's × while a page loads (UX-33).
                    "stop" => {
                        let _: () = msg_send![&*page.view, stopLoading];
                        None
                    }
                    _ => msg_send![&*page.view, reload],
                };
            }
        }
    });
}

/// Every 400 ms, on the main thread: tell the face what changed.
fn start_polling(app: &tauri::AppHandle) {
    if POLLING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(400));
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || tell(&handle));
        }
    });
}

fn tell(app: &tauri::AppHandle) {
    let mut words: Vec<(&'static str, PageWord)> = Vec::new();
    PAGES.with(|pages| {
        for (tab, page) in pages.borrow_mut().iter_mut() {
            if page.sheet.is_some() {
                watch_sheet_scrolls(tab, page);
            }
            unsafe {
                let loading: bool = msg_send![&*page.view, isLoading];
                if loading != page.loading {
                    page.loading = loading;
                    let word = if loading { "started" } else { "finished" };
                    words.push(("page:load", PageWord { tab: tab.clone(), text: word.into() }));
                }
                let title: Option<Retained<NSString>> = msg_send![&*page.view, title];
                let title = title.map(|t| t.to_string()).unwrap_or_default();
                if title != page.told_title {
                    page.told_title = title.clone();
                    // The borrow's own page for an address the machine
                    // could not reach (UX-22): a failed load, not a title.
                    if title == khor_node::tunnel::PAGE_FAILED_TITLE {
                        words.push(("page:fail", PageWord { tab: tab.clone(), text: format!("route|khor||{}", page.url) }));
                    } else {
                        words.push(("page:title", PageWord { tab: tab.clone(), text: title }));
                    }
                }
                // Whether there is a page behind and ahead, as two digits
                // (`10`: back only) — what the deck's ← → can do.
                let back: bool = msg_send![&*page.view, canGoBack];
                let forward: bool = msg_send![&*page.view, canGoForward];
                if page.told_steps != Some((back, forward)) {
                    page.told_steps = Some((back, forward));
                    let text = format!("{}{}", u8::from(back), u8::from(forward));
                    words.push(("page:steps", PageWord { tab: tab.clone(), text }));
                }
                // Where it is, once a load has ended — as the desktop
                // reports on arrival, never on departure.
                if !loading {
                    let u: Option<Retained<AnyObject>> = msg_send![&*page.view, URL];
                    if let Some(u) = u {
                        let s: Option<Retained<NSString>> = msg_send![&*u, absoluteString];
                        let at = s.map(|s| s.to_string()).unwrap_or_default();
                        // A view a failed load left on `about:blank` has
                        // gone nowhere (UX-22): the tab keeps its address,
                        // and 重试 its way back.
                        if !at.is_empty() && at != "about:blank" && at != page.told_url {
                            page.told_url = at.clone();
                            page.url = at.clone();
                            words.push(("page:went", PageWord { tab: tab.clone(), text: at }));
                        }
                    }
                }
            }
        }
    });
    for (event, word) in words {
        let _ = app.emit_to("main", event, word);
    }
}

/// khor's own webview, for `with_webview` callers.
pub fn face(app: &tauri::AppHandle) -> Option<tauri::Webview> {
    app.get_webview("main")
}

// ── the deck, drawn natively over the page (UX-13, the owner 2026-10-05) ──
//
// The face reads its own DOM deck — laid out where it always is, covered
// by the page — into rectangles: glass shapes, glyphs and faces as PNGs,
// the title and the count as text, and where a finger lands
// (`lib/native-deck.ts`). Drawn here exactly as read, over the page, so
// the page scrolls under the glass; the page's scroll view keeps a
// bottom inset of the deck (or the folded strip) so its last line rests
// clear. One reading per change; `None` puts the deck away (the pill is
// the address editor then, and the DOM deck is the deck).

/// A shape of glass: a capsule, a circle, the strip's band.
#[derive(Debug, serde::Deserialize)]
pub struct DeckGlass {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    r: f64,
    sel: bool,
}

/// A glyph or a face, a PNG at the screen's scale (base64).
#[derive(Debug, serde::Deserialize)]
pub struct DeckImage {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    png: String,
}

/// A line of text in the system font.
#[derive(Debug, serde::Deserialize)]
pub struct DeckLabel {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    text: String,
    size: f64,
    weight: f64,
    color: String,
    center: bool,
}

/// Where a finger lands: the DOM control's index, or the pill's swipe.
#[derive(Debug, serde::Deserialize)]
pub struct DeckHit {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    seat: i64,
    swipe: bool,
}

/// The capsule a progress line is clipped to, with its corner.
#[derive(Debug, serde::Deserialize)]
pub struct DeckClip {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    r: f64,
}

/// The pill's progress line while its tab reloads (UX-35 §2): where it
/// runs, the glass it is clipped to, its ink, and the DOM sweep's period
/// (ms, 0 where it stands still) and `cubic-bezier`.
#[derive(Debug, serde::Deserialize)]
pub struct DeckProgress {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    clip: DeckClip,
    color: String,
    ms: f64,
    ease: [f32; 4],
}

/// One stop of the rim's gradient: a colour at a share of the line.
#[derive(Debug, serde::Deserialize)]
pub struct DeckStop {
    color: String,
    at: f64,
}

/// `--glass-rim`: a one-point ring of a linear gradient at a CSS angle.
#[derive(Debug, serde::Deserialize)]
pub struct DeckRim {
    angle: f64,
    stops: Vec<DeckStop>,
}

/// `--glass-shadow`'s widest layer.
#[derive(Debug, serde::Deserialize)]
pub struct DeckShadow {
    y: f64,
    blur: f64,
    spread: f64,
    color: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckDrawing {
    h: f64,
    inset: f64,
    dark: bool,
    tint: String,
    sel_tint: String,
    rim: DeckRim,
    #[serde(default)]
    shadow: Option<DeckShadow>,
    glass: Vec<DeckGlass>,
    images: Vec<DeckImage>,
    labels: Vec<DeckLabel>,
    hits: Vec<DeckHit>,
    /// Absent when no load runs: the line goes with the reading.
    #[serde(default)]
    progress: Option<DeckProgress>,
}

struct Deck {
    view: Retained<AnyObject>,
    h: f64,
    /// The pill, in the deck's points: where a sideways swipe changes tab.
    pill: Option<(f64, f64, f64, f64)>,
}

thread_local! {
    static DECKS: RefCell<HashMap<String, Deck>> = RefCell::new(HashMap::new());
    /// The tabs a deck was ever drawn for, by the tag their deck carries.
    static DECK_TABS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static DECK_TARGET: Retained<DeckTarget> = unsafe { msg_send![DeckTarget::class(), new] };
    /// When each tab's progress line began running: a reading redraws the
    /// whole deck, and the sweep picks up where it was rather than
    /// starting over at every change of title during a load.
    static RUNS: RefCell<HashMap<String, std::time::Instant>> = RefCell::new(HashMap::new());
}

/// UIKit's `UIEdgeInsets`, for the page's scroll view.
#[repr(C)]
#[derive(Clone, Copy)]
struct Insets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

// SAFETY: four CGFloats (f64 on every iOS this builds for), UIKit's layout.
unsafe impl objc2::Encode for Insets {
    const ENCODING: objc2::Encoding =
        objc2::Encoding::Struct("UIEdgeInsets", &[f64::ENCODING, f64::ENCODING, f64::ENCODING, f64::ENCODING]);
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and this class
    // has no ivars and no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "KhorDeckTarget"]
    struct DeckTarget;

    impl DeckTarget {
        // SAFETY: a UIControl action with its sender.
        #[unsafe(method(hit:))]
        fn hit(&self, sender: *mut AnyObject) {
            let seat: isize = unsafe { msg_send![sender, tag] };
            let deck: *mut AnyObject = unsafe { msg_send![sender, superview] };
            said(deck, &seat.to_string());
        }

        // The recognizer belongs to the title's hit area, so it keeps
        // tracking when an upward swipe leaves the pill's rectangle.
        #[unsafe(method(overview:))]
        fn overview(&self, rec: *mut AnyObject) {
            let title: *mut AnyObject = unsafe { msg_send![rec, view] };
            let deck: *mut AnyObject = unsafe { msg_send![title, superview] };
            said(deck, "overview");
        }

        // SAFETY: a gesture recogniser's action with the recogniser.
        #[unsafe(method(swipe:))]
        fn swipe(&self, rec: *mut AnyObject) {
            let deck: *mut AnyObject = unsafe { msg_send![rec, view] };
            let at: NSPoint = unsafe { msg_send![rec, locationInView: deck] };
            let tag: isize = unsafe { msg_send![deck, tag] };
            let on_pill = DECK_TABS.with(|t| t.borrow().get(tag as usize).cloned()).is_some_and(|tab| {
                DECKS.with(|d| {
                    d.borrow().get(&tab).and_then(|k| k.pill).is_some_and(|(x, y, w, h)| {
                        at.x >= x && at.x <= x + w && at.y >= y && at.y <= y + h
                    })
                })
            });
            if !on_pill {
                return;
            }
            // 1: right (the previous tab), 2: left (the next), Safari's.
            let way: usize = unsafe { msg_send![rec, direction] };
            said(deck, if way == 2 { "swipe:1" } else { "swipe:-1" });
        }
    }
);

/// A press on the deck, said to the face as `page:deck`.
fn said(deck: *mut AnyObject, text: &str) {
    let Some(app) = APP.get() else { return };
    let tag: isize = unsafe { msg_send![deck, tag] };
    let Some(tab) = DECK_TABS.with(|t| t.borrow().get(tag as usize).cloned()) else { return };
    let _ = app.emit_to(
        "main",
        "page:deck",
        PageWord {
            tab,
            text: text.to_owned(),
        },
    );
}

/// `#rrggbb`, `#rgb`, `rgb(…)` or `rgba(…)` as UIColor; ink if unread.
unsafe fn color(text: &str) -> Retained<AnyObject> {
    let t = text.trim();
    let (mut r, mut g, mut b, mut a) = (0.1, 0.1, 0.08, 1.0);
    if let Some(hex) = t.strip_prefix('#') {
        let hex = if hex.len() == 3 { hex.chars().flat_map(|c| [c, c]).collect::<String>() } else { hex.to_owned() };
        if let Ok(v) = u32::from_str_radix(&hex[..6.min(hex.len())], 16) {
            r = f64::from((v >> 16) & 0xff) / 255.0;
            g = f64::from((v >> 8) & 0xff) / 255.0;
            b = f64::from(v & 0xff) / 255.0;
        }
    } else if let Some(inner) = t.strip_prefix("rgba(").or_else(|| t.strip_prefix("rgb(")) {
        let n: Vec<f64> = inner.trim_end_matches(')').split(',').filter_map(|p| p.trim().parse().ok()).collect();
        if n.len() >= 3 {
            (r, g, b) = (n[0] / 255.0, n[1] / 255.0, n[2] / 255.0);
            a = n.get(3).copied().unwrap_or(1.0);
        }
    }
    unsafe { msg_send![class!(UIColor), colorWithRed: r, green: g, blue: b, alpha: a] }
}

unsafe fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w.max(0.0), h.max(0.0)))
}

/// Draws (or, with `None`, puts away) `tab`'s deck over its page.
pub fn deck(tab: &str, drawing: Option<DeckDrawing>) {
    // Whatever was drawn before goes; a reading is the whole deck.
    DECKS.with(|d| {
        if let Some(old) = d.borrow_mut().remove(tab) {
            unsafe {
                let _: () = msg_send![&*old.view, removeFromSuperview];
            }
        }
    });
    let Some(dr) = drawing else {
        RUNS.with(|r| r.borrow_mut().remove(tab));
        inset(tab, 0.0);
        return;
    };
    let tag = DECK_TABS.with(|t| {
        let mut t = t.borrow_mut();
        t.iter().position(|x| x == tab).unwrap_or_else(|| {
            t.push(tab.to_owned());
            t.len() - 1
        })
    });
    let page = PAGES.with(|p| p.borrow().get(tab).map(|p| p.view.clone()));
    let Some(page) = page else { return };
    unsafe {
        let pf: NSRect = msg_send![&*page, frame];
        let a: Allocated<AnyObject> = msg_send![class!(UIView), alloc];
        let view: Retained<AnyObject> = msg_send![a, initWithFrame: rect(pf.origin.x, pf.origin.y + pf.size.height - dr.h, pf.size.width, dr.h)];
        let _: () = msg_send![&*view, setTag: tag as isize];
        // 1 light, 2 dark: the glass and the system font follow khor's
        // theme, not the phone's.
        let _: () = msg_send![&*view, setOverrideUserInterfaceStyle: if dr.dark { 2isize } else { 1isize }];
        let hidden: bool = msg_send![&*page, isHidden];
        let _: () = msg_send![&*view, setHidden: hidden];
        // **The DOM glass, drawn natively** (UX-38 §5, the owner: the two
        // decks did not read as one). The DOM's is a thin grey tint over
        // the ground blurred and saturated (`--glass`, `--glass-blur`), a
        // one-pixel rim of a gradient (`--glass-rim`) and a soft drop
        // shadow (`--glass-shadow`). UIKit's ultra-thin material carries
        // the same blur and saturation but lays a white (dark: black) veil
        // over them — measured side by side over the same bands at 3×:
        // the red under the pill read pink natively, red in the DOM — so
        // its veil is taken off and the token's tint is laid on instead.
        // ⚠️ 墓碑: the material as it comes, and a flat 50% white border.
        let blur: Retained<AnyObject> = msg_send![class!(UIBlurEffect), effectWithStyle: 6isize];
        for g in &dr.glass {
            if let Some(sh) = &dr.shadow {
                view_shadow(&view, g, sh);
            }
            let a: Allocated<AnyObject> = msg_send![class!(UIVisualEffectView), alloc];
            let fx: Retained<AnyObject> = msg_send![a, initWithEffect: &*blur];
            let _: () = msg_send![&*fx, setFrame: rect(g.x, g.y, g.w, g.h)];
            let _: () = msg_send![&*fx, setClipsToBounds: true];
            let layer: *mut AnyObject = msg_send![&*fx, layer];
            let _: () = msg_send![layer, setCornerRadius: g.r];
            let content: *mut AnyObject = msg_send![&*fx, contentView];
            let tint = color(if g.sel { &dr.sel_tint } else { &dr.tint });
            let _: () = msg_send![content, setBackgroundColor: &*tint];
            let _: () = msg_send![&*view, addSubview: &*fx];
            let content_layer: *mut AnyObject = msg_send![content, layer];
            rim_on(content_layer, g, &dr.rim);
        }
        let scale: f64 = {
            let screen: *mut AnyObject = msg_send![class!(UIScreen), mainScreen];
            msg_send![screen, scale]
        };
        for im in &dr.images {
            let text = NSString::from_str(&im.png);
            let a: Allocated<AnyObject> = msg_send![class!(NSData), alloc];
            let data: Option<Retained<AnyObject>> = msg_send![a, initWithBase64EncodedString: &*text, options: 0usize];
            let Some(data) = data else { continue };
            let img: Option<Retained<AnyObject>> = msg_send![class!(UIImage), imageWithData: &*data, scale: scale];
            let Some(img) = img else { continue };
            let a: Allocated<AnyObject> = msg_send![class!(UIImageView), alloc];
            let iv: Retained<AnyObject> = msg_send![a, initWithImage: &*img];
            let _: () = msg_send![&*iv, setFrame: rect(im.x, im.y, im.w, im.h)];
            let _: () = msg_send![&*view, addSubview: &*iv];
        }
        for l in &dr.labels {
            let a: Allocated<AnyObject> = msg_send![class!(UILabel), alloc];
            let lab: Retained<AnyObject> = msg_send![a, initWithFrame: rect(l.x, l.y, l.w, l.h)];
            let text = NSString::from_str(&l.text);
            let _: () = msg_send![&*lab, setText: &*text];
            // CSS weights to UIFontWeight: 400 regular (0), 500 medium
            // (0.23), 600 semibold (0.3), 700 bold (0.4).
            let w = match l.weight as i64 {
                ..=449 => 0.0f64,
                450..=549 => 0.23,
                550..=649 => 0.3,
                _ => 0.4,
            };
            let font: Retained<AnyObject> = msg_send![class!(UIFont), systemFontOfSize: l.size, weight: w];
            let _: () = msg_send![&*lab, setFont: &*font];
            let ink = color(&l.color);
            let _: () = msg_send![&*lab, setTextColor: &*ink];
            // 1 centre, 4 natural; 4 truncating the tail.
            let _: () = msg_send![&*lab, setTextAlignment: if l.center { 1isize } else { 4isize }];
            let _: () = msg_send![&*lab, setLineBreakMode: 4isize];
            let _: () = msg_send![&*view, addSubview: &*lab];
        }
        match &dr.progress {
            Some(p) => run_line(&view, tab, p),
            None => {
                RUNS.with(|r| r.borrow_mut().remove(tab));
            }
        }
        let mut pill = None;
        DECK_TARGET.with(|target| {
            for h in &dr.hits {
                if h.swipe {
                    pill = Some((h.x, h.y, h.w, h.h));
                    continue;
                }
                // 0: UIButtonTypeCustom — a clear press area over what is drawn.
                let b: Retained<AnyObject> = msg_send![class!(UIButton), buttonWithType: 0isize];
                let _: () = msg_send![&*b, setFrame: rect(h.x, h.y, h.w, h.h)];
                let _: () = msg_send![&*b, setTag: h.seat as isize];
                // 64: UIControlEventTouchUpInside.
                let _: () = msg_send![&*b, addTarget: &**target, action: objc2::sel!(hit:), forControlEvents: 64usize];
                if h.seat == 3 {
                    let a: Allocated<AnyObject> = msg_send![class!(UISwipeGestureRecognizer), alloc];
                    let rec: Retained<AnyObject> = msg_send![a, initWithTarget: &**target, action: objc2::sel!(overview:)];
                    // UISwipeGestureRecognizerDirectionUp (1 << 2).
                    let _: () = msg_send![&*rec, setDirection: 4usize];
                    let _: () = msg_send![&*b, addGestureRecognizer: &*rec];
                }
                let _: () = msg_send![&*view, addSubview: &*b];
            }
            if pill.is_some() {
                for way in [1usize, 2usize] {
                    let a: Allocated<AnyObject> = msg_send![class!(UISwipeGestureRecognizer), alloc];
                    let rec: Retained<AnyObject> = msg_send![a, initWithTarget: &**target, action: objc2::sel!(swipe:)];
                    let _: () = msg_send![&*rec, setDirection: way];
                    let _: () = msg_send![&*view, addGestureRecognizer: &*rec];
                }
            }
        });
        let parent: *mut AnyObject = msg_send![&*page, superview];
        if !parent.is_null() {
            let _: () = msg_send![parent, addSubview: &*view];
            crate::menu_overlay::ios::raise();
        }
        // The material builds its layers once it is in a window: its veil
        // is taken off after that (measured: taken before, it stayed).
        let _: () = msg_send![&*view, layoutIfNeeded];
        let subs: Retained<AnyObject> = msg_send![&*view, subviews];
        let n: usize = msg_send![&*subs, count];
        for i in 0..n {
            let sub: *mut AnyObject = msg_send![&*subs, objectAtIndex: i];
            let fx: bool = msg_send![sub, isKindOfClass: class!(UIVisualEffectView)];
            if fx {
                unveil(&*sub);
            }
        }
        DECKS.with(|d| {
            d.borrow_mut().insert(tab.to_owned(), Deck { view, h: dr.h, pill });
        });
    }
    inset(tab, dr.inset);
}

/// A rounded rectangle as a `CGPath` (UIKit's, autoreleased with it).
unsafe fn rounded(x: f64, y: f64, w: f64, h: f64, r: f64) -> *mut std::ffi::c_void {
    unsafe {
        let p: *mut AnyObject = msg_send![class!(UIBezierPath), bezierPathWithRoundedRect: rect(x, y, w, h), cornerRadius: r];
        msg_send![p, CGPath]
    }
}

/// Takes the material's own veil off a glass: the effect view's overlay
/// subviews (`_UIVisualEffectSubview`) hidden, its backdrop — the blur and
/// the saturation — kept. A future UIKit without them is left as it is.
/// ⚠️ Unmeasured: build 2 hid them before the deck was in a window and
/// the veil stayed (the pill over red read 213,124,123, the DOM 236,32,31).
unsafe fn unveil(fx: &AnyObject) {
    unsafe {
        let subs: Retained<AnyObject> = msg_send![fx, subviews];
        let n: usize = msg_send![&*subs, count];
        for i in 0..n {
            let sub: *mut AnyObject = msg_send![&*subs, objectAtIndex: i];
            let name = std::ffi::CStr::from_ptr(objc2::ffi::class_getName(objc2::ffi::object_getClass(sub)));
            if name.to_bytes().windows(19).any(|w| w == b"VisualEffectSubview") {
                let _: () = msg_send![sub, setHidden: true];
            }
        }
    }
}

/// The rim: `--glass-rim`'s gradient on a one-point ring inside the glass,
/// along the CSS angle over the glass's own box.
unsafe fn rim_on(layer: *mut AnyObject, g: &DeckGlass, rim: &DeckRim) {
    if rim.stops.is_empty() {
        return;
    }
    unsafe {
        let grad: Retained<AnyObject> = msg_send![class!(CAGradientLayer), layer];
        let _: () = msg_send![&*grad, setFrame: rect(0.0, 0.0, g.w, g.h)];
        let colors: Retained<AnyObject> = msg_send![class!(NSMutableArray), array];
        let at: Retained<AnyObject> = msg_send![class!(NSMutableArray), array];
        for st in &rim.stops {
            let c = color(&st.color);
            let cg: *mut AnyObject = msg_send![&*c, CGColor];
            let _: () = msg_send![&*colors, addObject: cg];
            let n: Retained<AnyObject> = msg_send![class!(NSNumber), numberWithDouble: st.at];
            let _: () = msg_send![&*at, addObject: &*n];
        }
        let _: () = msg_send![&*grad, setColors: &*colors];
        let _: () = msg_send![&*grad, setLocations: &*at];
        // CSS: 0deg points up, 90deg right; the line is as long as the box
        // is across that way.
        let a = rim.angle.to_radians();
        let (dx, dy) = (a.sin(), -a.cos());
        let len = (g.w * dx).abs() + (g.h * dy).abs();
        let p = |k: f64| NSPoint::new((g.w / 2.0 + k * dx * len / 2.0) / g.w.max(1.0), (g.h / 2.0 + k * dy * len / 2.0) / g.h.max(1.0));
        let _: () = msg_send![&*grad, setStartPoint: p(-1.0)];
        let _: () = msg_send![&*grad, setEndPoint: p(1.0)];
        let ring: Retained<AnyObject> = msg_send![class!(CAShapeLayer), layer];
        let _: () = msg_send![&*ring, setPath: rounded(0.5, 0.5, g.w - 1.0, g.h - 1.0, (g.r - 0.5).max(0.0))];
        let _: () = msg_send![&*ring, setLineWidth: 1.0f64];
        let ink: *mut AnyObject = msg_send![class!(UIColor), blackColor];
        let cg: *mut std::ffi::c_void = msg_send![ink, CGColor];
        let _: () = msg_send![&*ring, setStrokeColor: cg];
        let _: () = msg_send![&*ring, setFillColor: std::ptr::null_mut::<std::ffi::c_void>()];
        let _: () = msg_send![&*grad, setMask: &*ring];
        let _: () = msg_send![layer, addSublayer: &*grad];
    }
}

/// The glass's drop shadow, outside it only, as CSS draws a box's: a
/// shadow of the shape, cut by the shape (the glass is see-through).
unsafe fn view_shadow(deck: &AnyObject, g: &DeckGlass, sh: &DeckShadow) {
    unsafe {
        let pad = sh.blur + sh.y.abs() + sh.spread.abs();
        let a: Allocated<AnyObject> = msg_send![class!(UIView), alloc];
        let v: Retained<AnyObject> = msg_send![a, initWithFrame: rect(g.x - pad, g.y - pad, g.w + 2.0 * pad, g.h + 2.0 * pad)];
        let _: () = msg_send![&*v, setUserInteractionEnabled: false];
        let shape: Retained<AnyObject> = msg_send![class!(CALayer), layer];
        let _: () = msg_send![&*shape, setFrame: rect(0.0, 0.0, g.w + 2.0 * pad, g.h + 2.0 * pad)];
        let s = sh.spread;
        let _: () = msg_send![&*shape, setShadowPath: rounded(pad - s, pad - s, g.w + 2.0 * s, g.h + 2.0 * s, (g.r + s).max(0.0))];
        let c = color(&sh.color);
        let cg: *mut std::ffi::c_void = msg_send![&*c, CGColor];
        let _: () = msg_send![&*shape, setShadowColor: cg];
        let _: () = msg_send![&*shape, setShadowOpacity: 1.0f32];
        // CSS blur is twice UIKit's radius.
        let _: () = msg_send![&*shape, setShadowRadius: sh.blur / 2.0];
        let _: () = msg_send![&*shape, setShadowOffset: NSSize::new(0.0, sh.y)];
        // Cut out the glass itself: everything but the shape shows.
        let cut: Retained<AnyObject> = msg_send![class!(CAShapeLayer), layer];
        let outer: *mut AnyObject = msg_send![class!(UIBezierPath), bezierPathWithRect: rect(0.0, 0.0, g.w + 2.0 * pad, g.h + 2.0 * pad)];
        let inner: *mut AnyObject = msg_send![class!(UIBezierPath), bezierPathWithRoundedRect: rect(pad, pad, g.w, g.h), cornerRadius: g.r];
        let _: () = msg_send![outer, appendPath: inner];
        let path: *mut std::ffi::c_void = msg_send![outer, CGPath];
        let _: () = msg_send![&*cut, setPath: path];
        let even = NSString::from_str("even-odd");
        let _: () = msg_send![&*cut, setFillRule: &*even];
        let vl: *mut AnyObject = msg_send![&*v, layer];
        let _: () = msg_send![vl, addSublayer: &*shape];
        let _: () = msg_send![vl, setMask: &*cut];
        let _: () = msg_send![deck, addSubview: &*v];
    }
}

/// **The progress line under the pill** (UX-35 §2; UX-33 §2 in the DOM):
/// a 2-pt run along the capsule's bottom edge, clipped to its corners,
/// swept from the left over and over in the DOM's period and curve —
/// Safari's — until a reading comes without it. The DOM pill is the
/// source of truth; this draws what it read.
unsafe fn run_line(deck: &AnyObject, tab: &str, p: &DeckProgress) {
    let since = RUNS.with(|r| *r.borrow_mut().entry(tab.to_owned()).or_insert_with(std::time::Instant::now));
    unsafe {
        let a: Allocated<AnyObject> = msg_send![class!(UIView), alloc];
        let clip: Retained<AnyObject> = msg_send![a, initWithFrame: rect(p.clip.x, p.clip.y, p.clip.w, p.clip.h)];
        let _: () = msg_send![&*clip, setClipsToBounds: true];
        let _: () = msg_send![&*clip, setUserInteractionEnabled: false];
        let layer: *mut AnyObject = msg_send![&*clip, layer];
        let _: () = msg_send![layer, setCornerRadius: p.clip.r];
        let a: Allocated<AnyObject> = msg_send![class!(UIView), alloc];
        let line: Retained<AnyObject> = msg_send![a, initWithFrame: rect(0.0, 0.0, 0.0, 0.0)];
        let ink = color(&p.color);
        let _: () = msg_send![&*line, setBackgroundColor: &*ink];
        let layer: *mut AnyObject = msg_send![&*line, layer];
        // Grown from its left end: the anchor first, then the frame, so
        // the frame lands where it was read.
        let _: () = msg_send![layer, setAnchorPoint: NSPoint::new(0.0, 0.5)];
        let _: () = msg_send![&*line, setFrame: rect(p.x - p.clip.x, p.y - p.clip.y, p.w, p.h)];
        if p.ms > 0.0 {
            let key = NSString::from_str("transform.scale.x");
            let anim: Retained<AnyObject> = msg_send![class!(CABasicAnimation), animationWithKeyPath: &*key];
            let from: Retained<AnyObject> = msg_send![class!(NSNumber), numberWithDouble: 0.0f64];
            let to: Retained<AnyObject> = msg_send![class!(NSNumber), numberWithDouble: 1.0f64];
            let _: () = msg_send![&*anim, setFromValue: &*from];
            let _: () = msg_send![&*anim, setToValue: &*to];
            let period = p.ms / 1000.0;
            let _: () = msg_send![&*anim, setDuration: period];
            let _: () = msg_send![&*anim, setRepeatCount: f32::INFINITY];
            let _: () = msg_send![&*anim, setRemovedOnCompletion: false];
            let _: () = msg_send![&*anim, setTimeOffset: since.elapsed().as_secs_f64() % period];
            // `functionWithControlPoints::::` has unnamed parts, which
            // `msg_send!` cannot spell.
            let [c1x, c1y, c2x, c2y] = p.ease;
            let curve: *mut AnyObject = objc2::runtime::MessageReceiver::send_message(
                class!(CAMediaTimingFunction),
                objc2::sel!(functionWithControlPoints::::),
                (c1x, c1y, c2x, c2y),
            );
            if !curve.is_null() {
                let _: () = msg_send![&*anim, setTimingFunction: curve];
            }
            let name = NSString::from_str("progress");
            let _: () = msg_send![layer, addAnimation: &*anim, forKey: &*name];
        }
        let _: () = msg_send![&*clip, addSubview: &*line];
        let _: () = msg_send![deck, addSubview: &*clip];
    }
}

/// The page's scroll view keeps `bottom` points clear at its end: the
/// deck's height, the strip's when folded, 0 with no native deck.
fn inset(tab: &str, bottom: f64) {
    PAGES.with(|p| {
        if let Some(page) = p.borrow().get(tab) {
            let e = Insets { top: 0.0, left: 0.0, bottom, right: 0.0 };
            unsafe {
                let _: () = msg_send![&*page.scroll, setContentInset: e];
                let _: () = msg_send![&*page.scroll, setVerticalScrollIndicatorInsets: e];
            }
        }
    });
}

/// The deck follows its page: moved with it, hidden with it, gone with it.
fn deck_follows(tab: &str) {
    let frame = PAGES.with(|p| p.borrow().get(tab).map(|p| unsafe { msg_send![&*p.view, frame] }));
    let hidden = PAGES.with(|p| p.borrow().get(tab).map(|p| unsafe { msg_send![&*p.view, isHidden] }));
    DECKS.with(|d| {
        let d = d.borrow();
        let Some(k) = d.get(tab) else { return };
        unsafe {
            if let Some(pf) = frame {
                let pf: NSRect = pf;
                let _: () = msg_send![&*k.view, setFrame: rect(pf.origin.x, pf.origin.y + pf.size.height - k.h, pf.size.width, k.h)];
            }
            if let Some(h) = hidden {
                let h: bool = h;
                let _: () = msg_send![&*k.view, setHidden: h];
            }
        }
    });
}
