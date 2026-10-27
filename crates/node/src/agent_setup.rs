//! Setup reply states are independent of native process retirement.

pub(crate) fn is_terminal(status: Option<&str>) -> bool {
    matches!(status, Some("done" | "failed" | "cancelled"))
}
