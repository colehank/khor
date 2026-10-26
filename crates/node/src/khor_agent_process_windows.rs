use super::{Pipes, SpawnProfile};
use std::{
    cmp::Ordering,
    ffi::{OsStr, OsString},
    io::{self, Read, Write},
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::ExitStatusExt,
    },
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    ptr::{null, null_mut},
    sync::Arc,
    thread,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_IO_PENDING, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE,
        HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    Globalization::{CSTR_EQUAL, CSTR_LESS_THAN, CompareStringOrdinal},
    Security::{
        Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom},
        SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, OPEN_EXISTING,
        PIPE_ACCESS_INBOUND, PIPE_ACCESS_OUTBOUND, ReadFile, WriteFile,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        JobObjects::{
            CreateJobObjectW, JOB_OBJECT_LIMIT_BREAKAWAY_OK, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
            QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
        },
        Pipes::{ConnectNamedPipe, CreateNamedPipeW, PIPE_REJECT_REMOTE_CLIENTS},
        Threading::{
            CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, CreateEventW, CreateProcessW,
            DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess,
            INFINITE, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION,
            STARTF_USESTDHANDLES, STARTUPINFOEXW, SetEvent, UpdateProcThreadAttribute,
            WaitForMultipleObjects, WaitForSingleObject,
        },
    },
};

pub(super) struct Process {
    process: OwnedHandle,
    job: Option<OwnedHandle>,
    cancel: Arc<OwnedHandle>,
    status: Option<ExitStatus>,
}

impl Process {
    pub(super) fn cancel_io(&mut self) {
        let _ = unsafe { SetEvent(self.cancel.as_raw_handle()) };
    }

    pub(super) fn exited(&mut self) -> io::Result<bool> {
        match unsafe { WaitForSingleObject(self.process.as_raw_handle(), 0) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(io::Error::last_os_error()),
        }
    }

    pub(super) fn terminate(&mut self) -> io::Result<()> {
        let job = self.job.as_ref().expect("process owns its job until drop");
        check(unsafe { TerminateJobObject(job.as_raw_handle(), 1) })
    }

    pub(super) fn wait(&mut self) -> io::Result<ExitStatus> {
        if unsafe { WaitForSingleObject(self.process.as_raw_handle(), INFINITE) } != WAIT_OBJECT_0 {
            return Err(io::Error::last_os_error());
        }
        let status = match self.status {
            Some(status) => status,
            None => {
                let mut code = 0;
                check(unsafe { GetExitCodeProcess(self.process.as_raw_handle(), &mut code) })?;
                let status = ExitStatus::from_raw(code);
                self.status = Some(status);
                status
            }
        };
        // A signalled leader does not imply its descendants have released inherited pipes.
        while self.active_processes()? != 0 {
            thread::sleep(Duration::from_millis(5));
        }
        Ok(status)
    }

