//! 连接诊断 (docs/CONNECT.md 度量与验收): two real `khor serve`
//! processes on temporary homes pair, and the one that dialled reports
//! how long the dial took to get an answer and where it settled — through
//! its resident (`Node::road_tallies`) and on `khor doctor`'s `roads` row.
//!
//! Both homes are set to use only a `khor relay --dev` this test starts
//! on loopback (`relays.json`, only-mine), so nothing here touches the
//! Khor tier's real relay.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use khor_node::Node;
use tokio::time::timeout;

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-roads-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

const PROXY_VARS: [&str; 6] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
];

/// A child process, stopped by pid however the test leaves — TERM
/// first, so a serve's keeper takes its serve down with it.
struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .args(["-TERM", &self.0.id().to_string()])
            .status();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self.0.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Every khor this test starts: its own home, no relay from the shell,
/// no proxy (iroh's probes stopped reading the macOS system proxy on
/// 2026-10-01 — vendor/iroh/KHOR.md, patch 2 — so clearing the
/// variables is the whole of it).
fn khor(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_khor"));
    cmd.env("KHOR_HOME", home)
        .env("KHOR_WEB_PORT", "0")
        .env_remove("KHOR_RELAY")
        .env_remove("KHOR_SESSION");
    for k in PROXY_VARS {
        cmd.env_remove(k);
    }
    cmd
}

/// `khor relay --dev` on a free loopback port; its url.
fn local_relay(dir: &Path) -> (Reaped, String) {
    let mut child = khor(dir)
        .args(["relay", "--dev", "--bind", "127.0.0.1:0", "--state"])
        .arg(dir.join("state"))
        .stdout(Stdio::piped())
        .stderr(std::fs::File::create(dir.join("relay.log")).unwrap())
        .spawn()
        .unwrap();
    let first = std::io::BufReader::new(child.stdout.take().unwrap())
        .lines()
        .next()
        .expect("a banner line")
        .unwrap();
    let at = first.find("http://").expect("a url on the first line");
    let url = first[at..].split_whitespace().next().unwrap().to_string();
    (Reaped(child), url)
}

/// A home that binds with that relay and nothing else.
fn home_on(dir: &Path, relay: &str) {
    std::fs::create_dir_all(dir.join(".khor")).unwrap();
    std::fs::write(
        dir.join(".khor").join("relays.json"),
        serde_json::json!({ "relays": [{ "url": relay }], "only_mine": true }).to_string(),
    )
    .unwrap();
}

fn serve(dir: &Path, name: &str) -> Reaped {
    Reaped(
        khor(dir)
            .arg("serve")
            .env("KHOR_NAME", name)
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(dir.join("serve.log")).unwrap())
            .spawn()
            .unwrap(),
    )
}

fn doctor_row(dir: &Path, key: &str) -> serde_json::Value {
    let out = khor(dir).args(["doctor", "--json"]).output().unwrap();
    let rows: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "doctor --json did not print JSON ({e}): {}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    rows.as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == key)
        .cloned()
        .unwrap_or_else(|| panic!("no {key} row in {rows}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_dialling_resident_reports_first_reply_time_and_where_its_dials_settled() {
    let (ra, rb, rr) = (scratch("alpha"), scratch("beta"), scratch("relay"));
    let (_relay, url) = local_relay(&rr.0);
    home_on(&ra.0, &url);
    home_on(&rb.0, &url);
    let _a = serve(&ra.0, "alpha");
    let _b = serve(&rb.0, "beta");
    timeout(Duration::from_secs(60), async {
        while !(ra.0.join(".khor/endpoint.json").exists() && rb.0.join(".khor/endpoint.json").exists())
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("both serves stand up");

    let b = Node::open_as(rb.0.clone(), "beta").unwrap();
    // Control: before beta has dialled anybody its resident answers, and
    // the answer is empty — which the doctor says as 未量, not as a row
    // of zeros.
    assert_eq!(
        b.road_tallies().await,
        Ok(vec![]),
        "a resident that has dialled nobody has no tally"
    );
    assert_eq!(doctor_row(&rb.0, "roads")["state"], "unmeasured");

    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let ticket = a.invite().expect("alpha mints a ticket through its resident");
    let started = Instant::now();
    let peer = timeout(Duration::from_secs(90), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("pairing succeeds");
    assert_eq!(peer, "alpha");
    let pairing_took = started.elapsed();
    let alpha_id = a.device_str().to_owned();

    // The pairing was beta's dial; the resident has its time now, and
    // within the settle window where it settled.
    let deadline = Instant::now() + Duration::from_secs(30);
    let tally = loop {
        let got = b.road_tallies().await.expect("beta's resident answers");
        if let Some((_, t)) = got.iter().find(|(id, _)| *id == alpha_id)
            && t.direct + t.relay > 0
        {
            break *t;
        }
        assert!(
            Instant::now() < deadline,
            "beta never settled a dial to alpha: {got:?}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    };
    let ms = tally.first_reply_ms.expect("a dial that came back has a time");
    assert!(
        ms > 0 && u128::from(ms) <= pairing_took.as_millis(),
        "the first reply ({ms}ms) came inside the pairing that made it ({pairing_took:?})"
    );
    // Two residents on one machine, each with its loopback address in
    // the ticket and the table: the direct road is there to take, so a
    // dial that settled took it.
    assert!(
        tally.direct >= 1 && tally.relay == 0,
        "two residents on one machine settle direct: {tally:?}"
    );

    let row = doctor_row(&rb.0, "roads");
    let said = row["said"].as_str().unwrap_or_default().to_owned();
    // The reading itself, for whoever runs this with --nocapture.
    eprintln!("beta -> alpha: {tally:?}; pairing took {pairing_took:?}; doctor: {said}");
    assert_eq!(row["state"], "ok", "{row}");
    assert!(
        said.contains("alpha") && said.contains(&format!("{}/", tally.direct)),
        "the doctor names alpha and its direct count: {said}"
    );
}
