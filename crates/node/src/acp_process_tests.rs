use super::*;
use std::time::Duration;

#[cfg(unix)]
#[test]
fn injected_transport_preserves_permissions_cooperative_cancel_and_shared_shutdown() {
    let root = std::env::temp_dir().join(format!(
        "khor-owned-acp-{}",
        crate::link::fresh_hex().unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    let script = root.join("agent.py");
    std::fs::write(&script, include_str!("fixtures/acp-owned.py")).unwrap();
    let command = serde_json::json!({"command":"python3","args":[script]}).to_string();
    let transport = spawn(&command).unwrap();
    let lifecycle = transport.lifecycle.clone();
    let control = transport.control.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (handle, mut events) = tokio::time::timeout(
            Duration::from_secs(3),
            khor_acp::start_configured_on(
                transport.streams,
                transport.lifecycle,
                root.clone(),
                None,
                khor_acp::Environment::default(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        let handle = Arc::new(handle);
        for cancel in [true, false] {
            let operation = control.begin_operation().unwrap();
            let pending = {
                let handle = handle.clone();
                tokio::spawn(async move { handle.prompt("synthetic permission turn").await })
            };
            let ask = loop {
                match tokio::time::timeout(Duration::from_secs(3), events.recv())
                    .await
                    .unwrap()
                    .unwrap()
                {
                    khor_acp::Event::Ask(ask) => break ask,
                    _ => {}
                }
            };
            if cancel {
                assert!(control.cancel_operation(&operation, Duration::from_secs(1)));
                handle.cancel().unwrap();
            }
            ask.dismiss();
            let stop = tokio::time::timeout(Duration::from_secs(3), pending)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(
                format!("{stop:?}"),
                if cancel { "Cancelled" } else { "EndTurn" }
            );
            assert!(control.complete_operation(&operation));
        }
        let clone = handle.clone();
        let outcome = tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            outcome,
            khor_acp::ShutdownOutcome::ProcessReaped(_)
        ));
        assert_eq!(clone.shutdown().await.unwrap(), outcome);
        lifecycle.confirmed_exit().await.unwrap();
        assert!(clone.prompt("no admission after shutdown").await.is_err());
    });
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_invalid_executable_fails_before_a_lifecycle_is_published() {
    assert!(spawn(r#"{"command":"/khor-synthetic-missing-executable"}"#).is_err());
}

#[test]
fn acp_launch_uses_the_complete_environment_then_applies_explicit_overrides() {
    let command =
        command_of(r#"{"command":"synthetic","env":{"KHOR_ACP_EXPLICIT":"synthetic-value"}}"#)
            .unwrap();
    let environment: std::collections::BTreeMap<_, _> = command
        .get_envs()
        .map(|(key, value)| (key.to_os_string(), value.map(|value| value.to_os_string())))
        .collect();
    for (key, value) in std::env::vars_os() {
        if key != "KHOR_ACP_EXPLICIT" {
            assert!(
                environment.get(&key) == Some(&Some(value)),
                "ACP lost an inherited environment entry"
            );
        }
    }
    assert_eq!(
        environment.get(std::ffi::OsStr::new("KHOR_ACP_EXPLICIT")),
        Some(&Some("synthetic-value".into()))
    );
    assert_eq!(
        command.get_current_dir(),
        Some(std::env::current_dir().unwrap().as_path())
    );
}

#[cfg(unix)]
#[test]
fn refused_initialization_closes_the_owned_process_without_a_handle() {
    let root = std::env::temp_dir().join(format!(
        "khor-owned-refusal-{}",
        crate::link::fresh_hex().unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    let script = root.join("agent.py");
    std::fs::write(&script, include_str!("fixtures/acp-owned.py")).unwrap();
    let transport =
        spawn(&serde_json::json!({"command":"python3","args":[script,"refuse"]}).to_string())
            .unwrap();
    let lifecycle = transport.lifecycle.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        assert!(
            tokio::time::timeout(
                Duration::from_secs(3),
                khor_acp::start_configured_on(
                    transport.streams,
                    transport.lifecycle,
                    root.clone(),
                    None,
                    khor_acp::Environment::default()
                )
            )
            .await
            .unwrap()
            .is_err()
        );
        futures::FutureExt::now_or_never(lifecycle.confirmed_exit())
            .expect("refusal returned before process confirmation")
            .unwrap();
        let pid = std::fs::read_to_string(script.with_extension("pid"))
            .unwrap()
            .parse::<libc::pid_t>()
            .unwrap();
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "refusal returned before the actual child was reaped"
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    });
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn dropping_the_driver_runtime_retires_the_process_even_when_a_handle_survives() {
    let root = std::env::temp_dir().join(format!(
        "khor-owned-runtime-{}",
        crate::link::fresh_hex().unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    let script = root.join("agent.py");
    std::fs::write(&script, include_str!("fixtures/acp-owned.py")).unwrap();
    let transport =
        spawn(&serde_json::json!({"command":"python3","args":[script]}).to_string()).unwrap();
    let lifecycle = transport.lifecycle.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (handle, _events) = runtime
        .block_on(khor_acp::start_configured_on(
            transport.streams,
            transport.lifecycle,
            root.clone(),
            None,
            khor_acp::Environment::default(),
        ))
        .unwrap();
    drop(runtime);
    let check = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    check.block_on(async {
        let outcome = tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            outcome,
            khor_acp::ShutdownOutcome::ProcessReaped(_)
        ));
        lifecycle.confirmed_exit().await.unwrap();
        assert!(
            handle
                .prompt("old handle must not admit work")
                .await
                .is_err()
        );
    });
    drop(handle);
    std::fs::remove_dir_all(root).unwrap();
}
