//! Canonical version metadata. This value is not proof of a persistent import.
use super::{Reader, write_version};
use crate::transfer::{Error, MAX_METADATA_BYTES, Result};
use loro::VersionVector;

const MAGIC: &[u8] = b"khor-version-v1\0";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentVersion {
    pub epoch: u32,
    pub version: VersionVector,
}

impl DocumentVersion {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&self.epoch.to_be_bytes());
        write_version(&mut bytes, &self.version)?;
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_METADATA_BYTES {
            return Err(Error::ResourceLimit(
                "document version exceeds metadata budget",
            ));
        }
        let mut input = Reader(bytes);
        if input.take(MAGIC.len())? != MAGIC {
            return Err(Error::InvalidManifest(
                "unsupported document version revision",
            ));
        }
        let value = Self {
            epoch: input.u32()?,
            version: input.version()?,
        };
        if !input.0.is_empty() {
            return Err(Error::InvalidManifest("trailing document version bytes"));
        }
        Ok(value)
    }
}
