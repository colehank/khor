//! Complete caller-visible outcomes for the lab's logical directory requests.
//! Raw start/end records survive a later assertion; an abandoned future records
//! cancellation instead of silently disappearing from the sample population.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::{Duration, Instant, SystemTime};

use serde_json::json;

static NEXT: AtomicU64 = AtomicU64::new(1);
const REVISION: &str = "attempt_outcomes_v4";

/// Keep the fault window's actual timestamps beside the caller outcomes.
pub(crate) fn phase(case: &str, phase: &str, at_ms: u64) {
    eprintln!(
        "[netlab-phase] {}",
        json!({
            "schema": 1, "measurement_revision": REVISION,
            "pid": std::process::id(), "case": case, "phase": phase, "at_ms": at_ms,
        })
    );
}

struct Attempt<'a> {
    case: &'a str,
    node: &'a str,
    to: &'a str,
    id: u64,
    began: Instant,
    within: Duration,
    finished: bool,
}

impl<'a> Attempt<'a> {
    fn begin(case: &'a str, node: &'a str, to: &'a str, within: Duration) -> Self {
        let attempt = Self {
            case,
            node,
            to,
            id: NEXT.fetch_add(1, Relaxed),
            began: Instant::now(),
            within,
            finished: false,
        };
        attempt.emit("started", None, None, Duration::ZERO);
        attempt
    }

    fn emit(&self, phase: &str, outcome: Option<&str>, error: Option<&str>, took: Duration) {
        let at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        eprintln!(
            "[netlab-attempt] {}",
            json!({
                "schema": 1, "measurement_revision": REVISION,
                "pid": std::process::id(), "attempt": self.id,
                "case": self.case, "from": self.node, "to": self.to,
                "operation": "list", "scope": "logical_request",
                "phase": phase, "at_ms": at,
                "within_ms": self.within.as_millis(),
                "elapsed_us": took.as_micros(), "outcome": outcome, "error": error,
            })
        );
    }

    fn finish(&mut self, outcome: &str, error: Option<&str>) -> Duration {
        let took = self.began.elapsed();
        self.finished = true;
        self.emit("finished", Some(outcome), error, took);
        took
    }
}

impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let outcome = if std::thread::panicking() {
                "aborted"
            } else {
                "cancelled"
            };
            self.finish(outcome, Some("measurement future dropped"));
        }
    }
}

/// The original outer budget includes recording work. Internal retries remain
/// part of this one logical caller request, rather than becoming extra successes.
pub(crate) async fn measure(
    case: &str,
    node: &str,
    to: &str,
    within: Duration,
    request: impl Future<Output = Result<(), String>>,
) -> (Duration, Option<String>) {
    let mut attempt = Attempt::begin(case, node, to, within);
    let until = tokio::time::Instant::from_std(attempt.began + within);
    let (outcome, error) = match tokio::time::timeout_at(until, request).await {
        Ok(Ok(())) => ("ok", None),
        Ok(Err(error)) => ("error", Some(error)),
        Err(_) => ("timeout", Some(format!("no answer in {within:?}"))),
    };
    let took = attempt.finish(outcome, error.as_deref());
    (took, error)
}
