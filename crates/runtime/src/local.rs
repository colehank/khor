use std::{
    io,
    path::{Path, PathBuf},
};

pub fn local_directory() -> io::Result<PathBuf> {
    if let Some(root) = std::env::var_os("KHOR_RUNTIME_DIR") {
        return Ok(root.into());
    }
    #[cfg(target_os = "linux")]
    let root = PathBuf::from(format!("/var/tmp/khor-runtime-{}", unsafe {
        libc::getuid()
    }));
    #[cfg(target_os = "macos")]
    let root = {
        let length = unsafe { libc::confstr(libc::_CS_DARWIN_USER_DIR, std::ptr::null_mut(), 0) };
        if length == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut bytes = vec![0; length];
        if unsafe { libc::confstr(libc::_CS_DARWIN_USER_DIR, bytes.as_mut_ptr().cast(), length) }
            != length
        {
            return Err(io::Error::other("local user directory changed"));
        }
        bytes.pop();
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(std::ffi::OsString::from_vec(bytes)).join("khor-runtime")
    };
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("LOCALAPPDATA unavailable"))?
        .join("Khor/runtime");
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "local hosting is unavailable",
    ));
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    Ok(root)
}

pub(crate) fn validate(path: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err(io::Error::other(
            "runtime directory is not an owned directory",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
        if metadata.uid() != unsafe { libc::getuid() } || metadata.mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "runtime directory is not private",
            ));
        }
        let name = std::ffi::CString::new(path.as_os_str().as_bytes())?;
        let mut info: libc::statfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statfs(name.as_ptr(), &mut info) } != 0 {
            return Err(io::Error::last_os_error());
        }
        #[cfg(target_os = "linux")]
        let local = matches!(
            info.f_type as u64,
            0xef53 | 0x58465342 | 0x9123683e | 0x01021994 | 0x794c7630 | 0x2fc12fc1 | 0xf2f52010
        );
        #[cfg(target_os = "macos")]
        let local = info.f_flags & libc::MNT_LOCAL as u32 != 0;
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let local = false;
        if !local {
            return Err(io::Error::other(
                "runtime requires a verified local filesystem",
            ));
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Storage::FileSystem::{GetDriveTypeW, GetVolumePathNameW},
            System::WindowsProgramming::{DRIVE_FIXED, DRIVE_RAMDISK},
        };
        let wide: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut volume = vec![0; 32768];
        if unsafe { GetVolumePathNameW(wide.as_ptr(), volume.as_mut_ptr(), volume.len() as u32) }
            == 0
        {
            return Err(io::Error::last_os_error());
        }
        if !matches!(
            unsafe { GetDriveTypeW(volume.as_ptr()) },
            DRIVE_FIXED | DRIVE_RAMDISK
        ) {
            return Err(io::Error::other(
                "runtime requires a verified local filesystem",
            ));
        }
    }
    Ok(())
}

pub(crate) fn available(path: &Path) -> io::Result<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(path.as_os_str().as_bytes())?;
        let mut info: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(name.as_ptr(), &mut info) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok((info.f_bavail as u64).saturating_mul(info.f_frsize as u64))
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut free = 0;
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(free)
    }
}
