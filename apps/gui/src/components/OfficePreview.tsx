import { createPortal } from "react-dom";
import { useCallback, useEffect, useEffectEvent, useId, useRef, useState, type ComponentProps } from "react";
import {
  IconBack, IconClose, IconDocumentComment, IconDocumentNotes, IconDocumentRevisions, IconDocumentThumbnails,
  IconForward, IconMinus, IconPlus, IconSearch,
} from "@/components/icons";
import { ContentSearch } from "@/components/ContentSearch";
import { RegionState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { OfficeAnnotations, OfficeComments, OfficeThumbnails } from "@/components/office/OfficePanels";
import { WordReview } from "@/components/office/WordReview";
import type { OfficeCommentAnchor, OfficePages, OfficeThread } from "@/components/office/types";
import { loadOfficeReader, type OfficeReader } from "@/components/office/viewer";
import { gui } from "@/gen/catalog";
import { assertPageUrl, openSystemUrl } from "@/lib/client-opener";
import { imeKey } from "@/lib/ime";
import { visiblePageOwner } from "@/lib/native-page-yield";
import { cn } from "@/lib/utils";
import { useLinkHome } from "@/views/linkHome";
import docxParser from "@office-parser/docx_parser_bg.wasm?url";
import pptxParser from "@office-parser/pptx_parser_bg.wasm?url";
import xlsxParser from "@office-parser/xlsx_parser_bg.wasm?url";

export const OFFICE_PREVIEW_LIMIT = 16 * 1024 * 1024;
const PARSERS = { docx: docxParser, pptx: pptxParser, xlsx: xlsxParser };
export type OfficeFormat = keyof typeof PARSERS;
export type OfficeFailure = "unavailable" | "too-large";
export function officeFormat(path: string): OfficeFormat | null {
  const name = path.split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  const extension = dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
  return extension === "docx" || extension === "pptx" || extension === "xlsx" ? extension : null;
}

async function documentBytes(source: string | File): Promise<ArrayBuffer> {
  if (typeof source !== "string") return source.arrayBuffer();
  const raw = atob(source);
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < bytes.length; i++) bytes[i] = raw.charCodeAt(i);
  return bytes.buffer;
}

type Lock = "encrypted" | "invalid-password" | "unsupported-encryption";
type Zoom = "zoomIn" | "zoomOut" | "fitWidth" | "fitPage" | "actual";

function OfficeTool({ className, ...props }: ComponentProps<typeof Button>) {
  return <Button variant="ghost" size="sm" type="button" title={props["aria-label"]} {...props}
    className={cn("min-h-(--hit) min-w-(--hit) shrink-0 gap-in aria-pressed:bg-fill-strong aria-pressed:text-foreground [&_svg]:size-affordance", className)}/>;
}

