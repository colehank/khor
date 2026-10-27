//! Node-owned immutable plans and recoverable import receipts. Blob bytes and
//! ownership remain exclusively in khor_sync::transfer.
use crate::proto::{
    DocAckResult, DocAcknowledgement, DocChunk, DocManifest, DocProof, DocProofKind,
    DocTransferState,
};
use khor_sync::{
    doc_transfer::{
        self, Descriptor, DocumentVersion, Encoding, PrepareSpec, Prepared, ReceiveSpec,
    },
    store::Doc,
    transfer::{Manifest, Progress, TransferStore},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};
mod io;
pub(crate) use io::RootGate as RootMutationGuard;
pub(crate) use io::publish_anchor;
pub(crate) mod pairing;
pub(crate) mod registry;
use registry::with_doc;

#[derive(Clone, Debug)]
pub(crate) struct Error {
    pub kind: crate::proto::DocTransferFailure,
    pub why: String,
}
impl Error {
    pub(crate) fn busy() -> Self {
        Self {
            kind: crate::proto::DocTransferFailure::Busy,
            why: "document transfer state busy".into(),
        }
    }
    pub(crate) fn from_std<E: std::error::Error + 'static>(error: E) -> Self {
        let mut current: &(dyn std::error::Error + 'static) = &error;
        let mut kind = crate::proto::DocTransferFailure::Storage;
        loop {
            if let Some(owned) = current.downcast_ref::<Self>() {
                return owned.clone();
            }
            if let Some(core) = current.downcast_ref::<khor_sync::transfer::Error>() {
                match core {
                    khor_sync::transfer::Error::Busy => {
                        kind = crate::proto::DocTransferFailure::Busy
                    }
                    khor_sync::transfer::Error::ResourceLimit(_) => {
                        kind = crate::proto::DocTransferFailure::ResourceLimit
                    }
                    _ => (),
                }
            }
            if current
                .downcast_ref::<khor_sync::doc_transfer::ReceiveError>()
                .is_some()
                && kind == crate::proto::DocTransferFailure::Storage
            {
                kind = crate::proto::DocTransferFailure::Apply;
            }
            let Some(source) = current.source() else {
                break;
            };
            current = source;
        }
        Self {
            kind,
            why: error.to_string(),
        }
    }
}
impl From<String> for Error {
    fn from(why: String) -> Self {
        Self {
            kind: crate::proto::DocTransferFailure::Storage,
            why,
        }
    }
}
impl From<&str> for Error {
    fn from(why: &str) -> Self {
        Self {
            kind: crate::proto::DocTransferFailure::Invalid,
            why: why.into(),
        }
    }
}
impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.why
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.why, f)
    }
}
impl std::error::Error for Error {}

const MAX_RECORDS: usize = 64;
const MAX_RECORD_BYTES: usize = 192 * 1024;

impl DocManifest {
    /// Shared byte envelope; the purpose owner must validate its binding.
    pub(crate) fn blob_core(&self) -> Result<Manifest, Error> {
        let value = Manifest {
            source: self.source,
            recipient: self.recipient,
            binding: self.binding.to_vec(),
            total_bytes: self.total_bytes,
            digest: self.digest,
        };
        value.validate().map_err(Error::from_std)?;
        Ok(value)
    }
    pub(crate) fn core(&self) -> Result<Manifest, Error> {
        let value = self.blob_core()?;
        Descriptor::from_manifest(&value).map_err(Error::from_std)?;
        Ok(value)
    }
    pub(crate) fn id(&self) -> Result<String, Error> {
        Ok(self.core()?.id().map_err(Error::from_std)?.to_string())
    }
    pub(crate) fn descriptor(&self) -> Result<Descriptor, Error> {
        Descriptor::from_manifest(&self.core()?).map_err(Error::from_std)
    }
}
impl From<&Manifest> for DocManifest {
    fn from(value: &Manifest) -> Self {
        Self {
            source: value.source,
            recipient: value.recipient,
            binding: value.binding.clone().into(),
            total_bytes: value.total_bytes,
            digest: value.digest,
        }
    }
}
impl DocProof {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        let descriptor = self.manifest.descriptor()?;
        let version = DocumentVersion::decode(&self.persisted_version).map_err(Error::from_std)?;
        if version.epoch != descriptor.epoch
            || !version.version.includes_vv(&descriptor.base)
            || !version.version.includes_vv(&descriptor.target)
        {
            return Err("completion proof does not cover its fixed document target".into());
        }
        Ok(())
    }
    pub(crate) fn acknowledgement(&self) -> Result<DocAcknowledgement, Error> {
        self.validate()?;
        let mut hash = blake3::Hasher::new();
        hash.update(b"khor-doc-proof-v1\0");
        hash.update(self.manifest.id()?.as_bytes());
        hash.update(&self.persisted_version);
        hash.update(&[match self.kind {
            DocProofKind::Applied => 0,
            DocProofKind::AlreadyHaveEquivalent => 1,
        }]);
        Ok(DocAcknowledgement {
            manifest: self.manifest.clone(),
            proof_hash: *hash.finalize().as_bytes(),
        })
    }
}

