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
//! three events. A link that asks for a new window opens in place: a
//! `WKUIDelegate` would be a class of khor's own, and the desktop's answer
//! (a new tab) is the face's to mint; on the phone the page follows it.
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
    /// Where it was last told to be, and when.
    told_y: f64,
    told_at: Option<std::time::Instant>,
}

thread_local! {
    static PAGES: RefCell<HashMap<String, Page>> = RefCell::new(HashMap::new());
    /// The one observer every page's scroll view reports to.
    static WATCH: Retained<ScrollWatch> = unsafe { msg_send![ScrollWatch::class(), new] };
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
        // SAFETY: the signature is NSKeyValueObserving's.
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn observe_value(
            &self,
            _key: *mut AnyObject,
            object: *mut AnyObject,
            _change: *mut AnyObject,
            _context: *mut std::ffi::c_void,
        ) {
            scrolled(object);
        }
    }
);

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
        let Ok(mut pages) = pages.try_borrow_mut() else { return };
        for (tab, page) in pages.iter_mut() {
            if Retained::as_ptr(&page.scroll) as *mut AnyObject != view {
                continue;
            }
            let at: NSPoint = unsafe { msg_send![&*page.scroll, contentOffset] };
            let dy = at.y - page.told_y;
            if dy.abs() < 1.0 {
                return;
            }
            let now = std::time::Instant::now();
            if page.told_at.is_some_and(|t| now.duration_since(t).as_millis() < 16) {
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

/// A link that asks for a new window navigates this page instead.
const SAME_TAB: &str = "document.addEventListener('click', function (e) { \
var a = e.target && e.target.closest ? e.target.closest('a[target]') : null; \
if (a && a.target !== '_self') a.target = '_self'; }, true); \
window.open = function (u) { if (u) location.href = u; return null; };";

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
                    let _: () = msg_send![&*page.view, setHidden: false];
                    return true;
                }
                unwatch(page);
                let _: () = msg_send![&*page.view, removeFromSuperview];
                pages.remove(tab);
            }
            false
        });
        if reused {
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
        add_script(controller, SAME_TAB);
        let a: Allocated<AnyObject> = msg_send![class!(WKWebView), alloc];
        let view: Retained<AnyObject> = msg_send![a, initWithFrame: rect, configuration: &*cfg];
        if let Some(agent) = &wiring.agent {
            let ua = NSString::from_str(agent);
            let _: () = msg_send![&*view, setCustomUserAgent: &*ua];
        }
        // The page's own back and forward, as Safari's edge swipes.
        let _: () = msg_send![&*view, setAllowsBackForwardNavigationGestures: true];
        let parent: *mut AnyObject = msg_send![face, superview];
        let _: () = msg_send![parent, addSubview: &*view];
        load(&view, url);
        let scroll: Retained<AnyObject> = msg_send![&*view, scrollView];
        WATCH.with(|w| {
            let key = NSString::from_str("contentOffset");
            // 1: NSKeyValueObservingOptionNew.
            let _: () = msg_send![&*scroll, addObserver: &**w, forKeyPath: &*key, options: 1usize, context: std::ptr::null_mut::<std::ffi::c_void>()];
        });
        PAGES.with(|pages| {
            pages.borrow_mut().insert(
                tab.to_owned(),
                Page {
                    view,
                    exit: exit.to_owned(),
                    url: url.to_owned(),
                    told_url: url.to_owned(),
                    told_title: String::new(),
                    loading: true,
                    told_steps: None,
                    scroll,
                    told_y: 0.0,
                    told_at: None,
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
}

pub fn hide(tab: &str, hidden: bool) {
    PAGES.with(|pages| {
        if let Some(page) = pages.borrow().get(tab) {
            unsafe {
                let _: () = msg_send![&*page.view, setHidden: hidden];
            }
        }
    });
}

/// Stops watching a page's scroll view, before the page goes.
fn unwatch(page: &Page) {
    WATCH.with(|w| {
        let key = NSString::from_str("contentOffset");
        unsafe {
            let _: () = msg_send![&*page.scroll, removeObserver: &**w, forKeyPath: &*key];
        }
    });
}

pub fn close(tab: &str) {
    PAGES.with(|pages| {
        if let Some(page) = pages.borrow_mut().remove(tab) {
            unwatch(&page);
            unsafe {
                let _: () = msg_send![&*page.view, removeFromSuperview];
            }
        }
    });
}

pub fn step(tab: &str, way: &str) {
    PAGES.with(|pages| {
        if let Some(page) = pages.borrow().get(tab) {
            unsafe {
                let _: Option<Retained<AnyObject>> = match way {
                    "back" => msg_send![&*page.view, goBack],
                    "forward" => msg_send![&*page.view, goForward],
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
                    words.push(("page:title", PageWord { tab: tab.clone(), text: title }));
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
                        if !at.is_empty() && at != page.told_url {
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
