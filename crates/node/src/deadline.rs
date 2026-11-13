//! The resident operation's immutable deadline, scoped to its own future.
//!
//! Task-local storage carries it through the existing operation handlers.
//! Network stages pass the absolute deadline explicitly. A separately spawned
//! task must deliberately inherit it or start its own background budget.

use std::future::Future;

use tokio::time::Instant;

tokio::task_local! {
    static BY: Instant;
}

pub(crate) fn current() -> Option<Instant> {
    BY.try_with(|by| *by).ok()
}

pub(crate) fn earlier(by: Option<Instant>) -> Option<Instant> {
    match (by, current()) {
        (Some(explicit), Some(operation)) => Some(explicit.min(operation)),
        (explicit, operation) => explicit.or(operation),
    }
}

pub(crate) fn expired(by: Option<Instant>) -> bool {
    by.is_some_and(|by| Instant::now() >= by)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Expired;

/// Wait only inside an already chosen absolute deadline. A ready result
/// cannot extend it; dropping the future retains its normal cancellation owner.
pub(crate) async fn wait<T>(
    by: Option<Instant>,
    future: impl Future<Output = T>,
) -> Result<T, Expired> {
    if expired(by) {
        return Err(Expired);
    }
    let result = match by {
        Some(by) => tokio::time::timeout_at(by, future)
            .await
            .map_err(|_| Expired)?,
        None => future.await,
    };
    if expired(by) {
        return Err(Expired);
    }
    Ok(result)
}

pub(crate) async fn scope<T>(by: Instant, future: impl Future<Output = T>) -> T {
    let by = current().map_or(by, |parent| parent.min(by));
    BY.scope(by, future).await
}
