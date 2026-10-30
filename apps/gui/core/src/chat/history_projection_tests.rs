use super::*;

fn record() -> HistoryRecordRef {
    HistoryRecordRef { start: 1, end: 10, missing_start: false, missing_end: false,
        item: None, cursor: "same-file-snapshot".into() }
}

#[test]
fn scaffold_body_response_cannot_promote_frame_types_or_silently_lose_expansion() {
    let update = serde_json::json!({"update":{"sessionUpdate":"agent_message_chunk",
        "content":{"type":"text","text":"marker"}},"kind":"capture_gap"});
    let response = body_projection_response(record(), khor_node::session_content::BodyProjection {
        updates: vec![update; 129], complete: true,
    }, &|| false).unwrap();
    assert_eq!(response.entries.len(), 128);
    assert!(!response.complete);
    assert_eq!(response.record.cursor, "same-file-snapshot");
    let encoded = serde_json::to_value(&response).unwrap();
    assert!(encoded["entries"].as_array().unwrap().iter().all(|entry| entry["frame"]["kind"] == "note"));
    assert!(encoded_size(&response).unwrap() <= HISTORY_PAYLOAD_BYTES);
}

#[test]
fn scaffold_body_payload_and_cancellation_stay_bounded() {
    let update = serde_json::json!({"update":{"sessionUpdate":"agent_message_chunk",
        "content":{"type":"text","text":"x".repeat(30000)}}});
    let response = body_projection_response(record(), khor_node::session_content::BodyProjection {
        updates: vec![update.clone(); 128], complete: true,
    }, &|| false).unwrap();
    assert!(!response.complete);
    assert!(response.entries.len() < 128);
    assert!(encoded_size(&response).unwrap() <= HISTORY_PAYLOAD_BYTES);
    let error = body_projection_response(record(), khor_node::session_content::BodyProjection {
        updates: vec![update], complete: true,
    }, &|| true).err().unwrap();
    assert!(error.contains("cancelled"));
}
