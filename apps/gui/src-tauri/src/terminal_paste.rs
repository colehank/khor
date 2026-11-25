//! The terminal key row's iOS paste button. UIKit supplies the item providers
//! for a press on UIPasteControl, and the face owns the destination terminal.
use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};

use block2::RcBlock;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, class, define_class, msg_send, sel};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_ui_kit::{UIResponder, UIView};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
pub struct Geometry {
    pub frame: [f64; 4],
    pub clip: [f64; 4],
    pub radius: f64,
    pub dark: bool,
}

type Report = Arc<dyn Fn(Value) + Send + Sync>;
const UTF8_TEXT: &str = "public.utf8-plain-text";

struct TargetState {
    report: Report,
    drag_x: Cell<f64>,
}

define_class!(
    // SAFETY: UIView implements the paste configuration via UIResponder.
    // The target is attached to the same window as the paste control.
    #[unsafe(super(UIView, UIResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KhorTerminalPasteTarget"]
    #[ivars = TargetState]
    struct Target;

    impl Target {
        #[unsafe(method(canPasteItemProviders:))]
        fn can_paste(&self, providers: &AnyObject) -> bool {
            unsafe {
                let count: usize = msg_send![providers, count];
                (0..count).any(|index| {
                    let item: *mut AnyObject = msg_send![providers, objectAtIndex: index];
                    let text: bool = msg_send![item, canLoadObjectOfClass: class!(NSString)];
                    text || supports(item, UTF8_TEXT) || crate::edit_menu::PICTURES.iter().any(|&(uti, _)| supports(item, uti))
                })
            }
        }

        #[unsafe(method(pasteItemProviders:))]
        fn paste(&self, providers: &AnyObject) {
            // The completion may run after UIKit releases the original array.
            let retained = unsafe { Retained::retain(providers as *const AnyObject as *mut AnyObject) };
            if let Some(providers) = retained {
                read_text(providers, 0, self.ivars().report.clone());
            }
        }

        #[unsafe(method(gestureRecognizerShouldBegin:))]
        fn can_scroll(&self, pan: &AnyObject) -> bool {
            let velocity: NSPoint = unsafe { msg_send![pan, velocityInView: std::ptr::null::<AnyObject>()] };
            velocity.x.abs() > velocity.y.abs()
        }

        #[unsafe(method(dragPasteKey:))]
        fn drag(&self, pan: &AnyObject) {
            let state: isize = unsafe { msg_send![pan, state] };
            if state == 1 {
                self.ivars().drag_x.set(0.0);
            }
            if (1..=3).contains(&state) {
                // Window coordinates stay still while the key itself moves.
                let translation: NSPoint = unsafe { msg_send![pan, translationInView: std::ptr::null::<AnyObject>()] };
                let previous = self.ivars().drag_x.replace(translation.x);
                (self.ivars().report)(json!({ "scroll": previous - translation.x }));
            }
        }
    }
);

struct Seat {
    owner: String,
    host: Retained<Target>,
    control: Retained<AnyObject>,
}

thread_local! {
    static SEAT: RefCell<Option<Seat>> = const { RefCell::new(None) };
}

fn rect([x, y, w, h]: [f64; 4]) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}

