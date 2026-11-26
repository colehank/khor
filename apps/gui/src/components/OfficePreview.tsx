import { useEffect, useEffectEvent, useRef, useState } from "react";
import { IconBack, IconForward } from "@/components/icons";
import { RegionState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
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

type Viewer = { load(bytes: ArrayBuffer): Promise<void>; destroy(): void };
type Slide = { index: number; count: number };

async function documentBytes(source: string | File): Promise<ArrayBuffer> {
  if (typeof source !== "string") return source.arrayBuffer();
  const raw = atob(source);
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < bytes.length; i++) bytes[i] = raw.charCodeAt(i);
  return bytes.buffer;
}

export function OfficePreview({ format, source, onFallback }: {
  format: OfficeFormat;
  source: string | File;
  onFallback: (reason: OfficeFailure) => void;
}) {
  const mount = useRef<HTMLDivElement>(null);
  const navigate = useRef<((index: number) => Promise<void>) | null>(null);
  const [ready, setReady] = useState(false);
  const [slide, setSlide] = useState<Slide | null>(null);
  const reportFailure = useEffectEvent(onFallback);

  useEffect(() => {
    const host = mount.current;
    if (!host) return;
    const abort = new AbortController();
    let viewer: Viewer | undefined;
    let parserUrl: string | undefined;
    let resize: ResizeObserver | undefined;
    const active = () => !abort.signal.aborted;
    const fallback = (error?: unknown) => {
      if (!active()) return;
      const code = error && typeof error === "object" && "code" in error ? error.code : null;
      reportFailure(code === "ooxml-resource-limit" || code === "ooxml-decoded-image-limit" ? "too-large" : "unavailable");
    };
    const timeout = setTimeout(fallback, 20_000);
    setReady(false);
    setSlide(null);

    async function load(host: HTMLDivElement) {
      // Blob URLs let worker fetches use the same bundled bytes in HTTP
      // browsers and in Tauri's platform-specific asset schemes.
      const response = await fetch(PARSERS[format], { signal: abort.signal });
      if (!response.ok) throw new Error("Office parser unavailable");
      const parserBytes = await response.arrayBuffer();
      const bytes = await documentBytes(source);
      if (!active()) return;
      parserUrl = URL.createObjectURL(new Blob([parserBytes], { type: "application/wasm" }));
      const options = {
        wasmUrl: parserUrl,
        mode: "worker" as const,
        enableHyperlinks: false,
        resourceLimits: { maxArchiveEntryBytes: 32 * 1024 * 1024, maxTotalInflatedBytes: 96 * 1024 * 1024 },
        onError: fallback,
      };
      if (format === "docx") {
        const { DocxScrollViewer } = await import("@silurus/ooxml/docx");
        if (!active()) return;
        viewer = new DocxScrollViewer(host, {
          ...options, enableTextSelection: true, refitOnResize: true, background: "transparent",
          comments: false,
        });
      } else if (format === "xlsx") {
        const { XlsxViewer } = await import("@silurus/ooxml/xlsx");
        if (!active()) return;
        viewer = new XlsxViewer(host, { ...options, showZoomSlider: false, comments: false });
      } else {
        const { PptxViewer } = await import("@silurus/ooxml/pptx");
        if (!active()) return;
        const canvas = document.createElement("canvas");
        host.append(canvas);
        const slides = new PptxViewer(canvas, {
          ...options, enableTextSelection: true, enableMediaPlayback: false,
          width: host.clientWidth,
          onSlideChange: (index, count) => { if (active()) setSlide({ index, count }); },
        });
        viewer = slides;
        navigate.current = index => slides.goToSlide(index);
        await slides.load(bytes);
        if (!active()) return;
        await slides.fitPage();
        if (!active()) return;
        resize = new ResizeObserver(() => { if (active()) void slides.fitPage().catch(fallback); });
        resize.observe(host);
        return;
      }
      await viewer.load(bytes);
    }

    void load(host).then(() => {
      clearTimeout(timeout);
      if (active()) setReady(true);
    }, fallback);
    return () => {
      abort.abort();
      clearTimeout(timeout);
      resize?.disconnect();
      navigate.current = null;
      viewer?.destroy();
      host.replaceChildren();
      if (parserUrl) URL.revokeObjectURL(parserUrl);
    };
  }, [format, source]);

  return <div data-office-preview={format} data-ready={ready || undefined}
    className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
    <div className="relative min-h-0 flex-1 overflow-hidden">
      <div ref={mount} className={format === "pptx" ?
        "absolute inset-0 grid place-items-center overflow-hidden select-text" :
        "absolute inset-0 overflow-hidden select-text"}/>
      {!ready && <div className="absolute inset-0 bg-background"><RegionState state="loading-never"/></div>}
    </div>
    {ready && slide && <div className="flex flex-none items-center justify-center gap-in border-t px-3 text-callout">
      <Button size="icon" variant="ghost" className="min-h-touch min-w-touch" aria-label={gui.page_back} disabled={slide.index <= 0}
        onClick={() => void navigate.current?.(slide.index - 1).catch(() => onFallback("unavailable"))}>
        <IconBack className="size-affordance"/>
      </Button>
      <span aria-live="polite" className="tabular-nums">{slide.index + 1} / {slide.count}</span>
      <Button size="icon" variant="ghost" className="min-h-touch min-w-touch" aria-label={gui.page_forward} disabled={slide.index + 1 >= slide.count}
        onClick={() => void navigate.current?.(slide.index + 1).catch(() => onFallback("unavailable"))}>
        <IconForward className="size-affordance"/>
      </Button>
    </div>}
  </div>;
}
