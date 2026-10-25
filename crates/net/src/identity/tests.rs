use super::*;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "khor-identity-test-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn key(&self) -> PathBuf {
        self.0.join("identity.key")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_same_identity_comes_back_on_the_second_call() {
    let dir = Scratch::new();
    let a = load_or_create(&dir.key()).unwrap();
    let original = std::fs::read(dir.key()).unwrap();
    let b = load_or_create(&dir.key()).unwrap();
    assert_eq!(a.public(), b.public());
    assert_eq!(std::fs::read(dir.key()).unwrap(), original);
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn a_world_readable_key_is_refused_rather_than_used() {
    use std::os::unix::fs::PermissionsExt;
    let dir = Scratch::new();
    load_or_create(&dir.key()).unwrap();
    let original = std::fs::read(dir.key()).unwrap();
    std::fs::set_permissions(dir.key(), std::fs::Permissions::from_mode(0o644)).unwrap();
    let err = load_or_create(&dir.key()).unwrap_err().to_string();
    let probe = msg::identity_too_open('\u{0}', '\u{0}');
    assert!(err.contains(probe.split('\u{0}').next().unwrap()), "{err}");
    assert!(err.contains("chmod 600"), "{err}");
    assert_eq!(std::fs::read(dir.key()).unwrap(), original);
    assert_eq!(
        std::fs::metadata(dir.key()).unwrap().permissions().mode() & 0o777,
        0o644
    );
}

#[test]
fn corrupt_existing_files_are_never_replaced() {
    for size in [0, 31, 33, 1024 * 1024] {
        let dir = Scratch::new();
        let pending = prepare_private(&dir.key(), &[7; 32]).unwrap();
        pending.publish(&dir.key()).unwrap();
        drop(pending);
        let original = vec![7; size];
        std::fs::write(dir.key(), &original).unwrap();
        assert!(load_or_create(&dir.key()).is_err(), "size {size}");
        assert_eq!(std::fs::read(dir.key()).unwrap(), original);
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
    }
}

#[test]
fn candidate_is_complete_and_private_before_publication() {
    let dir = Scratch::new();
    let data = [42; 32];
    let pending = prepare_private(&dir.key(), &data).unwrap();
    assert!(!dir.key().exists());
    assert_eq!(std::fs::read(&pending.path).unwrap(), data);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&pending.path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    pending.publish(&dir.key()).unwrap();
    drop(pending);
    assert_eq!(std::fs::read(dir.key()).unwrap(), data);
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
}

#[test]
fn a_losing_candidate_preserves_the_winner_and_cleans_up() {
    let dir = Scratch::new();
    let first = prepare_private(&dir.key(), &[1; 32]).unwrap();
    let second = prepare_private(&dir.key(), &[2; 32]).unwrap();
    first.publish(&dir.key()).unwrap();
    second.publish(&dir.key()).unwrap();
    drop((first, second));
    assert_eq!(load_or_create(&dir.key()).unwrap().to_bytes(), [1; 32]);
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
}

#[test]
fn a_publish_failure_does_not_fall_back_to_a_move() {
    let dir = Scratch::new();
    // Real syscall failure without mocking the filesystem: hard links to a
    // directory are forbidden, while a rename fallback would move it.
    let candidate = dir.0.join("cannot-hard-link-a-directory");
    std::fs::create_dir(&candidate).unwrap();
    let pending = PendingIdentity {
        path: candidate.clone(),
    };
    assert!(pending.publish(&dir.key()).is_err());
    assert!(candidate.is_dir());
    assert!(!dir.key().exists());
}

#[test]
fn abandoning_a_candidate_only_removes_its_temporary_file() {
    let dir = Scratch::new();
    let pending = prepare_private(&dir.key(), &[5; 32]).unwrap();
    drop(pending);
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn an_existing_dangling_symlink_is_not_replaced() {
    let dir = Scratch::new();
    let target = dir.0.join("missing");
    std::os::unix::fs::symlink(&target, dir.key()).unwrap();
    assert!(load_or_create(&dir.key()).is_err());
    assert_eq!(std::fs::read_link(dir.key()).unwrap(), target);
    assert!(!target.exists());
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
}

#[test]
#[ignore = "worker launched by concurrent_first_creators_share_one_complete_private_identity"]
fn creation_child() {
    let dir = PathBuf::from(std::env::var_os("KHOR_IDENTITY_TEST_DIR").unwrap());
    let child = std::env::var("KHOR_IDENTITY_TEST_CHILD").unwrap();
    std::fs::write(dir.join(format!("ready-{child}")), []).unwrap();
    wait_for(|| dir.join("release").exists());
    let key = load_or_create(&dir.join("identity.key")).unwrap();
    std::fs::write(
        dir.join(format!("public-{child}")),
        format!("{:?}", key.public()),
    )
    .unwrap();
}

fn wait_for(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "identity test subprocess timed out"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

struct ReadObserver {
    done: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<(usize, Vec<String>)>>,
}

impl ReadObserver {
    fn finish(mut self) -> (usize, Vec<String>) {
        self.done.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap()
    }
}

impl Drop for ReadObserver {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

// Reap workers even if an assertion fails, so they cannot outlive the scratch
// directory and leave background processes behind.
struct Workers(Vec<std::process::Child>);

impl Drop for Workers {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn concurrent_first_creators_share_one_complete_private_identity() {
    const CHILDREN: usize = 16;
    for _ in 0..4 {
        let dir = Scratch::new();
        let mut workers = Workers(Vec::new());
        for child in 0..CHILDREN {
            workers.0.push(
                Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", "identity::tests::creation_child", "--ignored"])
                    .env("KHOR_IDENTITY_TEST_DIR", &dir.0)
                    .env("KHOR_IDENTITY_TEST_CHILD", child.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .unwrap(),
            );
        }
        wait_for(|| (0..CHILDREN).all(|child| dir.0.join(format!("ready-{child}")).exists()));
        let done = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(Barrier::new(2));
        let (reader_done, reader_ready, path) = (done.clone(), ready.clone(), dir.key());
        let reader = std::thread::spawn(move || {
            reader_ready.wait();
            let mut seen = 0;
            let mut invalid = Vec::new();
            while !reader_done.load(Ordering::Acquire) {
                match File::open(&path) {
                    Ok(mut file) => {
                        seen += 1;
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let mode = file.metadata().unwrap().permissions().mode() & 0o777;
                            if mode & 0o077 != 0 {
                                invalid.push(format!("mode {mode:o}"));
                            }
                        }
                        let mut raw = Vec::new();
                        file.read_to_end(&mut raw).unwrap();
                        if raw.len() != 32 {
                            invalid.push(format!("{} bytes", raw.len()));
                        }
                    }
                    Err(err) if err.kind() == ErrorKind::NotFound => {}
                    Err(err) => invalid.push(err.to_string()),
                }
            }
            (seen, invalid)
        });
        ready.wait();
        let reader = ReadObserver {
            done,
            thread: Some(reader),
        };
        std::fs::write(dir.0.join("release"), []).unwrap();
        let statuses: Vec<_> = workers
            .0
            .iter_mut()
            .map(|child| {
                let mut status = None;
                wait_for(|| {
                    status = child.try_wait().unwrap();
                    status.is_some()
                });
                status.unwrap()
            })
            .collect();
        // The reader has a published target to inspect even on a slow scheduler.
        std::thread::sleep(Duration::from_millis(10));
        let (seen, invalid) = reader.finish();
        assert!(
            statuses.iter().all(|status| status.success()),
            "{statuses:?}"
        );
        assert!(seen > 0);
        assert!(invalid.is_empty(), "{invalid:?}");
        let winner = format!("{:?}", load_or_create(&dir.key()).unwrap().public());
        for child in 0..CHILDREN {
            assert_eq!(
                std::fs::read_to_string(dir.0.join(format!("public-{child}"))).unwrap(),
                winner
            );
        }
        assert!(!std::fs::read_dir(&dir.0).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".khor-identity-")
        }));
    }
}
