// 删掉一个文件或目录，问一次 (工作区批④).
//
// The user's framing (2026-09-14):「khor 可以删文件啊,用户仍然右键某个
// 文件可以删除。但提示,跟 VS Code 一样,可以不再提醒」— so: a dialog
// with the thing being deleted named in it, and a tick that puts the
// dialog away for good.
//
// **Three sentences, not one with a word swapped.** What is being
// agreed to differs: a file on this machine goes to the recycle bin and
// can be fetched back out; a directory goes and takes everything under
// it; anything on another machine — and anything at all on a platform
// with no bin — is simply gone. A single sentence with a slot in it
// would make that difference a matter of wording.
//
// **The tick only silences the reversible one.** An act nobody can undo
// must not be silenceable by a tick somebody ticked once about
// something else, so the far-machine sentence is asked every time and
// the tick is not drawn beside it. That is not a setting anybody can
// find and change later, which is exactly why it must not cover the
// case where being wrong is permanent.
//
// **Which sentence it is comes from the node** (`fetchTrashHere`), not
// from the browser: the same page is served to a phone looking at a
// desktop's face, so a platform read off `navigator` would be the
// phone's platform and the desktop's disk.
import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";

/** What this removal will actually do — the node's answer, not a guess.
    `gone` covers every far machine and this one where there is no bin. */
export type RemoveKind = "trash" | "trash-dir" | "gone";

/** Whether the person has said not to ask again.

    **Per browser, like every other thing this app remembers about one
    screen** (`use-list-width`'s reasoning): it is a habit, not a
    decision about the network, and a habit does not belong in a
    replicated document. Storage can throw — a private window, blocked
    site data — and a throw reads as「not ticked」, which is the safe
    direction: the worst it costs is one dialog. */
const NEVER_ASK = "khor.remove-never-ask";

export function neverAsks(): boolean {
  try {
    return window.localStorage.getItem(NEVER_ASK) === "1";
  } catch {
    return false;
  }
}

function rememberNeverAsk() {
  try {
    window.localStorage.setItem(NEVER_ASK, "1");
  } catch {
    // Only the memory of the tick is lost, and the cost of that is
    // being asked again.
  }
}

export function RemoveDialog({
  open,
  name,
  kind,
  refused,
  onOpenChange,
  onConfirm,
}: {
  open: boolean;
  /** The thing being deleted, as the row prints it. */
  name: string;
  kind: RemoveKind;
  /** What the node said when it would not do it. The dialog stays open
      over it — the person is standing where they pressed, and the path
      they picked is still what the button is about. */
  refused: string | null;
  onOpenChange: (open: boolean) => void;
  /** Rejecting is how the caller says it did not happen (`RowMenu`'s
      rule); resolving closes the dialog. */
  onConfirm: (neverAgain: boolean) => Promise<void> | void;
}) {
  const [never, setNever] = useState(false);
  const [going, setGoing] = useState(false);
  const word =
    kind === "trash"
      ? gui.file_remove_trash(name)
      : kind === "trash-dir"
        ? gui.file_remove_dir(name)
        : gui.file_remove_gone(name);
  const press = () => {
    if (going) return;
    setGoing(true);
    if (never) rememberNeverAsk();
    Promise.resolve()
      .then(() => onConfirm(never))
      .then(
        () => {
          // **Let go of the press before closing.** `onOpenChange(false)`
          // shuts this from the outside, which does not run the
          // `Dialog`'s own change handler below — so the flag set on the
          // press would still be set the next time the sheet opened, and
          // 删掉 would come up dead.
          //
          // It was live from 工作区批④ and invisible, because
          // `FilesPane` dropped its listing on a successful delete and
          // took this component down with it: a reset nobody wrote and
          // nothing guaranteed. The workspace's tree keeps its sheet
          // mounted, and the second delete in `check-face` 4i found it.
          setGoing(false);
          setNever(false);
          onOpenChange(false);
        },
        () => setGoing(false),
      );
  };
  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        // A fresh opening asks a fresh question: the tick is not a
        // half-made decision to carry into the next file.
        if (!o) {
          setNever(false);
          setGoing(false);
        }
        onOpenChange(o);
      }}
    >
      <DialogContent data-remove-dialog>
        <DialogHeader>
          <DialogTitle>{gui.delete}</DialogTitle>
        </DialogHeader>
        <div data-remove-says className="text-callout">
          {word}
        </div>
        {/* Not drawn where being wrong is permanent (file head). */}
        {kind !== "gone" && (
          <label className="flex items-center gap-2 text-callout text-muted-foreground">
            <input
              data-remove-never-ask
              type="checkbox"
              checked={never}
              onChange={(e) => setNever(e.target.checked)}
              // The finger floor on the box itself, so what lights up
              // under a thumb is what is pressable (tokens.css
              // `--touch`); the label beside it is part of the target
              // because a `label` wrapping its own input already is.
              className="size-4 min-h-touch min-w-touch"
            />
            {gui.never_ask}
          </label>
        )}
        {refused !== null && (
          <div data-remove-refused className="text-callout text-destructive">
            {refused}
          </div>
        )}
        <DialogFooter>
          <Button data-remove-go disabled={going} onClick={press} className="min-h-touch">
            {gui.delete}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
