import {overlayHeights} from "./overlay-policy";
/** Detents share the screen's foot and stay below its platform chrome.
 * The keyboard can leave less room than either the preferred detent or
 * the top inset, so every returned height is bounded and nonnegative. */
export type ContentSheetBounds = { height: number; foot: number; top?: number };

export function contentSheetHeights(bounds: ContentSheetBounds, shares: { half: number; full: number }) {
  return overlayHeights(bounds,{initial:'medium',maximum:'large',content:bounds.height,mediumShare:shares.half,largeShare:shares.full});
}
