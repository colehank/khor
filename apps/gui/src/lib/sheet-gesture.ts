/** Menus own a pan for its entire lifetime, including at their scroll edges.
 * Unlike the sheet's main document, a candidate list never hands movement
 * to the sheet behind it. Shared by form and content sheets. */
export function sheetGestureReserved(target: Element | null): boolean {
  return !!target?.closest('[data-sheet-hold], [data-menu-frame], [data-menu-viewport], [role="listbox"], [role="menu"], input[type="range"]');
}
