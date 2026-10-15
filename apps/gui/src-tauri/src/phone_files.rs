//! iOS folder selection. UIKit grants access; a coordinated, bounded
//! copy runs off the main thread while that access stays held. The face
//! receives only app-owned staged paths, including empty directories.
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, NSObject};
use objc2::{ClassType, class, define_class, msg_send};
use objc2_foundation::NSString;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

type Answer = Result<Vec<String>, String>;
struct Pending {
    app: tauri::AppHandle,
    root: PathBuf,
    tx: tokio::sync::oneshot::Sender<Answer>,
}
struct Scope {
    url: Retained<AnyObject>,
    granted: bool,
}
thread_local! {
    static BUSY:Cell<bool> = const {Cell::new(false)};
    static PENDING:RefCell<Option<Pending>> = const {RefCell::new(None)};
    static SCOPES:RefCell<Vec<Scope>> = const {RefCell::new(Vec::new())};
    static PICKER:RefCell<Option<Retained<AnyObject>>> = const {RefCell::new(None)};
    static DELEGATE:Retained<FolderDelegate> = unsafe {msg_send![FolderDelegate::class(),new]};
}

fn release() {
    SCOPES.with(|scopes| {
        for scope in scopes.borrow_mut().drain(..) {
            if scope.granted {
                // SAFETY: each successful start is paired with one stop,
                // after the worker has finished accessing the directory.
                unsafe {
                    let _: () = msg_send![&*scope.url, stopAccessingSecurityScopedResource];
                }
            }
        }
    });
    PICKER.with(|picker| {
        picker.borrow_mut().take();
    });
    BUSY.with(|busy| busy.set(false));
}

define_class!(
    // SAFETY: no ivars or extra NSObject subclassing requirements.
    #[unsafe(super(NSObject))]
    #[name="KhorFolderPickerDelegate"]
    struct FolderDelegate;
    impl FolderDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn picked(&self,_picker:*mut AnyObject,urls:*mut AnyObject) {
            let Some(pending)=PENDING.with(|p|p.borrow_mut().take()) else {return;};
            let mut paths=Vec::new();
            // SAFETY: UIDocumentPickerDelegate hands an NSArray of NSURLs
            // on the main thread. Retain each until coordinated copying ends.
            unsafe {
                let count:usize=msg_send![urls,count];
                for i in 0..count {
                    let url:*mut AnyObject=msg_send![urls,objectAtIndex:i];
                    let Some(url)=Retained::retain(url) else {continue;};
                    let path:Option<Retained<NSString>>=msg_send![&*url,path];
                    let granted:bool=msg_send![&*url,startAccessingSecurityScopedResource];
                    if let Some(path)=path {paths.push(path.to_string());}
                    SCOPES.with(|s|s.borrow_mut().push(Scope {url,granted}));
                }
            }
            std::thread::spawn(move || {
                let mut staged=Vec::new();
                let result=(|| -> Answer {
                    for path in paths {staged.push(objc2::rc::autoreleasepool(|_| coordinated_copy(&pending.root,&path))?);}
                    Ok(staged.clone())
                })();
                if result.is_err() {khor_gui_core::term::term_drop_forget(&pending.root,&staged);}
                let _=pending.app.run_on_main_thread(move || {release();let _=pending.tx.send(result);});
            });
        }
        #[unsafe(method(documentPickerWasCancelled:))]
        fn cancelled(&self,_picker:*mut AnyObject) {
            let pending=PENDING.with(|p|p.borrow_mut().take());
            release();
            if let Some(pending)=pending {let _=pending.tx.send(Ok(Vec::new()));}
        }
    }
);

fn coordinated_copy(root: &Path, path: &str) -> Result<String, String> {
    let result = std::rc::Rc::new(RefCell::new(None));
    let answer = result.clone();
    let root = root.to_path_buf();
    let accessor = block2::RcBlock::new(move |url: *mut AnyObject| {
        // SAFETY: the coordinator supplies a readable NSURL, potentially
        // adjusted by its file provider, for this accessor's duration.
        let path: Option<Retained<NSString>> = unsafe { msg_send![url, path] };
        let out = path
            .ok_or_else(|| khor_catalog::msg::folder_picker_unavailable().to_owned())
            .and_then(|p| {
                khor_gui_core::term::term_drop_stage_native(&root, Path::new(&p.to_string()))
            });
        *answer.borrow_mut() = Some(out);
    });
    // SAFETY: Foundation objects stay on this worker. Coordination is
    // synchronous; the block and NSError pointer outlive the whole call.
    unsafe {
        let text = NSString::from_str(path);
        let url: Retained<AnyObject> =
            msg_send![class!(NSURL),fileURLWithPath:&*text,isDirectory:true];
        let coordinator: Allocated<AnyObject> = msg_send![class!(NSFileCoordinator), alloc];
        let coordinator: Retained<AnyObject> =
            msg_send![coordinator,initWithFilePresenter:std::ptr::null_mut::<AnyObject>()];
        let mut error: *mut AnyObject = std::ptr::null_mut();
        let _: () = msg_send![&*coordinator,coordinateReadingItemAtURL:&*url,options:0usize,error:&mut error,byAccessor:&*accessor];
        if !error.is_null() {
            let why: Retained<NSString> = msg_send![error, localizedDescription];
            return Err(why.to_string());
        }
    }
    let out = result
        .borrow_mut()
        .take()
        .unwrap_or_else(|| Err(khor_catalog::msg::folder_picker_unavailable().to_owned()));
    out
}

pub async fn pick(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Answer {
    let (tx, rx) = tokio::sync::oneshot::channel();
    window.with_webview(move |view| {
        if BUSY.with(|busy|busy.replace(true)) {let _=tx.send(Err(khor_catalog::msg::folder_picker_busy().to_owned()));return;}
        // SAFETY: with_webview runs on the main thread, and inner is
        // this window's live WKWebView. Walk only presented controllers.
        unsafe {
            let wk=view.inner() as *mut AnyObject;
            let ui_window:*mut AnyObject=msg_send![wk,window];
            let mut top:*mut AnyObject=if ui_window.is_null(){std::ptr::null_mut()}else{msg_send![ui_window,rootViewController]};
            if top.is_null() {BUSY.with(|b|b.set(false));let _=tx.send(Err(khor_catalog::msg::folder_picker_unavailable().to_owned()));return;}
            loop {let next:*mut AnyObject=msg_send![top,presentedViewController];if next.is_null(){break;}top=next;}
            let folder=NSString::from_str("public.folder");
            let types:Retained<AnyObject>=msg_send![class!(NSArray),arrayWithObject:&*folder];
            let picker:Allocated<AnyObject>=msg_send![class!(UIDocumentPickerViewController),alloc];
            // Open mode grants directory access. Import mode cannot
            // express folders; the coordinated copy above owns the bytes.
            let picker:Retained<AnyObject>=msg_send![picker,initWithDocumentTypes:&*types,inMode:1usize];
            let _:()=msg_send![&*picker,setAllowsMultipleSelection:true];
            DELEGATE.with(|delegate| {let _:()=msg_send![&*picker,setDelegate:&**delegate];});
            PENDING.with(|p|*p.borrow_mut()=Some(Pending{app,root:khor_node::Node::root_from_env(),tx}));
            let _:()=msg_send![top,presentViewController:&*picker,animated:true,completion:std::ptr::null::<block2::Block<dyn Fn()>>()];
            PICKER.with(|p|*p.borrow_mut()=Some(picker));
        }
    }).map_err(|e|e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}
