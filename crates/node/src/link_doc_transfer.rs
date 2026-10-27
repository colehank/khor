//! The normal sync pump: owned bounded wire values cross awaits; Loaded,
//! metadata leases and blob owners exist only in admitted blocking steps.
use super::{Line, SyncProgress};
use crate::{
    Node,
    doc_transfer_state::{self as state, Context, Error, Notice, State},
    proto::{
        self, DocAckResult, DocAcknowledgement, DocChunk, DocManifest, DocTransferAction as Action,
        DocTransferFailure as Failure, DocTransferReply as Reply,
        DocTransferState as TransferState, DocumentInventory, Request, Response,
    },
};
use khor_sync::{doc_transfer::DocumentVersion, transfer::CHUNK_BYTES};
use std::time::{Duration, Instant};

static WORK: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
static WAITING: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(8);
const VISIT_BYTES: usize = 32 * 1024 * 1024;
const VISIT_TIME: Duration = Duration::from_secs(30);
type PairRoute = Option<([u8; 32], [u8; 32])>;

fn fits_step_budget(starting: bool, charged_bytes: usize, quantum_chunks: usize) -> bool {
    let reserved_chunks = if starting { 2 } else { 1 };
    quantum_chunks + reserved_chunks <= 4
        && charged_bytes + reserved_chunks * CHUNK_BYTES <= VISIT_BYTES
}

/// Known body bytes contribute to public progress. Unknown attempts only
/// consume capacity; they never claim that a peer received or imported data.
#[derive(Default)]
struct Usage {
    known_bytes: usize,
    uncertain_bytes: usize,
    chunks: usize,
}
impl Usage {
    fn known(&mut self, bytes: usize) {
        self.known_bytes += bytes;
        self.chunks += bytes.div_ceil(CHUNK_BYTES);
    }
    fn uncertain(&mut self, bytes: usize) {
        self.uncertain_bytes += bytes;
        self.chunks += bytes.div_ceil(CHUNK_BYTES);
    }
    fn charged(&self) -> usize {
        self.known_bytes + self.uncertain_bytes
    }
    fn add(&mut self, other: &Self) {
        self.known_bytes += other.known_bytes;
        self.uncertain_bytes += other.uncertain_bytes;
        self.chunks += other.chunks;
    }
}

struct StepOutcome {
    usage: Usage,
    result: Result<(), Error>,
}

async fn blocking<T: Send + 'static>(
    context: Context,
    job: impl FnOnce(&Context, &mut State) -> Result<T, Error> + Send + 'static,
) -> Result<(T, Vec<Notice>), Error> {
    on_worker(context, move |context| {
        let mut state = State::open(context)?;
        let value = job(context, &mut state)?;
        let notices = state.notice_ids(context)?;
        Ok((value, notices))
    }).await
}

pub(super) async fn on_worker<T: Send + 'static>(
    context: Context,
    job: impl FnOnce(&Context) -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    // Queue at most eight owned jobs, and spawn only after the single heavy
    // seat is acquired. A transient local step must not skip Seen/Pins just
    // because an unrelated admitted step is finishing its flush.
    let waiting = WAITING.try_acquire().map_err(|_| Error::busy())?;
    let permit = WORK.acquire().await.map_err(|_| Error::busy())?;
    drop(waiting);
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        job(&context)
    })
    .await
    .map_err(|e| format!("document transfer worker: {e}"))?
}

