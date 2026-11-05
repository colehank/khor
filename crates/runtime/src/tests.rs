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
fn live_contracts_follow_kernel_occupancy_while_idle_rollback_remains_available() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let first = image(&store, "one", b"first");
    let old = id(&first);
    let lease = first.lease().try_clone().unwrap();
    drop(first);
    assert_eq!(store.occupied_contracts().unwrap(), vec![contract()]);
    drop(lease);
    assert!(store.occupied_contracts().unwrap().is_empty());
    assert_eq!(store.contracts().unwrap(), vec![contract()]);
    let selected = store.hold(&old).unwrap();
    assert_eq!(store.occupied_contracts().unwrap(), vec![contract()]);
    drop(selected);
    assert!(store.occupied_contracts().unwrap().is_empty());
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
fn usage_counts_retained_and_live_old_bytes_once_and_cleanup_only_releases_idle_images() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let old = image(&store, "one", b"first");
    let lease = old.lease().try_clone().unwrap();
    drop(old);
    drop(image(&store, "two", b"second"));
    drop(image(&store, "three", b"third"));
    let usage = store.usage_all().unwrap();
    assert_eq!(usage.retained_bytes, 11);
    assert_eq!(usage.active_old_bytes, 5);
    assert_eq!(usage.idle_bytes, 0);
    store.collect_all().unwrap();
    assert_eq!(store.usage_all().unwrap().active_old_bytes, 5);
    drop(lease);
    assert_eq!(store.usage_all().unwrap().idle_bytes, 5);
    store.collect_all().unwrap();
    let usage = store.usage_all().unwrap();
    assert_eq!(usage.retained_bytes, 11);
    assert_eq!(usage.active_old_bytes + usage.idle_bytes, 0);
}

#[test]
fn node_collection_reaches_registered_cli_stores_and_preserves_live_and_rollback_images() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let parent = fixture
        .0
        .join("cli")
        .join("a".repeat(64))
        .join("khor-macos-arm64");
    let mut directories = fs::DirBuilder::new();
    directories.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directories.mode(0o700);
    }
    directories.create(&parent).unwrap();
    let child = Store::open(
        parent.join("versions"),
        Limits {
            free_bytes: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    let first = image(&child, "one", b"first");
    let old = id(&first);
    let lease = first.lease().try_clone().unwrap();
    drop(first);
    let second = image(&child, "two", b"second");
    let rollback = second.path().to_path_buf();
    drop(second);
    let third = image(&child, "three", b"third");
    let current = third.path().to_path_buf();
    drop(third);
    assert!(store.collect_all().unwrap().occupied.contains(&old));
    assert!(rollback.exists() && current.exists());
    drop(lease);
    assert!(store.collect_all().unwrap().removed.contains(&old));
    assert!(rollback.exists() && current.exists());
    let unowned = parent.join("unclaimed");
    fs::write(&unowned, b"personal data").unwrap();
    store.collect_all().unwrap();
    assert_eq!(fs::read(unowned).unwrap(), b"personal data");
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
        let flags = unsafe { libc::fcntl(raw, libc::F_GETFD) };
        assert!(
            flags >= 0,
            "the inherited descriptor must remain open in this process"
        );
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
    }
    fs::write(root.join("child.ready"), b"ready").unwrap();
    wait_for(|| root.join("child.release").exists());
}

#[cfg(unix)]
#[test]
#[ignore = "requires the real standalone Bun agent via KHOR_TEST_AGENT_BIN"]
fn actual_standalone_agent_keeps_its_image_lease_after_initialization() {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::process::CommandExt,
        process::{Child, Stdio},
        sync::mpsc,
        thread::JoinHandle,
    };
    struct OwnedAgent {
        child: Child,
        reader: Option<JoinHandle<()>>,
    }
    impl Drop for OwnedAgent {
        fn drop(&mut self) {
            // This unreaped leader owns its explicitly created process group.
            unsafe {
                libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
            }
            let _ = self.child.wait();
            if let Some(reader) = self.reader.take() {
                reader.join().unwrap();
            }
        }
    }
    let program = PathBuf::from(
        std::env::var_os("KHOR_TEST_AGENT_BIN").expect("real compiled agent required"),
    );
    let fixture = Fixture::new();
    let store = fixture.store();
    let credentials = Fixture::new();
    fs::create_dir(&credentials.0).unwrap();
    let image = store
        .publish(
            Role::Agent,
            "standalone-agent",
            contract(),
            BTreeMap::new(),
            Source::File(&program),
        )
        .unwrap();
    let old = id(&image);
    let mut command = Command::new(image.path());
    command
        .process_group(0)
        .env_clear()
        .env("HOME", &credentials.0)
        .env("USERPROFILE", &credentials.0)
        .current_dir(&credentials.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    image.lease().attach_unix(&mut command).unwrap();
    let mut agent = OwnedAgent {
        child: command.spawn().unwrap(),
        reader: None,
    };
    drop(command); // Only the initialized child may retain the command's inherited descriptor.
    let stdout = agent.child.stdout.take().unwrap();
    let (sent, received) = mpsc::sync_channel(1);
    agent.reader = Some(std::thread::spawn(move || {
        let mut offered = false;
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else {
                break;
            };
            if line.len() > 16 * 1024 {
                break;
            }
            if !offered
                && serde_json::from_str::<serde_json::Value>(&line)
                    .is_ok_and(|frame| frame["type"] == "auth_prompt")
            {
                let _ = sent.send(());
                offered = true;
            }
        }
    }));
    let mut frame = serde_json::to_vec(&serde_json::json!({"type":"manage","operation":"login","account":{"id":"lease-fixture","provider":"anthropic","baseUrl":null},"credentialPath":credentials.0.join("auth.json"),"authType":"api_key","deviceId":"0".repeat(64)})).unwrap();
    frame.push(b'\n');
    agent
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&frame)
        .unwrap();
    received
        .recv_timeout(Duration::from_secs(15))
        .expect("compiled agent must finish initialization and wait for an API key");
    drop(image);
    drop(
        store
            .publish(
                Role::Agent,
                "two",
                contract(),
                BTreeMap::new(),
                Source::Bytes(b"second"),
            )
            .unwrap(),
    );
    drop(
        store
            .publish(
                Role::Agent,
                "three",
                contract(),
                BTreeMap::new(),
                Source::Bytes(b"third"),
            )
            .unwrap(),
    );
    assert!(
        store.collect().unwrap().occupied.contains(&old),
        "initialized Bun must retain its own runtime image"
    );
    drop(agent);
    assert!(store.collect().unwrap().removed.contains(&old));
}

