// 导入 (docs/SHELL.md 第八轮, 2026-09-22:「session 里放什么，应该是用户
// 决定，不是我们自动。用户可以通过+导入已有 session，包括 acp。」).
//
// **What khor found and has not put on the list**, three sources in one
// table (`Node::import_candidates`): an agent somebody started in their
// own terminal (a watched tui), a tmux session (a terminal), and a
// conversation a registered ACP agent said it has (`session/list`, as
// recorded). Pick any number, import them; each lands on the list in the
// form it has. Deleting an imported row later takes it off the list and
// touches nothing else, which is why the note under the title says so
// before anybody imports anything.
//
// **The ACP half keeps the new-session wizard's rules, moved here whole**
// (一个动作一个位置): the four answers are four sentences — never asked,
// cannot list, has none, here they are — because the next move differs
// in each; and asking starts an agent process (1.5s warm, 5.64s cold),
// so it is a button a person presses and never a timer.
import { useEffect, useMemo, useState } from "react";

import {
  agentAsk,
  agentSeen,
  agents as fetchAgents,
  fetchImportCandidates,
  importSessions,
  type AgentRow,
  type AgentSaid,
  type CandidateRow,
} from "@/api";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago } from "@/words";

/** A form, in the words the panel shows it in. */
function formWord(form: string): string {
  if (form === "shell") return gui.view_terminal;
  if (form === "gui") return gui.import_form_gui;
  return gui.import_form_tui;
}

/** What one agent's recorded answer is, as the section's mark. */
function markOf(answer: AgentSaid | undefined): string {
  if (answer === undefined) return "reading";
  if (answer.said === "unasked") return "unasked";
  if (answer.said === "cannot") return "cannot";
  return answer.rows.length === 0 ? "none" : "sessions";
}

