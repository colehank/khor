import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";

/** ACP v1 session/update payloads; Rust validates these against the pinned SDK.
 * The private credential/MCP pipe remains private and is not an ACP transport.
 */
export function acpUpdates(event: AgentSessionEvent): Record<string, unknown>[] {
  if (event.type === "message_update") {
    const e = event.assistantMessageEvent;
    if (e.type === "text_delta" || e.type === "thinking_delta") return [{
      sessionUpdate: e.type === "text_delta" ? "agent_message_chunk" : "agent_thought_chunk",
      content: { type: "text", text: e.delta },
    }];
  }
  if (event.type === "tool_execution_start") return [{
    sessionUpdate: "tool_call", toolCallId: event.toolCallId, title: event.toolName,
    kind: toolKind(event.toolName), status: "in_progress", rawInput: event.args,
  }];
  if (event.type === "tool_execution_update" || event.type === "tool_execution_end") {
    const result = event.type === "tool_execution_end" ? event.result : event.partialResult;
    const content = (Array.isArray(result?.content) ? result.content : []).filter((c: { type?: string; text?: unknown }) => c.type === "text" && typeof c.text === "string")
      .map((c: { text: string }) => ({ type: "content", content: { type: "text", text: c.text } }));
    return [{ sessionUpdate: "tool_call_update", toolCallId: event.toolCallId,
      status: event.type === "tool_execution_end" ? event.isError ? "failed" : "completed" : "in_progress",
      rawOutput: result ?? null, content }];
  }
  return [];
}

function toolKind(name: string): string {
  const plain = name.replace(/^mcp__khor__/, "");
  if (["devices", "sessions", "files", "read", "agents", "task_status", "task_wait", "task_list"].includes(plain)) return "read";
  if (["search", "find"].includes(plain)) return "search";
  return "other";
}
