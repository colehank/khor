//! Windows process observations use owned handles, never Unix signal emulation.
use std::{
    os::windows::{ffi::OsStringExt, io::{AsRawHandle, FromRawHandle, OwnedHandle}},
    path::PathBuf,
};
use windows_sys::Win32::{
    Foundation::{ERROR_ACCESS_DENIED, WAIT_TIMEOUT},
    System::Threading::{OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, SYNCHRONIZATION_SYNCHRONIZE},
};

fn open(pid: u32, access: u32) -> std::io::Result<OwnedHandle> {
    if pid == 0 { return Err(std::io::Error::from_raw_os_error(87)); }
    let handle = unsafe { OpenProcess(access, 0, pid) };
    if handle.is_null() { return Err(std::io::Error::last_os_error()); }
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

pub(crate) fn alive(pid: u32) -> bool {
    match open(pid, SYNCHRONIZATION_SYNCHRONIZE) {
        Ok(handle) => unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) == WAIT_TIMEOUT },
        Err(error) => error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32),
    }
}

pub(crate) fn parent_id() -> Option<u32> {
    use sysinfo::{Pid, ProcessesToUpdate, ProcessRefreshKind, System};
    let pid = Pid::from_u32(std::process::id());
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing());
    system.process(pid)?.parent().map(Pid::as_u32)
}

pub(crate) fn executable(pid: u32) -> Option<PathBuf> {
    let handle = open(pid, PROCESS_QUERY_LIMITED_INFORMATION).ok()?;
    let mut buffer = vec![0u16; 32768];
    let mut size = buffer.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle.as_raw_handle(), 0, buffer.as_mut_ptr(), &mut size) } == 0 { return None; }
    Some(std::ffi::OsString::from_wide(&buffer[..size as usize]).into())
}

pub(crate) fn terminate(pid: u32, descendants: bool) -> Result<(), String> {
    let pids = if descendants { crate::adaptor::Procs::snapshot().subtree(&[pid]) } else { vec![pid] };
    // Open before terminating: a reused numeric pid cannot change an owned handle.
    let mut handles = Vec::new();
    for pid in pids {
        match open(pid, PROCESS_TERMINATE | SYNCHRONIZATION_SYNCHRONIZE) {
            Ok(handle) => handles.push(handle),
            Err(error) if error.raw_os_error() == Some(87) => {}, // already exited
            Err(error) => return Err(error.to_string()),
        }
    }
    for handle in handles {
        if unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } != WAIT_TIMEOUT { continue; }
        if unsafe { TerminateProcess(handle.as_raw_handle(), 1) } == 0
            && unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } == WAIT_TIMEOUT {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_identity_and_exit_do_not_keep_dead_pids_alive() {
        let own = std::process::id();
        assert!(alive(own));
        assert!(executable(own).unwrap().is_file());
        assert!(parent_id().is_some_and(|pid| pid != own && alive(pid)));
        assert!(!alive(0));
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "ping -n 30 127.0.0.1 >NUL"])
            .spawn().unwrap();
        assert!(alive(child.id()));
        terminate(child.id(), true).unwrap();
        child.wait().unwrap();
        assert!(!alive(child.id()));
    }
}
