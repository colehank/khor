//! Allocation of a vertical pan between a sheet and its page, in points.
//! Up expands first. Down scrolls to the top before shrinking the sheet.
/// The unconsumed part of a native pan when it first reaches the sheet.
/// `before` and `now` are offsets from the scroll view's top inset.
pub fn handoff(height: f64, full: f64, up: f64, before: f64, now: f64) -> Option<f64> {
    if up > 0.0 {
        return (height < full - 1.0).then_some(up);
    }
    if up < 0.0 && now <= 1.0 {
        // UIKit has already scrolled this part of the move. Do not spend
        // it a second time shrinking the sheet at the boundary.
        let left = up + before.max(0.0).min(-up);
        return (left < 0.0).then_some(left);
    }
    None
}

pub fn travel(height: f64, scroll: f64, up: f64, full: f64, max_scroll: f64) -> (f64, f64) {
    let mut h = height.clamp(0.0, full);
    let mut y = scroll.clamp(0.0, max_scroll);
    if up >= 0.0 {
        let grow = up.min(full - h);
        h += grow;
        y = (y + up - grow).min(max_scroll);
    } else {
        let back = (-up).min(y);
        y -= back;
        h = (h + up + back).max(0.0);
    }
    (h, y)
}
#[cfg(test)]
mod tests {
    use super::{handoff, travel};
    #[test]
    fn crossing_scroll_top_only_hands_over_unused_travel() {
        // This move spent 12 points getting the page to its top; only
        // the remaining 8 points may shrink the sheet.
        assert_eq!(handoff(700.0, 700.0, -20.0, 12.0, 0.0), Some(-8.0));
        assert_eq!(handoff(700.0, 700.0, -4.0, 0.0, 0.0), Some(-4.0));
        assert_eq!(handoff(700.0, 700.0, -12.0, 12.0, 0.0), None);
        assert_eq!(handoff(700.0, 700.0, -8.0, 20.0, 12.0), None);
        assert_eq!(handoff(400.0, 700.0, 8.0, 120.0, 128.0), Some(8.0));
        assert_eq!(handoff(700.0, 700.0, 8.0, 0.0, 8.0), None);
    }
    #[test]
    fn expands_then_scrolls_and_reverses_without_a_jump() {
        assert_eq!(travel(400.0, 0.0, 350.0, 700.0, 1000.0), (700.0, 50.0));
        assert_eq!(travel(700.0, 50.0, -80.0, 700.0, 1000.0), (670.0, 0.0));
    }
    #[test]
    fn preserves_existing_position_while_growing() {
        assert_eq!(travel(400.0, 120.0, 100.0, 700.0, 1000.0), (500.0, 120.0));
        assert_eq!(travel(700.0, 120.0, -90.0, 700.0, 1000.0), (700.0, 30.0));
    }
    #[test]
    fn short_pages_and_extreme_pulls_are_bounded() {
        assert_eq!(travel(400.0, 0.0, 2000.0, 700.0, 0.0), (700.0, 0.0));
        assert_eq!(travel(400.0, 0.0, -2000.0, 700.0, 0.0), (0.0, 0.0));
    }
}
