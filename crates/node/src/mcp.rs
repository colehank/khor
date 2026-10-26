//! khor's verbs, offered to an agent as tools (docs/AGENT.md).
//!
//! The same explicit tool directory backs the external stdio server and the
//! built-in Pi agent. Both validate arguments and call the existing Node APIs.
//! Rust's agent host owns approval for its writes; external MCP clients retain
//! their own approval policy. Tool metadata describes this split but never
//! grants permission by itself.

use khor_catalog::msg;
use serde_json::{Value, json};

use crate::Node;

/// The protocol version this server speaks. Sent back verbatim in the
/// initialize result: a client that asked for something else learns it
/// from the answer rather than from a silence.
const PROTOCOL: &str = "2024-11-05";

/// One tool: the name claude sees, what it is for, and its arguments.
///
/// **What it is for is the catalog's** (`[msg] mcp-*`): a model reads it
/// and says it back to a person, in the language khor is speaking — so
/// it is a word like any other, looked up at the moment it is sent.
struct Tool {
    name: &'static str,
    about: fn() -> &'static str,
    /// Looking, not moving (docs/AGENT.md: 读免批). The split is
    /// declared **here**, beside the thing it describes, and the
    /// allowlist handed to the vendor is generated from it — a second
    /// list of names would be a second place to forget one, and the
    /// forgotten direction is the dangerous one.
    reads: bool,
    /// JSON Schema for the arguments. Kept literal rather than derived:
    /// the schema *is* the interface, and a generated one drifts from
    /// the prose beside it without anybody noticing.
    schema: fn() -> Value,
}

fn no_args() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

fn task_schema(extra: &[&str], required_extra: &[&str]) -> Value {
    let mut properties = serde_json::Map::new();
    for key in ["machine", "agent", "task"]
        .into_iter()
        .chain(extra.iter().copied())
    {
        properties.insert(key.into(), json!({"type":"string"}));
    }
    let required: Vec<_> = ["agent", "task"]
        .into_iter()
        .chain(required_extra.iter().copied())
        .collect();
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

const TOOLS: &[Tool] = &[
    Tool {
        name: "devices",
        reads: true,
        about: msg::mcp_devices,
        schema: no_args,
    },
    Tool {
        name: "sessions",
        reads: true,
        about: msg::mcp_sessions,
        schema: no_args,
    },
    Tool {
        name: "ls",
        reads: true,
        about: msg::mcp_ls,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "machine": { "type": "string", "description": msg::mcp_ls_machine() },
                    "path": { "type": "string", "description": msg::mcp_ls_path() }
                },
                "required": ["machine"],
                "additionalProperties": false
            })
        },
    },
    Tool {
        name: "open",
        reads: false,
        about: msg::mcp_open,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "machine": { "type": "string", "description": msg::mcp_open_machine() },
                    "command": { "type": "string", "description": msg::mcp_open_command() },
                    "title": { "type": "string", "description": msg::mcp_open_title() },
                    "cwd": { "type": "string", "description": msg::mcp_open_cwd() }
                },
                "required": ["machine", "command"],
                "additionalProperties": false
            })
        },
    },
    Tool {
        name: "read",
        reads: true,
        about: msg::mcp_read,
        schema: || json!({"type":"object","properties":{"machine":{"type":"string"},"path":{"type":"string"}},"required":["machine","path"],"additionalProperties":false}),
    },
    Tool {
        name: "search",
        reads: true,
        about: msg::mcp_search,
        schema: || json!({"type":"object","properties":{"machine":{"type":"string"},"path":{"type":"string"},"query":{"type":"string"}},"required":["machine","path","query"],"additionalProperties":false}),
    },
    Tool {
        name: "close",
        reads: false,
        about: msg::mcp_close,
        schema: || {
            json!({
                "type": "object",
                "properties": { "session": { "type": "string", "description": msg::mcp_close_session() } },
                "required": ["session"],
                "additionalProperties": false
            })
        },
    },
    Tool {
        name: "tell",
        reads: false,
        about: msg::mcp_tell,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "machine": { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["machine", "text"],
                "additionalProperties": false
            })
        },
    },
    Tool {
        name: "agents",
        reads: true,
        about: msg::mcp_agents,
        schema: || json!({"type":"object","properties":{"machine":{"type":"string"}},"additionalProperties":false}),
    },
    Tool {
        name: "agent_connect", reads:false, about:msg::mcp_agent_connect,
        schema: || json!({"type":"object","properties":{"machine":{"type":"string"},"agent":{"type":"string"},"url":{"type":"string"}},"required":["agent","url"],"additionalProperties":false}),
    },
    Tool {
        name: "agent_disconnect", reads:false, about:msg::mcp_agent_disconnect,
        schema: || json!({"type":"object","properties":{"machine":{"type":"string"},"agent":{"type":"string"}},"required":["agent"],"additionalProperties":false}),
    },
    Tool {
        name: "delegate",
        reads: false,
        about: msg::mcp_delegate,
        schema: || {
            json!({"type":"object","properties":{
            "machine":{"type":"string"},"agent":{"type":"string"},"prompt":{"type":"string"},
            "request":{"type":"string","description":"Stable unique message ID. Reuse exactly this ID and identical arguments after an uncertain response."},
            "context":{"type":"string","description":"Previously returned context ID for the same agent."},
            "task":{"type":"string","description":"A2A task ID awaiting input, to answer the same task. Omit for a new task. ACP follow-ups use context only."},
            "cwd":{"type":"string","description":"Existing absolute working directory on the target machine, for a new context only."}
        },"required":["agent","prompt","request"],"additionalProperties":false})
        },
    },
    Tool {
        name: "task_status",
        reads: true,
        about: msg::mcp_task_status,
        schema: || task_schema(&[], &[]),
    },
    Tool {
        name: "task_wait",
        reads: true,
        about: msg::mcp_task_wait,
        schema: || task_schema(&["revision"], &["revision"]),
    },
    Tool {
        name: "task_cancel",
        reads: false,
        about: msg::mcp_task_cancel,
        schema: || task_schema(&[], &[]),
    },
    Tool {
        name: "task_answer",
        reads: false,
        about: msg::mcp_task_answer,
        schema: || task_schema(&["ask", "option"], &["ask"]),
    },
    Tool {
        name: "task_list",
        reads: true,
        about: msg::mcp_task_list,
        schema: || {
            json!({"type":"object","properties":{
        "machine":{"type":"string"},"agent":{"type":"string"},"context":{"type":"string"},"after":{"type":"string"}
    },"required":["agent"],"additionalProperties":false})
        },
    },
];

