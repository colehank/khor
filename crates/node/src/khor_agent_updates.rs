//! Bounded display snapshots of standard ACP updates, never execution input.
use crate::AgentTurn;
use serde_json::Value;
use std::collections::HashMap;

use super::output::{TextStream, redact as redact_text};

const LIMIT: usize = 256 * 1024;
const FRAME_LIMIT: usize = 48 * 1024;
const COUNT: usize = 512;

#[derive(Clone, Eq, Hash, PartialEq)]
enum StreamKey {
    Message,
    Thought,
    Compaction(String),
}

impl StreamKey {
    fn id_len(&self) -> usize {
        match self {
            Self::Compaction(id) => id.len(),
            _ => 0,
        }
    }
}

struct Stream<'a> {
    text: TextStream<'a>,
    writable: bool,
}

/// Raw matching state belongs to this execution, never the saved AgentTurn.
pub(super) struct Recorder<'a> {
    key: &'a str,
    matcher: TextStream<'a>,
    streams: HashMap<StreamKey, Stream<'a>>,
}

impl<'a> Recorder<'a> {
    pub(super) fn new(key: &'a str) -> Self {
        Self {
            key,
            matcher: TextStream::new(key),
            streams: HashMap::new(),
        }
    }

    fn retained_len(&self) -> usize {
        self.streams
            .iter()
            .map(|(id, stream)| id.id_len() + stream.text.retained_len())
            .sum()
    }

    pub(super) fn push(&mut self, agent: &mut AgentTurn, mut value: Value) {
        if value["sessionUpdate"] == "notice" {
            return;
        }
        if self.key.is_empty() {
            store(agent, value);
            return;
        }
        let kind = value["sessionUpdate"].as_str().unwrap_or("");
        let stream_key = match kind {
            "agent_message_chunk" => Some(StreamKey::Message),
            "agent_thought_chunk" => Some(StreamKey::Thought),
            "compaction_update" | "compaction_summary_chunk" => value["compactionId"]
                .as_str()
                .map(|id| StreamKey::Compaction(id.to_owned())),
            _ => None,
        };
        if kind == "compaction_update" && value.get("summary").is_some() {
            if let Some(key) = &stream_key {
                self.streams.remove(key);
            }
        }
        let mut texts = Vec::new();
        if matches!(
            kind,
            "agent_message_chunk" | "agent_thought_chunk" | "compaction_summary_chunk"
        ) {
            if value["content"]["type"] == "text" {
                if let Some(text) = value["content"]["text"].as_str() {
                    texts.push((None, text.to_owned()));
                }
            }
        } else if kind == "compaction_update" {
            if let Some(summary) = value["summary"].as_array() {
                for (index, block) in summary.iter().enumerate() {
                    if block["type"] == "text" {
                        if let Some(text) = block["text"].as_str() {
                            texts.push((Some(index), text.to_owned()));
                        }
                    }
                }
            }
        }

        // Tool-call patches replace content/rawOutput; only the protocol's
        // append-only message, thought and compaction text use stream state.
        redact(&mut value, self.key);
        let active = stream_key.filter(|_| !texts.is_empty());
        if let Some(key) = &active {
            if !self.streams.contains_key(key) {
                if self.streams.len() >= COUNT || self.retained_len() + key.id_len() > LIMIT {
                    agent.updates_truncated = true;
                    return;
                }
                self.streams.insert(
                    key.clone(),
                    Stream {
                        text: self.matcher.fresh(),
                        writable: true,
                    },
                );
            }
            let stream = self.streams.get_mut(key).unwrap();
            if !stream.writable {
                agent.updates_truncated = true;
                return;
            }
            for (index, text) in texts {
                let display = stream.text.push(&text);
                if let Some(index) = index {
                    value["summary"][index]["text"] = display.into();
                } else {
                    value["content"]["text"] = display.into();
                }
            }
            if self.retained_len() > LIMIT {
                let stream = self.streams.get_mut(key).unwrap();
                stream.writable = false;
                stream.text.finish();
                agent.updates_truncated = true;
                return;
            }
        }
        let stored = store(agent, value);
        if !stored {
            if let Some(stream) = active.and_then(|key| self.streams.get_mut(&key)) {
                stream.writable = false;
                stream.text.finish();
            }
        }
    }

