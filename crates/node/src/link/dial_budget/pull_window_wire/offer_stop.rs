//! The conversation receiver stop and sender cut must wake a real offered
//! file pull. The partial remains owned by the higher-level run_take caller.
use super::*;

struct NamedJob(String);

impl Drop for NamedJob {
    fn drop(&mut self) {
        crate::road::forget_job(&self.0);
    }
}

async fn observe_offer_stop(first: bool, cut: bool) {
    let fixture = Fixture::build(if first { Held::First } else { Held::All }, true).await;
    let file = chat::FileRef {
        name: "offered.bin".into(),
        size: fixture.bytes.len() as i64,
        digest: blake3::hash(&fixture.bytes).to_hex().to_string(),
        holder: String::new(),
    };
    let message = chat::Message {
        id: format!("offer-stop-{}", fixture.server.id()),
        at: 1,
        from: chat::Sender {
            id: fixture.server.id().to_string(),
            name: "peer".into(),
        },
        body: chat::MsgBody::Files(vec![file.clone()]),
        retracted: false,
        edited: false,
        fates: Default::default(),
        to: Vec::new(),
        note: String::new(),
        command: None,
    };
    let token = NamedJob(crate::road::sent_key(
        &message.id,
        fixture.node.device_str(),
    ));
    let (job, fresh) = crate::road::job(&token.0);
    assert!(fresh);
    let expected_issued = if first { 1 } else { PULL_AHEAD as usize + 1 };
    let expected_stopped = if first { 1 } else { PULL_AHEAD as usize };
    let peer = fixture
        .node
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "peer")
        .unwrap();
    let partial = crate::road::partial_of(fixture.root.path(), &file.digest).unwrap();
    let marker = crate::road::marker_of(fixture.root.path(), &file.digest).unwrap();
    let dest = fixture.root.path().join(&file.name);
    let before_moved;
    let returned;
    let cancelled;
    let clean_at_return;
    let elapsed;
    {
        let pull = fixture.node.pull_offer(
            &fixture.ep,
            &peer,
            &file,
            fixture.root.path(),
            crate::road::Mode::Offer,
            Some(job.as_ref()),
        );
        tokio::pin!(pull);
        tokio::select! {
            outcome = &mut pull => panic!("held pull returned before stop: {outcome:?}"),
            _ = fixture.wait(|seen| seen.issued.len() == expected_issued) => {},
        }
        before_moved = job.moved.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(before_moved, if first { 0 } else { proto::SLICE });
        let started = std::time::Instant::now();
        if cut {
            job.cut("sender left");
        } else {
            // Real conversation stop entry; its channel is not consulted while
            // this exact named Job is live, so no fake chat document is needed.
            fixture.node.stop_sent("unused-channel", &message).unwrap();
        }
        assert!(job.is_stopped());
        // This bound tests wakeup/ownership on an already established wire,
        // with replies deliberately held. It is not a WAN latency sample.
        let outcome = tokio::time::timeout(Duration::from_millis(500), &mut pull).await;
        elapsed = started.elapsed();
        returned = outcome.is_ok();
        cancelled = matches!(&outcome, Ok(Err(why)) if *why == crate::drops::cancelled());
        clean_at_return = !marker.exists()
            && !dest.exists()
            && partial.exists()
            && std::fs::metadata(&partial).unwrap().len() == before_moved;
        eprintln!(
            "[pull-offer-stop] {}",
            serde_json::json!({
                "event":"caller_observed", "first":first, "cut":cut, "peer":fixture.server.id().to_string(),
                "bound_ms":500, "elapsed_us":elapsed.as_micros(),
                "returned_without_caller_drop":returned, "cancelled":cancelled,
                "clean_at_return":clean_at_return, "moved_before":before_moved,
                "moved_after":job.moved.load(std::sync::atomic::Ordering::Relaxed),
                "issued":fixture.seen.lock().unwrap().issued,
            })
        );
        // On a red implementation only now drop the still-pending caller;
        // cleanup cannot be mistaken for a successful Job stop observation.
    }
    fixture
        .wait(|seen| seen.stopped.len() == expected_stopped)
        .await;
    let peer = fixture
        .node
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "peer")
        .unwrap();
    let control = tokio::time::timeout(
        Duration::from_secs(5),
        fixture.node.ask_on(&fixture.ep, &peer, &Request::Sessions),
    )
    .await
    .unwrap();
    let same_wire = fixture
        .node
        .wires
        .live(&peer.id, wires::Proto::Main)
        .is_some_and(|wire| wire.stable_id() == fixture.local.stable_id());
    let control_ok = matches!(control, Ok(Response::SessionRows { .. }))
        && same_wire
        && fixture.local.close_reason().is_none()
        && fixture.remote.close_reason().is_none();
    let remote_stopped = fixture.seen.lock().unwrap().stopped.len();
    eprintln!(
        "[pull-offer-stop] {}",
        serde_json::json!({
            "event":"cleanup_observed", "first":first, "cut":cut, "peer":fixture.server.id().to_string(),
            "remote_stopped":remote_stopped, "expected_stopped":expected_stopped,
            "same_wire_control":control_ok, "caller_returned_before_cleanup":returned,
            "job_table_cleanup_is_fixture_owned":true,
        })
    );
    job.finish(Err(crate::drops::cancelled()));
    fixture.close().await;
    assert!(
        control_ok,
        "a job stop must not retire the shared healthy wire"
    );
    assert!(
        returned && cancelled,
        "the named Job was stopped but the live caller still waited for its held reply: {elapsed:?}"
    );
    assert!(
        clean_at_return,
        "the offered pull must retire its marker and leave the exact partial to its run_take owner"
    );
    assert_eq!(
        job.moved.load(std::sync::atomic::Ordering::Relaxed),
        before_moved
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conversation_stop_wakes_the_first_pending_offer_reply() {
    observe_offer_stop(true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conversation_stop_wakes_every_pending_offer_reply() {
    observe_offer_stop(false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sender_cut_wakes_the_first_pending_offer_reply() {
    observe_offer_stop(true, true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sender_cut_wakes_every_pending_offer_reply() {
    observe_offer_stop(false, true).await;
}
