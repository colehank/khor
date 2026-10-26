use super::*;
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "khor-bindings-test-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SERIAL.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn output(&self) -> PathBuf {
        self.0.join("bindings")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn paths(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| (*s).into()).collect()
}
fn emit(dir: &Path, name: &str, body: &str) -> Result<()> {
    fs::write(dir.join(name), format!("{HEADER}{body}\n")).map_err(|e| e.to_string())
}
fn simple(dir: &Path, body: &str) -> Result<()> {
    generate(dir, false, &paths(&["Current.ts"]), |stage| {
        emit(stage, "Current.ts", body)
    })
}
fn bytes(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    entries(dir)
        .unwrap()
        .into_iter()
        .map(|(name, file)| (name, fs::read(file).unwrap()))
        .collect()
}

#[test]
fn actual_exports_match_fresh_output_after_a_managed_obsolete_generation() {
    let scratch = Scratch::new();
    let fresh = scratch.0.join("fresh");
    let reused = scratch.0.join("reused");
    let planned = crate::planned_files().unwrap();
    generate(&fresh, false, &planned, crate::export).unwrap();
    generate(&reused, false, &planned, crate::export).unwrap();
    let mut old = inspect(&reused, false).unwrap().inventory;
    for name in ["PlaceRow.ts", "QuotaWindowKind.ts", "Wheel.ts"] {
        assert!(
            !planned.contains(name),
            "fixture must be obsolete in the current roots"
        );
        emit(&reused, name, "export type PreviousGeneration = string;").unwrap();
        old.files.insert(
            name.into(),
            blake3::hash(&fs::read(reused.join(name)).unwrap())
                .to_hex()
                .to_string(),
        );
    }
    // These really represent a prior *managed* generation; an unlisted file
    // must be rejected rather than silently treated as generated authority.
    fs::remove_file(reused.join(INVENTORY)).unwrap();
    write_json(&reused.join(INVENTORY), &old).unwrap();
    generate(&reused, false, &planned, crate::export).unwrap();
    assert_eq!(bytes(&fresh), bytes(&reused));
    assert_eq!(
        inspect(&reused, false).unwrap().inventory.files.len(),
        planned.len()
    );
}

#[test]
fn partial_export_failure_keeps_the_previous_output_and_catalog_unchanged() {
    let scratch = Scratch::new();
    let output = scratch.output();
    simple(&output, "export type Current = string;").unwrap();
    fs::write(scratch.0.join("catalog.ts"), b"independent catalog").unwrap();
    let previous = bytes(&output);
    let error = generate(
        &output,
        false,
        &paths(&["Current.ts", "Second.ts"]),
        |stage| {
            emit(stage, "Current.ts", "export type Current = number;")?;
            Err("injected export IO failure".into())
        },
    )
    .unwrap_err();
    assert!(error.contains("injected export IO failure"));
    assert_eq!(bytes(&output), previous);
    assert_eq!(
        fs::read(scratch.0.join("catalog.ts")).unwrap(),
        b"independent catalog"
    );
    assert!(!exists(&Layout::new(&output).unwrap().journal).unwrap());
    assert!(
        fs::read_dir(&scratch.0).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".stage-"))
    );
}

#[test]
fn an_over_budget_plan_never_exports_or_changes_the_previous_output_or_journal() {
    let scratch = Scratch::new();
    let output = scratch.output();
    simple(&output, "export type Current = string;").unwrap();
    let previous = bytes(&output);
    let layout = Layout::new(&output).unwrap();
    let planned = (0..=MAX_FILES).map(|n| format!("Type{n}.ts")).collect();
    for journal in [None, Some(b"retained publication record".as_slice())] {
        if let Some(contents) = journal {
            fs::write(&layout.journal, contents).unwrap();
        }
        let siblings: BTreeSet<_> = fs::read_dir(&scratch.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        let error = generate(&output, false, &planned, |_| {
            panic!("an over-budget plan must be rejected before export")
        })
        .unwrap_err();
        assert_eq!(error, "too many planned bindings");
        assert_eq!(bytes(&output), previous);
        assert_eq!(fs::read(&layout.journal).ok().as_deref(), journal);
        assert_eq!(
            fs::read_dir(&scratch.0)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<BTreeSet<_>>(),
            siblings
        );
    }
}

#[test]
fn an_over_budget_default_directory_is_rejected_before_export_or_publication() {
    let scratch = Scratch::new();
    let output = scratch.output();
    fs::create_dir(&output).unwrap();
    for n in 0..=MAX_FILES {
        emit(&output, &format!("Type{n}.ts"), "export type Old = string;").unwrap();
    }
    let previous = bytes(&output);
    assert_eq!(previous.len(), MAX_FILES + 1);
    let error = generate(&output, true, &paths(&["Current.ts"]), |_| {
        panic!("an over-budget default directory must be rejected before export")
    })
    .unwrap_err();
    assert_eq!(error, "unknown bindings inventory");
    assert_eq!(bytes(&output), previous);
    assert!(!output.join(INVENTORY).exists());
    let layout = Layout::new(&output).unwrap();
    assert!(!layout.journal.exists());
    assert_eq!(
        fs::read_dir(&scratch.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            output.file_name().unwrap().to_owned(),
            layout.lock.file_name().unwrap().to_owned()
        ])
    );
}