    pub(super) fn finish(&mut self, agent: &mut AgentTurn) {
        for (key, mut stream) in std::mem::take(&mut self.streams) {
            let tail = stream.text.finish();
            if !stream.writable || tail.is_empty() {
                continue;
            }
            let kind = match &key {
                StreamKey::Message => "agent_message_chunk",
                StreamKey::Thought => "agent_thought_chunk",
                StreamKey::Compaction(_) => "compaction_update",
            };
            let kind = redact_text(kind, self.key);
            let id = match &key {
                StreamKey::Compaction(id) => Some(redact_text(id, self.key)),
                _ => None,
            };
            let at = agent.updates.iter().rposition(|encoded| {
                serde_json::from_str::<Value>(encoded).is_ok_and(|value| {
                    value["sessionUpdate"].as_str() == Some(kind.as_ref())
                        && id
                            .as_ref()
                            .is_none_or(|id| value["compactionId"].as_str() == Some(id.as_ref()))
                })
            });
            let Some(at) = at else {
                agent.updates_truncated = true;
                continue;
            };
            let mut value: Value = serde_json::from_str(&agent.updates[at]).unwrap();
            let text_kind = redact_text("text", self.key);
            let block = if id.is_some() {
                value["summary"].as_array_mut().and_then(|summary| {
                    summary
                        .iter_mut()
                        .rev()
                        .find(|block| block["type"].as_str() == Some(text_kind.as_ref()))
                })
            } else {
                value.get_mut("content")
            };
            let Some(text) = block.and_then(|block| block.get_mut("text")) else {
                agent.updates_truncated = true;
                continue;
            };
            let Some(prior) = text.as_str() else {
                agent.updates_truncated = true;
                continue;
            };
            *text = format!("{prior}{tail}").into();
            let encoded = value.to_string();
            let total = agent.updates.iter().map(String::len).sum::<usize>();
            if total - agent.updates[at].len() + encoded.len() <= LIMIT {
                agent.updates[at] = encoded;
            } else {
                agent.updates_truncated = true;
            }
        }
    }
}

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

