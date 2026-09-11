import { useEffect, useState } from "react";

import { renameMachine, renameSession } from "@/api";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/**
 * A name somebody may change: double-click, type, Enter. One component
 * behind every place a name is editable, so no two of them can come to
 * mean different things by the same gesture.
 *
 * **Two subjects share it, and that is the point.** Sessions had this
 * first; machines got it on 2026-09-11, and the machine page was told
 * to reuse it rather than grow its own — the standing complaint that
 * the machine page and the quota page keep re-inventing each other's
 * parts (docs/handoff/001.md 挂账). Adding a third subject is a `kind`
 * and a line in `WRITE`, not another file.
 *
 * The name is the user's override over whatever the row derives for
 * itself (`Node::rename_session`, `Node::rename_machine`); an emptied
 * box takes it back, so the derived name — a vendor's own title, a
 * machine's reported channel — shows again. Esc (or clicking away)
 * backs out without writing anything.
 *
 * **The hot zone is the title text alone.** In the list the title sits
 * inside the button that opens the row, and the row must keep behaving
 * like a row everywhere else — so entering the editor stops the event
 * here, and the editor's own clicks stop too: a click that placed a
 * cursor must not read as "open this row" over and over. The first
 * click of the double-click still selects the row, which is fine — it
 * is the row being renamed.
 *
 * # A double-click was the only way in, and a double-click is a pointer
 *
 * Nothing focused it, no key opened it, and nothing on the glass said
 * it could be renamed at all — so on a keyboard, or with a screen
 * reader, the feature did not exist. Now the resting title is a control
 * where it is allowed to be one: it takes focus, F2 or Enter opens the
 * editor (F2 is the rename key everywhere a file manager exists, and it
 * is announced through `aria-keyshortcuts` rather than printed), and a
 * pointer gets the same sentence as a tooltip.
 *
 * **`nested` is the list saying "not here".** In a row the title lives
 * inside the button that opens the session, and a control inside a
 * control is invalid markup and worse behaviour: Enter would both
 * rename and open. So the row opts out, and the keyboard path is the
 * one in the detail header — reached by opening the row, which is
 * exactly what a keyboard user was already doing. **One path, in the
 * one place it is well-formed**, rather than two paths of which one is
 * broken.
 *
 * **What this does not fix:** renaming without opening the row. A list
 * row could carry its own F2, but that key would have to belong to the
 * row rather than to the title, and the row does not know what a title
 * is (docs/UX.md 状态呈现 — the list paints what the library sends).
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
/**
 * Who to tell, by subject. **Both answer the name the row now shows**,
 * or `null` when only the backend can say what it fell back to — the
 * echo below is built on exactly that distinction, so the two writers
 * must agree about it rather than each having its own shape.
 */
const WRITE = {
  session: (id: string, name: string) => renameSession(id, name).then(() => null),
  machine: (id: string, name: string) => renameMachine(id, name),
} as const;

export function EditableName({
  kind = "session",
  id,
  title,
  className,
  inputClassName,
  nested,
}: {
  /** Which table the id belongs to. Defaults to the first subject that
      had this, so every existing call site reads unchanged. */
  kind?: keyof typeof WRITE;
  id: string;
  title: string;
  /** Carried by the resting text (beside its own `truncate`). */
  className?: string;
  /** Carried by the editor (beside its own chrome). */
  inputClassName?: string;
  /**
   * This title sits inside another control, so it must not become one
   * (the file head: one keyboard path, in the one place it is
   * well-formed). The list row passes it; nothing else should need to.
   */
  nested?: boolean;
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
    const open = () => {
      setDraft(echo ?? title);
      setFailed(false);
      setEditing(true);
    };
    return (
      <span className={cn("truncate", className)}>
        <span
          data-editable-name
          data-session-title={kind === "session" ? "" : undefined}
          data-machine-name={kind === "machine" ? "" : undefined}
          // A control only where that is well-formed (the file head).
          role={nested ? undefined : "button"}
          tabIndex={nested ? undefined : 0}
          title={nested ? undefined : gui.rename}
          aria-keyshortcuts={nested ? undefined : "F2"}
          // The box a finger aims at, on the standalone one. `inline-flex`
          // because a bare inline box ignores a minimum height — and the
          // outer span keeps the truncation, so the text still clips
          // rather than pushing the header's controls off the end.
          className={cn(
            !nested &&
              "inline-flex min-h-touch min-w-touch max-w-full items-center truncate rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-ring",
          )}
          onDoubleClick={(e) => {
            e.stopPropagation();
            open();
          }}
          onKeyDown={
            nested
              ? undefined
              : (e) => {
                  if (e.key !== "F2" && e.key !== "Enter") return;
                  e.preventDefault();
                  e.stopPropagation();
                  open();
                }
          }
        >
          {shown}
        </span>
      </span>
    );
  }
  const submit = () => {
    const name = draft.trim();
    WRITE[kind](id, name).then(
      (shown) => {
        setEditing(false);
        // An emptied box falls back to a name only the backend knows,
        // so there is nothing honest to echo unless the write answered
        // one — the poll fills in the rest.
        setEcho(name === "" ? shown : name);
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
