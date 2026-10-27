//! The agent registry over a real wire (批⑥; 账本: 网络类改动必须真连,
//! 还要有对照组). Two nodes on one machine, real UDP: a registration made
//! on one is offerable on the other, and forgetting it travels back.
//!
//! # Why this is not in the registry's own unit test
//!
//! `khor_sync::agents` merges two documents in one process, which proves
//! the CRDT and nothing about the transport. The document is named by a
//! **string** in two places on this side of the fence — the serve's
//! `Request::DocTransfer` dispatch and the sync round — and a name that is
//! wrong, or missing from one of them, would leave every in-process test
//! green while no registration ever travelled. The whole reason this is
//! a document rather than a preference file is the sentence "say it
//! once, true on every machine", and that sentence is exactly what only
//! a wire can check.
//!
//! # Its own file, with its own clock (issue #73)
//!
//! On a Mac operated over ssh a freshly compiled test binary is a new
//! face to macOS, and its first network-stack touch (interface
//! enumeration, SystemConfiguration) hangs 20-40s per process waiting on
//! an authorization prompt nobody can ever click. `transfer.rs` already
//! carries 45-90s deadlines for it; `pairing.rs` and `files_wire.rs`
//! still carry 10-20s ones and time out here for that reason and no
//! other (checked 2026-08-21 01:35 — removing this batch's own sync
//! round left them red exactly as before).
//!
//! So the deadlines here are transfer.rs's, and the runtime is
//! multi-thread for transfer.rs's other reason: that stall is
//! synchronous, and on a current-thread runtime it freezes tokio's clock
//! so every timeout in the file silently stretches with it.

mod util;

use std::path::Path;
use std::time::Duration;

use khor_node::Node;
use tokio::time::timeout;

/// A scratch tree that **takes itself away however the test leaves**
/// (`crates/node/src/tests.rs::Scratch` has the whole reasoning: a
/// cleanup written at the end of a test is skipped by the assertion
/// that fails, so a red run leaks a directory every time it is red).
/// A copy per file, which is this repo's convention for test helpers —
/// it costs no dev-dependency.
struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;

    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-agentwire-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

/// **A registered agent really crosses the wire, and forgetting it
/// crosses back.**
///
/// The assertion *before* the sync is the load-bearing half. Without it
/// this passes on two nodes that shared a store, on a sync that did
/// nothing, and on a registry that was somehow global — every way of
/// being wrong except the one it is checking.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_registered_agent_reaches_the_other_machine() {
    let ra = root("a");
    let rb = root("b");

    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    timeout(Duration::from_secs(45), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    b.register_agent("gemini", &["gemini".to_owned(), "--acp".to_owned()])
        .unwrap();
    assert_eq!(
        a.agent("gemini").unwrap(),
        None,
        "control: alpha has not heard of it before a sync — without this line \
         the test proves nothing about the wire"
    );

    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    let landed = a
        .agent("gemini")
        .unwrap()
        .expect("the registration reached alpha");
    assert_eq!(
        landed.typed(),
        "gemini --acp",
        "and it arrived whole — a command with its arguments, not just a name"
    );

    // Forgetting travels too. A registry that only ever grew would leave
    // every other machine offering an agent the person removed.
    b.forget_agent("gemini").unwrap();
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    assert_eq!(
        a.agent("gemini").unwrap(),
        None,
        "forgetting reaches alpha too"
    );

    serve.abort();
    let _ = std::fs::remove_dir_all(&*ra);
}

