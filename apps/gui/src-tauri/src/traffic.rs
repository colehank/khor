//! Where macOS's three window buttons sit in khor's own title row
//! (docs/SHELL.md 第十一轮), **measured rather than written down**.
//!
//! # What `trafficLightPosition`'s `y` actually means
//!
//! Not 「how far from the top」. tao (`view.rs::inset_traffic_lights`)
//! sets the system title-bar container's height to `button height + y`
//! and pins that container to the top of the window; the buttons keep
//! their own place **inside** the container, and AppKit's origin is at
//! the bottom. So with `b` the button's own offset from the container's
//! bottom edge:
//!
//! ```text
//! button top from window top = (bh + y) - (b + bh) = y - b
//! button centre             = y - b + bh/2
//! ```
//!
//! `y: 12` therefore put the centre at 10 on a row 38 tall, whose centre
//! is 19 — the whole trio nine points high (measured 2026-09-23 on this
//! Mac by `examples/traffic_lights.rs`: `bh` 14, `b` 9; and what the
//! user saw).
//!
//! # Why it is measured
//!
//! `bh` is the SDK's (14 on the macOS 26 SDK, 12 on older ones), `b` is
//! AppKit's, and the row's height is a token that can move. Any of the
//! three written into a config file is a number that is right until
//! somebody rebuilds with a different SDK — so khor asks the window
//! itself, once, before the main window is built, and hands tao the
//! answer. **It is still tao's mechanism**: tao re-applies this number
//! on every draw, which is what keeps the buttons in place; khor only
//! makes the number the right one.

#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2::{class, msg_send};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// What one window's three buttons are, as the system draws them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lights {
    /// One button's height — the SDK's, never khor's.
    pub button: f64,
    /// A button's own offset from the bottom of the title-bar container,
    /// which is the term `y` has to carry (see the module head).
    pub inside: f64,
    /// How wide the trio is, first button's left edge to the last one's
    /// right edge — the SDK's spacing, untouched.
    pub span: f64,
}

/// The `y` that puts the trio's centre on the centre of a row `row_h`
/// tall, given what the buttons measured.
pub fn inset_y(row_h: f64, m: &Lights) -> f64 {
    row_h / 2.0 + m.inside - m.button / 2.0
}

/// How much width the row must keep clear at its left end: the margin,
/// the trio, and a gap before whatever the row draws first.
pub fn room(x: f64, gap: f64, m: &Lights) -> f64 {
    x + m.span + gap
}

/// Measures the three buttons on a window of khor's own style, **without
/// showing it**: a window off screen, asked and closed. Done before the
/// main window exists, because the number it produces is what that
/// window is built with.
///
/// `None` when the system draws no buttons (it always does on a titled
/// window, so this is the honest answer for a future that does not).
#[cfg(target_os = "macos")]
pub fn measure() -> Option<Lights> {
    unsafe {
        // titled | closable | miniaturizable | resizable | fullSizeContentView
        let style: usize = 1 | 2 | 4 | 8 | (1 << 15);
        let rect = NSRect::new(
            NSPoint::new(-10_000.0, -10_000.0),
            NSSize::new(480.0, 320.0),
        );
        let win: *mut AnyObject = msg_send![class!(NSWindow), alloc];
        let win: *mut AnyObject = msg_send![
            win,
            initWithContentRect: rect,
            styleMask: style,
            backing: 2usize,
            defer: objc2::runtime::Bool::NO
        ];
        if win.is_null() {
            return None;
        }
        let _: () = msg_send![win, setReleasedWhenClosed: objc2::runtime::Bool::NO];
        let close: *mut AnyObject = msg_send![win, standardWindowButton: 0usize];
        let zoom: *mut AnyObject = msg_send![win, standardWindowButton: 2usize];
        if close.is_null() || zoom.is_null() {
            let _: () = msg_send![win, close];
            return None;
        }
        let c: NSRect = msg_send![close, frame];
        let z: NSRect = msg_send![zoom, frame];
        let out = Lights {
            button: c.size.height,
            inside: c.origin.y,
            span: z.origin.x + z.size.width - c.origin.x,
        };
        let _: () = msg_send![win, close];
        (out.button > 0.0 && out.span > 0.0).then_some(out)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn measure() -> Option<Lights> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The arithmetic, against the numbers that were actually wrong.**
    /// This Mac draws a 14-tall button sitting 9 above its container's
    /// bottom; on a 38 row that makes 21 the answer, for a centre of 19
    /// — and `12`, which is what the config said, puts that centre at
    /// 10, nine points high (docs/SHELL.md 第十一轮's reading; the same
    /// numbers `npm run wk` reads off a real window).
    #[test]
    fn the_inset_puts_the_trio_on_the_rows_centre() {
        let now = Lights {
            button: 14.0,
            inside: 9.0,
            span: 60.0,
        };
        let centre = |y: f64| y - now.inside + now.button / 2.0;
        assert_eq!(inset_y(38.0, &now), 21.0);
        assert_eq!(centre(inset_y(38.0, &now)), 19.0);
        // What `12` produced.
        assert_eq!(centre(12.0), 10.0);
        // A taller row moves it, and an older SDK's smaller button does
        // too — which is the whole reason this is not a constant.
        assert_eq!(centre(inset_y(48.0, &now)), 24.0);
        let older = Lights {
            button: 12.0,
            inside: 6.0,
            span: 52.0,
        };
        let older_centre = |y: f64| y - older.inside + older.button / 2.0;
        assert_eq!(older_centre(inset_y(38.0, &older)), 19.0);
        assert_eq!(room(20.0, 12.0, &older), 84.0);
    }
}
