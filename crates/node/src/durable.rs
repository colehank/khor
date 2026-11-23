//! Making one file's bytes durable, with the same call on every platform.
//!
//! The callers here have just written a file and want its bytes on the
//! disk before the rename or the acknowledgement that makes them
//! reachable. Unix lets `fsync` run on a descriptor opened for reading
//! only; Windows refuses `FlushFileBuffers` on such a handle with
//! `ERROR_ACCESS_DENIED` (os error 5), which reads as a permission
//! problem at the call site even though nothing is wrong with the file.
//!
//! Opening for writing is what the Windows call needs, and every caller
//! has write permission by construction: the bytes it is flushing are
//! bytes it just wrote. A directory's own sync is a different thing --
//! Windows has no equivalent, so those sites stay `#[cfg(unix)]`.

use std::path::Path;

/// Flush `path`'s bytes to the disk.
///
/// Returns the platform's own error text, as the call sites report errors
/// to a person.
pub(crate) fn sync_file(path: &Path) -> Result<(), String> {
    let mut options = std::fs::OpenOptions::new();
    #[cfg(windows)]
    options.write(true);
    #[cfg(not(windows))]
    options.read(true);
    options
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())
}
