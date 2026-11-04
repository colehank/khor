use objc2::{msg_send, rc::Retained, runtime::AnyObject};

/// # Safety
/// Main thread, with a live app-owned WKWebView.
pub unsafe fn presenter(view: &AnyObject) -> Option<Retained<AnyObject>> {
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        if window.is_null() || msg_send![window, isHidden] {
            return None;
        }
        let scene: *mut AnyObject = msg_send![window, windowScene];
        if !scene.is_null() {
            let state: isize = msg_send![scene, activationState];
            if state != 0 {
                return None;
            }
        }
        let root: *mut AnyObject = msg_send![window, rootViewController];
        let root = Retained::retain(root)?;
        let presented: *mut AnyObject = msg_send![&*root, presentedViewController];
        let transition: *mut AnyObject = msg_send![&*root, transitionCoordinator];
        let presenting: bool = msg_send![&*root, isBeingPresented];
        let dismissing: bool = msg_send![&*root, isBeingDismissed];
        if !presented.is_null() || !transition.is_null() || presenting || dismissing {
            return None;
        }
        Some(root)
    }
}

/// # Safety
/// Main thread, with a retained presenter and controller.
pub unsafe fn present(presenter: &AnyObject, controller: &AnyObject) -> bool {
    unsafe { present_after(presenter, controller, None) }
}

/// # Safety
/// Main thread. UIKit copies the completion block before this call returns.
pub unsafe fn present_after(
    presenter: &AnyObject,
    controller: &AnyObject,
    completion: Option<&block2::Block<dyn Fn()>>,
) -> bool {
    unsafe {
        let completion=completion.map_or(std::ptr::null::<AnyObject>(),|block|block as *const _ as *const AnyObject);
        let _: () = msg_send![presenter, presentViewController: controller, animated: true, completion: completion];
        let presented: *mut AnyObject = msg_send![presenter, presentedViewController];
        std::ptr::eq(presented, controller)
    }
}
