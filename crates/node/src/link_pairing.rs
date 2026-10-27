//! Recoverable Pair admission and a purpose-limited signed-roster upload.
//! Every disk step owns the existing bounded worker and drops its leases before
//! a network await. Tickets are checked once, then only the authenticated
//! caller, fixed nonce and immutable header can resume that durable claim.
use super::{Line, documents};
use crate::{
    Node,
    doc_transfer_state::{Context, Error, State, pairing as state},
    proto::*,
    trust::PairTrust,
};
use khor_net::trust::{self as nt, Id};
use khor_sync::{
    doc_transfer::DocumentVersion,
    transfer::{CHUNK_BYTES, Progress},
};
use std::time::{Duration, Instant};

fn bad_invite() -> Error {
    khor_catalog::msg::bad_invite().into()
}
fn id(text: &str) -> Result<Id, Error> {
    nt::id_from_hex(text).ok_or_else(bad_invite)
}
fn covers(have: &[u8], target: &[u8]) -> Result<bool, Error> {
    let have = DocumentVersion::decode(have).map_err(Error::from_std)?;
    let target = DocumentVersion::decode(target).map_err(Error::from_std)?;
    Ok(have.epoch == target.epoch && have.version.includes_vv(&target.version))
}
fn canonical_version(context: &Context) -> Result<serde_bytes::ByteBuf, Error> {
    Ok(context
        .version("devices")?
        .encode()
        .map_err(Error::from_std)?
        .into())
}
fn claim(state: &State, remote: Id, claim: Id, nonce: Id) -> Result<state::Claim, Error> {
    state
        .pair_claim(claim)
        .filter(|c| c.caller == remote && c.nonce == nonce)
        .ok_or_else(bad_invite)
}
fn live(trust: &PairTrust, row: &state::Claim) -> Result<(), Error> {
    if row.phase == PairPhase::Receiving || !trust.pair_replay_allowed(row.caller, row.anchor)? {
        return Err(bad_invite());
    }
    Ok(())
}
fn progress(state: &State, row: &state::Claim) -> Result<PairReply, Error> {
    let roster = if row.phase != PairPhase::Receiving && !row.roster_released {
        row.roster_out
            .as_ref()
            .map(|m| state.read_roster_blob(m, 0))
            .transpose()?
    } else {
        None
    };
    let received = if row.phase == PairPhase::Receiving {
        row.header
            .roster
            .as_ref()
            .map(|m| state.roster_offset(m))
            .transpose()?
            .unwrap_or(0)
    } else {
        row.header.roster.as_ref().map_or(0, |m| m.total_bytes)
    };
    Ok(PairReply::Progress {
        claim: row.id,
        phase: row.phase,
        name: row.name.clone(),
        devices_have: row.devices_have.clone(),
        roster_manifest: row.roster_out.clone(),
        roster,
        roster_received: received,
    })
}
fn commit(
    trust: &PairTrust,
    context: &Context,
    state: &mut State,
    row: state::Claim,
) -> Result<PairReply, Error> {
    if row.phase != PairPhase::Receiving {
        live(trust, &row)?;
        if let Some(m) = &row.header.roster {
            trust.release_pair_input(state, row.id, m)?;
        }
        return progress(state, &row);
    }
    let _fence = state::root_fence(&context.root)?;
    state.recover_pairing(trust, context)?;
    let row = state.pair_claim(row.id).ok_or("missing recovered claim")?;
    if row.phase != PairPhase::Receiving {
        live(trust, &row)?;
        if let Some(m) = &row.header.roster {
            trust.release_pair_input(state, row.id, m)?;
        }
        return progress(state, &row);
    }
    if let Some(m) = &row.header.roster {
        if !matches!(
            state.roster_progress(m)?,
            Some(Progress::VerifiedComplete { .. })
        ) {
            return progress(state, &row);
        }
    }
    let prepared = trust.prepare_pair_settlement(
        context,
        state,
        row.id,
        row.caller,
        false,
        row.header.name.clone(),
        row.header.addrs.clone(),
        row.header.roster.clone(),
    );
    if let Err(error) = prepared {
        // Only pre-Ready decisions can release the claim; a recorded Ready
        // intent is recovered instead of rolling back any root mutation.
        if !state.pair_settlement(row.id).is_some_and(|p| p.ready) {
            state.abort_pair(row.id)?;
            return Ok(PairReply::Uncommitted {
                claim: row.id,
                why: error.why.chars().take(4096).collect(),
            });
        }
        return Err(error);
    }
    trust.apply_pair_settlement(context, state, row.id)?;
    if let Some(m) = &row.header.roster {
        trust.release_pair_input(state, row.id, m)?;
    }
    progress(
        state,
        &state
            .pair_claim(row.id)
            .ok_or("pairing owner disappeared")?,
    )
}

