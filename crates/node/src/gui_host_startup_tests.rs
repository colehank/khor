//! Actual owned ByteStreams → startup Store → committed canonical base.
//! The outer worker isolates process-global cwd/resume state from other tests.
use super::*;
use std::io::BufRead;

const ROWS: usize = 1025;
const PAYLOAD: usize = 64 * 1024;
const WORKER: &str = "gui_host::cancel_tests::startup_tests::long_load_worker";

#[test]
fn a_long_load_commits_the_complete_base_before_host_ready() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "khor-acp-long-load-{}",
        crate::link::fresh_hex().unwrap()
    )));
    std::fs::create_dir(&scratch.0).unwrap();
    let mut worker = Worker(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", WORKER, "--ignored", "--nocapture"])
            .env("KHOR_ACP_STOP_SCRATCH", &scratch.0)
            .current_dir(&scratch.0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let until = Instant::now() + Duration::from_secs(45);
    let status = loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < until,
            "long-load worker exceeded its deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    worker
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    worker
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    if let Some(evidence) = std::env::var_os("KHOR_ACP_TEST_EVIDENCE") {
        let evidence = PathBuf::from(evidence).join("long-load");
        std::fs::create_dir_all(&evidence).unwrap();
        for name in ["observations.json", "requests.log", "pid.json"] {
            let source = scratch.0.join("long-load").join(name);
            if source.is_file() {
                std::fs::copy(source, evidence.join(name)).unwrap();
            }
        }
        std::fs::write(evidence.join("worker.stdout"), &stdout).unwrap();
        std::fs::write(evidence.join("worker.stderr"), &stderr).unwrap();
    }
    assert!(status.success(), "{stdout}\n{stderr}");
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("test result:"))
            .count(),
        1,
        "owned helper must have exactly one real test summary: {stdout}"
    );
    assert!(
        stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "owned helper did not actually run its one test: {stdout}"
    );
    println!("[S_LONG_LOAD_WORKER]\n{stdout}\n[S_LONG_LOAD_WORKER_END]");
}

