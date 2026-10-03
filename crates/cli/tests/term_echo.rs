//! **A keystroke's echo from a far terminal** (round NET-2, item 2):
//! beta opens `cat` on alpha, attaches the way every face does (`reach`,
//! then the host handshake over the pipe the resident binds), types one
//! byte at a time and times each until it comes back; then streams a
//! burst of output and times that. Over loopback direct, and with
//! `TERM_RTT_MS` set, relay-only behind a slow road (`scratch`).
//!
//! The face's own beat (`TerminalPane` polls the feed every 50 ms) is
//! not in these numbers; they are the network half.

mod scratch;

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use scratch::*;

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_keystroke_comes_back_in_a_round_trip_and_output_streams() {
    let rtt: u64 = std::env::var("TERM_RTT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let road = (rtt > 0).then(|| Impair {
        rtt: Duration::from_millis(rtt),
        loss_pct: 0,
        stall: Duration::ZERO,
    });
    let (p, _road) = pair_on("echo", road).await;
    let sid = p
        .beta
        .open_on(
            "alpha",
            "shell",
            "echo",
            "",
            &["/bin/cat".to_owned()],
            (80, 24),
        )
        .await
        .expect("cat opens on alpha");
    let (addr, cookie) = p.beta.reach("alpha", &sid).await.expect("beta reaches it");
    let stream = tokio::task::spawn_blocking(move || {
        let mut s = khor_node::host::connect_at(&addr, &cookie, 80, 24).expect("the host welcomes");
        s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        // Drain the replay.
        s.set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        let mut buf = [0u8; 65536];
        while s.read(&mut buf).map(|n| n > 0).unwrap_or(false) {}
        s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();

        let mut echoes = Vec::new();
        for i in 0..30 {
            let key = b'a' + (i % 26) as u8;
            let began = Instant::now();
            khor_node::host::write_frame(&mut s, &khor_node::host::ClientOp::Input(vec![key]))
                .unwrap();
            loop {
                let n = s.read(&mut buf).expect("the echo comes back");
                if buf[..n].contains(&key) {
                    break;
                }
            }
            echoes.push(began.elapsed());
            // Let `cat`'s own copy of the line go by unseen.
            std::thread::sleep(Duration::from_millis(50));
            s.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
            while s.read(&mut buf).map(|n| n > 0).unwrap_or(false) {}
            s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        }

        // A burst: fifty lines of 79 characters typed at once (a tty
        // line holds at most 1024) — the echo and `cat` send back about
        // twice that.
        let line: Vec<u8> = (0..50)
            .flat_map(|_| (0..79).map(|i| b'a' + (i % 26) as u8).chain([b'\r']))
            .collect();
        let began = Instant::now();
        khor_node::host::write_frame(&mut s, &khor_node::host::ClientOp::Input(line)).unwrap();
        let mut got = 0usize;
        while got < 7000 {
            match s.read(&mut buf) {
                Ok(k) => got += k,
                Err(e) => panic!("the burst stopped at {got} bytes: {e}"),
            }
        }
        let burst = began.elapsed();
        let _ = s.write_all(&[]);
        (echoes, got, burst)
    })
    .await
    .unwrap();
    let (echoes, got, burst) = stream;
    let worst = *echoes.iter().max().unwrap();
    let med = median(echoes);
    eprintln!(
        "[echo] added RTT {rtt} ms: key->echo median {med:?}, worst {worst:?}; {got} bytes of burst back in {burst:?}"
    );
    let _ = p.beta.close_on("alpha", &sid).await;
}

/// **The program never waits for its slowest reader** (round NET-2): a
/// face that stops reading — a far viewer behind a stalled relay, a phone
/// frozen in the background — used to fill its socket and block the
/// host's writes, and with them the program's output for every face.
/// Here one attached face never reads while another does; the reading
/// face keeps receiving at full speed, and the stalled one is let go.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_face_that_stops_reading_does_not_stop_the_program() {
    let home = scratch("stall");
    home_on(&home.0, "http://127.0.0.1:9");
    let out = khor(&home.0)
        .args(["open", "-d", "--title", "flood", "--", "/bin/sh", "-c"])
        .arg("i=0; while :; do echo \"line $i of a terminal that never stops talking\"; i=$((i+1)); done")
        .env("KHOR_NAME", "solo")
        .output()
        .unwrap();
    let sid = String::from_utf8_lossy(&out.stdout)
        .trim()
        .lines()
        .last()
        .unwrap()
        .to_owned();
    assert!(
        sid.contains('/'),
        "a session id: {sid:?} {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // This machine's own host, read off its file the way an attach here
    // does (no resident needed for a local session).
    let dir = khor_node::live::sessions_dir(&home.0).join(sid.replace('/', "-"));
    let hf = {
        let mut tries = 0;
        loop {
            match khor_node::host::read_host_file(&dir) {
                Ok(hf) => break hf,
                Err(_) if tries < 50 => {
                    tries += 1;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => panic!("no host file under {}: {e}", dir.display()),
            }
        }
    };
    let (addr, cookie) = (format!("127.0.0.1:{}", hf.port), hf.cookie);
    let (got, stalled_closed) = tokio::task::spawn_blocking(move || {
        // The face that never reads.
        let stalled = khor_node::host::connect_at(&addr, &cookie, 80, 24).unwrap();
        std::thread::sleep(Duration::from_secs(2));
        // The face that does.
        let mut reading = khor_node::host::connect_at(&addr, &cookie, 80, 24).unwrap();
        reading
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let began = Instant::now();
        let mut got = 0usize;
        let mut buf = [0u8; 65536];
        while began.elapsed() < Duration::from_secs(5) {
            match reading.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(k) => got += k,
            }
        }
        // The stalled face: let go by the host, so its socket ends.
        let mut stalled = stalled;
        stalled
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let mut closed = false;
        for _ in 0..10_000 {
            match stalled.read(&mut buf) {
                Ok(0) => {
                    closed = true;
                    break;
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                    closed = true;
                    break;
                }
                Err(_) => break,
            }
        }
        (got, closed)
    })
    .await
    .unwrap();
    eprintln!(
        "[stall] beside a face that never reads: the reading face got {} KiB in 5 s; the stalled face let go: {stalled_closed}",
        got / 1024
    );
    let _ = khor(&home.0).args(["close", &sid]).output();
    assert!(
        got > 1024 * 1024,
        "the reading face starved: {got} bytes in 5 s"
    );
    assert!(stalled_closed, "the stalled face was never let go");
}
