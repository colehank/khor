// 导入 (docs/SHELL.md 第八轮, 2026-09-22:「session 里放什么，应该是用户
// 决定，不是我们自动。用户可以通过+导入已有 session，包括 acp。」).
//
// **What khor found and has not put on the list**, three sources in one
// table (`Node::import_candidates`): an agent somebody started in their
// own terminal (a watched tui), a tmux session (a terminal), and a
// conversation a registered ACP agent said it has (`session/list`, as
// recorded). Deleting an imported row later takes it off the list and
// touches nothing else, which is why the note at the foot says so.
//
// **A search, not a form** (IMPORT-BRIEF, 2026-10-09, the owner:「现在，
// 导入的时候，逻辑混乱，用户可能不期望按目录选，而可以搜索机器上的会话」;
// drawn and approved in the 1458 lab). A field at the top, focused on
// open, filters at once over the title, the directory, the machine and
// the running command (a tmux row's title is its session's name); the
// machines are a row of chips, every reachable one by default; a tap on
// a row imports it and opens it, so importing several is doing it again.
// One row per tmux session and nothing deeper: once it is open, khor
// recognises what runs inside, as it does for any terminal. ⚠️ 墓碑: two
// popups (所有机器 / 所有目录) over a list of check boxes and 「导入 N 个」.
//
// **The ACP half keeps the new-session wizard's rules, moved here whole**
// (一个动作一个位置): the four answers are four sentences — never asked,
// cannot list, has none, here they are — because the next move differs
// in each; and asking starts an agent process (1.5s warm, 5.64s cold),
// so it is a button a person presses and never a timer. What an agent
// lists joins the results above it.
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import {
  agentAsk,
  agentSeen,
  agents as fetchAgents,
  fetchDevices,
  fetchImportCandidates,
  importSessions,
  type AgentRow,
  type AgentSaid,
  type CandidateRow,
  type DeviceRow,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { sessionMark } from "@/components/BrandIcons";
import { IconClose, IconSearch } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { imeKey } from "@/lib/ime";
import { useHereHosts } from "@/lib/here";
import { cn } from "@/lib/utils";
import { ago, basename, connectionKey, connectionKeyWord } from "@/words";

/** What one agent's recorded answer is, as the section's mark. */
function markOf(answer: AgentSaid | undefined): string {
  if (answer === undefined) return "reading";
  if (answer.said === "unasked") return "unasked";
  if (answer.said === "cannot") return "cannot";
  return answer.rows.length === 0 ? "none" : "sessions";
}

/** **Why a machine is not in 「全部」**, or `null` for one that is: it
    cannot be reached or is paused — `connectionKey`'s `down` / `paused`
    and their words (不通 / 已暂停), the one derivation every surface
    reads (WORDS-1008). Its chip stays, with the word; pressed, its
    last-known sessions are listed. A machine the table has not named is
    taken as reachable — what it reported is all there is. */
function away(d: DeviceRow | undefined): string | null {
  if (!d) return null;
  const key = connectionKey(d);
  return key === "down" || key === "paused" ? connectionKeyWord(key) : null;
}

/** The fields a typed word is looked for in. */
const fields = (r: CandidateRow) => [r.title, r.place ?? "", r.machine, r.command ?? ""];

/** Every word typed is somewhere in the row, case aside. */
function matches(r: CandidateRow, words: string[]): boolean {
  if (words.length === 0) return true;
  const hay = fields(r).map((f) => f.toLowerCase());
  return words.every((w) => hay.some((f) => f.includes(w)));
}

/** A text with the typed words in the full ink, so a row says why it
    matched. */
function Hit({ text, words }: { text: string; words: string[] }) {
  if (words.length === 0 || text === "") return <>{text}</>;
  const low = text.toLowerCase();
  const on = new Array<boolean>(text.length).fill(false);
  for (const w of words) {
    for (let i = low.indexOf(w); i >= 0; i = low.indexOf(w, i + 1)) {
      for (let j = i; j < i + w.length; j++) on[j] = true;
    }
  }
  const parts: ReactNode[] = [];
  for (let i = 0; i < text.length; ) {
    let j = i;
    while (j < text.length && on[j] === on[i]) j++;
    const piece = text.slice(i, j);
    parts.push(
      on[i] ? (
        <mark key={i} data-import-hit className="bg-transparent font-semibold text-foreground">
          {piece}
        </mark>
      ) : (
        piece
      ),
    );
    i = j;
  }
  return <>{parts}</>;
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
  const [devices, setDevices] = useState<DeviceRow[]>([]);
  const [q, setQ] = useState("");
  /** The chips pressed, by device id; none is 「全部」. */
  const [picked, setPicked] = useState<Set<string>>(new Set());
  /** The row Enter opens, while something is typed. */
  const [active, setActive] = useState(0);
  const [going, setGoing] = useState<string | null>(null);
  const [registered, setRegistered] = useState<AgentRow[]>([]);
  const [said, setSaid] = useState<Record<string, AgentSaid>>({});
  const [asking, setAsking] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const field = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const why = (e: unknown) => setError(String(e instanceof Error ? e.message : e));
  /** **A phone neither lists an agent's conversations nor asks for them**
      (docs/IOS.md 第二步): one is reopened on this machine, and asking
      starts the agent here — a phone starts nothing. The node leaves
      them out of the candidates there; this leaves out the section that
      would ask. */
  const here = useHereHosts();

  const reload = () =>
    fetchImportCandidates()
      .then((r) => setRows(r))
      .catch(why);

  // Read what is there — never asked on a timer: the candidates are a
  // sweep this machine already makes, and the agents' answers are what
  // was recorded (0.017s each).
  useEffect(() => {
    if (!open) return;
    setQ("");
    setPicked(new Set());
    setActive(0);
    setGoing(null);
    setError(null);
    setRows(null);
    reload();
    fetchDevices()
      .then(setDevices)
      .catch(() => {
        /* no table: every machine reads as reachable, and no face */
      });
    if (!here) return;
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
  }, [open, here]);

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

  const words = useMemo(
    () =>
      q
        .toLowerCase()
        .split(/\s+/)
        .filter((w) => w !== ""),
    [q],
  );
  const byId = useMemo(() => new Map(devices.map((d) => [d.id, d])), [devices]);
  // The chips: the machines that have something to import — this one
  // first, then by name, the ones out of reach last.
  const machines = useMemo(() => {
    const seen = new Map<string, string>();
    for (const r of rows ?? []) seen.set(r.device, r.machine);
    return [...seen]
      .map(([id, name]) => ({ id, name, device: byId.get(id), away: away(byId.get(id)) }))
      .sort(
        (a, b) =>
          Number(a.away !== null) - Number(b.away !== null) ||
          Number(b.device?.me ?? false) - Number(a.device?.me ?? false) ||
          a.name.localeCompare(b.name),
      );
  }, [rows, byId]);
  const inScope = (r: CandidateRow) => (picked.size === 0 ? away(byId.get(r.device)) === null : picked.has(r.device));
  // Newest first, which is the node's order.
  const shown = (rows ?? []).filter((r) => inScope(r) && matches(r, words));

  useEffect(() => setActive(0), [q, picked]);
  useEffect(() => {
    list.current?.querySelector<HTMLElement>("[data-import-row][data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const press = (id: string | null) =>
    setPicked((was) => {
      if (id === null) return new Set();
      const next = new Set(was);
      if (!next.delete(id)) next.add(id);
      return next;
    });

  /** **A tap imports it and opens it** (IMPORT-BRIEF ruling 4,「直接打开
      呀」): the shell opens what comes back (`onImported`). */
  const go = (r: CandidateRow) => {
    if (going) return;
    setGoing(r.id);
    setError(null);
    importSessions([r.id])
      .then((done) => {
        onOpenChange(false);
        onImported(done);
      })
      .catch(why)
      .finally(() => setGoing(null));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        form
        data-import-dialog
        onOpenAutoFocus={(e) => {
          e.preventDefault();
          field.current?.focus();
        }}
      >
        <DialogHeader>
          <DialogTitle>{gui.import_title}</DialogTitle>
        </DialogHeader>
        {/* `min-w-0` for the new-session wizard's reason: a grid item
            defaults to `min-width: auto`, and one long directory would
            widen the whole dialog past its edge. */}
        <div data-import-body className="flex min-h-0 min-w-0 flex-col gap-row">
          {/* The search field (components/SearchField): the text box with
              a magnifier, and a clear mark while something is typed. */}
          <div data-import-find className="relative flex-none">
            <IconSearch className="pointer-events-none absolute top-1/2 left-in size-affordance -translate-y-1/2 text-tertiary" />
            <input
              ref={field}
              data-import-field
              data-slot="input"
              type="search"
              enterKeyHint="go"
              autoCapitalize="none"
              autoCorrect="off"
              spellCheck={false}
              placeholder={gui.import_search}
              aria-label={gui.import_search}
              value={q}
              onChange={(e) => setQ(e.target.value)}
              onKeyDown={(e) => {
                if (imeKey(e)) return;
                if (e.key === "ArrowDown") {
                  e.preventDefault();
                  setActive((i) => Math.min(shown.length - 1, i + 1));
                } else if (e.key === "ArrowUp") {
                  e.preventDefault();
                  setActive((i) => Math.max(0, i - 1));
                } else if (e.key === "Enter" && shown[active]) {
                  e.preventDefault();
                  go(shown[active]);
                }
              }}
              className={cn(
                // `appearance-none`: WebKit draws a search box its own way
                // and drops the padding the magnifier stands in.
                "h-ctl-md min-h-touch w-full min-w-0 appearance-none rounded-sm border-0 bg-fill ps-7 pe-8 text-body text-foreground placeholder:text-tertiary",
                "focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring",
                "[&::-webkit-search-cancel-button]:hidden",
              )}
            />
            {q !== "" && (
              <button
                type="button"
                data-import-clear
                aria-label={gui.search_clear}
                // The caret stays in the field.
                onPointerDown={(e) => e.preventDefault()}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => {
                  setQ("");
                  field.current?.focus();
                }}
                className="absolute top-1/2 right-1 grid size-touch -translate-y-1/2 place-items-center"
              >
                <span className="grid size-5 place-items-center rounded-full bg-fill-strong text-muted-foreground">
                  <IconClose className="size-3" />
                </span>
              </button>
            )}
          </div>

          {/* **The machines as chips** (ruling 2), the browser's machine
              chips' look (`MachineChips`): 「全部」 is every reachable
              machine; a machine's chip narrows to it, and several are an
              or. Not drawn while one machine has everything. */}
          {machines.length > 1 && (
            <div
              data-import-chips
              role="group"
              aria-label={gui.devices_tab}
              className="-mx-pane flex flex-none gap-in overflow-x-auto px-pane py-xxs"
            >
              <Chip on={picked.size === 0} onClick={() => press(null)} mark={{ "data-import-chip": "" }}>
                <span className="px-xxs whitespace-nowrap">{gui.import_all}</span>
              </Chip>
              {machines.map((m) => (
                <Chip
                  key={m.id}
                  on={picked.has(m.id)}
                  onClick={() => press(m.id)}
                  face
                  mark={{ "data-import-chip": m.name, ...(m.away !== null ? { "data-away": "" } : {}) }}
                >
                  <MachineAvatar face={m.device?.face ?? null} className="size-tab-face flex-none" />
                  <span className="whitespace-nowrap">{m.name}</span>
                  {m.away !== null && <span className="whitespace-nowrap text-caption text-tertiary">{m.away}</span>}
                </Chip>
              ))}
            </div>
          )}

          <div ref={list} data-import-rows className="-mx-in flex min-h-0 min-w-0 flex-col overflow-y-auto overscroll-contain px-in">
            {rows === null ? null : rows.length === 0 ? (
              <Calm sentence={gui.import_empty} hint={gui.import_empty_hint} mark={{ "data-import-empty": "" }} />
            ) : shown.length === 0 ? (
              <Calm sentence={gui.no_matches} mark={{ "data-import-nomatch": "" }} />
            ) : (
              shown.map((r, i) => (
                <Row
                  key={r.id}
                  row={r}
                  words={words}
                  active={i === active && q !== ""}
                  going={going === r.id}
                  onPick={() => go(r)}
                />
              ))
            )}
          </div>

          {here && registered.length > 0 && (
            <div data-import-agents className="flex min-w-0 flex-none flex-col gap-1 border-t pt-row text-callout">
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
                        <span data-agent-seen-text className="ml-2 text-caption text-muted-foreground">
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
                      aria-busy={asking === a.name || undefined}
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
            <span data-import-error className="flex-none text-callout text-destructive">
              {error}
            </span>
          )}
          {/* What a tap does and what a later delete does not — said
              while there is something to tap. */}
          {rows !== null && rows.length > 0 && (
            <p data-import-note className="flex-none text-caption text-muted-foreground">
              {gui.import_note}
            </p>
          )}
        </div>
        <DialogFooter>
          <Button data-dialog-cancel onClick={() => onOpenChange(false)} className="min-h-touch">
            {gui.delete_keep}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Chip({
  on,
  onClick,
  face,
  mark,
  children,
}: {
  on: boolean;
  onClick: () => void;
  /** It starts with a machine's face, which sits at the round end. */
  face?: boolean;
  mark: Record<string, string>;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      {...mark}
      data-on={on || undefined}
      aria-pressed={on}
      onClick={onClick}
      className={cn(
        "glass flex h-glass-btn flex-none items-center gap-in rounded-full text-callout",
        face ? "ps-1 pe-row" : "px-row",
        on && "glass-sel",
      )}
    >
      {children}
    </button>
  );
}

/**
 * One candidate: the kind's glyph, the title, and 「机器 · 目录 · 2 小时前」
 * under it (WORDS-1008); a terminal's running command at the end of the
 * title's line. **The title first**: it takes what it needs and the
 * command the rest, never under a third of the line; both cut at their
 * ends. An agent's row has no command — its glyph names the agent.
 */
function Row({
  row,
  words,
  active,
  going,
  onPick,
}: {
  row: CandidateRow;
  words: string[];
  active: boolean;
  going: boolean;
  onPick: () => void;
}) {
  const Glyph = sessionMark(row.form, row.category ?? row.agent);
  const dir = row.place ? basename(row.place) : "";
  const command = row.form === "shell" ? row.command : null;
  return (
    <button
      type="button"
      data-import-row={row.id}
      data-import-form={row.form}
      data-active={active || undefined}
      // While it is being imported it is the chosen row, and wears its
      // one ink (`app.css`, a picked row in a floating list).
      data-on={going || undefined}
      aria-busy={going || undefined}
      onClick={onPick}
      className={cn(
        "flex min-h-row-2 w-full min-w-0 flex-none items-center gap-row rounded-md px-in py-xxs text-left",
        "hover:bg-fill data-[active]:bg-fill data-[on=true]:bg-fill-strong",
      )}
    >
      <span data-import-glyph className="grid size-8 flex-none place-items-center text-muted-foreground">
        <Glyph className="size-5" />
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="flex min-w-0 items-baseline gap-row">
          <span data-import-title className="min-w-0 flex-[0_1_auto] truncate text-body">
            <Hit text={row.title || dir || row.id} words={words} />
          </span>
          {command && (
            <span
              data-import-command
              className="min-w-[35%] flex-[1_1_0%] truncate text-right font-mono text-caption text-muted-foreground"
            >
              <Hit text={command} words={words} />
            </span>
          )}
        </span>
        <span data-import-where className="truncate text-callout text-muted-foreground">
          <Hit text={row.machine} words={words} />
          {dir && (
            <>
              {" · "}
              <Hit text={dir} words={words} />
            </>
          )}
          {row.at_ms > 0 && (
            <>
              {" · "}
              <span className="tabular-nums">{ago(row.at_ms)}</span>
            </>
          )}
        </span>
      </span>
    </button>
  );
}

/** Nothing to list, one calm sentence in the list's place
    (components/EmptyState). */
function Calm({ sentence, hint, mark }: { sentence: string; hint?: string; mark: Record<string, string> }) {
  return (
    <div {...mark} className="flex min-h-40 flex-1 flex-col items-center justify-center gap-xxs px-base text-center">
      <span className="text-body text-muted-foreground">{sentence}</span>
      {hint && <span className="text-callout text-muted-foreground">{hint}</span>}
    </div>
  );
}
