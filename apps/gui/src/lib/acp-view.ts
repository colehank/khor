/** ACP v1 display projection shared by hosted sessions and durable khor turns. */
export type AcpDiff = { path: string; old: string | null; next: string };
export type AcpPatch = { path: string; text: string };
export type AcpTool = {
  id: string; title: string; kind: string; status: string; raw: unknown;
  where: string[]; diff: AcpDiff | null; out: string | null;
  diffs?: AcpDiff[]; patches?: AcpPatch[];
};
export type AcpCompaction = {who:"compaction";id:string;status:string;summary:string;error:string|null};
export type AcpResource = {who:"resource";kind:string;name:string;uri?:string;data?:string;mime?:string};
export type AcpItem = AcpCompaction | AcpResource | { who: "user" | "agent" | "thought"; text: string; messageId?: string } | (AcpTool & { who: "tool" });

// Count the JSON data's actual two-space display before materializing it.
// Transport values are plain JSON; non-JSON/cyclic input cannot claim a
// complete bounded projection. The normal renderer retains JSON.stringify.
function prettyInputFits(value: unknown, limit: number): boolean {
  let bytes = 0;
  const add = (size: number) => (bytes += size) <= limit;
  const string = (text: string) => {
    if (!add(2)) return false;
    for (const char of text) {
      const point = char.codePointAt(0)!;
      const escaped = char === '"' || char === '\\' || '\b\t\n\f\r'.includes(char);
      const size = escaped ? 2 : point < 32 || (point >= 0xd800 && point <= 0xdfff) ? 6
        : point < 0x80 ? 1 : point < 0x800 ? 2 : point < 0x10000 ? 3 : 4;
      if (!add(size)) return false;
    }
    return true;
  };
  const ancestors = new Set<object>();
  const walk = (entry: unknown, depth: number): boolean => {
    if (typeof entry === 'string') return string(entry);
    if (entry === null || entry === undefined) return add(4);
    if (typeof entry === 'boolean') return add(entry ? 4 : 5);
    if (typeof entry === 'number') return add(Number.isFinite(entry) ? String(entry).length : 4);
    if (typeof entry !== 'object' || depth >= 128 || ancestors.has(entry)) return false;
    const array = Array.isArray(entry);
    if (!array && Object.getPrototypeOf(entry) !== Object.prototype && Object.getPrototypeOf(entry) !== null) return false;
    ancestors.add(entry);
    if (!add(1)) return false;
    const keys = array ? null : Object.keys(entry).filter(key => {
      const type = typeof (entry as Record<string, unknown>)[key];
      return type !== 'undefined' && type !== 'function' && type !== 'symbol';
    });
    const count = array ? entry.length : keys!.length;
    for (let index = 0; index < count; index++) {
      if ((index && !add(1)) || !add(1 + (depth + 1) * 2)) return false;
      if (keys && (!string(keys[index]) || !add(2))) return false;
      const child = array ? entry[index] : (entry as Record<string, unknown>)[keys![index]];
      if (!walk(child, depth + 1)) return false;
    }
    ancestors.delete(entry);
    return (!count || add(1 + depth * 2)) && add(1);
  };
  return walk(value, 0);
}

/** One definition of the input text used by tool rendering, copying and
 * source search. A finite budget rejects before constructing pretty JSON. */
export function toolInputText(raw: unknown, maxBytes = Infinity): {
  text: string | undefined; command: boolean; complete: boolean;
} {
  const command = raw && typeof raw === 'object' && typeof (raw as Record<string, unknown>).command === 'string'
    ? (raw as { command: string }).command : null;
  if (command !== null) {
    if (Number.isFinite(maxBytes)) {
      let bytes = 0;
      for (const char of command) {
        const point = char.codePointAt(0)!;
        bytes += point < 0x80 ? 1 : point < 0x800 ? 2 : point < 0x10000 ? 3 : 4;
        if (bytes > maxBytes) return { text: undefined, command: Boolean(command), complete: false };
      }
    }
    return { text: command, command: Boolean(command), complete: true };
  }
  if (raw !== undefined && Number.isFinite(maxBytes) && !prettyInputFits(raw, maxBytes))
    return { text: undefined, command: false, complete: false };
  return { text: JSON.stringify(raw, null, 2), command: false, complete: true };
}
export type AcpUpdate = {
  messageId?: string;
  compactionId?:string;summary?:{type?:string;text?:string}[]|null;error?:string|null;
  sessionUpdate?: string; toolCallId?: string; title?: string | null; kind?: string; status?: string;
  rawInput?: unknown; rawOutput?: unknown; locations?: { path?: string; line?: number }[];
  content?: { type?: string; text?: string; uri?:string;name?:string;data?:string;mimeType?:string;resource?:{text?:string;uri?:string;mimeType?:string} } | { type?: string; content?: { type?: string; text?: string }; path?: string; oldText?: string | null; newText?: string }[] | null;
  entries?: { content?: string; status?: string; priority?: string }[];
};

