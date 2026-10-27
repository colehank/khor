//! The room a column opens beside the page, and giving it back exactly
//! (UX-39 §9 → UX-46 §2).
//!
//! 「右上角的icon,都是点击后,在最右侧,拓宽窗口, 而不是挤占现有窗口」 — the
//! user, 2026-09-16; then the owner, 2026-10-07 17:20: 「打开多大，关闭了不
//! 影响原来的大小是要做好的…本来的窗口还变了」. Two columns ask for room
//! this way: the session's 附栏 (`App`) and the workspace's sessions
//! (`WorkspacePane`).
//!
//! # Why it is a record and not a delta
//!
//! Until 2026-10-07 the face sent `+w` on the press and `-w` on the close
//! (`widen(delta)`), and the window was right only when both landed in
//! full on a window nobody touched in between. It came back wrong three
//! ways, all read off the code: a window near the screen's right edge
//! grew by less than `w` (the ceiling) and then gave back all of `w`, so
//! every open and close left it narrower; a column that went away with
//! its page (leaving the workspace, the 附栏 left standing on another
//! landing) never asked for the `-w` at all, so the window stayed grown;
//! and the two sides' numbers could disagree whenever the face's width
//! for the column changed between the press and the close.
//!
//! So the window, not the face, keeps the account: [`grow`] notes the
//! frame before (`from`) and after (`to`) and what each edge gave, and
//! [`give_back`] answers the close from that note — `from` exactly when
//! the window still stands at `to`; the size it had at the press, where
//! it stands now, when the person only moved it while the column was
//! open; and nothing at all when they resized it (the lead's ruling
//! 2026-10-07 on UX-46: a size somebody chose is theirs — the narrow
//! layout included, where taking the column off a window they had just
//! made narrow would shrink it under their hand).
//!
//! # Which way it grows
//!
//! **To the right, then to the left for what the right edge could not
//! give** (UX-46 §2: 「a window that could not grow rightward … and grew
//! leftward shrinks back the same way」). **A column with a floor takes
//! less before it moves the window** (sb-a, 2026-10-09, the owner: 「侧栏
//! 展开时, 原来的页面位置不要动, 侧栏展开自己额外扩展出去」): the 附栏
//! asks for its width and the least it can live with; where the right edge
//! has at least that, the column is what the right edge has and the window
//! stays where it is; only below it does the window move left, and then by
//! what the floor still needs. The answer is the width the column got. ⚠️ 墓碑 (2026-09-16 → 10-07):
//! 「The origin is left alone」 — a window against the screen's right
//! edge kept its size and the column took its room off the page, which
//! is still the answer when neither edge has room. Fullscreen and
//! maximised windows never grow: the column takes its room inside.
//!
//! Physical pixels throughout, the window manager's own unit; the one
//! number from the face (the column's CSS width, logical) is scaled at
//! the door, and what each edge gave is kept logical so a window dragged
//! to a screen of another scale still gives back the column's width.

/// A window's outer frame, physical pixels, origin top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Frame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Frame {
    /// The same frame, give or take a pixel of rounding — what a window
    /// nobody touched reads back as.
    fn same(&self, o: &Frame) -> bool {
        (self.x - o.x).abs() < 1.5
            && (self.y - o.y).abs() < 1.5
            && (self.w - o.w).abs() < 1.5
            && (self.h - o.h).abs() < 1.5
    }
}

/// One column's room, as the window took it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Grown {
    /// The frame at the press.
    pub from: Frame,
    /// The frame the window stood at once it had grown.
    pub to: Frame,
    /// What the right edge gave, logical pixels.
    pub right: f64,
    /// What the left edge gave, logical pixels.
    pub left: f64,
    /// The scale `to` was read at, so a window dragged to a screen of
    /// another scale is not taken for a resized one.
    pub scale: f64,
}

