use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[tokio::test]
async fn shutdown_is_once_and_a_cancelled_waiter_does_not_cancel_the_owner() {
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    let (done, rx) = watch::channel(None);
    let lifecycle = TransportLifecycle::new(
        move || {
            count.fetch_add(1, Ordering::SeqCst);
        },
        rx,
    );
    let clone = lifecycle.clone();
    lifecycle.request_shutdown();
    clone.request_shutdown();
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    let pending = {
        let clone = clone.clone();
        tokio::spawn(async move { clone.confirmed_exit().await })
    };
    pending.abort();
    let _ = pending.await;
    assert!(
        tokio::time::timeout(Duration::from_millis(10), clone.confirmed_exit())
            .await
            .is_err()
    );
    done.send_replace(Some(Ok(ProcessExit { code: None })));
    assert_eq!(
        lifecycle.confirmed_exit().await.unwrap(),
        ProcessExit { code: None }
    );
    drop(lifecycle);
    drop(clone);
    assert_eq!(requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn losing_a_process_owner_is_not_confirmation() {
    let (done, rx) = watch::channel(None);
    let lifecycle = TransportLifecycle::new(|| {}, rx);
    drop(done);
    assert!(lifecycle
        .confirmed_exit()
        .await
        .unwrap_err()
        .contains("without exit confirmation"));
}

#[tokio::test]
async fn closing_admission_does_not_wait_for_other_handle_clones_or_a_pending_rpc() {
    let (close, rx) = oneshot::channel();
    let (done, result) = watch::channel(None);
    let gate = Arc::new(CloseGate::new(close, result, None));
    let pending = {
        let gate = gate.clone();
        tokio::spawn(async move {
            gate.while_open(std::future::pending::<Result<(), String>>())
                .await
        })
    };
    gate.request();
    rx.await.unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert_eq!(
        gate.driver_closed().await.unwrap(),
        ShutdownOutcome::DriverClosed
    );
    done.send_replace(Some(Ok(ShutdownOutcome::DriverClosed)));
    assert_eq!(
        gate.completed().await.unwrap(),
        ShutdownOutcome::DriverClosed
    );
}

#[tokio::test]
async fn a_delivered_terminal_response_is_not_replaced_by_a_simultaneous_close() {
    let (close, _rx) = oneshot::channel();
    let (_done, result) = watch::channel(None);
    let gate = Arc::new(CloseGate::new(close, result, None));
    let completed = gate.clone();
    let result = gate
        .while_open(async move {
            completed.request();
            Ok(7)
        })
        .await;
    assert_eq!(result.unwrap(), 7);
    assert!(gate.while_open(async { Ok(8) }).await.is_err());
}

#[tokio::test]
async fn a_dropped_driver_result_uses_the_independent_process_confirmation() {
    for result in [
        Ok(ProcessExit { code: None }),
        Err("synthetic owner could not confirm reap".to_owned()),
    ] {
        let (done, completion) = watch::channel(None);
        let provider = TransportLifecycle::new(|| {}, completion);
        let (close, _rx) = oneshot::channel();
        let (driver, result_rx) = watch::channel(None);
        let gate = CloseGate::new(close, result_rx, Some(provider));
        drop(driver);
        done.send_replace(Some(result.clone()));
        assert_eq!(
            gate.completed().await,
            result.map(ShutdownOutcome::ProcessReaped)
        );
    }
}

#[tokio::test]
async fn a_dropped_unowned_driver_is_not_a_fabricated_confirmation() {
    let (close, _rx) = oneshot::channel();
    let (driver, result) = watch::channel(None);
    let gate = CloseGate::new(close, result, None);
    drop(driver);
    assert!(gate
        .completed()
        .await
        .unwrap_err()
        .contains("without shutdown confirmation"));
}

#[tokio::test]
async fn an_owned_open_refusal_waits_for_confirmation_and_keeps_cleanup_failures_explicit() {
    for completion in [
        Ok(ProcessExit { code: None }),
        Err("synthetic reap failure".to_owned()),
    ] {
        let (done, rx) = watch::channel(None);
        let requested = Arc::new(tokio::sync::Notify::new());
        let signal = requested.clone();
        let lifecycle = TransportLifecycle::new(move || signal.notify_one(), rx);
        let agent = r#"{"command":"/khor-synthetic-missing-ACP-executable"}"#
            .parse::<agent_client_protocol::AcpAgent>()
            .unwrap();
        let pending = tokio::spawn(crate::start_configured_on(
            agent,
            lifecycle,
            std::env::temp_dir(),
            None,
            crate::Environment::default(),
        ));
        tokio::time::timeout(Duration::from_secs(1), requested.notified())
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(
            !pending.is_finished(),
            "new API returned a refusal before its caller-owned process was confirmed"
        );
        let failed = completion.is_err();
        done.send_replace(Some(completion));
        let refusal = tokio::time::timeout(Duration::from_secs(1), pending)
            .await
            .unwrap()
            .unwrap()
            .err()
            .expect("missing command is a refusal");
        assert!(matches!(refusal, crate::Refusal::Wont(_)));
        assert_eq!(refusal.said().contains("cleanup was not confirmed"), failed);
        if failed {
            assert!(refusal.said().contains("synthetic reap failure"));
        }
    }
}
