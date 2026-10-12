// The peek's head bar (cm-e item 2): VS Code's dirty-diff peek has one
// over the old lines — 「第 i 处，共 n 处」, the two steps, 「还原」, the way
// out — and khor's had none, so walking the changes meant finding the next
// bar by eye. Drawn by React because the glyphs and their tooltips are
// React's; mounted into the CodeMirror widget `git.ts` draws, one root a
// peek, rendered before the widget is measured (`flushSync`) so the block
// has its height the first time CodeMirror reads it.
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import type { ReactElement } from "react";

import { IconChevronDown, IconChevronUp, IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { gui } from "@/gen/catalog";

export type HeadActs = {
  /** The hunk's place among the file's, from 1, and how many. */
  at: number;
  of: number;
  step: (by: 1 | -1) => void;
  /** Null on a file that takes no edit: the button is not drawn. */
  revert: (() => void) | null;
  close: () => void;
};

/** A glyph-only control names itself under the pointer (F1.7). */
function tipped(button: ReactElement, words: string) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{button}</TooltipTrigger>
      <TooltipContent side="top">{words}</TooltipContent>
    </Tooltip>
  );
}

function Head({ at, of, step, revert, close }: HeadActs) {
  // The caret stays in the text: a press here is not a click into it.
  const keep = (e: { preventDefault(): void }) => e.preventDefault();
  const glyph = "min-h-touch min-w-touch size-ctl-md";
  return (
    <>
      <span data-peek-count className="me-auto truncate text-muted-foreground tabular-nums">
        {gui.git_hunk_count(at, of)}
      </span>
      {/* One change has no other to walk to: the steps are not drawn. */}
      {of > 1 &&
        tipped(
          <Button variant="ghost" size="icon" className={glyph} data-peek-prev aria-label={gui.git_hunk_prev} onMouseDown={keep} onClick={() => step(-1)}>
            <IconChevronUp className="size-affordance" />
          </Button>,
          gui.git_hunk_prev,
        )}
      {of > 1 &&
        tipped(
          <Button variant="ghost" size="icon" className={glyph} data-peek-next aria-label={gui.git_hunk_next} onMouseDown={keep} onClick={() => step(1)}>
            <IconChevronDown className="size-affordance" />
          </Button>,
          gui.git_hunk_next,
        )}
      {revert !== null && (
        <Button variant="ghost" size="sm" className="min-h-touch" data-peek-revert onMouseDown={keep} onClick={revert}>
          {gui.git_revert}
        </Button>
      )}
      {tipped(
        <Button variant="ghost" size="icon" className={glyph} data-peek-close aria-label={gui.close} onMouseDown={keep} onClick={close}>
          <IconClose className="size-affordance" />
        </Button>,
        gui.close,
      )}
    </>
  );
}

const roots = new WeakMap<HTMLElement, Root>();

/** The head, drawn into `dom` now. */
export function mountHead(dom: HTMLElement, acts: HeadActs) {
  const root = createRoot(dom);
  roots.set(dom, root);
  flushSync(() =>
    root.render(
      <TooltipProvider>
        <Head {...acts} />
      </TooltipProvider>,
    ),
  );
}

export function unmountHead(dom: HTMLElement) {
  const root = roots.get(dom);
  roots.delete(dom);
  // Not inside CodeMirror's own update: React refuses to unmount a root
  // synchronously while it may be rendering.
  if (root) queueMicrotask(() => root.unmount());
}
