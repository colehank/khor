// **Search at the bottom on a phone** (UX-11, the owner's ruling on
// ux-9's I4, 2026-10-04). HIG Search fields (iOS 26): 「Place search at
// the bottom if there's room」 — as a tab bar's search item or a toolbar's
// — expanding into a field above the keyboard; Apple's own apps (Music,
// Photos) put a search circle beside the tab bar. On the narrow face the
// landing's search moves there: a circle right of the bottom bar's
// capsule (`App`'s nav row) which, pressed, turns the row into the
// landing's own search field with the keyboard up; the list under it
// narrows in place, and 取消 gives the row back to the bar. The head
// keeps 筛选 and ＋. A wide window keeps its search in the column's head
// and ⌘F.
//
// The circle and the field belong to different components — the bar is
// the shell's, the field the landing's (`PaneBar`) — so the circle says
// 「open」 as an event, and the field is drawn into the row's seat.

/** Pressed: the landing on screen opens its search into the seat. */
export const SEARCH_OPEN = "khor:search-open";

/** The bottom row's seat, where a landing draws its open search. */
export function searchSeat(): HTMLElement | null {
  return document.querySelector<HTMLElement>("[data-bottom-search-seat]");
}
