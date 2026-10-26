//! Shared checks for a complete current-format document body, not an apply proof.
use super::{Descriptor, Encoding};
use crate::transfer::{self, MAX_BLOB_BYTES, Manifest};
use loro::{EncodedBlobMode, LoroDoc};
use std::fmt;

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
