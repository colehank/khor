import { useEffect, useRef, useState, type MouseEvent } from "react";

import { renameAccount, renameMachine, renameSession } from "@/api";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/**
 * How long a press on a name waits for the second half of a double-click
 * before it counts as a single press.
 *
 * **Most double-clicks land well inside this.** The system's own window
 * is longer (macOS defaults to half a second) and the page cannot read
 * it; a double-click slower than this but still inside it does both —
 * the single press lands, then the editor opens. What it never does is
 * the thing this exists to stop: a press that shows something and a
 * double-click that takes it away again, one blink apiece.
 */
const NAME_WAIT_MS = 300;

/**
 * A single press on a control that **holds a name somebody may rename**
 * (2026-09-29: the front session tab, and the narrow head that stands in
 * for it — a press shows the day, a double-click on the name renames).
 *
 * Both gestures start with the same click, so the click on the name has
 * to wait to learn which one it is: `act` runs once the double-click
 * window has passed with no second press, and never if one came (the
 * second click, `detail` 2, cancels it; the `dblclick` then opens the
 * editor as it always has). **Only a click on the name waits** — the
 * editor's hot zone is the name's own text, so a press on the face or on
 * the blank beside it cannot be a rename and acts at once — and so do
 * Enter and Space (`detail` 0): a key is never half a double-click.
 */
export function useSinglePress(act: () => void): (e: MouseEvent) => void {
  const wait = useRef<number | null>(null);
  // The act of the render the press lands in: a timer holding the act of
  // the render it started in would run a stale one.
  const latest = useRef(act);
  latest.current = act;
  useEffect(
    () => () => {
      if (wait.current !== null) window.clearTimeout(wait.current);
    },
    [],
  );
  return (e) => {
    if (wait.current !== null) {
      window.clearTimeout(wait.current);
      wait.current = null;
    }
    if (e.detail > 1) return;
    const onName = e.target instanceof Element && e.target.closest("[data-editable-name]") !== null;
    if (e.detail === 0 || !onName) {
      latest.current();
      return;
    }
    wait.current = window.setTimeout(() => {
      wait.current = null;
      latest.current();
    }, NAME_WAIT_MS);
  };
}

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
 * stays open wearing the refusal, because closing it would paint the
 * exact picture a successful rename paints while nothing moved. **The
 * refusal is the writer's own sentence where it has one** (工作区第五轮
 * 第 36 条: 「已有同名 → 拒绝，理由在输入框下一行，不是弹窗」) — 改不了名
 * is what khor says when it has nothing better, and 「那个名字已经有了」
 * is a fact only the machine holding the disk knows.
 *
 * # The third subject is a file, and it brought two doors with it
 *
 * 工作区第五轮 第 36 条 wanted this box on a tree row: rename a file in
 * place, and type a new file's name into a row that does not exist yet.
 * Neither fits `WRITE`, and the head above says a third subject is a
 * line in that table — so here is why this one is not.
 *
 * **`write`** replaces the table for one call site. `WRITE`'s two take
 * `(id, name)` because a session and a machine are named by an id this
 * component can carry; a file is named by a machine **and** a path, and
 * the machine is not this box's to know. Adding it as a prop would put
 * a machine on every session's title for the sake of files.
 *
 * **`open` / `onClose`** make the editor controlled, for the row that
 * has no resting text to double-click: a file being created is an
 * editor and nothing else, and it goes away when it is done. Given
 * neither, this is the uncontrolled box it has always been.
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
  // **The third subject that is a line here, as the head promised**
  // (2026-09-30, the owner: 「这里的 list 也要和 session 一样，支持双击
  // 名字改名」). An account has no reported name to fall back to, so an
  // emptied box is refused in the node's words (`Node::rename_account`)
  // and lands under the box like any refusal.
  account: (id: string, name: string) => renameAccount(id, name).then(() => null),
} as const;

