import { useEffect, useState } from "react";

import { renameSession } from "@/api";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/**
 * A session's title, and the way to change it: double-click, type,
 * Enter. One component behind the detail header and the list row, so
 * the two cannot come to mean different things by the same gesture.
 *
 * The name is the user's override over whatever the row derives for
 * itself (`Node::rename_session`); an emptied box takes it back, so the
 * vendor's own name — or the title the session was opened with — shows
 * again. Esc (or clicking away) backs out without writing anything.
 *
 * **The hot zone is the title text alone.** In the list the title sits
 * inside the button that opens the row, and the row must keep behaving
 * like a row everywhere else — so entering the editor stops the event
 * here, and the editor's own clicks stop too: a click that placed a
 * cursor must not read as "open this row" over and over. The first
 * click of the double-click still selects the row, which is fine — it
 * is the row being renamed.
 *
 * **The row is the truth; the echo is a bridge.** The list polls on its
 * own clock, so for a beat after Enter the row still carries the old
 * name. `echo` covers exactly that beat and retires the moment the row
 * changes — it must not linger, or a rename made on another device
 * would be papered over by this screen's stale memory of its own.
 *
 * A failed write says so where it was typed (the pin's rule): the box
 * stays open wearing the failure word, because closing it would paint
 * the exact picture a successful rename paints while nothing moved.
 */
export function SessionTitle({
  id,
  title,
  className,
  inputClassName,
}: {
  id: string;
  title: string;
  /** Carried by the resting text (beside its own `truncate`). */
  className?: string;
  /** Carried by the editor (beside its own chrome). */
  inputClassName?: string;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [failed, setFailed] = useState(false);
  const [echo, setEcho] = useState<string | null>(null);
  // The poll caught up (or another device renamed it): the row speaks
  // for itself again.
  useEffect(() => {
    setEcho(null);
  }, [title]);
  const shown = echo ?? (title || id);
  if (!editing) {
    // Two spans on purpose: the outer one carries the layout (truncate
    // wants a block-ish box, and in the list that box is as wide as the
    // column), the inner one is exactly as wide as the text — and the
    // inner one is the hot zone. Hung on the outer box instead, the
    // "title" would be double-clickable across the row's whole blank
    // stretch, which is the row's ground, not the title. Found by the
    // hot-zone probe, not by reading: the first cut hung it on the box.
    return (
      <span className={cn("truncate", className)}>
        <span
          data-session-title
          onDoubleClick={(e) => {
            e.stopPropagation();
            setDraft(echo ?? title);
            setFailed(false);
            setEditing(true);
          }}
        >
          {shown}
        </span>
      </span>
    );
  }
  const submit = () => {
    const name = draft.trim();
    renameSession(id, name).then(
      () => {
        setEditing(false);
        // An emptied box falls back to a title only the backend knows,
        // so there is nothing honest to echo — the poll fills it in.
        setEcho(name === "" ? null : name);
      },
      () => setFailed(true),
    );
  };
  return (
    <>
      <input
        data-title-input
        autoFocus
        value={draft}
        aria-label={gui.rename}
        onClick={(e) => e.stopPropagation()}
        onDoubleClick={(e) => e.stopPropagation()}
        onChange={(e) => {
          setDraft(e.target.value);
          setFailed(false);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") submit();
          else if (e.key === "Escape") setEditing(false);
        }}
        onBlur={() => setEditing(false)}
        onFocus={(e) => e.currentTarget.select()}
        className={cn("min-w-0 rounded-sm bg-secondary px-1 outline-none", inputClassName)}
      />
      {failed && (
        <span data-rename-failed className="flex-none text-sm text-state-failed">
          {gui.rename_failed}
        </span>
      )}
    </>
  );
}
