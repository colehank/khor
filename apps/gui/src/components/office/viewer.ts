import type { OfficeFormat } from "@/components/OfficePreview";
import type { OfficeNavigation, OfficePages, OfficeThread } from "@/components/office/types";
import { gui } from "@/gen/catalog";
import type { DefinedName } from "@silurus/ooxml/xlsx";

export type OfficeViewer = {
  destroy(): void;
  findText(query: string): Promise<readonly { matchIndex: number }[]>;
  findNext(): Promise<{ matchIndex: number } | null>;
  findPrev(): Promise<{ matchIndex: number } | null>;
  clearFind(): void;
  zoomIn(): void | Promise<void>;
  zoomOut(): void | Promise<void>;
  fitWidth(): void | Promise<void>;
  fitPage(): void | Promise<void>;
  setScale(scale: number): void | Promise<void>;
};

export type OfficeReader = {
  viewer: OfficeViewer;
  pages?: OfficeNavigation;
  comments: () => OfficeThread[];
  revisions?: (show: boolean) => Promise<void>;
  notes?: () => string | null;
  copy?: () => Promise<string>;
  dispose: () => void;
};

type Callbacks = {
  signal: AbortSignal;
  error: (error: unknown) => void;
  externalLink: (url: string) => void;
  slide: (page: OfficePages) => void;
};

const renderers = () => Promise.all([
  import("@silurus/ooxml/math").then(m => m.math),
  import("@silurus/ooxml/chart-ex").then(m => m.chartEx),
  import("@silurus/ooxml/three-d").then(m => m.threeD),
  import("@silurus/ooxml/region-map").then(m => m.regionMap),
  import("@silurus/ooxml/tiff").then(m => m.tiff),
]);

const THUMBNAIL_WIDTH = 200;
type LinkTarget = { kind: "external"; url: string } | { kind: "internal"; ref: string; slideIndex?: number };

