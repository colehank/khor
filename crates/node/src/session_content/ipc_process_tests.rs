//! A real resident in another process. The controller holds its TCP writer
//! open while the receiver's remaining-budget lease releases a blocked reply.

use super::*;
use std::{
    io::{Read as _, Write as _},
    os::{
        fd::{AsFd, OwnedFd},
        unix::{
            fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
            net::UnixStream,
        },
    },
    process::{Child, Command, Stdio},
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

const CHILD_TEST: &str = "session_content::wire_tests::process_contracts::child_resident";
const ROOT_ENV: &str = "KHOR_TEST_IPC_PROCESS_ROOT";

async fn record(writer: &mut (impl tokio::io::AsyncWrite + Unpin), value: serde_json::Value) {
    let bytes = serde_json::to_vec(&value).unwrap();
    writer
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .await
        .unwrap();
    writer.write_all(&bytes).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn child_resident() {
    let Some(root) = std::env::var_os(ROOT_ENV) else {
        return;
    };
    let by = tokio::time::Instant::now() + Duration::from_secs(60);
    let root = PathBuf::from(root);
    assert_eq!(root.canonicalize().unwrap(), root);
    assert!(
        !std::fs::symlink_metadata(&root)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let owner: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("ipc-owner.json")).unwrap()).unwrap();
    assert_eq!(owner["owner"], "khor ipc process fixture");
    assert_eq!(
        owner["uid"].as_u64().unwrap(),
        u64::from(root.metadata().unwrap().uid())
    );
    util::hermetic(&root);
    let stdin = std::io::stdin();
    let fd = stdin.as_fd().try_clone_to_owned().unwrap();
    let socket = UnixStream::from(fd);
    socket.set_nonblocking(true).unwrap();
    let socket = tokio::net::UnixStream::from_std(socket).unwrap();
    let (reader, mut writer) = tokio::io::split(socket);
    let mut reader = BufReader::new(reader);
    let held = HoldWork::install(&root, Stage::Response);
    let node = Node::open_as(root.clone(), "ipc-process-home").unwrap();
    let responses = node.content_responses.clone();
    let mut serving = tokio::spawn(async move { node.serve().await });
    timeout(Duration::from_secs(30), async {
        while !root.join(".khor/endpoint.json").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    record(
        &mut writer,
        serde_json::json!({"ready":true,"pid":std::process::id()}),
    )
    .await;
    loop {
        let mut command = String::new();
        let next = tokio::select! {
            value = reader.read_line(&mut command) => value.unwrap_or_default(),
            _ = tokio::time::sleep_until(by) => 0,
        };
        if next == 0 || command.trim() == "stop" {
            break;
        }
        assert!(command.len() <= 128);
        if command.trim() == "release" {
            held.release();
        }
        let released = *held.1.released.lock().unwrap();
        record(
            &mut writer,
            serde_json::json!({
                "entered":held.1.count.load(Ordering::Acquire),
                "permits":responses.available_permits(),
                "released":released
            }),
        )
        .await;
    }
    held.release();
    let closed = crate::link::stop_serving(&root).await;
    let joined = matches!(
        timeout(Duration::from_secs(30), &mut serving).await,
        Ok(Ok(Ok(())))
    );
    let clean = joined && matches!(closed, Ok(crate::khor_agent::RuntimeClose::Closed));
    record(&mut writer, serde_json::json!({"resident_joined":clean})).await;
    assert!(clean, "the isolated resident must truly join");
}

struct Resident {
    child: Option<Child>,
    control: Option<UnixStream>,
}
impl Resident {
    fn start(root: &Path) -> Self {
        let (control, child_control) = UnixStream::pair().unwrap();
        control
            .set_read_timeout(Some(Duration::from_secs(35)))
            .unwrap();
        control
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let stderr = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(root.join("ipc-child.log"))
            .unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([CHILD_TEST, "--exact", "--nocapture", "--test-threads=1"])
            .env_clear()
            .env(ROOT_ENV, root)
            .stdin(Stdio::from(OwnedFd::from(child_control)))
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr));
        for name in [
            "PATH",
            "LD_LIBRARY_PATH",
            "DYLD_LIBRARY_PATH",
            "DYLD_FALLBACK_LIBRARY_PATH",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let child = command.spawn().unwrap();
        let mut resident = Self {
            child: Some(child),
            control: Some(control),
        };
        let ready = resident.read();
        assert_eq!(ready["ready"], true);
        assert_eq!(
            ready["pid"].as_u64().unwrap(),
            u64::from(resident.child.as_ref().unwrap().id())
        );
        resident
            .control
            .as_ref()
            .unwrap()
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        resident
    }
    fn read(&mut self) -> serde_json::Value {
        let control = self.control.as_mut().unwrap();
        let mut length = [0; 4];
        control.read_exact(&mut length).unwrap();
        let length = u32::from_be_bytes(length) as usize;
        assert!(length <= 4096);
        let mut bytes = vec![0; length];
        control.read_exact(&mut bytes).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
    fn request(&mut self, command: &str) -> serde_json::Value {
        self.control
            .as_mut()
            .unwrap()
            .write_all(format!("{command}\n").as_bytes())
            .unwrap();
        self.read()
    }
    fn finish(mut self) {
        self.control
            .as_ref()
            .unwrap()
            .set_read_timeout(Some(Duration::from_secs(35)))
            .unwrap();
        assert_eq!(self.request("stop")["resident_joined"], true);
        self.control.take();
        let by = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                assert!(status.success());
                self.child.take();
                break;
            }
            assert!(
                std::time::Instant::now() < by,
                "joined child must exit without a forced kill"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Resident {
    fn drop(&mut self) {
        self.control.take();
        if let Some(child) = self.child.as_mut() {
            let by = std::time::Instant::now() + Duration::from_secs(5);
            while std::time::Instant::now() < by {
                if child.try_wait().ok().flatten().is_some() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn target(endpoint: &serde_json::Value, cookie: &str) -> crate::ipc::Target {
    crate::ipc::Target {
        port: endpoint["ipc_port"].as_u64().unwrap() as u16,
        cookie: cookie.into(),
        max_version: endpoint["ipc_max_version"].as_u64().unwrap() as u8,
    }
}

async fn raw(endpoint: &serde_json::Value, action: Action, extra: bool) -> tokio::net::TcpStream {
    let mut stream = tokio::net::TcpStream::connect((
        "127.0.0.1",
        endpoint["ipc_port"].as_u64().unwrap() as u16,
    ))
    .await
    .unwrap();
    let payload = crate::proto::encode(&crate::ipc::Handoff {
        cookie: endpoint["ipc_cookie"].as_str().unwrap().into(),
        op: crate::ipc::Op::ContentRead { action },
    })
    .unwrap();
    let header = crate::ipc::lease::header(
        payload.len(),
        tokio::time::Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    let mut frame = header.to_vec();
    frame.extend(payload);
    if extra {
        frame.push(0);
    }
    stream.write_all(&frame).await.unwrap();
    stream
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_process_budget_releases_a_reply_without_caller_eof_or_manual_release() {
    assert!(crate::deadline::current().is_none());
    assert_eq!(crate::link::CONTENT_RESPONSES, 32);
    let tree = crate::testdir::TempTree::fresh("content-ipc-process-budget");
    let root = tree.path().canonicalize().unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(
        root.join("ipc-owner.json"),
        serde_json::to_vec(&serde_json::json!({
            "owner":"khor ipc process fixture","uid":root.metadata().unwrap().uid()
        }))
        .unwrap(),
    )
    .unwrap();
    let id = "01a17c1a-a542-7822-b01d-82a832a158e6";
    let history: Vec<_> = (0..500)
        .flat_map(|n| message(&format!("process-{n}")))
        .collect();
    std::fs::write(source(&root, id), history).unwrap();
    let mut resident = Resident::start(&root);
    let endpoint: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join(".khor/endpoint.json")).unwrap()).unwrap();
    assert_eq!(
        endpoint["pid"].as_u64().unwrap(),
        u64::from(resident.child.as_ref().unwrap().id())
    );
    assert_eq!(endpoint["ipc_max_version"], 2);
    let node = Node::open_as(root.clone(), "ipc-process-home").unwrap();
    let observation = ObservationRef {
        owner: node.device(),
        home: node.device(),
        namespace: ObservationNamespace::Native,
        agent: "codex".into(),
        native_id: id.into(),
        imported_at: Millis(1),
        updated_at: None,
    };
    let locator =
        ContentLocator::observed(SessionId("observe/ipc-process-budget".into()), &observation)
            .unwrap();
    let Reply::Resolved(Resolution::Available { binding, .. }) =
        crate::link::content_on_resident(&root, Action::Resolve { locator })
            .await
            .unwrap()
    else {
        panic!("child fixture must resolve");
    };
    let request = "1".repeat(32);
    crate::link::content_on_resident(
        &root,
        Action::Begin {
            binding: binding.clone(),
            request: request.clone(),
        },
    )
    .await
    .unwrap();
    let began = std::time::Instant::now();
    let mut stream = raw(
        &endpoint,
        Action::Window {
            binding: binding.clone(),
            request,
            cursor: None,
            newer: false,
        },
        false,
    )
    .await;
    timeout(Duration::from_secs(2), async {
        while resident.request("stats")["entered"] != 1 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let before = resident.request("stats");
    assert_eq!(before["permits"], crate::link::CONTENT_RESPONSES - 1);
    assert_eq!(before["released"], false);
    timeout(Duration::from_secs(2), async {
        while resident.request("stats")["permits"] != crate::link::CONTENT_RESPONSES {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        began.elapsed() >= Duration::from_millis(800),
        "the advertised second was not replaced by immediate cancellation"
    );
    assert_eq!(resident.request("stats")["released"], false);
    let mut bytes = Vec::new();
    let ended = timeout(Duration::from_secs(2), stream.read_to_end(&mut bytes))
        .await
        .unwrap();
    assert!(ended.is_ok() || ended.is_err_and(|e| e.kind() == std::io::ErrorKind::ConnectionReset));
    assert!(
        bytes.is_empty(),
        "the held response was cancelled before its write"
    );
    drop(stream);
    let elapsed_ms = began.elapsed().as_millis();
    assert_eq!(resident.request("release")["released"], true);
    let bad = "2".repeat(32);
    let reply = crate::ipc::call_target(
        target(&endpoint, "wrong-cookie"),
        crate::ipc::Op::ContentRead {
            action: Action::Begin {
                binding: binding.clone(),
                request: bad.clone(),
            },
        },
        || None,
    )
    .await
    .unwrap();
    assert!(matches!(reply, crate::ipc::Reply::Refused { .. }));
    assert!(
        crate::link::content_on_resident(
            &root,
            Action::Window {
                binding: binding.clone(),
                request: bad,
                cursor: None,
                newer: false
            }
        )
        .await
        .is_err()
    );
    let extra = "3".repeat(32);
    let mut stream = raw(
        &endpoint,
        Action::Begin {
            binding: binding.clone(),
            request: extra.clone(),
        },
        true,
    )
    .await;
    let mut bytes = Vec::new();
    let _ = timeout(Duration::from_secs(2), stream.read_to_end(&mut bytes))
        .await
        .unwrap();
    assert!(bytes.is_empty());
    assert!(
        crate::link::content_on_resident(
            &root,
            Action::Window {
                binding,
                request: extra,
                cursor: None,
                newer: false
            }
        )
        .await
        .is_err()
    );
    let reply = crate::ipc::call(
        endpoint["ipc_port"].as_u64().unwrap() as u16,
        endpoint["ipc_cookie"].as_str().unwrap(),
        crate::ipc::Op::Hops,
    )
    .await
    .unwrap();
    assert!(matches!(reply, crate::ipc::Reply::Hops { .. }));
    assert_eq!(resident.request("stats")["permits"], 32);
    resident.finish();
    eprintln!(
        "[ipc-process-lease] {}",
        serde_json::json!({"elapsed_ms":elapsed_ms,"resident_joined":true,"child_reaped":true,"initial_permits":31,"final_permits":32,"caller_eof":false,"manual_response_release":false})
    );
}