/// The tools a scheduler may use without being asked (docs/AGENT.md:
/// 看一律免批). Spelled the way the vendor names them — `mcp__<server>__
/// <tool>` — because that is what its allowlist matches on.
pub fn free_tools() -> Vec<String> {
    TOOLS
        .iter()
        .filter(|t| t.reads)
        .map(|t| format!("mcp__khor__{}", t.name))
        .collect()
}

/// Runs the server until stdin closes. One line in, one line out — the
/// framing MCP's stdio transport specifies.
pub fn serve_stdio(root: std::path::PathBuf) -> Result<(), String> {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        // A notification carries no id and takes no answer. Replying to
        // one is how a client ends up waiting for a response to a
        // message it never asked a question with.
        let Some(id) = req.get("id").cloned() else {
            continue;
        };
        let method = req["method"].as_str().unwrap_or("");
        let reply = match method {
            "initialize" => json!({
                "protocolVersion": PROTOCOL,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "khor", "version": env!("CARGO_PKG_VERSION") }
            }),
            "tools/list" => tools(),
            "tools/call" => {
                let name = req["params"]["name"].as_str().unwrap_or("");
                let args = req["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                match call(&root, name, &args) {
                    Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
                    // An error inside a tool call is **not** a protocol
                    // error: the model is meant to read it and try
                    // something else, which it cannot do if the
                    // transport swallows the sentence.
                    Err(why) => json!({
                        "content": [{ "type": "text", "text": why }],
                        "isError": true
                    }),
                }
            }
            "ping" => json!({}),
            other => {
                let frame = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": msg::unknown_action(other) }
                });
                writeln!(out, "{frame}").map_err(|e| e.to_string())?;
                out.flush().map_err(|e| e.to_string())?;
                continue;
            }
        };
        let frame = json!({ "jsonrpc": "2.0", "id": id, "result": reply });
        writeln!(out, "{frame}").map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// One tool call, answered as the text the model will read.
