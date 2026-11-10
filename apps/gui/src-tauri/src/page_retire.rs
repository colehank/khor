//! Destroy a document's activity before relinquishing a native page.
//! Hiding a tab deliberately does not use this: only close/replacement does.
use objc2::{msg_send, runtime::AnyObject};
use objc2_foundation::NSString;

/// # Safety
/// `view` is a live WKWebView, called on its platform's main thread.
pub unsafe fn stop(view: &AnyObject) {
    unsafe {
        let _: () = msg_send![view, stopLoading];
        // WebKit may retain a detached view for an outstanding callback or
        // media presentation. Removing its superview alone does not stop it.
        let can_pause: bool = msg_send![view, respondsToSelector: objc2::sel!(pauseAllMediaPlaybackWithCompletionHandler:)];
        if can_pause {
            let _: () = msg_send![view, pauseAllMediaPlaybackWithCompletionHandler: std::ptr::null::<AnyObject>()];
        }
        let _: () = msg_send![view, setNavigationDelegate: std::ptr::null::<AnyObject>()];
        let _: () = msg_send![view, setUIDelegate: std::ptr::null::<AnyObject>()];
        // Replacing the document also ends script/audio contexts and players
        // inside cross-origin frames. Keep the data store/cookies untouched.
        let blank = NSString::from_str("");
        let _: *mut AnyObject =
            msg_send![view, loadHTMLString: &*blank, baseURL: std::ptr::null::<AnyObject>()];
    }
}
