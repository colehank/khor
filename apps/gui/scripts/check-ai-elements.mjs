// Production App / ChatView, with only the bridge replaced. No test UI.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { build } from "esbuild";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const port = Number(process.env.AI_ELEMENTS_PORT ?? 23772);
const out = process.env.AI_ELEMENTS_OUT ?? join(tmpdir(), `khor-ai-elements-${process.pid}`);
mkdirSync(out, { recursive: true });
const modelFile = join(out, "agent-diff.mjs");
await build({ entryPoints: [join(gui, "src/lib/agent-diff.ts")], outfile: modelFile, bundle: true, platform: "node", format: "esm", logLevel: "silent" });
const { agentDiff, agentDiffContext } = await import(pathToFileURL(modelFile).href);
for (const [before, after] of [
  ["a\nx\nz\n", "a\ny\nz\n"], ["a\n", "a\nx\n"], [null, "new file\n"], ["old\n", ""],
  ["same\n", "same\n"], ["a", "a\n"], ["a\n", "a"], ["a\r\nb\r\n", "a\nb\n"],
  ["a\na\nb\na", "a\nb\na\na"], ["\n\n", "\n"],
]) {
  const rows = agentDiff(before, after);
  const normalized = (s) => (s ?? "").replace(/\r\n/g, "\n").replace(/\n$/, "");
  assert.equal(rows.filter((r) => r.kind !== "added").map((r) => r.text).join("\n"), normalized(before), "diff reconstructs original lines");
  assert.equal(rows.filter((r) => r.kind !== "removed").map((r) => r.text).join("\n"), normalized(after), "diff reconstructs resulting lines");
  for (const kind of ["before", "after"]) assert.deepEqual(rows.map((r) => r[kind]).filter((n) => n !== null), Array.from({ length: rows.filter((r) => r[kind] !== null).length }, (_, n) => n + 1), "old and new line numbers are independently continuous");
}
const shared = Array.from({ length: 40 }, (_, n) => `const value${n} = ${n};`);
const contextual = agentDiff(shared.join("\n"), shared.map((s, n) => n === 18 ? "const value18 = 99;" : s).join("\n"));
assert.equal(contextual.filter((r) => r.kind === "removed").length, 1);
assert.equal(contextual.filter((r) => r.kind === "added").length, 1);
const gaps = agentDiffContext(contextual, new Set()).filter((r) => r.kind === "gap");
assert.equal(gaps.length, 2, "long unchanged runs collapse around real changes");
assert.deepEqual(agentDiffContext(contextual, new Set(gaps.map((r) => r.at))), contextual, "revealing gaps recovers every line");
assert.deepEqual(agentDiffContext(contextual, new Set(), true), contextual, "full view preserves all lines");
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const note = (sessionUpdate, rest) => ({ kind: "note", update: { update: { sessionUpdate, ...rest } } });
const thought = (text) => note("agent_thought_chunk", { content: { type: "text", text } });
const longPath = `/work/${"nested-project/".repeat(12)}src/session.ts`;
const history = (sessionUpdate, rest) => ({ ...note(sessionUpdate, rest), kind: "history" });
const reviewFrames = () => [
  history("user_message_chunk", { content: { type: "text", text: "修一下工作区里的撤销快捷键，切换文件后也要保留编辑状态。" } }),
  history("agent_message_chunk", { content: { type: "text", text: "我先检查编辑器的快捷键绑定和文件切换时的状态保存。" } }),
  history("agent_thought_chunk", { content: { type: "text", text: "撤销应当由当前编辑器处理。先确认系统快捷键有没有被全局菜单截获，再检查每个文件的历史是否独立保存。" } }),
  history("tool_call", { toolCallId: "review-read", title: "Read /work/khor/apps/gui/src/components/CodeEditor.tsx", kind: "read", status: "completed", rawInput: { file_path: "/work/khor/apps/gui/src/components/CodeEditor.tsx" } }),
  history("tool_call", { toolCallId: "review-search", title: "检查全局快捷键与编辑器焦点", kind: "search", status: "completed", rawInput: { pattern: "Mod-z|historyKeymap", path: "/work/khor/apps/gui/src" }, content: [{ type: "content", content: { type: "text", text: "CodeEditor.tsx: historyKeymap\nnative_edit.rs: focused editor dispatch" } }] }),
  history("tool_call", { toolCallId: "review-edit", title: "Edit /work/khor/apps/gui/src/components/CodeEditor.tsx", kind: "edit", status: "completed", rawInput: { file_path: "/work/khor/apps/gui/src/components/CodeEditor.tsx" }, content: [{ type: "diff", path: "/work/khor/apps/gui/src/components/CodeEditor.tsx", oldText: "const extensions = [\n  defaultKeymap,\n  languageSupport,\n];", newText: "const extensions = [\n  history(),\n  keymap.of([...historyKeymap, ...defaultKeymap]),\n  languageSupport,\n];" }] }),
  history("tool_call", { toolCallId: "review-test", title: "编辑历史回归通过", kind: "execute", status: "completed", rawInput: { command: "npm run editor-history" }, content: [{ type: "content", content: { type: "text", text: "Undo / redo passed\nHistory retained across file switches\nNo cross-file history changes" } }] }),
  history("agent_message_chunk", { content: { type: "text", text: "快捷键和编辑历史已经接好。\n\n- **⌘Z / ⇧⌘Z** 交给当前编辑器处理\n- 切换文件后，撤销和重做历史各自保留\n- 预览与源码切换不会重置历史\n\n浏览器回归已通过，接下来检查 Mac 原生窗口中的系统菜单。" } }),
  { kind: "history_end" },
  { kind: "ask", ask: 99, title: "打开独立的 Mac 测试窗口，验证系统菜单的撤销与重做？", options: [["allow-once", "本次允许"], ["deny", "暂不运行"]] },
];
const results = [];
// Reduced-motion-only coverage previously hid a two-second disclosure animation.
async function checkDisclosureMotion(page, trigger) {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  if (await trigger.getAttribute("aria-expanded") !== "true") await trigger.click();
  const id = await trigger.getAttribute("aria-controls");
  assert(id, "expanded disclosure exposes its content relationship");
  const content = page.locator(`[id="${id}"]`);
  const checkDuration = async () => {
    const timing = await content.evaluate((el) => {
      const s = getComputedStyle(el);
      return { name: s.animationName, durations: s.animationDuration.split(",").map((v) => Number.parseFloat(v) * (v.trim().endsWith("ms") ? 1 : 1000)) };
    });
    assert(timing.durations.every((ms) => ms <= 200), `disclosure must finish within 200 ms: ${JSON.stringify(timing)}`);
  };
  if (await trigger.getAttribute("aria-expanded") === "true") {
    await trigger.click();
    if (await content.count()) await checkDuration();
    await content.waitFor({ state: "hidden", timeout: 700 });
  }
  await trigger.click();
  await content.waitFor();
  await checkDuration();
  await page.waitForFunction((id) => {
    const el = document.getElementById(id);
    return el && el.getAnimations().every((a) => a.playState === "finished");
  }, id, { timeout: 700 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  assert.equal(await content.evaluate((el) => getComputedStyle(el).animationName), "none", "reduced motion disables the height animation");
}
let browser, vite;
try {
  vite = spawn("node_modules/.bin/vite", ["--host", "127.0.0.1", "--port", String(port), "--strictPort"], {
    cwd: gui, env: { ...process.env, KHOR_FACE_GATE: "1" }, stdio: ["ignore", "pipe", "pipe"],
  });
  let ready = false, log = "";
  vite.stdout.on("data", (b) => { ready ||= String(b).includes(`:${port}`); });
  vite.stderr.on("data", (b) => { log += b; });
  for (let i = 0; i < 150 && !ready && vite.exitCode === null; i++) await sleep(100);
  assert(ready, log || "Vite did not start");
  browser = await chromium.launch({ channel: "chrome", headless: process.env.AI_ELEMENTS_HEADED !== "1" });
  for (const [name, width, scheme] of [["desktop-light", 1280, "light"], ["desktop-dark", 1280, "dark"], ["mobile-light", 390, "light"], ["mobile-dark", 390, "dark"]]) {
    const context = await browser.newContext({ viewport: { width, height: 844 }, colorScheme: scheme, reducedMotion: "reduce", hasTouch: width < 600, isMobile: width < 600 });
    const page = await context.newPage(), errors = [], requests = [];
    page.on("pageerror", (error) => { errors.push(error.message); console.error(error.message); });
    const reply = (value) => (route) => route.fulfill({ contentType: "application/json", body: JSON.stringify(value) });
    const row = (id, title) => ({ id, kind: "gui", title, word: "idle", at_ms: Date.now(), unread: 0, source: null, pinned: false, stale: false, category: "claude", group: "", face: null, hosted: true, attachable: false, last: null, via: null, via_face: null, home: "gate" });
    await page.route("**/sessions", reply([row("gui/elements-a", "Elements A"), row("gui/elements-b", "Elements B")]));
    await page.route("**/devices", reply([{ id: "e".repeat(64), name: "gate", me: true, face: null, pinned: false, vitals: null, hop: null, sponsor: null, watch: [], watched: null, seat: { left: 50, top: 50 }, hosts: true }]));
    for (const path of ["dir_pins", "web_pins", "recents"]) await page.route(`**/${path}`, reply([]));
    for (const path of ["chat_replay", "chat_leave", "chat_say", "chat_stop"]) await page.route(`**/${path}`, reply(null));
    await page.route("**/hooks_state", reply({ installed: false }));
    await page.route("**/quota", reply({ quota: null, trouble: null }));
    let reviewPending = false;
    await page.route("**/chat_open", (route) => {
      if (reviewPending) { queue = reviewFrames(); reviewPending = false; }
      return reply(true)(route);
    });
    await page.route("**/tabs_list", reply({ tabs: [], active: "" }));
    let queue = [{ kind: "history_end" }, { kind: "turning" }, thought("先检查工作区。")], otherQueue = [], cursor = 0;
    await page.route("**/chat_poll", (route) => {
      const id = JSON.parse(route.request().postData()).id;
      const frames = id === "gui/elements-a" ? queue : otherQueue;
      if (id === "gui/elements-a") queue = []; else otherQueue = [];
      cursor += frames.length;
      return reply({ frames, next: cursor, gone: false, lost: false, stall: null })(route);
    });
    let reject = true, holdAnswer = true, releaseAnswer;
    await page.route("**/chat_answer", async (route) => {
      requests.push(JSON.parse(route.request().postData()));
      if (reject) {
        reject = false;
        await route.fulfill({ status: 503, contentType: "text/plain", body: "Permission reply could not reach the machine" });
      } else {
        if (holdAnswer) { holdAnswer = false; await new Promise((resolve) => { releaseAnswer = resolve; }); }
        await reply(null)(route);
      }
    });
    await page.goto(`http://127.0.0.1:${port}/?bridge=${port}`);
    await page.locator("[data-list] [data-row-open]").filter({ hasText: "Elements A" }).click();
    const reasoning = page.locator("[data-reasoning-toggle]").first();
    await reasoning.waitFor();
    await page.waitForFunction(() => document.querySelector("[data-reasoning-toggle]")?.getAttribute("aria-expanded") === "true");
    queue.push(thought("\n路径和权限都需要确认。"));
    await page.getByText("路径和权限都需要确认。", { exact: false }).waitFor();
    await reasoning.click();
    queue.push(thought("\n仍在思考。"));
    await sleep(650);
    assert.equal(await reasoning.getAttribute("aria-expanded"), "false", "explicit collapse survives chunks");
    await reasoning.focus(); await page.keyboard.press("Enter");
    assert.equal(await reasoning.getAttribute("aria-expanded"), "true", "keyboard opens reasoning");
    queue.push(note("agent_message_chunk", { content: { type: "text", text: "我会先读文件，再运行测试。" } }));
    await page.getByText("我会先读文件，再运行测试。", { exact: true }).waitFor();
    assert.equal(await reasoning.getAttribute("aria-expanded"), "true", "explicit open survives completion");
    await checkDisclosureMotion(page, reasoning);
    const thoughtSize = await page.locator("[data-chat-reasoning] p").first().evaluate((el) => getComputedStyle(el).fontSize);
    const answerSize = await page.getByText("我会先读文件，再运行测试。", { exact: true }).evaluate((el) => getComputedStyle(el).fontSize);
    assert.equal(thoughtSize, answerSize, "reasoning retains the conversation's reading size");
    queue.push(thought("第二段思考。"));
    const second = page.locator("[data-reasoning-toggle]").nth(1); await second.waitFor();
    assert.equal(await second.getAttribute("aria-expanded"), "true");
    queue.push(note("tool_call", { toolCallId: "read-long", title: `Read ${longPath}`, kind: "read", status: "in_progress", rawInput: { path: longPath } }));
    const tool = page.locator("[data-chat-tool]"); await tool.waitFor();
    assert.equal(await second.getAttribute("aria-expanded"), "false", "automatic reasoning closes on next activity");
    const toggle = tool.locator("[data-tool-toggle]"); await toggle.click();
    const header = await toggle.boundingBox();
    const headerLimit = await page.evaluate(() => Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--touch")));
    assert(header.height <= headerLimit + 1, "long paths stay a single compact tool row");
    const controls = await toggle.getAttribute("aria-controls"); assert(controls, "details have an accessible relationship");
    queue.push(note("tool_call_update", { toolCallId: "read-long", status: "failed", content: [{ type: "content", content: { type: "text", text: "Permission denied\n" + ("a-very-long-output ".repeat(30) + "\n").repeat(80) } }] }));
    await page.waitForFunction(() => document.querySelector("[data-chat-tool]")?.getAttribute("data-tool-status") === "failed");
    assert.equal(await toggle.getAttribute("aria-controls"), controls, "a streamed update preserves the control");
    assert.equal(await toggle.getAttribute("aria-expanded"), "true");
    await tool.locator("[data-tool-out]").waitFor();
    const metrics = await tool.locator("[data-tool-out]").evaluate((el) => ({ h: el.clientHeight, full: el.scrollHeight }));
    assert(metrics.h > 0 && metrics.h < 844 && metrics.full > metrics.h, "large output scrolls inside a bounded viewport");
    await checkDisclosureMotion(page, toggle);
    await toggle.click();
    assert.equal(await tool.locator("[data-tool-out]").count(), 0, "collapsed output is unmounted");
    queue.push(note("tool_call", { toolCallId: "no-details", title: "Search project", kind: "search", status: "completed" }));
    await page.waitForFunction(() => document.querySelectorAll("[data-chat-tool]").length === 2);
    assert.equal(await page.locator("[data-chat-tool]").nth(1).locator("button").count(), 0, "no empty disclosure");
    queue.push(note("tool_call", { toolCallId: "edit-file", title: "Edit src/session.ts", kind: "edit", status: "completed", rawInput: { file_path: "/work/src/session.ts" }, content: [{ type: "diff", path: "/work/src/session.ts", oldText: "const shared = true;\nconst value = 1;\nexport { value };\n", newText: "const shared = true;\nconst value = 2;\nexport { value };\n" }] }));
    const edit = page.locator('[data-chat-tool][data-tool-kind="edit"]');
    await edit.locator("[data-tool-toggle]").click();
    await edit.locator("[data-diff-before]").first().waitFor();
    assert((await edit.locator("[data-diff-before]").textContent()).includes("const value = 1;"));
    assert((await edit.locator("[data-diff-after]").textContent()).includes("const value = 2;"));
    assert.equal(await edit.locator("[data-diff-before]").count(), 1, "only the changed old line is removed");
    assert.equal(await edit.locator("[data-diff-after]").count(), 1, "unchanged context must not be marked added");
    assert.equal(await edit.locator("[data-tool-raw]").count(), 0, "diff replaces redundant raw input");
    const entries = [{ content: "读取文件", status: "completed" }, { content: "运行测试", status: "in_progress" }];
    queue.push(note("plan", { entries }));
    const plan = page.locator("[data-chat-plan]"); await plan.waitFor();
    await plan.locator("button").click();
    queue.push(note("plan", { entries: entries.map((entry) => ({ ...entry, status: "completed" })) }));
    await sleep(650);
    assert.equal(await plan.locator("button").getAttribute("aria-expanded"), "false", "a plan update respects the reader");
    await plan.locator("button").click();
    await page.waitForFunction(() => [...document.querySelectorAll("[data-plan-entry]")].every((el) => el.getAttribute("data-plan-status") === "completed"));
    await checkDisclosureMotion(page, plan.locator("button"));
    queue.push({ kind: "ask", ask: 31, title: `允许读取 ${longPath}？`, options: [["allow-once", "仅允许本次读取这个文件"], ["deny", "拒绝读取，保留当前工作区不变"]] });
    let option = page.locator('[data-ask-option="allow-once"]'); await option.click();
    await page.locator("[data-chat-ask] [role=alert]").waitFor();
    assert.equal(requests.length, 1); await option.click();
    await page.locator("[data-ask-pending]").waitFor();
    assert.equal(await page.locator("[data-ask-pending]").getAttribute("data-ask-phase"), "sending", "request stays visibly submitting while transport is pending");
    assert.equal(await page.locator("[data-ask-answer]").count(), 0, "transport success must not imply server confirmation");
    assert.equal(await option.isDisabled(), true, "pending choice remains visible and disabled");
    await option.dispatchEvent("click");
    releaseAnswer();
    await page.locator('[data-ask-pending][data-ask-phase="waiting"]').waitFor();
    await sleep(350);
    assert.equal(requests.length, 2, "one retry, no double submit");
    assert.deepEqual(requests[1], { id: "gui/elements-a", ask: 31, option: "allow-once" });
    queue.push({ kind: "answered", ask: 31, option: "deny" });
    await page.waitForFunction(() => document.querySelector("[data-ask-answer]")?.textContent === "拒绝读取，保留当前工作区不变");
    assert.equal(await page.locator("[data-ask-option]").count(), 0, "authoritative reply settles confirmation");
    await checkDisclosureMotion(page, page.locator("[data-chat-ask] [data-slot=trace-collapsible-trigger]"));
    queue.push({ kind: "ask", ask: 32, title: "需要选择下一步", options: [["allow", "允许"], ["deny", "拒绝"]] });
    await page.locator('[data-ask-option="deny"]').click();
    queue.push({ kind: "answered", ask: 32, option: "deny" }, { kind: "turn", stop: "EndTurn" });
    await page.waitForFunction(() => document.querySelectorAll("[data-ask-option]").length === 0);
    queue.push({ kind: "ask", ask: 34, title: "已结束的权限请求", options: [["allow", "允许"]] }, { kind: "answered", ask: 34, option: null });
    await page.getByText("请求已结束", { exact: true }).waitFor();
    const overflow = await page.evaluate(() => ({ viewport: innerWidth, width: document.documentElement.scrollWidth, column: document.querySelector("[data-chat-column]").getBoundingClientRect().right }));
    assert(overflow.width <= overflow.viewport && overflow.column <= overflow.viewport, JSON.stringify(overflow));
    // Review the initial, open permission state as well as the decision.
    queue.push({ kind: "ask", ask: 33, title: "运行工作区测试？", options: [["allow", "本次允许"], ["deny", "拒绝"]] });
    await page.locator('[data-ask-option="allow"]').waitFor();
    if (width < 600) {
      const target = await page.locator('[data-ask-option="allow"]').boundingBox();
      const touch = await page.evaluate(() => Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--touch")));
      assert(target.height >= touch, "permission options retain the shared touch target");
    }
    await page.locator("[data-chat-scroll]").evaluate((el) => { el.scrollTop = el.scrollHeight; });
    await page.evaluate(() => window.getSelection()?.removeAllRanges());
    await page.screenshot({ path: join(out, `${name}-stress.png`) });
    reviewPending = true;
    await page.reload();
    const firstRow = page.locator("[data-list] [data-row-open]").filter({ hasText: "Elements A" });
    await firstRow.waitFor({ state: "attached" });
    if (await firstRow.isVisible()) await firstRow.click();
    const change = page.locator('[data-tool-id="review-edit"]');
    await change.locator("[data-tool-toggle]").click();
    await change.locator("[data-diff-before]").first().waitFor();
    await page.locator('[data-ask-option="allow-once"]').waitFor();
    const geometry = await page.locator('[data-chat-column] [data-slot="trace-icon"] svg').evaluateAll((icons) => icons.map((icon) => {
      const a = icon.getBoundingClientRect(), b = icon.parentElement.getBoundingClientRect();
      return Math.abs(a.width - b.width) < 1 && Math.abs(a.height - b.height) < 1;
    }));
    assert(geometry.length >= 4 && geometry.every(Boolean), "activity icons fit one shared slot");
    const disclosureGap = await change.locator("[data-tool-toggle]").evaluate((button) => {
      const label = button.querySelector('[data-slot="trace-line"]').getBoundingClientRect();
      const arrow = button.lastElementChild.getBoundingClientRect();
      return arrow.left - label.right;
    });
    assert(disclosureGap >= 0 && disclosureGap <= 12, "disclosure sits beside its label");
    await page.locator("[data-chat-scroll]").evaluate((el) => { el.scrollTop = 0; });
    await page.screenshot({ path: join(out, `${name}.png`) });
    await page.locator("[data-chat-scroll]").evaluate((el) => { el.scrollTop = el.scrollHeight; });
    await page.locator("[data-chat-bottom]").waitFor({ state: "hidden" });
    await page.screenshot({ path: join(out, `${name}-approval.png`) });
    queue.push(note("agent_message_chunk", { content: { type: "text", text: "\n\n" + Array.from({ length: 35 }, (_, i) => `阅读记录 ${i + 1}。这里保留足够长的历史，用于检查展开与流式回复时的位置。`).join("\n\n") } }),
      note("tool_call", { toolCallId: "scroll-anchor", title: "Read scrolling details", kind: "read", status: "completed", rawInput: { path: "/work/scroll.txt" } }),
      note("agent_message_chunk", { content: { type: "text", text: "这是尾部阅读位置。" } }));
    const scrollTool = page.locator('[data-tool-id="scroll-anchor"]'), scrollToggle = scrollTool.locator("[data-tool-toggle]");
    await scrollToggle.waitFor();
    await scrollToggle.scrollIntoViewIfNeeded();
    await page.locator("[data-chat-scroll]").evaluate((el) => { el.scrollTop = el.scrollHeight; el.dispatchEvent(new Event("scroll")); });
    if (await page.locator("[data-chat-bottom]").count()) await page.locator("[data-chat-bottom]").click();
    const anchorTop = (await scrollToggle.boundingBox()).y;
    await scrollToggle.click();
    queue.push(note("tool_call_update", { toolCallId: "scroll-anchor", content: [{ type: "content", content: { type: "text", text: Array.from({ length: 40 }, (_, i) => `streamed output ${i}`).join("\n") } }] }));
    await scrollTool.getByText("streamed output 39", { exact: false }).waitFor();
    await page.waitForTimeout(300);
    const anchorAfter = (await scrollToggle.boundingBox()).y;
    assert(Math.abs(anchorAfter - anchorTop) <= 2, `manual disclosure keeps the chosen header in place during output updates: ${anchorTop} -> ${anchorAfter}, ${JSON.stringify(await page.locator('[data-chat-scroll]').evaluate((el) => ({ top: el.scrollTop, height: el.scrollHeight, view: el.clientHeight })))}`);
    await page.locator("[data-chat-bottom]").click();
    await page.emulateMedia({ reducedMotion: "no-preference" });
    queue.push({ kind: "turning" }, thought(Array.from({ length: 15 }, (_, i) => `流式思考 ${i}。`).join("\n\n")));
    await page.getByText("流式思考 14。", { exact: true }).waitFor();
    await page.waitForTimeout(350);
    const tailGap = await page.locator("[data-chat-scroll]").evaluate((el) => el.scrollHeight - el.clientHeight - el.scrollTop);
    assert(tailGap <= 2, `tail follows animated streaming growth, gap=${tailGap}`);
    await page.emulateMedia({ reducedMotion: "reduce" });
    const reading = page.getByText("阅读记录 18。这里保留足够长的历史，用于检查展开与流式回复时的位置。", { exact: true });
    await page.locator("[data-chat-scroll]").evaluate((el) => {
      const paragraph = [...el.querySelectorAll("p")].find((p) => p.textContent?.startsWith("阅读记录 18。"));
      el.scrollTop += paragraph.getBoundingClientRect().top - el.getBoundingClientRect().top - 80;
      el.dispatchEvent(new Event("scroll"));
    });
    const readingTop = (await reading.boundingBox()).y;
    queue.push(note("plan", { entries: Array.from({ length: 20 }, (_, i) => ({ content: `新增计划 ${i}`, status: "pending" })) }));
    await page.getByText("新增计划 19", { exact: true }).waitFor({ state: "attached" });
    await page.waitForTimeout(100);
    assert(Math.abs((await reading.boundingBox()).y - readingTop) <= 2, "reading history keeps its paragraph in place when the plan above changes height");
    // Different sessions may use the same numeric ask ID. The local
    // optimistic answer must not make the next session unanswerable.
    otherQueue = [{ kind: "history_end" }, { kind: "ask", ask: 31, title: "Another session", options: [["different", "另一会话的选项"]] }];
    if (width < 600) {
      const folded = page.locator("[data-session-bar] [data-deck-strip]");
      if (await folded.isVisible()) await folded.click();
      await page.locator("[data-session-deck] [data-back]").click();
    }
    await page.locator("[data-list] [data-row-open]").filter({ hasText: "Elements B" }).click();
    await page.locator('[data-ask-option="different"]').click();
    otherQueue.push({ kind: "answered", ask: 31, option: "different" });
    await page.waitForFunction(() => document.querySelector("[data-ask-answer]")?.textContent === "另一会话的选项");
    assert.deepEqual(requests.at(-1), { id: "gui/elements-b", ask: 31, option: "different" }, "answer state belongs to its session");
    assert.deepEqual(errors, []);
    results.push({ name, reasoning: "stream / manual override / keyboard", tool: "update / expand / output bound / no empty disclosure", diff: "real changes / independent line numbers / context model", scroll: "manual disclosure anchor / history anchor / animated tail following", plan: "live update preserves choice", permission: "submitting / waiting / retry / exact IDs / no duplicate / authoritative reply", overflow, errors });
    await context.close();
    console.log(`${name}: production ChatView interactions passed`);
  }
  writeFileSync(join(out, "results.json"), JSON.stringify(results, null, 2));
  console.log("AI Elements evidence:", out);
} finally {
  if (browser) await browser.close();
  if (vite) { vite.kill("SIGTERM"); if (vite.exitCode === null) await new Promise((resolve) => vite.once("exit", resolve)); }
}
