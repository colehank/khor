import type { ComponentType } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import type { SystemKind } from "@/gen/bindings/SystemKind";
import { machineMark, sessionMark } from "@/components/BrandIcons";
import { FaceMark, SIZES, type KindMark } from "@/components/FaceMark";
import { FileKindIcon } from "@/components/FileKindIcon";
import { IconBranch, IconBrowser, IconFiles, IconPlus } from "@/components/icons";

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

/** A new tab's (UX-45): ＋ on the blank face, until a kind is chosen. */
export function newTabKind(): KindMark {
  return { key: "new-tab", glyph: drawn(IconPlus) };
}

/** A machine's kind mark: the system family the node reported, or the
    plain machine mark for one that could not say — never a guess
    (`machineMark`). */
export function machineKind(system: SystemKind | null | undefined): KindMark {
  return { key: system ?? "machine", glyph: drawn(machineMark(system)) };
}

// ── The other marks (UX-32: one assembly for every row and tab) ────────
//
// Each is made in one place, here, so a row and a tab about the same
// thing wear the same mark, and the gate reads one key per kind.

/** A workspace's: a folder, or the branch when it is a repository
    (docs/SHELL.md 第四轮 第 2 条:「文件夹, 是 git 仓库时换分支」). */
export function workspaceKind(repo: boolean): KindMark {
  return repo ? { key: "branch", glyph: drawn(IconBranch) } : { key: "folder", glyph: drawn(IconFiles) };
}

/** A file's: its kind's glyph, the set the tree draws (`FileKindIcon`,
    VS Code's colours) — a picture of its own, not inked by the face. */
export function fileKind(name: string): KindMark {
  return { key: "file", glyph: (c) => <FileKindIcon name={name} className={c} />, picture: true };
}

/** A page's: **the site's own icon** where one was fetched (UX-22, the
    owner 2026-10-05), drawn as it came; the browser's glyph before or
    without one. */
export function pageKind(favicon: string | undefined): KindMark {
  return favicon
    ? { key: "favicon", glyph: (c) => <img src={favicon} alt="" className={`${c} rounded-sm`} />, picture: true }
    : { key: "page", glyph: drawn(IconBrowser) };
}

/** An account's: the provider's mark (Claude's, OpenAI's — `sessionMark`
    already judges a vendor's glyph for an agent session). */
export function providerKind(provider: string): KindMark {
  return { key: `provider:${provider}`, glyph: drawn(sessionMark("agent", provider)) };
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
      face={{ machine: face }}
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
      face={{ machine: face }}
      mark={machineKind(system)}
      size={size}
      className={className}
    />
  );
}
