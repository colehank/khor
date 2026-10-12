// **One file's symbols** (cm-d item 5, the file page's 「…」): what the
// file defines, from its syntax tree (`lib/code/symbols`) — GitHub's
// 「Symbols」 on its file page. The one kind of popup (README 「一种弹窗」),
// the dialog 「git 历史」 is: a list of rows, each a name indented by what
// it stands inside and its line at the right; a press closes it and goes
// to that line, a third of the way down (KB-CARET rule 6).
//
// The list is taken when the dialog opens; the item is offered only where
// there is something to list (`symbolsIn`), so it is never empty.
//
// **A find row at its top** (cm-f; a 5000-symbol file is a list nobody
// scans): the 附栏's `SideFind`, pinned while the list scrolls under it.
// It narrows by name, any case, the list keeping the file's order; with
// nothing left it says the lists' one sentence, `no_matches`. On the wide
// face it has the focus as the dialog opens; on a phone it waits for a
// tap, or the keyboard would cover the list. Return with one row left
// goes there; Esc with text clears it, and with none puts the dialog away.
import { useMemo, useState } from "react";

import { SideFind } from "@/components/SideFind";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { goLineIn, symbolsIn } from "@/lib/code/place";
import type { CodeSymbol } from "@/lib/code/symbols";

export function SymbolsSheet({ open, onOpenChange, path }: { open: boolean; onOpenChange: (open: boolean) => void; path: string }) {
  const list = useMemo(() => (open ? (symbolsIn(path) ?? []) : []), [open, path]);
  const [query, setQuery] = useState("");
  // Each opening starts with an empty box.
  const [was, setWas] = useState(open);
  if (open !== was) {
    setWas(open);
    if (open) setQuery("");
  }
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q === "" ? list : list.filter((s) => s.name.toLowerCase().includes(q));
  }, [list, query]);
  const go = (s: CodeSymbol) => {
    onOpenChange(false);
    goLineIn(path, s.line);
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        form
        data-file-symbols
        className="flex max-h-[70dvh] flex-col !gap-0"
        onOpenAutoFocus={(e) => {
          e.preventDefault();
          if (document.documentElement.hasAttribute("data-narrow") || window.matchMedia("(pointer: coarse)").matches) return;
          (e.currentTarget as HTMLElement | null)?.querySelector<HTMLElement>("[data-side-find-field]")?.focus();
        }}
        onEscapeKeyDown={(e) => {
          if (query === "") return;
          e.preventDefault();
          setQuery("");
        }}
      >
        <DialogTitle className="px-in pb-in">{gui.keys_group_symbol}</DialogTitle>
        <SideFind label={gui.keys_group_symbol} query={query} onQuery={setQuery} onEnter={() => shown.length === 1 && go(shown[0])} className="px-0 pb-in" />
        <div className="flex min-h-0 min-w-0 flex-col overflow-y-auto">
          {shown.length === 0 ? (
            <div data-file-symbols-none className="px-in py-2 text-callout text-muted-foreground">
              {gui.no_matches}
            </div>
          ) : (
            shown.map((s) => (
              <button
                key={`${s.line}:${s.name}`}
                type="button"
                data-file-symbol={s.line}
                onClick={() => go(s)}
                className="flex min-h-list-row min-w-0 flex-none items-center gap-row rounded-md px-in text-left hover:bg-fill"
              >
                <span
                  className="min-w-0 flex-1 truncate font-mono text-body text-foreground"
                  // One step of the file's own indent per level, in the
                  // monospace the name is in.
                  style={{ paddingInlineStart: `${Math.min(s.depth, 6) * 2}ch` }}
                >
                  {s.name}
                </span>
                <span className="flex-none text-caption tabular-nums text-tertiary">{s.line}</span>
              </button>
            ))
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
