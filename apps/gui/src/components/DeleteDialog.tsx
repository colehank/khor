// The one window every sidebar's 删除 raises (docs/SHELL.md 第四轮 第 3
// 条:「删除这些, 都要弹出一个弹窗, 若勾选不再提醒, 后续删除操作都不提醒」).
//
// **One window, five sentences.** What deleting does differs by column —
// a session is closed, a machine leaves the network, a workspace comes
// off the list with its directory untouched, an account's registration
// goes, a tab closes — and the column says which, in one sentence. The
// word on the button is the same everywhere, because the act is.
//
// **不再提醒 is a setting, not a habit of this screen**: ticking it sets
// `confirm-delete` off (`khor config`), so every column stops asking at
// once, and 设置 → 安全 turns it back on. That is the difference from
// the file tree's `RemoveDialog`, whose tick is a habit one browser keeps
// and which never silences an act that cannot be undone.
//
// **On a phone it is an action sheet** (UX-10; ux-9 I8 and B4). A delete
// a person chose from a row's swipe or its menu is confirmed the way iOS
// confirms an intentional destructive act: from the bottom, a one-line
// title and the consequence, the red 删除 on top and 取消 on its own
// below (HIG Action sheets: 「to offer choices related to an intentional
// action」; destructive first, Cancel last). No 不再提醒 there — iOS has
// no checkbox in a confirmation; 设置 → 安全 turns the asking off. A Mac,
// a wide window and an iPad keep the alert, whose suppression box is
// AppKit's own convention.
import { useState } from "react";

import { setConfig } from "@/api";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { useCoarse } from "@/hooks/use-coarse";
import { useNarrow } from "@/hooks/use-narrow";
import { useSettingsReread } from "@/settings";

export type Pending = {
  /** Which one — the row's name as the list prints it. The title is a
      question naming it (components/Dialog:「删除 mandala？」); absent,
      the title falls back to the bare verb. */
  name?: string;
  /** What deleting this row does. */
  says: string;
  /** The act. Rejecting keeps the window open with the node's own
      sentence under it (`RowMenu`'s rule, one surface over). */
  run: () => Promise<void> | void;
};

export function DeleteDialog({ pending, onDone }: { pending: Pending | null; onDone: () => void }) {
  const [never, setNever] = useState(false);
  const [going, setGoing] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  const reread = useSettingsReread();
  const narrow = useNarrow();
  const coarse = useCoarse();
  const sheet = narrow && coarse;
  const reset = () => {
    setNever(false);
    setGoing(false);
    setRefused(null);
  };
  const press = () => {
    if (!pending || going) return;
    setGoing(true);
    setRefused(null);
    // The tick is written first and on its own: a delete the node then
    // refuses is no reason to forget that the person asked not to be
    // asked again.
    const tick = never ? setConfig("confirm-delete", "off").then(reread, () => {}) : Promise.resolve();
    tick
      .then(() => pending.run())
      .then(
        () => {
          reset();
          onDone();
        },
        (e: unknown) => {
          setGoing(false);
          setRefused(String(e instanceof Error ? e.message : e));
        },
      );
  };
  const keep = () => {
    reset();
    onDone();
  };
  if (sheet) {
    return (
      <Dialog open={pending !== null} onOpenChange={(o) => !o && keep()}>
        <DialogContent
          data-delete-dialog
          data-action-sheet
          // Two groups on the scrim, not one card: the question with its
          // act, then 取消 apart — iOS's action sheet.
          className="top-auto bottom-[calc(var(--safe-bottom)+var(--gap-in))] w-[calc(100%-2*var(--gap-in))] max-w-none translate-y-0 gap-in border-0 bg-transparent p-0 shadow-none"
        >
          <div className="flex flex-col overflow-hidden rounded-lg bg-popover">
            <DialogHeader className="items-center gap-xxs px-pane pt-base pb-in text-center">
              <DialogTitle className="text-callout font-title text-muted-foreground">
                {pending?.name ? gui.delete_named(pending.name) : gui.delete}
              </DialogTitle>
              <div data-delete-says className="text-callout text-muted-foreground">
                {pending?.says}
              </div>
              {refused !== null && (
                <div data-delete-refused className="text-callout text-destructive">
                  {refused}
                </div>
              )}
            </DialogHeader>
            <button
              type="button"
              data-delete-go
              disabled={going}
              aria-busy={going || undefined}
              onClick={press}
              className="min-h-touch border-t text-title-sm text-destructive"
            >
              {gui.delete}
            </button>
          </div>
          <button
            type="button"
            data-delete-keep
            onClick={keep}
            className="min-h-touch rounded-lg bg-popover text-title-sm font-title"
          >
            {gui.delete_keep}
          </button>
        </DialogContent>
      </Dialog>
    );
  }
  return (
    <Dialog
      open={pending !== null}
      onOpenChange={(o) => {
        if (!o) keep();
      }}
    >
      <DialogContent data-delete-dialog>
        <DialogHeader>
          <DialogTitle>{pending?.name ? gui.delete_named(pending.name) : gui.delete}</DialogTitle>
        </DialogHeader>
        <div data-delete-says className="text-body text-muted-foreground">
          {pending?.says}
        </div>
        <label className="flex items-center gap-2 text-callout text-muted-foreground">
          <input
            data-delete-never-ask
            type="checkbox"
            checked={never}
            onChange={(e) => setNever(e.target.checked)}
            // The finger floor on the box itself (tokens.css `--touch`).
            className="size-4 min-h-touch min-w-touch"
          />
          {gui.never_ask}
        </label>
        {refused !== null && (
          <div data-delete-refused className="text-callout text-destructive">
            {refused}
          </div>
        )}
        <DialogFooter>
          {/* components/Dialog: 取消 on the left as a push button, and the
              one act on the right — `destructive`, since it throws a thing
              away; the olive `primary` is for acts that make something. */}
          <Button data-delete-keep onClick={keep}>
            {gui.delete_keep}
          </Button>
          <Button variant="destructive" data-delete-go disabled={going} aria-busy={going || undefined} onClick={press}>
            {gui.delete}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
