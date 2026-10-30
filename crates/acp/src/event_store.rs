//! One bounded event-store owner: physical I/O job, finite prefix and receiver.
mod budget;
mod disk;
pub(super) use budget::InteractionCredit;
use budget::{Budget, EncodedCredit, CONTROL_ITEMS, WINDOW_RECORDS};
use disk::{check_cancel, invalid, Record, RecordKind, Spool, HEADER_BYTES, IO_CHUNK};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// Count the original serde representation before allocating its encoded queue entry.
/// The SDK's one current decoded object is a separate maxSingleFrame transient.
#[derive(Default)]
struct EncodedSize(usize);
impl Write for EncodedSize {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| invalid("event encoded size overflow"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn encoded_size(value: &impl serde::Serialize) -> Result<usize, String> {
    let mut count = EncodedSize::default();
    serde_json::to_writer(&mut count, value).map_err(|error| error.to_string())?;
    count
        .0
        .checked_add(HEADER_BYTES)
        .ok_or_else(|| "event encoded size overflow".into())
}

use super::{Event, SessionId, SessionNotification};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, Weak};
use tokio::sync::{watch, Notify};

#[derive(Debug)]
struct ScopeIdentity;
/// A finite prefix of one actual event store, never a mutable queue-length sample.
#[derive(Clone, Debug)]
pub struct EventPrefix {
    scope: Arc<ScopeIdentity>,
    through: u64,
}
#[derive(Clone, Debug, Default)]
struct Progress {
    written: u64,
    failed: Option<Arc<str>>,
}
struct Queued {
    record: Record,
    interaction: Option<Event>,
    _encoded: Option<EncodedCredit>,
}
struct Output {
    sequence: Option<u64>,
    event: Event,
}
#[derive(Clone)]
enum SourceEnd {
    Eof {
        through: u64,
    },
    /// The transport ended; its actual process/shutdown fact is still pending.
    Closing {
        through: u64,
        sequence: u64,
    },
    Closed {
        through: u64,
        sequence: u64,
        error: Option<Arc<str>>,
    },
}
impl SourceEnd {
    fn data_through(&self) -> u64 {
        match self {
            Self::Eof { through }
            | Self::Closing { through, .. }
            | Self::Closed { through, .. } => *through,
        }
    }
    fn event_sequence(&self) -> Option<u64> {
        match self {
            Self::Eof { .. } => None,
            Self::Closing { sequence, .. } | Self::Closed { sequence, .. } => Some(*sequence),
        }
    }
    fn confirmed(&self) -> bool {
        !matches!(self, Self::Closing { .. })
    }
    fn error(&self) -> Option<Arc<str>> {
        match self {
            Self::Closed { error, .. } => error.clone(),
            _ => None,
        }
    }
}

struct State {
    queue: VecDeque<Queued>,
    accepted: u64,
    written: u64,
    delivered: u64,
    read_requested: bool,
    output: Option<Output>,
    end: Option<SourceEnd>,
    failed: Option<Arc<str>>,
    receiver_alive: bool,
    ready_enqueued: bool,
    session: Option<SessionId>,
    done: bool,
}
struct Shared {
    state: Mutex<State>,
    io_changed: Condvar,
    cancelled: AtomicBool,
    data: Arc<Notify>,
    progress: watch::Sender<Progress>,
    completed: watch::Sender<Option<Result<(), String>>>,
    budget: Arc<Budget>,
    scope: Arc<ScopeIdentity>,
}
impl Shared {
    fn new() -> Arc<Self> {
        let (progress, _) = watch::channel(Progress::default());
        let (completed, _) = watch::channel(None);
        Arc::new(Self {
            state: Mutex::new(State {
                queue: VecDeque::with_capacity(WINDOW_RECORDS + CONTROL_ITEMS + 1),
                accepted: 0,
                written: 0,
                delivered: 0,
                read_requested: false,
                output: None,
                end: None,
                failed: None,
                receiver_alive: true,
                ready_enqueued: false,
                session: None,
                done: false,
            }),
            io_changed: Condvar::new(),
            cancelled: AtomicBool::new(false),
            data: Arc::new(Notify::new()),
            progress,
            completed,
            budget: Budget::new(),
            scope: Arc::new(ScopeIdentity),
        })
    }
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
    fn writable(state: &State) -> Result<(), String> {
        if let Some(error) = &state.failed {
            return Err(error.to_string());
        }
        if !state.receiver_alive {
            return Err("ACP event receiver closed".into());
        }
        if state.end.is_some() || state.done {
            return Err("ACP event source closed".into());
        }
        Ok(())
    }
    fn enqueue(&self, mut item: Queued) -> Result<(), String> {
        let mut state = self.state();
        Self::writable(&state)?;
        let sequence = state
            .accepted
            .checked_add(1)
            .ok_or("ACP event sequence overflow")?;
        item.record.sequence = sequence;
        state.accepted = sequence;
        state.queue.push_back(item);
        drop(state);
        self.io_changed.notify_one();
        Ok(())
    }
    fn wake_all(&self) {
        self.io_changed.notify_all();
        self.data.notify_waiters();
    }
    fn merge_progress(&self, update: Progress) {
        // A physical write may finish just before cancel but publish afterward.
        // Never let that older successful snapshot erase the failure or cursor.
        self.progress.send_modify(|current| {
            current.written = current.written.max(update.written);
            if current.failed.is_none() {
                current.failed = update.failed;
            }
        });
    }
    fn pulse_progress(&self) {
        let state = self.state();
        let progress = Progress {
            written: state.written,
            failed: state.failed.clone(),
        };
        drop(state);
        self.merge_progress(progress);
    }
    fn cancel(&self, reason: String) {
        // The same mutex as Condvar::wait closes the sleep/notify race. The
        // atomic is also visible to physical I/O without taking this mutex.
        let reason = Arc::from(reason);
        let mut state = self.state();
        if state.done {
            return;
        }
        if state.failed.is_none() {
            state.failed = Some(reason);
        }
        self.cancelled.store(true, Ordering::Release);
        let progress = Progress {
            written: state.written,
            failed: state.failed.clone(),
        };
        drop(state);
        self.budget.close();
        self.merge_progress(progress);
        self.wake_all();
    }
    fn source_eof(&self) {
        let mut state = self.state();
        if state.end.is_none() && state.failed.is_none() {
            state.end = Some(SourceEnd::Eof {
                through: state.accepted,
            });
        }
        drop(state);
        self.budget.close();
        self.pulse_progress();
        self.io_changed.notify_all();
    }
    fn begin_close(&self) {
        let mut state = self.state();
        if state.end.is_some() || state.failed.is_some() {
            return;
        }
        let Some(sequence) = state.accepted.checked_add(1) else {
            drop(state);
            self.cancel("ACP event sequence overflow".into());
            return;
        };
        let through = state.accepted;
        state.accepted = sequence;
        state.end = Some(SourceEnd::Closing { through, sequence });
        drop(state);
        self.budget.close();
        self.pulse_progress();
        self.io_changed.notify_all();
    }
    fn closed(&self, error: Option<String>) {
        self.begin_close();
        let error = error.map(Arc::<str>::from);
        let mut state = self.state();
        if let Some(SourceEnd::Closing { through, sequence }) = &state.end {
            state.end = Some(SourceEnd::Closed {
                through: *through,
                sequence: *sequence,
                error,
            });
        }
        drop(state);
        self.pulse_progress();
        self.io_changed.notify_all();
    }
}
struct ProducerScope(Weak<Shared>);
impl Drop for ProducerScope {
    fn drop(&mut self) {
        if let Some(shared) = self.0.upgrade() {
            shared.source_eof();
        }
    }
}
#[derive(Clone)]
pub(super) struct Sender {
    shared: Arc<Shared>,
    _producer: Arc<ProducerScope>,
}
impl Sender {
    async fn serialized(
        &self,
        kind: RecordKind,
        value: &impl serde::Serialize,
    ) -> Result<(), String> {
        let bytes = encoded_size(value)?;
        let credit = self.shared.budget.encoded(bytes).await?;
        let payload = serde_json::to_vec(value).map_err(|error| error.to_string())?;
        if payload.len().checked_add(HEADER_BYTES) != Some(bytes) {
            return Err("ACP event encoding changed during admission".into());
        }
        self.shared.enqueue(Queued {
            record: Record {
                sequence: 0,
                kind,
                payload,
            },
            interaction: None,
            _encoded: Some(credit),
        })
    }
    pub(super) async fn note(&self, note: &SessionNotification) -> Result<(), String> {
        let session = self.shared.state().session.clone();
        if session
            .as_ref()
            .is_some_and(|session| *session != note.session_id)
        {
            let error = "ACP event belongs to another native session".to_owned();
            self.shared.cancel(error.clone());
            return Err(error);
        }
        self.serialized(RecordKind::Note, note).await
    }
    pub(super) fn accept_session(&self, session: SessionId) -> Result<(), String> {
        let mut state = self.shared.state();
        Shared::writable(&state)?;
        if state
            .session
            .as_ref()
            .is_some_and(|known| *known != session)
        {
            return Err("ACP opening belongs to another native session".into());
        }
        state.session = Some(session);
        Ok(())
    }
    pub(super) fn accepts_interactions(&self) -> bool {
        let state = self.shared.state();
        state.session.is_some() && Shared::writable(&state).is_ok()
    }
    pub(super) fn receiver_released(&self) -> bool {
        self.shared.state().ready_enqueued
    }
    pub(super) async fn elicitation_done(&self, id: &str) -> Result<(), String> {
        self.serialized(RecordKind::ElicitDone, &id).await
    }
    pub(super) fn interaction_credit(
        &self,
        request: &impl serde::Serialize,
    ) -> Result<InteractionCredit, String> {
        self.shared.budget.interaction(encoded_size(request)?)
    }
    /// The question owns its permit until answer/drop, including after delivery.
    pub(super) fn interaction(&self, event: Event) -> Result<(), String> {
        let credit = match &event {
            Event::Ask(ask) => &ask._store_credit,
            Event::Elicit(question) => &question._store_credit,
            _ => return Err("ACP event is not an interaction".into()),
        };
        if !credit.belongs_to(&self.shared.budget) {
            return Err("ACP interaction belongs to another event owner".into());
        }
        self.shared.enqueue(Queued {
            record: Record {
                sequence: 0,
                kind: RecordKind::Interaction,
                payload: Vec::new(),
            },
            interaction: Some(event),
            _encoded: None,
        })
    }
    pub(super) fn ready(&self, session: SessionId) -> Result<(), String> {
        let mut state = self.shared.state();
        Shared::writable(&state)?;
        if state.ready_enqueued {
            return Err("ACP opening was already published".into());
        }
        if state
            .session
            .as_ref()
            .is_some_and(|known| *known != session)
        {
            return Err("ACP opening belongs to another native session".into());
        }
        let sequence = state
            .accepted
            .checked_add(1)
            .ok_or("ACP event sequence overflow")?;
        state.accepted = sequence;
        state.ready_enqueued = true;
        // One already-decoded session context, not another data queue/copy.
        state.session = Some(session);
        state.queue.push_back(Queued {
            record: Record {
                sequence,
                kind: RecordKind::Ready,
                payload: Vec::new(),
            },
            interaction: None,
            _encoded: None,
        });
        drop(state);
        self.shared.io_changed.notify_one();
        Ok(())
    }
    pub(super) fn begin_close(&self) {
        self.shared.begin_close();
    }
    pub(super) fn closed(&self, error: Option<String>) {
        self.shared.closed(error);
    }
    pub(super) fn cancel(&self, reason: String) {
        self.shared.cancel(reason);
    }
    async fn problem(&self, opening: bool) -> String {
        let mut progress = self.shared.progress.subscribe();
        loop {
            let problem = {
                let state = self.shared.state();
                state.failed.clone().or_else(|| {
                    if opening {
                        state.end.as_ref().map(|end| {
                            end.error().unwrap_or_else(|| {
                                Arc::from("The agent disconnected before opening")
                            })
                        })
                    } else {
                        None
                    }
                })
            };
            if let Some(error) = problem {
                return error.to_string();
            }
            if progress.changed().await.is_err() {
                return "ACP event owner ended without confirmation".into();
            }
        }
    }
    pub(super) async fn failed(&self) -> String {
        self.problem(false).await
    }
    pub(super) async fn opening_ended(&self) -> String {
        self.problem(true).await
    }
    pub(super) async fn shutdown(&self) -> Result<(), String> {
        let mut completed = self.shared.completed.subscribe();
        loop {
            if let Some(result) = completed.borrow_and_update().clone() {
                return result;
            }
            completed
                .changed()
                .await
                .map_err(|_| "ACP event I/O owner ended without confirmation")?;
        }
    }
}

pub(super) struct EventFence {
    shared: Arc<Shared>,
}
impl EventFence {
    pub(super) async fn wait(&self) -> Result<EventPrefix, String> {
        let through = {
            let state = self.shared.state();
            if !state.receiver_alive {
                return Err("ACP event receiver closed".into());
            }
            if let Some(error) = &state.failed {
                return Err(error.to_string());
            }
            state.accepted
        };
        let mut progress = self.shared.progress.subscribe();
        loop {
            let snapshot = progress.borrow_and_update().clone();
            if let Some(error) = snapshot.failed {
                return Err(error.to_string());
            }
            if snapshot.written >= through {
                return Ok(EventPrefix {
                    scope: self.shared.scope.clone(),
                    through,
                });
            }
            progress
                .changed()
                .await
                .map_err(|_| "ACP event I/O owner ended without a fence")?;
        }
    }
}

/// One receiver and one decoded result slot; cancelled recv futures cannot queue reads.
pub struct EventReceiver {
    shared: Arc<Shared>,
    ended: bool,
    waiting: Option<std::pin::Pin<Box<tokio::sync::futures::OwnedNotified>>>,
}
impl EventReceiver {
    pub fn remaining_through(&self, prefix: &EventPrefix) -> Result<u64, String> {
        if !Arc::ptr_eq(&self.shared.scope, &prefix.scope) {
            return Err("ACP fence belongs to another event store".into());
        }
        let state = self.shared.state();
        if let Some(error) = &state.failed {
            return Err(error.to_string());
        }
        if prefix.through > state.accepted {
            return Err("ACP fence exceeds the accepted event prefix".into());
        }
        Ok(prefix.through.saturating_sub(state.delivered))
    }
    fn take_output(&mut self) -> Result<Option<Event>, String> {
        let mut state = self.shared.state();
        if let Some(output) = state.output.take() {
            if let Some(sequence) = output.sequence {
                if state.delivered.checked_add(1) != Some(sequence) {
                    drop(state);
                    drop(output);
                    return Err("ACP event delivery sequence changed".into());
                }
                state.delivered = sequence;
            }
            if matches!(&output.event, Event::Closed(_)) {
                self.ended = true;
            }
            drop(state);
            self.shared.io_changed.notify_one();
            return Ok(Some(output.event));
        }
        if let Some(error) = &state.failed {
            self.ended = true;
            return Ok(Some(Event::Closed(Some(error.to_string()))));
        }
        if state.done {
            self.ended = true;
            return Ok(None);
        }
        state.read_requested = true;
        drop(state);
        self.shared.io_changed.notify_one();
        Ok(None)
    }
    pub fn poll_recv(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Event>> {
        use std::future::Future;
        use std::task::Poll;
        loop {
            if self.ended {
                return Poll::Ready(None);
            }
            if self.waiting.is_none() {
                let mut waiting = Box::pin(self.shared.data.clone().notified_owned());
                waiting.as_mut().enable();
                self.waiting = Some(waiting);
            }
            match self.take_output() {
                Ok(Some(event)) => {
                    self.waiting = None;
                    return Poll::Ready(Some(event));
                }
                Ok(None) if self.ended => {
                    self.waiting = None;
                    return Poll::Ready(None);
                }
                Err(error) => {
                    self.waiting = None;
                    self.shared.cancel(error.clone());
                    self.ended = true;
                    return Poll::Ready(Some(Event::Closed(Some(error))));
                }
                Ok(None) => {}
            }
            if self
                .waiting
                .as_mut()
                .unwrap()
                .as_mut()
                .poll(cx)
                .is_pending()
            {
                return Poll::Pending;
            }
            self.waiting = None;
        }
    }
    pub async fn recv(&mut self) -> Option<Event> {
        std::future::poll_fn(|cx| self.poll_recv(cx)).await
    }
    pub fn try_recv(&mut self) -> Result<Event, tokio::sync::mpsc::error::TryRecvError> {
        use tokio::sync::mpsc::error::TryRecvError;
        if self.ended {
            return Err(TryRecvError::Disconnected);
        }
        match self.take_output() {
            Ok(Some(event)) => {
                self.waiting = None;
                Ok(event)
            }
            Ok(None) if self.ended => {
                self.waiting = None;
                Err(TryRecvError::Disconnected)
            }
            Ok(None) => Err(TryRecvError::Empty),
            Err(error) => {
                self.waiting = None;
                self.shared.cancel(error.clone());
                self.ended = true;
                Ok(Event::Closed(Some(error)))
            }
        }
    }
}
impl Drop for EventReceiver {
    fn drop(&mut self) {
        self.shared.state().receiver_alive = false;
        self.shared.cancel("ACP event receiver closed".into());
    }
}

/// Serde runs on the same I/O worker. Even a large string crosses cancellation
/// checks as its source is consumed; no decode/reassembly runs in recv/try_recv.
struct DecodeInput<'a> {
    bytes: &'a [u8],
    cancelled: &'a AtomicBool,
}
impl Read for DecodeInput<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        check_cancel(self.cancelled)?;
        let amount = buffer.len().min(self.bytes.len()).min(IO_CHUNK);
        buffer[..amount].copy_from_slice(&self.bytes[..amount]);
        self.bytes = &self.bytes[amount..];
        Ok(amount)
    }
}
fn decode<T: serde::de::DeserializeOwned>(
    payload: &[u8],
    cancelled: &AtomicBool,
) -> Result<T, String> {
    let source = DecodeInput {
        bytes: payload,
        cancelled,
    };
    let buffered = io::BufReader::with_capacity(IO_CHUNK, source);
    let decoded = serde_json::from_reader(buffered).map_err(|error| error.to_string())?;
    check_cancel(cancelled).map_err(|error| error.to_string())?;
    Ok(decoded)
}

