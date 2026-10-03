// **A machine's commands, searched** (roadmap 2.6): every session's
// finished commands on the terminal's machine (`khor_node::commands::
// search_history`, read where they ran), by words, by the directory the
// terminal stands in, by failure. Laid out as fzf's ⌃R is, and as this
// app's full-screen pages keep their controls: the field at the foot, the
// newest command just above it. Picking one puts it on the prompt
// without running it — the shell's own history search does the same —
// or, while a program runs, copies it.
import { useEffect, useRef, useState } from "react";

import { fetchCommandHistory } from "@/api";
import { shortDir } from "@/components/DoingLine";
import { GlassCircle } from "@/components/Glass";
import { IconClose } from "@/components/icons";
import type { HistoryRow } from "@/gen/bindings/HistoryRow";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago } from "@/words";

/** Typing settles this long before the machine is asked (ms). */
const ASK_AFTER_MS = 150;

export function TermHistory({
  machine,
  cwd,
  onPick,
  onClose,
}: {
  machine: string;
  /** Where the terminal stands now, for 「这个目录」. */
  cwd: string | undefined;
  onPick: (text: string) => void;
  onClose: () => void;
}) {
  const [text, setText] = useState("");
  const [here, setHere] = useState(false);
  const [failed, setFailed] = useState(false);
  const [rows, setRows] = useState<HistoryRow[] | null>(null);
  const [at, setAt] = useState(0);
  const listRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let gone = false;
    const timer = window.setTimeout(() => {
      fetchCommandHistory(machine, text, here ? (cwd ?? "") : "", failed)
        .then((r) => {
          if (gone) return;
          setRows(r);
          setAt(0);
        })
        .catch(() => !gone && setRows([]));
    }, ASK_AFTER_MS);
    return () => {
      gone = true;
      window.clearTimeout(timer);
    };
  }, [machine, text, here, failed, cwd]);

  // The newest at the foot, chosen; ↑ goes back in time.
  useEffect(() => {
    listRef.current?.querySelector(`[data-term-history-row="${at}"]`)?.scrollIntoView({ block: "nearest" });
  }, [at, rows]);

  const shown = rows ?? [];
  const keyed = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    } else if (e.key === "ArrowUp" || (e.key === "p" && e.ctrlKey)) {
      e.preventDefault();
      setAt((i) => Math.min(i + 1, shown.length - 1));
    } else if (e.key === "ArrowDown" || (e.key === "n" && e.ctrlKey)) {
      e.preventDefault();
      setAt((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing) {
      e.preventDefault();
      const r = shown[at];
      if (r) onPick(r.text);
    }
  };

  const chip = (on: boolean) =>
    cn("min-h-touch shrink-0 rounded-full px-3 text-callout", on ? "bg-fill-strong" : "text-muted-foreground hover:bg-fill");

  return (
    <div data-term-history role="dialog" aria-label={gui.command_history} className="absolute inset-0 z-30 flex flex-col bg-background" onKeyDown={keyed}>
      <div ref={listRef} className="flex min-h-0 flex-1 flex-col-reverse overflow-y-auto">
        {rows !== null && shown.length === 0 && (
          <p data-term-history-none className="p-4 text-center text-callout text-muted-foreground">
            {gui.command_history_none}
          </p>
        )}
        {shown.map((r, i) => (
          <button
            key={`${r.text}\u0000${i}`}
            type="button"
            data-term-history-row={i}
            data-on={i === at}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => onPick(r.text)}
            className={cn("flex w-full flex-col items-start gap-0.5 border-b border-border px-3 py-2 text-left hover:bg-fill", i === at && "bg-fill")}
          >
            <span className="w-full truncate text-body" style={{ fontFamily: "var(--font-term)" }}>
              {r.text}
            </span>
            <span className="flex w-full gap-2 text-caption text-muted-foreground">
              {r.cwd && <span className="truncate">{shortDir(r.cwd)}</span>}
              <span className="shrink-0">{ago(r.ended_ms ?? r.started_ms)}</span>
              {r.exit !== null && r.exit !== 0 && <span className="shrink-0 text-[var(--state-failed)]">{gui.command_history_exit(String(r.exit))}</span>}
              {r.runs > 1 && <span className="shrink-0">{gui.command_history_runs(String(r.runs))}</span>}
            </span>
          </button>
        ))}
      </div>
      <div className="flex flex-none items-center gap-in border-t border-border p-2">
        <input
          data-term-history-field
          autoFocus={!window.matchMedia("(pointer: coarse)").matches}
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder={gui.command_history_search}
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          className="min-h-touch min-w-0 flex-1 rounded-full bg-fill px-3 text-body outline-none"
          style={{ fontFamily: "var(--font-term)" }}
        />
        {cwd && (
          <button type="button" data-term-history-here aria-pressed={here} className={chip(here)} onClick={() => setHere(!here)}>
            {gui.command_history_here}
          </button>
        )}
        <button type="button" data-term-history-failed aria-pressed={failed} className={chip(failed)} onClick={() => setFailed(!failed)}>
          {gui.command_history_failed}
        </button>
        <GlassCircle aria-label={gui.close} data-term-history-close onClick={onClose}>
          <IconClose />
        </GlassCircle>
      </div>
    </div>
  );
}
