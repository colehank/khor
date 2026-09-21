import { Fragment, useRef, useState, type ReactNode } from "react";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuLabel,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { cn } from "@/lib/utils";

/**
 * One thing a row can be told to do.
 *
 * **The words are the caller's, all of them** — the label, the sentence
 * a destructive one has to show first, and whatever a failure left to
 * say. This file holds no vocabulary: the catalog is looked up at the
 * last moment by whoever knows what kind of row this is
 * (docs/UX.md 文案).
 */
export type RowAction = {
  /** Stable name for this action, the anchor verification asserts on —
      never the word, which belongs to the catalog. */
  key: string;
  label: string;
  /** What pressing it costs, in this row's own terms. Present means the
      action takes two presses: the first shows this, the second does
      it. Absent means one press. */
  confirm?: string;
  /** **A 删除**, and what deleting this row does, in one sentence
      (docs/SHELL.md 第四轮 第 3 条). `SideList` turns it into the one
      delete window every sidebar shares — with 不再提醒 — instead of a
      second press on the menu. */
  ask?: string;
  /** Something the caller wants said here right now — a refusal that
      came back from the node, in the node's own sentence. It is the
      caller's state, not this menu's: what failed outlives one opening
      of a menu, and the menu is not the thing that knows it. */
  note?: string;
  /** This item's last attempt did not take. It wears the failure colour
      — and the caller has already put the word for *what* failed in
      `label`, because only the caller knows which action was tried. */
  failed?: boolean;
  /** **Rejecting is how an action says it did not happen.** A rejection
      leaves the menu open, so the row's own report — a label that has
      become the failure word, a `note` — is on screen where the press
      was made, rather than behind a menu that closed on a lie
      (docs/UX.md 做了但没变化 / 失败). */
  run: () => Promise<void> | void;
};

/**
 * The row's actions, on right-click and on long-press.
 *
 * **One mechanism for both faces, which is the whole reason this
 * exists.** The controls it replaces appeared on hover, and hover is
 * a thing a phone does not have — a control only a pointer can summon
 * does not exist there. `contextmenu` and a 700ms press are the same
 * gesture on two kinds of screen, and Radix's trigger answers both
 * (and suppresses the touch callout that would otherwise select the
 * row's text on the way).
 *
 * **It renders no element of its own** — `asChild` puts the trigger on
 * whatever the caller hands it, and the virtual anchor Radix positions
 * the menu from is the pointer, not a box. So nothing here changes what
 * the list is made of, which matters more than it sounds: a row's
 * position is measured with `offsetTop` for the FLIP, and a wrapper
 * element would be a new offset parent.
 *
 * **Hand it something that is already a target.** The trigger carries an
 * `onPointerDown` — that is the long press — and a press handler on a
 * bare `<div>` is reachable by pointer alone, which `check-face`'s
 * section 4 asserts against. Given a control, the handler lands on
 * something focus can reach and a keyboard's Menu key can fire
 * `contextmenu` on.
 */
export function RowMenu({ items, children }: { items: RowAction[]; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  // Which action has shown its sentence and is waiting for the second
  // press. **Cleared when the menu closes**: an arming is about this
  // opening of this menu, and a confirm still armed the next time round
  // would turn one press into the destructive one.
  const [armed, setArmed] = useState<string | null>(null);
  /** Whether this closing follows an action that ran. **A ref, read
      once at close**: Radix hands focus back to the trigger when the
      menu closes, and after an action that opened something else —
      a takeover that turns the row into a live conversation — that
      restore raced the new pane's own focusing of its box and won
      about one time in three (smoke 25f, diagnosed 2026-09-18:
      `focus is on BUTTON[data-row-open]`). A dismissal (Esc, a press
      outside) still gives focus back: nothing else asked for it. */
  const ran = useRef(false);
  const change = (next: boolean) => {
    setOpen(next);
    if (!next) setArmed(null);
  };
  const press = (a: RowAction) => {
    if (a.confirm && armed !== a.key) {
      setArmed(a.key);
      return;
    }
    Promise.resolve()
      .then(a.run)
      .then(
        () => {
          ran.current = true;
          change(false);
        },
        // Stays open, and goes back to un-armed: the sentence has been
        // read and answered, and what is worth saying now is whatever
        // the caller put in `note`.
        () => setArmed(null),
      );
  };
  return (
    <ContextMenu open={open} onOpenChange={change}>
      {/* `data-row-menu-trigger` lands on the caller's element (asChild):
          the anchor a gate right-clicks when it compares one sidebar's
          menu with another's (docs/SHELL.md 第二轮). */}
      <ContextMenuTrigger asChild data-row-menu-trigger="">
        {children}
      </ContextMenuTrigger>
      <ContextMenuContent
        data-row-menu
        onCloseAutoFocus={(e) => {
          if (!ran.current) return;
          ran.current = false;
          e.preventDefault();
        }}
      >
        {items.map((a) => (
          <Fragment key={a.key}>
            {a.confirm && armed === a.key && (
              <ContextMenuLabel data-menu-confirm={a.key} className="text-muted-foreground">
                {a.confirm}
              </ContextMenuLabel>
            )}
            <ContextMenuItem
              data-menu-item={a.key}
              data-armed={armed === a.key}
              // The finger floor (tokens.css `--touch`), on the item
              // rather than on some box inside it, so what lights up
              // under the pointer is what is pressable — the same rule
              // the buttons this menu replaces were wearing.
              //
              // **`focus:` has to be said twice for the failure colour**,
              // and this is not belt-and-braces: an item that was just
              // pressed is the focused item, and the stock
              // `focus:text-accent-foreground` is a pseudo-class — it
              // outranks a plain utility no matter which is written
              // last. Measured before this line existed: the word had
              // become 钉不上 and the colour was still the ordinary one.
              className={cn(
                "min-h-touch",
                a.failed && "text-state-failed focus:text-state-failed",
              )}
              onSelect={(e) => {
                // Radix closes on select; every path through `press`
                // decides for itself whether closing is the truth.
                e.preventDefault();
                press(a);
              }}
            >
              {a.label}
            </ContextMenuItem>
            {a.note && (
              // `text-destructive`, the same clothes the detail pane's
              // close refusal wears — one fact, one colour, wherever it
              // is printed.
              <ContextMenuLabel data-menu-note={a.key} className="text-destructive">
                {a.note}
              </ContextMenuLabel>
            )}
          </Fragment>
        ))}
      </ContextMenuContent>
    </ContextMenu>
  );
}
