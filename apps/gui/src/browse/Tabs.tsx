// **Every tab strip in the app, with everything that comes with one**
// (docs/SHELL.md 第四轮 第 9 条:「标签页的逻辑也要统一, 也要抽象」) — the
// `SideList` of tabs.
//
// Three places draw tabs (a session's, a workspace's files, and the
// browse landing's pages). Until this file they shared only the drawing
// (`TabStrip`), and each wired the rest for itself. Here it is written
// once: the strip (`TabStrip` — every tab one width, shedding, the
// wheel, into view, ＋ after the last) and the region under it.
//
// ⚠️ 墓碑 (UX-45): the count and the switcher it opened (第 10 条:「点击
// 计数按钮展开后, 再次点击要能收起」), in the region under the strip. The
// strip draws every tab now, down to its face, and scrolls past that, so
// there is no count and nothing for a switcher to show that is hidden.
//
// A place hands in its tabs, which is in front, and what focusing and
// closing one does; what the region shows is its `children`.
//
// **What is not here**: 标签页上限. It is enforced where a tab is
// *opened*, which is the tab store's business and not the strip's — the
// browse tabs' in the node (`khor tabs open` obeys it too), the other two
// in their hooks through one shared judgment (`tab-limit.ts`).
import { type ReactNode } from "react";

import { TabStrip, type TabFace } from "@/browse/TabStrip";
import { Region } from "@/components/Region";
import { band } from "@/lib/scroll-edge";
import { cn } from "@/lib/utils";

export function Tabs({
  faces,
  active,
  onFocus,
  onClose,
  onNew,
  head,
  tail,
  stripData,
  bodyClass,
  frame = true,
  caption,
  frontAct = false,
  children,
}: {
  /** The front tab's act at the capsule's end (`TabStrip` `frontAct`). */
  frontAct?: boolean;
  /** A stale region's one line at R0's right end, after the strip
      (`Region caption`; components/RegionState). Framed strips only. */
  caption?: ReactNode;
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  onNew?: () => void;
  head?: ReactNode;
  tail?: ReactNode;
  /** Anchors on the strip's wrapper — each place's gates know its strip
      by a name of its own (`data-session-tabs`, `data-ws-tab-strip`). */
  stripData?: Record<string, string>;
  /** How the region under the strip sits in its column. */
  bodyClass?: string;
  /**
   * Whether the strip is a region's first row with the board under it
   * (`Region`, docs/SHELL.md 第十三轮 第 5 条) — the sessions landing.
   * `false` where the strip heads one column among others (the
   * workspace's middle column, between its tree and its sessions): the
   * body is the card's colour flat under the strip, so the open tab
   * still joins what it opens.
   */
  frame?: boolean;
  children: ReactNode;
}) {
  const strip = (
    <div {...stripData} className="flex min-w-0 flex-1 flex-col">
      <TabStrip
        frontAct={frontAct}
        faces={faces}
        active={active}
        onFocus={onFocus}
        onClose={onClose}
        onNew={onNew}
        head={head}
        tail={tail}
      />
    </div>
  );
  return frame ? (
    <Region kind="tabs" head={strip} caption={caption} className={bodyClass}>
      {children}
    </Region>
  ) : (
    <>
      {/* A band like a region's head (`lib/scroll-edge`): what scrolls
          under the strip passes beneath it on glass. */}
      <div ref={band} data-band className="flex h-pane-head flex-none">
        {strip}
      </div>
      <div className={cn("min-h-0 flex-1 bg-card", bodyClass)}>{children}</div>
    </>
  );
}
