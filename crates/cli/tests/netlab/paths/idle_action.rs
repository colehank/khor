//! Ownership admission before measuring a first foreground call after idle.
//! These guards are fixture evidence, not network recovery or energy SLOs.
use super::*;

use tokio::sync::oneshot;

struct Dropped(Option<oneshot::Sender<()>>);

impl Drop for Dropped {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

struct AbortControl(tokio::task::AbortHandle);

impl Drop for AbortControl {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "first-action fixture ownership admission, separate from network SLOs"]
async fn p8g_cancelled_with_call_releases_the_device_future() {
    let s = "p8g-with-cancel";
    let pair = speed_pair(s, 0).await;
    let (started, ready) = oneshot::channel();
    let (left, mut gone) = oneshot::channel();
    let (release, released) = oneshot::channel();
    {
        let request = pair.a.with(move |_| async move {
            let _dropped = Dropped(Some(left));
            let _ = started.send(());
            let _ = released.await;
        });
        tokio::pin!(request);
        tokio::select! {
            ready = ready => ready.expect("device future started"),
            _ = &mut request => panic!("the controlled request must remain pending"),
            _ = tokio::time::sleep(Duration::from_secs(5)) => panic!("device future startup deadline"),
        }
    }
    let stopped = matches!(
        tokio::time::timeout(Duration::from_secs(1), &mut gone).await,
        Ok(Ok(()))
    );
    // Reap even the deliberately red old implementation before asserting.
    let _ = release.send(());
    if !stopped {
        tokio::time::timeout(Duration::from_secs(5), gone)
            .await
            .expect("old detached control exits after release")
            .expect("device future drop was observed");
    }
    result(
        s,
        stopped,
        format!("caller_drop_released_device_future {stopped}"),
    );
    assert!(
        stopped,
        "dropping the caller must release its device future"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "first-action fixture ownership admission, separate from network SLOs"]
async fn p8g_detached_device_future_is_detected_and_reaped() {
    let s = "p8g-detached-control";
    let pair = speed_pair(s, 0).await;
    let (started, ready) = oneshot::channel();
    let (left, mut gone) = oneshot::channel();
    let task = pair
        .a
        .dev
        .spawn(move |_| async move {
            let _dropped = Dropped(Some(left));
            let _ = started.send(());
            std::future::pending::<()>().await;
        })
        .unwrap();
    let cleanup = AbortControl(task.abort_handle());
    tokio::time::timeout(Duration::from_secs(5), ready)
        .await
        .expect("raw control startup deadline")
        .expect("raw device future started");
    drop(task);
    let detected = tokio::time::timeout(Duration::from_millis(100), &mut gone)
        .await
        .is_err();
    cleanup.0.abort();
    if detected {
        tokio::time::timeout(Duration::from_secs(5), gone)
            .await
            .expect("raw detached control is reaped")
            .expect("raw device future drop was observed");
    }
    result(
        s,
        detected,
        format!("detached_future_detected_and_reaped {detected}"),
    );
    assert!(detected, "the guard detects a deliberately detached future");
}
