// Whether a page's scroller has content above its top edge — the one
// fact the head's fade needs (`app.css` `[data-nav-clear][data-scrolled]`,
// the audit fix plan F4.11). One listener for every scroller, capturing,
// so a page needs nothing of its own to get it.
/** The scrollers whose top edge fades once scrolled. */
export const EDGED = "[data-nav-clear], [data-pane-body], [data-files-list]";

export function followScrollEdge(): void {
  document.addEventListener(
    "scroll",
    (e) => {
      const el = e.target;
      // Every scroller under a head (V12): a page's, a list's body, a
      // machine's listing.
      if (!(el instanceof HTMLElement) || !el.matches(EDGED)) return;
      const up = el.scrollTop > 0;
      if (up !== el.hasAttribute("data-scrolled")) el.toggleAttribute("data-scrolled", up);
    },
    true,
  );
}
