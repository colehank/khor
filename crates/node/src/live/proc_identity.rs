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
