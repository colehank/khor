// A file read first under a finger, edited on request (P1, 2026-10-03:
// the owner's ruling, 「像备忘录」).
//
// On a phone a file opens to be read: no editor over its text, so a tap
// never raises the keyboard. The file's head offers 「编辑」; pressing it
// puts the caret where the reader was and the keyboard up, and the item
// reads 「保存」 until pressed again, which saves and returns to reading
// (keyboard down). A touch outside the text while editing ends the
// typing (`FileView`'s blur) and discards nothing: the item still reads
// 保存. A save that does not land (a clash) leaves it editing. Under a
// mouse none of this exists: a click edits, as it always has.
import { useCallback, useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";

import { gui } from "@/gen/catalog";
import { useCoarse } from "@/hooks/use-coarse";
import { beginEditing } from "@/views/FileView";

export type EditItem = { label: string; writing: boolean; onPress: () => void };

export function useReadFirst(
  /** The open file; a new one opens to be read again. */
  key: string | null,
  /** Whether the buffer differs from the disk. */
  dirty: boolean,
  /** Asks the view to save (its `saveAt` counter). */
  save: () => void,
) {
  const coarse = useCoarse();
  const [writing, setWriting] = useState(false);
  const [can, setCan] = useState(false);
  const saving = useRef(false);
  /** The box holding the view, where its editor is found. */
  const box = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    setWriting(false);
    saving.current = false;
  }, [key]);
  /** For the view's `onSaved`: the save 保存 asked for landed. */
  const saved = useCallback(() => {
    if (!saving.current) return;
    saving.current = false;
    setWriting(false);
  }, []);
  const press = () => {
    if (!writing) {
      // The editor has to exist before the focus, and the focus has to
      // happen inside this press for the keyboard to come up.
      flushSync(() => setWriting(true));
      beginEditing(box.current);
      return;
    }
    (document.activeElement as HTMLElement | null)?.blur?.();
    if (dirty) {
      saving.current = true;
      save();
    } else setWriting(false);
  };
  // The catalog refuses a second key for one text, so 编辑 and 保存 are
  // the words the account editor and the watched folders already say.
  const item: EditItem | null =
    coarse && can && key !== null
      ? { label: writing ? gui.watch_save : gui.accounts_edit, writing, onPress: press }
      : null;
  return { reading: coarse && !writing, onCanEdit: setCan, saved, box, item };
}
