//! Published MessagePack shapes, independent of today's enum ordering.
use super::*;
use crate::proto::{self, Request, Response};

// These are the deployed rc20 shapes. Deliberately keep a different order
// from the current enums: MessagePack words must be names, not ordinals.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum PublishedRequest {
    Ls { path: String },
    OpenShellOnce { start: PublishedShellStart },
    Sessions,
}
#[derive(Debug, Serialize, Deserialize)]
enum PublishedResponse {
    SessionRows { rows: Vec<khor_core::Session> },
    Refused { why: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PublishedShellStart {
    token: String,
    title: String,
    cwd: String,
    command: String,
    cols: u16,
    rows: u16,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum PublishedOp {
    OpenShellOn {
        machine: String,
        start: PublishedShellStart,
    },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PublishedHandoff {
    cookie: String,
    op: PublishedOp,
}

fn published_start() -> PublishedShellStart {
    PublishedShellStart {
        token: "published-shell-token".into(),
        title: "shell 中文".into(),
        cwd: "/fixture space".into(),
        command: "printf '%s' '$HOME'".into(),
        cols: 100,
        rows: 30,
    }
}

fn legacy_start() -> proto::LegacyShellStart {
    let start = published_start();
    proto::LegacyShellStart {
        token: start.token,
        title: start.title,
        cwd: start.cwd,
        command: start.command,
        cols: start.cols,
        rows: start.rows,
    }
}

fn terminal_start() -> crate::shell_start::ShellStart {
    let mut start: crate::shell_start::ShellStart = legacy_start().into();
    start.command.clear();
    start.argv = Some(vec!["program".into(), "literal $HOME %PATH% 中文".into()]);
    start.headless = true;
    start
}

fn assert_legacy_defaults(start: proto::LegacyShellStart) {
    let start: crate::shell_start::ShellStart = start.into();
    let expected = published_start();
    assert_eq!(start.token, expected.token);
    assert_eq!(start.title, expected.title);
    assert_eq!(start.cwd, expected.cwd);
    assert_eq!(start.command, expected.command);
    assert_eq!((start.cols, start.rows), (expected.cols, expected.rows));
    assert!(!start.headless);
    assert_eq!(start.argv, None);
}

#[test]
fn published_startup_request_keeps_all_six_positional_fields_in_both_directions() {
    let published = PublishedRequest::OpenShellOnce {
        start: published_start(),
    };
    let bytes = proto::encode(&published).unwrap();
    assert_eq!(
        proto::encode_request(&Request::OpenShellOnce {
            start: legacy_start(),
        })
        .unwrap(),
        bytes
    );
    assert_eq!(
        proto::decode::<PublishedRequest>(&bytes).unwrap(),
        published
    );
    match proto::decode::<Request>(&bytes).unwrap() {
        Request::OpenShellOnce { start } => assert_legacy_defaults(start),
        other => panic!("unexpected request: {other:?}"),
    }
}

#[test]
fn published_ipc_handoff_keeps_the_old_startup_shape_in_both_directions() {
    let published = PublishedHandoff {
        cookie: "fixture-cookie".into(),
        op: PublishedOp::OpenShellOn {
            machine: "fixture-machine".into(),
            start: published_start(),
        },
    };
    let current = crate::ipc::Handoff {
        cookie: "fixture-cookie".into(),
        op: crate::ipc::Op::OpenShellOn {
            machine: "fixture-machine".into(),
            start: legacy_start(),
        },
    };
    // IPC uses the same positional MessagePack codec as the node wire.
    let bytes = proto::encode(&current).unwrap();
    assert_eq!(bytes, proto::encode(&published).unwrap());
    assert_eq!(
        proto::decode::<PublishedHandoff>(&bytes).unwrap(),
        published
    );
    let decoded: crate::ipc::Handoff = proto::decode(&bytes).unwrap();
    assert_eq!(decoded.cookie, "fixture-cookie");
    match decoded.op {
        crate::ipc::Op::OpenShellOn { machine, start } => {
            assert_eq!(machine, "fixture-machine");
            assert_legacy_defaults(start);
        }
        other => panic!("unexpected handoff: {other:?}"),
    }
    assert_eq!(
        serde_json::to_value(&current).unwrap(),
        serde_json::to_value(&published).unwrap()
    );
}

#[test]
fn new_terminal_startup_words_are_refused_before_old_peers_can_drop_arguments() {
    let request = Request::OpenTerminalOnce {
        start: terminal_start(),
    };
    let bytes = proto::encode_request(&request).unwrap();
    let error = proto::decode::<PublishedRequest>(&bytes).unwrap_err();
    assert!(error.contains("OpenTerminalOnce"), "{error}");
    let decoded: Request = proto::decode(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        serde_json::to_value(&request).unwrap()
    );
    let handoff = crate::ipc::Handoff {
        cookie: "fixture-cookie".into(),
        op: crate::ipc::Op::OpenTerminalOn {
            machine: "fixture-machine".into(),
            start: terminal_start(),
        },
    };
    let bytes = proto::encode(&handoff).unwrap();
    let error = proto::decode::<PublishedHandoff>(&bytes).unwrap_err();
    assert!(error.contains("OpenTerminalOn"), "{error}");
    let decoded: crate::ipc::Handoff = proto::decode(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        serde_json::to_value(&handoff).unwrap()
    );
}

#[test]
fn optional_windows_shell_preserves_default_startup_frames_and_cannot_be_silently_dropped() {
    use crate::shell_start::{ShellStart, WindowsShell};

    #[derive(Debug, Serialize, Deserialize)]
    struct BeforeShellSelection {
        token: String,
        title: String,
        cwd: String,
        command: String,
        cols: u16,
        rows: u16,
        headless: bool,
        argv: Option<Vec<String>>,
    }

    let mut start = terminal_start();
    let before: BeforeShellSelection =
        serde_json::from_value(serde_json::to_value(&start).unwrap()).unwrap();
    let bytes = proto::encode(&before).unwrap();
    assert_eq!(proto::encode(&start).unwrap(), bytes);
    assert_eq!(proto::decode::<ShellStart>(&bytes).unwrap().shell, None);

    for shell in [WindowsShell::PowerShell, WindowsShell::Cmd] {
        start.shell = Some(shell);
        let bytes = proto::encode(&start).unwrap();
        assert!(proto::decode::<BeforeShellSelection>(&bytes).is_err());
        assert_eq!(
            proto::decode::<ShellStart>(&bytes).unwrap().shell,
            Some(shell)
        );
    }
}

#[test]
fn execution_words_are_refused_instead_of_dispatching_any_published_operation() {
    let action = crate::execution::Action::CommandTask(crate::command_task::Request::Start {
        token: "new-command-task-token".into(),
        command: String::new(),
        argv: Some(vec!["program".into(), "literal $HOME %PATH% 中文".into()]),
        cwd: "/fixture".into(),
        timeout_ms: Some(1000),
    });
    let request = Request::Execution {
        action: action.clone(),
    };
    let bytes = proto::encode_request(&request).unwrap();
    let error = proto::decode::<PublishedRequest>(&bytes).unwrap_err();
    assert!(error.contains("Execution"), "{error}");
    let decoded = proto::question(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(decoded).unwrap(),
        serde_json::to_value(request).unwrap()
    );
    let handoff = crate::ipc::Handoff {
        cookie: "fixture-cookie".into(),
        op: crate::ipc::Op::ExecutionOn {
            machine: "fixture-machine".into(),
            action,
        },
    };
    let bytes = proto::encode(&handoff).unwrap();
    let error = proto::decode::<PublishedHandoff>(&bytes).unwrap_err();
    assert!(error.contains("ExecutionOn"), "{error}");
    let decoded: crate::ipc::Handoff = proto::decode(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(decoded).unwrap(),
        serde_json::to_value(handoff).unwrap()
    );
    let value =
        crate::execution::Reply::Terminal(crate::terminal_control::Reply::Ended { value: None });
    let bytes = proto::encode_response(&Response::Execution {
        value: value.clone(),
    })
    .unwrap();
    assert!(
        proto::decode::<PublishedResponse>(&bytes)
            .unwrap_err()
            .contains("Execution")
    );
    assert!(matches!(
        proto::decode::<Response>(&bytes),
        Ok(Response::Execution { .. })
    ));
    let bytes = proto::encode(&crate::ipc::Reply::Execution { value }).unwrap();
    assert!(
        proto::decode::<PublishedResponse>(&bytes)
            .unwrap_err()
            .contains("Execution")
    );
    assert!(matches!(
        proto::decode::<crate::ipc::Reply>(&bytes),
        Ok(crate::ipc::Reply::Execution { .. })
    ));
}

#[test]
fn published_directory_and_session_words_keep_their_actual_messagepack_bytes() {
    let published = proto::encode(&PublishedRequest::Sessions).unwrap();
    assert_eq!(published, b"\xa8Sessions");
    assert_eq!(
        proto::encode_request(&Request::Sessions).unwrap(),
        published
    );
    assert!(matches!(
        proto::decode::<Request>(&published),
        Ok(Request::Sessions)
    ));

    let published = PublishedRequest::Ls {
        path: "/fixture".into(),
    };
    let bytes = proto::encode_request(&Request::Ls {
        path: "/fixture".into(),
    })
    .unwrap();
    assert_eq!(bytes, proto::encode(&published).unwrap());
    assert_eq!(
        proto::decode::<PublishedRequest>(&bytes).unwrap(),
        published
    );
    assert!(
        matches!(proto::decode::<Request>(&bytes), Ok(Request::Ls { path }) if path == "/fixture")
    );

    let published = PublishedResponse::Refused {
        why: "fixture refusal".into(),
    };
    let bytes = proto::encode_response(&Response::Refused {
        why: "fixture refusal".into(),
    })
    .unwrap();
    assert_eq!(bytes, proto::encode(&published).unwrap());
    assert!(
        matches!(proto::decode::<PublishedResponse>(&bytes).unwrap(), PublishedResponse::Refused { why } if why == "fixture refusal")
    );
    assert!(
        matches!(proto::decode::<Response>(&bytes), Ok(Response::Refused { why }) if why == "fixture refusal")
    );

    let bytes = proto::encode_response(&Response::SessionRows { rows: vec![] }).unwrap();
    assert!(
        matches!(proto::decode::<PublishedResponse>(&bytes).unwrap(), PublishedResponse::SessionRows { rows } if rows.is_empty())
    );
}

#[test]
fn an_import_word_is_refused_by_published_decoders_without_becoming_an_old_operation() {
    let request = Request::SessionImport {
        home: khor_core::DeviceId([42; 32]),
        action: khor_core::session_import::SessionImportAction::Read,
    };
    let bytes = proto::encode_request(&request).unwrap();
    let why = proto::decode::<PublishedRequest>(&bytes).unwrap_err();
    assert!(why.contains("SessionImport"), "{why}");
    assert!(matches!(
        proto::decode::<Request>(&bytes),
        Ok(Request::SessionImport { .. })
    ));
    let bytes = proto::encode_response(&Response::SessionImport {
        value: khor_core::session_import::SessionImportReply::Imported {
            home: "fixture".into(),
            id: "fixture-import".into(),
            intent: khor_core::session_import::ImportIntent::Import,
        },
    })
    .unwrap();
    assert!(
        proto::decode::<PublishedResponse>(&bytes)
            .unwrap_err()
            .contains("SessionImport")
    );
    assert!(matches!(
        proto::decode::<Response>(&bytes),
        Ok(Response::SessionImport { .. })
    ));
}

#[test]
fn audited_additions_preserve_the_published_runtime_node_and_viewer_contract() {
    let contract = current().unwrap().release.contract;
    assert_eq!(
        contract.node,
        "khor/0:sha256:7bdf9a8cedcc26b783f3a6714899df875cbcb429e2f1ee64aa5a7d065e75fcda",
        "published live residents must remain eligible during upgrade"
    );
    assert_eq!(contract.terminal, 5);
    assert_eq!(crate::host::CONTROL_VERSION, 6);
}
