//! One real SDK-owned subprocess, blocked first notification, then cancellation.
//! Resource counters are encoded ownership; RSS samples are separate OS observations.
use super::*;
use agent_client_protocol::{IngressBudget, IngressObserver};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, Weak,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const WINDOW_BYTES: usize = 8 * 1024 * 1024;
const WINDOW_FRAMES: usize = 256;
const BY: Duration = Duration::from_secs(8);

tokio::task_local! { static PRESSURE: Arc<Pressure>; }
pub(super) struct Pressure {
    first: AtomicBool,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
    close: Mutex<Option<Weak<CloseGate>>>,
    ready: AtomicBool,
}
impl Pressure {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            first: AtomicBool::new(false),
            entered: tokio::sync::Notify::new(),
            resume: tokio::sync::Notify::new(),
            close: Mutex::new(None),
            ready: AtomicBool::new(false),
        })
    }
    pub(super) fn attach(&self, close: &Arc<CloseGate>) {
        *self.close.lock().unwrap() = Some(Arc::downgrade(close));
    }
    pub(super) fn opened(&self) {
        self.ready.store(true, Ordering::Release);
    }
    pub(super) async fn note(&self) {
        if !self.first.swap(true, Ordering::AcqRel) {
            self.entered.notify_one();
            self.resume.notified().await;
        }
    }
    fn close(&self) -> Arc<CloseGate> {
        self.close
            .lock()
            .unwrap()
            .as_ref()
            .and_then(Weak::upgrade)
            .expect("actual opening installed its close owner")
    }
}
pub(super) fn current() -> Option<Arc<Pressure>> {
    PRESSURE.try_with(Clone::clone).ok()
}
struct Fixture {
    root: PathBuf,
    cleaned: bool,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "khor-startup-pressure-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            root: root.canonicalize().unwrap(),
            cleaned: false,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.cleaned {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
struct Release<'a> {
    pressure: &'a Pressure,
    path: PathBuf,
}
impl Drop for Release<'_> {
    fn drop(&mut self) {
        if let Some(close) = self
            .pressure
            .close
            .lock()
            .unwrap()
            .as_ref()
            .and_then(Weak::upgrade)
        {
            close.request();
        }
        let _ = fs::write(&self.path, b"cleanup");
        self.pressure.resume.notify_one();
    }
}
fn rss_kib() -> u64 {
    let output = Command::new("/bin/ps")
        .args(["-p", &std::process::id().to_string(), "-o", "rss="])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}
