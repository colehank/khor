/**
 * **A list seat's find row in the 附栏** (UX-40, the owner 2026-10-07
 * 01:50: 「浏览器、工作区这类有 list 的，得支持查找和新建」). The row every
 * list already wears on its landing (the search box, then ＋; 2026-09-28,
 * `PaneBar`, the tree's head), put back under the 附栏's one head: the
 * head says who and which seat, this row does the seat's work. Pinned,
 * never pulled down out of hiding: on the phone a pull down is the
 * sheet's own way away. ＋ only where there is something to make
 * (progressive disclosure): 最近 has none.
 */
import type { ReactNode } from "react";

import { IconSearch } from "@/components/icons";
import { Input } from "@/components/ui/input";

export function SideFind({
  label,
  query,
  onQuery,
  plus,
}: {
  /** Names the box and stands in as its placeholder. */
  label: string;
  query: string;
  onQuery: (q: string) => void;
  /** The seat's ＋, when it has one. */
  plus?: ReactNode;
}) {
  return (
    <div data-side-find data-search-glass className="flex min-h-pane-head flex-none items-center gap-in px-(--page-inset)">
      <div data-search-wrap className="relative min-w-0 flex-1">
        <IconSearch className="pointer-events-none absolute top-1/2 left-2 z-10 -translate-y-1/2 text-tertiary" />
        <Input
          data-side-find-field
          data-search-field
          type="search"
          aria-label={label}
          placeholder={label}
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Escape" && query !== "" && (e.stopPropagation(), onQuery(""))}
          className="min-h-touch pl-7"
        />
      </div>
      {plus}
    </div>
  );
}
