//! The terminal key row's iOS paste button. UIKit supplies the item providers
//! for a press on UIPasteControl, and the face owns the destination terminal.
use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};

use block2::RcBlock;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, class, define_class, msg_send, sel};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_ui_kit::UIResponder;
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

struct TargetState {
    report: Report,
    drag_x: Cell<f64>,
}

define_class!(
    // SAFETY: UIResponder supplies UIPasteConfigurationSupporting's
    // configuration property. The delegate methods match UIKit's signatures.
    #[unsafe(super(UIResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KhorTerminalPasteTarget"]
    #[ivars = TargetState]
    struct Target;

    impl Target {
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
    host: Retained<AnyObject>,
    control: Retained<AnyObject>,
    _target: Retained<Target>,
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
        if slot.as_ref().is_none_or(|seat| seat.owner != owner) {
            remove(&mut slot);
            // SAFETY: with_webview supplied a live face on UIKit's thread.
            *slot = unsafe { make_seat(web, main, owner, g.radius, report) };
        }
        let Some(seat) = slot.as_ref() else { return false };
        // SAFETY: the retained UIView and UIPasteControl are on UIKit's thread.
        unsafe {
            let [x, y, width, height] = g.frame;
            let [left, top, _, _] = g.clip;
            let _: () = msg_send![&*seat.host, setFrame: rect(g.clip)];
            let _: () = msg_send![&*seat.control, setFrame: rect([x - left, y - top, width, height])];
            let _: () = msg_send![&*seat.host, setOverrideUserInterfaceStyle: if g.dark { 2isize } else { 1isize }];
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
    web: *mut AnyObject,
    main: MainThreadMarker,
    owner: String,
    radius: f64,
    report: Report,
) -> Option<Seat> {
    // Resolve these classes at runtime for the iOS 15 browser fallback.
    let control_class = AnyClass::get(c"UIPasteControl")?;
    let config_class = AnyClass::get(c"UIPasteControlConfiguration")?;
    unsafe {
        let host: Retained<AnyObject> = msg_send![class!(UIView), new];
        let _: () = msg_send![&*host, setClipsToBounds: true];
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
        let target: Retained<Target> = msg_send![super(allocated), init];
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
        let _: () = msg_send![&*host, addGestureRecognizer: &*pan];
        let _: () = msg_send![&*host, addSubview: &*control];
        let _: () = msg_send![web, addSubview: &*host];
        Some(Seat {
            owner,
            host,
            control,
            _target: target,
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
                            read_text(other.clone(), index + 1, report.clone());
                        }
                    });
                let _: *mut AnyObject = msg_send![item, loadObjectOfClass: class!(NSString), completionHandler: &*complete];
                return;
            }
        }
        read_images(&providers, report);
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
                let kind = NSString::from_str(uti);
                let supported: bool = msg_send![item, hasItemConformingToTypeIdentifier: &*kind];
                if supported {
                    items.push((item, kind, mime));
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
