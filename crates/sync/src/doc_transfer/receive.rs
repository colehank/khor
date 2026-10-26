//! Bind a verified stage to an authorized document, then use its strict apply.
use super::{
    BodyError, Descriptor,
    body::{read_verified_body, validate_body},
};
use crate::{
    store::{ApplyError, ApplySpec, Doc, FlushedImport, Loaded},
    transfer::{self, Progress, Sink},
};
use std::fmt;

/// Authenticated identities and authorized document selected by Node dispatch.
#[derive(Clone, Debug)]
pub struct ReceiveSpec {
    pub document: String,
    pub source: [u8; 32],
    pub recipient: [u8; 32],
}

#[derive(Debug)]
pub enum ReceiveError {
    WrongDocument { expected: String, received: String },
    WrongSource,
    WrongRecipient,
    Incomplete { next_offset: u64 },
    Transfer(transfer::Error),
    Body(BodyError),
    Apply(ApplyError),
}
impl fmt::Display for ReceiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongDocument { expected, received } => write!(
                f,
                "document {received:?} differs from authorized {expected:?}"
            ),
            Self::WrongSource => {
                f.write_str("document stage source differs from authenticated source")
            }
            Self::WrongRecipient => {
                f.write_str("document stage recipient differs from local recipient")
            }
            Self::Incomplete { next_offset } => {
                write!(f, "document stage is incomplete at {next_offset}")
            }
            Self::Transfer(why) => fmt::Display::fmt(why, f),
            Self::Body(why) => fmt::Display::fmt(why, f),
            Self::Apply(why) => fmt::Display::fmt(why, f),
        }
    }
}
impl std::error::Error for ReceiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transfer(why) => Some(why),
            Self::Body(why) => Some(why),
            Self::Apply(why) => Some(why),
            _ => None,
        }
    }
}
impl From<transfer::Error> for ReceiveError {
    fn from(why: transfer::Error) -> Self {
        Self::Transfer(why)
    }
}
impl From<BodyError> for ReceiveError {
    fn from(why: BodyError) -> Self {
        Self::Body(why)
    }
}

/// Does not release the stage or issue a network Applied receipt. The body is
/// a local full Vec up to 64 MiB; only each read is limited to 256 KiB. On any
/// error the caller still owns the Sink and its staged bytes for retry/cleanup.
/// Doc mutation/fence/flush semantics are those of `Loaded::apply_checked`.
pub fn apply_staged<D: Doc>(
    loaded: &mut Loaded<D>,
    spec: &ReceiveSpec,
    sink: &mut Sink,
) -> Result<FlushedImport, ReceiveError> {
    let manifest = sink.manifest();
    let descriptor = Descriptor::from_manifest(manifest)?;
    if descriptor.document != spec.document {
        return Err(ReceiveError::WrongDocument {
            expected: spec.document.clone(),
            received: descriptor.document,
        });
    }
    if manifest.source != spec.source {
        return Err(ReceiveError::WrongSource);
    }
    if manifest.recipient != spec.recipient {
        return Err(ReceiveError::WrongRecipient);
    }
    if let Progress::Stored { next_offset } = sink.progress()? {
        return Err(ReceiveError::Incomplete { next_offset });
    }
    let manifest = sink.manifest().clone();
    let body = read_verified_body(sink)?;
    validate_body(&descriptor, &manifest, &body)?;
    let result = loaded
        .apply_checked(
            &ApplySpec {
                expected_epoch: descriptor.epoch,
                base_vv: descriptor.base,
                target_vv: descriptor.target,
            },
            &body,
        )
        .map_err(ReceiveError::Apply);
    drop(body);
    result
}