/// The general stream limit must not admit a padded DocTransfer to its state
/// handler. This uses only the existing public wire so candidate01 can run it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn padded_doc_requests_are_refused_before_receiver_transfer_state_exists() {
    use khor_net::endpoint::{self, ALPN};
    use khor_node::proto::{
        self, DOC_TRANSFER_FRAME_BYTES, DocChunk, DocManifest, DocTransferAction, DocTransferReply,
        DocTransferState, MAX_FRAME, Request, Response,
    };
    use khor_sync::{
        doc_transfer::{Descriptor, DocumentVersion, Encoding},
        pins::{PinDoc, pins_dir},
        store::{self, Doc},
        transfer::CHUNK_BYTES,
    };

    struct Resident(tokio::task::JoinHandle<Result<(), String>>);
    impl Drop for Resident {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    impl Resident {
        async fn stop(mut self) {
            self.0.abort();
            let _ = (&mut self.0).await;
        }
    }
    async fn raw(conn: &iroh::endpoint::Connection, bytes: &[u8]) -> Response {
        timeout(Duration::from_secs(45), async {
            let (mut send, mut recv) = conn.open_bi().await.unwrap();
            send.write_all(bytes).await.unwrap();
            send.finish().unwrap();
            proto::decode(&recv.read_to_end(MAX_FRAME).await.unwrap()).unwrap()
        })
        .await
        .expect("the actual raw request must receive an answer")
    }
    fn response_kind(response: &Response) -> String {
        match response {
            Response::Refused { why } => format!("Refused({why})"),
            Response::InventoryNotAdmitted { .. } => "InventoryNotAdmitted".into(),
            Response::DocTransfer {
                reply:
                    DocTransferReply::State {
                        state: DocTransferState::Missing,
                        ..
                    },
            } => "DocTransfer.Missing".into(),
            Response::DocTransfer {
                reply: DocTransferReply::Confirmed { .. },
            } => "DocTransfer.Confirmed".into(),
            Response::DocTransfer {
                reply:
                    DocTransferReply::Exchanged {
                        received: Some(DocTransferState::Stored { next_offset }),
                        ..
                    },
            } => format!("DocTransfer.Stored({next_offset})"),
            _ => "unexpected response variant".into(),
        }
    }

    let issuer_root = root("raw-issuer");
    let receiver_root = root("raw-receiver");
    let issuer_server = Node::open_as(issuer_root.to_path_buf(), "issuer").unwrap();
    let issuer_resident = Resident(tokio::spawn(async move { issuer_server.serve().await }));
    wait_for_endpoint_file(&issuer_root).await;
    let issuer = Node::open_as(issuer_root.to_path_buf(), "issuer").unwrap();
    let receiver = Node::open_as(receiver_root.to_path_buf(), "receiver").unwrap();
    timeout(
        Duration::from_secs(60),
        receiver.pair(&issuer.invite().unwrap()),
    )
    .await
    .expect("the real Pair must finish")
    .unwrap();
    assert_eq!(issuer.devices().unwrap().len(), 2);
    assert_eq!(receiver.devices().unwrap().len(), 2);
    issuer_resident.stop().await;

    let receiver_server = Node::open_as(receiver_root.to_path_buf(), "receiver").unwrap();
    let receiver_resident = Resident(tokio::spawn(async move { receiver_server.serve().await }));
    wait_for_endpoint_file(&receiver_root).await;
    let direct = timeout(Duration::from_secs(45), async {
        loop {
            let me = receiver
                .devices()
                .unwrap()
                .into_iter()
                .find(|d| d.id == receiver.device_str())
                .unwrap();
            if !me.addrs.is_empty() {
                break me.addrs;
            }
            assert!(
                !receiver_resident.0.is_finished(),
                "receiver resident exited"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("the receiver must publish its real addresses");
    let secret =
        khor_net::identity::load_or_create(&issuer_root.join(".khor/identity.key")).unwrap();
    let sender = *secret.public().as_bytes();
    assert_eq!(
        khor_net::trust::id_hex(&sender),
        issuer.device_str(),
        "the raw connection must authenticate as the actually paired issuer"
    );
    let destination = khor_net::trust::id_from_hex(receiver.device_str()).unwrap();
    let ep = endpoint::bind(secret, issuer.relay_map()).await.unwrap();
    let address = endpoint::dial_addr(receiver.device_str(), &direct, &[]).unwrap();
    let conn = timeout(Duration::from_secs(45), ep.connect(address, ALPN))
        .await
        .expect("the paired identity must reach the receiver")
        .unwrap();
    assert_eq!(*conn.remote_id().as_bytes(), destination);

    let base = {
        let loaded = store::load_pile::<PinDoc>(&pins_dir(&receiver_root), 0xFA01).unwrap();
        DocumentVersion {
            epoch: loaded.store.epoch(),
            version: loaded.doc.version(),
        }
    };
    let (body, target) = {
        let mut loaded = store::load_pile::<PinDoc>(&pins_dir(&issuer_root), 0xFA02).unwrap();
        for i in 0u64..6000 {
            loaded
                .doc
                .set(
                    &format!("tui/raw/{i}/{}", blake3::hash(&i.to_be_bytes()).to_hex()),
                    true,
                )
                .unwrap();
        }
        loaded.store.flush(&loaded.doc).unwrap();
        assert_eq!(loaded.store.epoch(), base.epoch);
        (
            loaded.doc.changes_since(&base.version).unwrap(),
            loaded.doc.version(),
        )
    };
    assert!(
        body.len() > CHUNK_BYTES && body.len() as u64 <= khor_sync::transfer::MAX_BLOB_BYTES,
        "a real flushed Pins delta must have a full nonfinal canonical chunk"
    );
    let manifest = Descriptor {
        document: "pins".into(),
        epoch: base.epoch,
        encoding: Encoding::Updates,
        base: base.version.clone(),
        target: target.clone(),
    }
    .bind(
        sender,
        destination,
        body.len() as u64,
        *blake3::hash(&body).as_bytes(),
    )
    .unwrap();
    let manifest = DocManifest {
        source: manifest.source,
        recipient: manifest.recipient,
        binding: manifest.binding.into(),
        total_bytes: manifest.total_bytes,
        digest: manifest.digest,
    };
    let chunk = DocChunk {
        manifest,
        offset: 0,
        bytes: body[..CHUNK_BYTES].to_vec().into(),
    };
    let controls = [
        receiver_root.join(".khor/doc-transfer-state"),
        receiver_root.join(".khor/doc-transfer-blobs"),
    ];
    assert!(
        controls.iter().all(|p| !p.exists()),
        "new receiver must start without transfer state"
    );
    // This already-existing public request proves actual roster admission on
    // the same connection and that the general 6 MiB stream cap stays intact.
    let mut general = proto::encode(&Request::Versions {
        docs: vec![],
        readings: vec![],
    })
    .unwrap();
    general.resize(2 * DOC_TRANSFER_FRAME_BYTES, 0);
    let admitted = raw(&conn, &general).await;
    assert!(
        matches!(admitted, Response::Versions { docs, readings } if docs.is_empty() && readings.is_empty()),
        "a paired non-DocTransfer request in the general frame budget must reach inventory"
    );
    assert!(controls.iter().all(|p| !p.exists()));
    let mut outcomes = Vec::new();
    for action in [
        DocTransferAction::Write {
            chunk: chunk.clone(),
        },
        DocTransferAction::Confirm { proofs: vec![] },
    ] {
        let mut frame = proto::encode(&Request::DocTransfer { action }).unwrap();
        frame.resize(2 * DOC_TRANSFER_FRAME_BYTES, 0);
        assert!(frame.len() > DOC_TRANSFER_FRAME_BYTES && frame.len() <= MAX_FRAME);
        let reply = raw(&conn, &frame).await;
        let untouched = controls.iter().all(|p| !p.exists());
        eprintln!(
            "[Q039-RAW] padded_bytes={} reply={} transfer_dirs_absent={untouched}",
            frame.len(),
            response_kind(&reply)
        );
        outcomes.push((reply, untouched));
    }
    // The same authenticated connection must still reach the actual handler
    // with a legal full chunk: identity refusal/IO cannot stand in for B1.
    let have = DocumentVersion {
        epoch: base.epoch,
        version: target,
    }
    .encode()
    .unwrap();
    let legal = proto::encode(&Request::DocTransfer {
        action: DocTransferAction::Exchange {
            document: "pins".into(),
            have: have.into(),
            offer: Some(chunk),
            proofs: vec![],
            acknowledgements: vec![],
        },
    })
    .unwrap();
    assert!(legal.len() <= DOC_TRANSFER_FRAME_BYTES);
    let accepted = raw(&conn, &legal).await;
    let created_for_legal_request = controls.iter().all(|p| p.exists());
    eprintln!(
        "[Q039-RAW] legal_bytes={} chunk_bytes={CHUNK_BYTES} reply={} transfer_dirs_created={created_for_legal_request}",
        legal.len(),
        response_kind(&accepted)
    );
    ep.close().await;
    receiver_resident.stop().await;
    assert!(
        matches!(accepted, Response::DocTransfer { reply: DocTransferReply::Exchanged {
        received: Some(DocTransferState::Stored { next_offset }), .. } } if next_offset == CHUNK_BYTES as u64),
        "same-connection legal canonical chunk must reach the real admitted receiver handler"
    );
    assert!(created_for_legal_request);
    for (reply, untouched) in outcomes {
        assert!(
            matches!(reply, Response::Refused { why } if why == "document transfer request exceeds 1 MiB"),
            "raw padded frames need the dedicated byte-cap refusal, not identity/transport failure"
        );
        assert!(
            untouched,
            "a rejected padded frame must not create transfer metadata or blob state"
        );
    }
}
