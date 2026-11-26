import { useEffect, useId, useLayoutEffect, useRef, useState, type RefObject } from "react";
import type { OfficeComment, WordReviewEntry, WordReviewSource } from "@/components/office/types";
import { WORD_PAGE_SURFACE } from "@/components/office/viewer";
import { gui } from "@/gen/catalog";

type TextLine = { page: number; top: number; height: number };
type VisibleEntry = { entry: WordReviewEntry; x: number | null; y: number; line: TextLine | null };
type CardPosition = { top: number; height: number };
type CommentFrame = {
  page: number; left: number; top: number; width: number; height: number; entries: WordReviewEntry[];
};
type ReviewConnector = { entry: WordReviewEntry; x: number; y: number; end: number; line: TextLine };

function groupConnectors(connectors: ReviewConnector[]): ReviewConnector[][] {
  const center = ({ top, height }: TextLine) => top + height / 2;
  connectors.sort((a, b) => a.line.page - b.line.page || center(a.line) - center(b.line) || a.x - b.x);
  const groups: ReviewConnector[][] = [];
  for (const connector of connectors) {
    const group = groups.at(-1);
    const first = group?.[0].line;
    const line = connector.line;
    // Underlines, deletions and comment boxes can attach at different
    // heights, even though their original text is on the same visual line.
    if (group && first && line.page === first.page && line.height > 0 && first.height > 0 &&
        Math.abs(center(line) - center(first)) <= Math.min(line.height, first.height) / 2) {
      group.push(connector);
    } else {
      groups.push([connector]);
    }
  }
  return groups;
}

function revisionLabel(kind: string) {
  switch (kind) {
    case "insertion": return gui.office_change_inserted;
    case "deletion": return gui.office_change_deleted;
    case "moveFrom": return gui.office_change_moved_from;
    case "moveTo": return gui.office_change_moved_to;
    default: return gui.office_tool_revisions;
  }
}

function ReviewMessage({ message, action, selected, select }: {
  message: OfficeComment;
  action: string;
  selected?: boolean;
  select?: () => void;
}) {
  const date = message.date ? new Date(message.date) : null;
  const time = date && Number.isFinite(date.getTime()) ?
    date.toLocaleString(undefined, { dateStyle: "short", timeStyle: "short" }) : null;
  const heading = <>
    <span className="font-label [overflow-wrap:anywhere]">{message.author?.trim() || gui.office_unknown_author}</span>
    <span className="text-muted-foreground">{action}</span>
  </>;
  const headingLayout = "flex flex-wrap items-baseline gap-x-in text-callout";
  return <>
    <header className="mb-xxs">
      {select ? <button type="button" data-word-review-select aria-pressed={selected}
        className={headingLayout + " w-full rounded-sm text-start focus-visible:outline-2 focus-visible:outline-ring"}
        onClick={select}>{heading}</button> : <div className={headingLayout}>{heading}</div>}
    </header>
    <div className="select-text whitespace-pre-wrap break-words text-body">{message.text}</div>
    {time && <time dateTime={message.date} title={date?.toLocaleString()}
      className="mt-xxs block tabular-nums text-caption text-muted-foreground">{time}</time>}
  </>;
}