    fn active_processes(&self) -> io::Result<u32> {
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        check(unsafe {
            QueryInformationJobObject(
                self.job.as_ref().unwrap().as_raw_handle(),
                JobObjectBasicAccountingInformation,
                (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                null_mut(),
            )
        })?;
        Ok(accounting.ActiveProcesses)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.cancel_io();
        // Even spawn-thread failure must confirm the leader has stopped before
        // returning the executor claim. Keep the owned job through the wait.
        if self.terminate().and_then(|_| self.wait()).is_err() {
            drop(self.job.take());
            unsafe {
                WaitForSingleObject(self.process.as_raw_handle(), INFINITE);
            }
        }
        drop(self.job.take());
    }
}

/// The caller supplies an env_clear command with regular args. Only explicit env entries
/// are used; this adapter deliberately never inherits the GUI's environment or invokes a shell.
pub(super) fn spawn(command: Command) -> io::Result<(Process, Pipes)> {
    spawn_with_profile(command, SpawnProfile::Contained)
}

pub(super) fn spawn_with_profile(
    command: Command,
    profile: SpawnProfile,
) -> io::Result<(Process, Pipes)> {
    let environment = explicit_environment(&command)?;
    let executable = resolve_executable(command.get_program(), &environment)?;
    let application = wide_z(executable.as_os_str())?;
    let mut command_line = command_line(executable.as_os_str(), command.get_args())?;
    let environment = environment_block(&environment)?;
    let directory = command
        .get_current_dir()
        .map(|dir| wide_z(dir.as_os_str()))
        .transpose()?;

    let job = owned(unsafe { CreateJobObjectW(null(), null()) })?;
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if matches!(profile, SpawnProfile::McpWorker) {
        limits.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_BREAKAWAY_OK;
    }
    check(unsafe {
        SetInformationJobObject(
            job.as_raw_handle(),
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    })?;

    let cancel = Arc::new(event()?);
    let (parent_stdin, child_stdin) = pipe(false, cancel.clone())?;
    let (parent_stdout, child_stdout) = pipe(true, cancel.clone())?;
    let (parent_stderr, child_stderr) = pipe(true, cancel.clone())?;
    let child_handles = [
        child_stdin.as_raw_handle(),
        child_stdout.as_raw_handle(),
        child_stderr.as_raw_handle(),
    ];
    let job_handles = [job.as_raw_handle()];
    let mut attributes = AttributeList::new()?;
    attributes.set(PROC_THREAD_ATTRIBUTE_HANDLE_LIST, &child_handles)?;
    attributes.set(PROC_THREAD_ATTRIBUTE_JOB_LIST, &job_handles)?;

    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = child_stdin.as_raw_handle();
    startup.StartupInfo.hStdOutput = child_stdout.as_raw_handle();
    startup.StartupInfo.hStdError = child_stderr.as_raw_handle();
    startup.lpAttributeList = attributes.as_mut_ptr();
    let mut info = PROCESS_INFORMATION::default();
    check(unsafe {
        CreateProcessW(
            application.as_ptr(),
            command_line.as_mut_ptr(),
            null(),
            null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
            environment.as_ptr().cast(),
            directory.as_ref().map_or(null(), |dir| dir.as_ptr()),
            &startup.StartupInfo,
            &mut info,
        )
    })?;
    // Both returned handles are owned immediately; no fallible work follows creation.
    let process = unsafe { OwnedHandle::from_raw_handle(info.hProcess) };
    let main_thread = unsafe { OwnedHandle::from_raw_handle(info.hThread) };
    drop(main_thread);
    drop(attributes);
    drop((child_stdin, child_stdout, child_stderr));
    Ok((
        Process {
            process,
            job: Some(job),
            cancel,
            status: None,
        },
        Pipes {
            stdin: Box::new(parent_stdin),
            stdout: Box::new(parent_stdout),
            stderr: Box::new(parent_stderr),
        },
    ))
}

fn check(result: i32) -> io::Result<()> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }
}

fn event() -> io::Result<OwnedHandle> {
    owned(unsafe { CreateEventW(null(), 1, 0, null()) })
}

