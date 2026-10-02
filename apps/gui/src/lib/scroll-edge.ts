// Whether a page's scroller has content above its top edge — the one
// fact the head's fade needs (`app.css` `[data-nav-clear][data-scrolled]`,
// the audit fix plan F4.11). One listener for every scroller, capturing,
// so a page needs nothing of its own to get it.
export function followScrollEdge(): void {
  document.addEventListener(
    "scroll",
    (e) => {
      const el = e.target;
      if (!(el instanceof HTMLElement) || !el.matches("[data-nav-clear]")) return;
      const up = el.scrollTop > 0;
      if (up !== el.hasAttribute("data-scrolled")) el.toggleAttribute("data-scrolled", up);
    },
    true,
  );
}
