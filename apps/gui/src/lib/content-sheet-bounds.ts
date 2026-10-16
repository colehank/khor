/** Detents share the screen's foot and stay below its platform chrome.
 * The keyboard can leave less room than either the preferred detent or
 * the top inset, so every returned height is bounded and nonnegative. */
export type ContentSheetBounds = { height: number; foot: number; top?: number };

export function contentSheetHeights(bounds: ContentSheetBounds, shares: { half: number; full: number }) {
  const nonnegative = (value: number) => Number.isFinite(value) ? Math.max(0, value) : 0;
  const height = nonnegative(bounds.height), foot = nonnegative(bounds.foot), top = nonnegative(bounds.top ?? 0);
  const available = Math.max(0, height - foot - top);
  const full = Math.min(available, Math.max(0, height * shares.full - foot));
  const half = Math.min(full, Math.max(0, height * shares.half - foot));
  return { half, full };
}
