// **Every tab strip in the app, with everything that comes with one**
// (docs/SHELL.md 第四轮 第 9 条:「标签页的逻辑也要统一, 也要抽象」) — the
// `SideList` of tabs.
//
// Three places draw tabs (a session's, a workspace's files, and the
// browse landing's narrow screen). Until this file they shared only the
// drawing (`TabStrip`), and each wired the rest for itself: whether the
// count opens a switcher, where the switcher goes, what picking in it
// does. The third round's missing `onSwitch` on the workspace was that
// wiring, forgotten in one of the three. Here it is written once:
//
//   strip     `TabStrip` — compress, overflow, the wheel, into view;
//   count     a switch (第 10 条:「点击计数按钮展开后, 再次点击要能收起」):
//             press, the switcher is up; press again, it goes and the tab
//             in front is the one that was; Esc and a pick put it away too;
//   switcher  `TabSwitcher`, with its box, in the region under the strip —
//             so the strip stays and still says which tab you will come
//             back to.
//
// A place hands in its tabs, which is in front, and what focusing and
// closing one does; what the region shows when the switcher is down is
// its `children`.
//
// **What is not here**: 标签页上限. It is enforced where a tab is
// *opened*, which is the tab store's business and not the strip's — the
// browse tabs' in the node (`khor tabs open` obeys it too), the other two
// in their hooks through one shared judgment (`tab-limit.ts`).
import { useEffect, useState, type ReactNode } from "react";

import { TabStrip, TabSwitcher, type TabFace } from "@/browse/TabStrip";
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
  glass = false,
  children,
}: {
  /** The strip's glass look (`TabStrip` `glass`): the session tabs. */
  glass?: boolean;
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
  const [open, setOpen] = useState(false);
  // A tab put in front from anywhere — the list beside, a shortcut — is
  // an answer to 「which tab」, and the switcher was the question.
  useEffect(() => setOpen(false), [active]);
  useEffect(() => {
    if (faces.length === 0) setOpen(false);
  }, [faces.length]);
  useEffect(() => {
    if (!open) return;
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [open]);
  const strip = (
    <div {...stripData} className="flex min-w-0 flex-1 flex-col">
      <TabStrip
        faces={faces}
        active={active}
        onFocus={onFocus}
        onClose={onClose}
        onNew={onNew}
        head={head}
        tail={tail}
        onSwitch={() => setOpen((o) => !o)}
        switching={open}
        glass={glass}
      />
    </div>
  );
  const body = open ? (
    <TabSwitcher
      faces={faces}
      active={active}
      onFocus={(id) => {
        onFocus(id);
        setOpen(false);
      }}
      onClose={onClose}
    />
  ) : (
    children
  );
  return frame ? (
    <Region kind="tabs" head={strip} caption={caption} className={bodyClass}>
      {body}
    </Region>
  ) : (
    <>
      {/* A band like a region's head (`lib/scroll-edge`): what scrolls
          under the strip passes beneath it on glass. */}
      <div ref={band} data-band className="flex h-pane-head flex-none">
        {strip}
      </div>
      <div className={cn("min-h-0 flex-1 bg-card", bodyClass)}>{body}</div>
    </>
  );
}
