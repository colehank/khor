//! Private acceptance for the new Pair/Vouch path. Heavy cases run in their
//! own actual libtest process; helpers are ignored and never count as parents.
use super::*;
use crate::proto;
use khor_sync::{
    devices::{DeviceDoc, devices_dir},
    pins::{PinDoc, pins_dir},
    store,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::time::timeout;
#[path = "../tests/util/mod.rs"]
mod util;
const BY: Duration = Duration::from_secs(90);

struct Barrier {
    root: PathBuf,
    claim: Id,
    phase: String,
    marker: PathBuf,
}
static BARRIER: OnceLock<Mutex<Option<Barrier>>> = OnceLock::new();
/// Exact root/claim, installed only by the owned worker below. No production
/// environment switch, wire marker or extra dispatch is present.
pub(crate) fn checkpoint(context: &Context, claim: Id, phase: &str) {
    let Some(slot) = BARRIER.get() else {
        return;
    };
    let barrier = {
        let mut held = slot.lock().unwrap();
        if !held
            .as_ref()
            .is_some_and(|b| b.root == context.root && b.claim == claim && b.phase == phase)
        {
            return;
        }
        held.take().unwrap()
    };
    publish(
        &barrier.marker,
        &(std::process::id(), nt::id_hex(&claim), phase),
    );
    let by = Instant::now() + BY;
    while Instant::now() < by {
        std::thread::park_timeout(Duration::from_millis(25));
    }
    panic!("owned Pair caller was not killed at its exact settlement checkpoint");
}
struct Case(PathBuf, std::sync::Arc<std::sync::atomic::AtomicBool>);
impl Case {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "khor-pair-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(
            root,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        )
    }
    fn node(&self, part: &str) -> Node {
        let root = self.0.join(part);
        util::hermetic(&root);
        Node::open_hosting(root, part, false).unwrap()
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        if self.1.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = fs::remove_dir_all(&self.0);
        } else {
            eprintln!("owned Pair worker still alive; preserving its scratch");
        }
    }
}
fn publish<T: Serialize>(path: &Path, value: &T) {
    use std::io::Write;
    let bytes = serde_json::to_vec(value).unwrap();
    assert!(bytes.len() < 64 * 1024);
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path).unwrap();
    f.write_all(&bytes).unwrap();
    f.sync_all().unwrap();
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    assert!(fs::metadata(path).unwrap().len() < 64 * 1024);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
async fn wait_file(path: &Path) {
    timeout(BY, async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("real owned process must reach checkpoint");
}
struct Server(tokio::task::JoinHandle<Result<(), String>>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}
impl Server {
    async fn start(root: &Path, name: &str) -> Self {
        let node = Node::open_hosting(root.into(), name, false).unwrap();
        let server = Self(tokio::spawn(node.serve()));
        wait_file(&root.join(".khor/endpoint.json")).await;
        assert!(!server.0.is_finished());
        server
    }
    async fn stop(mut self) {
        self.0.abort();
        let _ = (&mut self.0).await;
    }
}
struct Worker(Child, bool, std::sync::Arc<std::sync::atomic::AtomicBool>);
impl Worker {
    fn start(case: &Case, control: &Path, log: &Path) -> Self {
        case.1.store(false, std::sync::atomic::Ordering::SeqCst);
        let output = fs::File::create(log).unwrap();
        Self(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "link::pairing::tests::interrupted_pair_caller_worker",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("KHOR_PAIR_RECOVERY_CONTROL", control)
                .stdin(Stdio::null())
                .stdout(output.try_clone().unwrap())
                .stderr(output)
                .spawn()
                .unwrap(),
            false,
            case.1.clone(),
        )
    }
    fn wait_success(&mut self, log: &Path) {
        let by = Instant::now() + BY;
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                self.1 = true;
                self.2.store(true, std::sync::atomic::Ordering::SeqCst);
                if !status.success() {
                    eprintln!("{}", fs::read_to_string(log).unwrap());
                }
                assert!(
                    status.success(),
                    "actual fresh verifier must pass: {status}"
                );
                break;
            }
            assert!(
                Instant::now() < by,
                "actual fresh verifier exceeded its deadline"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn kill_wait(&mut self) {
        use std::os::unix::process::ExitStatusExt;
        self.0.kill().unwrap();
        let by = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                self.1 = true;
                self.2.store(true, std::sync::atomic::Ordering::SeqCst);
                assert_eq!(status.signal(), Some(9));
                break;
            }
            assert!(Instant::now() < by, "owned Pair caller must be reaped");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if !self.1 {
            let _ = self.0.kill();
            let by = Instant::now() + Duration::from_secs(5);
            while Instant::now() < by {
                if self.0.try_wait().is_ok_and(|s| s.is_some()) {
                    self.1 = true;
                    self.2.store(true, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(
                self.1,
                "retain owned scratch: Pair caller could not be reaped"
            );
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Control {
    root: PathBuf,
    ticket: String,
    operation: Id,
    phase: String,
    marker: PathBuf,
}

#[test]
fn pair_roster_frames_are_bounded_and_cannot_be_document_proofs() {
    let raw = vec![7; CHUNK_BYTES + 1];
    let issuer = [1; 32];
    let caller = [2; 32];
    let nonce = [3; 32];
    let token = "abcdef";
    let operation = state::claim_id(issuer, caller, state::token_hash(token).unwrap(), nonce);
    let manifest =
        state::roster_manifest(operation, state::CALLER_ROSTER, caller, issuer, &raw).unwrap();
    assert!(manifest.descriptor().is_err());
    assert!(
        state::authorize_blob(&manifest, operation, state::ISSUER_ROSTER, caller, issuer).is_err()
    );
    assert!(
        state::authorize_blob(&manifest, [4; 32], state::CALLER_ROSTER, caller, issuer).is_err()
    );
    assert!(
        state::authorize_blob(&manifest, operation, state::CALLER_ROSTER, issuer, caller).is_err()
    );
    let have = DocumentVersion {
        epoch: 0,
        version: Default::default(),
    }
    .encode()
    .unwrap()
    .into();
    let header = PairHeader {
        name: "n".repeat(256),
        addrs: vec!["a".repeat(240); 64],
        devices_have: have,
        roster: Some(manifest.clone()),
    };
    let request = Request::Pair {
        action: PairAction::Start {
            token: token.into(),
            nonce,
            header: header.clone(),
            roster: Some(DocChunk {
                manifest: manifest.clone(),
                offset: 0,
                bytes: raw[..CHUNK_BYTES].to_vec().into(),
            }),
        },
    };
    let encoded = proto::encode_request(&request).unwrap();
    assert!(encoded.len() < proto::DOC_TRANSFER_FRAME_BYTES);
    proto::question(&encoded).unwrap();
    let mut padded = encoded;
    padded.resize(proto::DOC_TRANSFER_FRAME_BYTES + 1, 0);
    assert!(padded.len() < proto::MAX_FRAME);
    assert!(proto::question(&padded).is_err());
    let mut too_large = header;
    too_large.addrs = vec!["a".repeat(256); 64];
    assert!(state::validate_header(&too_large).is_err());
    assert_eq!(proto::MAX_FRAME, 6 * 1024 * 1024);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "owned SIGKILL workers and real Node Pair; run exact in isolation"]
async fn interrupted_pair_settlement_recovers_before_bootstrap_and_never_replays_after_leave() {
    for phase in ["anchor", "flushed"] {
        let case = Case::new(phase);
        let issuer = case.node("issuer");
        let caller = case.node("caller");
        let other = case.node("other");
        let server = Server::start(issuer.root(), "issuer").await;
        let ticket = issuer.invite().unwrap();
        let decoded = khor_net::endpoint::Ticket::decode(&ticket).unwrap();
        let context = caller.doc_context().unwrap();
        let remote = id(issuer.device_str()).unwrap();
        let hash = state::token_hash(&decoded.token).unwrap();
        let nonce = [19; 32];
        let operation = state::claim_id(remote, context.local, hash, nonce);
        let trust = PairTrust::new(context.clone(), "caller".into());
        let raw = trust.roster_offer().unwrap();
        let manifest =
            state::roster_manifest(operation, state::CALLER_ROSTER, context.local, remote, &raw)
                .unwrap();
        let initial_root = caller.root_state().root;
        {
            let mut state = State::open(&context).unwrap();
            state
                .save_local(state::Local {
                    id: operation,
                    issuer: remote,
                    nonce,
                    token_hash: hash,
                    header: PairHeader {
                        name: "caller".into(),
                        addrs: Vec::new(),
                        devices_have: canonical_version(&context).unwrap(),
                        roster: Some(manifest.clone()),
                    },
                    initial_anchor: trust.anchor_id(),
                    initial_epoch: context.version("devices").unwrap().epoch,
                    phase: PairPhase::Receiving,
                    anchor: None,
                    peer_name: String::new(),
                    peer_have: Vec::new().into(),
                    peer_roster: None,
                    roster_released: false,
                    trust_accepted: false,
                    devices_out: None,
                    devices_in: None,
                    devices_offer_decided: false,
                    completed_ms: None,
                })
                .unwrap();
            state.prepare_roster_blob(&manifest, &raw).unwrap();
        }
        let control = case.0.join("control.json");
        let marker = case.0.join("checkpoint.json");
        publish(
            &control,
            &Control {
                root: context.root.clone(),
                ticket: ticket.clone(),
                operation,
                phase: phase.into(),
                marker: marker.clone(),
            },
        );
        let log = case.0.join("caller.log");
        let mut worker = Worker::start(&case, &control, &log);
        wait_file(&marker).await;
        let (actual_pid, actual_id, actual_phase): (u32, String, String) = read(&marker);
        assert_eq!(actual_pid, worker.0.id());
        assert_eq!(actual_id, nt::id_hex(&operation));
        assert_eq!(actual_phase, phase);
        // Independent real business state written while the killed settlement
        // owns its leases must survive the actual fresh-process replay.
        let mut pins = store::load_pile::<PinDoc>(&pins_dir(&context.root), 0x3957).unwrap();
        pins.doc.set("tui/pair-concurrent", true).unwrap();
        pins.store.flush(&pins.doc).unwrap();
        drop(pins);
        worker.kill_wait();
        eprintln!("[Q039-PAIR-RECOVERY] phase={phase} owned_signal=9");
        assert!(
            !fs::read_to_string(&log)
                .unwrap()
                .contains("[PAIR-WORKER] returned")
        );
        let pre = State::open(&context).unwrap();
        let plan = pre.pair_settlement(operation).unwrap();
        assert!(plan.ready && !plan.finished);
        assert_ne!(plan.before_anchor, plan.desired_anchor);
        assert_eq!(
            fs::read_to_string(context.root.join(".khor/root.pub"))
                .unwrap()
                .trim(),
            issuer.root_state().root.unwrap()
        );
        assert!(
            context.root.join(".khor/root.key").exists(),
            "old key retires only after checked persisted work"
        );
        if phase == "flushed" {
            assert!(
                covers(
                    &canonical_version(&context).unwrap(),
                    plan.target.as_ref().unwrap()
                )
                .unwrap()
            );
        }
        drop(pre);
        // A different actual process performs Node::open's pre-bootstrap
        // recovery and the public same-claim handshake, not this parent.
        let verify_control = case.0.join("verify-control.json");
        let report = case.0.join("verified.json");
        publish(
            &verify_control,
            &Control {
                root: context.root.clone(),
                ticket: ticket.clone(),
                operation,
                phase: "verify".into(),
                marker: report.clone(),
            },
        );
        let verify_log = case.0.join("verify.log");
        let mut verifier = Worker::start(&case, &verify_control, &verify_log);
        verifier.wait_success(&verify_log);
        let (verifier_pid, actual_claim, verified): (u32, String, bool) = read(&report);
        assert_eq!(verifier_pid, verifier.0.id());
        assert_ne!(verifier_pid, actual_pid);
        assert_eq!(actual_claim, nt::id_hex(&operation));
        assert!(verified);
        eprintln!("{}", fs::read_to_string(&verify_log).unwrap());
        let reopened = Node::open_hosting(context.root.clone(), "caller", false).unwrap();
        assert_ne!(reopened.root_state().root, initial_root);
        assert_eq!(reopened.root_state().root, issuer.root_state().root);
        assert!(!context.root.join(".khor/root.key").exists());
        assert!(
            store::load_pile::<PinDoc>(&pins_dir(&context.root), 0x3958)
                .unwrap()
                .doc
                .pinned("tui/pair-concurrent")
        );
        assert!(
            State::open(&context)
                .unwrap()
                .pair_settlement(operation)
                .unwrap()
                .finished
        );
        let completed = timeout(BY, reopened.pair(&ticket)).await.unwrap().unwrap();
        assert_eq!(completed, "issuer");
        assert_eq!(
            timeout(BY, reopened.pair(&ticket)).await.unwrap().unwrap(),
            "issuer"
        );
        let other_root = other.root_state().root;
        let error = timeout(BY, other.pair(&ticket)).await.unwrap().unwrap_err();
        assert_eq!(
            khor_catalog::read(&error).map(|(key, _)| key),
            Some("msg.bad-invite")
        );
        assert_eq!(other.root_state().root, other_root);
        assert_eq!(other.devices().unwrap().len(), 1);
        timeout(BY, reopened.leave("caller"))
            .await
            .unwrap()
            .unwrap();
        let after_leave = reopened.root_state().root;
        assert!(timeout(BY, reopened.pair(&ticket)).await.unwrap().is_err());
        assert_eq!(reopened.root_state().root, after_leave);
        assert_eq!(reopened.devices().unwrap().len(), 1);
        server.stop().await;
        eprintln!(
            "[Q039-PAIR-RECOVERY] phase={phase} fresh_bootstrap=true same_claim=true wrong_caller_rejected=true leave_preserved=true"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "owned Pair caller helper; not an independent business parent"]
async fn interrupted_pair_caller_worker() {
    let control: Control = read(&PathBuf::from(
        std::env::var_os("KHOR_PAIR_RECOVERY_CONTROL").expect("owned worker only"),
    ));
    if control.phase == "verify" {
        let caller = Node::open_hosting(control.root.clone(), "caller", false).unwrap();
        let context = caller.doc_context().unwrap();
        assert!(!control.root.join(".khor/root.key").exists());
        assert!(
            store::load_pile::<PinDoc>(&pins_dir(&control.root), 0x3962)
                .unwrap()
                .doc
                .pinned("tui/pair-concurrent")
        );
        {
            let state = State::open(&context).unwrap();
            let plan = state.pair_settlement(control.operation).unwrap();
            assert!(plan.finished);
            assert!(
                covers(
                    &canonical_version(&context).unwrap(),
                    plan.target.as_ref().unwrap()
                )
                .unwrap()
            );
        }
        assert_eq!(
            timeout(BY, caller.pair(&control.ticket))
                .await
                .unwrap()
                .unwrap(),
            "issuer"
        );
        assert_eq!(
            timeout(BY, caller.pair(&control.ticket))
                .await
                .unwrap()
                .unwrap(),
            "issuer"
        );
        let state = State::open(&context).unwrap();
        let row = state.pair_local(control.operation).unwrap();
        assert_eq!(row.phase, PairPhase::BootstrapComplete);
        assert!(state.pending_docs(row.issuer).unwrap().is_empty());
        publish(
            &control.marker,
            &(std::process::id(), nt::id_hex(&control.operation), true),
        );
        eprintln!(
            "[PAIR-VERIFIER] fresh_process=true target_preserved=true same_claim_completed=true"
        );
        return;
    }
    *BARRIER.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(Barrier {
        root: control.root.clone(),
        claim: control.operation,
        phase: control.phase,
        marker: control.marker,
    });
    let caller = Node::open_hosting(control.root, "caller", false).unwrap();
    let _ = timeout(BY, caller.pair(&control.ticket)).await;
    eprintln!("[PAIR-WORKER] returned");
    panic!("caller returned before its SIGKILL checkpoint");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real signed roster chunks and authenticated Node; run exact in isolation"]
async fn large_vouch_grant_is_upload_only_and_its_receipt_is_idempotent() {
    let case = Case::new("vouch");
    let source = case.node("source");
    let receiver = case.node("receiver");
    let server = Server::start(source.root(), "source").await;
    let ticket = source.invite().unwrap();
    timeout(BY, receiver.pair(&ticket)).await.unwrap().unwrap();
    server.stop().await;
    let source_id = id(source.device_str()).unwrap();
    let receiver_id = id(receiver.device_str()).unwrap();
    let key = khor_net::root::load(&source.root().join(".khor/root.key"))
        .unwrap()
        .unwrap();
    let mut members = vec![
        nt::Member::device(source_id),
        nt::Member::device(receiver_id),
    ];
    for n in 0u32..4096 {
        members.push(nt::Member::device(
            *blake3::hash(&n.to_be_bytes()).as_bytes(),
        ));
    }
    let mut roster =
        nt::verify_roster(&nt::genesis(&key, crate::now_ms(), &members), &key.public()).unwrap();
    while roster.seq < 10 {
        roster = nt::verify_roster(&roster.refresh(&key, crate::now_ms()), &key.public()).unwrap();
    }
    let raw = roster.bytes();
    assert!(raw.len() > CHUNK_BYTES);
    assert!(raw.len() as u64 <= khor_sync::transfer::MAX_BLOB_BYTES);
    let certificate = roster.signed_cert(&source_id).unwrap().to_vec();
    assert!(certificate.len() <= 1024);
    {
        let mut loaded =
            store::load_pile::<DeviceDoc>(&devices_dir(receiver.root()), 0x3959).unwrap();
        loaded.doc.keep_only(receiver.device_str()).unwrap();
        let keys = loaded
            .doc
            .rosters()
            .into_iter()
            .map(|(k, _)| k)
            .collect::<Vec<_>>();
        loaded.doc.drop_rosters(&keys).unwrap();
        loaded.store.flush(&loaded.doc).unwrap();
    }
    assert!(receiver.admit(source.device_str()).is_err());
    let server = Server::start(receiver.root(), "receiver").await;
    let addrs = receiver.endpoint_file().unwrap().unwrap().addrs;
    let manifest = state::vouch_manifest(source_id, receiver_id, raw).unwrap();
    let operation = state::blob_id(&manifest).unwrap();
    let chunk = |offset: usize| DocChunk {
        manifest: manifest.clone(),
        offset: offset as u64,
        bytes: raw[offset..raw.len().min(offset + CHUNK_BYTES)]
            .to_vec()
            .into(),
    };
    let response = timeout(
        BY,
        util::raw_request(
            source.secret_key().clone(),
            receiver.device_str(),
            &addrs,
            &Request::Vouch {
                action: VouchAction::Start {
                    name: "source".into(),
                    addrs: Vec::new(),
                    certificate: certificate.into(),
                    roster: chunk(0),
                },
            },
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        matches!(response,Response::Vouched {reply:VouchReply {phase:VouchPhase::Stored {next_offset},..}} if next_offset==CHUNK_BYTES as u64)
    );
    assert!(
        receiver.admit(source.device_str()).is_err(),
        "Verified prefix is not admission"
    );
    let forbidden = util::raw_request(
        source.secret_key().clone(),
        receiver.device_str(),
        &addrs,
        &Request::Pair {
            action: PairAction::ReadRoster {
                claim: operation,
                nonce: [0; 32],
                manifest: manifest.clone(),
                offset: 0,
            },
        },
    )
    .await
    .unwrap();
    assert!(matches!(forbidden, Response::Refused { .. }));
    let impostor = util::raw_request(
        iroh::SecretKey::from_bytes(&[77; 32]),
        receiver.device_str(),
        &addrs,
        &Request::Vouch {
            action: VouchAction::Write {
                operation,
                chunk: chunk(CHUNK_BYTES),
            },
        },
    )
    .await
    .unwrap();
    assert!(matches!(impostor, Response::Refused { .. }));
    let context = receiver.doc_context().unwrap();
    assert_eq!(
        State::open(&context)
            .unwrap()
            .roster_offset(&manifest)
            .unwrap(),
        CHUNK_BYTES as u64
    );
    for offset in (CHUNK_BYTES..raw.len()).step_by(CHUNK_BYTES) {
        let response = util::raw_request(
            source.secret_key().clone(),
            receiver.device_str(),
            &addrs,
            &Request::Vouch {
                action: VouchAction::Write {
                    operation,
                    chunk: chunk(offset),
                },
            },
        )
        .await
        .unwrap();
        if offset + CHUNK_BYTES >= raw.len() {
            assert!(matches!(
                response,
                Response::Vouched {
                    reply: VouchReply {
                        phase: VouchPhase::Vouched,
                        ..
                    }
                }
            ));
        } else {
            assert!(matches!(
                response,
                Response::Vouched {
                    reply: VouchReply {
                        phase: VouchPhase::Stored { .. },
                        ..
                    }
                }
            ));
        }
    }
    assert_eq!(receiver.admit(source.device_str()), Ok(()));
    let before = canonical_version(&context).unwrap();
    for action in [
        VouchAction::Status { operation },
        VouchAction::Finish { operation },
        VouchAction::Finish { operation },
        VouchAction::Write {
            operation,
            chunk: chunk(CHUNK_BYTES),
        },
    ] {
        let response = util::raw_request(
            source.secret_key().clone(),
            receiver.device_str(),
            &addrs,
            &Request::Vouch { action },
        )
        .await
        .unwrap();
        assert!(matches!(
            response,
            Response::Vouched {
                reply: VouchReply {
                    phase: VouchPhase::Vouched,
                    ..
                }
            }
        ));
    }
    assert_eq!(canonical_version(&context).unwrap(), before);
    assert!(
        State::open(&context)
            .unwrap()
            .roster_progress(&manifest)
            .unwrap()
            .is_none()
    );
    {
        let mut loaded =
            store::load_pile::<DeviceDoc>(&devices_dir(receiver.root()), 0x3960).unwrap();
        loaded.doc.set_gone(source.device_str(), true).unwrap();
        loaded.store.flush(&loaded.doc).unwrap();
    }
    let response = util::raw_request(
        source.secret_key().clone(),
        receiver.device_str(),
        &addrs,
        &Request::Vouch {
            action: VouchAction::Status { operation },
        },
    )
    .await
    .unwrap();
    assert!(matches!(response, Response::Refused { .. }));
    assert!(
        store::load_pile::<DeviceDoc>(&devices_dir(receiver.root()), 0x3961)
            .unwrap()
            .doc
            .gone_ids()
            .contains(&source.device_str().to_owned())
    );
    server.stop().await;
    eprintln!(
        "[Q039-VOUCH] canonical_chunks=true wrong_identity_rejected=true upload_only=true receipt_stable=true late_write_no_stage=true gone_preserved=true"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real authenticated Pair claims and invalid signed body; run exact in isolation"]
async fn receiving_pair_claim_rejects_changed_owner_and_releases_invalid_roster_without_burning() {
    let case = Case::new("claim");
    let issuer = case.node("issuer");
    let caller = case.node("caller");
    let other = case.node("other");
    let server = Server::start(issuer.root(), "issuer").await;
    let ticket = issuer.invite().unwrap();
    let decoded = khor_net::endpoint::Ticket::decode(&ticket).unwrap();
    let local = id(issuer.device_str()).unwrap();
    let remote = id(caller.device_str()).unwrap();
    let hash = state::token_hash(&decoded.token).unwrap();
    let nonce = [21; 32];
    let operation = state::claim_id(local, remote, hash, nonce);
    let raw = vec![0x7b; CHUNK_BYTES + 1];
    let manifest =
        state::roster_manifest(operation, state::CALLER_ROSTER, remote, local, &raw).unwrap();
    let header = PairHeader {
        name: "caller".into(),
        addrs: vec![],
        devices_have: canonical_version(&caller.doc_context().unwrap()).unwrap(),
        roster: Some(manifest.clone()),
    };
    let chunk = DocChunk {
        manifest: manifest.clone(),
        offset: 0,
        bytes: raw[..CHUNK_BYTES].to_vec().into(),
    };
    let addrs = issuer.endpoint_file().unwrap().unwrap().addrs;
    let response = util::raw_request(
        caller.secret_key().clone(),
        issuer.device_str(),
        &addrs,
        &Request::Pair {
            action: PairAction::Start {
                token: decoded.token.clone(),
                nonce,
                header: header.clone(),
                roster: Some(chunk.clone()),
            },
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(response,Response::Paired {reply:PairReply::Progress {phase:PairPhase::Receiving,roster_received,..}} if roster_received==CHUNK_BYTES as u64)
    );
    let context = issuer.doc_context().unwrap();
    let ticket_file = issuer.root().join(".khor/invites").join(nt::id_hex(&hash));
    assert!(ticket_file.exists());
    assert!(
        issuer
            .devices()
            .unwrap()
            .iter()
            .all(|row| row.id != caller.device_str())
    );
    let wrong_nonce = util::raw_request(
        caller.secret_key().clone(),
        issuer.device_str(),
        &addrs,
        &Request::Pair {
            action: PairAction::Resume {
                claim: operation,
                nonce: [22; 32],
            },
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(wrong_nonce,Response::Refused {why} if khor_catalog::read(&why).is_some_and(|(key,_)|key=="msg.bad-invite"))
    );
    let mut changed = header.clone();
    changed.name = "changed".into();
    let changed = util::raw_request(
        caller.secret_key().clone(),
        issuer.device_str(),
        &addrs,
        &Request::Pair {
            action: PairAction::Start {
                token: decoded.token.clone(),
                nonce,
                header: changed,
                roster: Some(chunk),
            },
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(changed,Response::Refused {why} if khor_catalog::read(&why).is_some_and(|(key,_)|key=="msg.bad-invite"))
    );
    let error = timeout(BY, other.pair(&ticket)).await.unwrap().unwrap_err();
    assert_eq!(
        khor_catalog::read(&error).map(|(key, _)| key),
        Some("msg.bad-invite")
    );
    // Knowing the journal's tokenHash is not knowing the original credential.
    let hashed_token = nt::id_hex(&hash);
    let impostor = id(other.device_str()).unwrap();
    let fake_operation = state::claim_id(
        local,
        impostor,
        state::token_hash(&hashed_token).unwrap(),
        [23; 32],
    );
    let fake_manifest =
        state::roster_manifest(fake_operation, state::CALLER_ROSTER, impostor, local, &raw)
            .unwrap();
    let fake_header = PairHeader {
        name: "other".into(),
        addrs: vec![],
        devices_have: canonical_version(&other.doc_context().unwrap()).unwrap(),
        roster: Some(fake_manifest.clone()),
    };
    let response = util::raw_request(
        other.secret_key().clone(),
        issuer.device_str(),
        &addrs,
        &Request::Pair {
            action: PairAction::Start {
                token: hashed_token,
                nonce: [23; 32],
                header: fake_header,
                roster: Some(DocChunk {
                    manifest: fake_manifest,
                    offset: 0,
                    bytes: raw[..CHUNK_BYTES].to_vec().into(),
                }),
            },
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(response,Response::Refused {why} if khor_catalog::read(&why).is_some_and(|(key,_)|key=="msg.bad-invite"))
    );
    assert_eq!(
        State::open(&context)
            .unwrap()
            .roster_offset(&manifest)
            .unwrap(),
        CHUNK_BYTES as u64
    );
    let response = util::raw_request(
        caller.secret_key().clone(),
        issuer.device_str(),
        &addrs,
        &Request::Pair {
            action: PairAction::WriteRoster {
                claim: operation,
                nonce,
                chunk: DocChunk {
                    manifest: manifest.clone(),
                    offset: CHUNK_BYTES as u64,
                    bytes: raw[CHUNK_BYTES..].to_vec().into(),
                },
            },
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(response,Response::Paired {reply:PairReply::Progress {phase:PairPhase::Receiving,roster_received,..}} if roster_received==raw.len() as u64)
    );
    let response = util::raw_request(
        caller.secret_key().clone(),
        issuer.device_str(),
        &addrs,
        &Request::Pair {
            action: PairAction::Commit {
                claim: operation,
                nonce,
            },
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        response,
        Response::Paired {
            reply: PairReply::Uncommitted { .. }
        }
    ));
    assert!(ticket_file.exists());
    {
        let state = State::open(&context).unwrap();
        assert!(state.pair_claim(operation).is_none());
        assert!(state.roster_progress(&manifest).unwrap().is_none());
    }
    assert_eq!(
        timeout(BY, caller.pair(&ticket)).await.unwrap().unwrap(),
        "issuer"
    );
    assert!(!ticket_file.exists());
    server.stop().await;
    eprintln!(
        "[Q039-PAIR-CLAIM] immutable_header=true wrong_owner_rejected=true hash_not_credential=true bad_signature_before_burn=true valid_ticket_reusable=true"
    );
}
