//! A trusted completion document above sibling WKWebViews, never inside a site.
//!
//! UI and filtering remain the shared OmniMenu. Only its inert, resolved DOM
//! crosses this boundary; the new configuration has no Tauri IPC, proxy, page
//! shim or app scheme handler. WebKit can send only height and candidate keys.
//! All UIKit/WebKit operations in this module run on the main thread.
use super::MenuRequest;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, NSObject, Sel};
use objc2::{ClassType, class, define_class, msg_send};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_ui_kit::UIView;
use std::cell::RefCell;
use tauri::Emitter;

struct Panel {
    owner: String,
    request: MenuRequest,
    backdrop: Retained<AnyObject>,
    view: Retained<AnyObject>,
    material: Retained<AnyObject>,
    outside: Retained<AnyObject>,
    radius: f64,
    origin: NSPoint,
    loaded: bool,
}
thread_local! {
    static PANEL: RefCell<Option<Panel>> = const { RefCell::new(None) };
    static TARGET: Retained<MenuTarget> = unsafe { msg_send![MenuTarget::class(), new] };
}
static APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

/// A candidate list is never an editor. Refuse native first-responder
/// ownership before WebKit dispatches any DOM mouse/touch event, so tapping a
/// row cannot dismiss the address field's keyboard in that earlier phase.
fn menu_webview_class() -> Result<&'static AnyClass, String> {
    static CLASS: std::sync::OnceLock<Option<&'static AnyClass>> = std::sync::OnceLock::new();
    extern "C" fn cannot_focus(_view: *mut AnyObject, _selector: Sel) -> Bool {
        Bool::NO
    }
    CLASS
        .get_or_init(|| {
            let mut builder = ClassBuilder::new(c"KhorCompletionWebView", class!(WKWebView))?;
            // SAFETY: UIResponder's selector has no arguments and returns BOOL.
            // No other WebView class or instance is changed.
            unsafe {
                builder.add_method(
                    objc2::sel!(canBecomeFirstResponder),
                    cannot_focus as extern "C" fn(*mut AnyObject, Sel) -> Bool,
                );
                // WKWebView's own becomeFirstResponder implementation can
                // succeed even after canBecomeFirstResponder returned false
                // (verified on the native iOS simulator). Refuse that path too.
                builder.add_method(
                    objc2::sel!(becomeFirstResponder),
                    cannot_focus as extern "C" fn(*mut AnyObject, Sel) -> Bool,
                );
            }
            Some(builder.register())
        })
        .as_ref()
        .copied()
        .ok_or_else(|| "the completion WebView class could not be registered".into())
}

define_class!(
    // SAFETY: UIView's hit test keeps its native subtree intact. Only a hit
    // on the otherwise empty host itself passes through to the webpage.
    #[unsafe(super(UIView))]
    #[name = "KhorMenuBackdrop"]
    struct MenuBackdrop;
    impl MenuBackdrop {
        #[unsafe(method(hitTest:withEvent:))]
        fn hit_test(&self, point: NSPoint, event: *mut AnyObject) -> *mut AnyObject {
            let hit: *mut AnyObject = unsafe { msg_send![super(self), hitTest: point, withEvent: event] };
            if std::ptr::eq(hit, self as *const Self as *const AnyObject) {
                std::ptr::null_mut()
            } else {
                hit
            }
        }
    }
);

