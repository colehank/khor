//! A relay reached through the HTTP proxy the environment names
//! (docs/CONNECT.md 路径: UDP 被封走 443 中继、必要时经代理 — iroh's
//! `Builder::proxy_from_env`).
//!
//! Its own test binary on purpose: it sets a process-wide environment
//! variable, and every endpoint any other test in the same process bound
//! while it was set would go through a proxy that answers nothing.
//!
//! The proxy here only records the first line of each request and never
//! answers, so no relay is ever reached — the relay address is in
//! TEST-NET-2 (RFC 5737) and nothing routes it. What is asserted is
//! that the attempt arrived at the proxy.

use std::time::Duration;

use khor_net::endpoint::{self, Relay};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::time::timeout;

const RELAY: &str = "http://198.51.100.7:3340";
const PROXY_VARS: [&str; 6] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
];

/// A TCP listener that sends back the first line of every request it
/// is handed, and holds the connection open without answering.
async fn recording_proxy() -> (String, mpsc::UnboundedReceiver<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut line = Vec::new();
                let mut byte = [0u8; 1];
                while sock.read_exact(&mut byte).await.is_ok() && byte[0] != b'\n' {
                    line.push(byte[0]);
                }
                let _ = tx.send(String::from_utf8_lossy(&line).trim().to_string());
                // Held, not answered: a proxy that never completes the
                // tunnel keeps iroh from ever reaching the relay.
                tokio::time::sleep(Duration::from_secs(30)).await;
            });
        }
    });
    (url, rx)
}

#[tokio::test]
async fn the_relay_road_goes_through_the_proxy_the_environment_names() {
    for k in PROXY_VARS {
        unsafe { std::env::remove_var(k) };
    }
    let (proxy, mut heard) = recording_proxy().await;
    let relays = [Relay::bare(RELAY)];

    // Control: with no proxy in the environment the relay road goes
    // straight at the relay, and the proxy hears nothing — otherwise the
    // line below would prove only that *something* talks to it.
    let ep = endpoint::bind(iroh::SecretKey::generate(), &relays)
        .await
        .unwrap();
    let quiet = timeout(Duration::from_secs(3), heard.recv()).await;
    assert!(
        quiet.is_err(),
        "control: with no proxy set, the proxy must hear nothing, heard {quiet:?}"
    );
    ep.close().await;

    unsafe { std::env::set_var("HTTPS_PROXY", &proxy) };
    let ep = endpoint::bind(iroh::SecretKey::generate(), &relays)
        .await
        .unwrap();
    let line = timeout(Duration::from_secs(20), heard.recv())
        .await
        .expect("with HTTPS_PROXY set, the relay road must reach the proxy within 20s")
        .expect("the recording proxy stopped");
    assert!(
        line.contains("198.51.100.7:3340"),
        "the proxy must be asked for the relay, heard {line:?}"
    );
    ep.close().await;
    unsafe { std::env::remove_var("HTTPS_PROXY") };
}
