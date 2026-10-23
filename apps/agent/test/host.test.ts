import { test } from "node:test";
import assert from "node:assert/strict";
import { createServer, type ServerResponse } from "node:http";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve as absolute } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

function stream(response: ServerResponse, delta: unknown, finish: string) {
  response.writeHead(200, { "content-type": "text/event-stream" });
  const chunk = (d: unknown, reason: string | null) => `data: ${JSON.stringify({ id: "fixture", object: "chat.completion.chunk", created: 1, model: "fixture", choices: [{ index: 0, delta: d, finish_reason: reason }] })}\n\n`;
  response.write(chunk(delta, null)); response.write(chunk({}, finish)); response.end("data: [DONE]\n\n");
}

test("the real Pi host uses MCP only, supplies the selected key, and restores model context", { timeout: 25000 }, async () => {
  const stateDir = await mkdtemp(join(tmpdir(), "khor-pi-host-"));
  // Packaged startup must not load project dotenv or Bun preloads before
  // the host can isolate credentials, tools and extension discovery.
  await writeFile(join(stateDir, ".env"), "OPENAI_API_KEY=fixture-ambient-key\n");
  await writeFile(join(stateDir, "bunfig.toml"), 'preload = ["./unexpected.js"]\n');
  await writeFile(join(stateDir, "unexpected.js"), 'throw new Error("Unexpected project preload");\n');
  const requests: Record<string, any>[] = [];
  let authenticated = true, toolsCalled = 0;
  const server = createServer(async (request, response) => {
    let body = ""; for await (const chunk of request) body += chunk;
    authenticated &&= request.headers.authorization === "Bearer fixture-selected-key";
    const value = JSON.parse(body); requests.push(value);
    if (requests.length === 1) stream(response, { role: "assistant", tool_calls: [{ index: 0, id: "fixture-call", type: "function", function: { name: "mcp__khor__devices", arguments: "{}" } }] }, "tool_calls");
    else stream(response, { role: "assistant", content: requests.length === 2 ? "network found" : "context restored" }, "stop");
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address(); assert.ok(address && typeof address === "object");
  const turn = (text: string): Promise<Record<string, any>[]> => new Promise((resolve, reject) => {
    const compiled = process.env.KHOR_TEST_AGENT_BIN;
    const child = spawn(compiled ? absolute(compiled) : process.execPath, compiled ? [] : [fileURLToPath(new URL("../src/host.ts", import.meta.url))], { cwd: stateDir, env: { PATH: process.env.PATH }, stdio: ["pipe", "pipe", "pipe"] });
    const events: Record<string, any>[] = []; let diagnostics = "";
    child.stderr.on("data", (s) => { diagnostics += String(s).replaceAll("fixture-selected-key", "[redacted]"); });
    const lines = createInterface({ input: child.stdout });
    lines.on("line", (line) => {
      try {
        const event = JSON.parse(line); events.push(event);
        if (event.type !== "mcp") return;
        const req = event.message; if (req.id === undefined) return;
        let result: unknown;
        if (req.method === "initialize") result = { protocolVersion: "2024-11-05", capabilities: { tools: {} }, serverInfo: { name: "khor", version: "fixture" } };
        else if (req.method === "tools/list") result = { tools: [{ name: "devices", description: "List network devices", inputSchema: { type: "object", properties: {} } }] };
        else if (req.method === "tools/call") { toolsCalled++; result = { content: [{ type: "text", text: "fixture-device" }] }; }
        else result = {};
        child.stdin.write(`${JSON.stringify({ type: "mcp", message: { jsonrpc: "2.0", id: req.id, result } })}\n`);
      } catch (error) { child.kill(); reject(error); }
    });
    child.on("error", reject);
    child.on("exit", (code) => code === 0 ? resolve(events) : reject(new Error(`Host exited ${code}: ${diagnostics}; ${JSON.stringify(events.filter((e) => e.type !== "mcp"))}`)));
    child.stdin.write(`${JSON.stringify({ type: "start", protocol: 1, run: "fixture-run", text, stateDir, model: "fixture", account: { provider: "newapi", baseUrl: `http://127.0.0.1:${address.port}`, key: "fixture-selected-key" } })}\n`);
  });
  try {
    const first = await turn("Remember fixture-task, then list my devices");
    assert.ok(first.some((e) => e.type === "done"));
    const settings=first.find(e=>e.type==='settings')?.settings;
    assert.equal(settings.model,'fixture');assert(settings.configOptions.some((o:any)=>o.id==='auto_compaction'&&o.type==='boolean'));
    assert(!first.some(e=>e.type==='update'&&e.update.sessionUpdate==='usage_update'),'unknown gateway context window is not measured');
    assert.equal(first.filter((e) => e.type === "update" && e.update.sessionUpdate === "agent_message_chunk").map((e) => e.update.content.text).join(""), "network found");
    const call = first.find((e) => e.type === "update" && e.update.sessionUpdate === "tool_call");
    const done = first.find((e) => e.type === "update" && e.update.sessionUpdate === "tool_call_update" && e.update.status === "completed");
    assert.equal(call?.update.toolCallId, "fixture-call");
    assert.equal(done?.update.toolCallId, call?.update.toolCallId);
    assert.ok(JSON.stringify(done?.update.content).includes("fixture-device"));
    assert.equal(toolsCalled, 1);
    assert.ok(authenticated, "the model received credentials other than the selected account");
    assert.deepEqual(requests[0]!.tools.map((t: any) => t.function.name), ["mcp__khor__devices"]);
    assert.ok(requests[1]!.messages.some((m: any) => m.role === "tool" && m.content.includes("fixture-device")));
    await turn("What did I ask before?");
    assert.ok(requests[2]!.messages.some((m: any) => m.role === "user" && JSON.stringify(m.content).includes("fixture-task")));
    assert.ok(requests[2]!.messages.some((m: any) => m.role === "tool"));
  } finally { server.closeAllConnections(); await new Promise<void>((resolve) => server.close(() => resolve())); await rm(stateDir, { recursive: true, force: true }); }
});
