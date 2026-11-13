//! Distinguish real STOP_SENDING from connection closure and prove wire reuse.
use super::*;
use std::collections::HashSet;

#[derive(Clone)]
struct Event {
    wire: usize,
    stream: String,
    kind: &'static str,
    explicit_stop: Option<bool>,
}
#[derive(Default)]
struct Probe {
    events: Mutex<Vec<Event>>,
    changed: tokio::sync::Notify,
}
static PROBES: OnceLock<Mutex<HashMap<PathBuf, Arc<Probe>>>> = OnceLock::new();

pub(super) fn note(
    root: &Path,
    wire: usize,
    stream: String,
    kind: &'static str,
    explicit_stop: Option<bool>,
) {
    let Some(probes) = PROBES.get() else { return };
    let probe = probes.lock().unwrap().get(root).cloned();
    if let Some(probe) = probe {
        probe.events.lock().unwrap().push(Event {
            wire,
            stream,
            kind,
            explicit_stop,
        });
        probe.changed.notify_one();
    }
}
struct Observing(PathBuf, Arc<Probe>);
impl Observing {
    fn start(root: &Path) -> Self {
        let probe = Arc::new(Probe::default());
        assert!(
            PROBES
                .get_or_init(Mutex::default)
                .lock()
                .unwrap()
                .insert(root.into(), probe.clone())
                .is_none()
        );
        Self(root.into(), probe)
    }
    fn events(&self) -> Vec<Event> {
        self.1.events.lock().unwrap().clone()
    }
    async fn stopped(&self, count: usize) {
        timeout(Duration::from_secs(5), async {
            loop {
                let changed = self.1.changed.notified();
                if self.events().iter().filter(|e| e.kind == "stop").count() >= count {
                    return;
                }
                changed.await;
            }
        })
        .await
        .expect("all remote streams must observe explicit STOP_SENDING");
    }
}
impl Drop for Observing {
    fn drop(&mut self) {
        if let Some(probes) = PROBES.get() {
            let mut probes = probes.lock().unwrap_or_else(|error| error.into_inner());
            if probes.get(&self.0).is_some_and(|p| Arc::ptr_eq(p, &self.1)) {
                probes.remove(&self.0);
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn all_cancelled_reply_streams_observe_stop_and_keep_the_same_wire() {
    let root_a = crate::testdir::TempTree::fresh("content-stop-reader");
    let root_b = crate::testdir::TempTree::fresh("content-stop-home");
    util::hermetic(&root_a);
    util::hermetic(&root_b);
    let mut serve_a = Server::start(&root_a, "stop-reader").await;
    let mut serve_b = Server::start(&root_b, "stop-home").await;
    let a = Arc::new(Node::open_as(root_a.path(), "stop-reader").unwrap());
    let b = Node::open_as(root_b.path(), "stop-home").unwrap();
    timeout(Duration::from_secs(60), a.pair(&b.invite().unwrap()))
        .await
        .unwrap()
        .unwrap();
    let id = "01a17c1a-a542-7822-b01d-82a832a158e6";
    let history: Vec<_> = (0..500)
        .flat_map(|n| message(&format!("stop-{n}")))
        .collect();
    std::fs::write(source(&root_b, id), history).unwrap();
    let observation = ObservationRef {
        owner: b.device(),
        home: b.device(),
        namespace: ObservationNamespace::Native,
        agent: "codex".into(),
        native_id: id.into(),
        imported_at: Millis(1),
        updated_at: None,
    };
    let locator =
        ContentLocator::observed(SessionId("observe/explicit-stop".into()), &observation).unwrap();
    let Reply::Resolved(Resolution::Available { binding, .. }) =
        a.content_read(Action::Resolve { locator }).await.unwrap()
    else {
        panic!("private home fixture must resolve");
    };
    let observed = Observing::start(&root_b);
    let held = HoldWork::install(&root_b, Stage::Response);
    let mut reads = Vec::new();
    let mut tasks = Vec::new();
    for _ in 0..crate::link::CONTENT_RESPONSES {
        let key = a.content_begin(&binding).await.unwrap();
        let read = ClientRead::claim(&root_a, binding.clone(), &key).unwrap();
        let (node, worker) = (a.clone(), read.clone());
        tasks.push(tokio::spawn(async move {
            node.content_page(&worker, None, false).await
        }));
        reads.push(read);
    }
    held.entered(32).await;
    assert_eq!(serve_a.responses.available_permits(), 0);
    assert_eq!(serve_b.responses.available_permits(), 0);
    let windows: Vec<_> = observed
        .events()
        .into_iter()
        .filter(|e| e.kind == "window")
        .collect();
    assert_eq!(windows.len(), 32);
    let wire = windows[0].wire;
    assert!(windows.iter().all(|e| e.wire == wire));
    assert_eq!(
        windows
            .iter()
            .map(|e| &e.stream)
            .collect::<HashSet<_>>()
            .len(),
        32
    );
    assert!(!observed.events().iter().any(|e| e.kind == "stop"));
    for task in tasks {
        task.abort();
        let _ = task.await;
    }
    observed.stopped(32).await;
    settled_responses(&[&serve_a, &serve_b]).await;
    let stops: Vec<_> = observed
        .events()
        .into_iter()
        .filter(|e| e.kind == "stop")
        .collect();
    assert_eq!(stops.len(), 32);
    assert!(
        stops
            .iter()
            .all(|e| e.explicit_stop == Some(true) && e.wire == wire)
    );
    assert_eq!(
        stops.iter().map(|e| &e.stream).collect::<HashSet<_>>(),
        windows.iter().map(|e| &e.stream).collect()
    );
    assert!(!*held.1.released.lock().unwrap());
    let before = observed.events().len();
    let key = a.content_begin(&binding).await.unwrap();
    a.content_cancel(&binding, &key).await.unwrap();
    let after = observed.events();
    assert!(
        after[before..]
            .iter()
            .any(|e| e.kind == "begin" && e.wire == wire)
    );
    assert!(
        after[before..]
            .iter()
            .any(|e| e.kind == "cancel" && e.wire == wire)
    );
    assert!(!*held.1.released.lock().unwrap());
    held.release();
    assert_eq!(page(&a, &binding, None, false).await.events.len(), 128);
    drop(reads);
    drop(held);
    serve_a.stop().await;
    serve_b.stop().await;
    eprintln!(
        "[quic-explicit-stop] {}",
        serde_json::json!({"window_streams":32,"explicit_stops":32,"connection_close_substitutes":0,"same_wire":true,"control_while_reply_held":true})
    );
}