function sheetLink(ref: string, sheet: number, names: string[], defined: readonly DefinedName[]) {
  const lookup = new Map(defined.map(name => [name.name.toLowerCase(), name.formula]));
  const visited = new Set<string>();
  while (true) {
    const target = ref.trim().replace(/^[=#]+/, "");
    const cell = /^(?:(?:'((?:[^']|'')+)'|([^!]+))!)?(\$?[A-Z]{1,3}\$?\d+)(?::\$?[A-Z]{1,3}\$?\d+)?$/i.exec(target);
    if (cell) {
      const name = (cell[1]?.replaceAll("''", "'") ?? cell[2])?.toLowerCase();
      const index = name ? names.findIndex(candidate => candidate.toLowerCase() === name) : sheet;
      return index < 0 ? null : { index, ref: cell[3].replaceAll("$", "") };
    }
    const key = target.toLowerCase();
    const next = lookup.get(key);
    if (!next || visited.has(key)) return null;
    visited.add(key);
    ref = next;
  }
}

export async function loadOfficeReader(
  format: OfficeFormat, host: HTMLElement, bytes: ArrayBuffer, wasmUrl: string, password: string | undefined,
  callbacks: Callbacks,
): Promise<OfficeReader> {
  const { signal, error, slide, externalLink } = callbacks;
  const [math, chartEx, threeD, regionMap, tiff] = await renderers();
  signal.throwIfAborted();
  const options = {
    wasmUrl, password, mode: "worker" as const, math, chartEx, threeD, regionMap, tiff,
    workerTimeoutMs: 20_000,
    resourceLimits: { maxArchiveEntryBytes: 32 * 1024 * 1024, maxTotalInflatedBytes: 96 * 1024 * 1024 },
  };
  const actions = {
    enableHyperlinks: true,
    onError: error,
  };
  // A viewer borrows the parsed model. Attach both lifetimes before awaiting
  // any work so a closed tab cannot keep its workers, canvas or document alive.
  let viewer: OfficeViewer | undefined;
  let closeModel: (() => void) | undefined;
  const dispose = () => {
    signal.removeEventListener("abort", dispose);
    viewer?.destroy();
    viewer = undefined;
    closeModel?.();
    closeModel = undefined;
  };
  signal.addEventListener("abort", dispose, { once: true });
  try {
    if (format === "docx") {
      const { DocxDocument, DocxScrollViewer } = await import("@silurus/ooxml/docx");
      signal.throwIfAborted();
      const document = await DocxDocument.load(bytes, options);
      closeModel = () => document.destroy();
      signal.throwIfAborted();
      const word = DocxScrollViewer.fromDocument(host, document, {
        ...actions, enableTextSelection: true, refitOnResize: true, background: "transparent",
        onHyperlinkClick: (target: LinkTarget) => {
          if (target.kind === "external") externalLink(target.url);
          else {
            const index = document.getBookmarkPage(target.ref);
            if (index !== undefined) word.scrollToPage(index);
          }
        },
      });
      viewer = word;
      return {
        viewer, dispose,
        pages: {
          page: () => ({ index: word.topVisiblePage, count: word.pageCount }),
          go: index => word.scrollToPage(index),
          thumbnail: index => document.renderPageToBitmap(index, { width: THUMBNAIL_WIDTH, dpr: 1 }),
        },
        comments: () => {
          const replies = new Map<string, OfficeThread[]>();
          for (const comment of document.comments) {
            if (!comment.parentId) continue;
            const group = replies.get(comment.parentId) ?? [];
            group.push({ ...comment, key: comment.id });
            replies.set(comment.parentId, group);
          }
          return document.comments.filter(comment => !comment.parentId).map(comment => ({
            ...comment, key: comment.id, replies: replies.get(comment.id),
            jump: () => word.goToComment(comment.id),
          }));
        },
        revisions: show => word.setShowTrackedChanges(show),
      };
    }
    if (format === "xlsx") {
      const { XlsxViewer, XlsxWorkbook } = await import("@silurus/ooxml/xlsx");
      signal.throwIfAborted();
      const workbook = await XlsxWorkbook.load(bytes, options);
      closeModel = () => workbook.destroy();
      signal.throwIfAborted();
      const followCell = async (ref: string) => {
        const index = sheets.sheetIndex;
        const sheet = await workbook.getWorksheet(index);
        const target = sheetLink(ref, index, sheets.sheetNames, sheet.definedNames ?? []);
        if (!target || signal.aborted || index !== sheets.sheetIndex) return;
        if (target.index !== index) await sheets.goToSheet(target.index);
        if (!signal.aborted) await sheets.scrollToCell(target.ref);
      };
      const sheets = XlsxViewer.fromWorkbook(host, workbook, {
        ...actions, showZoomSlider: false, comments: true,
        onHyperlinkClick: (target: LinkTarget) => {
          if (target.kind === "external") externalLink(target.url);
          else void followCell(target.ref).catch(error);
        },
      });
      viewer = sheets;
      return {
        viewer: sheets, dispose,
        comments: () => {
          const sheet = sheets.sheetIndex;
          return sheets.getComments().map((comment, i) => ({
            ...comment, key: comment.id ?? String(i), location: comment.cellRef,
            jump: () => sheets.goToComment(sheet, comment.cellRef),
          }));
        },
        copy: async () => {
          const { status } = await sheets.copySelection();
          switch (status) {
            case "copied": return gui.copied;
            case "empty-selection": return gui.office_copy_select;
            case "too-large": return gui.office_copy_limit;
            case "unsupported-multiple-areas": return gui.office_copy_contiguous;
            default: return gui.copy_failed;
          }
        },
      };
    }
    const { PptxPresentation, PptxViewer } = await import("@silurus/ooxml/pptx");
    signal.throwIfAborted();
    const presentation = await PptxPresentation.load(bytes, options);
    closeModel = () => presentation.destroy();
    signal.throwIfAborted();
    const canvas = document.createElement("canvas");
    canvas.className = "m-auto shrink-0";
    host.append(canvas);
    const slides = PptxViewer.fromPresentation(canvas, presentation, {
      ...actions, enableTextSelection: true, enableMediaPlayback: true, width: host.clientWidth,
      onSlideChange: (index, count) => slide({ index, count }),
      onHyperlinkClick: (target: LinkTarget) => {
        if (target.kind === "external") externalLink(target.url);
        else if (target.slideIndex !== undefined) void slides.goToSlide(target.slideIndex).catch(error);
      },
    });
    viewer = slides;
    await slides.fitPage();
    signal.throwIfAborted();
    return {
      viewer: slides, dispose,
      pages: {
        page: () => ({ index: slides.slideIndex, count: slides.slideCount }),
        go: index => slides.goToSlide(index),
        thumbnail: index => presentation.renderSlideToBitmap(index, { width: THUMBNAIL_WIDTH, dpr: 1 }),
      },
      comments: () => presentation.getComments(slides.slideIndex).map((comment, i) => ({
        ...comment, key: comment.id ?? String(i),
      })),
      notes: () => presentation.getNotes(slides.slideIndex),
    };
  } catch (error) {
    dispose();
    throw error;
  }
}
