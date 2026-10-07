// **khor's own items in the system's edit menu** (item 54). Page code
// cannot add to iOS's menu over a selection (拷贝 · 查询 · 翻译 · 搜索网页 ·
// 共享); the native side can (`src-tauri/src/edit_menu.rs`, the web view's
// `buildMenu(with:)`). So the page says, while a selection is up, which
// items the next menu carries, and the native side calls back here with
// the one chosen — 粘贴 with the pasteboard's text (else its picture),
// read natively, since a page reading the clipboard outside a gesture
// gets iOS's own paste prompt instead.
import { editMenu, editMenuAt, editMenuHide } from "@/api";
import type { MenuBox } from "@/lib/term-select";

/** A picture iOS's pasteboard held for 粘贴 when it held no text: its
    type and its bytes in base64 (item 60 part 4). */
export type EditImage = { type: string; data: string };

export type EditItem = { id: string; title: string; run: (text?: string, image?: EditImage) => void };

let offered: EditItem[] = [];

/** The items the next menu over a selection adds; `[]` clears them. */
export function offerEditMenu(items: EditItem[]) {
  const same = items.length === offered.length && items.every((it, i) => it.id === offered[i].id && it.title === offered[i].title);
  offered = items;
  if (!same) void editMenu(items.map((it) => [it.id, it.title])).catch(() => {});
}

/** **The system's menu at a selection khor drew** (item 73: with the
    keyboard up the terminal selects in its own cells, and iOS has no
    selection to show its menu at). Shown at `at` with `items`; `copy` is
    what 拷贝 writes, natively. A chosen item comes back as above. */
let shownAt = false;
export function showEditMenuAt(at: MenuBox, items: EditItem[], copy: string) {
  offered = items;
  shownAt = true;
  void editMenuAt(at, items.map((it) => [it.id, it.title]), copy).catch(() => {});
}

/** Puts it away, if it is up. */
export function hideEditMenuAt() {
  if (!shownAt) return;
  shownAt = false;
  void editMenuHide().catch(() => {});
}

declare global {
  interface Window {
    __khorEditMenu?: (id: string, text?: string, image?: EditImage) => void;
  }
}

window.__khorEditMenu = (id, text, image) => offered.find((it) => it.id === id)?.run(text, image ?? undefined);
