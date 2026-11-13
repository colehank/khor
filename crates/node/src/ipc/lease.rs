//! Opt-in request framing and caller ownership. The legacy protocol uses EOF
//! as its frame boundary; only the explicit v2 frame can use EOF as cancellation.

use std::{future::Future, time::Duration};

use tokio::{
    io::{AsyncRead, AsyncReadExt},
    time::Instant,
};

use super::{Handoff, validate_handoff};
use crate::{
    deadline,
    proto::{self, MAX_FRAME},
};

pub(crate) const VERSION: u8 = 2;
const MAGIC: &[u8; 9] = b"\xffKHORIPC2";
const HEADER_BYTES: usize = MAGIC.len() + 4 + 8;

pub(crate) struct Request {
    pub(crate) handoff: Handoff,
    pub(crate) by: Option<Instant>,
}

/// Snapshot the remaining budget after payload serialization and validation.
/// No Instant or wall clock crosses the process boundary.
pub(crate) fn header(bytes: usize, by: Instant) -> Result<[u8; HEADER_BYTES], String> {
    if bytes == 0 || bytes > MAX_FRAME {
        return Err("invalid IPC payload length".into());
    }
    let micros = by.saturating_duration_since(Instant::now()).as_micros();
    let micros = u64::try_from(micros).map_err(|_| "IPC budget is too large")?;
    if micros == 0 {
        return Err("IPC caller deadline expired".into());
    }
    let mut header = [0; HEADER_BYTES];
    header[..MAGIC.len()].copy_from_slice(MAGIC);
    header[9..13].copy_from_slice(&(bytes as u32).to_be_bytes());
    header[13..].copy_from_slice(&micros.to_be_bytes());
    Ok(header)
}

/// The local framing phase has its own bounded admission time. A leased
/// body's reads also consume the budget anchored when its header arrived.
pub(crate) async fn read(reader: &mut (impl AsyncRead + Unpin)) -> Result<Request, String> {
    let framing_by = deadline::earlier(Some(Instant::now() + super::DEADLINE_LOCAL));
    let request = async {
        let first = reader.read_u8().await.map_err(|e| e.to_string())?;
        let (bytes, budget) = if first == MAGIC[0] {
            let mut encoded = [0; HEADER_BYTES];
            encoded[0] = first;
            reader
                .read_exact(&mut encoded[1..])
                .await
                .map_err(|e| e.to_string())?;
            if &encoded[..MAGIC.len()] != MAGIC {
                return Err("unsupported IPC lease frame".into());
            }
            let length = u32::from_be_bytes(encoded[9..13].try_into().unwrap()) as usize;
            let micros = u64::from_be_bytes(encoded[13..].try_into().unwrap());
            if length == 0 || length > MAX_FRAME || micros == 0 {
                return Err("invalid IPC length or budget".into());
            }
            let arrived = Instant::now();
            let by = arrived
                .checked_add(Duration::from_micros(micros))
                .ok_or("IPC budget is too large")?;
            let mut bytes = vec![0; length];
            deadline::wait(Some(by), reader.read_exact(&mut bytes))
                .await
                .map_err(|_| "IPC caller deadline expired")?
                .map_err(|e| e.to_string())?;
            (bytes, Some((by, arrived)))
        } else {
            let mut bytes = vec![first];
            (&mut *reader)
                .take(MAX_FRAME as u64)
                .read_to_end(&mut bytes)
                .await
                .map_err(|e| e.to_string())?;
            if bytes.len() > MAX_FRAME {
                return Err("IPC payload exceeds its wire budget".into());
            }
            (bytes, None)
        };
        let handoff = proto::decode::<Handoff>(&bytes)?;
        validate_handoff(&handoff, bytes.len())?;
        let by = budget.map(|(by, arrived)| by.min(arrived + handoff.op.deadline()));
        if deadline::expired(by) {
            return Err("IPC caller deadline expired".into());
        }
        Ok(Request { handoff, by })
    };
    deadline::wait(framing_by, request)
        .await
        .map_err(|_| "IPC frame deadline expired")?
}

