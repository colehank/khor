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
  onSave,
  onDrop,
  onKeep,
}: {
  /** The file's own name, or empty when nothing is being asked about —
      which is also how this says it is not open. */
  name: string | null;
  /** Write it, then close. Rejecting leaves the tab where it is: a save
      that did not land must not take the buffer with it. */
  onSave: () => void;
  onDrop: () => void;
  onKeep: () => void;
}) {
  return (
    <Dialog open={name !== null} onOpenChange={(o) => (o ? undefined : onKeep())}>
      <DialogContent data-close-draft>
        <DialogHeader>
          <DialogTitle>{gui.file_unsaved}</DialogTitle>
        </DialogHeader>
        <div data-close-draft-says className="text-sm">
          {gui.tab_close_draft(name ?? "")}
        </div>
        <DialogFooter>
          {/* 别关 first in the source and last on the row, which is
              where a dialog's least destructive answer sits in this app
              already (`RemoveDialog`'s single button is the act, and
              the way out is the sheet's own close). */}
          <Button variant="ghost" data-close-keep onClick={onKeep} className="min-h-touch">
            {gui.tab_close_keep}
          </Button>
          <Button variant="outline" data-close-drop onClick={onDrop} className="min-h-touch">
            {gui.tab_close_drop}
          </Button>
          <Button data-close-save onClick={onSave} className="min-h-touch">
            {gui.tab_close_save}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
