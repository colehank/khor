// A read-only fixture bridge in front of the production App. Never
// installed in the normal Vite config, native app, or production build.
import { defineConfig, mergeConfig, type Plugin } from "vite";
import base from "../vite.config";

type Frame = Record<string, unknown>;
const update = (sessionUpdate: string, rest: Record<string, unknown>, history = true): Frame => ({
  kind: history ? "history" : "note", update: { update: { sessionUpdate, ...rest } },
});
const text = (value: string) => ({ content: { type: "text", text: value } });
const ids = ["gui/ai-elements-review", "gui/ai-elements-plan", "gui/ai-elements-stream", "gui/codex-review", "wide/all"];
const titles = ["AI Elements · 组件总览", "AI Elements · 计划与确认", "AI Elements · 流式响应", "Codex · 工具与补丁", "khor · 网络任务"];
const device = { id: "e".repeat(64), name: "验收环境", me: true, face: null, pinned: false,
  vitals: null, hop: null, sponsor: null, watch: [], watched: null, seat: { left: 50, top: 50 }, hosts: true, receive_at: "/preview/files" };
const boot = (): Frame => ({ kind: "agent", name: "Claude", replays: true, needs_login: false, images: false, embeds: false, lists: false });
const editorSource = (updated: boolean) => [
  'import { EditorState } from "@codemirror/state";',
  'import { keymap } from "@codemirror/view";',
  'import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";',
  "", "const preferences = {", "  lineNumbers: true,", "  lineWrapping: true,", "  tabSize: 2,", "  autoSave: true,", "};", "",
  "const extensions = [", ...(updated ? ["  history(),", "  keymap.of([...historyKeymap, ...defaultKeymap]),"] : ["  keymap.of(defaultKeymap),"]),
  "  languageSupport,", "];", "", "export function createEditor(doc: string) {", "  return EditorState.create({", "    doc,", "    extensions,", "  });", "}", "",
].join("\n");
function sample(index: number): Frame[] {
  if (index === 3) return [{ ...boot(), name: "Codex" },
    update("user_message_chunk", text("检查结果，并展示实际修改的补丁。")),
    update("agent_thought_chunk", text("工具与补丁来自 Codex app-server 的结构化事件；补丁保留原来的范围与行号。")),
    update("tool_call", { toolCallId: "codex-command", title: "npm run editor-history", kind: "execute", status: "in_progress", rawInput: { command: "npm run editor-history", cwd: "/preview" } }),
    update("tool_call_update", { toolCallId: "codex-command", status: "completed", content: [{ type: "content", content: { type: "text", text: "3 checks passed" } }] }),
    update("tool_call", { toolCallId: "codex-patch", title: "fileChange", kind: "edit", status: "completed", locations: [{ path: "/preview/src/CodeEditor.tsx" }], rawOutput: { changes: [{ path: "/preview/src/CodeEditor.tsx", kind: { type: "update" }, diff: "@@ -12,4 +12,5 @@\n const extensions = [\n-  keymap.of(defaultKeymap),\n+  history(),\n+  keymap.of([...historyKeymap, ...defaultKeymap]),\n   languageSupport,\n ];" }] } }),
    update("plan", { entries: [{ content: "检查快捷键", status: "completed" }, { content: "验证修改", status: "completed" }] }),
    update("agent_message_chunk", text("检查完成。展开工具可看到执行结果与原始补丁；未把补丁片段当成完整文件。")), { kind: "history_end" },
  ];
  if (index === 2) return [boot(), update("user_message_chunk", text("演示一轮流式思考和回复。")), { kind: "history_end" }];
  if (index === 1) return [boot(),
    update("user_message_chunk", text("检查这次改动，并在发布前让我确认。")),
    update("agent_message_chunk", text("代码检查已经完成，正在等待最后一项确认。")),
    update("plan", { entries: [{ content: "读取改动和影响范围", status: "completed" }, { content: "运行前端检查", status: "completed" }, { content: "确认后执行下一步", status: "in_progress" }] }),
    { kind: "history_end" }, { kind: "ask", ask: 201, title: "运行独立的原生 App 验收？这个示例只改变预览状态。", options: [["allow-once", "本次允许"], ["deny", "暂不运行"]] },
  ];
  return [boot(),
    update("user_message_chunk", text("修一下编辑器的撤销快捷键，切换文件后也要保留历史。")),
    update("agent_message_chunk", text("我先检查快捷键和文件切换时的状态保存。")),
    update("agent_thought_chunk", text("撤销应交给当前编辑器。先排查系统菜单是否截获了快捷键，再检查每个文件的历史是否独立。")),
    update("tool_call", { toolCallId: "preview-read", title: "Read CodeEditor.tsx", kind: "read", status: "completed", rawInput: { file_path: "/preview/src/CodeEditor.tsx" }, content: [{ type: "content", content: { type: "text", text: "Found editor extensions and per-file history state." } }] }),
    update("tool_call", { toolCallId: "preview-edit", title: "Edit CodeEditor.tsx", kind: "edit", status: "completed", content: [{ type: "diff", path: "/preview/src/CodeEditor.tsx", oldText: editorSource(false), newText: editorSource(true) }] }),
    update("tool_call", { toolCallId: "preview-test", title: "Run npm run editor-history", kind: "execute", status: "completed", rawInput: { command: "npm run editor-history" }, content: [{ type: "content", content: { type: "text", text: "✓ undo / redo\n✓ switch files\n✓ return from preview\n3 checks passed" } }] }),
    update("tool_call", { toolCallId: "preview-error", title: "Run native window check", kind: "execute", status: "failed", rawInput: { command: "npm run editor-history-native" }, content: [{ type: "content", content: { type: "text", text: "This preview does not start native applications.\nOpen the confirmation below to inspect the approval UI." } }] }),
    update("agent_message_chunk", text("快捷键和编辑历史已接好。\n\n- **⌘Z / ⇧⌘Z** 交给当前编辑器\n- 每个文件保留自己的撤销历史\n- 源码与预览切换不会重置历史\n\n浏览器检查已通过，原生窗口检查需要另行确认。")),
    { kind: "history_end" }, { kind: "ask", ask: 101, title: "打开独立窗口，验证系统菜单的撤销与重做？", options: [["allow-once", "本次允许"], ["deny", "暂不运行"]] },
  ];
}

