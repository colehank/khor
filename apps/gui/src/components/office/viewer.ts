import type { OfficeFormat } from "@/components/OfficePreview";
import type { OfficeCommentAnchor, OfficeNavigation, OfficePages, OfficeThread, WordReviewEntry, WordReviewPoint, WordReviewSource } from "@/components/office/types";
import { gui } from "@/gen/catalog";
import { visiblePageOwner } from "@/lib/native-page-yield";
import { anchorSurface, surfaceAt, type BeginPreviewPinch, type PreviewPoint } from "@/lib/preview-pinch";
import type { PptxComment, PptxPresentation } from "@silurus/ooxml/pptx";
import type { DefinedName } from "@silurus/ooxml/xlsx";

export type OfficeViewer = {
  destroy(): void;
  findText(query: string): Promise<readonly { matchIndex: number }[]>;
  findNext(): Promise<{ matchIndex: number } | null>;
  findPrev(): Promise<{ matchIndex: number } | null>;
  clearFind(): void;
  zoomIn(): void | Promise<void>;
  zoomOut(): void | Promise<void>;
  fitPage(): void | Promise<void>;
  setScale(scale: number): void | Promise<void>;
  getScale(): number;
};

export type OfficeReader = {
  viewer: OfficeViewer;
  pinch: BeginPreviewPinch;
  pages?: OfficeNavigation;
  comments: () => OfficeThread[];
  wordReview?: WordReviewSource;
  revisions?: (show: boolean) => Promise<void>;
  notes?: () => string | null;
  dispose: () => void;
};

type Callbacks = {
  signal: AbortSignal;
  error: (error: unknown) => void;
  externalLink: (url: string) => void;
  position: (page: OfficePages) => void;
};

function pinchSurface(viewer: OfficeViewer, viewport: HTMLElement,
  surface: () => HTMLElement | undefined, origin: PreviewPoint, signal: AbortSignal) {
  const follow = anchorSurface(viewport, surface, origin);
  const initial = viewer.getScale();
  if (!follow) return;
  return async (center: PreviewPoint, factor: number) => {
    const scale = initial * factor;
    if (Math.abs(scale - viewer.getScale()) > 0.001) await viewer.setScale(scale);
    if (!signal.aborted) follow(center);
  };
}

const renderers = () => Promise.all([
  import("@silurus/ooxml/math").then(m => m.math),
  import("@silurus/ooxml/chart-ex").then(m => m.chartEx),
  import("@silurus/ooxml/three-d").then(m => m.threeD),
  import("@silurus/ooxml/region-map").then(m => m.regionMap),
  import("@silurus/ooxml/tiff").then(m => m.tiff),
]);

const THUMBNAIL_WIDTH = 200;
export const WORD_PAGE_SURFACE = '[data-ooxml-selection-surface="docx"][data-page-index]';
// Silurus 0.90.2 assigns these inks to tracked-change authors in first-use
// order. Its worker paints the canvas and accepts no colour override.
const WORD_REVISION_INKS = ["#c00000", "#0070c0", "#00b050", "#7030a0", "#e97132", "#196b24", "#9e480e", "#525252"];
type LinkTarget = { kind: "external"; url: string } | { kind: "internal"; ref: string; slideIndex?: number };

function visibleWordPage(host: HTMLElement, preferred: number): number {
  const viewport = host.getBoundingClientRect();
  let index = preferred;
  let visibleHeight = 0;
  // The nearest page to the top can be the previous page when scrolling
  // stops at the end. Use the page that occupies most of the viewport.
  for (const page of host.querySelectorAll<HTMLElement>(WORD_PAGE_SURFACE)) {
    const bounds = page.getBoundingClientRect();
    const height = Math.min(bounds.bottom, viewport.bottom) - Math.max(bounds.top, viewport.top);
    const pageIndex = Number(page.dataset.pageIndex);
    if (height > visibleHeight || (height > 0 && height === visibleHeight && pageIndex === preferred)) {
      index = pageIndex;
      visibleHeight = height;
    }
  }
  return index;
}

function revealWordRevision(host: HTMLElement, index: number, points: WordReviewPoint[]): boolean {
  const page = host.querySelector<HTMLElement>(`${WORD_PAGE_SURFACE}[data-page-index="${index}"]`);
  // Each selection layer sits in a positioned page, directly inside the scroll host.
  const scrollHost = page?.parentElement?.parentElement;
  if (!page || !scrollHost) return false;
  const bounds = page.getBoundingClientRect();
  const viewport = scrollHost.getBoundingClientRect();
  const point = points.find(({ y }) => {
    const top = bounds.top + y * bounds.height;
    return top >= viewport.top && top <= viewport.bottom;
  }) ?? points[0];
  if (!point) return false;
  scrollHost.scrollBy({
    top: bounds.top + point.y * bounds.height - viewport.top - viewport.height / 2,
    left: bounds.left + point.x * bounds.width - viewport.left - viewport.width / 2,
    behavior: "instant",
  });
  return true;
}

