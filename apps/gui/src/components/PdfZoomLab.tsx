import { useEffect, useRef, useState, type ReactNode } from "react";
import { AnnotationMode, GlobalWorkerOptions, getDocument } from "pdfjs-dist";
import { EventBus, PDFLinkService, PDFViewer } from "pdfjs-dist/web/pdf_viewer.mjs";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import "pdfjs-dist/web/pdf_viewer.css";
import { PreviewZoom } from "@/components/PreviewZoom";
import { RegionState } from "@/components/RegionState";

GlobalWorkerOptions.workerSrc = workerUrl;

/** Lab renderer for trying the shared zoom controls on real, selectable PDF pages. */
export default function PdfZoomLab({ url, fallback }: { url: string; fallback: ReactNode }) {
  const port = useRef<HTMLDivElement>(null);
  const pages = useRef<HTMLDivElement>(null);
  const viewer = useRef<PDFViewer | null>(null);
  const [ready, setReady] = useState(false);
  const [failed, setFailed] = useState(false);
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
    const task = getDocument({ url });
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

  if (failed) return fallback;
  return <div data-pdf-zoom-lab data-band-boundary data-ready={ready || undefined}
    className="relative min-h-0 flex-1 overflow-hidden">
    <div ref={port} className="absolute inset-0 overflow-auto">
      <div ref={pages} className="pdfViewer"/>
    </div>
    {ready ? <PreviewZoom onZoom={steps => viewer.current?.updateScale({ steps })}/> :
      <div className="absolute inset-0 grid place-items-center"><RegionState state="loading-never"/></div>}
  </div>;
}