export function WordReview({ id, host, source, showComments, showRevisions, active, jump }: {
  id: string;
  host: RefObject<HTMLElement | null>;
  source: WordReviewSource;
  showComments: boolean;
  showRevisions: boolean;
  active?: string;
  jump: (entry: WordReviewEntry) => void;
}) {
  const rail = useRef<HTMLElement>(null);
  const cards = useRef<HTMLDivElement>(null);
  const documentClip = useId();
  const [entries, setEntries] = useState<VisibleEntry[]>([]);
  const [commentFrames, setCommentFrames] = useState<CommentFrame[]>([]);
  const [positions, setPositions] = useState<CardPosition[]>([]);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportSize, setViewportSize] = useState({ width: 0, railX: 0, branchX: 0, height: 0 });
  const { authorInk } = source;

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
      const frames: CommentFrame[] = [];
      const shown = new Set<string>();
      for (const page of document.querySelectorAll<HTMLElement>(WORD_PAGE_SURFACE)) {
        const index = Number(page.dataset.pageIndex);
        const bounds = page.getBoundingClientRect();
        if (bounds.bottom <= viewport.top || bounds.top >= viewport.bottom) continue;
        const entries = cache.get(index);
        if (showComments && entries?.some(entry => entry.ranges?.length)) frames.push({
          page: index, entries, left: bounds.left - viewport.left, top: bounds.top - viewport.top,
          width: bounds.width, height: bounds.height,
        });
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
            line: { page: index, top: bounds.top + anchor.lineTop * bounds.height - viewport.top,
              height: anchor.lineHeight * bounds.height },
          });
          shown.add(entry.key);
        }
      }
      visible.sort((a, b) => a.y - b.y);
      for (const entry of unanchored) visible.push({ entry, x: null, y: 0, line: null });
      const width = document.clientWidth;
      const tokens = getComputedStyle(margin);
      const railX = width + margin.clientLeft + parseFloat(tokens.getPropertyValue("--gap-in"));
      const branchX = Math.max(0, width - parseFloat(tokens.getPropertyValue("--ctl-sm")));
      const height = margin.clientHeight;
      setViewportSize(previous => previous.width === width && previous.railX === railX &&
        previous.branchX === branchX && previous.height === height ? previous : { width, railX, branchX, height });
      setCommentFrames(previous => previous.length === frames.length && previous.every((frame, i) =>
        frame.page === frames[i].page && frame.entries === frames[i].entries && frame.left === frames[i].left &&
        frame.top === frames[i].top && frame.width === frames[i].width && frame.height === frames[i].height,
      ) ? previous : frames);
      setEntries(previous => previous.length === visible.length && previous.every((item, i) =>
        item.entry === visible[i].entry && item.x === visible[i].x && item.y === visible[i].y &&
        item.line?.page === visible[i].line?.page && item.line?.top === visible[i].line?.top &&
        item.line?.height === visible[i].line?.height,
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
  const groups = groupConnectors(entries.flatMap(({ entry, x, y, line }, index) => {
    const card = positions[index];
    if (x === null || !line || !card) return [];
    return [{ entry, x, y, line, end: card.top + card.height / 2 - scrollTop }];
  }));
  return <>
    <aside id={id} ref={rail} data-word-review
      aria-label={showComments && showRevisions ? gui.office_tool_comments + " · " + gui.office_tool_revisions :
        showComments ? gui.office_tool_comments : gui.office_tool_revisions}
      className="relative w-2/5 max-w-64 min-w-0 flex-none overflow-y-auto border-s bg-background"
      onScroll={event => setScrollTop(event.currentTarget.scrollTop)}>
      <div ref={cards} className="relative" style={{ height: last && `calc(${last.top + last.height}px + var(--gap-in))` }}>
        {entries.map(({ entry }, index) => <article key={entry.key} data-word-review-card
          data-revision={entry.revision} data-active={entry.key === active || undefined}
          onClick={entry.jump ? event => {
            if (event.target instanceof Element && event.target.closest("button")) return;
            const selection = event.currentTarget.ownerDocument.getSelection();
            if (selection?.isCollapsed === false && selection.containsNode(event.currentTarget, true)) return;
            jump(entry);
          } : undefined}
          style={{ top: positions[index]?.top ?? 0, borderInlineStartColor: authorInk(entry.author) }}
          className="absolute inset-x-in min-w-0 rounded-md border-s-2 bg-fill px-in py-in data-active:bg-fill-strong">
          <div className="max-h-24 overflow-y-auto">
            <ReviewMessage message={entry} action={entry.revision ? revisionLabel(entry.revision) : gui.office_tool_comments}
              selected={entry.key === active} select={entry.jump ? () => jump(entry) : undefined}/>
            {entry.replies?.map((reply, i) => <div key={i} className="mt-in border-s ps-in"
              style={{ borderInlineStartColor: authorInk(reply.author) }}>
              <ReviewMessage message={reply} action={gui.office_reply}/>
            </div>)}
          </div>
        </article>)}
      </div>
    </aside>
    <svg aria-hidden="true" data-word-review-connectors className="pointer-events-none absolute inset-0 z-10 size-full overflow-hidden">
      <defs>
        <clipPath id={documentClip}><rect width={viewportSize.width} height={viewportSize.height}/></clipPath>
      </defs>
      {showComments && <g clipPath={`url(#${documentClip})`}>
        {commentFrames.map(frame => <g key={frame.page}>
          {frame.entries.map(comment => {
            if (!comment.ranges?.length) return null;
            const ink = authorInk(comment.author);
            const selected = comment.key === active;
            return <g key={comment.key}>
              {comment.ranges.map((range, i) => <polygon key={i} data-word-comment-range
                points={range.map(({ x, y }) => `${frame.left + x * frame.width},${frame.top + y * frame.height}`).join(" ")}
                fill={ink} fillOpacity={selected ? 0.44 : 0.30} stroke={ink}
                strokeWidth={selected ? 1.5 : 1} strokeLinejoin="round"/>)}
            </g>;
          })}
        </g>)}
      </g>}
      {groups.map(group => {
        const visible = group.filter(({ end }) => end >= 0 && end <= viewportSize.height);
        if (visible.length === 0) return null;
        const first = group[0];
        const shared = group.length > 1;
        const rowX = Math.max(...group.map(({ x }) => x));
        const rowY = Math.max(0, Math.min(...group.map(({ line }) => line.top)));
        const junction = Math.max(rowX, viewportSize.branchX);
        const sharedInk = group.every(({ entry }) => entry.author === first.entry.author) ?
          authorInk(first.entry.author) : "oklch(57% 0 0)";
        return <g key={first.entry.key} fill="none">
          {shared && <path data-word-review-trunk d={`M ${rowX} ${rowY} H ${junction}`}
            stroke={sharedInk} strokeWidth="1.25" strokeDasharray="3 3"/>}
          {visible.map(({ entry, x, y, end }) => {
            const isComment = Boolean(entry.ranges?.length);
            const selectedComment = entry.key === active && isComment;
            const ink = authorInk(entry.author);
            const path = shared ? `M ${junction} ${rowY} L ${viewportSize.railX} ${end}` :
              isComment ? `M ${x} ${y} H ${viewportSize.width} L ${viewportSize.railX} ${end}` :
                `M ${x} ${y} L ${viewportSize.railX} ${end}`;
            return <g key={entry.key}>
              <path d={path} data-word-review-connector data-word-review-branch={shared || undefined}
                strokeDasharray={selectedComment ? undefined : "3 3"} stroke={ink}
                strokeWidth={entry.key === active ? 1.75 : 1.25}/>
              {!shared && selectedComment && <circle cx={x} cy={y} r="2" fill={ink}/>}
            </g>;
          })}
        </g>;
      })}
    </svg>
  </>;
}
