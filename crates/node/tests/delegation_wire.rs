//! Real ACP process + detached host + authenticated mesh + IPC. No model/key.
#[path = "util/delegation_pi.rs"]
mod pi;
mod util;
use khor_node::{
    Node,
    delegation::{
        Action, Answer, Binding, BindingProtocol, BoundAction, Record, TaskRef,
        protocol::{Message, Part, Role, TaskState},
    },
    gui_host::{GuiOp, gui_host_main},
    host::{Hello, Welcome, read_frame, write_frame},
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "_ghost") {
        let root = PathBuf::from(std::env::var_os("KHOR_HOME").unwrap());
        let result = gui_host_main(
            root,
            PathBuf::from(&args[1]),
            args[2].clone(),
            args[4..].to_vec(),
        );
        if let Err(why) = result {
            eprintln!("{why}");
            std::process::exit(1);
        }
        return;
    }
    const BOUND_PARENT: &str =
        "paired_bound_delegation_keeps_authenticated_identity_and_original_parent";
    if args.iter().any(|s| s == "--list") {
        println!("local_and_remote_delegation: test");
        #[cfg(unix)]
        println!("{BOUND_PARENT}: test");
        return;
    }
    let exact: Vec<_> = args
        .iter()
        .enumerate()
        .filter(|(_, arg)| *arg == "--exact")
        .collect();
    if !exact.is_empty() {
        if exact.len() != 1 {
            eprintln!("one exact acceptance parent is required");
            std::process::exit(2);
        }
        let selected = args.get(exact[0].0 + 1).map(String::as_str);
        match selected {
            Some(BOUND_PARENT) => {
                #[cfg(not(unix))]
                {
                    eprintln!("paired Bound process evidence is unsupported on this platform");
                    std::process::exit(3);
                }
                #[cfg(unix)]
                {
                    println!("running 1 test");
                    tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(4)
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(run_bound_pair());
                    println!("test {BOUND_PARENT} ... ok");
                    println!(
                        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out"
                    );
                    return;
                }
            }
            Some("local_and_remote_delegation") => {}
            _ => {
                eprintln!("unknown exact acceptance parent: {selected:?}");
                std::process::exit(2);
            }
        }
    }
    println!("running 1 test");
    let status =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args(["build", "-q", "-p", "khor-acp", "--bin", "acp-stub"])
            .status()
            .unwrap();
    assert!(status.success());
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(run());
    println!("test local_and_remote_delegation ... ok");
}
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Hosts(PathBuf);
impl Drop for Hosts {
    fn drop(&mut self) {
        let node = Node::open(self.0.clone()).unwrap();
        if let Ok(store) = node.delegation_store() {
            let to = khor_node::delegation::AgentTarget {
                home: node.device(),
                agent: "fixture".into(),
            };
            // Both callers' sessions, including those created just before a failure.
            let mut callers = vec![node.device()];
            if let Ok(bytes) = std::fs::read(self.0.join("caller.json")) {
                callers.push(serde_json::from_slice(&bytes).unwrap());
            }
            for caller in callers {
                for record in store.list(caller, &to, None, None, 100).unwrap_or_default() {
                    if let Some(session) = record.context_session {
                        let live = khor_node::live::LiveKind::new(self.0.clone(), node.device());
                        if let Some(dir) = live.dir_of(&session.id) {
                            if let Ok(hf) = khor_node::host::read_host_file(&dir) {
                                if let Ok(mut socket) =
                                    std::net::TcpStream::connect(("127.0.0.1", hf.port))
                                {
                                    socket
                                        .set_read_timeout(Some(Duration::from_secs(2)))
                                        .unwrap();
                                    let _ = write_frame(
                                        &mut socket,
                                        &Hello {
                                            cookie: hf.cookie,
                                            cols: 0,
                                            rows: 0,
                                        },
                                    );
                                    if read_frame::<Welcome>(&mut socket).is_ok() {
                                        let _ = write_frame(&mut socket, &GuiOp::Close);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
struct Serving(tokio::task::JoinHandle<Result<(), String>>);
impl Drop for Serving {
    fn drop(&mut self) {
        self.0.abort();
    }
}
fn root(tag: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!(
        "khor-delegation-{tag}-{}",
        khor_node::link::fresh_hex().unwrap()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    util::hermetic(&dir);
    Scratch(dir)
}
async fn serve(root: &Path, name: &str) -> Serving {
    let node = Node::open_as(root.to_path_buf(), name).unwrap();
    let serving = Serving(tokio::spawn(async move { node.serve().await }));
    tokio::time::timeout(Duration::from_secs(60), async {
        while !root.join(".khor/endpoint.json").exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    serving
}
fn task(answer: Answer) -> Record {
    if let Answer::Task { record } = answer {
        *record
    } else {
        panic!("expected task")
    }
}
fn message(id: &str, context: Option<String>) -> Message {
    let mut message = Message::new(Role::User, vec![Part::text("fixture instruction")]);
    message.message_id = id.into();
    message.context_id = context;
    message
}
async fn read(node: &Node, machine: &str, id: &str) -> Record {
    task(
        node.delegation_on(
            machine,
            Action::Get {
                agent: "fixture".into(),
                task: id.into(),
            },
        )
        .await
        .unwrap(),
    )
}
fn pending(record: &Record) -> Option<&serde_json::Value> {
    record
        .task
        .status
        .message
        .as_ref()?
        .metadata
        .as_ref()?
        .get("khor.approval")
}
async fn wait(node: &Node, machine: &str, id: &str, terminal: bool) -> Record {
    let until = Instant::now() + Duration::from_secs(25);
    loop {
        let record = read(node, machine, id).await;
        if if terminal {
            record.task.status.state.is_terminal()
        } else {
            pending(&record).is_some()
        } {
            return record;
        }
        assert!(Instant::now() < until, "task stalled: {record:?}");
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}
async fn run() {
    let alpha_root = root("alpha");
    let beta_root = root("beta");
    let alpha = Node::open_as(alpha_root.0.clone(), "alpha").unwrap();
    let stub = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("acp-stub");
    alpha
        .register_agent(
            "fixture",
            &[
                stub.to_string_lossy().into_owned(),
                "--unique-session".into(),
            ],
        )
        .unwrap();
    let _hosts = Hosts(alpha_root.0.clone());
    let local_input = message("local-first", None);
    let local = task(
        alpha
            .delegation_on(
                "alpha",
                Action::Send {
                    agent: "fixture".into(),
                    message: local_input.clone(),
                    parent: None,
                },
            )
            .await
            .unwrap(),
    );
    let local = wait(&alpha, "alpha", &local.task.id, false).await;
    assert!(local.context_session.is_some());
    assert!(
        serde_json::to_string(&local.task.status.message)
            .unwrap()
            .contains("thinking it over")
    );
    let again = task(
        alpha
            .delegation_on(
                "alpha",
                Action::Send {
                    agent: "fixture".into(),
                    message: local_input.clone(),
                    parent: None,
                },
            )
            .await
            .unwrap(),
    );
    assert_eq!(again.task.id, local.task.id);
    let mut changed = local_input;
    changed.parts = vec![Part::text("changed")];
    assert!(
        alpha
            .delegation_on(
                "alpha",
                Action::Send {
                    agent: "fixture".into(),
                    message: changed,
                    parent: None
                }
            )
            .await
            .is_err()
    );
    // Existing GUI permission and remote task_answer use the same pending ask.
    let ask = pending(&local).unwrap()["ask"].as_u64().unwrap();
    alpha
        .delegation_on(
            "alpha",
            Action::Answer {
                agent: "fixture".into(),
                task: local.task.id.clone(),
                ask,
                option: Some("go".into()),
            },
        )
        .await
        .unwrap();
    let done = wait(&alpha, "alpha", &local.task.id, true).await;
    assert_eq!(done.task.status.state, TaskState::Completed);
    assert!(
        serde_json::to_string(&done.task.status.message)
            .unwrap()
            .contains("picked:go")
    );
    assert!(
        alpha
            .delegation_on(
                "alpha",
                Action::Answer {
                    agent: "fixture".into(),
                    task: local.task.id.clone(),
                    ask,
                    option: Some("go".into())
                }
            )
            .await
            .is_err()
    );
    // The next task uses the same conversation and never rewrites the old task.
    let follow = task(
        alpha
            .delegation_on(
                "alpha",
                Action::Send {
                    agent: "fixture".into(),
                    message: message("local-follow", Some(done.task.context_id.clone())),
                    parent: None,
                },
            )
            .await
            .unwrap(),
    );
    assert_eq!(follow.context_session, done.context_session);
    let follow = wait(&alpha, "alpha", &follow.task.id, false).await;
    alpha
        .delegation_on(
            "alpha",
            Action::Cancel {
                agent: "fixture".into(),
                task: follow.task.id.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        wait(&alpha, "alpha", &follow.task.id, true)
            .await
            .task
            .status
            .state,
        TaskState::Canceled
    );
    assert_eq!(read(&alpha, "alpha", &done.task.id).await.task, done.task);

    pi::run(&alpha).await;

    let _alpha = serve(&alpha_root.0, "alpha").await;
    let beta = Node::open_as(beta_root.0.clone(), "beta").unwrap();
    tokio::time::timeout(Duration::from_secs(45), beta.pair(&alpha.invite().unwrap()))
        .await
        .unwrap()
        .unwrap();
    std::fs::write(
        alpha_root.0.join("caller.json"),
        serde_json::to_vec(&beta.device()).unwrap(),
    )
    .unwrap();
    let _beta = serve(&beta_root.0, "beta").await;
    assert!(
        matches!(beta.delegation_on(alpha.device_str(), Action::Agents).await.unwrap(), Answer::Agents { agents } if agents.iter().any(|a| a.name == "fixture" && a.available))
    );
    // Even a paired device cannot read another caller's task through the API.
    assert!(
        beta.delegation_on(
            alpha.device_str(),
            Action::Get {
                agent: "fixture".into(),
                task: done.task.id.clone()
            }
        )
        .await
        .is_err()
    );
    let remote_input = message("remote-first", None);
    let remote = task(
        beta.delegation_on(
            alpha.device_str(),
            Action::Send {
                agent: "fixture".into(),
                message: remote_input.clone(),
                parent: Some(khor_node::delegation::TaskRef {
                    home: beta.device(),
                    id: "parent-run".into(),
                }),
            },
        )
        .await
        .unwrap(),
    );
    assert_eq!(remote.caller, beta.device());
    assert_eq!(remote.target.home, alpha.device());
    let remote = wait(&beta, alpha.device_str(), &remote.task.id, false).await;
    let after_restart = {
        drop(_beta);
        let beta_again = serve(&beta_root.0, "beta").await;
        let row = read(&beta, alpha.device_str(), &remote.task.id).await;
        assert_eq!(row.task.id, remote.task.id);
        beta_again
    };
    let ask = pending(&remote).unwrap()["ask"].as_u64().unwrap();
    beta.delegation_on(
        alpha.device_str(),
        Action::Answer {
            agent: "fixture".into(),
            task: remote.task.id.clone(),
            ask,
            option: Some("go".into()),
        },
    )
    .await
    .unwrap();
    let completed = wait(&beta, alpha.device_str(), &remote.task.id, true).await;
    assert_eq!(completed.task.status.state, TaskState::Completed);
    assert!(
        serde_json::to_string(&completed.task.status.message)
            .unwrap()
            .contains("picked:go")
    );
    let rows = beta
        .delegation_on(
            alpha.device_str(),
            Action::List {
                agent: "fixture".into(),
                context: None,
                after: None,
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(rows, Answer::Tasks { records, .. } if records.len() == 1 && records[0].task.id == completed.task.id)
    );
    let request = khor_node::proto::Request::Delegation {
        action: Action::Send {
            agent: "fixture".into(),
            message: remote_input,
            parent: None,
        },
    };
    assert!(!request.rides_early());
    assert!(!request.is_reading());
    // A fresh unpaired endpoint cannot discover even the agent registry.
    let target = beta
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.id == alpha.device_str())
        .unwrap();
    let relay = [khor_net::endpoint::Relay::bare(util::loop_relay())];
    let stranger = khor_net::endpoint::bind_with(iroh::SecretKey::generate(), &relay, false)
        .await
        .unwrap();
    let address =
        khor_net::endpoint::dial_addr(&target.id, &target.addrs, &[util::loop_relay().into()])
            .unwrap();
    let connection = tokio::time::timeout(
        Duration::from_secs(20),
        stranger.connect(address, khor_net::endpoint::ALPN),
    )
    .await
    .unwrap()
    .unwrap();
    let (mut send, mut recv) = connection.open_bi().await.unwrap();
    send.write_all(
        &khor_node::proto::encode(&khor_node::proto::Request::Delegation {
            action: Action::Agents,
        })
        .unwrap(),
    )
    .await
    .unwrap();
    send.finish().unwrap();
    let bytes = recv.read_to_end(khor_node::proto::MAX_FRAME).await.unwrap();
    let response: khor_node::proto::Response = khor_node::proto::decode(&bytes).unwrap();
    assert!(matches!(
        response,
        khor_node::proto::Response::Refused { .. }
    ));
    stranger.close().await;
    drop(after_restart);
}

// This explicit parent uses the existing self-reexecuting host entry above;
// unlike the legacy default parent it never starts Cargo from inside the test.
#[cfg(unix)]
async fn bound_record(
    node: &Node,
    home: &str,
    binding: &Binding,
    task: &str,
) -> Result<Record, String> {
    let answer = node
        .delegation_on(
            home,
            Action::Bound {
                binding: binding.clone(),
                action: BoundAction::Get { task: task.into() },
            },
        )
        .await?;
    match answer {
        Answer::Task { record } => Ok(*record),
        _ => Err("Bound Get returned no Task".into()),
    }
}
#[cfg(unix)]
async fn bound_until(
    node: &Node,
    home: &str,
    binding: &Binding,
    task: &str,
    canceled: bool,
) -> Result<Record, String> {
    let by = Instant::now() + Duration::from_secs(20);
    loop {
        let record = bound_record(node, home, binding, task).await?;
        if if canceled {
            record.task.status.state == TaskState::Canceled
        } else {
            pending(&record).is_some()
        } {
            return Ok(record);
        }
        if record.task.status.state.is_terminal() || Instant::now() >= by {
            return Err(
                "actual bound Task did not reach the required permission/terminal state".into(),
            );
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
// Send deliberately malformed bindings to the selected receiver over an
// authenticated wire. delegation_on correctly routes by binding.home, so it
// cannot itself be used to probe a false home at a different receiver.
#[cfg(unix)]
async fn bound_raw(
    caller_root: &Path,
    target: &str,
    addrs: &[String],
    action: Action,
) -> Result<khor_node::proto::Response, String> {
    let key = khor_net::identity::load_or_create(&caller_root.join(".khor/identity.key"))
        .map_err(|e| e.to_string())?;
    let relays = [khor_net::endpoint::Relay::bare(util::loop_relay())];
    let endpoint = khor_net::endpoint::bind_with(key, &relays, false)
        .await
        .map_err(|e| e.to_string())?;
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let address = khor_net::endpoint::dial_addr(target, addrs, &[util::loop_relay().into()])
            .map_err(|e| e.to_string())?;
        let connection = endpoint
            .connect(address, khor_net::endpoint::ALPN)
            .await
            .map_err(|e| e.to_string())?;
        let (mut send, mut recv) = connection.open_bi().await.map_err(|e| e.to_string())?;
        send.write_all(&khor_node::proto::encode(
            &khor_node::proto::Request::Delegation { action },
        )?)
        .await
        .map_err(|e| e.to_string())?;
        send.finish().map_err(|e| e.to_string())?;
        let bytes = recv
            .read_to_end(khor_node::proto::MAX_FRAME)
            .await
            .map_err(|e| e.to_string())?;
        khor_node::proto::decode(&bytes)
    })
    .await
    .map_err(|_| "bound authenticated probe timed out".to_owned());
    endpoint.close().await;
    result?
}
#[cfg(unix)]
fn bound_refused(answer: khor_node::proto::Response, identity: bool) -> Result<(), String> {
    match answer {
        khor_node::proto::Response::Refused { why }
            if why
                == if identity {
                    "Delegation binding identity mismatch"
                } else {
                    "Message ID or session binding conflict"
                } =>
        {
            Ok(())
        }
        _ => Err("actual receiver did not refuse the bound probe as required".into()),
    }
}
#[cfg(unix)]
fn bound_pid_absent(pid: u32) -> bool {
    let status = unsafe { libc::kill(pid as i32, 0) };
    status == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}
#[cfg(unix)]
fn bound_processes(host: u32) -> Result<Vec<u32>, String> {
    let output = std::process::Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid="])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("could not read owned host descendants".into());
    }
    let rows: Vec<(u32, u32)> = String::from_utf8(output.stdout)
        .map_err(|e| e.to_string())?
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
        })
        .collect();
    let mut pids = vec![host];
    loop {
        let before = pids.len();
        for (pid, parent) in &rows {
            if pids.contains(parent) && !pids.contains(pid) {
                pids.push(*pid);
            }
        }
        if pids.len() == before {
            return Ok(pids);
        }
    }
}
#[cfg(unix)]
fn bound_host_pids(
    node: &Node,
    root: &Path,
    caller: khor_core::DeviceId,
) -> Result<Vec<u32>, String> {
    let target = khor_node::delegation::AgentTarget {
        home: node.device(),
        agent: "fixture".into(),
    };
    let mut pids = Vec::new();
    for record in node
        .delegation_store()?
        .list(caller, &target, None, None, 10)
        .map_err(|e| e.to_string())?
    {
        if let Some(session) = record.context_session {
            let live = khor_node::live::LiveKind::new(root.to_path_buf(), node.device());
            if let Some(directory) = live.dir_of(&session.id) {
                let host = khor_node::host::read_host_file(&directory)?;
                pids.extend(bound_processes(host.host_pid)?);
            }
        }
    }
    pids.sort_unstable();
    pids.dedup();
    Ok(pids)
}

#[cfg(unix)]
async fn run_bound_pair() {
    let alpha_root = root("bound-alpha");
    let beta_root = root("bound-beta");
    let alpha = Node::open_as(alpha_root.0.clone(), "bound-alpha").unwrap();
    let beta = Node::open_as(beta_root.0.clone(), "bound-beta").unwrap();
    let stub = PathBuf::from(
        std::env::var_os("KHOR_TEST_ACP_STUB").expect("set the verified actual ACP stub"),
    );
    assert!(stub.is_absolute() && stub.is_file());
    alpha
        .register_agent(
            "fixture",
            &[
                stub.to_string_lossy().into_owned(),
                "--unique-session".into(),
            ],
        )
        .unwrap();
    std::fs::write(
        alpha_root.0.join("caller.json"),
        serde_json::to_vec(&beta.device()).unwrap(),
    )
    .unwrap();
    let hosts = Hosts(alpha_root.0.clone());
    let mut alpha_serving = serve(&alpha_root.0, "bound-alpha").await;
    let paired =
        tokio::time::timeout(Duration::from_secs(45), beta.pair(&alpha.invite().unwrap())).await;
    let mut beta_serving = serve(&beta_root.0, "bound-beta").await;
    let mut observed_pids = Vec::new();
    let result = tokio::time::timeout(Duration::from_secs(90), async {
        paired.map_err(|_| "actual pairing timed out".to_owned())??;
        let selected = beta
            .devices()?
            .into_iter()
            .find(|d| d.id == alpha.device_str())
            .ok_or("actual paired target missing")?;
        let Answer::Binding { binding } = beta
            .delegation_on(
                alpha.device_str(),
                Action::ResolveBinding {
                    agent: "fixture".into(),
                    task: None,
                },
            )
            .await?
        else {
            return Err("ResolveBinding returned no actual binding".into());
        };
        if binding.home != alpha.device()
            || binding.caller != beta.device()
            || binding.agent != "fixture"
            || !matches!(&binding.protocol, BindingProtocol::Native)
        {
            return Err("resolved binding did not select the real home/caller/agent".into());
        }
        let parent = TaskRef {
            home: beta.device(),
            id: "original-bound-parent".into(),
        };
        let input = message("bound-first-input", None);
        let send = Action::Bound {
            binding: binding.clone(),
            action: BoundAction::Send {
                message: input.clone(),
                parent: Some(parent.clone()),
            },
        };
        let Answer::Task { record: submitted } =
            beta.delegation_on(alpha.device_str(), send.clone()).await?
        else {
            return Err("actual Bound Send returned no Task".into());
        };
        let before = bound_until(
            &beta,
            alpha.device_str(),
            &binding,
            &submitted.task.id,
            false,
        )
        .await?;
        if pending(&before)
            .and_then(|ask| ask["ask"].as_u64())
            .is_none()
            || before.task.id != submitted.task.id
            || before.caller != beta.device()
            || before.target.home != alpha.device()
            || before.parent.as_ref() != Some(&parent)
        {
            return Err("actual Bound Send/Get changed task ownership or original parent".into());
        }
        observed_pids = bound_host_pids(&alpha, &alpha_root.0, beta.device())?;
        if observed_pids.len() < 2 {
            return Err("actual ACP host and its live child were not both observed".into());
        }
        for pid in &observed_pids {
            println!("actual bound host/ACP PID {pid}: live before probes");
        }
        let mut forged_caller = binding.clone();
        forged_caller.caller = alpha.device();
        bound_refused(
            bound_raw(
                &beta_root.0,
                &selected.id,
                &selected.addrs,
                Action::Bound {
                    binding: forged_caller,
                    action: BoundAction::Get {
                        task: before.task.id.clone(),
                    },
                },
            )
            .await?,
            true,
        )?;
        let mut forged_home = binding.clone();
        forged_home.home = beta.device();
        bound_refused(
            bound_raw(
                &beta_root.0,
                &selected.id,
                &selected.addrs,
                Action::Bound {
                    binding: forged_home,
                    action: BoundAction::Cancel {
                        task: before.task.id.clone(),
                    },
                },
            )
            .await?,
            true,
        )?;
        let beta_selected = alpha
            .devices()?
            .into_iter()
            .find(|d| d.id == beta.device_str())
            .ok_or("actual paired reverse target missing")?;
        bound_refused(
            bound_raw(
                &alpha_root.0,
                &beta_selected.id,
                &beta_selected.addrs,
                Action::Bound {
                    binding: binding.clone(),
                    action: BoundAction::Get {
                        task: before.task.id.clone(),
                    },
                },
            )
            .await?,
            true,
        )?;
        // A changed-parent retry is an actual Store conflict, not a new lineage.
        bound_refused(
            bound_raw(
                &beta_root.0,
                &selected.id,
                &selected.addrs,
                Action::Bound {
                    binding: binding.clone(),
                    action: BoundAction::Send {
                        message: input,
                        parent: Some(TaskRef {
                            home: beta.device(),
                            id: "later-bound-parent".into(),
                        }),
                    },
                },
            )
            .await?,
            false,
        )?;
        let after_refusals =
            bound_record(&beta, alpha.device_str(), &binding, &before.task.id).await?;
        if pending(&after_refusals).is_none() || after_refusals.parent.as_ref() != Some(&parent) {
            return Err("refused probes affected the actual pending task".into());
        }
        let Answer::Task { record: duplicate } =
            beta.delegation_on(alpha.device_str(), send).await?
        else {
            return Err("same-input Bound retry returned no actual Task".into());
        };
        if duplicate.task.id != before.task.id || duplicate.parent.as_ref() != Some(&parent) {
            return Err("same-input Bound retry changed task identity".into());
        }
        let cancel = beta
            .delegation_on(
                alpha.device_str(),
                Action::Bound {
                    binding: binding.clone(),
                    action: BoundAction::Cancel {
                        task: before.task.id.clone(),
                    },
                },
            )
            .await?;
        if !matches!(cancel, Answer::Task { record } if record.task.id == before.task.id) {
            return Err("actual Bound Cancel returned a different task".into());
        }
        let final_record =
            bound_until(&beta, alpha.device_str(), &binding, &before.task.id, true).await?;
        let rows = alpha
            .delegation_store()?
            .list(beta.device(), &before.target, None, None, 10)
            .map_err(|e| e.to_string())?;
        if rows.len() != 1
            || rows[0].task.id != before.task.id
            || rows[0].parent.as_ref() != Some(&parent)
            || final_record.parent.as_ref() != Some(&parent)
        {
            return Err("bound probes created another Task or changed original parent".into());
        }
        Ok::<_, String>(())
    })
    .await
    .map_err(|_| "bound paired exercise exceeded its absolute deadline".to_owned())
    .and_then(|v| v);
    // Retire real hosts before any final behavior assertion, including failures.
    let discovered = bound_host_pids(&alpha, &alpha_root.0, beta.device());
    if let Ok(pids) = &discovered {
        observed_pids.extend(pids);
    }
    observed_pids.sort_unstable();
    observed_pids.dedup();
    drop(hosts);
    let by = Instant::now() + Duration::from_secs(5);
    while observed_pids.iter().any(|pid| !bound_pid_absent(*pid)) && Instant::now() < by {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let live: Vec<_> = observed_pids
        .iter()
        .filter(|pid| !bound_pid_absent(**pid))
        .copied()
        .collect();
    for pid in &observed_pids {
        println!(
            "actual bound host/ACP PID {pid}: ESRCH={}",
            !live.contains(pid)
        );
    }
    for serving in [&alpha_serving, &beta_serving] {
        serving.0.abort();
    }
    let alpha_stopped = tokio::time::timeout(Duration::from_secs(5), &mut alpha_serving.0).await;
    let beta_stopped = tokio::time::timeout(Duration::from_secs(5), &mut beta_serving.0).await;
    assert!(
        discovered.is_ok(),
        "owned host discovery failed during cleanup"
    );
    assert!(
        live.is_empty(),
        "owned bound host/ACP PIDs remain: {live:?}"
    );
    assert!(
        alpha_stopped.is_ok() && beta_stopped.is_ok(),
        "actual node serving tasks did not retire"
    );
    assert!(result.is_ok(), "bound paired behavior failed: {result:?}");
}