impl Node {
    pub(super) fn doc_context(&self) -> Result<Context, Error> {
        Ok(Context {
            root: self.root().clone(),
            writer: self.writer_peer(),
            local: khor_net::trust::id_from_hex(self.device_str())
                .ok_or("invalid local document identity")?,
        })
    }
    pub(super) async fn doc_step<T: Send + 'static>(
        &self,
        job: impl FnOnce(&Context, &mut State) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let context = self.doc_context()?;
        let (value, notices) = blocking(context.clone(), job).await?;
        self.deliver_doc_notices(context, notices).await?;
        Ok(value)
    }
    pub(super) async fn deliver_doc_notices(&self, context: Context, notices: Vec<Notice>) -> Result<(), Error> {
        for Notice {
            id,
            document: name,
            remote,
            retired_pen,
        } in notices
        {
            if let Some(pen) = retired_pen {
                let peer = khor_net::trust::id_hex(&remote);
                self.wires.note(
                    Some(&peer),
                    crate::journal::What::PenRetired {
                        doc: name.clone(),
                        name: format!("{pen:016x}"),
                    },
                );
            }
            if name == "devices" {
                self.follow_after_sync();
            }
            if name == crate::wide::BROADCAST {
                self.take_arrived();
            }
            blocking(context.clone(), move |_, state| state.mark_notified(&id)).await?;
        }
        Ok(())
    }
    pub(super) async fn answer_doc_transfer(&self, remote: &str, action: Action) -> Reply {
        let Some(remote) = khor_net::trust::id_from_hex(remote) else {
            return failed("invalid remote document identity".into());
        };
        let local = match self.doc_context() {
            Ok(context) => context.local,
            Err(error) => return failed(error),
        };
        if let Err(error) = validate_roles(&action, local, remote) {
            return failed(error);
        }
        if let Err(why) = validate_action(&action) {
            return failed(Error {
                kind: Failure::Invalid,
                why: why.why,
            });
        }
        match self
            .doc_step(move |context, state| handle(context, state, remote, action))
            .await
        {
            Ok(reply) => reply,
            Err(why) => failed(why),
        }
    }
    pub(super) async fn pending_doc_transfers(&self, remote: &str) -> Result<Vec<String>, Error> {
        let remote =
            khor_net::trust::id_from_hex(remote).ok_or("invalid remote document identity")?;
        self.doc_step(move |_, state| state.pending_docs(remote))
            .await
    }
    async fn doc_exchange(
        &self,
        line: &Line,
        action: Action,
        usage: &mut Usage,
        route: PairRoute,
    ) -> Result<Reply, Error> {
        validate_action(&action)?;
        let local = self.doc_context()?.local;
        let remote =
            khor_net::trust::id_from_hex(&line.id).ok_or("invalid remote document identity")?;
        validate_roles(&action, remote, local)?;
        let sent = match &action {
            Action::Exchange {
                offer: Some(chunk), ..
            }
            | Action::Write { chunk } => chunk.bytes.len(),
            _ => 0,
        };
        let possible_reply = if matches!(&action, Action::Exchange { .. } | Action::Read { .. }) {
            CHUNK_BYTES
        } else {
            0
        };
        let mut attempt = super::DocAttempt::default();
        let request = match route {
            None => Request::DocTransfer { action },
            Some((claim, nonce)) => Request::Pair {
                action: crate::proto::PairAction::Devices { claim, nonce, action },
            },
        };
        let response = self
            .exchange_doc_on(line, &request, &mut attempt)
            .await;
        // Preserve both directions before transport/decode/role/shape/local
        // errors can return. Every repeated application request counts again.
        if attempt.request_written {
            usage.known(sent);
        } else if attempt.started {
            usage.uncertain(sent);
        }
        if let Some(bytes) = attempt.response_body {
            usage.known(bytes);
        } else if attempt.started {
            usage.uncertain(possible_reply);
        }
        let reply = match (route, response.map_err(super::Miss::words)?) {
            (None, Response::DocTransfer { reply }) => reply,
            (Some((expected, _)), Response::Paired { reply: crate::proto::PairReply::Devices { claim, reply } }) if claim == expected => reply,
            (_, Response::Refused { why }) => return Err(why.into()),
            _ => return Err("unexpected document route or response".into()),
        };
        validate_reply_roles(&reply, local, remote)?;
        Ok(reply)
    }
    pub(super) async fn resolve_doc_inventory(
        &self,
        line: &Line,
        request: &Request,
        rows: Vec<(String, DocumentInventory)>,
    ) -> Result<Vec<(String, DocumentInventory)>, Error> {
        let Request::Versions { docs: asked, .. } = request else {
            return Err("invalid inventory opening".into());
        };
        let mut resolved = std::collections::BTreeMap::new();
        for (name, row) in rows {
            let row = if !asked.iter().any(|(selected, _)| selected == &name) {
                DocumentInventory::Failed {
                    why: "peer declared an unselected document".into(),
                }
            } else {
                row
            };
            if resolved.insert(name.clone(), row).is_some() {
                resolved.insert(
                    name,
                    DocumentInventory::Failed {
                        why: "peer declared a duplicate document".into(),
                    },
                );
            }
        }
        loop {
            let pending: Vec<_> = resolved
                .iter()
                .filter_map(|(name, row)| {
                    matches!(row, DocumentInventory::Deferred).then_some(name.clone())
                })
                .take(16)
                .collect();
            if pending.is_empty() {
                return Ok(resolved.into_iter().collect());
            }
            // Empty digest forces a concrete current version/error. This
            // smaller page must retire at least one selected name.
            let question = Request::Versions {
                docs: pending
                    .iter()
                    .map(|name| (name.clone(), serde_bytes::ByteBuf::new()))
                    .collect(),
                readings: Vec::new(),
            };
            let reply = self
                .exchange_on(line, &question)
                .await
                .map_err(super::Miss::words)?;
            let (rows, _) = super::versions_answer(reply)?;
            let mut seen = std::collections::BTreeSet::new();
            let mut advanced = 0;
            for (name, row) in rows {
                if !pending.contains(&name) || !seen.insert(name.clone()) {
                    return Err("invalid inventory continuation names".into());
                }
                if !matches!(row, DocumentInventory::Deferred) {
                    advanced += 1;
                }
                resolved.insert(name, row);
            }
            if advanced == 0 || seen.len() != pending.len() {
                return Err(
                    "inventory continuation made no progress or omitted a selected name".into(),
                );
            }
        }
    }
    pub(super) async fn transfer_documents(
        &self,
        line: &Line,
        docs: Vec<(String, DocumentInventory)>,
    ) -> SyncProgress {
        self.transfer_documents_route(line, docs, None).await
    }
    pub(super) async fn transfer_pair_documents(
        &self, line: &Line, version: serde_bytes::ByteBuf, claim: [u8; 32], nonce: [u8; 32],
    ) -> SyncProgress {
        self.transfer_documents_route(line, vec![("devices".into(), DocumentInventory::Version { version })], Some((claim, nonce))).await
    }
    async fn transfer_documents_route(
        &self, line: &Line, docs: Vec<(String, DocumentInventory)>, route: PairRoute,
    ) -> SyncProgress {
        let Some(remote) = khor_net::trust::id_from_hex(&line.id) else {
            let mut p = SyncProgress::default();
            p.fail("invalid remote document identity".into());
            return p;
        };
        let mut work: Vec<_> = docs
            .into_iter()
            .map(|(name, inventory)| match inventory {
                DocumentInventory::Version { version } => match DocumentVersion::decode(&version) {
                    Ok(have) => Work {
                        name,
                        have: Some(have),
                        started: false,
                        push: None,
                        push_at: 0,
                        pull: None,
                        failure: None,
                        apart: false,
                    },
                    Err(why) => Work::failed(name, why.to_string()),
                },
                DocumentInventory::Failed { why } => Work::failed(name, why),
                DocumentInventory::Deferred => {
                    Work::failed(name, "unresolved document inventory continuation".into())
                }
            })
            .collect();
        let began = Instant::now();
        let mut usage = Usage::default();
        while work.iter().any(|w| !w.done())
            && usage.charged() < VISIT_BYTES
            && began.elapsed() < VISIT_TIME
        {
            let mut advanced = false;
            for item in &mut work {
                if item.done() {
                    continue;
                }
                let mut quantum_bytes = 0usize;
                let mut quantum_chunks = 0usize;
                #[cfg(test)]
                flow_tests::quantum(self.root(), &item.name);
                while !item.done()
                    && quantum_chunks < 4
                    && quantum_bytes < 4 * CHUNK_BYTES
                    && usage.charged() < VISIT_BYTES
                    && began.elapsed() < VISIT_TIME
                {
                    let reserved_chunks = if item.started { 1 } else { 2 };
                    if !fits_step_budget(!item.started, usage.charged(), quantum_chunks)
                        || quantum_bytes + reserved_chunks * CHUNK_BYTES > 4 * CHUNK_BYTES
                    {
                        break;
                    }
                    #[cfg(test)]
                    let kind = if !item.started {
                        flow_tests::StepKind::Exchange
                    } else if item.push.is_some() {
                        flow_tests::StepKind::Write
                    } else if item.pull.is_some() {
                        flow_tests::StepKind::Read
                    } else {
                        flow_tests::StepKind::Idle
                    };
                    #[cfg(test)]
                    flow_tests::before_step(self.root(), &item.name, kind).await;
                    let step = self.transfer_step(line, remote, item, route).await;
                    usage.add(&step.usage);
                    quantum_bytes += step.usage.charged();
                    quantum_chunks += step.usage.chunks;
                    #[cfg(test)]
                    flow_tests::after_step(
                        self.root(),
                        &item.name,
                        kind,
                        step.result.as_ref().err().map(|e| e.kind.clone()),
                    )
                    .await;
                    match step.result {
                        Ok(()) => advanced = true,
                        Err(why) if why.kind == Failure::Busy => break,
                        Err(why) => {
                            item.failure = Some(why.why);
                            break;
                        }
                    }
                    // A control-only step must not monopolize this document.
                    if quantum_chunks == 0 {
                        break;
                    }
                }
                tokio::task::yield_now().await;
            }
            if !advanced {
                break;
            }
        }
        let cleanup = self
            .flush_doc_receipts(line, remote, began + VISIT_TIME, &mut usage, route)
            .await;
        let mut progress = SyncProgress::default();
        progress.moved = usage.known_bytes;
        // Aggregate by fixed registry/selected-document order, never by the
        // time a later fair-pump request happened to finish.
        for item in work {
            let pending = !item.done();
            if let Some(note) =
                crate::journal::sync_failed(&line.id, &item.name, item.failure.as_deref())
            {
                self.wires.note(Some(&line.id), note);
            }
            if let Some(why) = item.failure {
                progress.fail(why);
            } else if item.apart {
                progress.apart.push(item.name);
            } else if pending {
                progress.pending.push(item.name);
            }
        }
        if let Err(why) = cleanup {
            if why.kind == Failure::Busy {
                progress.pending.push("receipts".into());
            } else {
                progress.fail(why.why);
            }
        }
        progress
    }
    async fn transfer_step(&self, line: &Line, remote: [u8; 32], work: &mut Work, route: PairRoute) -> StepOutcome {
        let mut usage = Usage::default();
        let result = self
            .transfer_step_inner(line, remote, work, &mut usage, route)
            .await;
        StepOutcome { usage, result }
    }
    async fn transfer_step_inner(
        &self,
        line: &Line,
        remote: [u8; 32],
        work: &mut Work,
        usage: &mut Usage,
        route: PairRoute,
    ) -> Result<(), Error> {
        if !work.started {
            let name = work.name.clone();
            let have = work
                .have
                .clone()
                .ok_or("missing document inventory version")?;
            let packet = self
                .doc_step(move |context, state| {
                    let local = context.version(&name)?;
                    let offer = if local.epoch == have.epoch {
                        match route { Some((operation, _)) => state.pair_offer(context, remote, operation, &have)?, None => state.offer(context, remote, &name, &have)? }
                    } else {
                        None
                    };
                    let (proofs, acknowledgements) = match route {
                        Some((operation, _)) => (
                            state.pair_document_proofs(remote, operation, 1)?,
                            state.pair_document_acknowledgements(remote, operation, 1)?,
                        ),
                        None => (
                            state.pending_proofs(remote, 1),
                            state.pending_acknowledgements(remote, 1)?,
                        ),
                    };
                    Ok((
                        local.encode().map_err(|e| e.to_string())?,
                        offer,
                        proofs,
                        acknowledgements,
                    ))
                })
                .await?;
            let (have, offer, proofs, acknowledgements) = packet;
            let sent_manifest = offer.as_ref().map(|c| c.manifest.clone());
            let reply = self
                .doc_exchange(
                    line,
                    Action::Exchange {
                        document: work.name.clone(),
                        have: have.into(),
                        offer,
                        proofs,
                        acknowledgements,
                    },
                    usage,
                    route,
                )
                .await?;
            let Reply::Exchanged {
                document,
                have,
                received,
                offer,
                proof_acknowledgements,
                acknowledgement_results,
            } = reply
            else {
                return Err(reply_error(reply));
            };
            if document != work.name {
                return Err("document exchange answered another document".into());
            }
            let have = DocumentVersion::decode(&have).map_err(|e| e.to_string())?;
            if work
                .have
                .as_ref()
                .is_some_and(|old| old.epoch != have.epoch)
            {
                work.apart = true;
                return Ok(());
            }
            let local_epoch = self
                .doc_step({
                    let name = work.name.clone();
                    move |context, _| Ok(context.version(&name)?.epoch)
                })
                .await?;
            if have.epoch != local_epoch {
                work.apart = true;
                return Ok(());
            }
            if let Some(chunk) = &offer {
                if chunk.manifest.descriptor()?.document != work.name {
                    return Err("offered a different document".into());
                }
            }
            work.have = Some(have);
            work.push = sent_manifest;
            work.pull = offer.as_ref().map(|chunk| chunk.manifest.clone());
            // Keep the opening pending until both directions and receipts are
            // handled. A late successful assignment must not erase Missing.
            let mut needs_exchange = matches!(&received, Some(TransferState::Missing));
            #[cfg(test)]
            flow_tests::before_receipts(self.root(), &work.name).await;
            self.take_receipt_answers(remote, proof_acknowledgements, acknowledgement_results, route)
                .await?;
            if let Some(received) = received {
                self.take_push_state(remote, work, received).await?;
            }
            if let Some(chunk) = offer {
                let state = self
                    .doc_step(move |context, state| match route { Some((operation, _)) => state.pair_receive(context, remote, operation, &chunk, true), None => state.receive(context, remote, &chunk, true) })
                    .await?;
                needs_exchange |= matches!(&state, TransferState::Missing);
                take_pull_state(work, state)?;
            }
            work.started = !needs_exchange;
            return Ok(());
        }
        if let Some(manifest) = work.push.clone() {
            let at = work.push_at;
            let chunk = self
                .doc_step(move |context, state| state.read_outgoing(context, remote, &manifest, at))
                .await?;
            let reply = self
                .doc_exchange(
                    line,
                    Action::Write {
                        chunk: chunk.clone(),
                    },
                    usage,
                    route,
                )
                .await?;
            let Reply::State { manifest, state } = reply else {
                return Err(reply_error(reply));
            };
            if manifest != chunk.manifest {
                return Err("write acknowledgement names another manifest".into());
            }
            self.take_push_state(remote, work, state).await?;
            return Ok(());
        }
        if let Some(manifest) = work.pull.clone() {
            let query = manifest.clone();
            let (state, offset) = self
                .doc_step(move |context, state| {
                    Ok((
                        state.status(context, remote, &query)?,
                        state.incoming_offset(&query)?,
                    ))
                })
                .await?;
            if matches!(
                state,
                TransferState::Complete { .. } | TransferState::Apart { .. }
            ) {
                take_pull_state(work, state)?;
                return Ok(());
            }
            let reply = self
                .doc_exchange(
                    line,
                    Action::Read {
                        manifest: manifest.clone(),
                        offset,
                    },
                    usage,
                    route,
                )
                .await?;
            let Reply::Chunk { chunk } = reply else {
                return Err(reply_error(reply));
            };
            if chunk.manifest != manifest || chunk.offset != offset {
                return Err("read answered a different manifest or offset".into());
            }
            let state = self
                .doc_step(move |context, state| match route { Some((operation, _)) => state.pair_receive(context, remote, operation, &chunk, false), None => state.receive(context, remote, &chunk, false) })
                .await?;
            take_pull_state(work, state)?;
            return Ok(());
        }
        Ok(())
    }
    async fn take_push_state(
        &self,
        remote: [u8; 32],
        work: &mut Work,
        reply: TransferState,
    ) -> Result<(), Error> {
        match reply {
            TransferState::Complete { proof } => {
                let sent = work
                    .push
                    .clone()
                    .ok_or("completion does not bind the active outgoing manifest")?;
                self.doc_step(move |context, state| {
                    state.accept_sent(context, remote, &sent, &proof)
                })
                .await?;
                work.push = None;
            }
            TransferState::Stored { next_offset } => {
                let manifest = work
                    .push
                    .as_ref()
                    .ok_or("stored status without outgoing plan")?;
                if next_offset > manifest.total_bytes
                    || (next_offset != manifest.total_bytes
                        && next_offset % CHUNK_BYTES as u64 != 0)
                {
                    return Err("invalid stored document offset".into());
                }
                work.push_at = if next_offset == manifest.total_bytes && next_offset > 0 {
                    ((next_offset - 1) / CHUNK_BYTES as u64) * CHUNK_BYTES as u64
                } else {
                    next_offset
                };
            }
            TransferState::Missing => {
                work.started = false;
            }
            TransferState::Apart { .. } => work.apart = true,
            TransferState::Failed {
                kind: Failure::Busy,
                ..
            } => return Err(Error::busy()),
            TransferState::Failed { kind, why } => return Err(Error { kind, why }),
        }
        Ok(())
    }
    async fn take_receipt_answers(
        &self,
        remote: [u8; 32],
        acknowledgements: Vec<DocAcknowledgement>,
        results: Vec<DocAckResult>,
        route: PairRoute,
    ) -> Result<(), Error> {
        if acknowledgements.len() > 2 || results.len() > 2 {
            return Err("too many document receipt acknowledgements".into());
        }
        self.doc_step(move |context, state| {
            if let Some((operation, _)) = route {
                return state.pair_receipt_answers(
                    context,
                    remote,
                    operation,
                    &acknowledgements,
                    &results,
                );
            }
            for ack in acknowledgements {
                state.acknowledge(remote, &ack)?;
            }
            for result in results {
                state.acknowledged(context, remote, &result)?;
            }
            Ok(())
        })
        .await
    }
    async fn flush_doc_receipts(
        &self,
        line: &Line,
        remote: [u8; 32],
        deadline: Instant,
        usage: &mut Usage,
        route: PairRoute,
    ) -> Result<(), Error> {
        // At most64 records per direction, batch2. There is no unbounded task
        // queue and no per-document empty Begin/Commit round trip.
        for _ in 0..64 {
            if Instant::now() >= deadline {
                return Err(Error::busy());
            }
            let (proofs, acks) = self
                .doc_step(move |_, state| {
                    match route {
                        Some((operation, _)) => Ok((
                            state.pair_document_proofs(remote, operation, 2)?,
                            state.pair_document_acknowledgements(remote, operation, 2)?,
                        )),
                        None => Ok((
                            state.pending_proofs(remote, 2),
                            state.pending_acknowledgements(remote, 2)?,
                        )),
                    }
                })
                .await?;
            if proofs.is_empty() && acks.is_empty() {
                return Ok(());
            }
            if !proofs.is_empty() {
                match self
                    .doc_exchange(line, Action::Confirm { proofs }, usage, route)
                    .await?
                {
                    Reply::Confirmed { acknowledgements } => {
                        self.take_receipt_answers(remote, acknowledgements, Vec::new(), route)
                            .await?
                    }
                    other => return Err(reply_error(other)),
                }
            }
            if !acks.is_empty() {
                match self
                    .doc_exchange(
                        line,
                        Action::Acknowledge {
                            acknowledgements: acks,
                        },
                        usage,
                        route,
                    )
                    .await?
                {
                    Reply::Acknowledged { results } => {
                        self.take_receipt_answers(remote, Vec::new(), results, route)
                            .await?
                    }
                    other => return Err(reply_error(other)),
                }
            }
        }
        Err(Error::busy())
    }
}