define_class!(
    // SAFETY: NSObject has no subclassing requirements; no ivars or Drop.
    #[unsafe(super(NSObject))]
    #[name = "KhorMenuTarget"]
    struct MenuTarget;
    impl MenuTarget {
        // SAFETY: WKScriptMessageHandler's exact Objective-C signature.
        #[unsafe(method(userContentController:didReceiveScriptMessage:))]
        fn message(&self, _controller: *mut AnyObject, message: *mut AnyObject) {
            receive(message);
        }
        // A completed outside tap observes the original page interaction; it
        // never cancels that touch, and a drag never recognizes as a tap.
        #[unsafe(method(outside:))]
        fn outside(&self, sender: *mut AnyObject) {
            let state: isize = unsafe { msg_send![sender, state] };
            if state == 3 { dismiss_current(true); }
        }
        #[unsafe(method(gestureRecognizer:shouldReceiveTouch:))]
        fn receive_touch(&self, recognizer: *mut AnyObject, touch: *mut AnyObject) -> bool {
            PANEL.with(|panel| {
                let panel = panel.borrow();
                let Some(p) = panel.as_ref() else { return false; };
                if !std::ptr::eq(&*p.outside, recognizer) { return false; }
                unsafe {
                    let touched: *mut AnyObject = msg_send![touch, view];
                    if !touched.is_null() {
                        let in_menu: bool = msg_send![touched, isDescendantOfView: &*p.view];
                        if in_menu { return false; }
                    }
                    // The host is bounded above/below the field. Taps in the
                    // field remain edits, not outside dismissals.
                    let point: NSPoint = msg_send![touch, locationInView: &*p.backdrop];
                    let bounds: NSRect = msg_send![&*p.backdrop, bounds];
                    point.x >= bounds.origin.x && point.y >= bounds.origin.y
                        && point.x <= bounds.origin.x + bounds.size.width
                        && point.y <= bounds.origin.y + bounds.size.height
                }
            })
        }
        #[unsafe(method(gestureRecognizer:shouldRecognizeSimultaneouslyWithGestureRecognizer:))]
        fn simultaneous(&self, _first: *mut AnyObject, _second: *mut AnyObject) -> bool { true }
        // SAFETY: WKNavigationDelegate's completion signature.
        #[unsafe(method(webView:didFinishNavigation:))]
        fn finished(&self, view: *mut AnyObject, _navigation: *mut AnyObject) {
            let update = PANEL.with(|panel| {
                let mut panel = panel.borrow_mut();
                let p = panel.as_mut()?;
                if !std::ptr::eq(&*p.view, view) { return None; }
                p.loaded = true;
                Some((p.view.clone(), p.request.clone()))
            });
            if let Some((view, request)) = update { update_document(&view, &request); }
        }
        // SAFETY: WKNavigationDelegate's policy signature. No navigation to sites.
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn policy(&self, _view: *mut AnyObject, action: *mut AnyObject, done: &block2::Block<dyn Fn(isize)>) {
            let allowed = unsafe {
                let request: *mut AnyObject = msg_send![action, request];
                let url: *mut AnyObject = msg_send![request, URL];
                if url.is_null() { false } else {
                    let text: Option<Retained<NSString>> = msg_send![url, absoluteString];
                    text.is_some_and(|s| s.to_string() == "about:blank")
                }
            };
            done.call((if allowed { 1 } else { 0 },));
        }
    }
);

// Injected as a document-end, main-frame-only WKUserScript. CSP remains
// script-src 'none': snapshot markup itself cannot install any script.
const BRIDGE: &str = r#"(()=>{
 let id='', observer;
 const post=value=>window.webkit.messageHandlers.khorMenu.postMessage({...value,id});
 function size(){const menu=document.querySelector('[data-omni-menu]');if(menu)post({h:menu.getBoundingClientRect().height,radius:parseFloat(getComputedStyle(menu).borderRadius)||0,dark:getComputedStyle(document.documentElement).colorScheme==='dark'});}
 function mounted(next){id=next;observer?.disconnect();const menu=document.querySelector('[data-omni-menu]');if(menu){observer=new ResizeObserver(size);observer.observe(menu);const active=menu.querySelector('[data-on="true"]');active?.scrollIntoView({block:'nearest'});}size();}
 window.__khorMenuReplace=(html,next)=>{
   const parsed=new DOMParser().parseFromString(html,'text/html');
   document.documentElement.style.colorScheme=parsed.documentElement.style.colorScheme;
   const previous=document.querySelector('[data-menu-viewport]');
   const scroll=previous?.scrollTop||0;
   document.head.replaceChildren(...parsed.head.childNodes);
   document.body.replaceChildren(...parsed.body.childNodes);
   const viewport=document.querySelector('[data-menu-viewport]');if(viewport)viewport.scrollTop=scroll;
   mounted(next);
 };
 document.addEventListener('mousedown',event=>{if(event.target.closest?.('[data-omni-item]'))event.preventDefault();});
 document.addEventListener('click',event=>{
   const row=event.target.closest?.('[data-omni-item]');if(!row)return;
   event.preventDefault();post({key:row.getAttribute('data-omni-item')});
 });
 document.addEventListener('keydown',event=>{if(event.key==='Escape')post({dismiss:true});});
})();"#;