#[test]
fn unmanaged_output_requires_default_generated_ownership_and_rejects_unknown_files() {
    let scratch = Scratch::new();
    let output = scratch.output();
    fs::create_dir(&output).unwrap();
    emit(&output, "Obsolete.ts", "export type Obsolete = number;").unwrap();
    let previous = bytes(&output);
    assert!(simple(&output, "export type Current = string;").is_err());
    assert_eq!(bytes(&output), previous);
    fs::write(output.join("notes.txt"), b"person's file").unwrap();
    assert!(
        generate(&output, true, &paths(&["Current.ts"]), |stage| emit(
            stage,
            "Current.ts",
            "export type Current = string;"
        ))
        .is_err()
    );
    assert_eq!(
        fs::read(output.join("notes.txt")).unwrap(),
        b"person's file"
    );
    assert!(output.join("Obsolete.ts").exists());
    fs::remove_file(output.join("notes.txt")).unwrap();
    generate(&output, true, &paths(&["Current.ts"]), |stage| {
        emit(stage, "Current.ts", "export type Current = string;")
    })
    .unwrap();
    assert!(!output.join("Obsolete.ts").exists());
    assert!(inspect(&output, false).unwrap().managed);
    emit(&output, "Unlisted.ts", "export type Unlisted = string;").unwrap();
    let changed = bytes(&output);
    assert!(
        generate(&output, true, &paths(&["Current.ts"]), |_| {
            panic!("a marker cannot authorize an unlisted generated file")
        })
        .is_err()
    );
    assert_eq!(bytes(&output), changed);
}

#[test]
fn unknown_partial_stage_and_backup_entries_are_preserved_before_cleanup() {
    let scratch = Scratch::new();
    let output = scratch.output();
    simple(&output, "export type Current = string;").unwrap();
    let previous = bytes(&output);
    let mut stage_path = None;
    let error = generate(&output, false, &paths(&["Current.ts"]), |stage| {
        stage_path = Some(stage.to_path_buf());
        emit(stage, "Current.ts", "export type Current = number;")?;
        emit(stage, "Unknown.ts", "export type Unknown = string;")?;
        Err("partial export failed".into())
    })
    .unwrap_err();
    assert!(error.contains("stage retained"));
    let stage = stage_path.unwrap();
    assert!(stage.join("Current.ts").exists() && stage.join("Unknown.ts").exists());
    assert_eq!(bytes(&output), previous);

    let layout = Layout::new(&output).unwrap();
    let failure = generate_with(
        &output,
        false,
        &paths(&["Current.ts"]),
        |stage| emit(stage, "Current.ts", "export type Current = number;"),
        || {
            let journal = layout.journal()?;
            fs::write(
                layout.parent.join(journal.backup).join("person.txt"),
                b"untouched",
            )
            .map_err(|e| e.to_string())?;
            Err("publication failed".into())
        },
    );
    assert!(failure.is_err());
    let journal = layout.journal().unwrap();
    let backup = layout.parent.join(journal.backup);
    assert_eq!(
        fs::read(backup.join("Current.ts")).unwrap(),
        previous["Current.ts"]
    );
    assert_eq!(fs::read(backup.join("person.txt")).unwrap(), b"untouched");
    assert!(backup.join(INVENTORY).exists());
}

#[test]
fn publication_error_restores_the_complete_previous_directory() {
    let scratch = Scratch::new();
    let output = scratch.output();
    simple(&output, "export type Current = string;").unwrap();
    let previous = bytes(&output);
    let result = generate_with(
        &output,
        false,
        &paths(&["Current.ts"]),
        |stage| emit(stage, "Current.ts", "export type Current = number;"),
        || Err("injected publication failure".into()),
    );
    assert!(result.unwrap_err().contains("previous bindings restored"));
    assert_eq!(bytes(&output), previous);
    assert!(!exists(&Layout::new(&output).unwrap().journal).unwrap());
    simple(&output, "export type Current = number;").unwrap();
}

#[cfg(unix)]
#[test]
fn static_symlinks_never_change_the_outside_target() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    let outside = scratch.0.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("sentinel"), b"outside").unwrap();
    let output = scratch.output();
    symlink(&outside, &output).unwrap();
    assert!(simple(&output, "ignored").is_err());
    fs::remove_file(&output).unwrap();
    fs::create_dir(&output).unwrap();
    symlink(outside.join("sentinel"), output.join("Current.ts")).unwrap();
    assert!(
        generate(&output, true, &paths(&["Current.ts"]), |_| panic!(
            "must reject before export"
        ))
        .is_err()
    );
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"outside");
    fs::remove_file(output.join("Current.ts")).unwrap();
    let layout = Layout::new(&output).unwrap();
    symlink(outside.join("sentinel"), &layout.journal).unwrap();
    assert!(simple(&output, "ignored").is_err());
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"outside");
    let parent = scratch.0.join("parent-link");
    symlink(&outside, &parent).unwrap();
    assert!(simple(&parent.join("bindings"), "ignored").is_err());
    assert!(!outside.join("bindings").exists());
}

