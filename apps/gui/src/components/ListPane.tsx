// The app's **one** secondary sidebar.
//
// Every landing that keeps a column beside its content draws this: the
// sessions / machines / workspaces list, 额度's accounts, the
// workspace's file tree and its sessions column. Before 2026-09-20 each
// of those spelled the same four things out for itself — a ground, a
// top row, a scroller, a width remembered at a seam — and they had
// drifted apart on the first of them: 额度's column was the app's
// `--background` while every other was `--card`, which is what the user
// saw ( docs/SHELL.md 第 6 条:「除了额度页面之外的侧边栏底色都是深灰,
// 但额度页是黑色. 这看起来没有用统一的逻辑进行编写」).
//
// **The judgment that batch was given is not「fix the七处」, it is
//「同类的东西只剩一份写法」.** So this file owns all four, and a
// landing hands in only what is its own: what the top row holds and
// what the body lists.
//
//   ground   `bg-card`, one token, every pane (第 6 条).
//   head     whatever the caller passes, at `--pane-head` — the same
//            height the tab strip's band takes, because the two sit at
//            the same line of the window (第 5 条, tokens.css).
//   scroll   the body is the scroller and nothing above it moves; the
//            scroll stops at this pane's edge rather than handing the
//            rest to the page (第 1 条, `app.css`).
//   width    the seam is drawn here too, so a pane and the thing that
//            sizes it cannot be added one without the other (第 4 条:
//            every landing with a column can put it away, because the
//            rail toggles `pane` and every pane has one).
import { useLayoutEffect, useState, type CSSProperties, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { IconReload } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { PULL_HOLD, usePullRefresh, type PullState } from "@/hooks/use-pull-refresh";

import { SideListSeat } from "@/components/SideHead";
import type { PaneWidth } from "@/hooks/use-list-width";
import { cn } from "@/lib/utils";

export function ListPane({
  mark,
  name,
  pane,
  narrow,
  side = "left",
  seam,
  mountSeamFirst = false,
  head,
  scroll = true,
  seat,
  ground,
  className,
  onRefresh,
  children,
}: {
  /** Pull to refresh under a finger (`use-pull-refresh`): what a pull
      past the threshold asks. Absent, the list does not pull. */
  onRefresh?: () => Promise<unknown>;
  /** The data attribute this pane answers to (`data-list`,
      `data-ws-tree`…). **A prop rather than one name for all of them**:
      the gates and the smoke reach for a particular column, and a
      single shared attribute would make「the workspace's tree」
      unsayable. The shell is one; the addresses stay four. */
  mark: string;
  /** Read aloud, never drawn: no pane in this app carries a visible
      title (docs/UX.md). */
  name: string;
  /**
   * The width this pane was dragged to, and whether it is put away.
   * `null` where a pane has no seam — on the narrow face there is one
   * screen, so there is nothing beside it to give room to.
   */
  pane: PaneWidth | null;
  narrow: boolean;
  /** Which side of the content this pane sits on: the rule between
      them belongs to the pane, and it is the edge facing the content. */
  side?: "left" | "right";
  /** The attributes the seam wears, or nothing where the caller draws
      its own. Only ever drawn beside a `pane`, and never on narrow. */
  seam?: Record<string, string>;
  /** Whether the seam is laid down **before** the pane in the row.
      A pane on the right of the content is grabbed by its left edge,
      so its seam is the element before it; a pane on the left, after.
      Both spellings are one prop rather than the caller placing the
      seam itself, because a pane whose seam was somebody else's to
      draw is exactly how 额度 came to have neither. */
  mountSeamFirst?: boolean;
  /** The top row — a `PaneBar`, or a head a landing built itself. */
  head?: ReactNode;
  /**
   * Whether the shell is the scroller. False where the body is not one
   * list: the file tree holds its own scroller under a sticky head of
   * its own, and the sessions column swaps its list for a whole session.
   */
  scroll?: boolean;
  /**
   * `"side"`: this is **the** sidebar — the column beside the rail —
   * and it is one block with it (docs/SHELL.md 第十三轮 第 1 条): the
   * block's ground rather than the card's, no rule at its right edge
   * (the change of ground is the edge), and on the wide face it stands
   * in the block under the first row wherever the landing wrote it
   * (`SideListSeat`). The workspace's own two columns pass nothing and
   * keep the card.
   */
  seat?: "side";
  /**
   * `"side"` without a seat: on the block's ground, in its own place
   * — the workspace's tree and its sessions column, which are sidebars
   * of the page they stand in (2026-09-20 第 6 条: every secondary
   * sidebar on one ground) but not the one beside the rail.
   */
  ground?: "side";
  className?: string;
  children: ReactNode;
}) {
  // Wide, the width is the viewer's own, dragged at the seam and
  // remembered (`useListWidth`); put away it is zero, **not unmounted**
  // — what a pane holds (a typed search, a scroll position) is state
  // worth keeping through a collapse. Narrow keeps the class-driven
  // flow: there the pane is the whole screen.
  const away = pane !== null && pane.collapsed;
  const [body, setBody] = useState<HTMLDivElement | null>(null);
  const pull = usePullRefresh(body, onRefresh);
  const pulling = !!onRefresh && (pull.state !== "idle" || pull.pull > 0);
  const dragging = pull.state === "pulling" || pull.state === "armed";
  // **Narrow, the title rides too**: there the large title is the
  // screen's own head (`SideHead`, `[data-side-l0]`), a sibling of this
  // pane under the nav host, so the pull is said on the host and the
  // spinner stands above the title, where iOS puts it (the audit fix
  // plan F6.5). Wide, the pane's own head and list ride and the
  // spinner stands at the pane's top.
  const host = narrow ? (body?.closest<HTMLElement>("[data-nav-host]") ?? null) : null;
  const [above, setAbove] = useState<{ top: number; left: number; width: number } | null>(null);
  useLayoutEffect(() => {
    if (!host) return;
    host.style.setProperty("--pull", `${pull.pull}px`);
    if (pulling) host.dataset.pullOn = "";
    else delete host.dataset.pullOn;
    if (dragging) host.dataset.pullDrag = "";
    else delete host.dataset.pullDrag;
  }, [host, pull.pull, pulling, dragging]);
  useLayoutEffect(() => {
    if (!host || !pulling) {
      setAbove(null);
      return;
    }
    const head = host.querySelector("[data-side-l0]")?.getBoundingClientRect();
    const box = body?.getBoundingClientRect();
    if (head && box) setAbove({ top: head.top - pull.pull, left: box.left, width: box.width });
    // Measured once as a pull begins; the title's place does not move
    // under a finger.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [host, pulling]);
  // The seam between this pane and the content: drag to size, drag
  // under the snap line to put the pane away, double-click for the
  // default, arrows to walk its stops. It takes no room — a hit margin
  // overhanging the pane's border — so a pane that is away paints it as
  // the one visible strip left, which is also the way to pull the pane
  // back out.
  //
  // **Reachable from a keyboard**, which the list pane's was not for
  // its first two batches: it had the role and the pointer gestures and
  // nothing else, so on a keyboard a pane could not be put away or
  // brought back at all.
  const rule =
    seam && pane !== null && !narrow ? (
      <div
        {...seam}
        role="separator"
        aria-orientation="vertical"
        aria-label={gui.list_seam}
        tabIndex={0}
        {...pane.aria}
        {...pane.handleProps}
        className={cn(
          "relative z-10 -mx-1 w-2 shrink-0 cursor-col-resize transition-colors",
          // Focus has to be visible on a strip that is mostly not
          // there: the ring is the whole seam lighting up, because
          // there is no glyph to put a ring around.
          "outline-none hover:bg-border focus-visible:bg-ring",
          away && "bg-border",
        )}
      />
    ) : null;
  const onSide = seat === "side";
  const pane_ = (
    <>
      {mountSeamFirst && rule}
      <section
        {...{ [mark]: "" }}
        aria-label={name}
        // A pull moves the pane's own parts, never its box (`app.css`
        // `[data-pull-host]`): the head and the list ride `--pull` down
        // and the spinner stands in the room they leave. Only while a
        // pull is on: a transform at rest would still make the head a
        // stacking context and a containing block for anything fixed.
        data-pull-host={pulling ? "" : undefined}
        data-pull-drag={dragging ? "" : undefined}
        className={cn(
          "flex h-full min-h-0 flex-col overflow-hidden",
          onRefresh && "relative",
          onSide || ground === "side" ? "bg-side" : "bg-card",
          !narrow && !away && !onSide && (side === "left" ? "border-r" : "border-l"),
          // **The sidebar beside the rail takes the block's width, and
          // sets none of its own** (docs/SHELL.md 第十五轮 第 4 条): the
          // one number is `--list-w` on the block (`App`, `app.css`),
          // and a second `style.width` here is how the block and its
          // list came to disagree (挂账 2).
          onSide && !narrow && "min-w-0 flex-1",
          className,
        )}
        style={{
          ...(narrow || pane === null || onSide ? {} : { width: away ? 0 : pane.width }),
          ...(onRefresh ? ({ "--pull": `${pull.pull}px` } as CSSProperties) : {}),
        }}
      >
        {onRefresh &&
          (host ? above && createPortal(<PullSpinner {...pull} at={above} />, document.body) : <PullSpinner {...pull} />)}
        {head}
        {scroll ? (
          // **Named so the shell's scroll rule has something to point
          // at** — `app.css` keys the containment off the overflow
          // utility itself, and this attribute is what a probe measures
          // (第 1 条).
          <div ref={setBody} data-pane-body className="min-h-0 flex-1 overflow-y-auto">
            {children}
          </div>
        ) : (
          children
        )}
      </section>
      {!mountSeamFirst && rule}
    </>
  );
  return onSide && !narrow ? <SideListSeat>{pane_}</SideListSeat> : pane_;
}

/**
 * The spinner a pull shows above the pane's title (the audit fix plan
 * F6.5): the family's reload mark, turning with the pull and spinning
 * while the shell asks — and no word on screen. The four moments are
 * said to a screen reader only, in the catalog's words.
 */
function PullSpinner({
  state,
  pull,
  at,
}: {
  state: PullState;
  pull: number;
  /** Narrow: where the screen's title stood when the pull began — the
      spinner is fixed over the room the title leaves. */
  at?: { top: number; left: number; width: number };
}) {
  const word =
    state === "pulling"
      ? gui.pull_to_sync
      : state === "armed"
        ? gui.release_to_sync
        : state === "syncing"
          ? gui.syncing
          : state === "done"
            ? gui.synced_now
            : "";
  return (
    <div
      data-pull={state}
      aria-hidden={state === "idle" || undefined}
      className={cn(
        "pointer-events-none flex items-center justify-center overflow-hidden text-muted-foreground",
        at ? "fixed z-50" : "absolute inset-x-0",
      )}
      style={
        {
          height: pull,
          // Wide, under the band over the column (`lib/scroll-edge`): the
          // list's top is the band's bottom, and the room opens there.
          top: at ? at.top : "var(--band-h)",
          ...(at && { left: at.left, width: at.width }),
          "--pull-turn": `${Math.min(1, pull / PULL_HOLD)}turn`,
        } as CSSProperties
      }
    >
      {(state !== "idle" || pull > 0) && (
        <IconReload
          className={cn(
            "size-5 flex-none",
            state === "syncing" ? "animate-spin motion-reduce:animate-none" : "rotate-(--pull-turn)",
          )}
        />
      )}
      <span role="status" className="sr-only">
        {word}
      </span>
    </div>
  );
}
