import type { CSSProperties, ReactNode } from "react";

import { cn } from "@/lib/utils";

/**
 * **A face and the mark on its corner — one primitive, five sidebars**
 * (docs/SHELL.md 第四轮 第 2 条:「统一头像的原语一致性」).
 *
 * What sits in the middle is the column's: a machine's avatar for a
 * session, a machine, a workspace and a browsing tab, the dial for an
 * account. What sits on the corner says *what kind of thing* the row is
 * — a session's kind, a machine, a folder or a branch, a page, a
 * provider — on a solid punch-out, or the mark's strokes dissolve into
 * the face under it.
 *
 * A `halo` is the session's state ring and nobody else's: the six state
 * words belong to sessions (the palette ruling), so the prop exists for
 * that one caller rather than as a colour any row could wear.
 *
 * Extracted from `SessionFace` (2026-09-02's assembly); `SessionFace`
 * is now this with a halo and a kind mark.
 */
export function FaceMark({
  face,
  badge,
  mark,
  halo,
  badgeData,
  badgeClass = "size-kind-mark",
  beside = false,
  className,
}: {
  face: ReactNode;
  /** The corner glyph, already sized; absent draws no corner at all. */
  badge?: ReactNode;
  /** What the corner says, as a key — the anchor a gate reads, never a
      word. */
  mark?: string;
  /** A session's state key (`busy`…), for the ring outside the face. */
  halo?: string;
  /** Extra anchors on the corner, for the surfaces that were measured
      by an older name (`data-kind-mark` on a session's). */
  badgeData?: Record<string, string>;
  /** The corner's frame. A list row's by default; a tab hands in its
      own, smaller one with a face sized for a tab (`SessionFace`). */
  badgeClass?: string;
  /**
   * **The corner stands beside the face instead of on it** — for a face
   * whose middle carries words: the 额度 dial prints its reading there,
   * and a corner laid over the dial's lower right sat on the number's
   * last glyph (2026-09-28 audit 第 8 条:「39%」「124$」 lost it). The
   * box grows by the corner's width on the right and the corner sits in
   * that room, so the reading is whole and the arc is not covered
   * either — the dial's box already hugs its ink (`Face` `ratio`), so
   * there is no blank corner of it to stand in.
   */
  beside?: boolean;
  className?: string;
}) {
  return (
    // `flex`, not inline: an inline wrap grows the baseline's few
    // pixels under the face and the mark's corner offset bites into
    // that gap instead of the face (mandala's .badged-face fix, the
    // flex spelling of display:block+line-height:0).
    <span data-side-face className={cn("relative flex flex-none", beside && "pr-kind-mark", className)}>
      {face}
      {halo !== undefined && (
        // Behind nothing and in front of nothing — it paints outside the
        // face's box entirely (`app.css`), so it can neither cover the
        // artwork nor be covered by it.
        <span
          data-halo={halo}
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 rounded-full"
          style={{ "--halo-color": `var(--state-${halo})` } as CSSProperties}
        />
      )}
      {badge !== undefined && (
        <span
          data-side-badge={mark ?? ""}
          {...badgeData}
          className={cn(
            "absolute -right-0.5 -bottom-0.5 grid place-items-center rounded-sm bg-card text-muted-foreground",
            badgeClass,
          )}
        >
          {badge}
        </span>
      )}
    </span>
  );
}