/// The frame a window at `from` grows to for a column `want` physical
/// pixels wide that can live with `least` (`least >= want`: no give),
/// inside the screen's work area `area` (also a frame). `None` = no room
/// on either side, nothing to do.
pub(crate) fn grow(from: Frame, area: Frame, want: f64, least: f64, scale: f64) -> Option<Grown> {
    if want <= 0.0 || scale <= 0.0 {
        return None;
    }
    let least = least.clamp(0.0, want);
    let room = (area.x + area.w - (from.x + from.w)).max(0.0);
    let right = want.min(room);
    let left = if right >= least {
        0.0
    } else {
        (least - right).min((from.x - area.x).max(0.0))
    };
    if right + left < 1.0 {
        return None;
    }
    let to = Frame {
        x: from.x - left,
        y: from.y,
        w: from.w + right + left,
        h: from.h,
    };
    Some(Grown {
        from,
        to,
        right: right / scale,
        left: left / scale,
        scale,
    })
}

/// Where a window standing at `now` goes when the column closes; `None`
/// leaves it where and as it is.
pub(crate) fn give_back(g: &Grown, now: Frame, scale: f64) -> Option<Frame> {
    if now.same(&g.to) {
        return Some(g.from);
    }
    // Compared in logical pixels: the same window on a screen of another
    // scale has another physical size and is not resized.
    let resized = (now.w / scale - g.to.w / g.scale).abs() >= 1.0
        || (now.h / scale - g.to.h / g.scale).abs() >= 1.0;
    if resized {
        return None;
    }
    // Only moved: their place, less the column, off the edges it was added
    // at — the size it had at the press.
    let left = g.left * scale;
    let right = g.right * scale;
    Some(Frame {
        x: now.x + left,
        y: now.y,
        w: (now.w - left - right).max(1.0),
        h: now.h,
    })
}

/// The notes, by window label and column. Two columns may hold room at
/// once (a window grown for one, then the other); each answers its own
/// close.
fn notes() -> &'static std::sync::Mutex<std::collections::HashMap<(String, String), Grown>> {
    static NOTES: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<(String, String), Grown>>,
    > = std::sync::OnceLock::new();
    NOTES.get_or_init(Default::default)
}

fn frame_of(window: &tauri::Window) -> Result<Frame, String> {
    let at = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    Ok(Frame {
        x: at.x as f64,
        y: at.y as f64,
        w: size.width as f64,
        h: size.height as f64,
    })
}

/// Puts the window's outer frame at `to`, and answers whether it is
/// there **now** — whether a [`frame_of`] read straight after sees it.
///
/// **On the Mac, in one synchronous `setFrame:display:`** (fix-c,
/// 2026-10-08, the owner: 「点击收起的时候，原窗口宽度和侧栏宽度一起没
/// 变」). tauri's `set_size` and `set_position` reach tao, which hands
/// both to `dispatch_async` on the main queue — and a command without
/// `async` already runs on the main thread, so neither had happened when
/// [`widen`] read the frame back to note where the window stood. The
/// note said `to` = `from`; the close found the window wider than that,
/// took it for one the person had resized (their size stands), and left
/// it grown with the session stretched across the column's room. Every
/// open, every road. One `setFrame:` also moves and sizes the window as
/// one change, where the two queued calls were two.
///
/// Elsewhere (and off the main thread) tauri's two calls, which answer
/// `false`: the caller notes the frame it asked for rather than a read
/// that may still be the old one.
fn place(window: &tauri::Window, to: Frame) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    if place_now(window, to)? {
        return Ok(true);
    }
    place_queued(window, to)?;
    Ok(false)
}

