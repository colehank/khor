import { useCallback, useEffect, useEffectEvent, useRef, useState } from "react";
import {
  IconBack, IconChat, IconClose, IconDocumentNotes, IconDocumentRevisions, IconForward,
  IconMinus, IconPageGrid, IconPlus, IconSearch,
} from "@/components/icons";
import { GlassCapsule, GlassCircle, GlassItem } from "@/components/Glass";
import { ContentSearch } from "@/components/ContentSearch";
import { RegionState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { OfficeComments, OfficePanel, OfficeThumbnails } from "@/components/office/OfficePanels";
import type { OfficePages } from "@/components/office/types";
import { loadOfficeReader, type OfficeReader } from "@/components/office/viewer";
import { gui } from "@/gen/catalog";
import { assertPageUrl, openSystemUrl } from "@/lib/client-opener";
import { useCoarse } from "@/hooks/use-coarse";
import { imeKey } from "@/lib/ime";
import { visiblePageOwner } from "@/lib/native-page-yield";
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
type Panel = "pages" | "comments" | "notes";
const LABELED_TOOL = "@md/office:flex @md/office:w-auto @md/office:gap-in @md/office:px-in";

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
  const [slide, setSlide] = useState<OfficePages | null>(null);
  const [panel, setPanel] = useState<Panel | null>(null);
  const [tracked, setTracked] = useState(false);
  const [scale, setScale] = useState(1);
  const [finding, setFinding] = useState(false);
  const [query, setQuery] = useState("");
  const [matches, setMatches] = useState<{ index: number; total: number } | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const home = useLinkHome();
  const coarse = useCoarse();

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
    setSlide(null);
    setPanel(null);
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
        slide: page => { if (active()) setSlide(page); },
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
          event.defaultPrevented || !region || !visiblePageOwner(region) || panel) return;
      const focused = document.activeElement;
      if (focused && focused !== document.body && !region.contains(focused) &&
          focused.closest('input, textarea, [contenteditable="true"], [role="dialog"]')) return;
      event.preventDefault();
      setFinding(true);
      region.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
    };
    document.addEventListener("keydown", keyboard);
    return () => document.removeEventListener("keydown", keyboard);
  }, [reader, panel]);

  const page = reader?.pages;
  const startFind = () => setFinding(true);
  const closePanel = () => setPanel(null);
  return <div ref={root} tabIndex={-1} data-office-preview={format} data-band-boundary data-ready={!!reader || undefined}
    className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden outline-none">
    {reader && <div data-office-toolbar className="@container/office flex h-pane-head min-w-0 flex-none items-center border-b bg-background">
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
        className="flex h-full min-w-0 flex-1 items-center gap-in overflow-x-auto whitespace-nowrap px-row [scrollbar-width:none]">
        <GlassCircle data-office-find-open aria-label={gui.search_file} tip={!coarse && gui.search_file}
          onClick={startFind}><IconSearch/></GlassCircle>
        <GlassCapsule role="group" aria-label={gui.office_tool_zoom}>
          <GlassItem data-office-zoom-out aria-label={gui.office_zoom_out} tip={!coarse && gui.office_zoom_out}
            onClick={() => zoom("zoomOut")}><IconMinus/></GlassItem>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <GlassItem data-office-zoom aria-label={gui.office_tool_zoom + " " + Math.round(scale * 100) + "%"}
                tip={!coarse && gui.office_tool_zoom} className="w-auto min-w-glass-item-w px-in text-callout tabular-nums">
                {Math.round(scale * 100)}%
              </GlassItem>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="center">
              <DropdownMenuItem data-office-fit-width onSelect={() => zoom("fitWidth")}>{gui.office_fit_width}</DropdownMenuItem>
              {page && <DropdownMenuItem data-office-fit-page onSelect={() => zoom("fitPage")}>{gui.office_fit_page}</DropdownMenuItem>}
              <DropdownMenuItem data-office-actual-size onSelect={() => zoom("actual")}>{gui.office_actual_size}</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <GlassItem data-office-zoom-in aria-label={gui.office_zoom_in} tip={!coarse && gui.office_zoom_in}
            onClick={() => zoom("zoomIn")}><IconPlus/></GlassItem>
        </GlassCapsule>
        <GlassCapsule>
          {page && <GlassItem data-office-pages aria-label={gui.office_thumbnails} tip={!coarse && gui.office_thumbnails}
            className={LABELED_TOOL} onClick={() => setPanel("pages")}>
            <IconPageGrid/><span className="hidden text-callout @md/office:inline">{gui.office_tool_pages}</span>
          </GlassItem>}
          <GlassItem data-office-review aria-label={gui.office_comments} tip={!coarse && gui.office_comments}
            className={LABELED_TOOL} onClick={() => setPanel("comments")}>
            <IconChat/><span className="hidden text-callout @md/office:inline">{gui.office_tool_comments}</span>
          </GlassItem>
          {reader.revisions && <GlassItem data-office-revisions aria-pressed={tracked} on={tracked}
            aria-label={tracked ? gui.office_hide_revisions : gui.office_revisions}
            tip={!coarse && (tracked ? gui.office_hide_revisions : gui.office_revisions)}
            className={LABELED_TOOL} onClick={changeRevisions}>
            <IconDocumentRevisions/><span className="hidden text-callout @md/office:inline">{gui.office_tool_revisions}</span>
          </GlassItem>}
          {reader.notes && <GlassItem data-office-notes aria-label={gui.office_notes} tip={!coarse && gui.office_notes}
            className={LABELED_TOOL} onClick={() => setPanel("notes")}>
            <IconDocumentNotes/><span className="hidden text-callout @md/office:inline">{gui.office_tool_notes}</span>
          </GlassItem>}
        </GlassCapsule>
      </div>}
    </div>}
    <div data-office-document className="relative min-h-0 flex-1 overflow-hidden">
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
    {reader && slide && page && <div className="flex flex-none items-center justify-center gap-in border-t px-3 text-callout">
      <Button size="icon" variant="ghost" className="min-h-touch min-w-touch" aria-label={gui.page_back} disabled={slide.index <= 0}
        onClick={() => run(() => page.go(slide.index - 1))}>
        <IconBack className="size-affordance"/>
      </Button>
      <span aria-live="polite" className="tabular-nums">{slide.index + 1} / {slide.count}</span>
      <Button size="icon" variant="ghost" className="min-h-touch min-w-touch" aria-label={gui.page_forward} disabled={slide.index + 1 >= slide.count}
        onClick={() => run(() => page.go(slide.index + 1))}>
        <IconForward className="size-affordance"/>
      </Button>
    </div>}
    {notice && <div role="status" className="flex flex-none items-center justify-between gap-in border-t px-row text-callout text-muted-foreground">
      <span>{notice}</span>
      <Button size="icon" variant="ghost" aria-label={gui.close} onClick={() => setNotice(null)}><IconClose/></Button>
    </div>}
    {reader && panel === "pages" && page && <OfficeThumbnails pages={page} close={closePanel}
      go={index => { closePanel(); run(() => page.go(index)); }}/>}
    {reader && panel === "comments" && <OfficeComments threads={reader.comments()} close={closePanel}
      jump={go => { closePanel(); run(go); }}/>}
    {reader && panel === "notes" && <OfficePanel title={gui.office_notes} close={closePanel}>
      <p className="select-text whitespace-pre-wrap break-words text-body">{reader.notes?.() || gui.office_no_notes}</p>
    </OfficePanel>}
  </div>;
}
