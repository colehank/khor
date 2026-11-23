//! Host the registered Windows document previewer in an owned read-only window.
use khor_gui_core::file_handoff::ReadyFile;
use std::{cell::RefCell, os::windows::ffi::OsStrExt, path::Path};
use tauri::{Manager, Window, WindowEvent, window::WindowBuilder};
use tokio::sync::oneshot;
use windows::{
    Win32::{
        Foundation::{E_POINTER, HWND, LPARAM, RECT, S_FALSE, WPARAM},
        System::{
            Com::{
                CLSCTX_DISABLE_AAA, CLSCTX_LOCAL_SERVER, CLSIDFromString, COINIT_APARTMENTTHREADED,
                CoCreateInstance, CoInitializeEx, CoUninitialize, IStream, STGM_READ,
                STGM_SHARE_DENY_NONE,
            },
            Ole::IObjectWithSite,
        },
        UI::{
            Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_ESCAPE, VK_W},
            Shell::{
                ASSOCF_NONE, ASSOCSTR_SHELLEXTENSION, AssocQueryStringW, IPreviewHandler,
                IPreviewHandlerFrame, IPreviewHandlerFrame_Impl, PREVIEWHANDLERFRAMEINFO,
                PropertiesSystem::{IInitializeWithFile, IInitializeWithStream},
                SHCreateStreamOnFileEx,
            },
            WindowsAndMessaging::{GetClientRect, MSG, PostMessageW, WM_CLOSE, WM_KEYDOWN},
        },
    },
    core::{Error, GUID, IUnknown, Interface, PCWSTR, PWSTR, implement, w},
};

const LABEL: &str = "file-preview";

fn handler_id(path: &Path) -> Option<GUID> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(
        extension.as_str(),
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx"
    ) {
        return None;
    }
    let extension: Vec<u16> = format!(".{extension}\0").encode_utf16().collect();
    let mut id = [0u16; 40];
    let mut len = id.len() as u32;
    // SAFETY: both inputs are terminated and the writable buffer has len UTF-16 units.
    unsafe {
        AssocQueryStringW(
            ASSOCF_NONE,
            ASSOCSTR_SHELLEXTENSION,
            PCWSTR(extension.as_ptr()),
            w!("{8895B1C6-B41F-4C1C-A562-0D564250836F}"),
            Some(PWSTR(id.as_mut_ptr())),
            &mut len,
        )
        .ok()
        .ok()?;
        CLSIDFromString(PCWSTR(id.as_ptr())).ok()
    }
}

pub fn supported(path: &str) -> bool {
    handler_id(Path::new(path)).is_some()
}

struct Apartment;
impl Apartment {
    fn enter() -> windows::core::Result<Self> {
        // SAFETY: paired on this same UI thread by Drop, including S_FALSE.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: this value is kept in the window's UI-thread storage.
        unsafe { CoUninitialize() };
    }
}

#[implement(IPreviewHandlerFrame)]
struct Frame {
    window: HWND,
}
impl IPreviewHandlerFrame_Impl for Frame_Impl {
    fn GetWindowContext(&self) -> windows::core::Result<PREVIEWHANDLERFRAMEINFO> {
        Ok(PREVIEWHANDLERFRAMEINFO::default())
    }

