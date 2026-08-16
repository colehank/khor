import type { SessionRow } from "../api";
import { gui } from "../gen/catalog";
import { ago, word } from "../words";

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
    return <div className="notice">{gui.empty_sessions}</div>;
  }
  return (
    <div>
      {rows.map((r) => (
        <button
          key={r.id}
          className={`row${r.id === selected ? " on" : ""}`}
          data-word={r.word}
          onClick={() => onSelect(r)}
        >
          <span className="avatar-blank" aria-hidden="true" />
          <span className="row-main">
            <div className="row-title">{r.title || r.id}</div>
            <div className="row-sub">
              <span className="word" style={{ color: `var(--state-${r.word})` }}>
                {word(r.word)}
              </span>
              {r.source && <span>{gui.from_device(r.source.device)}</span>}
            </div>
          </span>
          <span className="row-side">
            {/* No stamp yet is no age — not an epoch-sized number. */}
            {r.at_ms > 0 && <span>{ago(r.at_ms)}</span>}
            {r.unread > 0 && <span className="unread">{r.unread}</span>}
          </span>
        </button>
      ))}
    </div>
  );
}
