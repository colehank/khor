//! Canonical document binding for the current byte-transfer protocol.
//!
//! The byte manifest binds identities, length and digest. This binding adds
//! the document, epoch, encoding and fixed base/target versions. A valid
//! descriptor is metadata, not proof that its body was imported or flushed.

use loro::VersionVector;

use crate::transfer::{Error, MAX_METADATA_BYTES, Manifest, Result};

mod body;
mod outbound;
mod receive;
mod version;
pub use body::BodyError;
pub use outbound::{Outbound, OutboundError, PrepareSpec, Prepared, prepare, resume};
pub use receive::{ReceiveError, ReceiveSpec, apply_staged};
pub use version::DocumentVersion;

const MAGIC: &[u8] = b"khor-doc-v1\0";
const VERSION_ENTRY_BYTES: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Encoding {
    Updates,
    Snapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Descriptor {
    pub document: String,
    pub epoch: u32,
    pub encoding: Encoding,
    pub base: VersionVector,
    pub target: VersionVector,
}

impl Descriptor {
    /// Document authorization belongs to the node dispatch, before staging.
    /// Base and target may be concurrent; neither must include the other.
    pub fn bind(
        &self,
        source: [u8; 32],
        recipient: [u8; 32],
        total_bytes: u64,
        digest: [u8; 32],
    ) -> Result<Manifest> {
        if self.document.is_empty() {
            return Err(Error::InvalidManifest("empty document name"));
        }
        if self.document.len() > MAX_METADATA_BYTES {
            return Err(Error::ResourceLimit(
                "document binding exceeds metadata budget",
            ));
        }
        let mut binding = Vec::new();
        binding.extend_from_slice(MAGIC);
        binding.extend_from_slice(&(self.document.len() as u32).to_be_bytes());
        binding.extend_from_slice(self.document.as_bytes());
        binding.extend_from_slice(&self.epoch.to_be_bytes());
        binding.push(match self.encoding {
            Encoding::Updates => 0,
            Encoding::Snapshot => 1,
        });
        write_version(&mut binding, &self.base)?;
        write_version(&mut binding, &self.target)?;
        let manifest = Manifest {
            source,
            recipient,
            binding,
            total_bytes,
            digest,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Refuse alternate spellings so one descriptor has exactly one binding.
    pub fn from_manifest(manifest: &Manifest) -> Result<Self> {
        manifest.validate()?;
        let mut input = Reader(&manifest.binding);
        if input.take(MAGIC.len())? != MAGIC {
            return Err(Error::InvalidManifest(
                "unsupported document binding revision",
            ));
        }
        let name_len = input.u32()? as usize;
        let document = std::str::from_utf8(input.take(name_len)?)
            .map_err(|_| Error::InvalidManifest("document name is not UTF-8"))?;
        if document.is_empty() {
            return Err(Error::InvalidManifest("empty document name"));
        }
        let descriptor = Self {
            document: document.to_owned(),
            epoch: input.u32()?,
            encoding: match input.take(1)?[0] {
                0 => Encoding::Updates,
                1 => Encoding::Snapshot,
                _ => return Err(Error::InvalidManifest("unsupported document encoding")),
            },
            base: input.version()?,
            target: input.version()?,
        };
        if !input.0.is_empty() {
            return Err(Error::InvalidManifest("trailing document binding bytes"));
        }
        Ok(descriptor)
    }
}

fn write_version(out: &mut Vec<u8>, version: &VersionVector) -> Result<()> {
    if version.iter().any(|(_, counter)| *counter < 0) {
        return Err(Error::InvalidManifest("negative document version counter"));
    }
    let entries: Vec<_> = version
        .iter()
        .filter(|(_, counter)| **counter > 0)
        .map(|(writer, counter)| (*writer, *counter))
        .take(MAX_METADATA_BYTES / VERSION_ENTRY_BYTES + 1)
        .collect();
    if out.len() + 4 + entries.len() * VERSION_ENTRY_BYTES > MAX_METADATA_BYTES {
        return Err(Error::ResourceLimit(
            "document versions exceed metadata budget",
        ));
    }
    let mut entries = entries;
    entries.sort_unstable_by_key(|(writer, _)| *writer);
    out.extend_from_slice(&(entries.len() as u32).to_be_bytes());
    for (writer, counter) in entries {
        out.extend_from_slice(&writer.to_be_bytes());
        out.extend_from_slice(&(counter as u32).to_be_bytes());
    }
    Ok(())
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let bytes = self
            .0
            .get(..count)
            .ok_or(Error::InvalidManifest("truncated document binding"))?;
        self.0 = &self.0[count..];
        Ok(bytes)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn version(&mut self) -> Result<VersionVector> {
        let count = self.u32()? as usize;
        if count > self.0.len() / VERSION_ENTRY_BYTES {
            return Err(Error::InvalidManifest("truncated document version"));
        }
        let mut version = VersionVector::default();
        let mut previous = None;
        for _ in 0..count {
            let writer = u64::from_be_bytes(self.take(8)?.try_into().unwrap());
            let counter = self.u32()?;
            if previous.is_some_and(|previous| writer <= previous)
                || counter == 0
                || counter > i32::MAX as u32
            {
                return Err(Error::InvalidManifest("noncanonical document version"));
            }
            version.insert(writer, counter as i32);
            previous = Some(writer);
        }
        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> Descriptor {
        Descriptor {
            document: "pins".into(),
            epoch: 1,
            encoding: Encoding::Updates,
            base: [(1, 2)].into_iter().collect(),
            target: [(3, 4)].into_iter().collect(),
        }
    }

    fn bind(descriptor: &Descriptor) -> Manifest {
        descriptor.bind([11; 32], [12; 32], 17, [13; 32]).unwrap()
    }

    #[test]
    fn concurrent_versions_have_a_fixed_binary_contract() {
        let descriptor = descriptor();
        assert!(!descriptor.target.includes_vv(&descriptor.base));
        assert!(!descriptor.base.includes_vv(&descriptor.target));
        let manifest = bind(&descriptor);
        let expected = b"khor-doc-v1\0\0\0\0\x04pins\0\0\0\x01\0\0\0\0\x01\0\0\0\0\0\0\0\x01\0\0\0\x02\0\0\0\x01\0\0\0\0\0\0\0\x03\0\0\0\x04";
        assert_eq!(manifest.binding, expected);
        assert_eq!(Descriptor::from_manifest(&manifest).unwrap(), descriptor);
    }

    #[test]
    fn insertion_order_and_empty_writers_do_not_change_identity() {
        let mut first = descriptor();
        first.base = [(7, 9), (1, 2)].into_iter().collect();
        let mut second = first.clone();
        second.base = [(1, 2), (6, 0), (7, 9)].into_iter().collect();
        assert_eq!(bind(&first), bind(&second));
        assert_eq!(bind(&first).id().unwrap(), bind(&second).id().unwrap());
    }

    #[test]
    fn every_document_and_transfer_field_participates_in_identity() {
        let original = descriptor();
        let manifest = bind(&original);
        let mut alternatives = Vec::new();
        let mut changed = original.clone();
        changed.document = "seen".into();
        alternatives.push(bind(&changed));
        changed = original.clone();
        changed.epoch += 1;
        alternatives.push(bind(&changed));
        changed = original.clone();
        changed.encoding = Encoding::Snapshot;
        alternatives.push(bind(&changed));
        changed = original.clone();
        changed.base.insert(1, 3);
        alternatives.push(bind(&changed));
        changed = original.clone();
        changed.target.insert(3, 5);
        alternatives.push(bind(&changed));
        for field in 0..4 {
            let mut changed = manifest.clone();
            match field {
                0 => changed.source[0] ^= 1,
                1 => changed.recipient[0] ^= 1,
                2 => changed.total_bytes += 1,
                _ => changed.digest[0] ^= 1,
            }
            alternatives.push(changed);
        }
        for alternative in alternatives {
            assert_ne!(manifest.id().unwrap(), alternative.id().unwrap());
        }
    }

    #[test]
    fn malformed_and_noncanonical_bindings_are_refused() {
        let valid = bind(&descriptor());
        for length in 0..valid.binding.len() {
            let mut truncated = valid.clone();
            truncated.binding.truncate(length);
            assert!(Descriptor::from_manifest(&truncated).is_err());
        }
        let mut extra = valid.clone();
        extra.binding.push(0);
        assert!(Descriptor::from_manifest(&extra).is_err());
        // Header, document length/name, epoch and encoding precede base VV.
        let base = MAGIC.len() + 4 + 4 + 4 + 1;
        for (at, bytes) in [
            (0, vec![b'X']),
            (MAGIC.len() + 4, vec![0xff]),
            (base - 1, vec![2]),
            (base, u32::MAX.to_be_bytes().to_vec()),
            (base + 4 + 8, 0u32.to_be_bytes().to_vec()),
            (base + 4 + 8, u32::MAX.to_be_bytes().to_vec()),
        ] {
            let mut bad = valid.clone();
            bad.binding[at..at + bytes.len()].copy_from_slice(&bytes);
            assert!(Descriptor::from_manifest(&bad).is_err());
        }
        let mut two = descriptor();
        two.base.insert(7, 8);
        let mut out_of_order = bind(&two);
        let first = out_of_order.binding[base + 4..base + 4 + 12].to_vec();
        let second = out_of_order.binding[base + 4 + 12..base + 4 + 24].to_vec();
        out_of_order.binding[base + 4..base + 4 + 12].copy_from_slice(&second);
        out_of_order.binding[base + 4 + 12..base + 4 + 24].copy_from_slice(&first);
        assert!(Descriptor::from_manifest(&out_of_order).is_err());
        out_of_order.binding[base + 4..base + 4 + 12].copy_from_slice(&first);
        assert!(
            Descriptor::from_manifest(&out_of_order).is_err(),
            "duplicate writer"
        );
    }

    #[test]
    fn invalid_versions_and_metadata_or_body_overflow_are_refused() {
        let mut invalid = descriptor();
        invalid.base.insert(9, -1);
        assert!(invalid.bind([0; 32], [1; 32], 0, [0; 32]).is_err());
        invalid = descriptor();
        invalid.document.clear();
        assert!(invalid.bind([0; 32], [1; 32], 0, [0; 32]).is_err());
        invalid.document = "x".repeat(MAX_METADATA_BYTES);
        assert!(invalid.bind([0; 32], [1; 32], 0, [0; 32]).is_err());
        invalid = descriptor();
        invalid.target = (0..6000).map(|writer| (writer, 1)).collect();
        assert!(invalid.bind([0; 32], [1; 32], 0, [0; 32]).is_err());
        assert!(
            descriptor()
                .bind(
                    [0; 32],
                    [1; 32],
                    crate::transfer::MAX_BLOB_BYTES + 1,
                    [0; 32]
                )
                .is_err()
        );
    }
}
