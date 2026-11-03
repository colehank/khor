use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
};

/// Closing the last inherited handle releases occupancy, including after a crash.
pub struct Lease(File);

impl Lease {
    pub(crate) fn make_inherited_private(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let Some(raw) = std::env::var("KHOR_RUNTIME_LEASE_FD")
                .ok()
                .and_then(|text| text.parse::<i32>().ok())
                .filter(|raw| *raw > 2)
            else {
                return Ok(());
            };
            let mut stat: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe { libc::fstat(raw, &mut stat) } != 0 {
                return Ok(());
            }
            let own = self.0.metadata()?;
            if stat.st_dev as u64 != own.dev() || stat.st_ino as u64 != own.ino() {
                return Ok(());
            }
            let flags = unsafe { libc::fcntl(raw, libc::F_GETFD) };
            if flags < 0 || unsafe { libc::fcntl(raw, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
            {
                return Err(io::Error::last_os_error());
            }
            // Keep the inherited descriptor for this process's life. It can be a reused
            // borrowed descriptor; taking ownership here could close another Rust File.
        }
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::{
                Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation},
                Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle},
            };
            let raw = std::io::stdin().as_raw_handle();
            let mut input = BY_HANDLE_FILE_INFORMATION::default();
            let mut own = BY_HANDLE_FILE_INFORMATION::default();
            if unsafe { GetFileInformationByHandle(raw, &mut input) } == 0
                || unsafe { GetFileInformationByHandle(self.0.as_raw_handle(), &mut own) } == 0
            {
                return Ok(());
            }
            if (
                input.dwVolumeSerialNumber,
                input.nFileIndexHigh,
                input.nFileIndexLow,
            ) == (
                own.dwVolumeSerialNumber,
                own.nFileIndexHigh,
                own.nFileIndexLow,
            ) && unsafe { SetHandleInformation(raw, HANDLE_FLAG_INHERIT, 0) } == 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
    pub(crate) fn take(path: &Path) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
            options.share_mode(FILE_SHARE_READ);
        }
        let file = options.open(path)?;
        #[cfg(unix)]
        file.lock_shared()?;
        Ok(Self(file))
    }

    pub fn file(&self) -> &File {
        &self.0
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self(self.0.try_clone()?))
    }

    pub(crate) fn vacant(path: &Path) -> io::Result<bool> {
        #[cfg(unix)]
        {
            let file = OpenOptions::new().read(true).write(true).open(path)?;
            match file.try_lock() {
                Ok(()) => Ok(true),
                Err(std::fs::TryLockError::WouldBlock) => Ok(false),
                Err(std::fs::TryLockError::Error(error)) => Err(error),
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::{
                Foundation::ERROR_SHARING_VIOLATION,
                Storage::FileSystem::{
                    DELETE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
                },
            };
            match OpenOptions::new()
                .access_mode(DELETE)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
                .open(path)
            {
                Ok(_) => Ok(true),
                Err(error) if error.raw_os_error() == Some(ERROR_SHARING_VIOLATION as i32) => {
                    Ok(false)
                }
                Err(error) => Err(error),
            }
        }
    }

    /// Only designated headless Khor hosts have an unused stdin suitable for this handoff.
    pub fn attach_host(&self, command: &mut std::process::Command) -> io::Result<()> {
        #[cfg(unix)]
        self.attach_unix(command)?;
        #[cfg(windows)]
        command.stdin(std::process::Stdio::from(self.0.try_clone()?));
        Ok(())
    }

    #[cfg(unix)]
    pub fn attach_unix(&self, command: &mut std::process::Command) -> io::Result<()> {
        use std::os::{fd::AsRawFd, unix::process::CommandExt};
        let file = self.0.try_clone()?;
        let raw = file.as_raw_fd();
        command.env("KHOR_RUNTIME_LEASE_FD", raw.to_string());
        unsafe {
            command.pre_exec(move || {
                let raw = file.as_raw_fd();
                let flags = libc::fcntl(raw, libc::F_GETFD);
                if flags < 0 || libc::fcntl(raw, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        Ok(())
    }
}