fn pipe(parent_reads: bool, cancel: Arc<OwnedHandle>) -> io::Result<(Pipe, OwnedHandle)> {
    let mut nonce = [0u8; 16];
    let status = unsafe {
        BCryptGenRandom(
            null_mut(),
            nonce.as_mut_ptr(),
            nonce.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status < 0 {
        return Err(io::Error::other(format!(
            "pipe nonce generation failed: {status:#x}"
        )));
    }
    let nonce: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    let name = wide_z(OsStr::new(&format!(
        "\\\\.\\pipe\\khor-agent-{}-{nonce}",
        std::process::id()
    )))?;
    let server = owned(unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            (if parent_reads {
                PIPE_ACCESS_INBOUND
            } else {
                PIPE_ACCESS_OUTBOUND
            }) | FILE_FLAG_OVERLAPPED
                | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_REJECT_REMOTE_CLIENTS,
            1,
            8192,
            8192,
            0,
            null(),
        )
    })?;
    let security = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    let client = owned(unsafe {
        CreateFileW(
            name.as_ptr(),
            if parent_reads {
                GENERIC_WRITE
            } else {
                GENERIC_READ
            },
            0,
            &security,
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    })?;
    // A client may connect between CreateNamedPipe and ConnectNamedPipe; that is success.
    let completion = event()?;
    let mut overlap = OVERLAPPED::default();
    overlap.hEvent = completion.as_raw_handle();
    if unsafe { ConnectNamedPipe(server.as_raw_handle(), &mut overlap) } == 0 {
        let error = io::Error::last_os_error();
        match error.raw_os_error().map(|code| code as u32) {
            Some(ERROR_PIPE_CONNECTED) => {}
            Some(ERROR_IO_PENDING) => {
                let mut bytes = 0;
                check(unsafe {
                    GetOverlappedResult(server.as_raw_handle(), &overlap, &mut bytes, 1)
                })?;
            }
            _ => return Err(error),
        }
    }
    Ok((
        Pipe {
            handle: server,
            cancel,
        },
        client,
    ))
}

struct Pipe {
    handle: OwnedHandle,
    cancel: Arc<OwnedHandle>,
}

impl Pipe {
    fn cancelled(&self) -> bool {
        unsafe { WaitForSingleObject(self.cancel.as_raw_handle(), 0) == WAIT_OBJECT_0 }
    }

    fn transfer(&mut self, buffer: *mut u8, length: usize, read: bool) -> io::Result<usize> {
        let stopped = || {
            if read {
                Ok(0)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "agent pipe cancelled",
                ))
            }
        };
        if length == 0 {
            return Ok(0);
        }
        if self.cancelled() {
            return stopped();
        }
        let completion = event()?;
        let mut overlap = OVERLAPPED::default();
        overlap.hEvent = completion.as_raw_handle();
        let mut bytes = 0;
        let length = length.min(u32::MAX as usize) as u32;
        let result = unsafe {
            if read {
                ReadFile(
                    self.handle.as_raw_handle(),
                    buffer,
                    length,
                    &mut bytes,
                    &mut overlap,
                )
            } else {
                WriteFile(
                    self.handle.as_raw_handle(),
                    buffer.cast_const(),
                    length,
                    &mut bytes,
                    &mut overlap,
                )
            }
        };
        if result != 0 {
            check(unsafe {
                GetOverlappedResult(self.handle.as_raw_handle(), &overlap, &mut bytes, 1)
            })?;
            return Ok(bytes as usize);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
            return if read && error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
                Ok(0)
            } else {
                Err(error)
            };
        }
        let events = [self.cancel.as_raw_handle(), completion.as_raw_handle()];
        let waited = unsafe { WaitForMultipleObjects(2, events.as_ptr(), 0, INFINITE) };
        if waited != WAIT_OBJECT_0 + 1 {
            let wait_error = io::Error::last_os_error();
            // Cancellation is asynchronous: pin both OVERLAPPED and the caller's buffer
            // until its completion is acknowledged, even if the completion won the race.
            unsafe {
                CancelIoEx(self.handle.as_raw_handle(), &overlap);
                GetOverlappedResult(self.handle.as_raw_handle(), &overlap, &mut bytes, 1);
            }
            return if waited == WAIT_OBJECT_0 {
                stopped()
            } else {
                Err(wait_error)
            };
        }
        if unsafe { GetOverlappedResult(self.handle.as_raw_handle(), &overlap, &mut bytes, 1) } == 0
        {
            let error = io::Error::last_os_error();
            return if read && error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
                Ok(0)
            } else {
                Err(error)
            };
        }
        Ok(bytes as usize)
    }
}

impl Read for Pipe {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.transfer(buffer.as_mut_ptr(), buffer.len(), true)
    }
}

impl Write for Pipe {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.transfer(buffer.as_ptr().cast_mut(), buffer.len(), false)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct AttributeList {
    storage: Vec<usize>,
}

impl AttributeList {
    fn new() -> io::Result<Self> {
        let mut bytes = 0;
        unsafe { InitializeProcThreadAttributeList(null_mut(), 2, 0, &mut bytes) };
        if bytes == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut storage = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        check(unsafe {
            InitializeProcThreadAttributeList(storage.as_mut_ptr().cast(), 2, 0, &mut bytes)
        })?;
        Ok(Self { storage })
    }

    fn as_mut_ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.storage.as_mut_ptr().cast()
    }

    fn set(&mut self, attribute: u32, handles: &[HANDLE]) -> io::Result<()> {
        check(unsafe {
            UpdateProcThreadAttribute(
                self.as_mut_ptr(),
                0,
                attribute as usize,
                handles.as_ptr().cast(),
                std::mem::size_of_val(handles),
                null_mut(),
                null(),
            )
        })
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.as_mut_ptr()) };
    }
}

fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let value: Vec<u16> = value.encode_wide().collect();
    if value.contains(&0) {
        Err(invalid("process string contains NUL"))
    } else {
        Ok(value)
    }
}

fn wide_z(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut value = wide(value)?;
    value.push(0);
    Ok(value)
}

fn command_line<'a>(
    program: &'a OsStr,
    arguments: impl Iterator<Item = &'a OsStr>,
) -> io::Result<Vec<u16>> {
    let mut line = Vec::new();
    for argument in std::iter::once(program).chain(arguments) {
        if !line.is_empty() {
            line.push(b' ' as u16);
        }
        line.push(b'"' as u16);
        let mut slashes = 0;
        for unit in wide(argument)? {
            if unit == b'\\' as u16 {
                slashes += 1;
            } else {
                line.extend(std::iter::repeat_n(
                    b'\\' as u16,
                    slashes * if unit == b'"' as u16 { 2 } else { 1 },
                ));
                if unit == b'"' as u16 {
                    line.push(b'\\' as u16);
                }
                line.push(unit);
                slashes = 0;
            }
        }
        line.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
        line.push(b'"' as u16);
    }
    line.push(0);
    if line.len() > 32_767 {
        return Err(invalid("process command line is too long"));
    }
    Ok(line)
}

fn environment_order(left: &[u16], right: &[u16]) -> Ordering {
    match unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left.len() as i32,
            right.as_ptr(),
            right.len() as i32,
            1,
        )
    } {
        CSTR_LESS_THAN => Ordering::Less,
        CSTR_EQUAL => Ordering::Equal,
        _ => Ordering::Greater,
    }
}

fn explicit_environment(command: &Command) -> io::Result<Vec<(Vec<u16>, OsString)>> {
    let mut entries = Vec::new();
    for (key, value) in command.get_envs() {
        let Some(value) = value else { continue };
        let key = wide(key)?;
        if key.is_empty() || key.contains(&(b'=' as u16)) {
            return Err(invalid("invalid environment name"));
        }
        wide(value)?;
        entries.push((key, value.to_os_string()));
    }
    entries.sort_by(|left, right| environment_order(&left.0, &right.0));
    Ok(entries)
}

fn environment_block(entries: &[(Vec<u16>, OsString)]) -> io::Result<Vec<u16>> {
    let mut block = Vec::new();
    for (key, value) in entries {
        block.extend_from_slice(key);
        block.push(b'=' as u16);
        block.extend(wide(value)?);
        block.push(0);
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    Ok(block)
}

fn resolve_executable(
    program: &OsStr,
    environment: &[(Vec<u16>, OsString)],
) -> io::Result<PathBuf> {
    let units = wide(program)?;
    if units.is_empty() || units.contains(&(b'"' as u16)) {
        return Err(invalid("invalid process program"));
    }
    let program = Path::new(program);
    if program.file_name().is_none() {
        return Err(invalid("process program has no file name"));
    }
    if program
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
    {
        return Err(invalid(
            "batch sidecars require an explicit command interpreter",
        ));
    }
    let with_exe = |path: &Path| {
        let mut value = path.as_os_str().to_os_string();
        value.push(".exe");
        PathBuf::from(value)
    };
    if program.components().count() > 1 {
        let candidate = if program.extension().is_none() && with_exe(program).is_file() {
            with_exe(program)
        } else {
            program.to_path_buf()
        };
        return std::path::absolute(candidate);
    }
    let path_name: Vec<u16> = "PATH".encode_utf16().collect();
    let path = environment
        .iter()
        .find(|entry| environment_order(&entry.0, &path_name) == Ordering::Equal);
    if let Some((_, paths)) = path {
        for directory in std::env::split_paths(paths).filter(|path| !path.as_os_str().is_empty()) {
            let candidate = directory.join(program);
            let candidate = if program.extension().is_none() {
                with_exe(&candidate)
            } else {
                candidate
            };
            if candidate.is_file() {
                return std::path::absolute(candidate);
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "sidecar program not found in explicit PATH",
    ))
}
