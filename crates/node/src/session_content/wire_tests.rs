use super::{ContentEvent, ContentLocator, ReadBinding, remote::*};
use crate::adaptor::{
    claude::Utterance,
    transcript_page::{Page, Position, RecordChunk},
};
use crate::{Node, SessionId};
use khor_core::{DeviceId, Millis, ObservationNamespace, ObservationRef};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::time::timeout;

#[path = "../../tests/util/mod.rs"]
mod util;

#[derive(Default)]
struct Gate {
    count: AtomicUsize,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
    released: Mutex<bool>,
    release: Condvar,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Stage {
    Io,
    Response,
}
fn gates() -> &'static Mutex<HashMap<(PathBuf, Stage), Arc<Gate>>> {
    static GATES: OnceLock<Mutex<HashMap<(PathBuf, Stage), Arc<Gate>>>> = OnceLock::new();
    GATES.get_or_init(|| Mutex::new(HashMap::new()))
}
pub(super) fn pause_io(root: &Path) {
    let gate = gates()
        .lock()
        .unwrap()
        .get(&(root.into(), Stage::Io))
        .cloned();
    if let Some(gate) = gate {
        gate.count.fetch_add(1, Ordering::Release);
        gate.entered.notify_one();
        let mut released = gate.released.lock().unwrap();
        while !*released {
            released = gate.release.wait(released).unwrap();
        }
    }
}
pub(crate) async fn pause_response(root: &Path) {
    let gate = gates()
        .lock()
        .unwrap()
        .get(&(root.into(), Stage::Response))
        .cloned();
    if let Some(gate) = gate {
        gate.count.fetch_add(1, Ordering::Release);
        gate.entered.notify_one();
        loop {
            if *gate.released.lock().unwrap() {
                gate.resume.notify_one();
                break;
            }
            gate.resume.notified().await;
        }
    }
}
struct HoldWork(PathBuf, Arc<Gate>, Stage);
impl HoldWork {
    fn install(root: &Path, stage: Stage) -> Self {
        let gate = Arc::new(Gate::default());
        assert!(
            gates()
                .lock()
                .unwrap()
                .insert((root.into(), stage), gate.clone())
                .is_none()
        );
        Self(root.into(), gate, stage)
    }
    async fn entered(&self, count: usize) {
        timeout(Duration::from_secs(15), async {
            while self.1.count.load(Ordering::Acquire) < count {
                self.1.entered.notified().await;
            }
        })
        .await
        .expect("all owned work must reach the explicit barrier");
    }
    fn release(&self) {
        *self
            .1
            .released
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = true;
        self.1.release.notify_all();
        self.1.resume.notify_one();
    }
}
impl Drop for HoldWork {
    fn drop(&mut self) {
        self.release();
        let mut gates = gates().lock().unwrap_or_else(|error| error.into_inner());
        if gates
            .get(&(self.0.clone(), self.2))
            .is_some_and(|gate| Arc::ptr_eq(gate, &self.1))
        {
            gates.remove(&(self.0.clone(), self.2));
        }
    }
}
struct Server {
    task: Option<tokio::task::JoinHandle<Result<(), String>>>,
    responses: Arc<tokio::sync::Semaphore>,
}
impl Server {
    async fn start(root: &Path, name: &str) -> Self {
        let node = Node::open_as(root.to_path_buf(), name).unwrap();
        let responses = node.content_responses.clone();
        let server = Self {
            task: Some(tokio::spawn(async move { node.serve().await })),
            responses,
        };
        timeout(Duration::from_secs(90), async {
            while !root.join(".khor/endpoint.json").exists() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("private server must publish its endpoint");
        server
    }
    async fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
fn message(text: &str) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&serde_json::json!({
        "type":"event_msg", "payload":{"type":"agent_message", "message":text}
    }))
    .unwrap();
    bytes.push(b'\n');
    bytes
}
fn source(root: &Path, id: &str) -> PathBuf {
    let path = root.join(format!(
        ".codex/sessions/2026/10/29/rollout-2026-10-29T12-00-00-{id}.jsonl"
    ));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    path
}
fn text(page: &Page<ContentEvent>) -> Vec<&str> {
    page.events
        .iter()
        .filter_map(|event| match &event.payload {
            ContentEvent::Native(Utterance::Agent(text)) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}
async fn page(
    node: &Node,
    binding: &ReadBinding,
    cursor: Option<crate::adaptor::transcript_page::Cursor>,
    newer: bool,
) -> Page<ContentEvent> {
    let key = node.content_begin(binding).await.unwrap();
    let request = ClientRead::claim(node.root(), binding.clone(), &key).unwrap();
    node.content_page(&request, cursor, newer).await.unwrap()
}

async fn settled_responses(servers: &[&Server]) {
    timeout(Duration::from_secs(5), async {
        while servers
            .iter()
            .any(|server| server.responses.available_permits() != crate::link::CONTENT_RESPONSES)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("response owners must retire after their final writes");
}

async fn cancel_held_io(
    a: &Arc<Node>,
    binding: &ReadBinding,
    root_b: &Path,
    serve_a: &Server,
    serve_b: &Server,
) {
    let held = HoldWork::install(root_b, Stage::Io);
    let mut reads = Vec::new();
    let mut tasks = Vec::new();
    for _ in 0..32 {
        let key = a.content_begin(binding).await.unwrap();
        let request = ClientRead::claim(a.root(), binding.clone(), &key).unwrap();
        let (node, worker) = (a.clone(), request.clone());
        tasks.push(tokio::spawn(async move {
            node.content_page(&worker, None, false).await
        }));
        reads.push((key, request));
    }
    held.entered(32).await;
    assert_eq!(serve_a.responses.available_permits(), 0);
    assert_eq!(serve_b.responses.available_permits(), 0);
    timeout(
        Duration::from_secs(10),
        a.content_cancel(binding, &reads[0].0),
    )
    .await
    .expect("cancel must answer before the IO barrier is released")
    .unwrap();
    assert_eq!(held.1.count.load(Ordering::Acquire), 32);
    assert!(!*held.1.released.lock().unwrap());
    assert!(reads[0].1.check().is_err());
    let capacity = a.content_begin(binding).await.unwrap_err();
    assert!(
        capacity.contains("too many pending content read requests"),
        "{capacity}"
    );
    held.release();
    for (n, task) in tasks.into_iter().enumerate() {
        let result = timeout(Duration::from_secs(30), task)
            .await
            .unwrap()
            .unwrap();
        if n == 0 {
            assert!(result.err().unwrap().contains("cancelled"));
        } else {
            assert_eq!(result.unwrap().events.len(), 128);
        }
    }
    drop(reads);
    drop(held);
    let key = a.content_begin(binding).await.unwrap();
    a.content_cancel(binding, &key).await.unwrap();

    settled_responses(&[serve_a, serve_b]).await;
}

async fn response_limit(a: &Node, binding: &ReadBinding) {
    let key = a.content_begin(binding).await.unwrap();
    let request = ClientRead::claim(a.root(), binding.clone(), &key).unwrap();
    let result = timeout(
        Duration::from_secs(10),
        a.content_page(&request, None, false),
    )
    .await
    .expect("full response lane must refuse without waiting for IO");
    let error = result.err().unwrap();
    assert!(
        error.contains("too many buffered content responses"),
        "{error}"
    );
    a.content_cancel(binding, &key).await.unwrap();
}

async fn cancel_held_responses(
    a: &Arc<Node>,
    binding: &ReadBinding,
    root_b: &Path,
    serve_a: &Server,
    serve_b: &Server,
) {
    let held = HoldWork::install(root_b, Stage::Response);
    let mut reads = Vec::new();
    let mut tasks = Vec::new();
    for _ in 0..32 {
        let key = a.content_begin(binding).await.unwrap();
        let request = ClientRead::claim(a.root(), binding.clone(), &key).unwrap();
        let (node, worker) = (a.clone(), request.clone());
        tasks.push(tokio::spawn(async move {
            node.content_page(&worker, None, false).await
        }));
        reads.push((key, request));
    }
    held.entered(32).await;
    assert_eq!(
        serve_a.responses.available_permits(),
        0,
        "IPC relay retains its response owners"
    );
    assert_eq!(
        serve_b.responses.available_permits(),
        0,
        "home retains its encoded reply owners"
    );
    response_limit(a, binding).await;
    timeout(
        Duration::from_secs(10),
        a.content_cancel(binding, &reads[0].0),
    )
    .await
    .expect("cancel must answer while completed replies are held")
    .unwrap();
    assert!(!*held.1.released.lock().unwrap());
    assert_eq!(serve_a.responses.available_permits(), 0);
    assert_eq!(serve_b.responses.available_permits(), 0);
    response_limit(a, binding).await;
    assert_eq!(
        held.1.count.load(Ordering::Acquire),
        32,
        "no unbounded extra IO/response was queued"
    );
    held.release();
    for (n, task) in tasks.into_iter().enumerate() {
        let result = timeout(Duration::from_secs(30), task)
            .await
            .unwrap()
            .unwrap();
        if n == 0 {
            assert!(result.err().unwrap().contains("cancelled"));
        } else {
            assert_eq!(result.unwrap().events.len(), 128);
        }
    }
    drop(reads);
    drop(held);
    settled_responses(&[serve_a, serve_b]).await;
    assert_eq!(page(a, binding, None, false).await.events.len(), 128);
    settled_responses(&[serve_a, serve_b]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn canonical_content_routes_without_metadata_owner_and_cancels_through_busy_real_wires() {
    let root_a = crate::testdir::TempTree::fresh("content-wire-reader");
    let root_b = crate::testdir::TempTree::fresh("content-wire-home");
    util::hermetic(&root_a);
    util::hermetic(&root_b);
    let mut serve_a = Server::start(&root_a, "read-here").await;
    let mut serve_b = Server::start(&root_b, "content-here").await;
    let a = Arc::new(Node::open_as(root_a.path(), "read-here").unwrap());
    let b = Node::open_as(root_b.path(), "content-here").unwrap();
    timeout(Duration::from_secs(60), a.pair(&b.invite().unwrap()))
        .await
        .expect("private nodes must pair")
        .unwrap();
    let id = "01a17c1a-a542-7822-b01d-82a832a158e6";
    let path = source(&root_b, id);
    let mut history = Vec::new();
    for n in 0..5000 {
        history.extend(message(&format!("home-{n}")));
    }
    std::fs::write(&path, &history).unwrap();
    std::fs::write(source(&root_a, id), message("wrong-home-bait")).unwrap();
    let offline_owner = DeviceId([77; 32]);
    assert!(
        a.devices()
            .unwrap()
            .iter()
            .all(|device| device.id != offline_owner.hex())
    );
    let observation = ObservationRef {
        owner: offline_owner,
        home: b.device(),
        namespace: ObservationNamespace::Native,
        agent: "codex".into(),
        native_id: id.into(),
        imported_at: Millis(1),
        updated_at: None,
    };
    let view = SessionId("observe/no-alias-on-any-online-node".into());
    let locator = ContentLocator::observed(view.clone(), &observation).unwrap();
    let reply = a.content_read(Action::Resolve { locator }).await.unwrap();
    let Reply::Resolved(Resolution::Available { binding, .. }) = reply else {
        panic!("actual home source not resolved");
    };
    assert_eq!(binding.home(), b.device());
    assert_eq!(binding.view(), &view);
    let alias = format!("{}.json", blake3::hash(view.0.as_bytes()).to_hex());
    assert!(!root_a.join(".khor/observations").join(&alias).exists());
    assert!(!root_b.join(".khor/observations").join(&alias).exists());
    let tail = page(&a, &binding, None, false).await;
    assert_eq!(tail.events.len(), 128);
    assert_eq!(text(&tail).last(), Some(&"home-4999"));
    let older = page(&a, &binding, tail.older.clone(), false).await;
    assert_eq!(text(&older).last(), Some(&"home-4871"));
    assert!(tail.events.iter().all(|event| !matches!(&event.payload, ContentEvent::Native(Utterance::Agent(t)) if t == "wrong-home-bait")));

    // The identical transport also reads every managed ACP through its journal.
    use super::journal::{Identity, Journal, Opening};
    let notification: agent_client_protocol::schema::v1::SessionNotification =
        serde_json::from_value(serde_json::json!({"sessionId":"opaque/acp/id",
            "update":{"sessionUpdate":"agent_message_chunk",
                "content":{"type":"text","text":"wire-captured-acp"}}}))
        .unwrap();
    let captured = serde_json::to_value(&notification).unwrap();
    let identity = Identity {
        home: b.device(),
        agent: "any-registry-agent".into(),
        native_id: "opaque/acp/id".into(),
    };
    let mut writer = Journal::open(&root_b, identity.clone(), Opening::Load).unwrap();
    writer.note(&notification).unwrap();
    writer.commit_base().unwrap();
    writer
        .accept("turn", "accepted without echo", &[], &[], None)
        .unwrap();
    drop(writer);
    let writer = Journal::open(&root_b, identity, Opening::Resume).unwrap();
    drop(writer);
    let acp_observation = ObservationRef {
        namespace: ObservationNamespace::Acp,
        agent: "any-registry-agent".into(),
        native_id: "opaque/acp/id".into(),
        ..observation.clone()
    };
    let acp_locator = ContentLocator::observed(
        SessionId("observe/acp-with-offline-owner".into()),
        &acp_observation,
    )
    .unwrap();
    let acp_reply = a
        .content_read(Action::Resolve {
            locator: acp_locator,
        })
        .await
        .unwrap();
    let Reply::Resolved(Resolution::Available {
        binding: acp_binding,
        capture: Some(capture),
        ..
    }) = acp_reply
    else {
        panic!("canonical ACP capture not available over the same wire");
    };
    assert_eq!(capture.opening, "load");
    assert_eq!(capture.gaps, 1);
    let acp_page = page(&a, &acp_binding, None, false).await;
    assert!(
        acp_page
            .events
            .iter()
            .any(|event| matches!(&event.payload,ContentEvent::Update { update: value, .. } if value==&captured))
    );
    assert!(acp_page.events.iter().any(|event|matches!(&event.payload,ContentEvent::CaptureGap {opening,interrupted:true} if opening=="resume")));
    assert!(acp_page.events.iter().any(|event|matches!(&event.payload,ContentEvent::Update { update: value, .. } if value["update"]["content"]["text"]=="accepted without echo")));

    // A paired request cannot borrow another binding's acknowledged ticket.
    let key = a.content_begin(&binding).await.unwrap();
    let other = ReadBinding::new(
        SessionId("observe/another-view".into()),
        binding.reference().clone(),
    )
    .unwrap();
    let error = a
        .content_read(Action::Window {
            binding: other,
            request: key.clone(),
            cursor: None,
            newer: false,
        })
        .await
        .err()
        .unwrap();
    assert!(error.contains("another source"), "{error}");
    let request = ClientRead::claim(&root_a, binding.clone(), &key).unwrap();
    assert_eq!(
        a.content_page(&request, None, false)
            .await
            .unwrap()
            .events
            .len(),
        128
    );
    drop(request);

    // The payload has no owner field: an unpaired identity cannot cancel A's ticket.
    let key = a.content_begin(&binding).await.unwrap();
    let endpoint: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root_b.join(".khor/endpoint.json")).unwrap())
            .unwrap();
    let addrs: Vec<String> = serde_json::from_value(endpoint["addrs"].clone()).unwrap();
    let answer = util::raw_request(
        iroh::SecretKey::generate(),
        endpoint["id"].as_str().unwrap(),
        &addrs,
        &crate::proto::Request::ContentRead {
            action: Action::Cancel {
                home: b.device(),
                request: key.clone(),
            },
        },
    )
    .await
    .unwrap();
    assert!(matches!(answer, crate::proto::Response::Refused { .. }));
    let request = ClientRead::claim(&root_a, binding.clone(), &key).unwrap();
    assert_eq!(
        a.content_page(&request, None, false)
            .await
            .unwrap()
            .events
            .len(),
        128
    );
    drop(request);

    // Early cancellation is acknowledged over both resident and peer transports.
    let key = a.content_begin(&binding).await.unwrap();
    a.content_cancel(&binding, &key).await.unwrap();
    assert!(ClientRead::claim(&root_a, binding.clone(), &key).is_err());
    let error = a
        .content_read(Action::Window {
            binding: binding.clone(),
            request: key,
            cursor: None,
            newer: false,
        })
        .await
        .err()
        .unwrap();
    assert!(error.contains("cancelled or expired"), "{error}");

    cancel_held_io(&a, &binding, &root_b, &serve_a, &serve_b).await;
    cancel_held_responses(&a, &binding, &root_b, &serve_a, &serve_b).await;

    // Complete oversized records stay explicit, with byte-exact bounded chunks.
    use std::io::Write;
    let large = message(&"中🙂".repeat(22000));
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(&large)
        .unwrap();
    let growth = page(&a, &binding, Some(tail.newer.clone()), true).await;
    let reference = growth
        .references
        .iter()
        .find(|reference| !reference.missing_start && !reference.missing_end)
        .unwrap();
    let raw_cursor = growth.cursor_before(Position {
        start: reference.range.start,
        end: reference.range.end,
        item: 0,
    });
    let key = a.content_begin(&binding).await.unwrap();
    let request = ClientRead::claim(&root_a, binding.clone(), &key).unwrap();
    let first = a
        .content_record(&request, raw_cursor.clone(), reference.range.start)
        .await
        .unwrap();
    assert_eq!(first.bytes.as_slice(), &large[..first.bytes.len()]);
    assert_eq!(first.bytes.len(), 64 * 1024);
    drop(request);
    let mut next = first.next;
    while let Some(from) = next {
        let key = a.content_begin(&binding).await.unwrap();
        let request = ClientRead::claim(&root_a, binding.clone(), &key).unwrap();
        let chunk = a
            .content_record(&request, raw_cursor.clone(), from)
            .await
            .unwrap();
        let offset = (from - reference.range.start) as usize;
        assert_eq!(
            chunk.bytes.as_slice(),
            &large[offset..offset + chunk.bytes.len()]
        );
        assert!(chunk.bytes.len() <= 64 * 1024);
        next = chunk.next;
    }
    let previous = path.with_extension("retired");
    std::fs::rename(&path, &previous).unwrap();
    std::fs::write(&path, message("replacement-home")).unwrap();
    let reset = page(&a, &binding, Some(growth.newer), true).await;
    assert!(reset.reset);
    assert_eq!(text(&reset), ["replacement-home"]);
    let key = a.content_begin(&binding).await.unwrap();
    let request = ClientRead::claim(&root_a, binding.clone(), &key).unwrap();
    let error = a
        .content_record(&request, raw_cursor, reference.range.start)
        .await
        .err()
        .unwrap();
    assert!(error.contains("history record changed"), "{error}");
    a.content_cancel(&binding, &key).await.unwrap();
    drop(request);
    // Pause physically closes the private home wire; it must not fall back to A's bait.
    let key = a.content_begin(&binding).await.unwrap();
    let request = ClientRead::claim(&root_a, binding.clone(), &key).unwrap();
    crate::pause::set_on(&root_b, true).unwrap();
    let disconnected = timeout(
        Duration::from_secs(45),
        a.content_page(&request, None, false),
    )
    .await
    .expect("disconnected read must be bounded");
    assert!(
        disconnected.is_err(),
        "a disconnected home cannot become a local history read"
    );
    let _ = a.content_cancel(&binding, &key).await;
    assert!(request.check().is_err());
    crate::pause::set_on(&root_b, false).unwrap();
    a.content_cancel(&binding, &key).await.unwrap();
    drop(request);
    let restored = page(&a, &binding, None, false).await;
    assert_eq!(text(&restored), ["replacement-home"]);
    serve_a.stop().await;
    serve_b.stop().await;
}

#[test]
fn only_idempotent_cancel_retries_and_raw_envelopes_remain_bounded() {
    use crate::proto::{Request, Response};
    let home = DeviceId([2; 32]);
    let binding = ReadBinding::new(
        SessionId("view".into()),
        super::ContentReference {
            home,
            namespace: ObservationNamespace::Native,
            agent: "codex".into(),
            native_id: "exact".into(),
        },
    )
    .unwrap();
    let cancel = Request::ContentRead {
        action: Action::Cancel {
            home,
            request: "0".repeat(32),
        },
    };
    assert!(cancel.is_reading());
    assert!(!cancel.rides_early());
    assert!(
        !Request::ContentRead {
            action: Action::Begin {
                binding: binding.clone(),
                request: "0".repeat(32)
            }
        }
        .is_reading()
    );
    assert!(
        !Request::ContentRead {
            action: Action::Window {
                binding: binding.clone(),
                request: "0".repeat(32),
                cursor: None,
                newer: false
            }
        }
        .is_reading()
    );
    let reply = Response::ContentRead {
        reply: Reply::Record {
            source: binding.source_id(),
            chunk: RecordChunk {
                offset: 0,
                bytes: vec![255; 64 * 1024],
                next: None,
            },
        },
    };
    let bytes = crate::proto::encode_response(&reply).unwrap();
    assert!(
        bytes.len() < 65 * 1024,
        "raw bytes must use the existing binary codec"
    );
    crate::proto::answer(&bytes).unwrap();
    let oversized = Response::ContentRead {
        reply: Reply::Record {
            source: binding.source_id(),
            chunk: RecordChunk {
                offset: 0,
                bytes: vec![255; 64 * 1024 + 1],
                next: None,
            },
        },
    };
    assert!(crate::proto::encode_response(&oversized).is_err());
    let invalid = Request::ContentRead {
        action: Action::Cancel {
            home,
            request: "x".repeat(33),
        },
    };
    assert!(crate::proto::encode_request(&invalid).is_err());
    assert!(crate::proto::question(&crate::proto::encode(&invalid).unwrap()).is_err());
}
