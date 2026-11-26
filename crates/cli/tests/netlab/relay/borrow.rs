//! A retained borrow accepts fresh browser requests after its relay flow stalls.

use std::time::{Duration, Instant};

use khor_core::Hop;
use patchbay::Lab;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use super::{flows, wedge};
use crate::lab::{mesh, node, relay};
use crate::nat::{Net, router};

async fn browser_request(proxy: &str, target: &str) -> Result<(), String> {
    let mut pipe = BufReader::new(TcpStream::connect(proxy).await.map_err(|e| e.to_string())?);
    let request = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n\r\n");
    pipe.get_mut()
        .write_all(request.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let mut line = String::new();
    pipe.read_line(&mut line).await.map_err(|e| e.to_string())?;
    if !line.starts_with("HTTP/1.1 200 ") {
        return Err(format!("CONNECT: {line:?}"));
    }
    loop {
        line.clear();
        if pipe.read_line(&mut line).await.map_err(|e| e.to_string())? == 0 {
            return Err("EOF before CONNECT headers ended".into());
        }
        if line == "\r\n" {
            break;
        }
    }
    pipe.get_mut()
        .write_all(b"GET / HTTP/1.1\r\nHost: origin\r\nConnection: close\r\n\r\n")
        .await
        .map_err(|e| e.to_string())?;
    let mut reply = String::new();
    pipe.read_to_string(&mut reply)
        .await
        .map_err(|e| e.to_string())?;
    if !reply.ends_with("borrow-ok") {
        return Err(format!("unexpected origin response: {reply:?}"));
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real browser CONNECT and origin request on an isolated Relay fault"]
async fn borrowed_web_connect_recovers_after_relay_wedge() {
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, "borrow-probe", "relay", None).await;
    let ra = router(&lab, "left", Net::Strict).await;
    let rb = router(&lab, "right", Net::Strict).await;
    let da = lab.add_device("a").uplink(ra.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(rb.id()).build().await.unwrap();
    let a = node("borrow-probe", da, "alpha", &[&r], &[]).await;
    let b = node("borrow-probe", db, "beta", &[&r], &[]).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    mesh(&[&a, &b]).await;
    a.until(&b, Duration::from_secs(45), |l| {
        l.hop == Hop::Relay && l.rtt_ms.is_some()
    })
    .await
    .expect("original relay route");
    let (port_tx, port_rx) = oneshot::channel();
    let origin = b
        .dev
        .spawn(move |_| async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            port_tx
                .send(listener.local_addr().unwrap().to_string())
                .unwrap();
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut stream = BufReader::new(stream);
                    let mut line = String::new();
                    loop {
                        line.clear();
                        if stream.read_line(&mut line).await.unwrap() == 0 {
                            return;
                        }
                        if line == "\r\n" {
                            break;
                        }
                    }
                    let reply = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Length: 9\r\nConnection: close\r\n\r\n",
                        "borrow-ok",
                    );
                    stream.get_mut().write_all(reply.as_bytes()).await.unwrap();
                });
            }
        })
        .unwrap();
    let target = port_rx.await.unwrap();
    let (session, proxy) = a
        .with(|node| async move { node.borrow_unlisted("beta").await.unwrap() })
        .await;
    for request in 0..3 {
        if request == 1 {
            let client_ip = ra.uplink_ip().unwrap();
            let old = flows(&r.dev, client_ip);
            assert_eq!(
                old.len(),
                1,
                "one original relay flow before injecting the fault"
            );
            wedge(&r.dev, client_ip, &old);
        }
        let (proxy, target) = (proxy.clone(), target.clone());
        let started = Instant::now();
        let result = a
            .with(move |_| async move {
                match tokio::time::timeout(
                    Duration::from_secs(12),
                    browser_request(&proxy, &target),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err("browser caller exhausted its 12 s deadline".into()),
                }
            })
            .await;
        eprintln!(
            "[borrow-connect] request={request} latency_ms={} result={result:?}",
            started.elapsed().as_millis()
        );
        result.unwrap_or_else(|error| panic!("browser request {request} failed: {error}"));
    }
    a.with(move |node| async move { node.drop_borrow(&session).await.unwrap() })
        .await;
    origin.abort();
}
