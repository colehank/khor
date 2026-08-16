import type { SessionRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago, word } from "@/words";

// Order and words come from the node untouched: the list never
// re-derives a judgment the backend already made (docs/UX.md).
export function SessionsList({
  rows,
  selected,
  onSelect,
}: {
  rows: SessionRow[];
  selected: string | null;
  onSelect: (row: SessionRow) => void;
}) {
  if (rows.length === 0) {
    return <div className="p-4 text-sm text-muted-foreground">{gui.empty_sessions}</div>;
  }
  return (
    <div>
      {rows.map((r) => (
        <button
          key={r.id}
          type="button"
          data-word={r.word}
          onClick={() => onSelect(r)}
          className={cn(
            "flex w-full items-center gap-3 px-4 py-2 text-left hover:bg-secondary",
            r.id === selected && "bg-accent hover:bg-accent",
          )}
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
      ))}
    </div>
  );
}
