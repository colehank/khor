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
