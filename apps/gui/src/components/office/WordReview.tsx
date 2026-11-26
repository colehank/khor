import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";
import { IconDocumentComment, IconDocumentRevisions } from "@/components/icons";
import { CommentText } from "@/components/office/OfficePanels";
import type { OfficeThread, WordReviewEntry, WordReviewSource } from "@/components/office/types";
import { WORD_PAGE_SURFACE } from "@/components/office/viewer";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";

type VisibleEntry = { entry: WordReviewEntry; x: number | null; y: number };
type CardPosition = { top: number; height: number };

function revisionLabel(kind: string) {
  switch (kind) {
    case "insertion": return gui.office_change_inserted;
    case "deletion": return gui.office_change_deleted;
    case "moveFrom": return gui.office_change_moved_from;
    case "moveTo": return gui.office_change_moved_to;
    default: return gui.office_tool_revisions;
  }
}

export function WordReview({ id, host, source, showComments, showRevisions, active, jump }: {
  id: string;
  host: RefObject<HTMLElement | null>;
  source: WordReviewSource;
  showComments: boolean;
  showRevisions: boolean;
  active?: string;
  jump: (thread: OfficeThread) => void;
}) {
  const rail = useRef<HTMLElement>(null);
  const cards = useRef<HTMLDivElement>(null);
  const [entries, setEntries] = useState<VisibleEntry[]>([]);
  const [positions, setPositions] = useState<CardPosition[]>([]);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportSize, setViewportSize] = useState({ railX: 0, height: 0 });

  useEffect(() => {
    const document = host.current;
    const margin = rail.current;
    if (!document || !margin) return;
    const cache = new Map<number, WordReviewEntry[]>();
    const pending = new Set<number>();
    const unanchored = source.unanchored(showComments, showRevisions);
    let disposed = false;
    let frame = 0;
    const update = () => {
      frame = 0;
      const viewport = document.getBoundingClientRect();
      const visible: VisibleEntry[] = [];
      const shown = new Set<string>();
      for (const page of document.querySelectorAll<HTMLElement>(WORD_PAGE_SURFACE)) {
        const index = Number(page.dataset.pageIndex);
        const bounds = page.getBoundingClientRect();
        if (bounds.bottom <= viewport.top || bounds.top >= viewport.bottom) continue;
        const entries = cache.get(index);
        if (!entries && !pending.has(index)) {
          pending.add(index);
          void source.page(index, showComments, showRevisions).then(items => {
            if (disposed) return;
            if (cache.size >= 12) cache.delete(cache.keys().next().value!);
            cache.set(index, items);
            schedule();
          }, () => {}).finally(() => pending.delete(index));
        }
        for (const entry of entries ?? []) {
          if (shown.has(entry.key)) continue;
          const anchor = entry.points.find(point => {
            const y = bounds.top + point.y * bounds.height;
            return y >= viewport.top && y <= viewport.bottom;
          });
          if (!anchor) continue;
          const x = bounds.left + anchor.x * bounds.width - viewport.left;
          visible.push({
            entry, x: x >= 0 && x <= viewport.width ? x : null,
            y: bounds.top + anchor.y * bounds.height - viewport.top,
          });
          shown.add(entry.key);
        }
      }
      visible.sort((a, b) => a.y - b.y);
      for (const entry of unanchored) visible.push({ entry, x: null, y: 0 });
      const railX = document.clientWidth + margin.clientLeft +
        parseFloat(getComputedStyle(margin).getPropertyValue("--gap-in"));
      const height = margin.clientHeight;
      setViewportSize(previous => previous.railX === railX && previous.height === height ? previous : { railX, height });
      setEntries(previous => previous.length === visible.length && previous.every((item, i) =>
        item.entry === visible[i].entry && item.x === visible[i].x && item.y === visible[i].y,
      ) ? previous : visible);
    };
    const schedule = () => {
      if (!disposed && !frame) frame = requestAnimationFrame(update);
    };
    const resize = new ResizeObserver(schedule);
    resize.observe(document);
    resize.observe(margin);
    const rendered = new MutationObserver(schedule);
    rendered.observe(document, { childList: true, subtree: true, attributes: true, attributeFilter: ["data-page-index", "style"] });
    document.addEventListener("scroll", schedule, { capture: true, passive: true });
    schedule();
    return () => {
      disposed = true;
      cancelAnimationFrame(frame);
      resize.disconnect();
      rendered.disconnect();
      document.removeEventListener("scroll", schedule, true);
    };
  }, [host, source, showComments, showRevisions]);

  useLayoutEffect(() => {
    const margin = rail.current;
    if (!margin || !cards.current) return;
    const layout = () => {
      const nodes = [...(cards.current?.children ?? [])] as HTMLElement[];
      const gap = parseFloat(getComputedStyle(margin).getPropertyValue("--gap-in"));
      const limit = margin.clientHeight - gap;
      let bottom = gap;
      const next = nodes.map((node, index) => {
        const height = node.offsetHeight;
        const top = Math.max(bottom, entries[index].y - height / 2);
        bottom = top + height + gap;
        return { top, height };
      });
      const needed = next.reduce((sum, item) => sum + item.height + gap, 0);
      if (needed <= limit && bottom > margin.clientHeight) {
        let end = limit;
        for (let i = next.length - 1; i >= 0; i--) {
          const card = next[i];
          card.top = Math.min(card.top, end - card.height);
          end = card.top - gap;
        }
      }
      setPositions(previous => previous.length === next.length &&
        previous.every((item, i) => item.top === next[i].top && item.height === next[i].height) ? previous : next);
    };
    const resize = new ResizeObserver(layout);
    resize.observe(margin);
    for (const node of cards.current.children) resize.observe(node);
    layout();
    return () => resize.disconnect();
  }, [entries]);

  const last = positions.at(-1);
  return <>
    <aside id={id} ref={rail} data-word-review
      aria-label={showComments && showRevisions ? gui.office_tool_comments + " · " + gui.office_tool_revisions :
        showComments ? gui.office_tool_comments : gui.office_tool_revisions}
      className="relative w-2/5 max-w-64 min-w-0 flex-none overflow-y-auto border-s bg-background"
      onScroll={event => setScrollTop(event.currentTarget.scrollTop)}>
      <div ref={cards} className="relative" style={{ height: last && `calc(${last.top + last.height}px + var(--gap-in))` }}>
        {entries.map(({ entry }, index) => <article key={entry.key} data-word-review-card
          data-revision={entry.revision} data-active={entry.key === active || undefined}
          style={{ top: positions[index]?.top ?? 0 }}
          className="absolute inset-x-in min-w-0 rounded-md border-s-2 border-s-ring bg-fill px-in py-in data-active:bg-fill-strong">
          <div className="mb-xxs flex min-w-0 items-center gap-in text-callout text-muted-foreground">
            {entry.revision ? <IconDocumentRevisions className="size-affordance shrink-0"/> :
              <IconDocumentComment className="size-affordance shrink-0"/>}
            <span>{entry.revision ? revisionLabel(entry.revision) : gui.office_tool_comments}</span>
          </div>
          <div className="max-h-24 overflow-y-auto">
            <CommentText comment={entry} hideDate/>
            {entry.replies?.map((reply, i) => <div key={i} className="mt-in border-s ps-in"><CommentText comment={reply} hideDate/></div>)}
          </div>
          {entry.jump && <Button size="sm" variant="ghost" onClick={() => jump(entry)}>{gui.office_go_comment}</Button>}
        </article>)}
      </div>
    </aside>
    <svg aria-hidden="true" data-word-review-connectors className="pointer-events-none absolute inset-0 z-10 size-full overflow-hidden">
      {entries.map(({ entry, x, y }, index) => {
        const card = positions[index];
        if (x === null || !card) return null;
        const end = card.top + card.height / 2 - scrollTop;
        if (end < 0 || end > viewportSize.height) return null;
        const path = `M ${x} ${y} L ${viewportSize.railX} ${end}`;
        return <g key={entry.key} fill="none" strokeDasharray="3 3">
          <path d={path} className="stroke-background/85" strokeWidth="3"/>
          <path d={path} data-word-review-connector className="stroke-ring"
            strokeWidth={entry.key === active ? 1.5 : 1}/>
        </g>;
      })}
    </svg>
  </>;
}