/// `setFrame:display:` on the window's NSWindow, on the main thread or
/// not at all (`false`). AppKit's frame is in points from the bottom of
/// the primary screen; the top it is measured from is read off this
/// window's own two frames (tao's top-left one and AppKit's), so the
/// conversion is tao's own and no screen is asked.
#[cfg(target_os = "macos")]
fn place_now(window: &tauri::Window, to: Frame) -> Result<bool, String> {
    use objc2::runtime::AnyObject;
    use objc2::{MainThreadMarker, msg_send};
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    if MainThreadMarker::new().is_none() {
        return Ok(false);
    }
    let ns = window.ns_window().map_err(|e| e.to_string())? as *mut AnyObject;
    if ns.is_null() {
        return Ok(false);
    }
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let at = window.outer_position().map_err(|e| e.to_string())?;
    // SAFETY: a live NSWindow (tauri's), messaged on the main thread.
    unsafe {
        let now: NSRect = msg_send![ns, frame];
        let top = at.y as f64 / scale + now.origin.y + now.size.height;
        let h = to.h / scale;
        let r = NSRect::new(
            NSPoint::new(to.x / scale, top - to.y / scale - h),
            NSSize::new(to.w / scale, h),
        );
        let _: () = msg_send![ns, setFrame: r, display: true];
    }
    Ok(true)
}

/// tauri sets the **inner** size, so the frame's own margins (the title
/// bar where the content does not run under it) are taken off first —
/// zero on the Mac's overlay title bar, measured 2026-09-09 (`PageBox`'s
/// `view`), and not on every platform.
fn place_queued(window: &tauri::Window, to: Frame) -> Result<(), String> {
    use tauri::{PhysicalPosition, PhysicalSize};
    let outer = window.outer_size().map_err(|e| e.to_string())?;
    let inner = window.inner_size().map_err(|e| e.to_string())?;
    let dw = outer.width as f64 - inner.width as f64;
    let dh = outer.height as f64 - inner.height as f64;
    window
        .set_size(PhysicalSize::new(
            (to.w - dw).max(1.0).round() as u32,
            (to.h - dh).max(1.0).round() as u32,
        ))
        .map_err(|e| e.to_string())?;
    window
        .set_position(PhysicalPosition::new(
            to.x.round() as i32,
            to.y.round() as i32,
        ))
        .map_err(|e| e.to_string())
}

/// Grows the window for `column`, `w` logical pixels wide (lib.rs's
/// `widen` command). Answers the logical width it grew by — 0 when it
/// could not (fullscreen, maximised, no room at either edge, or already
/// grown for this column), and the column then takes its room off the
/// page.
/// `least`: the narrowest the column may take before the window moves
/// left (`None` = the whole `w`).
pub(crate) fn widen(
    window: &tauri::Window,
    column: String,
    w: f64,
    least: Option<f64>,
) -> Result<f64, String> {
    let key = (window.label().to_string(), column);
    if notes()
        .lock()
        .map_err(|e| e.to_string())?
        .contains_key(&key)
    {
        return Ok(0.0);
    }
    if window.is_fullscreen().unwrap_or(false) || window.is_maximized().unwrap_or(false) {
        return Ok(0.0);
    }
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let from = frame_of(window)?;
    // `current_monitor` answers `None` on a window the platform has not
    // placed yet: then there is no edge to stop at, only the right one
    // to ask of, and the window manager is the one that says no.
    let area = match window.current_monitor().map_err(|e| e.to_string())? {
        Some(mon) => {
            let a = mon.work_area();
            Frame {
                x: a.position.x as f64,
                y: a.position.y as f64,
                w: a.size.width as f64,
                h: a.size.height as f64,
            }
        }
        None => Frame {
            x: from.x,
            y: from.y,
            w: from.w + w * scale,
            h: from.h,
        },
    };
    let Some(mut g) = grow(from, area, w * scale, least.unwrap_or(w) * scale, scale) else {
        return Ok(0.0);
    };
    // What the window really stands at, where it is there to be read:
    // the window manager may round or clamp, and the close compares
    // against this. Where the move is still queued a read is the old
    // frame (`place`), and the frame asked for is the better note.
    if place(window, g.to)?
        && let Ok(now) = frame_of(window)
    {
        g.to = now;
    }
    let grew = g.right + g.left;
    notes().lock().map_err(|e| e.to_string())?.insert(key, g);
    Ok(grew)
}

