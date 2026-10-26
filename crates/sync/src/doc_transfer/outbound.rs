//! Freeze one flushed document export into the shared byte-transfer store.

use std::{fmt, io};

use loro::VersionVector;

use super::{
    BodyError, Descriptor, Encoding,
    body::{read_verified_body, validate_body},
};
use crate::{
    store::{Doc, Loaded},
    transfer::{
        self, CHUNK_BYTES, Cancelled, MAX_BLOB_BYTES, Manifest, Progress, Sink, TransferStore,
    },
};

/// Node dispatch authenticates these identities and authorizes the document.
#[derive(Clone, Debug)]
pub struct PrepareSpec {
    pub document: String,
    pub source: [u8; 32],
    pub recipient: [u8; 32],
    pub receiver_epoch: u32,
    pub base: VersionVector,
    pub encoding: Encoding,
}

pub enum Prepared {
    UpToDate { epoch: u32, version: VersionVector },
    Transfer(Outbound),
}

/// Flush failures can leave local disk changes. Neither outcome is a network
/// Applied receipt, and this adapter does not promise power-loss durability.
#[derive(Debug)]
pub enum OutboundError {
    WrongEpoch {
        receiver: u32,
        loaded: u32,
    },
    Flush(String),
    Export(String),
    VersionChanged,
    Body(BodyError),
    /// The exact manifest permits explicit resume/cancel after a staging error.
    Staging {
        manifest: Manifest,
        cause: transfer::Error,
    },
    /// Retained bytes remain available for explicit cleanup after validation.
    StagingBody {
        manifest: Manifest,
        cause: BodyError,
    },
    Transfer(transfer::Error),
}

impl fmt::Display for OutboundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongEpoch { receiver, loaded } => {
                write!(
                    f,
                    "outbound epoch {loaded} differs from receiver {receiver}"
                )
            }
            Self::Flush(why) => write!(f, "outbound flush: {why}"),
            Self::Export(why) => write!(f, "outbound export: {why}"),
            Self::VersionChanged => f.write_str("document changed during outbound preparation"),
            Self::Body(why) => fmt::Display::fmt(why, f),
            Self::Transfer(why) => fmt::Display::fmt(why, f),
            Self::Staging { manifest, cause } => {
                write!(f, "outbound staging {:?}: {cause}", manifest.id())
            }
            Self::StagingBody { manifest, cause } => {
                write!(f, "outbound staged body {:?}: {cause}", manifest.id())
            }
        }
    }
}

impl std::error::Error for OutboundError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Body(why) | Self::StagingBody { cause: why, .. } => Some(why),
            Self::Transfer(why) | Self::Staging { cause: why, .. } => Some(why),
            _ => None,
        }
    }
}
impl From<BodyError> for OutboundError {
    fn from(why: BodyError) -> Self {
        Self::Body(why)
    }
}
impl From<transfer::Error> for OutboundError {
    fn from(why: transfer::Error) -> Self {
        Self::Transfer(why)
    }
}

/// Owns a verified immutable body and one core owner, with no Doc/pile lease.
/// Drop releases the owner but retains the stage and its quota reservation.
pub struct Outbound {
    descriptor: Descriptor,
    sink: Sink,
    store: TransferStore,
}

impl Outbound {
    pub fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }

    pub fn manifest(&self) -> &Manifest {
        self.sink.manifest()
    }

    /// Read exactly one canonical chunk (or empty at EOF), at most 256 KiB.
    /// The file API may return short reads, so fill the chunk before returning.
    pub fn read_chunk(&mut self, at: u64) -> transfer::Result<Vec<u8>> {
        let total = self.manifest().total_bytes;
        if at > total || (at != total && at % CHUNK_BYTES as u64 != 0) {
            return Err(transfer::Error::InvalidChunk(
                "noncanonical outbound read offset",
            ));
        }
        let count = (total - at).min(CHUNK_BYTES as u64) as usize;
        let mut bytes = vec![0; count];
        let mut filled = 0;
        while filled < count {
            let read = self
                .sink
                .read_verified(at + filled as u64, &mut bytes[filled..])?;
            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "verified outbound body was truncated",
                )
                .into());
            }
            filled += read;
        }
        Ok(bytes)
    }

    /// Explicitly discard this idle stage using the same shared store. Core
    /// cancellation still refuses another active owner; no Doc is rolled back.
    pub fn discard(self) -> transfer::Result<Cancelled> {
        let manifest = self.manifest().clone();
        let store = self.store.clone();
        drop(self);
        store.cancel(&manifest)
    }
}

