//! Real operations calibrate the shared caller-outcome recorder.

use std::future::Future;
use std::time::{Duration, Instant};

async fn measure(
    case: &str,
    node: &str,
    within: Duration,
    request: impl Future<Output = Result<(), String>>,
) -> (Duration, Option<String>) {
    crate::attempts::measure(case, node, "beta", within, request).await
}

/// Real success, remote error, a frozen peer's timeout, cancellation, then
/// recovery. The external parser must retain every started request exactly once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "isolated outcome calibration; never alongside timed netlab"]
async fn real_request_outcomes_survive_errors_timeouts_and_cancellation() {
    let pair = super::pair("measure-check", async |_, _| {}).await;
    let target = pair.b.home.0.join("measure-target");
    std::fs::create_dir(&target).unwrap();
    let target = target.to_string_lossy().into_owned();
    let read = |path: String| {
        pair.a
            .with(move |n| async move { n.ls_of("beta", &path).await.map(|_| ()) })
    };
    let within = Duration::from_secs(5);
    let (_, error) = measure("m-ok", &pair.a.id, within, read(target.clone())).await;
    assert!(error.is_none(), "real request succeeds: {error:?}");
    let (_, error) = measure(
        "m-error",
        &pair.a.id,
        within,
        read(format!("{target}/missing")),
    )
    .await;
    assert!(error.is_some(), "the absent remote directory is an error");

    struct Thaw(i32);
    impl Drop for Thaw {
        fn drop(&mut self) {
            crate::scratch::signal(self.0, "-CONT");
        }
    }
    let thaw = Thaw(pair.b.pid());
    pair.b.freeze();
    let began = Instant::now();
    loop {
        let proc = format!("/proc/{}/status", pair.b.pid());
        let status = std::fs::read_to_string(proc).unwrap();
        if status.lines().any(|line| line.starts_with("State:\tT")) {
            break;
        }
        assert!(began.elapsed() < Duration::from_secs(1), "peer is stopped");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let short = Duration::from_millis(200);
    let (took, error) = measure("m-timeout", &pair.a.id, short, read(target.clone())).await;
    assert!(
        took >= short && error.is_some(),
        "frozen peer reaches the deadline"
    );
    let (started, observed) = tokio::sync::oneshot::channel();
    let request = async {
        let _ = started.send(());
        read(target.clone()).await
    };
    let mut pending = Box::pin(measure("m-cancel", &pair.a.id, within, request));
    tokio::select! {
        _ = &mut pending => panic!("a frozen peer must not answer before cancellation"),
        () = async {
            observed.await.expect("the logical request was actually polled");
            tokio::time::sleep(Duration::from_millis(50)).await;
        } => {}
    }
    drop(pending);
    drop(thaw);
    let (_, error) = measure("m-resumed", &pair.a.id, within, read(target)).await;
    assert!(
        error.is_none(),
        "the same peer answers after resuming: {error:?}"
    );
    crate::lab::result(
        "measure-check",
        true,
        "five real logical requests: two successes, one error, one timeout, one cancellation",
    );
}
