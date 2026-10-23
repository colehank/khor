import { useCallback, useLayoutEffect, useRef, useState, type MouseEvent } from "react";

type Anchor = { node: Element; top: number };

/** One scroll owner for streaming, disclosures, and viewport changes.
 * Layout-driven scroll events must not be mistaken for a reader leaving the tail.
 */
export function useChatScroll(revision: unknown) {
  const scroller = useRef<HTMLDivElement>(null), column = useRef<HTMLDivElement>(null);
  const following = useRef(true), anchor = useRef<Anchor | null>(null);
  const position = useRef({ top: 0, height: 0 });
  const [away, setAway] = useState(false), [fresh, setFresh] = useState(false);
  const remember = useCallback(() => {
    const el = scroller.current, body = column.current;
    if (!el || !body) return;
    const top = el.getBoundingClientRect().top;
    const node = [...body.children].find((child) => child.getBoundingClientRect().bottom > top);
    anchor.current = node ? { node, top: node.getBoundingClientRect().top - top } : null;
  }, []);
  const sync = useCallback(() => {
    const el = scroller.current;
    if (!el || el.clientHeight === 0) return;
    if (following.current) el.scrollTop = el.scrollHeight;
    else if (anchor.current?.node.isConnected) {
      const { node, top } = anchor.current;
      const delta = node.getBoundingClientRect().top - el.getBoundingClientRect().top - top;
      if (Math.abs(delta) > 0.5) el.scrollTop += delta;
    }
    if (!following.current) setAway(el.scrollHeight > el.clientHeight);
    position.current = { top: el.scrollTop, height: el.scrollHeight };
  }, []);
  useLayoutEffect(() => {
    const el = scroller.current, body = column.current;
    if (!el || !body) return;
    const observer = new ResizeObserver(sync);
    observer.observe(el); observer.observe(body);
    sync();
    return () => observer.disconnect();
  }, [sync]);
  useLayoutEffect(() => {
    sync();
    if (!following.current) setFresh(true);
  }, [revision, sync]);

  const onScroll = () => {
    const el = scroller.current;
    if (!el || el.clientHeight === 0) return;
    // A content resize can dispatch scroll before the ResizeObserver callback.
    if (el.scrollHeight !== position.current.height) { sync(); return; }
    if (Math.abs(el.scrollTop - position.current.top) < 0.5) return;
    const near = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    following.current = near;
    setAway(!near);
    if (near) { anchor.current = null; setFresh(false); }
    else remember();
    position.current = { top: el.scrollTop, height: el.scrollHeight };
  };
  const captureDisclosure = (event: MouseEvent<HTMLDivElement>) => {
    if (!(event.target instanceof Element)) return;
    const button = event.target.closest('[data-slot="trace-collapsible-trigger"], [data-diff-expand]');
    const el = scroller.current;
    if (!button || !el) return;
    const node = button.matches("[data-diff-expand]")
      ? button.closest("[data-chat-tool]")?.querySelector("[data-tool-toggle]") ?? button
      : button;
    anchor.current = { node, top: node.getBoundingClientRect().top - el.getBoundingClientRect().top };
    following.current = false;
    setAway(el.scrollHeight > el.clientHeight);
  };
  const toBottom = () => {
    following.current = true; anchor.current = null;
    setAway(false); setFresh(false); sync();
  };
  return { scroller, column, following, onScroll, captureDisclosure, toBottom, away, fresh };
}
