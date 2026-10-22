use objc2::{
    ClassType, class, define_class, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, NSObject},
};
use objc2_foundation::NSString;
use std::{cell::RefCell, path::PathBuf};
#[link(name = "QuickLook", kind = "framework")]
unsafe extern "C" {
    #[link_name = "OBJC_CLASS_$_QLPreviewController"]
    static PREVIEW: AnyClass;
}
struct Held {
    _controller: Retained<AnyObject>,
    _delegate: Retained<PreviewDelegate>,
    url: Retained<AnyObject>,
    reply: tokio::sync::oneshot::Sender<Result<bool, String>>,
}
thread_local! { static HELD:RefCell<Option<Held>>=const {RefCell::new(None)}; }
define_class!(
    #[unsafe(super(NSObject))]
    #[name="KhorFilePreviewDelegate"]
    struct PreviewDelegate;
    impl PreviewDelegate {
        #[unsafe(method(numberOfPreviewItemsInPreviewController:))]
        fn count(&self,_controller:*mut AnyObject)->isize {1}
        #[unsafe(method(previewController:previewItemAtIndex:))]
        fn item(&self,_controller:*mut AnyObject,_index:isize)->*mut AnyObject {
            HELD.with(|held|held.borrow().as_ref().map(|h|&*h.url as *const AnyObject as *mut AnyObject).unwrap_or(std::ptr::null_mut()))
        }
        #[unsafe(method(previewController:editingModeForPreviewItem:))]
        fn editing(&self,_controller:*mut AnyObject,_item:*mut AnyObject)->isize {0}
        #[unsafe(method(previewControllerDidDismiss:))]
        fn dismissed(&self,_controller:*mut AnyObject) {
            // Keep self alive while releasing the preview owner.
            let _keep = unsafe { Retained::retain(self as *const Self as *mut Self) };
            if let Some(held)=HELD.with(|held|held.borrow_mut().take()){let _=held.reply.send(Ok(true));}
        }
    }
);
pub async fn open(view: tauri::Webview, path: PathBuf) -> Result<bool, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    view.with_webview(move |webview| {
        let fail=||khor_catalog::msg::file_preview_unavailable().to_owned();
        if HELD.with(|h|h.borrow().is_some()){let _=tx.send(Err(fail()));return;}
        // SAFETY: with_webview runs on the main thread. All retained
        // UIKit objects and the data source remain here until dismissal.
        unsafe {
            let wk=webview.inner() as *mut AnyObject;
            let window:*mut AnyObject=msg_send![wk,window];
            let mut top:*mut AnyObject=if window.is_null(){std::ptr::null_mut()}else{msg_send![window,rootViewController]};
            if top.is_null(){let _=tx.send(Err(fail()));return;}
            loop{let next:*mut AnyObject=msg_send![top,presentedViewController];if next.is_null(){break;}top=next;}
            let text=NSString::from_str(&path.to_string_lossy());
            let url:Retained<AnyObject>=msg_send![class!(NSURL),fileURLWithPath:&*text,isDirectory:false];
            let can:bool=msg_send![&PREVIEW,canPreviewItem:&*url];
            if !can {let _=tx.send(Err(fail()));return;}
            let controller:Retained<AnyObject>=msg_send![&PREVIEW,new];
            let delegate:Retained<PreviewDelegate>=msg_send![PreviewDelegate::class(),new];
            let _:()=msg_send![&*controller,setDataSource:&*delegate];
            let _:()=msg_send![&*controller,setDelegate:&*delegate];
            // Present the system preview; no editor or write callbacks.
            let controller_ptr=&*controller as *const AnyObject;
            HELD.with(|held|*held.borrow_mut()=Some(Held{_controller:controller,_delegate:delegate,url,reply:tx}));
            let _:()=msg_send![top,presentViewController:controller_ptr,animated:true,completion:std::ptr::null::<AnyObject>()];
        }
    }).map_err(|e|e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}
