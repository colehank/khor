//! **A session made once, when the answer is lost on the road** (round
//! NET-7, from ux's first-run walk: the Mac quit mid-create, the phone
//! said 「没有响应」, and the session had been started all the same).
//!
//! Two scratch machines, relay-only, both reaching the relay through one
//! road that holds each crossing half a second and can be **cut** —
//! every connection on it closed, new ones refused. (Both, not just the
//! asker: iroh reaches a machine through *its* home relay, so an asker
//! with a second road to the relay takes the answer that way.) Beta asks
//! alpha for a terminal session; the moment alpha has registered it,
//! while the answer is still crossing back, the road is cut. Beta's create fails, as the phone's did. The road heals
//! and beta asks again with the same token: the answer is the session
//! the first ask made, and alpha holds one, not two.
//!
//! The control is the same without a token: the second ask makes a
//! second session — the bug the token is for.

mod scratch;

use std::time::{Duration, Instant};

use khor_node::Node;
use scratch::*;
use tokio::time::timeout;

/// How many of alpha's own sessions carry `title`.
fn made(alpha: &Node, title: &str) -> usize {
    alpha
        .sessions()
        .unwrap_or_default()
        .iter()
        .filter(|r| r.source.is_none() && r.session.title == title)
        .count()
}

/// Beta asks for a terminal on alpha; the road is cut the moment alpha
/// has the session. What the ask said.
async fn ask_and_lose_the_answer(
    alpha: &Node,
    beta: &Node,
    cut: &Cut,
    title: &str,
    token: Option<&str>,
) -> Result<String, String> {
    let watching = async {
        let began = Instant::now();
        while made(alpha, title) == 0 {
            assert!(
                began.elapsed() < Duration::from_secs(60),
                "alpha never made {title}"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        cut.cut();
    };
    let asking = async {
        timeout(
            Duration::from_secs(120),
            beta.open_on(
                "alpha",
                "shell",
                title,
                "",
                &["/bin/cat".to_owned()],
                (80, 24),
                token,
            ),
        )
        .await
        .expect("the ask must end")
        .map(|id| id.0)
    };
    let ((), said) = tokio::join!(watching, asking);
    said
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_create_whose_answer_was_lost_is_made_once_when_asked_again() {
    // SAFETY: before any endpoint in this process binds.
    unsafe { std::env::set_var("KHOR_RELAY_ONLY", "1") };
    own_tmux();
    let (ra, rb, rr) = (scratch("once-a"), scratch("once-b"), scratch("once-r"));
    let (_relay, direct) = local_relay(&rr.0);
    let to: std::net::SocketAddr = direct
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .unwrap();
    let rtt = Duration::from_millis(2000);
    let (at, cut, _road) = cuttable_road(
        to,
        Impair {
            rtt,
            loss_pct: 0,
            stall: rtt,
        },
    )
    .await;
    home_on(&ra.0, &format!("http://{at}"));
    home_on(&rb.0, &format!("http://{at}"));
    let env = [("KHOR_RELAY_ONLY", "1")];
    let _a = serve_with(&ra.0, "alpha", &env);
    let _b = serve_with(&rb.0, "beta", &env);
    stood_up(&[&ra.0, &rb.0]).await;
    let alpha = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = Node::open_as(rb.0.clone(), "beta").unwrap();
    tokio::time::sleep(Duration::from_secs(10)).await;
    let ticket = alpha.invite().unwrap();
    let began = Instant::now();
    loop {
        match timeout(Duration::from_secs(90), beta.pair(&ticket))
            .await
            .expect("pairing must not hang")
        {
            Ok(_) => break,
            Err(why) if began.elapsed() < Duration::from_secs(120) => {
                eprintln!("[once] pairing: {why}")
            }
            Err(why) => panic!("pairs: {why}"),
        }
    }
    // A first session, so the wire is up and the next ask is only the ask.
    timeout(
        Duration::from_secs(90),
        beta.open_on(
            "alpha",
            "shell",
            "warm",
            "",
            &["/bin/cat".to_owned()],
            (80, 24),
            None,
        ),
    )
    .await
    .expect("warm-up must end")
    .expect("a warm-up session opens");

    // With a token.
    let token = "4b1f8e2a-9c3d-4e5f-8a7b-6c5d4e3f2a1b";
    let lost = ask_and_lose_the_answer(&alpha, &beta, &cut, "once", Some(token)).await;
    eprintln!("[once] the first ask said {lost:?}");
    assert!(
        lost.is_err(),
        "the answer was lost, so the ask failed: {lost:?}"
    );
    assert_eq!(
        made(&alpha, "once"),
        1,
        "alpha made the session all the same"
    );
    cut.heal();
    let began = Instant::now();
    let again = loop {
        match timeout(
            Duration::from_secs(60),
            beta.open_on(
                "alpha",
                "shell",
                "once",
                "",
                &["/bin/cat".to_owned()],
                (80, 24),
                Some(token),
            ),
        )
        .await
        .expect("the retry must end")
        {
            Ok(id) => break id.0,
            Err(why) if began.elapsed() < Duration::from_secs(90) => {
                eprintln!("[once] retry (road coming back): {why}");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(why) => panic!("the retry is answered: {why}"),
        }
    };
    let ids: Vec<String> = alpha
        .sessions()
        .unwrap()
        .into_iter()
        .filter(|r| r.session.title == "once")
        .map(|r| r.session.id.0)
        .collect();
    eprintln!("[once] the retry answered {again}; alpha holds {ids:?}");
    assert_eq!(
        ids,
        vec![again],
        "the retry answers the session the first ask made, and there is one"
    );

    // The control: no token, and the retry makes a second.
    let lost = ask_and_lose_the_answer(&alpha, &beta, &cut, "twice", None).await;
    assert!(lost.is_err(), "the answer was lost again: {lost:?}");
    cut.heal();
    let began = Instant::now();
    while let Err(why) = timeout(
        Duration::from_secs(60),
        beta.open_on(
            "alpha",
            "shell",
            "twice",
            "",
            &["/bin/cat".to_owned()],
            (80, 24),
            None,
        ),
    )
    .await
    .expect("the retry must end")
    {
        assert!(
            began.elapsed() < Duration::from_secs(90),
            "the retry is answered: {why}"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    assert_eq!(
        made(&alpha, "twice"),
        2,
        "without a token the retry made a second session"
    );
}
