use std::{
    ffi::{CStr, c_char},
    path::PathBuf,
    sync::Arc,
};

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
unsafe extern "C" fn read_terminal_paste_probe(path: *const c_char) {
    // The Swift fixture owns the app-private path for this read.
    let path = PathBuf::from(
        unsafe { CStr::from_ptr(path) }
            .to_string_lossy()
            .into_owned(),
    );
    terminal_paste::read(Arc::new(move |value| {
        let temporary = path.with_extension("tmp");
        if std::fs::write(&temporary, value.to_string()).is_ok() {
            let _ = std::fs::rename(temporary, &path);
        }
    }))
}
