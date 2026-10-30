use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn marker(n: usize) -> SessionNotification {
    serde_json::from_value(
        serde_json::json!({"sessionId":"finite-event-store","update":{
            "sessionUpdate":"agent_message_chunk","content":{"type":"text","text":n.to_string()}
        }}),
    )
    .unwrap()
}
async fn opened_store() -> (event_store::Sender, EventReceiver, EventFence) {
    let (source, mut output, fence) = event_store::channel();
    source
        .ready(SessionId::new("finite-event-store".to_owned()))
        .unwrap();
    assert!(matches!(output.recv().await, Some(Event::Ready { .. })));
    (source, output, fence)
}

#[tokio::test]
async fn a_fence_crosses_the_actual_forwarders_queued_batch_before_ack() {
    let (source, mut output, fence) = opened_store().await;
    for n in 0..4096 {
        source.note(&marker(n)).await.unwrap();
    }
    // No historical note has been consumed. The worker must be able to spill
    // the whole base and acknowledge a finite prefix before any receiver drain.
    let prefix = fence.wait().await.unwrap();
    assert_eq!(output.remaining_through(&prefix).unwrap(), 4096);
    for n in 0..4096 {
        match output.recv().await.unwrap() {
            Event::Note(note) => assert_eq!(
                serde_json::to_value(note).unwrap()["update"]["content"]["text"],
                n.to_string()
            ),
            _ => panic!("unexpected event"),
        }
    }
    assert_eq!(output.remaining_through(&prefix).unwrap(), 0);
    drop(output);
    source.shutdown().await.unwrap();
}

#[tokio::test]
async fn later_traffic_does_not_extend_a_fence_forever() {
    let (source, output, fence) = opened_store().await;
    let stopped = Arc::new(AtomicBool::new(false));
    let count = Arc::new(AtomicUsize::new(0));
    let producer = {
        let stopped = stopped.clone();
        let count = count.clone();
        let source = source.clone();
        tokio::spawn(async move {
            while !stopped.load(Ordering::Acquire) {
                for n in 0..256 {
                    if source.note(&marker(n)).await.is_err() {
                        return;
                    }
                }
                count.fetch_add(256, Ordering::Release);
                tokio::task::yield_now().await;
            }
        })
    };
    while count.load(Ordering::Acquire) == 0 {
        tokio::task::yield_now().await;
    }
    let prefix = tokio::time::timeout(Duration::from_secs(2), fence.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(output.remaining_through(&prefix).unwrap() >= 256);
    assert!(!producer.is_finished());
    stopped.store(true, Ordering::Release);
    producer.await.unwrap();
    drop(output);
    source.shutdown().await.unwrap();
}

#[tokio::test]
async fn source_eof_confirms_its_tail_and_a_closed_target_is_an_explicit_error() {
    let (source, mut output, fence) = event_store::channel();
    source.ready(SessionId::new("7".to_owned())).unwrap();
    drop(source);
    let prefix = fence.wait().await.unwrap();
    assert_eq!(output.remaining_through(&prefix).unwrap(), 1);
    assert!(matches!(output.recv().await, Some(Event::Ready { .. })));
    assert!(output.recv().await.is_none());
    fence.wait().await.unwrap();

    let (source, output, fence) = event_store::channel();
    assert!(output
        .remaining_through(&prefix)
        .unwrap_err()
        .contains("another event store"));
    drop(output);
    assert!(source
        .ready(SessionId::new("8".to_owned()))
        .unwrap_err()
        .contains("receiver closed"));
    assert!(fence.wait().await.unwrap_err().contains("receiver closed"));
    source.shutdown().await.unwrap();
}

#[tokio::test]
async fn closing_freezes_the_prefix_but_closed_waits_for_the_actual_shutdown_fact() {
    let (source, mut output, fence) = opened_store().await;
    source.note(&marker(1)).await.unwrap();
    source.note(&marker(2)).await.unwrap();
    // Isolate the shutdown fact from write latency: both notes are already on
    // disk before the logical terminal event is reserved.
    fence.wait().await.unwrap();
    source.begin_close();
    assert!(source
        .note(&marker(3))
        .await
        .unwrap_err()
        .contains("closed"));
    let mut terminal_prefix = Box::pin(fence.wait());
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut terminal_prefix)
            .await
            .is_err()
    );
    source.closed(Some("confirmed fixture shutdown".into()));
    let prefix = terminal_prefix.await.unwrap();
    assert_eq!(output.remaining_through(&prefix).unwrap(), 3);
    for n in [1, 2] {
        match output.recv().await.unwrap() {
            Event::Note(note) => assert_eq!(
                serde_json::to_value(note).unwrap()["update"]["content"]["text"],
                n.to_string()
            ),
            _ => panic!("accepted tail was replaced by a terminal event"),
        }
    }
    assert!(matches!(output.recv().await,
        Some(Event::Closed(Some(error))) if error == "confirmed fixture shutdown"));
    assert_eq!(output.remaining_through(&prefix).unwrap(), 0);
    assert!(output.recv().await.is_none());
    source.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_opening_that_has_ended_cannot_publish_ready() {
    let (source, mut output, fence) = event_store::channel();
    source.begin_close();
    assert!(source
        .ready(SessionId::new("ended-opening".to_owned()))
        .unwrap_err()
        .contains("closed"));
    source.closed(None);
    let prefix = fence.wait().await.unwrap();
    assert_eq!(output.remaining_through(&prefix).unwrap(), 1);
    assert!(matches!(output.recv().await, Some(Event::Closed(None))));
    assert!(output.recv().await.is_none());
    assert_eq!(output.remaining_through(&prefix).unwrap(), 0);
    source.shutdown().await.unwrap();
}

#[test]
fn dropping_an_unpolled_runtime_wakes_a_surviving_fence_with_an_error() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    // Retain the actual source outside the runtime: its drop must not turn the
    // intended cancellation into an already-completed, legitimate source EOF.
    let (_source, _output, fence) = {
        let _entered = runtime.enter();
        event_store::channel()
    };
    drop(runtime);
    let next = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let error = next
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(1), fence.wait())
                .await
                .unwrap()
        })
        .unwrap_err();
    assert!(error.contains("cancelled"));
}
