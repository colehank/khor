//! Shared by the wire-control halves of the acceptance tests. Each test
//! binary compiles this module and uses only the helpers it needs, so a
//! helper unused by one is dead code there — allowed, not a real unused.
#![allow(dead_code)]

use khor_net::endpoint::{self, ALPN, TUNNEL_ALPN};
use khor_node::proto::{self, MAX_FRAME, Request, Response};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::time::timeout;

/// A bare wire request from an arbitrary key — the shape a hostile or
/// merely confused client would send.
pub async fn raw_request(
    secret: iroh::SecretKey,
    target_id: &str,
    target_addrs: &[String],
    req: &Request,
) -> Result<Response, String> {
    let ep = endpoint::bind(secret, &[])
        .await
        .map_err(|e| e.to_string())?;
    let addr = endpoint::dial_addr(target_id, target_addrs, &[]).map_err(|e| e.to_string())?;
    let conn = timeout(Duration::from_secs(10), ep.connect(addr, ALPN))
        .await
        .map_err(|_| "dial timed out".to_string())?
        .map_err(|e| e.to_string())?;
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    send.write_all(&proto::encode(req)?)
        .await
        .map_err(|e| e.to_string())?;
    send.finish().map_err(|e| e.to_string())?;
    let bytes = recv
        .read_to_end(MAX_FRAME)
        .await
        .map_err(|e| e.to_string())?;
    let resp = proto::decode(&bytes)?;
    ep.close().await;
    Ok(resp)
}

/// A bare borrow-tunnel dial from an arbitrary key: speak the tunnel
/// ALPN, ask for `dest`, and return the exit's one status byte. The
/// control half of the tunnel test — an unpaired key must get `REFUSED`
/// (1) here while a paired key gets through the same live serve.
pub async fn raw_tunnel(
    secret: iroh::SecretKey,
    target_id: &str,
    target_addrs: &[String],
    dest: &str,
) -> Result<u8, String> {
    let ep = endpoint::bind(secret, &[])
        .await
        .map_err(|e| e.to_string())?;
    let addr = endpoint::dial_addr(target_id, target_addrs, &[]).map_err(|e| e.to_string())?;
    let conn = timeout(Duration::from_secs(10), ep.connect(addr, TUNNEL_ALPN))
        .await
        .map_err(|_| "dial timed out".to_string())?
        .map_err(|e| e.to_string())?;
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    let bytes = dest.as_bytes();
    send.write_all(&(bytes.len() as u16).to_be_bytes())
        .await
        .map_err(|e| e.to_string())?;
    send.write_all(bytes).await.map_err(|e| e.to_string())?;
    let mut status = [0u8; 1];
    recv.read_exact(&mut status)
        .await
        .map_err(|e| e.to_string())?;
    ep.close().await;
    Ok(status[0])
}

/// **The tests' own relay**: iroh-relay's server in plain HTTP on a
/// loopback port, one per test binary, on a thread and runtime of its own
/// so it outlives every test's runtime. Its url.
pub fn loop_relay() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let relay = iroh_relay::server::RelayConfig::new(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    0,
                )));
                let mut cfg = iroh_relay::server::ServerConfig::default();
                cfg.relay = Some(relay);
                let server = iroh_relay::server::Server::spawn(cfg)
                    .await
                    .expect("the tests' relay starts");
                let url = format!("http://{}", server.http_addr().expect("it listens"));
                let _ = tx.send(url);
                std::future::pending::<()>().await;
            });
        });
        rx.recv().expect("the tests' relay answers with its url")
    })
}

/// **A scratch home that touches no real relay** (round NET-2): it binds
/// with [`loop_relay`] and nothing else — `only_mine`, so neither the
/// Khor tier nor n0's relays and lookup are in its map (`relays.json`,
/// the file 设置 → 网络 writes). Call before the home's first node opens.
pub fn hermetic(root: &Path) {
    own_tmux();
    std::fs::create_dir_all(root.join(".khor")).unwrap();
    std::fs::write(
        root.join(".khor").join("relays.json"),
        format!(
            r#"{{"relays":[{{"url":"{}"}}],"only_mine":true}}"#,
            loop_relay()
        ),
    )
    .unwrap();
}

/// **This test process's own tmux, never the person's** (the CLI
/// harness's `own_tmux`, for the same incident): every session a test
/// here opens, and every sweep, talks to a server under a directory of
/// this binary's own on a socket of its own, and nothing it starts is
/// told it sits inside a tmux. Short, for the socket path's limit.
pub fn own_tmux() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        // One fixed directory for every test run rather than one per
        // process: nothing accumulates, and no test here starts a tmux
        // server on it, only asks it.
        let dir = std::env::temp_dir().join("khor-test-tmux");
        std::fs::create_dir_all(&dir).unwrap();
        // SAFETY: once per test binary, before its first node opens; the
        // values only ever point at this process's own.
        unsafe {
            std::env::set_var("TMUX_TMPDIR", &dir);
            std::env::set_var("KHOR_TMUX_SOCKET", "khor-scratch");
            std::env::remove_var("TMUX");
            std::env::remove_var("TMUX_PANE");
        }
    });
}
