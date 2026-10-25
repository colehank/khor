use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::{oneshot, watch};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessExit {
    pub code: Option<i32>,
}

/// The caller owns the child and publishes this receipt only after reap and I/O joins.
/// Dropping or cancelling a waiter never cancels that owner.
#[derive(Clone)]
pub struct TransportLifecycle(Arc<Provider>);
struct Provider {
    requested: AtomicBool,
    request: Box<dyn Fn() + Send + Sync>,
    completed: watch::Receiver<Option<Result<ProcessExit, String>>>,
}
impl Provider {
    fn request_once(&self) {
        if !self.requested.swap(true, Ordering::AcqRel) {
            (self.request)();
        }
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        self.request_once();
    }
}
impl TransportLifecycle {
    pub fn new(
        request: impl Fn() + Send + Sync + 'static,
        completed: watch::Receiver<Option<Result<ProcessExit, String>>>,
    ) -> Self {
        Self(Arc::new(Provider {
            requested: AtomicBool::new(false),
            request: Box::new(request),
            completed,
        }))
    }
    pub fn request_shutdown(&self) {
        self.0.request_once();
    }
    pub async fn confirmed_exit(&self) -> Result<ProcessExit, String> {
        let mut completed = self.0.completed.clone();
        loop {
            if let Some(result) = completed.borrow_and_update().clone() {
                return result;
            }
            completed
                .changed()
                .await
                .map_err(|_| "ACP process owner ended without exit confirmation".to_owned())?;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShutdownOutcome {
    /// The legacy SDK driver ended; this is not a child-reap receipt.
    DriverClosed,
    ProcessReaped(ProcessExit),
}

pub(super) struct CloseGate {
    close: Mutex<Option<oneshot::Sender<()>>>,
    requested: watch::Sender<bool>,
    result: watch::Receiver<Option<Result<ShutdownOutcome, String>>>,
    provider: Option<TransportLifecycle>,
}
impl CloseGate {
    pub(super) fn new(
        close: oneshot::Sender<()>,
        result: watch::Receiver<Option<Result<ShutdownOutcome, String>>>,
        provider: Option<TransportLifecycle>,
    ) -> Self {
        let (requested, _) = watch::channel(false);
        Self {
            close: Mutex::new(Some(close)),
            requested,
            result,
            provider,
        }
    }
    pub(super) fn request(&self) {
        let first = !self.requested.send_replace(true);
        if let Some(close) = self.close.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = close.send(());
        }
        if first {
            if let Some(provider) = &self.provider {
                provider.request_shutdown();
            }
        }
    }
    pub(super) async fn driver_closed(&self) -> Result<ShutdownOutcome, String> {
        self.request();
        match &self.provider {
            Some(provider) => provider
                .confirmed_exit()
                .await
                .map(ShutdownOutcome::ProcessReaped),
            None => Ok(ShutdownOutcome::DriverClosed),
        }
    }
    pub(super) async fn completed(&self) -> Result<ShutdownOutcome, String> {
        let mut result = self.result.clone();
        loop {
            if let Some(result) = result.borrow_and_update().clone() {
                return result;
            }
            if result.changed().await.is_err() {
                return match &self.provider {
                    Some(provider) => provider
                        .confirmed_exit()
                        .await
                        .map(ShutdownOutcome::ProcessReaped),
                    None => Err("ACP connection ended without shutdown confirmation".to_owned()),
                };
            }
        }
    }
    pub(super) fn check_open(&self) -> Result<(), String> {
        if *self.requested.borrow() {
            Err(
                "ACP transport is closing; the pending operation may already have been received"
                    .into(),
            )
        } else {
            Ok(())
        }
    }
    pub(super) async fn while_open<T>(
        &self,
        future: impl Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        self.check_open()?;
        let mut requested = self.requested.subscribe();
        tokio::select! {
            biased;
            // Preserve a final response already delivered before the driver closed.
            result = future => result,
            _ = requested.wait_for(|requested| *requested) => Err("ACP transport is closing; the pending operation may already have been received".into()),
        }
    }
}

pub(super) struct Opening(pub Option<Arc<CloseGate>>);
impl Drop for Opening {
    fn drop(&mut self) {
        if let Some(close) = &self.0 {
            close.request();
        }
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
