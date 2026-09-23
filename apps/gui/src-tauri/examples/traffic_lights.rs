//! Does the number khor works out really centre macOS's three buttons on
//! its title row? (docs/SHELL.md 第十一轮; `npm run wk` runs this.)
//!
//! A real AppKit window in khor's style, and **tao's own placement
//! repeated on it**: `trafficLightPosition`'s `y` sets the system title
//! bar container's height to `button height + y` with the container
//! pinned to the window's top, and the buttons keep their own place
//! inside it (`tao::platform_impl::macos::view::inset_traffic_lights` —
//! the seven lines this repeats). khor cannot call that at run time (it
//! is taken when the window is built), so what this measures is the
//! arithmetic: given the rule, does `traffic::inset_y` put the trio's
//! centre on the row's centre — for more than one row height, since the
//! height is a token that can move and the button size is the SDK's.
//!
//! One JSON line per row height: the height asked for, the trio's centre
//! measured from the window's top, and the spacing between the buttons
//! (which nothing here may change).
//!
//! Two arms, and the first is the point. `written-down` is the number
//! `tauri.conf.json` carried until this round (`y: 12`) — without it,
//! 「the trio is centred」 and 「this window centres it anyway」 read the
//! same. `measured` is what khor now works out.

#[cfg(target_os = "macos")]
#[path = "../src/traffic.rs"]
mod traffic;

#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::runtime::{AnyObject, Bool};
    use objc2::{class, msg_send};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    let mode = std::env::args().nth(1).unwrap_or_else(|| "measured".into());

    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let _: Bool = msg_send![app, setActivationPolicy: 1isize];
        let _: () = msg_send![app, finishLaunching];

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
        let _: () = msg_send![win, setTitleVisibility: 1isize];
        let _: () = msg_send![win, makeKeyAndOrderFront: std::ptr::null_mut::<AnyObject>()];
        let spin = |secs: f64| {
            let run: *mut AnyObject = msg_send![class!(NSRunLoop), currentRunLoop];
            let until: *mut AnyObject =
                msg_send![class!(NSDate), dateWithTimeIntervalSinceNow: secs];
            let _: () = msg_send![run, runUntilDate: until];
        };
        spin(0.4);

        let Some(lights) = traffic::measure() else {
            println!("{{\"skipped\":\"no window buttons\"}}");
            return;
        };
        let close: *mut AnyObject = msg_send![win, standardWindowButton: 0usize];
        let mini: *mut AnyObject = msg_send![win, standardWindowButton: 1usize];
        let zoom: *mut AnyObject = msg_send![win, standardWindowButton: 2usize];
        let holder: *mut AnyObject = msg_send![close, superview];
        let container: *mut AnyObject = msg_send![holder, superview];

        for row in [38.0f64, 52.0] {
            let y = if mode == "written-down" {
                12.0
            } else {
                traffic::inset_y(row, &lights)
            };
            // tao's placement, repeated: the container's height becomes
            // the button's plus `y`, its top stays at the window's top,
            // and the buttons are moved only sideways.
            let c: NSRect = msg_send![close, frame];
            let win_frame: NSRect = msg_send![win, frame];
            let mut bar: NSRect = msg_send![container, frame];
            bar.size.height = c.size.height + y;
            bar.origin.y = win_frame.size.height - bar.size.height;
            let _: () = msg_send![container, setFrame: bar];
            let space = {
                let m: NSRect = msg_send![mini, frame];
                m.origin.x - c.origin.x
            };
            for (i, b) in [close, mini, zoom].into_iter().enumerate() {
                let mut r: NSRect = msg_send![b, frame];
                r.origin.x = 20.0 + i as f64 * space;
                let _: () = msg_send![b, setFrameOrigin: r.origin];
            }
            spin(0.3);

            // Where the trio actually landed, in the window's own
            // coordinates, top down — the question the user asked.
            let in_window: NSRect = {
                let b: NSRect = msg_send![close, bounds];
                msg_send![close, convertRect: b, toView: std::ptr::null_mut::<AnyObject>()]
            };
            let content: *mut AnyObject = msg_send![win, contentView];
            let content_box: NSRect = msg_send![content, bounds];
            let centre =
                content_box.size.height - (in_window.origin.y + in_window.size.height / 2.0);
            let after: NSRect = msg_send![mini, frame];
            let close_now: NSRect = msg_send![close, frame];
            println!(
                "{{\"row\":{row},\"centre\":{:.2},\"spacing\":{:.2},\"button\":{:.2},\"y\":{y:.2},\"room\":{:.2}}}",
                centre,
                after.origin.x - close_now.origin.x,
                close_now.size.height,
                // What the row must keep clear for the trio, by the same
                // measurement — the face's `--traffic` (`traffic_room`).
                traffic::room(20.0, 12.0, &lights)
            );
        }
        let _: () = msg_send![win, close];
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("{{\"skipped\":\"not macOS\"}}");
}