export function ImportDialog({
  open,
  onOpenChange,
  onImported,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The session ids now on the list, in the order imported. */
  onImported: (ids: string[]) => void;
}) {
  const [rows, setRows] = useState<CandidateRow[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [machine, setMachine] = useState<string>("");
  const [place, setPlace] = useState<string>("");
  const [registered, setRegistered] = useState<AgentRow[]>([]);
  const [said, setSaid] = useState<Record<string, AgentSaid>>({});
  const [asking, setAsking] = useState<string | null>(null);
  const [going, setGoing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const why = (e: unknown) => setError(String(e instanceof Error ? e.message : e));

  const reload = () =>
    fetchImportCandidates()
      .then((r) => setRows(r))
      .catch(why);

  // Read what is there — never asked on a timer: the candidates are a
  // sweep this machine already makes, and the agents' answers are what
  // was recorded (0.017s each).
  useEffect(() => {
    if (!open) return;
    setPicked(new Set());
    setError(null);
    setRows(null);
    reload();
    fetchAgents()
      .then((list) => {
        setRegistered(list);
        for (const a of list) {
          agentSeen(a.name)
            .then((answer) => setSaid((was) => ({ ...was, [a.name]: answer })))
            .catch(why);
        }
      })
      .catch(why);
  }, [open]);

  /** **The only thing here that starts a process.** */
  const ask = (name: string) => {
    if (asking) return;
    setAsking(name);
    agentAsk(name)
      .then((answer) => {
        setSaid((was) => ({ ...was, [name]: answer }));
        return reload();
      })
      .catch(why)
      .finally(() => setAsking(null));
  };

  const machines = useMemo(() => [...new Set((rows ?? []).map((r) => r.machine))].sort(), [rows]);
  const places = useMemo(
    () => [...new Set((rows ?? []).map((r) => r.place ?? "").filter((p) => p !== ""))].sort(),
    [rows],
  );
  const shown = (rows ?? []).filter(
    (r) => (machine === "" || r.machine === machine) && (place === "" || r.place === place),
  );

  const toggle = (id: string) =>
    setPicked((was) => {
      const next = new Set(was);
      if (!next.delete(id)) next.add(id);
      return next;
    });

  const go = () => {
    if (going || picked.size === 0) return;
    setGoing(true);
    setError(null);
    // In the table's order, which is newest first — what the person
    // sees is the order it lands in.
    const ids = (rows ?? []).filter((r) => picked.has(r.id)).map((r) => r.id);
    importSessions(ids)
      .then((done) => {
        onOpenChange(false);
        onImported(done);
      })
      .catch(why)
      .finally(() => setGoing(false));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-import-dialog>
        <DialogHeader>
          <DialogTitle>{gui.import_title}</DialogTitle>
        </DialogHeader>
        {/* `min-w-0` for the new-session wizard's reason: a grid item
            defaults to `min-width: auto`, and one long directory would
            widen the whole dialog past its edge. */}
        <div className="flex min-w-0 flex-col gap-3">
          <p data-import-note className="text-xs text-muted-foreground">
            {gui.import_note}
          </p>
          {(machines.length > 1 || places.length > 1) && (
            <div data-import-filters className="flex min-w-0 flex-wrap gap-1">
              {machines.length > 1 &&
                ["", ...machines].map((m) => (
                  <Button
                    key={`m:${m}`}
                    size="sm"
                    variant={machine === m ? "secondary" : "ghost"}
                    data-import-machine={m}
                    data-on={machine === m}
                    onClick={() => setMachine(m)}
                  >
                    {m === "" ? gui.import_all_machines : m}
                  </Button>
                ))}
              {places.length > 1 &&
                ["", ...places].map((p) => (
                  <Button
                    key={`p:${p}`}
                    size="sm"
                    variant={place === p ? "secondary" : "ghost"}
                    data-import-place={p}
                    data-on={place === p}
                    title={p}
                    className="max-w-48 min-w-0"
                    onClick={() => setPlace(p)}
                  >
                    <span className="truncate">{p === "" ? gui.import_all_places : (p.split("/").pop() ?? p)}</span>
                  </Button>
                ))}
            </div>
          )}
          {rows !== null && shown.length === 0 && (
            <span data-import-empty className="text-sm text-muted-foreground">
              {gui.import_empty}
            </span>
          )}
          {shown.length > 0 && (
            <div data-import-rows className="flex max-h-72 min-w-0 flex-col overflow-y-auto">
              {shown.map((r) => {
                const on = picked.has(r.id);
                return (
                  <label
                    key={r.id}
                    data-import-row={r.id}
                    data-import-form={r.form}
                    data-on={on}
                    className={cn(
                      "flex min-h-touch min-w-0 cursor-pointer items-center gap-2 rounded-md px-2 text-sm hover:bg-muted/50",
                      on && "bg-accent",
                    )}
                  >
                    <input type="checkbox" checked={on} onChange={() => toggle(r.id)} className="flex-none" />
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate">{r.title || r.place || r.id}</span>
                      <span className="truncate text-xs text-muted-foreground">
                        {[r.machine, formWord(r.form), r.place ?? ""].filter((w) => w !== "").join(" · ")}
                      </span>
                    </span>
                    {r.at_ms > 0 && (
                      <span className="flex-none text-xs tabular-nums text-muted-foreground">
                        {ago(r.at_ms)}
                      </span>
                    )}
                  </label>
                );
              })}
            </div>
          )}
          {registered.length > 0 && (
            <div data-import-agents className="flex min-w-0 flex-col gap-1 text-sm">
              <span className="text-muted-foreground">{gui.import_agents}</span>
              {registered.map((a) => {
                const mark = markOf(said[a.name]);
                const sentence =
                  mark === "unasked"
                    ? gui.new_session_seen_unasked
                    : mark === "cannot"
                      ? gui.new_session_seen_cannot
                      : mark === "none"
                        ? gui.new_session_seen_none
                        : null;
                return (
                  <div
                    key={a.name}
                    data-agent-seen={a.name}
                    data-said={mark}
                    className="flex min-w-0 items-baseline justify-between gap-2"
                  >
                    <span className="min-w-0 flex-1 truncate">
                      {a.name}
                      {sentence !== null && (
                        <span data-agent-seen-text className="ml-2 text-xs text-muted-foreground">
                          {sentence}
                        </span>
                      )}
                    </span>
                    <Button
                      size="sm"
                      variant={mark === "unasked" ? "secondary" : "ghost"}
                      data-agent-ask={a.name}
                      data-asking={asking === a.name}
                      disabled={asking !== null}
                      onClick={() => ask(a.name)}
                    >
                      {asking === a.name ? gui.new_session_seen_asking : gui.new_session_seen_ask}
                    </Button>
                  </div>
                );
              })}
            </div>
          )}
          {error !== null && (
            <span data-import-error className="text-sm text-destructive">
              {error}
            </span>
          )}
        </div>
        <DialogFooter>
          <Button data-import-go disabled={picked.size === 0 || going} onClick={go} className="min-h-touch">
            {gui.import_go(picked.size)}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
