import labMedia from './data/media.generated.json';
// Data-only fixture for the complete production App; never imported by its normal entry.
import {navigationFixture} from './navigation';
import {pairingFixture} from './pairing';
import {labDevices,labModels,labQuota,labRoot,labUsage,labSettings,labAbout,labRelays,labRoadShare,labDoctor} from './world';
import {canonicalChatFixture} from '../../scripts/fixtures/canonical-chat-routes.mjs';
import type {DeviceRow} from '../../src/gen/bindings/DeviceRow';
import type {SessionRow} from '../../src/gen/bindings/SessionRow';
import type {Waiting} from '../../src/gen/bindings/Waiting';
import providers from '../../../agent/providers.json';

type Frame = Record<string, unknown>;
const update = (sessionUpdate: string, rest: Record<string, unknown>): Frame => ({
  kind: "note", update: { update: { sessionUpdate, ...rest } },
});
const text = (value: string) => ({ content: { type: "text", text: value } });
const ids = ["gui/ai-elements-review", "gui/ai-elements-plan", "gui/ai-elements-stream", "gui/codex-review", "wide/all"];
const titles = ["AI Elements · 组件总览", "AI Elements · 计划与确认", "AI Elements · 流式响应", "Codex · 工具与补丁", "khor · 网络任务"];
const device: DeviceRow = {
  id: "e".repeat(64), name: "验收环境", shown: "验收环境", label: null,
  me: true, paused: false, face: null, pinned: false, vitals: null,
  hop: null, heard_ms: null, rtt_ms: null, down_ms: null, last_rtt_ms: null,
  since_ms: null, last_hop: null, relay: null, via: null, sponsor: null, ssh_jump: null,
  roads: [], watch: [], watched: null, seat: {left: 50, top: 50}, hosts: true,
  receive_dir: null, receive_at: "/preview/files", receive_for: null, receive_why: null,
};
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
    update("tool_call", { toolCallId: "codex-patch", title: "fileChange", kind: "edit", status: "completed", locations: [{ path: "/preview/project/src/CodeEditor.tsx" }], rawOutput: { changes: [{ path: "/preview/project/src/CodeEditor.tsx", kind: { type: "update" }, diff: "@@ -12,4 +12,5 @@\n const extensions = [\n-  keymap.of(defaultKeymap),\n+  history(),\n+  keymap.of([...historyKeymap, ...defaultKeymap]),\n   languageSupport,\n ];" }] } }),
    update("plan", { entries: [{ content: "检查快捷键", status: "completed" }, { content: "验证修改", status: "completed" }] }),
    update("agent_message_chunk", text("检查完成。展开工具可看到执行结果与原始补丁；未把补丁片段当成完整文件。")),
  ];
  if (index === 2) return [boot(), update("user_message_chunk", text("演示一轮流式思考和回复。")),];
  if (index === 1) return [boot(),
    update("user_message_chunk", text("检查这次改动，并在发布前让我确认。")),
    update("agent_message_chunk", text("代码检查已经完成，正在等待最后一项确认。")),
    update("plan", { entries: [{ content: "读取改动和影响范围", status: "completed" }, { content: "运行前端检查", status: "completed" }, { content: "确认后执行下一步", status: "in_progress" }] }), { kind: "ask", ask: 201, title: "运行独立的原生 App 验收？这个示例只改变预览状态。", options: [["allow-once", "本次允许"], ["deny", "暂不运行"]] },
  ];
  return [boot(),
    update("user_message_chunk", text("修一下编辑器的撤销快捷键，切换文件后也要保留历史。")),
    update("agent_message_chunk", text("我先检查快捷键和文件切换时的状态保存。")),
    update("agent_thought_chunk", text("撤销应交给当前编辑器。先排查系统菜单是否截获了快捷键，再检查每个文件的历史是否独立。")),
    update("tool_call", { toolCallId: "preview-read", title: "Read CodeEditor.tsx", kind: "read", status: "completed", rawInput: { file_path: "/preview/project/src/CodeEditor.tsx" }, content: [{ type: "content", content: { type: "text", text: "Found editor extensions and per-file history state." } }] }),
    update("tool_call", { toolCallId: "preview-edit", title: "Edit CodeEditor.tsx", kind: "edit", status: "completed", content: [{ type: "diff", path: "/preview/project/src/CodeEditor.tsx", oldText: editorSource(false), newText: editorSource(true) }] }),
    update("tool_call", { toolCallId: "preview-test", title: "Run npm run editor-history", kind: "execute", status: "completed", rawInput: { command: "npm run editor-history" }, content: [{ type: "content", content: { type: "text", text: "✓ undo / redo\n✓ switch files\n✓ return from preview\n3 checks passed" } }] }),
    update("tool_call", { toolCallId: "preview-error", title: "Run native window check", kind: "execute", status: "failed", rawInput: { command: "npm run editor-history-native" }, content: [{ type: "content", content: { type: "text", text: "This preview does not start native applications.\nOpen the confirmation below to inspect the approval UI." } }] }),
    update("agent_message_chunk", text("快捷键和编辑历史已接好。\n\n- **⌘Z / ⇧⌘Z** 交给当前编辑器\n- 每个文件保留自己的撤销历史\n- 源码与预览切换不会重置历史\n\n浏览器检查已通过，原生窗口检查需要另行确认。")), { kind: "ask", ask: 101, title: "打开独立窗口，验证系统菜单的撤销与重做？", options: [["allow-once", "本次允许"], ["deny", "暂不运行"]] },
  ];
}

