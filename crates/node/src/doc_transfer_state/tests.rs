use super::*;
use khor_sync::pins::{PinDoc, pins_dir};
use khor_sync::transfer::CHUNK_BYTES;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "khor-doc-state-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn context(&self, label: &str, identity: u8) -> Context {
        let root = self.0.join(label);
        fs::create_dir_all(root.join(".khor")).unwrap();
        Context {
            root,
            writer: identity as u64 + 90000,
            local: [identity; 32],
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn state<T>(context: &Context, run: impl FnOnce(&mut State) -> T) -> T {
    let mut state = State::open(context).unwrap();
    run(&mut state)
}
fn write(context: &Context, session: &str) {
    let mut loaded =
        khor_sync::store::load_pile::<PinDoc>(&pins_dir(&context.root), context.writer).unwrap();
    loaded.doc.set(session, true).unwrap();
    loaded.store.flush(&loaded.doc).unwrap();
}
fn first(source: &Context, receiver: &Context) -> DocChunk {
    let have = receiver.version("pins").unwrap();
    state(source, |s| {
        s.offer(source, receiver.local, "pins", &have)
            .unwrap()
            .unwrap()
    })
}
fn receive(receiver: &Context, source: &Context, chunk: &DocChunk) -> DocProof {
    state(receiver, |s| {
        match s.receive(receiver, source.local, chunk, true).unwrap() {
            DocTransferState::Complete { proof } => proof,
            other => panic!("expected complete small real Loro import: {other:?}"),
        }
    })
}

#[test]
fn checked_import_receipt_survives_reopen_and_both_sides_clean_exact_stages() {
    let _serial = super::io::TEST_SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new();
    let a = fixture.context("a", 11);
    let b = fixture.context("b", 12);
    write(&a, "tui/receipt");
    write(&b, "tui/local-concurrent");
    let chunk = first(&a, &b);
    let proof = receive(&b, &a, &chunk);
    assert_eq!(proof.kind, DocProofKind::Applied);
    state(&b, |s| {
        assert!(
            matches!(s.status(&b,a.local,&chunk.manifest).unwrap(),DocTransferState::Complete{proof:again} if again==proof)
        )
    });
    let target = chunk.manifest.descriptor().unwrap().target;
    assert!(b.version("pins").unwrap().version.includes_vv(&target));
    {
        let loaded = khor_sync::store::load_pile::<PinDoc>(&pins_dir(&b.root), b.writer).unwrap();
        assert!(loaded.doc.pinned("tui/receipt"));
        assert!(loaded.doc.pinned("tui/local-concurrent"));
    }
    let ack = state(&a, |s| s.accept(&a, b.local, &proof, false).unwrap());
    state(&a, |s| {
        assert!(s.blobs.retained_manifests().unwrap().is_empty())
    });
    state(&b, |s| {
        assert_eq!(s.blobs.retained_manifests().unwrap().len(), 1)
    });
    let result = state(&b, |s| s.acknowledge(a.local, &ack).unwrap());
    state(&a, |s| s.acknowledged(&a, b.local, &result).unwrap());
    state(&a, |s| assert!(s.records.outgoing.is_empty()));
    state(&b, |s| {
        assert!(s.records.incoming.is_empty());
        assert!(s.blobs.retained_manifests().unwrap().is_empty());
        assert_eq!(
            s.receive(&b, a.local, &chunk, false).unwrap(),
            DocTransferState::Missing
        );
        assert!(
            s.records.incoming.is_empty(),
            "late Write must not recreate a receipt/stage"
        );
    });
}

#[test]
fn an_offered_target_stays_fixed_after_late_local_ops_and_owner_reopen() {
    let _serial = super::io::TEST_SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new();
    let a = fixture.context("a", 21);
    let b = fixture.context("b", 22);
    {
        let mut loaded =
            khor_sync::store::load_pile::<PinDoc>(&pins_dir(&a.root), a.writer).unwrap();
        for i in 0u64..6000 {
            loaded
                .doc
                .set(
                    &format!("tui/{i}/{}", blake3::hash(&i.to_be_bytes()).to_hex()),
                    true,
                )
                .unwrap();
        }
        loaded.store.flush(&loaded.doc).unwrap();
    }
    let offered = first(&a, &b);
    assert!(offered.manifest.total_bytes > CHUNK_BYTES as u64);
    let fixed = offered.manifest.descriptor().unwrap().target;
    write(&a, "tui/late-after-plan");
    assert_ne!(a.version("pins").unwrap().version, fixed);
    let reopened = first(&a, &b);
    assert_eq!(
        reopened, offered,
        "same body/manifest/target survives state and core owner drop"
    );
    state(&a, |s| {
        let tail = s
            .read_outgoing(&a, b.local, &offered.manifest, CHUNK_BYTES as u64)
            .unwrap();
        assert_eq!(tail.manifest, offered.manifest);
        assert!(!tail.bytes.is_empty());
    });
    assert!(
        !b.version("pins").unwrap().version.includes_vv(&fixed),
        "reading a spool is not Doc Applied"
    );
}

#[test]
fn a_recreated_equivalent_receipt_reconfirms_old_ack_without_deleting_new_ids() {
    let _serial = super::io::TEST_SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new();
    let a = fixture.context("a", 31);
    let b = fixture.context("b", 32);
    write(&a, "tui/old");
    let old = first(&a, &b);
    let proof = receive(&b, &a, &old);
    let ack = state(&a, |s| s.accept(&a, b.local, &proof, false).unwrap());
    // Receiver cleans, but the Released reply is lost. Sender keeps NeedAck.
    state(&b, |s| {
        s.acknowledge(a.local, &ack).unwrap();
    });
    let equivalent = receive(&b, &a, &old);
    assert_eq!(equivalent.kind, DocProofKind::AlreadyHaveEquivalent);
    let reconfirm = state(&b, |s| s.acknowledge(a.local, &ack).unwrap());
    assert!(matches!(&reconfirm,DocAckResult::Reconfirm{proof} if proof==&equivalent));
    state(&a, |s| s.acknowledged(&a, b.local, &reconfirm).unwrap());
    let new_ack = state(&a, |s| {
        s.pending_acknowledgements(b.local, 1)
            .unwrap()
            .pop()
            .unwrap()
    });
    let released = state(&b, |s| s.acknowledge(a.local, &new_ack).unwrap());
    state(&a, |s| s.acknowledged(&a, b.local, &released).unwrap());
    write(&a, "tui/new-generation");
    let new = first(&a, &b);
    assert_ne!(new.manifest, old.manifest);
    receive(&b, &a, &new);
    state(&b, |s| {
        assert!(matches!(
            s.acknowledge(a.local, &ack).unwrap(),
            DocAckResult::Absent { .. }
        ));
        assert!(
            s.blobs
                .retained_manifests()
                .unwrap()
                .iter()
                .any(|m| DocManifest::from(m) == new.manifest)
        );
    });
    // A duplicate Confirm after sender GC only reissues this exact old ACK.
    state(&a, |s| {
        s.accept(&a, b.local, &equivalent, true).unwrap();
        assert!(
            s.records
                .outgoing
                .iter()
                .any(|r| r.manifest() == &new.manifest)
        );
    });
}

#[test]
fn wrong_authenticated_direction_is_rejected_before_staging_or_doc_mutation() {
    let _serial = super::io::TEST_SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new();
    let a = fixture.context("a", 41);
    let b = fixture.context("b", 42);
    write(&a, "tui/binding");
    let chunk = first(&a, &b);
    let before = b.version("pins").unwrap();
    state(&b, |s| {
        assert!(s.receive(&b, [99; 32], &chunk, true).is_err());
        let mut wrong = chunk.clone();
        wrong.manifest.recipient = [98; 32];
        assert!(s.receive(&b, a.local, &wrong, true).is_err());
        assert!(s.records.incoming.is_empty());
        assert!(s.blobs.retained_manifests().unwrap().is_empty());
    });
    assert_eq!(b.version("pins").unwrap(), before);
}
