import { useEffect, useRef, useState, type ReactNode } from "react";
import { IconBack, IconForward, IconMore, IconSearch } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, DialogTitle, TaskActions, TaskOverlay } from "@/components/ui/dialog";
import { GlassCapsule, GlassItem } from "@/components/Glass";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import type { OfficeAction, OfficeComment, OfficeNavigation, OfficeThread, OfficeTools } from "@/components/office/types";
import { gui } from "@/gen/catalog";

export function OfficePanel({ title, close, initialHeight = "fit", children }: {
  title: string;
  close: () => void;
  initialHeight?: "fit" | "medium";
  children: ReactNode;
}) {
  return <Dialog open onOpenChange={open => { if (!open) close(); }}>
    <TaskOverlay purpose="selection" initialHeight={initialHeight}>
      <DialogTitle>{title}</DialogTitle>
      <TaskActions close={<Button onClick={close}>{gui.close}</Button>}/>
      {children}
    </TaskOverlay>
  </Dialog>;
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
    className="min-h-0 max-h-full min-w-0 max-w-full object-contain"
    aria-hidden="true"/>;
}

const PAGE_BATCH = 6;

export function OfficeThumbnails({ pages, close, go }: {
  pages: OfficeNavigation;
  close: () => void;
  go: (index: number) => void;
}) {
  const { index, count } = pages.page();
  const [batch, setBatch] = useState(() => Math.floor(Math.max(0, index) / PAGE_BATCH));
  const [number, setNumber] = useState(String(Math.max(0, index) + 1));
  const start = batch * PAGE_BATCH;
  const end = Math.min(count, start + PAGE_BATCH);
  return <OfficePanel title={gui.office_thumbnails} close={close} initialHeight="medium">
    <form className="flex items-center gap-in" onSubmit={event => {
      event.preventDefault();
      const page = Number(number);
      if (Number.isInteger(page) && page > 0 && page <= count) go(page - 1);
    }}>
      <label className="flex min-w-0 flex-1 items-center gap-in text-body">
        {gui.office_page_number}
        <Input type="number" min={1} max={count} value={number} onChange={event => setNumber(event.target.value)}
          className="min-w-0 tabular-nums"/>
      </label>
      <Button type="submit">{gui.office_page_go}</Button>
    </form>
    <div className="grid grid-cols-2 gap-in sm:grid-cols-3" data-office-thumbnails>
      {Array.from({ length: end - start }, (_, offset) => {
        const page = start + offset;
        return <button key={page} type="button" aria-current={page === index ? "page" : undefined}
          aria-label={gui.office_page_number + " " + (page + 1)} data-office-thumbnail={page + 1}
          className="flex min-w-0 flex-col items-center gap-in rounded-lg bg-fill p-2 aria-current:bg-fill-strong hover:bg-fill-strong"
          onClick={() => go(page)}>
          <span className="grid aspect-square w-full place-items-center overflow-hidden">
            <Thumbnail pages={pages} index={page}/>
          </span>
          <span className="text-callout tabular-nums">{page + 1}</span>
        </button>;
      })}
    </div>
    {count > PAGE_BATCH && <div className="flex items-center justify-center gap-in">
      <Button size="icon" variant="ghost" aria-label={gui.page_back} disabled={start === 0} onClick={() => setBatch(n => n - 1)}>
        <IconBack/>
      </Button>
      <span className="text-callout tabular-nums">{start + 1}–{end} / {count}</span>
      <Button size="icon" variant="ghost" aria-label={gui.page_forward} disabled={end === count} onClick={() => setBatch(n => n + 1)}>
        <IconForward/>
      </Button>
    </div>}
  </OfficePanel>;
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

export function OfficeComments({ threads, close, jump }: {
  threads: OfficeThread[];
  close: () => void;
  jump: (run: NonNullable<OfficeThread["jump"]>) => void;
}) {
  return <OfficePanel title={gui.office_comments} close={close}>
    {threads.length === 0 ? <p className="text-body text-muted-foreground">{gui.office_no_comments}</p> :
      <div data-office-comments className="flex flex-col divide-y">
        {threads.map(thread => <article key={thread.key} className="min-w-0 py-in">
          <CommentText comment={thread}/>
          {thread.replies?.map((reply, i) => <div key={i} className="mt-in border-s ps-row"><CommentText comment={reply}/></div>)}
          {thread.jump && <Button size="sm" variant="ghost" onClick={() => jump(thread.jump!)}>
            {thread.location ?? gui.office_go_comment}
          </Button>}
        </article>)}
      </div>}
  </OfficePanel>;
}

export function OfficeAttachmentTools({ tools }: { tools: OfficeTools }) {
  const pending = useRef<OfficeAction | null>(null);
  return <GlassCapsule aria-label={gui.file_actions} role="group">
    <GlassItem aria-label={gui.search_file} data-office-find-open onClick={tools.find}><IconSearch/></GlassItem>
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <GlassItem data-office-more aria-label={gui.rail_more}><IconMore/></GlassItem>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" onCloseAutoFocus={() => {
        const action = pending.current;
        pending.current = null;
        action?.run();
      }}>
        {tools.items.map(action => <DropdownMenuItem key={action.key} data-file-more-item={action.key}
          onSelect={() => { if (action.afterClose) pending.current = action; else action.run(); }}>
          {action.label}
        </DropdownMenuItem>)}
      </DropdownMenuContent>
    </DropdownMenu>
  </GlassCapsule>;
}
