import { Fragment } from "react";

import type { SessionRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago, groupLabel, word } from "@/words";

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
// re-derives a judgment the backend already made (docs/UX.md). That
// includes the order pinned rows arrive in — `Node::sessions` floats
// them, this file only paints the mark. **There is no comparison
// function in here, and adding one is the bug.**
export function SessionsList({
  rows,
  query,
  words,
  selected,
  onSelect,
  onPin,
}: {
  rows: SessionRow[];
  query: string;
  words: string[];
  selected: string | null;
  onSelect: (row: SessionRow) => void;
  onPin: (row: SessionRow) => void;
}) {
  const shown = visibleSessions(rows, query, words);
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
      {shown.map((r, i) => (
        <Fragment key={r.id}>
          {/* A heading starts where the group changes between
              neighbouring rows. The rows arrive grouped, so this notices
              a boundary rather than deciding one — and it means the
              empty group (the mode that does not group) prints no
              headings at all, with no special case. */}
          {r.group !== "" && r.group !== shown[i - 1]?.group && (
            <div
              data-group={r.group}
              className="px-4 pt-3 pb-1 text-xs text-muted-foreground"
            >
              {groupLabel(r.group)}
            </div>
          )}
          {/* The row is a strip holding two controls, not a button
              itself: the pin cannot be nested inside the button that
              opens the row, because a button inside a button is not a
              thing the DOM has. The strip was cut one batch before the
              pin arrived, so no anchor in here had to move when it
              did. */}
          <div
          data-row={r.id}
          data-word={r.word}
          data-pinned={r.pinned}
          className={cn(
            "flex w-full items-center pr-2 hover:bg-secondary",
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
          {/* The row's second control, and the reason the strip exists:
              a button cannot live inside the button that opens the row.
              **Always drawn, never hover-only** — the narrow face has no
              pointer to reveal it with, and a control you can only reach
              with a mouse is not reachable on a phone. Resting state is
              quiet (muted, tilted); pinned is upright and takes the
              foreground, so the mark is legible without hovering. */}
          <Button
            variant="ghost"
            size="icon"
            data-row-pin
            data-on={r.pinned}
            aria-label={r.pinned ? gui.unpin : gui.pin}
            onClick={() => onPin(r)}
            className="flex-none text-muted-foreground data-[on=true]:text-foreground"
          >
            <IconPin pinned={r.pinned} />
          </Button>
        </div>
        </Fragment>
      ))}
    </div>
  );
}
