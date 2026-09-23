//! Where macOS's three window buttons sit in the left block's first row
//! (docs/SHELL.md 第十一轮, 第十三轮), **measured rather than written
//! down** — and the three numbers the row is drawn from, written here
//! once for the one side that cannot read a token.
//!
//! # What `trafficLightPosition`'s `y` actually means
//!
//! Not 「how far from the top」. tao (`view.rs::inset_traffic_lights`)
//! sets the system title bar container's height to `button height + y`
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
//! user saw). The row is `--pane-head` (48) since 第十三轮 — the left
//! block's first row, the same token the tab band wears — and the
//! arithmetic is unchanged.
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

/// **How tall the left block's first row is** (docs/SHELL.md 第十三轮 第
/// 1 条), duplicated here from `src/styles/tokens.css`'s `--pane-head`
/// (`--touch` + `--tab-lip`) for the one thing CSS cannot do: tell macOS
/// where to put the three system buttons so they sit on this row's
/// centre. `check-boot.mjs` reads both and refuses a disagreement — the
/// same arrangement the window's two grounds already have.
pub const PANE_HEAD: f64 = 48.0;

/// **Where the first button's left edge goes**: the rail's own glyph
/// box — a `--rail-item` box centred in `--rail-w`, `(64 − 48) / 2` (第
/// 十三轮 第 2 条:「三颗按钮的第一颗左边缘 = 竖栏图标框的左边缘」). Read
/// against `tokens.css` by `check-boot.mjs`.
pub const X: f64 = 8.0;

/// The gap after the trio before the row draws the search box —
/// `--gap-in`, the distance between the parts of one thing. Read against
/// `tokens.css` by `check-boot.mjs`.
pub const GAP: f64 = 8.0;

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
    /// A button's own offset from the bottom of the title bar container,
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
    /// bottom; on the 48 row (`PANE_HEAD`) that makes 26 the answer, for
    /// a centre of 24 — and `12`, which is what the config said, puts
    /// that centre at 10, fourteen points high (docs/SHELL.md 第十一轮's
    /// reading on the 38 row it had then; the same numbers `npm run wk`
    /// reads off a real window).
    #[test]
    fn the_inset_puts_the_trio_on_the_rows_centre() {
        let now = Lights {
            button: 14.0,
            inside: 9.0,
            span: 60.0,
        };
        let centre = |y: f64| y - now.inside + now.button / 2.0;
        assert_eq!(inset_y(PANE_HEAD, &now), 26.0);
        assert_eq!(centre(inset_y(PANE_HEAD, &now)), 24.0);
        // What `12` produced.
        assert_eq!(centre(12.0), 10.0);
        // A taller row moves it, and an older SDK's smaller button does
        // too — which is the whole reason this is not a constant.
        assert_eq!(centre(inset_y(60.0, &now)), 30.0);
        let older = Lights {
            button: 12.0,
            inside: 6.0,
            span: 52.0,
        };
        let older_centre = |y: f64| y - older.inside + older.button / 2.0;
        assert_eq!(older_centre(inset_y(PANE_HEAD, &older)), 24.0);
        // The room the row keeps clear: from the rail's glyph box, the
        // trio, and one gap — where the search box starts.
        assert_eq!(room(X, GAP, &older), 68.0);
        assert_eq!(room(X, GAP, &now), 76.0);
    }
}
