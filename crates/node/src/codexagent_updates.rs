//! Codex 0.163 app-server items -> ACP v1 updates. No execution or approval here.
use agent_client_protocol::schema::v1::SessionUpdate;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct Updates {
    output: HashMap<String, String>,
    messages: HashSet<String>,
}
fn text(value: &str) -> Value {
    json!({"type":"content","content":{"type":"text","text":value}})
}
fn status(item: &Value, completed: bool) -> &'static str {
    if !item["error"].is_null() || item["exitCode"].as_i64().is_some_and(|n| n != 0) {
        return "failed";
    }
    match item["status"].as_str().unwrap_or("") {
        "failed" | "declined" | "cancelled" | "canceled" => "failed",
        "completed" => "completed",
        _ if completed => "completed",
        _ => "in_progress",
    }
}
impl Updates {
    pub(super) fn take(&mut self, method: &str, params: &Value) -> Vec<SessionUpdate> {
        let mut values = Vec::new();
        match method {
            "item/agentMessage/delta"
            | "item/reasoning/summaryTextDelta"
            | "item/reasoning/textDelta" => {
                if let Some(delta) = params["delta"].as_str() {
                    if method == "item/agentMessage/delta" {
                        if let Some(id) = params["itemId"].as_str() {
                            self.messages.insert(id.into());
                        }
                    }
                    values.push(json!({"sessionUpdate":if method == "item/agentMessage/delta" { "agent_message_chunk" } else { "agent_thought_chunk" },"content":{"type":"text","text":delta}}));
                }
            }
            "thread/tokenUsage/updated" => {
                let usage = &params["tokenUsage"];
                if let (Some(used), Some(size)) = (
                    usage["last"]["totalTokens"].as_u64(),
                    usage["modelContextWindow"].as_u64().filter(|n| *n > 0),
                ) {
                    values.push(json!({"sessionUpdate":"usage_update","used":used,"size":size}));
                }
            }
            "turn/plan/updated" => {
                if let Some(plan) = params["plan"].as_array() {
                    let entries: Vec<Value> = plan.iter().filter_map(|step| Some(json!({"content":step["step"].as_str()?,"priority":"medium","status":match step["status"].as_str()? {"inProgress"=>"in_progress","completed"=>"completed",_=>"pending"}}))).collect();
                    values.push(json!({"sessionUpdate":"plan","entries":entries}));
                }
            }
            "item/commandExecution/outputDelta" | "item/fileChange/outputDelta" => {
                if let (Some(id), Some(delta)) =
                    (params["itemId"].as_str(), params["delta"].as_str())
                {
                    let output = self.output.entry(id.into()).or_default();
                    if output.len() < 96 * 1024 {
                        let mut end = delta.len().min(96 * 1024 - output.len());
                        while !delta.is_char_boundary(end) {
                            end -= 1;
                        }
                        output.push_str(&delta[..end]);
                        if end < delta.len() {
                            output.push_str(khor_catalog::msg::console_output_cut());
                        }
                    }
                    values.push(json!({"sessionUpdate":"tool_call_update","toolCallId":id,"content":[text(output)]}));
                }
            }
            "item/started" | "item/completed" => {
                let item = &params["item"];
                let (Some(id), Some(kind)) = (item["id"].as_str(), item["type"].as_str()) else {
                    return vec![];
                };
                let completed = method == "item/completed";
                if kind == "agentMessage" {
                    if completed && !self.messages.remove(id) {
                        if let Some(body) = item["text"].as_str() {
                            values.push(json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":body}}));
                        }
                    }
                } else if !matches!(kind, "userMessage" | "reasoning" | "") {
                    let title = item["command"]
                        .as_str()
                        .or_else(|| item["title"].as_str())
                        .or_else(|| item["tool"].as_str())
                        .or_else(|| item["name"].as_str())
                        .unwrap_or(kind);
                    let tool_kind = match kind {
                        "commandExecution" => "execute",
                        "fileChange" => "edit",
                        "webSearch" => "search",
                        "imageView" => "read",
                        _ => "other",
                    };
                    let mut content = Vec::new();
                    let partial = if completed {
                        self.output.remove(id)
                    } else {
                        None
                    };
                    if let Some(output) = item["aggregatedOutput"].as_str().or(partial.as_deref()) {
                        content.push(text(output));
                    }
                    if let Some(result) = item["result"]["content"].as_array() {
                        content.extend(result.iter().filter_map(|c| {
                            if c["type"] == "text" {
                                c["text"].as_str().map(text)
                            } else {
                                None
                            }
                        }));
                    }
                    if let Some(error) = item["error"]["message"].as_str() {
                        content.push(text(error));
                    }
                    let locations: Vec<Value> = item["changes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|c| c["path"].as_str().map(|p| json!({"path":p})))
                        .collect();
                    let raw_input = if kind == "commandExecution" {
                        json!({"command":item["command"],"cwd":item["cwd"]})
                    } else {
                        item["arguments"].clone()
                    };
                    values.push(json!({"sessionUpdate":if completed {"tool_call_update"} else {"tool_call"},"toolCallId":id,
                        "title":title,"kind":tool_kind,"status":status(item, completed),"rawInput":raw_input,"rawOutput":item,"content":content,"locations":locations}));
                }
            }
            _ => {}
        }
        values
            .into_iter()
            .filter_map(|value| serde_json::from_value(value).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value(update: SessionUpdate) -> Value {
        serde_json::to_value(update).unwrap()
    }
    #[test]
    fn completes_commands_mcp_and_file_patches_and_preserves_plan() {
        let mut u = Updates::default();
        let start = u.take("item/started", &json!({"item":{"id":"cmd","type":"commandExecution","command":"false","cwd":"/work","status":"inProgress"}}));
        assert_eq!(value(start[0].clone())["kind"], "execute");
        assert_eq!(
            value(
                u.take(
                    "item/commandExecution/outputDelta",
                    &json!({"itemId":"cmd","delta":"one "})
                )
                .remove(0)
            )["content"][0]["content"]["text"],
            "one "
        );
        let done = value(u.take("item/completed", &json!({"item":{"id":"cmd","type":"commandExecution","command":"false","status":"completed","exitCode":1,"aggregatedOutput":"one two"}})).remove(0));
        assert_eq!(done["status"], "failed");
        assert_eq!(done["content"][0]["content"]["text"], "one two");
        let patch = value(u.take("item/completed", &json!({"item":{"id":"edit","type":"fileChange","status":"completed","changes":[{"path":"/work/a","kind":{"type":"update"},"diff":"@@ -12 +12 @@\n-old\n+new"}]}})).remove(0));
        assert_eq!(
            patch["rawOutput"]["changes"][0]["diff"],
            "@@ -12 +12 @@\n-old\n+new"
        );
        assert_eq!(patch["locations"][0]["path"], "/work/a");
        let plan = value(
            u.take(
                "turn/plan/updated",
                &json!({"plan":[{"step":"verify","status":"inProgress"}]}),
            )
            .remove(0),
        );
        assert_eq!(plan["entries"][0]["status"], "in_progress");
        let mcp = value(u.take("item/completed", &json!({"item":{"id":"mcp","type":"mcpToolCall","tool":"search","status":"completed","result":{"content":[{"type":"text","text":"result"}]}}})).remove(0));
        assert_eq!(mcp["content"][0]["content"]["text"], "result");
    }
    #[test]
    fn message_completion_is_not_duplicated_and_unknown_updates_are_ignored() {
        let mut u = Updates::default();
        assert_eq!(
            u.take(
                "item/agentMessage/delta",
                &json!({"itemId":"msg","delta":"hi"})
            )
            .len(),
            1
        );
        assert!(
            u.take(
                "item/completed",
                &json!({"item":{"id":"msg","type":"agentMessage","text":"hi"}})
            )
            .is_empty()
        );
        assert_eq!(
            u.take(
                "item/completed",
                &json!({"item":{"id":"other","type":"agentMessage","text":"fallback"}})
            )
            .len(),
            1
        );
        assert!(u.take("future/notification", &json!({})).is_empty());
    }
}
