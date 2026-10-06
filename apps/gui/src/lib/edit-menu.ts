// **khor's own items in the system's edit menu** (item 54). Page code
// cannot add to iOS's menu over a selection (拷贝 · 查询 · 翻译 · 搜索网页 ·
// 共享); the native side can (`src-tauri/src/edit_menu.rs`, the web view's
// `buildMenu(with:)`). So the page says, while a selection is up, which
// items the next menu carries, and the native side calls back here with
// the one chosen — 粘贴 with the pasteboard's text, read natively, since
// a page reading the clipboard outside a gesture gets iOS's own paste
// prompt instead.
import { editMenu } from "@/api";

export type EditItem = { id: string; title: string; run: (text?: string) => void };

let offered: EditItem[] = [];

/** The items the next menu over a selection adds; `[]` clears them. */
export function offerEditMenu(items: EditItem[]) {
  const same = items.length === offered.length && items.every((it, i) => it.id === offered[i].id && it.title === offered[i].title);
  offered = items;
  if (!same) void editMenu(items.map((it) => [it.id, it.title])).catch(() => {});
}

declare global {
  interface Window {
    __khorEditMenu?: (id: string, text?: string) => void;
  }
}

window.__khorEditMenu = (id, text) => offered.find((it) => it.id === id)?.run(text);
