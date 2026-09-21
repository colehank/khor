import type { Avatar } from "@/gen/bindings/Avatar";
import { MachineAvatar } from "@/components/Avatar";
import { sessionMark } from "@/components/BrandIcons";
import { FaceMark } from "@/components/FaceMark";

/** The corner mark on a session's face: the shape is `sessionMark`'s
    one judgment; this only sizes it. (Moved here from `SessionsList`
    with the assembly around it.) */
function KindMark({ kind, category }: { kind: string; category: string | null }) {
  const M = sessionMark(kind, category);
  return <M className="size-kind-mark-glyph" />;
}

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
  className,
}: {
  face: Avatar | null;
  /** The state key (`busy`…) — the halo's colour, never a display word. */
  word: string;
  kind: string;
  category: string | null;
  className?: string;
}) {
  // **`FaceMark` with a halo and a kind mark** (docs/SHELL.md 第四轮 第 2
  // 条): the assembly this file used to draw by hand is the one every
  // sidebar row wears now; a session's is the only one with a halo.
  return (
    <FaceMark
      face={<MachineAvatar face={face} className="size-avatar" />}
      halo={word}
      mark={category ?? kind}
      badgeData={{ "data-kind-mark": category ?? kind }}
      badge={<KindMark kind={kind} category={category} />}
      className={className}
    />
  );
}