async function slideCommentAnchor(
  deck: PptxPresentation, comment: Readonly<PptxComment>, page: number, surface: HTMLElement,
): Promise<OfficeCommentAnchor | null> {
  const anchors = comment.anchors ?? [];
  const ids = anchors.flatMap(anchor => "elementId" in anchor && anchor.elementId ? [anchor.elementId] : []);
  const hit = ids.length ? (await deck.getElementBoundsByIds(page, ids))[0]?.bounds : undefined;
  if (hit) return {
    surface, page, x: hit.x / deck.slideWidth, y: hit.y / deck.slideHeight,
    width: hit.width / deck.slideWidth, height: hit.height / deck.slideHeight,
  };
  if ((anchors.length === 0 || anchors.some(anchor => anchor.type === "slide")) &&
      typeof comment.x === "number" && Number.isFinite(comment.x) &&
      typeof comment.y === "number" && Number.isFinite(comment.y)) {
    return { surface, page, x: comment.x / deck.slideWidth, y: comment.y / deck.slideHeight, width: 0, height: 0 };
  }
  return null;
}

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
  const { signal, error, position, externalLink } = callbacks;
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
      const { DocxDocument, DocxScrollViewer, resolveDocxCommentThreads, resolveRevisionAnchorRuns } =
        await import("@silurus/ooxml/docx");
      signal.throwIfAborted();
      const document = await DocxDocument.load(bytes, options);
      closeModel = () => document.destroy();
      signal.throwIfAborted();
      let current: OfficePages | undefined;
      const publishPage = (top: number, count: number) => {
        const index = visibleWordPage(host, top);
        if (current?.index === index && current.count === count) return;
        current = { index, count };
        position(current);
      };
      const word = DocxScrollViewer.fromDocument(host, document, {
        ...actions, enableTextSelection: true, refitOnResize: true, background: "transparent",
        onVisiblePageChange: publishPage,
        comments: { cards: false, markers: false, includeResolved: true },
        onHyperlinkClick: (target: LinkTarget) => {
          if (target.kind === "external") externalLink(target.url);
          else {
            const index = document.getBookmarkPage(target.ref);
            if (index !== undefined) word.scrollToPage(index);
          }
        },
      });
      viewer = word;
      let frame = 0;
      const scroll = () => {
        if (frame) return;
        frame = requestAnimationFrame(() => {
          frame = 0;
          publishPage(word.topVisiblePage, word.pageCount);
        });
      };
      host.addEventListener("scroll", scroll, { capture: true, passive: true });
      closeModel = () => {
        host.removeEventListener("scroll", scroll, true);
        cancelAnimationFrame(frame);
        document.destroy();
      };
      const comments = () => {
        const replies = new Map<string, OfficeThread[]>();
        for (const comment of document.comments) {
          if (!comment.parentId) continue;
          const group = replies.get(comment.parentId) ?? [];
          group.push({ ...comment, key: comment.id });
          replies.set(comment.parentId, group);
        }
        return document.comments.filter(comment => !comment.parentId).map(comment => ({
          ...comment, key: comment.id, replies: replies.get(comment.id),
        }));
      };
      const revisionAuthors = [...new Set(document.revisions.map(({ author }) => author ?? ""))];
      const authorInks = new Map(revisionAuthors.map((author, index) =>
        [author, WORD_REVISION_INKS[index % WORD_REVISION_INKS.length]],
      ));
      let commentAuthors = 0;
      for (const { author } of document.comments) {
        if (!author || authorInks.has(author)) continue;
        authorInks.set(author, `oklch(57% 0.14 ${(51 + 137.508 * commentAuthors++) % 360})`);
      }
      const wordReview: WordReviewSource = {
        authorInk: author => authorInks.get(author ?? "") ?? "oklch(57% 0 0)",
        async page(index, showComments, showRevisions) {
          const { widthPt, heightPt } = document.pageSize(index);
          const width = widthPt * 4 / 3;
          const height = heightPt * 4 / 3;
          const runs = await document.collectPageRuns(index, { width, showTrackedChanges: showRevisions });
          const point = (
            rect: { x: number; y: number; width: number; height: number; transform?: string }, underline = false,
          ) => {
            let x = rect.x + rect.width;
            let y = rect.y + rect.height * (underline ? 1 : 0.5);
            if (rect.transform) {
              const centerX = rect.x + rect.width / 2;
              const centerY = rect.y + rect.height / 2;
              const offset = new DOMMatrix(rect.transform).transformPoint({ x: x - centerX, y: y - centerY });
              x = centerX + offset.x;
              y = centerY + offset.y;
            }
            return {
              x: x / width, y: y / height, lineTop: rect.y / height,
              lineHeight: rect.transform ? 0 : rect.height / height,
            };
          };
          const entries: WordReviewEntry[] = [];
          if (showComments) {
            for (const { root, replies, anchors } of resolveDocxCommentThreads(
              document.comments, document.commentAnchorRanges(), runs, { includeResolved: true },
            )) {
              const ranges = anchors.flatMap(anchor => anchor.rects.map(rect => {
                // Silurus positions the rectangle first and rotates it around
                // its top-left corner, the same origin as the text layer.
                const matrix = rect.transform ? new DOMMatrix(rect.transform) : undefined;
                return [[0, 0], [rect.width, 0], [rect.width, rect.height], [0, rect.height]].map(([x, y]) => {
                  const corner = matrix?.transformPoint({ x, y }) ?? { x, y };
                  return { x: (rect.x + corner.x) / width, y: (rect.y + corner.y) / height };
                });
              }));
              const points = ranges.map(([, topRight, bottomRight]) => ({
                ...topRight, lineTop: topRight.y,
                lineHeight: topRight.x === bottomRight.x ? Math.max(0, bottomRight.y - topRight.y) : 0,
              }));
              if (points.length) entries.push({
                ...root, key: "comment-" + root.id, replies, points, ranges,
                jump: () => word.goToComment(root.id, { pageIndex: index }),
              });
            }
          }
          if (showRevisions) {
            const revisions = document.revisions;
            const pointsByRevision = new Map<number, WordReviewEntry["points"]>();
            for (const anchor of document.revisionAnchorRanges()) {
              const kind = revisions[anchor.revisionIndex]?.kind;
              const points = resolveRevisionAnchorRuns(anchor, runs).map(run => point({
                ...(run.highlightBounds ?? { x: run.x, y: run.y, width: run.w, height: run.h }), transform: run.transform,
              }, kind === "insertion" || kind === "moveTo"));
              if (points.length) pointsByRevision.set(anchor.revisionIndex, [
                ...(pointsByRevision.get(anchor.revisionIndex) ?? []), ...points,
              ]);
            }
            for (const [i, points] of pointsByRevision) {
              const revision = revisions[i];
              if (revision) entries.push({
                ...revision, key: "revision-" + i, revision: revision.kind, points,
                jump: async () => revealWordRevision(host, index, points),
              });
            }
          }
          return entries;
        },
        unanchored(showComments, showRevisions) {
          const entries: WordReviewEntry[] = [];
          if (showComments) {
            const ids = new Set(document.commentAnchorRanges().map(range => range.commentId));
            for (const comment of comments()) {
              if (!ids.has(comment.key)) entries.push({ ...comment, key: "comment-" + comment.key, points: [] });
            }
          }
          if (showRevisions) {
            const ids = new Set(document.revisionAnchorRanges().map(range => range.revisionIndex));
            document.revisions.forEach((revision, index) => {
              if (!ids.has(index)) entries.push({ ...revision, key: "revision-" + index, revision: revision.kind, points: [] });
            });
          }
          return entries;
        },
      };
      return {
        viewer, dispose,
        pinch: (origin, gesture) => {
          const page = surfaceAt(host, WORD_PAGE_SURFACE, origin);
          const viewport = page?.parentElement?.parentElement;
          if (!page || !viewport) return;
          const selector = `${WORD_PAGE_SURFACE}[data-page-index="${page.dataset.pageIndex}"]`;
          return pinchSurface(word, viewport, () => host.querySelector<HTMLElement>(selector) ?? undefined,
            origin, gesture);
        },
        pages: {
          page: () => current ?? { index: word.topVisiblePage, count: word.pageCount },
          go: index => {
            word.scrollToPage(index);
            publishPage(index, word.pageCount);
          },
          thumbnail: index => document.renderPageToBitmap(index, { width: THUMBNAIL_WIDTH, dpr: 1 }),
        },
        comments, wordReview,
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
      let syncSheetArrows: ((index: number, count: number) => void) | undefined;
      const sheets = XlsxViewer.fromWorkbook(host, workbook, {
        ...actions, showZoomSlider: false, comments: false,
        onSheetChange: (index, count) => {
          syncSheetArrows?.(index, count);
          position({ index, count });
        },
        onHyperlinkClick: (target: LinkTarget) => {
          if (target.kind === "external") externalLink(target.url);
          else void followCell(target.ref).catch(error);
        },
      });
      viewer = sheets;
      // Keep the vendor tabs and their sheet list. Replace the primary
      // action of the two tab-strip arrows with worksheet navigation.
      const arrows = [...host.querySelectorAll<HTMLButtonElement>("[data-xlsx-tab-nav]")].map(button => {
        const previous = button.dataset.xlsxTabNav === "prev";
        const label = previous ? gui.office_sheet_previous : gui.office_sheet_next;
        button.title = label;
        button.setAttribute("aria-label", label);
        button.addEventListener("click", event => {
          event.stopImmediatePropagation();
          if (button.disabled || signal.aborted) return;
          void (previous ? sheets.prevSheet() : sheets.nextSheet()).catch(error);
        }, { capture: true, signal });
        return { button, previous };
      });
      syncSheetArrows = (index, count) => {
        for (const { button, previous } of arrows) button.disabled = previous ? index <= 0 : index + 1 >= count;
      };
      syncSheetArrows(sheets.sheetIndex, sheets.sheetCount);
      const document = host.ownerDocument;
      const nativeCopy = (event: ClipboardEvent) => {
        const focus = document.activeElement;
        if (event.defaultPrevented || !host.contains(focus) || !visiblePageOwner(host) ||
            !sheets.selectionState || document.getSelection()?.toString()) return;
        if (focus instanceof HTMLElement && (focus.isContentEditable || focus.closest("input, textarea, select"))) return;
        // The viewer already owns the keyboard shortcut. The copy event is
        // also sent by the system Edit menu and the browser's copy action.
        event.preventDefault();
        void sheets.copySelection().catch(() => {});
      };
      document.addEventListener("copy", nativeCopy);
      closeModel = () => {
        document.removeEventListener("copy", nativeCopy);
        workbook.destroy();
      };
      return {
        viewer: sheets, dispose,
        pinch: (origin, gesture) => {
          const viewport = host.querySelector<HTMLElement>("[data-xlsx-viewport-input]");
          const cell = sheets.getCellAt(origin.x, origin.y);
          const bounds = cell && sheets.getCellViewportRect(cell);
          if (!viewport || !cell || !bounds?.width || !bounds.height) return;
          const canvas = sheets.canvasElement.getBoundingClientRect();
          const x = (origin.x - canvas.left - bounds.x) / bounds.width;
          const y = (origin.y - canvas.top - bounds.y) / bounds.height;
          const initial = sheets.getScale(), sheet = sheets.sheetIndex;
          let previous = origin;
          return (center, factor) => {
            if (gesture.aborted || sheet !== sheets.sheetIndex) return;
            const oldScale = sheets.getScale();
            sheets.setScale(initial * factor);
            let left = previous.x - center.x, top = previous.y - center.y;
            if (oldScale !== sheets.getScale()) {
              const rect = sheets.getCellViewportRect(cell);
              const canvas = sheets.canvasElement.getBoundingClientRect();
              if (rect) {
                left = canvas.left + rect.x + x * rect.width - center.x;
                top = canvas.top + rect.y + y * rect.height - center.y;
              }
            }
            viewport.scrollBy({ left, top, behavior: "instant" });
            previous = center;
          };
        },
        comments: () => {
          const sheet = sheets.sheetIndex;
          return sheets.getComments().map((comment, i) => ({
            ...comment, key: comment.id ?? String(i), location: comment.cellRef,
            jump: () => sheets.goToComment(sheet, comment.cellRef),
          }));
        },
      };
    }
    const { PptxPresentation, PptxViewer } = await import("@silurus/ooxml/pptx");
    signal.throwIfAborted();
    const presentation = await PptxPresentation.load(bytes, options);
    closeModel = () => presentation.destroy();
    signal.throwIfAborted();
    const canvas = document.createElement("canvas");
    host.append(canvas);
    const slides = PptxViewer.fromPresentation(canvas, presentation, {
      ...actions, enableTextSelection: true, enableMediaPlayback: true, width: host.clientWidth,
      onSlideChange: (index, count) => position({ index, count }),
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
      pinch: (origin, gesture) => pinchSurface(slides, host, () => canvas, origin, gesture),
      pages: {
        page: () => ({ index: slides.slideIndex, count: slides.slideCount }),
        go: index => slides.goToSlide(index),
        thumbnail: index => presentation.renderSlideToBitmap(index, { width: THUMBNAIL_WIDTH, dpr: 1 }),
      },
      comments: () => {
        const page = slides.slideIndex;
        const surface = canvas.parentElement;
        return presentation.getComments(page).map((comment, i) => ({
          ...comment, key: comment.id ?? String(i),
          jump: surface ? () => slideCommentAnchor(presentation, comment, page, surface) : undefined,
        }));
      },
      notes: () => presentation.getNotes(slides.slideIndex),
    };
  } catch (error) {
    dispose();
    throw error;
  }
}
