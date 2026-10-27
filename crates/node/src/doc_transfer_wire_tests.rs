use crate::proto::*;
use khor_sync::{
    doc_transfer::{Descriptor, DocumentVersion, Encoding},
    transfer::{CHUNK_BYTES, MAX_BLOB_BYTES, MAX_METADATA_BYTES},
};

fn version(first: u64, count: u32) -> DocumentVersion {
    // Decode the real canonical format, without depending on Loro's map
    // iteration order. High-byte writer ids exercise their complete encoding.
    let mut bytes = b"khor-version-v1\0".to_vec();
    bytes.extend_from_slice(&7u32.to_be_bytes());
    bytes.extend_from_slice(&count.to_be_bytes());
    for index in 0..count {
        bytes.extend_from_slice(&(first + u64::from(index)).to_be_bytes());
        bytes.extend_from_slice(&(i32::MAX as u32).to_be_bytes());
    }
    DocumentVersion::decode(&bytes).unwrap()
}

fn largest_components() -> (String, DocChunk, DocProof, DocAcknowledgement, Vec<u8>) {
    let name = format!("wide/{}", "a".repeat(126));
    let first = 0x8080_8080_8080_0000;
    // This name and 5,438 entries fill the core manifest's entire
    // 65,416-byte binding budget; persisted/have can be larger.
    let descriptor = Descriptor {
        document: name.clone(),
        epoch: 7,
        encoding: Encoding::Updates,
        base: version(first, 2719).version,
        target: version(first + 2719, 2719).version,
    };
    let manifest = descriptor
        .bind([0xff; 32], [0xfe; 32], MAX_BLOB_BYTES, [0xfd; 32])
        .unwrap();
    assert_eq!(manifest.binding.len(), 65_416);
    let persisted = version(first, 5459).encode().unwrap();
    assert!(persisted.len() <= MAX_METADATA_BYTES);
    assert!(persisted.len() + 12 > MAX_METADATA_BYTES);
    let chunk = DocChunk {
        manifest: (&manifest).into(),
        offset: 0,
        bytes: vec![0xff; CHUNK_BYTES].into(),
    };
    let proof = DocProof {
        manifest: chunk.manifest.clone(),
        persisted_version: persisted.clone().into(),
        kind: DocProofKind::AlreadyHaveEquivalent,
    };
    let ack = proof.acknowledgement().unwrap();
    (name, chunk, proof, ack, persisted)
}

#[test]
fn full_canonical_chunks_and_worst_legal_metadata_fit_the_actual_envelopes() {
    let (name, chunk, proof, ack, have) = largest_components();
    let mut reverse_proof = proof.clone();
    std::mem::swap(
        &mut reverse_proof.manifest.source,
        &mut reverse_proof.manifest.recipient,
    );
    let request = Request::DocTransfer {
        action: DocTransferAction::Exchange {
            document: name.clone(),
            have: have.clone().into(),
            offer: Some(chunk.clone()),
            proofs: vec![reverse_proof.clone()],
            acknowledgements: vec![ack.clone()],
        },
    };
    let encoded_request = encode_request(&request).unwrap();
    let Request::DocTransfer { action } = question(&encoded_request).unwrap() else {
        panic!("the maximum request must remain a document request");
    };
    super::validate_roles(&action, [0xfe; 32], [0xff; 32]).unwrap();
    let mut reverse = chunk.clone();
    std::mem::swap(
        &mut reverse.manifest.source,
        &mut reverse.manifest.recipient,
    );
    let response = Response::DocTransfer {
        reply: DocTransferReply::Exchanged {
            document: name,
            have: have.into(),
            received: Some(DocTransferState::Complete {
                proof: proof.clone(),
            }),
            offer: Some(reverse),
            proof_acknowledgements: vec![reverse_proof.acknowledgement().unwrap()],
            acknowledgement_results: vec![DocAckResult::Reconfirm { proof }],
        },
    };
    let encoded_response = encode_response(&response).unwrap();
    println!(
        "DocTransfer worst-envelope request={} response={} chunk={CHUNK_BYTES}",
        encoded_request.len(),
        encoded_response.len()
    );
    for bytes in [&encoded_request, &encoded_response] {
        assert!(
            bytes.len() > 512 * 1024,
            "512 KiB is insufficient for this legal envelope"
        );
        assert!(bytes.len() <= DOC_TRANSFER_FRAME_BYTES);
    }
    assert!(matches!(decode::<Request>(&encoded_request).unwrap(),
        Request::DocTransfer { action: DocTransferAction::Exchange { offer: Some(c), .. } }
            if c.bytes.len() == CHUNK_BYTES));
    assert!(matches!(
        answer(&encoded_response).unwrap(),
        Response::DocTransfer { .. }
    ));

    for reply in [
        DocTransferReply::Confirmed {
            acknowledgements: vec![ack.clone(), ack],
        },
        DocTransferReply::Acknowledged {
            results: vec![
                DocAckResult::Reconfirm {
                    proof: reverse_proof.clone(),
                },
                DocAckResult::Reconfirm {
                    proof: reverse_proof,
                },
            ],
        },
    ] {
        assert!(
            encode_response(&Response::DocTransfer { reply })
                .unwrap()
                .len()
                <= DOC_TRANSFER_FRAME_BYTES
        );
    }
}

#[test]
fn malformed_chunks_and_oversized_raw_doc_frames_are_refused() {
    let (_, mut chunk, _, _, _) = largest_components();
    chunk.bytes = vec![0xff; CHUNK_BYTES - 1].into();
    assert!(
        encode_request(&Request::DocTransfer {
            action: DocTransferAction::Write { chunk },
        })
        .is_err(),
        "a nonfinal chunk cannot be shortened to fit a frame"
    );
    let response = Response::DocTransfer {
        reply: DocTransferReply::Failed {
            kind: DocTransferFailure::Storage,
            why: "x".repeat(DOC_TRANSFER_FRAME_BYTES),
        },
    };
    let unchecked = encode(&response).unwrap();
    assert!(unchecked.len() > DOC_TRANSFER_FRAME_BYTES);
    assert!(answer(&unchecked).is_err());
    assert!(encode_response(&response).is_err());
}

#[test]
fn every_doc_action_is_retryable_after_authentication_and_never_early_data() {
    let (document, chunk, proof, ack, have) = largest_components();
    let manifest = chunk.manifest.clone();
    for action in [
        DocTransferAction::Exchange {
            document,
            have: have.into(),
            offer: Some(chunk.clone()),
            proofs: vec![],
            acknowledgements: vec![],
        },
        DocTransferAction::Write { chunk },
        DocTransferAction::Read {
            manifest: manifest.clone(),
            offset: 0,
        },
        DocTransferAction::Status { manifest },
        DocTransferAction::Confirm {
            proofs: vec![proof],
        },
        DocTransferAction::Acknowledge {
            acknowledgements: vec![ack],
        },
    ] {
        let request = Request::DocTransfer { action };
        assert!(request.is_reading());
        assert!(!request.rides_early());
        assert_eq!(request.budget(), std::time::Duration::from_secs(30));
        encode_request(&request).unwrap();
    }
}
