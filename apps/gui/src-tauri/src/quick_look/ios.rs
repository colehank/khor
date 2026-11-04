use objc2::{
    ClassType, class, define_class, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, NSObject},
};
use objc2_foundation::NSString;
use std::cell::RefCell;
use khor_gui_core::file_handoff::ReadyFile;
#[link(name = "QuickLook", kind = "framework")]
unsafe extern "C" {
    #[link_name = "OBJC_CLASS_$_QLPreviewController"]
    static PREVIEW: AnyClass;
}
struct Held {
    controller: Retained<AnyObject>,
    _file: ReadyFile,
    _delegate: Retained<PreviewDelegate>,
    url: Retained<AnyObject>,
    reply: tokio::sync::oneshot::Sender<Result<bool, String>>,
}
thread_local! { static HELD:RefCell<Option<Held>>=const {RefCell::new(None)}; }
fn owns(controller:*mut AnyObject,held:&Held)->bool {std::ptr::eq(controller,&*held.controller)}
fn finish(controller:*mut AnyObject,result:Result<bool,String>) {
    let held=HELD.with(|held| {
        let mut held=held.borrow_mut();
        if held.as_ref().is_some_and(|held|owns(controller,held)){held.take()}else{None}
    });
    if let Some(held)=held {let _=held.reply.send(result);}
}
define_class!(
    #[unsafe(super(NSObject))]
    #[name="KhorFilePreviewDelegate"]
    struct PreviewDelegate;
    impl PreviewDelegate {
        #[unsafe(method(numberOfPreviewItemsInPreviewController:))]
        fn count(&self,controller:*mut AnyObject)->isize {HELD.with(|held|if held.borrow().as_ref().is_some_and(|held|owns(controller,held)){1}else{0})}
        #[unsafe(method(previewController:previewItemAtIndex:))]
        fn item(&self,controller:*mut AnyObject,index:isize)->*mut AnyObject {
            HELD.with(|held|held.borrow().as_ref().filter(|h|index==0&&owns(controller,h)).map(|h|&*h.url as *const AnyObject as *mut AnyObject).unwrap_or(std::ptr::null_mut()))
        }
        #[unsafe(method(previewController:editingModeForPreviewItem:))]
        fn editing(&self,_controller:*mut AnyObject,_item:*mut AnyObject)->isize {0}
        #[unsafe(method(previewControllerDidDismiss:))]
        fn dismissed(&self,controller:*mut AnyObject) {
            // Keep self alive while releasing the preview owner.
            let _keep = unsafe { Retained::retain(self as *const Self as *mut Self) };
            finish(controller,Ok(true));
        }
    }
);
pub async fn open(view: tauri::Webview, file: ReadyFile) -> Result<bool, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    view.with_webview(move |webview| {
        let fail=||khor_catalog::msg::file_preview_unavailable().to_owned();
        if file.is_cancelled(){let _=tx.send(Err(khor_node::drops::cancelled()));return;}
        if HELD.with(|h|h.borrow().is_some()){let _=tx.send(Err(fail()));return;}
        // SAFETY: with_webview runs on the main thread. All retained
        // UIKit objects and the data source remain here until dismissal.
        unsafe {
            let wk=webview.inner() as *mut AnyObject;
            let Some(top)=crate::phone_presentation::presenter(&*wk) else{let _=tx.send(Err(fail()));return;};
            let path=file.path();
            let text=NSString::from_str(&path.to_string_lossy());
            let url:Retained<AnyObject>=msg_send![class!(NSURL),fileURLWithPath:&*text,isDirectory:false];
            let can:bool=msg_send![&PREVIEW,canPreviewItem:&*url];
            if !can {let _=tx.send(Err(fail()));return;}
            let controller:Retained<AnyObject>=msg_send![&PREVIEW,new];
            let delegate:Retained<PreviewDelegate>=msg_send![PreviewDelegate::class(),new];
            let _:()=msg_send![&*controller,setDataSource:&*delegate];
            let _:()=msg_send![&*controller,setDelegate:&*delegate];
            // Present the system preview; no editor or write callbacks.
            let controller_ptr=&*controller as *const AnyObject as *mut AnyObject;
            HELD.with(|held|*held.borrow_mut()=Some(Held{controller,_file:file,_delegate:delegate,url,reply:tx}));
            if !crate::phone_presentation::present(&top,&*controller_ptr){finish(controller_ptr,Err(fail()));}
        }
    }).map_err(|e|e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}