struct Work {
    name: String,
    have: Option<DocumentVersion>,
    started: bool,
    push: Option<DocManifest>,
    push_at: u64,
    pull: Option<DocManifest>,
    failure: Option<String>,
    apart: bool,
}
impl Work {
    fn failed(name: String, why: String) -> Self {
        Self {
            name,
            have: None,
            started: true,
            push: None,
            push_at: 0,
            pull: None,
            failure: Some(why),
            apart: false,
        }
    }
    fn done(&self) -> bool {
        self.failure.is_some()
            || self.apart
            || (self.started && self.push.is_none() && self.pull.is_none())
    }
}
fn take_pull_state(work: &mut Work, state: TransferState) -> Result<(), Error> {
    match state {
        TransferState::Complete { proof } => {
            if work.pull.as_ref() != Some(&proof.manifest) {
                return Err("incoming completion changed its manifest".into());
            }
            work.pull = None;
        }
        TransferState::Stored { .. } => {}
        TransferState::Missing => {
            work.started = false;
        }
        TransferState::Apart { .. } => work.apart = true,
        TransferState::Failed {
            kind: Failure::Busy,
            ..
        } => return Err(Error::busy()),
        TransferState::Failed { kind, why } => return Err(Error { kind, why }),
    }
    Ok(())
}
fn failed(error: Error) -> Reply {
    Reply::Failed {
        kind: error.kind,
        why: bounded_detail(error.why),
    }
}
fn bounded_detail(mut why: String) -> String {
    if why.len() > 4096 {
        let mut boundary = 4096 - "…".len();
        while !why.is_char_boundary(boundary) {
            boundary -= 1;
        }
        why.truncate(boundary);
        why.push('…');
    }
    why
}
fn reply_error(reply: Reply) -> Error {
    match reply {
        Reply::Failed { kind, why } => Error { kind, why },
        _ => Error::from("unexpected document transfer response variant"),
    }
}

