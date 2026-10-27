//! Q039 N0b: a large devices bootstrap must survive a retry by the same caller.
//! Uses real Node pairing and the hermetic relay, without a replacement peer.
mod util;

use khor_net::endpoint::Ticket;
use khor_node::Node;
use khor_sync::{
    devices::{DeviceDoc, devices_dir},
    store::{self, Doc},
    transfer::MAX_BLOB_BYTES,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{task::JoinHandle, time::timeout};

const OPERATION_BY: Duration = Duration::from_secs(90);
const WORKER_BY: Duration = Duration::from_secs(180);
const PROCESS_BY: Duration = Duration::from_secs(210);

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

#[test]
fn a_large_pair_bootstrap_converges_and_the_same_caller_can_retry() {
    let base = std::env::var_os("KHOR_NODE_PAIR_BOOTSTRAP_SCRATCH")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&base).unwrap();
    let root = base.join(format!(
        "khor-pair-bootstrap-{}-{}",
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
                "large_pair_bootstrap_worker",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("KHOR_NODE_PAIR_BOOTSTRAP_ROOT", &worker.root)
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
            "pair bootstrap worker exceeded its process deadline\n{}",
            fs::read_to_string(&log_path).unwrap_or_default()
        );
        std::thread::sleep(Duration::from_millis(25));
    };
    let text = fs::read_to_string(&log_path).unwrap();
    eprintln!("{text}");
    assert!(
        text.contains("[Q039-PAIR] started"),
        "the ordinary parent must actually execute its worker"
    );
    assert!(status.success(), "the real Pair worker failed: {status}");
    assert!(
        text.contains("[Q039-PAIR] passed"),
        "zero tests or an unrelated passing worker is not acceptance"
    );
}

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
                "the fixture issuer exited before binding"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("the fixture issuer must bind within its deadline");
}

fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