#[test]
fn durable_reservation_preserves_the_pending_image_and_both_existing_selections() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let previous = image(&store, "previous", b"previous");
    let previous_path = previous.path().to_path_buf();
    drop(previous);
    let current = image(&store, "current", b"current");
    let current_path = current.path().to_path_buf();
    drop(current);
    let pending = store
        .stage_checked(
            Role::Cli,
            "pending",
            contract(),
            BTreeMap::new(),
            Source::Bytes(b"pending"),
            |_| Ok(()),
        )
        .unwrap();
    let candidate = pending.path().to_path_buf();
    let key = pending.id().unwrap();
    store.reserve(&pending).unwrap();
    store.reserve(&pending).unwrap();
    let other = Fixture::new();
    assert!(other.store().reserve(&pending).is_err());
    drop(pending);
    drop(store);
    let store = fixture.store();
    store.collect().unwrap();
    assert!(candidate.exists());
    assert!(current_path.exists());
    assert!(previous_path.exists());
    assert_eq!(store.usage_all().unwrap().idle_bytes, 0);
    store.release_reservation(&key).unwrap();
    store.collect().unwrap();
    assert!(!candidate.exists());
    assert!(current_path.exists() && previous_path.exists());
}

#[test]
fn unselected_candidates_cannot_evict_current_or_rollback_and_other_stores_cannot_select_them() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let previous = image(&store, "previous", b"previous");
    let previous_path = previous.path().to_path_buf();
    drop(previous);
    let current = image(&store, "current", b"current");
    let current_path = current.path().to_path_buf();
    drop(current);
    for attempt in 0..4 {
        let candidate = store
            .stage_checked(
                Role::Cli,
                &format!("attempt-{attempt}"),
                contract(),
                BTreeMap::new(),
                Source::Bytes(b"candidate"),
                |_| Ok(()),
            )
            .unwrap();
        assert!(previous_path.exists());
        assert!(current_path.exists());
        let other = Fixture::new();
        assert!(other.store().select(&candidate).is_err());
        drop(candidate);
        store.collect().unwrap();
    }
    let ready = store
        .stage_checked(
            Role::Cli,
            "ready",
            contract(),
            BTreeMap::new(),
            Source::Bytes(b"ready"),
            |_| Ok(()),
        )
        .unwrap();
    store.select(&ready).unwrap();
    assert!(current_path.exists());
    assert!(!previous_path.exists());
    let held = Store::image_for_program(ready.path()).unwrap().unwrap();
    assert_eq!(held.path(), ready.path());
}

#[test]
fn compatibility_accounts_for_occupied_node_installations_without_treating_idle_copies_as_live() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let installation = fixture
        .0
        .join("cli")
        .join("a".repeat(64))
        .join("khor-macos-arm64");
    fs::create_dir_all(&installation).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [
            fixture.0.join("cli"),
            installation.parent().unwrap().to_path_buf(),
            installation.clone(),
        ] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }
    let child = Store::open(
        installation.join("versions"),
        Limits {
            cache_bytes: 1024 * 1024,
            free_bytes: 0,
        },
    )
    .unwrap();
    let live = image(&child, "node-local", b"local CLI");
    assert_eq!(store.occupied_contracts().unwrap(), vec![contract()]);
    drop(live);
    assert!(store.occupied_contracts().unwrap().is_empty());
}