fn process(pid: u32) -> Option<String> {
    let output = Command::new("/bin/ps")
        .args([
            "-p",
            &pid.to_string(),
            "-o",
            "pid=,ppid=,pgid=,lstart=,comm=",
        ])
        .output()
        .unwrap();
    if output.status.success() {
        Some(String::from_utf8(output.stdout).unwrap())
    } else {
        assert_eq!(output.status.code(), Some(1));
        None
    }
}
fn fake_agent(root: &Path) -> (AcpAgent, PathBuf) {
    let script = root.join("agent.py");
    fs::write(&script, r#"import json,os,sys,time
from pathlib import Path
root=Path(sys.argv[1]); sys.stderr=(root/'agent-errors.log').open('w')
(root/'pid.json').write_text(json.dumps({'pid':os.getpid(),'parent':os.getppid(),'root':str(root.resolve())}))
def reply(req,result):
 print(json.dumps({'jsonrpc':'2.0','id':req['id'],'result':result}),flush=True)
for line in sys.stdin:
 req=json.loads(line)
 with (root/'requests.log').open('a') as log: log.write(req.get('method','')+'\n')
 if req.get('method')=='initialize': reply(req,{'protocolVersion':1,'agentCapabilities':{'loadSession':True}})
 elif req.get('method')=='session/load':
  sid=req['params']['sessionId']; payload='x'*(64*1024)
  for n in range(384):
   print(json.dumps({'jsonrpc':'2.0','method':'session/update','params':{'sessionId':sid,'update':{'sessionUpdate':'agent_message_chunk','content':{'type':'text','text':str(n)+':'+payload}}}}),flush=True)
  (root/'produced').write_text('384')
  while not (root/'release').exists(): time.sleep(.01)
  reply(req,{})
 else: raise RuntimeError('unexpected request '+str(req))
"#).unwrap();
    let python = PathBuf::from(
        std::env::var_os("KHOR_TEST_PYTHON").expect("pinned private-fixture interpreter required"),
    );
    assert!(python.is_absolute() && python.is_file());
    let agent = serde_json::json!({"command":python,"args":[script,root],"env":{"HOME":root,"PYTHONDONTWRITEBYTECODE":"1"}}).to_string().parse().unwrap();
    (agent, python)
}
async fn pressure_boundary(
    observer: &IngressObserver,
) -> (agent_client_protocol::IngressStats, u64) {
    let mut sampled_rss = rss_kib();
    let mut next_sample = 1024 * 1024;
    loop {
        let stats = observer.stats();
        if stats.charged_bytes >= next_sample {
            sampled_rss = sampled_rss.max(rss_kib());
            next_sample = stats.charged_bytes + 1024 * 1024;
        }
        if stats.waiting || stats.charged_bytes >= WINDOW_BYTES * 2 {
            return (stats, sampled_rss);
        }
        tokio::task::yield_now().await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_frame_pressure_is_bounded_and_cancellation_retires_the_actual_agent() {
    let mut fixture = Fixture::new();
    let pressure = Pressure::new();
    let _release = Release {
        pressure: &pressure,
        path: fixture.root.join("release"),
    };
    let budget = IngressBudget::new(WINDOW_BYTES, WINDOW_FRAMES).unwrap();
    let observer = budget.observer();
    let before_rss = rss_kib();
    let (agent, python) = fake_agent(&fixture.root);
    let opening = tokio::spawn(PRESSURE.scope(
        pressure.clone(),
        start_transport(
            agent.with_ingress_budget(budget),
            None,
            fixture.root.clone(),
            Some("pressure-owned-session".into()),
            Environment::default(),
        ),
    ));
    tokio::time::timeout(BY, pressure.entered.notified())
        .await
        .unwrap();
    assert!(
        !opening.is_finished(),
        "the real load response remains withheld"
    );
    let pid_proof: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.root.join("pid.json")).unwrap()).unwrap();
    let pid = pid_proof["pid"].as_u64().unwrap() as u32;
    assert_eq!(
        pid_proof["parent"].as_u64().unwrap(),
        u64::from(std::process::id())
    );
    assert_eq!(
        pid_proof["root"].as_str().unwrap(),
        fixture.root.to_str().unwrap()
    );
    let born = process(pid).expect("owned agent remains live");
    let ids: Vec<u32> = born
        .split_whitespace()
        .take(3)
        .map(|id| id.parse().unwrap())
        .collect();
    assert_eq!(ids, [pid, std::process::id(), pid]);
    let executable = born
        .split_whitespace()
        .skip(8)
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        fs::canonicalize(executable).unwrap(),
        fs::canonicalize(python).unwrap()
    );
    let (at_pressure, sampled_rss) = tokio::time::timeout(BY, pressure_boundary(&observer))
        .await
        .unwrap();
    assert_eq!(
        process(pid).as_deref(),
        Some(born.as_str()),
        "same owned PID/birth/executable before cancellation"
    );
    let close = pressure.close();
    close.request();
    let result = tokio::time::timeout(BY, opening).await.unwrap().unwrap();
    assert!(result.is_err());
    let shutdown = tokio::time::timeout(BY, close.completed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        shutdown,
        ShutdownOutcome::DriverClosed,
        "SDK-owned transport is not a fabricated caller-reap receipt"
    );
    tokio::time::timeout(BY, async {
        while process(pid).is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let final_stats = observer.stats();
    assert!(
        final_stats.closed
            && final_stats.charged_bytes == 0
            && final_stats.charged_frames == 0
            && final_stats.undecoded_line_bytes == 0,
        "all accepted credits must land after actual teardown: {final_stats:?}"
    );
    assert!(!pressure.ready.load(Ordering::Acquire));
    let requests = fs::read_to_string(fixture.root.join("requests.log")).unwrap();
    assert_eq!(
        requests
            .lines()
            .filter(|line| *line == "session/load")
            .count(),
        1
    );
    assert!(!requests.lines().any(|line| line == "session/new"));
    assert_eq!(
        at_pressure.largest_oversize_bytes, 0,
        "normal-sized input must not use the exclusive-oversize exception"
    );
    assert!(at_pressure.largest_line_bytes < WINDOW_BYTES);
    let sampled_rss = sampled_rss.max(rss_kib());
    eprintln!(
        "[S_STARTUP_PRESSURE] {}",
        serde_json::json!({"window_bytes":WINDOW_BYTES,"window_frames":WINDOW_FRAMES,"peak_charged_bytes":final_stats.peak_charged_bytes,"peak_charged_frames":final_stats.peak_charged_frames,"largest_completed_line_bytes":at_pressure.largest_line_bytes,"decoded_frame_bytes_at_pressure":at_pressure.decoded_frame_bytes,"reserved_parse_bytes_at_pressure":at_pressure.reserved_parse_bytes(),"undecoded_complete_line_bytes_at_pressure":at_pressure.undecoded_line_bytes,"oversize_bytes":at_pressure.largest_oversize_bytes,"rss_before_kib":before_rss,"maximum_sampled_rss_kib":sampled_rss,"rss_scope":"real test process samples, not kernel peak or payload-byte identity","child_pid":pid,"child_proof":born,"actual_child_gone":true,"sdk_driver_closed":true,"retained_bytes_after":final_stats.charged_bytes,"ready":false,"load_count":1,"new_count":0})
    );
    fixture.cleaned = true;
    assert!(
        final_stats.peak_charged_bytes <= WINDOW_BYTES
            && final_stats.peak_charged_frames <= WINDOW_FRAMES,
        "startup ingress retained the whole history instead of its window: bytes={} frames={}",
        final_stats.peak_charged_bytes,
        final_stats.peak_charged_frames
    );
}
