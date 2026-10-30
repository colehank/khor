//! Encoded and pending-interaction credit belonging to one event store.
const WINDOW_BYTES: usize = 8 * 1024 * 1024;
pub(super) const WINDOW_RECORDS: usize = 256;
pub(super) const CONTROL_ITEMS: usize = 16;
const CONTROL_BYTES: usize = 128 * 1024;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct BudgetStats {
    /// Reserved encoded bytes, including encoding, queued and physical-write work.
    encoded_bytes: usize,
    records: usize,
    pub(super) peak_encoded_bytes: usize,
    pub(super) peak_records: usize,
    pub(super) largest_exclusive_record: usize,
    /// The credit follows an interaction all the way through answer/drop.
    controls: usize,
    control_bytes: usize,
    peak_controls: usize,
    peak_control_bytes: usize,
    closed: bool,
}
#[derive(Debug)]
pub(super) struct Budget {
    state: std::sync::Mutex<BudgetStats>,
    space: tokio::sync::Notify,
}
impl Budget {
    pub(super) fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            state: std::sync::Mutex::new(BudgetStats::default()),
            space: tokio::sync::Notify::new(),
        })
    }
    fn state(&self) -> std::sync::MutexGuard<'_, BudgetStats> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
    pub(super) async fn encoded(
        self: &std::sync::Arc<Self>,
        bytes: usize,
    ) -> Result<EncodedCredit, String> {
        loop {
            // Registration precedes the mutex check. Both credit return and
            // cancellation notify this same wait; no data queue is involved.
            let space = self.space.notified();
            tokio::pin!(space);
            space.as_mut().enable();
            {
                let mut state = self.state();
                if state.closed {
                    return Err("ACP event store is closed".into());
                }
                let fits = state.records < WINDOW_RECORDS
                    && if bytes > WINDOW_BYTES {
                        state.records == 0
                    } else {
                        state
                            .encoded_bytes
                            .checked_add(bytes)
                            .is_some_and(|total| total <= WINDOW_BYTES)
                    };
                if fits {
                    state.encoded_bytes = state
                        .encoded_bytes
                        .checked_add(bytes)
                        .ok_or("ACP event byte accounting overflow")?;
                    state.records += 1;
                    state.peak_encoded_bytes = state.peak_encoded_bytes.max(state.encoded_bytes);
                    state.peak_records = state.peak_records.max(state.records);
                    if bytes > WINDOW_BYTES {
                        state.largest_exclusive_record = state.largest_exclusive_record.max(bytes);
                    }
                    return Ok(EncodedCredit {
                        budget: self.clone(),
                        bytes,
                    });
                }
            }
            space.await;
        }
    }
    pub(super) fn interaction(
        self: &std::sync::Arc<Self>,
        bytes: usize,
    ) -> Result<InteractionCredit, String> {
        let mut state = self.state();
        if state.closed {
            return Err("ACP event store is closed".into());
        }
        if bytes > CONTROL_BYTES || state.controls == CONTROL_ITEMS {
            return Err("ACP pending interaction budget is exhausted".into());
        }
        state.controls += 1;
        state.control_bytes += bytes; // At most 16 records of at most 128 KiB.
        state.peak_controls = state.peak_controls.max(state.controls);
        state.peak_control_bytes = state.peak_control_bytes.max(state.control_bytes);
        Ok(InteractionCredit {
            budget: self.clone(),
            bytes,
        })
    }
    pub(super) fn close(&self) {
        self.state().closed = true;
        self.space.notify_waiters();
    }
    pub(super) fn stats(&self) -> BudgetStats {
        *self.state()
    }
}
#[derive(Debug)]
pub(super) struct EncodedCredit {
    budget: std::sync::Arc<Budget>,
    bytes: usize,
}
impl Drop for EncodedCredit {
    fn drop(&mut self) {
        let mut state = self.budget.state();
        state.encoded_bytes -= self.bytes;
        state.records -= 1;
        drop(state);
        self.budget.space.notify_waiters();
    }
}
/// Moves with Ask/Elicitation through queue, receiver and answer/drop.
/// A separately cancelled SDK response task cannot release it prematurely.
#[derive(Debug)]
pub(crate) struct InteractionCredit {
    budget: std::sync::Arc<Budget>,
    bytes: usize,
}
impl Drop for InteractionCredit {
    fn drop(&mut self) {
        let mut state = self.budget.state();
        state.controls -= 1;
        state.control_bytes -= self.bytes;
    }
}

impl InteractionCredit {
    pub(super) fn belongs_to(&self, budget: &std::sync::Arc<Budget>) -> bool {
        std::sync::Arc::ptr_eq(&self.budget, budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn byte_count_and_exclusive_credit_return_or_cancel_the_same_waiter() {
        // This tests actual admission/credit ownership, not disk throughput or RSS.
        let budget = Budget::new();
        let mut held = Vec::new();
        for _ in 0..8 {
            held.push(budget.encoded(WINDOW_BYTES / 8).await.unwrap());
        }
        let mut waiting = Box::pin(budget.encoded(1));
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut waiting)
                .await
                .is_err()
        );
        budget.close();
        assert!(waiting.await.unwrap_err().contains("closed"));
        assert_eq!(budget.stats().peak_encoded_bytes, WINDOW_BYTES);
        drop(held);
        assert_eq!(budget.stats().encoded_bytes, 0);
        assert_eq!(budget.stats().records, 0);

        let budget = Budget::new();
        let mut held = Vec::new();
        for _ in 0..WINDOW_RECORDS {
            held.push(budget.encoded(1).await.unwrap());
        }
        let mut waiting = Box::pin(budget.encoded(1));
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut waiting)
                .await
                .is_err()
        );
        drop(held.pop());
        let returned = tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(budget.stats().peak_records, WINDOW_RECORDS);
        drop(returned);
        drop(held);

        // A legal giant is charged in full and is exclusive, rather than being
        // clamped to the window or rejected because total history is long.
        let giant = budget.encoded(WINDOW_BYTES + 1).await.unwrap();
        assert_eq!(budget.stats().encoded_bytes, WINDOW_BYTES + 1);
        assert_eq!(budget.stats().records, 1);
        let mut waiting = Box::pin(budget.encoded(1));
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut waiting)
                .await
                .is_err()
        );
        drop(giant);
        let returned = tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .unwrap()
            .unwrap();
        drop(returned);
        assert_eq!(budget.stats().largest_exclusive_record, WINDOW_BYTES + 1);

        let mut questions = Vec::new();
        for _ in 0..CONTROL_ITEMS {
            questions.push(budget.interaction(CONTROL_BYTES).unwrap());
        }
        assert!(budget.interaction(1).is_err());
        assert!(budget.interaction(CONTROL_BYTES + 1).is_err());
        drop(questions.pop());
        let returned = budget.interaction(1).unwrap();
        assert_eq!(budget.stats().peak_controls, CONTROL_ITEMS);
        assert_eq!(
            budget.stats().peak_control_bytes,
            CONTROL_ITEMS * CONTROL_BYTES
        );
        drop(returned);
        drop(questions);
        assert_eq!(budget.stats().control_bytes, 0);
        assert_eq!(budget.stats().controls, 0);
    }
}
