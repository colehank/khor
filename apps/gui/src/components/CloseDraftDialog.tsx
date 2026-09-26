// 关一个还有草稿的标签，问一句 (工作区批⑧).
//
// 「关一个有草稿的标签问『保存 / 不保存 / 取消』」— docs/WORKSPACE.md
// 第二轮, from the user's VS Code framing of the whole strip.
//
// **Three exits, three sentences, and no tick.** `RemoveDialog` next
// door carries a 「以后不再问」 because putting a file in the recycle
// bin can be undone; throwing away what somebody typed cannot, and the
// rule that dialog states applies here whole: an act nobody can undo
// must not be silenceable by a tick made once about something else.
//
// **The name is in the sentence.** A person with seven tabs open asked
// 「还没存」 has been told the least useful half of the fact; which one
// is the question.
//
// # The quit guard uses the same sheet, and has two exits
//
// 工作区第五轮 第 39 条: closing the app over unsaved files asks here
// too, because it is the same question one step larger — and a second
// sheet with the same three buttons in different words would be two
// places to keep one promise.
//
// What it does not offer is 存了再退. Saving N files is N saves, each
// with its own digest contract and its own way to be refused, and a
// button that saved four of five and then quit would be the worst
// answer khor could give. The way to save them is the way to save any
// file: go back to the tab. That is one press away, and 别退 is the
// press.
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";

export function CloseDraftDialog({
  name,
  quitting,
  onSave,
  onDrop,
  onKeep,
}: {
  /** The file's own name, or null when nothing is being asked about —
      which is also how this says it is not open. */
  name: string | null;
  /** How many files are unsaved, when what is being closed is **the
      app** rather than one tab (工作区第五轮 第 39 条). Absent is the
      tab's question; present opens the sheet on its own, so the two
      callers never have to agree about a name that does not exist. */
  quitting?: number;
  /** Write it, then close. Rejecting leaves the tab where it is: a save
      that did not land must not take the buffer with it. Never called
      while quitting — there is no 存 button there. */
  onSave: () => void;
  onDrop: () => void;
  onKeep: () => void;
}) {
  const quit = quitting !== undefined;
  return (
    <Dialog
      open={quit || name !== null}
      onOpenChange={(o) => (o ? undefined : onKeep())}
    >
      <DialogContent data-close-draft data-quit={quit || undefined}>
        <DialogHeader>
          <DialogTitle>{gui.file_unsaved}</DialogTitle>
        </DialogHeader>
        <div data-close-draft-says className="text-body text-muted-foreground">
          {quit ? gui.quit_drafts(quitting) : gui.tab_close_draft(name ?? "")}
        </div>
        <DialogFooter>
          {/* 别关 first in the source and last on the row, which is
              where a dialog's least destructive answer sits in this app
              already (`RemoveDialog`'s single button is the act, and
              the way out is the sheet's own close). */}
          <Button data-close-keep onClick={onKeep}>
            {quit ? gui.quit_keep : gui.tab_close_keep}
          </Button>
          <Button data-close-drop onClick={onDrop}>
            {quit ? gui.quit_drop : gui.tab_close_drop}
          </Button>
          {!quit && (
            <Button variant="primary" data-close-save onClick={onSave}>
              {gui.tab_close_save}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