/// Pure limits are checked before the shared journal is opened. Authenticated
/// role and exact-operation checks are repeated by the owning disk step.
pub(crate) fn validate_pair_action(action: &PairAction) -> Result<(), Error> {
    match action {
        PairAction::Start {
            token,
            header,
            roster,
            ..
        } => {
            state::token_hash(token)?;
            state::validate_header(header)?;
            state::validate_scalars(&header.name, &header.addrs, token.len() + 32 + 32)?;
            if roster.as_ref().map(|c| &c.manifest) != header.roster.as_ref()
                || roster.as_ref().is_some_and(|c| c.offset != 0)
            {
                return Err("pairing first roster must match its complete header".into());
            }
            if let Some(c) = roster {
                state::validate_chunk(c)?;
            }
        }
        PairAction::WriteRoster { chunk, .. } => state::validate_chunk(chunk)?,
        PairAction::ReadRoster {
            manifest, offset, ..
        } => {
            manifest.blob_core()?;
            if *offset >= manifest.total_bytes || offset % CHUNK_BYTES as u64 != 0 {
                return Err("noncanonical roster read".into());
            }
        }
        PairAction::TrustAccepted {
            roster: Some(m), ..
        } => {
            m.blob_core()?;
        }
        PairAction::Devices { action, .. } => documents::validate_action(action)?,
        PairAction::Finish { have, target, .. } => {
            DocumentVersion::decode(have).map_err(Error::from_std)?;
            DocumentVersion::decode(target).map_err(Error::from_std)?;
        }
        _ => (),
    }
    Ok(())
}
pub(crate) fn validate_pair_reply(reply: &PairReply) -> Result<(), Error> {
    match reply {
        PairReply::Progress {
            phase,
            name,
            devices_have,
            roster_manifest,
            roster,
            roster_received,
            ..
        } => {
            state::validate_scalars(name, &[], 0)?;
            if *phase != PairPhase::Receiving {
                DocumentVersion::decode(devices_have).map_err(Error::from_std)?;
            }
            if let Some(m) = roster_manifest {
                m.blob_core()?;
            }
            if let Some(c) = roster {
                state::validate_chunk(c)?;
                if Some(&c.manifest) != roster_manifest.as_ref() || c.offset != 0 {
                    return Err("pairing response roster mismatch".into());
                }
            }
            if *roster_received > khor_sync::transfer::MAX_BLOB_BYTES {
                return Err("invalid roster progress".into());
            }
        }
        PairReply::Roster { chunk, .. } => state::validate_chunk(chunk)?,
        PairReply::Devices { reply, .. } => documents::validate_reply(reply)?,
        PairReply::Aborted { .. } => (),
        PairReply::Uncommitted { why, .. } if why.len() <= 4096 => (),
        PairReply::Uncommitted { .. } => return Err("unbounded pairing diagnostic".into()),
    }
    Ok(())
}
pub(crate) fn validate_vouch_action(action: &VouchAction) -> Result<(), Error> {
    match action {
        VouchAction::Start {
            name,
            addrs,
            certificate,
            roster,
        } => {
            state::validate_scalars(name, addrs, certificate.len())?;
            if certificate.len() > 1024 || roster.offset != 0 {
                return Err("invalid Vouch header".into());
            }
            state::validate_chunk(roster)?;
        }
        VouchAction::Write { chunk, .. } => state::validate_chunk(chunk)?,
        _ => (),
    }
    Ok(())
}

