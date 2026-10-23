import { Chunk } from "@codemirror/merge";
import { Text } from "@codemirror/state";

export type AgentDiffLine = {
  kind: "context" | "removed" | "added";
  text: string;
  before: number | null;
  after: number | null;
  noNewline?: boolean;
};
export type AgentDiffGap = { kind: "gap"; at: number; count: number };

/** A unified patch is a fragment with original line coordinates, not a file. */
export function agentPatch(text: string): (AgentDiffLine | { kind: "heading"; text: string })[] {
  const out: (AgentDiffLine | { kind: "heading"; text: string })[] = [];
  let before = 0, after = 0, hunk = false;
  for (const line of text.replace(/\r\n/g, "\n").split("\n")) {
    const header = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (header) { before = Number(header[1]); after = Number(header[2]); hunk = true; out.push({ kind: "heading", text: line }); }
    else if (hunk && line.startsWith("\\ No newline")) { const last = out.at(-1); if (last && last.kind !== "heading") last.noNewline = true; }
    else if (hunk && line.startsWith("-")) out.push({ kind: "removed", text: line.slice(1), before: before++, after: null });
    else if (hunk && line.startsWith("+")) out.push({ kind: "added", text: line.slice(1), before: null, after: after++ });
    else if (hunk && line.startsWith(" ")) out.push({ kind: "context", text: line.slice(1), before: before++, after: after++ });
    else if (line) out.push({ kind: "heading", text: line });
  }
  return out;
}

/** Reuse the editor's bounded diff engine, without creating an editor view. */
export function agentDiff(before: string | null, after: string): AgentDiffLine[] {
  const a = (before ?? "").replace(/\r\n/g, "\n"), b = after.replace(/\r\n/g, "\n");
  const lines = (value: string) => value === "" ? [] : value.replace(/\n$/, "").split("\n");
  const old = lines(a), next = lines(b), A = Text.of(a.split("\n")), B = Text.of(b.split("\n"));
  const out: AgentDiffLine[] = [];
  let i = 0, j = 0;
  const context = () => { out.push({ kind: "context", text: old[i], before: ++i, after: ++j }); };
  for (const c of Chunk.build(A, B, { scanLimit: 1000, timeout: 30 })) {
    const startA = Math.min(old.length, A.lineAt(Math.min(c.fromA, A.length)).number - 1);
    const startB = Math.min(next.length, B.lineAt(Math.min(c.fromB, B.length)).number - 1);
    while (i < startA && j < startB) context();
    const endA = c.toA === c.fromA ? startA : Math.min(old.length, A.lineAt(Math.min(c.toA - 1, A.length)).number);
    const endB = c.toB === c.fromB ? startB : Math.min(next.length, B.lineAt(Math.min(c.toB - 1, B.length)).number);
    while (i < endA) out.push({ kind: "removed", text: old[i], before: ++i, after: null, noNewline: i === old.length && !a.endsWith("\n") });
    while (j < endB) out.push({ kind: "added", text: next[j], before: null, after: ++j, noNewline: j === next.length && !b.endsWith("\n") });
  }
  while (i < old.length && j < next.length) context();
  return out;
}

/** Three lines of context on either side; gaps can be revealed independently. */
export function agentDiffContext(lines: AgentDiffLine[], expanded: ReadonlySet<number>, full = false): (AgentDiffLine | AgentDiffGap)[] {
  if (full) return lines;
  const out: (AgentDiffLine | AgentDiffGap)[] = [];
  for (let i = 0; i < lines.length;) {
    if (lines[i].kind !== "context") { out.push(lines[i++]); continue; }
    const start = i;
    while (i < lines.length && lines[i].kind === "context") i++;
    const left = start === 0 ? 0 : 3, right = i === lines.length ? 0 : 3;
    if (expanded.has(start) || i - start <= left + right) out.push(...lines.slice(start, i));
    else {
      out.push(...lines.slice(start, start + left));
      out.push({ kind: "gap", at: start, count: i - start - left - right });
      out.push(...lines.slice(i - right, i));
    }
  }
  return out;
}
