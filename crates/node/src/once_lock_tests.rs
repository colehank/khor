use super::{LOCKS, lock, made, note};
use std::{
    future::{Future, poll_fn},
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    task::Poll,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::OwnedMutexGuard;

fn scope() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "ol-{}-{nonce:x}-{}-",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

async fn assert_pending<F: Future>(mut future: Pin<&mut F>) {
    poll_fn(|cx| {
        assert!(
            future.as_mut().poll(cx).is_pending(),
            "a second lock entered an occupied token"
        );
        Poll::Ready(())
    })
    .await;
}

async fn acquire<F: Future>(future: F) -> F::Output {
    tokio::time::timeout(Duration::from_secs(2), future)
        .await
        .expect("token waiter deadlocked")
}

fn entries_in(scope: &str) -> usize {
    LOCKS
        .get()
        .unwrap()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .keys()
        .filter(|token| token.starts_with(scope))
        .count()
}

#[tokio::test]
async fn completed_unique_tokens_do_not_accumulate_historical_mutexes() {
    let scope = scope();
    let mut first = None;
    for index in 0..10_000 {
        let guard = lock(&format!("{scope}{index}")).await;
        if index == 0 {
            first = Some(Arc::downgrade(OwnedMutexGuard::mutex(&guard)));
        }
        drop(guard);
    }
    // Pin this test's current entry while inspecting only its unique namespace.
    let current = lock(&format!("{scope}current")).await;
    assert_eq!(
        entries_in(&scope),
        1,
        "completed historical tokens still retain registry entries"
    );
    assert!(
        first.unwrap().upgrade().is_none(),
        "the first idle mutex is still strongly retained"
    );
    drop(current);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_requests_for_one_token_never_overlap_their_critical_sections() {
    let token = Arc::new(format!("{}shared", scope()));
    let active = Arc::new(AtomicU64::new(0));
    let mut requests = Vec::new();
    for _ in 0..128 {
        let token = token.clone();
        let active = active.clone();
        requests.push(tokio::spawn(async move {
            let held = lock(&token).await;
            assert_eq!(active.fetch_add(1, Ordering::SeqCst), 0);
            tokio::task::yield_now().await;
            assert_eq!(active.fetch_sub(1, Ordering::SeqCst), 1);
            drop(held);
        }));
    }
    for request in requests {
        acquire(request).await.unwrap();
    }
    assert_eq!(active.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn queued_waiter_keeps_one_live_mutex_across_owner_release_and_sweeps() {
    let scope = scope();
    let token = format!("{scope}shared");
    let owner = lock(&token).await;
    let original = Arc::downgrade(OwnedMutexGuard::mutex(&owner));
    let mut queued = Box::pin(lock(&token));
    assert_pending(queued.as_mut()).await;
    let other = lock(&format!("{scope}other")).await;
    drop(owner);
    let next = acquire(queued).await;
    assert!(original.ptr_eq(&Arc::downgrade(OwnedMutexGuard::mutex(&next))));

    let mut third = Box::pin(lock(&token));
    assert_pending(third.as_mut()).await;
    // Another lookup sweeps while one holder and one queued waiter own the mutex.
    let sweep = lock(&format!("{scope}sweep")).await;
    assert_eq!(entries_in(&scope), 3);
    drop(next);
    let last = acquire(third).await;
    assert!(original.ptr_eq(&Arc::downgrade(OwnedMutexGuard::mutex(&last))));
    drop((last, other, sweep));
}

#[tokio::test]
async fn cancelling_a_queued_waiter_preserves_the_survivor_and_releases_the_idle_mutex() {
    let scope = scope();
    let token = format!("{scope}shared");
    let owner = lock(&token).await;
    let original = Arc::downgrade(OwnedMutexGuard::mutex(&owner));
    let mut cancelled = Box::pin(lock(&token));
    assert_pending(cancelled.as_mut()).await;
    let mut survivor = Box::pin(lock(&token));
    assert_pending(survivor.as_mut()).await;
    drop(cancelled);
    let sweep = lock(&format!("{scope}sweep")).await;
    assert!(
        original.upgrade().is_some(),
        "a queued survivor lost its original mutex"
    );
    drop(owner);
    let survivor = acquire(survivor).await;
    assert!(original.ptr_eq(&Arc::downgrade(OwnedMutexGuard::mutex(&survivor))));
    drop(survivor);
    assert!(
        original.upgrade().is_none(),
        "cancelled waiters or the registry retain an idle mutex"
    );
    let after = lock(&format!("{scope}after")).await;
    assert_eq!(entries_in(&scope), 2, "dead shared key was not swept");
    drop((sweep, after));
}

#[tokio::test]
async fn cancelling_the_only_waiter_allows_the_last_owner_to_release_the_mutex() {
    let scope = scope();
    let token = format!("{scope}shared");
    let owner = lock(&token).await;
    let original = Arc::downgrade(OwnedMutexGuard::mutex(&owner));
    let mut cancelled = Box::pin(lock(&token));
    assert_pending(cancelled.as_mut()).await;
    drop(cancelled);
    drop(owner);
    assert!(
        original.upgrade().is_none(),
        "the cancelled queue left an idle mutex alive"
    );
    let replacement = acquire(lock(&token)).await;
    assert!(!original.ptr_eq(&Arc::downgrade(OwnedMutexGuard::mutex(&replacement))));
}

#[tokio::test]
async fn different_tokens_can_be_held_at_the_same_time() {
    let scope = scope();
    let first = lock(&format!("{scope}first")).await;
    let second = acquire(lock(&format!("{scope}second"))).await;
    assert!(!Arc::ptr_eq(
        OwnedMutexGuard::mutex(&first),
        OwnedMutexGuard::mutex(&second)
    ));
}

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn releasing_a_token_mutex_does_not_forget_the_persistent_creation_receipt() {
    let scope = scope();
    let token = format!("{scope}create");
    let root = Scratch(std::env::temp_dir().join(format!("khor-{scope}receipt")));
    std::fs::create_dir(&root.0).unwrap();
    let mut effects = 0;
    {
        let _held = lock(&token).await;
        assert!(made(&root.0, &token).is_none());
        effects += 1;
        note(&root.0, &token, "shell/persistent-once-fixture").unwrap();
    }
    let receipt = root.0.join(".khor/opened").join(&token);
    let before = std::fs::read(&receipt).unwrap();
    let modified = std::fs::metadata(&receipt).unwrap().modified().unwrap();
    let sweep = lock(&format!("{scope}sweep")).await;
    {
        let _replayed = lock(&token).await;
        let id = match made(&root.0, &token) {
            Some(id) => id,
            None => {
                effects += 1;
                note(&root.0, &token, "shell/unwanted-second-effect").unwrap();
                "shell/unwanted-second-effect".into()
            }
        };
        assert_eq!(id, "shell/persistent-once-fixture");
    }
    assert_eq!(effects, 1, "a reclaimed mutex replayed the creation effect");
    assert_eq!(std::fs::read(&receipt).unwrap(), before);
    assert_eq!(
        std::fs::metadata(&receipt).unwrap().modified().unwrap(),
        modified
    );
    drop(sweep);
}
