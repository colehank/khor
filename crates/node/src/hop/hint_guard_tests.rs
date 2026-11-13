//! A cold address-book reset cannot disturb an existing authenticated wire.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_live_peer_keeps_its_ip_hints_and_its_actual_wire() {
    let mut f = Publication::new().await;
    let serving = tokio::spawn({
        let conn = f.outer_remote.clone();
        async move {
            let mut recv = conn.accept_uni().await.unwrap();
            recv.read_to_end(32).await.unwrap() == b"keep-live-wire"
        }
    });
    let removed = tokio::time::timeout(
        Duration::from_secs(2),
        f.ep.discard_ip_hints_if_idle(
            f.middle_ep.id(),
            n0_future::time::Instant::now() + Duration::from_secs(1),
        ),
    )
    .await
    .unwrap();
    let mut send = f.outer_local.open_uni().await.unwrap();
    send.write_all(b"keep-live-wire").await.unwrap();
    send.finish().unwrap();
    let answered = tokio::time::timeout(Duration::from_secs(2), serving)
        .await
        .unwrap()
        .unwrap();
    let open = f.outer_local.close_reason().is_none() && f.outer_remote.close_reason().is_none();
    stop(&mut f.middle_task).await;
    f.middle_task = Owned(Some(tokio::spawn(async {})));
    f.close(0).await;
    assert_eq!(
        removed, 0,
        "execution-time live ownership must refuse the reset"
    );
    assert!(open && answered);
}
