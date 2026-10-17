/** A menu's glass stays still; only its rows scroll. The same cap is
 * measured by the native completion window and the in-page menus. */
import { useLayoutEffect, useRef, type CSSProperties, type ReactNode } from 'react';
import { cn } from '@/lib/utils';
export const MENU_TALLEST = 240;
export function MenuViewport({
  children,
  active,
  room,
  className
}: {
  children: ReactNode;
  active?: number;
  room?: CSSProperties['maxHeight'];
  className?: string;
}) {
  const viewport = useRef<HTMLDivElement>(null),
    items = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const el = viewport.current,
      body = items.current;
    if (!el || !body) return;
    const mark = () => {
      el.toggleAttribute('data-cut-top', el.scrollTop > 0);
      el.toggleAttribute('data-cut-bottom', el.scrollHeight - el.clientHeight - el.scrollTop > 1);
    };
    const watch = new ResizeObserver(mark);
    watch.observe(el);
    watch.observe(body);
    el.addEventListener('scroll', mark, {
      passive: true
    });
    mark();
    return () => {
      watch.disconnect();
      el.removeEventListener('scroll', mark);
    };
  }, []);
  const reveal = (chosen: HTMLElement) => {
    const el = viewport.current;
    if (!el) return;
    const a = el.getBoundingClientRect(),
      b = chosen.getBoundingClientRect();
    const fade = parseFloat(getComputedStyle(el).getPropertyValue('--fade-menu')) || 0;
    const inset = Math.min(fade, Math.max(0, (a.height - b.height) / 2));
    if (b.top < a.top + inset) el.scrollTop -= a.top + inset - b.top;else if (b.bottom > a.bottom - inset) el.scrollTop += b.bottom - a.bottom + inset;
  };
  useLayoutEffect(() => {
    const chosen = viewport.current?.querySelector<HTMLElement>('[data-on=true]');
    if (chosen) reveal(chosen);
  }, [active]);
  return <div ref={viewport} data-menu-viewport className={cn('min-h-0 overflow-x-hidden overflow-y-auto overscroll-contain', className)} style={{
    maxHeight: room === undefined ? MENU_TALLEST : `min(${MENU_TALLEST}px, ${typeof room === 'number' ? room + 'px' : room})`
  }} onFocusCapture={e => reveal(e.target as HTMLElement)} onWheel={e => e.stopPropagation()} onTouchMove={e => e.stopPropagation()}><div ref={items} data-menu-items className="min-w-0 p-menu-inset">{children}</div></div>;
}