export function OfficePreview({ format, source, onFallback }: {
  format: OfficeFormat;
  source: string | File;
  onFallback: (reason: OfficeFailure) => void;
}) {
  const root = useRef<HTMLDivElement>(null);
  const mount = useRef<HTMLDivElement>(null);
  const password = useRef<string>(undefined);
  const fit = useRef<"fitWidth" | "fitPage" | null>("fitPage");
  const [lock, setLock] = useState<Lock | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [reader, setReader] = useState<OfficeReader | null>(null);
  const [position, setPosition] = useState<OfficePages | null>(null);
  const [showThumbnails, setShowThumbnails] = useState(false);
  const [showComments, setShowComments] = useState(false);
  const [showNotes, setShowNotes] = useState(false);
  const [selectedComment, setSelectedComment] = useState<{
    key: string; context: number; anchor: OfficeCommentAnchor | null;
  } | null>(null);
  const commentRequest = useRef(0);
  const thumbnailsId = useId();
  const commentsId = useId();
  const notesId = useId();
  const [tracked, setTracked] = useState(false);
  const [scale, setScale] = useState(1);
  const [finding, setFinding] = useState(false);
  const [query, setQuery] = useState("");
  const [matches, setMatches] = useState<{ index: number; total: number } | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const home = useLinkHome();

  const reportFailure = useEffectEvent(onFallback);
  const follow = useEffectEvent(async (url: string) => {
    try {
      assertPageUrl(url);
      if (home?.openPage) await home.openPage(url);
      else await openSystemUrl(url);
    } catch {
      setNotice(gui.office_action_failed);
    }
  });

  useEffect(() => {
    const host = mount.current;
    if (!host) return;
    const abort = new AbortController();
    let opened: OfficeReader | undefined;
    let parserUrl: string | undefined;
    let resize: ResizeObserver | undefined;
    const active = () => !abort.signal.aborted;
    const fallback = (error?: unknown) => {
      if (!active()) return;
      const code = error && typeof error === "object" && "code" in error ? error.code : null;
      if (code === "encrypted" || code === "invalid-password" || code === "unsupported-encryption") {
        setLock(code);
        return;
      }
      reportFailure(code === "ooxml-resource-limit" || code === "ooxml-decoded-image-limit" ? "too-large" : "unavailable");
    };
    const timeout = setTimeout(fallback, 20_000);
    const pass = password.current;
    password.current = undefined;
    fit.current = "fitPage";
    setReader(null);
    setLock(null);
    setPosition(null);
    setShowThumbnails(false);
    setShowComments(false);
    setShowNotes(false);
    setSelectedComment(null);
    setTracked(false);
    setFinding(false);
    setQuery("");
    setNotice(null);

    async function load(host: HTMLDivElement) {
      const response = await fetch(PARSERS[format], { signal: abort.signal });
      if (!response.ok) throw new Error("Office parser unavailable");
      const [parserBytes, bytes] = await Promise.all([response.arrayBuffer(), documentBytes(source)]);
      if (!active()) return;
      parserUrl = URL.createObjectURL(new Blob([parserBytes], { type: "application/wasm" }));
      const view = await loadOfficeReader(format, host, bytes, parserUrl, pass, {
        signal: abort.signal,
        error: fallback,
        externalLink: url => { if (active()) void follow(url); },
        scale: value => { if (active()) setScale(value); },
        position: page => { if (active()) setPosition(page); },
      });
      if (!active()) { view.dispose(); return; }
      opened = view;
      if (format === "pptx") {
        const { viewer } = view;
        // Keep automatic fit while the user is using a fit mode. Manual
        // zoom survives resizing the sidebar and showing the find bar.
        resize = new ResizeObserver(() => {
          const mode = fit.current;
          if (active() && mode) void Promise.resolve(viewer[mode]()).catch(fallback);
        });
        resize.observe(host);
      }
      setScale(view.viewer.getScale());
      setReader(view);
    }

    void load(host).catch(fallback).finally(() => clearTimeout(timeout));
    return () => {
      abort.abort();
      commentRequest.current++;
      clearTimeout(timeout);
      resize?.disconnect();
      opened?.dispose();
      host.replaceChildren();
      if (parserUrl) URL.revokeObjectURL(parserUrl);
    };
  }, [format, source, attempt]);

  useEffect(() => {
    if (!reader) return;
    const { viewer } = reader;
    let active = true;
    const text = query.trim();
    setMatches(null);
    if (!finding || !text) {
      viewer.clearFind();
      return;
    }
    const id = setTimeout(() => {
      void viewer.findText(text).then(async results => {
        if (!active) return;
        const first = results.length ? await viewer.findNext() : null;
        if (active) setMatches({ index: first?.matchIndex ?? 0, total: results.length });
      }).catch(() => { if (active) setNotice(gui.office_action_failed); });
    }, 160);
    return () => {
      active = false;
      clearTimeout(id);
    };
  }, [reader, finding, query, tracked]);

  const nextMatch = (direction: 1 | -1) => {
    if (!reader || !matches?.total) return;
    const { viewer } = reader;
    void (direction === 1 ? viewer.findNext() : viewer.findPrev()).then(match => {
      if (match) setMatches(current => current ? { ...current, index: match.matchIndex } : null);
    }).catch(() => setNotice(gui.office_action_failed));
  };
  const closeFind = () => { setFinding(false); setQuery(""); root.current?.focus({ preventScroll: true }); };
  const run = useCallback((operation: () => void | Promise<unknown>) => {
    setNotice(null);
    try { void Promise.resolve(operation()).catch(() => setNotice(gui.office_action_failed)); }
    catch { setNotice(gui.office_action_failed); }
  }, []);
  const zoom = useCallback((command: Zoom) => {
    if (!reader) return;
    fit.current = command === "fitWidth" || command === "fitPage" ? command : null;
    if (command === "actual") run(() => reader.viewer.setScale(1));
    else run(() => reader.viewer[command]());
  }, [reader, run]);
  const changeRevisions = useCallback(() => {
    const revisions = reader?.revisions;
    if (!revisions) return;
    run(async () => {
      const next = !tracked;
      await revisions(next);
      setTracked(next);
    });
  }, [reader, run, tracked]);
  useEffect(() => {
    if (!reader) return;
    const keyboard = (event: KeyboardEvent) => {
      const region = root.current;
      if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey || event.key.toLowerCase() !== "f" ||
          event.defaultPrevented || !region || !visiblePageOwner(region)) return;
      const focused = document.activeElement;
      if (focused && focused !== document.body && !region.contains(focused) &&
          focused.closest('input, textarea, [contenteditable="true"], [role="dialog"]')) return;
      event.preventDefault();
      setFinding(true);
      region.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
    };
    document.addEventListener("keydown", keyboard);
    return () => document.removeEventListener("keydown", keyboard);
  }, [reader]);

  const pages = reader?.pages;
  const page = position ?? pages?.page();
  const wordReview = reader?.wordReview;
  const context = format === "docx" ? 0 : position?.index ?? 0;
  const activeComment = showComments && selectedComment?.context === context ? selectedComment : null;
  const anchor = activeComment?.anchor;
  const selectComment = (thread: OfficeThread) => {
    const jump = thread.jump;
    if (!jump) return;
    const request = ++commentRequest.current;
    run(async () => {
      const result = await jump();
      if (request !== commentRequest.current) return;
      setSelectedComment({ key: thread.key, context, anchor: typeof result === "object" ? result : null });
      if (result === false) setNotice(gui.office_action_failed);
    });
  };
  const toggleComments = () => {
    commentRequest.current++;
    setSelectedComment(null);
    setShowComments(open => !open);
  };

  return <div ref={root} tabIndex={-1} data-office-preview={format} data-band-boundary data-ready={!!reader || undefined}
    className="@container/office flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden outline-none">
    {reader && <div data-office-toolbar className="flex h-pane-head min-w-0 flex-none items-center border-b bg-background">
      {finding ? <div data-office-find onKeyDownCapture={event => {
        if (event.key === "Escape" && !imeKey(event)) {
          event.preventDefault();
          event.stopPropagation();
          closeFind();
        }
      }} className="flex min-w-0 flex-1 items-center gap-in px-row">
        <div className="min-w-0 flex-1">
          <ContentSearch label={gui.search_file} value={query} onChange={setQuery} onEditing={() => {}}
            onEnter={() => nextMatch(1)}/>
        </div>
        <div className="flex flex-none items-center">
          {query.trim() && <span data-office-find-count role="status" className="text-caption tabular-nums text-muted-foreground">
            {matches === null ? gui.office_searching : matches.total ? (matches.index + 1) + " / " + matches.total : gui.no_matches}
          </span>}
          <Button size="icon" variant="ghost" aria-label={gui.match_prev} disabled={!matches?.total} onClick={() => nextMatch(-1)}><IconBack/></Button>
          <Button size="icon" variant="ghost" aria-label={gui.match_next} disabled={!matches?.total} onClick={() => nextMatch(1)}><IconForward/></Button>
          <Button size="icon" variant="ghost" aria-label={gui.close} onClick={closeFind}><IconClose/></Button>
        </div>
      </div> : <div role="toolbar" aria-label={gui.file_actions}
        className="flex h-full min-w-0 flex-1 items-center gap-xxs overflow-x-auto px-in">
        <OfficeTool data-office-find-open aria-label={gui.search_file} onClick={() => setFinding(true)}><IconSearch/></OfficeTool>
        <div role="group" aria-label={gui.office_tool_zoom} className="flex shrink-0 items-center border-s ps-xxs">
          <OfficeTool data-office-zoom-out aria-label={gui.office_zoom_out} onClick={() => zoom("zoomOut")}><IconMinus/></OfficeTool>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <OfficeTool data-office-zoom aria-label={gui.office_tool_zoom + " " + Math.round(scale * 100) + "%"}
                className="tabular-nums">{Math.round(scale * 100)}%</OfficeTool>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="center">
              <DropdownMenuItem data-office-fit-width onSelect={() => zoom("fitWidth")}>{gui.office_fit_width}</DropdownMenuItem>
              {pages && <DropdownMenuItem data-office-fit-page onSelect={() => zoom("fitPage")}>{gui.office_fit_page}</DropdownMenuItem>}
              <DropdownMenuItem data-office-actual-size onSelect={() => zoom("actual")}>{gui.office_actual_size}</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <OfficeTool data-office-zoom-in aria-label={gui.office_zoom_in} onClick={() => zoom("zoomIn")}><IconPlus/></OfficeTool>
        </div>
        <div className="flex shrink-0 items-center border-s ps-xxs">
          {pages && <OfficeTool data-office-pages aria-label={gui.office_thumbnails} aria-pressed={showThumbnails}
            aria-controls={thumbnailsId} onClick={() => setShowThumbnails(on => !on)}>
            <IconDocumentThumbnails/><span className="hidden @lg/office:inline">{gui.office_tool_pages}</span>
          </OfficeTool>}
          <OfficeTool data-office-review aria-label={gui.office_comments} aria-pressed={showComments}
            aria-controls={commentsId} onClick={toggleComments}>
            <IconDocumentComment/><span className="hidden @lg/office:inline">{gui.office_tool_comments}</span>
          </OfficeTool>
          {reader.revisions && <OfficeTool data-office-revisions aria-pressed={tracked}
            aria-controls={wordReview ? commentsId : undefined}
            aria-label={tracked ? gui.office_hide_revisions : gui.office_revisions} onClick={changeRevisions}>
            <IconDocumentRevisions/><span className="hidden @lg/office:inline">{gui.office_tool_revisions}</span>
          </OfficeTool>}
          {reader.notes && <OfficeTool data-office-notes aria-label={gui.office_notes} aria-pressed={showNotes}
            aria-controls={notesId} onClick={() => setShowNotes(open => !open)}>
            <IconDocumentNotes/><span className="hidden @lg/office:inline">{gui.office_tool_notes}</span>
          </OfficeTool>}
        </div>
      </div>}
    </div>}
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-h-0 flex-1 flex-col @2xl/office:flex-row">
        {pages && page && showThumbnails && <OfficeThumbnails id={thumbnailsId} pages={pages} position={page}
          go={index => run(() => pages.go(index))}/>}
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          <div className="relative flex min-h-0 min-w-0 flex-1">
            <div data-office-document className={cn("relative min-h-0 min-w-0 flex-1 overflow-hidden",
              format === "docx" && !showComments && "[&_[data-overlay=comment-marker]]:invisible")}>
              <div ref={mount} className={format === "pptx" ?
                "absolute inset-0 flex overflow-auto select-text [&>*]:m-auto [&>*]:shrink-0" :
                "absolute inset-0 overflow-hidden select-text"}/>
              {!reader && <div className="absolute inset-0 grid place-items-center bg-background">
                {lock === "unsupported-encryption" ? <RegionState state="error" message={gui.office_encryption_unsupported}/> :
                  lock ? <form data-office-unlock className="grid w-full max-w-sm gap-row p-page" onSubmit={event => {
                    event.preventDefault();
                    const secret = new FormData(event.currentTarget).get("password");
                    if (typeof secret !== "string") return;
                    event.currentTarget.reset();
                    password.current = secret;
                    setAttempt(n => n + 1);
                  }}>
                    <h2 className="text-headline">{gui.office_password_title}</h2>
                    <label className="grid gap-in text-callout">
                      {gui.add_auth_password}
                      <Input name="password" type="password" required autoFocus autoComplete="off" />
                    </label>
                    {lock === "invalid-password" && <span role="alert" className="text-callout text-muted-foreground">{gui.office_password_incorrect}</span>}
                    <Button type="submit" className="justify-self-end">{gui.office_unlock}</Button>
                  </form> : <RegionState state="loading-never"/>}
              </div>}
            </div>
            {wordReview && (showComments || tracked) && <WordReview id={commentsId} host={mount} source={wordReview}
              showComments={showComments} showRevisions={tracked} active={activeComment?.key} jump={selectComment}/>}
            {reader && !wordReview && showComments && <OfficeAnnotations id={commentsId} title={gui.office_tool_comments} beside>
              <OfficeComments threads={reader.comments()} active={activeComment?.key} jump={selectComment}/>
            </OfficeAnnotations>}
          </div>
          {reader && showNotes && <OfficeAnnotations id={notesId} title={gui.office_notes}>
            <p className="py-in select-text whitespace-pre-wrap break-words text-body">{reader.notes?.() || gui.office_no_notes}</p>
          </OfficeAnnotations>}
        </div>
      </div>
    </div>
    {anchor && anchor.page === page?.index && createPortal(
      <div data-office-comment-anchor aria-hidden="true" className="pointer-events-none absolute z-10"
        style={{ left: anchor.x * 100 + "%", top: anchor.y * 100 + "%",
          width: anchor.width * 100 + "%", height: anchor.height * 100 + "%" }}>
        {anchor.width > 0 && anchor.height > 0 && <div className="absolute inset-0 rounded-sm outline-2 outline-ring"/>}
        <span className="grid size-ctl-sm place-items-center rounded-full border bg-background text-foreground">
          <IconDocumentComment className="size-affordance"/>
        </span>
      </div>, anchor.surface,
    )}
    {format === "pptx" && reader && page && pages && <div className="flex flex-none items-center justify-center gap-in border-t px-3 text-callout">
      <Button size="icon" variant="ghost" className="min-h-touch min-w-touch" aria-label={gui.page_back} disabled={page.index <= 0}
        onClick={() => run(() => pages.go(page.index - 1))}>
        <IconBack className="size-affordance"/>
      </Button>
      <span aria-live="polite" className="tabular-nums">{page.index + 1} / {page.count}</span>
      <Button size="icon" variant="ghost" className="min-h-touch min-w-touch" aria-label={gui.page_forward} disabled={page.index + 1 >= page.count}
        onClick={() => run(() => pages.go(page.index + 1))}>
        <IconForward className="size-affordance"/>
      </Button>
    </div>}
    {notice && <div role="status" className="flex flex-none items-center justify-between gap-in border-t px-row text-callout text-muted-foreground">
      <span>{notice}</span>
      <Button size="icon" variant="ghost" aria-label={gui.close} onClick={() => setNotice(null)}><IconClose/></Button>
    </div>}
  </div>;
}