///
/// **Plain text, not JSON.** What comes back is read by a language
/// model and quoted to a person, so it is the same lines `khor` prints
/// in a terminal — one source for what a session row says, whoever is
/// looking.
pub(crate) fn call(root: &std::path::Path, name: &str, args: &Value) -> Result<String, String> {
    call_with_parent(root, name, args, None)
}

pub(crate) fn call_with_parent(
    root: &std::path::Path,
    name: &str,
    args: &Value,
    parent: Option<crate::delegation::TaskRef>,
) -> Result<String, String> {
    validate(name, args)?;
    let s = |k: &str| {
        args.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned()
    };
    let n = Node::open(root.to_path_buf())?;
    match name {
        "agents" | "agent_connect" | "agent_disconnect" | "delegate" | "task_status" | "task_wait" | "task_cancel" | "task_answer"
        | "task_list" => {
            use crate::delegation::{
                Action,
                protocol::{Message, Part, Role},
            };
            let optional = |key: &str| {
                let value = s(key);
                if value.is_empty() { None } else { Some(value) }
            };
            let action = match name {
                "agents" => Action::Agents,
                "agent_connect" => Action::Connect {agent:s("agent"),url:s("url")},
                "agent_disconnect" => Action::Disconnect {agent:s("agent")},
                "delegate" => {
                    let mut message = Message::new(Role::User, vec![Part::text(s("prompt"))]);
                    message.message_id = s("request");
                    message.context_id = optional("context");
                    message.task_id = optional("task");
                    if let Some(cwd) = optional("cwd") {
                        message.metadata = Some([("khor.cwd".into(), json!(cwd))].into());
                    }
                    if let Some(parent) = &parent {
                        crate::delegation::track_child(
                            root,
                            parent,
                            &s("machine"),
                            &s("agent"),
                            &message.message_id,
                            None,
                        )?;
                    }
                    Action::Send {
                        agent: s("agent"),
                        message,
                        parent: parent.as_ref().map(|parent| crate::delegation::original_parent(root, parent, &s("machine"), &s("agent"), &s("request"))).transpose()?,
                    }
                }
                "task_status" => Action::Get {
                    agent: s("agent"),
                    task: s("task"),
                },
                "task_wait" => Action::Wait {
                    agent: s("agent"),
                    task: s("task"),
                    revision: s("revision")
                        .parse()
                        .map_err(|_| msg::agent_arguments().to_owned())?,
                },
                "task_cancel" => Action::Cancel {
                    agent: s("agent"),
                    task: s("task"),
                },
                "task_answer" => {
                    let ask = s("ask")
                        .parse()
                        .map_err(|_| msg::agent_arguments().to_owned())?;
                    if let Some(parent) = &parent {
                        // Answer can resume an already-known task before its
                        // reply arrives. Retiring this worker must not erase
                        // the parent's ability to find/cancel that work.
                        crate::delegation::track_child(
                            root,
                            parent,
                            &s("machine"),
                            &s("agent"),
                            &format!("task:{}", s("task")),
                            Some(&s("task")),
                        )?;
                    }
                    Action::Answer {
                        agent: s("agent"),
                        task: s("task"),
                        ask,
                        option: optional("option"),
                    }
                },
                "task_list" => Action::List {
                    agent: s("agent"),
                    context: optional("context"),
                    after: optional("after"),
                },
                _ => unreachable!(),
            };
            let answer = runtime()?.block_on(n.delegation_on(&s("machine"), action))?;
            if name == "delegate" || name == "task_answer" {
                if let (Some(parent), crate::delegation::Answer::Task { record }) =
                    (&parent, &answer)
                {
                    crate::delegation::track_child(
                        root,
                        parent,
                        &s("machine"),
                        &s("agent"),
                        &if name == "delegate" { s("request") } else { format!("task:{}", record.task.id) },
                        Some(&record.task.id),
                    )?;
                }
            }
            // Canonical task JSON includes task/context IDs, revision, partial
            // output and pending permission; the caller can resume or cancel it.
            serde_json::to_string(&answer).map_err(|e| e.to_string())
        }
        "devices" => {
            let mut out = String::new();
            for d in n.devices()? {
                // By id, not by name: two machines can share a
                // hostname, and then the name test marks both rows
                // 「(本机)」 — the one thing on this line a reader uses
                // to know where they are (`khor devices` says the same).
                // The catalog's word, the same one `khor devices` prints:
                // a literal here drifted from it once (half-width
                // brackets after the catalog went full-width).
                let here = if d.id == n.device_str() {
                    khor_catalog::cli::this_machine()
                } else {
                    ""
                };
                out.push_str(&format!("{}\t{}…{here}\n", d.shown(), &d.id[..16]));
            }
            Ok(out)
        }
        "sessions" => {
            let mut out = String::new();
            for v in n.sessions()? {
                let where_ = v
                    .source
                    .map(|(m, _)| m)
                    .unwrap_or_else(|| n.name().to_owned());
                out.push_str(&format!(
                    "{}\t{}\t{}\t{}\n",
                    v.session.id.0,
                    v.session.state.state.key(),
                    v.session.title,
                    where_
                ));
            }
            Ok(out)
        }
        "ls" => {
            let rt = runtime()?;
            let (at, entries, truncated) = rt.block_on(n.ls_of(&s("machine"), &s("path")))?;
            let mut out = format!("{at}\n");
            for e in entries {
                out.push_str(&format!("{}{}\n", e.name, if e.dir { "/" } else { "" }));
            }
            if truncated {
                // The no-silent-caps rule reaches the model too: a
                // listing that was cut must not read as a whole one.
                out.push_str(msg::mcp_listing_cut());
                out.push('\n');
            }
            Ok(out)
        }
        "open" => {
            let command = s("command");
            if command.is_empty() {
                return Err(msg::mcp_open_needs_command().into());
            }
            let title = if s("title").is_empty() {
                command.clone()
            } else {
                s("title")
            };
            let rt = runtime()?;
            let id = rt.block_on(n.open_on(
                &s("machine"),
                khor_core::kind::SHELL,
                &title,
                &s("cwd"),
                &["sh".into(), "-c".into(), command],
                (80, 24),
                None,
            ))?;
            Ok(id.0)
        }
        "read" => {
            let (total, _, bytes, truncated) =
                runtime()?.block_on(n.read_all(&s("machine"), &s("path"), 48 * 1024))?;
            let text = decode_text(&bytes, truncated)?;
            Ok(json!({"text":text,"total_bytes":total,"truncated":truncated}).to_string())
        }
        "search" => {
            let found = runtime()?.block_on(n.search(&s("machine"), &s("path"), &s("query")))?;
            serde_json::to_string(&found).map_err(|e| e.to_string())
        }
        "close" => {
            let rt = runtime()?;
            rt.block_on(n.close_anywhere(&crate::SessionId(s("session"))))?;
            Ok(msg::mcp_closed().into())
        }
        "tell" => {
            let typed = crate::post::addressed(&s("machine"), &s("text"));
            runtime()?.block_on(n.post(&typed, &[]))?;
            Ok(msg::mcp_told().into())
        }
        other => Err(msg::unknown_action(other)),
    }
}

