//! A local resume must reach the actual connection admission before another poll.
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_resume_accepts_a_new_connection_before_the_next_pause_poll() {
    let root_a = crate::testdir::TempTree::fresh("resume-reader");
    let root_b = crate::testdir::TempTree::fresh("resume-home");
    util::hermetic(&root_a);
    util::hermetic(&root_b);
    let mut serve_a = Server::start(&root_a, "resume-reader").await;
    let mut serve_b = Server::start(&root_b, "resume-home").await;
    let a = Node::open_as(root_a.path(), "resume-reader").unwrap();
    let b = Node::open_as(root_b.path(), "resume-home").unwrap();
    timeout(Duration::from_secs(60), a.pair(&b.invite().unwrap()))
        .await
        .unwrap()
        .unwrap();
    let directory = root_b.path().join("only-this-resume-fixture");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("resume-marker.txt"), b"owned fixture\n").unwrap();
    let directory = directory.to_str().unwrap();
    let (_, entries, truncated) = a.ls_of("resume-home", directory).await.unwrap();
    assert!(!truncated);
    assert!(
        entries
            .iter()
            .any(|entry| entry.name == "resume-marker.txt")
    );

    let held = HoldWork::install(&root_b, Stage::PausePoll);
    crate::pause::set_on(&root_b, true).unwrap();
    held.entered(1).await;
    assert!(b.paused());
    // The real observer has set its admission bit and closed the old
    // wires. Keep the next poll blocked rather than racing a 500ms timer.
    crate::pause::set_on(&root_b, false).unwrap();
    assert!(!b.paused());
    let restored = timeout(Duration::from_secs(3), a.ls_of("resume-home", directory))
        .await
        .expect("local resume must accept a new connection within the existing call budget")
        .expect("a completed local resume must not retain the old paused admission");
    assert!(!*held.1.released.lock().unwrap());
    assert!(!restored.2);
    assert!(
        restored
            .1
            .iter()
            .any(|entry| entry.name == "resume-marker.txt")
    );
    held.release();
    serve_a.stop().await;
    serve_b.stop().await;
    eprintln!(
        "[resume-admission] local_resume=true next_poll_held=true real_directory_verified=true"
    );
}