enum Work {
    Write(Queued),
    Read,
    TerminalWritten(u64),
    Finish(SourceEnd),
    Cancel,
}
fn next_work(shared: &Shared, prefer_read: bool) -> Work {
    let mut state = shared.state();
    loop {
        if shared.cancelled.load(Ordering::Acquire) {
            return Work::Cancel;
        }
        if let Some(end) = &state.end {
            if end.confirmed() && state.queue.is_empty() && state.written >= end.data_through() {
                if let Some(sequence) = end.event_sequence() {
                    if state.written < sequence {
                        return Work::TerminalWritten(sequence);
                    }
                }
                if state.delivered >= end.data_through() && state.output.is_none() {
                    return Work::Finish(end.clone());
                }
            }
        }
        let readable = state.read_requested
            && state.output.is_none()
            && state.written.min(
                state
                    .end
                    .as_ref()
                    .map_or(u64::MAX, |end| end.data_through()),
            ) > state.delivered;
        if readable && (prefer_read || state.queue.is_empty()) {
            state.read_requested = false;
            return Work::Read;
        }
        if let Some(item) = state.queue.pop_front() {
            return Work::Write(item);
        }
        state = shared
            .io_changed
            .wait(state)
            .unwrap_or_else(|poison| poison.into_inner());
    }
}
fn publish_written(shared: &Shared, sequence: u64) {
    let mut state = shared.state();
    state.written = sequence;
    let progress = Progress {
        written: sequence,
        failed: state.failed.clone(),
    };
    drop(state);
    shared.merge_progress(progress);
}
fn read_event(
    shared: &Shared,
    record: Record,
    interactions: &mut std::collections::HashMap<u64, Event>,
) -> Result<Output, String> {
    let event = match record.kind {
        RecordKind::Note => {
            let note: SessionNotification = decode(&record.payload, &shared.cancelled)?;
            let session = shared
                .state()
                .session
                .clone()
                .ok_or("ACP event store has no native session binding")?;
            if note.session_id != session {
                return Err("ACP event belongs to another native session".into());
            }
            Event::Note(note)
        }
        RecordKind::Ready => Event::Ready {
            session: shared
                .state()
                .session
                .clone()
                .ok_or("ACP opening has no native session binding")?,
        },
        RecordKind::ElicitDone => Event::ElicitDone(decode(&record.payload, &shared.cancelled)?),
        RecordKind::Interaction => interactions
            .remove(&record.sequence)
            .ok_or("ACP interaction marker has no live question")?,
    };
    Ok(Output {
        sequence: Some(record.sequence),
        event,
    })
}

