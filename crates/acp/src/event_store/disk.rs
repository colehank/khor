//! Private segmented FIFO, owned only by the one physical I/O worker.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SEGMENT_BYTES: u64 = 8 * 1024 * 1024;
pub(super) const IO_CHUNK: usize = 64 * 1024;
pub(super) const HEADER_BYTES: usize = 32;
const MAGIC: &[u8; 8] = b"KHORAC01";
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

pub(super) fn check_cancel(cancelled: &AtomicBool) -> io::Result<()> {
    if cancelled.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "ACP event store cancelled",
        ))
    } else {
        Ok(())
    }
}
pub(super) fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// One fresh private directory. It is not rooted in the agent's workspace/home.
struct OwnedDirectory {
    path: PathBuf,
    #[cfg(unix)]
    identity: (u64, u64),
    #[cfg(not(unix))]
    created: SystemTime,
    armed: bool,
}
impl OwnedDirectory {
    fn create() -> io::Result<Self> {
        let base = std::env::temp_dir().canonicalize()?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        for _ in 0..32 {
            let serial = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!(
                "khor-acp-events-{}-{stamp}-{serial}",
                std::process::id()
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => {
                    let metadata = fs::symlink_metadata(&path)?;
                    #[cfg(unix)]
                    let identity = {
                        use std::os::unix::fs::MetadataExt;
                        (metadata.dev(), metadata.ino())
                    };
                    return Ok(Self {
                        path,
                        #[cfg(unix)]
                        identity,
                        #[cfg(not(unix))]
                        created: metadata.created()?,
                        armed: true,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "private event store name collision",
        ))
    }
    fn remove(&mut self) -> io::Result<()> {
        if !self.armed {
            return Ok(());
        }
        let metadata = fs::symlink_metadata(&self.path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(invalid("event store directory identity changed"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if (metadata.dev(), metadata.ino()) != self.identity {
                return Err(invalid("event store directory identity changed"));
            }
        }
        #[cfg(not(unix))]
        if metadata.created()? != self.created {
            return Err(invalid("event store directory identity changed"));
        }
        fs::remove_dir_all(&self.path)?;
        self.armed = false;
        Ok(())
    }
}
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecordKind {
    Note = 0,
    Ready = 1,
    ElicitDone = 2,
    Interaction = 3,
}
impl TryFrom<u8> for RecordKind {
    type Error = io::Error;
    fn try_from(value: u8) -> io::Result<Self> {
        match value {
            0 => Ok(Self::Note),
            1 => Ok(Self::Ready),
            2 => Ok(Self::ElicitDone),
            3 => Ok(Self::Interaction),
            _ => Err(invalid("unknown event store record kind")),
        }
    }
}
pub(super) struct Record {
    pub(super) sequence: u64,
    pub(super) kind: RecordKind,
    pub(super) payload: Vec<u8>,
}

/// Only the one store I/O worker owns these files and cursors. No per-history index.
/// At most one writer and one reader file are open, regardless of segment count.
pub(super) struct Spool {
    writer: Option<File>,
    reader: Option<File>,
    tail: u64,
    head: u64,
    tail_committed: u64,
    read_offset: u64,
    accepted: u64,
    read_sequence: u64,
    poisoned: bool,
    // Last so normal field destruction closes both files before directory cleanup.
    directory: OwnedDirectory,
}
impl Spool {
    pub(super) fn create(cancelled: &AtomicBool) -> io::Result<Self> {
        check_cancel(cancelled)?;
        let directory = OwnedDirectory::create()?;
        let writer = Self::new_segment(&directory.path, 0)?;
        Ok(Self {
            writer: Some(writer),
            reader: None,
            tail: 0,
            head: 0,
            tail_committed: 0,
            read_offset: 0,
            accepted: 0,
            read_sequence: 0,
            poisoned: false,
            directory,
        })
    }
    fn segment(directory: &Path, number: u64) -> PathBuf {
        directory.join(format!("{number:016x}.part"))
    }
    fn new_segment(directory: &Path, number: u64) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(Self::segment(directory, number))
    }
    pub(super) fn append(&mut self, record: &Record, cancelled: &AtomicBool) -> io::Result<()> {
        if self.poisoned {
            return Err(invalid("event store writer is poisoned"));
        }
        let result = self.append_inner(record, cancelled);
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    fn append_inner(&mut self, record: &Record, cancelled: &AtomicBool) -> io::Result<()> {
        check_cancel(cancelled)?;
        if self.accepted.checked_add(1) != Some(record.sequence) {
            return Err(invalid("event store sequence is not contiguous"));
        }
        let payload_bytes = u64::try_from(record.payload.len())
            .map_err(|_| invalid("event record size overflow"))?;
        let record_bytes = payload_bytes
            .checked_add(HEADER_BYTES as u64)
            .ok_or_else(|| invalid("event record size overflow"))?;
        let next_end = self
            .tail_committed
            .checked_add(record_bytes)
            .ok_or_else(|| invalid("event segment size overflow"))?;
        if self.tail_committed != 0 && next_end > SEGMENT_BYTES {
            let next = self
                .tail
                .checked_add(1)
                .ok_or_else(|| invalid("event segment index overflow"))?;
            // Close the old tail before opening another: even rotation keeps
            // the one-writer/one-reader descriptor bound.
            drop(self.writer.take());
            let writer = Self::new_segment(&self.directory.path, next)?;
            self.writer = Some(writer);
            self.tail = next;
            self.tail_committed = 0;
        }
        let mut header = [0u8; HEADER_BYTES];
        header[..8].copy_from_slice(MAGIC);
        header[8..16].copy_from_slice(&record.sequence.to_be_bytes());
        header[16..24].copy_from_slice(&payload_bytes.to_be_bytes());
        header[24] = record.kind as u8;
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| invalid("event writer closed"))?;
        writer.write_all(&header)?;
        for bytes in record.payload.chunks(IO_CHUNK) {
            check_cancel(cancelled)?;
            writer.write_all(bytes)?;
        }
        check_cancel(cancelled)?;
        // Private prefix becomes readable only after every byte is written.
        self.tail_committed += record_bytes;
        self.accepted = record.sequence;
        Ok(())
    }
    pub(super) fn next(&mut self, cancelled: &AtomicBool) -> io::Result<Option<Record>> {
        check_cancel(cancelled)?;
        if self.poisoned {
            return Err(invalid("event store writer is poisoned"));
        }
        if self.read_sequence == self.accepted {
            return Ok(None);
        }
        loop {
            check_cancel(cancelled)?;
            if self.reader.is_none() {
                self.reader = Some(File::open(Self::segment(&self.directory.path, self.head))?);
            }
            let reader = self.reader.as_mut().unwrap();
            let end = if self.head == self.tail {
                self.tail_committed
            } else {
                reader.metadata()?.len()
            };
            if self.read_offset == end && self.head < self.tail {
                drop(self.reader.take());
                fs::remove_file(Self::segment(&self.directory.path, self.head))?;
                self.head += 1;
                self.read_offset = 0;
                continue;
            }
            if end.saturating_sub(self.read_offset) < HEADER_BYTES as u64 {
                return Err(invalid("incomplete event store header"));
            }
            reader.seek(SeekFrom::Start(self.read_offset))?;
            let mut header = [0u8; HEADER_BYTES];
            reader.read_exact(&mut header)?;
            if &header[..8] != MAGIC || header[25..].iter().any(|byte| *byte != 0) {
                return Err(invalid("invalid event store header"));
            }
            let sequence = u64::from_be_bytes(header[8..16].try_into().unwrap());
            let size = u64::from_be_bytes(header[16..24].try_into().unwrap());
            if self.read_sequence.checked_add(1) != Some(sequence)
                || size > end - self.read_offset - HEADER_BYTES as u64
            {
                return Err(invalid("incomplete or out-of-order event store record"));
            }
            let size_usize =
                usize::try_from(size).map_err(|_| invalid("event record size overflow"))?;
            let mut payload = Vec::new();
            payload
                .try_reserve_exact(size_usize)
                .map_err(io::Error::other)?;
            let mut chunk = [0u8; IO_CHUNK];
            while payload.len() < size_usize {
                check_cancel(cancelled)?;
                let bytes = (size_usize - payload.len()).min(IO_CHUNK);
                reader.read_exact(&mut chunk[..bytes])?;
                payload.extend_from_slice(&chunk[..bytes]);
            }
            check_cancel(cancelled)?;
            let kind = RecordKind::try_from(header[24])?;
            self.read_offset += HEADER_BYTES as u64 + size;
            self.read_sequence = sequence;
            return Ok(Some(Record {
                sequence,
                kind,
                payload,
            }));
        }
    }
    pub(super) fn cleanup(mut self) -> io::Result<()> {
        drop(self.reader.take());
        drop(self.writer.take());
        self.directory.remove()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_rotation_failure_poison_cannot_publish_or_retry_the_next_record() {
        let cancelled = AtomicBool::new(false);
        let mut spool = Spool::create(&cancelled).unwrap();
        let root = spool.directory.path.clone();
        spool
            .append(
                &Record {
                    sequence: 1,
                    kind: RecordKind::Note,
                    payload: vec![b'a'; SEGMENT_BYTES as usize - HEADER_BYTES],
                },
                &cancelled,
            )
            .unwrap();
        // A real create_new collision is independent of user/root permissions.
        fs::write(Spool::segment(&root, 1), b"owned fault fixture").unwrap();
        let second = Record {
            sequence: 2,
            kind: RecordKind::Ready,
            payload: Vec::new(),
        };
        assert_eq!(
            spool.append(&second, &cancelled).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(spool.accepted, 1);
        assert_eq!(spool.tail_committed, SEGMENT_BYTES);
        assert!(spool
            .append(&second, &cancelled)
            .unwrap_err()
            .to_string()
            .contains("poisoned"));
        assert!(spool
            .next(&cancelled)
            .err()
            .unwrap()
            .to_string()
            .contains("poisoned"));
        spool.cleanup().unwrap();
        assert!(!root.exists(), "failed writer retained its private parts");
    }
}