/// One future owns work, response buffers and permits through the final write.
/// Extra input, EOF, read failure, or budget expiry drops that entire future.
pub(crate) async fn within<T>(
    reader: &mut (impl AsyncRead + Unpin),
    by: Option<Instant>,
    operation: impl Future<Output = Result<T, String>>,
) -> Result<T, String> {
    let Some(by) = by else { return operation.await };
    let by = deadline::earlier(Some(by)).expect("a lease supplies its deadline");
    deadline::scope(by, async {
        tokio::select! {
            biased;
            _ = reader.read_u8() => Err("IPC caller lease ended".into()),
            result = deadline::wait(Some(by), operation) => {
                result.map_err(|_| "IPC caller deadline expired")?
            }
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use tokio::io::AsyncWriteExt;

    fn payload() -> Vec<u8> {
        proto::encode(&Handoff {
            cookie: "cookie".into(),
            op: super::super::Op::Hops,
        })
        .unwrap()
    }

    #[tokio::test]
    async fn legacy_half_close_remains_a_request_boundary() {
        let (mut writer, mut reader) = tokio::io::duplex(4096);
        let bytes = payload();
        assert_ne!(
            bytes[0], MAGIC[0],
            "a Handoff is an array or map, never negative integer -1"
        );
        writer.write_all(&bytes).await.unwrap();
        writer.shutdown().await.unwrap();
        let request = read(&mut reader).await.unwrap();
        assert_eq!(request.handoff.cookie, "cookie");
        assert!(request.by.is_none());
        assert_eq!(within(&mut reader, None, async { Ok(7) }).await.unwrap(), 7);
    }

    #[tokio::test]
    async fn leased_payload_finishes_without_half_close() {
        let (mut writer, mut reader) = tokio::io::duplex(4096);
        let bytes = payload();
        writer
            .write_all(&header(bytes.len(), Instant::now() + Duration::from_secs(1)).unwrap())
            .await
            .unwrap();
        writer.write_all(&bytes).await.unwrap();
        let request = tokio::time::timeout(Duration::from_millis(200), read(&mut reader))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(request.handoff.cookie, "cookie");
        assert!(
            request
                .by
                .is_some_and(|by| by <= Instant::now() + Duration::from_secs(1))
        );
        assert_eq!(
            within(&mut reader, request.by, async { Ok(9) })
                .await
                .unwrap(),
            9
        );
        drop(writer);
    }

    #[tokio::test]
    async fn body_wait_cannot_remint_the_received_budget() {
        let (mut writer, mut reader) = tokio::io::duplex(4096);
        writer
            .write_all(
                &header(payload().len(), Instant::now() + Duration::from_millis(60)).unwrap(),
            )
            .await
            .unwrap();
        let request = tokio::time::timeout(Duration::from_millis(500), read(&mut reader))
            .await
            .unwrap();
        assert!(
            request.is_err(),
            "the body never arrived but its header budget elapsed"
        );
        drop(writer);
    }

    #[tokio::test]
    async fn advertised_budget_cannot_extend_the_operations_safety_cap() {
        let (mut writer, mut reader) = tokio::io::duplex(4096);
        let bytes = payload();
        writer
            .write_all(&header(bytes.len(), Instant::now() + Duration::from_secs(86400)).unwrap())
            .await
            .unwrap();
        writer.write_all(&bytes).await.unwrap();
        let request = read(&mut reader).await.unwrap();
        assert!(request.by.unwrap() <= Instant::now() + request.handoff.op.deadline());
        drop(writer);
    }

    #[tokio::test]
    async fn a_local_parent_deadline_also_bounds_the_entire_lease() {
        let (writer, mut reader) = tokio::io::duplex(4096);
        let by = Instant::now() + Duration::from_secs(10);
        let parent = Instant::now() + Duration::from_millis(60);
        let result = tokio::time::timeout(
            Duration::from_millis(500),
            deadline::scope(
                parent,
                within(
                    &mut reader,
                    Some(by),
                    std::future::pending::<Result<(), String>>(),
                ),
            ),
        )
        .await
        .expect("the helper must not ignore its earlier local owner");
        assert!(result.is_err());
        drop(writer);
    }

    #[tokio::test]
    async fn malformed_or_truncated_frames_never_decode_a_handoff() {
        let good = header(payload().len(), Instant::now() + Duration::from_secs(1)).unwrap();
        let mut bad_magic = good;
        bad_magic[8] = b'3';
        let mut too_large = good;
        too_large[9..13].copy_from_slice(&((MAX_FRAME + 1) as u32).to_be_bytes());
        let mut zero_budget = good;
        zero_budget[13..].fill(0);
        let mut zero_length = good;
        zero_length[9..13].fill(0);
        for bytes in [
            bad_magic.to_vec(),
            too_large.to_vec(),
            zero_budget.to_vec(),
            zero_length.to_vec(),
            good[..10].to_vec(),
            good.to_vec(),
        ] {
            let (mut writer, mut reader) = tokio::io::duplex(4096);
            writer.write_all(&bytes).await.unwrap();
            writer.shutdown().await.unwrap();
            assert!(read(&mut reader).await.is_err());
        }
        assert!(header(payload().len(), Instant::now()).is_err());
        assert!(header(MAX_FRAME + 1, Instant::now() + Duration::from_secs(1)).is_err());
    }

    struct Released(Arc<AtomicBool>);
    impl Drop for Released {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn lease_end_and_extra_input_drop_unpolled_work_and_its_owner() {
        for extra in [false, true] {
            let (mut writer, mut reader) = tokio::io::duplex(4096);
            if extra {
                writer.write_all(&[0]).await.unwrap();
            } else {
                writer.shutdown().await.unwrap();
            }
            let released = Arc::new(AtomicBool::new(false));
            let owner = Released(released.clone());
            let began = Arc::new(AtomicBool::new(false));
            let started = began.clone();
            let work = async move {
                let _owner = owner;
                started.store(true, Ordering::SeqCst);
                std::future::pending::<Result<(), String>>().await
            };
            assert!(
                within(
                    &mut reader,
                    Some(Instant::now() + Duration::from_secs(1)),
                    work
                )
                .await
                .is_err()
            );
            assert!(!began.load(Ordering::SeqCst));
            assert!(released.load(Ordering::SeqCst));
        }
    }

    #[tokio::test]
    async fn deadline_and_disconnect_release_a_blocked_response_owner() {
        for disconnected in [false, true] {
            let (client, server) = tokio::io::duplex(1);
            let (mut reader, mut writer) = tokio::io::split(server);
            let released = Arc::new(AtomicBool::new(false));
            let owner = Released(released.clone());
            let work = async move {
                let _owner = owner;
                writer.write_all(&[0; 8]).await.map_err(|e| e.to_string())
            };
            let by = Instant::now() + Duration::from_millis(60);
            let result = if disconnected {
                drop(client);
                within(&mut reader, Some(by), work).await
            } else {
                let result = within(&mut reader, Some(by), work).await;
                drop(client);
                result
            };
            assert!(result.is_err());
            assert!(released.load(Ordering::SeqCst));
        }
    }
}
