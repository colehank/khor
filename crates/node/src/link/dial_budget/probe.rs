//! Pairing diagnostics must stop with their owner and return the first success.

use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const NOT_PROBE: &[u8] =
    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
const ANSWER: &[u8] = b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n";

struct Probe {
    url: String,
    ready: Option<tokio::sync::oneshot::Receiver<()>>,
    replied: Option<tokio::sync::oneshot::Receiver<()>>,
    closed: Option<tokio::sync::oneshot::Receiver<bool>>,
    serving: Aborts<()>,
}

impl Probe {
    async fn new(answer: Option<tokio::sync::oneshot::Receiver<()>>) -> Self {
        // Never mutate process proxy settings while other network tasks run.
        // A proxy would make this direct loopback fixture invalid.
        assert!(
            ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy"]
                .iter()
                .all(|key| std::env::var(key).ok().is_none_or(|value| value.is_empty())),
            "HTTP probe fixture requires no environment proxy"
        );
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let url = format!("http://{}/probe", listener.local_addr().unwrap());
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (sent, replied) = tokio::sync::oneshot::channel();
        let (ended, closed) = tokio::sync::oneshot::channel();
        let serving = Aborts(Some(tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let headers =
                    tokio::time::timeout(Duration::from_millis(500), read_headers(&mut stream))
                        .await;
                let Ok(Some(headers)) = headers else {
                    continue;
                };
                // Ignore relay transport upgrades; observe the doctor's exact
                // ordinary GET, even if a pairing dial tries the same relay.
                let upgrade = headers
                    .windows(8)
                    .any(|part| part.eq_ignore_ascii_case(b"upgrade:"));
                if !headers.starts_with(b"GET /probe HTTP/1.1\r\n") || upgrade {
                    let _ = stream.write_all(NOT_PROBE).await;
                    continue;
                }
                entered.send(()).unwrap();
                match answer {
                    Some(release) => {
                        release.await.unwrap();
                        stream.write_all(ANSWER).await.unwrap();
                        let _ = sent.send(());
                    }
                    None => {
                        let mut byte = [0];
                        let stopped = matches!(stream.read(&mut byte).await, Ok(0));
                        let _ = ended.send(stopped);
                    }
                }
                return;
            }
        })));
        Self {
            url,
            ready: Some(ready),
            replied: Some(replied),
            closed: Some(closed),
            serving,
        }
    }

    async fn ready(&mut self) {
        tokio::time::timeout(Duration::from_secs(2), self.ready.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }

    async fn closed(&mut self) -> bool {
        matches!(
            tokio::time::timeout(Duration::from_millis(800), self.closed.take().unwrap()).await,
            Ok(Ok(true))
        )
    }

    async fn stop(&mut self) {
        self.serving.0.as_ref().unwrap().abort();
        let _ = self.serving.0.as_mut().unwrap().await;
    }
}

async fn read_headers(stream: &mut tokio::net::TcpStream) -> Option<Vec<u8>> {
    let mut headers = Vec::new();
    while headers.len() < 16 * 1024 {
        let mut chunk = [0; 1024];
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        headers.extend_from_slice(&chunk[..n]);
        if headers.windows(4).any(|part| part == b"\r\n\r\n") {
            return Some(headers);
        }
    }
    None
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_all_relay_probes_closes_their_http_requests() {
    let mut slow = Probe::new(None).await;
    let probing = crate::doctor::any_relay_answers(vec![slow.url.clone()]);
    let mut asking = Aborts(Some(tokio::spawn(probing)));
    slow.ready().await;
    asking.0.as_ref().unwrap().abort();
    assert!(matches!(asking.0.as_mut().unwrap().await, Err(error) if error.is_cancelled()));
    let closed = slow.closed().await;
    slow.stop().await;
    assert!(
        closed,
        "cancelled relay probe left its HTTP request running"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_pairing_also_cancels_its_relay_probe() {
    let mut slow = Probe::new(None).await;
    let (_root, node, peer, ep) = fixture().await;
    let node = std::sync::Arc::new(node);
    let ticket = Ticket {
        id: peer.id,
        name: "scratch issuer".into(),
        direct: vec!["127.0.0.1:9".into()],
        relays: vec![slow.url.clone()],
        token: "unused-cancellation-fixture".into(),
    }
    .encode()
    .unwrap();
    let mut asking = Aborts(Some(tokio::spawn({
        let node = node.clone();
        let ep = ep.clone();
        async move { node.pair_with(&ep, &ticket, vec![]).await }
    })));
    slow.ready().await;
    assert!(
        tokio::time::timeout(Duration::from_millis(20), asking.0.as_mut().unwrap())
            .await
            .is_err(),
        "fixture pairing must still be waiting for its unreachable issuer"
    );
    asking.0.as_ref().unwrap().abort();
    assert!(matches!(asking.0.as_mut().unwrap().await, Err(error) if error.is_cancelled()));
    let closed = slow.closed().await;
    ep.close().await;
    slow.stop().await;
    assert_eq!(node.wires.counts.snapshot().dial_failed, 0);
    assert!(
        closed,
        "cancelled pairing left its diagnostic probe running"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_successful_relay_probe_cancels_slower_requests() {
    let mut slow = Probe::new(None).await;
    let (release, reply_now) = tokio::sync::oneshot::channel();
    let mut fast = Probe::new(Some(reply_now)).await;
    let started = tokio::time::Instant::now();
    let probing = crate::doctor::any_relay_answers(vec![slow.url.clone(), fast.url.clone()]);
    let mut asking = Aborts(Some(tokio::spawn(probing)));
    slow.ready().await;
    fast.ready().await;
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_millis(500), fast.replied.take().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "fixture replies must precede the 2.6s HTTP timeout"
    );
    let answer = super::wire::observed_by(
        &mut asking,
        tokio::time::Instant::now() + Duration::from_millis(500),
    )
    .await;
    let closed = slow.closed().await;
    slow.stop().await;
    fast.stop().await;
    assert_eq!(
        answer,
        Some(true),
        "a stalled first URL must not delay a successful second URL"
    );
    assert!(
        closed,
        "a winning relay probe left a slower HTTP request running"
    );
}