#[derive(Clone)]
pub(crate) struct Context {
    pub root: PathBuf,
    pub writer: u64,
    pub local: [u8; 32],
}
impl Context {
    pub(crate) fn root(&self) -> &PathBuf {
        &self.root
    }
    pub(crate) fn writer_peer(&self) -> u64 {
        self.writer
    }
    pub(crate) fn version(&self, name: &str) -> Result<DocumentVersion, Error> {
        with_doc!(self, name, |loaded| Ok(DocumentVersion {
            epoch: loaded.store.epoch(),
            version: loaded.doc.version()
        }))
    }
    fn apply(
        &self,
        name: &str,
        remote: [u8; 32],
        sink: &mut khor_sync::transfer::Sink,
    ) -> Result<(DocumentVersion, bool, Option<u64>), Error> {
        let _trust = if name == "devices" {
            let guard = io::root_gate(&self.root)?;
            if pairing::needs_recovery(&self.root, self.local)? {
                return Err(Error::busy());
            }
            Some(guard)
        } else { None };
        with_doc!(self, name, |loaded| {
            let before = loaded.doc.version();
            let result = doc_transfer::apply_staged(
                loaded,
                &ReceiveSpec {
                    document: name.into(),
                    source: remote,
                    recipient: self.local,
                },
                sink,
            )
            .map_err(Error::from_std)?;
            Ok((
                DocumentVersion {
                    epoch: result.epoch(),
                    version: result.version().clone(),
                },
                before != *result.version(),
                result.retired_pen(),
            ))
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
enum Outgoing {
    Preparing(DocManifest),
    Ready(DocManifest),
    Accepted(DocProof),
    NeedAck(DocProof),
}
impl Outgoing {
    fn manifest(&self) -> &DocManifest {
        match self {
            Self::Preparing(m) | Self::Ready(m) => m,
            Self::Accepted(p) | Self::NeedAck(p) => &p.manifest,
        }
    }
    fn active(&self) -> bool {
        matches!(self, Self::Preparing(_) | Self::Ready(_))
    }
}
#[derive(Clone, Serialize, Deserialize)]
enum Incoming {
    Receiving(DocManifest),
    Importing {
        manifest: DocManifest,
        before: serde_bytes::ByteBuf,
    },
    Complete {
        proof: DocProof,
        notify: bool,
        retired_pen: Option<u64>,
    },
    Cleanup(DocProof),
}
impl Incoming {
    fn manifest(&self) -> &DocManifest {
        match self {
            Self::Receiving(m) => m,
            Self::Importing { manifest, .. } => manifest,
            Self::Complete { proof, .. } | Self::Cleanup(proof) => &proof.manifest,
        }
    }
}
#[derive(Default, Serialize, Deserialize)]
struct Records {
    outgoing: Vec<Outgoing>,
    incoming: Vec<Incoming>,
    // Last and omitted when empty, so pre-pairing N1 journals keep their
    // canonical two-field representation and all retained Doc stages.
    #[serde(default, skip_serializing_if = "pairing::Records::is_empty")]
    pairing: pairing::Records,
}

pub(crate) struct Notice {
    pub id: String,
    pub document: String,
    pub remote: [u8; 32],
    pub retired_pen: Option<u64>,
}

pub(crate) struct State {
    _gate: io::Gate,
    journal: io::Journal,
    records: Records,
    pub(crate) blobs: TransferStore,
    local: [u8; 32],
}
impl State {
    pub(crate) fn open(context: &Context) -> Result<Self, Error> {
        let root = context.root.join(".khor/doc-transfer-state");
        let gate = io::gate(&root)?;
        let (journal, body) = io::Journal::load(&root, |bytes| {
            validate_records(bytes, context.local).map_err(String::from)
        })?;
        let records = match body {
            Some(bytes) => rmp_serde::from_slice(&bytes).map_err(Error::from_std)?,
            None => Records::default(),
        };
        let blobs = TransferStore::open(context.root.join(".khor/doc-transfer-blobs"))
            .map_err(Error::from_std)?;
        let mut state = Self {
            _gate: gate,
            journal,
            records,
            blobs,
            local: context.local,
        };
        state.reconcile_references()?;
        state.recover_completed_cleanup()?;
        Ok(state)
    }
    fn recover_completed_cleanup(&mut self) -> Result<(), Error> {
        let mut changed = false;
        for index in 0..self.records.outgoing.len() {
            if let Outgoing::Accepted(proof) = self.records.outgoing[index].clone() {
                self.blobs
                    .cancel(&proof.manifest.core()?)
                    .map_err(Error::from_std)?;
                self.records.outgoing[index] = Outgoing::NeedAck(proof);
                changed = true;
            }
        }
        let cleanup: Vec<_> = self
            .records
            .incoming
            .iter()
            .filter_map(|r| match r {
                Incoming::Cleanup(p) => Some(p.manifest.clone()),
                _ => None,
            })
            .collect();
        for manifest in cleanup {
            self.blobs
                .cancel(&manifest.core()?)
                .map_err(Error::from_std)?;
            self.records.incoming.retain(|r| r.manifest() != &manifest);
            changed = true;
        }
        if changed {
            self.save()?;
        }
        Ok(())
    }
    fn save(&mut self) -> Result<(), Error> {
        let bytes = rmp_serde::to_vec(&self.records).map_err(Error::from_std)?;
        validate_records(&bytes, self.local)?;
        self.journal.save(&bytes).map_err(Error::from)
    }
    fn reconcile_references(&self) -> Result<(), Error> {
        for manifest in self.blobs.retained_manifests().map_err(Error::from_std)? {
            let wire = DocManifest::from(&manifest);
            let referenced = self.records.incoming.iter().any(|r| r.manifest() == &wire)
                || self.records.outgoing.iter().any(|r| {
                    if let Outgoing::Preparing(expected) = r {
                        same_preparing(expected, &wire)
                    } else {
                        r.manifest() == &wire
                    }
                });
            if !referenced && !self.records.pairing.references(&wire) {
                return Err(
                    "unreferenced retained document stage; explicit reconciliation required".into(),
                );
            }
        }
        Ok(())
    }
    pub(crate) fn pending_docs(&self, remote: [u8; 32]) -> Result<Vec<String>, Error> {
        let mut names = BTreeSet::new();
        for manifest in self
            .records
            .outgoing
            .iter()
            .map(Outgoing::manifest)
            .chain(self.records.incoming.iter().map(Incoming::manifest))
        {
            if manifest.source == remote || manifest.recipient == remote {
                names.insert(manifest.descriptor()?.document);
            }
        }
        Ok(names.into_iter().collect())
    }
    pub(crate) fn pending_proofs(&self, remote: [u8; 32], limit: usize) -> Vec<DocProof> {
        self.scoped_proofs(remote, limit, false)
    }
    pub(crate) fn scoped_proofs(&self, remote: [u8; 32], limit: usize, devices_only: bool) -> Vec<DocProof> {
        self.records
            .incoming
            .iter()
            .filter_map(|r| match r {
                Incoming::Complete { proof, .. } if proof.manifest.source == remote
                    && (!devices_only || proof.manifest.descriptor().is_ok_and(|d| d.document == "devices")) => {
                    Some(proof.clone())
                }
                _ => None,
            })
            .take(limit)
            .collect()
    }
    pub(crate) fn pending_acknowledgements(
        &self,
        remote: [u8; 32],
        limit: usize,
    ) -> Result<Vec<DocAcknowledgement>, Error> {
        self.scoped_acknowledgements(remote, limit, false)
    }
    pub(crate) fn scoped_acknowledgements(&self, remote: [u8; 32], limit: usize, devices_only: bool) -> Result<Vec<DocAcknowledgement>, Error> {
        self.records
            .outgoing
            .iter()
            .filter_map(|r| match r {
                Outgoing::NeedAck(proof) if proof.manifest.recipient == remote
                    && (!devices_only || proof.manifest.descriptor().is_ok_and(|d| d.document == "devices")) => {
                    Some(proof.acknowledgement())
                }
                _ => None,
            })
            .take(limit)
            .collect()
    }
    pub(crate) fn notice_ids(&self, context: &Context) -> Result<Vec<Notice>, Error> {
        let mut notices = Vec::new();
        for row in &self.records.incoming {
            if let Incoming::Complete {
                proof,
                notify: true,
                retired_pen,
            } = row
            {
                let descriptor = proof.manifest.descriptor()?;
                let saved =
                    DocumentVersion::decode(&proof.persisted_version).map_err(Error::from_std)?;
                if let Ok(current) = context.version(&descriptor.document) {
                    if current.epoch == saved.epoch && current.version.includes_vv(&saved.version) {
                        notices.push(Notice {
                            id: proof.manifest.id()?,
                            document: descriptor.document,
                            remote: proof.manifest.source,
                            retired_pen: *retired_pen,
                        });
                    }
                }
            }
        }
        Ok(notices)
    }
    pub(crate) fn mark_notified(&mut self, id: &str) -> Result<(), Error> {
        for row in &mut self.records.incoming {
            if let Incoming::Complete { proof, notify, .. } = row {
                if proof.manifest.id()? == id {
                    *notify = false;
                }
            }
        }
        self.save()
    }
    pub(crate) fn offer(
        &mut self,
        context: &Context,
        remote: [u8; 32],
        name: &str,
        have: &DocumentVersion,
    ) -> Result<Option<DocChunk>, Error> {
        self.offer_encoded(context, remote, name, have, Encoding::Updates)
    }
    fn offer_encoded(&mut self, context: &Context, remote: [u8;32], name: &str, have: &DocumentVersion, encoding: Encoding) -> Result<Option<DocChunk>, Error> {
        let active = self.records.outgoing.iter().position(|r| {
            r.active()
                && r.manifest().recipient == remote
                && r.manifest().descriptor().is_ok_and(|d| d.document == name)
        });
        if let Some(index) = active {
            if encoding == Encoding::Snapshot {
                let active = self.records.outgoing[index].manifest().descriptor()?;
                if active.encoding != encoding
                    || active.epoch != have.epoch
                    || active.base != have.version
                {
                    // A normal plan keeps its immutable target and owner;
                    // Pair must wait rather than change its fixed grant base.
                    return Err(Error::busy());
                }
            }
            match self.records.outgoing[index].clone() {
                Outgoing::Ready(manifest) => {
                    return self.read_outgoing(context, remote, &manifest, 0).map(Some);
                }
                Outgoing::Preparing(expected) => {
                    let matches: Vec<_> = self
                        .blobs
                        .retained_manifests()
                        .map_err(Error::from_std)?
                        .into_iter()
                        .map(|m| DocManifest::from(&m))
                        .filter(|m| same_preparing(&expected, m))
                        .collect();
                    if matches.len() > 1 {
                        return Err(
                            "ambiguous prepared document stages; reconciliation required".into(),
                        );
                    }
                    if let Some(manifest) = matches.into_iter().next() {
                        let core = manifest.core()?;
                        let sink = self
                            .blobs
                            .open_existing(&core)
                            .map_err(Error::from_std)?
                            .ok_or("prepared stage disappeared")?;
                        let complete = matches!(
                            sink.progress().map_err(Error::from_std)?,
                            Progress::VerifiedComplete { .. }
                        );
                        drop(sink);
                        if complete {
                            self.records.outgoing[index] = Outgoing::Ready(manifest.clone());
                            self.save()?;
                            return self.read_outgoing(context, remote, &manifest, 0).map(Some);
                        }
                        // Preparing has never been offered. Its exact incomplete
                        // spool is disposable; flushed user history stays intact.
                        self.blobs.cancel(&core).map_err(Error::from_std)?;
                    }
                    self.records.outgoing.remove(index);
                    self.save()?;
                }
                _ => unreachable!(),
            }
        }
        with_doc!(context, name, |loaded| {
            let descriptor = Descriptor {
                document: name.into(),
                epoch: loaded.store.epoch(),
                encoding,
                base: have.version.clone(),
                target: loaded.doc.version(),
            };
            if descriptor.epoch != have.epoch {
                return Err("document epochs differ".into());
            }
            let expected = DocManifest::from(
                &descriptor
                    .bind(context.local, remote, 0, [0; 32])
                    .map_err(Error::from_std)?,
            );
            self.records
                .outgoing
                .push(Outgoing::Preparing(expected.clone()));
            self.save()?;
            match doc_transfer::prepare(
                loaded,
                &PrepareSpec {
                    document: name.into(),
                    source: context.local,
                    recipient: remote,
                    receiver_epoch: have.epoch,
                    base: have.version.clone(),
                    encoding,
                },
                &self.blobs,
            )
            .map_err(Error::from_std)?
            {
                Prepared::UpToDate { .. } => {
                    self.remove_preparing(&expected);
                    self.save()?;
                    Ok(None)
                }
                Prepared::Transfer(mut outbound) => {
                    let manifest = DocManifest::from(outbound.manifest());
                    if !same_preparing(&expected, &manifest) {
                        return Err("prepared target changed".into());
                    }
                    let bytes = outbound.read_chunk(0).map_err(Error::from_std)?;
                    self.remove_preparing(&expected);
                    self.records
                        .outgoing
                        .push(Outgoing::Ready(manifest.clone()));
                    self.save()?;
                    drop(outbound);
                    Ok(Some(DocChunk {
                        manifest,
                        offset: 0,
                        bytes: bytes.into(),
                    }))
                }
            }
        })
    }
    fn remove_preparing(&mut self, expected: &DocManifest) {
        self.records
            .outgoing
            .retain(|r| !matches!(r,Outgoing::Preparing(m) if m==expected));
    }
    pub(crate) fn read_outgoing(
        &self,
        context: &Context,
        remote: [u8; 32],
        manifest: &DocManifest,
        offset: u64,
    ) -> Result<DocChunk, Error> {
        authorize(manifest, context.local, remote)?;
        if !self
            .records
            .outgoing
            .iter()
            .any(|r| matches!(r,Outgoing::Ready(m) if m==manifest))
        {
            return Err("unknown outgoing document plan".into());
        }
        if context.version(&manifest.descriptor()?.document)?.epoch != manifest.descriptor()?.epoch
        {
            return Err("outgoing document epoch changed".into());
        }
        // Ready is published only for a locally prepared, body-validated
        // immutable spool. Reopen still recovers/checks every core byte and
        // digest; repeated chunks need not decode that same Loro body again.
        let core = manifest.core()?;
        let mut sink = self.blobs.open_existing(&core)
            .map_err(Error::from_std)?
            .ok_or("outgoing document body is missing")?;
        if !matches!(sink.progress().map_err(Error::from_std)?, Progress::VerifiedComplete { .. }) {
            return Err(Error::from_std(khor_sync::transfer::Error::InvalidStage("outgoing body is not verified complete")));
        }
        let total = core.total_bytes;
        if offset > total || (offset != total && offset % khor_sync::transfer::CHUNK_BYTES as u64 != 0) {
            return Err(Error::from_std(khor_sync::transfer::Error::InvalidChunk("noncanonical outbound read offset")));
        }
        let count = (total - offset).min(khor_sync::transfer::CHUNK_BYTES as u64) as usize;
        let mut bytes = vec![0; count];
        let mut filled = 0;
        while filled < count {
            let read = sink.read_verified(offset + filled as u64, &mut bytes[filled..]).map_err(Error::from_std)?;
            if read == 0 {
                return Err(Error::from_std(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "verified outbound body was truncated")));
            }
            filled += read;
        }
        drop(sink);
        Ok(DocChunk {
            manifest: manifest.clone(),
            offset,
            bytes: bytes.into(),
        })
    }
    pub(crate) fn incoming_offset(&self, manifest: &DocManifest) -> Result<u64, Error> {
        if !self
            .records
            .incoming
            .iter()
            .any(|r| r.manifest() == manifest)
        {
            return Ok(0);
        }
        let Some(sink) = self
            .blobs
            .open_existing(&manifest.core()?)
            .map_err(Error::from_std)?
        else {
            return Ok(0);
        };
        Ok(sink.next_offset())
    }
    pub(crate) fn receive(
        &mut self,
        context: &Context,
        remote: [u8; 32],
        chunk: &DocChunk,
        create: bool,
    ) -> Result<DocTransferState, Error> {
        authorize(&chunk.manifest, remote, context.local)?;
        let descriptor = chunk.manifest.descriptor()?;
        if !create
            && !self
                .records
                .incoming
                .iter()
                .any(|r| r.manifest() == &chunk.manifest)
        {
            return Ok(DocTransferState::Missing);
        }
        let version = context.version(&descriptor.document)?;
        if version.epoch != descriptor.epoch {
            return Ok(DocTransferState::Apart {
                expected: descriptor.epoch,
                actual: version.epoch,
            });
        }
        if let Some(state) = self.completed(context, &chunk.manifest, &version)? {
            return Ok(state);
        }
        let index = self
            .records
            .incoming
            .iter()
            .position(|r| r.manifest() == &chunk.manifest);
        if index.is_none() && !create {
            return Ok(DocTransferState::Missing);
        }
        if index.is_none() {
            self.records
                .incoming
                .push(Incoming::Receiving(chunk.manifest.clone()));
            self.save()?;
        }
        let mut sink = if create {
            self.blobs
                .receive(chunk.manifest.core()?)
                .map_err(Error::from_std)?
        } else {
            let Some(sink) = self
                .blobs
                .open_existing(&chunk.manifest.core()?)
                .map_err(Error::from_std)?
            else {
                return Ok(DocTransferState::Missing);
            };
            sink
        };
        let progress = sink
            .append(chunk.offset, &chunk.bytes)
            .map_err(Error::from_std)?;
        if let Progress::Stored { next_offset } = progress {
            return Ok(DocTransferState::Stored { next_offset });
        }
        let before = context.version(&descriptor.document)?;
        let index = self
            .records
            .incoming
            .iter()
            .position(|r| r.manifest() == &chunk.manifest)
            .ok_or("incoming record disappeared")?;
        self.records.incoming[index] = Incoming::Importing {
            manifest: chunk.manifest.clone(),
            before: before.encode().map_err(Error::from_std)?.into(),
        };
        self.save()?;
        let (version, changed, retired_pen) =
            context.apply(&descriptor.document, remote, &mut sink)?;
        drop(sink);
        #[cfg(all(test, unix))]
        crash_tests::after_flush(context, &chunk.manifest, self);
        let proof = DocProof {
            manifest: chunk.manifest.clone(),
            persisted_version: version.encode().map_err(Error::from_std)?.into(),
            kind: DocProofKind::Applied,
        };
        self.records.incoming[index] = Incoming::Complete {
            proof: proof.clone(),
            notify: changed,
            retired_pen,
        };
        self.save()?;
        Ok(DocTransferState::Complete { proof })
    }
    fn completed(
        &mut self,
        _context: &Context,
        manifest: &DocManifest,
        version: &DocumentVersion,
    ) -> Result<Option<DocTransferState>, Error> {
        let descriptor = manifest.descriptor()?;
        let index = self
            .records
            .incoming
            .iter()
            .position(|r| r.manifest() == manifest);
        if let Some(index) = index {
            if let Incoming::Complete { proof, .. } = self.records.incoming[index].clone() {
                let saved =
                    DocumentVersion::decode(&proof.persisted_version).map_err(Error::from_std)?;
                if !version.version.includes_vv(&saved.version) {
                    return Err(
                        "persisted receipt version is no longer present; reconciliation required"
                            .into(),
                    );
                }
                return Ok(Some(DocTransferState::Complete { proof }));
            }
        }
        if !version.version.includes_vv(&descriptor.base)
            || !version.version.includes_vv(&descriptor.target)
        {
            return Ok(None);
        }
        let changed = match index.map(|i| &self.records.incoming[i]) {
            Some(Incoming::Importing { before, .. }) => {
                DocumentVersion::decode(before)
                    .map_err(Error::from_std)?
                    .version
                    != version.version
            }
            _ => false,
        };
        let proof = DocProof {
            manifest: manifest.clone(),
            persisted_version: version.encode().map_err(Error::from_std)?.into(),
            kind: DocProofKind::AlreadyHaveEquivalent,
        };
        let row = Incoming::Complete {
            proof: proof.clone(),
            notify: changed,
            retired_pen: None,
        };
        if let Some(index) = index {
            self.records.incoming[index] = row;
        } else {
            self.records.incoming.push(row);
        }
        self.save()?;
        Ok(Some(DocTransferState::Complete { proof }))
    }
    pub(crate) fn status(
        &mut self,
        context: &Context,
        remote: [u8; 32],
        manifest: &DocManifest,
    ) -> Result<DocTransferState, Error> {
        authorize(manifest, remote, context.local)?;
        let descriptor = manifest.descriptor()?;
        let version = context.version(&descriptor.document)?;
        if version.epoch != descriptor.epoch {
            return Ok(DocTransferState::Apart {
                expected: descriptor.epoch,
                actual: version.epoch,
            });
        }
        if let Some(state) = self.completed(context, manifest, &version)? {
            return Ok(state);
        }
        if !self
            .records
            .incoming
            .iter()
            .any(|r| r.manifest() == manifest)
        {
            return Ok(DocTransferState::Missing);
        }
        let Some(sink) = self
            .blobs
            .open_existing(&manifest.core()?)
            .map_err(Error::from_std)?
        else {
            return Ok(DocTransferState::Missing);
        };
        Ok(DocTransferState::Stored {
            next_offset: sink.next_offset(),
        })
    }
    pub(crate) fn accept(
        &mut self,
        context: &Context,
        remote: [u8; 32],
        proof: &DocProof,
        confirmation_request: bool,
    ) -> Result<DocAcknowledgement, Error> {
        self.validate_completion(context, remote, proof)?;
        self.accept_checked(proof, confirmation_request)
    }
    pub(crate) fn accept_sent(
        &mut self,
        context: &Context,
        remote: [u8; 32],
        sent: &DocManifest,
        proof: &DocProof,
    ) -> Result<DocAcknowledgement, Error> {
        if sent != &proof.manifest {
            return Err("completion does not bind the active outgoing manifest".into());
        }
        self.validate_completion(context, remote, proof)?;
        if self.records.outgoing.iter().any(|r| r.manifest() == sent) {
            return self.accept_checked(proof, false);
        }
        // A parallel authenticated pump may have confirmed and retired the
        // exact manifest this Work sent before its Complete reply arrived.
        // Keep only the terminal ACK retry, never a new body or Ready plan.
        let acknowledgement = proof.acknowledgement()?;
        self.records.outgoing.push(Outgoing::NeedAck(proof.clone()));
        self.save()?;
        Ok(acknowledgement)
    }
    fn validate_completion(
        &self,
        context: &Context,
        remote: [u8; 32],
        proof: &DocProof,
    ) -> Result<(), Error> {
        proof.validate()?;
        authorize(&proof.manifest, context.local, remote)?;
        let descriptor = proof.manifest.descriptor()?;
        let local = context.version(&descriptor.document)?;
        if local.epoch != descriptor.epoch || !local.version.includes_vv(&descriptor.target) {
            return Err("outgoing completion belongs to an obsolete source version".into());
        }
        Ok(())
    }
    fn accept_checked(
        &mut self,
        proof: &DocProof,
        confirmation_request: bool,
    ) -> Result<DocAcknowledgement, Error> {
        let id = proof.manifest.id()?;
        let index = self
            .records
            .outgoing
            .iter()
            .position(|r| r.manifest() == &proof.manifest);
        if let Some(index) = index {
            self.records.outgoing[index] = Outgoing::Accepted(proof.clone());
            self.save()?;
            self.blobs
                .cancel(&proof.manifest.core()?)
                .map_err(Error::from_std)?;
            if confirmation_request {
                self.records.outgoing.remove(index);
            } else {
                self.records.outgoing[index] = Outgoing::NeedAck(proof.clone());
            }
            self.save()?;
        } else if !confirmation_request {
            // An unsolicited response cannot retire an arbitrary or newer plan.
            return Err(format!("completion does not name an outgoing plan: {id}").into());
        }
        // A repeated Confirm may follow deletion of the acknowledged outgoing
        // plan. It only reissues the exact receipt ACK; it cannot delete bytes
        // or a new active generation. The receiver still owns its proof.
        proof.acknowledgement()
    }
    pub(crate) fn acknowledge(
        &mut self,
        remote: [u8; 32],
        ack: &DocAcknowledgement,
    ) -> Result<DocAckResult, Error> {
        authorize(&ack.manifest, remote, self.local)?;
        let index = self
            .records
            .incoming
            .iter()
            .position(|r| r.manifest() == &ack.manifest);
        let Some(index) = index else {
            return Ok(DocAckResult::Absent {
                acknowledgement: ack.clone(),
            });
        };
        let proof = match self.records.incoming[index].clone() {
            Incoming::Complete { proof, .. } | Incoming::Cleanup(proof) => proof,
            _ => return Err("receipt acknowledgement precedes completion".into()),
        };
        if proof.acknowledgement()? != *ack {
            // Same exact manifest, but a replay after prior GC may have
            // produced an explicit Equivalent proof with a newer disk VV.
            // Keep the receipt/body until the sender confirms that proof.
            return Ok(DocAckResult::Reconfirm { proof });
        }
        self.records.incoming[index] = Incoming::Cleanup(proof);
        self.save()?;
        self.blobs
            .cancel(&ack.manifest.core()?)
            .map_err(Error::from_std)?;
        self.records.incoming.remove(index);
        self.save()?;
        Ok(DocAckResult::Released {
            acknowledgement: ack.clone(),
        })
    }
    pub(crate) fn acknowledged(
        &mut self,
        context: &Context,
        remote: [u8; 32],
        result: &DocAckResult,
    ) -> Result<(), Error> {
        if let DocAckResult::Reconfirm { proof } = result {
            proof.validate()?;
            authorize(&proof.manifest, self.local, remote)?;
            if self
                .records
                .outgoing
                .iter()
                .any(|r| r.manifest() == &proof.manifest)
            {
                self.accept(context, remote, proof, false)?;
            }
            return Ok(());
        }
        let ack = match result {
            DocAckResult::Released { acknowledgement }
            | DocAckResult::Absent { acknowledgement } => acknowledgement,
            DocAckResult::Reconfirm { .. } => unreachable!(),
        };
        authorize(&ack.manifest, self.local, remote)?;
        let index = self.records.outgoing.iter().position(
            |r| matches!(r, Outgoing::NeedAck(p) if p.acknowledgement().is_ok_and(|a|a==*ack)),
        );
        if let Some(index) = index {
            self.records.outgoing.remove(index);
            self.save()?;
        }
        Ok(())
    }
}

pub(crate) fn authorize(
    manifest: &DocManifest,
    source: [u8; 32],
    recipient: [u8; 32],
) -> Result<(), Error> {
    if manifest.source != source || manifest.recipient != recipient {
        return Err("document manifest identity or direction mismatch".into());
    }
    if !registry::authorized_name(&manifest.descriptor()?.document) {
        return Err("unknown document transfer name".into());
    }
    Ok(())
}
fn same_preparing(expected: &DocManifest, actual: &DocManifest) -> bool {
    expected.source == actual.source
        && expected.recipient == actual.recipient
        && expected.binding == actual.binding
}
fn validate_records(bytes: &[u8], local: [u8; 32]) -> Result<(), Error> {
    if bytes.len() > io::MAX_IMAGE {
        return Err("document metadata image exceeds budget".into());
    }
    let records: Records = rmp_serde::from_slice(bytes)
        .map_err(|e| format!("invalid document transfer metadata: {e}"))?;
    if rmp_serde::to_vec(&records).map_err(Error::from_std)? != bytes {
        return Err("noncanonical document transfer metadata".into());
    }
    if records.outgoing.len() > MAX_RECORDS || records.incoming.len() > MAX_RECORDS {
        return Err("document transfer metadata count exceeds budget".into());
    }
    records.pairing.validate(local)?;
    let mut ids = BTreeSet::new();
    let mut active = BTreeSet::new();
    for row in &records.outgoing {
        let manifest = row.manifest();
        authorize(manifest, local, manifest.recipient)?;
        if rmp_serde::to_vec(row).map_err(Error::from_std)?.len() > MAX_RECORD_BYTES {
            return Err("outgoing document record exceeds budget".into());
        }
        if !ids.insert(manifest.id()?) {
            return Err("duplicate outgoing document record".into());
        }
        if row.active() && !active.insert((manifest.recipient, manifest.descriptor()?.document)) {
            return Err("multiple active plans for peer/document".into());
        }
        if let Outgoing::Accepted(proof) | Outgoing::NeedAck(proof) = row {
            proof.validate()?;
        }
    }
    ids.clear();
    for row in &records.incoming {
        let manifest = row.manifest();
        authorize(manifest, manifest.source, local)?;
        if rmp_serde::to_vec(row).map_err(Error::from_std)?.len() > MAX_RECORD_BYTES {
            return Err("incoming document record exceeds budget".into());
        }
        if !ids.insert(manifest.id()?) {
            return Err("duplicate incoming document record".into());
        }
        match row {
            Incoming::Complete { proof, .. } | Incoming::Cleanup(proof) => proof.validate()?,
            Incoming::Importing { before, .. } => {
                DocumentVersion::decode(before).map_err(Error::from_std)?;
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod crash_tests;
#[cfg(test)]
mod tests;

/// Only the private flow fixture can hold the real metadata gate deliberately.
/// No product environment variable or alternate storage implementation exists.
#[cfg(test)]
pub(crate) struct TestMetadataGate {
    _gate: io::Gate,
}

#[cfg(test)]
pub(crate) fn test_metadata_gate(root: &std::path::Path) -> Result<TestMetadataGate, Error> {
    Ok(TestMetadataGate {
        _gate: io::gate(&root.join(".khor/doc-transfer-state"))?,
    })
}