fn update_document(view: &AnyObject, request: &MenuRequest) {
    let Some(html) = &request.html else { return };
    let source = format!(
        "window.__khorMenuReplace?.({},{});",
        serde_json::to_string(html).unwrap(),
        serde_json::to_string(&request.id).unwrap()
    );
    unsafe {
        let _: () = msg_send![view, evaluateJavaScript: &*NSString::from_str(&source), completionHandler: std::ptr::null::<AnyObject>()];
    }
}

/// Show/update from khor's own main WebView only. CSS coordinates become the
/// sibling view's UIKit points; no physical-pixel conversion is applied.
pub fn show(
    app: &tauri::AppHandle,
    face: *mut AnyObject,
    owner: &str,
    request: MenuRequest,
) -> Result<bool, String> {
    request.validate()?;
    if owner != "main" {
        return Err("this face cannot host a phone menu".into());
    }
    let Some(html) = &request.html else {
        return Ok(false);
    };
    if html.is_empty() {
        return Ok(false);
    }
    let _ = APP.set(app.clone());
    unsafe {
        let parent: *mut AnyObject = msg_send![face, superview];
        if parent.is_null() {
            return Ok(false);
        }
        let base: NSRect = msg_send![face, frame];
        let same = PANEL.with(|p| {
            p.borrow()
                .as_ref()
                .is_some_and(|p| p.request.id == request.id && p.owner == owner)
        });
        if !same {
            dismiss_current(false);
        }
        super::claim(owner, &request)?;
        if same {
            let update = PANEL.with(|panel| {
                let mut panel = panel.borrow_mut();
                let p = panel.as_mut().unwrap();
                let changed = p.request.html != request.html;
                p.request = request.clone();
                p.origin = base.origin;
                if changed && p.loaded {
                    Some(p.view.clone())
                } else {
                    None
                }
            });
            if let Some(view) = update {
                update_document(&view, &request);
            }
            reposition();
            raise();
            return Ok(true);
        }
        let config: Retained<AnyObject> = msg_send![class!(WKWebViewConfiguration), new];
        let store: Retained<AnyObject> =
            msg_send![class!(WKWebsiteDataStore), nonPersistentDataStore];
        let _: () = msg_send![&*config, setWebsiteDataStore: &*store];
        let controller: *mut AnyObject = msg_send![&*config, userContentController];
        let source = NSString::from_str(BRIDGE);
        let script: Allocated<AnyObject> = msg_send![class!(WKUserScript), alloc];
        let script: Retained<AnyObject> = msg_send![script, initWithSource: &*source, injectionTime: 1isize, forMainFrameOnly: true];
        let _: () = msg_send![controller, addUserScript: &*script];
        TARGET.with(|target| {let _: () = msg_send![controller, addScriptMessageHandler: &**target, name: &*NSString::from_str("khorMenu")];});
        let a: Allocated<AnyObject> = msg_send![menu_webview_class()?, alloc];
        let view: Retained<AnyObject> =
            msg_send![a, initWithFrame: rect(0.,0.,request.at.w,1.), configuration: &*config];
        TARGET.with(|target| {
            let _: () = msg_send![&*view, setNavigationDelegate: &**target];
        });
        let clear: Retained<AnyObject> = msg_send![class!(UIColor), clearColor];
        let _: () = msg_send![&*view, setOpaque: false];
        let _: () = msg_send![&*view, setBackgroundColor: &*clear];
        let _: () = msg_send![&*view, setAllowsLinkPreview: false];
        let scroll: *mut AnyObject = msg_send![&*view, scrollView];
        let _: () = msg_send![scroll, setBackgroundColor: &*clear];
        let _: () = msg_send![scroll, setBounces: false];
        let _: () = msg_send![scroll, setScrollEnabled: false];
        // A native material behind the transparent WK view can blur the sibling
        // webpage. CSS backdrop-filter alone cannot sample another WebView.
        let effect: Retained<AnyObject> = msg_send![class!(UIBlurEffect), effectWithStyle: 8isize];
        let a: Allocated<AnyObject> = msg_send![class!(UIVisualEffectView), alloc];
        let material: Retained<AnyObject> = msg_send![a, initWithEffect: &*effect];
        let _: () = msg_send![&*material, setUserInteractionEnabled: false];
        let layer: *mut AnyObject = msg_send![&*material, layer];
        let _: () = msg_send![layer, setMasksToBounds: true];
        let a: Allocated<AnyObject> = msg_send![MenuBackdrop::class(), alloc];
        let bounds: NSRect = msg_send![parent, bounds];
        let backdrop: Retained<AnyObject> = msg_send![a, initWithFrame: bounds];
        let _: () = msg_send![&*backdrop, setBackgroundColor: &*clear];
        let _: () = msg_send![&*backdrop, addSubview: &*material];
        let _: () = msg_send![&*backdrop, addSubview: &*view];
        let _: () = msg_send![parent, addSubview: &*backdrop];
        let outside = TARGET.with(|target| {
            let a: Allocated<AnyObject> = msg_send![class!(UITapGestureRecognizer), alloc];
            let recognizer: Retained<AnyObject> =
                msg_send![a, initWithTarget: &**target, action: objc2::sel!(outside:)];
            let _: () = msg_send![&*recognizer, setCancelsTouchesInView: false];
            let _: () = msg_send![&*recognizer, setDelaysTouchesBegan: false];
            let _: () = msg_send![&*recognizer, setDelaysTouchesEnded: false];
            let _: () = msg_send![&*recognizer, setDelegate: &**target];
            let _: () = msg_send![parent, addGestureRecognizer: &*recognizer];
            recognizer
        });
        // Hidden until the trusted document has measured its actual menu.
        let _: () = msg_send![&*backdrop, setHidden: true];
        let _: Option<Retained<AnyObject>> = msg_send![&*view, loadHTMLString: &*NSString::from_str(html), baseURL: std::ptr::null::<AnyObject>()];
        PANEL.with(|p| {
            *p.borrow_mut() = Some(Panel {
                owner: owner.into(),
                request,
                backdrop,
                view,
                material,
                outside,
                radius: 0.0,
                origin: base.origin,
                loaded: false,
            })
        });
    }
    reposition();
    raise();
    Ok(true)
}

