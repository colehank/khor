import type { Avatar } from "@/gen/bindings/Avatar";
import { MachineAvatar } from "@/components/Avatar";
import { sessionMark } from "@/components/BrandIcons";
import { FaceMark } from "@/components/FaceMark";

/** The corner mark on a session's face: the shape is `sessionMark`'s
    one judgment; this only sizes it. (Moved here from `SessionsList`
    with the assembly around it.) */
function KindMark({ kind, category, className }: { kind: string; category: string | null; className: string }) {
  const M = sessionMark(kind, category);
  return <M className={className} />;
}

/**
 * The two sizes the assembly is drawn at, each whole: the face, the
 * corner's frame and the glyph in it move together, because a row's
 * corner on a tab's face is bigger than the face (tokens.css
 * `--kind-mark-tab`).
 *
 * **`tab` is for a strip's tab and for what stands in for one** (the
 * narrow detail header). It exists because the row's size was used
 * there and did not fit: 32 plus the halo is 40, and a tab is 28 — the
 * ring stood 6 over the tab's top edge (2026-09-28 audit 第 6 条).
 * `check-face` measures the whole ink — halo and corner — against the
 * tab's box and the header's.
 */
const SIZES = {
  row: { face: "size-avatar", frame: "size-kind-mark", glyph: "size-kind-mark-glyph" },
  // `size-4`: the bare machine face a browsing tab wears beside it
  // (`TabStrip`'s `bare`) — one tab, one face size, whichever kind.
  tab: { face: "size-4", frame: "size-kind-mark-tab", glyph: "size-kind-mark-tab-glyph" },
} as const;

/**
 * One session's identity, assembled the one way it is drawn anywhere:
 * the machine's face carrying the row, a halo around it in the state's
 * own colour, and the kind mark on its corner.
 *
 * **Extracted from the list row on 2026-09-02** —「标签页的标签头像
 * 应该和session列表一致。状态灯是环在外侧,右下角是类型」— so the tab
 * strip, the switcher and the narrow detail header draw *this* rather
 * than each their own approximation. One assembly, four surfaces, no
 * drift.
 *
 * **Three channels, three questions, one each.** The halo says how it
 * is, the mark says what it is, the word (wherever the surface prints
 * one) says it in letters. The mark used to say both — shape for what,
 * colour for how — and that is the shape of the mistake the palette
 * ruling took apart on the brand green: one signal answering two
 * questions cannot be changed for one of them without lying about the
 * other. So the mark is neutral and the halo carries the state.
 *
 * The halo paints entirely outside the face's box (`app.css`
 * `[data-halo]` — colour, the busy breath, and the reduced-motion
 * guard all live there, never here). Its gap ring is `--card`, the
 * ground the list row sits on; on the tab strip's grounds that token
 * is a shade off the real ground and reads as part of the ring —
 * close enough not to buy a prop for, said here so a future mismatch
 * has its cause written down.
 *
 * The mark sits on a solid punch-out, or the transparent brand strokes
 * dissolve into the face.
 */
export function SessionFace({
  face,
  word,
  kind,
  category,
  size = "row",
  className,
}: {
  face: Avatar | null;
  /** The state key (`busy`…) — the halo's colour, never a display word. */
  word: string;
  kind: string;
  category: string | null;
  /** A list row's (the switcher's rows too) or a tab's — see `SIZES`. */
  size?: keyof typeof SIZES;
  className?: string;
}) {
  const s = SIZES[size];
  // **`FaceMark` with a halo and a kind mark** (docs/SHELL.md 第四轮 第 2
  // 条): the assembly this file used to draw by hand is the one every
  // sidebar row wears now; a session's is the only one with a halo.
  return (
    <FaceMark
      face={<MachineAvatar face={face} className={s.face} />}
      halo={word}
      mark={category ?? kind}
      badgeData={{ "data-kind-mark": category ?? kind }}
      badgeClass={s.frame}
      badge={<KindMark kind={kind} category={category} className={s.glyph} />}
      className={className}
    />
  );
}
