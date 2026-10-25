import { useCallback, useLayoutEffect, useRef, useState, type MouseEvent } from "react";

type Anchor = { node: Element; top: number };
const READER_SCROLL_IDLE_MS = 200;

/** One scroll owner for streaming, disclosures, and viewport changes.
 * Layout-driven scroll events must not be mistaken for a reader leaving the tail.
 */
export function useChatScroll(revision: unknown) {
  const scroller = useRef<HTMLDivElement>(null), column = useRef<HTMLDivElement>(null);
  const following = useRef(true), anchor = useRef<Anchor | null>(null);
  const layoutFrame = useRef(0), readerUntil = useRef(0);
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
  }, []);
  const readPosition = useCallback(() => {
    const el = scroller.current;
    if (!el) return;
    const near = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    following.current = near;
    setAway(!near);
    if (near) { anchor.current = null; setFresh(false); }
    else remember();
    position.current = { top: el.scrollTop, height: el.scrollHeight, viewport: el.clientHeight };
  }, [remember]);
  const acceptReaderScroll = useCallback(() => {
    const el = scroller.current;
    if (!el || performance.now() > readerUntil.current || Math.abs(el.scrollTop - position.current.top) < 0.5) return false;
    // Inertia extends ownership; an input that never scrolls expires on its own.
    readerUntil.current = performance.now() + READER_SCROLL_IDLE_MS;
    releaseLayout();
    readPosition();
    return true;
  }, [readPosition, releaseLayout]);
  const sync = useCallback(function sync() {
    const el = scroller.current;
    if (!el || el.clientHeight === 0) return;
    if (acceptReaderScroll()) return;
    const resized = el.scrollHeight !== position.current.height || el.clientHeight !== position.current.viewport;
    if (resized && !layoutFrame.current) {
      // Caret/layout scroll restoration can arrive after ResizeObserver.
      layoutFrame.current = requestAnimationFrame(() => {
        layoutFrame.current = 0;
        sync();
      });
    }
    if (following.current) el.scrollTop = el.scrollHeight;
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
      if (event instanceof KeyboardEvent && (!/^(ArrowUp|ArrowDown|PageUp|PageDown|Home|End| )$/.test(event.key)
        || (event.target instanceof Element && event.target.closest("input, textarea, [contenteditable]")))) return;
      if (event instanceof PointerEvent && event.type === "pointermove" && event.buttons === 0) return;
      readerUntil.current = performance.now() + READER_SCROLL_IDLE_MS;
      releaseLayout();
    };
    const focusChanged = () => {
      composerHadFocus.current = document.activeElement?.matches("[data-composer-text]") ?? false;
    };
    focusChanged();
    document.addEventListener("focusin", focusChanged);
    const inputs = ["wheel", "touchmove", "pointerdown", "pointermove", "keydown"] as const;
    for (const input of inputs) el.addEventListener(input, readerInput, { passive: true });
    sync();
    return () => {
      observer.disconnect();
      document.removeEventListener("focusin", focusChanged);
      releaseLayout();
      for (const input of inputs) el.removeEventListener(input, readerInput);
    };
  }, [sync, releaseLayout]);
  useLayoutEffect(() => {
    sync();
    if (!following.current) setFresh(true);
  }, [revision, sync]);

  const onScroll = () => {
    const el = scroller.current;
    if (!el || el.clientHeight === 0 || acceptReaderScroll()) return;
    const focus = document.activeElement;
    const composerOwnsLayout = focus?.matches("[data-composer-text]")
      || (composerHadFocus.current && (focus === document.body || !document.hasFocus()));
    // Native caret restoration can outlive composer focus; BODY is not a reader action.
    if (composerOwnsLayout || layoutFrame.current || el.scrollHeight !== position.current.height || el.clientHeight !== position.current.viewport) {
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
    readerUntil.current = 0;
    releaseLayout();
    anchor.current = { node, top: node.getBoundingClientRect().top - el.getBoundingClientRect().top };
    following.current = false;
    setAway(el.scrollHeight > el.clientHeight);
  };
  const toBottom = () => {
    readerUntil.current = 0;
    releaseLayout();
    following.current = true; anchor.current = null;
    setAway(false); setFresh(false); sync();
  };
  return { scroller, column, following, onScroll, captureDisclosure, toBottom, away, fresh };
}
