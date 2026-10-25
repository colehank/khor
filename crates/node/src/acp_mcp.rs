//! Translate ACP server descriptors at the two native CLI boundaries only.
use agent_client_protocol::schema::v1::McpServer;
use serde_json::{Value, json};
pub(super) fn servers(servers: &[McpServer], codex: bool) -> Result<Value, String> {
    let mut result = serde_json::Map::new();
    for server in servers {
        let v = serde_json::to_value(server).map_err(|e| e.to_string())?;
        let name = v["name"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("MCP server needs a name")?;
        let pairs = |key: &str| -> Value {
            Value::Object(
                v[key]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| Some((v["name"].as_str()?.to_owned(), v["value"].clone())))
                    .collect(),
            )
        };
        let config = match v["type"].as_str() {
            Some("http" | "sse") => {
                if codex && v["type"] == "sse" {
                    return Err("Codex does not support SSE MCP servers".into());
                }
                if codex {
                    json!({"url":v["url"],"http_headers":pairs("headers")})
                } else {
                    json!({"type":v["type"],"url":v["url"],"headers":pairs("headers")})
                }
            }
            _ => json!({"command":v["command"],"args":v["args"],"env":pairs("env")}),
        };
        if result.insert(name.into(), config).is_some() {
            return Err(format!("Duplicate MCP server: {name}"));
        }
    }
    Ok(Value::Object(result))
}
