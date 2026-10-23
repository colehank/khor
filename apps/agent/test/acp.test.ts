import { test } from "node:test";
import assert from "node:assert/strict";
import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";
import { acpUpdates } from "../src/acp.ts";

const updates = (event: unknown) => acpUpdates(event as AgentSessionEvent);
test("Pi presentation preserves thought, tool identity, progress and failure", () => {
  assert.deepEqual(updates({ type: "message_update", assistantMessageEvent: { type: "thinking_delta", delta: "checking" } }), [{ sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "checking" } }]);
  const start = updates({ type: "tool_execution_start", toolCallId: "tool-1", toolName: "mcp__khor__devices", args: { machine: "peer" } })[0]!;
  assert.equal(start.toolCallId, "tool-1"); assert.equal(start.kind, "read"); assert.equal(start.status, "in_progress");
  for (const [type, status] of [["tool_execution_update", "in_progress"], ["tool_execution_end", "failed"]]) {
    const result = { content: [{ type: "text", text: "one" }, { type: "text", text: "two" }] };
    const event = updates({ type, toolCallId: "tool-1", partialResult: result, result, isError: true })[0]!;
    assert.equal(event.toolCallId, "tool-1"); assert.equal(event.status, status);
    assert.deepEqual(event.content, result.content.map((content) => ({ type: "content", content })));
  }
  assert.deepEqual(updates({ type: "agent_settled" }), []);
});
