//! Shared checks for a complete current-format document body, not an apply proof.
use super::{Descriptor, Encoding};
use crate::transfer::{self, CHUNK_BYTES, MAX_BLOB_BYTES, Manifest, Progress, Sink};
use loro::{EncodedBlobMode, LoroDoc};
use std::{fmt, io};

#[derive(Debug)]
pub enum BodyError {
    Transfer(transfer::Error),
    InvalidLoro(String),
    EncodingMismatch {
        expected: Encoding,
        actual: EncodedBlobMode,
    },
}
impl fmt::Display for BodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transfer(why) => fmt::Display::fmt(why, f),
            Self::InvalidLoro(why) => write!(f, "invalid document Loro body: {why}"),
            Self::EncodingMismatch { expected, actual } => {
                write!(f, "document encoding {actual} differs from {expected:?}")
            }
        }
    }
}
impl std::error::Error for BodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transfer(why) => Some(why),
            _ => None,
        }
    }
}
impl From<transfer::Error> for BodyError {
    fn from(why: transfer::Error) -> Self {
        Self::Transfer(why)
    }
}

/// Byte/format validation only. Complete target coverage and pending imports
/// must still be checked by the receiver's Doc apply boundary.
pub(super) fn validate_body(
    descriptor: &Descriptor,
    manifest: &Manifest,
    body: &[u8],
) -> Result<(), BodyError> {
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
        .map_err(|why| BodyError::InvalidLoro(why.to_string()))?;
    if !matches!(
        (descriptor.encoding, meta.mode),
        (Encoding::Updates, EncodedBlobMode::Updates)
            | (Encoding::Snapshot, EncodedBlobMode::Snapshot)
    ) {
        return Err(BodyError::EncodingMismatch {
            expected: descriptor.encoding,
            actual: meta.mode,
        });
    }
    // Metadata VVs cover only exported writers; they are not a complete target.
    Ok(())
}

/// Read only a verified complete body. The full Vec is local; each file read
/// is bounded to 256 KiB and short reads must fill it or fail with EOF.
pub(super) fn read_verified_body(sink: &mut Sink) -> transfer::Result<Vec<u8>> {
    if !matches!(sink.progress()?, Progress::VerifiedComplete { .. }) {
        return Err(transfer::Error::InvalidStage(
            "document stage is incomplete",
        ));
    }
    let total = sink.manifest().total_bytes;
    if total > MAX_BLOB_BYTES {
        return Err(transfer::Error::ResourceLimit(
            "document body exceeds 64 MiB",
        ));
    }
    let mut body = vec![0; total as usize];
    let mut at = 0;
    while at < body.len() {
        let end = (at + CHUNK_BYTES).min(body.len());
        let count = sink.read_verified(at as u64, &mut body[at..end])?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "verified document body was truncated",
            )
            .into());
        }
        at += count;
    }
    Ok(body)
}
