//! DeepSeek Harness's conversation events, as utterances.
//!
//! DSH writes JSON lines: a `session` header, then `user/message`s,
//! `assistant/message`s and `tool/result`s (with `tool/call` events that
//! repeat what the assistant message already said — those are skipped, or
//! every call would be painted twice).
//!
//! The shapes this translates from:
//!
//! ```text
//! {"type":"user/message","data":{"content":[{"type":"text","text":…}]}}
//! {"type":"assistant/message","data":{"message":{"content":[
//!     {"type":"reasoning","text":…},{"type":"text","text":…},
//!     {"type":"tool-call","id":…,"name":…,"arguments":"<json string>"}]}}}
//! {"type":"tool/result","data":{"toolCallId":…,"message":{"content":[
//!     {"type":"text","text":…}]}}}
//! ```
//!
//! **The blocks leave here in claude's shape**, not DSH's: `tool-call`
//! becomes claude's `tool_use` block and a result becomes a `tool_result`
//! block. That is deliberate — [`crate::cagent::said`] and
//! [`crate::cagent::settled`] are the one translation from a vendor block
//! to the protocol's update, and a second translation per vendor is how a
//! replay once lost tool titles and statuses (see `Utterance`'s own note).
//! The arguments string is parsed into an object for the same reason: the
//! title, the locations and the diff all read object fields.
//!
//! # What is skipped
//!
//! `session`, `session/title`, `step/*`, `turn/*`, `job/*`, `sandbox/*`,
//! `approval/*`, `agent/inbox/*`, `request/*`, `subagent/*` and the
//! `tool/call` duplicates: bookkeeping and machines talking to themselves.
//! A person's replay shows what was said and what was run.

use super::claude::Utterance;

/// DSH's own JSONL in, utterances out.
pub fn utterances_of(text: &str) -> Vec<Utterance> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            // A line this cannot read is one line: DSH's format grows, and
            // dropping the whole transcript over a field it added would
            // lose the conversation to keep the reader simple.
            continue;
        };
        match event.get("type").and_then(|t| t.as_str()) {
            Some("user/message") => {
                if let Some(blocks) = event.pointer("/data/content").and_then(|c| c.as_array()) {
                    for block in blocks {
                        if let Some(text) = text_of(block) {
                            out.push(Utterance::User(text));
                        }
                    }
                }
            }
            Some("assistant/message") => {
                if let Some(blocks) = event
                    .pointer("/data/message/content")
                    .and_then(|c| c.as_array())
                {
                    for block in blocks {
                        match block.get("type").and_then(|t| t.as_str()) {
                            Some("text") => {
                                if let Some(text) = text_of(block) {
                                    out.push(Utterance::Agent(text));
                                }
                            }
                            Some("reasoning") => {
                                // An empty reasoning block is DSH's replay
                                // placeholder, not something that was
                                // thought.
                                if let Some(text) = text_of(block).filter(|t| !t.trim().is_empty())
                                {
                                    out.push(Utterance::Thought(text));
                                }
                            }
                            Some("tool-call") => {
                                out.push(Utterance::Tool(tool_use(block)));
                            }
                            _ => {}
                        }
                    }
                }
            }
            Some("tool/result") => {
                let call = event
                    .pointer("/data/toolCallId")
                    .and_then(|c| c.as_str())
                    .unwrap_or("tool");
                let content = event
                    .pointer("/data/message/content")
                    .and_then(|c| c.as_array())
                    .map(|blocks| {
                        blocks
                            .iter()
                            .filter_map(|block| text_of(block))
                            .map(|text| serde_json::json!({ "type": "text", "text": text }))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let failed = event
                    .pointer("/data/isError")
                    .and_then(|e| e.as_bool())
                    .or_else(|| {
                        event
                            .pointer("/data/message/isError")
                            .and_then(|e| e.as_bool())
                    })
                    .unwrap_or(false);
                out.push(Utterance::ToolDone(serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": call,
                    "content": content,
                    "is_error": failed,
                })));
            }
            // `tool/call` is the assistant message's own block, said again.
            _ => {}
        }
    }
    out
}