    fn TranslateAccelerator(&self, message: *const MSG) -> windows::core::Result<()> {
        // SAFETY: COM passes a readable MSG for the duration of this call.
        let message = unsafe { message.as_ref() }.ok_or_else(|| Error::from_hresult(E_POINTER))?;
        let key = message.wParam.0;
        // SAFETY: GetKeyState has no pointer arguments.
        let control = unsafe { GetKeyState(i32::from(VK_CONTROL.0)) } < 0;
        if message.message == WM_KEYDOWN
            && (key == usize::from(VK_ESCAPE.0) || (control && key == usize::from(VK_W.0)))
        {
            // SAFETY: the frame belongs to this preview window. A posted close
            // runs after the current preview-handler call has unwound.
            return unsafe { PostMessageW(Some(self.window), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        Err(Error::from_hresult(S_FALSE))
    }
}

struct Mounted {
    handler: IPreviewHandler,
    site: Option<IObjectWithSite>,
    _file: ReadyFile,
    _apartment: Apartment,
    closed: Option<oneshot::Sender<()>>,
}
impl Drop for Mounted {
    fn drop(&mut self) {
        // SAFETY: the handler was initialized on this same UI thread.
        unsafe {
            let _ = self.handler.Unload();
            if let Some(site) = &self.site {
                let _ = site.SetSite(None::<&IUnknown>);
            }
        }
    }
}

thread_local! {
    static PREVIEW: RefCell<Option<Mounted>> = const { RefCell::new(None) };
}

fn mount(window: &Window, file: ReadyFile, closed: oneshot::Sender<()>) -> Result<(), String> {
    if file.is_cancelled() {
        return Err(khor_node::drops::cancelled());
    }
    let id = handler_id(file.path()).ok_or_else(unavailable)?;
    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    let path: Vec<u16> = dunce::simplified(file.path())
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let load = || -> windows::core::Result<Mounted> {
        let apartment = Apartment::enter()?;
        // SAFETY: this closure runs in the UI STA. Windows activates only the
        // system's out-of-process handler for this registered document type.
        unsafe {
            let handler: IPreviewHandler = CoCreateInstance(
                &id,
                None::<&IUnknown>,
                CLSCTX_LOCAL_SERVER | CLSCTX_DISABLE_AAA,
            )?;
            if let Ok(initializer) = handler.cast::<IInitializeWithStream>() {
                let stream = SHCreateStreamOnFileEx(
                    PCWSTR(path.as_ptr()),
                    (STGM_READ | STGM_SHARE_DENY_NONE).0,
                    0,
                    false,
                    None::<&IStream>,
                )?;
                initializer.Initialize(&stream, STGM_READ.0)?;
            } else {
                handler
                    .cast::<IInitializeWithFile>()?
                    .Initialize(PCWSTR(path.as_ptr()), STGM_READ.0)?;
            }
            let mounted = Mounted {
                site: handler.cast::<IObjectWithSite>().ok(),
                handler,
                _file: file,
                _apartment: apartment,
                closed: Some(closed),
            };
            if let Some(site) = &mounted.site {
                let frame: IPreviewHandlerFrame = Frame { window: hwnd }.into();
                site.SetSite(&frame)?;
            }
            let mut rect = RECT::default();
            GetClientRect(hwnd, &mut rect)?;
            mounted.handler.SetWindow(hwnd, &rect)?;
            mounted.handler.DoPreview()?;
            Ok(mounted)
        }
    };
    let mounted = load().map_err(|error| {
        eprintln!("Windows document preview failed: {error}");
        unavailable()
    })?;
    PREVIEW.with(|preview| *preview.borrow_mut() = Some(mounted));
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

fn unavailable() -> String {
    khor_catalog::msg::file_preview_unavailable().into()
}

pub async fn open(view: tauri::Webview, file: ReadyFile) -> Result<bool, String> {
    let parent = view.window();
    let app = view.app_handle();
    if app.get_window(LABEL).is_some() || !supported(&file.path().to_string_lossy()) {
        return Err(unavailable());
    }
    let title = file
        .path()
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let mut builder = WindowBuilder::new(app, LABEL)
        .owner(&parent)
        .map_err(|error| error.to_string())?
        .title(title)
        .center()
        .visible(false);
    if let (Ok(size), Ok(scale)) = (parent.inner_size(), parent.scale_factor()) {
        builder = builder.inner_size(
            f64::from(size.width) / scale,
            f64::from(size.height) / scale,
        );
    }
    let window = builder.build().map_err(|error| error.to_string())?;
    let (closed, wait) = oneshot::channel();
    let (opened, ready) = oneshot::channel();
    let resize = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => resize_preview(&resize),
        WindowEvent::Destroyed => {
            if let Some(mut mounted) = PREVIEW.with(|preview| preview.borrow_mut().take()) {
                // Complete only after Unload and the staged-file lease have been released.
                let reply = mounted.closed.take();
                drop(mounted);
                if let Some(reply) = reply {
                    let _ = reply.send(());
                }
            }
        }
        _ => {}
    });
    let mount_window = window.clone();
    window
        .run_on_main_thread(move || {
            let result = mount(&mount_window, file, closed);
            if result.is_err() {
                let _ = mount_window.destroy();
            }
            let _ = opened.send(result);
        })
        .map_err(|error| error.to_string())?;
    ready.await.map_err(|_| unavailable())??;
    wait.await.map_err(|_| unavailable())?;
    Ok(true)
}

fn resize_preview(window: &Window) {
    let handler = PREVIEW.with(|preview| preview.borrow().as_ref().map(|p| p.handler.clone()));
    if let (Some(handler), Ok(hwnd)) = (handler, window.hwnd()) {
        let mut rect = RECT::default();
        // SAFETY: the event is on the UI STA and rect is writable.
        unsafe {
            if GetClientRect(hwnd, &mut rect).is_ok() {
                let _ = handler.SetRect(&rect);
            }
        }
    }
}
