//! Allocation of a vertical pan between a sheet and its page, in points.
//! Up expands first. Down scrolls to the top before shrinking the sheet.
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
    use super::travel;
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