fn decode_text(bytes: &[u8], truncated: bool) -> Result<&str, String> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(error) if truncated && error.error_len().is_none() => {
            std::str::from_utf8(&bytes[..error.valid_up_to()])
                .map_err(|_| msg::agent_text_file().to_owned())
        }
        Err(_) => Err(msg::agent_text_file().to_owned()),
    }
}

/// The same catalogue and dispatcher are used by external MCP clients and
/// khor's private Pi bridge. Tool metadata never grants permission by itself.
pub(crate) fn tools() -> Value {
    json!({"tools": TOOLS.iter().map(|t| json!({
        "name": t.name, "description": (t.about)(), "inputSchema": (t.schema)(),
        "annotations": {"readOnlyHint": t.reads}
    })).collect::<Vec<_>>()})
}

pub(crate) fn reads(name: &str) -> Option<bool> {
    TOOLS.iter().find(|t| t.name == name).map(|t| t.reads)
}

pub(crate) fn validate(name: &str, args: &Value) -> Result<(), String> {
    let tool = TOOLS
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| msg::unknown_action(name))?;
    let schema = (tool.schema)();
    let fields = args
        .as_object()
        .ok_or_else(|| msg::agent_arguments().to_owned())?;
    let properties = schema["properties"]
        .as_object()
        .expect("tool schema properties");
    if fields
        .iter()
        .any(|(key, value)| !properties.contains_key(key) || !value.is_string())
    {
        return Err(msg::agent_arguments().to_owned());
    }
    if schema["required"].as_array().is_some_and(|required| {
        required.iter().filter_map(Value::as_str).any(|key| {
            fields
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(|s| s.trim().is_empty())
        })
    }) {
        return Err(msg::agent_arguments().to_owned());
    }
    Ok(())
}

