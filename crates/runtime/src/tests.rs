use super::*;
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    process::Command,
    sync::{Arc, Barrier},
    time::{Duration, Instant},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let mut bytes = [0; 8];
        getrandom::fill(&mut bytes).unwrap();
        let path = std::env::temp_dir().join(format!(
            "khor-runtime-check-{:x}",
            u64::from_ne_bytes(bytes)
        ));
        Self(path)
    }
    fn store(&self) -> Store {
        Store::open(
            self.0.clone(),
            Limits {
                cache_bytes: 512 * 1024 * 1024,
                free_bytes: 0,
            },
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn contract() -> Contract {
    Contract {
        terminal: 5,
        gui: "khor-gui/2".into(),
        node: "wire-fixture".into(),
        store: 1,
        journal: 1,
        agent: 1,
    }
}
fn image(store: &Store, version: &str, bytes: &[u8]) -> Image {
    store
        .publish(
            Role::Cli,
            version,
            contract(),
            BTreeMap::new(),
            Source::Bytes(bytes),
        )
        .unwrap()
}
fn id(image: &Image) -> String {
    image
        .path()
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .into()
}

#[test]
fn same_content_reuses_one_immutable_generation_and_changed_contract_does_not() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let first = image(&store, "one", b"program");
    let reused = image(&store, "one", b"program");
    assert_eq!(first.path(), reused.path());
    let mut changed = contract();
    changed.journal += 1;
    assert!(!contract().accepts(&changed));
    let incompatible = store
        .publish(
            Role::Cli,
            "one",
            changed,
            BTreeMap::new(),
            Source::Bytes(b"program"),
        )
        .unwrap();
    assert_ne!(first.path(), incompatible.path());
    assert_eq!(fs::read(first.path()).unwrap(), b"program");
}

#[test]
fn old_occupied_generation_survives_until_last_cloned_lease_closes() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let first = image(&store, "one", b"first");
    let old = id(&first);
    let clone = first.lease().try_clone().unwrap();
    drop(first);
    drop(image(&store, "two", b"second"));
    drop(image(&store, "three", b"third"));
    assert!(store.collect().unwrap().occupied.contains(&old));
    assert!(store.hold(&old).is_ok());
    drop(clone);
    assert!(store.collect().unwrap().removed.contains(&old));
    assert!(store.hold(&old).is_err());
}

#[test]
fn current_and_one_rollback_are_retained_and_a_failed_publication_does_not_evict_either() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let first = image(&store, "one", b"first");
    let a = id(&first);
    drop(first);
    let second = image(&store, "two", b"second");
    let b = id(&second);
    drop(second);
    let limited = Store::open(
        fixture.0.clone(),
        Limits {
            cache_bytes: 1,
            free_bytes: 0,
        },
    )
    .unwrap();
    assert!(
        limited
            .publish(
                Role::Cli,
                "three",
                contract(),
                BTreeMap::new(),
                Source::Bytes(b"third")
            )
            .is_err()
    );
    assert!(store.hold(&a).is_ok());
    assert!(store.hold(&b).is_ok());
    drop(image(&store, "three", b"third"));
    assert!(!fixture.0.join(&a).exists());
    assert!(store.hold(&b).is_ok());
}

#[test]
fn concurrent_publication_and_collection_never_remove_a_selected_image() {
    let fixture = Fixture::new();
    let store = Arc::new(fixture.store());
    let barrier = Arc::new(Barrier::new(6));
    let workers: Vec<_> = (0..6)
        .map(|number| {
            let store = store.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let bytes = format!("program-{number}");
                barrier.wait();
                let selected = image(&store, &number.to_string(), bytes.as_bytes());
                for _ in 0..4 {
                    store.collect().unwrap();
                    assert_eq!(fs::read(selected.path()).unwrap(), bytes.as_bytes());
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    store.collect().unwrap();
    assert_eq!(store.contracts().unwrap().len(), 2);
}

#[test]
fn unreadable_manifest_and_extra_resources_are_preserved() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let first = image(&store, "one", b"first");
    let a = id(&first);
    drop(first);
    fs::write(fixture.0.join(&a).join("unknown.data"), b"keep").unwrap();
    drop(image(&store, "two", b"second"));
    drop(image(&store, "three", b"third"));
    assert!(store.collect().unwrap().unknown.contains(&a));
    assert_eq!(
        fs::read(fixture.0.join(a).join("unknown.data")).unwrap(),
        b"keep"
    );
}

#[test]
fn failed_compatibility_check_keeps_both_previously_selected_versions() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let first = image(&store, "one", b"first");
    let a = id(&first);
    drop(first);
    let second = image(&store, "two", b"second");
    let b = id(&second);
    drop(second);
    let failed = store.publish_checked(
        Role::Cli,
        "three",
        contract(),
        BTreeMap::new(),
        Source::Bytes(b"incompatible"),
        |candidate| {
            // Verification is allowed to consult the same store without a reentrant catalog lock.
            let held = store.hold(&id(candidate))?;
            assert_eq!(held.path(), candidate.path());
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "incompatible storage",
            ))
        },
    );
    assert!(failed.is_err());
    store.collect().unwrap();
    assert!(store.hold(&a).is_ok());
    assert!(store.hold(&b).is_ok());
}