/// The text a DSH content block carries, if it carries any.
fn text_of(block: &serde_json::Value) -> Option<String> {
    block
        .get("text")
        .and_then(|t| t.as_str())
        .map(str::to_owned)
        .filter(|text| !text.is_empty())
}

/// One DSH `tool-call` block as claude's `tool_use`.
///
/// `arguments` is a JSON **string** in DSH's events (`"{\"file_path\":…}"`);
/// the protocol's readers want the object, so it is parsed here. A string
/// that is not JSON is kept under `arguments` rather than dropped: a tool
/// that ran with odd arguments still ran.
fn tool_use(block: &serde_json::Value) -> serde_json::Value {
    let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("tool");
    let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("tool");
    let input = match block.get("arguments") {
        Some(serde_json::Value::String(raw)) => serde_json::from_str::<serde_json::Value>(raw)
            .unwrap_or_else(|_| serde_json::json!({ "arguments": raw })),
        Some(value) if value.is_object() => value.clone(),
        _ => serde_json::json!({}),
    };
    serde_json::json!({
        "type": "tool_use",
        "id": id,
        "name": name,
        "input": input,
    })
}

/// One line of a conversation, as a reader shows it.
///
/// The wire shape is deliberately flat: the roster and the CLI only need
/// to say who spoke and what was said. Rendering tool calls richly is the
/// protocol's job (`cagent::said`), not this one.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Line {
    /// `user`, `assistant`, `thought`, `tool` or `tool-result`.
    pub who: &'static str,
    pub text: String,
}

/// The longest single line this hands out. A tool result can be a whole
/// file; a reader that wants all of it opens the conversation, not a row.
const LINE: usize = 8 * 1024;

/// The conversation as lines to show, out of the same utterances.
pub fn lines_of(text: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for utterance in utterances_of(text) {
        let line = match utterance {
            Utterance::User(text) => Line { who: "user", text },
            Utterance::Agent(text) => Line {
                who: "assistant",
                text,
            },
            Utterance::Thought(text) => Line {
                who: "thought",
                text,
            },
            Utterance::Tool(block) => Line {
                who: "tool",
                text: tool_line(&block),
            },
            Utterance::ToolDone(block) => Line {
                who: "tool-result",
                text: result_line(&block),
            },
            // Only claude's rich `Content` blocks, which DSH never makes.
            Utterance::Content { .. } => continue,
        };
        out.push(Line {
            who: line.who,
            text: clamped(line.text),
        });
    }
    out
}

fn clamped(text: String) -> String {
    if text.len() <= LINE {
        return text;
    }
    let mut cut = LINE;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = text[..cut].to_owned();
    out.push('…');
    out
}

/// One tool call as `name {input}` — the name alone is the one thing every
/// call of a kind has in common (`Utterance::Tool`'s own note).
fn tool_line(block: &serde_json::Value) -> String {
    let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("tool");
    let input = block
        .get("input")
        .map(|input| serde_json::to_string(input).unwrap_or_default())
        .unwrap_or_default();
    if input.is_empty() {
        name.to_owned()
    } else {
        format!("{name} {input}")
    }
}