impl Node {
    async fn pair_step<T: Send + 'static>(
        &self,
        job: impl FnOnce(&PairTrust, &Context, &mut State) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let context = self.doc_context()?;
        let name = self.name().to_owned();
        let (value, notices) = documents::on_worker(context.clone(), move |context| {
            let trust = PairTrust::new(context.clone(), name);
            let mut state = State::open(context)?;
            let value = job(&trust, context, &mut state)?;
            Ok((value, state.notice_ids(context)?))
        })
        .await?;
        self.deliver_doc_notices(context, notices).await?;
        Ok(value)
    }
    pub(super) async fn answer_pair(
        &self,
        remote: &str,
        action: PairAction,
    ) -> Result<Response, String> {
        validate_pair_action(&action)?;
        let remote = id(remote)?;
        let doc_operation = match &action {
            PairAction::Devices { claim, .. } => Some(*claim),
            _ => None,
        };
        let result = self
            .pair_step(move |trust, context, state| match action {
                PairAction::Start {
                    token,
                    nonce,
                    header,
                    roster,
                } => {
                    let hash = state::token_hash(&token)?;
                    let operation = state::claim_id(context.local, remote, hash, nonce);
                    if let Some(m) = &header.roster {
                        state::authorize_blob(
                            m,
                            operation,
                            state::CALLER_ROSTER,
                            remote,
                            context.local,
                        )?;
                    }
                    let row = if let Some(saved) = state.claim_for_ticket(hash) {
                        if saved.id != operation
                            || saved.caller != remote
                            || saved.nonce != nonce
                            || saved.header != header
                        {
                            return Err(bad_invite());
                        }
                        saved
                    } else {
                        trust.collect_pairing(context, state)?;
                        let path = context.root.join(".khor/invites").join(nt::id_hex(&hash));
                        if !path.exists() {
                            return Err(bad_invite());
                        }
                        if !Self::invite_is_fresh(&path) {
                            std::fs::remove_file(&path).map_err(Error::from_std)?;
                            return Err(khor_catalog::msg::invite_expired(
                                super::invite_window_minutes(),
                            )
                            .into());
                        }
                        let minted_ms = std::fs::read_to_string(path)
                            .map_err(Error::from_std)?
                            .trim()
                            .parse::<i64>()
                            .map_err(Error::from_std)?;
                        let row = state::Claim {
                            id: operation,
                            caller: remote,
                            nonce,
                            token_hash: hash,
                            minted_ms,
                            header,
                            name: trust.name().into(),
                            phase: PairPhase::Receiving,
                            anchor: None,
                            roster_out: None,
                            roster_released: false,
                            devices_have: Vec::new().into(),
                            devices_out: None,
                            devices_in: None,
                            devices_offer_decided: false,
                            completed_ms: None,
                        };
                        state.save_claim(row.clone())?;
                        row
                    };
                    if row.phase != PairPhase::Receiving {
                        live(trust, &row)?;
                        if let Some(m) = &row.header.roster {
                            trust.release_pair_input(state, row.id, m)?;
                        }
                        return progress(state, &row);
                    }
                    if let Some(chunk) = roster {
                        state.receive_roster_blob(&chunk, true)?;
                    }
                    // The common small roster completes with this very request.
                    if row
                        .header
                        .roster
                        .as_ref()
                        .is_none_or(|m| m.total_bytes <= CHUNK_BYTES as u64)
                    {
                        commit(trust, context, state, row)
                    } else {
                        progress(state, &row)
                    }
                }
                PairAction::Resume {
                    claim: operation,
                    nonce,
                } => {
                    let row = claim(state, remote, operation, nonce)?;
                    if row.phase != PairPhase::Receiving {
                        live(trust, &row)?;
                        if let Some(m) = &row.header.roster {
                            trust.release_pair_input(state, row.id, m)?;
                        }
                    }
                    progress(state, &row)
                }
                PairAction::WriteRoster {
                    claim: operation,
                    nonce,
                    chunk,
                } => {
                    let row = claim(state, remote, operation, nonce)?;
                    if row.phase != PairPhase::Receiving
                        || row.header.roster.as_ref() != Some(&chunk.manifest)
                    {
                        return Err(bad_invite());
                    }
                    state.receive_roster_blob(&chunk, false)?;
                    progress(state, &row)
                }
                PairAction::ReadRoster {
                    claim: operation,
                    nonce,
                    manifest,
                    offset,
                } => {
                    let row = claim(state, remote, operation, nonce)?;
                    live(trust, &row)?;
                    if row.roster_released || row.roster_out.as_ref() != Some(&manifest) {
                        return Err("unknown pairing roster read".into());
                    }
                    Ok(PairReply::Roster {
                        claim: operation,
                        chunk: state.read_roster_blob(&manifest, offset)?,
                    })
                }
                PairAction::Commit {
                    claim: operation,
                    nonce,
                } => {
                    let row = claim(state, remote, operation, nonce)?;
                    commit(trust, context, state, row)
                }
                PairAction::TrustAccepted {
                    claim: operation,
                    nonce,
                    roster,
                } => {
                    let mut row = claim(state, remote, operation, nonce)?;
                    live(trust, &row)?;
                    if row.roster_out != roster {
                        return Err("pairing roster acknowledgement mismatch".into());
                    }
                    // Publish acceptance before cancellation so a lost reply never
                    // recreates a roster whose receiver already settled it.
                    row.roster_released = true;
                    state.save_claim(row.clone())?;
                    if let Some(m) = roster {
                        state.release_roster_blob(&m)?;
                    }
                    progress(state, &row)
                }
                PairAction::Devices {
                    claim: operation,
                    nonce,
                    mut action,
                } => {
                    let mut row = claim(state, remote, operation, nonce)?;
                    live(trust, &row)?;
                    if !row.roster_released {
                        return Err("pairing trust acknowledgement is pending".into());
                    }
                    documents::validate_roles(&action, context.local, remote)?;
                    authorize_devices(&mut row, &action)?;
                    // The sender plan always uses the base captured by Start.
                    if let DocTransferAction::Exchange { have, .. } = &mut action {
                        *have = row.header.devices_have.clone();
                    }
                    state.save_claim(row.clone())?;
                    let reply =
                        documents::handle_pair(context, state, remote, action, Some(operation))?;
                    Ok(PairReply::Devices {
                        claim: operation,
                        reply,
                    })
                }
                PairAction::Finish {
                    claim: operation,
                    nonce,
                    have,
                    target,
                } => {
                    let mut row = claim(state, remote, operation, nonce)?;
                    live(trust, &row)?;
                    let settlement = state
                        .pair_settlement(operation)
                        .ok_or("missing pair settlement")?;
                    if !row.roster_released
                        || !settlement.finished
                        || state.pair_documents_pending(remote, operation)?
                        || !covers(&canonical_version(context)?, &target)?
                        || !covers(
                            &have,
                            settlement
                                .target
                                .as_ref()
                                .ok_or("missing settlement target")?,
                        )?
                    {
                        return Err("pairing bootstrap is still pending".into());
                    }
                    row.phase = PairPhase::BootstrapComplete;
                    row.completed_ms.get_or_insert(crate::now_ms());
                    state.save_claim(row.clone())?;
                    progress(state, &row)
                }
                PairAction::Abort {
                    claim: operation,
                    nonce,
                } => {
                    claim(state, remote, operation, nonce)?;
                    state.abort_pair(operation)?;
                    Ok(PairReply::Aborted { claim: operation })
                }
            })
            .await;
        let reply = match result {
            Ok(reply) => reply,
            Err(error) => match doc_operation {
                Some(claim) => PairReply::Devices {
                    claim,
                    reply: DocTransferReply::Failed {
                        kind: error.kind,
                        why: error.why.chars().take(1024).collect(),
                    },
                },
                None => return Err(error.into()),
            },
        };
        Ok(Response::Paired { reply })
    }

    async fn pair_request(
        &self,
        line: &Line,
        action: PairAction,
        expected: Id,
    ) -> Result<PairReply, Error> {
        let reply = match self
            .exchange_on(line, &Request::Pair { action })
            .await
            .map_err(super::Miss::words)?
        {
            Response::Paired { reply } => reply,
            Response::Refused { why } => return Err(why.into()),
            _ => return Err("unexpected pairing response".into()),
        };
        let received = match &reply {
            PairReply::Progress { claim, .. }
            | PairReply::Roster { claim, .. }
            | PairReply::Devices { claim, .. }
            | PairReply::Aborted { claim }
            | PairReply::Uncommitted { claim, .. } => *claim,
        };
        if received != expected {
            return Err("pairing response claim mismatch".into());
        }
        validate_pair_reply(&reply)?;
        if let PairReply::Uncommitted { why, .. } = reply {
            self.pair_step(move |_, _, state| state.abort_local_pair(expected))
                .await?;
            return Err(why.into());
        }
        Ok(reply)
    }

    pub(super) async fn pair_over(
        &self,
        conn: &iroh::endpoint::Connection,
        ticket: &khor_net::endpoint::Ticket,
        addrs: Vec<String>,
    ) -> Result<String, String> {
        let remote = id(&ticket.id)?;
        let hash = state::token_hash(&ticket.token)?;
        let nonce = id(&format!("{}{}", super::fresh_hex()?, super::fresh_hex()?))?;
        let addrs = addrs.clone();
        let local = self
            .pair_step(move |trust, context, state| {
                if let Some(saved) = state.local_for_ticket(remote, hash) {
                    if saved.phase != PairPhase::Receiving
                        && !trust.pair_replay_allowed(remote, saved.anchor)?
                    {
                        return Err(bad_invite());
                    }
                    if !saved.roster_released
                        && let Some(m) = &saved.header.roster
                    {
                        if !matches!(
                            state.roster_progress(m)?,
                            Some(Progress::VerifiedComplete { .. })
                        ) {
                            let raw = trust.fixed_roster_bytes(m)?;
                            state.prepare_roster_blob(m, &raw)?;
                        }
                    }
                    return Ok(saved);
                }
                trust.collect_pairing(context, state)?;
                let raw = trust.roster_offer()?;
                let operation = state::claim_id(remote, context.local, hash, nonce);
                let manifest = if raw.is_empty() {
                    None
                } else {
                    Some(state::roster_manifest(
                        operation,
                        state::CALLER_ROSTER,
                        context.local,
                        remote,
                        &raw,
                    )?)
                };
                let row = state::Local {
                    id: operation,
                    issuer: remote,
                    nonce,
                    token_hash: hash,
                    header: PairHeader {
                        name: trust.name().into(),
                        addrs,
                        devices_have: canonical_version(context)?,
                        roster: manifest.clone(),
                    },
                    initial_anchor: trust.anchor_id(),
                    initial_epoch: context.version("devices")?.epoch,
                    phase: PairPhase::Receiving,
                    anchor: None,
                    peer_name: String::new(),
                    peer_have: Vec::new().into(),
                    peer_roster: None,
                    roster_released: false,
                    trust_accepted: false,
                    devices_out: None,
                    devices_in: None,
                    devices_offer_decided: false,
                    completed_ms: None,
                };
                state.save_local(row.clone())?;
                if let Some(m) = manifest {
                    state.prepare_roster_blob(&m, &raw)?;
                }
                Ok(row)
            })
            .await?;
        let line = Line {
            id: ticket.id.clone(),
            name: ticket.name.clone(),
            proto: crate::wires::Proto::Main,
            conn: conn.clone(),
        };
        let operation = local.id;
        let nonce = local.nonce;
        let first = if !local.roster_released {
            let manifest = local.header.roster.clone();
            let chunk = self
                .pair_step(move |_, _, state| {
                    manifest
                        .as_ref()
                        .map(|m| state.read_roster_blob(m, 0))
                        .transpose()
                })
                .await?;
            PairAction::Start {
                token: ticket.token.clone(),
                nonce,
                header: local.header.clone(),
                roster: chunk,
            }
        } else {
            PairAction::Resume {
                claim: operation,
                nonce,
            }
        };
        let mut reply = self.pair_request(&line, first, operation).await?;
        let began = Instant::now();
        loop {
            if began.elapsed() > Duration::from_secs(240) {
                return Err("pairing remains pending; retry this ticket".into());
            }
            let PairReply::Progress {
                phase,
                name,
                devices_have,
                roster_manifest,
                roster,
                roster_received,
                ..
            } = reply
            else {
                return Err("unexpected pairing phase response".into());
            };
            if phase == PairPhase::Receiving {
                let manifest = local.header.roster.clone();
                let next =
                    if let Some(manifest) = manifest.filter(|m| roster_received < m.total_bytes) {
                        let chunk = self
                            .pair_step(move |_, _, state| {
                                state.read_roster_blob(&manifest, roster_received)
                            })
                            .await?;
                        PairAction::WriteRoster {
                            claim: operation,
                            nonce,
                            chunk,
                        }
                    } else {
                        PairAction::Commit {
                            claim: operation,
                            nonce,
                        }
                    };
                reply = self.pair_request(&line, next, operation).await?;
                continue;
            }
            let supplied_manifest = roster_manifest.clone();
            let supplied_name = name.clone();
            let supplied_have = devices_have.clone();
            let needs_roster = self
                .pair_step(move |_, context, state| {
                    if let Some(m) = &supplied_manifest {
                        state::authorize_blob(
                            m,
                            operation,
                            state::ISSUER_ROSTER,
                            remote,
                            context.local,
                        )?;
                    }
                    DocumentVersion::decode(&supplied_have).map_err(Error::from_std)?;
                    let mut row = state
                        .pair_local(operation)
                        .ok_or("missing local pairing intent")?;
                    if !row.peer_have.is_empty()
                        && (row.peer_have != supplied_have
                            || row.peer_roster != supplied_manifest
                            || row.peer_name != supplied_name)
                    {
                        return Err("issuer changed a fixed pairing response".into());
                    }
                    row.peer_have = supplied_have;
                    row.peer_roster = supplied_manifest;
                    row.peer_name = supplied_name;
                    row.roster_released = true;
                    state.save_local(row.clone())?;
                    if let Some(m) = &row.header.roster {
                        state.release_roster_blob(m)?;
                    }
                    if row.phase == PairPhase::Receiving {
                        if let Some(chunk) = roster {
                            state.receive_roster_blob(&chunk, true)?;
                        }
                    }
                    Ok(row.phase == PairPhase::Receiving)
                })
                .await?;
            if let Some(manifest) = roster_manifest.clone().filter(|_| needs_roster) {
                loop {
                    let m = manifest.clone();
                    let offset = self
                        .pair_step(move |_, _, state| state.roster_offset(&m))
                        .await?;
                    if offset >= manifest.total_bytes {
                        break;
                    }
                    let r = self
                        .pair_request(
                            &line,
                            PairAction::ReadRoster {
                                claim: operation,
                                nonce,
                                manifest: manifest.clone(),
                                offset,
                            },
                            operation,
                        )
                        .await?;
                    let PairReply::Roster { chunk, .. } = r else {
                        return Err("missing issuer roster chunk".into());
                    };
                    if chunk.manifest != manifest || chunk.offset != offset {
                        return Err("issuer roster read mismatch".into());
                    }
                    self.pair_step(move |_, _, state| {
                        state.receive_roster_blob(&chunk, offset == 0).map(|_| ())
                    })
                    .await?;
                }
            }
            let peer_name = name.clone();
            let peer_addrs = ticket.direct.clone();
            self.pair_step(move |trust, context, state| {
                let _fence = state::root_fence(&context.root)?;
                state.recover_pairing(trust, context)?;
                let row = state.pair_local(operation).ok_or("missing local pair")?;
                if row.phase == PairPhase::Receiving {
                    if context.version("devices")?.epoch != row.initial_epoch
                        || trust.anchor_id() != row.initial_anchor
                    {
                        return Err("local pairing root or epoch changed".into());
                    }
                    trust.prepare_pair_settlement(
                        context,
                        state,
                        operation,
                        remote,
                        true,
                        peer_name,
                        peer_addrs,
                        row.peer_roster.clone(),
                    )?;
                    trust.apply_pair_settlement(context, state, operation)?;
                }
                if !trust.pair_replay_allowed(
                    remote,
                    state
                        .pair_local(operation)
                        .ok_or("missing local pair")?
                        .anchor,
                )? {
                    return Err(bad_invite());
                }
                if let Some(m) = row.peer_roster {
                    trust.release_pair_input(state, operation, &m)?;
                }
                trust.release_local_settlement_output(state, operation)?;
                Ok(())
            })
            .await?;
            if phase == PairPhase::BootstrapComplete {
                self.pair_step(move |trust, context, state| {
                    let mut row = state.pair_local(operation).ok_or("missing local pair")?;
                    if !row.trust_accepted
                        || !trust.pair_replay_allowed(remote, row.anchor)?
                        || !covers(&canonical_version(context)?, &row.peer_have)?
                        || state.pair_documents_pending(remote, operation)?
                    {
                        return Err("completed issuer claim does not match local bootstrap".into());
                    }
                    row.phase = PairPhase::BootstrapComplete;
                    row.completed_ms = Some(crate::now_ms());
                    state.save_local(row)
                })
                .await?;
                return Ok(name);
            }
            // Record local admission before telling the sender it may discard
            // its roster. The immutable manifest remains in both journals.
            self.pair_request(
                &line,
                PairAction::TrustAccepted {
                    claim: operation,
                    nonce,
                    roster: roster_manifest,
                },
                operation,
            )
            .await?;
            self.pair_step(move |_, _, state| {
                let mut row = state.pair_local(operation).ok_or("missing local pair")?;
                row.trust_accepted = true;
                state.save_local(row)
            })
            .await?;
            loop {
                let moved = self
                    .transfer_pair_documents(&line, devices_have.clone(), operation, nonce)
                    .await;
                if let Some(why) = moved.first_failure {
                    return Err(why);
                }
                if !moved.apart.is_empty() {
                    return Err("pairing document epochs are apart".into());
                }
                if moved.pending.is_empty() {
                    break;
                }
                if began.elapsed() > Duration::from_secs(240) {
                    return Err("pairing bootstrap remains pending; retry this ticket".into());
                }
            }
            let (have, target) = self
                .pair_step(move |_, context, state| {
                    if state.pair_documents_pending(remote, operation)? {
                        return Err("pairing receipts remain pending".into());
                    }
                    let target = state
                        .pair_settlement(operation)
                        .and_then(|s| s.target)
                        .ok_or("missing local target")?;
                    Ok((canonical_version(context)?, target))
                })
                .await?;
            let completed = self
                .pair_request(
                    &line,
                    PairAction::Finish {
                        claim: operation,
                        nonce,
                        have,
                        target,
                    },
                    operation,
                )
                .await?;
            if !matches!(
                completed,
                PairReply::Progress {
                    phase: PairPhase::BootstrapComplete,
                    ..
                }
            ) {
                return Err("pairing completion remains pending".into());
            }
            self.pair_step(move |trust, _, state| {
                let mut row = state.pair_local(operation).ok_or("missing local pair")?;
                if !trust.pair_replay_allowed(remote, row.anchor)? {
                    return Err(bad_invite());
                }
                row.phase = PairPhase::BootstrapComplete;
                row.completed_ms = Some(crate::now_ms());
                state.save_local(row)
            })
            .await?;
            return Ok(name);
        }
    }

    pub(super) async fn answer_vouch(
        &self,
        remote: &str,
        action: VouchAction,
    ) -> Result<Response, String> {
        validate_vouch_action(&action)?;
        let remote = id(remote)?;
        let reply = self
            .pair_step(move |trust, context, state| {
                let finishing = matches!(&action, VouchAction::Finish { .. });
                let (mut row, chunk, create) = match action {
                    VouchAction::Start {
                        name,
                        addrs,
                        certificate,
                        roster,
                    } => {
                        let anchor = trust.authorize_vouch_header(remote, &certificate)?;
                        state::authorize_blob(
                            &roster.manifest,
                            [0; 32],
                            state::VOUCH_ROSTER,
                            remote,
                            context.local,
                        )?;
                        let operation = state::blob_id(&roster.manifest)?;
                        let row = if let Some(row) = state.vouch(operation) {
                            if row.remote != remote
                                || row.anchor != anchor
                                || row.manifest != roster.manifest
                                || row.name != name
                                || row.addrs != addrs
                            {
                                return Err("Vouch header changed".into());
                            }
                            row
                        } else {
                            let row = state::Vouch {
                                id: operation,
                                remote,
                                anchor,
                                name,
                                addrs,
                                manifest: roster.manifest.clone(),
                                phase: VouchPhase::Stored { next_offset: 0 },
                                target: None,
                                completed_ms: None,
                            };
                            state.save_vouch(row.clone())?;
                            row
                        };
                        (row, Some(roster), true)
                    }
                    VouchAction::Write { operation, chunk } => {
                        let row = state
                            .vouch(operation)
                            .filter(|r| r.remote == remote && r.manifest == chunk.manifest)
                            .ok_or("unknown Vouch upload")?;
                        (row, Some(chunk), false)
                    }
                    VouchAction::Status { operation } | VouchAction::Finish { operation } => {
                        let row = state
                            .vouch(operation)
                            .filter(|r| r.remote == remote)
                            .ok_or("unknown Vouch operation")?;
                        if trust.anchor_id() != Some(row.anchor) {
                            return Err("Vouch root changed".into());
                        }
                        if finishing {
                            state.finish_vouch(operation)?;
                            return Ok(VouchReply {
                                operation,
                                phase: row.phase,
                            });
                        }
                        (row, None, false)
                    }
                };
                if trust.anchor_id() != Some(row.anchor) {
                    return Err("Vouch root changed".into());
                }
                if row.phase != VouchPhase::Vouched {
                    if let Some(chunk) = chunk {
                        row.phase = state.receive_roster_blob(&chunk, create)?;
                        state.save_vouch(row.clone())?;
                    }
                    if row.phase == VouchPhase::VerifiedBlob {
                        trust.apply_vouch(state, &mut row)?;
                        row.phase = VouchPhase::Vouched;
                        row.completed_ms = Some(crate::now_ms());
                        state.save_vouch(row.clone())?;
                    }
                }
                if row.phase == VouchPhase::Vouched
                    && !trust.pair_replay_allowed(remote, Some(row.anchor))?
                {
                    return Err(khor_catalog::msg::vouch_refused().into());
                }
                Ok(VouchReply {
                    operation: row.id,
                    phase: row.phase,
                })
            })
            .await?;
        Ok(Response::Vouched { reply })
    }
    pub(super) async fn vouch_over(
        &self,
        line: &Line,
        name: String,
        addrs: Vec<String>,
        raw: Vec<u8>,
    ) -> Result<(), Error> {
        let remote = id(&line.id)?;
        let sender = self
            .pair_step(move |trust, context, state| {
                if let Some(saved) = state.vouch_sender(remote) {
                    if !matches!(
                        state.roster_progress(&saved.manifest)?,
                        Some(Progress::VerifiedComplete { .. })
                    ) {
                        if raw.len() as u64 != saved.manifest.total_bytes
                            || blake3::hash(&raw).as_bytes() != &saved.manifest.digest
                        {
                            return Err("fixed Vouch spool needs reconciliation".into());
                        }
                        state.prepare_roster_blob(&saved.manifest, &raw)?;
                    }
                    return Ok(saved);
                }
                let certificate = trust
                    .vouch_certificate()
                    .ok_or("no signed Vouch certificate")?;
                let manifest = state::vouch_manifest(context.local, remote, &raw)?;
                let sender = state::VouchOut {
                    manifest,
                    name,
                    addrs,
                    certificate: certificate.into(),
                };
                state.prepare_vouch_blob(sender.clone(), &raw)?;
                Ok(sender)
            })
            .await?;
        let manifest = sender.manifest;
        let operation = state::blob_id(&manifest)?;
        let m = manifest.clone();
        let chunk = self
            .pair_step(move |_, _, state| state.read_roster_blob(&m, 0))
            .await?;
        let began = Instant::now();
        let mut action = VouchAction::Start {
            name: sender.name,
            addrs: sender.addrs,
            certificate: sender.certificate,
            roster: chunk,
        };
        loop {
            if began.elapsed() > Duration::from_secs(90) {
                return Err("Vouch remains pending".into());
            }
            let reply = match self
                .exchange_on(line, &Request::Vouch { action })
                .await
                .map_err(super::Miss::words)?
            {
                Response::Vouched { reply } if reply.operation == operation => reply,
                Response::Refused { why } => return Err(why.into()),
                _ => return Err("unexpected Vouch response".into()),
            };
            match reply.phase {
                VouchPhase::Vouched => {
                    let m = manifest.clone();
                    self.pair_step(move |_, _, state| state.finish_vouch_blob(&m))
                        .await?;
                    let _ = self
                        .exchange_on(
                            line,
                            &Request::Vouch {
                                action: VouchAction::Finish { operation },
                            },
                        )
                        .await;
                    return Ok(());
                }
                VouchPhase::Stored { next_offset } => {
                    if next_offset >= manifest.total_bytes || next_offset % CHUNK_BYTES as u64 != 0
                    {
                        return Err("invalid Vouch resume offset".into());
                    }
                    let m = manifest.clone();
                    let chunk = self
                        .pair_step(move |_, _, state| state.read_roster_blob(&m, next_offset))
                        .await?;
                    action = VouchAction::Write { operation, chunk };
                }
                VouchPhase::VerifiedBlob => {
                    action = VouchAction::Status { operation };
                }
            }
        }
    }
}

