//! A real AppKit window wearing `window_buttons` — the wk family's arm
//! for the three buttons (docs/SHELL.md 插队; `npm run wk` runs it).
//!
//! Builds a window in khor's style (titled, full-size content view, a
//! transparent title bar with the title hidden — `titleBarStyle:
//! Overlay` in tauri.conf.json), squeezes the buttons, then walks the
//! relayouts in order and prints the three frames after each, one JSON
//! line per step:
//!
//! - `resize`, `appearance`, `key`, `subview` — each followed by the
//!   squeeze the app's own event hooks make (the insurance);
//! - `relayout` — AppKit lays the title bar out afresh **with no hook of
//!   ours behind it** (a title change does it), standing in for every
//!   event nobody has listed.
//!
//! Mode `events-only` is the app before 2026-09-22 — squeeze on events
//! and nothing else — and must leave the buttons out of place after
//! `relayout`: the gate's control arm, proof the step really moves them.
//! Mode `watch` also installs `window_buttons::watch`, and every step
//! must end squeezed.

#[cfg(target_os = "macos")]
#[path = "../src/window_buttons.rs"]
mod window_buttons;

#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::runtime::{AnyObject, Bool};
    use objc2::{class, msg_send};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    let watching = std::env::args().nth(1).as_deref() != Some("events-only");
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        // NSApplicationActivationPolicyAccessory: a window, no Dock icon.
        let _: Bool = msg_send![app, setActivationPolicy: 1isize];
        let _: () = msg_send![app, finishLaunching];

        // titled | closable | miniaturizable | resizable | fullSizeContentView
        let style: usize = 1 | 2 | 4 | 8 | (1 << 15);
        let rect = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(900.0, 600.0));
        let win: *mut AnyObject = msg_send![class!(NSWindow), alloc];
        let win: *mut AnyObject = msg_send![
            win,
            initWithContentRect: rect,
            styleMask: style,
            backing: 2usize,
            defer: Bool::NO
        ];
        let _: () = msg_send![win, setReleasedWhenClosed: Bool::NO];
        let _: () = msg_send![win, setTitlebarAppearsTransparent: Bool::YES];
        // NSWindowTitleHidden = 1
        let _: () = msg_send![win, setTitleVisibility: 1isize];
        let _: () = msg_send![win, makeKeyAndOrderFront: std::ptr::null_mut::<AnyObject>()];

        let spin = |secs: f64| {
            let run: *mut AnyObject = msg_send![class!(NSRunLoop), currentRunLoop];
            let until: *mut AnyObject =
                msg_send![class!(NSDate), dateWithTimeIntervalSinceNow: secs];
            let _: () = msg_send![run, runUntilDate: until];
        };
        let report = |step: &str| {
            let mut xs = Vec::new();
            let mut ends = Vec::new();
            for i in 0..3usize {
                let b: *mut AnyObject = msg_send![win, standardWindowButton: i];
                if b.is_null() {
                    continue;
                }
                let f: NSRect = msg_send![b, frame];
                xs.push(f.origin.x);
                ends.push(f.origin.x + f.size.width);
            }
            println!(
                "{{\"step\":\"{step}\",\"xs\":{:?},\"ends\":{:?},\"watching\":{watching}}}",
                xs, ends
            );
        };

        spin(0.5);
        report("sdk");
        window_buttons::squeeze(win);
        if watching {
            window_buttons::watch(win);
        }
        spin(0.3);
        report("squeezed");

        let bigger = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(1000.0, 650.0));
        let _: () = msg_send![win, setFrame: bigger, display: Bool::YES];
        window_buttons::squeeze(win);
        spin(0.3);
        report("resize");

        let dark = NSString::from_str("NSAppearanceNameDarkAqua");
        let look: *mut AnyObject = msg_send![class!(NSAppearance), appearanceNamed: &*dark];
        let _: () = msg_send![win, setAppearance: look];
        window_buttons::squeeze(win);
        spin(0.3);
        report("appearance");

        let _: () = msg_send![win, makeKeyAndOrderFront: std::ptr::null_mut::<AnyObject>()];
        window_buttons::squeeze(win);
        spin(0.3);
        report("key");

        let content: *mut AnyObject = msg_send![win, contentView];
        let kid: *mut AnyObject = msg_send![class!(NSView), alloc];
        let kid: *mut AnyObject = msg_send![kid, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, 100.0))];
        let _: () = msg_send![content, addSubview: kid];
        window_buttons::squeeze(win);
        spin(0.3);
        report("subview");

        // No hook of ours: AppKit lays the title bar out on its own.
        let title = NSString::from_str("khor");
        let _: () = msg_send![win, setTitle: &*title];
        spin(0.5);
        report("relayout");

        let _: () = msg_send![win, close];
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("{{\"skipped\":\"not macOS\"}}");
}