#[test]
fn a_corrupted_inactive_program_is_retained_without_blocking_a_valid_selection() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let first = image(&store, "one", b"first");
    let old = id(&first);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(first.path(), fs::Permissions::from_mode(0o600)).unwrap();
    }
    fs::write(first.path(), b"corrupt").unwrap();
    drop(first);
    drop(image(&store, "two", b"second"));
    drop(image(&store, "three", b"third"));
    assert!(store.collect().unwrap().unknown.contains(&old));
    assert!(fixture.0.join(old).is_dir());
}

#[test]
fn interrupted_collection_and_publication_are_finished_without_trusting_pids() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let first = image(&store, "one", b"first");
    let a = id(&first);
    let program = first.path().file_name().unwrap().to_owned();
    drop(first);
    let manifest = fs::read(fixture.0.join(&a).join("manifest.json")).unwrap();
    let staging = fixture.0.join(".publish-interrupted");
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join("transaction.json"), &manifest).unwrap();
    fs::write(staging.join(&program), b"partial").unwrap();
    let garbage = fixture.0.join(format!(".gc-{a}"));
    fs::rename(fixture.0.join(&a), &garbage).unwrap();
    fs::remove_file(garbage.join(program)).unwrap();
    store.collect().unwrap();
    assert!(!staging.exists());
    assert!(!garbage.exists());
}

#[cfg(unix)]
#[test]
fn unknown_symlinks_are_never_followed_for_collection() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let elsewhere = Fixture::new();
    let other = elsewhere.store();
    let protected = image(&other, "other", b"foreign");
    let fake = fixture.0.join("a".repeat(64));
    std::os::unix::fs::symlink(protected.path().parent().unwrap(), &fake).unwrap();
    assert!(store.collect().unwrap().unknown.contains(&"a".repeat(64)));
    assert_eq!(fs::read(protected.path()).unwrap(), b"foreign");
}

fn wait_for(mut condition: impl FnMut() -> bool) {
    let by = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(
            Instant::now() < by,
            "owned runtime fixture did not reach its barrier"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn inherited_child_lease_survives_parent_process_exit_and_is_reclaimed_after_child_exit() {
    let fixture = Fixture::new();
    let _store = fixture.store();
    let mut parent = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "tests::lease_parent_process", "--nocapture"])
        .env("KHOR_RUNTIME_CHECK_ROOT", &fixture.0)
        .spawn()
        .unwrap();
    wait_for(|| fixture.0.join("child.ready").exists());
    assert!(parent.wait().unwrap().success());
    let store = fixture.store();
    let old = fs::read_to_string(fixture.0.join("selected.id")).unwrap();
    drop(image(&store, "two", b"second"));
    drop(image(&store, "three", b"third"));
    assert!(store.collect().unwrap().occupied.contains(&old));
    fs::write(fixture.0.join("child.release"), b"exit").unwrap();
    wait_for(|| {
        store.collect().unwrap();
        !fixture.0.join(&old).exists()
    });
}

#[test]
fn lease_parent_process() {
    let Some(root) = std::env::var_os("KHOR_RUNTIME_CHECK_ROOT") else {
        return;
    };
    let store = Store::open(
        root.clone().into(),
        Limits {
            cache_bytes: 512 * 1024 * 1024,
            free_bytes: 0,
        },
    )
    .unwrap();
    let current = std::env::current_exe().unwrap();
    let selected = store
        .publish(
            Role::Cli,
            "process-fixture",
            contract(),
            BTreeMap::new(),
            Source::File(&current),
        )
        .unwrap();
    fs::write(PathBuf::from(&root).join("selected.id"), id(&selected)).unwrap();
    let mut child = Command::new(selected.path());
    child
        .args(["--exact", "tests::lease_child_process", "--nocapture"])
        .env("KHOR_RUNTIME_CHECK_CHILD", &root)
        .env_remove("KHOR_RUNTIME_CHECK_ROOT");
    selected.lease().attach_host(&mut child).unwrap();
    child.spawn().unwrap();
    wait_for(|| PathBuf::from(&root).join("child.ready").exists());
    // Deliberately skip Rust destructors: occupancy must live in the child, not a parent callback.
    std::process::exit(0);
}

#[test]
fn lease_child_process() {
    let Some(root) = std::env::var_os("KHOR_RUNTIME_CHECK_CHILD") else {
        return;
    };
    let root = PathBuf::from(root);
    enter_current().unwrap();
    #[cfg(unix)]
    {
        let raw: i32 = std::env::var("KHOR_RUNTIME_LEASE_FD")
            .unwrap()
            .parse()
            .unwrap();
        assert_ne!(
            unsafe { libc::fcntl(raw, libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
    }
    fs::write(root.join("child.ready"), b"ready").unwrap();
    wait_for(|| root.join("child.release").exists());
}
