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
import { listenNativeDragDrop } from "./native-events";
import { clearPathDrag, nativePathEnter, nativePathLeave, takeNativePath } from "./path-drag";

export type WindowDrop = { paths: string[]; x: number; y: number; from?: string | null };

export const WINDOW_DROP_EVENT = "khor:window-drop";

/** Wry 0.55 reports AppKit points and GTK widget coordinates despite
 * Tauri's PhysicalPosition wrapper; only WebView2 sends physical pixels. */
export function dropPoint(position: { x: number; y: number }, platform: string, scale: number) {
  const divisor = /Win/i.test(platform) ? scale : 1;
  return { x: position.x / divisor, y: position.y / divisor };
}

const handlers = new Set<(d: WindowDrop) => void>();
let stopListening: (() => void) | undefined;
const deliver = (drop: WindowDrop) => {
  for (const handler of handlers) {
    try { handler(drop); }
    catch {
      // A consumer's error text may itself contain dropped paths.
      console.error("khor: window drop handler failed");
    }
  }
};

/** One native listener resolves the source once, then every pane checks
 * its active state and bounds. A hidden retained terminal cannot consume
 * an internal drag before the visible terminal sees it. */
export function onWindowDrop(handler: (d: WindowDrop) => void): () => void {
  handlers.add(handler);
  if (handlers.size === 1) {
    let undo: (() => void) | undefined;
    if (inApp) undo = listenNativeDragDrop((e) => {
      const payload = e.payload;
      if (payload.type === "enter") { nativePathEnter(payload.paths); return; }
      if (payload.type === "leave") { nativePathLeave(); return; }
      if (payload.type !== "drop" || !payload.position) return;
      const source = payload.paths.length ? null : takeNativePath();
      if (payload.paths.length) nativePathEnter(payload.paths);
      if (!payload.paths.length && !source) return;
      deliver({ paths: source ? [source.path] : payload.paths, from: source?.machine ?? null,
        ...dropPoint(payload.position, navigator.platform, devicePixelRatio) });
    });
    const seam = (e: Event) => {
      const d = (e as CustomEvent<WindowDrop>).detail;
      if (d && Array.isArray(d.paths)) deliver(d);
    };
    if (import.meta.env.DEV) window.addEventListener(WINDOW_DROP_EVENT, seam);
    stopListening = () => {
      undo?.();
      clearPathDrag();
      if (import.meta.env.DEV) window.removeEventListener(WINDOW_DROP_EVENT, seam);
    };
  }
  return () => {
    handlers.delete(handler);
    if (!handlers.size) { stopListening?.(); stopListening = undefined; }
  };
}
