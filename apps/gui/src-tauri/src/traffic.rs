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
//! user saw). The row is `--pane-head` since 第十三轮 — the left
//! block's first row, the same token the tab band wears: 48 until
//! 2026-09-29, **38 since** (macOS's compact toolbar, 「整行收紧到 38」),
//! which puts the centre back at 19 and `y` at 21 on this Mac. The
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
//!
//! # Why it is kept on disk
//!
//! Measuring builds and closes a window, and on the launch path that was
//! about 30 ms before khor's own window could be built (UX-8's marks; up
//! to 170 in UX-7's). The answer depends only on the system and the
//! build, so it is kept in the store (`lights`), keyed by both, and
//! measured again only when either changes. After the window is built,
//! `check` reads the real window's buttons — no window of its own — and
//! rewrites what is kept if it ever disagrees, so a wrong copy lives one
//! launch at most.

/// **How tall the left block's first row is** (52 at every pointer since
/// the redesign, 2026-10-02; macOS's compact 38 before it), duplicated
/// here from `src/styles/tokens.css`'s `--pane-head` for the one thing
/// CSS cannot do: tell macOS where to put the three system buttons so
/// they sit on this row's centre — the centreline the landing strip,
/// the title and the tabs share. `check-boot.mjs` reads both and refuses
/// a disagreement — the same arrangement the window's two grounds
/// already have.
pub const PANE_HEAD: f64 = 52.0;

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
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
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

/// What the buttons measured at startup, kept so the trio can be put
/// back after AppKit moves it (`place`).
static MEASURED: std::sync::OnceLock<Lights> = std::sync::OnceLock::new();

/// Keeps a measurement for `place`.
pub fn remember(m: Lights) {
    let _ = MEASURED.set(m);
}

/// What `remember` kept — the numbers the main window was built with.
#[cfg(target_os = "macos")]
pub fn remembered() -> Option<Lights> {
    MEASURED.get().copied()
}

/// The file the measurement is kept in between launches: beside the
/// store's other small files, so a second store (`KHOR_HOME`) keeps its
/// own and nothing here reaches outside it.
#[cfg(target_os = "macos")]
fn kept_path() -> std::path::PathBuf {
    khor_node::Node::root_from_env()
        .join(".khor")
        .join("traffic.json")
}

/// One kept measurement and what it is true of.
#[cfg(target_os = "macos")]
#[derive(serde::Serialize, serde::Deserialize)]
struct Kept {
    system: String,
    lights: Lights,
}

/// What a kept measurement must match: the system's version, which the
/// buttons' size and inset come from, and this build's, which fixes the
/// SDK it was linked against.
#[cfg(target_os = "macos")]
fn system() -> String {
    let os = objc2_foundation::NSProcessInfo::processInfo().operatingSystemVersionString();
    format!("{os} / khor {}", env!("CARGO_PKG_VERSION"))
}

/// The buttons' measurement for building the main window: **the kept one
/// when it is for this system and build**, measured (and kept) otherwise.
/// Main thread only, the way `measure` is.
#[cfg(target_os = "macos")]
pub fn lights() -> Option<Lights> {
    let system = system();
    if let Some(kept) = std::fs::read(kept_path())
        .ok()
        .and_then(|b| kept_for(&b, &system))
    {
        return Some(kept);
    }
    let lights = measure()?;
    keep(system, lights);
    Some(lights)
}

/// What a kept file says, if it was written for `system`. Anything else
/// — another system or build, a file cut short, a format from some other
/// day — is no answer, and the buttons are measured again.
#[cfg(target_os = "macos")]
fn kept_for(bytes: &[u8], system: &str) -> Option<Lights> {
    serde_json::from_slice::<Kept>(bytes)
        .ok()
        .filter(|k| k.system == system)
        .map(|k| k.lights)
}

/// Writes a measurement down. Best effort: a store that cannot take it
/// costs the next launch one more measurement, nothing else.
#[cfg(target_os = "macos")]
fn keep(system: String, lights: Lights) {
    let path = kept_path();
    if let (Some(dir), Ok(bytes)) = (path.parent(), serde_json::to_vec(&Kept { system, lights })) {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(path, bytes);
    }
}

/// **Checks the kept numbers against the real window**, once it exists:
/// reads its own three buttons (no window is built for this), and when
/// they disagree with what the window was built from, keeps the real
/// ones for the next launch — this one's buttons stay where tao put
/// them, since tao re-applies the builder's number on every draw. The
/// button's offset inside its container is AppKit's own and
/// tao's inset moves only the container and the buttons' `x`, so the
/// built window answers the same question `measure` asks. Main thread
/// only.
#[cfg(target_os = "macos")]
pub fn check(win: *mut std::ffi::c_void) {
    let Some(used) = MEASURED.get().copied() else {
        return;
    };
    let Some(real) = (unsafe { read(win as *mut AnyObject) }) else {
        return;
    };
    if real == used {
        return;
    }
    keep(system(), real);
}

