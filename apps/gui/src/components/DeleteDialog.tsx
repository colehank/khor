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
import { useState } from "react";

import { setConfig } from "@/api";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
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
  return (
    <Dialog
      open={pending !== null}
      onOpenChange={(o) => {
        if (!o) {
          reset();
          onDone();
        }
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
          <Button
            data-delete-keep
            onClick={() => {
              reset();
              onDone();
            }}
          >
            {gui.delete_keep}
          </Button>
          <Button variant="destructive" data-delete-go disabled={going} onClick={press}>
            {gui.delete}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
