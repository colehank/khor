//! Native session-history queries and controls. Read the engine on its owning
//! thread and use engine commands for navigation.

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
        if raw.is_null() {
            return Err("page has no native webview".into());
        }
        // SAFETY: with_webview hands over the retained WKWebView on main.
        Ok(unsafe { webkit(raw) })
    }
    #[cfg(target_os = "windows")]
    {
        // SAFETY: with_webview executes on the WebView2 controller's
        // owning thread; the handles stay alive for this query.
        unsafe {
            let core = view
                .controller()
                .CoreWebView2()
                .map_err(|e| e.to_string())?;
            let mut back = Default::default();
            let mut forward = Default::default();
            core.CanGoBack(&mut back).map_err(|e| e.to_string())?;
            core.CanGoForward(&mut forward).map_err(|e| e.to_string())?;
            Ok(PageSteps {
                back: back.as_bool(),
                forward: forward.as_bool(),
            })
        }
    }
    #[cfg(target_os = "linux")]
    {
        use webkit2gtk::WebViewExt;
        let native = view.inner();
        Ok(PageSteps {
            back: native.can_go_back(),
            forward: native.can_go_forward(),
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = view;
        Err("native page history is unavailable on this platform".into())
    }
}

#[cfg(all(desktop, not(target_os = "macos")))]
#[derive(Clone, Copy)]
pub(crate) enum Direction {
    Back,
    Forward,
}

#[cfg(all(desktop, not(target_os = "macos")))]
pub(crate) async fn step(view: &tauri::Webview, direction: Direction) -> Result<(), String> {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        // Tauri's Webview API has no history controls yet. Dispatch to the
        // native view and carry any engine error back to the caller.
        let (tx, rx) = tokio::sync::oneshot::channel();
        view.with_webview(move |native| {
            let _ = tx.send(step_native(native, direction));
        })
        .map_err(|error| error.to_string())?;
        rx.await.map_err(|error| error.to_string())?
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let script = match direction {
            Direction::Back => "history.back()",
            Direction::Forward => "history.forward()",
        };
        view.eval(script).map_err(|error| error.to_string())
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn step_native(view: tauri::webview::PlatformWebview, direction: Direction) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // SAFETY: with_webview runs this closure on the controller's owning
        // thread, and the view and its controller are live during the call.
        unsafe {
            let core = view
                .controller()
                .CoreWebView2()
                .map_err(|error| error.to_string())?;
            let mut available = Default::default();
            match direction {
                Direction::Back => core.CanGoBack(&mut available),
                Direction::Forward => core.CanGoForward(&mut available),
            }
            .map_err(|error| error.to_string())?;
            if !available.as_bool() {
                return Ok(());
            }
            match direction {
                Direction::Back => core.GoBack(),
                Direction::Forward => core.GoForward(),
            }
            .map_err(|error| error.to_string())
        }
    }
    #[cfg(target_os = "linux")]
    {
        use webkit2gtk::WebViewExt;
        let native = view.inner();
        match direction {
            Direction::Back => native.go_back(),
            Direction::Forward => native.go_forward(),
        }
        Ok(())
    }
}
