// **One file's symbols** (cm-d item 5, the file page's 「…」): what the
// file defines, from its syntax tree (`lib/code/symbols`) — GitHub's
// 「Symbols」 on its file page. The one kind of popup (README 「一种弹窗」),
// the dialog 「git 历史」 is: a list of rows, each a name indented by what
// it stands inside and its line at the right; a press closes it and goes
// to that line, a third of the way down (KB-CARET rule 6).
//
// The list is taken when the dialog opens; the item is offered only where
// there is something to list (`symbolsIn`), so it is never empty.
import { useMemo } from "react";

import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { goLineIn, symbolsIn } from "@/lib/code/place";

export function SymbolsSheet({ open, onOpenChange, path }: { open: boolean; onOpenChange: (open: boolean) => void; path: string }) {
  const list = useMemo(() => (open ? (symbolsIn(path) ?? []) : []), [open, path]);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent form data-file-symbols className="max-h-[70dvh] !gap-0 overflow-y-auto">
        <DialogTitle className="px-in pb-in">{gui.keys_group_symbol}</DialogTitle>
        <div className="flex min-w-0 flex-col">
          {list.map((s) => (
            <button
              key={`${s.line}:${s.name}`}
              type="button"
              data-file-symbol={s.line}
              onClick={() => {
                onOpenChange(false);
                goLineIn(path, s.line);
              }}
              className="flex min-h-list-row min-w-0 items-center gap-row rounded-md px-in text-left hover:bg-fill"
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
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
