//! Reads the general pasteboard for a user-pressed terminal key.
use std::sync::{Arc, Mutex};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, class, msg_send};
use objc2_foundation::NSString;
use serde_json::{Value, json};

type Report = Arc<dyn Fn(Value) + Send + Sync>;
const UTF8_TEXT: &str = "public.utf8-plain-text";

pub fn read(report: Report) {
    let Some(_) = MainThreadMarker::new() else {
        report(Value::Null);
        return;
    };
    // One read for an explicit key press. UIKit applies the user's
    // cross-app clipboard permission before delivering the item providers.
    let providers: Option<Retained<AnyObject>> = unsafe {
        let pasteboard: Retained<AnyObject> = msg_send![class!(UIPasteboard), generalPasteboard];
        msg_send![&*pasteboard, itemProviders]
    };
    if let Some(providers) = providers {
        read_text(providers, 0, report);
    } else {
        report(Value::Null);
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
        if items.is_empty() {
            report(Value::Null);
            return;
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
                    report(if images.is_empty() {
                        Value::Null
                    } else {
                        json!({ "images": images })
                    });
                }
            });
            let _: *mut AnyObject = msg_send![item, loadDataRepresentationForTypeIdentifier: &*kind, completionHandler: &*complete];
        }
    }
}
