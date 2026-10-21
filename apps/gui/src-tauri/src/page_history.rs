//! Native session-history availability. A JavaScript history length cannot
//! distinguish entries behind the current one from entries ahead of it.
//! Read the engine on its owning thread, never a page-provided estimate.

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub(crate) struct PageSteps {
    pub back: bool,
    pub forward: bool,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) unsafe fn webkit(view: *mut objc2::runtime::AnyObject) -> PageSteps {
    // SAFETY: callers provide a live WKWebView on the main thread.
    unsafe {
        PageSteps {
            back: objc2::msg_send![view, canGoBack],
            forward: objc2::msg_send![view, canGoForward],
        }
    }
}

#[cfg(desktop)]
pub(crate) fn desktop(view: tauri::webview::PlatformWebview) -> Result<PageSteps, String> {
    #[cfg(target_os = "macos")]
    {
        let raw = view.inner() as *mut objc2::runtime::AnyObject;
        if raw.is_null() { return Err("page has no native webview".into()); }
        // SAFETY: with_webview hands over the retained WKWebView on main.
        Ok(unsafe { webkit(raw) })
    }
    #[cfg(target_os = "windows")]
    {
        // SAFETY: with_webview executes on the WebView2 controller's
        // owning thread; the handles stay alive for this query.
        unsafe {
            let core = view.controller().CoreWebView2().map_err(|e| e.to_string())?;
            let mut back = Default::default();
            let mut forward = Default::default();
            core.CanGoBack(&mut back).map_err(|e| e.to_string())?;
            core.CanGoForward(&mut forward).map_err(|e| e.to_string())?;
            Ok(PageSteps { back: back.as_bool(), forward: forward.as_bool() })
        }
    }
    #[cfg(target_os = "linux")]
    {
        use webkit2gtk::WebViewExt;
        let native = view.inner();
        Ok(PageSteps { back: native.can_go_back(), forward: native.can_go_forward() })
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    { let _ = view; Err("native page history is unavailable on this platform".into()) }
}
