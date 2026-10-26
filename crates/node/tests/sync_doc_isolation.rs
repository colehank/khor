//! Q039-A: a failed required document must not suppress independent documents.
//! Real Node pairing/sync over the test relay; no replacement Node sync driver.
mod util;

use khor_node::Node;
use khor_sync::{
    accounts::{AccountDoc, accounts_dir},
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

#[test]
fn a_local_inventory_load_failure_reports_error_but_seen_and_pins_still_sync() {
    run_worker("inventory-local");
}

#[test]
fn a_remote_inventory_load_failure_reports_error_but_seen_and_pins_still_sync() {
    run_worker("inventory-remote");
}

#[test]
fn a_large_devices_gap_converges_through_real_node_transfer() {
    run_worker("convergence");
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

async fn exercise_inventory(root: &Path, case: &str) {
    let receiver = root.join("receiver");
    let sender = root.join("sender");
    util::hermetic(&receiver);
    util::hermetic(&sender);
    let resident = Node::open_hosting(receiver.clone(), "isolation-receiver", false).unwrap();
    assert!(!resident.relays_n0());
    assert_eq!(resident.relay_map().len(), 1);
    let serving = Serving(tokio::spawn(resident.serve()));
    listening(&receiver, &serving).await;
    let issuer = Node::open_hosting(receiver.clone(), "isolation-receiver", false).unwrap();
    let client = Node::open_hosting(sender.clone(), "isolation-sender", false).unwrap();
    assert!(!client.relays_n0());
    assert_eq!(client.relay_map().len(), 1);
    assert_eq!(
        timeout(OPERATION_BY, client.pair(&issuer.invite().unwrap()))
            .await
            .expect("inventory fixture pairing deadline")
            .expect("real pairing succeeds before introducing a store fault"),
        "isolation-receiver"
    );
    assert_eq!(client.devices().unwrap().len(), 2);
    assert_eq!(issuer.devices().unwrap().len(), 2);

    let faulty_home = if case == "inventory-local" {
        &sender
    } else {
        &receiver
    };
    let faulty_accounts = accounts_dir(faulty_home);
    // First create/load/compact a genuine, empty AccountDoc pile. No account
    // provider or credential is registered. Only after pairing, replace this
    // fixture's persisted directory with an ordinary file.
    fs::create_dir_all(&faulty_accounts).unwrap();
    {
        let mut valid = store::load_pile::<AccountDoc>(&faulty_accounts, 0x3930).unwrap();
        assert!(valid.broken.is_empty());
        assert!(valid.doc.all().is_empty());
        let snapshot = valid.store.compact(&valid.doc).unwrap();
        assert!(snapshot.is_file());
    }
    let saved_accounts = root.join("valid-accounts-before-inventory-fault");
    fs::rename(&faulty_accounts, &saved_accounts).unwrap();
    fs::write(
        &faulty_accounts,
        b"Q039: owned account directory replaced by a file",
    )
    .unwrap();
    assert!(faulty_accounts.is_file() && saved_accounts.is_dir());
    let io_error = fs::read_dir(&faulty_accounts)
        .err()
        .expect("real directory I/O must reject an ordinary file");
    let expected_reason = khor_catalog::msg::cant_list_dir(faulty_accounts.display(), io_error);
    let preflight_error = store::load_pile::<AccountDoc>(&faulty_accounts, 0x3931)
        .err()
        .expect("the actual AccountDoc load must fail");
    assert_eq!(preflight_error, expected_reason);

    {
        let mut seen = store::load_pile::<SeenDoc>(&seen_dir(&sender), 0x3932).unwrap();
        seen.doc.mark(SESSION, 123).unwrap();
        seen.store.flush(&seen.doc).unwrap();
        let mut pins = store::load_pile::<PinDoc>(&pins_dir(&sender), 0x3933).unwrap();
        pins.doc.set(SESSION, true).unwrap();
        pins.store.flush(&pins.doc).unwrap();
    }
    assert_eq!(
        store::load_pile::<SeenDoc>(&seen_dir(&receiver), 0x3934)
            .unwrap()
            .doc
            .watermark(SESSION),
        0
    );
    assert!(
        !store::load_pile::<PinDoc>(&pins_dir(&receiver), 0x3935)
            .unwrap()
            .doc
            .pinned(SESSION)
    );
    assert!(
        !serving.0.is_finished(),
        "the real receiver must still be serving"
    );
    eprintln!(
        "[Q039-INVENTORY] paired case={case} faulty_accounts={} expected_reason={expected_reason} baseline_seen=0 baseline_pin=false",
        faulty_accounts.display()
    );

    // The sender has no resident. Only this one real Node round may push its
    // independent docs. A Refused response's current fallback policy is not
    // simulated here; observed Err/Ok and actual disk contents decide the case.
    let outcomes = timeout(OPERATION_BY, client.sync_now())
        .await
        .expect("the inventory round must not hang")
        .expect("sync_now must return per-device outcomes");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].0, "isolation-receiver");
    let verdict = &outcomes[0].1;
    let seen = store::load_pile::<SeenDoc>(&seen_dir(&receiver), 0x3936)
        .unwrap()
        .doc
        .watermark(SESSION);
    let pinned = store::load_pile::<PinDoc>(&pins_dir(&receiver), 0x3937)
        .unwrap()
        .doc
        .pinned(SESSION);
    eprintln!(
        "[Q039-INVENTORY] outcome case={case} verdict={verdict:?} seen={seen} pinned={pinned}"
    );
    if let Err(why) = verdict {
        assert!(
            why.contains(&expected_reason),
            "an unrelated dial/transport failure is not the inventory business case: {why}"
        );
    }
    assert!(
        faulty_accounts.is_file() && saved_accounts.is_dir(),
        "sync must not erase the fixture's damaged or preserved store"
    );
    assert_eq!(
        seen, 123,
        "an accounts inventory load error must not suppress independent Seen"
    );
    assert!(
        pinned,
        "an accounts inventory load error must not suppress independent Pins"
    );
    verdict.as_ref().expect_err(
        "a concrete selected accounts load failure must remain visible as a round error",
    );
    serving.stop().await;
}