/// A finished tool's text, as its result blocks carry it.
fn result_line(block: &serde_json::Value) -> String {
    let mut parts = Vec::new();
    if let Some(blocks) = block.get("content").and_then(|c| c.as_array()) {
        for part in blocks {
            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                parts.push(text.to_owned());
            }
        }
    }
    let mut said = parts.join("\n");
    if block
        .get("is_error")
        .and_then(|e| e.as_bool())
        .unwrap_or(false)
        && !said.is_empty()
    {
        said.insert_str(0, "failed: ");
    }
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: &str = r#"{"type":"user/message","seq":8,"time":1,"data":{"content":[{"type":"text","text":"read the doc"}]}}"#;
    const ASSISTANT: &str = r#"{"type":"assistant/message","seq":19,"time":2,"data":{"turn":1,"step":1,"message":{"role":"assistant","content":[{"type":"reasoning","text":"plan first"},{"type":"text","text":"I'll read it."},{"type":"tool-call","id":"call_1","name":"read","arguments":"{\"file_path\": \"F:\\\\doc.md\"}"}]}}}"#;
    const RESULT: &str = r#"{"type":"tool/result","seq":21,"time":3,"data":{"toolCallId":"call_1","message":{"role":"tool","content":[{"type":"text","text":"<path>doc.md</path>"}]}}}"#;
    const FAILED_RESULT: &str = r#"{"type":"tool/result","seq":22,"time":4,"data":{"toolCallId":"call_2","isError":true,"message":{"content":[{"type":"text","text":"no such file"}]}}}"#;
    const PLUMBING: &str = r#"{"type":"step/start","seq":1,"data":{"turn":1,"step":1}}"#;

    /// A conversation comes out in speaking order, with reasoning as
    /// thought — an empty reasoning block is DSH's replay placeholder and
    /// is not painted.
    #[test]
    fn a_conversation_comes_out_in_order() {
        let said = utterances_of(&format!(
            "{PLUMBING}\n{USER}\n{ASSISTANT}\n{PLUMBING}\n{{\"type\":\"assistant/message\",\"data\":{{\"message\":{{\"content\":[{{\"type\":\"reasoning\",\"text\":\"\"}}]}}}}}}"
        ));
        assert_eq!(
            said,
            vec![
                Utterance::User("read the doc".into()),
                Utterance::Thought("plan first".into()),
                Utterance::Agent("I'll read it.".into()),
                Utterance::Tool(serde_json::json!({
                    "type": "tool_use",
                    "id": "call_1",
                    "name": "read",
                    "input": { "file_path": "F:\\doc.md" },
                })),
            ]
        );
    }

    /// The tool's arguments arrive as a JSON *string* and leave as the
    /// object the protocol's readers want — this is what lets a replay
    /// show the file a tool touched instead of the word `read`.
    #[test]
    fn a_tool_call_is_claude_tool_use_with_parsed_arguments() {
        let Utterance::Tool(block) = &utterances_of(ASSISTANT)[2] else {
            panic!("third utterance is the tool call");
        };
        assert_eq!(block["type"], "tool_use");
        assert_eq!(block["input"]["file_path"], "F:\\doc.md");
    }

    /// A tool finishing is a `tool_result` block the protocol already
    /// knows how to settle, failure flag included.
    #[test]
    fn a_tool_result_is_claude_tool_result() {
        let said = utterances_of(&format!("{RESULT}\n{FAILED_RESULT}"));
        assert_eq!(
            said,
            vec![
                Utterance::ToolDone(serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": "call_1",
                    "content": [{ "type": "text", "text": "<path>doc.md</path>" }],
                    "is_error": false,
                })),
                Utterance::ToolDone(serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": "call_2",
                    "content": [{ "type": "text", "text": "no such file" }],
                    "is_error": true,
                })),
            ]
        );
    }

    /// `tool/call` repeats the assistant message's own block; painting both
    /// would draw every tool twice.
    #[test]
    fn the_duplicated_tool_call_event_is_skipped() {
        let duplicate = r#"{"type":"tool/call","seq":20,"data":{"callId":"call_1","name":"read","arguments":"{}"}}"#;
        assert!(utterances_of(duplicate).is_empty());
    }

    /// A line this cannot read loses one line, not the conversation.
    #[test]
    fn a_corrupt_line_does_not_lose_the_rest() {
        let said = utterances_of(&format!("not json\n{USER}"));
        assert_eq!(said, vec![Utterance::User("read the doc".into())]);
    }
}