fn store(agent: &mut AgentTurn, mut value: Value) -> bool {
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
            return false;
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
        return false;
    }
    true
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

    fn values(agent: &AgentTurn) -> Vec<Value> {
        agent
            .updates
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect()
    }

    fn text(agent: &AgentTurn, kind: &str) -> String {
        values(agent)
            .iter()
            .filter(|v| v["sessionUpdate"] == kind)
            .filter_map(|v| v["content"]["text"].as_str())
            .collect()
    }

    fn summary(agent: &AgentTurn, id: &str) -> String {
        values(agent)
            .iter()
            .filter(|v| v["compactionId"] == id)
            .filter_map(|v| v["summary"].as_array())
            .flatten()
            .filter_map(|v| v["text"].as_str())
            .collect()
    }

    #[test]
    fn snapshots_keep_receive_order_and_compaction_patches_keep_their_meaning() {
        let mut agent = turn();
        let mut recorder = Recorder::new("");
        recorder.push(
            &mut agent,
            json!({"sessionUpdate":"usage_update","used":900,"size":1000}),
        );
        recorder.push(&mut agent, json!({"sessionUpdate":"compaction_update","compactionId":"c","status":"in_progress","summary":[{"type":"text","text":"kept"}]}));
        recorder.push(
            &mut agent,
            json!({"sessionUpdate":"compaction_update","compactionId":"c","status":"completed"}),
        );
        recorder.push(
            &mut agent,
            json!({"sessionUpdate":"usage_update","used":200,"size":1000}),
        );
        let snapshots = values(&agent);
        assert_eq!(snapshots[0]["summary"][0]["text"], "kept");
        assert_eq!(snapshots.last().unwrap()["used"], 200);
        recorder.push(&mut agent, json!({"sessionUpdate":"compaction_update","compactionId":"c","status":"completed","summary":null}));
        assert!(values(&agent)[0]["summary"].is_null());
        let count = agent.updates.len();
        recorder.push(
            &mut agent,
            json!({"sessionUpdate":"notice","title":"temporary","severity":"info"}),
        );
        assert_eq!(agent.updates.len(), count);
    }

    #[test]
    fn compacts_chunks_and_updates_without_losing_order_or_credentials() {
        let mut agent = turn();
        let mut recorder = Recorder::new("secret");
        for text in ["one ", "secret"] {
            recorder.push(&mut agent, json!({"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":text}}));
        }
        recorder.push(&mut agent, json!({"sessionUpdate":"tool_call","toolCallId":"1","title":"read","status":"in_progress","rawInput":{"token":"secret"}}));
        recorder.push(&mut agent, json!({"sessionUpdate":"tool_call_update","toolCallId":"1","status":"completed","content":[{"type":"content","content":{"type":"text","text":"ok"}}]}));
        recorder.finish(&mut agent);
        assert_eq!(agent.updates.len(), 2);
        assert!(!agent.updates.join("").contains("secret"));
        assert_eq!(values(&agent)[1]["status"], "completed");
    }

    #[test]
    fn bounds_display_data_and_preserves_final_tool_status() {
        let mut agent = turn();
        let mut recorder = Recorder::new("");
        recorder.push(&mut agent, json!({"sessionUpdate":"tool_call","toolCallId":"1","title":"read","status":"in_progress"}));
        recorder.push(&mut agent, json!({"sessionUpdate":"tool_call_update","toolCallId":"1","status":"failed","content":[{"type":"content","content":{"type":"text","text":"x".repeat(LIMIT)}}]}));
        assert!(agent.updates_truncated);
        assert!(agent.updates.iter().map(String::len).sum::<usize>() <= LIMIT);
        assert_eq!(values(&agent)[0]["status"], "failed");
        let older: AgentTurn =
            serde_json::from_value(json!({"model":"old","approval":null})).unwrap();
        assert!(older.updates.is_empty());
    }

    #[test]
    fn independent_text_streams_survive_other_updates_and_never_rescan_markers() {
        let mut agent = turn();
        let mut recorder = Recorder::new("redact");
        for kind in ["agent_message_chunk", "agent_thought_chunk"] {
            recorder.push(
                &mut agent,
                json!({"sessionUpdate":kind,"content":{"type":"text","text":"before red"}}),
            );
            assert_eq!(text(&agent, kind), "before ");
        }
        recorder.push(&mut agent, json!({"sessionUpdate":"compaction_summary_chunk","compactionId":"c","content":{"type":"text","text":"red"}}));
        recorder.push(&mut agent, json!({"sessionUpdate":"compaction_summary_chunk","compactionId":"d","content":{"type":"text","text":"partial red"}}));
        recorder.push(
            &mut agent,
            json!({"sessionUpdate":"usage_update","used":1,"size":2}),
        );
        recorder.push(&mut agent, json!({"sessionUpdate":"tool_call","toolCallId":"tool","title":"read","content":[{"type":"content","content":{"type":"text","text":"red"}}]}));
        recorder.push(&mut agent, json!({"sessionUpdate":"tool_call_update","toolCallId":"tool","status":"completed","content":[{"type":"content","content":{"type":"text","text":"act"}}],"rawOutput":"redact"}));
        let tool = values(&agent)
            .into_iter()
            .find(|v| v["toolCallId"] == "tool")
            .unwrap();
        assert_eq!(
            tool["content"][0]["content"]["text"], "act",
            "tool content replaces, rather than appends"
        );
        assert_eq!(tool["rawOutput"], "[redacted]");
        for kind in ["agent_message_chunk", "agent_thought_chunk"] {
            recorder.push(
                &mut agent,
                json!({"sessionUpdate":kind,"content":{"type":"text","text":"act after"}}),
            );
        }
        recorder.push(&mut agent, json!({"sessionUpdate":"compaction_summary_chunk","compactionId":"c","content":{"type":"text","text":"act"}}));
        let order: Vec<_> = values(&agent)
            .iter()
            .map(|v| v["sessionUpdate"].clone())
            .collect();
        recorder.finish(&mut agent);
        assert_eq!(
            text(&agent, "agent_message_chunk"),
            "before [redacted] after"
        );
        assert_eq!(
            text(&agent, "agent_thought_chunk"),
            "before [redacted] after"
        );
        assert_eq!(summary(&agent, "c"), "[redacted]");
        assert_eq!(summary(&agent, "d"), "partial red");
        assert_eq!(
            order,
            values(&agent)
                .iter()
                .map(|v| v["sessionUpdate"].clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn replacing_or_clearing_a_compaction_discards_its_old_matching_tail() {
        let mut agent = turn();
        let mut recorder = Recorder::new("redact");
        recorder.push(&mut agent, json!({"sessionUpdate":"compaction_summary_chunk","compactionId":"c","content":{"type":"text","text":"red"}}));
        recorder.push(&mut agent, json!({"sessionUpdate":"compaction_update","compactionId":"c","summary":[{"type":"text","text":"replacement red"}]}));
        recorder.push(&mut agent, json!({"sessionUpdate":"compaction_summary_chunk","compactionId":"c","content":{"type":"text","text":"act"}}));
        recorder.finish(&mut agent);
        assert_eq!(summary(&agent, "c"), "replacement [redacted]");
        recorder.push(&mut agent, json!({"sessionUpdate":"compaction_summary_chunk","compactionId":"c","content":{"type":"text","text":"red"}}));
        recorder.push(
            &mut agent,
            json!({"sessionUpdate":"compaction_update","compactionId":"c","summary":null}),
        );
        recorder.finish(&mut agent);
        assert!(values(&agent)[0]["summary"].is_null());
    }

    #[test]
    fn truncated_streams_cannot_reappear_when_later_snapshots_free_space() {
        let mut agent = turn();
        let mut recorder = Recorder::new("fixture-split-key");
        recorder.push(&mut agent, json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"safe fixture-"}}));
        recorder.push(&mut agent, json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":format!("split-key{}", "x".repeat(LIMIT))}}));
        recorder.push(
            &mut agent,
            json!({"sessionUpdate":"usage_update","used":1,"size":2}),
        );
        recorder.push(&mut agent, json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"resurrected"}}));
        recorder.finish(&mut agent);
        assert_eq!(text(&agent, "agent_message_chunk"), "safe ");
        assert!(agent.updates_truncated);
        assert!(agent.updates.iter().map(String::len).sum::<usize>() <= LIMIT);
    }

    #[test]
    fn pending_compaction_state_is_bounded_with_the_display_budget() {
        let mut agent = turn();
        let mut recorder = Recorder::new("fixture-split-key");
        for id in 0..COUNT + 10 {
            recorder.push(&mut agent, json!({"sessionUpdate":"compaction_summary_chunk","compactionId":id.to_string(),"content":{"type":"text","text":"fixture-"}}));
        }
        assert!(recorder.streams.len() <= COUNT);
        assert!(recorder.retained_len() <= LIMIT);
        assert!(agent.updates_truncated);
        recorder.finish(&mut agent);
        assert!(agent.updates.iter().map(String::len).sum::<usize>() <= LIMIT);
    }
}
