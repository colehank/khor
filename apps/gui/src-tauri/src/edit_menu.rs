//! **khor's own items in the system's edit menu** (item 54, iOS).
//!
//! A long press on the terminal's text layer (`lib/term-select`) gets
//! iOS's own selection and its edit menu: 拷贝 · 查询 · 翻译 · 搜索网页 ·
//! 共享. Page code cannot add to that menu. Since iOS 16 the menu is a
//! `UIEditMenuInteraction`, and its suggested actions are gathered from
//! the responder chain's `buildMenu(with:)` with `builder.system ==
//! .context` (WWDC22, "Adopt desktop-class editing interactions"). So the
//! web view — on the chain above its content view — is moved to a subclass
//! of its own class whose `buildMenuWithBuilder:` calls the original, then
//! adds what the page last offered (`edit_menu`), inline, after the
//! standard edit items. A chosen item calls the page back
//! (`window.__khorEditMenu`), with the pasteboard's text for 粘贴: a page
//! that reads the clipboard outside a gesture gets iOS's own paste prompt.
//!
//! Blink Shell does the same from native code over its web view
//! (`Blink/Terminal/TermView.m`: Copy, Paste, Open ‹url›, Search, Share).
//!
//! **A future UIKit degrades to the system's menu, never to a crash**: a
//! subclass that cannot be made leaves the web view as it was, and an
//! unknown menu identifier leaves the menu as the system built it. One
//! object, not the class: nothing else in the process changes. Main thread
//! only (`with_webview`, `buildMenuWithBuilder:` and the actions run
//! there).
use std::sync::Mutex;

use block2::RcBlock;
use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, Sel};
use objc2::{class, msg_send, sel};
use objc2_foundation::NSString;

/// What the page offered for the next menu: `(id, title)` pairs.
static OFFERED: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

const WITH_MENU: &std::ffi::CStr = c"KhorWebViewWithEditMenu";

#[link(name = "UIKit", kind = "framework")]
unsafe extern "C" {
    static UIMenuStandardEdit: &'static AnyObject;
    static UIMenuRoot: &'static AnyObject;
}

/// The items the next menu over a selection adds (`edit_menu`).
pub fn offer(items: Vec<(String, String)>) {
    if let Ok(mut offered) = OFFERED.lock() {
        *offered = items;
    }
}

/// Moves the web view to the subclass that adds khor's items.
pub fn install(webview: *mut std::ffi::c_void) {
    if webview.is_null() {
        return;
    }
    // SAFETY: `webview` is the live WKWebView `with_webview` handed over,
    // on the main thread. The subclass adds one method and no ivars, so
    // the object's size and layout are its class's (`set_class`).
    let swapped = std::panic::catch_unwind(|| unsafe {
        let web = &*(webview as *mut AnyObject);
        let Some(with) = subclass_of(web.class()) else {
            return false;
        };
        AnyObject::set_class(web, with);
        true
    });
    if !matches!(swapped, Ok(true)) {
        eprintln!("the edit menu stays the system's: no subclass of the web view could be made");
    }
}

/// The subclass of `of` with khor's items: made once, and only ever
/// answered for the class it was made from.
fn subclass_of(of: &AnyClass) -> Option<&'static AnyClass> {
    if let Some(made) = AnyClass::get(WITH_MENU) {
        return (made.superclass() == Some(of)).then_some(made);
    }
    let mut b = ClassBuilder::new(WITH_MENU, of)?;
    // SAFETY: `buildMenuWithBuilder:` takes one object and returns void
    // (`v24@0:8@16`), which is this signature.
    unsafe { b.add_method(sel!(buildMenuWithBuilder:), build_menu as extern "C-unwind" fn(_, _, _)) };
    Some(b.register())
}

