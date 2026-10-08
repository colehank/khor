//! Round ROAD-1: give an address, every machine tries it (`khor reach`).
//!
//! Two scratch residents, paired over a loopback relay, and a listener
//! on loopback that says an ssh banner. Each resident has its own ssh
//! world (`KHOR_SSH_DIR`) whose known_hosts holds a **different** key for
//! that address — two LANs' `192.168.1.5` — so the view must flag that
//! the name means two machines. Nothing here reads a real `~/.ssh`.

mod scratch;

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use khor_node::{Node, ReachOutcome};
use scratch::*;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

/// A fresh ed25519 public key, as a known_hosts line for `name`.
fn known_hosts_with_key(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let key = dir.join("id");
    let made = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "", "-f"])
        .arg(&key)
        .status()
        .unwrap();
    assert!(made.success());
    let public = std::fs::read_to_string(key.with_extension("pub")).unwrap();
    std::fs::write(dir.join("known_hosts"), format!("{name} {}", public.trim())).unwrap();
    std::fs::write(dir.join("config"), "").unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_machine_tries_the_address_and_two_keys_mean_two_machines() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            let _ = s.write_all(b"SSH-2.0-OpenSSH_9.9 scratch\r\n").await;
        }
    });

    let (ra, rb, rr) = (scratch("reach-a"), scratch("reach-b"), scratch("reach-r"));
    let (_relay, url) = local_relay(&rr.0);
    home_on(&ra.0, &url);
    home_on(&rb.0, &url);
    let name = format!("[127.0.0.1]:{port}");
    let (ssh_a, ssh_b) = (ra.0.join("ssh"), rb.0.join("ssh"));
    known_hosts_with_key(&ssh_a, &name);
    known_hosts_with_key(&ssh_b, &name);
    let _a = serve_with(&ra.0, "alpha", &[("KHOR_SSH_DIR", ssh_a.to_str().unwrap())]);
    let _b = serve_with(&rb.0, "beta", &[("KHOR_SSH_DIR", ssh_b.to_str().unwrap())]);
    stood_up(&[&ra.0, &rb.0]).await;
    let alpha = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = Node::open_as(rb.0.clone(), "beta").unwrap();
    timeout(Duration::from_secs(90), beta.pair(&alpha.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let view = timeout(Duration::from_secs(40), beta.reach_probe("127.0.0.1", port))
        .await
        .expect("the probe is bounded")
        .expect("beta's resident answers");
    eprintln!("{view:#?}");
    assert_eq!(view.rows.len(), 2, "both machines tried: {view:?}");
    assert_eq!(view.rows[0].0, beta.device_str(), "this machine first");
    for (id, s) in &view.rows {
        assert_eq!(s.outcome, ReachOutcome::Reached, "{id}: {s:?}");
        assert_eq!(s.banner.as_deref(), Some("SSH-2.0-OpenSSH_9.9 scratch"));
        assert!(s.host_key.as_deref().is_some_and(|k| k.len() == 64), "{id} read its known_hosts: {s:?}");
    }
    assert!(view.disagree, "two keys for one name are two machines");
    assert!(view.entrance.is_some(), "a machine that got through is the entrance");

    // A port nobody holds: every machine says refused, and there is no
    // entrance.
    let closed = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    let view = beta.reach_probe("127.0.0.1", closed).await.unwrap();
    assert!(view.rows.iter().all(|(_, s)| s.outcome == ReachOutcome::Refused), "{view:?}");
    assert_eq!(view.entrance, None);

    // The verb: one line per machine, the entrance, the warning.
    let out = khor(&rb.0)
        .args(["reach", "127.0.0.1", "--port", &port.to_string()])
        .env("KHOR_SSH_DIR", &ssh_b)
        .env("LC_ALL", "en_US.UTF-8")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    eprintln!("{text}{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success());
    assert!(text.contains("alpha") && text.contains("beta"), "{text}");
    assert!(text.contains("SSH-2.0-OpenSSH_9.9") && text.contains("SHA256:"), "{text}");
    assert!(text.contains("Entrance:") && text.contains("different keys"), "{text}");
}