function details(u: AcpUpdate) {
  const content = Array.isArray(u.content) ? u.content : [];
  const diffs: AcpDiff[] = content.filter((c) => c.type === "diff" && typeof c.newText === "string").map((c) => ({ path: c.path ?? "", old: c.oldText ?? null, next: c.newText! }));
  const raw = u.rawOutput && typeof u.rawOutput === "object" ? u.rawOutput as Record<string, unknown> : {};
  // Codex supplies unified patches, not full before/after files. Preserve that
  // distinction instead of treating a patch fragment as a complete new file.
  const patches: AcpPatch[] = (Array.isArray(raw.changes) ? raw.changes : []).filter((c) => c && typeof c.path === "string" && typeof c.diff === "string").map((c) => ({ path: c.path, text: c.diff }));
  const texts = content.flatMap((c) => c.type === "content" && c.content?.type === "text" && typeof c.content.text === "string" ? [c.content.text] : []);
  const rawText = "content" in u || u.rawOutput == null ? null : typeof u.rawOutput === "string" ? u.rawOutput : JSON.stringify(u.rawOutput, null, 2);
  return { diffs, patches, out: texts.length ? texts.join("\n") : patches.length ? null : rawText };
}

export function applyAcpUpdate<T extends { who: string }>(out: (AcpItem | T)[], u: AcpUpdate, paintUser = true): boolean {
  if ((u.sessionUpdate==='compaction_update'||u.sessionUpdate==='compaction_summary_chunk')&&u.compactionId) {
    const at=out.findIndex(i=>i.who==='compaction'&&'id' in i&&i.id===u.compactionId);
    const prior: AcpCompaction=at<0?{who:'compaction',id:u.compactionId,status:'in_progress',summary:'',error:null}:out[at] as AcpCompaction;
    const next={...prior};
    if(u.sessionUpdate==='compaction_summary_chunk')next.summary+=!Array.isArray(u.content)&&u.content?.type==='text'?u.content.text??'':'';
    else {
      if(u.status)next.status=u.status;
      if('summary' in u)next.summary=(u.summary??[]).map(v=>v.type==='text'?v.text??'':'').join('');
      if('error' in u)next.error=u.error??null;
    }
    if(at<0)out.push(next);else out[at]=next;
    return true;
  }
  if (["agent_message_chunk", "agent_thought_chunk", "user_message_chunk"].includes(u.sessionUpdate ?? "")) {
    const who = u.sessionUpdate === "agent_message_chunk" ? "agent" : u.sessionUpdate === "agent_thought_chunk" ? "thought" : "user";
    const text = !Array.isArray(u.content) && u.content?.type === "text" ? u.content.text ?? "" : "";
    if(who==='user'&&!paintUser)return false;
    if(!text){
      const c=!Array.isArray(u.content)?u.content:null;
      if(!c||c.type==='text')return false;
      out.push({who:'resource',kind:c.type??'unknown',name:c.name??c.resource?.uri??c.uri??c.mimeType??'附件',uri:c.uri??c.resource?.uri,data:c.data,mime:c.mimeType});
      if(c.resource?.text)out.push({who,text:c.resource.text});
      return true;
    }
    const last = out[out.length - 1];
    const messageId = typeof u.messageId === "string" ? u.messageId : undefined;
    const priorId = last && "messageId" in last ? last.messageId : undefined;
    // Omission reports no new identity; only two different explicit IDs
    // establish a message boundary. Remember a newly reported ID thereafter.
    if (last?.who === who && "text" in last && typeof last.text === "string"
      && (priorId === undefined || messageId === undefined || priorId === messageId))
      out[out.length - 1] = { ...last, text: last.text + text, ...(messageId === undefined ? {} : { messageId }) };
    else out.push({ who, text, ...(messageId === undefined ? {} : { messageId }) });
    return true;
  }
  if (u.sessionUpdate !== "tool_call" && u.sessionUpdate !== "tool_call_update") return false;
  if (!u.toolCallId) return false;
  const data = details(u);
  if (u.sessionUpdate === "tool_call") {
    out.push({ who: "tool", id: u.toolCallId, title: u.title ?? "", kind: u.kind ?? "other", status: u.status ?? "pending", raw: u.rawInput ?? null,
      where: (u.locations ?? []).flatMap((l) => l.path ? [l.path] : []), diff: data.diffs[0] ?? null, ...data });
    return true;
  }
  for (let i = out.length - 1; i >= 0; i--) {
    if (out[i].who !== "tool" || (out[i] as AcpTool).id !== u.toolCallId) continue;
    const at = out[i] as AcpTool & { who: "tool" };
    const replacesContent = "content" in u;
    const clearsContent = replacesContent && (u.content == null || (Array.isArray(u.content) && u.content.length === 0));
    out[i] = { ...at, title: u.title ?? at.title, kind: u.kind ?? at.kind, status: u.status ?? at.status, raw: u.rawInput ?? at.raw,
      where: u.locations ? u.locations.flatMap((l) => l.path ? [l.path] : []) : at.where,
      // A reported edit remains a transcript artifact when Claude replaces
      // its content with a textual execution receipt. Explicit clears remove it.
      diff: clearsContent ? null : data.diffs[0] ?? at.diff,
      diffs: clearsContent ? [] : data.diffs.length ? data.diffs : at.diffs,
      patches: u.rawOutput != null ? data.patches : at.patches,
      out: replacesContent || u.rawOutput != null ? data.out : at.out };
    return true;
  }
  return false;
}

export function projectAcp(updates: readonly string[]) {
  const items: AcpItem[] = [];
  let plan: { content: string; status: string }[] = [];
  for (const encoded of updates) {
    try {
      const u = JSON.parse(encoded) as AcpUpdate;
      if (!u || typeof u !== "object") continue;
      if (u.sessionUpdate === "plan" && Array.isArray(u.entries)) plan = u.entries.map((e) => ({ content: e.content ?? "", status: e.status ?? "pending" }));
      else applyAcpUpdate(items, u);
    } catch { /* An older or damaged optional display record must not lose the turn. */ }
  }
  return { items, plan };
}
