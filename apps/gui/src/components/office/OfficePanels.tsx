import { useEffect, useRef, useState, type ReactNode } from "react";
import { IconBack, IconForward } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { OfficeComment, OfficeNavigation, OfficePages, OfficeThread } from "@/components/office/types";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

export function OfficeAnnotations({ id, title, beside, children }: {
  id: string;
  title: string;
  beside?: boolean;
  children: ReactNode;
}) {
  return <aside id={id} data-office-annotations={beside ? "comments" : "notes"} aria-label={title}
    className={cn("min-w-0 flex-none overflow-auto bg-background px-in py-in",
      beside ? "w-2/5 max-w-64 border-s @lg/office:px-row" : "max-h-44 border-t px-row")}>
    <div className="text-group text-muted-foreground">{title}</div>
    {children}
  </aside>;
}

function Thumbnail({ pages, index }: { pages: OfficeNavigation; index: number }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    let active = true;
    void pages.thumbnail(index).then(bitmap => {
      try {
        const target = canvas.current;
        if (!active || !target) return;
        target.width = bitmap.width;
        target.height = bitmap.height;
        target.getContext("2d")?.drawImage(bitmap, 0, 0);
      } finally { bitmap.close(); }
    }, () => {});
    return () => { active = false; };
  }, [pages, index]);
  return <canvas ref={canvas} width={3} height={4}
    className="min-h-0 max-h-full min-w-0 max-w-full object-contain" aria-hidden="true"/>;
}

const PAGE_BATCH = 6;

export function OfficeThumbnails({ id, pages, position, go }: {
  id: string;
  pages: OfficeNavigation;
  position: OfficePages;
  go: (index: number) => void;
}) {
  const { index, count } = position;
  const [chosenBatch, setBatch] = useState<number | null>(null);
  const [number, setNumber] = useState<string | null>(null);
  const batch = Math.min(chosenBatch ?? Math.floor(Math.max(0, index) / PAGE_BATCH), Math.floor(Math.max(0, count - 1) / PAGE_BATCH));
  const start = batch * PAGE_BATCH;
  const end = Math.min(count, start + PAGE_BATCH);
  const shownNumber = number ?? String(Math.max(0, index) + 1);
  const choosePage = (page: number) => {
    setNumber(null);
    setBatch(null);
    go(page);
  };
  return <aside id={id} data-office-thumbnails aria-label={gui.office_thumbnails}
    className="flex min-h-0 min-w-0 flex-none flex-col gap-in border-b bg-background p-in @2xl/office:w-36 @2xl/office:border-e @2xl/office:border-b-0">
    <div className="flex items-center justify-between gap-in @2xl/office:flex-wrap">
      <form className="flex min-w-0 items-center gap-in" onSubmit={event => {
        event.preventDefault();
        const page = Number(shownNumber);
        if (Number.isInteger(page) && page > 0 && page <= count) choosePage(page - 1);
      }}>
        <Button size="icon" type="button" variant="ghost" aria-label={gui.page_back} title={gui.page_back}
          disabled={index <= 0} onClick={() => choosePage(index - 1)}>
          <IconBack className="size-affordance"/>
        </Button>
        <Input type="number" aria-label={gui.office_page_number} title={gui.office_page_go} min={1} max={count}
          enterKeyHint="go"
          value={shownNumber} onChange={event => setNumber(event.target.value)}
          className="w-16 min-w-0 tabular-nums"/>
        <Button size="icon" type="button" variant="ghost" aria-label={gui.page_forward} title={gui.page_forward}
          disabled={index + 1 >= count} onClick={() => choosePage(index + 1)}>
          <IconForward className="size-affordance"/>
        </Button>
      </form>
      {count > PAGE_BATCH && <div className="flex items-center gap-xxs">
        <Button size="icon" variant="ghost" aria-label={gui.page_back} disabled={start === 0} onClick={() => setBatch(batch - 1)}>
          <IconBack className="size-affordance"/>
        </Button>
        <span className="text-caption tabular-nums">{start + 1}–{end}</span>
        <Button size="icon" variant="ghost" aria-label={gui.page_forward} disabled={end === count} onClick={() => setBatch(batch + 1)}>
          <IconForward className="size-affordance"/>
        </Button>
      </div>}
    </div>
    <div className="flex min-h-0 gap-in overflow-x-auto @2xl/office:flex-col @2xl/office:overflow-x-hidden @2xl/office:overflow-y-auto">
      {Array.from({ length: Math.max(0, end - start) }, (_, offset) => {
        const page = start + offset;
        return <button key={page} type="button" aria-current={page === index ? "page" : undefined}
          aria-label={gui.office_page_number + " " + (page + 1)} data-office-thumbnail={page + 1}
          data-current={page === index || undefined}
          className="flex w-20 shrink-0 flex-col items-center rounded-md p-in focus-visible:outline-2 focus-visible:outline-ring data-current:bg-fill-strong hover:bg-fill @2xl/office:w-full"
          onClick={() => choosePage(page)}>
          <span className="grid aspect-[4/3] w-full place-items-center overflow-hidden">
            <Thumbnail pages={pages} index={page}/>
          </span>
          <span className="text-caption tabular-nums text-muted-foreground">{page + 1}</span>
        </button>;
      })}
    </div>
  </aside>;
}

function CommentText({ comment }: { comment: OfficeComment }) {
  const date = comment.date ? new Date(comment.date) : null;
  const time = date && Number.isFinite(date.getTime()) ? date.toLocaleString() : null;
  return <>
    {(comment.author || time) && <div className="flex flex-wrap items-baseline gap-in text-callout text-muted-foreground">
      {comment.author && <span className="text-foreground">{comment.author}</span>}
      {time && <time className="tabular-nums" dateTime={comment.date}>{time}</time>}
    </div>}
    <div className="select-text whitespace-pre-wrap break-words text-body">{comment.text}</div>
  </>;
}

export function OfficeComments({ threads, active, jump }: {
  threads: OfficeThread[];
  active?: string;
  jump: (thread: OfficeThread) => void;
}) {
  return threads.length === 0 ? <p className="text-body text-muted-foreground">{gui.office_no_comments}</p> :
    <div data-office-comments className="flex flex-col divide-y">
      {threads.map(thread => <article key={thread.key} data-office-comment-thread data-active={thread.key === active || undefined}
        className="min-w-0 break-words rounded-md px-in py-in data-active:bg-fill">
        <CommentText comment={thread}/>
        {thread.replies?.map((reply, i) => <div key={i} className="mt-in border-s ps-row"><CommentText comment={reply}/></div>)}
        {thread.jump && <Button size="sm" variant="ghost" onClick={() => jump(thread)}>
          {thread.location ?? gui.office_go_comment}
        </Button>}
      </article>)}
    </div>;
}