enum WorkerEnd {
    Source(SourceEnd),
    Cancelled,
    Failed(String),
}
fn run_io(shared: Arc<Shared>) {
    let mut interactions = std::collections::HashMap::with_capacity(CONTROL_ITEMS);
    let mut spool = match Spool::create(&shared.cancelled) {
        Ok(spool) => Some(spool),
        Err(error) => {
            let end = if shared.cancelled.load(Ordering::Acquire) {
                WorkerEnd::Cancelled
            } else {
                WorkerEnd::Failed(error.to_string())
            };
            complete_io(&shared, None, interactions, end);
            return;
        }
    };
    let mut prefer_read = true;
    let end = loop {
        match next_work(&shared, prefer_read) {
            Work::Write(item) => {
                let result = spool
                    .as_mut()
                    .unwrap()
                    .append(&item.record, &shared.cancelled);
                if let Err(error) = result {
                    break if shared.cancelled.load(Ordering::Acquire) {
                        WorkerEnd::Cancelled
                    } else {
                        WorkerEnd::Failed(error.to_string())
                    };
                }
                let sequence = item.record.sequence;
                if let Some(event) = item.interaction {
                    if interactions.insert(sequence, event).is_some() {
                        break WorkerEnd::Failed("ACP interaction sequence was reused".into());
                    }
                }
                publish_written(&shared, sequence);
                // Remaining fields (including encoded credit) drop after the
                // physical write and publish. A read request never steals this.
                prefer_read = true;
            }
            Work::Read => {
                let record = match spool.as_mut().unwrap().next(&shared.cancelled) {
                    Ok(Some(record)) => record,
                    Ok(None) => {
                        break WorkerEnd::Failed("ACP committed event prefix has no record".into())
                    }
                    Err(error) => {
                        break if shared.cancelled.load(Ordering::Acquire) {
                            WorkerEnd::Cancelled
                        } else {
                            WorkerEnd::Failed(error.to_string())
                        }
                    }
                };
                let output = match read_event(&shared, record, &mut interactions) {
                    Ok(output) => output,
                    Err(error) => {
                        break if shared.cancelled.load(Ordering::Acquire) {
                            WorkerEnd::Cancelled
                        } else {
                            WorkerEnd::Failed(error)
                        }
                    }
                };
                let mut state = shared.state();
                if shared.cancelled.load(Ordering::Acquire) {
                    drop(state);
                    drop(output);
                    break WorkerEnd::Cancelled;
                }
                if state.output.is_some() {
                    drop(state);
                    drop(output);
                    break WorkerEnd::Failed("ACP decoded result slot is occupied".into());
                }
                state.output = Some(output);
                drop(state);
                shared.data.notify_waiters();
                prefer_read = false;
            }
            Work::TerminalWritten(sequence) => publish_written(&shared, sequence),
            Work::Finish(end) => break WorkerEnd::Source(end),
            Work::Cancel => break WorkerEnd::Cancelled,
        }
    };
    complete_io(&shared, spool.take(), interactions, end);
}
fn complete_io(
    shared: &Shared,
    spool: Option<Spool>,
    interactions: std::collections::HashMap<u64, Event>,
    end: WorkerEnd,
) {
    let failed = matches!(&end, WorkerEnd::Cancelled | WorkerEnd::Failed(_));
    if let WorkerEnd::Failed(error) = &end {
        // Reject/wake every producer before cleanup, not after a filesystem
        // operation that may take time. No later record can enter a dead job.
        shared.cancel(error.clone());
    }
    let (queued, output) = {
        let mut state = shared.state();
        let queued = std::mem::take(&mut state.queue);
        let output = if failed { state.output.take() } else { None };
        (queued, output)
    };
    // These destructors return permits/cancel questions. Never run them under
    // the shared mutex, and never report I/O shutdown while they remain queued.
    drop(queued);
    drop(output);
    drop(interactions);
    let cleanup = spool
        .map_or(Ok(()), Spool::cleanup)
        .map_err(|error| format!("ACP event spool cleanup failed: {error}"));
    let (mut resource_result, closed) = match end {
        WorkerEnd::Source(SourceEnd::Eof { .. }) => (Ok(()), None),
        WorkerEnd::Source(SourceEnd::Closed {
            sequence, error, ..
        }) => (
            Ok(()),
            Some(Output {
                sequence: Some(sequence),
                event: Event::Closed(error.map(|error| error.to_string())),
            }),
        ),
        WorkerEnd::Source(SourceEnd::Closing { .. }) => {
            (Err("ACP shutdown fact was not confirmed".into()), None)
        }
        WorkerEnd::Cancelled => (
            Ok(()),
            Some(Output {
                sequence: None,
                event: Event::Closed(None),
            }),
        ),
        WorkerEnd::Failed(error) => (
            Err(error.clone()),
            Some(Output {
                sequence: None,
                event: Event::Closed(Some(error)),
            }),
        ),
    };
    if let Err(error) = cleanup {
        resource_result = Err(error);
    }
    let mut state = shared.state();
    if let Err(error) = &resource_result {
        if state.failed.is_none() {
            state.failed = Some(Arc::from(error.as_str()));
        }
    }
    let closed = match &resource_result {
        Err(error) => Some(Output {
            sequence: None,
            event: Event::Closed(Some(error.clone())),
        }),
        Ok(()) => closed,
    };
    if state.receiver_alive {
        state.output = closed;
    }
    state.done = true;
    let progress = Progress {
        written: state.written,
        failed: state.failed.clone(),
    };
    drop(state);
    shared.budget.close();
    shared.merge_progress(progress);
    shared.completed.send_replace(Some(resource_result));
    shared.wake_all();
}