pub(super) fn handle(
    context: &Context,
    state: &mut State,
    remote: [u8; 32],
    action: Action,
) -> Result<Reply, Error> {
    handle_pair(context, state, remote, action, None)
}
pub(super) fn handle_pair(context: &Context, state: &mut State, remote: [u8;32], action: Action, operation: Option<[u8;32]>) -> Result<Reply, Error> {
    match action {
        Action::Exchange {
            document,
            have,
            offer,
            proofs,
            acknowledgements,
        } => {
            if !state::registry::authorized_name(&document) {
                return Err(khor_catalog::msg::unknown_doc(&document).into());
            }
            let mut proof_acknowledgements = Vec::new();
            for proof in proofs {
                proof_acknowledgements.push(state.accept(context, remote, &proof, true)?);
            }
            let mut acknowledgement_results = Vec::new();
            for ack in acknowledgements {
                acknowledgement_results.push(state.acknowledge(remote, &ack)?);
            }
            let received = offer.map(|chunk| {
                if chunk.offset != 0
                    || !chunk
                        .manifest
                        .descriptor()
                        .is_ok_and(|d| d.document == document)
                {
                    return TransferState::Failed {
                        kind: Failure::Invalid,
                        why: "exchange first chunk does not bind its document".into(),
                    };
                }
                match match operation { Some(id) => state.pair_receive(context, remote, id, &chunk, true), None => state.receive(context, remote, &chunk, true) } {
                    Ok(reply) => reply,
                    Err(error) => TransferState::Failed {
                        kind: error.kind,
                        why: bounded_detail(error.why),
                    },
                }
            });
            let have = DocumentVersion::decode(&have).map_err(|e| e.to_string())?;
            let local = context.version(&document)?;
            let offer = if have.epoch == local.epoch {
                match operation { Some(id) => state.pair_offer(context, remote, id, &have)?, None => state.offer(context, remote, &document, &have)? }
            } else {
                None
            };
            Ok(Reply::Exchanged {
                document,
                have: local.encode().map_err(|e| e.to_string())?.into(),
                received,
                offer,
                proof_acknowledgements,
                acknowledgement_results,
            })
        }
        Action::Write { chunk } => {
            let manifest = chunk.manifest.clone();
            let state = match operation { Some(id) => state.pair_receive(context, remote, id, &chunk, false)?, None => state.receive(context, remote, &chunk, false)? };
            Ok(Reply::State { manifest, state })
        }
        Action::Read { manifest, offset } => Ok(Reply::Chunk {
            chunk: state.read_outgoing(context, remote, &manifest, offset)?,
        }),
        Action::Status { manifest } => {
            let status = state.status(context, remote, &manifest)?;
            Ok(Reply::State {
                manifest,
                state: status,
            })
        }
        Action::Confirm { proofs } => {
            let mut acknowledgements = Vec::new();
            for proof in proofs {
                acknowledgements.push(state.accept(context, remote, &proof, true)?);
            }
            Ok(Reply::Confirmed { acknowledgements })
        }
        Action::Acknowledge { acknowledgements } => {
            let mut results = Vec::new();
            for ack in acknowledgements {
                results.push(state.acknowledge(remote, &ack)?);
            }
            Ok(Reply::Acknowledged { results })
        }
    }
}