/// The three numbers off a window's own buttons.
#[cfg(target_os = "macos")]
unsafe fn read(win: *mut AnyObject) -> Option<Lights> {
    unsafe {
        let close: *mut AnyObject = msg_send![win, standardWindowButton: 0usize];
        let zoom: *mut AnyObject = msg_send![win, standardWindowButton: 2usize];
        if close.is_null() || zoom.is_null() {
            return None;
        }
        let c: NSRect = msg_send![close, frame];
        let z: NSRect = msg_send![zoom, frame];
        let out = Lights {
            button: c.size.height,
            inside: c.origin.y,
            span: z.origin.x + z.size.width - c.origin.x,
        };
        (out.button > 0.0 && out.span > 0.0).then_some(out)
    }
}

/// Puts the three buttons where the builder put them: left edge at `X`,
/// centred on the first row. **Needed because AppKit lays the title bar
/// out again whenever the window's appearance changes**, and that
/// returns the buttons to its default spot; tao only re-insets them from
/// its view's `drawRect:`, which a theme change does not reach (2026-09-27,
/// the owner: 「系统的三个按钮和每个侧边导航没有对齐」). The same moves as
/// tao's `inset_traffic_lights`. Main thread only.
#[cfg(target_os = "macos")]
pub fn place(win: *mut std::ffi::c_void) {
    let Some(m) = MEASURED.get() else { return };
    let (x, y) = (X, inset_y(PANE_HEAD, m));
    unsafe {
        let w = win as *mut AnyObject;
        let close: *mut AnyObject = msg_send![w, standardWindowButton: 0usize];
        let mini: *mut AnyObject = msg_send![w, standardWindowButton: 1usize];
        let zoom: *mut AnyObject = msg_send![w, standardWindowButton: 2usize];
        if close.is_null() || mini.is_null() || zoom.is_null() {
            return;
        }
        let sup: *mut AnyObject = msg_send![close, superview];
        if sup.is_null() {
            return;
        }
        let container: *mut AnyObject = msg_send![sup, superview];
        if container.is_null() {
            return;
        }
        let close_rect: NSRect = msg_send![close, frame];
        let win_frame: NSRect = msg_send![w, frame];
        let mut bar: NSRect = msg_send![container, frame];
        bar.size.height = close_rect.size.height + y;
        bar.origin.y = win_frame.size.height - bar.size.height;
        let _: () = msg_send![container, setFrame: bar];
        let mini_rect: NSRect = msg_send![mini, frame];
        let step = mini_rect.origin.x - close_rect.origin.x;
        for (i, b) in [close, mini, zoom].into_iter().enumerate() {
            let r: NSRect = msg_send![b, frame];
            let origin = NSPoint::new(x + i as f64 * step, r.origin.y);
            let _: () = msg_send![b, setFrameOrigin: origin];
        }
    }
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
        let out = read(win);
        let _: () = msg_send![win, close];
        out
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
    /// bottom; on the 52 row (`PANE_HEAD`, since the redesign) that makes
    /// 28 the answer, for a centre of 26 — and `12`, which is what the
    /// config once said, puts that centre at 10 (docs/SHELL.md round
    /// 11's reading; the same numbers `npm run wk` reads off a real
    /// window). The 38 and 48 rows it had before are kept as heights.
    #[test]
    fn the_inset_puts_the_trio_on_the_rows_centre() {
        let now = Lights {
            button: 14.0,
            inside: 9.0,
            span: 60.0,
        };
        let centre = |y: f64| y - now.inside + now.button / 2.0;
        assert_eq!(inset_y(PANE_HEAD, &now), 28.0);
        assert_eq!(centre(inset_y(PANE_HEAD, &now)), 26.0);
        assert_eq!(centre(inset_y(38.0, &now)), 19.0);
        // What `12` produced.
        assert_eq!(centre(12.0), 10.0);
        // A taller row moves it — the 48 this row was until 2026-09-29,
        // and 60 — and an older SDK's smaller button does too, which is
        // the whole reason this is not a constant.
        assert_eq!(inset_y(48.0, &now), 26.0);
        assert_eq!(centre(inset_y(48.0, &now)), 24.0);
        assert_eq!(centre(inset_y(60.0, &now)), 30.0);
        let older = Lights {
            button: 12.0,
            inside: 6.0,
            span: 52.0,
        };
        let older_centre = |y: f64| y - older.inside + older.button / 2.0;
        assert_eq!(older_centre(inset_y(PANE_HEAD, &older)), 26.0);
        // The room the row keeps clear: the inset, the trio, and one gap
        // — where the landing strip starts, before x 78.
        assert_eq!(room(X, GAP, &older), 68.0);
        assert_eq!(room(X, GAP, &now), 76.0);
    }

    /// **A kept measurement is used only on the system and build it was
    /// taken on** — a macOS update or a new khor measures again — and a
    /// file that does not read is no answer rather than a wrong one.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_kept_measurement_holds_for_its_own_system_only() {
        let lights = Lights {
            button: 14.0,
            inside: 9.0,
            span: 60.0,
        };
        let here = "Version 26.0 / khor 0.1.14";
        let bytes = serde_json::to_vec(&Kept {
            system: here.into(),
            lights,
        })
        .unwrap();
        assert_eq!(kept_for(&bytes, here), Some(lights));
        assert_eq!(kept_for(&bytes, "Version 26.1 / khor 0.1.14"), None);
        assert_eq!(kept_for(&bytes, "Version 26.0 / khor 0.1.15"), None);
        assert_eq!(kept_for(&bytes[..bytes.len() / 2], here), None);
        assert_eq!(kept_for(b"", here), None);
    }
}
