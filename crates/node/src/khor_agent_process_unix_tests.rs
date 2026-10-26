use super::*;
use std::os::unix::process::ExitStatusExt;

fn ready_process(code: &str) -> (Process, std::process::ChildStdout) {
    let mut child = Command::new("python3")
        .args(["-I", "-S", "-c", code])
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let output = child.stdout.take().unwrap();
    (
        Process {
            child,
            owns_group: true,
            cancelled: Arc::new(AtomicBool::new(false)),
        },
        output,
    )
}

#[test]
fn retirement_during_the_darwin_exit_window_confirms_the_owned_leader() {
    for iteration in 0..32 {
        let (mut process, mut output) = ready_process("import os; os.write(1,b'x'); os._exit(0)");
        let mut ready = [0];
        output.read_exact(&mut ready).unwrap();
        // The actual child exits immediately after this byte. Darwin may reject
        // killpg before WNOHANG|WNOWAIT exposes its exit status to the parent.
        process
            .terminate()
            .unwrap_or_else(|error| panic!("iteration {iteration}: {error}"));
        let status = process.wait().unwrap();
        assert!(status.success() || status.signal() == Some(libc::SIGKILL));
    }
}

#[test]
fn a_live_owned_group_is_still_killed_and_lost_ownership_is_rejected() {
    let (mut process, mut output) =
        ready_process("import os,signal,time; signal.alarm(5); os.write(1,b'x'); time.sleep(30)");
    let mut ready = [0];
    output.read_exact(&mut ready).unwrap();
    process.owns_group = false;
    let result = process.terminate();
    process.owns_group = true;
    assert!(result.unwrap_err().to_string().contains("ownership lost"));
    process.terminate().unwrap();
    assert_eq!(process.wait().unwrap().signal(), Some(libc::SIGKILL));
}