pub(crate) fn validate_action(action: &Action) -> Result<(), Error> {
    match action {
        Action::Exchange {
            document,
            have,
            offer,
            proofs,
            acknowledgements,
        } => {
            if !state::registry::authorized_name(document) {
                return Err(khor_catalog::msg::unknown_doc(document).into());
            }
            DocumentVersion::decode(have).map_err(|e| e.to_string())?;
            if proofs.len() > 1 || acknowledgements.len() > 1 {
                return Err("exchange receipt batch exceeds one".into());
            }
            if let Some(chunk) = offer {
                validate_chunk(chunk)?;
            }
            for proof in proofs {
                proof.validate()?;
            }
            for ack in acknowledgements {
                ack.manifest.core()?;
            }
        }
        Action::Write { chunk } => validate_chunk(chunk)?,
        Action::Read { manifest, offset } => {
            let total = manifest.core()?.total_bytes;
            if *offset > total || (*offset != total && *offset % CHUNK_BYTES as u64 != 0) {
                return Err("noncanonical document read offset".into());
            }
        }
        Action::Status { manifest } => {
            manifest.core()?;
        }
        Action::Confirm { proofs } => {
            if proofs.len() > 2 {
                return Err("document proof batch exceeds two".into());
            }
            for proof in proofs {
                proof.validate()?;
            }
        }
        Action::Acknowledge { acknowledgements } => {
            if acknowledgements.len() > 2 {
                return Err("document ack batch exceeds two".into());
            }
            for ack in acknowledgements {
                ack.manifest.core()?;
            }
        }
    }
    if proto::encode(&Request::DocTransfer {
        action: action.clone(),
    })?
    .len()
        > proto::DOC_TRANSFER_FRAME_BYTES
    {
        return Err("document transfer frame exceeds 1 MiB".into());
    }
    Ok(())
}
fn validate_chunk(chunk: &DocChunk) -> Result<(), Error> {
    let total = chunk.manifest.core()?.total_bytes;
    if chunk.offset > total
        || (chunk.offset != total && chunk.offset % CHUNK_BYTES as u64 != 0)
        || chunk.bytes.len() as u64 != (total - chunk.offset).min(CHUNK_BYTES as u64)
    {
        return Err("noncanonical document chunk".into());
    }
    Ok(())
}

