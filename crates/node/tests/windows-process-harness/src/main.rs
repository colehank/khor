#[cfg(windows)]
#[path = "../../../src/khor_agent_process.rs"]
mod supervised;

#[cfg(windows)]
#[path = "../../../src/khor_agent_process_windows.rs"]
mod process;

#[cfg(windows)]
struct Pipes {
    stdin: Box<dyn std::io::Write + Send>,
    stdout: Box<dyn std::io::Read + Send>,
    stderr: Box<dyn std::io::Read + Send>,
}

#[cfg(not(windows))]
fn main() {
    eprintln!("this harness must run on Windows; a cross-check is compilation only");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--fixture")) {
        fixture();
        return;
    }
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(30));
        eprintln!("Windows process harness exceeded its cleanup fuse");
        // Exiting closes every non-inherited kill-on-close Job, including a stalled check.
        std::process::exit(124);
    });
    checks::run();
}

#[cfg(windows)]
fn fixture() {
    use std::io::{Read, Write};
    match std::env::var("KHOR_PROCESS_MODE").unwrap().as_str() {
        "echo" => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap();
            for arg in std::env::args_os().skip(2) {
                println!("arg:{}", checks::wide_hex(&arg));
            }
            println!(
                "cwd:{}",
                checks::wide_hex(std::env::current_dir().unwrap().as_os_str())
            );
            println!(
                "env:{}",
                checks::wide_hex(&std::env::var_os("KHOR_PROCESS_VALUE").unwrap())
            );
            println!(
                "removed:{}",
                std::env::var_os("KHOR_PROCESS_REMOVED").is_none()
            );
            println!(
                "inherited:{}",
                std::env::var_os("KHOR_PROCESS_PARENT_ONLY").is_some()
            );
            if let Ok(handle) = std::env::var("KHOR_PROCESS_EXCLUDED_HANDLE") {
                unsafe {
                    windows_sys::Win32::System::Threading::SetEvent(
                        handle.parse::<usize>().unwrap() as *mut std::ffi::c_void,
                    );
                }
            }
            println!("input:{input}");
            eprint!("stderr-complete");
            std::process::exit(23);
        }
        "leader" | "tree" => {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap());
            child.arg("--fixture").env("KHOR_PROCESS_MODE", "sleep");
            let descendant = child.spawn().unwrap();
            println!("{} {}", std::process::id(), descendant.id());
            std::io::stdout().flush().unwrap();
            if std::env::var("KHOR_PROCESS_MODE").unwrap() == "leader" {
                return;
            }
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        "sleep" => loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        },
        mode => panic!("unknown fixture mode {mode}"),
    }
}

#[cfg(windows)]
mod checks {
    use super::{Pipes, process, supervised};
    use std::{
        ffi::{OsStr, OsString},
        io::{BufRead, BufReader, Read, Write},
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            io::{AsRawHandle, FromRawHandle, OwnedHandle},
        },
        path::PathBuf,
        process::Command,
        thread,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
        Security::SECURITY_ATTRIBUTES,
        System::Threading::{CreateEventW, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
    };

    pub(super) fn wide_hex(value: &OsStr) -> String {
        value
            .encode_wide()
            .map(|unit| format!("{unit:04x}"))
            .collect()
    }

    fn command(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("--fixture")
            .env_clear()
            .env("KHOR_PROCESS_MODE", mode);
        command
    }

