import type { SessionRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { PinButton } from "@/components/PinButton";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { pinnedFirst } from "@/lib/pins";
import { ago, word } from "@/words";

/**
 * Which rows the pane bar's search and filter leave standing.
 *
 * The filter matches on the state **key**, the one the node sent — never
 * on the displayed word, and never on a state this layer worked out for
 * itself (docs/UX.md 状态呈现). Search reads what the row shows plus its
 * id, since the id is what the CLI prints and what people paste.
 */
export function visibleSessions(rows: SessionRow[], query: string, words: string[]) {
  const q = query.trim().toLowerCase();
  return rows.filter(
    (r) =>
      (words.length === 0 || words.includes(r.word)) &&
      (q === "" || `${r.title} ${r.id}`.toLowerCase().includes(q)),
  );
}

// Order and words come from the node untouched: the list never
// re-derives a judgment the backend already made (docs/UX.md). Pinning
// is the one exception and it is a partition, not a competing sort —
// `pinnedFirst` carries the argument.
export function SessionsList({
  rows,
  query,
  words,
  selected,
  onSelect,
  pinned,
  onTogglePin,
}: {
  rows: SessionRow[];
  query: string;
  words: string[];
  selected: string | null;
  onSelect: (row: SessionRow) => void;
  pinned: ReadonlySet<string>;
  onTogglePin: (key: string) => void;
}) {
  const shown = pinnedFirst(visibleSessions(rows, query, words), (r) => r.id, pinned);
  if (shown.length === 0) {
    // "Nothing here" and "nothing matched" are different facts, and the
    // wrong one is a lie the user has no way to catch: someone who
    // filtered down to zero would read "还没有 session" and believe the
    // machine had lost their work.
    return (
      <div data-empty className="p-4 text-sm text-muted-foreground">
        {rows.length === 0 ? gui.empty_sessions : gui.no_matches}
      </div>
    );
  }
  return (
    <div>
      {shown.map((r) => (
        // The row is a strip, not a button: the pin lives inside it and
        // a button inside a button is not a thing the DOM has. The strip
        // carries the row's identity and its highlight; the part you
        // open is the button within.
        <div
          key={r.id}
          data-row={r.id}
          data-word={r.word}
          className={cn(
            "flex w-full items-center gap-1 pr-2 hover:bg-secondary",
            r.id === selected && "bg-accent hover:bg-accent",
          )}
        >
          <button
            type="button"
            data-row-open
            onClick={() => onSelect(r)}
            className="flex min-w-0 flex-1 items-center gap-3 py-2 pl-4 text-left"
          >
            {/* The machine's face, not the agent's mark: on one screen
                most rows run the same agent, so the agent glyph is the
                thing that tells rows apart least. A face is one per
                machine and is what the eye picks up scanning. */}
            <MachineAvatar face={r.face} className="size-avatar" />
            <span className="min-w-0 flex-1">
              <span className="block truncate">{r.title || r.id}</span>
              <span className="flex items-center gap-2 overflow-hidden whitespace-nowrap text-sm text-muted-foreground">
                <span data-word-text style={{ color: `var(--state-${r.word})` }}>
                  {word(r.word)}
                </span>
                {r.source && <span>{gui.from_device(r.source.device)}</span>}
              </span>
            </span>
            <span className="flex flex-none flex-col items-end gap-1 text-xs text-muted-foreground">
              {/* No stamp yet is no age — not an epoch-sized number. */}
              {r.at_ms > 0 && <span>{ago(r.at_ms)}</span>}
              {r.unread > 0 && (
                <span
                  data-unread
                  className="min-w-4 rounded-full bg-badge px-1 text-center leading-4 text-badge-foreground"
                >
                  {r.unread}
                </span>
              )}
            </span>
          </button>
          <PinButton pinned={pinned.has(r.id)} onToggle={() => onTogglePin(r.id)} />
        </div>
      ))}
    </div>
  );
}
