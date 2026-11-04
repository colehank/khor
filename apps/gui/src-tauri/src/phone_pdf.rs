use khor_gui_core::file_handoff::ReadyFile;
use objc2::{
    ClassType, class, define_class, msg_send,
    rc::{Allocated, Retained},
    runtime::{AnyClass, AnyObject, NSObject},
    sel,
};
use objc2_foundation::{NSPoint, NSRect, NSString};
use std::{cell::RefCell, collections::VecDeque, path::PathBuf};

#[link(name = "PDFKit", kind = "framework")]
unsafe extern "C" {
    #[link_name = "OBJC_CLASS_$_PDFDocument"]
    static DOCUMENT: AnyClass;
    #[link_name = "OBJC_CLASS_$_PDFView"]
    static VIEW: AnyClass;
    #[link_name = "OBJC_CLASS_$_PDFDestination"]
    static DESTINATION: AnyClass;
    static PDFViewPageChangedNotification: *const AnyObject;
}

#[derive(Clone, Copy)]
struct Position {
    page: usize,
    point: NSPoint,
    zoom: f64,
}
struct Reader {
    file: ReadyFile,
    navigation: Retained<AnyObject>,
    content: Retained<AnyObject>,
    pdf: Retained<AnyObject>,
    document: Retained<AnyObject>,
    delegate: Retained<ReaderDelegate>,
    reply: tokio::sync::oneshot::Sender<Result<bool, String>>,
}
thread_local! {
    static READER:RefCell<Option<Reader>>=const{RefCell::new(None)};
    static POSITIONS:RefCell<VecDeque<(PathBuf,Position)>>=const{RefCell::new(VecDeque::new())};
}

fn owns(reader: &Reader, identity: *const AnyObject) -> bool {
    std::ptr::eq(&*reader.navigation, identity)
}

unsafe fn remember(reader: &Reader) {
    unsafe {
        let destination: *mut AnyObject = msg_send![&*reader.pdf, currentDestination];
        if destination.is_null() {
            return;
        }
        let page: *mut AnyObject = msg_send![destination, page];
        if page.is_null() {
            return;
        }
        let index: usize = msg_send![&*reader.document,indexForPage:page];
        let count: usize = msg_send![&*reader.document, pageCount];
        let point: NSPoint = msg_send![destination, point];
        let scale: f64 = msg_send![&*reader.pdf, scaleFactor];
        let fit: f64 = msg_send![&*reader.pdf, scaleFactorForSizeToFit];
        let zoom = scale / fit;
        if index >= count
            || !point.x.is_finite()
            || !point.y.is_finite()
            || !zoom.is_finite()
            || zoom <= 0.0
        {
            return;
        }
        POSITIONS.with(|positions| {
            let mut positions = positions.borrow_mut();
            positions.retain(|(path, _)| path != reader.file.path());
            positions.push_back((
                reader.file.path().to_owned(),
                Position {
                    page: index,
                    point,
                    zoom,
                },
            ));
            while positions.len() > 64 {
                positions.pop_front();
            }
        });
    }
}

fn finish(identity: *const AnyObject, result: Result<bool, String>) {
    let reader = READER.with(|reader| {
        let mut reader = reader.borrow_mut();
        if reader.as_ref().is_some_and(|reader| owns(reader, identity)) {
            reader.take()
        } else {
            None
        }
    });
    let Some(reader) = reader else {
        return;
    };
    // SAFETY: UIKit callbacks and with_webview run on the main thread;
    // the owned native objects and original stay alive through this cleanup.
    unsafe {
        let center: *mut AnyObject = msg_send![class!(NSNotificationCenter), defaultCenter];
        let _: () = msg_send![center,removeObserver:&*reader.delegate];
        if result.is_ok() {
            remember(&reader);
        }
    }
    let _ = reader.reply.send(result);
}

