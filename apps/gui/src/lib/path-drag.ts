// **A path dragged out of khor's own listings** (item 60): a row of the
// 附栏's 工作区 tree or of a machine page's listing carries its machine and
// its path, and a terminal it is dropped on puts that file where the
// session's machine reads it (`term_drop` with `from`) — the path itself
// when it is the same machine, the pull when it is not.
//
// HTML carries its source machine and path; native Wry may intercept its
// target drop with no OS paths, so window-drop resolves that same source.
// Finder still arrives with real OS paths. Plain text rides too, so a
// text field or another app can keep receiving the path as words.
import type { DragEvent } from "react";

/** The drag's own type; nothing outside khor reads it. */
export const PATH_DRAG = "application/x-khor-path";

/** A dragged row: the machine it is on (`null`: this one) and its path. */
export type DraggedPath = { machine: string | null; path: string };

// Native Wry intercepts the target's HTML drop, including an internal
// path with no OS file paths. Keep its source only for this drag. Native
// Drop is the success signal; dragend never submits anything itself.
let serial = 0;
let pending: { token: string; source: Element; path: DraggedPath; entered: boolean; ended: boolean } | null = null;
let consumed: string | null = null;
let expiry: ReturnType<typeof setTimeout> | undefined;
const cancelKey = (event: KeyboardEvent) => { if (event.key === "Escape") clearPending(); };
export const clearPathDrag = () => {
  clearTimeout(expiry);
  pending = null;
  window.removeEventListener("keydown", cancelKey, true);
  window.removeEventListener("blur", clearPathDrag);
};
const clearPending = clearPathDrag;

export function nativePathEnter(paths: string[]) {
  // A new OS drag cannot inherit a completed or cancelled HTML source.
  if (paths.length || pending?.ended) clearPending();
  if (pending) pending.entered = true;
}
export function nativePathLeave() {
  if (pending?.ended) clearPending();
  else if (pending) pending.entered = false;
}
export function takeNativePath(): DraggedPath | null {
  if (!pending?.entered) return null;
  if (!pending.source.isConnected) { clearPending(); return null; }
  const { token, path } = pending;
  consumed = token;
  clearPending();
  return path;
}

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
      clearPending();
      const token = String(++serial);
      consumed = null;
      window.addEventListener("keydown", cancelKey, true);
      window.addEventListener("blur", clearPathDrag);
      pending = { token, source: e.currentTarget, path: { machine, path }, entered: false, ended: false };
      e.dataTransfer.setData(PATH_DRAG, JSON.stringify({ machine, path, token }));
      e.dataTransfer.setData("text/plain", path);
      e.dataTransfer.effectAllowed = "copy";
    },
    onDragEnd: (e: DragEvent) => {
      if (!pending || pending.source !== e.currentTarget) return;
      // Wry owns acceptance, so WebKit's dropEffect is not authoritative.
      // A native Leave cancels; only this same drag's native Drop may
      // consume an ended source during the IPC tail below.
      if (!pending.entered) { clearPending(); return; }
      pending.ended = true;
      const token = pending.token;
      // Rust-to-JS delivery may finish after WebKit's dragend. This grace
      // belongs to the same accepted native drag only; a new Enter clears
      // it, and only native Drop can consume it. No delayed paste runs.
      expiry = setTimeout(() => { if (pending?.token === token) clearPending(); }, 500);
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
    const v = JSON.parse(e.dataTransfer.getData(PATH_DRAG)) as Partial<DraggedPath> & { token?: string };
    if (typeof v.path !== "string" || !v.path) return null;
    if (v.token) {
      if (consumed === v.token) return null;
      consumed = v.token;
      if (pending?.token === v.token) clearPending();
    }
    return { machine: typeof v.machine === "string" && v.machine ? v.machine : null, path: v.path };
  } catch {
    return null;
  }
}
