use std::{
    ffi::{CStr, c_char},
    path::PathBuf,
    sync::Arc,
};

use objc2::runtime::AnyObject;

mod edit_menu {
    pub(crate) const PICTURES: [(&str, &str); 4] = [
        ("public.png", "image/png"),
        ("public.jpeg", "image/jpeg"),
        ("public.heic", "image/heic"),
        ("com.compuserve.gif", "image/gif"),
    ];
}

#[path = "../../../src-tauri/src/terminal_paste.rs"]
mod terminal_paste;

#[unsafe(no_mangle)]
unsafe extern "C" fn mount_terminal_paste_probe(
    web: *mut AnyObject,
    frame: *const [f64; 4],
    path: *const c_char,
) -> bool {
    // Swift passes its live WKWebView, four coordinates, and an app-private
    // output path for the result. Only the clipboard payload is read.
    let frame = unsafe { *frame };
    let path = PathBuf::from(
        unsafe { CStr::from_ptr(path) }
            .to_string_lossy()
            .into_owned(),
    );
    terminal_paste::place(
        web,
        "paste-probe".into(),
        Some(terminal_paste::Geometry {
            frame,
            clip: frame,
            radius: 8.0,
            dark: true,
        }),
        Arc::new(move |value| {
            if value.get("scroll").is_some() {
                return;
            }
            let temporary = path.with_extension("tmp");
            if std::fs::write(&temporary, value.to_string()).is_ok() {
                let _ = std::fs::rename(temporary, &path);
            }
        }),
    )
}
