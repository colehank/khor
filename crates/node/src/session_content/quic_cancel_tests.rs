//! Cancelling a local caller must retire both IPC and real QUIC reply owners.
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn leased_ipc_cancellation_releases_all_remote_encoded_replies() {
    let root_a = crate::testdir::TempTree::fresh("content-quic-cancel-reader");
    let root_b = crate::testdir::TempTree::fresh("content-quic-cancel-home");
    util::hermetic(&root_a);
    util::hermetic(&root_b);
    let mut serve_a = Server::start(&root_a, "cancel-reader").await;
    let mut serve_b = Server::start(&root_b, "cancel-home").await;
    let a = Arc::new(Node::open_as(root_a.path(), "cancel-reader").unwrap());
    let b = Node::open_as(root_b.path(), "cancel-home").unwrap();
    timeout(Duration::from_secs(60), a.pair(&b.invite().unwrap()))
        .await
        .expect("private nodes must pair")
        .unwrap();
    let id = "01a17c1a-a542-7822-b01d-82a832a158e6";
    let history: Vec<_> = (0..500)
        .flat_map(|n| message(&format!("quic-cancel-{n}")))
        .collect();
    std::fs::write(source(&root_b, id), history).unwrap();
    let observation = ObservationRef {
        owner: b.device(),
        home: b.device(),
        namespace: ObservationNamespace::Native,
        agent: "codex".into(),
        native_id: id.into(),
        imported_at: Millis(1),
        updated_at: None,
    };
    let locator =
        ContentLocator::observed(SessionId("observe/quic-cancel".into()), &observation).unwrap();
    let Reply::Resolved(Resolution::Available { binding, .. }) =
        a.content_read(Action::Resolve { locator }).await.unwrap()
    else {
        panic!("remote private source must resolve");
    };
    let held = HoldWork::install(&root_b, Stage::Response);
    let mut reads = Vec::new();
    let mut tasks = Vec::new();
    for _ in 0..crate::link::CONTENT_RESPONSES {
        let key = a.content_begin(&binding).await.unwrap();
        let read = ClientRead::claim(&root_a, binding.clone(), &key).unwrap();
        let (node, worker) = (a.clone(), read.clone());
        tasks.push(tokio::spawn(async move {
            node.content_page(&worker, None, false).await
        }));
        reads.push(read);
    }
    held.entered(crate::link::CONTENT_RESPONSES).await;
    assert_eq!(serve_a.responses.available_permits(), 0);
    assert_eq!(serve_b.responses.available_permits(), 0);
    for task in tasks {
        task.abort();
        let _ = task.await;
    }
    settled_responses(&[&serve_a, &serve_b]).await;
    assert!(!*held.1.released.lock().unwrap());
    assert_eq!(held.1.count.load(Ordering::Acquire), 32);
    // Control operations still answer while the encoded reply barrier is held.
    let key = a.content_begin(&binding).await.unwrap();
    a.content_cancel(&binding, &key).await.unwrap();
    assert!(!*held.1.released.lock().unwrap());
    held.release();
    assert_eq!(page(&a, &binding, None, false).await.events.len(), 128);
    settled_responses(&[&serve_a, &serve_b]).await;
    drop(reads);
    drop(held);
    serve_a.stop().await;
    serve_b.stop().await;
}