unsafe fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w.max(0.), h.max(0.)))
}

fn reposition() {
    PANEL.with(|panel| {
        let p = panel.borrow();
        let Some(p) = p.as_ref() else { return };
        unsafe {
            let at = &p.request.at;
            let parent: *mut AnyObject = msg_send![&*p.backdrop, superview];
            if parent.is_null() {
                return;
            }
            let bounds: NSRect = msg_send![parent, bounds];
            // Do not cover the field itself: typing, caret placement and the
            // keyboard continue in the original main WebView.
            let edge = (p.origin.y + at.y).clamp(0., bounds.size.height);
            let area = if at.above {
                rect(0., 0., bounds.size.width, edge)
            } else {
                rect(0., edge, bounds.size.width, bounds.size.height - edge)
            };
            let _: () = msg_send![&*p.backdrop,setFrame:area];
            let old: NSRect = msg_send![&*p.view, frame];
            let h = old.size.height.min(area.size.height);
            let w = at.w.min(bounds.size.width);
            let x = (p.origin.x + at.x).clamp(0., (bounds.size.width - w).max(0.));
            let y = if at.above {
                (area.size.height - h).max(0.)
            } else {
                0.
            };
            let frame = rect(x, y, w, h);
            let _: () = msg_send![&*p.view,setFrame:frame];
            let _: () = msg_send![&*p.material,setFrame:frame];
            let layer: *mut AnyObject = msg_send![&*p.material, layer];
            let radius = p.radius.min(w / 2.).min(h / 2.);
            let _: () = msg_send![layer, setCornerRadius: radius];
        }
    });
}

