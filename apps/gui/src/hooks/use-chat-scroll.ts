import { useCallback, useLayoutEffect, useRef, useState, type MouseEvent } from "react";

type Anchor = { node: Element; top: number };
const SCROLL_IDLE_MS = 200;

/** One scroll owner for streaming, disclosures, and viewport changes.
 * Layout-driven scroll events must not be mistaken for a reader leaving the tail.
 */
export function useChatScroll(revision: unknown) {
  const scroller = useRef<HTMLDivElement>(null), column = useRef<HTMLDivElement>(null);
  const following = useRef(true), readingLatest = useRef(true), anchor = useRef<Anchor | null>(null);
  const layoutFrame = useRef(0), readerUntil = useRef(0), positionRevision = useRef(0);
  const layoutUntil = useRef(0);
  const readerFrame = useRef(0);
  const composerHadFocus = useRef(false);
  const position = useRef({ top: 0, height: 0, viewport: 0 });
  const [away, setAway] = useState(false), [fresh, setFresh] = useState(false);
  const remember = useCallback(() => {
    const el = scroller.current, body = column.current;
    if (!el || !body) return;
    const top = el.getBoundingClientRect().top;
    const node = [...body.children].find((child) => child.getBoundingClientRect().bottom > top);
    anchor.current = node ? { node, top: node.getBoundingClientRect().top - top } : null;
  }, []);
  const releaseLayout = useCallback(() => {
    cancelAnimationFrame(layoutFrame.current);
    layoutFrame.current = 0;
    layoutUntil.current = 0;
  }, []);
  const releaseReader = useCallback(() => {
    cancelAnimationFrame(readerFrame.current);
    readerFrame.current = 0;
    readerUntil.current = 0;
  }, []);
  const readPosition = useCallback(() => {
    const el = scroller.current;
    if (!el) return;
    positionRevision.current++;
    const movingAway = el.scrollTop < position.current.top;
    const near = !movingAway && el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    following.current = near;
    readingLatest.current = near;
    setAway(!near);
    if (near) { anchor.current = null; setFresh(false); }
    else remember();
    position.current = { top: el.scrollTop, height: el.scrollHeight, viewport: el.clientHeight };
  }, [remember]);
  const acceptReaderScroll = useCallback(() => {
    const el = scroller.current;
    if (!el || (!readerFrame.current && performance.now() > readerUntil.current) || Math.abs(el.scrollTop - position.current.top) < 0.5) return false;
    // Inertia extends ownership; an input that never scrolls expires on its own.
    cancelAnimationFrame(readerFrame.current);
    readerFrame.current = 0;
    readerUntil.current = performance.now() + SCROLL_IDLE_MS;
    releaseLayout();
    readPosition();
    return true;
  }, [readPosition, releaseLayout]);
  const sync = useCallback(function sync() {
    const el = scroller.current;
    if (!el || el.clientHeight === 0) return;
    if (acceptReaderScroll()) return;
    const resized = el.scrollHeight !== position.current.height || el.clientHeight !== position.current.viewport;
    // Native offset restoration may arrive after the observer's follow-up
    // frame. Keep that layout ownership until it settles; real input above
    // always wins immediately and clears this lease.
    if (resized) layoutUntil.current = performance.now() + SCROLL_IDLE_MS;
    if (resized && !layoutFrame.current) {
      // Caret/layout scroll restoration can arrive after ResizeObserver.
      layoutFrame.current = requestAnimationFrame(() => {
        layoutFrame.current = 0;
        sync();
      });
    }
    if (following.current) {
      const bottom = Math.max(0, el.scrollHeight - el.clientHeight);
      if (Math.abs(el.scrollTop - bottom) > 0.5) el.scrollTop = bottom;
    }
    else if (anchor.current?.node.isConnected) {
      const { node, top } = anchor.current;
      const delta = node.getBoundingClientRect().top - el.getBoundingClientRect().top - top;
      if (Math.abs(delta) > 0.5) el.scrollTop += delta;
    }
    if (!following.current) setAway(el.scrollHeight > el.clientHeight);
    position.current = { top: el.scrollTop, height: el.scrollHeight, viewport: el.clientHeight };
  }, [acceptReaderScroll]);
  useLayoutEffect(() => {
    const el = scroller.current, body = column.current;
    if (!el || !body) return;
    const observer = new ResizeObserver(sync);
    observer.observe(el); observer.observe(body);
    const readerInput = (event: Event) => {
      const target = event.target instanceof Element ? event.target : null;
      if (event instanceof KeyboardEvent) {
        if (!/^(ArrowUp|ArrowDown|PageUp|PageDown|Home|End| )$/.test(event.key)
          || target?.closest("input, textarea, select, [contenteditable]")) return;
        if (event.key === " " && target?.closest("button, [role=button]")) return;
      }
      if (event instanceof PointerEvent) {
        if (event.type === "pointermove" && event.buttons === 0) return;
        // Activating a control may shrink its card and clamp scrollTop. That
        // is layout, not a request to pause canonical reading. Wheel/touch
        // gestures over the same control still take ownership below.
        if (target?.closest("button, [role=button], a, input, textarea, select, [contenteditable=true]")) return;
      }
      positionRevision.current++;
      readerUntil.current = performance.now() + SCROLL_IDLE_MS;
      cancelAnimationFrame(readerFrame.current);
      // Start idle time after the browser can apply this input, even after a busy handler.
      readerFrame.current = requestAnimationFrame(() => {
        readerFrame.current = 0;
        readerUntil.current = performance.now() + SCROLL_IDLE_MS;
      });
      releaseLayout();
    };
    const focusChanged = () => {
      composerHadFocus.current = document.activeElement?.matches("[data-composer-text]") ?? false;
    };
    focusChanged();
    document.addEventListener("focusin", focusChanged);
    const inputs = ["wheel", "touchmove", "pointerdown", "pointermove", "keydown"] as const;
    // A passive WebKit wheel may scroll before delivering its input intent.
    for (const input of inputs) el.addEventListener(input, readerInput, { passive: input !== "wheel" });
    sync();
    return () => {
      observer.disconnect();
      document.removeEventListener("focusin", focusChanged);
      releaseLayout();
      releaseReader();
      for (const input of inputs) el.removeEventListener(input, readerInput);
    };
  }, [sync, releaseLayout, releaseReader]);
  // Draft and control renders can change the live viewport without changing
  // the transcript revision. History scrolling owns its position until the
  // browser delivers its scroll event; unrelated renders must not restore it.
  useLayoutEffect(() => { if (following.current) sync(); });
  useLayoutEffect(() => {
    if (following.current) return;
    sync();
    if (!following.current) setFresh(true);
  }, [revision, sync]);

  const capturePositionTicket = useCallback(() => positionRevision.current, []);
  /** A late page may restore only the position it captured. The guard runs
   * before moving anything or retiring reader ownership; newer input wins. */
  const acceptRestoredPosition = useCallback((ticket: number, restore: () => void) => {
    const el = scroller.current;
    if (!el || ticket !== positionRevision.current) return false;
    releaseReader();
    releaseLayout();
    restore();
    remember();
    position.current = { top: el.scrollTop, height: el.scrollHeight, viewport: el.clientHeight };
    return true;
  }, [releaseLayout, releaseReader, remember]);
  const onScroll = () => {
    const el = scroller.current;
    if (!el || el.clientHeight === 0 || acceptReaderScroll()) return;
    const focus = document.activeElement;
    const composerOwnsLayout = focus?.matches("[data-composer-text]")
      || (composerHadFocus.current && (focus === document.body || !document.hasFocus()));
    // Native caret restoration can outlive composer focus; BODY is not a reader action.
    if (composerOwnsLayout || layoutFrame.current || (following.current && performance.now() < layoutUntil.current) || el.scrollHeight !== position.current.height || el.clientHeight !== position.current.viewport) {
      sync();
      return;
    }
    if (Math.abs(el.scrollTop - position.current.top) >= 0.5) readPosition();
  };
  const captureDisclosure = (event: MouseEvent<HTMLDivElement>) => {
    if (!(event.target instanceof Element)) return;
    const button = event.target.closest('[data-slot="trace-collapsible-trigger"], [data-diff-expand]');
    const el = scroller.current;
    if (!button || !el) return;
    const node = button.matches("[data-diff-expand]")
      ? button.closest("[data-chat-tool]")?.querySelector("[data-tool-toggle]") ?? button
      : button;
    positionRevision.current++;
    releaseReader();
    releaseLayout();
    anchor.current = { node, top: node.getBoundingClientRect().top - el.getBoundingClientRect().top };
    // Inspecting a disclosure pins its geometry. It does not say the reader
    // left latest; only a real reader scroll or explicit history navigation does.
    following.current = false;
    setAway(el.scrollHeight > el.clientHeight);
  };
  const toBottom = () => {
    positionRevision.current++;
    releaseReader();
    releaseLayout();
    following.current = true; readingLatest.current = true; anchor.current = null;
    setAway(false); setFresh(false); sync();
  };
  return { scroller, column, following, readingLatest, onScroll, captureDisclosure, toBottom, capturePositionTicket, acceptRestoredPosition, away, fresh };
}