/// Reopen an exact retained spool without loading/exporting a Doc or choosing
/// a new target. Missing returns None. Incomplete/invalid retained stages are
/// reported with their manifest and kept for explicit resume/cleanup policy.
/// Revalidation still uses a local full Vec, not a bounded Loro/RSS claim.
pub fn resume(
    store: &TransferStore,
    manifest: &Manifest,
) -> Result<Option<Outbound>, OutboundError> {
    let descriptor = Descriptor::from_manifest(manifest)?;
    let Some(mut sink) = store
        .open_existing(manifest)
        .map_err(|cause| OutboundError::Staging {
            manifest: manifest.clone(),
            cause,
        })?
    else {
        return Ok(None);
    };
    let body = read_verified_body(&mut sink).map_err(|cause| OutboundError::Staging {
        manifest: manifest.clone(),
        cause,
    })?;
    validate_body(&descriptor, manifest, &body).map_err(|cause| OutboundError::StagingBody {
        manifest: manifest.clone(),
        cause,
    })?;
    drop(body);
    Ok(Some(Outbound {
        descriptor,
        sink,
        store: store.clone(),
    }))
}

/// Capture, flush, export, validate and stage synchronously. Callers must not
/// mutate the Doc through another alias during this call. Later local writes
/// cannot change the returned descriptor/body. Loro still exports a full Vec;
/// the size check bounds accepted bytes, not its peak export allocation.
pub fn prepare<D: Doc>(
    loaded: &mut Loaded<D>,
    spec: &PrepareSpec,
    store: &TransferStore,
) -> Result<Prepared, OutboundError> {
    let epoch = loaded.store.epoch();
    if spec.receiver_epoch != epoch {
        return Err(OutboundError::WrongEpoch {
            receiver: spec.receiver_epoch,
            loaded: epoch,
        });
    }
    let descriptor = Descriptor {
        document: spec.document.clone(),
        epoch,
        encoding: spec.encoding,
        base: spec.base.clone(),
        target: loaded.doc.version(),
    };
    // Validate versions and the complete metadata budget before any flush.
    descriptor.bind(spec.source, spec.recipient, 0, [0; 32])?;
    loaded
        .store
        .flush(&loaded.doc)
        .map_err(OutboundError::Flush)?;
    if loaded.doc.version() != descriptor.target {
        return Err(OutboundError::VersionChanged);
    }
    if spec.base.includes_vv(&descriptor.target) {
        return Ok(Prepared::UpToDate {
            epoch,
            version: descriptor.target,
        });
    }
    let body = match spec.encoding {
        Encoding::Updates => loaded.doc.changes_since(&spec.base),
        Encoding::Snapshot => loaded.doc.snapshot(),
    }
    .map_err(OutboundError::Export)?;
    if loaded.doc.version() != descriptor.target {
        return Err(OutboundError::VersionChanged);
    }
    if body.len() as u64 > MAX_BLOB_BYTES {
        return Err(transfer::Error::ResourceLimit("outbound body exceeds 64 MiB").into());
    }
    let manifest = descriptor.bind(
        spec.source,
        spec.recipient,
        body.len() as u64,
        *blake3::hash(&body).as_bytes(),
    )?;
    validate_body(&descriptor, &manifest, &body)?;
    let sink = stage(store, &manifest, &body)
        .map_err(|cause| OutboundError::Staging { manifest, cause })?;
    drop(body);
    Ok(Prepared::Transfer(Outbound {
        descriptor,
        sink,
        store: store.clone(),
    }))
}

fn stage(store: &TransferStore, manifest: &Manifest, body: &[u8]) -> transfer::Result<Sink> {
    let mut sink = store.receive(manifest.clone())?;
    let mut at = sink.next_offset() as usize;
    while at < body.len() {
        let end = (at + CHUNK_BYTES).min(body.len());
        sink.append(at as u64, &body[at..end])?;
        at = end;
    }
    if !matches!(sink.progress()?, Progress::VerifiedComplete { .. }) {
        return Err(transfer::Error::InvalidStage(
            "outbound staging is incomplete",
        ));
    }
    Ok(sink)
}