unsafe fn page_label(pdf: &AnyObject, document: &AnyObject, content: &AnyObject) {
    unsafe {
        let page: *mut AnyObject = msg_send![pdf, currentPage];
        if page.is_null() {
            return;
        }
        let index: usize = msg_send![document,indexForPage:page];
        let count: usize = msg_send![document, pageCount];
        if index >= count {
            return;
        }
        let text = NSString::from_str(&format!("{} / {}", index + 1, count));
        let item: *mut AnyObject = msg_send![content, navigationItem];
        let _: () = msg_send![item,setPrompt:&*text];
    }
}

fn appeared(identity: *const AnyObject) {
    let current = READER.with(|reader| {
        reader
            .borrow()
            .as_ref()
            .filter(|reader| owns(reader, identity))
            .map(|reader| {
                (
                    reader.pdf.clone(),
                    reader.document.clone(),
                    reader.content.clone(),
                    reader.file.path().to_owned(),
                )
            })
    });
    let Some((pdf, document, content, path)) = current else {
        return;
    };
    let position = POSITIONS.with(|positions| {
        positions
            .borrow()
            .iter()
            .find(|(key, _)| key == &path)
            .map(|(_, position)| *position)
    });
    // SAFETY: the presentation completed on UIKit's main thread. The
    // destination is restored after layout, in PDF page coordinates.
    unsafe {
        let _: () = msg_send![&*pdf, layoutIfNeeded];
        if let Some(position) = position {
            let page: *mut AnyObject = msg_send![&*document,pageAtIndex:position.page];
            if !page.is_null() {
                let fit: f64 = msg_send![&*pdf, scaleFactorForSizeToFit];
                if fit.is_finite() && fit > 0.0 {
                    let _: () = msg_send![&*pdf,setScaleFactor:fit*position.zoom];
                }
                let destination: Allocated<AnyObject> = msg_send![&DESTINATION, alloc];
                let destination: Option<Retained<AnyObject>> =
                    msg_send![destination,initWithPage:page,atPoint:position.point];
                if let Some(destination) = destination {
                    let _: () = msg_send![&*pdf,goToDestination:&*destination];
                }
            }
        }
        page_label(&pdf, &document, &content);
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name="KhorPDFReaderDelegate"]
    struct ReaderDelegate;
    impl ReaderDelegate {
        #[unsafe(method(close:))]
        fn close(&self,_sender:*mut AnyObject) {
            let _keep=unsafe{Retained::retain(self as *const Self as *mut Self)};
            let navigation=READER.with(|reader|reader.borrow().as_ref().filter(|reader|std::ptr::eq(&*reader.delegate,self)).map(|reader|reader.navigation.clone()));
            let Some(navigation)=navigation else{return;};
            let identity=&*navigation as *const AnyObject;
            let done=block2::RcBlock::new(move ||finish(identity,Ok(true)));
            // SAFETY: a UIBarButtonItem action is delivered on the main thread.
            unsafe {let _:()=msg_send![&*navigation,dismissViewControllerAnimated:true,completion:&*done];}
        }
        #[unsafe(method(presentationControllerDidDismiss:))]
        fn dismissed(&self,controller:*mut AnyObject) {
            let _keep=unsafe{Retained::retain(self as *const Self as *mut Self)};
            // SAFETY: UIKit supplies the presentation controller on its main thread.
            let navigation:*mut AnyObject=unsafe{msg_send![controller,presentedViewController]};
            finish(navigation,Ok(true));
        }
        #[unsafe(method(pageChanged:))]
        fn page_changed(&self,notification:*mut AnyObject) {
            let current=READER.with(|reader|reader.borrow().as_ref().filter(|reader|std::ptr::eq(&*reader.delegate,self))
                .map(|reader|(reader.pdf.clone(),reader.document.clone(),reader.content.clone())));
            let Some((pdf,document,content))=current else{return;};
            // SAFETY: registered only for this PDFView's main-thread notifications.
            unsafe {
                let object:*mut AnyObject=msg_send![notification,object];
                if std::ptr::eq(object,&*pdf){page_label(&pdf,&document,&content);}
            }
        }
    }
);