function previewBridge(): Plugin {
  let frames = ids.map((_, i) => sample(i));
  const streaming = new Set<number>();
  const generation = ids.map(() => 0);
  const timers = new Set<ReturnType<typeof setTimeout>>();
  const khorUpdates = [
    { sessionUpdate: "agent_thought_chunk", ...text("先查看网络中的设备，再读取目标机器的会话。") },
    { sessionUpdate: "tool_call", toolCallId: "khor-devices", title: "mcp__khor__devices", kind: "read", status: "completed", rawInput: {}, content: [{ type: "content", content: { type: "text", text: "验收环境 · online\n临时测试设备 · online" } }] },
    { sessionUpdate: "tool_call", toolCallId: "khor-sessions", title: "mcp__khor__sessions", kind: "read", status: "completed", rawInput: { machine: "临时测试设备" }, content: [{ type: "content", content: { type: "text", text: "fixture-session · idle" } }] },
    { sessionUpdate: "agent_message_chunk", ...text("**网络检查完成**。两台设备在线，目标机器有一个空闲会话。\n\n这是隔离示例，不连接实际设备或执行命令。") },
  ].map((u) => JSON.stringify(u));
  const previewAt = Date.now();
  const khorReceipt = () => ({ id: "khor-preview-run", at_ms: previewAt, from: device.name, from_me: true, to: [], text: null, files: null, note: null, unknown: null, retracted: false,
    command: { id: "khor-preview-run", line: "看看网络里的设备与会话。", owner: device.id, owner_name: device.name, status: "done", started_at_ms: Date.now() - 2000, ended_at_ms: Date.now(), stdout: "", stderr: "", data_json: null, session: null, error: null,
      agent: { model: "fixture-model", approval: null, updated_at_ms: 100, updates: khorUpdates, updates_truncated: false } } });
  const later = (fn: () => void, ms: number) => {
    const timer = setTimeout(() => { timers.delete(timer); fn(); }, ms); timers.add(timer);
  };
  function run(index: number, message = "这是上游组件的流式回复。思考、工具结果与正文分别呈现，界面使用同一套组件和图标。") {
    if (streaming.has(index)) return;
    streaming.add(index); frames[index].push({ kind: "turning" });
    const revision = ++generation[index];
    const publish = (frame: Frame) => { if (generation[index] === revision) frames[index].push(frame); };
    const thought = ["先检查输入。", "\n确认消息、思考与工具调用各自的状态。", "\n使用上游的文字与展开方式，不替换成应用诊断图标。"];
    thought.forEach((part, i) => later(() => publish(update("agent_thought_chunk", text(part), false)), 300 + i * 650));
    const chunks = message.match(/.{1,5}/gu) ?? [];
    chunks.forEach((part, i) => later(() => publish(update("agent_message_chunk", text(part), false)), 2500 + i * 180));
    later(() => { if (generation[index] !== revision) return; frames[index].push({ kind: "turn", stop: "EndTurn" }); streaming.delete(index); }, 2800 + chunks.length * 180);
  }
  return {
    name: "ai-elements-isolated-preview",
    transformIndexHtml() {
      return [{ tag: "script", injectTo: "head-prepend", children: `
        document.title = 'khor · AI Elements 验收（示例会话）';
        localStorage.setItem('khor.lang', 'zh');
        const query = new URLSearchParams(location.search), theme = query.get('theme');
        if (['light','dark','auto'].includes(theme)) localStorage.setItem('khor.theme', theme);
        if (!localStorage.getItem('khor.sessions.tabs')) localStorage.setItem('khor.sessions.tabs', JSON.stringify({tabs:['${ids[0]}'],active:'${ids[0]}'}));
      ` }];
    },
    configureServer(server) {
      server.httpServer?.on("close", () => { for (const timer of timers) clearTimeout(timer); });
      server.middlewares.use(async (req, res, next) => {
        const path = (req.url ?? "").split("?")[0].slice(1);
        if (path === "__ai_preview/reset") {
          for (const timer of timers) clearTimeout(timer); timers.clear(); streaming.clear(); frames = ids.map((_, i) => sample(i));
          res.setHeader("content-type", "application/json"); res.end("true"); return;
        }
        if (req.method !== "POST") return next();
        let body: Record<string, unknown>;
        try {
          let raw = ""; for await (const chunk of req) { raw += String(chunk); if (raw.length > 1000000) throw new Error("Preview request too large"); }
          body = JSON.parse(raw || "{}");
        } catch { res.statusCode = 400; res.end('"Invalid preview request"'); return; }
        const index = Math.max(0, ids.indexOf(String(body.id)));
        let answer: unknown = null;
        switch (path) {
          case "sessions": answer = ids.map((id, i) => ({ id, kind: i === 4 ? "broadcast" : "gui", title: titles[i], word: streaming.has(i) ? "busy" : "idle", at_ms: Date.now(), unread: 0, source: null, pinned: false, stale: false, category: i === 4 ? "khor" : i === 3 ? "codex" : "claude", group: "", face: null, hosted: true, attachable: false, last: "隔离示例 · 不连接真实智能体", via: null, via_face: null, home: device.name })); break;
          case "devices": answer = [device]; break;
          case "machine_log": answer = { messages: [khorReceipt()], broken: 0 }; break;
          case "khor_agent": answer = { config: { home: device.id, account: "fixture", model: "fixture-model" }, accounts: [{ id: "fixture", name: "隔离测试账户", home: device.id, home_name: device.name }], run: null }; break;
          case "console_catalog": answer = []; break;
          case "chat_open": if (index === 2 && frames[index].length === 3) run(index); answer = true; break;
          case "chat_replay": break;
          case "chat_poll": case "chat_wait": {
            const since = Math.max(0, Number(body.since) || 0);
            answer = { frames: frames[index].slice(since), next: frames[index].length, gone: false, lost: false, stall: null }; break;
          }
          case "chat_answer": later(() => frames[index].push({ kind: "answered", ask: body.ask, option: body.option }, update("agent_message_chunk", text(body.option === "deny" ? "已取消。这次预览没有执行任何系统操作。" : "已确认。此处仅演示确认状态，不执行系统操作。"), false)), 900); break;
          case "chat_say": run(index, "收到。这是验收环境的示例回复，没有调用模型或执行命令。你可以展开上面的工具调用，检查输出与复制按钮。"); break;
          case "chat_stop": generation[index]++; streaming.delete(index); frames[index].push({ kind: "turn", stop: "Cancelled" }); break;
          case "tabs_list": answer = { tabs: [], active: "" }; break;
          case "hooks_state": answer = { installed: false }; break;
          case "quota": answer = { quota: null, trouble: null }; break;
          case "config": case "dir_pins": case "web_pins": case "recents": case "workspaces": case "agents": case "quota_board": answer = []; break;
          case "seen_snapshot": case "seen": case "session_seen": case "chat_leave": break;
          default: res.statusCode = 400; answer = `验收环境未提供 ${path}`;
        }
        res.setHeader("content-type", "application/json; charset=utf-8");
        res.setHeader("cache-control", "no-store"); res.end(JSON.stringify(answer));
      });
    },
  };
}

export default mergeConfig(base, defineConfig({ plugins: [previewBridge()], server: { host: "127.0.0.1", port: 23776, strictPort: true } }));
