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
//! A load that fails is heard by a navigation delegate of khor's own
//! (`PageFail`, UX-22): it implements only the two failure methods, so
//! every other decision is WebKit's default, as it was with none.
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
    /// Every page's navigation delegate (WebKit holds it weakly).
    static FAIL: Retained<PageFail> = unsafe { msg_send![PageFail::class(), new] };
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
    }
);

/// **A load that failed says why** (UX-22): `page:fail` with the error
/// (`page_fail::word`), and the light out at once rather than at the next
/// poll.
fn failed(view: *mut AnyObject, error: *mut AnyObject) {
    let Some(app) = APP.get() else { return };
    // SAFETY: WebKit hands a live NSError.
    let Some(text) = (unsafe { crate::page_fail::word(error) }) else { return };
    let tab = PAGES.with(|pages| {
        let mut pages = pages.try_borrow_mut().ok()?;
        let (tab, page) = pages.iter_mut().find(|(_, p)| Retained::as_ptr(&p.view) as *mut AnyObject == view)?;
        page.loading = false;
        Some(tab.clone())
    });
    let Some(tab) = tab else { return };
    let _ = app.emit_to("main", "page:load", PageWord { tab: tab.clone(), text: "finished".into() });
    let _ = app.emit_to("main", "page:fail", PageWord { tab, text });
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
        FAIL.with(|f| {
            let _: () = msg_send![&*view, setNavigationDelegate: &**f];
        });
        // Attached hidden when the face stopped wanting it during the
        // dial (`page_wanted`); `show_page` brings it back.
        let _: () = msg_send![&*view, setHidden: !crate::page_is_wanted(tab)];
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
    deck_follows(tab);
}

pub fn hide(tab: &str, hidden: bool) {
    PAGES.with(|pages| {
        if let Some(page) = pages.borrow().get(tab) {
            unsafe {
                let _: () = msg_send![&*page.view, setHidden: hidden];
            }
        }
    });
    deck_follows(tab);
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
    deck(tab, None);
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

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckDrawing {
    h: f64,
    inset: f64,
    dark: bool,
    tint: String,
    sel_tint: String,
    rim: String,
    glass: Vec<DeckGlass>,
    images: Vec<DeckImage>,
    labels: Vec<DeckLabel>,
    hits: Vec<DeckHit>,
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
        // 6: UIBlurEffectStyleSystemUltraThinMaterial — the DOM glass's
        // blur and saturation; the tint over it is `--glass`.
        let blur: Retained<AnyObject> = msg_send![class!(UIBlurEffect), effectWithStyle: 6isize];
        for g in &dr.glass {
            let a: Allocated<AnyObject> = msg_send![class!(UIVisualEffectView), alloc];
            let fx: Retained<AnyObject> = msg_send![a, initWithEffect: &*blur];
            let _: () = msg_send![&*fx, setFrame: rect(g.x, g.y, g.w, g.h)];
            let _: () = msg_send![&*fx, setClipsToBounds: true];
            let layer: *mut AnyObject = msg_send![&*fx, layer];
            let _: () = msg_send![layer, setCornerRadius: g.r];
            let _: () = msg_send![layer, setBorderWidth: 1.0f64];
            let rim = color(&dr.rim);
            let cg: *mut std::ffi::c_void = msg_send![&*rim, CGColor];
            let _: () = msg_send![layer, setBorderColor: cg];
            let content: *mut AnyObject = msg_send![&*fx, contentView];
            let tint = color(if g.sel { &dr.sel_tint } else { &dr.tint });
            let _: () = msg_send![content, setBackgroundColor: &*tint];
            let _: () = msg_send![&*view, addSubview: &*fx];
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
        }
        DECKS.with(|d| {
            d.borrow_mut().insert(tab.to_owned(), Deck { view, h: dr.h, pill });
        });
    }
    inset(tab, dr.inset);
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
