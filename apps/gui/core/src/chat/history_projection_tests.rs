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
        updates: (10..139).map(|index| (index, update.clone(), false)).collect(), transforms: Vec::new(), complete: true,
    }, &|| false).unwrap();
    assert_eq!(response.entries.len(), 128);
    assert_eq!(response.entries[127].key, "projection:137");
    assert!(!response.complete);
    assert_eq!(response.record.cursor, "same-file-snapshot");
    let encoded = serde_json::to_value(&response).unwrap();
    assert!(encoded["entries"].as_array().unwrap().iter().all(|entry| entry["frame"]["kind"] == "note"));
    assert!(response.entries.iter().all(|entry|entry.record.is_none()),
        "caller scaffold content cannot manufacture a trustworthy original record");
    assert!(encoded_size(&response).unwrap() <= HISTORY_PAYLOAD_BYTES);
}

#[test]
fn scaffold_body_payload_and_cancellation_stay_bounded() {
    let update = serde_json::json!({"update":{"sessionUpdate":"agent_message_chunk",
        "content":{"type":"text","text":"x".repeat(30000)}}});
    let response = body_projection_response(record(), khor_node::session_content::BodyProjection {
        updates: (0..128).map(|index| (index, update.clone(), false)).collect(), transforms: Vec::new(), complete: true,
    }, &|| false).unwrap();
    assert!(!response.complete);
    assert!(response.entries.len() < 128);
    assert!(encoded_size(&response).unwrap() <= HISTORY_PAYLOAD_BYTES);
    let error = body_projection_response(record(), khor_node::session_content::BodyProjection {
        updates: vec![(7, update, false)], transforms: Vec::new(), complete: true,
    }, &|| true).err().unwrap();
    assert!(error.contains("cancelled"));
}

#[test]
fn projection_facts_have_only_the_two_existing_typed_rules_and_a_metadata_budget() {
    let projected = body_projection_response(record(), khor_node::session_content::BodyProjection {
        updates: Vec::new(), transforms: vec![
            khor_node::session_content::BodyTransform::JsonString { pointer: "/payload/input".into(), parsed: false },
            khor_node::session_content::BodyTransform::ToolOutput { item: 7, max_bytes: 16384, suffix: "\n… cut".into() },
        ], complete: true,
    }, &|| false).unwrap();
    let value = serde_json::to_value(projected).unwrap();
    assert_eq!(value["transforms"][0], serde_json::json!({"kind":"json_string","pointer":"/payload/input","parsed":false}));
    assert_eq!(value["transforms"][1], serde_json::json!({"kind":"tool_output","item":7,"max_bytes":16384,"suffix":"\n… cut"}));
    let error = body_projection_response(record(), khor_node::session_content::BodyProjection {
        updates: Vec::new(), transforms: vec![khor_node::session_content::BodyTransform::ToolOutput {
            item: 0, max_bytes: 16384, suffix: "x".repeat(16385),
        }], complete: true,
    }, &|| false).err().unwrap();
    assert!(error.contains("metadata budget"));
}

#[test]
fn scaffold_projection_preserves_reader_message_boundaries_without_promoting_payload_claims() {
    let update = serde_json::json!({"starts_message":true,"update":{"sessionUpdate":"agent_message_chunk",
        "messageId":"same-message","content":{"type":"text","text":"body"}}});
    let response = body_projection_response(record(), khor_node::session_content::BodyProjection {
        updates: vec![(0, update.clone(), false), (1, update.clone(), true)],
        transforms: Vec::new(), complete: true,
    }, &|| false).unwrap();
    assert!(!response.entries[0].starts_message, "an SDK field cannot replace read provenance");
    assert!(response.entries[1].starts_message);
    assert!(response.entries.iter().all(|entry|entry.record.is_none()));
    assert_eq!((response.record.start,response.record.end),(1,10),
        "the outer checked selection is preserved separately, without promoting it into each projected entry");
    for entry in &response.entries {
        assert!(matches!(&entry.frame,ChatFrame::Note {update:body} if body==&update));
    }
    assert!(response.complete && encoded_size(&response).unwrap() <= HISTORY_PAYLOAD_BYTES);
}
