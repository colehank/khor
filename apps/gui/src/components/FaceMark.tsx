import type { CSSProperties, ReactNode } from "react";

import { UnderMark } from "@/components/Avatar";
import { cn } from "@/lib/utils";
import { useSetting } from "@/settings";

/**
 * What a row's kind mark is, as a column hands it in.
 *
 * `glyph` draws the mark **at a size this file picks**: the same mark is
 * 18 on a face and 14 in a corner (`SIZES`), so a column that sized it
 * itself would be a column that knows which way the marks are drawn.
 * A render function rather than a component, so a browsing tab's favicon
 * — an `<img>` built per row — is not remounted on every render.
 */
export type KindMark = {
  /** What it says, as a key — the anchor a gate reads, never a word. */
  key: string;
  glyph: (className: string) => ReactNode;
  /** Extra anchors on the mark, for the surfaces that were measured by
      an older name (`data-kind-mark` on a session's). */
  data?: Record<string, string>;
};

/**
 * The two sizes the assembly is drawn at, each whole, in both ways of
 * drawing the mark.
 *
 * - `face`: the box the face is drawn in.
 * - `on`: the mark painted **on** the face (「浮现」): `--kind-mark` 18 on
 *   the row's 32 and `--kind-mark-tab` 12 on a tab's 16 — both leave a
 *   whole-pixel margin (7 and 2) all round, so the mark lands on whole
 *   pixels on a 1× screen (the owner's is 2560×1440 at 1×). **12, not
 *   the corner's 10, on a tab** (2026-10-02, seen at 1×): at 10 the
 *   terminal's strokes are under a pixel and a 几何 face's own shapes
 *   read through them; the tab is where the mark is smallest, so it
 *   takes the most of the face it can while keeping a ring of colour to
 *   say which machine. Not simplified and not sent to the corner — the
 *   setting is the one place that decides where a mark goes.
 * - `frame` / `glyph`: the corner's punch-out and the glyph in it
 *   (「角标」), unchanged from before the setting existed.
 *
 * **`tab` is for a strip's tab and for what stands in for one** (the
 * narrow detail header): the row's 32 plus the halo is 40, and a tab is
 * 28 (2026-09-28 audit 第 6 条).
 */
export const SIZES = {
  row: { face: "size-avatar", on: "size-kind-mark", frame: "size-kind-mark", glyph: "size-kind-mark-glyph" },
  tab: { face: "size-4", on: "size-kind-mark-tab", frame: "size-kind-mark-tab", glyph: "size-kind-mark-tab-glyph" },
} as const;

/** Whether this app draws kind marks in the corner (「角标」) rather
    than on the face (「浮现」, the default). Read here and nowhere else:
    every avatar that carries a kind is a `FaceMark`. */
export function useMarkInCorner(): boolean {
  // The reader tests for the value that is *not* the default
  // (`src/settings.ts`): not answered yet, and `face`, are one case.
  return useSetting("kind-mark") === "corner";
}

/**
 * **A face and what kind of thing it is — one primitive, every place an
 * avatar says a kind** (docs/SHELL.md 第四轮 第 2 条:「统一头像的原语一致
 * 性」; 2026-10-02 two ways of drawing it).
 *
 * What sits in the middle is the column's: a machine's avatar for a
 * session, a machine, a workspace and a browsing tab, the dial for an
 * account. The mark says *what kind of thing* the row is — a session's
 * kind, a machine's system, a folder or a repository, a page — and it is
 * drawn one of two ways, by the 外观 setting 类型标记 (the owner,
 * 2026-10-02:「对象类型的绘制形式，角标、浮现这样。不做对调了，就是这两种
 * 选择」):
 *
 * - **浮现** (`face`, the default): the mark is painted on the face,
 *   centred, in `--mark-ink`, and the face under it wears `--mark-veil`
 *   (`UnderMark`). No corner. One avatar says two things at once: the
 *   shape is what kind, the colours are which machine.
 * - **角标** (`corner`): the face is the avatar and the mark sits on a
 *   solid punch-out at its bottom right, or its strokes dissolve into
 *   the face under it — what khor drew before the setting.
 *
 * Anchors: the mark carries `data-side-mark` (its key) and
 * `data-mark-at` (`face` / `corner`) either way; only the corner is
 * `data-side-badge`.
 *
 * A `halo` is the session's state ring and nobody else's: the six state
 * words belong to sessions (the palette ruling), so the prop exists for
 * that one caller rather than as a colour any row could wear. It paints
 * outside the face's box, so it is the same ring in both ways.
 */
export function FaceMark({
  face,
  mark,
  halo,
  size = "row",
  className,
}: {
  face: ReactNode;
  /** What kind of thing this is; absent draws the face alone. */
  mark?: KindMark;
  /** A session's state key (`busy`…), for the ring outside the face. */
  halo?: string;
  /** A list row's (the switcher's rows too) or a tab's — see `SIZES`. */
  size?: keyof typeof SIZES;
  /* ⚠️ 墓碑 (2026-09-28 → 09-30): `beside` stood the corner next to the
     face instead of on it, for the 额度 dial, whose reading its corner
     covered (audit 第 8 条). The 额度 row's mark opens its title now (the
     owner, 2026-09-30:「把类型的头像做成标题的开头」), so no face carries
     words under a corner and the prop went with its one caller. */
  className?: string;
}) {
  const corner = useMarkInCorner();
  const s = SIZES[size];
  const onFace = mark !== undefined && !corner;
  return (
    // `flex`, not inline: an inline wrap grows the baseline's few
    // pixels under the face and the mark's corner offset bites into
    // that gap instead of the face (mandala's .badged-face fix, the
    // flex spelling of display:block+line-height:0).
    <span data-side-face data-mark-way={mark ? (corner ? "corner" : "face") : undefined} className={cn("relative flex flex-none", className)}>
      <UnderMark.Provider value={onFace}>{face}</UnderMark.Provider>
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
      {mark !== undefined &&
        (onFace ? (
          // **On the face**: centred over the face's box, the mark's one
          // ink. The veil is the face's (`UnderMark`), so it takes the
          // face's crop — round, rounded or square.
          <span
            data-side-mark={mark.key}
            data-mark-at="face"
            {...mark.data}
            aria-hidden="true"
            className="pointer-events-none absolute inset-0 grid place-items-center text-mark-ink"
          >
            {mark.glyph(s.on)}
          </span>
        ) : (
          <span
            data-side-mark={mark.key}
            data-side-badge={mark.key}
            data-mark-at="corner"
            {...mark.data}
            className={cn(
              "absolute -right-0.5 -bottom-0.5 grid place-items-center rounded-sm bg-card text-muted-foreground",
              s.frame,
            )}
          >
            {mark.glyph(s.glyph)}
          </span>
        ))}
    </span>
  );
}