pub async fn open(view: tauri::Webview, file: ReadyFile) -> Result<bool, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    view.with_webview(move |webview| {
        let fail=||khor_catalog::msg::file_preview_unavailable().to_owned();
        if file.is_cancelled(){let _=tx.send(Err(khor_node::drops::cancelled()));return;}
        if READER.with(|reader|reader.borrow().is_some()){let _=tx.send(Err(fail()));return;}
        // SAFETY: with_webview supplies a live WKWebView on the main thread.
        unsafe {
            let wk=&*(webview.inner() as *mut AnyObject);
            let Some(top)=crate::phone_presentation::presenter(wk) else {let _=tx.send(Err(fail()));return;};
            let path=NSString::from_str(&file.path().to_string_lossy());
            let url:Retained<AnyObject>=msg_send![class!(NSURL),fileURLWithPath:&*path,isDirectory:false];
            let document:Allocated<AnyObject>=msg_send![&DOCUMENT,alloc];
            let document:Option<Retained<AnyObject>>=msg_send![document,initWithURL:&*url];
            let Some(document)=document else {let _=tx.send(Err(fail()));return;};
            let count:usize=msg_send![&*document,pageCount];
            let locked:bool=msg_send![&*document,isLocked];
            if count==0 || locked {let _=tx.send(Err(fail()));return;}
            let frame:NSRect=msg_send![wk,bounds];
            let pdf:Allocated<AnyObject>=msg_send![&VIEW,alloc];
            let pdf:Retained<AnyObject>=msg_send![pdf,initWithFrame:frame];
            let _:()=msg_send![&*pdf,setDisplayMode:1isize];
            let _:()=msg_send![&*pdf,setAutoScales:true];
            let _:()=msg_send![&*pdf,setDocument:&*document];
            let content:Retained<AnyObject>=msg_send![class!(UIViewController),new];
            let _:()=msg_send![&*content,setView:&*pdf];
            let title=NSString::from_str(&file.path().file_name().unwrap_or_default().to_string_lossy());
            let _:()=msg_send![&*content,setTitle:&*title];
            let delegate:Retained<ReaderDelegate>=msg_send![ReaderDelegate::class(),new];
            let close:Allocated<AnyObject>=msg_send![class!(UIBarButtonItem),alloc];
            let close:Retained<AnyObject>=msg_send![close,initWithBarButtonSystemItem:0isize,target:&*delegate,action:sel!(close:)];
            let item:*mut AnyObject=msg_send![&*content,navigationItem];
            let pages=NSString::from_str(&format!("1 / {count}"));
            let _:()=msg_send![item,setPrompt:&*pages];
            let _:()=msg_send![item,setRightBarButtonItem:&*close];
            let navigation:Allocated<AnyObject>=msg_send![class!(UINavigationController),alloc];
            let navigation:Retained<AnyObject>=msg_send![navigation,initWithRootViewController:&*content];
            let _:()=msg_send![&*navigation,setModalPresentationStyle:0isize];
            let center:*mut AnyObject=msg_send![class!(NSNotificationCenter),defaultCenter];
            let _:()=msg_send![center,addObserver:&*delegate,selector:sel!(pageChanged:),name:PDFViewPageChangedNotification,object:&*pdf];
            let identity=&*navigation as *const AnyObject;
            let shown=block2::RcBlock::new(move ||appeared(identity));
            READER.with(|reader|*reader.borrow_mut()=Some(Reader{file,navigation:navigation.clone(),content,pdf,document,delegate,reply:tx}));
            if !crate::phone_presentation::present_after(&top,&navigation,Some(&*shown)){finish(identity,Err(fail()));return;}
            let presentation:*mut AnyObject=msg_send![&*navigation,presentationController];
            if !presentation.is_null() {
                let delegate=READER.with(|reader|reader.borrow().as_ref().map(|reader|reader.delegate.clone()));
                if let Some(delegate)=delegate {let _:()=msg_send![presentation,setDelegate:&*delegate];}
            }
        }
    }).map_err(|error|error.to_string())?;
    rx.await.map_err(|error| error.to_string())?
}