fn rss_kib() -> u64 {
    let result = Command::new("/bin/ps")
        .args(["-p", &std::process::id().to_string(), "-o", "rss="])
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn wait_marker(
    path: &std::path::Path,
    host: &std::thread::JoinHandle<Result<(), String>>,
    until: Instant,
) {
    while !path.is_file() {
        assert!(!host.is_finished(), "host ended before {}", path.display());
        assert!(Instant::now() < until, "missing {}", path.display());
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "isolated owned ByteStreams long-load worker"]
fn long_load_worker() {
    let scratch = PathBuf::from(std::env::var_os("KHOR_ACP_STOP_SCRATCH").unwrap())
        .canonicalize()
        .unwrap();
    let marker = scratch.join("long-load");
    std::fs::create_dir(&marker).unwrap();
    let root = scratch.join("store");
    std::fs::create_dir(&root).unwrap();
    let python =
        PathBuf::from(std::env::var_os("KHOR_ACP_TEST_PYTHON").expect("pinned Python required"));
    assert!(python.is_absolute() && python.is_file());
    let script = scratch.join("agent.py");
    std::fs::write(&script, r#"import json,os,sys,time
from pathlib import Path
root=Path(sys.argv[1]); rows=int(sys.argv[2]); size=int(sys.argv[3])
(root/'pid.json').write_text(json.dumps({'pid':os.getpid(),'pgid':os.getpgrp(),'parent':os.getppid(),'root':str(root.resolve())}))
def send(value): print(json.dumps(value,separators=(',',':')),flush=True)
for line in sys.stdin:
 req=json.loads(line)
 with (root/'requests.log').open('a') as log: log.write(req.get('method','')+'\n')
 if req.get('method')=='initialize':
  send({'jsonrpc':'2.0','id':req['id'],'result':{'protocolVersion':1,'agentCapabilities':{'loadSession':True}}})
 elif req.get('method')=='session/load':
  sid=req['params']['sessionId']; payload='x'*size
  for n in range(rows):
   send({'jsonrpc':'2.0','method':'session/update','params':{'sessionId':sid,'update':{'sessionUpdate':'agent_message_chunk','content':{'type':'text','text':str(n)+':'+payload}}}})
   if n+1 in (256,512,rows):
    (root/('phase-'+str(n+1))).write_text(str((n+1)*size))
    while not (root/('release-'+str(n+1))).exists(): time.sleep(.005)
  send({'jsonrpc':'2.0','id':req['id'],'result':{}})
 else: raise RuntimeError('unexpected method '+str(req.get('method')))
"#).unwrap();
    // No shell, model, network endpoint or user home is used by this peer.
    let command = serde_json::json!({"command":python,"args":[script,marker,ROWS.to_string(),PAYLOAD.to_string()],
        "env":{"HOME":scratch,"PYTHONDONTWRITEBYTECODE":"1"}}).to_string();
    unsafe {
        std::env::set_var(RESUME_ENV, "owned-long-load-session");
        std::env::set_var(VENDOR_ENV, "acp:owned-long-load");
    }
    let ready = scratch.join("long-load.ready");
    let host = {
        let root = root.clone();
        let ready = ready.clone();
        std::thread::spawn(move || {
            gui_host_main_with_timeouts(
                root,
                ready,
                "bounded startup fixture".into(),
                vec![command],
                HostTimeouts::default(),
            )
        })
    };
    let until = Instant::now() + Duration::from_secs(35);
    let mut samples = vec![serde_json::json!({"payload_bytes":0,"rss_kib":rss_kib()})];
    for count in [256, 512, ROWS] {
        wait_marker(&marker.join(format!("phase-{count}")), &host, until);
        assert!(
            !ready.exists(),
            "HostReady overtook the withheld Load response"
        );
        samples.push(serde_json::json!({"payload_bytes":count*PAYLOAD,"rss_kib":rss_kib()}));
        std::fs::write(marker.join(format!("release-{count}")), b"continue").unwrap();
    }
    wait_marker(&ready, &host, until);
    let pid: serde_json::Value =
        serde_json::from_slice(&std::fs::read(marker.join("pid.json")).unwrap()).unwrap();
    assert_eq!(pid["parent"].as_u64(), Some(u64::from(std::process::id())));
    assert_eq!(pid["pid"], pid["pgid"]);
    assert_eq!(pid["root"].as_str(), Some(marker.to_str().unwrap()));
    let pid = pid["pid"].as_i64().unwrap() as libc::pid_t;
    assert_alive(pid);
    let id = SessionId(await_file(&ready).trim().to_owned());
    let node = crate::Node::open(root.clone()).unwrap();
    let live = LiveKind::new(root.clone(), node.device());
    let directory = live.dir_of(&id).unwrap();
    let identity: JournalIdentity =
        serde_json::from_slice(&std::fs::read(directory.join("journal.json")).unwrap()).unwrap();
    let journal = identity.directory(&root);
    assert!(!journal.join("pending").exists());
    let current = journal.join("current");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(current.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["opening"], "load");
    assert_eq!(manifest["gaps"], 0);
    let records = current.join("records.jsonl");
    let bytes = std::fs::metadata(&records).unwrap().len();
    assert!(bytes > 64 * 1024 * 1024);
    let mut count = 0;
    // One bounded record at a time, never read_to_string of the whole base.
    for line in std::io::BufReader::new(std::fs::File::open(&records).unwrap()).lines() {
        let record: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        assert_eq!(record["kind"], "update");
        let text = record["notification"]["update"]["content"]["text"]
            .as_str()
            .unwrap();
        let (sequence, payload) = text.split_once(':').unwrap();
        assert_eq!(sequence.parse::<usize>().unwrap(), count);
        assert_eq!(payload.len(), PAYLOAD);
        assert!(payload.bytes().all(|byte| byte == b'x'));
        count += 1;
    }
    assert_eq!(count, ROWS, "accepted base was truncated or duplicated");
    let mut conn = extra_face(&root, &scratch, "long-load");
    let mut reader = conn.try_clone().unwrap();
    let (tx, notes) = mpsc::sync_channel(16);
    let reading = std::thread::spawn(move || {
        while let Ok(note) = read_frame::<GuiNote>(&mut reader) {
            if tx.send(note).is_err() {
                break;
            }
        }
    });
    let token = [71; 16];
    write_frame(&mut conn, &GuiOp::Capture { token }).unwrap();
    let fence = await_capture(&notes, token);
    let page = capture_page(&node, &fence, false);
    // A 64KiB payload plus its JSON envelope is deliberately above the inline
    // record budget. Exercise the real explicit-reference path, not a smaller
    // fixture or a raised window threshold.
    let reference = page
        .references
        .iter()
        .max_by_key(|reference| reference.range.end)
        .expect("the tail should be an explicit raw record reference");
    assert!(!reference.missing_start && !reference.missing_end);
    assert_eq!(reference.range.end, bytes);
    assert!(reference.range.end - reference.range.start <= 128 * 1024);
    let cursor = page.cursor_before(crate::adaptor::transcript_page::Position {
        start: reference.range.start,
        end: reference.range.end,
        item: 0,
    });
    let raw = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut raw = Vec::new();
            let mut next = Some(reference.range.start);
            while let Some(from) = next {
                let key = node.content_begin(&fence.binding).await.unwrap();
                let request = crate::session_content::remote::ClientRead::claim(
                    node.root(),
                    fence.binding.clone(),
                    &key,
                )
                .unwrap();
                let chunk = node
                    .content_record(&request, cursor.clone(), from)
                    .await
                    .unwrap();
                assert_eq!(chunk.offset, from);
                assert!(!chunk.bytes.is_empty() && chunk.bytes.len() <= 64 * 1024);
                assert_eq!(from, reference.range.start + raw.len() as u64);
                raw.extend_from_slice(&chunk.bytes);
                assert!(raw.len() as u64 <= reference.range.end - reference.range.start);
                next = chunk.next;
                if let Some(next) = next {
                    assert_eq!(next, from + chunk.bytes.len() as u64);
                }
            }
            assert_eq!(
                raw.len() as u64,
                reference.range.end - reference.range.start
            );
            raw
        });
    let tail: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let text = tail["notification"]["update"]["content"]["text"]
        .as_str()
        .unwrap();
    let (sequence, payload) = text.split_once(':').unwrap();
    assert_eq!(sequence, "1024");
    assert_eq!(payload.len(), PAYLOAD);
    assert!(payload.bytes().all(|byte| byte == b'x'));
    close_host(host, &mut conn, &notes, pid);
    drop(conn);
    drop(notes);
    reading.join().unwrap();
    let requests = std::fs::read_to_string(marker.join("requests.log")).unwrap();
    assert_eq!(
        requests
            .lines()
            .filter(|method| *method == "session/load")
            .count(),
        1
    );
    assert_eq!(
        requests
            .lines()
            .filter(|method| *method == "session/new")
            .count(),
        0
    );
    assert!(
        !std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(&format!("khor-acp-events-{}-", std::process::id())))
    );
    std::fs::write(marker.join("observations.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "rows":count,"payload_bytes":ROWS*PAYLOAD,"canonical_bytes":bytes,"samples":samples,
        "rss_scope":"process samples at producer checkpoints, not kernel peak or an 8MiB hard RSS claim",
        "load":1,"new":0,"ready_after_committed_base":true,"child_reaped":true
    })).unwrap()).unwrap();
}
