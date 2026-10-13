import { useEffect, useId, useRef, useState } from "react";
import { consoleCatalog, consoleComplete, type ConsoleChoice, type ConsoleCommand, type ConsoleCompletion } from "@/api";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import type { CommandRecord } from "@/gen/bindings/CommandRecord";
import type { SessionRow } from "@/api";
import { StatusWord } from "@/components/StatusWord";

/** Completion is owned by the public command parser, including its
 * replacement range. The capsule remains the only keyboard input. */
export function useConsoleCompletion(line: string, caret: number) {
  const [catalog, setCatalog] = useState<ConsoleCommand[]>([]);
  const [answer, setAnswer] = useState<{ line: string; caret: number; offer: ConsoleCompletion } | null>(null);
  const [pick, setPick] = useState(0);
  const [shut, setShut] = useState(false);
  const snapshot = useRef({ line, caret });
  snapshot.current = { line, caret };
  useEffect(() => { void consoleCatalog().then(setCatalog, () => {}); }, []);
  useEffect(() => {
    setPick(0);
    setShut(false);
    setAnswer(null);
    if (!line.startsWith("/")) return;
    let live = true;
    // The catalog can answer the first word immediately while argument
    // completions ask the node for its machines, sessions and disk.
    const prefix = line.slice(0, caret);
    if (/^\/[^\s]*$/.test(prefix)) {
      const word = prefix.slice(1).toLowerCase();
      const tail = /^\S*/.exec(line.slice(caret))?.[0] ?? "";
      setAnswer({ line, caret, offer: { start: 0, end: caret + tail.length, choices: catalog.filter((c) => c.word.startsWith(word) || c.aliases.some((a) => a.startsWith(word))).map((c) => ({ value: `/${c.word}`, label: `/${c.word}`, detail: c.usage, kind: "command" })) } });
    }
    const timer = setTimeout(() => {
      void consoleComplete(line, caret).then((next) => {
        if (live) setAnswer({ line, caret, offer: next });
      }, () => {});
    }, 150);
    return () => { live = false; clearTimeout(timer); };
  }, [line, caret, catalog]);
  const offer = answer?.line === line && answer.caret === caret ? answer.offer : null;
  const choices = offer?.choices ?? [];
  return {
    choices, pick, setPick,
    open: line.startsWith("/") && !shut && choices.length > 0,
    close: () => setShut(true),
    take: (choice: ConsoleChoice) => {
      if (!offer || snapshot.current.line !== line || snapshot.current.caret !== caret) return null;
      const suffix = line.slice(offer.end);
      const finished = ["command", "machine", "session", "argument", "file"].includes(choice.kind);
      const value = choice.value + (finished && !/\s$/.test(choice.value) && !/^\s/.test(suffix) ? " " : "");
      const next = `${line.slice(0, offer.start)}${value}${suffix}`;
      setShut(true);
      return { line: next, caret: offer.start + value.length };
    },
  };
}

export function ConsoleMenu({ choices, pick, onPick }: { choices: ConsoleChoice[]; pick: number; onPick: (choice: ConsoleChoice) => void }) {
  const id = useId();
  useEffect(() => { document.getElementById(`${id}-${pick}`)?.scrollIntoView({ block: "nearest" }); }, [id, pick]);
  return (
    <div data-console-menu role="listbox" aria-label={gui.console_title} className="absolute bottom-full left-0 z-20 mb-1 max-h-56 w-full overflow-y-auto rounded-lg border bg-popover p-1 shadow-(--shadow-float)">
      {choices.map((choice, n) => (
        <button key={`${choice.value}-${n}`} id={`${id}-${n}`} type="button" role="option" aria-selected={n === pick} data-console-choice={choice.kind} data-on={n === pick} className={cn("flex min-h-touch w-full flex-col items-start justify-center rounded-sm px-2 py-1 text-left text-callout pointer-fine:min-h-ctl-md", n === pick && "bg-fill-strong text-foreground")} onPointerDown={(e) => e.preventDefault()} onMouseDown={(e) => e.preventDefault()} onClick={() => onPick(choice)}>
          <span className="w-full truncate font-mono">{choice.label}</span>
          {choice.detail && <span className={cn("w-full truncate text-caption", n !== pick && "text-secondary-foreground")}>{choice.detail}</span>}
        </button>
      ))}
      <div className="px-2 py-1 text-caption text-muted-foreground">{gui.console_hint}</div>
    </div>
  );
}

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

export function ConsoleResult({ command: c, onSession, linked }: { command: CommandRecord; onSession?: (id: string) => void; linked?: SessionRow }) {
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