/// A separate acceptance oracle for the same legitimate history: unlike the
/// existing cap-rejection case, the complete fixed target must reach disk.
async fn exercise_convergence(root: &Path) {
    const MAX_VISITS: usize = 4;
    let receiver = root.join("receiver");
    let sender = root.join("sender");
    util::hermetic(&receiver);
    util::hermetic(&sender);

    let resident = Node::open_hosting(receiver.clone(), "isolation-receiver", false).unwrap();
    assert!(!resident.relays_n0());
    assert_eq!(resident.relay_map().len(), 1);
    let serving = Serving(tokio::spawn(resident.serve()));
    listening(&receiver, &serving).await;
    let issuer = Node::open_hosting(receiver.clone(), "isolation-receiver", false).unwrap();
    let client = Node::open_hosting(sender.clone(), "isolation-sender", false).unwrap();
    assert!(!client.relays_n0());
    assert_eq!(client.relay_map().len(), 1);
    assert_eq!(
        timeout(OPERATION_BY, client.pair(&issuer.invite().unwrap()))
            .await
            .expect("convergence fixture pairing deadline")
            .expect("real pairing succeeds before creating the large history"),
        "isolation-receiver"
    );
    assert_eq!(client.devices().unwrap().len(), 2);
    assert_eq!(issuer.devices().unwrap().len(), 2);

    let before = store::load_pile::<DeviceDoc>(&devices_dir(&receiver), 0x3940).unwrap();
    let before_version = before.doc.version();
    let receiver_epoch = before.store.epoch();
    drop(before);
    let expected_receive_dir = root.join("converged-receive-directory");
    let expected_receive_dir_text = expected_receive_dir.display().to_string();
    let (source_version, source_epoch, gap_bytes) = {
        let mut source = store::load_pile::<DeviceDoc>(&devices_dir(&sender), 0x3941).unwrap();
        let me = source.doc.get(client.device_str()).unwrap();
        large_address_history(&source.doc, &me.id, &me.name, &me.addrs);
        source
            .doc
            .set_receive_dir(client.device_str(), &expected_receive_dir_text)
            .unwrap();
        source.store.flush(&source.doc).unwrap();
        let restored = source.doc.get(client.device_str()).unwrap();
        assert_eq!(restored.name, me.name);
        assert_eq!(restored.addrs, me.addrs);
        assert_eq!(source.doc.all().len(), 2, "no artificial machine to dial");
        (
            source.doc.version(),
            source.store.epoch(),
            source.doc.changes_since(&before_version).unwrap().len(),
        )
    };
    assert_eq!(source_epoch, receiver_epoch);
    assert!(
        !before_version.includes_vv(&source_version),
        "the receiver must start without the fixed source target"
    );
    assert!(
        gap_bytes > wire::MAX_BYTES,
        "exceed the actual old live-wire cap"
    );
    assert!(
        gap_bytes > 24 * 1024 * 1024,
        "retain the full 25.4 MB history"
    );
    assert!(
        gap_bytes as u64 <= khor_sync::transfer::MAX_BLOB_BYTES,
        "the full history must fit the current bounded transfer contract"
    );
    {
        let mut seen = store::load_pile::<SeenDoc>(&seen_dir(&sender), 0x3942).unwrap();
        seen.doc.mark(SESSION, 123).unwrap();
        seen.store.flush(&seen.doc).unwrap();
        let mut pins = store::load_pile::<PinDoc>(&pins_dir(&sender), 0x3943).unwrap();
        pins.doc.set(SESSION, true).unwrap();
        pins.store.flush(&pins.doc).unwrap();
    }
    assert_eq!(
        store::load_pile::<SeenDoc>(&seen_dir(&receiver), 0x3944)
            .unwrap()
            .doc
            .watermark(SESSION),
        0
    );
    assert!(
        !store::load_pile::<PinDoc>(&pins_dir(&receiver), 0x3945)
            .unwrap()
            .doc
            .pinned(SESSION)
    );
    assert!(!expected_receive_dir.exists());
    eprintln!(
        "[Q039-CONVERGENCE] prepared raw_gap={gap_bytes} old_cap={} epoch={source_epoch}/{receiver_epoch} baseline_seen=0 baseline_pin=false max_visits={MAX_VISITS}",
        wire::MAX_BYTES
    );

    // The sender has no resident. Every visit uses the public Node driver;
    // the fixture never transports or imports the exported history itself.
    let mut completed = false;
    let mut cap_rejections = 0;
    for visit in 1..=MAX_VISITS {
        let outcomes = timeout(OPERATION_BY, client.sync_now())
            .await
            .expect("a convergence visit must not hang")
            .expect("sync_now returns per-device outcomes");
        assert_eq!(
            outcomes.len(),
            1,
            "only the real paired receiver is visited"
        );
        assert_eq!(outcomes[0].0, "isolation-receiver");
        let verdict = &outcomes[0].1;
        if let Err(why) = verdict {
            assert!(
                khor_catalog::read(why).is_some_and(|(key, _)| key == "msg.outbound-too-big"),
                "an unrelated sync/transport error is not the large-history oracle: {why}"
            );
            cap_rejections += 1;
        }
        let seen = store::load_pile::<SeenDoc>(&seen_dir(&receiver), 0x3946)
            .unwrap()
            .doc
            .watermark(SESSION);
        let pinned = store::load_pile::<PinDoc>(&pins_dir(&receiver), 0x3947)
            .unwrap()
            .doc
            .pinned(SESSION);
        assert_eq!(
            seen, 123,
            "the first visit must already persist independent Seen"
        );
        assert!(
            pinned,
            "the first visit must already persist independent Pins"
        );
        let after = store::load_pile::<DeviceDoc>(&devices_dir(&receiver), 0x3948).unwrap();
        assert_eq!(after.store.epoch(), receiver_epoch);
        let covered = after.doc.version().includes_vv(&source_version);
        let row = after.doc.get(client.device_str()).unwrap();
        let content_matches =
            row.receive_dir.as_deref() == Some(expected_receive_dir_text.as_str());
        eprintln!(
            "[Q039-CONVERGENCE] visit={visit} verdict={verdict:?} seen={seen} pinned={pinned} target_covered={covered} content_matches={content_matches} cap_rejections={cap_rejections} final_name={:?} final_addrs={:?}",
            row.name, row.addrs
        );
        assert_eq!(after.doc.all().len(), 2);
        if covered && content_matches {
            verdict
                .as_ref()
                .expect("a completed transfer must not retain a hard error");
            completed = true;
            break;
        }
    }
    // Release the actual resident before reporting the intended business red.
    serving.stop().await;
    assert!(
        completed,
        "the fixed devices target and its receive-dir content must converge after {MAX_VISITS} real Node visits; raw_gap={gap_bytes}, cap_rejections={cap_rejections}"
    );
}

#[test]
#[ignore = "dedicated process fixture, actually launched by the ordinary parents"]
fn node_doc_isolation_worker() {
    let case = std::env::var("KHOR_NODE_ISOLATION_CASE").expect("parent supplies its fixture case");
    assert!(matches!(
        case.as_str(),
        "oversize" | "epoch" | "inventory-local" | "inventory-remote" | "convergence"
    ));
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
        timeout(WORKER_BY, async {
            if case == "convergence" {
                exercise_convergence(&root).await;
            } else if case.starts_with("inventory-") {
                exercise_inventory(&root, &case).await;
            } else {
                exercise(&root, &case).await;
            }
        })
        .await
        .expect("the complete Node fixture must finish within its total deadline");
    });
    runtime.shutdown_timeout(Duration::from_secs(2));
    eprintln!("[Q039-NODE] passed case={case}");
}