extern "C-unwind" fn build_menu(this: &AnyObject, _cmd: Sel, builder: *mut AnyObject) {
    // SAFETY: UIKit calls this on the main thread with a live builder;
    // every selector is documented UIKit (`UIMenuBuilder`, `UIMenuSystem`,
    // `UIAction`, `UIMenu`). The superclass is the web view's own class,
    // whose implementation runs first.
    unsafe {
        let Some(base) = AnyClass::get(WITH_MENU).and_then(|c| c.superclass()) else {
            return;
        };
        let _: () = msg_send![super(this, base), buildMenuWithBuilder: builder];
        if builder.is_null() {
            return;
        }
        let system: *mut AnyObject = msg_send![&*builder, system];
        let context: *mut AnyObject = msg_send![class!(UIMenuSystem), contextSystem];
        if system != context {
            return;
        }
        let offered = OFFERED.lock().map(|o| o.clone()).unwrap_or_default();
        if offered.is_empty() {
            return;
        }
        let web = this as *const AnyObject as usize;
        let children: *mut AnyObject = msg_send![class!(NSMutableArray), array];
        let none: *mut AnyObject = std::ptr::null_mut();
        for (id, title) in offered {
            let handler = RcBlock::new(move |_action: *mut AnyObject| pick(web, &id));
            let title = NSString::from_str(&title);
            let action: *mut AnyObject = msg_send![
                class!(UIAction),
                actionWithTitle: &*title,
                image: none,
                identifier: none,
                handler: &*handler
            ];
            if !action.is_null() {
                let _: () = msg_send![children, addObject: action];
            }
        }
        let empty = NSString::from_str("");
        // UIMenuOptionsDisplayInline: a group in the menu, not a submenu.
        let menu: *mut AnyObject = msg_send![
            class!(UIMenu),
            menuWithTitle: &*empty,
            image: none,
            identifier: none,
            options: 1usize,
            children: children
        ];
        if menu.is_null() {
            return;
        }
        let edit: *mut AnyObject = msg_send![&*builder, menuForIdentifier: UIMenuStandardEdit];
        if edit.is_null() {
            let _: () = msg_send![&*builder, insertChildMenu: menu, atStartOfMenuForIdentifier: UIMenuRoot];
        } else {
            let _: () = msg_send![&*builder, insertSiblingMenu: menu, afterMenuForIdentifier: UIMenuStandardEdit];
        }
    }
}

// ── the menu at a selection khor drew (item 73) ──
//
// With the soft keyboard up the terminal selects in xterm's own cells and
// keeps the focus (`lib/term-select`), so no text selection is there for
// iOS to hang its menu on. The same system menu is shown at the selection
// by a `UIEditMenuInteraction` of khor's own on the web view's scroll
// view (iOS 16): its delegate answers only khor's items, never the
// suggested ones (they are the focused field's: 粘贴 into the hidden
// field, 全选). Presenting it moves no first responder: the keyboard stays.

const AT_DELEGATE: &std::ffi::CStr = c"KhorEditMenuAtDelegate";

/// What the menu at a point carries: the items, the text its 拷贝 writes,
/// and the selection's box in the scroll view's points.
struct At {
    items: Vec<(String, String)>,
    copy: Option<String>,
    rect: [f64; 4],
    web: usize,
}

static AT: Mutex<Option<At>> = Mutex::new(None);

/// The interaction, made once and kept (`Retained::into_raw`): the
/// delegate it holds weakly is kept the same way.
static AT_INTERACTION: Mutex<usize> = Mutex::new(0);

/// Shows the system's edit menu over `rect` (the page's CSS px, from the
/// document's origin) with `items`; `copy` is what 拷贝 writes.
pub fn show_at(webview: *mut std::ffi::c_void, rect: [f64; 4], items: Vec<(String, String)>, copy: Option<String>) {
    if webview.is_null() {
        return;
    }
    let shown = std::panic::catch_unwind(|| unsafe {
        // SAFETY: the live WKWebView `with_webview` handed over, on the
        // main thread; every selector is documented UIKit
        // (`UIEditMenuInteraction`, `UIEditMenuConfiguration`,
        // `UIScrollView`).
        let web = &*(webview as *mut AnyObject);
        let scroll: *mut AnyObject = msg_send![web, scrollView];
        let Some(scroll) = scroll.as_ref() else { return false };
        let zoom: f64 = msg_send![scroll, zoomScale];
        let [x, y, w, h] = rect;
        let rect = [x * zoom, y * zoom, w * zoom, h * zoom];
        if let Ok(mut at) = AT.lock() {
            *at = Some(At { items, copy, rect, web: webview as usize });
        }
        let Some(interaction) = interaction_on(scroll) else { return false };
        let none: *mut AnyObject = std::ptr::null_mut();
        let point = objc2_foundation::NSPoint::new(rect[0] + rect[2] / 2.0, rect[1]);
        let config: *mut AnyObject = msg_send![class!(UIEditMenuConfiguration), configurationWithIdentifier: none, sourcePoint: point];
        if config.is_null() {
            return false;
        }
        let _: () = msg_send![interaction, presentEditMenuWithConfiguration: config];
        true
    });
    if !matches!(shown, Ok(true)) {
        eprintln!("no edit menu at the terminal's selection: UIEditMenuInteraction is not there");
    }
}

