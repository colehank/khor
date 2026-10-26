//! Q039-A: a failed required document must not suppress independent documents.
//! Real Node pairing/sync over the test relay; no replacement Node sync driver.
mod util;

use khor_node::Node;
use khor_sync::{
    devices::{DeviceDoc, devices_dir},
    pins::{PinDoc, pins_dir},
    seen::{SeenDoc, seen_dir},
    store::{self, Doc},
    wire,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{task::JoinHandle, time::timeout};

const SESSION: &str = "tui/q039-doc-isolation";
const OPERATION_BY: Duration = Duration::from_secs(90);
const WORKER_BY: Duration = Duration::from_secs(180);
const PROCESS_BY: Duration = Duration::from_secs(210);

/// The parent owns the only child it can terminate. The child's relay thread,
/// Node tasks and private HOME cannot outlive this process-wide wall deadline.
struct Worker {
    root: PathBuf,
    child: Option<Child>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_worker(case: &str) {
    let base = std::env::var_os("KHOR_NODE_ISOLATION_SCRATCH")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&base).unwrap();
    let root = base.join(format!(
        "khor-doc-isolation-{case}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let mut worker = Worker { root, child: None };
    fs::create_dir(worker.root.join("private-home")).unwrap();
    let log_path = worker.root.join("worker.log");
    let log = fs::File::create(&log_path).unwrap();
    worker.child = Some(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "node_doc_isolation_worker",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("KHOR_NODE_ISOLATION_CASE", case)
            .env("KHOR_NODE_ISOLATION_ROOT", &worker.root)
            .env("HOME", worker.root.join("private-home"))
            .env("KHOR_CODEX_USAGE_URL", "http://127.0.0.1:9/no-vendor")
            .env("KHOR_CLAUDE_USAGE_URL", "http://127.0.0.1:9/no-vendor")
            .env_remove("CODEX_HOME")
            .env_remove("CLAUDE_CONFIG_DIR")
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("KHOR_HOME")
            .env_remove("KHOR_NAME")
            .env_remove("KHOR_RELAY")
            .env_remove("KHOR_RELAY_ONLY")
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap(),
    );
    let began = Instant::now();
    let status = loop {
        if let Some(status) = worker.child.as_mut().unwrap().try_wait().unwrap() {
            break status;
        }
        assert!(
            began.elapsed() < PROCESS_BY,
            "{case}: worker exceeded the total wall deadline\n{}",
            fs::read_to_string(&log_path).unwrap_or_default()
        );
        std::thread::sleep(Duration::from_millis(25));
    };
    let text = fs::read_to_string(&log_path).unwrap();
    eprintln!("{text}");
    assert!(
        text.contains(&format!("[Q039-NODE] started case={case}")),
        "the parent must actually execute its dedicated worker"
    );
    assert!(
        status.success(),
        "{case}: real Node worker failed: {status}"
    );
    assert!(
        text.contains(&format!("[Q039-NODE] passed case={case}")),
        "zero tests or an unrelated passing worker is not acceptance"
    );
}

#[test]
fn an_oversized_devices_gap_reports_failure_but_seen_and_pins_still_sync() {
    run_worker("oversize");
}

#[test]
fn a_different_devices_epoch_stays_apart_but_seen_and_pins_still_sync() {
    run_worker("epoch");
}

/// A failing assertion or timeout aborts this fixture's own resident. Worker
/// process teardown also reclaims the hermetic relay's dedicated runtime.
struct Serving(JoinHandle<Result<(), String>>);

impl Drop for Serving {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl Serving {
    async fn stop(mut self) {
        self.0.abort();
        let _ = (&mut self.0).await;
    }
}

async fn listening(root: &Path, serving: &Serving) {
    timeout(OPERATION_BY, async {
        while !root.join(".khor/endpoint.json").exists() {
            assert!(
                !serving.0.is_finished(),
                "the fixture resident exited before binding"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("the fixture resident must bind within its deadline");
}

fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

/// The same legitimate history as wire_catchup_gap, on the already paired
/// machine's existing id. Restore its real address hints before any sync.
fn large_address_history(doc: &DeviceDoc, id: &str, name: &str, real_addrs: &[String]) {
    let mut seed = 0x9e3779b97f4a7c15;
    for _ in 0..4096 {
        let addresses = (0..64)
            .map(|_| {
                let ip = (0x20010db8u128 << 96)
                    | (((next(&mut seed) as u128) << 64 | next(&mut seed) as u128)
                        & ((1u128 << 96) - 1));
                format!(
                    "[{}]:{}",
                    std::net::Ipv6Addr::from(ip),
                    1024 + next(&mut seed) % 55000
                )
            })
            .collect::<Vec<_>>();
        doc.upsert(id, name, &addresses).unwrap();
    }
    doc.upsert(id, name, real_addrs).unwrap();
}

async fn exercise(root: &Path, case: &str) {
    let receiver = root.join("receiver");
    let sender = root.join("sender");
    util::hermetic(&receiver);
    util::hermetic(&sender);

    // Existing public non-hosting mode keeps vendor discovery/process scans
    // out of this network-only fixture; pairing and sync are the real Node.
    let resident = Node::open_hosting(receiver.clone(), "isolation-receiver", false).unwrap();
    assert!(!resident.relays_n0());
    assert_eq!(resident.relay_map().len(), 1);
    let receiving_id = resident.device_str().to_owned();
    let serving = Serving(tokio::spawn(resident.serve()));
    listening(&receiver, &serving).await;
    let issuer = Node::open_hosting(receiver.clone(), "isolation-receiver", false).unwrap();
    let client = Node::open_hosting(sender.clone(), "isolation-sender", false).unwrap();
    assert!(!client.relays_n0());
    assert_eq!(client.relay_map().len(), 1);
    let paired = timeout(OPERATION_BY, client.pair(&issuer.invite().unwrap()))
        .await
        .expect("real pairing must not hang")
        .expect("real pairing must succeed before constructing the failing history");
    assert_eq!(paired, "isolation-receiver");
    assert_eq!(client.devices().unwrap().len(), 2);
    assert_eq!(issuer.devices().unwrap().len(), 2);

    let before = store::load_pile::<DeviceDoc>(&devices_dir(&receiver), 0x3901).unwrap();
    let before_version = before.doc.version();
    let receiver_epoch = before.store.epoch();
    let before_row = before.doc.get(client.device_str()).unwrap();
    drop(before);
    let must_not_follow = sender.join("must-not-follow-a-failed-devices-sync");
    let source_version;
    let unresolved_before;
    let gap_bytes;
    {
        let mut source = store::load_pile::<DeviceDoc>(&devices_dir(&sender), 0x3902).unwrap();
        let me = source.doc.get(client.device_str()).unwrap();
        assert_eq!(source.store.epoch(), receiver_epoch);
        if case == "oversize" {
            large_address_history(&source.doc, &me.id, &me.name, &me.addrs);
        }
        // Public Doc mutation only: Node::set_receive_dir would immediately
        // resolve this setting and would hide an erroneous follow_after_sync.
        source
            .doc
            .set_receive_dir(client.device_str(), &must_not_follow.display().to_string())
            .unwrap();
        source.store.flush(&source.doc).unwrap();
        let restored = source.doc.get(client.device_str()).unwrap();
        assert_eq!(restored.name, me.name);
        assert_eq!(restored.addrs, me.addrs);
        assert_eq!(
            source.doc.all().len(),
            2,
            "no artificial remote device to dial"
        );
        unresolved_before = restored.receive_for;
        source_version = source.doc.version();
        gap_bytes = source.doc.changes_since(&before_version).unwrap().len();
    }
    if case == "oversize" {
        assert!(
            gap_bytes > wire::MAX_BYTES,
            "fixture must exceed the real live-wire cap"
        );
        assert!(
            gap_bytes > 24 * 1024 * 1024,
            "retain the 25.4 MB research fixture"
        );
    } else {
        // A later on-disk epoch is not automatically reseeded down by this
        // build. This is actual store metadata, not a Node test-only policy.
        fs::write(
            devices_dir(&sender).join(".epoch"),
            format!("{}\n", store::EPOCH + 1),
        )
        .unwrap();
    }
    let sender_epoch = store::load_pile::<DeviceDoc>(&devices_dir(&sender), 0x3903)
        .unwrap()
        .store
        .epoch();
    if case == "epoch" {
        assert_ne!(sender_epoch, receiver_epoch);
    }
    {
        let mut seen = store::load_pile::<SeenDoc>(&seen_dir(&sender), 0x3904).unwrap();
        seen.doc.mark(SESSION, 123).unwrap();
        seen.store.flush(&seen.doc).unwrap();
        let mut pins = store::load_pile::<PinDoc>(&pins_dir(&sender), 0x3905).unwrap();
        pins.doc.set(SESSION, true).unwrap();
        pins.store.flush(&pins.doc).unwrap();
    }
    assert_eq!(
        store::load_pile::<SeenDoc>(&seen_dir(&receiver), 0x3906)
            .unwrap()
            .doc
            .watermark(SESSION),
        0
    );
    assert!(
        !store::load_pile::<PinDoc>(&pins_dir(&receiver), 0x3907)
            .unwrap()
            .doc
            .pinned(SESSION)
    );
    assert!(!must_not_follow.exists());
    eprintln!(
        "[Q039-NODE] paired case={case} receiver={receiving_id} sender={} raw_gap={gap_bytes} cap={} epoch={sender_epoch}/{receiver_epoch} baseline_seen=0 baseline_pin=false",
        client.device_str(),
        wire::MAX_BYTES
    );

    // Only B's explicit real Node round can push its Seen/Pins: B has no
    // resident or inbound request loop. Do not repair by syncing docs directly.
    let outcomes = timeout(OPERATION_BY, client.sync_now())
        .await
        .expect("the real sync round must not hang")
        .expect("sync_now must return a per-device outcome");
    assert_eq!(
        outcomes.len(),
        1,
        "only the actually paired receiver is visited"
    );
    let (name, verdict) = &outcomes[0];
    assert_eq!(name, "isolation-receiver");
    let seen = store::load_pile::<SeenDoc>(&seen_dir(&receiver), 0x3908)
        .unwrap()
        .doc
        .watermark(SESSION);
    let pinned = store::load_pile::<PinDoc>(&pins_dir(&receiver), 0x3909)
        .unwrap()
        .doc
        .pinned(SESSION);
    eprintln!("[Q039-NODE] outcome case={case} verdict={verdict:?} seen={seen} pinned={pinned}");
    if case == "oversize" {
        let error = verdict
            .as_ref()
            .expect_err("a required devices failure must not claim whole-round success");
        let cap_reason =
            khor_catalog::msg::outbound_too_big(gap_bytes / 1024, wire::MAX_BYTES / 1024);
        assert!(
            error.contains(&cap_reason),
            "retain the actual cap rejection, not an unrelated transport failure: {error}"
        );
    } else {
        // Existing modern behavior deliberately skips another epoch and
        // continues. This regression does not redefine it as a new round error.
        verdict
            .as_ref()
            .expect("another devices epoch remains an isolated skip");
    }
    assert!(
        !must_not_follow.exists(),
        "failed devices must not trigger receive-dir following"
    );
    let unchanged = store::load_pile::<DeviceDoc>(&devices_dir(&sender), 0x3910)
        .unwrap()
        .doc
        .get(client.device_str())
        .unwrap();
    assert_eq!(
        unchanged.receive_for, unresolved_before,
        "no follow_after_sync after devices failure"
    );
    let after = store::load_pile::<DeviceDoc>(&devices_dir(&receiver), 0x3911).unwrap();
    assert_eq!(after.store.epoch(), receiver_epoch);
    assert_eq!(
        after.doc.get(client.device_str()).unwrap().receive_dir,
        before_row.receive_dir,
        "failed or other-epoch devices changes must not merge"
    );
    assert!(
        !after.doc.version().includes_vv(&source_version),
        "the rejected devices history is still absent"
    );
    assert_eq!(
        seen, 123,
        "a devices failure must not suppress the same round's independent Seen document"
    );
    assert!(
        pinned,
        "a devices failure must not suppress the same round's independent Pins document"
    );
    serving.stop().await;
}

#[test]
#[ignore = "dedicated process fixture, actually launched by both isolation parents"]
fn node_doc_isolation_worker() {
    let case = std::env::var("KHOR_NODE_ISOLATION_CASE").expect("parent supplies its fixture case");
    assert!(matches!(case.as_str(), "oversize" | "epoch"));
    let root = PathBuf::from(
        std::env::var_os("KHOR_NODE_ISOLATION_ROOT").expect("parent owns this scratch root"),
    );
    eprintln!("[Q039-NODE] started case={case}");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        timeout(WORKER_BY, exercise(&root, &case))
            .await
            .expect("the complete Node fixture must finish within its total deadline");
    });
    runtime.shutdown_timeout(Duration::from_secs(2));
    eprintln!("[Q039-NODE] passed case={case}");
}
