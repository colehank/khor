use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn marker(n: usize) -> Event {
    Event::Ready {
        session: SessionId::new(n.to_string()),
    }
}

#[tokio::test]
async fn a_fence_crosses_the_actual_forwarders_queued_batch_before_ack() {
    let (source, input) = mpsc::unbounded_channel();
    let (target, mut output) = mpsc::unbounded_channel();
    for n in 0..4096 {
        source.send(marker(n)).unwrap();
    }
    let fence = start_event_forwarder(input, target);
    assert_eq!(output.len(), 0, "the forwarder has not been polled yet");
    fence.wait().await.unwrap();
    assert_eq!(output.len(), 4096);
    for n in 0..4096 {
        match output.try_recv().unwrap() {
            Event::Ready { session } => assert_eq!(session.0.as_ref(), n.to_string()),
            _ => panic!("unexpected event"),
        }
    }
}

#[tokio::test]
async fn later_traffic_does_not_extend_a_fence_forever() {
    let (source, input) = mpsc::unbounded_channel();
    let (target, _output) = mpsc::unbounded_channel();
    let fence = start_event_forwarder(input, target);
    let stopped = Arc::new(AtomicBool::new(false));
    let count = Arc::new(AtomicUsize::new(0));
    let producer = {
        let stopped = stopped.clone();
        let count = count.clone();
        tokio::spawn(async move {
            while !stopped.load(Ordering::Acquire) {
                for n in 0..256 {
                    if source.send(marker(n)).is_err() {
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
    tokio::time::timeout(Duration::from_secs(2), fence.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(!producer.is_finished());
    stopped.store(true, Ordering::Release);
    producer.await.unwrap();
}

#[tokio::test]
async fn source_eof_confirms_its_tail_and_a_closed_target_is_an_explicit_error() {
    let (source, input) = mpsc::unbounded_channel();
    let (target, mut output) = mpsc::unbounded_channel();
    source.send(marker(7)).unwrap();
    drop(source);
    let fence = start_event_forwarder(input, target);
    fence.wait().await.unwrap();
    assert!(matches!(output.recv().await, Some(Event::Ready { .. })));
    assert!(output.recv().await.is_none());
    fence.wait().await.unwrap();

    let (source, input) = mpsc::unbounded_channel();
    let (target, output) = mpsc::unbounded_channel();
    drop(output);
    source.send(marker(8)).unwrap();
    let fence = start_event_forwarder(input, target);
    assert!(fence.wait().await.unwrap_err().contains("receiver closed"));
}

#[test]
fn dropping_an_unpolled_runtime_wakes_a_surviving_fence_with_an_error() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (_source, input) = mpsc::unbounded_channel();
    let (target, _output) = mpsc::unbounded_channel();
    let fence = {
        let _entered = runtime.enter();
        start_event_forwarder(input, target)
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