/// Dropping the runtime before this future is ever polled still cancels the
/// physical worker and wakes all waits. It is not a second event owner.
struct RuntimeGuard {
    shared: Arc<Shared>,
    armed: bool,
}
impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        if self.armed {
            self.shared
                .cancel("ACP event I/O runtime was cancelled".into());
        }
    }
}
/// A queued-but-unstarted blocking job also owns a failure receipt.
struct WorkerGuard {
    shared: Arc<Shared>,
    armed: bool,
    started: bool,
}
impl Drop for WorkerGuard {
    fn drop(&mut self) {
        if self.armed {
            let reason = if self.started {
                "ACP event I/O worker ended without confirmation"
            } else {
                "ACP event I/O worker was cancelled before starting"
            };
            self.shared.cancel(reason.into());
            let end = if self.started {
                WorkerEnd::Failed(reason.into())
            } else {
                WorkerEnd::Cancelled
            };
            complete_io(&self.shared, None, std::collections::HashMap::new(), end);
        }
    }
}

pub(super) fn channel() -> (Sender, EventReceiver, EventFence) {
    let shared = Shared::new();
    let producer = Arc::new(ProducerScope(Arc::downgrade(&shared)));
    let sender = Sender {
        shared: shared.clone(),
        _producer: producer,
    };
    let worker_guard = WorkerGuard {
        shared: shared.clone(),
        armed: true,
        started: false,
    };
    let worker_shared = shared.clone();
    let worker = tokio::task::spawn_blocking(move || {
        let mut guard = worker_guard;
        guard.started = true;
        run_io(worker_shared);
        guard.armed = false;
    });
    // Construct outside the future, including the unpolled-runtime case.
    let runtime_guard = RuntimeGuard {
        shared: shared.clone(),
        armed: true,
    };
    tokio::spawn(async move {
        let mut guard = runtime_guard;
        if let Err(error) = worker.await {
            guard
                .shared
                .cancel(format!("ACP event I/O worker failed: {error}"));
        }
        guard.armed = false;
    });
    (
        sender,
        EventReceiver {
            shared: shared.clone(),
            ended: false,
            waiting: None,
        },
        EventFence { shared },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::Poll;

    #[tokio::test]
    async fn cancellation_wakes_a_waiting_receiver_and_the_idle_physical_job() {
        let (source, mut output, fence) = channel();
        let mut receiving = Box::pin(output.recv());
        std::future::poll_fn(|cx| {
            assert!(receiving.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        source.cancel("owned idle cancellation".into());
        assert!(
            matches!(tokio::time::timeout(std::time::Duration::from_secs(1), receiving)
            .await.unwrap(), Some(Event::Closed(Some(error))) if error == "owned idle cancellation")
        );
        assert!(fence
            .wait()
            .await
            .unwrap_err()
            .contains("owned idle cancellation"));
        tokio::time::timeout(std::time::Duration::from_secs(1), source.shutdown())
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn a_delivered_question_keeps_its_credit_until_answer_or_drop() {
        let (source, mut output, fence) = channel();
        let session = SessionId::new("owned-questions".to_owned());
        source.ready(session.clone()).unwrap();
        assert!(matches!(output.recv().await, Some(Event::Ready { .. })));
        let request: super::super::CreateElicitationRequest =
            serde_json::from_value(serde_json::json!({
                "mode":"form","sessionId":session,"message":"Choose",
                "requestedSchema":{"type":"object","properties":{}}
            }))
            .unwrap();
        let mut answers = Vec::new();
        for _ in 0..CONTROL_ITEMS {
            let credit = source.interaction_credit(&request).unwrap();
            let (answer, received) = tokio::sync::oneshot::channel();
            answers.push(received);
            source
                .interaction(Event::Elicit(super::super::Elicitation {
                    request: request.clone(),
                    answer,
                    _store_credit: credit,
                }))
                .unwrap();
        }
        let prefix = fence.wait().await.unwrap();
        let mut delivered = Vec::new();
        for _ in 0..CONTROL_ITEMS {
            match output.recv().await.unwrap() {
                Event::Elicit(question) => delivered.push(question),
                _ => panic!("pending question lost its actual event"),
            }
        }
        assert_eq!(output.remaining_through(&prefix).unwrap(), 0);
        assert!(
            source.interaction_credit(&request).is_err(),
            "dequeue released a question still owned by the UI"
        );
        delivered
            .pop()
            .unwrap()
            .answer(super::super::CreateElicitationResponse::new(
                super::super::ElicitationAction::Cancel,
            ))
            .unwrap();
        assert!(answers.pop().unwrap().await.is_ok());
        drop(source.interaction_credit(&request).unwrap());
        drop(delivered);
        for answer in answers {
            assert!(answer.await.is_err());
        }
        let credits: Vec<_> = (0..CONTROL_ITEMS)
            .map(|_| source.interaction_credit(&request).unwrap())
            .collect();
        drop(credits);
        drop(output);
        source.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn a_legal_giant_record_is_spilled_and_decoded_outside_the_receiver() {
        let (source, mut output, fence) = channel();
        let session = SessionId::new("owned-giant-record".to_owned());
        source.ready(session.clone()).unwrap();
        assert!(matches!(output.recv().await, Some(Event::Ready { .. })));
        let bytes = 9 * 1024 * 1024;
        let note: SessionNotification = serde_json::from_value(serde_json::json!({
            "sessionId":session,"update":{"sessionUpdate":"agent_message_chunk",
                "content":{"type":"text","text":"g".repeat(bytes)}}
        }))
        .unwrap();
        source.note(&note).await.unwrap();
        drop(note);
        let prefix = fence.wait().await.unwrap();
        assert_eq!(output.remaining_through(&prefix).unwrap(), 1);
        let stats = source.shared.budget.stats();
        assert!(stats.largest_exclusive_record > bytes);
        assert_eq!(stats.peak_encoded_bytes, stats.largest_exclusive_record);
        assert_eq!(stats.peak_records, 1);
        let event = tokio::time::timeout(std::time::Duration::from_secs(5), output.recv())
            .await
            .unwrap()
            .unwrap();
        match event {
            Event::Note(note) => {
                let note = serde_json::to_value(note).unwrap();
                let text = note["update"]["content"]["text"].as_str().unwrap();
                assert_eq!(text.len(), bytes);
                assert!(text.bytes().all(|byte| byte == b'g'));
            }
            _ => panic!("giant record was replaced or truncated"),
        }
        assert_eq!(output.remaining_through(&prefix).unwrap(), 0);
        drop(output);
        source.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn cancelled_prefix_cannot_be_revived_by_a_late_write_update() {
        // Controlled ledger timing, not a claim that a real disk write occurred.
        // One accepted prefix awaits its I/O notification while cancel races it.
        let shared = Shared::new();
        shared.state().accepted = 1;
        let fence = EventFence {
            shared: shared.clone(),
        };
        let mut waiting = Box::pin(fence.wait());
        std::future::poll_fn(|cx| {
            assert!(waiting.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        shared.state().written = 1;
        shared.cancel("fixture cancelled".into());
        shared.merge_progress(Progress {
            written: 1,
            failed: None,
        });
        assert!(waiting.await.unwrap_err().contains("cancelled"));
        assert!(shared.progress.borrow().failed.is_some());
    }
}
