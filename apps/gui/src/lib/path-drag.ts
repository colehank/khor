// **A path dragged out of khor's own listings** (item 60): a row of the
// 附栏's 工作区 tree or of a machine page's listing carries its machine and
// its path, and a terminal it is dropped on puts that file where the
// session's machine reads it (`term_drop` with `from`) — the path itself
// when it is the same machine, the pull when it is not.
//
// An HTML drag inside the page, not the window's file drop: the OS drop
// (Finder) arrives as tauri's event with real paths, and these rows are
// not files the OS knows about — they are often on another machine. The
// plain text rides too, so a drop anywhere else (a text field, another
// app) gets the path as words.
import type { DragEvent } from "react";

/** The drag's own type; nothing outside khor reads it. */
export const PATH_DRAG = "application/x-khor-path";

/** A dragged row: the machine it is on (`null`: this one) and its path. */
export type DraggedPath = { machine: string | null; path: string };

/** Whether the hand is a pointer that drags. **Not a finger**: on a
    phone a draggable row would turn its long press — the row's menu —
    into a lift (the phone's road into a terminal is item 60's 📎 panel). */
const drags = () => typeof window !== "undefined" && !window.matchMedia("(pointer: coarse)").matches;

/** The props that make a row's button a drag of its path. */
export function pathDrag(machine: string | null, path: string) {
  if (!drags()) return {};
  return {
    draggable: true,
    onDragStart: (e: DragEvent) => {
      e.dataTransfer.setData(PATH_DRAG, JSON.stringify({ machine, path } satisfies DraggedPath));
      e.dataTransfer.setData("text/plain", path);
      e.dataTransfer.effectAllowed = "copy";
    },
  };
}

/** Whether a drag carries a khor path (readable during `dragover`, where
    only the types are). */
export function carriesPath(e: Pick<DragEvent, "dataTransfer">): boolean {
  return e.dataTransfer.types.includes(PATH_DRAG);
}

/** The path a drop carries, or `null` when it carries none. */
export function droppedPath(e: Pick<DragEvent, "dataTransfer">): DraggedPath | null {
  try {
    const v = JSON.parse(e.dataTransfer.getData(PATH_DRAG)) as Partial<DraggedPath>;
    if (typeof v.path !== "string" || !v.path) return null;
    return { machine: typeof v.machine === "string" && v.machine ? v.machine : null, path: v.path };
  } catch {
    return null;
  }
}