pub(crate) fn validate_reply(reply: &Reply) -> Result<(), Error> {
    fn state(value: &TransferState) -> Result<(), Error> {
        match value {
            TransferState::Complete { proof } => proof.validate(),
            TransferState::Failed { why, .. } if why.len() > 4096 => {
                Err("document error detail exceeds budget".into())
            }
            _ => Ok(()),
        }
    }
    fn acks(values: &[DocAcknowledgement], limit: usize) -> Result<(), Error> {
        if values.len() > limit {
            return Err("document acknowledgement batch exceeds budget".into());
        }
        for ack in values {
            ack.manifest.core()?;
        }
        Ok(())
    }
    fn results(values: &[DocAckResult], limit: usize) -> Result<(), Error> {
        if values.len() > limit {
            return Err("document acknowledgement result batch exceeds budget".into());
        }
        for result in values {
            match result {
                DocAckResult::Released { acknowledgement }
                | DocAckResult::Absent { acknowledgement } => {
                    acknowledgement.manifest.core()?;
                }
                DocAckResult::Reconfirm { proof } => proof.validate()?,
            }
        }
        Ok(())
    }
    match reply {
        Reply::Exchanged {
            document,
            have,
            received,
            offer,
            proof_acknowledgements,
            acknowledgement_results,
        } => {
            if !state::registry::authorized_name(document) {
                return Err("unknown exchanged document".into());
            }
            DocumentVersion::decode(have).map_err(|e| e.to_string())?;
            if let Some(received) = received {
                state(received)?;
            }
            if let Some(chunk) = offer {
                validate_chunk(chunk)?;
            }
            acks(proof_acknowledgements, 1)?;
            results(acknowledgement_results, 1)?;
        }
        Reply::State {
            manifest,
            state: received,
        } => {
            manifest.core()?;
            state(received)?;
            if let TransferState::Complete { proof } = received {
                if proof.manifest != *manifest {
                    return Err("state proof names another manifest".into());
                }
            }
        }
        Reply::Chunk { chunk } => validate_chunk(chunk)?,
        Reply::Confirmed { acknowledgements } => acks(acknowledgements, 2)?,
        Reply::Acknowledged { results: values } => results(values, 2)?,
        Reply::Failed { why, .. } if why.len() > 4096 => {
            return Err("document error detail exceeds budget".into());
        }
        Reply::Failed { .. } => (),
    }
    Ok(())
}