/// Puts the menu at a point away, if it is up.
pub fn hide_at() {
    let held = AT_INTERACTION.lock().map(|i| *i).unwrap_or(0);
    if held == 0 {
        return;
    }
    // SAFETY: main thread; `held` is the interaction `interaction_on`
    // made and keeps for the process's life.
    let _ = std::panic::catch_unwind(|| unsafe {
        let _: () = msg_send![held as *mut AnyObject, dismissMenu];
    });
}

/// The interaction on `scroll`, made and added the first time.
///
/// # Safety
/// Main thread; `scroll` is the web view's live scroll view.
unsafe fn interaction_on(scroll: &AnyObject) -> Option<*mut AnyObject> {
    let mut held = AT_INTERACTION.lock().ok()?;
    if *held != 0 {
        return Some(*held as *mut AnyObject);
    }
    let class = match AnyClass::get(AT_DELEGATE) {
        Some(c) => c,
        None => {
            let mut b = ClassBuilder::new(AT_DELEGATE, class!(NSObject))?;
            if let Some(p) = objc2::runtime::AnyProtocol::get(c"UIEditMenuInteractionDelegate") {
                b.add_protocol(p);
            }
            // SAFETY: `editMenuInteraction:menuForConfiguration:suggestedActions:`
            // takes three objects and returns one; `…targetRectForConfiguration:`
            // takes two and returns a CGRect. These are those signatures.
            unsafe {
                b.add_method(
                    sel!(editMenuInteraction:menuForConfiguration:suggestedActions:),
                    menu_at as extern "C-unwind" fn(_, _, _, _, _) -> _,
                );
                b.add_method(
                    sel!(editMenuInteraction:targetRectForConfiguration:),
                    target_at as extern "C-unwind" fn(_, _, _, _) -> _,
                );
            }
            b.register()
        }
    };
    // SAFETY: documented UIKit; the delegate and the interaction are kept
    // for the process's life (`into_raw` of a +1 object: never released).
    unsafe {
        let delegate: *mut AnyObject = msg_send![class, new];
        if delegate.is_null() {
            return None;
        }
        let alloc: *mut AnyObject = msg_send![class!(UIEditMenuInteraction), alloc];
        let interaction: *mut AnyObject = msg_send![alloc, initWithDelegate: delegate];
        if interaction.is_null() {
            return None;
        }
        let _: () = msg_send![scroll, addInteraction: interaction];
        *held = interaction as usize;
        Some(interaction)
    }
}

extern "C-unwind" fn menu_at(
    _this: &AnyObject,
    _cmd: Sel,
    _interaction: *mut AnyObject,
    _config: *mut AnyObject,
    _suggested: *mut AnyObject,
) -> *mut AnyObject {
    let (items, web) = match AT.lock().ok().as_deref() {
        Some(Some(at)) => (at.items.clone(), at.web),
        _ => return std::ptr::null_mut(),
    };
    // SAFETY: UIKit calls this on the main thread; documented UIKit only.
    unsafe {
        let children: *mut AnyObject = msg_send![class!(NSMutableArray), array];
        let none: *mut AnyObject = std::ptr::null_mut();
        for (id, title) in items {
            let handler = RcBlock::new(move |_action: *mut AnyObject| {
                if id == "copy" {
                    copy_at();
                }
                pick(web, &id)
            });
            let title = NSString::from_str(&title);
            let action: *mut AnyObject = msg_send![
                class!(UIAction),
                actionWithTitle: &*title,
                image: none,
                identifier: none,
                handler: &*handler
            ];
            if !action.is_null() {
                let _: () = msg_send![children, addObject: action];
            }
        }
        msg_send![class!(UIMenu), menuWithChildren: children]
    }
}

