use std::{ffi::{CStr, c_char}, path::PathBuf, sync::Arc};
use objc2::runtime::AnyObject;

mod edit_menu {
    pub(crate) const PICTURES: [(&str, &str); 4] = [
        ("public.png", "image/png"), ("public.jpeg", "image/jpeg"),
        ("public.heic", "image/heic"), ("com.compuserve.gif", "image/gif"),
    ];
}
mod terminal_paste;

#[unsafe(no_mangle)]
extern "C" fn mount_terminal_paste_probe(face: *mut AnyObject, path: *const c_char) -> bool {
    let path = unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned();
    let path = PathBuf::from(path);
    let geometry = terminal_paste::Geometry {
        frame: [290.0, 120.0, 100.0, 42.0],
        clip: [290.0, 120.0, 100.0, 42.0],
        radius: 10.0, dark: false,
    };
    terminal_paste::place(face, "paste-probe".to_owned(), Some(geometry), Arc::new(move |value| {
        let temporary = path.with_extension("tmp");
        if std::fs::write(&temporary, value.to_string()).is_ok() {
            let _ = std::fs::rename(temporary, &path);
        }
    }))
}
