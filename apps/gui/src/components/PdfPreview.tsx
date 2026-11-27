import { useEffect, useRef, useState, type ReactNode } from "react";
import { AnnotationMode, GlobalWorkerOptions, getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import { EventBus, PDFLinkService, PDFViewer } from "pdfjs-dist/legacy/web/pdf_viewer.mjs";
import workerUrl from "pdfjs-dist/legacy/build/pdf.worker.min.mjs?url";
import "pdfjs-dist/web/pdf_viewer.css";
import { PreviewZoom } from "@/components/PreviewZoom";
import { RegionState } from "@/components/RegionState";
import { useCoarse } from "@/hooks/use-coarse";
import { anchorSurface, bindPreviewPinch, surfaceAt } from "@/lib/preview-pinch";

GlobalWorkerOptions.workerSrc = workerUrl;

export default function PdfPreview({ url, fallback }: { url: string; fallback: ReactNode }) {
  const port = useRef<HTMLDivElement>(null);
  const pages = useRef<HTMLDivElement>(null);
  const viewer = useRef<PDFViewer | null>(null);
  const [ready, setReady] = useState(false);
  const [failed, setFailed] = useState(false);
  const coarse = useCoarse();
  useEffect(() => {
    const container = port.current, pageList = pages.current;
    if (!container || !pageList) return;
    const abort = new AbortController();
    const eventBus = new EventBus();
    const linkService = new PDFLinkService({ eventBus });
    const options = {
      container, viewer: pageList, eventBus, linkService,
      annotationMode: AnnotationMode.DISABLE, abortSignal: abort.signal,
    };
    const pdf = new PDFViewer(options);
    viewer.current = pdf;
    linkService.setViewer(pdf);
    eventBus.on("pagesinit", () => {
      pdf.currentScaleValue = "page-width";
      setReady(true);
    }, { signal: abort.signal });
    const resize = new ResizeObserver(() => {
      if (pdf.currentScaleValue === "page-width") pdf.currentScaleValue = "page-width";
    });
    resize.observe(container);
    // The worker URL includes the asset origin and the immutable Lab version.
    const assets = new URL(import.meta.env.DEV ? "/node_modules/pdfjs-dist/" : "pdfjs/",
      new URL(workerUrl, document.baseURI));
    const task = getDocument({
      url,
      cMapUrl: new URL("cmaps/", assets).href,
      standardFontDataUrl: new URL("standard_fonts/", assets).href,
      wasmUrl: new URL("wasm/", assets).href,
      iccUrl: new URL("iccs/", assets).href,
    });
    void task.promise.then(document => {
      if (abort.signal.aborted) return;
      linkService.setDocument(document);
      pdf.setDocument(document);
    }).catch(() => { if (!abort.signal.aborted) setFailed(true); });
    return () => {
      abort.abort();
      resize.disconnect();
      viewer.current = null;
      pdf.setDocument(null);
      linkService.setDocument(null);
      void task.destroy().catch(() => {});
    };
  }, [url]);
  useEffect(() => {
    const viewport = port.current, pageList = pages.current, pdf = viewer.current;
    if (!coarse || !ready || !viewport || !pageList || !pdf) return;
    return bindPreviewPinch(viewport, origin => {
      const page = surfaceAt(pageList, ".page", origin);
      const follow = anchorSurface(viewport, () => page, origin);
      const initial = pdf.currentScale;
      if (!follow) return;
      return (center, factor) => {
        pdf.updateScale({ scaleFactor: initial * factor / pdf.currentScale, drawingDelay: 90 });
        follow(center);
      };
    });
  }, [coarse, ready]);

  if (failed) return fallback;
  return <div data-pdf-preview data-band-boundary data-ready={ready || undefined}
    className="relative min-h-0 flex-1 overflow-hidden">
    <div ref={port} className="absolute inset-0 overflow-auto touch-pan-x touch-pan-y">
      <div ref={pages} className="pdfViewer"/>
    </div>
    {ready ? <PreviewZoom onZoom={steps => viewer.current?.updateScale({ steps })}/> :
      <div className="absolute inset-0 grid place-items-center"><RegionState state="loading-never"/></div>}
  </div>;
}
