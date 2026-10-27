//! Private, bounded observations of the real Node flow. No product fault flag,
//! payload log, replacement peer, or synthetic transfer-step result exists.
use super::*;
use crate::doc_transfer_state::registry::ROUND_DOCS;
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tokio::{sync::Notify, task::JoinHandle, time::timeout};
#[path = "../tests/util/mod.rs"]
mod util;

const MAX_ROOTS: usize = 4;
const MAX_EVENTS: usize = 2048;
const MAX_MANIFESTS: usize = 64;
const BY: Duration = Duration::from_secs(90);
static CLOCK: AtomicU64 = AtomicU64::new(1);
static ROOTS: OnceLock<Mutex<BTreeMap<PathBuf, Weak<Probe>>>> = OnceLock::new();
fn roots() -> &'static Mutex<BTreeMap<PathBuf, Weak<Probe>>> {
    ROOTS.get_or_init(Default::default)
}
fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum StepKind {
    Exchange,
    Write,
    Read,
    Idle,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Point {
    Before(String, StepKind),
    After(String, StepKind),
    Receipts(String),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rpc {
    Versions,
    Exchange,
    Write,
    Read,
    Status,
    Confirm,
    Acknowledge,
    Other,
}
#[derive(Clone, Debug)]
struct Body {
    id: String,
    offset: u64,
    bytes: usize,
}
#[derive(Clone, Debug)]
enum Event {
    Wire {
        frame: u64,
        reply: bool,
        rpc: Rpc,
        doc: Option<String>,
        body: Option<Body>,
    },
    Point {
        point: Point,
        failure: Option<Failure>,
    },
    Quantum(String),
}
#[derive(Clone, Debug)]
struct Entry {
    order: u64,
    event: Event,
}
struct Pause {
    order: u64,
    point: Point,
    failure: Option<Failure>,
    released: AtomicBool,
    wake: Notify,
}
impl Pause {
    fn release(&self) {
        self.released.store(true, Ordering::Release);
        self.wake.notify_one();
    }
    async fn wait(&self) {
        while !self.released.load(Ordering::Acquire) {
            let notified = self.wake.notified();
            if self.released.load(Ordering::Acquire) {
                break;
            }
            notified.await;
        }
    }
}
struct Ticket(Arc<Pause>);
impl Drop for Ticket {
    fn drop(&mut self) {
        self.0.release();
    }
}
impl Ticket {
    fn release(self) {
        self.0.release();
    }
}
#[derive(Default)]
struct Trace {
    entries: Vec<Entry>,
    manifests: BTreeMap<String, DocManifest>,
    rules: BTreeMap<Point, bool>, // true=repeat, false=once
    waiting: VecDeque<Arc<Pause>>,
    active: Vec<Weak<Pause>>,
    issue: Option<String>,
}
struct Probe {
    root: PathBuf,
    peer: String,
    trace: Mutex<Trace>,
    wake: Notify,
}
impl Probe {
    fn fail(&self, why: impl Into<String>) {
        lock(&self.trace).issue.get_or_insert(why.into());
        self.wake.notify_waiters();
    }
    fn healthy(&self) {
        let issue = lock(&self.trace).issue.clone();
        assert!(issue.is_none(), "flow observation setup failed: {issue:?}");
    }
    fn note(&self, event: Event) -> u64 {
        let mut trace = lock(&self.trace);
        let order = CLOCK.fetch_add(1, Ordering::SeqCst);
        if trace.entries.len() == MAX_EVENTS {
            trace
                .issue
                .get_or_insert("flow event bound exceeded".into());
        } else {
            trace.entries.push(Entry { order, event });
        }
        drop(trace);
        self.wake.notify_one();
        order
    }
    fn intern(&self, manifest: &DocManifest) -> Option<String> {
        let id = match manifest.id() {
            Ok(id) => id,
            Err(_) => {
                self.fail("invalid observed manifest");
                return None;
            }
        };
        let mut trace = lock(&self.trace);
        if !trace.manifests.contains_key(&id) {
            if trace.manifests.len() == MAX_MANIFESTS {
                trace
                    .issue
                    .get_or_insert("flow manifest bound exceeded".into());
                return None;
            }
            trace.manifests.insert(id.clone(), manifest.clone());
        }
        Some(id)
    }
    fn body(&self, chunk: &DocChunk) -> Option<Body> {
        Some(Body {
            id: self.intern(&chunk.manifest)?,
            offset: chunk.offset,
            bytes: chunk.bytes.len(),
        })
    }
    fn rule(&self, point: Point, repeat: bool) {
        lock(&self.trace).rules.insert(point, repeat);
    }
    async fn checkpoint(&self, point: Point, failure: Option<Failure>) {
        let order = self.note(Event::Point {
            point: point.clone(),
            failure: failure.clone(),
        });
        let pause = {
            let mut trace = lock(&self.trace);
            match trace.rules.get(&point).copied() {
                None => None,
                Some(repeat) => {
                    if !repeat {
                        trace.rules.remove(&point);
                    }
                    trace.active.retain(|w| w.strong_count() > 0);
                    if trace.waiting.len() >= 8 || trace.active.len() >= 8 {
                        trace
                            .issue
                            .get_or_insert("flow barrier bound exceeded".into());
                        None
                    } else {
                        let pause = Arc::new(Pause {
                            order,
                            point,
                            failure,
                            released: AtomicBool::new(false),
                            wake: Notify::new(),
                        });
                        trace.active.push(Arc::downgrade(&pause));
                        trace.waiting.push_back(pause.clone());
                        Some(pause)
                    }
                }
            }
        };
        self.wake.notify_one();
        if let Some(pause) = pause {
            if timeout(BY, pause.wait()).await.is_err() {
                self.fail("flow barrier timed out");
                pause.release();
            }
        }
    }
    async fn next(&self) -> Ticket {
        timeout(BY, async {
            loop {
                self.healthy();
                let notified = self.wake.notified();
                if let Some(pause) = lock(&self.trace).waiting.pop_front() {
                    return Ticket(pause);
                }
                notified.await;
            }
        })
        .await
        .expect("an armed real-flow checkpoint must be reached")
    }
    fn entries(&self) -> Vec<Entry> {
        self.healthy();
        lock(&self.trace).entries.clone()
    }
    fn reset_measurement(&self) {
        self.healthy();
        let mut trace = lock(&self.trace);
        trace.entries.clear();
        trace.manifests.clear();
    }
    fn release_all(&self) {
        let mut trace = lock(&self.trace);
        trace.rules.clear();
        for pause in trace.active.iter().filter_map(Weak::upgrade) {
            pause.release();
        }
        for pause in trace.waiting.drain(..) {
            pause.release();
        }
    }
    fn manifest(&self, id: &str) -> DocManifest {
        self.healthy();
        lock(&self.trace).manifests[id].clone()
    }
    fn body_bytes(&self) -> usize {
        self.entries()
            .iter()
            .map(|e| match &e.event {
                Event::Wire { body: Some(b), .. } => b.bytes,
                _ => 0,
            })
            .sum()
    }
}
struct Scope(Arc<Probe>);
impl Scope {
    fn new(root: &Path, peer: &str) -> Self {
        let root = fs::canonicalize(root).unwrap();
        let mut all = lock(roots());
        all.retain(|_, w| w.strong_count() > 0);
        assert!(
            all.len() < MAX_ROOTS && !all.contains_key(&root),
            "flow root registration bound/conflict"
        );
        let probe = Arc::new(Probe {
            root: root.clone(),
            peer: peer.into(),
            trace: Mutex::new(Trace::default()),
            wake: Notify::new(),
        });
        all.insert(root, Arc::downgrade(&probe));
        Self(probe)
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        lock(roots()).remove(&self.0.root);
        let mut trace = lock(&self.0.trace);
        for pause in trace.active.iter().filter_map(Weak::upgrade) {
            pause.release();
        }
        for pause in trace.waiting.drain(..) {
            pause.release();
        }
        assert!(
            trace.issue.is_none() || std::thread::panicking(),
            "flow observation failed: {:?}",
            trace.issue
        );
    }
}
fn probe(root: &Path) -> Option<Arc<Probe>> {
    lock(roots()).get(root).and_then(Weak::upgrade)
}

/// Carried directly by this actual frame across awaits. Internal reading
/// dispatches explicitly pass None; no thread/task local infers wire origin.
pub(crate) struct FrameTag {
    probe: Option<Arc<Probe>>,
    frame: u64,
    admitted: AtomicBool,
    request: Mutex<Option<(Rpc, Option<String>)>>,
}
impl FrameTag {
    pub(crate) fn new(root: &Path, peer: &str) -> Self {
        Self {
            probe: probe(root).filter(|p| p.peer == peer),
            frame: CLOCK.fetch_add(1, Ordering::SeqCst),
            admitted: AtomicBool::new(false),
            request: Mutex::new(None),
        }
    }
}
pub(crate) fn admitted(tag: &FrameTag, request: &Request) {
    let Some(p) = &tag.probe else {
        return;
    };
    assert!(
        !tag.admitted.swap(true, Ordering::AcqRel),
        "one wire frame dispatched more than once"
    );
    let (rpc, doc, body) = match request {
        Request::Versions { .. } => (Rpc::Versions, None, None),
        Request::DocTransfer { action } => match action {
            Action::Exchange {
                document, offer, ..
            } => (
                Rpc::Exchange,
                Some(document.clone()),
                offer.as_ref().and_then(|c| p.body(c)),
            ),
            Action::Write { chunk } => (
                Rpc::Write,
                chunk.manifest.descriptor().ok().map(|d| d.document),
                p.body(chunk),
            ),
            Action::Read { manifest, .. } => (
                Rpc::Read,
                manifest.descriptor().ok().map(|d| d.document),
                None,
            ),
            Action::Status { manifest } => (
                Rpc::Status,
                manifest.descriptor().ok().map(|d| d.document),
                None,
            ),
            Action::Confirm { .. } => (Rpc::Confirm, None, None),
            Action::Acknowledge { .. } => (Rpc::Acknowledge, None, None),
        },
        _ => (Rpc::Other, None, None),
    };
    *lock(&tag.request) = Some((rpc, doc.clone()));
    p.note(Event::Wire {
        frame: tag.frame,
        reply: false,
        rpc,
        doc,
        body,
    });
}
pub(crate) fn reply_written(tag: &FrameTag, response: &Response) {
    if !tag.admitted.load(Ordering::Acquire) {
        return;
    }
    let Some(p) = &tag.probe else {
        return;
    };
    let Some((rpc, doc)) = lock(&tag.request).clone() else {
        p.fail("reply without admitted frame");
        return;
    };
    let body = match response {
        Response::DocTransfer {
            reply:
                Reply::Exchanged {
                    offer: Some(chunk), ..
                }
                | Reply::Chunk { chunk },
        } => p.body(chunk),
        _ => None,
    };
    p.note(Event::Wire {
        frame: tag.frame,
        reply: true,
        rpc,
        doc,
        body,
    });
}
pub(crate) fn quantum(root: &Path, doc: &str) {
    if let Some(p) = probe(root) {
        p.note(Event::Quantum(doc.into()));
    }
}
pub(crate) async fn before_step(root: &Path, doc: &str, kind: StepKind) {
    if let Some(p) = probe(root) {
        p.checkpoint(Point::Before(doc.into(), kind), None).await;
    }
}
pub(crate) async fn after_step(root: &Path, doc: &str, kind: StepKind, failure: Option<Failure>) {
    if let Some(p) = probe(root) {
        p.checkpoint(Point::After(doc.into(), kind), failure).await;
    }
}
pub(crate) async fn before_receipts(root: &Path, doc: &str) {
    if let Some(p) = probe(root) {
        p.checkpoint(Point::Receipts(doc.into()), None).await;
    }
}

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        if self.0.exists() {
            if let Err(error) = fs::remove_dir_all(&self.0) {
                eprintln!("[Q039-FLOW-CLEANUP-FAIL] {error}");
                assert!(
                    std::thread::panicking(),
                    "cleanup: private flow scratch removal failed"
                );
            }
        }
    }
}
struct Resident {
    task: JoinHandle<Result<(), String>>,
    root: PathBuf,
    rounds: Arc<Mutex<std::collections::HashMap<String, super::super::RoundWatch>>>,
    stopped: bool,
}
impl Drop for Resident {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Resident {
    fn start(node: Node) -> Self {
        let root = node.root().clone();
        let rounds = node.rounds.clone();
        let task = tokio::spawn(async move { node.serve_resumable().await });
        Self {
            task,
            root,
            rounds,
            stopped: false,
        }
    }
    async fn stop(&mut self) {
        if self.stopped {
            return;
        }
        if !self.task.is_finished() {
            assert!(
                super::super::deafen(&self.root).await,
                "cleanup: actual resident endpoint must close"
            );
            self.task.abort();
        }
        let _ = (&mut self.task).await;
        self.stopped = true;
    }
    async fn drained(&self) {
        timeout(BY, async {
            loop {
                let quiet = lock(&self.rounds)
                    .values()
                    .all(|(_, state)| state.borrow().is_some());
                if quiet {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("cleanup: all actual peer rounds must finish before scratch removal");
    }
}
struct Running(JoinHandle<Result<Vec<(String, Result<String, String>)>, String>>);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.abort();
    }
}
struct Fixture {
    resident: Resident,
    a: Arc<Node>,
    b: Node,
    _scratch: Scratch,
}
impl Fixture {
    async fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "khor-doc-flow-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let scratch = Scratch(fs::canonicalize(path).unwrap());
        for side in ["a", "b"] {
            let home = scratch.0.join(side);
            fs::create_dir(&home).unwrap();
            util::hermetic(&home);
        }
        let server = Node::open_hosting(scratch.0.join("b"), "flow-b", false).unwrap();
        let resident = Resident::start(server);
        timeout(BY, async {
            while !scratch.0.join("b/.khor/endpoint.json").exists() {
                assert!(!resident.task.is_finished(), "fixture resident exited");
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        let a = Arc::new(Node::open_hosting(scratch.0.join("a"), "flow-a", false).unwrap());
        let b = Node::open_hosting(scratch.0.join("b"), "flow-b", false).unwrap();
        timeout(BY, a.pair(&b.invite().unwrap()))
            .await
            .unwrap()
            .unwrap();
        let mut result = Self {
            resident,
            a,
            b,
            _scratch: scratch,
        };
        result.settle().await;
        result.no_source_resident();
        result
    }
    fn context(node: &Node) -> Context {
        Context {
            root: node.root().clone(),
            writer: node.writer_peer(),
            local: khor_net::trust::id_from_hex(node.device_str()).unwrap(),
        }
    }
    async fn settle(&mut self) {
        for _ in 0..6 {
            let verdict = timeout(BY, self.a.sync_now()).await.unwrap().unwrap();
            summary(verdict);
            let ca = Self::context(&self.a);
            let cb = Self::context(&self.b);
            let same = ROUND_DOCS
                .iter()
                .all(|name| ca.version(name).unwrap() == cb.version(name).unwrap());
            let pending_a = State::open(&ca).unwrap().pending_docs(cb.local).unwrap();
            let pending_b = State::open(&cb).unwrap().pending_docs(ca.local).unwrap();
            if same && pending_a.is_empty() && pending_b.is_empty() {
                return;
            }
        }
        panic!("setup: real small-doc sync and receipts must settle before tracing");
    }
    fn run(&self) -> Running {
        let a = self.a.clone();
        Running(tokio::spawn(async move { a.sync_now().await }))
    }
    fn scopes(&self) -> (Scope, Scope) {
        let sender = Scope::new(self.a.root(), self.b.device_str());
        let receiver = Scope::new(self.b.root(), self.a.device_str());
        (sender, receiver)
    }
    fn no_source_resident(&self) {
        assert!(
            !self.a.root().join(".khor/endpoint.json").exists(),
            "setup: A must be the public one-shot Node, never another resident"
        );
        let loaded = khor_sync::store::load_pile::<khor_sync::devices::DeviceDoc>(
            &khor_sync::devices::devices_dir(self.b.root()),
            self.b.writer_peer(),
        )
        .unwrap();
        assert!(
            loaded
                .doc
                .get(self.a.device_str())
                .unwrap()
                .addrs
                .is_empty(),
            "setup: the one-shot caller does not publish its temporary endpoint roads"
        );
    }
    async fn stop(&mut self) {
        self.resident.stop().await;
        for root in [self.a.root(), self.b.root()] {
            if let Some(p) = probe(root) {
                p.release_all();
            }
        }
        self.resident.drained().await;
        fs::remove_dir_all(&self._scratch.0)
            .expect("cleanup: remove actual private Node data before the business assertion");
        assert!(
            !self._scratch.0.exists(),
            "cleanup: private Node root remains"
        );
        eprintln!(
            "[Q039-FLOW-CLEANUP] real_resident_closed=true real_rounds_drained=true private_nodes_removed=true"
        );
    }
    fn independent(&self) -> bool {
        let seen = khor_sync::store::load_pile::<khor_sync::seen::SeenDoc>(
            &khor_sync::seen::seen_dir(self.b.root()),
            0xFA71,
        )
        .unwrap();
        let pins = khor_sync::store::load_pile::<khor_sync::pins::PinDoc>(
            &khor_sync::pins::pins_dir(self.b.root()),
            0xFA72,
        )
        .unwrap();
        seen.doc.watermark("tui/flow-independent") == 123 && pins.doc.pinned("tui/flow-independent")
    }
    fn small_changes(&self) {
        let mut seen = khor_sync::store::load_pile::<khor_sync::seen::SeenDoc>(
            &khor_sync::seen::seen_dir(self.a.root()),
            self.a.writer_peer(),
        )
        .unwrap();
        seen.doc.mark("tui/flow-independent", 123).unwrap();
        seen.store.flush(&seen.doc).unwrap();
        drop(seen);
        let mut pins = khor_sync::store::load_pile::<khor_sync::pins::PinDoc>(
            &khor_sync::pins::pins_dir(self.a.root()),
            self.a.writer_peer(),
        )
        .unwrap();
        pins.doc.set("tui/flow-independent", true).unwrap();
        pins.store.flush(&pins.doc).unwrap();
    }
    fn prepare(&self, names: &[&str]) -> BTreeMap<(String, bool), DocManifest> {
        let ca = Self::context(&self.a);
        let cb = Self::context(&self.b);
        let mut manifests = BTreeMap::new();
        for name in names {
            let va = ca.version(name).unwrap();
            let vb = cb.version(name).unwrap();
            for (source, other, have, side) in [(&ca, &cb, &vb, true), (&cb, &ca, &va, false)] {
                if let Some(chunk) = State::open(source)
                    .unwrap()
                    .offer(source, other.local, name, have)
                    .unwrap()
                {
                    assert!(chunk.manifest.total_bytes <= khor_sync::transfer::MAX_BLOB_BYTES);
                    manifests.insert((name.to_string(), side), chunk.manifest);
                }
            }
        }
        manifests
    }
    fn large_both_ways(&self, third: bool) -> BTreeMap<(String, bool), DocManifest> {
        for (node, seed) in [(&*self.a, 0x8b81_u64), (&self.b, 0x9b91_u64)] {
            device_history(node, if third { 3072 } else { 512 }, seed);
            agent_history(node, if third { 8192 } else { 1024 }, 2048, seed);
            if third {
                wide_history(node, seed);
            }
        }
        self.small_changes();
        let mut names = vec!["devices", "agents"];
        if third {
            names.push("wide/remaining");
        }
        let manifests = self.prepare(&names);
        self.no_source_resident();
        if third {
            for side in [true, false] {
                assert!(
                    manifests[&("wide/remaining".into(), side)].total_bytes > CHUNK_BYTES as u64,
                    "setup: both sides of the deferred Exchange have complete canonical first chunks"
                );
            }
        }
        for name in ["devices", "agents"] {
            for side in [true, false] {
                assert!(
                    manifests[&(name.into(), side)].total_bytes
                        > if third {
                            8 * 1024 * 1024
                        } else {
                            2 * CHUNK_BYTES as u64
                        },
                    "setup: fixed legitimate history must have enough complete canonical chunks"
                );
            }
        }
        manifests
    }
}
fn summary(rows: Vec<(String, Result<String, String>)>) -> String {
    assert_eq!(rows.len(), 1, "setup: only the real receiver is visited");
    assert_eq!(rows[0].0, "flow-b");
    rows.into_iter()
        .next()
        .unwrap()
        .1
        .expect("a legitimate controlled Busy is Pending, not a hard transport error")
}
fn moved(value: &str) -> usize {
    let (key, args) = khor_catalog::read(value).expect("public sync result has a catalog identity");
    match key {
        "msg.nothing-to-move" => 0,
        "msg.moved-bytes"
        | "msg.sync-partly-pending"
        | "msg.sync-partly-pending-and-apart"
        | "msg.sync-partly-apart" => args[0].parse().unwrap(),
        _ => panic!("unexpected sync summary {key}"),
    }
}
fn pending(value: &str, name: &str) -> bool {
    khor_catalog::read(value).is_some_and(|(key, args)| {
        matches!(
            key,
            "msg.sync-partly-pending" | "msg.sync-partly-pending-and-apart"
        ) && args
            .get(1)
            .is_some_and(|s| s.split(',').any(|d| d.trim() == name))
    })
}
fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}
fn text(seed: &mut u64, bytes: usize) -> String {
    (0..bytes)
        .map(|_| {
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
                [(next(seed) % 62) as usize] as char
        })
        .collect()
}
fn device_history(node: &Node, rounds: usize, mut seed: u64) {
    let mut loaded = khor_sync::store::load_pile::<khor_sync::devices::DeviceDoc>(
        &khor_sync::devices::devices_dir(node.root()),
        node.writer_peer(),
    )
    .unwrap();
    let me = loaded.doc.get(node.device_str()).unwrap();
    for _ in 0..rounds {
        let addrs = (0..64)
            .map(|_| {
                let ip = (0x20010db8u128 << 96)
                    | ((((next(&mut seed) as u128) << 64) | next(&mut seed) as u128)
                        & ((1u128 << 96) - 1));
                format!(
                    "[{}]:{}",
                    std::net::Ipv6Addr::from(ip),
                    1024 + next(&mut seed) % 55000
                )
            })
            .collect::<Vec<_>>();
        loaded.doc.upsert(&me.id, &me.name, &addrs).unwrap();
    }
    loaded.doc.upsert(&me.id, &me.name, &me.addrs).unwrap();
    loaded.store.flush(&loaded.doc).unwrap();
}
fn agent_history(node: &Node, rounds: usize, bytes: usize, mut seed: u64) {
    let mut loaded = khor_sync::store::load_pile::<khor_sync::agents::AgentDoc>(
        &khor_sync::agents::agents_dir(node.root()),
        node.writer_peer(),
    )
    .unwrap();
    for _ in 0..rounds {
        loaded
            .doc
            .set(
                node.name(),
                &khor_sync::agents::Spec {
                    command: "/usr/bin/true".into(),
                    args: vec![text(&mut seed, bytes)],
                },
            )
            .unwrap();
    }
    loaded
        .doc
        .set(
            node.name(),
            &khor_sync::agents::Spec {
                command: "/usr/bin/true".into(),
                args: vec![format!("fixed-{}", node.name())],
            },
        )
        .unwrap();
    loaded.store.flush(&loaded.doc).unwrap();
}
fn wide_history(node: &Node, mut seed: u64) {
    let dir = khor_sync::chat::wide_dir(node.root(), "wide/remaining").unwrap();
    let mut loaded = khor_sync::chat::open_channel(&dir, node.writer_peer()).unwrap();
    let from = khor_sync::chat::Sender {
        id: node.device_str().into(),
        name: node.name().into(),
    };
    loaded
        .doc
        .tell(&from, &text(&mut seed, 600 * 1024))
        .unwrap();
    loaded.store.flush(&loaded.doc).unwrap();
}

// Independent of the original many-operation Devices/Agents pressure case.
// A few legitimate chat writes keep the same real byte budget while avoiding
// that case's repeated import of hundreds of thousands of historical edits.
const CHAT_BUDGET_MAIN: [&str; 4] = [
    "wide/budget-main-01",
    "wide/budget-main-02",
    "wide/budget-main-03",
    "wide/budget-main-04",
];
const CHAT_BUDGET_DEFERRED: &str = "wide/budget-z-deferred";
const CHAT_BUDGET_PAYLOAD: usize = 4 * 1024 * 1024 + CHUNK_BYTES;

fn one_chat_message(node: &Node, board: &str, bytes: usize, mut seed: u64) {
    let began = Instant::now();
    let dir = khor_sync::chat::wide_dir(node.root(), board).unwrap();
    let mut loaded = khor_sync::chat::open_channel(&dir, node.writer_peer()).unwrap();
    let from = khor_sync::chat::Sender {
        id: node.device_str().into(),
        name: node.name().into(),
    };
    loaded.doc.tell(&from, &text(&mut seed, bytes)).unwrap();
    loaded.store.flush(&loaded.doc).unwrap();
    drop(loaded);
    eprintln!(
        "[Q039-CHAT-BUDGET-PHASE] phase=one-message-write side={} doc={board} payload_bytes={bytes} elapsed_ms={}",
        node.name(),
        began.elapsed().as_millis()
    );
}

#[derive(Default)]
struct ChatBudgetPhase {
    steps: usize,
    busy: usize,
    total: Duration,
    maximum: Duration,
}

fn target_present(node: &Node, manifest: &DocManifest) -> bool {
    let d = manifest.descriptor().unwrap();
    let v = Fixture::context(node).version(&d.document).unwrap();
    v.epoch == d.epoch && v.version.includes_vv(&d.target)
}
fn dump(sender: &Probe, receiver: &Probe) {
    for (side, probe) in [("sender", sender), ("receiver", receiver)] {
        for e in probe.entries() {
            match e.event {
                Event::Point { point, failure } => eprintln!(
                    "[Q039-FLOW-TRACE] side={side} order={} point={point:?} failure={failure:?}",
                    e.order
                ),
                event => eprintln!("[Q039-FLOW-TRACE] side={side} order={} {event:?}", e.order),
            }
        }
        for (id, manifest) in &lock(&probe.trace).manifests {
            let descriptor = manifest.descriptor().unwrap();
            eprintln!(
                "[Q039-FLOW-MANIFEST] side={side} id={id} doc={} epoch={} body_bytes={}",
                descriptor.document, descriptor.epoch, manifest.total_bytes
            );
        }
    }
}
fn assert_round_order(sender: &Probe, agents_complete_at: Option<u64>) {
    let entries = sender.entries();
    if let Some(order) = agents_complete_at {
        assert!(
            entries.iter().any(|e| e.order == order
                && matches!(&e.event,
                Event::Point { point: Point::After(doc, StepKind::Read), failure: None }
                if doc == "agents")),
            "completion proof must bind the actual successful Agents AfterRead event"
        );
    }
    let docs = entries
        .into_iter()
        .filter_map(|e| match e.event {
            Event::Quantum(doc) if doc == "devices" || doc == "agents" => Some((e.order, doc)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let jointly_pending = docs
        .iter()
        .filter(|(order, _)| agents_complete_at.is_none_or(|done| *order < done))
        .collect::<Vec<_>>();
    assert!(
        jointly_pending.len() >= 4
            && jointly_pending[0].1 == "devices"
            && jointly_pending[1].1 == "agents",
        "both selected actual Docs must obtain a first real quantum"
    );
    assert!(
        jointly_pending.windows(2).all(|p| p[0].1 != p[1].1),
        "real Busy Doc must not skip its independent pending sibling's turn"
    );
    if let Some(done) = agents_complete_at {
        assert!(
            docs.iter()
                .filter(|(order, _)| *order > done)
                .all(|(_, doc)| doc == "devices"),
            "only the remaining Doc may continue after both fixed Agents targets are covered"
        );
    }
}
fn assert_wire_integrity(sender: &Probe, receiver: &Probe) {
    let events = receiver.entries();
    let mut frames = BTreeMap::new();
    for e in &events {
        if let Event::Wire { frame, reply, .. } = &e.event {
            frames.entry(*frame).or_insert_with(|| (0, 0));
            if *reply {
                frames.get_mut(frame).unwrap().1 += 1;
            } else {
                frames.get_mut(frame).unwrap().0 += 1;
            }
        }
    }
    assert!(
        frames.values().all(|pair| *pair == (1, 1)),
        "setup: all measured real frames must be admitted and completely answered"
    );
    // B's outbound pump can have its own Quantum/Point events. Only
    // A's quanta and B's admitted incoming frames describe this visit.
    let mut combined = sender
        .entries()
        .into_iter()
        .filter(|e| matches!(e.event, Event::Quantum(_)))
        .collect::<Vec<_>>();
    combined.extend(
        events
            .into_iter()
            .filter(|e| matches!(e.event, Event::Wire { .. })),
    );
    combined.sort_by_key(|e| e.order);
    let mut doc = None;
    let mut chunks = 0;
    let mut bytes = 0;
    for e in combined {
        match e.event {
            Event::Quantum(name) => {
                doc = Some(name);
                chunks = 0;
                bytes = 0;
            }
            Event::Wire {
                doc: Some(name),
                body: Some(body),
                ..
            } if body.bytes > 0 => {
                assert_eq!(
                    doc.as_deref(),
                    Some(name.as_str()),
                    "wire body belongs to the current actual quantum"
                );
                assert!(
                    body.bytes <= CHUNK_BYTES
                        && (body.offset == receiver.manifest(&body.id).total_bytes
                            || body.offset % CHUNK_BYTES as u64 == 0)
                );
                chunks += 1;
                bytes += body.bytes;
                assert!(
                    chunks <= 4 && bytes <= 4 * CHUNK_BYTES,
                    "actual bidirectional quantum exceeded four canonical chunks"
                );
            }
            _ => (),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "isolated native Node flow; run the versioned acceptance entry with --ignored --exact"]
async fn a_stable_public_sync_uses_one_actual_versions_frame() {
    let mut f = Fixture::new().await;
    let ca = Fixture::context(&f.a);
    let cb = Fixture::context(&f.b);
    let before_a = ROUND_DOCS.map(|name| ca.version(name).unwrap());
    let before_b = ROUND_DOCS.map(|name| cb.version(name).unwrap());
    let (sender, receiver) = f.scopes();
    let value = summary(timeout(BY, f.a.sync_now()).await.unwrap().unwrap());
    let events = receiver.0.entries();
    let after_a = ROUND_DOCS.map(|name| ca.version(name).unwrap());
    let after_b = ROUND_DOCS.map(|name| cb.version(name).unwrap());
    f.stop().await;
    dump(&sender.0, &receiver.0);
    let asks = events
        .iter()
        .filter_map(|e| match &e.event {
            Event::Wire {
                reply: false, rpc, ..
            } => Some(*rpc),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        asks,
        [Rpc::Versions],
        "quiet public sync must be one actual wire opening, including early ingress"
    );
    assert_eq!(moved(&value), 0);
    assert_eq!(before_a, after_a);
    assert_eq!(before_b, after_b);
    sender.0.healthy();
    receiver.0.healthy();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "isolated native Node flow; run the versioned acceptance entry with --ignored --exact"]
async fn bidirectional_wire_bytes_remain_charged_when_a_real_sink_is_busy() {
    let mut f = Fixture::new().await;
    let (sender, receiver) = f.scopes();
    let fixed = f.large_both_ways(false);
    sender.0.reset_measurement();
    receiver.0.reset_measurement();
    sender
        .0
        .rule(Point::Before("devices".into(), StepKind::Write), true);
    sender
        .0
        .rule(Point::After("agents".into(), StepKind::Read), true);
    let mut running = f.run();
    let mut busy = 0;
    let mut independent = false;
    let mut agents_complete_at = None;
    let value = loop {
        tokio::select! {
            result = &mut running.0 => break summary(result.unwrap().unwrap()),
            before = sender.0.next() => {
                if before.0.point == Point::After("agents".into(), StepKind::Read) {
                    assert_eq!(before.0.failure, None, "the observed actual Agents read must succeed");
                    if target_present(&f.b, &fixed[&("agents".into(), true)])
                        && target_present(&f.a, &fixed[&("agents".into(), false)])
                    {
                        // Both fresh target VVs are read while the real successful
                        // step is paused, before any next foreground quantum.
                        agents_complete_at.get_or_insert(before.0.order);
                    }
                    before.release();
                    continue;
                }
                assert_eq!(before.0.point, Point::Before("devices".into(), StepKind::Write));
                let source_manifest = &fixed[&("devices".into(), true)];
                let blobs = khor_sync::transfer::TransferStore::open(f.b.root().join(".khor/doc-transfer-blobs")).unwrap();
                let held = blobs.open_existing(&source_manifest.core().unwrap()).unwrap().expect("setup: actual first Exchange already stored Receiving before owner conflict");
                sender.0.rule(Point::After("devices".into(), StepKind::Write), false); before.release();
                let after = sender.0.next().await; assert_eq!(after.0.failure, Some(Failure::Busy), "the actual receive must report core owner Busy");
                drop(held); after.release(); busy += 1;
                independent |= f.independent() && !target_present(&f.b, source_manifest);
            }
        }
    };
    let actual = receiver.0.body_bytes();
    let charged = moved(&value);
    let events = receiver.0.entries();
    f.stop().await;
    dump(&sender.0, &receiver.0);
    assert!(
        busy > 0 && independent,
        "setup: real short owner conflicts and independent Seen/Pin must occur while Devices remains incomplete"
    );
    assert!(events.iter().any(|e| matches!(&e.event, Event::Wire { rpc: Rpc::Exchange, doc: Some(d), body: Some(b), reply: true, .. } if d=="agents" && b.bytes==CHUNK_BYTES)), "the independent Doc must have a real reverse full chunk");
    assert_wire_integrity(&sender.0, &receiver.0);
    assert_round_order(&sender.0, agents_complete_at);
    eprintln!(
        "[Q039-FLOW] busy={busy} actual_body={actual} charged_body={charged} seen_pin_independent={independent} agents_complete_after_order={agents_complete_at:?}"
    );
    assert_eq!(
        charged, actual,
        "real failed/retransmitted body must remain charged after a Busy step"
    );
    assert!(
        actual <= VISIT_BYTES,
        "actual known body must respect the visit quota even under real Busy"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "isolated native Node flow; run the versioned acceptance entry with --ignored --exact"]
async fn a_real_last_single_chunk_budget_defers_a_bidirectional_exchange() {
    let mut f = Fixture::new().await;
    let (sender, receiver) = f.scopes();
    f.large_both_ways(true);
    sender.0.reset_measurement();
    receiver.0.reset_measurement();
    for (name, kind) in [
        ("devices", StepKind::Write),
        ("devices", StepKind::Read),
        ("agents", StepKind::Write),
        ("agents", StepKind::Read),
        ("wide/remaining", StepKind::Exchange),
    ] {
        sender.0.rule(Point::Before(name.into(), kind), true);
    }
    let mut running = f.run();
    let mut boundary = false;
    let mut pre_io_busy = 0;
    let value = loop {
        tokio::select! {
            result = &mut running.0 => break summary(result.unwrap().unwrap()),
            before = sender.0.next() => {
                let Point::Before(doc, kind) = before.0.point.clone() else { panic!("unexpected controller point"); };
                let actual = receiver.0.body_bytes(); let left = VISIT_BYTES.saturating_sub(actual);
                let near = left >= CHUNK_BYTES && left < 2 * CHUNK_BYTES;
                boundary |= near;
                let hold = if doc=="wide/remaining" { !near } else { near };
                if hold {
                    let gate = state::test_metadata_gate(f.a.root()).unwrap();
                    sender.0.rule(Point::After(doc.clone(), kind), false); before.release();
                    let after = sender.0.next().await; assert_eq!(after.0.failure, Some(Failure::Busy), "only the real metadata gate produces this pre-IO Busy");
                    drop(gate); after.release(); pre_io_busy += 1;
                } else { before.release(); }
            }
        }
    };
    let actual = receiver.0.body_bytes();
    let target_started = receiver.0.entries().iter().any(|e| matches!(&e.event, Event::Wire { reply: false, rpc: Rpc::Exchange, doc: Some(doc), .. } if doc=="wide/remaining"));
    let independent = f.independent();
    f.stop().await;
    dump(&sender.0, &receiver.0);
    assert!(
        boundary && pre_io_busy > 0 && independent,
        "setup: the unchanged30s visit must really reach the last-one-chunk window and persist Seen/Pin; no seeded budget is accepted"
    );
    assert_wire_integrity(&sender.0, &receiver.0);
    eprintln!(
        "[Q039-FLOW] last_chunk_reached={boundary} actual_body={actual} charged_body={} target_exchange={target_started}",
        moved(&value)
    );
    assert!(
        actual <= VISIT_BYTES && !target_started,
        "one real remaining chunk must not admit a two-body Exchange"
    );
    assert!(pending(&value, "wide/remaining"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "isolated native Node flow; run the versioned acceptance entry with --ignored --exact"]
async fn a_pure_pull_stays_pending_when_the_first_receipt_step_hits_a_real_gate() {
    let mut f = Fixture::new().await;
    let (sender, receiver) = f.scopes();
    agent_history(&f.b, 512, 1024, 0xaaff);
    let fixed = f.prepare(&["agents"]);
    let manifest = fixed[&("agents".into(), false)].clone();
    assert!(manifest.total_bytes > CHUNK_BYTES as u64);
    assert!(!target_present(&f.a, &manifest));
    sender.0.reset_measurement();
    receiver.0.reset_measurement();
    sender.0.rule(Point::Receipts("agents".into()), false);
    let mut running = f.run();
    let before = sender.0.next().await;
    assert!(receiver.0.entries().iter().any(|e| matches!(&e.event, Event::Wire { reply: true, rpc: Rpc::Exchange, doc: Some(doc), body: Some(b), .. } if doc=="agents" && b.bytes==CHUNK_BYTES)), "setup: actual reverse offer must have arrived before the local receipt barrier");
    let gate = state::test_metadata_gate(f.a.root()).unwrap();
    sender
        .0
        .rule(Point::After("agents".into(), StepKind::Exchange), false);
    before.release();
    let after = sender.0.next().await;
    assert_eq!(after.0.failure, Some(Failure::Busy));
    drop(gate);
    after.release();
    let first = summary(timeout(BY, &mut running.0).await.unwrap().unwrap().unwrap());
    let absent_first = !target_present(&f.a, &manifest);
    let correct_pending = pending(&first, "agents");
    dump(&sender.0, &receiver.0);
    drop(sender); // no more fixture pressure; the reopened public Node is real.
    f.no_source_resident(); // A fresh public Node cannot route through any old A resident.
    let reopened = Node::open_hosting(f.a.root().clone(), "flow-a", false).unwrap();
    let second = summary(timeout(BY, reopened.sync_now()).await.unwrap().unwrap());
    let present_after = target_present(&reopened, &manifest);
    let agent = {
        let loaded = khor_sync::store::load_pile::<khor_sync::agents::AgentDoc>(
            &khor_sync::agents::agents_dir(reopened.root()),
            reopened.writer_peer(),
        )
        .unwrap();
        loaded.doc.get("flow-b")
    };
    let same_manifest = receiver
        .0
        .entries()
        .iter()
        .filter_map(|e| match &e.event {
            Event::Wire {
                reply: true,
                rpc: Rpc::Exchange,
                doc: Some(doc),
                body: Some(b),
                ..
            } if doc == "agents" => Some(b.id.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    f.stop().await;
    assert!(
        absent_first
            && present_after
            && agent
                == Some(khor_sync::agents::Spec {
                    command: "/usr/bin/true".into(),
                    args: vec!["fixed-flow-b".into()]
                })
            && !pending(&second, "agents"),
        "setup/control: release and fresh reopen must genuinely complete the fixed reverse target"
    );
    assert!(
        same_manifest.len() >= 2 && same_manifest.iter().all(|id| id == &manifest.id().unwrap()),
        "restart must resume the same published immutable offer"
    );
    eprintln!(
        "[Q039-FLOW] pure_pull first_pending={correct_pending} first_target_absent={absent_first} reopened_target={present_after}"
    );
    assert!(
        correct_pending,
        "pure-pull receipt Busy must leave the actual agents document Pending, never false-done"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "isolated native Node flow; run the versioned acceptance entry with --ignored --exact"]
async fn few_op_chat_defers_a_two_body_exchange_with_one_real_chunk_left() {
    let mut f = Fixture::new().await;
    let (sender, receiver) = f.scopes();
    let setup_began = Instant::now();
    let mut names = CHAT_BUDGET_MAIN.to_vec();
    names.push(CHAT_BUDGET_DEFERRED);
    for (node, side_seed) in [(&*f.a, 0x431a_u64), (&f.b, 0x532b_u64)] {
        for (index, name) in names.iter().enumerate() {
            let bytes = if *name == CHAT_BUDGET_DEFERRED {
                600 * 1024
            } else {
                CHAT_BUDGET_PAYLOAD
            };
            one_chat_message(node, name, bytes, side_seed + index as u64);
        }
    }
    f.small_changes();
    let spool_began = Instant::now();
    let fixed = f.prepare(&names);
    eprintln!(
        "[Q039-CHAT-BUDGET-PHASE] phase=fixed-spool elapsed_ms={} setup_elapsed_ms={}",
        spool_began.elapsed().as_millis(),
        setup_began.elapsed().as_millis()
    );
    f.no_source_resident();
    let selected: Vec<_> =
        f.a.known_wide()
            .into_iter()
            .filter(|n| names.contains(&n.as_str()))
            .collect();
    assert_eq!(
        selected, names,
        "setup: actual wide registry order must select all four main Docs before the deferred Doc"
    );
    for name in CHAT_BUDGET_MAIN {
        for side in [true, false] {
            let bytes = fixed[&(name.to_owned(), side)].total_bytes;
            assert!(
                bytes > 4 * 1024 * 1024 && bytes <= 64 * 1024 * 1024,
                "setup: each legitimate few-op main Doc must export more than4MiB and fit the unchanged body cap"
            );
        }
    }
    for side in [true, false] {
        assert!(
            fixed[&(CHAT_BUDGET_DEFERRED.to_owned(), side)].total_bytes > CHUNK_BYTES as u64,
            "setup: both deferred first chunks must be full canonical chunks"
        );
    }
    sender.0.reset_measurement();
    receiver.0.reset_measurement();
    for name in &names {
        for kind in [StepKind::Exchange, StepKind::Write, StepKind::Read] {
            sender.0.rule(Point::Before((*name).into(), kind), true);
        }
    }
    let mut phases: BTreeMap<(String, StepKind), ChatBudgetPhase> = BTreeMap::new();
    let mut first_boundary_quanta = None;
    let mut pre_io_busy = 0;
    let visit_began = Instant::now();
    let mut running = f.run();
    let value = loop {
        tokio::select! {
            result = &mut running.0 => break summary(result.unwrap().unwrap()),
            before = sender.0.next() => {
                let Point::Before(doc, kind) = before.0.point.clone() else { panic!("setup: unexpected Chat budget controller point"); };
                assert!(names.contains(&doc.as_str()), "setup: controller must target only actual selected Chat Docs");
                let actual = receiver.0.body_bytes();
                let left = VISIT_BYTES.saturating_sub(actual);
                let near = left >= CHUNK_BYTES && left < 2 * CHUNK_BYTES;
                if near && first_boundary_quanta.is_none() {
                    first_boundary_quanta = Some(sender.0.entries().into_iter().filter_map(|e| match e.event {
                        Event::Quantum(name) if names.contains(&name.as_str()) => Some(name),
                        _ => None,
                    }).collect::<Vec<_>>());
                }
                let hold = if doc == CHAT_BUDGET_DEFERRED { !near } else { near };
                let gate = if hold { Some(state::test_metadata_gate(f.a.root()).unwrap()) } else { None };
                sender.0.rule(Point::After(doc.clone(), kind), false);
                let began = Instant::now();
                before.release();
                let after = sender.0.next().await;
                assert_eq!(after.0.point, Point::After(doc.clone(), kind), "setup: each real Step must finish before advancing the controller");
                let elapsed = began.elapsed();
                if hold {
                    assert_eq!(after.0.failure, Some(Failure::Busy), "setup: the short actual metadata gate must yield real pre-IO Busy");
                    pre_io_busy += 1;
                }
                drop(gate);
                let phase = phases.entry((doc, kind)).or_default();
                phase.steps += 1;
                phase.busy += usize::from(after.0.failure == Some(Failure::Busy));
                phase.total += elapsed;
                phase.maximum = phase.maximum.max(elapsed);
                assert!(phases.len() <= 15, "setup: bounded Chat phase registry overflow");
                after.release();
            }
        }
    };
    let visit_elapsed = visit_began.elapsed();
    let actual = receiver.0.body_bytes();
    let independent = f.independent();
    let events = receiver.0.entries();
    let deferred_frames: Vec<_> = events
        .iter()
        .filter_map(|e| match &e.event {
            Event::Wire {
                frame,
                reply,
                rpc: Rpc::Exchange,
                doc: Some(doc),
                body,
                ..
            } if doc == CHAT_BUDGET_DEFERRED => Some((*frame, *reply, body.clone())),
            _ => None,
        })
        .collect();
    let target_started = deferred_frames.iter().any(|(_, reply, _)| !*reply);
    f.stop().await;
    dump(&sender.0, &receiver.0);
    for ((doc, kind), phase) in &phases {
        eprintln!(
            "[Q039-CHAT-BUDGET-PHASE] phase=end-to-end-step doc={doc} kind={kind:?} steps={} busy={} elapsed_ms={} max_ms={}",
            phase.steps,
            phase.busy,
            phase.total.as_millis(),
            phase.maximum.as_millis()
        );
    }
    let boundary = first_boundary_quanta.is_some();
    eprintln!(
        "[Q039-CHAT-BUDGET] boundary={boundary} actual_body={actual} charged_body={} pre_io_busy={pre_io_busy} seen_pin={independent} target_exchange={target_started} visit_ms={}",
        moved(&value),
        visit_elapsed.as_millis()
    );
    assert!(
        boundary && pre_io_busy > 0 && independent,
        "setup: few-op Chat must really reach the unchanged32MiB last-one-chunk window within the original30s and persist Seen/Pin"
    );
    let quanta = first_boundary_quanta.unwrap();
    assert!(
        quanta.len() >= names.len()
            && quanta
                .iter()
                .enumerate()
                .all(|(n, doc)| doc.as_str() == names[n % names.len()]),
        "setup: all selected real Docs must get fair quanta in actual registry order before the first boundary"
    );
    assert_wire_integrity(&sender.0, &receiver.0);
    if target_started {
        assert_eq!(
            deferred_frames.len(),
            2,
            "setup: exactly one real deferred Exchange must have a complete request/reply pair"
        );
        let frame = deferred_frames[0].0;
        for (observed, reply, body) in &deferred_frames {
            let body = body
                .as_ref()
                .expect("setup: both actual deferred Exchange directions must carry a body");
            assert_eq!(*observed, frame);
            assert_eq!(body.offset, 0);
            assert_eq!(body.bytes, CHUNK_BYTES);
            let side = !*reply;
            assert_eq!(
                body.id,
                fixed[&(CHAT_BUDGET_DEFERRED.to_owned(), side)]
                    .id()
                    .unwrap()
            );
        }
    }
    assert!(
        actual <= VISIT_BYTES && !target_started,
        "one real remaining chunk must not admit the few-op Chat two-body Exchange"
    );
    assert!(pending(&value, CHAT_BUDGET_DEFERRED));
}
