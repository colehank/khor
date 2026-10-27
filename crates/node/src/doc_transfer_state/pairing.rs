//! Pair/Vouch ownership in the same bounded journal and blob quota as Doc.
//! No ticket secret or root private key is persisted here.
use super::{Context, DocChunk, DocManifest, Error, Records as AllRecords, State, io};
use crate::{
    proto::{DocAckResult, DocAcknowledgement, DocProof, PairHeader, PairPhase, VouchPhase},
    trust::PairTrust,
};
use khor_sync::{
    doc_transfer::DocumentVersion,
    transfer::{CHUNK_BYTES, MAX_BLOB_BYTES, Progress},
};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) type Id = [u8; 32];
const DOMAIN: &[u8] = b"khor-trust-roster-v1\0";
pub(crate) const CALLER_ROSTER: u8 = 0;
pub(crate) const ISSUER_ROSTER: u8 = 1;
pub(crate) const LOCAL_SETTLEMENT: u8 = 2;
pub(crate) const VOUCH_ROSTER: u8 = 3;
const MAX_CLAIMS: usize = 16;
const MAX_ROW: usize = 192 * 1024;

pub(crate) fn token_hash(token: &str) -> Result<Id, Error> {
    if token.is_empty() || token.len() > 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(khor_catalog::msg::bad_invite().into());
    }
    let mut hash = blake3::Hasher::new();
    hash.update(b"khor-pair-ticket-v1\0");
    hash.update(token.as_bytes());
    Ok(*hash.finalize().as_bytes())
}
pub(crate) fn claim_id(issuer: Id, caller: Id, token: Id, nonce: Id) -> Id {
    let mut hash = blake3::Hasher::new();
    hash.update(b"khor-pair-claim-v1\0");
    for bytes in [issuer, caller, token, nonce] {
        hash.update(&bytes);
    }
    *hash.finalize().as_bytes()
}
pub(crate) fn binding(operation: Id, role: u8) -> Vec<u8> {
    let mut bytes = Vec::from(DOMAIN);
    bytes.push(role);
    bytes.extend_from_slice(&operation);
    bytes
}
pub(crate) fn roster_manifest(
    operation: Id,
    role: u8,
    source: Id,
    recipient: Id,
    bytes: &[u8],
) -> Result<DocManifest, Error> {
    let manifest = DocManifest {
        source,
        recipient,
        binding: binding(operation, role).into(),
        total_bytes: bytes.len() as u64,
        digest: *blake3::hash(bytes).as_bytes(),
    };
    manifest.blob_core()?;
    Ok(manifest)
}
pub(crate) fn vouch_manifest(
    source: Id,
    recipient: Id,
    bytes: &[u8],
) -> Result<DocManifest, Error> {
    // The operation is the resulting manifest id; no circular id-in-binding.
    roster_manifest([0; 32], VOUCH_ROSTER, source, recipient, bytes)
}
pub(crate) fn blob_id(manifest: &DocManifest) -> Result<Id, Error> {
    let core = manifest.blob_core()?.id().map_err(Error::from_std)?;
    let mut hash = blake3::Hasher::new();
    hash.update(b"khor-vouch-operation-v1\0");
    hash.update(&manifest.source);
    hash.update(&manifest.recipient);
    hash.update(core.to_string().as_bytes());
    Ok(*hash.finalize().as_bytes())
}
pub(crate) fn authorize_blob(
    manifest: &DocManifest,
    operation: Id,
    role: u8,
    source: Id,
    recipient: Id,
) -> Result<(), Error> {
    manifest.blob_core()?;
    if manifest.source != source
        || manifest.recipient != recipient
        || manifest.binding.as_ref() != binding(operation, role).as_slice()
    {
        return Err("trust roster purpose, identity or operation mismatch".into());
    }
    Ok(())
}
pub(crate) fn validate_header(header: &PairHeader) -> Result<(), Error> {
    validate_scalars(&header.name, &header.addrs, 0)?;
    DocumentVersion::decode(&header.devices_have).map_err(Error::from_std)?;
    Ok(())
}
pub(crate) fn validate_scalars(name: &str, addrs: &[String], other: usize) -> Result<(), Error> {
    if name.len() > 256
        || addrs.len() > 64
        || addrs.iter().any(|s| s.len() > 256)
        || rmp_serde::to_vec(&(name, addrs))
            .map_err(Error::from_std)?
            .len()
            + other
            > 16 * 1024
    {
        return Err("pairing header exceeds its scalar budget".into());
    }
    Ok(())
}
pub(crate) fn validate_chunk(chunk: &DocChunk) -> Result<(), Error> {
    chunk.manifest.blob_core()?;
    let end = chunk
        .offset
        .checked_add(chunk.bytes.len() as u64)
        .ok_or("trust chunk overflow")?;
    if chunk.bytes.len() > CHUNK_BYTES
        || chunk.offset % CHUNK_BYTES as u64 != 0
        || end > chunk.manifest.total_bytes
        || (end != chunk.manifest.total_bytes && chunk.bytes.len() != CHUNK_BYTES)
    {
        return Err("noncanonical trust roster chunk".into());
    }
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Claim {
    pub id: Id,
    pub caller: Id,
    pub nonce: Id,
    pub token_hash: Id,
    pub minted_ms: i64,
    pub header: PairHeader,
    pub name: String,
    pub phase: PairPhase,
    pub anchor: Option<Id>,
    pub roster_out: Option<DocManifest>,
    pub roster_released: bool,
    pub devices_have: serde_bytes::ByteBuf,
    pub devices_out: Option<String>,
    pub devices_in: Option<String>,
    pub devices_offer_decided: bool,
    pub completed_ms: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Local {
    pub id: Id,
    pub issuer: Id,
    pub nonce: Id,
    pub token_hash: Id,
    pub header: PairHeader,
    pub initial_anchor: Option<Id>,
    pub initial_epoch: u32,
    pub phase: PairPhase,
    pub anchor: Option<Id>,
    pub peer_name: String,
    pub peer_have: serde_bytes::ByteBuf,
    pub peer_roster: Option<DocManifest>,
    pub roster_released: bool,
    pub trust_accepted: bool,
    pub devices_out: Option<String>,
    pub devices_in: Option<String>,
    pub devices_offer_decided: bool,
    pub completed_ms: Option<u64>,
}
/// A fixed signed desired roster is staged before this intent is saved. The
/// target VV is published before flush, so recovery never replays row hints
/// after that exact mutation has reached disk.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Settlement {
    pub id: Id,
    pub remote: Id,
    pub dialed: bool,
    pub before_anchor: Option<Id>,
    pub before_roster: Option<Id>,
    pub before_key: Option<Id>,
    pub desired_anchor: Option<Id>,
    pub desired_roster: Option<DocManifest>,
    pub epoch: u32,
    pub target: Option<serde_bytes::ByteBuf>,
    pub ready: bool,
    pub finished: bool,
    pub remote_name: String,
    pub remote_addrs: Vec<String>,
    pub row_before: Option<(String, Vec<String>)>,
    pub input_roster: Option<DocManifest>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Vouch {
    pub id: Id,
    pub remote: Id,
    pub anchor: Id,
    pub name: String,
    pub addrs: Vec<String>,
    pub manifest: DocManifest,
    pub phase: VouchPhase,
    pub target: Option<serde_bytes::ByteBuf>,
    pub completed_ms: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct VouchOut {
    pub manifest: DocManifest,
    pub name: String,
    pub addrs: Vec<String>,
    pub certificate: serde_bytes::ByteBuf,
}
#[derive(Default, Clone, Serialize, Deserialize)]
pub(crate) struct Records {
    pub claims: Vec<Claim>,
    pub local: Vec<Local>,
    pub settlements: Vec<Settlement>,
    pub vouches: Vec<Vouch>,
    pub vouch_out: Vec<VouchOut>,
}
impl Records {
    pub(super) fn is_empty(&self) -> bool {
        self.claims.is_empty()
            && self.local.is_empty()
            && self.settlements.is_empty()
            && self.vouches.is_empty()
            && self.vouch_out.is_empty()
    }
    pub(super) fn references(&self, manifest: &DocManifest) -> bool {
        self.claims.iter().any(|c| {
            c.header.roster.as_ref() == Some(manifest) || c.roster_out.as_ref() == Some(manifest)
        }) || self.local.iter().any(|c| {
            c.header.roster.as_ref() == Some(manifest) || c.peer_roster.as_ref() == Some(manifest)
        }) || self.settlements.iter().any(|s| {
            s.desired_roster.as_ref() == Some(manifest) || s.input_roster.as_ref() == Some(manifest)
        }) || self.vouches.iter().any(|v| &v.manifest == manifest)
            || self.vouch_out.iter().any(|row| &row.manifest == manifest)
    }
    pub(super) fn validate(&self, local: Id) -> Result<(), Error> {
        if self.claims.len() + self.local.len() > MAX_CLAIMS
            || self.settlements.len() > MAX_CLAIMS
            || self.vouches.len() > MAX_CLAIMS
            || self.vouch_out.len() > MAX_CLAIMS
        {
            return Err(Error {
                kind: crate::proto::DocTransferFailure::ResourceLimit,
                why: "pairing metadata quota exceeded".into(),
            });
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut tokens = std::collections::BTreeSet::new();
        for row in &self.claims {
            validate_header(&row.header)?;
            if row.caller == local
                || row.id != claim_id(local, row.caller, row.token_hash, row.nonce)
                || !ids.insert(row.id)
                || !tokens.insert(row.token_hash)
            {
                return Err("invalid or duplicate pairing claim".into());
            }
            if let Some(m) = &row.header.roster {
                authorize_blob(m, row.id, CALLER_ROSTER, row.caller, local)?;
            }
            if let Some(m) = &row.roster_out {
                authorize_blob(m, row.id, ISSUER_ROSTER, local, row.caller)?;
            }
            if row.phase != PairPhase::Receiving {
                DocumentVersion::decode(&row.devices_have).map_err(Error::from_std)?;
            }
            if row.completed_ms.is_some() != (row.phase == PairPhase::BootstrapComplete) {
                return Err("invalid pairing completion phase".into());
            }
            row_size(row)?;
        }
        for row in &self.local {
            validate_header(&row.header)?;
            if row.issuer == local
                || row.id != claim_id(row.issuer, local, row.token_hash, row.nonce)
                || !ids.insert(row.id)
            {
                return Err("invalid or duplicate local pair intent".into());
            }
            if let Some(m) = &row.header.roster {
                authorize_blob(m, row.id, CALLER_ROSTER, local, row.issuer)?;
            }
            if let Some(m) = &row.peer_roster {
                authorize_blob(m, row.id, ISSUER_ROSTER, row.issuer, local)?;
            }
            if !row.peer_have.is_empty() {
                DocumentVersion::decode(&row.peer_have).map_err(Error::from_std)?;
            }
            if row.completed_ms.is_some() != (row.phase == PairPhase::BootstrapComplete) {
                return Err("invalid local pairing completion phase".into());
            }
            row_size(row)?;
        }
        let mut plans = std::collections::BTreeSet::new();
        for row in &self.settlements {
            let owner = if row.dialed {
                self.local
                    .iter()
                    .find(|c| c.id == row.id)
                    .map(|c| (c.issuer, c.peer_roster.as_ref()))
            } else {
                self.claims
                    .iter()
                    .find(|c| c.id == row.id)
                    .map(|c| (c.caller, c.header.roster.as_ref()))
            };
            if owner != Some((row.remote, row.input_roster.as_ref())) || !plans.insert(row.id) {
                return Err("unowned or duplicate settlement intent".into());
            }
            if row.finished && (!row.ready || row.target.is_none()) {
                return Err("unfinished settlement marked complete".into());
            }
            if let Some(m) = &row.desired_roster {
                authorize_blob(
                    m,
                    row.id,
                    if row.dialed {
                        LOCAL_SETTLEMENT
                    } else {
                        ISSUER_ROSTER
                    },
                    local,
                    row.remote,
                )?;
            }
            if let Some(target) = &row.target {
                DocumentVersion::decode(target).map_err(Error::from_std)?;
            }
            row_size(row)?;
        }
        let mut vouches = std::collections::BTreeSet::new();
        for row in &self.vouches {
            authorize_blob(&row.manifest, [0; 32], VOUCH_ROSTER, row.remote, local)?;
            if blob_id(&row.manifest)? != row.id || !vouches.insert(row.id) {
                return Err("invalid Vouch operation".into());
            }
            validate_scalars(&row.name, &row.addrs, 0)?;
            if let Some(target) = &row.target {
                DocumentVersion::decode(target).map_err(Error::from_std)?;
            }
            row_size(row)?;
        }
        let mut peers = std::collections::BTreeSet::new();
        for row in &self.vouch_out {
            authorize_blob(
                &row.manifest,
                [0; 32],
                VOUCH_ROSTER,
                local,
                row.manifest.recipient,
            )?;
            validate_scalars(&row.name, &row.addrs, row.certificate.len())?;
            if row.certificate.len() > 1024 || !peers.insert(row.manifest.recipient) {
                return Err("invalid or duplicate Vouch sender plan".into());
            }
            row_size(row)?;
        }
        Ok(())
    }
}
fn row_size(row: &impl Serialize) -> Result<(), Error> {
    if rmp_serde::to_vec(row).map_err(Error::from_std)?.len() > MAX_ROW {
        return Err(Error {
            kind: crate::proto::DocTransferFailure::ResourceLimit,
            why: "pairing record exceeds 192 KiB".into(),
        });
    }
    Ok(())
}
pub(crate) fn needs_recovery(root: &Path, local: Id) -> Result<bool, Error> {
    let directory = root.join(".khor/doc-transfer-state");
    // A complete next image may contain the first Ready intent. Bootstrap
    // must go through Journal::load before deciding that no recovery exists.
    if io::has_publication(&directory)? {
        return Ok(true);
    }
    let Some(bytes) = io::peek(&directory)? else {
        return Ok(false);
    };
    super::validate_records(&bytes, local)?;
    let rows: AllRecords = rmp_serde::from_slice(&bytes).map_err(Error::from_std)?;
    Ok(rows
        .pairing
        .settlements
        .iter()
        .any(|s| s.ready && !s.finished))
}
pub(crate) fn root_fence(root: &Path) -> Result<io::RootGate, Error> {
    io::root_gate(root)
}

impl State {
    pub(crate) fn abort_local_pair(&mut self, id: Id) -> Result<(), Error> {
        let row = self.pair_local(id).ok_or("unknown local pair intent")?;
        if row.phase != PairPhase::Receiving || self.pair_settlement(id).is_some_and(|s| s.ready) {
            return Err("committed local pairing cannot be rolled back".into());
        }
        for m in [
            row.header.roster,
            row.peer_roster,
            self.pair_settlement(id).and_then(|s| s.desired_roster),
        ]
        .into_iter()
        .flatten()
        {
            self.release_roster_blob(&m)?;
        }
        self.records.pairing.local.retain(|c| c.id != id);
        self.records.pairing.settlements.retain(|c| c.id != id);
        self.save()
    }
    pub(crate) fn abort_pair(&mut self, id: Id) -> Result<(), Error> {
        if self.pair_settlement(id).is_some_and(|s| s.ready) {
            return Err("admitted pairing cannot be rolled back".into());
        }
        let row = self.pair_claim(id).ok_or("unknown pairing claim")?;
        if row.phase != PairPhase::Receiving {
            return Err("admitted pairing cannot be rolled back".into());
        }
        if let Some(m) = row.header.roster {
            self.release_roster_blob(&m)?;
        }
        if let Some(m) = self.pair_settlement(id).and_then(|s| s.desired_roster) {
            self.release_roster_blob(&m)?;
        }
        self.records.pairing.settlements.retain(|s| s.id != id);
        self.records.pairing.claims.retain(|s| s.id != id);
        self.save()
    }
    /// Completed replays outlive both the ticket and a day of lost replies.
    /// Unfinished intents are never expired. Only exact referenced bodies go.
    pub(crate) fn expired_pair_ids(&self, now: u64) -> Vec<Id> {
        self.records
            .pairing
            .claims
            .iter()
            .filter(|c| {
                c.completed_ms
                    .is_some_and(|t| now.saturating_sub(t) > 24 * 60 * 60 * 1000)
                    && (now as i128 - c.minted_ms as i128) > crate::link::INVITE_WINDOW_MS as i128
            })
            .map(|c| c.id)
            .chain(
                self.records
                    .pairing
                    .local
                    .iter()
                    .filter(|c| {
                        c.completed_ms
                            .is_some_and(|t| now.saturating_sub(t) > 24 * 60 * 60 * 1000)
                    })
                    .map(|c| c.id),
            )
            .collect()
    }
    pub(crate) fn collect_pairing(&mut self, now: u64) -> Result<(), Error> {
        let old = self.expired_pair_ids(now);
        for id in old {
            let manifests: Vec<_> = self
                .pair_claim(id)
                .into_iter()
                .flat_map(|c| [c.header.roster, c.roster_out])
                .flatten()
                .chain(
                    self.pair_local(id)
                        .into_iter()
                        .flat_map(|c| [c.header.roster, c.peer_roster])
                        .flatten(),
                )
                .chain(self.pair_settlement(id).and_then(|s| s.desired_roster))
                .collect();
            for m in manifests {
                self.release_roster_blob(&m)?;
            }
            self.records.pairing.claims.retain(|c| c.id != id);
            self.records.pairing.local.retain(|c| c.id != id);
            self.records.pairing.settlements.retain(|c| c.id != id);
            self.save()?;
        }
        let retired: Vec<_> = self
            .records
            .pairing
            .vouches
            .iter()
            .filter(|v| {
                v.completed_ms
                    .is_some_and(|t| now.saturating_sub(t) > 24 * 60 * 60 * 1000)
            })
            .cloned()
            .collect();
        for row in retired {
            self.release_roster_blob(&row.manifest)?;
            self.records.pairing.vouches.retain(|v| v.id != row.id);
            self.save()?;
        }
        Ok(())
    }
    pub(crate) fn finish_vouch(&mut self, id: Id) -> Result<(), Error> {
        let row = self.vouch(id).ok_or("unknown Vouch operation")?;
        if row.phase != VouchPhase::Vouched {
            return Err("Vouch is not durably admitted".into());
        }
        self.release_roster_blob(&row.manifest)?;
        // Retain the small receipt for duplicate Finish after a lost reply.
        Ok(())
    }
    pub(crate) fn pair_claim(&self, id: Id) -> Option<Claim> {
        self.records
            .pairing
            .claims
            .iter()
            .find(|c| c.id == id)
            .cloned()
    }
    pub(crate) fn pair_local(&self, id: Id) -> Option<Local> {
        self.records
            .pairing
            .local
            .iter()
            .find(|c| c.id == id)
            .cloned()
    }
    pub(crate) fn local_for_ticket(&self, issuer: Id, token: Id) -> Option<Local> {
        self.records
            .pairing
            .local
            .iter()
            .find(|c| c.issuer == issuer && c.token_hash == token)
            .cloned()
    }
    pub(crate) fn claim_for_ticket(&self, token: Id) -> Option<Claim> {
        self.records
            .pairing
            .claims
            .iter()
            .find(|c| c.token_hash == token)
            .cloned()
    }
    pub(crate) fn save_claim(&mut self, row: Claim) -> Result<(), Error> {
        self.records.pairing.claims.retain(|c| c.id != row.id);
        self.records.pairing.claims.push(row);
        self.save()
    }
    pub(crate) fn save_local(&mut self, row: Local) -> Result<(), Error> {
        self.records.pairing.local.retain(|c| c.id != row.id);
        self.records.pairing.local.push(row);
        self.save()
    }
    pub(crate) fn pair_settlement(&self, id: Id) -> Option<Settlement> {
        self.records
            .pairing
            .settlements
            .iter()
            .find(|s| s.id == id)
            .cloned()
    }
    pub(crate) fn save_settlement(&mut self, row: Settlement) -> Result<(), Error> {
        self.records.pairing.settlements.retain(|s| s.id != row.id);
        self.records.pairing.settlements.push(row);
        self.save()
    }
    pub(crate) fn unfinished_settlements(&self) -> Vec<Settlement> {
        self.records
            .pairing
            .settlements
            .iter()
            .filter(|s| s.ready && !s.finished)
            .cloned()
            .collect()
    }
    pub(crate) fn remove_unready_settlement(&mut self, id: Id) -> Result<(), Error> {
        if self.pair_settlement(id).is_some_and(|s| s.ready) {
            return Err("cannot discard a committed settlement intent".into());
        }
        self.records.pairing.settlements.retain(|s| s.id != id);
        self.save()
    }
    pub(crate) fn vouch(&self, id: Id) -> Option<Vouch> {
        self.records
            .pairing
            .vouches
            .iter()
            .find(|v| v.id == id)
            .cloned()
    }
    pub(crate) fn save_vouch(&mut self, row: Vouch) -> Result<(), Error> {
        self.records.pairing.vouches.retain(|v| v.id != row.id);
        self.records.pairing.vouches.push(row);
        self.save()
    }
    pub(crate) fn vouch_sender(&self, remote: Id) -> Option<VouchOut> {
        self.records
            .pairing
            .vouch_out
            .iter()
            .find(|row| row.manifest.recipient == remote)
            .cloned()
    }
    pub(crate) fn prepare_vouch_blob(&mut self, row: VouchOut, bytes: &[u8]) -> Result<(), Error> {
        if self.vouch_sender(row.manifest.recipient).is_some() {
            return Err("a fixed Vouch sender plan already exists".into());
        }
        let manifest = row.manifest.clone();
        self.records.pairing.vouch_out.push(row);
        self.save()?;
        self.prepare_roster_blob(&manifest, bytes)
    }
    pub(crate) fn finish_vouch_blob(&mut self, manifest: &DocManifest) -> Result<(), Error> {
        self.blobs
            .cancel(&manifest.blob_core()?)
            .map_err(Error::from_std)?;
        self.records
            .pairing
            .vouch_out
            .retain(|row| &row.manifest != manifest);
        self.save()
    }
    pub(crate) fn prepare_roster_blob(
        &mut self,
        manifest: &DocManifest,
        bytes: &[u8],
    ) -> Result<(), Error> {
        if !self.records.pairing.references(manifest)
            || bytes.len() as u64 != manifest.total_bytes
            || blake3::hash(bytes).as_bytes() != &manifest.digest
        {
            return Err("unowned or changed roster preparation".into());
        }
        let mut sink = self
            .blobs
            .receive(manifest.blob_core()?)
            .map_err(Error::from_std)?;
        for (index, chunk) in bytes.chunks(CHUNK_BYTES).enumerate() {
            sink.append((index * CHUNK_BYTES) as u64, chunk)
                .map_err(Error::from_std)?;
        }
        if !matches!(
            sink.progress().map_err(Error::from_std)?,
            Progress::VerifiedComplete { .. }
        ) {
            return Err("unverified roster spool".into());
        }
        Ok(())
    }
    pub(crate) fn receive_roster_blob(
        &mut self,
        chunk: &DocChunk,
        create: bool,
    ) -> Result<VouchPhase, Error> {
        validate_chunk(chunk)?;
        if !self.records.pairing.references(&chunk.manifest) {
            return Err("unknown roster transfer".into());
        }
        let mut sink = if create {
            self.blobs
                .receive(chunk.manifest.blob_core()?)
                .map_err(Error::from_std)?
        } else {
            self.blobs
                .open_existing(&chunk.manifest.blob_core()?)
                .map_err(Error::from_std)?
                .ok_or("missing roster stage; resume its first header")?
        };
        Ok(
            match sink
                .append(chunk.offset, &chunk.bytes)
                .map_err(Error::from_std)?
            {
                Progress::Stored { next_offset } => VouchPhase::Stored { next_offset },
                Progress::VerifiedComplete { .. } => VouchPhase::VerifiedBlob,
            },
        )
    }
    pub(crate) fn roster_offset(&self, manifest: &DocManifest) -> Result<u64, Error> {
        if !self.records.pairing.references(manifest) {
            return Err("unknown roster transfer".into());
        }
        Ok(self
            .blobs
            .open_existing(&manifest.blob_core()?)
            .map_err(Error::from_std)?
            .map_or(0, |s| s.next_offset()))
    }
    pub(crate) fn roster_progress(
        &self,
        manifest: &DocManifest,
    ) -> Result<Option<Progress>, Error> {
        self.blobs
            .open_existing(&manifest.blob_core()?)
            .map_err(Error::from_std)?
            .map(|sink| sink.progress().map_err(Error::from_std))
            .transpose()
    }
    pub(crate) fn read_roster_blob(
        &self,
        manifest: &DocManifest,
        offset: u64,
    ) -> Result<DocChunk, Error> {
        if !self.records.pairing.references(manifest) || offset % CHUNK_BYTES as u64 != 0 {
            return Err("unknown or unaligned roster read".into());
        }
        let mut sink = self
            .blobs
            .open_existing(&manifest.blob_core()?)
            .map_err(Error::from_std)?
            .ok_or("missing roster spool")?;
        let remaining = manifest
            .total_bytes
            .checked_sub(offset)
            .ok_or("roster read beyond end")?;
        let mut bytes = vec![0; remaining.min(CHUNK_BYTES as u64) as usize];
        let read = sink
            .read_verified(offset, &mut bytes)
            .map_err(Error::from_std)?;
        bytes.truncate(read);
        drop(sink);
        Ok(DocChunk {
            manifest: manifest.clone(),
            offset,
            bytes: bytes.into(),
        })
    }
    pub(crate) fn load_roster_blob(&self, manifest: &DocManifest) -> Result<Vec<u8>, Error> {
        if manifest.total_bytes > MAX_BLOB_BYTES || !self.records.pairing.references(manifest) {
            return Err("invalid roster body".into());
        }
        let mut sink = self
            .blobs
            .open_existing(&manifest.blob_core()?)
            .map_err(Error::from_std)?
            .ok_or("missing complete roster")?;
        if !matches!(
            sink.progress().map_err(Error::from_std)?,
            Progress::VerifiedComplete { .. }
        ) {
            return Err("roster still pending".into());
        }
        let mut body = Vec::with_capacity(manifest.total_bytes as usize);
        let mut buffer = vec![0; CHUNK_BYTES];
        while (body.len() as u64) < manifest.total_bytes {
            let read = sink
                .read_verified(body.len() as u64, &mut buffer)
                .map_err(Error::from_std)?;
            if read == 0 {
                return Err("incomplete verified roster read".into());
            }
            body.extend_from_slice(&buffer[..read]);
        }
        Ok(body)
    }
    pub(crate) fn release_roster_blob(&mut self, manifest: &DocManifest) -> Result<(), Error> {
        self.blobs
            .cancel(&manifest.blob_core()?)
            .map_err(Error::from_std)?;
        Ok(())
    }
    fn pair_document_ids(
        &self,
        remote: Id,
        operation: Id,
    ) -> Result<(Option<String>, Option<String>), Error> {
        if let Some(row) = self.pair_claim(operation) {
            if row.caller != remote {
                return Err("pairing document owner mismatch".into());
            }
            Ok((row.devices_in, row.devices_out))
        } else if let Some(row) = self.pair_local(operation) {
            if row.issuer != remote {
                return Err("pairing document owner mismatch".into());
            }
            Ok((row.devices_in, row.devices_out))
        } else {
            Err("unknown pairing document operation".into())
        }
    }
    fn pair_document_matches(
        manifest: &DocManifest,
        expected: Option<&str>,
        source: Id,
        recipient: Id,
    ) -> Result<bool, Error> {
        if expected != Some(manifest.id()?.as_str()) {
            return Ok(false);
        }
        super::authorize(manifest, source, recipient)?;
        if manifest.descriptor()?.document != "devices" {
            return Err("document is outside this pairing grant".into());
        }
        Ok(true)
    }
    pub(crate) fn pair_document_proofs(
        &self,
        remote: Id,
        operation: Id,
        limit: usize,
    ) -> Result<Vec<DocProof>, Error> {
        let (incoming, _) = self.pair_document_ids(remote, operation)?;
        let mut proofs = Vec::new();
        for row in &self.records.incoming {
            if proofs.len() == limit {
                break;
            }
            if let super::Incoming::Complete { proof, .. } = row
                && Self::pair_document_matches(
                    &proof.manifest,
                    incoming.as_deref(),
                    remote,
                    self.local,
                )?
            {
                proofs.push(proof.clone());
            }
        }
        Ok(proofs)
    }
    pub(crate) fn pair_document_acknowledgements(
        &self,
        remote: Id,
        operation: Id,
        limit: usize,
    ) -> Result<Vec<DocAcknowledgement>, Error> {
        let (_, outgoing) = self.pair_document_ids(remote, operation)?;
        let mut acknowledgements = Vec::new();
        for row in &self.records.outgoing {
            if acknowledgements.len() == limit {
                break;
            }
            if let super::Outgoing::NeedAck(proof) = row
                && Self::pair_document_matches(
                    &proof.manifest,
                    outgoing.as_deref(),
                    self.local,
                    remote,
                )?
            {
                acknowledgements.push(proof.acknowledgement()?);
            }
        }
        Ok(acknowledgements)
    }
    pub(crate) fn pair_receipt_answers(
        &mut self,
        context: &Context,
        remote: Id,
        operation: Id,
        acknowledgements: &[DocAcknowledgement],
        results: &[DocAckResult],
    ) -> Result<(), Error> {
        let (incoming, outgoing) = self.pair_document_ids(remote, operation)?;
        // Check the entire correlated reply before changing any receipt.
        for ack in acknowledgements {
            if !Self::pair_document_matches(
                &ack.manifest,
                incoming.as_deref(),
                remote,
                self.local,
            )? {
                return Err("receipt is outside this pairing grant".into());
            }
        }
        for result in results {
            let manifest = match result {
                DocAckResult::Reconfirm { proof } => &proof.manifest,
                DocAckResult::Released { acknowledgement }
                | DocAckResult::Absent { acknowledgement } => &acknowledgement.manifest,
            };
            if !Self::pair_document_matches(
                manifest,
                outgoing.as_deref(),
                self.local,
                remote,
            )? {
                return Err("receipt result is outside this pairing grant".into());
            }
        }
        for ack in acknowledgements {
            self.acknowledge(remote, ack)?;
        }
        for result in results {
            self.acknowledged(context, remote, result)?;
        }
        Ok(())
    }
    pub(crate) fn pair_documents_pending(
        &self,
        remote: Id,
        operation: Id,
    ) -> Result<bool, Error> {
        let (incoming, outgoing) = self.pair_document_ids(remote, operation)?;
        for row in &self.records.incoming {
            if Self::pair_document_matches(
                row.manifest(),
                incoming.as_deref(),
                remote,
                self.local,
            )? {
                return Ok(true);
            }
        }
        for row in &self.records.outgoing {
            if Self::pair_document_matches(
                row.manifest(),
                outgoing.as_deref(),
                self.local,
                remote,
            )? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(crate) fn pair_offer(
        &mut self,
        context: &Context,
        remote: Id,
        operation: Id,
        have: &DocumentVersion,
    ) -> Result<Option<DocChunk>, Error> {
        let (decided, key) = if let Some(c) = self.pair_claim(operation) {
            (c.devices_offer_decided, c.devices_out)
        } else {
            let c = self
                .pair_local(operation)
                .ok_or("unknown local pair offer")?;
            (c.devices_offer_decided, c.devices_out)
        };
        if decided {
            let manifest = key.as_ref().and_then(|id| {
                self.records.outgoing.iter().find_map(|r| match r {
                    super::Outgoing::Ready(m) if m.id().is_ok_and(|key| &key == id) => {
                        Some(m.clone())
                    }
                    _ => None,
                })
            });
            return manifest
                .map(|m| self.read_outgoing(context, remote, &m, 0))
                .transpose();
        }
        // Bootstrap must preserve the existing complete history snapshot, whose
        // size can be much smaller than a full Updates history. This is the
        // same canonical DocTransfer encoding/checked-import path.
        let chunk = self.offer_encoded(
            context,
            remote,
            "devices",
            have,
            khor_sync::doc_transfer::Encoding::Snapshot,
        )?;
        let key = chunk.as_ref().map(|c| c.manifest.id()).transpose()?;
        if let Some(mut c) = self.pair_claim(operation) {
            c.devices_offer_decided = true;
            c.devices_out = key;
            self.save_claim(c)?;
        } else {
            let mut c = self
                .pair_local(operation)
                .ok_or("unknown local pair offer")?;
            c.devices_offer_decided = true;
            c.devices_out = key;
            self.save_local(c)?;
        }
        Ok(chunk)
    }
    pub(crate) fn pair_receive(
        &mut self,
        context: &Context,
        remote: Id,
        operation: Id,
        chunk: &DocChunk,
        first: bool,
    ) -> Result<super::DocTransferState, Error> {
        let key = chunk.manifest.id()?;
        let expected = if let Some(c) = self.pair_claim(operation) {
            c.devices_have
        } else {
            self.pair_local(operation)
                .ok_or("unknown local pair receive")?
                .header
                .devices_have
        };
        let expected = DocumentVersion::decode(&expected).map_err(Error::from_std)?;
        let descriptor = chunk.manifest.descriptor()?;
        if descriptor.document != "devices"
            || descriptor.epoch != expected.epoch
            || descriptor.base != expected.version
        {
            return Err("pairing incoming base or epoch changed".into());
        }
        if let Some(mut c) = self.pair_claim(operation) {
            if c.devices_in.as_ref().is_some_and(|old| old != &key) {
                return Err("pairing incoming target changed".into());
            }
            c.devices_in = Some(key);
            self.save_claim(c)?;
        } else {
            let mut c = self
                .pair_local(operation)
                .ok_or("unknown local pair receive")?;
            if c.devices_in.as_ref().is_some_and(|old| old != &key) {
                return Err("pairing incoming target changed".into());
            }
            c.devices_in = Some(key);
            self.save_local(c)?;
        }
        self.receive(context, remote, chunk, first)
    }
    pub(crate) fn recover_pairing(
        &mut self,
        node: &PairTrust,
        context: &Context,
    ) -> Result<(), Error> {
        for settlement in self.unfinished_settlements() {
            node.apply_pair_settlement(context, self, settlement.id)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn adding_pairing_preserves_the_existing_empty_doc_journal_bytes() {
        #[derive(serde::Serialize)]
        struct Before {
            outgoing: Vec<u8>,
            incoming: Vec<u8>,
        }
        let before = rmp_serde::to_vec(&Before {
            outgoing: vec![],
            incoming: vec![],
        })
        .unwrap();
        let after = rmp_serde::to_vec(&super::AllRecords::default()).unwrap();
        assert_eq!(before, after);
        super::super::validate_records(&before, [42; 32]).unwrap();
    }
}