/// Gives back what [`widen`] took for `column` (lib.rs's `unwiden`).
/// Answers the logical width given back — 0 when nothing was taken, when
/// the person resized the window meanwhile (their size stands), or when
/// it went fullscreen meanwhile: leaving fullscreen puts back a frame of
/// AppKit's keeping, and the note is dropped rather than applied to it.
pub(crate) fn unwiden(window: &tauri::Window, column: String) -> Result<f64, String> {
    let key = (window.label().to_string(), column);
    let Some(g) = notes().lock().map_err(|e| e.to_string())?.remove(&key) else {
        return Ok(0.0);
    };
    if window.is_fullscreen().unwrap_or(false) {
        return Ok(0.0);
    }
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let now = frame_of(window)?;
    let Some(to) = give_back(&g, now, scale) else {
        return Ok(0.0);
    };
    place(window, to)?;
    Ok(g.right + g.left)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1512×945-point screen at 2×, the menu bar's 37 points off the top.
    const AREA: Frame = Frame {
        x: 0.0,
        y: 74.0,
        w: 3024.0,
        h: 1816.0,
    };
    const S: f64 = 2.0;

    #[test]
    fn grows_rightward_when_there_is_room() {
        let from = Frame {
            x: 100.0,
            y: 200.0,
            w: 2000.0,
            h: 1440.0,
        };
        let g = grow(from, AREA, 780.0, 780.0, S).unwrap();
        assert_eq!(
            g.to,
            Frame {
                x: 100.0,
                y: 200.0,
                w: 2780.0,
                h: 1440.0
            }
        );
        assert_eq!((g.right, g.left), (390.0, 0.0));
    }

    #[test]
    fn grows_leftward_for_what_the_right_edge_cannot_give() {
        // 64 physical pixels short of the right edge.
        let from = Frame {
            x: 800.0,
            y: 200.0,
            w: 2160.0,
            h: 1440.0,
        };
        let g = grow(from, AREA, 780.0, 780.0, S).unwrap();
        assert_eq!(
            g.to,
            Frame {
                x: 84.0,
                y: 200.0,
                w: 2940.0,
                h: 1440.0
            }
        );
        assert_eq!((g.right, g.left), (32.0, 358.0));
    }

    #[test]
    fn against_both_edges_it_grows_by_what_there_is() {
        let from = Frame {
            x: 100.0,
            y: 200.0,
            w: 2824.0,
            h: 1440.0,
        };
        let g = grow(from, AREA, 780.0, 780.0, S).unwrap();
        assert_eq!(
            g.to,
            Frame {
                x: 0.0,
                y: 200.0,
                w: 3024.0,
                h: 1440.0
            }
        );
        assert_eq!((g.right, g.left), (50.0, 50.0));
        let full = Frame {
            x: 0.0,
            y: 74.0,
            w: 3024.0,
            h: 1816.0,
        };
        assert_eq!(grow(full, AREA, 780.0, 780.0, S), None);
    }

    /// The bug the owner saw (「本来的窗口还变了」): near the right edge the
    /// window grew by less than the column, and the close took the whole
    /// column off. The note gives back exactly the frame it started at.
    #[test]
    fn an_untouched_window_comes_back_exactly() {
        for from in [
            Frame {
                x: 100.0,
                y: 200.0,
                w: 2000.0,
                h: 1440.0,
            },
            Frame {
                x: 800.0,
                y: 200.0,
                w: 2160.0,
                h: 1440.0,
            },
            Frame {
                x: 100.0,
                y: 200.0,
                w: 2824.0,
                h: 1440.0,
            },
        ] {
            let g = grow(from, AREA, 780.0, 780.0, S).unwrap();
            assert_eq!(give_back(&g, g.to, S), Some(from));
            // A pixel of the window manager's rounding is still untouched.
            let nudged = Frame {
                w: g.to.w + 1.0,
                ..g.to
            };
            assert_eq!(give_back(&g, nudged, S), Some(from));
        }
    }

    #[test]
    fn a_moved_window_keeps_its_move_and_gets_its_size_back() {
        let from = Frame {
            x: 800.0,
            y: 200.0,
            w: 2160.0,
            h: 1440.0,
        };
        let g = grow(from, AREA, 780.0, 780.0, S).unwrap();
        let moved = Frame {
            x: g.to.x - 40.0,
            y: g.to.y + 100.0,
            ..g.to
        };
        assert_eq!(
            give_back(&g, moved, S),
            Some(Frame {
                x: 760.0,
                y: 300.0,
                w: 2160.0,
                h: 1440.0
            })
        );
    }

    /// The lead's ruling (2026-10-07): a size somebody chose while the
    /// column was open is theirs — wider, narrower (the narrow layout),
    /// or only taller.
    #[test]
    fn a_resized_window_is_left_alone() {
        let from = Frame {
            x: 800.0,
            y: 200.0,
            w: 2160.0,
            h: 1440.0,
        };
        let g = grow(from, AREA, 780.0, 780.0, S).unwrap();
        for now in [
            Frame {
                w: g.to.w + 200.0,
                h: 1200.0,
                ..g.to
            },
            Frame { w: 1000.0, ..g.to },
            Frame {
                h: 1300.0,
                x: g.to.x + 30.0,
                ..g.to
            },
        ] {
            assert_eq!(give_back(&g, now, S), None);
        }
    }

    #[test]
    fn the_column_comes_off_in_logical_pixels_on_another_screen() {
        let from = Frame {
            x: 100.0,
            y: 200.0,
            w: 2000.0,
            h: 1440.0,
        };
        let g = grow(from, AREA, 780.0, 780.0, S).unwrap();
        // Dragged to a 1× screen: the same 1390×720 points, half the pixels.
        let there = Frame {
            x: 3100.0,
            y: 100.0,
            w: 1390.0,
            h: 720.0,
        };
        assert_eq!(
            give_back(&g, there, 1.0),
            Some(Frame {
                x: 3100.0,
                y: 100.0,
                w: 1000.0,
                h: 720.0
            })
        );
    }

    /// sb-a (2026-10-09): a column with a floor. 1280 points wanted, 720
    /// the least (640 and 360 logical at 2×).
    #[test]
    fn a_column_with_a_floor_takes_the_right_edge_first() {
        // Room for all of it: all of it, rightward.
        let from = Frame {
            x: 100.0,
            y: 200.0,
            w: 1400.0,
            h: 1440.0,
        };
        let g = grow(from, AREA, 1280.0, 720.0, S).unwrap();
        assert_eq!((g.right, g.left), (640.0, 0.0));
        // 1000 physical to the right edge: the column shrinks to it, the
        // window does not move.
        let from = Frame {
            x: 100.0,
            y: 200.0,
            w: 1924.0,
            h: 1440.0,
        };
        let g = grow(from, AREA, 1280.0, 720.0, S).unwrap();
        assert_eq!(
            g.to,
            Frame {
                x: 100.0,
                y: 200.0,
                w: 2924.0,
                h: 1440.0
            }
        );
        assert_eq!((g.right, g.left), (500.0, 0.0));
        // 400 to the right edge: below the floor, so the window moves left
        // by what the floor still needs, and no further.
        let from = Frame {
            x: 800.0,
            y: 200.0,
            w: 1824.0,
            h: 1440.0,
        };
        let g = grow(from, AREA, 1280.0, 720.0, S).unwrap();
        assert_eq!(
            g.to,
            Frame {
                x: 480.0,
                y: 200.0,
                w: 2544.0,
                h: 1440.0
            }
        );
        assert_eq!((g.right, g.left), (200.0, 160.0));
        assert_eq!(give_back(&g, g.to, S), Some(from));
    }
}
