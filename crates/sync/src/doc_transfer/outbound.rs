//! Freeze one flushed document export into the shared byte-transfer store.

use std::{fmt, io};

use loro::{EncodedBlobMode, LoroDoc, VersionVector};

use super::{Descriptor, Encoding};
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
    InvalidBody(String),
    EncodingMismatch {
        expected: Encoding,
        actual: EncodedBlobMode,
    },
    /// The exact manifest permits explicit resume/cancel after a staging error.
    Staging {
        manifest: Manifest,
        cause: transfer::Error,
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
            Self::InvalidBody(why) => write!(f, "invalid outbound Loro body: {why}"),
            Self::EncodingMismatch { expected, actual } => {
                write!(f, "outbound encoding {actual} differs from {expected:?}")
            }
            Self::Transfer(why) => fmt::Display::fmt(why, f),
            Self::Staging { manifest, cause } => {
                write!(f, "outbound staging {:?}: {cause}", manifest.id())
            }
        }
    }
}

impl std::error::Error for OutboundError {}
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

/// Byte/format validation only. Complete target coverage and pending imports
/// must still be checked by the receiver's Doc apply boundary.
pub(crate) fn validate_body(
    descriptor: &Descriptor,
    manifest: &Manifest,
    body: &[u8],
) -> Result<(), OutboundError> {
    if body.len() as u64 > MAX_BLOB_BYTES {
        return Err(transfer::Error::ResourceLimit("document body exceeds 64 MiB").into());
    }
    if descriptor.bind(
        manifest.source,
        manifest.recipient,
        manifest.total_bytes,
        manifest.digest,
    )? != *manifest
    {
        return Err(
            transfer::Error::InvalidManifest("document descriptor differs from manifest").into(),
        );
    }
    if body.len() as u64 != manifest.total_bytes {
        return Err(
            transfer::Error::InvalidStage("document body length differs from manifest").into(),
        );
    }
    if blake3::hash(body).as_bytes() != &manifest.digest {
        return Err(transfer::Error::DigestMismatch.into());
    }
    let meta = LoroDoc::decode_import_blob_meta(body, true)
        .map_err(|why| OutboundError::InvalidBody(why.to_string()))?;
    if !matches!(
        (descriptor.encoding, meta.mode),
        (Encoding::Updates, EncodedBlobMode::Updates)
            | (Encoding::Snapshot, EncodedBlobMode::Snapshot)
    ) {
        return Err(OutboundError::EncodingMismatch {
            expected: descriptor.encoding,
            actual: meta.mode,
        });
    }
    // Metadata VVs cover only exported writers; they are not a complete target.
    Ok(())
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