export function createLabFixture(terminal = true, options: {origin?: string; content?: boolean; items?:number; machines?:number; page?:boolean;pageSize?:number;settings?:boolean;notices?:boolean} = {}) {
  const devices = labDevices(device,options.machines ?? 3);
  const pairing = pairingFixture(devices, labDevices(device, 6));
  const navigation = navigationFixture(devices, options.origin ?? 'http://127.0.0.1:23776', options.content ?? false, options.items ?? 3, options.page ?? false);
  const quota = labQuota(devices);
  // Only Lab browser storage is touched; metadata comes from the native key table.
  const settingsKey = 'khor.lab.settings-values';
  const readSetting = (key:string) => { try { return localStorage.getItem(key); } catch { return null; } };
  // Match the synthetic review profile seeded by bootstrap, rather than
  // replacing its language/theme with the browser runner's environment.
  const settingsProfile = {lang:readSetting('khor.lang') ?? 'zh',theme:readSetting('khor.theme') ?? 'auto'};
  let settingsValues: Record<string,string> = {...settingsProfile};
  try { settingsValues = {...settingsProfile,...JSON.parse(readSetting(settingsKey) ?? '{}')}; } catch { /* Fresh isolated defaults. */ }
  const settingsRows = () => {
    // Component harnesses keep their existing minimal config answer. Only the
    // complete App Lab opts into rows generated for its source revision.
    if(!options.settings)return [];
    const lang = settingsValues.lang ?? 'auto';
    const language = lang === 'zh' || (lang === 'auto' && typeof navigator !== 'undefined' && navigator.language.startsWith('zh')) ? 'zh' : 'en';
    return labSettings[language].map(row => ({...row, value:settingsValues[row.name] ?? row.value}));
  };
  const terminalLeases = new Set<string>();
  let terminalLeaseNumber = 0;
  const preferences=ids.map((_,i)=>({
    correlatedSettings:true,
    model:i===3?'gpt-5.4':'claude-sonnet-4-6',
    modes:{currentModeId:'default',availableModes:[
      {id:'default',name:'逐次询问',description:'执行操作前向你确认'},
      {id:i===3?'workspace':'auto',name:i===3?'工作区内自动':'由 Claude 判断',description:i===3?'工作区内自动执行，超出范围仍需确认':'由 Claude 判断操作是否需要确认'},
      {id:'plan',name:'仅规划',description:'讨论和制定计划，不修改文件'},
    ]},
    options:[
      {id:'model',name:'模型',category:'model',type:'select',currentValue:i===3?'gpt-5.4':'claude-sonnet-4-6',options:i===3?[
        {value:'gpt-5.4',name:'GPT-5.4'},{value:'gpt-5.4-mini',name:'GPT-5.4 mini'},
      ]:[{value:'claude-sonnet-4-6',name:'Sonnet 4.6'},{value:'claude-opus-4-6',name:'Opus 4.6'}]},
      {id:'reasoning',name:'思考方式',category:'thought_level',type:'select',currentValue:'standard',options:[{value:'standard',name:'标准'},{value:'deep',name:'深入'}]},
    ],
  }));
  const initialPreferences=structuredClone(preferences);
  const sampleWithSettings=(i:number)=>{const source=sample(i);return [source[0],...(i===0?[update('user_message_chunk',{content:{type:'image',mimeType:'image/png',name:'设备协作示例.png',data:labMedia.image}})]:[]),{kind:'settings',settings:preferences[i]},...source.slice(1), ...(options.content ? [update('agent_message_chunk', text('Open Lab file: /preview/project/README.md\n\n[Open Lab page](' + options.origin + '/lab/example)'))] : [])];};
  let frames = ids.map((_, i) => sampleWithSettings(i));
  const makeCanonical = () => canonicalChatFixture({pageSize:options.pageSize??128,ids: ids.slice(0, 4),
    takeFrames: (id: string) => {
      const index = ids.indexOf(id);
      return frames[index].splice(0).filter(frame => frame.kind !== 'turning' || streaming.has(index));
    },
    onOpen: (id: string) => {const index = ids.indexOf(id);if (index === 2 && generation[index] === 0) run(index);},
  });
  let canonical = makeCanonical();
  const streaming = new Set<number>();
  const generation = ids.map(() => 0);
  const timers = new Set<ReturnType<typeof setTimeout>>();
  const initialKhorUpdates = [
    { sessionUpdate: "agent_thought_chunk", ...text("先查看网络中的设备，再读取目标机器的会话。") },
    { sessionUpdate: "tool_call", toolCallId: "khor-devices", title: "mcp__khor__devices", kind: "read", status: "completed", rawInput: {}, content: [{ type: "content", content: { type: "text", text: "验收环境 · online\n临时测试设备 · online" } }] },
    { sessionUpdate: "tool_call", toolCallId: "khor-sessions", title: "mcp__khor__sessions", kind: "read", status: "completed", rawInput: { machine: "临时测试设备" }, content: [{ type: "content", content: { type: "text", text: "fixture-session · idle" } }] },
    { sessionUpdate: "agent_message_chunk", ...text("**网络检查完成**。两台设备在线，目标机器有一个空闲会话。\n\n这是隔离示例，不连接实际设备或执行命令。") },
  ].map((u) => JSON.stringify(u));
  let khorUpdates = [...initialKhorUpdates];
  const account = { id: "fixture", name: "示例模型账户", home: device.id, home_name: device.name, provider: "anthropic", default_model: "claude-sonnet-4-6", configured: true, base_url: null, auth_type: "api_key" };
  const modelAccounts = [account,...quota.filter(row=>['deepseek','openrouter'].includes(row.reading.provider)).map(row=>({
    ...account,id:row.reading.account,name:row.reading.name,home:row.reading.home,home_name:row.home_name,
    provider:row.reading.provider,default_model:row.reading.provider==='deepseek' ? 'deepseek-chat' : 'openai/gpt-5.4',
  }))];
  let modelConfig = { home: device.id, account: account.id, model: account.default_model, options:{thought_level:"medium",auto_compaction:true} };
  let khorRunning=false,khorPrompt='看看网络里的设备与会话。';
  let khorApproval:{id:string;tool:string;arguments:string}|null=null;
  const khorAnswers:unknown[]=[];
  let jobCounter = 0;
  const jobs = new Map<string, any>();
  let failAcpSetup = false;
  const acpSetupCalls: {agent:string;machine:string;operation:string}[] = [];
  const interactionAnswers:unknown[]=[];
  const view = (data: unknown = null) => ({ config: modelConfig, accounts: modelAccounts, run: null, data });
  function agentAction(body: Record<string, any>) {
    if (body.action === "catalog") return view(providers);
    const p = body.payload ?? {};
    const models = labModels(modelAccounts.find(account=>account.id===(p.account??modelConfig.account))?.provider??'anthropic');
    if(body.action==='prompt'){khorRunning=true;khorPrompt=String(p.text);later(()=>{khorRunning=false;},10000);return view();}
    if(body.action==='stop'){khorRunning=false;khorApproval=null;return view();}
    if(body.action==='answer'&&p.run==='khor-preview-run'&&p.ask===khorApproval?.id){khorAnswers.push({ask:p.ask,allow:p.allow});khorApproval=null;khorRunning=false;return view();}
    if (body.action === "configure") { modelConfig = { home: body.machine, account: p.account, model: p.model,options:{thought_level:"medium",auto_compaction:true} }; return view(); }
    if (body.action !== "manage") return view();
    if (p.operation === 'acp_initialize' || p.operation === 'acp_authenticate') {
      acpSetupCalls.push({agent:String(p.agent),machine:String(body.machine),operation:p.operation});
      const id=`acp-${++jobCounter}`;
      jobs.set(id,failAcpSetup?{status:'failed',error:'Incoming transport closed: initialize',reason:'npm_cache_unavailable'}:
        {status:'done',result:{agentInfo:{name:String(p.agent).startsWith('codex')?'Codex':'Claude',version:'lab'},capabilities:{},authMethods:[]}});
      return view({job:id,status:'running'});
    }
    if (p.operation === 'acp_poll') return view(jobs.get(p.job)??{status:'failed',error:'示例已结束'});
    if (p.operation === 'acp_cancel') {jobs.delete(p.job);return view({status:'cancelled'});}
    if(p.operation==='session_settings'||p.operation==='session_set_option'){
      if(p.operation==='session_set_option'){
        if(p.configId==='model'){modelConfig.model=p.value;modelConfig.options.thought_level='medium';}
        else (modelConfig.options as Record<string,unknown>)[p.configId]=p.value;
      }
      const id=`session-${++jobCounter}`;
      jobs.set(id,{status:'done',result:{model:modelConfig.model,options:modelConfig.options,configOptions:[
        {id:'model',name:'模型',category:'model',type:'select',currentValue:modelConfig.model,options:models.map(model=>({value:model.id,name:model.name}))},
        {id:'thought_level',name:'思考强度',category:'thought_level',type:'select',currentValue:modelConfig.options.thought_level,options:['off','minimal','low','medium','high'].map(value=>({value,name:value}))},
        {id:'auto_compaction',name:'自动压缩',category:'model_config',type:'boolean',currentValue:modelConfig.options.auto_compaction},
      ]}});return view({job:id,status:'running'});
    }
    if (p.operation === "models") {
      const id = `models-${++jobCounter}`;
      jobs.set(id, { status: "done", result: { models } });
      return view({ job: id, status: "running" });
    }
    if (p.operation === "login") {
      const id = `login-${++jobCounter}`;
      jobs.set(id, { setup: p, status: "waiting", ask: 1, prompt: { type: p.authType === "oauth" ? "manual_code" : "secret", message: "隔离示例：请输入任意测试文字，不要输入真实凭据" } });
      return view({ job: id, status: "running" });
    }
    if (p.operation === "poll") { const { setup, ...safe } = jobs.get(p.job) ?? { status: "failed", error: "示例已结束" }; return view(safe); }
    if (p.operation === "reply") {
      const job = jobs.get(p.job); if (!job) return view({ status: "failed", error: "示例已结束" });
      const setup = job.setup, id = setup.account ?? `preview-${++jobCounter}`;
      const next = { ...account, id, name: setup.name, provider: setup.provider, base_url: setup.baseUrl || null, auth_type: setup.authType, default_model: "" };
      const index = modelAccounts.findIndex((a) => a.id === id); if (index >= 0) modelAccounts[index] = next; else modelAccounts.push(next);
      // Submitted values are deliberately discarded by this isolated boundary.
      jobs.set(p.job, { status: "done", account: id, accountInfo: next }); return view({ status: "running" });
    }
    if (p.operation === "cancel") { jobs.delete(p.job); return view({ status: "cancelled" }); }
    if (p.operation === "default") { const target = modelAccounts.find((a) => a.id === p.account); if (target) target.default_model = p.model; return view({ status: "done", account: p.account, accountInfo: target }); }
    return view();
  }
  const previewAt = Date.now();
  // Opt-in event data exercises the production App banner controller everywhere.
  // Default Lab scenes remain quiet; no alternate notification UI is mounted.
  const noticeEvents=(index:number):SessionRow['activity']=>{
    if(!options.notices||index>2)return [];
    const round=Math.floor((Date.now()-previewAt-2000)/18000);
    const offset=(index+2)%3;
    if(round<offset)return [];
    const last=round-(round-offset)%3;
    return [{id:`lab-notice:${index}:${last}`,at_ms:previewAt+2000+last*18000,
      kind:(["turn_completed","needs_input","failed"] as const)[index],resolved:false,
      text:null,duration_ms:null,exit:null}];
  };
  const khorReceipt = () => ({ id: "khor-preview-run", at_ms: previewAt, from: device.name, from_me: true, to: [], text: null, files: null, note: null, unknown: null, retracted: false,
    command: { id: "khor-preview-run", line: khorPrompt, owner: device.id, owner_name: device.name, status: khorApproval ? "blocked" : khorRunning ? "running" : "done", started_at_ms: Date.now() - 2000, ended_at_ms: khorRunning ? null : Date.now(), stdout: "", stderr: "", data_json: null, session: null, error: null,
      agent: { model: "fixture-model", approval: khorApproval, updated_at_ms: 100, updates: khorUpdates, updates_truncated: false } } });
  const later = (fn: () => void, ms: number) => {
    const timer = setTimeout(() => { timers.delete(timer); fn(); }, ms); timers.add(timer);
  };
  function run(index: number, message = "这是上游组件的流式回复。思考、工具结果与正文分别呈现，界面使用同一套组件和图标。") {
    if (streaming.has(index)) return;
    streaming.add(index); frames[index].push({ kind: "turning" });
    const revision = ++generation[index];
    const publish = (frame: Frame) => { if (generation[index] === revision) frames[index].push(frame); };
    const thought = ["先检查输入。", "\n确认消息、思考与工具调用各自的状态。", "\n使用上游的文字与展开方式，不替换成应用诊断图标。"];
    thought.forEach((part, i) => later(() => publish(update("agent_thought_chunk", text(part))), 300 + i * 650));
    const chunks = message.match(/.{1,5}/gu) ?? [];
    chunks.forEach((part, i) => later(() => publish(update("agent_message_chunk", text(part))), 2500 + i * 180));
    later(() => { if (generation[index] !== revision) return; frames[index].push({ kind: "turn", stop: "EndTurn" }); streaming.delete(index); }, 2800 + chunks.length * 180);
  }
  let pollMode='ok';let uploadMode='ok';const sentPrompts:unknown[]=[];
  let holdSettings=false,refuseSetting=false;const pendingSettings:(()=>void)[]=[];
  const applySetting=(apply:()=>void)=>{if(holdSettings)pendingSettings.push(apply);else apply();};
  return {
    dispose() { for (const timer of timers) clearTimeout(timer); timers.clear(); },
    request(path: string, body: Record<string, any> = {}): unknown {
      if (path === '__ai_preview/reset') {
          settingsValues={...settingsProfile};try { localStorage.removeItem(settingsKey); } catch { /* Server fixture has no browser storage. */ }
          holdSettings=false;refuseSetting=false;pendingSettings.length=0;
          failAcpSetup=false;acpSetupCalls.length=0;
          for (const timer of timers) clearTimeout(timer); timers.clear(); streaming.clear(); khorRunning=false; khorApproval=null; khorAnswers.length=0; generation.fill(0);
          preferences.splice(0,preferences.length,...structuredClone(initialPreferences));
          modelConfig={home:device.id,account:account.id,model:account.default_model,options:{thought_level:"medium",auto_compaction:true}};
          frames = ids.map((_, i) => sampleWithSettings(i));
          canonical = makeCanonical();
          khorUpdates = [...initialKhorUpdates];
        return true;
      }
      if(path==='__ai_preview/poll-mode'){pollMode=String(body.mode??'ok');return pollMode;}
      if(path==='__ai_preview/poll-state')return pollMode;
      if(path==='__ai_preview/upload-mode'){uploadMode=String(body.mode??'ok');return uploadMode;}
      if(path==='__ai_preview/upload-state')return uploadMode;
      if(path==='__ai_preview/says')return structuredClone(sentPrompts);
      if(path==='__ai_preview/settings-hold'){
        holdSettings=body.hold===true;
        if(!holdSettings)for(const apply of pendingSettings.splice(0))apply();
        return {pending:pendingSettings.length};
      }
      if(path==='__ai_preview/settings-pending')return pendingSettings.length;
      if(path==='__ai_preview/settings-refuse'){refuseSetting=body.refuse===true;return true;}
      if(path==='chat_put')return '/preview/uploads/'+String(body.name??'attachment');
      if(path==='chat_put_paths')return body.paths;
      if (path === '__ai_preview/acp-setup') {failAcpSetup=body.fail===true;return true;}
      if (path === '__ai_preview/acp-setup-calls') return structuredClone(acpSetupCalls);
      if (pairing.handles(path)) return structuredClone(pairing.request(path, body));
      if (navigation.handles(path)) return structuredClone(navigation.request(path, body));
      if (path === 'face_note') return null;
      if (terminal && body.id === 'shell/layout-lab' && path === 'attachment_begin' && body.kind === 'terminal') {
        const lease = 'terminal-fixture-' + ++terminalLeaseNumber;
        terminalLeases.add(lease);
        return lease;
      }
      if (terminal && String(body.id) === 'shell/layout-lab' && path.startsWith('term_')) {
        if (path === 'term_ended') return null;
        if (!terminalLeases.has(body.lease)) throw new Error('Unknown fixture terminal attachment');
        if (path === 'term_leave') terminalLeases.delete(body.lease);
        const screen = '\x1b[2J\x1b[Hkhor / layout-lab\r\n\r\n$ git status --short\r\n  clean working tree\r\n\r\nIsolated terminal: input is not executed.\r\npreview % ';
        return path === 'term_attach' ? {bytes: btoa(screen), seq: 1}
          : ['term_stream', 'term_wait'].includes(path) ? {bytes: '', seq: 1, gone: false, reset: false, stall: null} : null;
      }
      const projected = canonical.answer(path, body);
      if (projected !== null) {
        try { return structuredClone(projected.value); } finally { projected.finish?.(); }
      }
        const index = Math.max(0, ids.indexOf(String(body.id)));
        let answer: unknown = null;
        switch (path) {
          case "__ai_preview/khor-approval":khorRunning=body.pending!==false;khorApproval=body.pending===false?null:{id:`approval-${++jobCounter}`,tool:"close",arguments:JSON.stringify({session:"fixture-session",machine:"临时测试设备"})};answer=khorApproval;break;
          case "__ai_preview/khor-long": khorUpdates = [...initialKhorUpdates, JSON.stringify({ sessionUpdate: "agent_message_chunk", ...text(String(body.text ?? "")) })]; answer = true; break;
          case "__ai_preview/khor-answers":answer=khorAnswers;break;
          case "__ai_preview/acp": frames[index].push(...(Array.isArray(body.frames)?body.frames:[]));answer=true;break;
          case "__ai_preview/answers":answer=interactionAnswers;break;
          case "chat_respond":interactionAnswers.push(body);frames[index].push({kind:'elicit_answered',ask:body.ask,response:{action:'accept'}});break;
          case "chat_control":{
            if(body.operation==='set_option'||body.operation==='set_mode'){
              const refused=refuseSetting;
              applySetting(()=>{
                if(!refused){const payload=body.payload??{};
                  if(body.operation==='set_mode')preferences[index].modes.currentModeId=String(payload.mode);
                  else{const option=preferences[index].options.find(option=>option.id===payload.option);if(option)(option as {currentValue:unknown}).currentValue=payload.value;if(payload.option==='model')preferences[index].model=String(payload.value);}
                  frames[index].push({kind:'settings',settings:structuredClone(preferences[index])});
                }
                frames[index].push({kind:'control_result',token:body.token,operation:body.operation,data:null,error:refused?'setting-refused':null});
              });
            }else frames[index].push({kind:'control_result',token:body.token,operation:body.operation,data:{},error:null});break;
          }
          case "sessions": {
            const rows: SessionRow[] = ids.map((id, i) => ({id, kind: i === 4 ? 'broadcast' : 'gui', title: titles[i],
              activity: noticeEvents(i), observation_owner: options.notices?null:device.id, content_locator: null, imported: false, quiet: false,
              word: streaming.has(i)||i===2 ? 'busy' : i===1 ? 'blocked' : 'idle', at_ms: previewAt-i*45_000, unread:i===1 ? 3 : 0, source: null, pinned:i===0,
              stale: false, category: i === 4 ? 'khor' : i === 3 ? 'codex' : 'claude', group: '', face: devices[i%Math.max(1,devices.length)]?.face??null,
              hosted: true, can_control: true, attachable: false, last: '隔离示例 · 不连接真实智能体',
              via: null, via_face: null, home: devices[i%Math.max(1,devices.length)]?.name??device.name, execution_home: devices[i%Math.max(1,devices.length)]?.id??device.id, place:labRoot(devices[i%Math.max(1,devices.length)])+'/project', doing: null, mux: false, ended: null}));
            if (terminal) rows.push({...rows[0], id: 'shell/layout-lab', kind: 'shell', title: '终端 · 布局试验',
              category: 'shell', activity:[], attachable: true, last: '隔离终端示例'});
            answer = devices.length ? rows : [];
            break;
          }
          case 'session_recovery': answer = {id: body.id, home: device.id, current_run: 'fixture-live', next_attempt_epoch: null,
            execution: 'live', action: 'join', native: null, previous: null, reason: null}; break;
          case 'sync_now': answer = []; break;
          case 'day_strips': case 'page_icons': answer = {}; break;
          case 'face_here': break;
          case "devices": answer = devices; break;
          case "pending_list": answer = {rows:[],holds:false} satisfies Waiting; break;
          case "machine_log": answer = { messages: [khorReceipt()], broken: 0 }; break;
          case "khor_agent": answer = agentAction(body); break;
          case "console_catalog": answer = []; break;
          case "launch_presets": answer = []; break;
          case "chat_set_mode": applySetting(()=>{preferences[index].modes.currentModeId=String(body.mode);frames[index].push({kind:'settings',settings:structuredClone(preferences[index])});});break;
          case "chat_set_option": {
            applySetting(()=>{
            const option=preferences[index].options.find(o=>o.id===body.option);
            if(option)(option as {currentValue:unknown}).currentValue=body.value;
            if(body.option==='model')preferences[index].model=String(body.value);
            frames[index].push({kind:'settings',settings:structuredClone(preferences[index])});});break;
          }
          case "chat_answer": later(() => frames[index].push({ kind: "answered", ask: body.ask, option: body.option }, update("agent_message_chunk", text(body.option === "deny" ? "已取消。这次预览没有执行任何系统操作。" : "已确认。此处仅演示确认状态，不执行系统操作。"))), 900); break;
          case "chat_say": sentPrompts.push(structuredClone(body));run(index, "收到。这是验收环境的示例回复，没有调用模型或执行命令。你可以展开上面的工具调用，检查输出与复制按钮。"); break;
          case "chat_stop": generation[index]++; streaming.delete(index); frames[index].push({ kind: "turn", stop: "Cancelled" }); break;
          // Explicit no-PIN fixture facts; a failed read never unlocks production.
          case "lock_state": answer = {on: false, has_pin: false, timeout_min: 0, timeouts: [0, 1, 5, 15, 60], pin_min: 6, system_door: false}; break;
          case "quota_board": answer = quota; break;
          case "usage": answer = labUsage(); break;
          case "quota": answer = { quota:quota[0] ? {windows:quota[0].reading.windows,as_of:null,plan:'Max 20x',credits:null,fill:.36,source:'lab',limit_reached:false} : null,trouble:null }; break;
          case "agents": answer = [{name:'codex-acp',command:'npx -y @agentclientprotocol/codex-acp@2.2.2'},
            {name:'claude-acp',command:'npx -y @agentclientprotocol/claude-agent-acp@0.94.0'}];break;
          case "about": answer=labAbout;break;
          case "relays": answer=labRelays;break;
          case "road_share": answer=labRoadShare;break;
          case "doctor": answer=labDoctor;break;
          case "config": answer = settingsRows(); break;
          case "set_config": {
            const row = settingsRows().find(row => row.name === body.name);
            if(!row?.choices.some(choice => choice.value === body.value))throw new Error('Invalid Lab setting');
            settingsValues[body.name] = body.value;
            try { localStorage.setItem(settingsKey,JSON.stringify(settingsValues)); } catch { /* Server fixture keeps its values in memory. */ }
            answer=settingsRows();break;
          }
          case "dir_pins": case "web_pins": answer = []; break;
          case "seen_snapshot": case "seen": case "session_seen": break;
          default: throw new Error(`Lab fixture does not implement ${path}`);
        }
      return structuredClone(answer);
    },
  };
}