/// A fixed 12288 x 64 legitimate history on the issuer's existing device.
/// Keep this test local rather than changing that already frozen fixture.
fn large_address_history(doc: &DeviceDoc, id: &str, name: &str, real_addrs: &[String]) {
    let mut seed = 0x9e3779b97f4a7c15;
    for _ in 0..12288 {
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

async fn exercise(root: &Path) {
    let issuer_root = root.join("issuer");
    let caller_root = root.join("caller");
    let other_root = root.join("other");
    util::hermetic(&issuer_root);
    util::hermetic(&caller_root);
    util::hermetic(&other_root);
    let resident = Node::open_hosting(issuer_root.clone(), "bootstrap-issuer", false).unwrap();
    assert!(!resident.relays_n0());
    assert_eq!(resident.relay_map().len(), 1);
    let serving = Serving(tokio::spawn(resident.serve()));
    listening(&issuer_root, &serving).await;
    let issuer = Node::open_hosting(issuer_root.clone(), "bootstrap-issuer", false).unwrap();
    let caller = Node::open_hosting(caller_root.clone(), "bootstrap-caller", false).unwrap();
    assert!(!caller.relays_n0());
    assert_eq!(caller.relay_map().len(), 1);
    let issuer_id = issuer.device_str().to_owned();
    let caller_id = caller.device_str().to_owned();
    let expected_root = issuer
        .root_state()
        .root
        .expect("the fresh issuer has a root");
    assert!(issuer.holds_root());
    assert_ne!(caller.root_state().root.as_ref(), Some(&expected_root));
    let ticket = issuer.invite().unwrap();
    // The raw token is used only for its owned fixture path, never logged.
    let token = Ticket::decode(&ticket).unwrap().token;
    let invite_path = issuer_root.join(".khor/invites").join({
        let mut hash = blake3::Hasher::new();
        hash.update(b"khor-pair-ticket-v1\0");
        hash.update(token.as_bytes());
        hash.finalize().to_hex().to_string()
    });
    assert!(
        invite_path.is_file(),
        "the fixture starts with an unspent ticket"
    );
    let expected_receive_dir = root.join("issuer-receive-directory").display().to_string();
    let (target, epoch, snapshot_bytes) = {
        let mut loaded = store::load_pile::<DeviceDoc>(&devices_dir(&issuer_root), 0x3950).unwrap();
        let me = loaded.doc.get(&issuer_id).unwrap();
        large_address_history(&loaded.doc, &me.id, &me.name, &me.addrs);
        loaded
            .doc
            .set_receive_dir(&issuer_id, &expected_receive_dir)
            .unwrap();
        loaded.store.flush(&loaded.doc).unwrap();
        let restored = loaded.doc.get(&issuer_id).unwrap();
        assert_eq!(restored.name, me.name);
        assert_eq!(restored.addrs, me.addrs);
        assert_eq!(loaded.doc.all().len(), 1, "no artificial device is added");
        (
            loaded.doc.version(),
            loaded.store.epoch(),
            loaded.doc.snapshot().unwrap().len(),
        )
    };
    eprintln!(
        "[Q039-PAIR] prepared snapshot_bytes={snapshot_bytes} epoch={epoch} ticket_present=true"
    );
    assert!(
        snapshot_bytes > 24 * 1024 * 1024,
        "retain the full 25.4 MB history"
    );
    assert!(
        snapshot_bytes > khor_node::proto::MAX_FRAME,
        "the old inline reply cannot fit"
    );
    assert!(
        snapshot_bytes as u64 <= MAX_BLOB_BYTES,
        "the body fits the new transfer contract"
    );
    assert!(
        !store::load_pile::<DeviceDoc>(&devices_dir(&caller_root), 0x3951)
            .unwrap()
            .doc
            .version()
            .includes_vv(&target),
        "the caller starts without the fixed issuer history"
    );
    let first = timeout(OPERATION_BY, caller.pair(&ticket))
        .await
        .expect("the first real Pair call must finish");
    let admitted_after_first = store::load_pile::<DeviceDoc>(&devices_dir(&issuer_root), 0x3952)
        .unwrap()
        .doc
        .get(&caller_id)
        .is_some();
    let ticket_consumed_after_first = !invite_path.exists();
    let first_error_key = first
        .as_ref()
        .err()
        .and_then(|why| khor_catalog::read(why).map(|(key, _)| key));
    let first_error = first.as_ref().err().map(|why| {
        why.replace(&ticket, "[redacted]")
            .replace(&token, "[redacted]")
    });
    eprintln!(
        "[Q039-PAIR] first_ok={} first_error_key={first_error_key:?} first_error={first_error:?} issuer_has_caller={admitted_after_first} ticket_consumed={ticket_consumed_after_first}",
        first.is_ok()
    );
    assert!(
        admitted_after_first,
        "reach the real issuer enrollment, not an unrelated dial failure"
    );
    assert!(
        ticket_consumed_after_first,
        "reach the real token burn before testing retry recovery"
    );

    // Same public caller identity and same ticket. A completed local join
    // intent may answer this idempotently; it must not grant a new admission.
    let second = timeout(OPERATION_BY, caller.pair(&ticket))
        .await
        .expect("the same caller's retry must finish");
    let second_error_key = second
        .as_ref()
        .err()
        .and_then(|why| khor_catalog::read(why).map(|(key, _)| key));
    eprintln!(
        "[Q039-PAIR] retry_ok={} retry_error_key={second_error_key:?}",
        second.is_ok()
    );

    let other = Node::open_hosting(other_root.clone(), "bootstrap-other", false).unwrap();
    assert!(!other.relays_n0());
    assert_eq!(other.relay_map().len(), 1);
    let other_id = other.device_str().to_owned();
    let other_original_root = other.root_state().root;
    let other_result = timeout(OPERATION_BY, other.pair(&ticket))
        .await
        .expect("a different caller must receive a bounded refusal");
    let other_error = other_result
        .as_ref()
        .expect_err("a consumed ticket cannot admit a different identity");
    let other_error_key = khor_catalog::read(other_error).map(|(key, _)| key);
    eprintln!("[Q039-PAIR] other_error_key={other_error_key:?}");
    assert_eq!(
        other_error_key,
        Some("msg.bad-invite"),
        "the issuer must reject the consumed ticket; a transport or IO failure is not authentication refusal"
    );
    assert_eq!(other.devices().unwrap().len(), 1);
    assert_eq!(other.root_state().root, other_original_root);
    let issuer_after = store::load_pile::<DeviceDoc>(&devices_dir(&issuer_root), 0x3953).unwrap();
    assert_eq!(issuer_after.doc.all().len(), 2);
    assert!(issuer_after.doc.get(&caller_id).is_some());
    assert!(issuer_after.doc.get(&other_id).is_none());
    drop(issuer_after);

    let caller_after = store::load_pile::<DeviceDoc>(&devices_dir(&caller_root), 0x3954).unwrap();
    let target_covered = caller_after.doc.version().includes_vv(&target);
    let caller_epoch = caller_after.store.epoch();
    let rows = caller_after.doc.all().len();
    let content_matches = caller_after
        .doc
        .get(&issuer_id)
        .is_some_and(|row| row.receive_dir.as_deref() == Some(expected_receive_dir.as_str()));
    drop(caller_after);
    let reopened = Node::open_hosting(caller_root, "bootstrap-caller", false).unwrap();
    let caller_trust = reopened.root_state();
    let same_root = caller_trust.root.as_ref() == Some(&expected_root);
    let first_success = first.as_ref().is_ok_and(|name| name == "bootstrap-issuer");
    let retry_success = second.as_ref().is_ok_and(|name| name == "bootstrap-issuer");
    eprintln!(
        "[Q039-PAIR] result first_success={first_success} retry_success={retry_success} target_covered={target_covered} content_matches={content_matches} epoch={caller_epoch}/{epoch} rows={rows} same_root={same_root} holds={} standing={} other_refused=true",
        caller_trust.holds, caller_trust.standing
    );
    serving.stop().await;
    assert!(
        retry_success,
        "the same caller must recover or idempotently finish its already admitted large bootstrap"
    );
    assert!(
        first_success || retry_success,
        "at least one real Pair call must complete"
    );
    assert_eq!(caller_epoch, epoch);
    assert_eq!(rows, 2);
    assert!(
        target_covered,
        "the fixed issuer history must be persisted on the caller"
    );
    assert!(
        content_matches,
        "the issuer's receive-dir value must reach the caller"
    );
    assert!(
        same_root,
        "the caller must adopt the issuer's verified root"
    );
    assert!(!caller_trust.holds, "the caller's lone root must give way");
    assert_eq!(caller_trust.standing, "trusted");
}

#[test]
#[ignore = "dedicated process fixture, actually launched by the ordinary parent"]
fn large_pair_bootstrap_worker() {
    let root = PathBuf::from(
        std::env::var_os("KHOR_NODE_PAIR_BOOTSTRAP_ROOT").expect("parent owns this scratch root"),
    );
    eprintln!("[Q039-PAIR] started");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        timeout(WORKER_BY, exercise(&root))
            .await
            .expect("the complete real Pair fixture must finish within its deadline");
    });
    runtime.shutdown_timeout(Duration::from_secs(2));
    eprintln!("[Q039-PAIR] passed");
}