/// One runtime per call, on purpose: this server is a thin translator
/// and its calls are seconds apart, so a resident runtime would be a
/// thread pool asleep for the life of a conversation.
fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(msg::runtime_wont_start)
}

#[cfg(test)]
mod tests {
    #[test]
    fn tool_arguments_are_checked_before_execution() {
        use serde_json::json;
        assert!(super::validate("devices", &json!({})).is_ok());
        assert!(super::validate("devices", &json!({"surprise":"value"})).is_err());
        assert!(super::validate("open", &json!({"machine":"lab","command":42})).is_err());
        assert!(super::validate("open", &json!({"machine":"","command":"echo"})).is_err());
        assert!(super::validate("_host", &json!({})).is_err());
        assert!(super::validate("agent_connect", &json!({"agent":"local","url":"http://localhost:9000"})).is_ok());
        assert!(super::validate("agent_connect", &json!({"agent":"local","url":"http://localhost:9000","token":"secret"})).is_err());
        assert!(super::validate("delegate", &json!({"agent":"a2a:local","prompt":"reply","request":"unique","task":"task-id","context":"context-id"})).is_ok());
    }

    #[test]
    fn a_capped_text_file_may_end_inside_a_unicode_character() {
        let text = "ab界界";
        assert_eq!(
            super::decode_text(&text.as_bytes()[..6], true).unwrap(),
            "ab界"
        );
        assert!(super::decode_text(&text.as_bytes()[..6], false).is_err());
        assert!(super::decode_text(&[0xff, 0xfe], true).is_err());
    }
    /// **The free list is generated, and it is the reading half.**
    /// Two lists of tool names would be two places to forget one, and
    /// the direction that gets forgotten is the dangerous one: a
    /// missing name on the acting side is a gate that stopped asking.
    #[test]
    fn only_the_looking_tools_are_free() {
        let free = super::free_tools();
        assert!(free.contains(&"mcp__khor__devices".to_string()));
        assert!(free.contains(&"mcp__khor__sessions".to_string()));
        assert!(free.contains(&"mcp__khor__ls".to_string()));
        for acting in [
            "open",
            "close",
            "tell",
            "agent_connect",
            "agent_disconnect",
            "delegate",
            "task_answer",
            "task_cancel",
        ] {
            assert!(
                !free.contains(&format!("mcp__khor__{acting}")),
                "{acting} moves something on somebody's machine and must be asked about"
            );
        }
        assert_eq!(free.len(), super::TOOLS.iter().filter(|t| t.reads).count());
    }

    /// **The words are the catalog's** (`[msg] mcp-*`, both files —
    /// `khor_catalog`'s build refuses a key only one of them has). What
    /// is left to hold here is that no tool reads as nothing. The
    /// language is not switched in this test: it is process-wide, and
    /// the other unit tests read Chinese while this one runs
    /// (`tests/lang.rs` switches it, in a process of its own).
    #[test]
    fn every_tool_says_what_it_is_for() {
        for t in super::TOOLS {
            assert!(!(t.about)().trim().is_empty(), "{} says nothing", t.name);
        }
    }

    /// The names the vendor matches on are `mcp__<server>__<tool>`, and
    /// the server half is `khor` because that is what the config file
    /// the shim writes calls it. Spelled in two places by necessity —
    /// so it is asserted in one.
    #[test]
    fn the_free_names_are_spelled_the_way_the_vendor_matches() {
        for name in super::free_tools() {
            assert!(name.starts_with("mcp__khor__"), "{name}");
        }
    }
}