/// Mount or move the control in the face's UIKit coordinates, clipped to the
/// horizontal key row. Closing one terminal cannot hide a newer terminal's key.
pub fn place(
    web: *mut AnyObject,
    owner: String,
    geometry: Option<Geometry>,
    report: Report,
) -> bool {
    let Some(main) = MainThreadMarker::new() else {
        return false;
    };
    SEAT.with(|slot| {
        let mut slot = slot.borrow_mut();
        if geometry.is_none() {
            if slot.as_ref().is_some_and(|seat| seat.owner == owner) {
                remove(&mut slot);
            }
            return false;
        }
        let Some(g) = geometry else { return false };
        if web.is_null() || !g.frame.into_iter().chain(g.clip).chain([g.radius]).all(f64::is_finite)
            || g.clip[2] <= 0.0 || g.clip[3] <= 0.0 || g.frame[2] <= 0.0 || g.frame[3] <= 0.0
        {
            return false;
        }
        // Keep the system control above WebKit's own scrolling and keyboard
        // subviews. A viewport under reattachment cannot accept an overlay.
        let parent: *mut AnyObject = unsafe { msg_send![web, superview] };
        let window: *mut AnyObject = unsafe { msg_send![web, window] };
        if parent.is_null() || window.is_null() {
            if slot.as_ref().is_some_and(|seat| seat.owner == owner) {
                remove(&mut slot);
            }
            return false;
        }
        if slot.as_ref().is_none_or(|seat| seat.owner != owner) {
            remove(&mut slot);
            // SAFETY: with_webview supplied a live face on UIKit's thread.
            *slot = unsafe { make_seat(main, owner, g.radius, report) };
        }
        let Some(seat) = slot.as_ref() else { return false };
        // SAFETY: the retained UIView and UIPasteControl are on UIKit's thread.
        unsafe {
            let [x, y, width, height] = g.frame;
            let [left, top, _, _] = g.clip;
            let frame: NSRect = msg_send![web, convertRect: rect(g.clip), toView: parent];
            let _: () = msg_send![&*seat.host, setFrame: frame];
            let _: () = msg_send![&*seat.control, setFrame: rect([x - left, y - top, width, height])];
            let _: () = msg_send![&*seat.host, setOverrideUserInterfaceStyle: if g.dark { 2isize } else { 1isize }];
            let current: *mut AnyObject = msg_send![&*seat.host, superview];
            if current != parent {
                let _: () = msg_send![parent, addSubview: &*seat.host];
            }
        }
        true
    })
}

fn remove(slot: &mut Option<Seat>) {
    if let Some(seat) = slot.take() {
        unsafe {
            let _: () = msg_send![&*seat.host, removeFromSuperview];
        }
    }
}

unsafe fn make_seat(
    main: MainThreadMarker,
    owner: String,
    radius: f64,
    report: Report,
) -> Option<Seat> {
    // Resolve these classes at runtime for the iOS 15 browser fallback.
    let control_class = AnyClass::get(c"UIPasteControl")?;
    let config_class = AnyClass::get(c"UIPasteControlConfiguration")?;
    unsafe {
        let configuration: Retained<AnyObject> = msg_send![config_class, new];
        let color: Retained<AnyObject> = msg_send![class!(UIColor), labelColor];
        let fill: Retained<AnyObject> = msg_send![class!(UIColor), secondarySystemFillColor];
        let _: () = msg_send![&*configuration, setBaseForegroundColor: &*color];
        let _: () = msg_send![&*configuration, setBaseBackgroundColor: &*fill];
        let _: () = msg_send![&*configuration, setDisplayMode: 2usize]; // Label only
        let _: () = msg_send![&*configuration, setCornerRadius: radius.max(0.0)];
        let control: Allocated<AnyObject> = msg_send![control_class, alloc];
        let control: Retained<AnyObject> =
            msg_send![control, initWithConfiguration: &*configuration];
        let allocated = Target::alloc(main).set_ivars(TargetState {
            report,
            drag_x: Cell::new(0.0),
        });
        let target: Retained<Target> = msg_send![super(allocated), initWithFrame: NSRect::ZERO];
        let _: () = msg_send![&*target, setClipsToBounds: true];
        let text = NSString::from_str("public.text");
        let image = NSString::from_str("public.image");
        let types: Retained<AnyObject> = msg_send![class!(NSArray), arrayWithObject: &*text];
        let types: Retained<AnyObject> = msg_send![&*types, arrayByAddingObject: &*image];
        let paste_configuration: Allocated<AnyObject> =
            msg_send![class!(UIPasteConfiguration), alloc];
        let paste_configuration: Retained<AnyObject> =
            msg_send![paste_configuration, initWithAcceptableTypeIdentifiers: &*types];
        let _: () = msg_send![&*target, setPasteConfiguration: &*paste_configuration];
        let _: () = msg_send![&*control, setTarget: &*target];
        let identifier = NSString::from_str("khor-terminal-paste");
        let _: () = msg_send![&*control, setAccessibilityIdentifier: &*identifier];
        let pan: Allocated<AnyObject> = msg_send![class!(UIPanGestureRecognizer), alloc];
        let pan: Retained<AnyObject> =
            msg_send![pan, initWithTarget: &*target, action: sel!(dragPasteKey:)];
        let _: () = msg_send![&*pan, setDelegate: &*target];
        let _: () = msg_send![&*target, addGestureRecognizer: &*pan];
        let _: () = msg_send![&*target, addSubview: &*control];
        Some(Seat {
            owner,
            host: target,
            control,
        })
    }
}

