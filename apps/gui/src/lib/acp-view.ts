/** ACP v1 display projection shared by hosted sessions and durable khor turns. */
export type AcpDiff = { path: string; old: string | null; next: string };
export type AcpPatch = { path: string; text: string };
export type AcpTool = {
  id: string; title: string; kind: string; status: string; raw: unknown;
  where: string[]; diff: AcpDiff | null; out: string | null;
  diffs?: AcpDiff[]; patches?: AcpPatch[];
};
export type AcpItem = { who: "user" | "agent" | "thought"; text: string } | (AcpTool & { who: "tool" });
export type AcpUpdate = {
  sessionUpdate?: string; toolCallId?: string; title?: string | null; kind?: string; status?: string;
  rawInput?: unknown; rawOutput?: unknown; locations?: { path?: string; line?: number }[];
  content?: { type?: string; text?: string } | { type?: string; content?: { type?: string; text?: string }; path?: string; oldText?: string | null; newText?: string }[] | null;
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
  if (["agent_message_chunk", "agent_thought_chunk", "user_message_chunk"].includes(u.sessionUpdate ?? "")) {
    const who = u.sessionUpdate === "agent_message_chunk" ? "agent" : u.sessionUpdate === "agent_thought_chunk" ? "thought" : "user";
    const text = !Array.isArray(u.content) && u.content?.type === "text" ? u.content.text ?? "" : "";
    if (!text || (who === "user" && !paintUser)) return false;
    const last = out[out.length - 1];
    if (last?.who === who && "text" in last && typeof last.text === "string") out[out.length - 1] = { ...last, text: last.text + text };
    else out.push({ who, text });
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
