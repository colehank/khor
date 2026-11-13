//! An authenticated Stop RPC reaches the real receiver's dispatcher while
//! its pull uses a second, deliberately held connection to the source.
use super::*;

struct Tokens(Vec<String>);
impl Drop for Tokens {
    fn drop(&mut self) {
        for token in &self.0 {
            crate::road::forget_job(token);
        }
    }
}

async fn endpoint() -> iroh::Endpoint {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![ALPN.to_vec()])
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .bind()
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_an_admitted_stop_for_the_right_token_cancels_a_live_pull() {
    let fixture = Fixture::with_identity(Held::All, true).await;
    let controller = endpoint().await;
    let stranger = endpoint().await;
    {
        let loaded = fixture.node.devices_loaded().unwrap();
        loaded
            .doc
            .upsert(&controller.id().to_string(), "stopper", &[])
            .unwrap();
        fixture
            .node
            .enroll_after_pair(&loaded.doc, &[(&controller.id().to_string(), "stopper")])
            .unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    assert!(fixture.node.admit(&controller.id().to_string()).is_ok());
    assert!(fixture.node.admit(&stranger.id().to_string()).is_err());
    let receiver = Arc::new(fixture.ep.clone());
    let mut serving = Aborts(Some(tokio::spawn({
        let (node, ep) = (fixture.node.clone(), receiver.clone());
        async move {
            let mut children = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    incoming = ep.accept() => {
                        let Some(incoming) = incoming else { break };
                        let conn = incoming.accept().unwrap().await.unwrap();
                        assert_eq!(conn.alpn(), ALPN);
                        let remote = conn.remote_id().to_string();
                        let (node, ep) = (node.clone(), ep.clone());
                        children.spawn(async move {
                            let (_done, done) = tokio::sync::watch::channel(Some(true));
                            node.handle(conn, remote, ep, done, Some(ALPN.to_vec())).await
                        });
                    }
                    child = children.join_next(), if !children.is_empty() => {
                        child.unwrap().unwrap().unwrap();
                    }
                }
            }
            children.shutdown().await;
        }
    })));
    let (trusted, unknown) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(
            controller.connect(receiver.addr(), ALPN),
            stranger.connect(receiver.addr(), ALPN)
        )
    })
    .await
    .unwrap();
    let (trusted, unknown) = (trusted.unwrap(), unknown.unwrap());
    assert_eq!(trusted.remote_id(), fixture.ep.id());
    let suffix = fixture.ep.id().to_string();
    let tokens = Tokens(vec![
        format!("rpc-stop-{}", &suffix[..24]),
        format!("rpc-other-{}", &suffix[..24]),
    ]);
    assert!(tokens.0.iter().all(|token| crate::once::valid(token)));
    let (job, fresh) = crate::road::job(&tokens.0[0]);
    assert!(fresh);
    let landed = crate::files::landing(PATH, fixture.root.path()).unwrap();
    let dest = landed.dest.clone();
    let outcome;
    {
        let pull = fixture.node.pull_into(
            &fixture.ep,
            "peer",
            PATH,
            landed,
            Some(job.as_ref()),
            Some(crate::drops::CAP),
            false,
        );
        tokio::pin!(pull);
        let controls = async {
            fixture
                .wait(|seen| seen.issued.len() == PULL_AHEAD as usize + 1)
                .await;
            let denied = request(
                &unknown,
                "unknown stopper",
                &Request::DropStop {
                    token: tokens.0[0].clone(),
                },
            )
            .await
            .unwrap();
            assert!(matches!(denied, Response::Refused { .. }));
            assert!(!job.is_stopped() && unknown.close_reason().is_none());
            let malformed = format!("bad/{suffix}");
            let refused = request(
                &trusted,
                "malformed token",
                &Request::DropStop {
                    token: malformed.clone(),
                },
            )
            .await
            .unwrap();
            assert!(matches!(refused, Response::Refused { .. }));
            assert!(crate::road::peek(&malformed).is_none() && !job.is_stopped());
            let wrong = request(
                &trusted,
                "wrong token",
                &Request::DropStop {
                    token: tokens.0[1].clone(),
                },
            )
            .await
            .unwrap();
            assert!(matches!(wrong, Response::DropStopped));
            assert!(crate::road::peek(&tokens.0[1]).unwrap().is_stopped());
            assert!(!job.is_stopped());
            assert_eq!(fixture.seen.lock().unwrap().stopped.len(), 0);
            eprintln!(
                "[pull-stop-rpc] {}",
                serde_json::json!({
                    "event":"negative_controls", "receiver":fixture.ep.id().to_string(),
                    "controller":controller.id().to_string(), "stranger":stranger.id().to_string(),
                    "unpaired_refused":true, "malformed_token_refused":true, "wrong_token_did_not_stop_target":true,
                    "issued":fixture.seen.lock().unwrap().issued,
                })
            );
            let stopped = request(
                &trusted,
                "right token",
                &Request::DropStop {
                    token: tokens.0[0].clone(),
                },
            )
            .await
            .unwrap();
            assert!(matches!(stopped, Response::DropStopped));
            assert!(job.is_stopped());
            let repeated = request(
                &trusted,
                "idempotent stop",
                &Request::DropStop {
                    token: tokens.0[0].clone(),
                },
            )
            .await
            .unwrap();
            assert!(matches!(repeated, Response::DropStopped));
            assert!(trusted.close_reason().is_none());
        };
        outcome = tokio::time::timeout(Duration::from_secs(10), async {
            let (pull, ()) = tokio::join!(&mut pull, controls);
            pull
        })
        .await;
    }
    fixture
        .wait(|seen| seen.stopped.len() == PULL_AHEAD as usize)
        .await;
    let returned = matches!(&outcome, Ok(Err(why)) if *why == crate::drops::cancelled());
    let clean =
        !fixture.part().exists() && !PullNote::path_of(&fixture.part()).exists() && !dest.exists();
    let healthy = trusted.close_reason().is_none()
        && fixture.local.close_reason().is_none()
        && fixture.remote.close_reason().is_none();
    eprintln!(
        "[pull-stop-rpc] {}",
        serde_json::json!({
            "event":"complete", "receiver":fixture.ep.id().to_string(), "controller":controller.id().to_string(),
            "cancelled_without_caller_drop":returned, "landing_clean":clean, "connections_healthy":healthy,
            "stop_idempotent":true, "source_stop_count":fixture.seen.lock().unwrap().stopped.len(),
            "control_wire":trusted.stable_id(), "source_wire":fixture.local.stable_id(),
            "scope":"real receiver dispatcher; controlled file source; no mobility or host-child claim",
        })
    );
    job.finish(Err(crate::drops::cancelled()));
    serving.0.as_ref().unwrap().abort();
    let _ = serving.0.take().unwrap().await;
    tokio::join!(controller.close(), stranger.close());
    drop(receiver);
    fixture.close().await;
    assert!(
        returned && clean && healthy,
        "remote stop did not retire its named pull: {outcome:?}"
    );
}
