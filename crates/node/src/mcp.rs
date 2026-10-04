//! khor's verbs, offered to an agent as tools (docs/AGENT.md).
//!
//! # The Khor agent is not a new shape
//!
//! It is one conversation session whose agent can *see and move the
//! network*: the same ACP host, the same six words, the same 待批 —
//! what makes it the scheduler is this file, which hands claude the
//! verbs a person would have typed. Nothing here decides anything; it
//! translates one protocol into another and lets `Node` answer.
//!
//! # Why the protocol is written out rather than pulled in
//!
//! MCP over stdio is JSON-RPC 2.0 with four messages that matter:
//! `initialize`, the `notifications/initialized` that follows it,
//! `tools/list`, and `tools/call`. That is small enough to read in one
//! sitting, and khor already carries a JSON codec for the ACP side —
//! whereas an SDK would bring its own runtime opinions into a binary
//! whose whole install story is "one file, no dependencies".
//!
//! # Reading is free, moving is not
//!
//! The permission split is **not** enforced here, and that is
//! deliberate: an agent that could grant itself permission is not
//! bounded by anything this file writes. The gate is the vendor's own
//! (`--permission-mode`, surfaced through the shim as 待批 on the
//! conversation face), which is the same gate a person answers for any
//! other tool. What this file owes that gate is an honest **name** —
//! `open`, `close` and `tell` say what they do in their titles, so the
//! question a person is asked is a question they can answer.

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
            "tools/list" => json!({
                "tools": TOOLS.iter().map(|t| json!({
                    "name": t.name,
                    "description": (t.about)(),
                    "inputSchema": (t.schema)(),
                })).collect::<Vec<_>>()
            }),
            "tools/call" => {
                let name = req["params"]["name"].as_str().unwrap_or("");
                let args = req["params"]["arguments"].clone();
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
fn call(root: &std::path::Path, name: &str, args: &Value) -> Result<String, String> {
    let s = |k: &str| {
        args.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned()
    };
    let n = Node::open(root.to_path_buf())?;
    match name {
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
        "close" => {
            let rt = runtime()?;
            rt.block_on(n.close_anywhere(&crate::SessionId(s("session"))))?;
            Ok(msg::mcp_closed().into())
        }
        "tell" => {
            n.tell(&s("machine"), &s("text"))?;
            Ok(msg::mcp_told().into())
        }
        other => Err(msg::unknown_action(other)),
    }
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
        for acting in ["open", "close", "tell"] {
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
