//! Narrow filesystem operations for unpublished transfer controls.

use std::{fs::File, io, path::Path};

pub(super) fn check_control_file(file: &File) -> io::Result<()> {
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "control is not a file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "control has multiple links",
            ));
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        use std::{
            ffi::c_void,
            os::windows::{fs::MetadataExt, io::AsRawHandle},
        };
        #[repr(C)]
        struct StandardInfo {
            allocation: i64,
            end_of_file: i64,
            links: u32,
            delete_pending: u8,
            directory: u8,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetFileInformationByHandleEx(
                handle: *mut c_void,
                class: i32,
                info: *mut c_void,
                size: u32,
            ) -> i32;
        }
        let mut info = StandardInfo {
            allocation: 0,
            end_of_file: 0,
            links: 0,
            delete_pending: 0,
            directory: 0,
        };
        // SAFETY: File keeps the handle live. The repr(C) fields match
        // FILE_STANDARD_INFO, including its two single-byte BOOLEANs.
        let ok = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                1,
                (&mut info as *mut StandardInfo).cast(),
                std::mem::size_of::<StandardInfo>() as u32,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if info.links != 1
            || info.directory != 0
            || info.delete_pending != 0
            || meta.file_attributes() & 0x400 != 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "control is linked, reparse or pending deletion",
            ));
        }
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "control link count unavailable",
    ))
}

#[cfg(unix)]
fn c_path(path: &Path) -> io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))
}

#[cfg(target_vendor = "apple")]
pub(super) fn publish_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::{c_char, c_int, c_uint};
    unsafe extern "C" {
        fn renamex_np(source: *const c_char, destination: *const c_char, flags: c_uint) -> c_int;
    }
    let source = c_path(source)?;
    let destination = c_path(destination)?;
    // SAFETY: Both NUL-terminated paths outlive the call. Apple's userspace
    // stdio.h defines RENAME_EXCL=4; unsupported filesystems return an error.
    let result = unsafe { renamex_np(source.as_ptr(), destination.as_ptr(), 4) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) fn publish_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::c_long;
    // Linux UAPI / AOSP per-architecture unistd headers. Pointer widths
    // exclude x32/ILP32; ARM additionally requires the EABI syscall table.
    let number: c_long = match (std::env::consts::ARCH, usize::BITS) {
        ("x86_64", 64) => 316,
        ("aarch64", 64) => 276,
        ("x86", 32) => 353,
        ("arm", 32) if cfg!(any(target_abi = "eabi", target_abi = "eabihf")) => 382,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "exclusive rename syscall ABI unavailable",
            ));
        }
    };
    unsafe extern "C" {
        fn syscall(number: c_long, ...) -> c_long;
    }
    let source = c_path(source)?;
    let destination = c_path(destination)?;
    // SAFETY: The selected UAPI number is renameat2 for this exact ABI.
    // CString pointers are live and scalar varargs have native long width.
    // Bionic's renameat2 symbol needs API30 (our minSdk is24); syscall avoids
    // that startup dependency and also works with musl lacking that wrapper.
    let result = unsafe {
        syscall(
            number,
            -100 as c_long,
            source.as_ptr(),
            -100 as c_long,
            destination.as_ptr(),
            1 as c_long,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
pub(super) fn publish_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    fn wide(path: &Path) -> io::Result<Vec<u16>> {
        let mut out: Vec<_> = path.as_os_str().encode_wide().collect();
        if out.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path contains NUL",
            ));
        }
        out.push(0);
        Ok(out)
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32;
    }
    let source = wide(source)?;
    let destination = wide(destination)?;
    // SAFETY: Both UTF16 paths are NUL terminated and outlive the call.
    // Flags0 excludes replacement, copying and delayed moves; directories
    // must stay on one volume. Failure never falls back to std rename.
    let result = unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) };
    if result != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(
    target_vendor = "apple",
    target_os = "linux",
    target_os = "android",
    windows
)))]
pub(super) fn publish_no_replace(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "exclusive rename unavailable",
    ))
}
