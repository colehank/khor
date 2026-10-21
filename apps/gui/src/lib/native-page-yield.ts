/** An editor borrows its page's body, never its WebView handle. A release
 * asks the current page owner to recompute; it cannot show an old page. */
import { useLayoutEffect, type RefObject } from 'react';

const leases = new Map<string, Set<HTMLElement>>();
const listeners = new Set<() => void>();
const changed = () => { for (const notify of listeners) notify(); };

export function visiblePageOwner(el: Element): boolean {
  if (!el.isConnected || el.closest('[inert],[aria-hidden="true"],[hidden]')) return false;
  const css = getComputedStyle(el);
  return css.display !== 'none' && css.visibility !== 'hidden' && el.getClientRects().length > 0;
}

export function nativePageYielded(tab: string): boolean {
  return [...(leases.get(tab) ?? [])].some(visiblePageOwner);
}

export function watchNativePageYield(notify: () => void): () => void {
  listeners.add(notify);
  return () => { listeners.delete(notify); };
}

export function useNativePageYield(tab: string, editing: boolean, anchor: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    const el = anchor.current;
    if (!editing || !el) return;
    const set = leases.get(tab) ?? new Set<HTMLElement>();
    leases.set(tab, set);
    set.add(el);
    changed();
    return () => {
      set.delete(el);
      if (!set.size) leases.delete(tab);
      changed();
    };
  }, [tab, editing, anchor]);
}

/** Tooltips are optional hints. Suppress only those which would be
 * painted underneath a visible native page, without hiding that page. */
export function overlapsNativePage(el: Element): boolean {
  const r = el.getBoundingClientRect();
  return [...document.querySelectorAll('[data-native-page-visible="true"]')].some(page => {
    if (!visiblePageOwner(page)) return false;
    const b = page.getBoundingClientRect();
    return r.bottom > b.top && r.top < b.bottom && r.right > b.left && r.left < b.right;
  });
}
