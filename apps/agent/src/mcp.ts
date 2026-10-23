/** MCP travels over the private host pipe; Rust owns tool authorization. */
import { McpClient, toLlmContent, type JsonRpcMessage, type McpTransport, type McpTransportMessageListener, type McpTransportErrorListener, type McpTransportCloseListener } from "@earendil-works/pi-mcp";
import type { ToolDefinition } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import type { JsonValue } from "@earendil-works/pi-ai";

export class HostTransport implements McpTransport {
  private messages = new Set<McpTransportMessageListener>();
  private errors = new Set<McpTransportErrorListener>();
  private closes = new Set<McpTransportCloseListener>();
  private write: (message: JsonRpcMessage) => void;
  constructor(write: (message: JsonRpcMessage) => void) { this.write = write; }
  async start() {}
  async send(message: JsonRpcMessage, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    this.write(message);
  }
  receive(message: JsonRpcMessage) { for (const listener of this.messages) listener(message); }
  async close() { for (const listener of this.closes) listener(); this.closes.clear(); }
  onMessage(listener: McpTransportMessageListener) { this.messages.add(listener); return () => { this.messages.delete(listener); }; }
  onError(listener: McpTransportErrorListener) { this.errors.add(listener); return () => { this.errors.delete(listener); }; }
  onClose(listener: McpTransportCloseListener) { this.closes.add(listener); return () => { this.closes.delete(listener); }; }
}

export async function connectTools(transport: McpTransport) {
  const client = new McpClient({ name: "khor-agent", version: "1" });
  await client.connect(transport);
  const names = new Set<string>();
  const tools: ToolDefinition[] = (await client.listTools()).map((tool) => {
    const name = `mcp__khor__${tool.name}`.replace(/[^A-Za-z0-9_-]/g, "_").slice(0, 64);
    if (names.has(name)) throw new Error(`MCP tool name collision: ${name}`);
    names.add(name);
    return {
      name, label: tool.title ?? tool.name, description: tool.description ?? tool.name,
      parameters: Type.Unsafe({ ...tool.inputSchema, type: "object", properties: tool.inputSchema.properties ?? {} }),
      executionMode: "sequential",
      execute: async (_id, params, signal, onUpdate) => {
        const result = await client.callTool(tool.name, params as Record<string, unknown>, {
          signal, timeoutMs: tool.name === "delegate" ? 60_000 : 30_000,
          onProgress: (p) => onUpdate?.({ content: [{ type: "text", text: p.message ?? String(p.progress) }], details: undefined }),
        });
        return { content: toLlmContent(result), details: result.structuredContent, structuredContent: result.structuredContent as JsonValue | undefined, isError: result.isError === true };
      },
    };
  });
  return { client, tools };
}
