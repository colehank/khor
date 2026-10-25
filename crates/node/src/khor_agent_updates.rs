//! Bounded display snapshots of standard ACP updates, never execution input.
use crate::AgentTurn;
use serde_json::Value;

const LIMIT: usize = 256 * 1024;
const FRAME_LIMIT: usize = 48 * 1024;
const COUNT: usize = 512;

fn redact(value: &mut Value, key: &str) {
    match value {
        Value::String(s) => {
            if let std::borrow::Cow::Owned(redacted) = super::output::redact(s, key) {
                *s = redacted;
            }
        }
        Value::Array(xs) => xs.iter_mut().for_each(|x| redact(x, key)),
        Value::Object(xs) => xs.values_mut().for_each(|x| redact(x, key)),
        _ => {}
    }
}

pub(super) fn push(agent: &mut AgentTurn, mut value: Value, key: &str) {
    // Notices are transient, never durable conversation history.
    if value["sessionUpdate"] == "notice" {
        return;
    }
    redact(&mut value, key);
    if value.to_string().len() > FRAME_LIMIT {
        agent.updates_truncated = true;
        if let Some(object) = value.as_object_mut() {
            object.remove("rawInput");
            object.remove("rawOutput");
            if let Some(Value::Array(content)) = object.get_mut("content") {
                // Keep small, complete content blocks; never truncate a diff
                // and then claim that its fragment is a complete changed file.
                content.retain(|c| c.to_string().len() <= FRAME_LIMIT / 2);
            }
        }
    }
    let kind = value["sessionUpdate"].as_str().unwrap_or("");
    let id = value["toolCallId"].as_str();
    let mut replace = None;
    if kind == "tool_call_update" {
        replace = agent.updates.iter().rposition(|s| {
            serde_json::from_str::<Value>(s)
                .is_ok_and(|v| v["sessionUpdate"] == "tool_call" && v["toolCallId"].as_str() == id)
        });
        if let Some(at) = replace {
            let mut prior: Value =
                serde_json::from_str(&agent.updates[at]).expect("validated update");
            for (key, val) in value.as_object().expect("ACP update") {
                if key == "sessionUpdate" || key == "toolCallId" {
                    continue;
                }
                if val.is_null() && key != "content" {
                    continue;
                }
                prior[key] = val.clone();
            }
            value = prior;
        }
    } else if matches!(kind, "agent_message_chunk" | "agent_thought_chunk") {
        if let Some(last) = agent
            .updates
            .last()
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
        {
            if last["sessionUpdate"] == kind
                && last["content"]["type"] == "text"
                && value["content"]["type"] == "text"
            {
                let text = format!(
                    "{}{}",
                    last["content"]["text"].as_str().unwrap_or(""),
                    value["content"]["text"].as_str().unwrap_or("")
                );
                value["content"]["text"] = text.into();
                replace = Some(agent.updates.len() - 1);
            }
        }
    } else if matches!(kind, "compaction_update" | "compaction_summary_chunk") {
        replace = agent.updates.iter().rposition(|s| {
            serde_json::from_str::<Value>(s).is_ok_and(|v| {
                v["sessionUpdate"] == "compaction_update"
                    && v["compactionId"] == value["compactionId"]
            })
        });
        let mut prior=replace.and_then(|at|serde_json::from_str::<Value>(&agent.updates[at]).ok()).unwrap_or_else(||serde_json::json!({"sessionUpdate":"compaction_update","compactionId":value["compactionId"],"status":"in_progress"}));
        if kind == "compaction_summary_chunk" {
            if !prior["summary"].is_array() {
                prior["summary"] = serde_json::json!([]);
            }
            prior["summary"]
                .as_array_mut()
                .unwrap()
                .push(value["content"].clone());
        } else {
            for (k, v) in value.as_object().expect("ACP update") {
                prior[k] = v.clone();
            }
        }
        value = prior;
    } else if matches!(kind, "plan" | "usage_update" | "config_option_update") {
        replace = agent.updates.iter().rposition(|s| {
            serde_json::from_str::<Value>(s).is_ok_and(|v| v["sessionUpdate"] == kind)
        });
    }
    let mut encoded = value.to_string();
    let old = replace.map_or(0, |i| agent.updates[i].len());
    let total = agent.updates.iter().map(String::len).sum::<usize>();
    if total - old + encoded.len() > LIMIT {
        agent.updates_truncated = true;
        // A terminal tool status still matters when its large output cannot fit.
        if value["sessionUpdate"] == "tool_call" && replace.is_some() {
            let mut small: Value = serde_json::from_str(&agent.updates[replace.unwrap()]).unwrap();
            small["status"] = value["status"].clone();
            encoded = small.to_string();
        } else {
            return;
        }
    }
    if let Some(at) = replace {
        if matches!(
            value["sessionUpdate"].as_str(),
            Some("usage_update" | "config_option_update" | "plan")
        ) {
            agent.updates.remove(at);
            agent.updates.push(encoded);
        } else {
            agent.updates[at] = encoded;
        }
    } else if agent.updates.len() < COUNT {
        agent.updates.push(encoded);
    } else {
        agent.updates_truncated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn turn() -> AgentTurn {
        AgentTurn {
            model: "test".into(),
            approval: None,
            updated_at_ms: 0,
            updates: vec![],
            updates_truncated: false,
        }
    }
    #[test]
    fn snapshots_keep_receive_order_and_compaction_patches_keep_their_meaning(){
        let mut a=turn();
        push(&mut a,json!({"sessionUpdate":"usage_update","used":900,"size":1000}),"");
        push(&mut a,json!({"sessionUpdate":"compaction_update","compactionId":"c","status":"in_progress","summary":[{"type":"text","text":"kept"}]}),"");
        push(&mut a,json!({"sessionUpdate":"compaction_update","compactionId":"c","status":"completed"}),"");
        push(&mut a,json!({"sessionUpdate":"usage_update","used":200,"size":1000}),"");
        let values:Vec<Value>=a.updates.iter().map(|s|serde_json::from_str(s).unwrap()).collect();assert_eq!(values[0]["summary"][0]["text"],"kept");assert_eq!(values.last().unwrap()["used"],200);
        push(&mut a,json!({"sessionUpdate":"compaction_update","compactionId":"c","status":"completed","summary":null}),"");
        let compaction:Value=serde_json::from_str(&a.updates[0]).unwrap();assert!(compaction["summary"].is_null());
        let count=a.updates.len();push(&mut a,json!({"sessionUpdate":"notice","title":"temporary","severity":"info"}),"");assert_eq!(a.updates.len(),count);
    }
    #[test]
    fn compacts_chunks_and_updates_without_losing_order_or_credentials() {
        let mut a = turn();
        for text in ["one ", "secret"] {
            push(
                &mut a,
                json!({"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":text}}),
                "secret",
            );
        }
        push(
            &mut a,
            json!({"sessionUpdate":"tool_call","toolCallId":"1","title":"read","status":"in_progress","rawInput":{"token":"secret"}}),
            "secret",
        );
        push(
            &mut a,
            json!({"sessionUpdate":"tool_call_update","toolCallId":"1","status":"completed","content":[{"type":"content","content":{"type":"text","text":"ok"}}]}),
            "secret",
        );
        assert_eq!(a.updates.len(), 2);
        assert!(!a.updates.join("").contains("secret"));
        assert_eq!(
            serde_json::from_str::<Value>(&a.updates[1]).unwrap()["status"],
            "completed"
        );
    }
    #[test]
    fn bounds_display_data_and_preserves_final_tool_status() {
        let mut a = turn();
        push(
            &mut a,
            json!({"sessionUpdate":"tool_call","toolCallId":"1","title":"read","status":"in_progress"}),
            "",
        );
        push(
            &mut a,
            json!({"sessionUpdate":"tool_call_update","toolCallId":"1","status":"failed","content":[{"type":"content","content":{"type":"text","text":"x".repeat(LIMIT)}}]}),
            "",
        );
        assert!(a.updates_truncated);
        assert!(a.updates.iter().map(String::len).sum::<usize>() <= LIMIT);
        assert_eq!(
            serde_json::from_str::<Value>(&a.updates[0]).unwrap()["status"],
            "failed"
        );
        let older: AgentTurn =
            serde_json::from_value(json!({"model":"old","approval":null})).unwrap();
        assert!(older.updates.is_empty());
    }
}
