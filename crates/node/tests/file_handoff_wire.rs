//! An app-owned copy while the phone's resident endpoint is running.
//! The progress wrapper must use the existing authenticated file road
//! without displacing that endpoint or requiring a hosting machine.
use khor_node::{Node, road::Job};
use std::{fs, path::PathBuf, sync::atomic::Ordering, time::Duration};
mod util;

struct Scratch(PathBuf);
impl Scratch {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("khor-handoff-wire-{name}-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        util::hermetic(&root);
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
async fn ready(root: &Scratch) {
    tokio::time::timeout(Duration::from_secs(90), async {
        while !root.0.join(".khor/endpoint.json").is_file() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("resident endpoint must start");
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn original_copy_from_a_resident_phone_keeps_every_byte_and_its_resident() {
    let desk = Scratch::new("desk");
    let phone = Scratch::new("phone");
    let host = Node::open_as(desk.0.clone(), "handoff-desk").unwrap();
    let resident = tokio::spawn(async move { host.serve().await });
    ready(&desk).await;
    let host = Node::open_as(desk.0.clone(), "handoff-desk").unwrap();
    let client = Node::open_hosting(phone.0.clone(), "handoff-phone", false).unwrap();
    tokio::time::timeout(
        Duration::from_secs(60),
        client.pair(&host.invite().unwrap()),
    )
    .await
    .unwrap()
    .unwrap();
    let serving = Node::open_hosting(phone.0.clone(), "handoff-phone", false).unwrap();
    let phone_resident = tokio::spawn(async move { serving.serve().await });
    ready(&phone).await;
    let original: Vec<u8> = (0..(3 * khor_node::proto::SLICE + 123))
        .map(|i| (i % 251) as u8)
        .collect();
    let source = desk.0.join("original.pptx");
    fs::write(&source, &original).unwrap();
    let cache = phone.0.join("cache");
    fs::create_dir(&cache).unwrap();
    let job = Job::default();
    let copy = tokio::time::timeout(
        Duration::from_secs(60),
        client.pull_path_progress("handoff-desk", source.to_str().unwrap(), &cache, &job),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(fs::read(copy).unwrap(), original);
    assert_eq!(job.moved.load(Ordering::Relaxed), original.len() as u64);
    assert_eq!(job.total.load(Ordering::Relaxed), original.len() as u64);
    assert!(!client.hosts());
    assert!(
        !phone_resident.is_finished(),
        "original-file handoff displaced the phone's resident"
    );
    tokio::time::timeout(Duration::from_secs(60), client.sync_now())
        .await
        .unwrap()
        .unwrap();
    phone_resident.abort();
    resident.abort();
    let _ = phone_resident.await;
    let _ = resident.await;
}