fn receive(message: *mut AnyObject) {
    let parsed = unsafe {
        let frame: *mut AnyObject = msg_send![message, frameInfo];
        let main: bool = msg_send![frame, isMainFrame];
        if !main {
            return;
        }
        let sender: *mut AnyObject = msg_send![message, webView];
        let ours = PANEL.with(|p| {
            p.borrow()
                .as_ref()
                .is_some_and(|p| std::ptr::eq(&*p.view, sender))
        });
        if !ours {
            return;
        }
        let body: *mut AnyObject = msg_send![message, body];
        // A malformed bridge message must be ignored, not reach Foundation's
        // exception-raising non-container JSON encoder.
        if body.is_null() {
            return;
        }
        let dictionary: bool = msg_send![body, isKindOfClass: class!(NSDictionary)];
        if !dictionary {
            return;
        }
        let text: Option<Retained<AnyObject>> = msg_send![class!(NSJSONSerialization),dataWithJSONObject:body,options:0usize,error:std::ptr::null_mut::<*mut AnyObject>()];
        // NSJSONSerialization returns NSData, not NSString: decode explicitly.
        let Some(data) = text else {
            return;
        };
        let a: Allocated<NSString> = msg_send![class!(NSString), alloc];
        let text: Option<Retained<NSString>> = msg_send![a,initWithData:&*data,encoding:4usize];
        let Some(text) = text else {
            return;
        };
        serde_json::from_str::<serde_json::Value>(&text.to_string()).ok()
    };
    let Some(value) = parsed else {
        return;
    };
    let Some(id) = value["id"].as_str() else {
        return;
    };
    let current = PANEL.with(|p| {
        p.borrow()
            .as_ref()
            .is_some_and(|p| p.request.id == id && super::matches(&p.owner, id))
    });
    if !current {
        return;
    }
    if value["dismiss"].as_bool() == Some(true) {
        dismiss_current(true);
        return;
    }
    if let Some(key) = value["key"].as_str() {
        if let (Some(owner), Some(app)) = (super::pick_owner(id, key), APP.get()) {
            let _ = app.emit_to(&owner, "menu:pick", serde_json::json!({"id":id,"key":key}));
        }
    } else if let Some(h) = value["h"].as_f64() {
        if !super::accepts_size(id, h) {
            return;
        }
        PANEL.with(|panel|{if let Some(p)=panel.borrow_mut().as_mut(){unsafe{
            let old:NSRect=msg_send![&*p.view,frame];
            let _:()=msg_send![&*p.view,setFrame:rect(old.origin.x,old.origin.y,old.size.width,h.min(p.request.at.view))];
            p.radius=value["radius"].as_f64().filter(|r|r.is_finite()).unwrap_or(0.).max(0.);
            let _:()=msg_send![&*p.material,setOverrideUserInterfaceStyle:if value["dark"].as_bool()==Some(true){2isize}else{1isize}];
            let _:()=msg_send![&*p.backdrop,setHidden:false];
        }}});
        reposition();
        raise();
    }
}

pub fn hide(owner: &str, id: &str) {
    if super::matches(owner, id) {
        dismiss_current(false);
    }
}
pub fn dismiss_current(notify: bool) {
    let old = PANEL.with(|p| p.borrow_mut().take());
    if let Some(p) = old {
        super::dismiss(&p.owner, &p.request.id);
        unsafe {
            let parent: *mut AnyObject = msg_send![&*p.outside, view];
            if !parent.is_null() {
                let _: () = msg_send![parent, removeGestureRecognizer: &*p.outside];
            }
            let _: () = msg_send![&*p.outside, setDelegate: std::ptr::null::<AnyObject>()];
            let config: *mut AnyObject = msg_send![&*p.view, configuration];
            let controller: *mut AnyObject = msg_send![config, userContentController];
            let _: () = msg_send![controller,removeScriptMessageHandlerForName:&*NSString::from_str("khorMenu")];
            let _: () = msg_send![&*p.view,setNavigationDelegate:std::ptr::null::<AnyObject>()];
            let _: () = msg_send![&*p.view, stopLoading];
            let _: () = msg_send![&*p.backdrop, removeFromSuperview];
        }
        if notify {
            if let Some(app) = APP.get() {
                let _ = app.emit_to(
                    &p.owner,
                    "menu:dismiss",
                    serde_json::json!({"id":p.request.id}),
                );
            }
        }
    }
}
/// A newly shown page/deck is another sibling: keep the active menu in front.
pub fn raise() {
    PANEL.with(|p| {
        if let Some(p) = p.borrow().as_ref() {
            unsafe {
                let parent: *mut AnyObject = msg_send![&*p.backdrop, superview];
                if !parent.is_null() {
                    let _: () = msg_send![parent,bringSubviewToFront:&*p.backdrop];
                }
            }
        }
    });
}