#[test]
fn dependency_paths_are_checked_before_any_export_can_escape_the_stage() {
    #[allow(dead_code)]
    #[derive(ts_rs::TS)]
    #[ts(export_to = "../Escaped.ts")]
    struct Escaped {
        value: String,
    }
    #[allow(dead_code)]
    #[derive(ts_rs::TS)]
    struct Root {
        nested: Escaped,
    }
    let mut visitor = crate::Paths::default();
    ts_rs::TypeVisitor::visit::<Root>(&mut visitor);
    assert!(visitor.error.is_some());
    for name in [
        "../Escape.ts",
        "/tmp/Escape.ts",
        "sub/Type.ts",
        "C:\\Escape.ts",
        "./Type.ts",
        "Type.txt",
    ] {
        assert!(type_name(Path::new(name)).is_err(), "{name}");
    }
    assert_eq!(type_name(Path::new("Current.ts")).unwrap(), "Current.ts");
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn process(root: &Path, case: &str) -> Process {
    Process(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "managed::tests::bindings_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("KHOR_BINDINGS_TEST_ROOT", root)
            .env("KHOR_BINDINGS_TEST_CASE", case)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    )
}
#[test]
#[ignore = "owned child actually launched by the ordinary lock and crash tests"]
fn bindings_worker() {
    // Test-binary-only hooks: production gen_bindings has no fault env switch.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(20));
        std::process::exit(124);
    });
    let root = PathBuf::from(std::env::var_os("KHOR_BINDINGS_TEST_ROOT").unwrap());
    let output = root.join("bindings");
    match std::env::var("KHOR_BINDINGS_TEST_CASE").unwrap().as_str() {
        "busy" => {
            let error = simple(&output, "not admitted").unwrap_err();
            assert!(error.contains("busy"), "{error}");
            fs::write(
                root.join("busy-confirmed"),
                b"actual OS lock refused this process",
            )
            .unwrap();
        }
        "crash" => {
            generate_with(
                &output,
                false,
                &paths(&["Current.ts"]),
                |stage| emit(stage, "Current.ts", "export type Current = number;"),
                || {
                    fs::write(root.join("backup-ready"), b"old directory really moved")
                        .map_err(|e| e.to_string())?;
                    loop {
                        std::thread::park();
                    }
                },
            )
            .unwrap();
            panic!("crash fixture unexpectedly returned");
        }
        other => panic!("unknown worker case {other}"),
    }
}

#[test]
fn another_process_cannot_publish_while_the_real_output_lock_is_held() {
    let scratch = Scratch::new();
    let output = scratch.output();
    simple(&output, "export type Current = string;").unwrap();
    let previous = bytes(&output);
    let layout = Layout::new(&output).unwrap();
    let held = layout.lock().unwrap();
    let mut worker = process(&scratch.0, "busy");
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < until, "lock worker deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(scratch.0.join("busy-confirmed").is_file());
    assert_eq!(bytes(&output), previous);
    drop(held);
    simple(&output, "export type Current = number;").unwrap();
}

#[cfg(unix)]
#[test]
fn a_real_kill_after_the_old_directory_moves_recovers_before_the_next_export() {
    use std::os::unix::process::ExitStatusExt;
    let scratch = Scratch::new();
    let output = scratch.output();
    simple(&output, "export type Current = string;").unwrap();
    let previous = bytes(&output);
    let mut worker = process(&scratch.0, "crash");
    let until = Instant::now() + Duration::from_secs(10);
    while !scratch.0.join("backup-ready").exists() {
        assert!(
            worker.0.try_wait().unwrap().is_none(),
            "worker exited before moving old output"
        );
        assert!(Instant::now() < until, "crash worker deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!output.exists());
    let layout = Layout::new(&output).unwrap();
    let journal = layout.journal().unwrap();
    assert_eq!(bytes(&layout.parent.join(&journal.backup)), previous);
    worker.0.kill().unwrap();
    let status = worker.0.wait().unwrap();
    assert_eq!(
        status.signal(),
        Some(9),
        "must actually SIGKILL the publication worker"
    );
    let error = generate(&output, false, &paths(&["Current.ts"]), |_| {
        Err("stop after recovery".into())
    })
    .unwrap_err();
    assert!(error.contains("stop after recovery"));
    assert_eq!(bytes(&output), previous);
    assert!(!layout.journal.exists());
    simple(&output, "export type Current = number;").unwrap();
}