fn authorize_devices(row: &mut state::Claim, action: &DocTransferAction) -> Result<(), Error> {
    fn exact(manifest: &DocManifest, expected: &Option<String>) -> Result<(), Error> {
        if manifest.descriptor()?.document != "devices"
            || expected.as_ref() != Some(&manifest.id()?)
        {
            return Err("document is outside this pairing grant".into());
        }
        Ok(())
    }
    match action {
        DocTransferAction::Exchange {
            document,
            offer,
            proofs,
            acknowledgements,
            ..
        } => {
            if document != "devices" {
                return Err("pairing grants only devices".into());
            }
            if let Some(chunk) = offer {
                let descriptor = chunk.manifest.descriptor()?;
                let have = DocumentVersion::decode(&row.devices_have).map_err(Error::from_std)?;
                if descriptor.document != "devices"
                    || descriptor.epoch != have.epoch
                    || descriptor.base != have.version
                {
                    return Err("pairing devices base or epoch changed".into());
                }
                let key = chunk.manifest.id()?;
                if row.devices_in.as_ref().is_some_and(|old| old != &key) {
                    return Err("pairing incoming target changed".into());
                }
                row.devices_in = Some(key);
            }
            for proof in proofs {
                exact(&proof.manifest, &row.devices_out)?;
            }
            for ack in acknowledgements {
                exact(&ack.manifest, &row.devices_in)?;
            }
        }
        DocTransferAction::Read { manifest, .. } => exact(manifest, &row.devices_out)?,
        DocTransferAction::Status { manifest } => exact(manifest, &row.devices_in)?,
        DocTransferAction::Write { chunk } => exact(&chunk.manifest, &row.devices_in)?,
        DocTransferAction::Confirm { proofs } => {
            for proof in proofs {
                exact(&proof.manifest, &row.devices_out)?;
            }
        }
        DocTransferAction::Acknowledge { acknowledgements } => {
            for ack in acknowledgements {
                exact(&ack.manifest, &row.devices_in)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "link_pairing_tests.rs"]
pub(crate) mod tests;