    fn await_leader(process: &mut process::Process) {
        let until = Instant::now() + Duration::from_secs(5);
        while !process.exited().unwrap() {
            assert!(Instant::now() < until, "leader failed to exit");
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn process_handle(pid: u32) -> OwnedHandle {
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        assert!(!handle.is_null(), "cannot retain fixture process {pid}");
        unsafe { OwnedHandle::from_raw_handle(handle) }
    }

    fn assert_dead(handle: &OwnedHandle) {
        assert_eq!(
            unsafe { WaitForSingleObject(handle.as_raw_handle(), 5_000) },
            WAIT_OBJECT_0
        );
    }

    pub(super) fn run() {
        fn assert_send<T: Send>() {}
        assert_send::<process::Process>();
        let scratch = Scratch::new();
        echo_roundtrip(&scratch);
        println!(
            "PASS argv / UTF16 / env-clear / removed-env / cwd / stdin / stdout / stderr / exit-status"
        );
        bare_program(&scratch);
        println!("PASS bare program from explicit PATH with node + host.ts arguments");
        invalid_inputs();
        println!("PASS invalid program / missing PATH / batch / embedded NUL rejected");
        terminate_tree();
        println!("PASS job termination after leader exit kills pipe-owning descendant");
        drop_tree();
        println!("PASS dropping a live process kills leader and descendant via owned job");
        blocked_stdin();
        println!(
            "PASS independent I/O cancellation unblocks full stdin while child is still alive"
        );
        blocked_output();
        println!(
            "PASS independent I/O cancellation unblocks both reads while child holds output pipes"
        );
        supervised_deadline();
        println!("PASS supervisor deadline progresses through blocked stdin and repeated Stop");
        println!("PASS 8 Windows runtime groups");
    }

    fn echo_roundtrip(scratch: &Scratch) {
        let values: Vec<OsString> = [
            "",
            "normal",
            "two words",
            "\t",
            "\"quoted\"",
            "one\\\"two",
            "tail\\",
            "many\\\\\"quotes",
            "中文🙂",
        ]
        .into_iter()
        .map(OsString::from)
        .chain(std::iter::once(OsString::from_wide(&[
            0xd800, 0x78, 0xdc00,
        ])))
        .collect();
        let security = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: std::ptr::null_mut(),
            bInheritHandle: 1,
        };
        let excluded = unsafe { CreateEventW(&security, 1, 0, std::ptr::null()) };
        assert!(!excluded.is_null());
        let excluded = unsafe { OwnedHandle::from_raw_handle(excluded) };
        let mut command = command("echo");
        command
            .args(&values)
            .current_dir(&scratch.0)
            .env("KHOR_PROCESS_VALUE", "Unicode 值=two")
            .env(
                "KHOR_PROCESS_EXCLUDED_HANDLE",
                (excluded.as_raw_handle() as usize).to_string(),
            )
            .env("KHOR_PROCESS_REMOVED", "must disappear")
            .env_remove("KHOR_PROCESS_REMOVED");
        // The fuse thread never reads or changes the process environment.
        unsafe { std::env::set_var("KHOR_PROCESS_PARENT_ONLY", "private-parent-value") };
        let (mut child, mut pipes) = process::spawn(command).unwrap();
        unsafe { std::env::remove_var("KHOR_PROCESS_PARENT_ONLY") };
        pipes.stdin.write_all(b"pipe-input").unwrap();
        drop(pipes.stdin);
        await_leader(&mut child);
        assert_eq!(child.wait().unwrap().code(), Some(23));
        assert_eq!(child.wait().unwrap().code(), Some(23));
        let mut output = String::new();
        pipes.stdout.read_to_string(&mut output).unwrap();
        let mut error = String::new();
        pipes.stderr.read_to_string(&mut error).unwrap();
        let expected: Vec<String> = values
            .iter()
            .map(|value| format!("arg:{}", wide_hex(value)))
            .collect();
        assert_eq!(
            output.lines().take(values.len()).collect::<Vec<_>>(),
            expected
        );
        assert!(output.contains(&format!("cwd:{}", wide_hex(scratch.0.as_os_str()))));
        assert!(output.contains(&format!("env:{}", wide_hex(OsStr::new("Unicode 值=two")))));
        assert!(output.contains("removed:true\ninherited:false\ninput:pipe-input\n"));
        assert_eq!(error, "stderr-complete");
        assert_eq!(
            unsafe { WaitForSingleObject(excluded.as_raw_handle(), 0) },
            WAIT_TIMEOUT,
            "a handle outside HANDLE_LIST was inherited"
        );
    }

    fn bare_program(scratch: &Scratch) {
        std::fs::copy(std::env::current_exe().unwrap(), scratch.0.join("node.exe")).unwrap();
        let mut command = Command::new("node");
        command
            .args(["--fixture", "host.ts", "with spaces"])
            .env_clear()
            .env("Path", &scratch.0)
            .env("KHOR_PROCESS_MODE", "echo")
            .env("KHOR_PROCESS_VALUE", "bare");
        let (
            mut child,
            Pipes {
                stdin,
                mut stdout,
                mut stderr,
            },
        ) = process::spawn(command).unwrap();
        drop(stdin);
        await_leader(&mut child);
        assert_eq!(child.wait().unwrap().code(), Some(23));
        let mut output = String::new();
        stdout.read_to_string(&mut output).unwrap();
        let mut error = String::new();
        stderr.read_to_string(&mut error).unwrap();
        assert_eq!(error, "stderr-complete");
        assert!(output.starts_with(&format!(
            "arg:{}\narg:{}\n",
            wide_hex(OsStr::new("host.ts")),
            wide_hex(OsStr::new("with spaces"))
        )));
    }

    fn invalid_inputs() {
        for program in [
            "",
            "missing-unique-khor-sidecar.exe",
            "sidecar.cmd",
            "sidecar.bat",
            "nul\0program",
        ] {
            let mut command = Command::new(program);
            command.env_clear();
            assert!(process::spawn(command).is_err(), "accepted {program:?}");
        }
        let mut command = command("echo");
        command.arg("nul\0argument");
        assert!(process::spawn(command).is_err());
        let mut command = self::command("echo");
        command.env("KHOR_PROCESS_VALUE", "nul\0value");
        assert!(process::spawn(command).is_err());
        let mut command = self::command("echo");
        command.current_dir("nul\0directory");
        assert!(process::spawn(command).is_err());
    }

    fn start_tree(
        mode: &str,
    ) -> (
        process::Process,
        BufReader<Box<dyn Read + Send>>,
        u32,
        OwnedHandle,
    ) {
        let (
            child,
            Pipes {
                stdin,
                stdout,
                stderr: _,
            },
        ) = process::spawn(command(mode)).unwrap();
        drop(stdin);
        let mut output = BufReader::new(stdout);
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let ids: Vec<u32> = line
            .split_whitespace()
            .map(|part| part.parse().unwrap())
            .collect();
        let descendant = process_handle(ids[1]);
        assert_eq!(
            unsafe { WaitForSingleObject(descendant.as_raw_handle(), 0) },
            WAIT_TIMEOUT
        );
        (child, output, ids[0], descendant)
    }

    fn terminate_tree() {
        let (mut child, mut output, _, descendant) = start_tree("leader");
        await_leader(&mut child);
        assert_eq!(
            unsafe { WaitForSingleObject(descendant.as_raw_handle(), 0) },
            WAIT_TIMEOUT
        );
        child.terminate().unwrap();
        child.terminate().unwrap();
        assert!(child.wait().unwrap().success());
        assert_dead(&descendant);
        let mut remainder = String::new();
        output.read_to_string(&mut remainder).unwrap();
        assert!(remainder.is_empty(), "pipe did not end cleanly");
    }

    fn drop_tree() {
        let (child, _output, leader_pid, descendant) = start_tree("tree");
        let leader = process_handle(leader_pid);
        drop(child);
        for handle in [&leader, &descendant] {
            assert_eq!(
                unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) },
                WAIT_OBJECT_0,
                "Drop returned before the owned tree stopped"
            );
        }
    }

    fn supervised_deadline() {
        let control = std::sync::Arc::new(supervised::Control::default());
        let (mut session, pipes) =
            supervised::Session::spawn(command("sleep"), control.clone()).unwrap();
        let (sent, received) = std::sync::mpsc::channel();
        let mut stdin = pipes.stdin;
        let writer = thread::spawn(move || {
            let _ = sent.send(stdin.write_all(&vec![b'x'; 8 * 1024 * 1024]));
        });
        assert!(matches!(
            received.recv_timeout(Duration::from_millis(150)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        let started = Instant::now();
        control.stop();
        for seconds in [3, 6, 9] {
            thread::sleep(Duration::from_secs(seconds).saturating_sub(started.elapsed()));
            control.stop();
        }
        assert!(
            received
                .recv_timeout(Duration::from_secs(4))
                .unwrap()
                .is_err()
        );
        assert!(
            started.elapsed() < Duration::from_millis(12_500),
            "repeated Stop extended the first deadline"
        );
        assert!(!session.finish(false).unwrap().success());
        writer.join().unwrap();
    }

    fn blocked_stdin() {
        let (mut child, pipes) = process::spawn(command("sleep")).unwrap();
        let mut stdin = pipes.stdin;
        let (sent, received) = std::sync::mpsc::channel();
        let writer = thread::spawn(move || {
            let result = stdin.write_all(&vec![b'x'; 8 * 1024 * 1024]);
            sent.send(result).unwrap();
        });
        assert!(
            received.recv_timeout(Duration::from_millis(100)).is_err(),
            "stdin never blocked"
        );
        child.cancel_io();
        let cancelled = received.recv_timeout(Duration::from_secs(5));
        let leader_alive = !child.exited().unwrap();
        child.terminate().unwrap();
        child.wait().unwrap();
        writer.join().unwrap();
        assert!(cancelled.unwrap().is_err());
        assert!(leader_alive, "writer only ended because the child exited");
    }

    fn blocked_output() {
        let (
            mut child,
            Pipes {
                stdin,
                stdout,
                stderr,
            },
        ) = process::spawn(command("sleep")).unwrap();
        let (sent, received) = std::sync::mpsc::channel();
        let readers: Vec<_> = [stdout, stderr]
            .into_iter()
            .map(|mut pipe| {
                let sent = sent.clone();
                thread::spawn(move || {
                    let mut byte = [0];
                    sent.send(pipe.read(&mut byte)).unwrap();
                })
            })
            .collect();
        drop(sent);
        assert!(
            received.recv_timeout(Duration::from_millis(100)).is_err(),
            "read never blocked"
        );
        child.cancel_io();
        let first = received.recv_timeout(Duration::from_secs(5));
        let second = received.recv_timeout(Duration::from_secs(5));
        let leader_alive = !child.exited().unwrap();
        child.terminate().unwrap();
        child.wait().unwrap();
        drop(stdin);
        for reader in readers {
            reader.join().unwrap();
        }
        assert_eq!(first.unwrap().unwrap(), 0);
        assert_eq!(second.unwrap().unwrap(), 0);
        assert!(leader_alive, "reads only ended because the child exited");
    }

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("khor windows process {} 中文", std::process::id()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
