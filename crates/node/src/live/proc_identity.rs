//! Finite read-only facts about the current ghost and its registered foreground
//! child/group. Never enumerate processes, signal a process, or infer a writer
//! from a PID alone. Failure/permission/namespace ambiguity remains Unknown.
use super::recovery_attempt::{ProcessStamp, ProcessStart};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Unknown;

#[derive(Clone, PartialEq, Eq)]
struct Context {
    boot: String,
    namespace: Namespace,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Namespace {
    MacGlobal,
    Linux { device: u64, inode: u64 },
}
impl Context {
    fn same_platform(&self, value: &ProcessStamp) -> bool {
        matches!(
            (self.namespace, value.start),
            (Namespace::MacGlobal, ProcessStart::Mac { .. })
                | (Namespace::Linux { .. }, ProcessStart::Linux { .. })
        )
    }
    fn contains(&self, value: &ProcessStamp) -> bool {
        if self.boot != value.boot {
            return false;
        }
        match (self.namespace, value.start) {
            (Namespace::MacGlobal, ProcessStart::Mac { .. }) => true,
            (
                Namespace::Linux { device, inode },
                ProcessStart::Linux {
                    namespace_device,
                    namespace_inode,
                    ..
                },
            ) => device == namespace_device && inode == namespace_inode,
            _ => false,
        }
    }
}

/// Only this module can construct this ephemeral proof. It is scoped to the
/// recorded foreground group, not detached daemons or a remote upstream host.
pub(super) struct AbsentForeground {
    recorded: ProcessStamp,
    basis: Absence,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Absence {
    PreviousBoot,
    LeaderAndGroupAbsent,
}
impl AbsentForeground {
    pub(super) fn is_for(&self, recorded: &ProcessStamp) -> bool {
        &self.recorded == recorded
    }
    pub(super) fn basis(&self) -> Absence {
        self.basis
    }
}
pub(super) enum Foreground {
    Present,
    Absent(AbsentForeground),
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Leader {
    Same,
    Missing,
    Reused,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    Present,
    Absent,
    Unknown,
}

/// The caller's own PID is not supplied by a remote client or row metadata.
pub(super) fn current_host() -> Result<ProcessStamp, Unknown> {
    capture(std::process::id())?.ok_or(Unknown)
}

/// actual_pid must come from this invocation's owned Session/Transport. A
/// script/launcher name or descendant search is not an admissible substitute.
pub(super) fn child(actual_pid: Option<u32>, host: &ProcessStamp) -> Result<ProcessStamp, Unknown> {
    host.validate().map_err(|_| Unknown)?;
    if host.pid != std::process::id() {
        return Err(Unknown);
    }
    if !same_birth(host, &current_host()?) {
        return Err(Unknown);
    }
    let value = capture(actual_pid.ok_or(Unknown)?)?.ok_or(Unknown)?;
    if value.parent != host.pid || value.group != value.pid || !same_context(host, &value) {
        return Err(Unknown);
    }
    Ok(value)
}

/// Inspect only a previously registered exact foreground owner. A same-boot
/// absent/reused leader still requires the known PGID to be positively absent.
pub(super) fn foreground(recorded: &ProcessStamp) -> Foreground {
    if recorded.validate().is_err() || recorded.group != recorded.pid {
        return Foreground::Unknown;
    }
    let before = match context() {
        Ok(value) => value,
        Err(_) => return Foreground::Unknown,
    };
    if !before.same_platform(recorded) {
        return Foreground::Unknown;
    }
    if before.boot != recorded.boot {
        // No new PID/group lookup in a different boot can be rebound to the
        // old identity. This proves only the old local foreground is gone.
        return Foreground::Absent(AbsentForeground {
            recorded: recorded.clone(),
            basis: Absence::PreviousBoot,
        });
    }
    if !before.contains(recorded) {
        return Foreground::Unknown;
    }
    let leader = match capture(recorded.pid) {
        Ok(Some(current)) if same_birth(recorded, &current) => Leader::Same,
        Ok(Some(current)) if same_context(recorded, &current) => Leader::Reused,
        Ok(None) => Leader::Missing,
        _ => Leader::Unknown,
    };
    // Do not probe a group if its exact leader is still present or unreadable.
    let group = if matches!(leader, Leader::Missing | Leader::Reused) {
        known_group(recorded.group)
    } else {
        Group::Unknown
    };
    if context().as_ref() != Ok(&before) {
        return Foreground::Unknown;
    }
    decide(recorded, leader, group)
}
fn decide(recorded: &ProcessStamp, leader: Leader, group: Group) -> Foreground {
    match (leader, group) {
        (Leader::Same, _) => Foreground::Present,
        (Leader::Missing | Leader::Reused, Group::Absent) => Foreground::Absent(AbsentForeground {
            recorded: recorded.clone(),
            basis: Absence::LeaderAndGroupAbsent,
        }),
        _ => Foreground::Unknown,
    }
}
fn same_birth(a: &ProcessStamp, b: &ProcessStamp) -> bool {
    a.pid == b.pid && same_context(a, b) && a.start == b.start
}
pub(super) fn same_context(a: &ProcessStamp, b: &ProcessStamp) -> bool {
    if a.boot != b.boot {
        return false;
    }
    match (a.start, b.start) {
        (ProcessStart::Mac { .. }, ProcessStart::Mac { .. }) => true,
        (
            ProcessStart::Linux {
                namespace_device: a_dev,
                namespace_inode: a_ino,
                ..
            },
            ProcessStart::Linux {
                namespace_device: b_dev,
                namespace_inode: b_ino,
                ..
            },
        ) => a_dev == b_dev && a_ino == b_ino,
        _ => false,
    }
}
fn valid_pid(pid: u32) -> bool {
    pid > 1 && pid <= i32::MAX as u32
}

fn capture(pid: u32) -> Result<Option<ProcessStamp>, Unknown> {
    if !valid_pid(pid) {
        return Err(Unknown);
    }
    let before = context()?;
    let first = process(pid, &before)?;
    let second = process(pid, &before)?;
    if context()? != before {
        return Err(Unknown);
    }
    match (first, second) {
        (Some(a), Some(b)) if a == b => {
            b.validate().map_err(|_| Unknown)?;
            Ok(Some(b))
        }
        (None, None) => Ok(None),
        _ => Err(Unknown),
    }
}
fn uuid(bytes: &[u8]) -> Result<String, Unknown> {
    let bytes = bytes.strip_suffix(b"\0").unwrap_or(bytes);
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Unknown)?
        .trim_end_matches(['\r', '\n']);
    if text.len() != 36 {
        return Err(Unknown);
    }
    let mut result = String::with_capacity(32);
    for (at, byte) in text.bytes().enumerate() {
        if [8, 13, 18, 23].contains(&at) {
            if byte != b'-' {
                return Err(Unknown);
            }
        } else if byte.is_ascii_hexdigit() {
            result.push(byte.to_ascii_lowercase() as char);
        } else {
            return Err(Unknown);
        }
    }
    Ok(result)
}

#[cfg(target_os = "macos")]
fn context() -> Result<Context, Unknown> {
    let mut bytes = [0u8; 64];
    let mut length = bytes.len();
    let result = unsafe {
        libc::sysctlbyname(
            c"kern.bootsessionuuid".as_ptr(),
            bytes.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if result != 0 || length == 0 || length > bytes.len() {
        return Err(Unknown);
    }
    Ok(Context {
        boot: uuid(&bytes[..length])?,
        namespace: Namespace::MacGlobal,
    })
}
#[cfg(target_os = "macos")]
fn process(pid: u32, context: &Context) -> Result<Option<ProcessStamp>, Unknown> {
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&info);
    unsafe {
        *libc::__error() = 0;
    }
    let count = unsafe {
        libc::proc_pidinfo(
            pid as libc::pid_t,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size as libc::c_int,
        )
    };
    if count != size as libc::c_int {
        return if std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            && missing_pid(pid)
        {
            Ok(None)
        } else {
            Err(Unknown)
        };
    }
    if info.pbi_pid != pid {
        return Err(Unknown);
    }
    Ok(Some(ProcessStamp {
        pid,
        parent: info.pbi_ppid,
        group: info.pbi_pgid,
        boot: context.boot.clone(),
        start: ProcessStart::Mac {
            seconds: info.pbi_start_tvsec,
            microseconds: u32::try_from(info.pbi_start_tvusec).map_err(|_| Unknown)?,
        },
    }))
}
#[cfg(target_os = "macos")]
fn known_group(group: u32) -> Group {
    if !valid_pid(group) {
        return Group::Unknown;
    }
    let mut one = [0 as libc::pid_t; 1];
    unsafe {
        *libc::__error() = 0;
    }
    let count = unsafe {
        libc::proc_listpgrppids(
            group as libc::pid_t,
            one.as_mut_ptr().cast(),
            std::mem::size_of_val(&one) as libc::c_int,
        )
    };
    match (count, std::io::Error::last_os_error().raw_os_error()) {
        (0, Some(0)) => Group::Absent,
        (n, _) if n > 0 => Group::Present,
        _ => Group::Unknown,
    }
}

#[cfg(target_os = "linux")]
fn context() -> Result<Context, Unknown> {
    use std::os::unix::fs::MetadataExt;
    let boot = bounded_file(std::path::Path::new("/proc/sys/kernel/random/boot_id"), 64)?;
    let namespace = std::fs::metadata("/proc/self/ns/pid").map_err(|_| Unknown)?;
    if namespace.ino() == 0 {
        return Err(Unknown);
    }
    Ok(Context {
        boot: uuid(&boot)?,
        namespace: Namespace::Linux {
            device: namespace.dev(),
            inode: namespace.ino(),
        },
    })
}
#[cfg(target_os = "linux")]
fn process(pid: u32, context: &Context) -> Result<Option<ProcessStamp>, Unknown> {
    use std::os::unix::fs::MetadataExt;
    let file = match std::fs::File::open(format!("/proc/{pid}/stat")) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && missing_pid(pid) => {
            return Ok(None);
        }
        Err(_) => return Err(Unknown),
    };
    let bytes = bounded(file, 64 * 1024)?;
    let namespace = std::fs::metadata(format!("/proc/{pid}/ns/pid")).map_err(|_| Unknown)?;
    if context.namespace
        != (Namespace::Linux {
            device: namespace.dev(),
            inode: namespace.ino(),
        })
    {
        return Err(Unknown);
    }
    parse_linux(pid, &bytes, context)
}
#[cfg(any(target_os = "linux", test))]
fn parse_linux(pid: u32, bytes: &[u8], context: &Context) -> Result<Option<ProcessStamp>, Unknown> {
    let open = bytes.iter().position(|byte| *byte == b'(').ok_or(Unknown)?;
    let close = bytes
        .iter()
        .rposition(|byte| *byte == b')')
        .ok_or(Unknown)?;
    if open >= close {
        return Err(Unknown);
    }
    let actual = std::str::from_utf8(&bytes[..open])
        .map_err(|_| Unknown)?
        .trim()
        .parse::<u32>()
        .map_err(|_| Unknown)?;
    if actual != pid {
        return Err(Unknown);
    }
    let mut fields = std::str::from_utf8(&bytes[close + 1..])
        .map_err(|_| Unknown)?
        .split_ascii_whitespace();
    let state = fields.next().ok_or(Unknown)?;
    if state.len() != 1 || !state.as_bytes()[0].is_ascii_alphabetic() {
        return Err(Unknown);
    }
    let parent = fields.next().ok_or(Unknown)?.parse().map_err(|_| Unknown)?;
    let group = fields.next().ok_or(Unknown)?.parse().map_err(|_| Unknown)?;
    let ticks = fields
        .nth(16)
        .ok_or(Unknown)?
        .parse()
        .map_err(|_| Unknown)?;
    let Namespace::Linux { device, inode } = context.namespace else {
        return Err(Unknown);
    };
    let value = ProcessStamp {
        pid,
        parent,
        group,
        boot: context.boot.clone(),
        start: ProcessStart::Linux {
            ticks,
            namespace_device: device,
            namespace_inode: inode,
        },
    };
    value.validate().map_err(|_| Unknown)?;
    Ok(Some(value))
}
#[cfg(target_os = "linux")]
fn known_group(group: u32) -> Group {
    if !valid_pid(group) {
        return Group::Unknown;
    }
    // Signal zero performs only the existence/permission check for this one
    // registered group. No signal is delivered and no member is enumerated.
    let result = unsafe { libc::kill(-(group as libc::pid_t), 0) };
    if result == 0 {
        Group::Present
    } else if std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        Group::Absent
    } else {
        Group::Unknown
    }
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn missing_pid(pid: u32) -> bool {
    valid_pid(pid)
        && unsafe { libc::getpgid(pid as libc::pid_t) } == -1
        && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}
#[cfg(target_os = "linux")]
fn bounded_file(path: &std::path::Path, limit: usize) -> Result<Vec<u8>, Unknown> {
    bounded(std::fs::File::open(path).map_err(|_| Unknown)?, limit)
}
#[cfg(target_os = "linux")]
fn bounded(file: std::fs::File, limit: usize) -> Result<Vec<u8>, Unknown> {
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Unknown)?;
    if bytes.len() > limit {
        Err(Unknown)
    } else {
        Ok(bytes)
    }
}
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn context() -> Result<Context, Unknown> {
    Err(Unknown)
}
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn process(_: u32, _: &Context) -> Result<Option<ProcessStamp>, Unknown> {
    Err(Unknown)
}
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn known_group(_: u32) -> Group {
    Group::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_birth_namespace_and_group_facts_do_not_conflate_pid_reuse_with_an_owner() {
        let context = Context {
            boot: "a".repeat(32),
            namespace: Namespace::Linux {
                device: 4,
                inode: 99,
            },
        };
        let mut fields = vec!["0"; 20];
        fields[0] = "S";
        fields[1] = "10";
        fields[2] = "20";
        fields[19] = "12345";
        let mut raw = b"20 (name ) ( \xff) ".to_vec();
        raw.extend(fields.join(" ").as_bytes());
        let owner = parse_linux(20, &raw, &context).unwrap().unwrap();
        assert_eq!(owner.parent, 10);
        assert_eq!(owner.group, 20);
        assert!(
            owner.start
                == ProcessStart::Linux {
                    ticks: 12345,
                    namespace_device: 4,
                    namespace_inode: 99
                }
        );
        assert!(context.contains(&owner));
        assert!(parse_linux(21, &raw, &context).is_err());
        let mut reused = owner.clone();
        reused.start = ProcessStart::Linux {
            ticks: 12346,
            namespace_device: 4,
            namespace_inode: 99,
        };
        assert!(!same_birth(&owner, &reused));
        assert!(same_context(&owner, &reused));
        let elsewhere = Context {
            namespace: Namespace::Linux {
                device: 4,
                inode: 100,
            },
            ..context.clone()
        };
        assert!(!elsewhere.contains(&owner));
        assert!(
            !Context {
                namespace: Namespace::MacGlobal,
                ..context.clone()
            }
            .same_platform(&owner)
        );
        assert!(
            !Context {
                boot: "b".repeat(32),
                ..context
            }
            .contains(&owner)
        );
        assert!(matches!(
            decide(&owner, Leader::Missing, Group::Unknown),
            Foreground::Unknown
        ));
        assert!(matches!(
            decide(&owner, Leader::Missing, Group::Present),
            Foreground::Unknown
        ));
        assert!(matches!(
            decide(&owner, Leader::Reused, Group::Unknown),
            Foreground::Unknown
        ));
        assert!(matches!(
            decide(&owner, Leader::Same, Group::Absent),
            Foreground::Present
        ));
        match decide(&owner, Leader::Reused, Group::Absent) {
            Foreground::Absent(proof) => {
                assert!(proof.is_for(&owner));
                assert!(!proof.is_for(&reused));
                assert_eq!(proof.basis(), Absence::LeaderAndGroupAbsent);
            }
            _ => panic!("expected the exact old foreground absence proof"),
        }
        assert_eq!(
            uuid(b"12345678-ABCD-1234-ABCD-1234567890AB\0").unwrap(),
            "12345678abcd1234abcd1234567890ab"
        );
        assert!(uuid(b"12345678-abcd-1234-abcd-1234567890ab\n").is_ok());
        for value in [
            b"".as_slice(),
            b" 12345678-abcd-1234-abcd-1234567890ab",
            b"12345678abcd1234abcd1234567890ab",
            b"12345678-abcd-1234-abcd-1234567890az",
        ] {
            assert!(uuid(value).is_err());
        }
        assert!(!valid_pid(0));
        assert!(!valid_pid(1));
        assert!(!valid_pid(u32::MAX));
        assert!(valid_pid(2));
    }
}

#[cfg(all(test, target_os = "macos"))]
mod host_crash_tests {
    use super::*;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self, File},
        io::{Read, Write},
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::{Path, PathBuf},
        process::{Child, Command, Stdio},
        time::{Duration, Instant},
    };

    fn read(path: &Path) -> Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }
    fn try_save(path: &Path, value: Value) -> Result<(), String> {
        let mut file = File::create(path).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&mut file, &value).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        File::open(path.parent().ok_or("evidence parent missing")?)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| e.to_string())
    }
    fn save(path: &Path, value: Value) {
        try_save(path, value).unwrap();
    }
    fn hash(path: &Path) -> String {
        let mut source = File::open(path).unwrap();
        let mut digest = Sha256::new();
        let mut bytes = [0; 65536];
        loop {
            let n = source.read(&mut bytes).unwrap();
            if n == 0 {
                break;
            }
            digest.update(&bytes[..n]);
        }
        format!("{:x}", digest.finalize())
    }
    fn executable(name: &str) -> PathBuf {
        let path =
            PathBuf::from(std::env::var_os(name).expect("explicit actual producer path required"));
        assert!(path.is_absolute() && fs::canonicalize(&path).unwrap() == path);
        let info = fs::symlink_metadata(&path).unwrap();
        assert!(info.is_file() && info.permissions().mode() & 0o111 != 0);
        path
    }
    fn wait(mut condition: impl FnMut() -> bool, seconds: u64, why: &str) {
        let until = Instant::now() + Duration::from_secs(seconds);
        loop {
            let complete = condition();
            assert!(Instant::now() < until, "{why}");
            if complete {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[derive(Clone, serde::Serialize)]
    struct Sample {
        stamp: ProcessStamp,
        executable: PathBuf,
        uid: u32,
        argv: Vec<String>,
    }
    // Query only a previously registered PID. Kernel birth/context remains the
    // existing native owner; sysinfo supplies image/UID/argv, not a second epoch.
    fn sample(expected: &ProcessStamp, image: &Path) -> Result<Sample, String> {
        let first = capture(expected.pid)
            .map_err(|_| "birth unavailable")?
            .ok_or("PID absent")?;
        if !same_birth(expected, &first) || first.group != expected.group {
            return Err("registered process identity changed".into());
        }
        let pid = sysinfo::Pid::from_u32(expected.pid);
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[pid]),
            true,
            sysinfo::ProcessRefreshKind::nothing()
                .with_exe(sysinfo::UpdateKind::Always)
                .with_cmd(sysinfo::UpdateKind::Always)
                .with_user(sysinfo::UpdateKind::Always),
        );
        let process = system.process(pid).ok_or("registered PID disappeared")?;
        let actual = process
            .exe()
            .ok_or("process image unavailable")?
            .to_path_buf();
        let uid = **process.user_id().ok_or("process UID unavailable")?;
        if actual != image || uid != unsafe { libc::geteuid() } {
            return Err("process image or UID changed".into());
        }
        let argv = process
            .cmd()
            .iter()
            .map(|v| {
                v.to_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "non-UTF8 fixture argv".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let last = capture(expected.pid)
            .map_err(|_| "birth unavailable after sample")?
            .ok_or("PID gone after sample")?;
        if first != last {
            return Err("process changed during observation".into());
        }
        Ok(Sample {
            stamp: last,
            executable: actual,
            uid,
            argv,
        })
    }

    struct Fixture {
        evidence: PathBuf,
        root: PathBuf,
        work: PathBuf,
        tmp: PathBuf,
        cli: PathBuf,
        bridge_bin: PathBuf,
        python: PathBuf,
        bridge: Option<Child>,
        host: Option<ProcessStamp>,
        provider: Option<ProcessStamp>,
        row: Option<PathBuf>,
        old_meta: Option<Value>,
        crashed: bool,
        crash_files: Vec<(PathBuf, Value)>,
        port: u16,
        http: reqwest::Client,
        calls: Vec<Value>,
        started: Instant,
    }
    impl Fixture {
        fn new() -> Self {
            let evidence = PathBuf::from(
                std::env::var_os("KHOR_ACP_TEST_EVIDENCE")
                    .expect("owned evidence directory required"),
            )
            .join("host-crash-provider-live");
            assert!(evidence.is_absolute());
            fs::create_dir(&evidence).unwrap();
            let root = evidence.join("store");
            let work = evidence.join("work");
            let tmp = evidence.join("child-tmp");
            for path in [&root, &work, &tmp] {
                fs::create_dir(path).unwrap();
            }
            let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let port = listener.local_addr().unwrap().port();
            drop(listener);
            Self {
                evidence,
                root,
                work,
                tmp,
                cli: executable("KHOR_TEST_NATIVE_CLI"),
                bridge_bin: executable("KHOR_TEST_NATIVE_BRIDGE"),
                python: executable("KHOR_ACP_TEST_PYTHON"),
                bridge: None,
                host: None,
                provider: None,
                row: None,
                old_meta: None,
                crashed: false,
                crash_files: Vec::new(),
                port,
                http: reqwest::Client::builder().no_proxy().build().unwrap(),
                calls: Vec::new(),
                started: Instant::now(),
            }
        }
        fn command(&self, binary: &Path) -> Command {
            let mut command = Command::new(binary);
            command
                .env_clear()
                .current_dir(&self.work)
                .env(
                    "HOME",
                    std::env::var_os("HOME").expect("preserve actual HOME"),
                )
                .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin")
                .env(
                    "KHOR_PATH",
                    "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin",
                )
                .env("KHOR_HOME", &self.root)
                .env("TMPDIR", &self.tmp)
                .env("LANG", "en_US.UTF-8")
                .env("LC_ALL", "en_US.UTF-8")
                .env("SHELL", "/bin/sh")
                .env("KHOR_NO_MDNS", "1")
                .env("PYTHONDONTWRITEBYTECODE", "1")
                .stdin(Stdio::null());
            for name in ["USER", "LOGNAME"] {
                if let Some(value) = std::env::var_os(name) {
                    command.env(name, value);
                }
            }
            command
        }
        fn cli(&self, label: &str, arguments: &[String]) -> (u32, String) {
            let out = self.evidence.join(format!("{label}.stdout"));
            let mut child = self
                .command(&self.cli)
                .args(arguments)
                .stdout(File::create(&out).unwrap())
                .stderr(File::create(self.evidence.join(format!("{label}.stderr"))).unwrap())
                .spawn()
                .unwrap();
            let pid = child.id();
            let until = Instant::now() + Duration::from_secs(35);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if Instant::now() >= until {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("{label} exceeded original35s private CLI bound");
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            assert!(status.success(), "{label} failed: {status}");
            let bytes = fs::read(out).unwrap();
            assert!(bytes.len() <= 64 * 1024);
            (pid, String::from_utf8(bytes).unwrap())
        }
        async fn api(&mut self, method: &str, payload: Value, budget: Duration) -> Value {
            assert!(!budget.is_zero() && self.started.elapsed() < Duration::from_secs(120));
            let until = Instant::now() + budget;
            let mut response = self
                .http
                .post(format!("http://127.0.0.1:{}/{method}", self.port))
                .timeout(budget)
                .json(&payload)
                .send()
                .await
                .unwrap();
            let status = response.status();
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.unwrap() {
                assert!(body.len() + chunk.len() <= 4 * 1024 * 1024);
                body.extend_from_slice(&chunk);
            }
            self.calls.push(
                json!({"method": method, "payload": payload, "status": status.as_u16(),
                                   "body": String::from_utf8_lossy(&body)}),
            );
            save(&self.evidence.join("http.json"), json!(self.calls));
            assert!(
                Instant::now() < until,
                "{method} exceeded original request bound"
            );
            assert!(
                status.is_success(),
                "{method}: {}",
                String::from_utf8_lossy(&body)
            );
            serde_json::from_slice(&body).unwrap()
        }
        fn counts(&self) -> Value {
            let bytes = fs::read_to_string(self.work.join("stdin-stall/received.jsonl")).unwrap();
            let rows: Vec<Value> = bytes
                .lines()
                .map(|v| serde_json::from_str(v).unwrap())
                .collect();
            let mut counts = serde_json::Map::new();
            for method in [
                "initialize",
                "session/new",
                "session/load",
                "session/resume",
                "session/prompt",
            ] {
                counts.insert(
                    method.into(),
                    json!(rows.iter().filter(|v| v["method"] == method).count()),
                );
            }
            Value::Object(counts)
        }
        fn tmp_before_crash(&mut self) {
            let host = self.host.as_ref().unwrap();
            for entry in fs::read_dir(&self.tmp).unwrap() {
                let path = entry.unwrap().path();
                let name = path.file_name().unwrap().to_str().unwrap();
                assert!(name.starts_with(&format!("khor-acp-events-{}-", host.pid)));
                let meta = fs::symlink_metadata(&path).unwrap();
                assert!(meta.is_dir() && meta.uid() == unsafe { libc::geteuid() });
                self.crash_files.push((path.clone(), json!({"dev": meta.dev(), "ino": meta.ino(), "uid": meta.uid(), "directory": true})));
                for file in fs::read_dir(path).unwrap() {
                    let file = file.unwrap().path();
                    let meta = fs::symlink_metadata(&file).unwrap();
                    assert!(
                        meta.is_file()
                            && meta.nlink() == 1
                            && meta.uid() == unsafe { libc::geteuid() }
                    );
                    assert!(file.extension().is_some_and(|x| x == "part"));
                    self.crash_files.push((
                        file.clone(),
                        json!({"dev": meta.dev(), "ino": meta.ino(), "uid": meta.uid(),
                        "directory": false, "bytes": meta.len(), "sha256": hash(&file)}),
                    ));
                }
            }
            assert!(self.crash_files.len() <= 8);
            save(
                &self.evidence.join("crash-parts-before.json"),
                json!(self.crash_files),
            );
        }
        async fn run(&mut self) {
            let peer = self.work.join("peer.py");
            fs::write(&peer, include_bytes!("../fixtures/acp-cancel.py")).unwrap();
            let marker = self.work.join("stdin-stall");
            let agent = format!("s3-host-crash-{}", crate::link::fresh_hex().unwrap());
            self.cli(
                "register",
                &[
                    "agents".into(),
                    "add".into(),
                    agent.clone(),
                    "--".into(),
                    self.python.display().to_string(),
                    peer.display().to_string(),
                    "stdin-stall".into(),
                    marker.display().to_string(),
                ],
            );
            let (seed_pid, opened) = self.cli(
                "seed",
                &[
                    "open".into(),
                    "-d".into(),
                    "--agent".into(),
                    agent.clone(),
                    "--title".into(),
                    "S3 controlled host crash".into(),
                ],
            );
            let ids: Vec<_> = opened.lines().filter(|v| v.starts_with("tui/")).collect();
            assert_eq!(ids.len(), 1);
            let sid = ids[0].to_owned();
            let rows: Vec<_> = fs::read_dir(self.root.join(".khor/sessions"))
                .unwrap()
                .map(|v| v.unwrap().path())
                .filter(|p| {
                    p.join("meta.json").is_file() && read(&p.join("meta.json"))["id"] == sid
                })
                .collect();
            assert_eq!(rows.len(), 1);
            let row = rows[0].clone();
            self.row = Some(row.clone());
            let meta = read(&row.join("meta.json"));
            let host_file = read(&row.join("host.json"));
            let attempt = read(&row.join("recovery-attempt.json"));
            let host: ProcessStamp =
                serde_json::from_value(attempt["latest"]["host"].clone()).unwrap();
            let provider: ProcessStamp =
                serde_json::from_value(attempt["latest"]["provider"]["process"].clone()).unwrap();
            self.host = Some(host.clone());
            self.provider = Some(provider.clone());
            self.old_meta = Some(meta.clone());
            assert_eq!(meta["id"], sid);
            assert_eq!(meta["kind"], "gui");
            assert_eq!(meta["gui_native"]["agent"], agent);
            assert_eq!(meta["gui_native"]["native_id"], "synthetic-q037-session");
            assert_eq!(meta["pid"], host.pid);
            assert_eq!(host_file["host_pid"], host.pid);
            assert_eq!(host_file["child_pid"], host.pid);
            assert_eq!(attempt["latest"]["host"]["pid"], host.pid);
            assert_eq!(attempt["latest"]["phase"], "run_published");
            assert_eq!(attempt["latest"]["run_id"], meta["gui_run"]["id"]);
            assert_eq!(attempt["latest"]["arguments"]["native"], meta["gui_native"]);
            let peer_pid = read(&marker.join("pid.json"));
            assert_eq!(peer_pid["pid"], provider.pid);
            assert_eq!(peer_pid["pgid"], provider.group);
            assert_eq!(host.parent, seed_pid);
            assert_eq!(host.group, host.pid);
            assert_eq!(provider.parent, host.pid);
            assert_eq!(provider.group, provider.pid);
            assert_eq!(attempt["latest"]["provider"]["state"], "owned");
            assert_eq!(
                attempt["latest"]["support"],
                json!({"load": true, "resume": false})
            );
            let before_host = sample(&host, &self.cli).unwrap();
            let before_provider = sample(&provider, &self.python).unwrap();
            assert_eq!(before_provider.stamp.parent, host.pid);
            assert_eq!(
                before_provider.argv,
                vec![
                    self.python.display().to_string(),
                    peer.display().to_string(),
                    "stdin-stall".into(),
                    marker.display().to_string()
                ]
            );
            assert_eq!(before_host.argv[0], self.cli.display().to_string());
            assert_eq!(before_host.argv[1], "_ghost");
            assert!(Path::new(&before_host.argv[2]).starts_with(&self.tmp));
            assert_eq!(before_host.argv[3], "S3 controlled host crash");
            assert_eq!(before_host.argv[4], "--");
            let config: Value = serde_json::from_str(&before_host.argv[5]).unwrap();
            assert_eq!(config["command"], self.python.display().to_string());
            assert_eq!(config["args"], json!([peer, "stdin-stall", marker]));
            save(
                &self.evidence.join("owned-before.json"),
                json!({"meta": meta, "host_file": host_file, "attempt": attempt,
                 "host": before_host, "provider": before_provider}),
            );
            self.bridge = Some(
                self.command(&self.bridge_bin)
                    .env("BRIDGE_PORT", self.port.to_string())
                    .stdout(File::create(self.evidence.join("bridge.stdout")).unwrap())
                    .stderr(File::create(self.evidence.join("bridge.stderr")).unwrap())
                    .spawn()
                    .unwrap(),
            );
            wait(
                || std::net::TcpStream::connect(("127.0.0.1", self.port)).is_ok(),
                10,
                "bridge readiness",
            );
            let native_home: Vec<u8> =
                serde_json::from_value(meta["gui_native"]["home"].clone()).unwrap();
            let home: String = native_home.iter().map(|v| format!("{v:02x}")).collect();
            let run = meta["gui_run"]["id"].as_str().unwrap().to_owned();
            let offer = self
                .api(
                    "session_recovery",
                    json!({"home_id": home, "id": sid}),
                    Duration::from_secs(10),
                )
                .await;
            assert_eq!(offer["execution"], "live");
            assert_eq!(offer["action"], "join");
            assert_eq!(offer["id"], sid);
            assert_eq!(offer["home"], home);
            assert_eq!(offer["current_run"], run);
            let joined = self.api("recover_session", json!({"home_id": home, "id": sid, "expected_run": run,
                "token": {"epoch": offer["next_attempt_epoch"], "nonce": crate::link::fresh_hex().unwrap()}}), Duration::from_secs(31)).await;
            assert_eq!(
                joined,
                json!({"kind": "join", "id": sid, "current_run": run})
            );
            let lease = self
                .api(
                    "attachment_begin",
                    json!({"id": sid, "kind": "chat"}),
                    Duration::from_secs(10),
                )
                .await;
            assert_eq!(
                self.api(
                    "chat_open",
                    json!({"id": sid, "lease": lease}),
                    Duration::from_secs(10)
                )
                .await,
                true
            );
            let token = crate::link::fresh_hex().unwrap();
            let text = format!("S3 private pending host crash {token}");
            self.api("chat_say", json!({"id": sid, "lease": lease, "token": token, "text": text, "images": [], "files": []}), Duration::from_secs(10)).await;
            let until = Instant::now() + Duration::from_secs(5);
            let mut cursor = 0;
            let mut frames = Vec::new();
            loop {
                let page = self
                    .api(
                        "chat_poll",
                        json!({"id": sid, "lease": lease, "since": cursor}),
                        until.saturating_duration_since(Instant::now()),
                    )
                    .await;
                assert_eq!(page["lost"], false);
                cursor = page["next"].as_u64().unwrap();
                frames.extend(page["frames"].as_array().unwrap().iter().cloned());
                assert!(frames.len() <= 128);
                if frames
                    .iter()
                    .any(|v| v["kind"] == "send_committed" && v["token"] == token)
                    && frames.iter().any(|v| v["kind"] == "content_changed")
                {
                    break;
                }
                assert!(Instant::now() < until);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            let capture_token = crate::link::fresh_hex().unwrap();
            self.api(
                "chat_capture",
                json!({"id": sid, "lease": lease, "token": capture_token}),
                Duration::from_secs(10),
            )
            .await;
            let capture_until = Instant::now() + Duration::from_secs(5);
            let captured = loop {
                let page = self
                    .api(
                        "chat_poll",
                        json!({"id": sid, "lease": lease, "since": cursor}),
                        capture_until.saturating_duration_since(Instant::now()),
                    )
                    .await;
                assert_eq!(page["lost"], false);
                cursor = page["next"].as_u64().unwrap();
                if let Some(frame) = page["frames"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|v| v["kind"] == "captured" && v["token"] == capture_token)
                {
                    assert_eq!(frame["result"]["kind"], "available");
                    break frame["result"].clone();
                }
                assert!(Instant::now() < capture_until);
                tokio::time::sleep(Duration::from_millis(10)).await;
            };
            let history = json!({"id": sid, "binding": captured["binding"]});
            let request = self
                .api("history_begin", history.clone(), Duration::from_secs(10))
                .await;
            let page_payload = json!({"id": sid, "binding": captured["binding"], "requestKey": request,
                                     "cursor": captured["cursor"], "newer": false});
            let old_page = self
                .api(
                    "history_page",
                    page_payload.clone(),
                    Duration::from_secs(10),
                )
                .await;
            assert_eq!(old_page["reset"], false);
            let projected = serde_json::to_string(&old_page).unwrap();
            assert!(projected.contains(&text) && projected.contains("synthetic prompt admitted"));
            self.api(
                "history_cancel",
                json!({"id": sid, "binding": captured["binding"], "requestKey": request}),
                Duration::from_secs(10),
            )
            .await;
            let counts = self.counts();
            assert_eq!(
                counts,
                json!({"initialize": 2, "session/new": 1, "session/load": 0, "session/resume": 0, "session/prompt": 1})
            );
            // Accepted pending prompt/Note already reached the real canonical
            // writer. No more peer writes occur inside stdin-stall's loop.
            let binding = read(&row.join("journal.json"));
            let current_meta = read(&row.join("meta.json"));
            self.old_meta = Some(current_meta.clone());
            let current_attempt = read(&row.join("recovery-attempt.json"));
            // Read the actual canonical writer path after its real Capture
            // fence, never reconstruct a binding from a shortened native ID.
            let roots: Vec<_> = fs::read_dir(self.root.join(".khor/acp-journal"))
                .unwrap()
                .map(|v| v.unwrap().path())
                .collect();
            assert_eq!(roots.len(), 1);
            let raw_path = roots[0].join("current/records.jsonl");
            let raw = fs::read(&raw_path).unwrap();
            assert!(String::from_utf8_lossy(&raw).contains(&text));
            assert!(String::from_utf8_lossy(&raw).contains("synthetic prompt admitted"));
            self.tmp_before_crash();
            let current_host = sample(&host, &self.cli).unwrap();
            let current_provider = sample(&provider, &self.python).unwrap();
            assert_eq!(current_provider.stamp.parent, host.pid);
            save(
                &self.evidence.join("crash-intent.json"),
                json!({"host": current_host, "provider": current_provider,
                "signal": "SIGKILL host PID only", "binding": binding, "raw_sha256": hash(&raw_path)}),
            );
            assert_eq!(unsafe { libc::kill(host.pid as i32, libc::SIGKILL) }, 0);
            self.crashed = true;
            wait(
                || matches!(foreground(&host), Foreground::Absent(_)),
                5,
                "exact crashed host/group absence",
            );
            assert!(matches!(foreground(&provider), Foreground::Present));
            let orphan = sample(&provider, &self.python).unwrap();
            assert_eq!(
                orphan.stamp.parent, 1,
                "record actual Mac reparenting, do not adopt a new PID"
            );
            let unknown = self
                .api(
                    "session_recovery",
                    json!({"home_id": home, "id": sid}),
                    Duration::from_secs(10),
                )
                .await;
            // Persist the real public result and exact live foreground before
            // asserting the desired no-restart contract, including on red.
            save(
                &self.evidence.join("host-crash-observed.json"),
                json!({"id": sid, "home": home, "run": run, "offer": unknown,
                "host_absent": true, "provider_present": true, "provider": orphan,
                "counts": self.counts(), "meta": read(&row.join("meta.json")),
                "attempt": read(&row.join("recovery-attempt.json")),
                "binding": read(&row.join("journal.json")), "raw_sha256": hash(&raw_path),
                "provider_exit_receipt_fabricated": false}),
            );
            assert_eq!(unknown["id"], sid);
            assert_eq!(unknown["home"], home);
            assert_eq!(unknown["execution"], "unknown");
            assert_eq!(unknown["action"], "none");
            assert_eq!(unknown["current_run"], run);
            let payload = json!({"home_id": home, "id": sid, "expected_run": run,
                "token": {"epoch": unknown["next_attempt_epoch"], "nonce": crate::link::fresh_hex().unwrap()}});
            let refused = self
                .api("recover_session", payload.clone(), Duration::from_secs(31))
                .await;
            assert_eq!(refused["kind"], "unknown");
            assert_eq!(refused["id"], sid);
            assert_eq!(refused["attempt_epoch"], payload["token"]["epoch"]);
            assert_eq!(
                self.api("recover_session", payload, Duration::from_secs(31))
                    .await,
                refused
            );
            assert_eq!(self.counts(), counts);
            assert_eq!(read(&row.join("meta.json")), current_meta);
            assert_eq!(read(&row.join("recovery-attempt.json")), current_attempt);
            assert_eq!(read(&row.join("journal.json")), binding);
            assert_eq!(fs::read(&raw_path).unwrap(), raw);
            let request = self
                .api("history_begin", history, Duration::from_secs(10))
                .await;
            let returned = self
                .api(
                    "history_page",
                    json!({"id": sid, "binding": captured["binding"],
                "requestKey": request, "cursor": captured["cursor"], "newer": false}),
                    Duration::from_secs(10),
                )
                .await;
            assert_eq!(returned, old_page);
            self.api(
                "history_cancel",
                json!({"id": sid, "binding": captured["binding"], "requestKey": request}),
                Duration::from_secs(10),
            )
            .await;
            self.api(
                "chat_leave",
                json!({"id": sid, "lease": lease}),
                Duration::from_secs(10),
            )
            .await;
            assert!(matches!(foreground(&provider), Foreground::Present));
            save(
                &self.evidence.join("host-crash-oracles.json"),
                json!({"id": sid, "home": home, "run": run,
                "offer": unknown, "action": refused, "provider_after_host_crash": orphan,
                "counts": counts, "old_meta_attempt_binding_raw_unchanged": true,
                "provider_exit_receipt_fabricated": false}),
            );
            assert!(
                self.started.elapsed() < Duration::from_secs(50),
                "insurance is not the intended cleanup"
            );
        }
        fn cleanup(&mut self) -> Result<(), String> {
            let mut errors = Vec::new();
            let same_row =
                self.row
                    .as_ref()
                    .zip(self.old_meta.as_ref())
                    .is_none_or(|(row, expected)| {
                        fs::read(row.join("meta.json"))
                            .ok()
                            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                            .as_ref()
                            == Some(expected)
                    });
            if !same_row {
                errors.push(
                    "metadata changed; refuse to signal a possibly superseding execution"
                        .to_owned(),
                );
            } else {
                let provider_result = (|| -> Result<(), String> {
                    if let Some(provider) = self.provider.as_ref() {
                        if matches!(foreground(provider), Foreground::Present) {
                            let observed = sample(provider, &self.python)?;
                            if self.crashed {
                                let host = self.host.as_ref().ok_or("missing original host")?;
                                if !matches!(foreground(host), Foreground::Absent(_))
                                    || observed.stamp.parent != 1
                                {
                                    return Err("host absence/reparenting was not proven".into());
                                }
                            } else if observed.stamp.parent != provider.parent {
                                return Err(
                                    "provider parent changed outside controlled crash".into()
                                );
                            }
                            try_save(
                                &self.evidence.join("provider-cleanup-intent.json"),
                                json!({"provider": observed,
                        "signal": "SIGTERM original registered PGID only", "not_a_same_transport_exit_receipt": true}),
                            )?;
                            if unsafe { libc::kill(-(provider.group as i32), libc::SIGTERM) } != 0 {
                                return Err("provider group TERM failed".into());
                            }
                            let until = Instant::now() + Duration::from_secs(5);
                            while !matches!(foreground(provider), Foreground::Absent(_)) {
                                if Instant::now() >= until {
                                    return Err("provider group did not disappear".into());
                                }
                                std::thread::sleep(Duration::from_millis(10));
                            }
                        } else if !matches!(foreground(provider), Foreground::Absent(_)) {
                            return Err("provider cleanup ownership unknown".into());
                        }
                    }
                    Ok(())
                })();
                if let Err(error) = provider_result {
                    errors.push(error);
                }
                let host_result = (|| -> Result<(), String> {
                    if let Some(host) = self.host.as_ref() {
                        if matches!(foreground(host), Foreground::Present) {
                            sample(host, &self.cli)?;
                            if unsafe { libc::kill(host.pid as i32, libc::SIGKILL) } != 0 {
                                return Err("host cleanup failed".into());
                            }
                        }
                        let until = Instant::now() + Duration::from_secs(5);
                        while !matches!(foreground(host), Foreground::Absent(_)) {
                            if Instant::now() >= until {
                                return Err("host group did not disappear".into());
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                    Ok(())
                })();
                if let Err(error) = host_result {
                    errors.push(error);
                }
            }
            let bridge_result = (|| -> Result<(), String> {
                if let Some(bridge) = self.bridge.as_mut() {
                    if bridge.try_wait().map_err(|e| e.to_string())?.is_none() {
                        if unsafe { libc::kill(bridge.id() as i32, libc::SIGTERM) } != 0 {
                            return Err("bridge TERM failed".into());
                        }
                    }
                    let until = Instant::now() + Duration::from_secs(5);
                    while bridge.try_wait().map_err(|e| e.to_string())?.is_none() {
                        if Instant::now() >= until {
                            return Err("bridge was not reaped".into());
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
                Ok(())
            })();
            if let Err(error) = bridge_result {
                errors.push(error);
            }
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        }
        fn retained_crash_parts(&self) {
            let actual: Vec<_> = fs::read_dir(&self.tmp)
                .unwrap()
                .flat_map(|v| {
                    let path = v.unwrap().path();
                    let mut paths = vec![path.clone()];
                    if path.is_dir() {
                        paths.extend(fs::read_dir(path).unwrap().map(|v| v.unwrap().path()));
                    }
                    paths
                })
                .collect();
            assert_eq!(
                actual.len(),
                self.crash_files.len(),
                "unknown crash TMP residue"
            );
            for (path, expected) in &self.crash_files {
                let meta = fs::symlink_metadata(path).unwrap();
                assert_eq!(meta.dev(), expected["dev"].as_u64().unwrap());
                assert_eq!(meta.ino(), expected["ino"].as_u64().unwrap());
                assert_eq!(meta.uid() as u64, expected["uid"].as_u64().unwrap());
                if expected["directory"].as_bool().unwrap() {
                    assert!(meta.is_dir());
                } else {
                    assert!(meta.is_file() && meta.nlink() == 1);
                    assert_eq!(meta.len(), expected["bytes"].as_u64().unwrap());
                    assert_eq!(hash(path), expected["sha256"].as_str().unwrap());
                    let output = Command::new("/usr/sbin/lsof")
                        .args(["-Fn"])
                        .arg(path)
                        .output()
                        .unwrap();
                    assert!(
                        output.status.code() == Some(1)
                            && output.stdout.is_empty()
                            && output.stderr.is_empty()
                    );
                }
            }
            save(
                &self.evidence.join("retained-crash-parts.json"),
                json!({"known_originals_retained": self.crash_files,
                "child_tmp_absent": false, "all_exact_roles_closed": true, "unknown_tmp_allowed": false,
                "promoted_or_replayed": false, "provider_exited_receipt": false}),
            );
        }
    }

    #[test]
    #[ignore = "requires explicitly pinned current normal CLI/bridge and owned native admission"]
    fn a_crashed_host_cannot_restart_its_exact_live_foreground() {
        use futures::FutureExt;
        let mut fixture = Fixture::new();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let outcome = runtime.block_on(std::panic::AssertUnwindSafe(fixture.run()).catch_unwind());
        let cleanup = fixture.cleanup();
        save(
            &fixture.evidence.join("cleanup.json"),
            json!({"ok": cleanup.is_ok(), "error": cleanup.as_ref().err(), "seconds": fixture.started.elapsed().as_secs_f64()}),
        );
        assert!(
            cleanup.is_ok(),
            "exact original fixture cleanup failed: {cleanup:?}"
        );
        if fixture.crashed {
            // A genuine business red must still close the exact roles and
            // retain the already fingerprinted cold crash artifacts first.
            fixture.retained_crash_parts();
        }
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
        assert!(fixture.started.elapsed() < Duration::from_secs(120));
    }
}
