import type { ComponentType } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import type { SystemKind } from "@/gen/bindings/SystemKind";
import { MachineAvatar } from "@/components/Avatar";
import { machineMark, sessionMark } from "@/components/BrandIcons";
import { FaceMark, SIZES, type KindMark } from "@/components/FaceMark";

/** A glyph component as a `KindMark`'s drawing, at whatever size the
    assembly asks for. */
const drawn =
  (M: ComponentType<{ className?: string }>) =>
  (className: string) => <M className={className} />;

/** A session's kind mark: the shape is `sessionMark`'s one judgment;
    the anchor is the category when the node named one, else the kind.
    (Moved here from `SessionsList` with the assembly around it.) */
export function sessionKind(kind: string, category: string | null): KindMark {
  const key = category ?? kind;
  return { key, glyph: drawn(sessionMark(kind, category)), data: { "data-kind-mark": key } };
}

/** A machine's kind mark: the system family the node reported, or the
    plain machine mark for one that could not say — never a guess
    (`machineMark`). */
export function machineKind(system: SystemKind | null | undefined): KindMark {
  return { key: system ?? "machine", glyph: drawn(machineMark(system)) };
}

/**
 * One session's identity, assembled the one way it is drawn anywhere:
 * the machine's face carrying the row, a halo around it in the state's
 * own colour, and the kind mark — on the face or on its corner, as the
 * 外观 setting says (`FaceMark`).
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
 * guard all live there, never here). Its gap ring is `--halo-under`,
 * `--card` unless the ground says otherwise: the tab strip does
 * (2026-09-30) — a tab not in front stands on the frame, 7.1 / 9.8 L*
 * off the paper since 板是纸底是框, and a paper ring there read as a
 * second ring. The list rows' `--side` is still a shade off it (3.9 /
 * 4.4 L*), and on the macOS app the block is the system's material,
 * which no colour matches — said here so the mismatch has its cause
 * written down.
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
  /** A list row's (the switcher's rows too) or a tab's — see `FaceMark`'s
      `SIZES`. */
  size?: keyof typeof SIZES;
  className?: string;
}) {
  return (
    <FaceMark
      face={<MachineAvatar face={face} className={SIZES[size].face} />}
      halo={word}
      mark={sessionKind(kind, category)}
      size={size}
      className={className}
    />
  );
}

/**
 * **A machine's avatar with its system on it** — the machine list row's
 * own assembly, for any other place that shows a machine *as a kind of
 * machine* (the machine page's header, 2026-10-02: the page's head wears
 * the list row's avatar and follows the 类型标记 setting with it).
 */
export function MachineFace({
  face,
  system,
  size = "row",
  className,
}: {
  face: Avatar | null;
  system: SystemKind | null | undefined;
  size?: keyof typeof SIZES;
  className?: string;
}) {
  return (
    <FaceMark
      face={<MachineAvatar face={face} className={SIZES[size].face} />}
      mark={machineKind(system)}
      size={size}
      className={className}
    />
  );
}