pub(super) fn validate_roles(action: &Action, receiver: [u8; 32], sender: [u8; 32]) -> Result<(), Error> {
    match action {
        Action::Exchange {
            document,
            offer,
            proofs,
            acknowledgements,
            ..
        } => {
            if let Some(chunk) = offer {
                state::authorize(&chunk.manifest, sender, receiver)?;
                if chunk.offset != 0 || chunk.manifest.descriptor()?.document != *document {
                    return Err("first chunk does not bind exchange document".into());
                }
            }
            for proof in proofs {
                state::authorize(&proof.manifest, receiver, sender)?;
            }
            for ack in acknowledgements {
                state::authorize(&ack.manifest, sender, receiver)?;
            }
        }
        Action::Write { chunk } => state::authorize(&chunk.manifest, sender, receiver)?,
        Action::Read { manifest, .. } => state::authorize(manifest, receiver, sender)?,
        Action::Status { manifest } => state::authorize(manifest, sender, receiver)?,
        Action::Confirm { proofs } => {
            for proof in proofs {
                state::authorize(&proof.manifest, receiver, sender)?;
            }
        }
        Action::Acknowledge { acknowledgements } => {
            for ack in acknowledgements {
                state::authorize(&ack.manifest, sender, receiver)?;
            }
        }
    }
    Ok(())
}
fn validate_reply_roles(reply: &Reply, local: [u8; 32], remote: [u8; 32]) -> Result<(), Error> {
    fn proof_state(value: &TransferState, local: [u8; 32], remote: [u8; 32]) -> Result<(), Error> {
        if let TransferState::Complete { proof } = value {
            state::authorize(&proof.manifest, local, remote)?;
        }
        Ok(())
    }
    fn results(values: &[DocAckResult], local: [u8; 32], remote: [u8; 32]) -> Result<(), Error> {
        for result in values {
            let manifest = match result {
                DocAckResult::Released { acknowledgement }
                | DocAckResult::Absent { acknowledgement } => &acknowledgement.manifest,
                DocAckResult::Reconfirm { proof } => &proof.manifest,
            };
            state::authorize(manifest, local, remote)?;
        }
        Ok(())
    }
    match reply {
        Reply::Exchanged {
            received,
            offer,
            proof_acknowledgements,
            acknowledgement_results,
            ..
        } => {
            if let Some(received) = received {
                proof_state(received, local, remote)?;
            }
            if let Some(chunk) = offer {
                state::authorize(&chunk.manifest, remote, local)?;
            }
            for ack in proof_acknowledgements {
                state::authorize(&ack.manifest, remote, local)?;
            }
            results(acknowledgement_results, local, remote)?;
        }
        Reply::State {
            manifest,
            state: received,
        } => {
            state::authorize(manifest, local, remote)?;
            proof_state(received, local, remote)?;
        }
        Reply::Chunk { chunk } => state::authorize(&chunk.manifest, remote, local)?,
        Reply::Confirmed { acknowledgements } => {
            for ack in acknowledgements {
                state::authorize(&ack.manifest, remote, local)?;
            }
        }
        Reply::Acknowledged { results: values } => results(values, local, remote)?,
        Reply::Failed { .. } => (),
    }
    Ok(())
}

#[cfg(test)]
#[path = "doc_transfer_wire_tests.rs"]
mod wire_tests;

#[cfg(test)]
mod budget_tests {
    use super::{CHUNK_BYTES, VISIT_BYTES, fits_step_budget};

    #[test]
    fn an_opening_reserves_two_chunks_before_sending() {
        let one_left = VISIT_BYTES - CHUNK_BYTES;
        let two_left = VISIT_BYTES - 2 * CHUNK_BYTES;
        assert!(fits_step_budget(false, one_left, 3));
        assert!(fits_step_budget(true, two_left, 0));
        assert!(fits_step_budget(true, two_left, 2));
        assert!(!fits_step_budget(true, two_left, 3));
        assert!(
            !fits_step_budget(true, one_left, 0),
            "opening Exchange requires two canonical chunks of visit headroom"
        );
    }
}

#[cfg(test)]
#[path = "doc_transfer_flow_tests.rs"]
pub(crate) mod flow_tests;
