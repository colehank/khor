// Real Composer + MachineChat, with only the command transport substituted.
// The fixture advances the node independently of the last painted snapshot.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { writeFileSync, rmSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
const gui = fileURLToPath(new URL("..", import.meta.url));
const port = Number(process.env.CONSOLE_PORT ?? 1467);
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let vite, browser;
const entry = `${gui}/scripts/.console-gate-${process.pid}.tsx`;
try {
  vite = spawn("node_modules/.bin/vite", ["--port", String(port), "--strictPort"], { cwd: gui, stdio: ["ignore", "pipe", "pipe"] });
  let ready = false, exited = false;
  vite.stdout.on("data", (chunk) => { ready ||= String(chunk).includes(`:${port}`); });
  vite.on("exit", () => { exited = true; });
  for (let n = 0; n < 100 && !ready && !exited; n++) await pause(100);
  assert(ready, "vite did not start");
  await fetch(`http://localhost:${port}/src/main.tsx`);
  await pause(1000);
  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server", "--disable-features=LocalNetworkAccessChecks"] });
  const page = await browser.newPage({ viewport: { width: 1000, height: 650 } });
  const errors = [], diagnostics = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (event) => { if (event.type() === "error") diagnostics.push(event.text()); });
  page.on("requestfailed", (request) => diagnostics.push(`${request.url()} ${request.failure()?.errorText}`));
  const calls = [];
  let failNext = false, receiptNodeTail = 100;
  const message = (id, at, text) => ({ id, at_ms: at, text, from: "peer", from_me: false, to: [], files: null, note: null, unknown: null, retracted: false, command: null });
  let messages = [message("first", 100, "first visible")];
  const catalog = [{ word: "status", aliases: [], usage: "status", reach: "local", mode: "read", parameters: [] }, { word: "ls", aliases: [], usage: "ls MACHINE:PATH", reach: "remote", mode: "read", parameters: ["path"] }];
  await page.route(`http://127.0.0.1:${port}/**`, async (route) => {
    const cmd = new URL(route.request().url()).pathname.slice(1);
    const args = route.request().postDataJSON() ?? {};
    calls.push({ cmd, args, nodeTail: receiptNodeTail });
    let result = null;
    if (cmd === "machine_log") result = { messages: structuredClone(messages), broken: 0 };
    else if (cmd === "devices") result = [];
    else if (cmd === "khor_agent") {
      const config = { home: "local", account: "account-fixture", model: "fixture-model" };
      result = { config, accounts: [{ id: config.account, name: "fixture account", home: "local", home_name: "local", provider: "anthropic", default_model: "fixture-model", configured: true, base_url: null, auth_type: "api_key" }], run: null };
      if (args.action === "manage") result.data = args.payload.operation === "models" ? { job: "models-fixture", status: "running" } : { status: "done", result: { models: [{ id: "fixture-model", name: "Fixture", contextWindow: 128000, reasoning: false, image: false }] } };
      if (args.action === "prompt") {
        const id = "note";
        if (!messages.some((m) => m.id === id)) messages.push({ ...message(id, ++receiptNodeTail, null), command: { id, line: args.payload.text, owner: "local", owner_name: "local", status: "running", started_at_ms: Date.now(), ended_at_ms: null, stdout: "", stderr: "", data_json: null, session: null, error: null, agent: { model: config.model, approval: null } } });
        result.run = id;
      }
      if (args.action === "answer") {
        const m = messages.find((m) => m.id === args.payload.run);
        m.command.agent.approval = null; m.command.status = "done"; m.command.ended_at_ms = Date.now();
      }
    }
    else if (cmd === "console_catalog") result = catalog;
    else if (cmd === "console_complete") {
      const line = args.line;
      if (line === "/ls ") result = { start: 4, end: 4, choices: [{ value: "peer", label: "peer", detail: "machine", kind: "machine" }] };
      else if (line === "/ls peer /") result = { start: 9, end: line.length, choices: [{ value: "/data/", label: "data/", detail: "directory", kind: "directory" }] };
      else if (/^\/\w*$/.test(line)) result = { start: 0, end: args.caret, choices: catalog.filter((c) => c.word.startsWith(line.slice(1))).map((c) => ({ value: `/${c.word}`, label: `/${c.word}`, detail: c.usage, kind: "command" })) };
      else result = { start: args.caret, end: args.caret, choices: [] };
    } else if (cmd === "console_run") {
      if (failNext) { failNext = false; await route.fulfill({ status: 503, body: "retry this command" }); return; }
      const id = `job${calls.filter((c) => c.cmd === "console_run").length}`;
      messages.push({ ...message(id, ++receiptNodeTail, null), command: { id, line: args.line, owner: "local", owner_name: "local", status: "running", started_at_ms: Date.now(), ended_at_ms: null, stdout: "", stderr: "", data_json: null, session: null, error: null } });
      result = id;
    } else if (cmd === "post") { messages.push(message("note", ++receiptNodeTail, args.typed)); result = "note"; }
    await route.fulfill({ contentType: "application/json", body: JSON.stringify(result) });
  });
  writeFileSync(entry, `import React from "react";
import {createRoot} from "react-dom/client";
import {flushSync} from "react-dom";
import {MachineChat} from '../src/views/MachineChat.tsx';
import {TooltipProvider} from '../src/components/ui/tooltip.tsx';
import '../src/styles/tokens.css'; import '../src/styles/app.css';
const root=createRoot(document.getElementById('root')); window.jumps=[]; window.linked=[];
window.renderConsole=(active=true)=>flushSync(()=>root.render(React.createElement(TooltipProvider,null,React.createElement(MachineChat,{id:'wide/all',active,sessions:window.linked,onSession:id=>window.jumps.push(id)}))));
window.renderConsole();
`);
  await page.route("**/__console_gate?**", (route) => route.fulfill({ contentType: "text/html", body: `<!doctype html><html data-theme="light"><head><meta charset="utf-8"></head><body><div id="root" class="flex h-screen flex-col"></div><script type="module">
import RefreshRuntime from '/@react-refresh'; RefreshRuntime.injectIntoGlobalHook(window); window.$RefreshReg$=()=>{}; window.$RefreshSig$=()=>type=>type; window.__vite_plugin_react_preamble_installed__=true;
</script><script type="module">
import '/scripts/.console-gate-${process.pid}.tsx';
</script></body></html>` }));
  await page.goto(`http://localhost:${port}/__console_gate?bridge=${port}`);
  const box = page.locator("[data-chat-composer]");
  await box.waitFor({ timeout: 10000 }).catch(async (error) => { throw new Error(`${error.message}\npage errors: ${errors.concat(diagnostics).join("\n")}\n${await page.locator("body").innerText()}`); });
  await page.waitForFunction(() => document.querySelector("[data-msg='first']"));
  await pause(150);
  assert(calls.some((c) => c.cmd === "seen_snapshot" && c.args.messages.some((m) => m.id === "first" && m.at_ms === 100)), "rendered tail not acknowledged");
  await box.pressSequentially("/sta");
  await page.locator("[data-console-choice='command']").waitFor();
  await box.press("Tab");
  assert.equal(await box.inputValue(), "/status ");
  await pause(250);
  await box.press("Enter");
  await page.locator("[data-console-status='running']").waitFor();
  assert.equal(calls.filter((c) => c.cmd === "post").length, 0, "slash command posted as prose");
  const job = messages.find((m) => m.command);
  job.command.status = "done";
  job.command.data_json = '[{"machine":"peer","online":true}]';
  job.command.session = "session-from-command";
  job.command.ended_at_ms = Date.now();
  await page.evaluate(() => { window.linked=[{id:"session-from-command",word:"busy",last:"working step 1"}]; window.renderConsole(); });
  await page.locator("[data-console-table]").waitFor();
  await page.locator("[data-console-result] [data-status-word='busy']").waitFor();
  assert.equal(await page.locator("[data-console-session-latest]").innerText(), "working step 1");
  await page.evaluate(() => { window.linked[0]={...window.linked[0],word:"done",last:"finished step 2"}; window.renderConsole(); });
  await page.locator("[data-console-result] [data-status-word='done']").waitFor();
  if (process.env.CONSOLE_SHOTS) {
    const dir = "/Users/zgh/khor-drop/khor-console";
    mkdirSync(dir, { recursive: true });
    for (const [name, viewport] of [["wide", { width: 1200, height: 800 }], ["narrow", { width: 390, height: 844 }]]) {
      await page.setViewportSize(viewport);
      for (const theme of ["light", "dark"]) {
        await page.evaluate((theme) => { document.documentElement.dataset.theme = theme; }, theme);
        await pause(100);
        await page.screenshot({ path: `${dir}/${name}-${theme}.png` });
      }
    }
    await page.setViewportSize({ width: 1000, height: 650 });
    await page.evaluate(() => { document.documentElement.dataset.theme = "light"; });
  }

  await page.locator("[data-console-session]").click();
  assert.deepEqual(await page.evaluate(() => window.jumps), ["session-from-command"]);
  await box.fill("/ls ");
  await page.locator("[data-console-choice='machine']").waitFor();
  await box.press("ArrowDown"); await box.press("Tab");
  assert.equal(await box.inputValue(), "/ls peer ");
  await box.pressSequentially("/");
  await page.locator("[data-console-choice='directory']").waitFor(); await box.press("Tab");
  assert.equal(await box.inputValue(), "/ls peer /data/");
  await box.press("Escape");
  await box.fill("/status --retry"); await pause(250);
  await page.locator("[data-console-input-toggle]").click();
  const stdin = page.locator("[data-console-input]");
  assert.equal(await stdin.getAttribute("type"), "password");
  await stdin.fill("secret-stdin-value"); await box.focus(); failNext = true;
  await box.press("Enter"); await page.locator("[data-post-refused]").waitFor();
  assert.equal(await box.inputValue(), "/status --retry", "failed command lost draft");
  await box.press("Enter");
  await page.waitForFunction(() => document.querySelector("[data-chat-composer]").value === "");
  const retried = calls.filter((c) => c.cmd === "console_run" && c.args.line.includes("--retry"));
  assert.equal(retried.length, 2); assert.equal(retried[1].args.input, "secret-stdin-value"); assert.equal(await page.locator("[data-console-input-panel]").count(), 0); assert.equal(retried[0].args.token, retried[1].args.token, "retry token changed");
  await box.pressSequentially("@peer literal task");
  assert.equal(await page.locator("[data-at-picker]").count(), 0);
  await box.press("Enter");
  await page.waitForFunction(() => document.querySelector("[data-msg='note']"));
  const task = calls.find((c) => c.cmd === "khor_agent" && c.args.action === "prompt");
  assert.equal(task.args.payload.text, "@peer literal task");
  assert.equal(task.args.machine, "local");
  assert.equal(calls.filter((c) => c.cmd === "post").length, 0, "natural-language task posted as a note");
  const agentReceipt = messages.find((m) => m.id === "note").command;
  agentReceipt.stdout = "**streamed answer**";
  agentReceipt.agent.updates = [
    { sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "检查设备网络" } },
    { sessionUpdate: "tool_call", toolCallId: "pi-devices", title: "mcp__khor__devices", kind: "read", status: "in_progress", rawInput: {} },
    { sessionUpdate: "tool_call_update", toolCallId: "pi-devices", status: "completed", content: [{ type: "content", content: { type: "text", text: "fixture device found" } }] },
    { sessionUpdate: "agent_message_chunk", content: { type: "text", text: "**streamed answer**" } },
  ].map(JSON.stringify);
  agentReceipt.status = "blocked";
  agentReceipt.agent.approval = { id: "approval-fixture", tool: "open", arguments: '{"machine":"peer","command":"echo fixture"}' };
  await page.locator("[data-khor-agent-approval]").waitFor();
  assert.equal(await page.locator("[data-khor-agent-turn] strong").innerText(), "streamed answer");
  await page.locator('[data-khor-agent-turn] [data-tool-id="pi-devices"] [data-tool-toggle]').click();
  await page.getByText("fixture device found", { exact: true }).waitFor();
  assert.equal(await page.locator('[data-khor-agent-turn] [data-chat-reasoning]').count(), 1, "khor reasoning uses the shared ACP presentation");
  assert.equal(await page.locator('[data-khor-agent-turn] [data-tool-id="pi-devices"]').getAttribute("data-tool-status"), "completed");
  await page.locator('[data-khor-agent-approval] [data-ask-option="deny"]').click();
  await page.waitForFunction(() => !document.querySelector("[data-khor-agent-approval]"));
  assert.equal(calls.find((c) => c.cmd === "khor_agent" && c.args.action === "answer").args.payload.allow, false);
  await page.locator("[data-khor-agent-settings]").click();
  await page.locator("[data-khor-agent-model] input").fill("next-model");
  await page.locator("[data-khor-agent-save]").click();
  assert.equal(calls.find((c) => c.cmd === "khor_agent" && c.args.action === "configure").args.payload.model, "next-model");
  for (const theme of ["light", "dark"]) {
    await page.evaluate((theme) => { document.documentElement.dataset.theme = theme; }, theme);
    for (const width of [390, 1000]) {
      await page.setViewportSize({ width, height: 650 });
      const layout = await page.locator("[data-khor-agent-turn]").evaluate((el) => ({ turn: el.getBoundingClientRect().width, column: document.querySelector("[data-chat-column]").getBoundingClientRect().width }));
      assert(Math.abs(layout.turn - layout.column) < 2, "agent answer lost the shared reading width");
      await page.locator("[data-khor-agent-settings]").click();
      const bounds = await page.locator("[data-khor-agent-config]").boundingBox();
      assert(bounds && bounds.x >= -1 && bounds.x + bounds.width <= width + 1, `${theme}/${width}: config outside viewport`);
      assert(await page.locator("[data-khor-agent-save]").isVisible());
      await page.keyboard.press("Escape");
    }
  }
  await page.evaluate(() => { document.documentElement.dataset.theme = "light"; });
  await page.setViewportSize({ width: 1000, height: 650 });

  // A message arriving after the readout but before seen is processed is
  // deliberately newer than every visible bubble.
  // Resume following after opening tool details / resizing the pane.
  if (await page.locator("[data-machine-chat-bottom]").count()) await page.locator("[data-machine-chat-bottom]").click();
  receiptNodeTail = 999;
  messages.push(message("bounded", 500, "bounded visible"));
  await page.locator("[data-msg='bounded']").waitFor(); await pause(150);
  assert(calls.some((c) => c.cmd === "seen_snapshot" && c.args.messages.some((m) => m.id === "bounded" && m.at_ms === 500) && c.nodeTail === 999), "receipt was not snapshot bounded");
  assert(!calls.some((c) => c.cmd === "seen_snapshot" && (!Array.isArray(c.args.messages) || c.args.messages.some((m) => m.at_ms > 500))), "unseen node tail acknowledged");

  const beforeSameMs = calls.filter((c) => c.cmd === "seen_snapshot").length;
  assert(!calls.some((c) => c.cmd === "seen_snapshot" && c.args.messages.some((m) => m.id === "same-ms")), "unrendered same-ms message acknowledged");
  messages.push(message("same-ms", 500, "later message at the same millisecond"));
  await page.locator("[data-msg='same-ms']").waitFor(); await pause(150);
  assert(calls.filter((c) => c.cmd === "seen_snapshot").length > beforeSameMs, "same-ms rendered message skipped");
  assert(calls.some((c) => c.cmd === "seen_snapshot" && c.args.messages.some((m) => m.id === "same-ms" && m.at_ms === 500)), "same-ms identity absent from receipt");
  const versioned = messages.find((m) => m.id === "same-ms");
  versioned.at_ms = 501; versioned.text = "new version of the rendered message";
  await page.getByText("new version of the rendered message", { exact: true }).waitFor(); await pause(150);
  assert(calls.some((c) => c.cmd === "seen_snapshot" && c.args.messages.some((m) => m.id === "same-ms" && m.at_ms === 501)), "new rendered version skipped");
  const handed = await page.evaluate(async () => {
    const {tellDraft} = await import("/src/lib/composer-handoff.ts");
    return [tellDraft("peer"), tellDraft("peer's machine")];
  });
  assert.deepEqual(handed, ["/tell peer ", "/tell 'peer'\\''s machine' "]);

  for (let n = 0; n < 55; n++) messages.push(message(`history${n}`, 600 + n, `history line ${n}\n${"long content ".repeat(25)}`));
  await page.locator("[data-msg='history54']").waitFor(); await pause(150);
  await page.locator("[data-machine-chat-scroll]").evaluate((el) => { el.scrollTop = 0; el.dispatchEvent(new Event("scroll", { bubbles: true })); });
  const beforeHistory = calls.filter((c) => c.cmd === "seen_snapshot").length;
  messages.push(message("while-history", 700, "keep unread while reading history"));
  await page.locator("[data-msg='while-history']").waitFor(); await pause(150);
  assert.equal(calls.filter((c) => c.cmd === "seen_snapshot").length, beforeHistory, "history scroll marked unseen tail");
  await page.evaluate(() => window.renderConsole(false));
  await page.locator("[data-machine-chat-scroll]").evaluate((el) => { el.scrollTop = el.scrollHeight; el.dispatchEvent(new Event("scroll", { bubbles: true })); });
  await pause(100); messages.push(message("inactive", 800, "inactive pane"));
  await page.locator("[data-msg='inactive']").waitFor(); await pause(150);
  assert.equal(calls.filter((c) => c.cmd === "seen_snapshot").length, beforeHistory, "inactive pane marked read");
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", { configurable: true, get: () => true });
    document.dispatchEvent(new Event("visibilitychange")); window.renderConsole(true);
  });
  await pause(100); messages.push(message("background", 900, "background document"));
  await page.locator("[data-msg='background']").waitFor(); await pause(150);
  assert.equal(calls.filter((c) => c.cmd === "seen_snapshot").length, beforeHistory, "background document marked read");
  await page.evaluate(() => { Object.defineProperty(document, "hidden", { configurable: true, get: () => false }); document.dispatchEvent(new Event("visibilitychange")); });
  await pause(200);
  assert(calls.some((c) => c.cmd === "seen_snapshot" && c.args.messages.some((m) => m.id === "background" && m.at_ms === 900)), "visible tail did not resume read receipt");
  assert.deepEqual(errors, [], "browser errors");
  console.log("console gate: command completions/retry, structured results/navigation, khor agent tasks/configuration/approval/streaming and visible-tail receipts");
} finally {
  await browser?.close(); vite?.kill(); rmSync(entry, { force: true });
}
