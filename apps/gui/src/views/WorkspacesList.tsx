// The workspace landing's list: every machine's projects in one column
// (docs/UX.md 收件箱型 — this list is the thing being watched, because
// what happens inside a workspace happens while you are not looking).
//
// **Asked once per machine, not on a beat.** One ask is one real dial
// to one machine, and a list nobody is touching must not spend them.
// The landing asks a machine the first time it is seen and never again
// on its own; `check-acp` 第二批 holds the same line for the agent
// registry, and for the same measured reason.
//
// **Four answers, four faces.** 没问过 / 在问 / 它答不了 / 它答了（可能
// 是空的）. The one that costs the most to get wrong is the third: a
// machine that could not answer, painted as a machine with no projects,
// sends a person to look somewhere else instead of at that machine. So
// a refusal keeps its own row at the foot of the list, wearing the
// words that machine sent back.
import { useEffect, useRef, useState } from "react";

import { fetchWorkspaces, type DeviceRow, type WorkspaceRow } from "@/api";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago } from "@/words";

/** What one machine has said so far. Absent from the map is the fourth
    state, 没问过, and it is absent rather than a variant because that
    is the state the map starts in and the one an added machine has. */
type Asked =
  | { state: "asking" }
  | { state: "answered"; rows: WorkspaceRow[] }
  | { state: "refused"; why: string };

/** One row of the merged list: a workspace plus which machine it is on.
    The machine is carried per row rather than grouping the list by
    machine — the question this landing answers is「what am I working
    on」, and the answer is not sorted by hardware. */
type Row = { device: string; machine: string; ws: WorkspaceRow };

/** `/a/b/khor` → `khor`. The last real component, so a trailing slash
    does not answer with an empty name. */
export function basename(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/** The path without its last component, as a person reads it — what a
    row shows under the name. `~/w/khor` → `~/w`; a root-level checkout
    答 `/`. */
function parentShown(shown: string): string {
  const cut = shown.lastIndexOf("/");
  if (cut < 0) return "";
  return cut === 0 ? "/" : shown.slice(0, cut);
}

export function WorkspacesList({
  devices,
  selected,
  onOpen,
}: {
  devices: DeviceRow[];
  /** The open workspace, or null. Two fields rather than one joined
      key: two machines can hold the same path, and a separator would
      have to be a character no path contains — a sentinel this file
      does not need to invent (`scripts/check-bytes.mjs` on why they
      are not free). */
  selected: { device: string; path: string } | null;
  onOpen: (row: { device: string; machine: string; ws: WorkspaceRow }) => void;
}) {
  const [byDevice, setByDevice] = useState<Record<string, Asked>>({});
  // Which machines have been asked, ever. A ref and not state: asking
  // is a side effect that must happen exactly once per machine, and
  // state would re-run the effect that reads it.
  const asked = useRef<Set<string>>(new Set());

  useEffect(() => {
    for (const d of devices) {
      if (asked.current.has(d.id)) continue;
      asked.current.add(d.id);
      setByDevice((m) => ({ ...m, [d.id]: { state: "asking" } }));
      fetchWorkspaces(d.name)
        .then((rows) => setByDevice((m) => ({ ...m, [d.id]: { state: "answered", rows } })))
        .catch((e: unknown) =>
          setByDevice((m) => ({ ...m, [d.id]: { state: "refused", why: String(e) } })),
        );
    }
  }, [devices]);

  // Newest first, ties by path so two machines listing the same tree
  // print in one order. The node already sorted each machine's answer;
  // this re-sorts because the merge of several sorted lists is not one.
  const rows: Row[] = devices
    .flatMap((d) => {
      const a = byDevice[d.id];
      if (a?.state !== "answered") return [];
      return a.rows.map((ws) => ({ device: d.id, machine: d.name, ws }));
    })
    .sort((x, y) => y.ws.last_ms - x.ws.last_ms || x.ws.path.localeCompare(y.ws.path));

  const refused = devices
    .map((d) => ({ d, a: byDevice[d.id] }))
    .filter((x): x is { d: DeviceRow; a: { state: "refused"; why: string } } =>
      x.a?.state === "refused");
  const asking = devices.some((d) => byDevice[d.id]?.state === "asking");

  // Empty is only empty once every machine that can answer has. While
  // one is still asking, or one refused, this is not「nobody works
  // anywhere」— it is a list that is not finished.
  const isEmpty = rows.length === 0 && !asking && refused.length === 0;

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
      {isEmpty ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 py-10 text-center">
          <span className="text-fact font-title text-foreground">{gui.workspaces_empty}</span>
          <span className="text-aux text-muted-foreground">{gui.workspaces_how}</span>
        </div>
      ) : (
        rows.map((r) => {
          const isOpen =
            selected?.device === r.device && selected.path === r.ws.path;
          return (
            <button
              key={`${r.device}:${r.ws.path}`}
              data-ws-row
              type="button"
              onClick={() => onOpen(r)}
              className={cn(
                "flex min-h-touch w-full flex-col items-start gap-0.5 border-b border-border/50 px-3 py-2.5 text-left",
                isOpen && "bg-card",
              )}
            >
              <span className="flex w-full items-baseline gap-1.5">
                <span className="truncate text-fact font-title text-foreground">
                  {basename(r.ws.path)}
                </span>
                <span className="shrink-0 text-aux text-muted-foreground">{r.machine}</span>
              </span>
              <span className="flex w-full items-baseline gap-1.5 text-aux text-muted-foreground">
                <span className="truncate">{parentShown(r.ws.shown)}</span>
                <span className="ml-auto shrink-0 tabular-nums">{ago(r.ws.last_ms)}</span>
              </span>
            </button>
          );
        })
      )}

      {/* A machine that could not answer keeps its own line, wearing
          what it said. Sunk below the rows for the reason 失败 sinks in
          the session list: it is a fact to be found, not one to be
          answered. */}
      {refused.map(({ d, a }) => (
        <div
          key={d.id}
          className="flex flex-col gap-0.5 border-b border-border/50 px-3 py-2.5 text-aux text-muted-foreground"
        >
          <span className="font-label text-foreground/70">{d.name}</span>
          <span className="break-words">{a.why}</span>
        </div>
      ))}
    </div>
  );
}
