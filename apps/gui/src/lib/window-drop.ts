// **Files dropped on the window from outside** (Finder; item 60): tauri
// takes the OS drop before the webview sees it, so the real paths arrive
// on tauri's window-wide event — in the browser a dropped file is a
// `File` with no path at all. Each listener gets every drop with its
// point in CSS pixels and checks it against its own box.
//
// **The dev build also hears `khor:window-drop`** on `window` (detail
// `{ paths, x, y }`, CSS pixels): the face gate's way to drop without
// an OS drag, which no browser can synthesize with paths in it.
import { inApp } from "@/api";

export type WindowDrop = { paths: string[]; x: number; y: number };

export const WINDOW_DROP_EVENT = "khor:window-drop";

/** Calls `handler` for every drop on the window; returns the undo. */
export function onWindowDrop(handler: (d: WindowDrop) => void): () => void {
  let undo: (() => void) | undefined;
  let stopped = false;
  if (inApp) {
    void (async () => {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        const un = await getCurrentWebview().onDragDropEvent((e) => {
          if (e.payload.type !== "drop" || !e.payload.position) return;
          // The event's position is in physical pixels (tauri's
          // `PhysicalPosition`); boxes are in CSS pixels.
          handler({
            paths: e.payload.paths,
            x: e.payload.position.x / devicePixelRatio,
            y: e.payload.position.y / devicePixelRatio,
          });
        });
        if (stopped) un();
        else undo = un;
      } catch {
        // No tauri here after all: nothing to listen to.
      }
    })();
  }
  const seam = (e: Event) => {
    const d = (e as CustomEvent<WindowDrop>).detail;
    if (d && Array.isArray(d.paths)) handler(d);
  };
  if (import.meta.env.DEV) window.addEventListener(WINDOW_DROP_EVENT, seam);
  return () => {
    stopped = true;
    undo?.();
    if (import.meta.env.DEV) window.removeEventListener(WINDOW_DROP_EVENT, seam);
  };
}
