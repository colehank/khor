// **The folded capsule stands midway between the content's last line and
// the screen's foot** (item 72, the owner 2026-10-07: 「现在视觉上，缩小药丸
// 是贴着屏幕底部的，我想要的是终端内容到屏幕底部的居中位置」).
//
// Item 62 put the capsule in the home indicator's band and ended the
// content at the band's top. A terminal's grid is whole rows, though: the
// fit leaves the part-row that does not fit (and the box's own padding)
// empty above the band, so the last drawn row ends that much higher and
// the capsule, centred in the band, reads as stuck to the screen's edge.
// The capsule is lifted by half that empty strip, which puts its centre
// at the midpoint of the last row's bottom and the room's bottom. The
// grid keeps its rows: nothing about the box changes, the capsule only
// stands in the strip no row was drawn in.
//
// The content says where its last line ends by marking that element
// `data-fold-foot` (a terminal marks xterm's screen, whose height is its
// rows); a content with no mark leaves the capsule centred in the band.

/** How far (px) the capsule rises from the centre of its room, so that
    its centre is midway between `foot` (the last line's bottom) and the
    room's bottom. Never down: a foot below the room's top is not a foot. */
export const foldLift = (room: { top: number }, foot: number): number => Math.max(0, (room.top - foot) / 2);

/** **Where layout put the foot, not where it is drawn this frame**: a
    terminal's screen follows a finger between two rows by a part-row
    `translateY` (`lib/term-scroller`) and goes back to 0 when the scroll
    rests, with no resize to say so. The capsule seats on the grid, so
    the ancestors' shifts up to the content are taken back out. */
export function footBottom(foot: HTMLElement, content: HTMLElement): number {
  let y = foot.getBoundingClientRect().bottom;
  for (let at: HTMLElement | null = foot; at && at !== content; at = at.parentElement) {
    const t = getComputedStyle(at).transform;
    if (t && t !== "none") y -= new DOMMatrixReadOnly(t).m42;
  }
  return y;
}

/** The shown element marked as the content's last line, if any. */
function shownFoot(content: HTMLElement): HTMLElement | null {
  for (const el of content.querySelectorAll<HTMLElement>("[data-fold-foot]")) {
    if (el.getClientRects().length > 0) return el;
  }
  return null;
}

/**
 * Keep the folded capsule seated (`--fold-lift` on `room`, the folded
 * bar's `[data-deck-fold]`, read by `app.css`) while it is up. Placed again
 * whenever the grid refits (a font size, the fold's own refit, a
 * rotation) or the content's box moves (the keyboard going): the foot and
 * the content are watched for size, the window for a resize. Returns the
 * stop.
 */
export function seatFold(room: HTMLElement, content: HTMLElement): () => void {
  let foot: HTMLElement | null = null;
  const place = () => {
    const now = shownFoot(content);
    if (now !== foot) {
      if (foot) watch.unobserve(foot);
      foot = now;
      if (foot) watch.observe(foot);
    }
    const lift = foot ? foldLift(room.getBoundingClientRect(), footBottom(foot, content)) : 0;
    room.style.setProperty("--fold-lift", `${lift}px`);
  };
  const watch = new ResizeObserver(place);
  watch.observe(content);
  watch.observe(room);
  window.addEventListener("resize", place);
  place();
  return () => {
    watch.disconnect();
    window.removeEventListener("resize", place);
    room.style.removeProperty("--fold-lift");
  };
}
