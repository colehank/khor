//! Real ACP process + detached host + authenticated mesh + IPC. No model/key.
#[path = "util/delegation_pi.rs"]
mod pi;
mod util;
use khor_node::{
    Node,
    delegation::{
        Action, Answer, Record,
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
    if args.iter().any(|s| s == "--list") {
        println!("local_and_remote_delegation: test");
        return;
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
