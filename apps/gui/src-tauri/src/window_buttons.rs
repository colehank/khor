//! The three window buttons, kept inside the 64-wide rail (docs/SHELL.md
//! 插队, 2026-09-22:「左上角的系统按钮又超出侧边栏了, 注意守门」).
//!
//! **A file of its own so a real AppKit window can wear it outside the
//! app**: `examples/window_buttons.rs` includes this file by path and
//! drives it through the relayouts the gate names (`npm run wk`). The
//! app and the gate run the same lines.

use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSPoint, NSRect};

/// Where the buttons go: 14 wide at x = 7, 25, 43 — the trio ends at 57,
/// inside the rail with room on both sides. The size is the SDK's and
/// cannot be changed; the spacing is the older macOS one (18 apart), the
/// one an app linked against an older SDK still gets.
pub const XS: [f64; 3] = [7.0, 25.0, 43.0];

/// Moves each button to its place, **and does nothing to one already
/// there** — which is what lets a frame-change observer call this
/// without calling itself for ever: the move posts one more change, and
/// that one finds the button in place.
///
/// # Safety
/// `win` is a live `NSWindow`, and this runs on the main thread.
pub unsafe fn squeeze(win: *mut AnyObject) {
    // NSWindowButton: closeButton = 0, miniaturizeButton = 1, zoomButton = 2.
    for (i, x) in XS.into_iter().enumerate() {
        unsafe {
            let button: *mut AnyObject = msg_send![win, standardWindowButton: i];
            if button.is_null() {
                continue;
            }
            let frame: NSRect = msg_send![button, frame];
            if frame.origin.x == x {
                continue;
            }
            let _: () = msg_send![button, setFrameOrigin: NSPoint::new(x, frame.origin.y)];
        }
    }
}

/// **The buttons put themselves back.** Each one posts its frame changes
/// (`postsFrameChangedNotifications`), and an observer squeezes the
/// window whenever any of them moves — so whatever made AppKit lay the
/// title bar out again (a resize, a title, a toolbar, a screen, a Space,
/// or something nobody has listed yet) is answered the moment it lands.
///
/// It replaced a list of events that each squeezed once (resize, theme,
/// a child view, focus). That list is kept as insurance, but it was the
/// only road until 2026-09-22, and **a list of events is whack-a-mole**:
/// the buttons came back to the SDK's spacing on the sessions page from
/// an event the list did not have.
///
/// # Safety
/// `win` is a live `NSWindow` that outlives the app's run loop (the main
/// window does), and this runs on the main thread.
pub unsafe fn watch(win: *mut AnyObject) {
    use objc2::class;
    use objc2::runtime::Bool;
    use objc2_foundation::NSString;
    unsafe {
        let center: *mut AnyObject = msg_send![class!(NSNotificationCenter), defaultCenter];
        let name = NSString::from_str("NSViewFrameDidChangeNotification");
        for i in 0..XS.len() {
            let button: *mut AnyObject = msg_send![win, standardWindowButton: i];
            if button.is_null() {
                continue;
            }
            let _: () = msg_send![button, setPostsFrameChangedNotifications: Bool::YES];
            // **Twice, and both are needed** (measured by the gate): once
            // on the spot — delivered on the posting thread inside the
            // layout that moved the button (`queue: nil`) — and once when
            // that layout has finished, on the main run loop's next turn. AppKit
            // places the three one after another in one pass, and the
            // last one it placed (the zoom button) stayed at the SDK's
            // spacing with only the first: its move landed after the
            // squeeze its own notice started.
            let put_back = block2::RcBlock::new(move |_note: *mut AnyObject| {
                squeeze(win);
                let after = block2::RcBlock::new(move || squeeze(win));
                let main: *mut AnyObject = msg_send![class!(NSRunLoop), mainRunLoop];
                let _: () = msg_send![main, performBlock: &*after];
            });
            let token: *mut AnyObject = msg_send![
                center,
                addObserverForName: &*name,
                object: button,
                queue: std::ptr::null_mut::<AnyObject>(),
                usingBlock: &*put_back
            ];
            // For the life of the window, which is the life of the app:
            // the centre holds the token, and nothing ever removes it.
            let _ = token;
            std::mem::forget(put_back);
        }
    }
}
