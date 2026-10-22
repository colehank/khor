use objc2::{
    ClassType, class, define_class, msg_send,
    rc::{Allocated, Retained},
    runtime::{AnyClass, AnyObject, NSObject},
};
use objc2_foundation::{NSRect, NSString};
use std::{cell::RefCell, path::PathBuf};
#[link(name = "QuickLookUI", kind = "framework")]
unsafe extern "C" {
    #[link_name = "OBJC_CLASS_$_QLPreviewView"]
    static PREVIEW: AnyClass;
}
struct Held {
    window: Retained<AnyObject>,
    preview: Retained<AnyObject>,
    _delegate: Retained<PreviewDelegate>,
    monitor: Retained<AnyObject>,
    reply: tokio::sync::oneshot::Sender<Result<bool, String>>,
}
thread_local! {static HELD:RefCell<Option<Held>>=const{RefCell::new(None)};}
fn front() -> Option<Retained<AnyObject>> {
    let window = HELD.with(|held| held.borrow().as_ref().map(|h| h.window.clone()))?;
    // SAFETY: called only on AppKit's main thread by its own callbacks.
    let key: bool = unsafe { msg_send![&*window, isKeyWindow] };
    key.then_some(window)
}
pub fn menu(id: &str) -> bool {
    let Some(window) = front() else {
        return false;
    };
    if !matches!(id, "close-tab" | "close-window") {
        return false;
    }
    // SAFETY: retained live native preview, on the main thread.
    unsafe {
        let _: () = msg_send![&*window,performClose:std::ptr::null::<AnyObject>()];
    }
    true
}
define_class!(
    #[unsafe(super(NSObject))]
    #[name="KhorMacFilePreviewDelegate"]
    struct PreviewDelegate;
    impl PreviewDelegate {
        #[unsafe(method(windowWillClose:))]
        fn closed(&self,_notice:*mut AnyObject) {
            // Keep self alive while releasing the preview owner.
            let _keep = unsafe { Retained::retain(self as *const Self as *mut Self) };
            if let Some(held)=HELD.with(|held|held.borrow_mut().take()) {
                // SAFETY: the close notification holds this window while
                // we release its view, monitor and outstanding reply.
                unsafe {
                    let _:()=msg_send![class!(NSEvent),removeMonitor:&*held.monitor];
                    let _:()=msg_send![&*held.preview,close];
                }
                let _=held.reply.send(Ok(true));
            }
        }
    }
);
pub async fn open(view: tauri::Webview, path: PathBuf) -> Result<bool, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    view.with_webview(move |webview| {
        if HELD.with(|held|held.borrow().is_some()){let _=tx.send(Err(khor_catalog::msg::file_preview_unavailable().into()));return;}
        // SAFETY: this closure and every AppKit callback run on the main
        // thread. The preview owns its objects until windowWillClose.
        unsafe {
            let wk=webview.inner()as *mut AnyObject;
            let frame:NSRect=msg_send![wk,bounds];
            let window:Allocated<AnyObject>=msg_send![class!(NSWindow),alloc];
            let window:Retained<AnyObject>=msg_send![window,initWithContentRect:frame,styleMask:11usize,backing:2usize,defer:false];
            let _:()=msg_send![&*window,setReleasedWhenClosed:false];
            let title=NSString::from_str(&path.file_name().unwrap_or_default().to_string_lossy());
            let _:()=msg_send![&*window,setTitle:&*title];
            let content:*mut AnyObject=msg_send![&*window,contentView];
            let bounds:NSRect=msg_send![content,bounds];
            let preview:Allocated<AnyObject>=msg_send![&PREVIEW,alloc];
            let preview:Option<Retained<AnyObject>>=msg_send![preview,initWithFrame:bounds,style:0isize];
            let Some(preview)=preview else{let _=tx.send(Err(khor_catalog::msg::file_preview_unavailable().into()));return;};
            let text=NSString::from_str(&path.to_string_lossy());
            let url:Retained<AnyObject>=msg_send![class!(NSURL),fileURLWithPath:&*text,isDirectory:false];
            let _:()=msg_send![&*preview,setPreviewItem:&*url];
            let _:()=msg_send![&*preview,setAutostarts:true];
            let _:()=msg_send![&*preview,setShouldCloseWithWindow:false];
            let _:()=msg_send![&*window,setContentView:&*preview];
            let delegate:Retained<PreviewDelegate>=msg_send![PreviewDelegate::class(),new];
            let _:()=msg_send![&*window,setDelegate:&*delegate];
            // Escape and Cmd-W belong to this native window, never to the
            // file tab behind it. Let every other key reach Quick Look.
            let keys=block2::RcBlock::new(move |event:*mut AnyObject|->*mut AnyObject {
                if let Some(window)=front() {
                    let key:u16=msg_send![event,keyCode];let flags:usize=msg_send![event,modifierFlags];
                    if key==53||(key==13&&flags&(1<<20)!=0){let _:()=msg_send![&*window,performClose:std::ptr::null::<AnyObject>()];return std::ptr::null_mut();}
                }
                event
            });
            let monitor:Retained<AnyObject>=msg_send![class!(NSEvent),addLocalMonitorForEventsMatchingMask:1usize<<10,handler:&*keys];
            let window_ptr=&*window as *const AnyObject;
            HELD.with(|held|*held.borrow_mut()=Some(Held{window,preview,_delegate:delegate,monitor,reply:tx}));
            let _:()=msg_send![window_ptr,center];
            let _:()=msg_send![window_ptr,makeKeyAndOrderFront:std::ptr::null::<AnyObject>()];
        }
    }).map_err(|e|e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}
