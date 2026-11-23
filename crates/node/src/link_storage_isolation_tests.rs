use super::*;

#[tokio::test(flavor = "current_thread")]
async fn stalled_receipts_do_not_hold_the_directory_listing() {
    use std::{ffi::CString, fs, os::unix::ffi::OsStrExt, os::unix::fs::OpenOptionsExt};

    let root = tempfile::tempdir().unwrap();
    let listed = tempfile::tempdir().unwrap();
    fs::write(listed.path().join("ordinary.txt"), b"readable").unwrap();
    let node = Node::open_hosting(root.path().to_owned(), "storage-isolation", false).unwrap();
    let receipts = crate::road::received_dir(root.path());
    fs::create_dir_all(&receipts).unwrap();
    let blocked = receipts.join("blocked.json");
    let raw_path = CString::new(blocked.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(raw_path.as_ptr(), 0o600) }, 0);

    let reply = tokio::time::timeout(
        Duration::from_millis(650),
        node.list_here(listed.path().to_str().unwrap()),
    )
    .await;
    let release = fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&blocked);
    assert!(
        release.is_ok(),
        "the receipt worker must have reached the FIFO"
    );
    drop(release);

    let (_, rows, _) = reply
        .expect("directory returns while the receipt is stalled")
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "ordinary.txt");
    assert_eq!(rows[0].received_ms, None);
}

#[tokio::test(flavor = "current_thread")]
async fn blocked_admission_does_not_starve_the_resident_reactor() {
    use std::{ffi::CString, fs, io::Write, os::unix::ffi::OsStrExt};

    let root = tempfile::tempdir().unwrap();
    let node = Node::open_hosting(root.path().to_owned(), "storage-isolation", false).unwrap();
    let ep = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .bind()
        .await
        .unwrap();
    let anchor = root.path().join(".khor/root.pub");
    let contents = fs::read(&anchor).unwrap();
    fs::remove_file(&anchor).unwrap();
    let raw_path = CString::new(anchor.as_os_str().as_bytes()).unwrap();
    // This fixture stalls the real filesystem read with a FIFO, confined to its scratch home.
    assert_eq!(unsafe { libc::mkfifo(raw_path.as_ptr(), 0o600) }, 0);
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(550));
        let mut writer = fs::OpenOptions::new().write(true).open(&anchor).unwrap();
        writer.write_all(&contents).unwrap();
    });

    let started = tokio::time::Instant::now();
    let beat = async {
        tokio::time::sleep_until(started + Duration::from_millis(40)).await;
        started.elapsed()
    };
    let request = node.dispatch(
        node.device_str(),
        Request::Ls {
            path: root.path().display().to_string(),
        },
        &ep,
        None,
    );
    let (reply, elapsed) = tokio::join!(biased; request, beat);
    release.join().unwrap();
    ep.close().await;
    assert!(matches!(reply, Response::Dir { .. }), "{reply:?}");
    assert!(
        elapsed < Duration::from_millis(350),
        "a real admission file read stalled the reactor for {elapsed:?}"
    );
}

async fn bound_endpoint(secret: iroh::SecretKey) -> iroh::Endpoint {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(secret)
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![ALPN.to_vec()])
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .bind()
        .await
        .unwrap()
}

async fn read_over_wire(conn: &iroh::endpoint::Connection, path: &std::path::Path) -> Response {
    tokio::time::timeout(Duration::from_secs(3), async {
        let request = Request::FetchPath {
            path: path.display().to_string(),
            offset: 0,
        };
        let (mut send, mut recv) = conn.open_bi().await.unwrap();
        send.write_all(&proto::encode(&request).unwrap())
            .await
            .unwrap();
        send.finish().unwrap();
        proto::answer(&recv.read_to_end(MAX_FRAME).await.unwrap()).unwrap()
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn stalled_network_file_keeps_its_slot_while_a_fresh_wire_reads_local_files() {
    use std::{ffi::CString, fs, os::unix::ffi::OsStrExt, sync::Arc};

    let root = tempfile::tempdir().unwrap();
    let network = tempfile::tempdir().unwrap();
    let stalled = network.path().join("stalled");
    let raw_path = CString::new(stalled.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(raw_path.as_ptr(), 0o600) }, 0);
    let available = network.path().join("available");
    let local = root.path().join("local");
    fs::write(&available, b"network ready").unwrap();
    fs::write(&local, b"local ready").unwrap();

    let client = bound_endpoint(iroh::SecretKey::generate()).await;
    let newcomer = bound_endpoint(iroh::SecretKey::generate()).await;
    let mut node = Node::open_hosting(root.path().to_owned(), "storage-isolation", false).unwrap();
    node.storage_io
        .test_network_mount(network.path().to_owned(), Duration::from_millis(200));
    let mut table = node.devices_loaded().unwrap();
    let client_id = client.id().to_string();
    let newcomer_id = newcomer.id().to_string();
    table.doc.upsert(&client_id, "client", &[]).unwrap();
    table.doc.upsert(&newcomer_id, "newcomer", &[]).unwrap();
    node.enroll_after_pair(
        &table.doc,
        &[(&client_id, "client"), (&newcomer_id, "newcomer")],
    )
    .unwrap();
    table.store.flush(&table.doc).unwrap();

    let server = Arc::new(bound_endpoint(iroh::SecretKey::generate()).await);
    let node = Arc::new(node);
    let serving = tokio::spawn({
        let server = server.clone();
        async move {
            while let Some(incoming) = server.accept().await {
                let server = server.clone();
                let node = node.clone();
                tokio::spawn(async move {
                    let conn = incoming.accept().unwrap().await.unwrap();
                    let remote = conn.remote_id().to_string();
                    let (_done, verified) = tokio::sync::watch::channel(Some(true));
                    node.handle(conn, remote, server, verified, Some(ALPN.to_vec()))
                        .await
                        .unwrap();
                });
            }
        }
    });
    let connect = |ep: iroh::Endpoint| {
        let addr = server.addr();
        async move {
            tokio::time::timeout(Duration::from_secs(3), ep.connect(addr, ALPN))
                .await
                .unwrap()
                .unwrap()
        }
    };

    let wire = connect(client.clone()).await;
    let timed_out = read_over_wire(&wire, &stalled).await;
    assert!(matches!(timed_out, Response::Refused { why } if why == msg::storage_io_busy()));
    let blocked = read_over_wire(&wire, &available).await;
    assert!(matches!(blocked, Response::Refused { why } if why == msg::storage_io_busy()));

    // The FIFO still has no writer while a distinct peer handshakes and reads from local disk.
    let fresh_wire = connect(newcomer.clone()).await;
    let local_read = read_over_wire(&fresh_wire, &local).await;
    assert!(
        matches!(local_read, Response::PathSlice { bytes, .. } if bytes.as_ref() == b"local ready")
    );

    // An opener with O_NONBLOCK fails if the stalled request did not reach the kernel.
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&stalled)
        .unwrap();
    let recovered = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let response = read_over_wire(&wire, &available).await;
            if !matches!(response, Response::Refused { .. }) {
                break response;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        matches!(recovered, Response::PathSlice { bytes, .. } if bytes.as_ref() == b"network ready")
    );
    client.close().await;
    newcomer.close().await;
    server.close().await;
    serving.await.unwrap();
}