export function EditableName({
  kind = "session",
  id,
  title,
  className,
  inputClassName,
  nested,
  boxData,
  write,
  open: openFrom,
  onClose,
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
  /**
   * Attributes for the **outer** box — the one carrying `truncate`, and
   * therefore the one that actually clips.
   *
   * It exists because swapping a plain label for this component moves
   * the clipping box: a gate measuring `scrollWidth > clientWidth` has
   * to find the box that clips, and the inner span is exactly as wide
   * as its text, so it can never report clipping at all. Putting the
   * caller's hook on a wrapper *around* this component is the trap —
   * two nested `truncate` boxes clip at the same width, the outer one
   * then reads as never clipped, and an assertion that used to fire
   * goes quietly dead.
   */
  boxData?: Record<string, string>;
  /** Who to tell instead of [`WRITE`], for a subject whose writer needs
      more than an id (a file: a machine and a path). Answers the name
      the row now shows, or `null` where only the backend can say —
      exactly `WRITE`'s own contract, so `echo` works either way. */
  write?: (name: string) => Promise<string | null>;
  /** Controlled: the box is open because whoever owns the row says so,
      which is how a name is typed for something that does not exist
      yet. Absent leaves the double-click / F2 door as the only way in. */
  open?: boolean;
  /** Controlled mode's way out — Enter that took, Esc, or a click away.
      Called on every exit, so the row that opened this can put itself
      back however it opened. */
  onClose?: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  /** The refusal to show under the box, or `null`. **A sentence and
      not a flag**, so the machine's own words reach the person who
      typed the name (工作区第五轮 第 36 条). */
  const [refused, setRefused] = useState<string | null>(null);
  const [echo, setEcho] = useState<string | null>(null);
  // The poll caught up (or another device renamed it): the row speaks
  // for itself again.
  useEffect(() => {
    setEcho(null);
  }, [title]);
  // **A controlled box starts on the name it is about.** The
  // uncontrolled door seeds the draft where it opens it (`open` below);
  // this one is opened by somebody else, and a rename that began on an
  // empty box would make 「keep most of the name」 a retype. `title` is
  // constant for the life of one of these, so this runs once.
  useEffect(() => {
    if (openFrom !== true) return;
    setDraft(echo ?? title);
    setRefused(null);
    // `echo` is the bridge over one poll and must not re-seed the box
    // under somebody's fingers when it retires.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [openFrom, title]);
  const shown = echo ?? (title || id);
  const told = openFrom ?? editing;
  const shut = () => {
    setEditing(false);
    onClose?.();
  };
  if (!told) {
    // Two spans on purpose: the outer one carries the layout (truncate
    // wants a block-ish box, and in the list that box is as wide as the
    // column), the inner one is exactly as wide as the text — and the
    // inner one is the hot zone. Hung on the outer box instead, the
    // "title" would be double-clickable across the row's whole blank
    // stretch, which is the row's ground, not the title. Found by the
    // hot-zone probe, not by reading: the first cut hung it on the box.
    const open = () => {
      setDraft(echo ?? title);
      setRefused(null);
      setEditing(true);
    };
    return (
      <span {...boxData} className={cn("truncate", className)}>
        <span
          data-editable-name
          data-session-title={kind === "session" ? "" : undefined}
          data-machine-name={kind === "machine" ? "" : undefined}
          data-account-name={kind === "account" ? "" : undefined}
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
    (write ? write(name) : WRITE[kind](id, name)).then(
      (shown) => {
        shut();
        // An emptied box falls back to a name only the backend knows,
        // so there is nothing honest to echo unless the write answered
        // one — the poll fills in the rest.
        setEcho(name === "" ? shown : name);
      },
      // **The machine's own sentence, unwrapped.** What comes back is
      // an `Error` carrying the node's text; `String(e)` would put
      // 「Error: 」 in front of 「那个名字已经有了」 on the screen, which
      // is the transport's word in the middle of khor's (the tree reads
      // its refusals the same way).
      (e: unknown) =>
        setRefused(String(e instanceof Error ? e.message : e) || gui.rename_failed),
    );
  };
  return (
    <>
      <input
        data-title-input
        autoFocus
        value={draft}
        aria-label={gui.rename}
        // Enter takes the name and the box goes (UX-43 §3: the
        // keyboard's return key says so).
        enterKeyHint="done"
        onClick={(e) => e.stopPropagation()}
        onDoubleClick={(e) => e.stopPropagation()}
        onChange={(e) => {
          setDraft(e.target.value);
          setRefused(null);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") submit();
          else if (e.key === "Escape") shut();
        }}
        onBlur={shut}
        onFocus={(e) => e.currentTarget.select()}
        // The text field's ground and ring (components/TextField,
        // 2026-09-28) at the size of the name it stands in for.
        className={cn(
          "min-w-0 rounded-sm bg-fill px-1 focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring",
          inputClassName,
        )}
      />
      {refused !== null && (
        // Under the box and not in front of it: a person who has just
        // typed a name is looking at the box, and a dialog would take
        // the name away to tell them about it.
        <span data-rename-failed className="min-w-0 text-callout text-state-failed">
          {refused}
        </span>
      )}
    </>
  );
}