fn read_text(providers: Retained<AnyObject>, start: usize, report: Report) {
    // Text wins over accompanying images. The first nonempty string is
    // selected in provider order, including when an earlier load fails.
    unsafe {
        let count: usize = msg_send![&*providers, count];
        for index in start..count {
            let item: *mut AnyObject = msg_send![&*providers, objectAtIndex: index];
            let text: bool = msg_send![item, canLoadObjectOfClass: class!(NSString)];
            if text {
                let other = providers.clone();
                let complete =
                    RcBlock::new(move |string: *mut NSString, _error: *mut AnyObject| {
                        if let Some(text) = string
                            .as_ref()
                            .map(ToString::to_string)
                            .filter(|s| !s.is_empty())
                        {
                            report(json!({ "text": text }));
                        } else {
                            read_text_data(other.clone(), index, report.clone());
                        }
                    });
                let _: *mut AnyObject = msg_send![item, loadObjectOfClass: class!(NSString), completionHandler: &*complete];
                return;
            }
            if supports(item, UTF8_TEXT) {
                read_text_data(providers, index, report);
                return;
            }
        }
        read_images(&providers, report);
    }
}

unsafe fn supports(item: *mut AnyObject, type_identifier: &str) -> bool {
    let kind = NSString::from_str(type_identifier);
    unsafe { msg_send![item, hasItemConformingToTypeIdentifier: &*kind] }
}

fn read_text_data(providers: Retained<AnyObject>, index: usize, report: Report) {
    unsafe {
        let item: *mut AnyObject = msg_send![&*providers, objectAtIndex: index];
        if !supports(item, UTF8_TEXT) {
            read_text(providers, index + 1, report);
            return;
        }
        let kind = NSString::from_str(UTF8_TEXT);
        let complete = RcBlock::new(move |data: *mut AnyObject, _error: *mut AnyObject| {
            if let Some(data) = data.as_ref() {
                let size: usize = msg_send![data, length];
                if size > 0 {
                    let bytes: *const u8 = msg_send![data, bytes];
                    if let Ok(text) = std::str::from_utf8(std::slice::from_raw_parts(bytes, size)) {
                        report(json!({ "text": text }));
                        return;
                    }
                }
            }
            read_text(providers.clone(), index + 1, report.clone());
        });
        let _: *mut AnyObject = msg_send![item, loadDataRepresentationForTypeIdentifier: &*kind, completionHandler: &*complete];
    }
}

struct ImageBatch {
    pending: usize,
    parts: Vec<Option<Value>>,
}

fn read_images(providers: &AnyObject, report: Report) {
    // Only these granted providers are inspected. Keep the original order
    // when the providers complete on different queues.
    unsafe {
        let count: usize = msg_send![providers, count];
        let mut items = Vec::new();
        for index in 0..count {
            let item: *mut AnyObject = msg_send![providers, objectAtIndex: index];
            for (uti, mime) in crate::edit_menu::PICTURES {
                if supports(item, uti) {
                    items.push((item, NSString::from_str(uti), mime));
                    break;
                }
            }
        }
        let batch = Arc::new(Mutex::new(ImageBatch {
            pending: items.len(),
            parts: vec![None; items.len()],
        }));
        for (index, (item, kind, mime)) in items.into_iter().enumerate() {
            let batch = batch.clone();
            let report = report.clone();
            let complete = RcBlock::new(move |data: *mut AnyObject, _error: *mut AnyObject| {
                let encoded: Option<Retained<NSString>> = if data.is_null() {
                    None
                } else {
                    msg_send![data, base64EncodedStringWithOptions: 0usize]
                };
                let mut batch = batch.lock().unwrap_or_else(|error| error.into_inner());
                batch.parts[index] =
                    encoded.map(|data| json!({ "type": mime, "data": data.to_string() }));
                batch.pending -= 1;
                if batch.pending == 0 {
                    let images = std::mem::take(&mut batch.parts)
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    drop(batch);
                    if !images.is_empty() {
                        report(json!({ "images": images }));
                    }
                }
            });
            let _: *mut AnyObject = msg_send![item, loadDataRepresentationForTypeIdentifier: &*kind, completionHandler: &*complete];
        }
    }
}