extern "C-unwind" fn target_at(_this: &AnyObject, _cmd: Sel, _interaction: *mut AnyObject, _config: *mut AnyObject) -> objc2_foundation::NSRect {
    let [x, y, w, h] = match AT.lock().ok().as_deref() {
        Some(Some(at)) => at.rect,
        _ => [0.0; 4],
    };
    objc2_foundation::NSRect::new(objc2_foundation::NSPoint::new(x, y), objc2_foundation::NSSize::new(w, h))
}

/// 拷贝 at khor's selection: the cells' text the page handed over, onto
/// the pasteboard natively (a page's clipboard write outside a gesture
/// is refused).
fn copy_at() {
    let Some(text) = AT.lock().ok().and_then(|at| at.as_ref().and_then(|a| a.copy.clone())) else {
        return;
    };
    // SAFETY: main thread (a menu action); documented UIKit.
    unsafe {
        let board: *mut AnyObject = msg_send![class!(UIPasteboard), generalPasteboard];
        if let Some(board) = board.as_ref() {
            let text = NSString::from_str(&text);
            let _: () = msg_send![board, setString: &*text];
        }
    }
}

/// The chosen item, told to the page: `window.__khorEditMenu(id, text,
/// image)`, with the pasteboard's text for 粘贴, else its picture.
fn pick(web: usize, id: &str) {
    // SAFETY: on the main thread (a menu action); `web` is the web view
    // the menu was built over, alive while its menu is on screen.
    unsafe {
        let board: *mut AnyObject = if id == "paste" {
            msg_send![class!(UIPasteboard), generalPasteboard]
        } else {
            std::ptr::null_mut()
        };
        let text = board.as_ref().and_then(|b| {
            let s: *mut NSString = msg_send![b, string];
            s.as_ref().map(|s| s.to_string())
        });
        // No text: the first picture it holds, as its own bytes (item 60
        // part 4; the page drops it). Text wins, as in iTerm2.
        let image = match (&text, board.as_ref()) {
            (None, Some(b)) => pasted_image(b),
            _ => None,
        };
        let js = format!(
            "window.__khorEditMenu && window.__khorEditMenu({}, {}, {})",
            serde_json::to_string(id).unwrap_or_default(),
            serde_json::to_string(&text).unwrap_or_else(|_| "null".into()),
            serde_json::to_string(&image).unwrap_or_else(|_| "null".into()),
        );
        let js = NSString::from_str(&js);
        let done: Option<&block2::Block<dyn Fn(*mut AnyObject, *mut AnyObject)>> = None;
        let _: () = msg_send![web as *mut AnyObject, evaluateJavaScript: &*js, completionHandler: done];
    }
}

/// The picture types 粘贴 takes off the pasteboard, as uniform type
/// identifiers, with the type the page knows each by.
const PICTURES: [(&str, &str); 4] = [
    ("public.png", "image/png"),
    ("public.jpeg", "image/jpeg"),
    ("public.heic", "image/heic"),
    ("com.compuserve.gif", "image/gif"),
];

/// The pasteboard's first picture as `{type, data}`, its bytes in base64
/// (`NSData`'s own encoder), for `lib/term-paste`'s `pastedImage`.
///
/// # Safety
/// Main thread; `board` is the general pasteboard.
unsafe fn pasted_image(board: &AnyObject) -> Option<serde_json::Value> {
    for (uti, mime) in PICTURES {
        let uti = NSString::from_str(uti);
        let data: *mut AnyObject = unsafe { msg_send![board, dataForPasteboardType: &*uti] };
        let Some(data) = (unsafe { data.as_ref() }) else {
            continue;
        };
        let b64: *mut NSString = unsafe { msg_send![data, base64EncodedStringWithOptions: 0usize] };
        let b64 = unsafe { b64.as_ref() }?.to_string();
        return Some(serde_json::json!({ "type": mime, "data": b64 }));
    }
    None
}
