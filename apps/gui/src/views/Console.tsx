import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import type { CommandRecord } from "@/gen/bindings/CommandRecord";
import type { SessionRow } from "@/api";
import { StatusWord } from "@/components/StatusWord";
import { KhorAgentResult } from "@/views/KhorAgent";

function StructuredData({ raw }: { raw: string }) {
  let value: unknown;
  try { value = JSON.parse(raw); } catch { return <pre className="whitespace-pre-wrap break-all font-mono text-callout">{raw}</pre>; }
  if (Array.isArray(value) && value.length > 0 && value.every((row) => typeof row === "object" && row !== null && !Array.isArray(row))) {
    const rows = value as Record<string, unknown>[];
    const columns = [...new Set(rows.flatMap(Object.keys))];
    return <div className="overflow-x-auto"><table data-console-table className="w-full border-collapse text-left text-callout tabular-nums"><thead><tr>{columns.map((c) => <th key={c} className="border-b px-in py-1 font-title">{c}</th>)}</tr></thead><tbody>{rows.map((row, n) => <tr key={n}>{columns.map((c) => <td key={c} className="border-b px-in py-1 align-top whitespace-pre-wrap">{typeof row[c] === "string" ? row[c] : JSON.stringify(row[c]) ?? ""}</td>)}</tr>)}</tbody></table></div>;
  }
  return <pre data-console-json className="overflow-x-auto whitespace-pre-wrap break-all font-mono text-callout">{JSON.stringify(value, null, 2)}</pre>;
}

export function ConsoleResult({ command: c, onSession, linked, session="wide/all" }: { session?:string;command: CommandRecord; onSession?: (id: string) => void; linked?: SessionRow }) {
  if (c.agent) return <KhorAgentResult session={session} record={c} />;
  const status = ({ running: gui.console_running, done: gui.console_done, failed: gui.console_failed, interrupted: gui.console_interrupted, cancelled: gui.console_cancelled } as Record<string, string>)[c.status] ?? c.status;
  return (
    <section data-console-result={c.id} data-console-status={c.status} aria-busy={c.status === "running"} className="min-w-0 space-y-in rounded-lg border px-row py-in text-body">
      <div className="flex items-baseline gap-row"><code className="min-w-0 flex-1 break-all text-callout">{c.line}</code>{linked && c.status === "done" ? <StatusWord word={linked.word} /> : !(c.session && c.status === "done") && <span role="status" className={cn("flex-none text-caption", c.status === "failed" ? "text-state-failed" : c.status === "running" ? "text-state-busy" : "text-secondary-foreground")}>{status}</span>}</div>
      <div data-console-owner className="text-caption text-secondary-foreground">{gui.console_owner(c.owner_name || c.owner)}</div>
      {linked?.last && <div data-console-session-latest className="whitespace-pre-wrap break-all text-callout text-secondary-foreground">{linked.last}</div>}
      {c.data_json ? <StructuredData raw={c.data_json} /> : c.stdout && <pre data-console-stdout className="overflow-x-auto whitespace-pre-wrap break-all font-mono text-callout">{c.stdout}</pre>}
      {c.stderr && <pre data-console-stderr className="whitespace-pre-wrap break-all font-mono text-callout text-secondary-foreground">{c.stderr}</pre>}
      {c.error && <div data-console-error className="whitespace-pre-wrap break-all text-callout text-state-failed">{c.error}</div>}
      {c.session && onSession && <Button data-console-session={c.session} size="sm" variant="default" onClick={() => onSession(c.session!)}>{gui.console_open_session}</Button>}
    </section>
  );
}
